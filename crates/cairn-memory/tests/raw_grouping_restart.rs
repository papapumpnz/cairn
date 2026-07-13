use cairn_cas::{Cas, CasLimits, PutStatus};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalValue, EntityId, Hash32, ProducerRecord,
    RelationRecord, RelationRole, SchemaRef, SemanticsVersion,
};
use cairn_invent::InventionLimits;
use cairn_memory::{
    GroupingCohortArchiveError, GroupingMergeStore, ModuleStatus, RawGroupingCohortArchive,
    ReuseArtifactMetadata, GROUPING_COHORT_SCHEMA, GROUPING_COHORT_SCHEMA_VERSION,
};
use cairn_rkl::ExecLimits;
use cairn_theory::{
    RawGroupingError, RawGroupingEvidence, RawGroupingLimits, RawGroupingTrace,
    RawRelationGroupingInventor,
};
use cairn_verifier::{GroupingMergeVerifier, GroupingPromotionConsolidator};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
#[allow(clippy::too_many_lines)] // One complete persist/loss/replay/repromotion experiment.
fn raw_grouping_memory_is_reconstructed_from_canonical_evidence_after_restart() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let limits = RawGroupingLimits::default();
    let first = [
        episode("restart-vector", &[1, 4, 10]),
        episode("restart-sequence", &[2, 5, 11, 20]),
    ];
    let second = [
        episode("restart-table", &[3, 8, 16, 27, 41]),
        episode("restart-workflow", &[4, 9, 18, 31, 47, 66]),
    ];
    let learned = [&first, &second].map(|cohort| {
        RawRelationGroupingInventor
            .invent(cohort, limits, InventionLimits::default())
            .expect("initial invention")
    });
    assert_eq!(
        learned[0].semantic_hash().expect("first identity"),
        learned[1].semantic_hash().expect("second identity")
    );
    let blank_candidates = learned[0]
        .merge
        .constructor
        .proposal
        .candidates_evaluated
        .saturating_add(u64::try_from(learned[0].relations_evaluated).expect("roles"));
    let receipts = [(&learned[0], &first), (&learned[1], &second)].map(|(mechanism, cohort)| {
        GroupingMergeVerifier
            .verify_raw_reuse(
                mechanism,
                cohort,
                blank_candidates,
                1,
                limits,
                ExecLimits::default(),
            )
            .expect("initial verification")
    });
    let parents = [
        vec![parent(&cas, "first-a"), parent(&cas, "first-b")],
        vec![parent(&cas, "second-a"), parent(&cas, "second-b")],
    ];
    let archive = RawGroupingCohortArchive;
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
        trace.evidence.relations.reverse();
    }
    let mut alternate_metadata = metadata(parents[0].clone(), 99_000);
    alternate_metadata.producer.host_run_id = "different-process".to_owned();
    alternate_metadata.budget.cpu_nanos = 123_456;
    let duplicate = archive
        .persist(
            &cas,
            &learned[0],
            &reordered,
            &receipts[0],
            alternate_metadata,
        )
        .expect("canonical idempotence");
    assert_eq!(duplicate.artifact_id, stored[0].artifact_id);
    let exact_duplicate = archive
        .persist(
            &cas,
            &learned[0],
            &first,
            &receipts[0],
            metadata(parents[0].clone(), 1),
        )
        .expect("physical idempotence");
    assert_eq!(exact_duplicate.status, PutStatus::AlreadyPresent);
    assert!(matches!(
        archive.persist(
            &cas,
            &learned[0],
            &first,
            &receipts[0],
            metadata(Vec::new(), 3),
        ),
        Err(GroupingCohortArchiveError::MissingParent)
    ));

    let wrong_envelope = ArtifactEnvelope::new(
        SchemaRef::new(GROUPING_COHORT_SCHEMA, GROUPING_COHORT_SCHEMA_VERSION).expect("schema"),
        ArtifactKind::new("memory.wrong-kind").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        parents[0].clone(),
        None,
        producer(),
        budget(),
        4,
        CanonicalValue::Text("wrong".to_owned()),
    )
    .expect("wrong envelope");
    let wrong_blob = cas
        .put_envelope(&wrong_envelope)
        .expect("persist wrong envelope")
        .envelope_blob_id;

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
                .expect("load cohort")
        })
        .collect::<Vec<_>>();
    let blank_store = GroupingMergeStore::new();
    assert!(blank_store.persistent_mechanisms().next().is_none());
    assert!(matches!(
        archive.load(&reopened, wrong_blob),
        Err(GroupingCohortArchiveError::InvalidEnvelope)
    ));
    assert!(matches!(
        archive.replay(
            &loaded[0],
            RawGroupingLimits {
                maximum_entities: 1,
                ..limits
            },
            InventionLimits::default(),
            ExecLimits::default(),
        ),
        Err(GroupingCohortArchiveError::Invention(
            RawGroupingError::Capacity
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
        replayed[0].learned.semantic_hash().expect("first replay"),
        replayed[1].learned.semantic_hash().expect("second replay")
    );
    let certificate = GroupingPromotionConsolidator
        .consolidate(
            &replayed[0].learned,
            &[replayed[0].receipt.clone(), replayed[1].receipt.clone()],
        )
        .expect("restart consolidation");
    let mut store = GroupingMergeStore::new();
    let id = store
        .insert_provisional(replayed[0].learned.clone())
        .expect("restart provisional");
    for replay in &replayed {
        store
            .record_verified_reuse(id, replay.receipt.clone())
            .expect("restart receipt");
    }
    assert!(store.apply_verified_promotion(id, &certificate));
    assert_eq!(
        store.module(id).expect("module").status(),
        ModuleStatus::Persistent
    );

    let held_out = episode("restart-held-out", &[5, 11, 22, 36, 53, 73, 96]);
    let restored = store
        .persistent_mechanisms()
        .next()
        .expect("persistent proposal");
    let outcome = RawRelationGroupingInventor
        .predict(restored, &held_out.evidence, limits)
        .expect("post-restart proposal");
    assert_eq!(outcome, held_out.outputs);
    eprintln!(
        "RAW_GROUPING_RESTART artifacts={},{} status_restored=0 cohorts_reinvented=2 repromoted=1 candidates={blank_candidates}->1 examples=4->0 heldout_groups=6 exact=6",
        stored[0].artifact_id, stored[1].artifact_id
    );
}

fn episode(domain: &str, levels: &[i64]) -> RawGroupingTrace {
    let group_count = levels.len() - 1;
    let mut groups = Vec::new();
    for group_index in 0..group_count {
        let mut values = vec![levels[0], levels[1]];
        for level in levels.iter().take(group_index + 2).skip(2) {
            values.push(*level);
        }
        if group_index > 0 {
            values.extend(std::iter::repeat_n(levels[group_index + 1], group_index));
        }
        groups.push(values);
    }
    let mut entities = BTreeMap::new();
    let mut correct = Vec::new();
    let mut distractor = vec![Vec::new(); group_count];
    let mut occurrences = BTreeMap::<i64, usize>::new();
    for (group_index, values) in groups.iter().enumerate() {
        let mut ids = Vec::new();
        for (value_index, value) in values.iter().enumerate() {
            let id = EntityId(Hash32::digest(
                format!("{domain}:{group_index}:{value_index}").as_bytes(),
            ));
            entities.insert(id.clone(), *value);
            ids.push(id.clone());
            let occurrence = occurrences.entry(*value).or_default();
            distractor[*occurrence].push(id);
            *occurrence += 1;
        }
        correct.push(ids);
    }
    let mut relations = Vec::new();
    add_chains(&mut relations, "structural-membership", &correct);
    add_chains(&mut relations, "appearance-cluster", &distractor);
    RawGroupingTrace {
        episode: Hash32::digest(domain.as_bytes()),
        evidence: RawGroupingEvidence {
            entities,
            relations,
        },
        outputs: levels
            .iter()
            .skip(1)
            .map(|maximum| maximum - levels[0])
            .collect(),
    }
}

fn add_chains(relations: &mut Vec<RelationRecord>, role: &str, groups: &[Vec<EntityId>]) {
    for group in groups {
        for edge in group.windows(2) {
            relations.push(RelationRecord {
                role: RelationRole(role.to_owned()),
                subject: edge[0].clone(),
                object: edge[1].clone(),
            });
        }
    }
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.raw-observation", 1).expect("schema"),
        ArtifactKind::new("test.raw-observation").expect("kind"),
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
        component: "raw-grouping-restart-test".to_owned(),
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
                "cairn-raw-grouping-restart-{}-{sequence}",
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
