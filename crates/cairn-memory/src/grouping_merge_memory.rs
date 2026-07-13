use crate::ModuleStatus;
use cairn_core::Hash32;
use cairn_theory::LearnedGroupingMerge;
use cairn_verifier::{VerifiedGroupingPromotion, VerifiedGroupingReuse};
use std::collections::{BTreeMap, BTreeSet};

/// One proposal-only relation-bound mechanism and its verifier evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupingMergeModule {
    id: Hash32,
    mechanism: LearnedGroupingMerge,
    receipts: BTreeMap<Hash32, VerifiedGroupingReuse>,
    experiences: BTreeSet<Hash32>,
    status: ModuleStatus,
}

impl GroupingMergeModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }
    #[must_use]
    pub const fn mechanism(&self) -> &LearnedGroupingMerge {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupingMergeStoreError {
    Canonicalization,
    IdentityMismatch,
    UnknownModule,
    EmptyReplay,
    DuplicateReceipt,
    OverlappingExperience,
}

/// Proof-gated memory for a selected grouping plus executable merge rule.
#[derive(Clone, Debug, Default)]
pub struct GroupingMergeStore {
    modules: BTreeMap<Hash32, GroupingMergeModule>,
}

impl GroupingMergeStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Insert one task-local learned composition as a provisional proposal.
    ///
    /// # Errors
    ///
    /// Rejects canonicalization failure or an identity collision.
    pub fn insert_provisional(
        &mut self,
        mechanism: LearnedGroupingMerge,
    ) -> Result<Hash32, GroupingMergeStoreError> {
        let id = mechanism
            .semantic_hash()
            .map_err(|_| GroupingMergeStoreError::Canonicalization)?;
        if let Some(existing) = self.modules.get(&id) {
            if existing.mechanism.semantic_hash().ok() != Some(id) {
                return Err(GroupingMergeStoreError::IdentityMismatch);
            }
            return Ok(id);
        }
        self.modules.insert(
            id,
            GroupingMergeModule {
                id,
                mechanism,
                receipts: BTreeMap::new(),
                experiences: BTreeSet::new(),
                status: ModuleStatus::Provisional,
            },
        );
        Ok(id)
    }

    /// Record an opaque independent-verifier reuse receipt.
    ///
    /// # Errors
    ///
    /// Rejects unknown identity, empty replay, duplicate receipt, or reused
    /// experience evidence.
    pub fn record_verified_reuse(
        &mut self,
        id: Hash32,
        receipt: VerifiedGroupingReuse,
    ) -> Result<(), GroupingMergeStoreError> {
        let module = self
            .modules
            .get_mut(&id)
            .ok_or(GroupingMergeStoreError::UnknownModule)?;
        if receipt.mechanism() != id {
            return Err(GroupingMergeStoreError::IdentityMismatch);
        }
        if receipt.replayed_groups() == 0 || receipt.experiences().is_empty() {
            return Err(GroupingMergeStoreError::EmptyReplay);
        }
        let receipt_id = Hash32::digest(
            &receipt
                .experiences()
                .iter()
                .flat_map(|experience| experience.as_bytes().iter().copied())
                .collect::<Vec<_>>(),
        );
        if module.receipts.contains_key(&receipt_id) {
            return Err(GroupingMergeStoreError::DuplicateReceipt);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(GroupingMergeStoreError::OverlappingExperience);
        }
        module.experiences.extend(receipt.experiences());
        module.receipts.insert(receipt_id, receipt);
        Ok(())
    }

    /// Apply only a consolidator-issued certificate covering stored evidence.
    #[must_use]
    pub fn apply_verified_promotion(
        &mut self,
        id: Hash32,
        promotion: &VerifiedGroupingPromotion,
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

    pub fn persistent_mechanisms(&self) -> impl Iterator<Item = &LearnedGroupingMerge> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
            .map(|module| &module.mechanism)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&GroupingMergeModule> {
        self.modules.get(&id)
    }
}
