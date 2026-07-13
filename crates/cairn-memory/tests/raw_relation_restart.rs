use cairn_cas::{Cas, CasLimits};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalValue, EntityId, Hash32, ProducerRecord,
    SchemaRef, SemanticsVersion,
};
use cairn_curriculum::{
    DeterminismStatus, FailureDrivenObjectiveGenerator, LearningFailureCore, RedactionStatus,
    SearchFailure, ValueClass, FAILURE_CORE_SEMANTICS_VERSION,
};
use cairn_invent::InventionLimits;
use cairn_memory::{
    ModuleStatus, RawRelationCohortArchive, RawRelationCohortArchiveError, RawRelationStore,
    ReuseArtifactMetadata,
};
use cairn_rkl::ExecLimits;
use cairn_theory::{
    EntityObservationLimits, ObservationPredicateLimits, PredicateGroupingLimits,
    RawAttributedObservationPrimitive, RawRelationObservationInventor, RawRelationObservationTrace,
    RelationObservationError, RelationObservationLimits,
};
use cairn_verifier::{
    RawRelationObservationPromotionConsolidator, RawRelationObservationVerifier,
    RelationLanguageExtensionConsolidator, RelationLanguageExtensionLimits,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
#[allow(clippy::too_many_lines)] // Complete CAS authority-loss and re-promotion boundary.
fn complete_relation_program_is_reinvented_after_process_loss() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let relation_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = [
        [
            episode("restart-rel-sequence-a", &[2, 3, 4], 100, [0, 1, 2]),
            episode("restart-rel-table-a", &[2, 3, 4, 5], 300, [2, 0, 1]),
        ],
        [
            episode("restart-rel-vector-b", &[3, 4, 5], -500, [1, 2, 0]),
            episode("restart-rel-world-b", &[2, 3, 5, 6], 700, [2, 1, 0]),
        ],
    ];
    let learned = cohorts
        .iter()
        .map(|cohort| {
            RawRelationObservationInventor
                .invent(
                    cohort,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("initial relation invention")
        })
        .collect::<Vec<_>>();
    let receipts = learned
        .iter()
        .zip(&cohorts)
        .map(|(mechanism, cohort)| {
            RawRelationObservationVerifier
                .verify_reuse(
                    mechanism,
                    cohort,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("initial relation receipt")
        })
        .collect::<Vec<_>>();
    let archive = RawRelationCohortArchive;
    let stored = [0_usize, 1_usize].map(|index| {
        archive
            .persist(
                &cas,
                &learned[index],
                &cohorts[index],
                &receipts[index],
                metadata(
                    vec![parent(&cas, &format!("raw-relation-parent-{index}"))],
                    u64::try_from(index + 1).expect("index"),
                ),
            )
            .expect("persist raw relation evidence")
    });
    assert_eq!(
        stored
            .iter()
            .map(|artifact| artifact.artifact_id)
            .collect::<HashSet<_>>()
            .len(),
        2
    );
    assert!(matches!(
        archive.persist(
            &cas,
            &learned[0],
            &cohorts[0],
            &receipts[0],
            metadata(Vec::new(), 3),
        ),
        Err(RawRelationCohortArchiveError::MissingParent)
    ));

    drop(cas);
    drop(cohorts);
    drop(learned);
    drop(receipts);

    let reopened = Cas::open(&fixture.path, CasLimits::default()).expect("reopen CAS");
    reopened.recover().expect("recover CAS");
    let loaded = stored
        .iter()
        .map(|artifact| {
            archive
                .load(&reopened, artifact.envelope_blob_id)
                .expect("load evidence without authority")
        })
        .collect::<Vec<_>>();
    assert!(RawRelationStore::new()
        .persistent_modules()
        .next()
        .is_none());
    assert!(matches!(
        archive.replay(
            &loaded[0],
            RelationObservationLimits {
                maximum_relation_evaluations: 1,
                ..relation_limits
            },
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
            ExecLimits::default(),
        ),
        Err(RawRelationCohortArchiveError::Invention(
            RelationObservationError::Capacity
        ))
    ));
    let replayed = loaded
        .iter()
        .map(|artifact| {
            archive
                .replay(
                    artifact,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                    ExecLimits::default(),
                )
                .expect("fresh invention and independent replay")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        replayed[0].learned.mechanism_hash(),
        replayed[1].learned.mechanism_hash()
    );
    let promotion = RawRelationObservationPromotionConsolidator
        .consolidate(&[replayed[0].receipt.clone(), replayed[1].receipt.clone()])
        .expect("fresh relation consolidation");
    let mechanism = replayed[0]
        .learned
        .source_independent()
        .expect("source-independent replay");
    let extension = RelationLanguageExtensionConsolidator
        .consolidate(
            &mechanism,
            &promotion,
            &FailureDrivenObjectiveGenerator
                .generate(&restart_language_failure(), &BTreeSet::new())
                .expect("content-free restart language objective")
                .relation_language_growth_request(),
            RelationLanguageExtensionLimits::default(),
        )
        .expect("fresh language extension after evidence replay");
    assert_eq!(extension.maximum_key_arity(), 3);
    let mut store = RawRelationStore::new();
    let id = store
        .insert_provisional(mechanism)
        .expect("fresh provisional relation");
    for replay in replayed {
        store
            .record_verified_reuse(id, replay.receipt)
            .expect("fresh relation receipt");
    }
    assert!(store.apply_verified_promotion(id, &promotion));
    assert_eq!(
        store.module(id).expect("module").status(),
        ModuleStatus::Persistent
    );
    eprintln!(
        "RELATION_PROGRAM_RESTART artifacts={},{} authority_restored=0 cohorts_reinvented=2 repromoted=1 language_extensions_regenerated=1 structures_bindings=15->3 examples=4->0",
        stored[0].artifact_id, stored[1].artifact_id
    );
}

fn restart_language_failure() -> LearningFailureCore {
    LearningFailureCore {
        semantics_version: FAILURE_CORE_SEMANTICS_VERSION.to_owned(),
        failure: SearchFailure::NoDefinition { maximum_cost: 2 },
        parameter_types: vec![ValueClass::Entity, ValueClass::Relation],
        return_type: ValueClass::Relation,
        demonstrations: 4,
        varying_parameters: 3,
        distinct_output_classes: 3,
        constant_explanation_possible: false,
        direct_projection_candidates: 0,
        single_parameter_explanations: 0,
        exact_two_parameter_families: BTreeSet::new(),
        exact_three_parameter_families: BTreeSet::new(),
        determinism: DeterminismStatus::Deterministic,
        minimum_dependency_arity_lower_bound: 3,
        redaction: RedactionStatus::ContentFree,
    }
}

fn episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 3],
) -> RawRelationObservationTrace {
    let component_count = cardinalities.iter().sum::<usize>();
    let mut primitives = BTreeMap::new();
    let mut outputs = BTreeSet::new();
    let mut component = 0_usize;
    for (group, cardinality) in cardinalities.iter().copied().enumerate() {
        let signature = signature_offset + i64::try_from(group).expect("group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("cardinality"));
        for _ in 0..cardinality {
            let first = signature_offset * 3 + i64::try_from(component).expect("component") * 37;
            let second = signature_offset * -2 + i64::try_from(component).expect("component") * 19;
            let key = signature_offset * 20
                + 10_000
                + i64::try_from(component).expect("component key") * 1_000;
            for (member, observation) in [first, second, signature - first - second]
                .into_iter()
                .enumerate()
            {
                let id = EntityId(Hash32::digest(
                    format!("{domain}:{component}:{member}").as_bytes(),
                ));
                let first_key = i64::try_from(component).expect("component") * 100
                    + i64::try_from(member).expect("member");
                let logical = [
                    first_key,
                    key - first_key,
                    i64::try_from((component + member) % component_count).expect("distractor"),
                ];
                let mut attributes = vec![0_i64; 3];
                for (logical_index, destination) in permutation.into_iter().enumerate() {
                    attributes[destination] = logical[logical_index];
                }
                primitives.insert(
                    id,
                    RawAttributedObservationPrimitive {
                        observation,
                        attributes,
                    },
                );
            }
            component += 1;
        }
    }
    RawRelationObservationTrace {
        episode: Hash32::digest(domain.as_bytes()),
        primitives,
        outputs,
    }
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.raw-relation", 1).expect("schema"),
        ArtifactKind::new("test.raw-relation").expect("kind"),
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

fn metadata(parent_ids: Vec<cairn_core::ArtifactId>, logical: u64) -> ReuseArtifactMetadata {
    ReuseArtifactMetadata {
        parent_ids,
        producer: producer(),
        budget: budget(),
        created_at_logical: logical,
    }
}

fn producer() -> ProducerRecord {
    ProducerRecord {
        component: "raw-relation-restart-test".to_owned(),
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
                "cairn-raw-relation-restart-{}-{sequence}",
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
