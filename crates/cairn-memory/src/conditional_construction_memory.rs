use crate::ModuleStatus;
use cairn_core::Hash32;
use cairn_theory::ConditionalEntityConstructionProgram;
use cairn_verifier::VerifiedConditionalConstructionProgram;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConditionalConstructionSupportGeometry {
    pub input_cardinalities: BTreeSet<usize>,
    pub output_cardinalities: BTreeSet<usize>,
    pub relation_counts: BTreeSet<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConditionalConstructionModule {
    id: Hash32,
    program: ConditionalEntityConstructionProgram,
    cohorts: BTreeMap<Hash32, VerifiedConditionalConstructionProgram>,
    experiences: BTreeSet<Hash32>,
    support: ConditionalConstructionSupportGeometry,
    status: ModuleStatus,
}

impl ConditionalConstructionModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }
    #[must_use]
    pub const fn program(&self) -> &ConditionalEntityConstructionProgram {
        &self.program
    }
    #[must_use]
    pub const fn support(&self) -> &ConditionalConstructionSupportGeometry {
        &self.support
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
pub enum ConditionalConstructionPromotion {
    Provisional { verified_cohorts: usize },
    Persistent { verified_cohorts: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConditionalConstructionStoreError {
    Canonicalization,
    ProgramMismatch,
    EmptyReplay,
    DuplicateCohort,
    OverlappingExperience,
}

/// Proof-gated proposal library for conditional executable programs.
#[derive(Clone, Debug, Default)]
pub struct ConditionalConstructionStore {
    modules: BTreeMap<Hash32, ConditionalConstructionModule>,
}

impl ConditionalConstructionStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Record an independently replayed cohort. Two disjoint cohorts promote.
    ///
    /// # Errors
    ///
    /// Rejects identity mismatch, empty evidence, duplicate, or overlap.
    pub fn record_verified_cohort(
        &mut self,
        program: &ConditionalEntityConstructionProgram,
        receipt: VerifiedConditionalConstructionProgram,
    ) -> Result<ConditionalConstructionPromotion, ConditionalConstructionStoreError> {
        let id = program
            .semantic_hash()
            .map_err(|_| ConditionalConstructionStoreError::Canonicalization)?;
        if receipt.program() != id
            || receipt.replayed_inputs() == 0
            || receipt.replayed_outputs() == 0
        {
            return Err(ConditionalConstructionStoreError::EmptyReplay);
        }
        let cohort = cohort_hash(receipt.experiences());
        let module = self
            .modules
            .entry(id)
            .or_insert_with(|| ConditionalConstructionModule {
                id,
                program: program.clone(),
                cohorts: BTreeMap::new(),
                experiences: BTreeSet::new(),
                support: ConditionalConstructionSupportGeometry::default(),
                status: ModuleStatus::Provisional,
            });
        if module.program.semantic_hash().ok() != Some(id) {
            return Err(ConditionalConstructionStoreError::ProgramMismatch);
        }
        if module.cohorts.contains_key(&cohort) {
            return Err(ConditionalConstructionStoreError::DuplicateCohort);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(ConditionalConstructionStoreError::OverlappingExperience);
        }
        module
            .support
            .input_cardinalities
            .extend(receipt.observed_input_cardinalities());
        module
            .support
            .output_cardinalities
            .extend(receipt.observed_output_cardinalities());
        module
            .support
            .relation_counts
            .extend(receipt.observed_relation_counts());
        module.experiences.extend(receipt.experiences());
        module.cohorts.insert(cohort, receipt);
        let count = module.cohorts.len();
        if count >= 2 {
            module.status = ModuleStatus::Persistent;
            Ok(ConditionalConstructionPromotion::Persistent {
                verified_cohorts: count,
            })
        } else {
            Ok(ConditionalConstructionPromotion::Provisional {
                verified_cohorts: count,
            })
        }
    }

    pub fn persistent_programs(
        &self,
    ) -> impl Iterator<Item = &ConditionalEntityConstructionProgram> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
            .map(|module| &module.program)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&ConditionalConstructionModule> {
        self.modules.get(&id)
    }
}

fn cohort_hash(experiences: &BTreeSet<Hash32>) -> Hash32 {
    Hash32::digest(
        &experiences
            .iter()
            .flat_map(|experience| experience.as_bytes().iter().copied())
            .collect::<Vec<_>>(),
    )
}
