use cairn_cas::{
    validate_parent_graph, Blake3ContentHasher, Cas, CasError, CasLimits, ContentHasher,
    ParentPolicy, PutStatus,
};
use cairn_core::{ArtifactId, Hash32};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
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
