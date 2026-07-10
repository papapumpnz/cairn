use crate::{
    ArtifactId, CanonicalEncode, CanonicalError, CanonicalValue, Hash32, ARTIFACT_PROTOCOL_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

/// Validated `major.minor.patch` executable-semantics version.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SemanticsVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl SemanticsVersion {
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for SemanticsVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for SemanticsVersion {
    type Err = ArtifactError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split('.');
        let major = parse_version_part(parts.next(), value)?;
        let minor = parse_version_part(parts.next(), value)?;
        let patch = parse_version_part(parts.next(), value)?;
        if parts.next().is_some() {
            return Err(ArtifactError::InvalidVersion(value.to_owned()));
        }
        Ok(Self::new(major, minor, patch))
    }
}

fn parse_version_part(value: Option<&str>, complete: &str) -> Result<u16, ArtifactError> {
    let value = value.ok_or_else(|| ArtifactError::InvalidVersion(complete.to_owned()))?;
    if value.is_empty() || (value.len() > 1 && value.starts_with('0')) {
        return Err(ArtifactError::InvalidVersion(complete.to_owned()));
    }
    value
        .parse()
        .map_err(|_| ArtifactError::InvalidVersion(complete.to_owned()))
}

/// Versioned artifact schema reference.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SchemaRef {
    name: String,
    version: u32,
}

impl SchemaRef {
    /// Validate a schema name and positive version.
    ///
    /// # Errors
    ///
    /// Rejects names outside lowercase dotted identifier syntax or version 0.
    pub fn new(name: impl Into<String>, version: u32) -> Result<Self, ArtifactError> {
        let name = name.into();
        validate_name(&name, 128)?;
        if version == 0 {
            return Err(ArtifactError::InvalidSchemaVersion(version));
        }
        Ok(Self { name, version })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }
}

/// Validated artifact kind used in semantic identity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ArtifactKind(String);

impl ArtifactKind {
    /// Construct a lowercase dotted artifact kind.
    ///
    /// # Errors
    ///
    /// Rejects empty, oversized, uppercase, or punctuation-unsafe names.
    pub fn new(value: impl Into<String>) -> Result<Self, ArtifactError> {
        let value = value.into();
        validate_name(&value, 64)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_name(value: &str, maximum: usize) -> Result<(), ArtifactError> {
    let mut bytes = value.bytes();
    if value.len() > maximum
        || !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        || !bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_.-".contains(&byte)
        })
    {
        return Err(ArtifactError::InvalidName(value.to_owned()));
    }
    Ok(())
}

/// Non-semantic producer observation attached to an artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerRecord {
    pub component: String,
    pub version: String,
    pub commit: String,
    pub host_run_id: String,
    pub agent_id: Option<String>,
}

/// Actual consumed resources; excluded from semantic identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetRecord {
    pub work_units: u64,
    pub cpu_nanos: u64,
    pub peak_rss_bytes: u64,
}

/// Immutable typed artifact plus non-semantic observation metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactEnvelope<T> {
    pub protocol_version: u16,
    pub schema: SchemaRef,
    pub artifact_id: ArtifactId,
    pub artifact_kind: ArtifactKind,
    pub semantics_version: SemanticsVersion,
    pub parent_ids: Vec<ArtifactId>,
    pub task_scope: Option<Hash32>,
    pub producer: ProducerRecord,
    pub budget: BudgetRecord,
    pub created_at_logical: u64,
    pub payload_hash: Hash32,
    pub payload: T,
    pub signature: Option<Vec<u8>>,
}

impl<T: CanonicalEncode> ArtifactEnvelope<T> {
    /// Construct an envelope and its semantic identity from canonical payload.
    ///
    /// Parent order is normalized and duplicates or a reconstructed self-cycle
    /// fail closed. Producer and observation metadata do not affect identity.
    ///
    /// # Errors
    ///
    /// Returns typed canonicalization, duplicate-parent, or self-cycle errors.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        schema: SchemaRef,
        artifact_kind: ArtifactKind,
        semantics_version: SemanticsVersion,
        mut parent_ids: Vec<ArtifactId>,
        task_scope: Option<Hash32>,
        producer: ProducerRecord,
        budget: BudgetRecord,
        created_at_logical: u64,
        payload: T,
    ) -> Result<Self, ArtifactError> {
        parent_ids.sort();
        if parent_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ArtifactError::DuplicateParent);
        }
        let payload_bytes = payload.encode_canonical()?;
        let payload_hash = Hash32::digest(&payload_bytes);
        let artifact_id = semantic_artifact_id(
            ARTIFACT_PROTOCOL_VERSION,
            &schema,
            &artifact_kind,
            &semantics_version,
            &parent_ids,
            task_scope,
            payload_hash,
            payload_bytes.len(),
        )?;
        if parent_ids.contains(&artifact_id) {
            return Err(ArtifactError::ParentCycle(artifact_id));
        }
        Ok(Self {
            protocol_version: ARTIFACT_PROTOCOL_VERSION,
            schema,
            artifact_id,
            artifact_kind,
            semantics_version,
            parent_ids,
            task_scope,
            producer,
            budget,
            created_at_logical,
            payload_hash,
            payload,
            signature: None,
        })
    }

    /// Reconstruct payload and semantic hashes and compare every identity field.
    ///
    /// # Errors
    ///
    /// Rejects unsupported protocol, payload corruption, identity mismatch,
    /// duplicate parents, or a direct parent cycle.
    pub fn validate(&self) -> Result<(), ArtifactError> {
        if self.protocol_version != ARTIFACT_PROTOCOL_VERSION {
            return Err(ArtifactError::UnsupportedProtocol(self.protocol_version));
        }
        if self.parent_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ArtifactError::ParentsNotCanonical);
        }
        let payload_bytes = self.payload.encode_canonical()?;
        let payload_hash = Hash32::digest(&payload_bytes);
        if payload_hash != self.payload_hash {
            return Err(ArtifactError::PayloadHashMismatch {
                declared: self.payload_hash,
                actual: payload_hash,
            });
        }
        let artifact_id = semantic_artifact_id(
            self.protocol_version,
            &self.schema,
            &self.artifact_kind,
            &self.semantics_version,
            &self.parent_ids,
            self.task_scope,
            payload_hash,
            payload_bytes.len(),
        )?;
        if artifact_id != self.artifact_id {
            return Err(ArtifactError::ArtifactIdMismatch {
                declared: self.artifact_id,
                actual: artifact_id,
            });
        }
        if self.parent_ids.contains(&self.artifact_id) {
            return Err(ArtifactError::ParentCycle(self.artifact_id));
        }
        Ok(())
    }

    /// Serialize the complete envelope, including non-semantic metadata.
    ///
    /// # Errors
    ///
    /// Returns canonical encoding errors from the payload or envelope maps.
    pub fn encode_envelope(&self) -> Result<Vec<u8>, ArtifactError> {
        let payload = self.payload.encode_canonical()?;
        let value = CanonicalValue::Map(vec![
            text_entry(
                "protocol",
                CanonicalValue::Unsigned(u64::from(self.protocol_version)),
            ),
            text_entry("schema", schema_value(&self.schema)),
            text_entry("artifact_id", id_value(self.artifact_id)),
            text_entry("kind", CanonicalValue::Text(self.artifact_kind.0.clone())),
            text_entry(
                "semantics",
                CanonicalValue::Text(self.semantics_version.to_string()),
            ),
            text_entry("parents", parent_value(&self.parent_ids)),
            text_entry(
                "task_scope",
                self.task_scope.map_or(CanonicalValue::Null, hash_value),
            ),
            text_entry("producer", producer_value(&self.producer)),
            text_entry("budget", budget_value(&self.budget)),
            text_entry("created", CanonicalValue::Unsigned(self.created_at_logical)),
            text_entry("payload_hash", hash_value(self.payload_hash)),
            text_entry("payload", CanonicalValue::Bytes(payload)),
            text_entry(
                "signature",
                self.signature
                    .clone()
                    .map_or(CanonicalValue::Null, CanonicalValue::Bytes),
            ),
        ]);
        value.encode_canonical().map_err(ArtifactError::Canonical)
    }
}

// The identity tuple is intentionally explicit: omitting any member changes
// artifact semantics and must be review-visible at this call boundary.
#[allow(clippy::too_many_arguments)]
fn semantic_artifact_id(
    protocol: u16,
    schema: &SchemaRef,
    kind: &ArtifactKind,
    semantics: &SemanticsVersion,
    parents: &[ArtifactId],
    task_scope: Option<Hash32>,
    payload_hash: Hash32,
    payload_len: usize,
) -> Result<ArtifactId, ArtifactError> {
    let payload_len = u64::try_from(payload_len).map_err(|_| ArtifactError::LengthOverflow)?;
    let identity = CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.artifact.identity".to_owned()),
        CanonicalValue::Unsigned(u64::from(protocol)),
        schema_value(schema),
        CanonicalValue::Text(kind.0.clone()),
        CanonicalValue::Text(semantics.to_string()),
        parent_value(parents),
        task_scope.map_or(CanonicalValue::Null, hash_value),
        hash_value(payload_hash),
        CanonicalValue::Unsigned(payload_len),
    ]);
    Ok(ArtifactId::from_hash(Hash32::digest(
        &identity.encode_canonical()?,
    )))
}

fn text_entry(key: &str, value: CanonicalValue) -> (CanonicalValue, CanonicalValue) {
    (CanonicalValue::Text(key.to_owned()), value)
}

fn schema_value(schema: &SchemaRef) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Text(schema.name.clone()),
        CanonicalValue::Unsigned(u64::from(schema.version)),
    ])
}

fn hash_value(hash: Hash32) -> CanonicalValue {
    CanonicalValue::Bytes(hash.as_bytes().to_vec())
}

fn id_value(id: ArtifactId) -> CanonicalValue {
    hash_value(id.hash())
}

fn parent_value(parents: &[ArtifactId]) -> CanonicalValue {
    CanonicalValue::Array(parents.iter().copied().map(id_value).collect())
}

fn producer_value(producer: &ProducerRecord) -> CanonicalValue {
    CanonicalValue::Map(vec![
        text_entry(
            "component",
            CanonicalValue::Text(producer.component.clone()),
        ),
        text_entry("version", CanonicalValue::Text(producer.version.clone())),
        text_entry("commit", CanonicalValue::Text(producer.commit.clone())),
        text_entry(
            "host_run_id",
            CanonicalValue::Text(producer.host_run_id.clone()),
        ),
        text_entry(
            "agent_id",
            producer
                .agent_id
                .clone()
                .map_or(CanonicalValue::Null, CanonicalValue::Text),
        ),
    ])
}

fn budget_value(budget: &BudgetRecord) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Unsigned(budget.work_units),
        CanonicalValue::Unsigned(budget.cpu_nanos),
        CanonicalValue::Unsigned(budget.peak_rss_bytes),
    ])
}

/// Schema read compatibility decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaDecision {
    Current,
    AdditiveCompatible,
}

/// One registered schema and explicitly accepted additive reader versions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaDescriptor {
    pub name: String,
    pub current_version: u32,
    pub additive_read_versions: BTreeSet<u32>,
    pub maximum_payload_bytes: usize,
}

/// Deterministic fail-closed schema registry.
#[derive(Clone, Debug, Default)]
pub struct SchemaRegistry {
    schemas: BTreeMap<String, SchemaDescriptor>,
}

impl SchemaRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schemas: BTreeMap::new(),
        }
    }

    /// Register exactly one current descriptor per schema name.
    ///
    /// # Errors
    ///
    /// Rejects malformed names, version zero, an additive set containing the
    /// current version, zero payload capacity, or duplicate registration.
    pub fn register(&mut self, descriptor: SchemaDescriptor) -> Result<(), ArtifactError> {
        validate_name(&descriptor.name, 128)?;
        if descriptor.current_version == 0 {
            return Err(ArtifactError::InvalidSchemaVersion(0));
        }
        if descriptor.maximum_payload_bytes == 0 {
            return Err(ArtifactError::InvalidPayloadCapacity);
        }
        if descriptor
            .additive_read_versions
            .contains(&descriptor.current_version)
        {
            return Err(ArtifactError::InvalidCompatibilitySet);
        }
        if self.schemas.contains_key(&descriptor.name) {
            return Err(ArtifactError::DuplicateSchema(descriptor.name));
        }
        self.schemas.insert(descriptor.name.clone(), descriptor);
        Ok(())
    }

    /// Resolve a schema version without silently migrating incompatible input.
    ///
    /// # Errors
    ///
    /// Returns `UnsupportedSchema` for unknown names or versions.
    pub fn resolve(&self, schema: &SchemaRef) -> Result<SchemaDecision, ArtifactError> {
        let descriptor = self
            .schemas
            .get(schema.name())
            .ok_or_else(|| ArtifactError::UnsupportedSchema(schema.clone()))?;
        if schema.version() == descriptor.current_version {
            Ok(SchemaDecision::Current)
        } else if descriptor
            .additive_read_versions
            .contains(&schema.version())
        {
            Ok(SchemaDecision::AdditiveCompatible)
        } else {
            Err(ArtifactError::UnsupportedSchema(schema.clone()))
        }
    }

    /// Return the declared payload capacity after successful resolution.
    ///
    /// # Errors
    ///
    /// Returns `UnsupportedSchema` when resolution fails.
    pub fn maximum_payload_bytes(&self, schema: &SchemaRef) -> Result<usize, ArtifactError> {
        self.resolve(schema)?;
        self.schemas
            .get(schema.name())
            .map(|descriptor| descriptor.maximum_payload_bytes)
            .ok_or_else(|| ArtifactError::UnsupportedSchema(schema.clone()))
    }
}

/// Exact artifact construction or validation failure.
#[derive(Clone, Debug, PartialEq)]
pub enum ArtifactError {
    Canonical(CanonicalError),
    InvalidVersion(String),
    InvalidName(String),
    InvalidSchemaVersion(u32),
    InvalidPayloadCapacity,
    InvalidCompatibilitySet,
    DuplicateSchema(String),
    UnsupportedSchema(SchemaRef),
    UnsupportedProtocol(u16),
    DuplicateParent,
    ParentsNotCanonical,
    ParentCycle(ArtifactId),
    LengthOverflow,
    PayloadHashMismatch {
        declared: Hash32,
        actual: Hash32,
    },
    ArtifactIdMismatch {
        declared: ArtifactId,
        actual: ArtifactId,
    },
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonical(error) => {
                write!(formatter, "canonical artifact encoding failed: {error}")
            }
            Self::InvalidVersion(value) => write!(formatter, "invalid semantics version `{value}`"),
            Self::InvalidName(value) => write!(formatter, "invalid canonical name `{value}`"),
            Self::InvalidSchemaVersion(value) => {
                write!(formatter, "invalid schema version {value}")
            }
            Self::InvalidPayloadCapacity => {
                formatter.write_str("schema payload capacity must be positive")
            }
            Self::InvalidCompatibilitySet => {
                formatter.write_str("compatibility set contains current schema version")
            }
            Self::DuplicateSchema(value) => {
                write!(formatter, "schema `{value}` is already registered")
            }
            Self::UnsupportedSchema(value) => {
                write!(
                    formatter,
                    "unsupported schema {} version {}",
                    value.name, value.version
                )
            }
            Self::UnsupportedProtocol(value) => {
                write!(formatter, "unsupported artifact protocol {value}")
            }
            Self::DuplicateParent => formatter.write_str("parent artifact IDs are not unique"),
            Self::ParentsNotCanonical => {
                formatter.write_str("parent artifact IDs are not sorted and unique")
            }
            Self::ParentCycle(id) => write!(formatter, "artifact is its own parent: {id}"),
            Self::LengthOverflow => formatter.write_str("payload length exceeds canonical range"),
            Self::PayloadHashMismatch { declared, actual } => {
                write!(
                    formatter,
                    "payload hash mismatch: declared {declared}, actual {actual}"
                )
            }
            Self::ArtifactIdMismatch { declared, actual } => {
                write!(
                    formatter,
                    "artifact ID mismatch: declared {declared}, actual {actual}"
                )
            }
        }
    }
}

impl std::error::Error for ArtifactError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Canonical(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CanonicalError> for ArtifactError {
    fn from(value: CanonicalError) -> Self {
        Self::Canonical(value)
    }
}
