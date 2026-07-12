use crate::ModuleStatus;
use cairn_core::Hash32;
use cairn_theory::EntityConstructionProgram;
use cairn_verifier::VerifiedEntityConstructionProgram;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConstructionProgramSupportGeometry {
    pub observed_dimensions: BTreeSet<usize>,
    pub observed_extents: BTreeSet<Vec<u32>>,
    pub observed_entity_counts: BTreeSet<usize>,
    pub observed_point_cardinalities: BTreeSet<Vec<usize>>,
    pub observed_values: BTreeSet<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstructionProgramModule {
    id: Hash32,
    program: EntityConstructionProgram,
    cohorts: BTreeMap<Hash32, VerifiedEntityConstructionProgram>,
    experiences: BTreeSet<Hash32>,
    support: ConstructionProgramSupportGeometry,
    status: ModuleStatus,
}

impl ConstructionProgramModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }
    #[must_use]
    pub const fn program(&self) -> &EntityConstructionProgram {
        &self.program
    }
    #[must_use]
    pub const fn support(&self) -> &ConstructionProgramSupportGeometry {
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
pub enum ConstructionProgramPromotion {
    Provisional { verified_cohorts: usize },
    Persistent { verified_cohorts: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructionProgramStoreError {
    Canonicalization,
    ProgramMismatch,
    EmptyReplay,
    DuplicateCohort,
    OverlappingExperience,
}

/// Proof-gated persistent library of synthesized executable programs.
#[derive(Clone, Debug, Default)]
pub struct ConstructionProgramStore {
    modules: BTreeMap<Hash32, ConstructionProgramModule>,
}

impl ConstructionProgramStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Record one independently replayed cohort; two disjoint cohorts are
    /// required for persistence.
    ///
    /// # Errors
    ///
    /// Rejects identity mismatch, empty replay, duplicate, and overlap.
    pub fn record_verified_cohort(
        &mut self,
        program: &EntityConstructionProgram,
        receipt: VerifiedEntityConstructionProgram,
    ) -> Result<ConstructionProgramPromotion, ConstructionProgramStoreError> {
        let id = program
            .semantic_hash()
            .map_err(|_| ConstructionProgramStoreError::Canonicalization)?;
        if receipt.program() != id
            || receipt.replayed_entities() == 0
            || receipt.replayed_points() == 0
        {
            return Err(ConstructionProgramStoreError::EmptyReplay);
        }
        let cohort = cohort_hash(receipt.experiences());
        let module = self
            .modules
            .entry(id)
            .or_insert_with(|| ConstructionProgramModule {
                id,
                program: program.clone(),
                cohorts: BTreeMap::new(),
                experiences: BTreeSet::new(),
                support: ConstructionProgramSupportGeometry::default(),
                status: ModuleStatus::Provisional,
            });
        if module.program.semantic_hash().ok() != Some(id) {
            return Err(ConstructionProgramStoreError::ProgramMismatch);
        }
        if module.cohorts.contains_key(&cohort) {
            return Err(ConstructionProgramStoreError::DuplicateCohort);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(ConstructionProgramStoreError::OverlappingExperience);
        }
        module
            .support
            .observed_dimensions
            .extend(receipt.observed_dimensions());
        module
            .support
            .observed_extents
            .extend(receipt.observed_extents().iter().cloned());
        module
            .support
            .observed_entity_counts
            .extend(receipt.observed_entity_counts());
        module
            .support
            .observed_point_cardinalities
            .extend(receipt.observed_point_cardinalities().iter().cloned());
        module
            .support
            .observed_values
            .extend(receipt.observed_values());
        module.experiences.extend(receipt.experiences());
        module.cohorts.insert(cohort, receipt);
        let count = module.cohorts.len();
        if count >= 2 {
            module.status = ModuleStatus::Persistent;
            Ok(ConstructionProgramPromotion::Persistent {
                verified_cohorts: count,
            })
        } else {
            Ok(ConstructionProgramPromotion::Provisional {
                verified_cohorts: count,
            })
        }
    }

    pub fn persistent_programs(&self) -> impl Iterator<Item = &EntityConstructionProgram> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
            .map(|module| &module.program)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&ConstructionProgramModule> {
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
