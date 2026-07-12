use cairn_cas::{Cas, CasLimits};
use cairn_consolidator::{
    Consolidator, InternalMiningLimits, ProofDatum, ProofGraph, ProofGraphArchive,
    ProofGraphArtifactMetadata, ProofNode, ProofValueType,
};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BlobId, BudgetRecord, CanonicalValue, Hash32, ProducerRecord,
    SchemaRef, SemanticsVersion,
};
use cairn_curriculum::{
    AutonomousCurriculum, CurriculumCheckpointArchive, CurriculumCheckpointMetadata,
    DevelopmentalNeed, DevelopmentalObjective, DevelopmentalOutcome, ExperienceId,
    FailureCoreExtractor, FailureDrivenObjectiveGenerator, LearnedMechanismStatus, LearningAxis,
    MechanismTarget, SustainedLearningGate, SustainedLearningPoint,
};
use cairn_invent::{BindingExample, InventionError, InventionLimits, InventionProblem};
use cairn_memory::{
    InternalMechanismStore, InternalPromotionProgress, InternalReuseArtifactArchive,
    InternalReuseReplay, ModuleStatus, ReuseArtifactMetadata,
};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};
use cairn_verifier::InternalMechanismVerifier;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
struct FamilySpec {
    objective: &'static str,
    target: &'static str,
    operations: [&'static str; 4],
    base_episode: u8,
    first_host: (usize, usize),
    second_host: (usize, usize),
}

#[derive(Clone)]
struct PersistedFamily {
    graph_blobs: Vec<BlobId>,
    receipt_blobs: Vec<BlobId>,
    mechanism: Hash32,
    proposal: Hash32,
}

#[test]
#[allow(clippy::too_many_lines)] // Frozen multi-phase autonomous/restart audit.
fn autonomous_curriculum_grows_and_retains_a_restartable_mechanism_library() {
    let families = families();
    let fixture = TemporaryStore::new();
    let mut cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let mut curriculum =
        AutonomousCurriculum::new(objectives(), 12).expect("autonomous curriculum");
    let mut store = InternalMechanismStore::new();
    let mut persisted = Vec::<PersistedFamily>::new();
    let mut retention_hosts = Vec::<ProofGraph>::new();
    let mut points = Vec::<SustainedLearningPoint>::new();
    let mut completed_restarts = 0_u32;
    let mut curriculum_restores = 0_u32;
    let mut curriculum_checkpoints = Vec::new();
    let mut selected = Vec::new();

    for stage in 0..families.len() {
        let decision = curriculum.select_next().expect("autonomous base decision");
        selected.push(decision.objective.0.clone());
        let spec = families
            .iter()
            .find(|spec| spec.objective == decision.objective.0)
            .expect("generator owns selected objective");
        let sources = source_cohort(spec);
        let hosts = held_out_hosts(spec);
        let proposal = Consolidator
            .mine_internal_proof_subgraphs(&sources, limits())
            .expect("mine selected family");
        assert_eq!(
            proposal.pattern()[0].operation,
            spec.operations[0],
            "learner must discover the generator's hidden internal family"
        );
        let mechanism = Consolidator
            .compile_internal_proof_mechanism(&proposal)
            .expect("compile selected family");
        let compilation = InternalMechanismVerifier
            .verify_compilation(&proposal, &mechanism, &sources)
            .expect("verify selected family");
        let id = store
            .insert_provisional(mechanism.clone(), compilation.clone())
            .expect("new semantic mechanism");
        let receipts = hosts
            .iter()
            .map(|host| {
                let binding = Consolidator
                    .match_compiled_internal_mechanism(&mechanism, host, 64)
                    .expect("bind held-out host")
                    .remove(0);
                InternalMechanismVerifier
                    .verify_reuse(
                        &proposal,
                        &mechanism,
                        &compilation,
                        &sources,
                        host,
                        &binding,
                        64,
                    )
                    .expect("verify held-out reuse")
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            store
                .record_runtime_reuse(id, receipts[0].clone())
                .expect("first reuse"),
            InternalPromotionProgress::Pending { verified_reuses: 1 }
        ));
        assert!(matches!(
            store
                .record_runtime_reuse(id, receipts[1].clone())
                .expect("second reuse"),
            InternalPromotionProgress::Promoted(_)
        ));
        assert_eq!(
            store.get(id).expect("learned module").status(),
            ModuleStatus::Persistent
        );

        let retained = retention_hosts
            .iter()
            .enumerate()
            .filter(|(index, host)| {
                store
                    .get(persisted[*index].mechanism)
                    .is_some_and(|module| {
                        Consolidator
                            .match_compiled_internal_mechanism(module.mechanism(), host, 64)
                            .is_ok()
                    })
            })
            .count();
        assert_eq!(retained, retention_hosts.len());
        let negative_host = negative_host(spec.base_episode.saturating_add(9));
        let mut transfer_attempts = 1_u32;
        let mut negative_transfers = u32::from(
            Consolidator
                .match_compiled_internal_mechanism(&mechanism, &negative_host, 64)
                .is_ok(),
        );
        for prior in &retention_hosts {
            transfer_attempts = transfer_attempts.saturating_add(1);
            negative_transfers = negative_transfers.saturating_add(u32::from(
                Consolidator
                    .match_compiled_internal_mechanism(&mechanism, prior, 64)
                    .is_ok(),
            ));
        }
        assert_eq!(negative_transfers, 0);

        let stored = persist_family(&cas, &sources, &hosts, &receipts);
        persisted.push(PersistedFamily {
            graph_blobs: stored.0,
            receipt_blobs: stored.1,
            mechanism: mechanism.semantic_hash(),
            proposal: proposal.semantic_hash(),
        });
        retention_hosts.push(hosts[0].clone());

        let metrics = receipts[0].metrics();
        curriculum
            .record(DevelopmentalOutcome {
                decision_sequence: decision.sequence,
                objective: decision.objective,
                mechanism_evidence: mechanism.semantic_hash().to_hex(),
                independently_verified: true,
                held_out_transfer_cases: 2,
                candidate_work_without: metrics.candidate_windows_without,
                candidate_work_with: metrics.candidate_windows_with,
                examples_without: metrics.examples_without,
                examples_with: metrics.examples_with,
                retained_prior_tasks: u32::try_from(retained).expect("small"),
                prior_tasks: u32::try_from(stage).expect("small"),
                negative_transfers,
            })
            .expect("curriculum accepts measured learning");
        points.push(SustainedLearningPoint {
            learned_mechanisms: u32::try_from(store.len()).expect("small"),
            solved_capabilities: u32::try_from(store.len()).expect("small"),
            evaluated_capabilities: u32::try_from(families.len()).expect("small"),
            candidate_work_without: metrics.candidate_windows_without,
            candidate_work_with: metrics.candidate_windows_with,
            examples_without: metrics.examples_without,
            examples_with: metrics.examples_with,
            proof_units_without: metrics.proof_units_without,
            proof_units_with: metrics.proof_units_with,
            retained_prior_tasks: u32::try_from(retained).expect("small"),
            prior_tasks: u32::try_from(stage).expect("small"),
            negative_transfers,
            transfer_attempts,
            completed_restarts,
        });

        if stage == 1 || stage + 1 == families.len() {
            if stage + 1 == families.len() {
                let target = MechanismTarget("target-d".to_owned());
                assert_eq!(
                    curriculum
                        .record_counterexample(
                            &target,
                            "held-out-overlap-failure".to_owned(),
                            true,
                        )
                        .expect("quarantine failed mechanism"),
                    LearnedMechanismStatus::Quarantined
                );
                assert_eq!(
                    curriculum
                        .record_counterexample(
                            &target,
                            "held-out-nesting-failure".to_owned(),
                            true,
                        )
                        .expect("deprecate failed mechanism"),
                    LearnedMechanismStatus::Deprecated
                );
            }
            let curriculum_parent = parent(&cas, &format!("curriculum-stage-{stage}"));
            let checkpoint = CurriculumCheckpointArchive
                .persist(
                    &cas,
                    &curriculum.checkpoint(),
                    CurriculumCheckpointMetadata {
                        parent_ids: vec![curriculum_parent],
                        producer: producer(),
                        budget: budget(),
                        created_at_logical: 1_000 + u64::try_from(stage).expect("small"),
                    },
                )
                .expect("persist curriculum checkpoint");
            curriculum_checkpoints.push(checkpoint.artifact_id);
            drop(cas);
            cas = Cas::open(&fixture.path, CasLimits::default()).expect("restart CAS");
            cas.recover().expect("recover CAS");
            curriculum = CurriculumCheckpointArchive
                .load(&cas, checkpoint.envelope_blob_id)
                .expect("restore curriculum by replay");
            curriculum_restores = curriculum_restores.saturating_add(1);
            store = reconstruct_store(&cas, &persisted);
            completed_restarts = completed_restarts.saturating_add(1);
            points.last_mut().expect("point").completed_restarts = completed_restarts;
            assert_eq!(store.len(), persisted.len());
            assert!(persisted.iter().all(|family| {
                store
                    .get(family.mechanism)
                    .is_some_and(|module| module.status() == ModuleStatus::Persistent)
            }));
        }
    }

    let active_support = BTreeSet::from([
        MechanismTarget(families[0].target.to_owned()),
        MechanismTarget(families[1].target.to_owned()),
    ]);
    let generated = FailureDrivenObjectiveGenerator
        .generate(&unresolved_composition_core(), &active_support)
        .expect("generate objective from failure core");
    assert_eq!(generated.need, DevelopmentalNeed::MechanismComposition);
    curriculum
        .add_objective(generated.objective.clone())
        .expect("admit generated objective");
    let generated_checkpoint = CurriculumCheckpointArchive
        .persist(
            &cas,
            &curriculum.checkpoint(),
            CurriculumCheckpointMetadata {
                parent_ids: vec![parent(&cas, "generated-objective")],
                producer: producer(),
                budget: budget(),
                created_at_logical: 2_000,
            },
        )
        .expect("persist generated objective event");
    curriculum_checkpoints.push(generated_checkpoint.artifact_id);
    curriculum = CurriculumCheckpointArchive
        .load(&cas, generated_checkpoint.envelope_blob_id)
        .expect("restore generated objective boundary");
    curriculum_restores = curriculum_restores.saturating_add(1);
    let composition = curriculum
        .select_next()
        .expect("generated composition objective");
    assert_eq!(composition.axis, LearningAxis::Composition);
    assert_eq!(composition.objective, generated.objective.id);
    assert_eq!(selected, ["family-a", "family-b", "family-c", "family-d"]);
    assert_eq!(completed_restarts, 2);
    assert_eq!(curriculum_restores, 3);
    assert_eq!(curriculum_checkpoints.len(), 3);
    assert_eq!(
        curriculum
            .mechanisms()
            .get(&MechanismTarget("target-d".to_owned()))
            .expect("restored deprecated mechanism")
            .status,
        LearnedMechanismStatus::Deprecated
    );
    SustainedLearningGate
        .verify(&points)
        .expect("sustained autonomous learning gate");
    assert_eq!(points[0].candidate_work_with, 9);
    assert!(points.iter().all(|point| point.candidate_work_with == 9));
    assert!(points.iter().all(|point| point.examples_without == 4));
    assert!(points.iter().all(|point| point.examples_with == 1));
    assert!(points.iter().all(|point| point.proof_units_with == 4));
    assert_eq!(store.len(), 4);
    eprintln!(
        "G6_AUTONOMOUS_INTERNAL selected=family-a,family-b,family-c,family-d next=generated-composition generated_core={} generated_objective={} mechanisms=1->2->3->4 mechanism_hashes={},{},{},{} curriculum_checkpoints={},{},{} candidate_windows=18->9_each examples=4->1_each proof_units=28->4_each restarts=2 curriculum_restores=3 deprecated=target-d retained=6/6 negative_transfer=0/10",
        generated.failure_core,
        generated.objective.id.0,
        persisted[0].mechanism,
        persisted[1].mechanism,
        persisted[2].mechanism,
        persisted[3].mechanism,
        curriculum_checkpoints[0],
        curriculum_checkpoints[1],
        curriculum_checkpoints[2],
    );
}

fn families() -> [FamilySpec; 4] {
    [
        FamilySpec {
            objective: "family-a",
            target: "target-a",
            operations: [
                "select-role",
                "filter-relation",
                "map-effect",
                "render-effect",
            ],
            base_episode: 1,
            first_host: (5, 3),
            second_host: (2, 2),
        },
        FamilySpec {
            objective: "family-b",
            target: "target-b",
            operations: [
                "collect-members",
                "order-members",
                "fold-effect",
                "emit-fold",
            ],
            base_episode: 21,
            first_host: (5, 3),
            second_host: (2, 2),
        },
        FamilySpec {
            objective: "family-c",
            target: "target-c",
            operations: ["trace-links", "guard-terminal", "project-path", "emit-path"],
            base_episode: 41,
            first_host: (5, 3),
            second_host: (2, 2),
        },
        FamilySpec {
            objective: "family-d",
            target: "target-d",
            operations: [
                "group-equivalent",
                "select-canonical",
                "map-symbol",
                "emit-symbol",
            ],
            base_episode: 61,
            first_host: (5, 3),
            second_host: (2, 2),
        },
    ]
}

fn objectives() -> Vec<DevelopmentalObjective> {
    let specs = families();
    let hypotheses = [41, 31, 21, 11];
    let objectives = specs
        .iter()
        .zip(hypotheses)
        .enumerate()
        .map(
            |(index, (spec, competing_hypotheses))| DevelopmentalObjective {
                id: ExperienceId(spec.objective.to_owned()),
                axis: [
                    LearningAxis::CausalLaw,
                    LearningAxis::Cardinality,
                    LearningAxis::Value,
                    LearningAxis::Layout,
                ][index],
                produces: MechanismTarget(spec.target.to_owned()),
                prerequisites: BTreeSet::new(),
                competing_hypotheses,
                expected_transfer_microunits: 100,
                action_cost: 1,
                irreversible_risk: 0,
            },
        )
        .collect::<Vec<_>>();
    objectives
}

fn unresolved_composition_core() -> cairn_curriculum::LearningFailureCore {
    let integer = Type::Int(IntRange { lower: 0, upper: 8 });
    let left = VarName("anonymous-left".to_owned());
    let right = VarName("anonymous-right".to_owned());
    let examples = [(0, 0, 0), (0, 2, 2), (3, 0, 3), (2, 4, 6)]
        .into_iter()
        .map(|(a, b, output)| {
            let mut inputs = RuntimeEnvironment::new();
            inputs.insert(left.clone(), Value::Int(a));
            inputs.insert(right.clone(), Value::Int(b));
            BindingExample {
                inputs,
                output: Value::Int(output),
            }
        })
        .collect();
    let problem = InventionProblem {
        definition_name: "anonymous-unresolved-composition".to_owned(),
        parameters: vec![
            Binder {
                name: left,
                ty: integer.clone(),
            },
            Binder {
                name: right,
                ty: integer.clone(),
            },
        ],
        return_type: integer,
        literals: vec![Value::Int(0)],
        demonstrations: examples,
        limits: InventionLimits::default(),
    };
    FailureCoreExtractor
        .extract(
            &problem,
            &InventionError::CandidateCapacity { maximum: 1_000 },
        )
        .expect("redacted failure core")
}

fn source_cohort(spec: &FamilySpec) -> [ProofGraph; 4] {
    [
        host_graph(spec, spec.base_episode, 1, 1, "triangle", "red"),
        host_graph(spec, spec.base_episode + 1, 2, 2, "circle", "blue"),
        host_graph(spec, spec.base_episode + 2, 3, 1, "segment", "green"),
        host_graph(spec, spec.base_episode + 3, 1, 3, "polygon", "yellow"),
    ]
}

fn held_out_hosts(spec: &FamilySpec) -> [ProofGraph; 2] {
    [
        host_graph(
            spec,
            spec.base_episode + 10,
            spec.first_host.0,
            spec.first_host.1,
            "spiral",
            "violet",
        ),
        host_graph(
            spec,
            spec.base_episode + 11,
            spec.second_host.0,
            spec.second_host.1,
            "wave",
            "orange",
        ),
    ]
}

fn host_graph(
    spec: &FamilySpec,
    episode: u8,
    prefix_nodes: usize,
    suffix_nodes: usize,
    role: &str,
    effect: &str,
) -> ProofGraph {
    let mut nodes = vec![ProofNode {
        operation: "host-observation".to_owned(),
        output_type: ProofValueType::EntitySet,
        inputs: vec![],
        arguments: vec![datum(ProofValueType::Atom, &format!("world-{episode}"))],
    }];
    for index in 1..prefix_nodes {
        nodes.push(ProofNode {
            operation: format!("host-prefix-{episode}-{index}"),
            output_type: ProofValueType::Integer,
            inputs: vec![],
            arguments: vec![datum(ProofValueType::Integer, &index.to_string())],
        });
    }
    let start = nodes.len();
    nodes.extend([
        ProofNode {
            operation: spec.operations[0].to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![0],
            arguments: vec![datum(ProofValueType::Atom, role)],
        },
        ProofNode {
            operation: spec.operations[1].to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![u32::try_from(start).expect("small")],
            arguments: vec![datum(ProofValueType::Relation, "inside")],
        },
        ProofNode {
            operation: spec.operations[2].to_owned(),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(start + 1).expect("small")],
            arguments: vec![datum(ProofValueType::Atom, effect)],
        },
        ProofNode {
            operation: spec.operations[3].to_owned(),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(start + 2).expect("small")],
            arguments: vec![],
        },
    ]);
    let mut previous = start + 3;
    for index in 0..suffix_nodes {
        nodes.push(ProofNode {
            operation: format!("host-suffix-{episode}-{index}"),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(previous).expect("small")],
            arguments: vec![],
        });
        previous += 1;
    }
    ProofGraph {
        episode: Hash32::digest(&[episode]),
        output: u32::try_from(nodes.len() - 1).expect("small"),
        nodes,
    }
}

fn negative_host(episode: u8) -> ProofGraph {
    let spec = FamilySpec {
        objective: "negative",
        target: "negative",
        operations: ["unrelated-a", "unrelated-b", "unrelated-c", "unrelated-d"],
        base_episode: episode,
        first_host: (2, 1),
        second_host: (2, 1),
    };
    host_graph(&spec, episode, 2, 1, "noise", "none")
}

const fn limits() -> InternalMiningLimits {
    InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    }
}

fn persist_family(
    cas: &Cas,
    sources: &[ProofGraph],
    hosts: &[ProofGraph],
    receipts: &[cairn_verifier::VerifiedInternalReuse],
) -> (Vec<BlobId>, Vec<BlobId>) {
    let graph_archive = ProofGraphArchive;
    let stored = sources
        .iter()
        .chain(hosts)
        .enumerate()
        .map(|(index, graph)| {
            graph_archive
                .persist(
                    cas,
                    graph,
                    ProofGraphArtifactMetadata {
                        parent_ids: vec![parent(cas, &format!("graph-{index}-{}", graph.episode))],
                        producer: producer(),
                        budget: budget(),
                        created_at_logical: u64::try_from(index).expect("small"),
                    },
                )
                .expect("persist curriculum proof")
        })
        .collect::<Vec<_>>();
    let parents = stored
        .iter()
        .map(|item| item.artifact_id)
        .collect::<Vec<_>>();
    let receipt_archive = InternalReuseArtifactArchive;
    let receipts = receipts
        .iter()
        .enumerate()
        .map(|(index, receipt)| {
            receipt_archive
                .persist(
                    cas,
                    receipt,
                    ReuseArtifactMetadata {
                        parent_ids: parents.clone(),
                        producer: producer(),
                        budget: budget(),
                        created_at_logical: u64::try_from(index).expect("small") + 100,
                    },
                )
                .expect("persist curriculum reuse")
                .envelope_blob_id
        })
        .collect();
    (
        stored.iter().map(|item| item.envelope_blob_id).collect(),
        receipts,
    )
}

fn reconstruct_store(cas: &Cas, families: &[PersistedFamily]) -> InternalMechanismStore {
    let graph_archive = ProofGraphArchive;
    let receipt_archive = InternalReuseArtifactArchive;
    let mut store = InternalMechanismStore::new();
    for family in families {
        let graphs = family
            .graph_blobs
            .iter()
            .map(|blob| graph_archive.load(cas, *blob).expect("reload graph"))
            .collect::<Vec<_>>();
        let sources = &graphs[..4];
        let hosts = &graphs[4..];
        let proposal = Consolidator
            .mine_internal_proof_subgraphs(sources, limits())
            .expect("remine family");
        let mechanism = Consolidator
            .compile_internal_proof_mechanism(&proposal)
            .expect("recompile family");
        assert_eq!(proposal.semantic_hash(), family.proposal);
        assert_eq!(mechanism.semantic_hash(), family.mechanism);
        let compilation = InternalMechanismVerifier
            .verify_compilation(&proposal, &mechanism, sources)
            .expect("reverify family");
        let id = store
            .insert_provisional(mechanism.clone(), compilation.clone())
            .expect("restore provisional family");
        for (blob, host) in family.receipt_blobs.iter().zip(hosts) {
            let artifact = receipt_archive.load(cas, *blob).expect("reload receipt");
            let binding = Consolidator
                .match_compiled_internal_mechanism(&mechanism, host, 64)
                .expect("rebind family")
                .remove(0);
            let receipt = receipt_archive
                .replay(
                    artifact,
                    InternalReuseReplay {
                        proposal: &proposal,
                        mechanism: &mechanism,
                        compilation: &compilation,
                        sources,
                        held_out: host,
                        binding: &binding,
                        maximum_match_windows: 64,
                    },
                )
                .expect("replay family receipt");
            store
                .record_runtime_reuse(id, receipt)
                .expect("restore family ledger");
        }
        assert_eq!(
            store.get(id).expect("restored family").status(),
            ModuleStatus::Persistent
        );
    }
    store
}

fn datum(ty: ProofValueType, value: &str) -> ProofDatum {
    ProofDatum::digest(ty, value.as_bytes())
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.autonomous-curriculum", 1).expect("schema"),
        ArtifactKind::new("test.autonomous-curriculum").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        Vec::new(),
        None,
        producer(),
        budget(),
        0,
        CanonicalValue::Text(name.to_owned()),
    )
    .expect("parent");
    cas.put_envelope(&envelope)
        .expect("persist parent")
        .artifact_id
}

fn producer() -> ProducerRecord {
    ProducerRecord {
        component: "g6-autonomous-internal-curriculum".to_owned(),
        version: "0.1.0".to_owned(),
        commit: "test".to_owned(),
        host_run_id: "frozen".to_owned(),
        agent_id: Some("curriculum".to_owned()),
    }
}

const fn budget() -> BudgetRecord {
    BudgetRecord {
        work_units: 64,
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
                "cairn-g6-autonomous-{}-{sequence}",
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
