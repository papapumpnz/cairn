use cairn_cas::{
    validate_parent_graph, Blake3ContentHasher, Cas, CasError, CasLimits, ContentHasher,
    ParentPolicy, PutStatus,
};
use cairn_core::{
    ArtifactEnvelope, ArtifactId, ArtifactKind, BlobId, BudgetRecord, CanonicalLimits,
    CanonicalValue, Hash32, ProducerRecord, SchemaDescriptor, SchemaRef, SchemaRegistry,
    SemanticsVersion,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("cairn-cas-{}-{sequence}", std::process::id()));
        fs::create_dir_all(&root).expect("fixture root");
        Self { root }
    }

    fn cas(&self) -> Cas {
        Cas::open(&self.root, CasLimits::default()).expect("open CAS")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn put_get_verify_is_idempotent() {
    let fixture = Fixture::new();
    let cas = fixture.cas();
    let first = cas.put(b"immutable").expect("first put");
    let second = cas.put(b"immutable").expect("second put");
    assert_eq!(first.status, PutStatus::Stored);
    assert_eq!(second.status, PutStatus::AlreadyPresent);
    assert_eq!(first.id, second.id);
    assert_eq!(cas.get(first.id).expect("get").as_slice(), b"immutable");
    cas.verify(first.id).expect("verify");
}

#[test]
fn concurrent_same_blob_converges_without_overwrite() {
    let fixture = Fixture::new();
    let cas = Arc::new(fixture.cas());
    let barrier = Arc::new(Barrier::new(16));
    let handles = (0..16)
        .map(|_| {
            let cas = Arc::clone(&cas);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                cas.put(&[0x5a; 4096]).expect("concurrent put")
            })
        })
        .collect::<Vec<_>>();
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().expect("thread"))
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.status == PutStatus::Stored)
            .count(),
        1
    );
    assert!(outcomes.iter().all(|outcome| outcome.id == outcomes[0].id));
    cas.verify(outcomes[0].id).expect("verify");
}

#[derive(Debug)]
struct ConstantHasher;

impl ContentHasher for ConstantHasher {
    fn hash(&self, _bytes: &[u8]) -> Hash32 {
        Hash32::from_bytes([7; 32])
    }
}

#[test]
fn injected_hash_collision_preserves_first_blob() {
    let fixture = Fixture::new();
    let cas = Cas::open_with_hasher(
        &fixture.root,
        CasLimits::default(),
        Arc::new(ConstantHasher),
    )
    .expect("open");
    let id = cas.put(b"first").expect("first").id;
    assert!(matches!(
        cas.put(b"second"),
        Err(CasError::HashCollision { .. })
    ));
    assert_eq!(cas.get(id).expect("get first").as_slice(), b"first");
}

#[test]
fn corrupt_blob_is_rejected() {
    let fixture = Fixture::new();
    let cas = fixture.cas();
    let id = cas.put(b"original").expect("put").id;
    fs::write(blob_path(&fixture.root, id.hash()), b"altered").expect("corrupt");
    assert!(matches!(cas.get(id), Err(CasError::HashMismatch { .. })));
}

#[test]
fn accepted_blob_slices_are_immutable_shared_snapshots() {
    let fixture = Fixture::new();
    let cas = fixture.cas();
    let id = cas.put(b"snapshot").expect("put").id;
    let first = cas.get(id).expect("get");
    let second = first.clone();
    fs::write(blob_path(&fixture.root, id.hash()), b"mutated!").expect("mutate backing file");
    assert_eq!(first.as_slice(), b"snapshot");
    assert_eq!(second.slice(1..5), Some(&b"naps"[..]));
    assert_eq!(first.as_slice().as_ptr(), second.as_slice().as_ptr());
    assert!(matches!(cas.get(id), Err(CasError::HashMismatch { .. })));
}

#[test]
fn recovery_removes_temporary_and_rebuilds_missing_registration() {
    let fixture = Fixture::new();
    let cas = fixture.cas();
    let id = cas.put(b"recoverable").expect("put").id;
    fs::remove_file(registration_path(&fixture.root, id.hash())).expect("remove registration");
    fs::write(fixture.root.join("tmp/torn-write"), b"partial").expect("write torn");
    let report = cas.recover().expect("recover");
    assert_eq!(report.temporary_files_removed, 1);
    assert_eq!(report.registrations_recovered, 1);
    cas.verify(id).expect("verify recovered");
}

#[test]
fn malformed_registration_fails_closed() {
    let fixture = Fixture::new();
    let cas = fixture.cas();
    let id = cas.put(b"registered").expect("put").id;
    fs::write(registration_path(&fixture.root, id.hash()), b"torn").expect("tear");
    assert!(matches!(cas.verify(id), Err(CasError::InvalidRegistration)));
}

#[test]
fn declared_capacity_stops_before_write() {
    let fixture = Fixture::new();
    let cas = Cas::open(
        &fixture.root,
        CasLimits {
            maximum_blob_bytes: 3,
            maximum_recovery_entries: 10,
        },
    )
    .expect("open");
    assert!(matches!(
        cas.put(b"four"),
        Err(CasError::CapacityCap {
            requested: 4,
            maximum: 3
        })
    ));
}

#[test]
fn parent_cycle_and_missing_parent_are_distinct() {
    let first = ArtifactId::from_hash(Hash32::digest(b"first"));
    let second = ArtifactId::from_hash(Hash32::digest(b"second"));
    let missing = ArtifactId::from_hash(Hash32::digest(b"missing"));
    let cycle = BTreeMap::from([(first, vec![second]), (second, vec![first])]);
    assert!(matches!(
        validate_parent_graph(first, &cycle, ParentPolicy::RequirePresent, 10),
        Err(CasError::ParentCycle { .. })
    ));
    let incomplete = BTreeMap::from([(first, vec![missing])]);
    assert!(matches!(
        validate_parent_graph(first, &incomplete, ParentPolicy::RequirePresent, 10),
        Err(CasError::ParentMissing { parent }) if parent == missing
    ));
    validate_parent_graph(first, &incomplete, ParentPolicy::AllowMissing, 10)
        .expect("missing allowed");
}

#[test]
fn production_and_reference_hashers_agree() {
    let hasher = Blake3ContentHasher;
    assert_eq!(hasher.hash(b"same"), Hash32::digest(b"same"));
}

#[test]
fn canonical_envelope_crosses_cas_boundary_and_revalidates() {
    let fixture = Fixture::new();
    let cas = fixture.cas();
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.artifact", 1).expect("schema"),
        ArtifactKind::new("test.artifact").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        Vec::new(),
        None,
        ProducerRecord {
            component: "cas-test".to_owned(),
            version: "0.1.0".to_owned(),
            commit: "abcdef0".to_owned(),
            host_run_id: "test-run".to_owned(),
            agent_id: None,
        },
        BudgetRecord {
            work_units: 1,
            cpu_nanos: 2,
            peak_rss_bytes: 3,
        },
        1,
        CanonicalValue::Text("verified".to_owned()),
    )
    .expect("envelope");
    let stored = cas.put_envelope(&envelope).expect("put envelope");
    assert_eq!(stored.artifact_id, envelope.artifact_id);
    let mut registry = SchemaRegistry::new();
    registry
        .register(SchemaDescriptor {
            name: "test.artifact".to_owned(),
            current_version: 1,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 1024,
        })
        .expect("register");
    let loaded = cas
        .get_envelope(
            stored.envelope_blob_id,
            &registry,
            &BTreeSet::from([SemanticsVersion::new(1, 0, 0)]),
            CanonicalLimits::default(),
        )
        .expect("get envelope");
    assert_eq!(loaded, envelope);
}

#[test]
fn independent_processes_converge_on_one_blob() {
    let fixture = Fixture::new();
    let worker = env!("CARGO_BIN_EXE_cairn-cas-worker");
    let children = (0..8)
        .map(|_| {
            Command::new(worker)
                .arg(&fixture.root)
                .arg("put")
                .current_dir(&fixture.root)
                .stdout(Stdio::piped())
                .spawn()
                .expect("spawn worker")
        })
        .collect::<Vec<_>>();
    let outputs = children
        .into_iter()
        .map(|child| child.wait_with_output().expect("worker output"))
        .collect::<Vec<_>>();
    assert!(outputs.iter().all(|output| output.status.success()));
    let lines = outputs
        .iter()
        .map(|output| String::from_utf8(output.stdout.clone()).expect("UTF-8 output"))
        .collect::<Vec<_>>();
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("STORED "))
            .count(),
        1
    );
    assert_eq!(
        lines
            .iter()
            .map(|line| line.split_whitespace().nth(1).expect("worker ID"))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        1
    );
}

#[test]
fn abrupt_process_loss_after_blob_publish_recovers_exactly() {
    let fixture = Fixture::new();
    let worker = env!("CARGO_BIN_EXE_cairn-cas-worker");
    let status = Command::new(worker)
        .arg(&fixture.root)
        .arg("crash-after-blob")
        .current_dir(&fixture.root)
        .status()
        .expect("run crash worker");
    assert!(!status.success());

    let cas = fixture.cas();
    let report = cas.recover().expect("recover after worker loss");
    assert_eq!(report.temporary_files_removed, 1);
    assert_eq!(report.registrations_recovered, 1);
    let id = BlobId::from_hash(Hash32::digest(&[0x42; 4096]));
    cas.verify(id).expect("recovered blob verifies");
}

fn blob_path(root: &Path, hash: Hash32) -> PathBuf {
    fanout(root, "blobs", hash)
}

fn registration_path(root: &Path, hash: Hash32) -> PathBuf {
    fanout(root, "journal", hash)
}

fn fanout(root: &Path, kind: &str, hash: Hash32) -> PathBuf {
    let hex = hash.to_hex();
    root.join(kind).join(&hex[..2]).join(&hex[2..])
}
