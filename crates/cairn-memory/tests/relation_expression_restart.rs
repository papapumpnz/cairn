use cairn_cas::{Cas, CasLimits};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalValue, EntityId, Hash32, ProducerRecord,
    SchemaRef, SemanticsVersion,
};
use cairn_invent::InventionLimits;
use cairn_memory::{
    ModuleStatus, RelationExpressionCohortArchive, RelationExpressionCohortArchiveError,
    RelationExpressionStore, ReuseArtifactMetadata,
};
use cairn_rkl::ExecLimits;
use cairn_theory::{
    EntityObservationLimits, ObservationPredicateLimits, PredicateGroupingLimits,
    RawAttributedObservationPrimitive, RawRelationObservationTrace, RelationExpressionError,
    RelationExpressionInventor, RelationExpressionLimits, RelationExpressionShapePolicy,
    RelationKeyOperator,
};
use cairn_verifier::{RelationExpressionPromotionConsolidator, RelationExpressionVerifier};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
#[allow(clippy::too_many_lines)]
fn expression_memory_is_reinvented_and_repromoted_after_process_loss() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let expression_limits = RelationExpressionLimits {
        permitted_binary_operators: BTreeSet::from([
            RelationKeyOperator::Add,
            RelationKeyOperator::Minimum,
        ]),
        parameter_arity: 3,
        maximum_binary_nodes: 2,
        maximum_candidates: 1_024,
        maximum_relation_evaluations: 100_000_000,
        maximum_absolute_attribute: 1_000_000_000,
        shape_policy: RelationExpressionShapePolicy::LeftAssociated,
    };
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = [
        [
            episode(
                "expression-restart-sequence-a",
                &[2, 3, 4],
                100,
                [0, 1, 2, 3],
            ),
            episode(
                "expression-restart-table-a",
                &[2, 3, 4, 5],
                300,
                [3, 0, 2, 1],
            ),
        ],
        [
            episode(
                "expression-restart-vector-b",
                &[3, 4, 5],
                -700,
                [2, 1, 3, 0],
            ),
            episode(
                "expression-restart-world-b",
                &[2, 3, 5, 6],
                900,
                [1, 2, 0, 3],
            ),
        ],
    ];
    let learned = cohorts
        .iter()
        .map(|cohort| {
            RelationExpressionInventor
                .invent(
                    cohort,
                    &expression_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("initial expression invention")
        })
        .collect::<Vec<_>>();
    let receipts = learned
        .iter()
        .zip(&cohorts)
        .map(|(mechanism, cohort)| {
            RelationExpressionVerifier
                .verify_reuse(
                    mechanism,
                    cohort,
                    &expression_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("initial expression receipt")
        })
        .collect::<Vec<_>>();
    let archive = RelationExpressionCohortArchive;
    let stored = [0_usize, 1_usize].map(|index| {
        archive
            .persist(
                &cas,
                &learned[index],
                &cohorts[index],
                &receipts[index],
                metadata(
                    vec![parent(&cas, &format!("relation-expression-parent-{index}"))],
                    u64::try_from(index + 1).expect("index"),
                ),
            )
            .expect("persist expression evidence")
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
        Err(RelationExpressionCohortArchiveError::MissingParent)
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
    assert!(RelationExpressionStore::new()
        .persistent_modules()
        .next()
        .is_none());
    let capped = RelationExpressionLimits {
        maximum_relation_evaluations: 1,
        ..expression_limits.clone()
    };
    assert!(matches!(
        archive.replay(
            &loaded[0],
            &capped,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
            ExecLimits::default(),
        ),
        Err(RelationExpressionCohortArchiveError::Invention(
            RelationExpressionError::Capacity
        ))
    ));
    let replayed = loaded
        .iter()
        .map(|artifact| {
            archive
                .replay(
                    artifact,
                    &expression_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                    ExecLimits::default(),
                )
                .expect("fresh expression invention and replay")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        replayed[0].learned.mechanism_hash(),
        replayed[1].learned.mechanism_hash()
    );
    let promotion = RelationExpressionPromotionConsolidator
        .consolidate(&[replayed[0].receipt.clone(), replayed[1].receipt.clone()])
        .expect("fresh expression consolidation");
    assert_eq!(promotion.expressions_saved(), 6);
    let mut store = RelationExpressionStore::new();
    let id = store
        .insert_provisional(replayed[0].learned.source_independent())
        .expect("fresh provisional expression");
    for replay in replayed {
        store
            .record_verified_reuse(id, replay.receipt)
            .expect("fresh expression receipt");
    }
    assert!(store.apply_verified_promotion(id, &promotion));
    assert_eq!(
        store.module(id).expect("module").status(),
        ModuleStatus::Persistent
    );

    let future = episode(
        "expression-restart-future",
        &[2, 3, 4, 5, 6],
        -1_100,
        [2, 0, 3, 1],
    );
    let expected = future.outputs.clone();
    let mut answer_hidden = future;
    answer_hidden.outputs.clear();
    let proposals = RelationExpressionInventor
        .propose(
            store.module(id).expect("module").mechanism(),
            &answer_hidden,
            &expression_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
        )
        .expect("rebound proposal after restart");
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].output, expected);
    eprintln!(
        "RELATION_EXPRESSION_RESTART artifacts={},{} authority_restored=0 cohorts_reinvented=2 repromoted=1 expressions=4->1 examples=4->0 future_proposals=1 exact=1",
        stored[0].artifact_id, stored[1].artifact_id
    );
}

fn episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 4],
) -> RawRelationObservationTrace {
    let mut primitives = BTreeMap::new();
    let mut outputs = BTreeSet::new();
    let mut component = 0_usize;
    for (group, cardinality) in cardinalities.iter().copied().enumerate() {
        let signature = signature_offset + i64::try_from(group).expect("group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("cardinality"));
        for _ in 0..cardinality {
            let first = signature_offset * 3 + i64::try_from(component).expect("component") * 37;
            let second = signature_offset * -2 + i64::try_from(component).expect("component") * 19;
            let key = signature_offset * 20_000
                + 1_000_000
                + i64::try_from(component).expect("component") * 100_000;
            for (member, observation) in [first, second, signature - first - second]
                .into_iter()
                .enumerate()
            {
                let coordinate = i64::try_from(component * 17 + member + 3).expect("coordinate");
                let step = i64::try_from(cardinality + component + 1).expect("step") * 100;
                let (left, right, control) = match member {
                    0 => (coordinate, key - coordinate, key + step),
                    1 => (coordinate, key + step * 2 - coordinate, key),
                    _ => (coordinate, key - coordinate, key + step * 3),
                };
                let logical = [
                    left,
                    right,
                    control,
                    key + step * 20 + i64::try_from(member).expect("member"),
                ];
                let mut attributes = vec![0_i64; 4];
                for (source, destination) in permutation.into_iter().enumerate() {
                    attributes[destination] = logical[source];
                }
                primitives.insert(
                    EntityId(Hash32::digest(
                        format!("{domain}:{component}:{member}").as_bytes(),
                    )),
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
        SchemaRef::new("test.relation-expression", 1).expect("schema"),
        ArtifactKind::new("test.relation-expression").expect("kind"),
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
        component: "relation-expression-restart-test".to_owned(),
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
                "cairn-relation-expression-restart-{}-{sequence}",
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
