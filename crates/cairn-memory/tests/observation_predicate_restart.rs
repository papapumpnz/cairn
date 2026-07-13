use cairn_cas::{Cas, CasLimits};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalValue, EntityId, Hash32, ProducerRecord,
    SchemaRef, SemanticsVersion,
};
use cairn_invent::InventionLimits;
use cairn_memory::{
    ModuleStatus, ObservationPredicateCohortArchive, ObservationPredicateCohortArchiveError,
    ObservationPredicateStore, ReuseArtifactMetadata,
};
use cairn_rkl::ExecLimits;
use cairn_theory::{
    ObservationPredicateError, ObservationPredicateInventor, ObservationPredicateLimits,
    PredicateGroupingLimits, PrimitivePairEntity, PrimitivePairTrace,
};
use cairn_verifier::{ObservationPredicatePromotionConsolidator, ObservationPredicateVerifier};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
#[allow(clippy::too_many_lines)] // Exercises the complete process-loss trust boundary.
fn synthesized_feature_memory_is_reinvented_after_process_loss() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = source_cohorts();
    let learned = cohorts
        .iter()
        .map(|cohort| {
            ObservationPredicateInventor
                .invent(
                    cohort,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("initial invention")
        })
        .collect::<Vec<_>>();
    assert_eq!(learned[0].semantic_hash(), learned[1].semantic_hash());
    let receipts =
        [(&learned[0], &cohorts[0]), (&learned[1], &cohorts[1])].map(|(mechanism, cohort)| {
            ObservationPredicateVerifier
                .verify_reuse(
                    mechanism,
                    cohort,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("initial independent receipt")
        });
    let archive = ObservationPredicateCohortArchive;
    let parents = [
        vec![parent(&cas, "observation-feature-first")],
        vec![parent(&cas, "observation-feature-second")],
    ];
    let stored = [0, 1].map(|index| {
        archive
            .persist(
                &cas,
                &learned[index],
                &cohorts[index],
                &receipts[index],
                metadata(
                    parents[index].clone(),
                    u64::try_from(index + 1).expect("index"),
                ),
            )
            .expect("persist evidence cohort")
    });
    assert_eq!(
        stored
            .iter()
            .map(|artifact| artifact.artifact_id)
            .collect::<HashSet<_>>()
            .len(),
        2
    );
    let mut reordered = cohorts[0].clone();
    reordered.reverse();
    for trace in &mut reordered {
        trace.entities.reverse();
    }
    let duplicate = archive
        .persist(
            &cas,
            &learned[0],
            &reordered,
            &receipts[0],
            metadata(parents[0].clone(), 99),
        )
        .expect("canonical order invariant")
        .artifact_id;
    assert_eq!(duplicate, stored[0].artifact_id);
    assert!(matches!(
        archive.persist(
            &cas,
            &learned[0],
            &cohorts[0],
            &receipts[0],
            metadata(Vec::new(), 3),
        ),
        Err(ObservationPredicateCohortArchiveError::MissingParent)
    ));
    let incompatible_envelope = ArtifactEnvelope::new(
        SchemaRef::new("cairn.memory.observation-predicate-cohort", 2).expect("schema"),
        ArtifactKind::new("memory.observation-predicate-cohort").expect("kind"),
        SemanticsVersion::new(2, 0, 0),
        parents[0].clone(),
        Some(Hash32::digest(b"incompatible-scope")),
        producer(),
        budget(),
        4,
        CanonicalValue::Text("old-semantics-payload".to_owned()),
    )
    .expect("incompatible envelope");
    let incompatible = cas
        .put_envelope(&incompatible_envelope)
        .expect("persist incompatible envelope");
    assert!(archive.load(&cas, incompatible.envelope_blob_id).is_err());

    // Destroy every authority-bearing value before reopening the archive.
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
                .expect("load evidence only")
        })
        .collect::<Vec<_>>();
    assert!(ObservationPredicateStore::new()
        .persistent_mechanisms()
        .next()
        .is_none());
    assert!(matches!(
        archive.replay(
            &loaded[0],
            ObservationPredicateLimits {
                maximum_entities: 1,
                ..observation_limits
            },
            predicate_limits,
            InventionLimits::default(),
            ExecLimits::default(),
        ),
        Err(ObservationPredicateCohortArchiveError::Invention(
            ObservationPredicateError::Capacity
        ))
    ));
    let replayed = loaded
        .iter()
        .map(|artifact| {
            archive
                .replay(
                    artifact,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                    ExecLimits::default(),
                )
                .expect("re-invent and independently replay")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        replayed[0].learned.semantic_hash(),
        replayed[1].learned.semantic_hash()
    );
    let fresh_work = replayed[0].receipt.candidate_executions_without();
    let promotion = ObservationPredicatePromotionConsolidator
        .consolidate(
            &replayed[0].learned,
            &[replayed[0].receipt.clone(), replayed[1].receipt.clone()],
        )
        .expect("fresh consolidation");
    let mut store = ObservationPredicateStore::new();
    let id = store
        .insert_provisional(replayed[0].learned.clone())
        .expect("fresh provisional module");
    for replay in replayed {
        store
            .record_verified_reuse(id, replay.receipt)
            .expect("fresh receipt");
    }
    assert!(store.apply_verified_promotion(id, &promotion));
    assert_eq!(
        store.module(id).expect("module").status(),
        ModuleStatus::Persistent
    );
    let held_out = episode(
        "observation-feature-restart-heldout",
        &[2, 3, 4, 5, 6],
        -900,
        5_000,
    );
    let predicted = ObservationPredicateInventor
        .predict(
            store
                .persistent_mechanisms()
                .next()
                .expect("re-promoted proposal"),
            &held_out.entities,
            observation_limits,
            predicate_limits,
        )
        .expect("execute after restart");
    assert_eq!(predicted, held_out.outputs);
    eprintln!(
        "FEATURE_PROGRAM_RESTART artifacts={},{} authority_restored=0 cohorts_reinvented=2 repromoted=1 candidates={fresh_work}->1 examples=4->0 heldout_groups=5 exact=5",
        stored[0].artifact_id, stored[1].artifact_id
    );
}

fn source_cohorts() -> [[PrimitivePairTrace; 2]; 2] {
    [
        [
            episode("restart-feature-a1", &[2, 3, 4], 100, 1_000),
            episode("restart-feature-a2", &[2, 3, 4, 5], 300, 2_000),
        ],
        [
            episode("restart-feature-b1", &[2, 4, 5], -500, 3_000),
            episode("restart-feature-b2", &[3, 4, 5, 6], 700, 4_000),
        ],
    ]
}

fn episode(
    domain: &str,
    cardinalities: &[usize],
    sum_offset: i64,
    value_base: i64,
) -> PrimitivePairTrace {
    let mut entities = Vec::new();
    let mut outputs = BTreeSet::new();
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for (group_index, cardinality) in cardinalities.iter().copied().enumerate() {
        let sum = sum_offset + i64::try_from(group_index).expect("bounded group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("bounded cardinality"));
        for member in 0..cardinality {
            let member_i64 = i64::try_from(member).expect("bounded member");
            let first = sum_offset * 3 + member_i64 * 37 + 11;
            let second = sum_offset * -2 + member_i64 * 19 - 7;
            let values = [first, second, sum - first - second];
            let permutation = permutations[(group_index + member) % permutations.len()];
            entities.push(PrimitivePairEntity {
                id: EntityId(Hash32::digest(
                    format!("{domain}:{group_index}:{member}").as_bytes(),
                )),
                value: value_base
                    + i64::try_from(group_index).expect("bounded group") * 100
                    + member_i64,
                observations: permutation.iter().map(|index| values[*index]).collect(),
            });
        }
    }
    PrimitivePairTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities,
        outputs,
    }
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.observation-feature", 1).expect("schema"),
        ArtifactKind::new("test.observation-feature").expect("kind"),
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
        component: "observation-feature-restart-test".to_owned(),
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
                "cairn-observation-feature-restart-{}-{sequence}",
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
