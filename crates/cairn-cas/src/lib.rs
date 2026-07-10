//! Crash-safe append-only content-addressed storage for CAIRN artifacts.
//!
//! Blob and registration publication uses `create_new`, file synchronization,
//! and an atomic no-clobber hard link. A crash can therefore leave only an
//! unreferenced temporary file or a published blob missing its registration;
//! [`Cas::recover`] handles both states without mutating accepted blob bytes.

use cairn_core::{
    ArtifactEnvelope, ArtifactError, ArtifactId, BlobId, CanonicalEncode, CanonicalLimits,
    CanonicalValue, Hash32, HashParseError, SchemaRegistry, SemanticsVersion,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const REGISTRATION_MAGIC: &[u8; 4] = b"CRN1";
const REGISTRATION_LENGTH: usize = 4 + 32 + 8 + 32;
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// Explicit storage capacity bounds checked before allocation or writes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CasLimits {
    pub maximum_blob_bytes: u64,
    pub maximum_recovery_entries: usize,
}

impl Default for CasLimits {
    fn default() -> Self {
        Self {
            maximum_blob_bytes: 256 * 1024 * 1024,
            maximum_recovery_entries: 10_000_000,
        }
    }
}

/// Injectable identity function used to test collision handling.
pub trait ContentHasher: Send + Sync {
    fn hash(&self, bytes: &[u8]) -> Hash32;
}

/// Production BLAKE3 content identity.
#[derive(Clone, Copy, Debug, Default)]
pub struct Blake3ContentHasher;

impl ContentHasher for Blake3ContentHasher {
    fn hash(&self, bytes: &[u8]) -> Hash32 {
        Hash32::digest(bytes)
    }
}

/// Durable publication boundary exposed for deterministic fault injection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationPoint {
    BlobPublished,
    RegistrationPublished,
}

/// Testable crash/fault boundary. Production uses a no-op implementation.
pub trait FaultInjector: Send + Sync {
    /// Called only after the named filesystem state has been synchronized.
    ///
    /// # Errors
    ///
    /// May return an explicit injected failure. A subprocess test may instead
    /// terminate at this exact boundary to exercise crash recovery.
    fn checkpoint(&self, point: PublicationPoint) -> Result<(), CasError>;
}

#[derive(Debug)]
struct NoFault;

impl FaultInjector for NoFault {
    fn checkpoint(&self, _point: PublicationPoint) -> Result<(), CasError> {
        Ok(())
    }
}

/// Outcome of an idempotent content write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PutOutcome {
    pub id: BlobId,
    pub status: PutStatus,
}

/// Linked semantic artifact identity and physical envelope blob identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoredArtifact {
    pub artifact_id: ArtifactId,
    pub envelope_blob_id: BlobId,
    pub status: PutStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PutStatus {
    Stored,
    AlreadyPresent,
}

/// Immutable shared blob bytes. Slices are zero-copy after the bounded read.
#[derive(Clone, Debug)]
pub struct CasBlob {
    id: BlobId,
    bytes: Arc<[u8]>,
}

impl CasBlob {
    #[must_use]
    pub const fn id(&self) -> BlobId {
        self.id
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn slice(&self, range: std::ops::Range<usize>) -> Option<&[u8]> {
        self.bytes.get(range)
    }
}

/// Recovery actions performed without changing accepted blob content.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RecoveryReport {
    pub temporary_files_removed: usize,
    pub blobs_verified: usize,
    pub registrations_verified: usize,
    pub registrations_recovered: usize,
}

/// Append-only filesystem CAS.
pub struct Cas {
    root: PathBuf,
    limits: CasLimits,
    hasher: Arc<dyn ContentHasher>,
    fault_injector: Arc<dyn FaultInjector>,
}

impl fmt::Debug for Cas {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Cas")
            .field("root", &self.root)
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl Cas {
    /// Open or create a production BLAKE3 store.
    ///
    /// # Errors
    ///
    /// Returns a typed filesystem error if required directories cannot be
    /// created or synchronized.
    pub fn open(path: impl Into<PathBuf>, limits: CasLimits) -> Result<Self, CasError> {
        Self::open_with_components(
            path,
            limits,
            Arc::new(Blake3ContentHasher),
            Arc::new(NoFault),
        )
    }

    /// Open a store with an injected hasher for collision tests.
    ///
    /// # Errors
    ///
    /// Returns a typed filesystem error if initialization fails.
    pub fn open_with_hasher(
        path: impl Into<PathBuf>,
        limits: CasLimits,
        hasher: Arc<dyn ContentHasher>,
    ) -> Result<Self, CasError> {
        Self::open_with_components(path, limits, hasher, Arc::new(NoFault))
    }

    /// Open with independently injected hashing and crash boundaries.
    ///
    /// # Errors
    ///
    /// Returns a typed filesystem error if initialization fails.
    pub fn open_with_components(
        path: impl Into<PathBuf>,
        limits: CasLimits,
        hasher: Arc<dyn ContentHasher>,
        fault_injector: Arc<dyn FaultInjector>,
    ) -> Result<Self, CasError> {
        let root = path.into();
        create_directory(&root)?;
        create_directory(&root.join("blobs"))?;
        create_directory(&root.join("journal"))?;
        create_directory(&root.join("tmp"))?;
        sync_directory(&root)?;
        Ok(Self {
            root,
            limits,
            hasher,
            fault_injector,
        })
    }

    /// Store bytes once and append their immutable registration.
    ///
    /// Concurrent and cross-process writes of identical bytes converge on one
    /// path. If an injected or broken hash maps different bytes to that path,
    /// the existing content is preserved and `HashCollision` is returned.
    ///
    /// # Errors
    ///
    /// Returns capacity, collision, corruption, or filesystem errors.
    pub fn put(&self, bytes: &[u8]) -> Result<PutOutcome, CasError> {
        self.check_capacity(bytes.len())?;
        let id = BlobId::from_hash(self.hasher.hash(bytes));
        let target = self.blob_path(id);
        let stored = self.publish_blob(&target, bytes, id)?;
        self.publish_registration(id, bytes.len())?;
        Ok(PutOutcome {
            id,
            status: if stored {
                PutStatus::Stored
            } else {
                PutStatus::AlreadyPresent
            },
        })
    }

    /// Validate and store one complete canonical artifact envelope.
    ///
    /// # Errors
    ///
    /// Returns artifact validation/encoding failures before publication, then
    /// the same capacity, collision, corruption, or filesystem errors as
    /// [`Cas::put`].
    pub fn put_envelope<T: CanonicalEncode>(
        &self,
        envelope: &ArtifactEnvelope<T>,
    ) -> Result<StoredArtifact, CasError> {
        envelope.validate()?;
        let bytes = envelope.encode_envelope()?;
        let outcome = self.put(&bytes)?;
        Ok(StoredArtifact {
            artifact_id: envelope.artifact_id,
            envelope_blob_id: outcome.id,
            status: outcome.status,
        })
    }

    /// Load, decode, negotiate, and validate a stored artifact envelope.
    ///
    /// # Errors
    ///
    /// Returns storage integrity errors before canonical envelope, schema,
    /// semantics, payload, and identity validation errors.
    pub fn get_envelope(
        &self,
        blob_id: BlobId,
        registry: &SchemaRegistry,
        supported_semantics: &BTreeSet<SemanticsVersion>,
        limits: CanonicalLimits,
    ) -> Result<ArtifactEnvelope<CanonicalValue>, CasError> {
        let blob = self.get(blob_id)?;
        ArtifactEnvelope::decode_envelope(blob.as_slice(), registry, supported_semantics, limits)
            .map_err(CasError::Artifact)
    }

    /// Read and verify one bounded immutable blob.
    ///
    /// # Errors
    ///
    /// Returns not-found, capacity, hash mismatch, or filesystem errors.
    pub fn get(&self, id: BlobId) -> Result<CasBlob, CasError> {
        let bytes = self.read_and_verify(id)?;
        Ok(CasBlob {
            id,
            bytes: Arc::from(bytes),
        })
    }

    /// Verify content and registration without returning retained bytes.
    ///
    /// # Errors
    ///
    /// Returns precise absence, corruption, capacity, or filesystem failures.
    pub fn verify(&self, id: BlobId) -> Result<(), CasError> {
        let bytes = self.read_and_verify(id)?;
        self.verify_registration(id, bytes.len())
    }

    /// Recover safe crash residues and validate the complete local registry.
    ///
    /// Temporary files are never accepted. Published blobs missing a journal
    /// record receive a reconstructed record only after their path and BLAKE3
    /// content agree. Registered missing/corrupt blobs fail closed.
    ///
    /// # Errors
    ///
    /// Returns capacity, corruption, invalid path, or filesystem errors.
    pub fn recover(&self) -> Result<RecoveryReport, CasError> {
        let mut report = RecoveryReport::default();
        for path in sorted_files(&self.root.join("tmp"))? {
            fs::remove_file(&path).map_err(|source| io_error("remove temporary", &path, source))?;
            report.temporary_files_removed += 1;
        }

        let blobs = self.scan_ids("blobs")?;
        let registrations = self.scan_ids("journal")?;
        if blobs.len().max(registrations.len()) > self.limits.maximum_recovery_entries {
            return Err(CasError::RecoveryCapacity {
                actual: blobs.len().max(registrations.len()),
                maximum: self.limits.maximum_recovery_entries,
            });
        }
        for id in &registrations {
            self.verify(*id)?;
            report.registrations_verified += 1;
        }
        for id in &blobs {
            let bytes = self.read_and_verify(*id)?;
            report.blobs_verified += 1;
            if !registrations.contains(id) {
                self.publish_registration(*id, bytes.len())?;
                report.registrations_recovered += 1;
            }
        }
        Ok(report)
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn check_capacity(&self, length: usize) -> Result<(), CasError> {
        let length = u64::try_from(length).map_err(|_| CasError::LengthOverflow)?;
        if length > self.limits.maximum_blob_bytes {
            return Err(CasError::CapacityCap {
                requested: length,
                maximum: self.limits.maximum_blob_bytes,
            });
        }
        Ok(())
    }

    fn publish_blob(&self, target: &Path, bytes: &[u8], id: BlobId) -> Result<bool, CasError> {
        if target.exists() {
            self.verify_existing_bytes(target, bytes, id)?;
            return Ok(false);
        }
        let parent = target
            .parent()
            .ok_or_else(|| CasError::InternalInvariant("blob target has no parent".to_owned()))?;
        create_directory(parent)?;
        let temporary = self.temporary_path("blob");
        write_new_synced(&temporary, bytes)?;
        let result = fs::hard_link(&temporary, target);
        match result {
            Ok(()) => {
                sync_directory(parent)?;
                self.fault_injector
                    .checkpoint(PublicationPoint::BlobPublished)?;
                remove_temporary(&temporary)?;
                Ok(true)
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                remove_temporary(&temporary)?;
                self.verify_existing_bytes(target, bytes, id)?;
                Ok(false)
            }
            Err(source) => {
                let _cleanup_result = fs::remove_file(&temporary);
                Err(io_error("publish blob", target, source))
            }
        }
    }

    fn verify_existing_bytes(
        &self,
        path: &Path,
        expected: &[u8],
        id: BlobId,
    ) -> Result<(), CasError> {
        let existing = read_bounded(path, self.limits.maximum_blob_bytes)?;
        if existing != expected {
            return Err(CasError::HashCollision { id });
        }
        Ok(())
    }

    fn publish_registration(&self, id: BlobId, length: usize) -> Result<(), CasError> {
        let target = self.registration_path(id);
        let registration = registration_bytes(id, length)?;
        if target.exists() {
            return self.verify_registration(id, length);
        }
        let parent = target.parent().ok_or_else(|| {
            CasError::InternalInvariant("registration target has no parent".to_owned())
        })?;
        create_directory(parent)?;
        let temporary = self.temporary_path("registration");
        write_new_synced(&temporary, &registration)?;
        match fs::hard_link(&temporary, &target) {
            Ok(()) => {
                sync_directory(parent)?;
                self.fault_injector
                    .checkpoint(PublicationPoint::RegistrationPublished)?;
                remove_temporary(&temporary)
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                remove_temporary(&temporary)?;
                self.verify_registration(id, length)
            }
            Err(source) => {
                let _cleanup_result = fs::remove_file(&temporary);
                Err(io_error("publish registration", &target, source))
            }
        }
    }

    fn verify_registration(&self, id: BlobId, expected_length: usize) -> Result<(), CasError> {
        let path = self.registration_path(id);
        let bytes = fs::read(&path).map_err(|source| {
            if source.kind() == io::ErrorKind::NotFound {
                CasError::MissingRegistration { id }
            } else {
                io_error("read registration", &path, source)
            }
        })?;
        let (registered_id, registered_length) = parse_registration(&bytes)?;
        if registered_id != id || registered_length != expected_length as u64 {
            return Err(CasError::RegistrationMismatch { id });
        }
        Ok(())
    }

    fn read_and_verify(&self, id: BlobId) -> Result<Vec<u8>, CasError> {
        let path = self.blob_path(id);
        let bytes = match read_bounded(&path, self.limits.maximum_blob_bytes) {
            Err(CasError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                return Err(CasError::NotFound { id });
            }
            result => result?,
        };
        let actual = BlobId::from_hash(self.hasher.hash(&bytes));
        if actual != id {
            return Err(CasError::HashMismatch {
                expected: id,
                actual,
            });
        }
        Ok(bytes)
    }

    fn scan_ids(&self, directory: &str) -> Result<BTreeSet<BlobId>, CasError> {
        let root = self.root.join(directory);
        let mut ids = BTreeSet::new();
        for fanout in sorted_directories(&root)? {
            let prefix = fanout
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| CasError::InvalidStorePath(fanout.clone()))?;
            if prefix.len() != 2 || !prefix.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(CasError::InvalidStorePath(fanout));
            }
            for path in sorted_files(&fanout)? {
                let suffix = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| CasError::InvalidStorePath(path.clone()))?;
                let hash: Hash32 = format!("{prefix}{suffix}").parse()?;
                ids.insert(BlobId::from_hash(hash));
            }
        }
        Ok(ids)
    }

    fn blob_path(&self, id: BlobId) -> PathBuf {
        fanout_path(&self.root.join("blobs"), id)
    }

    fn registration_path(&self, id: BlobId) -> PathBuf {
        fanout_path(&self.root.join("journal"), id)
    }

    fn temporary_path(&self, kind: &str) -> PathBuf {
        let sequence = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        self.root
            .join("tmp")
            .join(format!("{kind}-{}-{sequence}", std::process::id()))
    }
}

fn registration_bytes(id: BlobId, length: usize) -> Result<Vec<u8>, CasError> {
    let length = u64::try_from(length).map_err(|_| CasError::LengthOverflow)?;
    let mut record = Vec::with_capacity(REGISTRATION_LENGTH);
    record.extend_from_slice(REGISTRATION_MAGIC);
    record.extend_from_slice(id.hash().as_bytes());
    record.extend_from_slice(&length.to_be_bytes());
    let checksum = Hash32::digest(&record);
    record.extend_from_slice(checksum.as_bytes());
    Ok(record)
}

fn parse_registration(bytes: &[u8]) -> Result<(BlobId, u64), CasError> {
    if bytes.len() != REGISTRATION_LENGTH || bytes.get(..4) != Some(REGISTRATION_MAGIC) {
        return Err(CasError::InvalidRegistration);
    }
    let body = bytes.get(..44).ok_or(CasError::InvalidRegistration)?;
    let checksum = bytes.get(44..).ok_or(CasError::InvalidRegistration)?;
    if Hash32::digest(body).as_bytes().as_slice() != checksum {
        return Err(CasError::InvalidRegistration);
    }
    let mut id_bytes = [0_u8; 32];
    id_bytes.copy_from_slice(bytes.get(4..36).ok_or(CasError::InvalidRegistration)?);
    let mut length_bytes = [0_u8; 8];
    length_bytes.copy_from_slice(bytes.get(36..44).ok_or(CasError::InvalidRegistration)?);
    Ok((
        BlobId::from_hash(Hash32::from_bytes(id_bytes)),
        u64::from_be_bytes(length_bytes),
    ))
}

fn fanout_path(root: &Path, id: BlobId) -> PathBuf {
    let hex = id.hash().to_hex();
    root.join(&hex[..2]).join(&hex[2..])
}

fn create_directory(path: &Path) -> Result<(), CasError> {
    fs::create_dir_all(path).map_err(|source| io_error("create directory", path, source))
}

fn sync_directory(path: &Path) -> Result<(), CasError> {
    let directory = File::open(path).map_err(|source| io_error("open directory", path, source))?;
    directory
        .sync_all()
        .map_err(|source| io_error("sync directory", path, source))
}

fn write_new_synced(path: &Path, bytes: &[u8]) -> Result<(), CasError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|source| io_error("create temporary", path, source))?;
    file.write_all(bytes)
        .map_err(|source| io_error("write temporary", path, source))?;
    file.sync_all()
        .map_err(|source| io_error("sync temporary", path, source))
}

fn remove_temporary(path: &Path) -> Result<(), CasError> {
    fs::remove_file(path).map_err(|source| io_error("remove temporary", path, source))
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, CasError> {
    let file = File::open(path).map_err(|source| io_error("open blob", path, source))?;
    let declared = file
        .metadata()
        .map_err(|source| io_error("read blob metadata", path, source))?
        .len();
    if declared > maximum {
        return Err(CasError::CapacityCap {
            requested: declared,
            maximum,
        });
    }
    let capacity = usize::try_from(declared).map_err(|_| CasError::LengthOverflow)?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| io_error("read blob", path, source))?;
    if bytes.len() as u64 > maximum {
        return Err(CasError::CapacityCap {
            requested: bytes.len() as u64,
            maximum,
        });
    }
    Ok(bytes)
}

fn sorted_files(path: &Path) -> Result<Vec<PathBuf>, CasError> {
    sorted_entries(path, false)
}

fn sorted_directories(path: &Path) -> Result<Vec<PathBuf>, CasError> {
    sorted_entries(path, true)
}

fn sorted_entries(path: &Path, directories: bool) -> Result<Vec<PathBuf>, CasError> {
    let mut entries = fs::read_dir(path)
        .map_err(|source| io_error("read directory", path, source))?
        .map(|entry| {
            entry
                .map(|value| value.path())
                .map_err(|source| io_error("read directory entry", path, source))
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.retain(|entry| entry.is_dir() == directories);
    entries.sort();
    Ok(entries)
}

fn io_error(operation: &'static str, path: &Path, source: io::Error) -> CasError {
    CasError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

/// Exact storage or integrity failure.
#[derive(Debug)]
pub enum CasError {
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    CapacityCap {
        requested: u64,
        maximum: u64,
    },
    RecoveryCapacity {
        actual: usize,
        maximum: usize,
    },
    LengthOverflow,
    NotFound {
        id: BlobId,
    },
    MissingRegistration {
        id: BlobId,
    },
    HashMismatch {
        expected: BlobId,
        actual: BlobId,
    },
    HashCollision {
        id: BlobId,
    },
    RegistrationMismatch {
        id: BlobId,
    },
    InvalidRegistration,
    InvalidStorePath(PathBuf),
    InvalidHash(HashParseError),
    Artifact(ArtifactError),
    ParentMissing {
        parent: ArtifactId,
    },
    ParentCycle {
        at: ArtifactId,
    },
    ParentGraphCapacity {
        maximum: usize,
    },
    InternalInvariant(String),
}

impl fmt::Display for CasError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => write!(formatter, "{operation} {}: {source}", path.display()),
            Self::CapacityCap { requested, maximum } => {
                write!(formatter, "blob needs {requested} bytes; cap is {maximum}")
            }
            Self::RecoveryCapacity { actual, maximum } => {
                write!(formatter, "recovery saw {actual} entries; cap is {maximum}")
            }
            Self::LengthOverflow => formatter.write_str("length exceeds address space"),
            Self::NotFound { id } => write!(formatter, "blob not found: {id}"),
            Self::MissingRegistration { id } => write!(formatter, "registration not found: {id}"),
            Self::HashMismatch { expected, actual } => {
                write!(
                    formatter,
                    "blob hash mismatch: expected {expected}, actual {actual}"
                )
            }
            Self::HashCollision { id } => {
                write!(formatter, "different bytes map to existing ID {id}")
            }
            Self::RegistrationMismatch { id } => {
                write!(formatter, "registration does not match {id}")
            }
            Self::InvalidRegistration => {
                formatter.write_str("registration record is malformed or torn")
            }
            Self::InvalidStorePath(path) => {
                write!(formatter, "invalid CAS path {}", path.display())
            }
            Self::InvalidHash(error) => write!(formatter, "invalid CAS digest path: {error}"),
            Self::Artifact(error) => write!(formatter, "invalid artifact envelope: {error}"),
            Self::ParentMissing { parent } => write!(formatter, "parent is missing: {parent}"),
            Self::ParentCycle { at } => write!(formatter, "parent cycle reaches {at}"),
            Self::ParentGraphCapacity { maximum } => {
                write!(formatter, "parent graph exceeds {maximum} nodes")
            }
            Self::InternalInvariant(message) => {
                write!(formatter, "internal CAS invariant: {message}")
            }
        }
    }
}

impl std::error::Error for CasError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidHash(source) => Some(source),
            Self::Artifact(source) => Some(source),
            _ => None,
        }
    }
}

impl From<HashParseError> for CasError {
    fn from(value: HashParseError) -> Self {
        Self::InvalidHash(value)
    }
}

impl From<ArtifactError> for CasError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

/// Missing-parent behavior for graph validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParentPolicy {
    RequirePresent,
    AllowMissing,
}

/// Validate a bounded artifact-parent graph independently of envelope creation.
///
/// # Errors
///
/// Returns the first canonical missing parent, cycle, or capacity failure.
pub fn validate_parent_graph(
    root: ArtifactId,
    graph: &BTreeMap<ArtifactId, Vec<ArtifactId>>,
    policy: ParentPolicy,
    maximum_nodes: usize,
) -> Result<(), CasError> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    visit_parents(
        root,
        graph,
        policy,
        maximum_nodes,
        &mut visiting,
        &mut visited,
    )
}

fn visit_parents(
    id: ArtifactId,
    graph: &BTreeMap<ArtifactId, Vec<ArtifactId>>,
    policy: ParentPolicy,
    maximum_nodes: usize,
    visiting: &mut BTreeSet<ArtifactId>,
    visited: &mut BTreeSet<ArtifactId>,
) -> Result<(), CasError> {
    if visited.contains(&id) {
        return Ok(());
    }
    if !visiting.insert(id) {
        return Err(CasError::ParentCycle { at: id });
    }
    if visiting.len() + visited.len() > maximum_nodes {
        return Err(CasError::ParentGraphCapacity {
            maximum: maximum_nodes,
        });
    }
    let Some(parents) = graph.get(&id) else {
        visiting.remove(&id);
        if policy == ParentPolicy::RequirePresent {
            return Err(CasError::ParentMissing { parent: id });
        }
        visited.insert(id);
        return Ok(());
    };
    let mut parents = parents.clone();
    parents.sort();
    parents.dedup();
    for parent in parents {
        visit_parents(parent, graph, policy, maximum_nodes, visiting, visited)?;
    }
    visiting.remove(&id);
    visited.insert(id);
    Ok(())
}
