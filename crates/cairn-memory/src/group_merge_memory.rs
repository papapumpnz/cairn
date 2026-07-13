use crate::ModuleStatus;
use cairn_core::Hash32;
use cairn_theory::LearnedGroupMerge;
use cairn_verifier::VerifiedGroupMerge;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct GroupMergeModule {
    id: Hash32,
    mechanism: LearnedGroupMerge,
    cohorts: BTreeMap<Hash32, VerifiedGroupMerge>,
    experiences: BTreeSet<Hash32>,
    status: ModuleStatus,
}

impl GroupMergeModule {
    #[must_use]
    pub const fn mechanism(&self) -> &LearnedGroupMerge {
        &self.mechanism
    }
    #[must_use]
    pub const fn status(&self) -> ModuleStatus {
        self.status
    }
    #[must_use]
    pub fn verified_experiences(&self) -> usize {
        self.experiences.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupMergePromotion {
    Provisional { verified_cohorts: usize },
    Persistent { verified_cohorts: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupMergeStoreError {
    Canonicalization,
    IdentityMismatch,
    EmptyReplay,
    DuplicateCohort,
    OverlappingExperience,
}

#[derive(Clone, Debug, Default)]
pub struct GroupMergeStore {
    modules: BTreeMap<Hash32, GroupMergeModule>,
}

impl GroupMergeStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Record an independently replayed cohort; two disjoint cohorts promote.
    ///
    /// # Errors
    ///
    /// Rejects identity mismatch, empty, duplicate, or overlapping evidence.
    pub fn record_verified_cohort(
        &mut self,
        mechanism: &LearnedGroupMerge,
        receipt: VerifiedGroupMerge,
    ) -> Result<GroupMergePromotion, GroupMergeStoreError> {
        let id = mechanism
            .semantic_hash()
            .map_err(|_| GroupMergeStoreError::Canonicalization)?;
        if receipt.mechanism() != id || receipt.replayed_groups() == 0 {
            return Err(GroupMergeStoreError::EmptyReplay);
        }
        let cohort = Hash32::digest(
            &receipt
                .experiences()
                .iter()
                .flat_map(|item| item.as_bytes().iter().copied())
                .collect::<Vec<_>>(),
        );
        let module = self.modules.entry(id).or_insert_with(|| GroupMergeModule {
            id,
            mechanism: mechanism.clone(),
            cohorts: BTreeMap::new(),
            experiences: BTreeSet::new(),
            status: ModuleStatus::Provisional,
        });
        if module.mechanism.semantic_hash().ok() != Some(id) {
            return Err(GroupMergeStoreError::IdentityMismatch);
        }
        if module.cohorts.contains_key(&cohort) {
            return Err(GroupMergeStoreError::DuplicateCohort);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(GroupMergeStoreError::OverlappingExperience);
        }
        module.experiences.extend(receipt.experiences());
        module.cohorts.insert(cohort, receipt);
        let count = module.cohorts.len();
        if count >= 2 {
            module.status = ModuleStatus::Persistent;
            Ok(GroupMergePromotion::Persistent {
                verified_cohorts: count,
            })
        } else {
            Ok(GroupMergePromotion::Provisional {
                verified_cohorts: count,
            })
        }
    }

    pub fn persistent_mechanisms(&self) -> impl Iterator<Item = &LearnedGroupMerge> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
            .map(|module| &module.mechanism)
    }
    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&GroupMergeModule> {
        self.modules.get(&id)
    }
}
