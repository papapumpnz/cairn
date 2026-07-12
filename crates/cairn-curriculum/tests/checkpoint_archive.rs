use cairn_cas::{Cas, CasLimits, PutStatus};
use cairn_core::{
    decode_canonical, ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalEncode,
    CanonicalLimits, CanonicalValue, Hash32, ProducerRecord, SchemaRef, SemanticsVersion,
};
use cairn_curriculum::{
    AutonomousCurriculum, AutonomousCurriculumError, CurriculumCheckpointArchive,
    CurriculumCheckpointArchiveError, CurriculumCheckpointMetadata, DevelopmentalObjective,
    DevelopmentalOutcome, ExperienceId, LearningAxis, MechanismTarget,
    CURRICULUM_CHECKPOINT_SCHEMA, CURRICULUM_CHECKPOINT_SCHEMA_VERSION,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn canonical_checkpoint_is_idempotent_and_restart_safe() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let parent = parent(&cas, "verified-learning-events");
    let mut curriculum = AutonomousCurriculum::new(objectives(), 4).expect("curriculum");
    let decision = curriculum.select_next().expect("decision");
    curriculum.record(success(&decision)).expect("record");
    let checkpoint = curriculum.checkpoint();
    let archive = CurriculumCheckpointArchive;
    let first = archive
        .persist(&cas, &checkpoint, metadata(parent, 1))
        .expect("persist checkpoint");
    let duplicate = archive
        .persist(&cas, &checkpoint, metadata(parent, 1))
        .expect("idempotent checkpoint");
    let changed_metadata = archive
        .persist(&cas, &checkpoint, metadata(parent, 2))
        .expect("metadata-independent identity");
    assert_eq!(duplicate.status, PutStatus::AlreadyPresent);
    assert_eq!(first.artifact_id, duplicate.artifact_id);
    assert_eq!(first.artifact_id, changed_metadata.artifact_id);
    drop(cas);

    let reopened = Cas::open(&fixture.path, CasLimits::default()).expect("reopen CAS");
    reopened.recover().expect("recover CAS");
    let mut restored = archive
        .load(&reopened, first.envelope_blob_id)
        .expect("load and replay");
    assert_eq!(restored.history(), curriculum.history());
    assert_eq!(
        restored.select_next().expect("next choice").objective,
        ExperienceId("base-b".to_owned())
    );
}

#[test]
fn checkpoint_archive_rejects_missing_lineage_malformed_and_divergent_state() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let archive = CurriculumCheckpointArchive;
    let mut curriculum = AutonomousCurriculum::new(objectives(), 4).expect("curriculum");
    let decision = curriculum.select_next().expect("decision");
    curriculum.record(success(&decision)).expect("record");
    let checkpoint = curriculum.checkpoint();
    assert!(matches!(
        archive.persist(
            &cas,
            &checkpoint,
            CurriculumCheckpointMetadata {
                parent_ids: Vec::new(),
                ..metadata(parent(&cas, "unused"), 1)
            }
        ),
        Err(CurriculumCheckpointArchiveError::MissingParent)
    ));

    let malformed = put_payload(
        &cas,
        CanonicalValue::Array(vec![CanonicalValue::Text("wrong-marker".to_owned())]),
        "malformed",
    );
    assert!(matches!(
        archive.load(&cas, malformed),
        Err(CurriculumCheckpointArchiveError::InvalidPayload(_))
    ));

    let bytes = checkpoint.encode_canonical().expect("canonical checkpoint");
    let mut divergent = decode_canonical(&bytes, CanonicalLimits::default()).expect("decode");
    let CanonicalValue::Array(root) = &mut divergent else {
        panic!("checkpoint root");
    };
    let CanonicalValue::Array(events) = &mut root[4] else {
        panic!("events");
    };
    let CanonicalValue::Array(event) = &mut events[0] else {
        panic!("event");
    };
    let CanonicalValue::Array(recorded_decision) = &mut event[1] else {
        panic!("decision");
    };
    recorded_decision[1] = CanonicalValue::Text("forged-objective".to_owned());
    let divergent = put_payload(&cas, divergent, "divergent");
    assert!(matches!(
        archive.load(&cas, divergent),
        Err(CurriculumCheckpointArchiveError::Replay(
            AutonomousCurriculumError::CheckpointReplayMismatch
        ))
    ));

    let valid_payload = decode_canonical(&bytes, CanonicalLimits::default()).expect("decode");
    let unsupported = put_payload_with_contract(
        &cas,
        valid_payload.clone(),
        "unsupported",
        SemanticsVersion::new(1, 0, 0),
        None,
    );
    assert!(matches!(
        archive.load(&cas, unsupported),
        Err(CurriculumCheckpointArchiveError::Storage(_))
    ));
    let scoped = put_payload_with_contract(
        &cas,
        valid_payload,
        "scoped",
        SemanticsVersion::new(1, 1, 0),
        Some(Hash32::digest(b"forbidden-task-scope")),
    );
    assert!(matches!(
        archive.load(&cas, scoped),
        Err(CurriculumCheckpointArchiveError::InvalidEnvelope)
    ));
}

fn objectives() -> Vec<DevelopmentalObjective> {
    [
        ("base-a", "mechanism-a", LearningAxis::CausalLaw, 10),
        ("base-b", "mechanism-b", LearningAxis::Cardinality, 8),
    ]
    .into_iter()
    .map(
        |(id, target, axis, competing_hypotheses)| DevelopmentalObjective {
            id: ExperienceId(id.to_owned()),
            axis,
            produces: MechanismTarget(target.to_owned()),
            prerequisites: BTreeSet::new(),
            competing_hypotheses,
            expected_transfer_microunits: 10,
            action_cost: 1,
            irreversible_risk: 0,
        },
    )
    .collect()
}

fn success(decision: &cairn_curriculum::CurriculumDecision) -> DevelopmentalOutcome {
    DevelopmentalOutcome {
        decision_sequence: decision.sequence,
        objective: decision.objective.clone(),
        mechanism_evidence: "verified-mechanism".to_owned(),
        independently_verified: true,
        held_out_transfer_cases: 2,
        candidate_work_without: 10,
        candidate_work_with: 2,
        examples_without: 4,
        examples_with: 1,
        retained_prior_tasks: 0,
        prior_tasks: 0,
        negative_transfers: 0,
    }
}

fn put_payload(cas: &Cas, payload: CanonicalValue, name: &str) -> cairn_core::BlobId {
    put_payload_with_contract(cas, payload, name, SemanticsVersion::new(1, 1, 0), None)
}

fn put_payload_with_contract(
    cas: &Cas,
    payload: CanonicalValue,
    name: &str,
    semantics: SemanticsVersion,
    task_scope: Option<Hash32>,
) -> cairn_core::BlobId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new(
            CURRICULUM_CHECKPOINT_SCHEMA,
            CURRICULUM_CHECKPOINT_SCHEMA_VERSION,
        )
        .expect("schema"),
        ArtifactKind::new("curriculum.checkpoint").expect("kind"),
        semantics,
        vec![parent(cas, name)],
        task_scope,
        producer(),
        budget(),
        2,
        payload,
    )
    .expect("envelope");
    cas.put_envelope(&envelope)
        .expect("persist payload")
        .envelope_blob_id
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.curriculum-evidence", 1).expect("schema"),
        ArtifactKind::new("test.curriculum-evidence").expect("kind"),
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

fn metadata(parent_id: cairn_core::ArtifactId, logical: u64) -> CurriculumCheckpointMetadata {
    CurriculumCheckpointMetadata {
        parent_ids: vec![parent_id],
        producer: producer(),
        budget: budget(),
        created_at_logical: logical,
    }
}

fn producer() -> ProducerRecord {
    ProducerRecord {
        component: "curriculum-checkpoint-test".to_owned(),
        version: "0.1.0".to_owned(),
        commit: "test".to_owned(),
        host_run_id: "restart".to_owned(),
        agent_id: Some("curriculum".to_owned()),
    }
}

const fn budget() -> BudgetRecord {
    BudgetRecord {
        work_units: 8,
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
                "cairn-curriculum-checkpoint-{}-{sequence}",
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
