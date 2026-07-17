//! Proposal-only memory for independently verified scalar expressions.
//!
//! Stored expressions never carry source-channel bindings and cannot emit an
//! accepted value. Runtime users must rebind them to current observations and
//! pass any resulting proposal through the current task's verifier boundary.

use crate::ModuleStatus;
use cairn_core::Hash32;
use cairn_theory::ScalarExpressionMechanism;
use cairn_verifier::{VerifiedScalarExpressionPromotion, VerifiedScalarExpressionReuse};
use std::collections::{BTreeMap, BTreeSet};

/// One source-independent scalar expression and its verifier-issued evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarExpressionModule {
    id: Hash32,
    mechanism: ScalarExpressionMechanism,
    receipts: BTreeMap<Hash32, VerifiedScalarExpressionReuse>,
    experiences: BTreeSet<Hash32>,
    status: ModuleStatus,
}

impl ScalarExpressionModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }

    #[must_use]
    pub const fn mechanism(&self) -> &ScalarExpressionMechanism {
        &self.mechanism
    }

    #[must_use]
    pub const fn status(&self) -> ModuleStatus {
        self.status
    }

    #[must_use]
    pub fn verified_reuses(&self) -> usize {
        self.receipts.len()
    }
}

/// Fail-closed scalar-expression memory terminal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarExpressionStoreError {
    Canonicalization,
    IdentityMismatch,
    UnknownModule,
    EmptyReplay,
    DuplicateReceipt,
    OverlappingExperience,
}

/// Exact, deterministic proposal memory keyed by canonical mechanism identity.
#[derive(Clone, Debug, Default)]
pub struct ScalarExpressionStore {
    modules: BTreeMap<Hash32, ScalarExpressionModule>,
}

impl ScalarExpressionStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Insert a source-independent expression without granting persistence.
    ///
    /// # Errors
    ///
    /// Rejects malformed expression/definition shape or canonical identity
    /// failure.
    pub fn insert_provisional(
        &mut self,
        mechanism: ScalarExpressionMechanism,
    ) -> Result<Hash32, ScalarExpressionStoreError> {
        let arity = usize::try_from(mechanism.expression.parameter_count())
            .map_err(|_| ScalarExpressionStoreError::IdentityMismatch)?;
        if arity < 2
            || mechanism.expression.binary_node_count() == 0
            || mechanism.definition.parameters.len() != arity
        {
            return Err(ScalarExpressionStoreError::IdentityMismatch);
        }
        let id = mechanism
            .semantic_hash()
            .map_err(|_| ScalarExpressionStoreError::Canonicalization)?;
        if let Some(existing) = self.modules.get(&id) {
            if existing.mechanism.semantic_hash().ok() != Some(id) {
                return Err(ScalarExpressionStoreError::IdentityMismatch);
            }
            return Ok(id);
        }
        self.modules.insert(
            id,
            ScalarExpressionModule {
                id,
                mechanism,
                receipts: BTreeMap::new(),
                experiences: BTreeSet::new(),
                status: ModuleStatus::Provisional,
            },
        );
        Ok(id)
    }

    /// Record one independently replayed, disjoint reuse cohort.
    ///
    /// # Errors
    ///
    /// Rejects mismatched identity, empty evidence, duplicate receipts, and
    /// overlapping experiences.
    pub fn record_verified_reuse(
        &mut self,
        id: Hash32,
        receipt: VerifiedScalarExpressionReuse,
    ) -> Result<(), ScalarExpressionStoreError> {
        let module = self
            .modules
            .get_mut(&id)
            .ok_or(ScalarExpressionStoreError::UnknownModule)?;
        if receipt.mechanism() != id {
            return Err(ScalarExpressionStoreError::IdentityMismatch);
        }
        if receipt.replayed_examples() == 0 || receipt.experiences().is_empty() {
            return Err(ScalarExpressionStoreError::EmptyReplay);
        }
        let receipt_id = Hash32::digest(
            &receipt
                .experiences()
                .iter()
                .flat_map(|experience| experience.as_bytes().iter().copied())
                .collect::<Vec<_>>(),
        );
        if module.receipts.contains_key(&receipt_id) {
            return Err(ScalarExpressionStoreError::DuplicateReceipt);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(ScalarExpressionStoreError::OverlappingExperience);
        }
        module.experiences.extend(receipt.experiences());
        module.receipts.insert(receipt_id, receipt);
        Ok(())
    }

    /// Apply a consolidator certificate only when it covers the complete local
    /// ledger and at least two independently scoped reuse cohorts.
    #[must_use]
    pub fn apply_verified_promotion(
        &mut self,
        id: Hash32,
        promotion: &VerifiedScalarExpressionPromotion,
    ) -> bool {
        let Some(module) = self.modules.get_mut(&id) else {
            return false;
        };
        if module.status != ModuleStatus::Provisional
            || promotion.mechanism() != id
            || promotion.experiences() != &module.experiences
            || usize::try_from(promotion.verified_reuses()).ok() != Some(module.receipts.len())
            || promotion.verified_reuses() < 2
        {
            return false;
        }
        module.status = ModuleStatus::Persistent;
        true
    }

    /// Iterate only mechanisms admitted by the independent promotion path.
    pub fn persistent_modules(&self) -> impl Iterator<Item = &ScalarExpressionModule> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&ScalarExpressionModule> {
        self.modules.get(&id)
    }
}
