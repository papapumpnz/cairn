use cairn_cas::{Cas, CasLimits};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalValue, Hash32, ProducerRecord,
    SchemaRef, SemanticsVersion,
};
use cairn_memory::{
    ModuleStatus, ReuseArtifactMetadata, ScalarExpressionCohortArchive,
    ScalarExpressionCohortArchiveError, ScalarExpressionStore,
};
use cairn_theory::{
    RelationExpressionLimits, RelationExpressionShapePolicy, RelationKeyOperator,
    ScalarExpressionError, ScalarExpressionExample, ScalarExpressionInventor,
    ScalarExpressionTrace,
};
use cairn_verifier::{ScalarExpressionPromotionConsolidator, ScalarExpressionVerifier};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
#[allow(clippy::too_many_lines)]
fn scalar_expression_memory_is_reinvented_after_process_loss() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let limits = RelationExpressionLimits {
        permitted_binary_operators: BTreeSet::from([
            RelationKeyOperator::Subtract,
            RelationKeyOperator::Minimum,
        ]),
        parameter_arity: 3,
        maximum_binary_nodes: 2,
        maximum_candidates: 4_096,
        maximum_relation_evaluations: 200_000_000,
        maximum_absolute_attribute: 1_000_000_000,
        shape_policy: RelationExpressionShapePolicy::FullBinaryTrees,
    };
    let cohorts = [
        [
            trace("scalar-restart-sequence-a", [0, 1, 2, 3], 1, 0),
            trace("scalar-restart-sequence-b", [3, 0, 2, 1], 2, 100),
        ],
        [
            trace("scalar-restart-vector-a", [2, 1, 3, 0], 3, -200),
            trace("scalar-restart-vector-b", [1, 2, 0, 3], 4, 300),
        ],
    ];
    let learned = cohorts
        .iter()
        .map(|cohort| {
            ScalarExpressionInventor
                .invent(cohort, &limits)
                .expect("initial scalar invention")
        })
        .collect::<Vec<_>>();
    let receipts = learned
        .iter()
        .zip(&cohorts)
        .map(|(mechanism, cohort)| {
            ScalarExpressionVerifier
                .verify_reuse(mechanism, cohort, &limits)
                .expect("initial scalar receipt")
        })
        .collect::<Vec<_>>();
    let archive = ScalarExpressionCohortArchive;
    let stored = [0_usize, 1_usize].map(|index| {
        archive
            .persist(
                &cas,
                &learned[index],
                &cohorts[index],
                &receipts[index],
                metadata(
                    vec![parent(&cas, &format!("scalar-expression-parent-{index}"))],
                    u64::try_from(index + 1).expect("index"),
                ),
            )
            .expect("persist scalar evidence")
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
        Err(ScalarExpressionCohortArchiveError::MissingParent)
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
    assert!(ScalarExpressionStore::new()
        .persistent_modules()
        .next()
        .is_none());
    let capped = RelationExpressionLimits {
        maximum_candidates: 1,
        ..limits.clone()
    };
    assert!(matches!(
        archive.replay(&loaded[0], &capped),
        Err(ScalarExpressionCohortArchiveError::Invention(
            ScalarExpressionError::Capacity
        ))
    ));
    let replayed = loaded
        .iter()
        .map(|artifact| {
            archive
                .replay(artifact, &limits)
                .expect("fresh scalar invention and replay")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        replayed[0].learned.mechanism_hash(),
        replayed[1].learned.mechanism_hash()
    );
    let promotion = ScalarExpressionPromotionConsolidator
        .consolidate(&[replayed[0].receipt.clone(), replayed[1].receipt.clone()])
        .expect("fresh scalar consolidation");
    assert_eq!(promotion.expressions_saved(), 14);
    let mut store = ScalarExpressionStore::new();
    let id = store
        .insert_provisional(replayed[0].learned.source_independent())
        .expect("fresh provisional scalar expression");
    for replay in replayed {
        store
            .record_verified_reuse(id, replay.receipt)
            .expect("fresh scalar receipt");
    }
    assert!(store.apply_verified_promotion(id, &promotion));
    assert_eq!(
        store.module(id).expect("module").status(),
        ModuleStatus::Persistent
    );

    let future = trace("scalar-restart-future", [1, 3, 0, 2], 5, -700);
    let exact = future
        .examples
        .iter()
        .filter(|example| {
            ScalarExpressionInventor
                .propose(
                    store.module(id).expect("module").mechanism(),
                    &example.inputs,
                    &limits,
                )
                .expect("future scalar proposals")
                .iter()
                .any(|(_, output)| *output == example.output)
        })
        .count();
    assert_eq!(exact, future.examples.len());
    eprintln!(
        "SCALAR_EXPRESSION_RESTART artifacts={},{} authority_restored=0 cohorts_reinvented=2 repromoted=1 expressions=8->1 examples=4->0 future_exact={exact}",
        stored[0].artifact_id, stored[1].artifact_id
    );
}

fn trace(domain: &str, permutation: [usize; 4], scale: i64, offset: i64) -> ScalarExpressionTrace {
    let logical_inputs = [
        [20_i64, 3_i64, 9_i64],
        [20, 12, 4],
        [-5, -10, 2],
        [7, 11, -3],
        [100, 40, 60],
        [-20, -5, -30],
        [0, 8, 2],
        [31, 31, 17],
    ];
    ScalarExpressionTrace {
        experience: Hash32::digest(domain.as_bytes()),
        examples: logical_inputs
            .into_iter()
            .enumerate()
            .map(|(index, logical)| {
                let scaled = logical.map(|value| value * scale + offset);
                let logical_channels = [
                    scaled[0],
                    scaled[1],
                    scaled[2],
                    10_000 * scale + offset + i64::try_from(index).expect("bounded index"),
                ];
                let mut inputs = vec![0_i64; logical_channels.len()];
                for (source, destination) in permutation.into_iter().enumerate() {
                    inputs[destination] = logical_channels[source];
                }
                ScalarExpressionExample {
                    inputs,
                    output: scaled[0] - scaled[1].min(scaled[2]),
                }
            })
            .collect(),
    }
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.scalar-expression", 1).expect("schema"),
        ArtifactKind::new("test.scalar-expression").expect("kind"),
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
        component: "scalar-expression-restart-test".to_owned(),
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
                "cairn-scalar-expression-restart-{}-{sequence}",
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
