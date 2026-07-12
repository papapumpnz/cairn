use crate::{ModuleStatus, ReuseArtifactMetadata};
use cairn_cas::{Cas, CasError, StoredArtifact};
use cairn_core::{
    ArtifactEnvelope, ArtifactError, ArtifactKind, BlobId, CanonicalEncode, CanonicalLimits,
    CanonicalValue, Hash32, SchemaDescriptor, SchemaRef, SchemaRegistry, SemanticsVersion,
};
use cairn_verifier::{
    LatentSchemaVerifier, LatentSchemaVerifyError, VerifiedLatentSchemaCompilation,
    VerifiedLatentSchemaTransfer,
};
use cairn_world::{
    ActionId, LatentEpisode, LatentMechanismSchema, LatentSchemaBinding, LatentSchemaError,
    LatentTransitionTheory, StateSpace,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const REQUIRED_INDEPENDENT_WORLDS: usize = 2;
pub const LATENT_WORLD_EVIDENCE_SCHEMA: &str = "cairn.memory.latent-world-evidence";
pub const LATENT_WORLD_EVIDENCE_SCHEMA_VERSION: u32 = 1;

/// One source-independent hidden-world mechanism and its proof-gated,
/// independently reproduced world evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentMechanismModule {
    schema: LatentMechanismSchema,
    validations: BTreeMap<Hash32, LatentWorldValidation>,
    status: ModuleStatus,
}

/// Explicit structural support proven for one persistent hidden-world schema.
/// Parameters not represented here remain unclaimed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LatentSupportGeometry {
    pub visible_dimensions: u16,
    pub latent_states: u8,
    pub action_roles: usize,
    pub independently_verified_worlds: usize,
}

impl LatentMechanismModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.schema.semantic_hash()
    }

    #[must_use]
    pub const fn schema(&self) -> &LatentMechanismSchema {
        &self.schema
    }

    #[must_use]
    pub const fn status(&self) -> ModuleStatus {
        self.status
    }

    #[must_use]
    pub fn independently_verified_worlds(&self) -> usize {
        self.validations.len()
    }

    #[must_use]
    pub fn support_geometry(&self) -> LatentSupportGeometry {
        LatentSupportGeometry {
            visible_dimensions: self.schema.dimensions(),
            latent_states: self.schema.latent_states(),
            action_roles: self.schema.roles().len(),
            independently_verified_worlds: self.validations.len(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LatentWorldValidation {
    compilation: VerifiedLatentSchemaCompilation,
    transfer: VerifiedLatentSchemaTransfer,
}

/// Automatic promotion result after accepting an opaque verifier evidence
/// pair. Callers cannot directly set memory status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatentPromotionProgress {
    Provisional {
        independently_verified_worlds: usize,
    },
    Persistent {
        independently_verified_worlds: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatentStoreError {
    SchemaMismatch,
    EvidenceMismatch,
    EmptyReplay,
    DuplicateExperience,
}

/// Proof-gated memory for transferable hidden-world mechanisms. Retrieval is
/// proposal-only; current-world rebinding and verification remain mandatory.
#[derive(Clone, Debug, Default)]
pub struct LatentMechanismStore {
    modules: BTreeMap<Hash32, LatentMechanismModule>,
}

impl LatentMechanismStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Record one independently compiled and replayed world. The store
    /// promotes automatically only after two distinct experience hashes.
    ///
    /// # Errors
    ///
    /// Rejects identity disagreement, compilation/transfer evidence mismatch,
    /// empty replay, and duplicate experience.
    pub fn record_verified_world(
        &mut self,
        schema: &LatentMechanismSchema,
        compilation: VerifiedLatentSchemaCompilation,
        transfer: VerifiedLatentSchemaTransfer,
    ) -> Result<LatentPromotionProgress, LatentStoreError> {
        let id = schema.semantic_hash();
        if compilation.schema() != id || transfer.schema() != id {
            return Err(LatentStoreError::SchemaMismatch);
        }
        if compilation.evidence() != transfer.experience() {
            return Err(LatentStoreError::EvidenceMismatch);
        }
        if compilation.replayed_rules() == 0 || transfer.replayed_transitions() == 0 {
            return Err(LatentStoreError::EmptyReplay);
        }
        let module = self
            .modules
            .entry(id)
            .or_insert_with(|| LatentMechanismModule {
                schema: schema.clone(),
                validations: BTreeMap::new(),
                status: ModuleStatus::Provisional,
            });
        if &module.schema != schema {
            return Err(LatentStoreError::SchemaMismatch);
        }
        let experience = transfer.experience();
        if module.validations.contains_key(&experience) {
            return Err(LatentStoreError::DuplicateExperience);
        }
        module.validations.insert(
            experience,
            LatentWorldValidation {
                compilation,
                transfer,
            },
        );
        let count = module.validations.len();
        if count >= REQUIRED_INDEPENDENT_WORLDS {
            module.status = ModuleStatus::Persistent;
            Ok(LatentPromotionProgress::Persistent {
                independently_verified_worlds: count,
            })
        } else {
            Ok(LatentPromotionProgress::Provisional {
                independently_verified_worlds: count,
            })
        }
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&LatentMechanismModule> {
        self.modules.get(&id)
    }

    /// Return only persistent schemas, in semantic-hash order, as untrusted
    /// proposals for bounded current-world rebinding.
    pub fn persistent_schemas(&self) -> impl Iterator<Item = &LatentMechanismSchema> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
            .map(|module| &module.schema)
    }
}

/// Untrusted canonical commitment to one schema compilation and exact visible
/// world replay. Authority is regained only through [`LatentWorldEvidenceArchive::replay`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentWorldEvidenceArtifact {
    schema: LatentMechanismSchema,
    experience: Hash32,
    replayed_rules: u64,
    replayed_transitions: u64,
}

impl LatentWorldEvidenceArtifact {
    #[must_use]
    pub const fn schema(&self) -> &LatentMechanismSchema {
        &self.schema
    }

    #[must_use]
    pub const fn experience(&self) -> Hash32 {
        self.experience
    }

    fn from_receipts(
        schema: &LatentMechanismSchema,
        compilation: &VerifiedLatentSchemaCompilation,
        transfer: &VerifiedLatentSchemaTransfer,
    ) -> Result<Self, LatentWorldArchiveError> {
        if schema.semantic_hash() != compilation.schema()
            || schema.semantic_hash() != transfer.schema()
            || compilation.evidence() != transfer.experience()
        {
            return Err(LatentWorldArchiveError::IdentityMismatch);
        }
        Ok(Self {
            schema: schema.clone(),
            experience: transfer.experience(),
            replayed_rules: compilation.replayed_rules(),
            replayed_transitions: transfer.replayed_transitions(),
        })
    }

    fn from_value(value: CanonicalValue) -> Result<Self, LatentWorldArchiveError> {
        let CanonicalValue::Array(fields) = value else {
            return Err(LatentWorldArchiveError::InvalidPayload);
        };
        let [marker, schema, experience, rules, transitions] = fields.as_slice() else {
            return Err(LatentWorldArchiveError::InvalidPayload);
        };
        if marker != &CanonicalValue::Text("cairn.memory.latent-world-evidence.v1".to_owned()) {
            return Err(LatentWorldArchiveError::InvalidPayload);
        }
        let CanonicalValue::Bytes(schema_bytes) = schema else {
            return Err(LatentWorldArchiveError::InvalidPayload);
        };
        let schema = LatentMechanismSchema::decode_canonical(
            schema_bytes,
            CanonicalLimits {
                max_input_bytes: 1_048_576,
                max_depth: 32,
                max_collection_items: 65_536,
                max_scalar_bytes: 1_048_576,
            },
        )
        .map_err(LatentWorldArchiveError::Schema)?;
        Ok(Self {
            schema,
            experience: latent_hash(experience)?,
            replayed_rules: latent_u64(rules)?,
            replayed_transitions: latent_u64(transitions)?,
        })
    }
}

impl CanonicalEncode for LatentWorldEvidenceArtifact {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.memory.latent-world-evidence.v1".to_owned()),
            CanonicalValue::Bytes(self.schema.encode_canonical()?),
            CanonicalValue::Bytes(self.experience.as_bytes().to_vec()),
            CanonicalValue::Unsigned(self.replayed_rules),
            CanonicalValue::Unsigned(self.replayed_transitions),
        ])
        .encode_canonical()
    }
}

/// Current-process evidence required to reconstruct verifier authority after
/// loading an untrusted latent-world commitment.
#[derive(Clone, Copy)]
pub struct LatentWorldReplay<'a> {
    pub theory: &'a LatentTransitionTheory,
    pub space: &'a StateSpace,
    pub actions: &'a [ActionId],
    pub episodes: &'a [LatentEpisode],
    pub binding: &'a LatentSchemaBinding,
    pub maximum_latent_permutations: u64,
}

/// File-backed CAS boundary for restart-safe latent-world evidence.
#[derive(Clone, Copy, Debug, Default)]
pub struct LatentWorldEvidenceArchive;

impl LatentWorldEvidenceArchive {
    /// Persist one independently compiled and transferred schema commitment.
    ///
    /// # Errors
    ///
    /// Rejects missing parent lineage, receipt disagreement, canonicalization,
    /// and storage failures.
    pub fn persist(
        &self,
        cas: &Cas,
        schema: &LatentMechanismSchema,
        compilation: &VerifiedLatentSchemaCompilation,
        transfer: &VerifiedLatentSchemaTransfer,
        metadata: ReuseArtifactMetadata,
    ) -> Result<StoredArtifact, LatentWorldArchiveError> {
        if metadata.parent_ids.is_empty() {
            return Err(LatentWorldArchiveError::MissingParent);
        }
        let payload = LatentWorldEvidenceArtifact::from_receipts(schema, compilation, transfer)?;
        let envelope = ArtifactEnvelope::new(
            latent_world_schema()?,
            latent_world_kind()?,
            latent_world_semantics(),
            metadata.parent_ids,
            Some(payload.experience),
            metadata.producer,
            metadata.budget,
            metadata.created_at_logical,
            payload,
        )?;
        cas.put_envelope(&envelope)
            .map_err(LatentWorldArchiveError::Storage)
    }

    /// Load a structurally validated but still untrusted evidence commitment.
    ///
    /// # Errors
    ///
    /// Rejects corrupt storage, unknown schema/semantics, missing lineage,
    /// malformed payload, or task-scope mismatch.
    pub fn load(
        &self,
        cas: &Cas,
        blob_id: BlobId,
    ) -> Result<LatentWorldEvidenceArtifact, LatentWorldArchiveError> {
        let mut registry = SchemaRegistry::new();
        registry.register(SchemaDescriptor {
            name: LATENT_WORLD_EVIDENCE_SCHEMA.to_owned(),
            current_version: LATENT_WORLD_EVIDENCE_SCHEMA_VERSION,
            additive_read_versions: BTreeSet::new(),
            maximum_payload_bytes: 1_048_576,
        })?;
        let envelope = cas.get_envelope(
            blob_id,
            &registry,
            &BTreeSet::from([latent_world_semantics()]),
            CanonicalLimits {
                max_input_bytes: 2_097_152,
                max_depth: 48,
                max_collection_items: 131_072,
                max_scalar_bytes: 1_048_576,
            },
        )?;
        if envelope.artifact_kind != latent_world_kind()? || envelope.parent_ids.is_empty() {
            return Err(LatentWorldArchiveError::InvalidEnvelope);
        }
        let artifact = LatentWorldEvidenceArtifact::from_value(envelope.payload)?;
        if envelope.task_scope != Some(artifact.experience) {
            return Err(LatentWorldArchiveError::InvalidEnvelope);
        }
        Ok(artifact)
    }

    /// Independently reconstruct compilation and current transfer receipts and
    /// require every identity and replay count to equal the loaded commitment.
    ///
    /// # Errors
    ///
    /// Rejects changed theories, episodes, action bindings, schema identity,
    /// or any verifier/replay disagreement.
    pub fn replay(
        &self,
        artifact: &LatentWorldEvidenceArtifact,
        evidence: LatentWorldReplay<'_>,
    ) -> Result<
        (
            VerifiedLatentSchemaCompilation,
            VerifiedLatentSchemaTransfer,
        ),
        LatentWorldArchiveError,
    > {
        let transfer = LatentSchemaVerifier
            .verify_transfer(
                &artifact.schema,
                evidence.space,
                evidence.actions,
                evidence.episodes,
                evidence.binding,
            )
            .map_err(LatentWorldArchiveError::Verification)?;
        let compilation = LatentSchemaVerifier
            .verify_compilation(
                evidence.theory,
                &artifact.schema,
                transfer.experience(),
                evidence.maximum_latent_permutations,
            )
            .map_err(LatentWorldArchiveError::Verification)?;
        if artifact.experience != transfer.experience()
            || artifact.replayed_rules != compilation.replayed_rules()
            || artifact.replayed_transitions != transfer.replayed_transitions()
        {
            return Err(LatentWorldArchiveError::ReplayMismatch);
        }
        Ok((compilation, transfer))
    }
}

#[derive(Debug)]
pub enum LatentWorldArchiveError {
    MissingParent,
    InvalidEnvelope,
    InvalidPayload,
    IdentityMismatch,
    ReplayMismatch,
    Verification(LatentSchemaVerifyError),
    Schema(LatentSchemaError),
    Artifact(ArtifactError),
    Storage(CasError),
}

impl fmt::Display for LatentWorldArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for LatentWorldArchiveError {}

impl From<ArtifactError> for LatentWorldArchiveError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<CasError> for LatentWorldArchiveError {
    fn from(value: CasError) -> Self {
        Self::Storage(value)
    }
}

fn latent_world_schema() -> Result<SchemaRef, ArtifactError> {
    SchemaRef::new(
        LATENT_WORLD_EVIDENCE_SCHEMA,
        LATENT_WORLD_EVIDENCE_SCHEMA_VERSION,
    )
}

fn latent_world_kind() -> Result<ArtifactKind, ArtifactError> {
    ArtifactKind::new("memory.latent-world-evidence")
}

const fn latent_world_semantics() -> SemanticsVersion {
    SemanticsVersion::new(1, 0, 0)
}

fn latent_hash(value: &CanonicalValue) -> Result<Hash32, LatentWorldArchiveError> {
    let CanonicalValue::Bytes(bytes) = value else {
        return Err(LatentWorldArchiveError::InvalidPayload);
    };
    let fixed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| LatentWorldArchiveError::InvalidPayload)?;
    Ok(Hash32::from_bytes(fixed))
}

fn latent_u64(value: &CanonicalValue) -> Result<u64, LatentWorldArchiveError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(LatentWorldArchiveError::InvalidPayload);
    };
    Ok(*value)
}
