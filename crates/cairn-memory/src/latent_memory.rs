use crate::ModuleStatus;
use cairn_core::Hash32;
use cairn_verifier::{VerifiedLatentSchemaCompilation, VerifiedLatentSchemaTransfer};
use cairn_world::LatentMechanismSchema;
use std::collections::BTreeMap;

const REQUIRED_INDEPENDENT_WORLDS: usize = 2;

/// One source-independent hidden-world mechanism and its proof-gated,
/// independently reproduced world evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentMechanismModule {
    schema: LatentMechanismSchema,
    validations: BTreeMap<Hash32, LatentWorldValidation>,
    status: ModuleStatus,
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
