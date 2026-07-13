use cairn_cas::{Cas, CasLimits};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalValue, EntityId, Hash32, ProducerRecord,
    SchemaRef, SemanticsVersion,
};
use cairn_invent::InventionLimits;
use cairn_memory::{
    ModuleStatus, PredicateGroupingCohortArchive, PredicateGroupingCohortArchiveError,
    PredicateGroupingStore, ReuseArtifactMetadata,
};
use cairn_rkl::ExecLimits;
use cairn_theory::{
    AttributedGroupingEntity, AttributedGroupingTrace, PredicateGroupingError,
    PredicateGroupingInventor, PredicateGroupingLimits,
};
use cairn_verifier::{PredicateGroupingPromotionConsolidator, PredicateGroupingVerifier};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
#[allow(clippy::too_many_lines)] // Complete evidence/loss/replay/repromotion boundary.
fn invented_predicate_memory_is_reconstructed_after_process_loss() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let limits = PredicateGroupingLimits::default();
    let first = [
        episode("restart-predicate-vector", &[(0, 0), (0, 1), (1, 0)], 1_000),
        episode(
            "restart-predicate-sequence",
            &[(2, 2), (2, 3), (3, 2), (3, 3)],
            2_000,
        ),
    ];
    let second = [
        episode(
            "restart-predicate-table",
            &[(10, 20), (10, 21), (11, 20)],
            3_000,
        ),
        episode(
            "restart-predicate-workflow",
            &[(30, 40), (30, 41), (31, 40), (31, 41)],
            4_000,
        ),
    ];
    let learned = [&first, &second].map(|cohort| {
        PredicateGroupingInventor
            .invent(cohort, limits, InventionLimits::default())
            .expect("initial invention")
    });
    assert_eq!(learned[0].semantic_hash(), learned[1].semantic_hash());
    let blank_candidates = learned[0]
        .merge
        .constructor
        .proposal
        .candidates_evaluated
        .saturating_add(
            u64::try_from(learned[0].relation_candidates_evaluated).expect("bounded candidates"),
        );
    let receipts = [(&learned[0], &first), (&learned[1], &second)].map(|(mechanism, cohort)| {
        PredicateGroupingVerifier
            .verify_reuse(
                mechanism,
                cohort,
                limits,
                ExecLimits::default(),
                blank_candidates,
                1,
            )
            .expect("initial receipt")
    });
    let parents = [
        vec![parent(&cas, "predicate-first")],
        vec![parent(&cas, "predicate-second")],
    ];
    let archive = PredicateGroupingCohortArchive;
    let stored = [
        archive
            .persist(
                &cas,
                &learned[0],
                &first,
                &receipts[0],
                metadata(parents[0].clone(), 1),
            )
            .expect("persist first"),
        archive
            .persist(
                &cas,
                &learned[1],
                &second,
                &receipts[1],
                metadata(parents[1].clone(), 2),
            )
            .expect("persist second"),
    ];
    assert_eq!(
        stored
            .iter()
            .map(|artifact| artifact.artifact_id)
            .collect::<HashSet<_>>()
            .len(),
        2
    );
    let mut reordered = first.clone();
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
        .expect("entity-order canonicalization");
    assert_eq!(duplicate.artifact_id, stored[0].artifact_id);
    assert!(matches!(
        archive.persist(
            &cas,
            &learned[0],
            &first,
            &receipts[0],
            metadata(Vec::new(), 3),
        ),
        Err(PredicateGroupingCohortArchiveError::MissingParent)
    ));

    drop(cas);
    drop(first);
    drop(second);
    drop(learned);
    drop(receipts);

    let reopened = Cas::open(&fixture.path, CasLimits::default()).expect("reopen CAS");
    reopened.recover().expect("recover CAS");
    let loaded = stored
        .iter()
        .map(|artifact| {
            archive
                .load(&reopened, artifact.envelope_blob_id)
                .expect("load untrusted evidence")
        })
        .collect::<Vec<_>>();
    assert!(PredicateGroupingStore::new()
        .persistent_mechanisms()
        .next()
        .is_none());
    assert!(matches!(
        archive.replay(
            &loaded[0],
            PredicateGroupingLimits {
                maximum_entities: 1,
                ..limits
            },
            InventionLimits::default(),
            ExecLimits::default(),
        ),
        Err(PredicateGroupingCohortArchiveError::Invention(
            PredicateGroupingError::Capacity
        ))
    ));
    let replayed = loaded
        .iter()
        .map(|artifact| {
            archive
                .replay(
                    artifact,
                    limits,
                    InventionLimits::default(),
                    ExecLimits::default(),
                )
                .expect("re-invent and replay")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        replayed[0].learned.semantic_hash(),
        replayed[1].learned.semantic_hash()
    );
    let promotion = PredicateGroupingPromotionConsolidator
        .consolidate(
            &replayed[0].learned,
            &[replayed[0].receipt.clone(), replayed[1].receipt.clone()],
        )
        .expect("fresh consolidation");
    let mut store = PredicateGroupingStore::new();
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
        "restart-predicate-held-out",
        &[(50, 60), (50, 61), (51, 60), (51, 61), (52, 60)],
        5_000,
    );
    let outcome = PredicateGroupingInventor
        .predict(
            store
                .persistent_mechanisms()
                .next()
                .expect("restored proposal"),
            &held_out.entities,
            limits,
        )
        .expect("post-restart execution");
    assert_eq!(outcome, held_out.outputs);
    eprintln!(
        "PREDICATE_RESTART artifacts={},{} status_restored=0 cohorts_reinvented=2 repromoted=1 candidates={blank_candidates}->1 examples=4->0 heldout_groups=5 exact=5",
        stored[0].artifact_id, stored[1].artifact_id
    );
}

fn episode(domain: &str, keys: &[(i64, i64)], value_base: i64) -> AttributedGroupingTrace {
    let mut entities = Vec::new();
    let mut outputs = BTreeSet::new();
    for (group_index, (first, second)) in keys.iter().copied().enumerate() {
        let cardinality = group_index + 2;
        outputs.insert(i64::try_from(cardinality - 1).expect("bounded cardinality"));
        for member in 0..cardinality {
            entities.push(AttributedGroupingEntity {
                id: EntityId(Hash32::digest(
                    format!("{domain}:{group_index}:{member}").as_bytes(),
                )),
                value: value_base
                    + i64::try_from(group_index).expect("bounded group") * 100
                    + i64::try_from(member).expect("bounded member"),
                features: vec![
                    first,
                    second,
                    i64::try_from(member).expect("bounded member"),
                ],
            });
        }
    }
    AttributedGroupingTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities,
        outputs,
    }
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.predicate-observation", 1).expect("schema"),
        ArtifactKind::new("test.predicate-observation").expect("kind"),
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
        component: "predicate-grouping-restart-test".to_owned(),
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
                "cairn-predicate-grouping-restart-{}-{sequence}",
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
