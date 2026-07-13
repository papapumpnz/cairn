use crate::ModuleStatus;
use cairn_core::Hash32;
use cairn_theory::LearnedExpansionMechanism;
use cairn_verifier::VerifiedExpansionMechanism;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct ExpansionModule {
    id: Hash32,
    mechanism: LearnedExpansionMechanism,
    cohorts: BTreeMap<Hash32, VerifiedExpansionMechanism>,
    experiences: BTreeSet<Hash32>,
    status: ModuleStatus,
}

impl ExpansionModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }
    #[must_use]
    pub const fn mechanism(&self) -> &LearnedExpansionMechanism {
        &self.mechanism
    }
    #[must_use]
    pub const fn status(&self) -> ModuleStatus {
        self.status
    }
    #[must_use]
    pub fn verified_cohorts(&self) -> usize {
        self.cohorts.len()
    }
    #[must_use]
    pub fn verified_experiences(&self) -> usize {
        self.experiences.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpansionPromotion {
    Provisional { verified_cohorts: usize },
    Persistent { verified_cohorts: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpansionStoreError {
    Canonicalization,
    IdentityMismatch,
    EmptyReplay,
    DuplicateCohort,
    OverlappingExperience,
}

#[derive(Clone, Debug, Default)]
pub struct ExpansionStore {
    modules: BTreeMap<Hash32, ExpansionModule>,
}

impl ExpansionStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Record one independent exact cohort; two disjoint cohorts promote.
    ///
    /// # Errors
    ///
    /// Rejects identity mismatch, empty, duplicate, and overlapping evidence.
    pub fn record_verified_cohort(
        &mut self,
        mechanism: &LearnedExpansionMechanism,
        receipt: VerifiedExpansionMechanism,
    ) -> Result<ExpansionPromotion, ExpansionStoreError> {
        let id = mechanism
            .semantic_hash()
            .map_err(|_| ExpansionStoreError::Canonicalization)?;
        if receipt.mechanism() != id
            || receipt.replayed_inputs() == 0
            || receipt.replayed_outputs() == 0
        {
            return Err(ExpansionStoreError::EmptyReplay);
        }
        let cohort = Hash32::digest(
            &receipt
                .experiences()
                .iter()
                .flat_map(|item| item.as_bytes().iter().copied())
                .collect::<Vec<_>>(),
        );
        let module = self.modules.entry(id).or_insert_with(|| ExpansionModule {
            id,
            mechanism: mechanism.clone(),
            cohorts: BTreeMap::new(),
            experiences: BTreeSet::new(),
            status: ModuleStatus::Provisional,
        });
        if module.mechanism.semantic_hash().ok() != Some(id) {
            return Err(ExpansionStoreError::IdentityMismatch);
        }
        if module.cohorts.contains_key(&cohort) {
            return Err(ExpansionStoreError::DuplicateCohort);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(ExpansionStoreError::OverlappingExperience);
        }
        module.experiences.extend(receipt.experiences());
        module.cohorts.insert(cohort, receipt);
        let count = module.cohorts.len();
        if count >= 2 {
            module.status = ModuleStatus::Persistent;
            Ok(ExpansionPromotion::Persistent {
                verified_cohorts: count,
            })
        } else {
            Ok(ExpansionPromotion::Provisional {
                verified_cohorts: count,
            })
        }
    }

    pub fn persistent_mechanisms(&self) -> impl Iterator<Item = &LearnedExpansionMechanism> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
            .map(|module| &module.mechanism)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&ExpansionModule> {
        self.modules.get(&id)
    }
}
