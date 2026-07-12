use crate::ModuleStatus;
use cairn_core::Hash32;
use cairn_theory::LayerMechanism;
use cairn_verifier::VerifiedLayerMechanism;
use std::collections::{BTreeMap, BTreeSet};

const REQUIRED_LAYER_COHORTS: usize = 2;

/// Explicit empirical boundary attached to a promoted layer mechanism. These
/// observations are evidence, not a claim of universal applicability.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LayerSupportGeometry {
    pub observed_dimensions: BTreeSet<usize>,
    pub observed_extents: BTreeSet<Vec<u32>>,
    pub observed_cardinalities: BTreeSet<Vec<usize>>,
    pub observed_backgrounds: BTreeSet<i64>,
    pub observed_foregrounds: BTreeSet<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerMechanismModule {
    id: Hash32,
    mechanism: LayerMechanism,
    cohorts: BTreeMap<Hash32, VerifiedLayerMechanism>,
    experiences: BTreeSet<Hash32>,
    support: LayerSupportGeometry,
    status: ModuleStatus,
}

impl LayerMechanismModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }

    #[must_use]
    pub const fn mechanism(&self) -> &LayerMechanism {
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

    #[must_use]
    pub const fn support(&self) -> &LayerSupportGeometry {
        &self.support
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayerPromotionProgress {
    Provisional { verified_cohorts: usize },
    Persistent { verified_cohorts: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayerStoreError {
    Canonicalization,
    MechanismMismatch,
    EmptyReplay,
    DuplicateCohort,
    OverlappingExperience,
}

/// Proof-gated memory for learned coordinate-layer construction and rendering
/// mechanisms. Persistent entries remain proposal sources only.
#[derive(Clone, Debug, Default)]
pub struct LayerMechanismStore {
    modules: BTreeMap<Hash32, LayerMechanismModule>,
}

impl LayerMechanismStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    /// Record one independently replayed cohort. Two disjoint cohorts are
    /// required before automatic persistence.
    ///
    /// # Errors
    ///
    /// Rejects identity disagreement, empty replay, duplicate cohort, and
    /// overlapping experience evidence.
    pub fn record_verified_cohort(
        &mut self,
        mechanism: &LayerMechanism,
        receipt: VerifiedLayerMechanism,
    ) -> Result<LayerPromotionProgress, LayerStoreError> {
        let id = mechanism
            .semantic_hash()
            .map_err(|_| LayerStoreError::Canonicalization)?;
        if receipt.mechanism() != id || receipt.replayed_points() == 0 {
            return Err(LayerStoreError::MechanismMismatch);
        }
        let cohort = cohort_hash(receipt.experiences());
        let module = self
            .modules
            .entry(id)
            .or_insert_with(|| LayerMechanismModule {
                id,
                mechanism: mechanism.clone(),
                cohorts: BTreeMap::new(),
                experiences: BTreeSet::new(),
                support: LayerSupportGeometry::default(),
                status: ModuleStatus::Provisional,
            });
        if module.mechanism != *mechanism {
            return Err(LayerStoreError::MechanismMismatch);
        }
        if module.cohorts.contains_key(&cohort) {
            return Err(LayerStoreError::DuplicateCohort);
        }
        if !module.experiences.is_disjoint(receipt.experiences()) {
            return Err(LayerStoreError::OverlappingExperience);
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
            .observed_cardinalities
            .extend(receipt.observed_cardinalities().iter().cloned());
        module
            .support
            .observed_backgrounds
            .extend(receipt.observed_backgrounds());
        module
            .support
            .observed_foregrounds
            .extend(receipt.observed_foregrounds());
        module.experiences.extend(receipt.experiences());
        module.cohorts.insert(cohort, receipt);
        let count = module.cohorts.len();
        if count >= REQUIRED_LAYER_COHORTS {
            module.status = ModuleStatus::Persistent;
            Ok(LayerPromotionProgress::Persistent {
                verified_cohorts: count,
            })
        } else {
            Ok(LayerPromotionProgress::Provisional {
                verified_cohorts: count,
            })
        }
    }

    pub fn persistent_mechanisms(&self) -> impl Iterator<Item = &LayerMechanism> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Persistent)
            .map(|module| &module.mechanism)
    }

    #[must_use]
    pub fn module(&self, id: Hash32) -> Option<&LayerMechanismModule> {
        self.modules.get(&id)
    }
}

fn cohort_hash(experiences: &BTreeSet<Hash32>) -> Hash32 {
    let bytes = experiences
        .iter()
        .flat_map(|experience| experience.as_bytes().iter().copied())
        .collect::<Vec<_>>();
    Hash32::digest(&bytes)
}
