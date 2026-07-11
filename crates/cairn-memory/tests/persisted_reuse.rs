use cairn_cas::{Cas, CasLimits, PutStatus};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalValue, CollectionDemonstration,
    CollectionProcedureProblem, Hash32, ProducerRecord, SchemaRef, SemanticsVersion,
};
use cairn_mechanism::{
    CardinalityRange, DerivationCertificate, DomainClass, Mechanism, Signature, SupportGeometry,
    VariableId,
};
use cairn_memory::{
    ModuleStatus, ProceduralMechanismStore, ReuseArchiveError, ReuseArtifactArchive,
    ReuseArtifactMetadata,
};
use cairn_rkl::{
    Binder, CheckedTheory, ConflictPolicy, Effect, Expr, Quantifier, Renderer, RewriteRule,
    RuleBinder, Schedule, Theory, TheoryChecker, Type, TypeChecker, VarName,
};
use cairn_verifier::{
    CounterfactualReuseMetrics, ProceduralExperimentBaseline, ProceduralPromotionConsolidator,
    PromotionVerifier,
};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
#[allow(clippy::too_many_lines)] // One end-to-end restart and promotion experiment.
fn persisted_reuse_replays_after_restart_and_drives_promotion() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let theory = identity_collection_theory("retain_member");
    let mechanism = mechanism(&theory);
    let problems = [
        problem(&[-3, 2], &[-8, 0, 7]),
        problem(&[-5, 1, 9], &[-4, 3, 11, 18]),
    ];
    let expected = [BTreeSet::from([-8, 0, 7]), BTreeSet::from([-4, 3, 11, 18])];
    let parent_ids = [parent(&cas, "experience-a"), parent(&cas, "experience-b")];
    let receipts = problems
        .iter()
        .zip(&expected)
        .enumerate()
        .map(|(index, (problem, expected))| {
            PromotionVerifier
                .verify_collection_reuse(
                    &theory,
                    problem,
                    expected,
                    Hash32::digest(format!("persisted-experience-{index}").as_bytes()),
                    metrics(),
                )
                .expect("initial independent verification")
        })
        .collect::<Vec<_>>();
    let archive = ReuseArtifactArchive;
    let stored = receipts
        .iter()
        .zip(parent_ids)
        .enumerate()
        .map(|(index, (receipt, parent_id))| {
            let evidence_metadata = metadata(parent_id, u64::try_from(index).expect("small") + 1);
            let first = archive
                .persist(&cas, receipt, evidence_metadata.clone())
                .expect("persist receipt");
            let repeated = archive
                .persist(&cas, receipt, evidence_metadata)
                .expect("idempotent persist");
            assert_eq!(first.artifact_id, repeated.artifact_id);
            assert_eq!(first.envelope_blob_id, repeated.envelope_blob_id);
            assert_eq!(repeated.status, PutStatus::AlreadyPresent);
            let mut observed_differently = metadata(parent_id, 10_000);
            observed_differently.producer.host_run_id = "different-process".to_owned();
            observed_differently.budget.cpu_nanos = 99;
            let alternate = archive
                .persist(&cas, receipt, observed_differently)
                .expect("non-semantic metadata variation");
            assert_eq!(first.artifact_id, alternate.artifact_id);
            first
        })
        .collect::<Vec<_>>();
    assert_eq!(
        stored
            .iter()
            .map(|artifact| artifact.artifact_id)
            .collect::<HashSet<_>>()
            .len(),
        2
    );

    drop(cas);
    drop(receipts);
    let reopened = Cas::open(&fixture.path, CasLimits::default()).expect("restart CAS");
    reopened.recover().expect("recover clean CAS");
    let loaded = stored
        .iter()
        .map(|stored| {
            archive
                .load(&reopened, stored.envelope_blob_id)
                .expect("load canonical receipt")
        })
        .collect::<Vec<_>>();
    let replayed = loaded
        .iter()
        .zip(&problems)
        .zip(&expected)
        .map(|((artifact, problem), expected)| {
            archive
                .replay(*artifact, &theory, problem, expected, metrics())
                .expect("independent restart replay")
        })
        .collect::<Vec<_>>();
    let token = ProceduralPromotionConsolidator
        .consolidate(
            &mechanism,
            &theory,
            ProceduralExperimentBaseline {
                target_absent_from_named_vocabulary: true,
                source_code_changes: 0,
                initial_success_parts_per_million: 0,
                uncompressed_cost: 30,
                compressed_cost: 5,
                false_acceptances_without: 0,
                false_acceptances_with: 0,
            },
            &replayed,
        )
        .expect("promotion from replayed persisted receipts");
    let mut restarted_store = ProceduralMechanismStore::new();
    let id = restarted_store
        .insert_provisional(
            theory.clone(),
            mechanism.support.clone(),
            mechanism.proof.source_proofs.clone(),
            mechanism.proof.verified_cases,
        )
        .expect("reconstruct provisional module");
    for receipt in &replayed {
        restarted_store
            .record_verifier_receipt(id, receipt)
            .expect("record replayed receipt");
    }
    assert!(restarted_store.apply_verified_promotion(id, &token));
    let module = restarted_store.modules().next().expect("promoted module");
    assert_eq!(module.status(), ModuleStatus::Persistent);
    assert_eq!(module.reuse_metrics().verified_invocations, 2);
    assert_eq!(module.reuse_metrics().necessary_invocations, 2);
    eprintln!(
        "PERSISTED_REUSE theory={} artifacts={},{} replayed=2 promoted=1 candidates=200->2 examples=8->2 proof_nodes=40->6",
        loaded[0].theory_hash(),
        stored[0].artifact_id,
        stored[1].artifact_id,
    );
}

#[test]
fn malformed_or_changed_persisted_reuse_fails_closed() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let theory = identity_collection_theory("retain_member");
    let problem = problem(&[-3, 2], &[-8, 0, 7]);
    let expected = BTreeSet::from([-8, 0, 7]);
    let receipt = PromotionVerifier
        .verify_collection_reuse(
            &theory,
            &problem,
            &expected,
            Hash32::digest(b"corruption-experience"),
            metrics(),
        )
        .expect("receipt");
    let archive = ReuseArtifactArchive;
    assert!(matches!(
        archive.persist(
            &cas,
            &receipt,
            ReuseArtifactMetadata {
                parent_ids: Vec::new(),
                ..metadata(parent(&cas, "unused"), 1)
            }
        ),
        Err(ReuseArchiveError::MissingParent)
    ));
    let stored = archive
        .persist(&cas, &receipt, metadata(parent(&cas, "valid"), 2))
        .expect("persist valid");
    let artifact = archive
        .load(&cas, stored.envelope_blob_id)
        .expect("load valid");
    assert!(matches!(
        archive.replay(
            artifact,
            &identity_collection_theory("changed_rule_identity"),
            &problem,
            &expected,
            metrics()
        ),
        Err(ReuseArchiveError::TheoryMismatch)
    ));
    assert!(matches!(
        archive.replay(
            artifact,
            &theory,
            &problem,
            &BTreeSet::from([99]),
            metrics()
        ),
        Err(ReuseArchiveError::VerificationFailure)
    ));
    let mut changed_metrics = metrics();
    changed_metrics.candidate_executions_without = 101;
    assert!(matches!(
        archive.replay(artifact, &theory, &problem, &expected, changed_metrics),
        Err(ReuseArchiveError::ReplayMismatch)
    ));

    let malformed = ArtifactEnvelope::new(
        SchemaRef::new(cairn_memory::REUSE_EVIDENCE_SCHEMA, 1).expect("schema"),
        ArtifactKind::new("memory.reuse-evidence").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        vec![parent(&cas, "malformed")],
        Some(Hash32::digest(b"malformed-scope")),
        producer(),
        budget(),
        3,
        CanonicalValue::Array(vec![CanonicalValue::Text("wrong-marker".to_owned())]),
    )
    .expect("malformed envelope remains structurally valid");
    let malformed = cas.put_envelope(&malformed).expect("store malformed");
    assert!(matches!(
        archive.load(&cas, malformed.envelope_blob_id),
        Err(ReuseArchiveError::InvalidPayload(_))
    ));
}

fn identity_collection_theory(rule_name: &str) -> CheckedTheory {
    let integer = Type::Int(cairn_rkl::IntRange {
        lower: -100,
        upper: 100,
    });
    let collection = VarName("collection".to_owned());
    let item = VarName("item".to_owned());
    TypeChecker::new(10_000)
        .check_theory(&Theory {
            inputs: vec![Binder {
                name: collection.clone(),
                ty: Type::List {
                    element: Box::new(integer.clone()),
                    maximum: 32,
                },
            }],
            definitions: Vec::new(),
            rules: vec![RewriteRule {
                name: rule_name.to_owned(),
                binders: vec![RuleBinder {
                    variable: Binder {
                        name: item.clone(),
                        ty: integer,
                    },
                    domain: Expr::Var(collection.clone()),
                    quantifier: Quantifier::ForEach,
                }],
                guard: Expr::Bool(true),
                effects: vec![Effect::Emit(Expr::Var(item))],
                priority: 0,
            }],
            schedule: Schedule::Sequence(vec![rule_name.to_owned()]),
            renderer: Renderer {
                canvas: Expr::Var(collection),
                layers: Vec::new(),
                conflict_policy: ConflictPolicy::SameValueIdempotent,
                background: Expr::Int(0),
            },
            assumptions: Vec::new(),
            proof_annotations: Vec::new(),
        })
        .expect("checked identity collection theory")
}

fn mechanism(theory: &CheckedTheory) -> Mechanism {
    Mechanism {
        signature: Signature {
            inputs: theory
                .theory()
                .inputs
                .iter()
                .map(|input| input.ty.clone())
                .collect(),
            output: theory.theory().inputs[0].ty.clone(),
        },
        variables: theory.theory().inputs.clone(),
        entity_definitions: theory.theory().definitions.clone(),
        preconditions: Expr::Bool(true),
        transition_rules: theory.theory().rules.clone(),
        invariants: vec![Expr::Bool(true)],
        termination: Expr::Bool(true),
        effects: theory
            .theory()
            .rules
            .iter()
            .flat_map(|rule| rule.effects.iter().cloned())
            .collect(),
        support: SupportGeometry {
            observed_domains: BTreeSet::from([
                DomainClass("sequence-batch".to_owned()),
                DomainClass("vector-entity-set".to_owned()),
            ]),
            observed_cardinalities: BTreeSet::from([CardinalityRange {
                minimum: 2,
                maximum: 4,
            }]),
            varied_variables: BTreeSet::from([VariableId("collection".to_owned())]),
            invariant_variables: BTreeSet::new(),
            unknown_variables: BTreeSet::new(),
            validated_compositions: BTreeSet::new(),
            failed_conditions: BTreeSet::new(),
        },
        proof: DerivationCertificate {
            source_proofs: BTreeSet::from([
                Hash32::digest(b"source-proof-a"),
                Hash32::digest(b"source-proof-b"),
            ]),
            verifier_semantics: "persisted-reuse-replay-v1".to_owned(),
            verified_cases: 4,
        },
    }
}

fn problem(demonstration: &[i64], prediction: &[i64]) -> CollectionProcedureProblem {
    CollectionProcedureProblem {
        maximum_cardinality: 32,
        demonstrations: vec![CollectionDemonstration {
            input: demonstration.to_vec(),
            output: demonstration.iter().copied().collect(),
        }],
        prediction_input: prediction.to_vec(),
        maximum_predictions: 1,
    }
}

const fn metrics() -> CounterfactualReuseMetrics {
    CounterfactualReuseMetrics {
        blank_solved: false,
        candidate_executions_without: 100,
        candidate_executions_with: 1,
        examples_without: 4,
        examples_with: 1,
        proof_nodes_without: 20,
        proof_nodes_with: 3,
    }
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.experience", 1).expect("schema"),
        ArtifactKind::new("test.experience").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        Vec::new(),
        None,
        producer(),
        budget(),
        0,
        CanonicalValue::Text(name.to_owned()),
    )
    .expect("parent envelope");
    cas.put_envelope(&envelope)
        .expect("persist parent")
        .artifact_id
}

fn metadata(parent_id: cairn_core::ArtifactId, logical: u64) -> ReuseArtifactMetadata {
    ReuseArtifactMetadata {
        parent_ids: vec![parent_id],
        producer: producer(),
        budget: budget(),
        created_at_logical: logical,
    }
}

fn producer() -> ProducerRecord {
    ProducerRecord {
        component: "persisted-reuse-test".to_owned(),
        version: "0.1.0".to_owned(),
        commit: "test".to_owned(),
        host_run_id: "restart-test".to_owned(),
        agent_id: Some("independent-verifier".to_owned()),
    }
}

const fn budget() -> BudgetRecord {
    BudgetRecord {
        work_units: 1,
        cpu_nanos: 0,
        peak_rss_bytes: 0,
    }
}

struct TemporaryStore {
    path: PathBuf,
}

impl TemporaryStore {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        Self {
            path: std::env::temp_dir().join(format!(
                "cairn-persisted-reuse-{}-{sequence}",
                std::process::id()
            )),
        }
    }
}

impl Drop for TemporaryStore {
    fn drop(&mut self) {
        let _ignored = std::fs::remove_dir_all(&self.path);
    }
}
