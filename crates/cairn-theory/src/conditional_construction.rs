use crate::{
    EntityConstruction, EntityConstructionProgramError, EntityConstructionProgramSynthesizer,
    EntityCorrespondenceExample, EntityFeature, EntityGeometryRender, EntityGeometryScene,
    EntitySelectionError, JointEntityConstructionProgram, LearnedEntitySelector,
    RelationalEntitySelectorInventor,
};
use cairn_core::{
    CanonicalEncode, CanonicalValue, EntityId, EntitySelectionDemonstration,
    EntitySelectionProblem, Hash32, RelationalEntityCollection,
};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use std::collections::BTreeSet;

/// One modality-neutral relational collection aligned with executable entity
/// geometry by content identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationalGeometryScene {
    pub geometry: EntityGeometryScene,
    pub relations: RelationalEntityCollection,
}

/// Unordered conditional-construction evidence.  No selected source IDs or
/// point correspondences are supplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalConstructionExample {
    pub source: RelationalGeometryScene,
    pub expected: EntityGeometryRender,
}

/// Executable composition of an invented relational guard and an invented
/// source-independent construction program.
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionalEntityConstructionProgram {
    pub selector: LearnedEntitySelector,
    pub construction: JointEntityConstructionProgram,
}

impl ConditionalEntityConstructionProgram {
    /// Return a canonical semantic identity excluding training statistics.
    ///
    /// # Errors
    ///
    /// Returns canonical encoding failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        self.encode_canonical().map(|bytes| Hash32::digest(&bytes))
    }
}

impl CanonicalEncode for ConditionalEntityConstructionProgram {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        let features = self
            .selector
            .features
            .iter()
            .map(|feature| match feature {
                EntityFeature::Attribute(id) => CanonicalValue::Array(vec![
                    CanonicalValue::Text("attribute".to_owned()),
                    CanonicalValue::Text(id.0.clone()),
                ]),
                EntityFeature::OutDegree(role) => CanonicalValue::Array(vec![
                    CanonicalValue::Text("out-degree".to_owned()),
                    CanonicalValue::Text(role.0.clone()),
                ]),
                EntityFeature::InDegree(role) => CanonicalValue::Array(vec![
                    CanonicalValue::Text("in-degree".to_owned()),
                    CanonicalValue::Text(role.0.clone()),
                ]),
                EntityFeature::TwoHopOutDegree(first, second) => CanonicalValue::Array(vec![
                    CanonicalValue::Text("two-hop-out-degree".to_owned()),
                    CanonicalValue::Text(first.0.clone()),
                    CanonicalValue::Text(second.0.clone()),
                ]),
            })
            .collect();
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.conditional-entity-construction.v1".to_owned()),
            CanonicalValue::Bytes(
                self.selector
                    .selector
                    .proposal
                    .definition
                    .encode_canonical()?,
            ),
            CanonicalValue::Array(features),
            CanonicalValue::Bytes(self.construction.encode_canonical()?),
        ])
        .encode_canonical()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConditionalConstructionReport {
    pub program: ConditionalEntityConstructionProgram,
    pub candidates_evaluated: u64,
    pub examples: usize,
    pub positive_bindings: usize,
    pub negative_bindings: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConditionalConstructionError {
    InsufficientEvidence,
    InvalidScene,
    AmbiguousPreservedAttribute,
    Capacity,
    Selection(EntitySelectionError),
    Construction(EntityConstructionProgramError),
    Prediction,
}

/// Reference online learner for conditional, changing-cardinality entity
/// construction.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConditionalConstructionSynthesizer;

impl ConditionalConstructionSynthesizer {
    /// Invent both the relational selector and the geometry program from
    /// unordered observations.
    ///
    /// # Errors
    ///
    /// Rejects malformed alignment, ambiguous preserved attributes, one-class
    /// evidence, bounded invention failure, and joint replay mismatch.
    pub fn synthesize(
        &self,
        examples: &[ConditionalConstructionExample],
        limits: InventionLimits,
    ) -> Result<ConditionalConstructionReport, ConditionalConstructionError> {
        if examples.len() < 2 {
            return Err(ConditionalConstructionError::InsufficientEvidence);
        }
        let mut demonstrations = Vec::with_capacity(examples.len());
        let mut filtered = Vec::with_capacity(examples.len());
        let mut positive_bindings = 0_usize;
        let mut negative_bindings = 0_usize;
        for example in examples {
            validate_alignment(&example.source)?;
            let expected_values = example
                .expected
                .entities
                .iter()
                .map(|entity| entity.value)
                .collect::<BTreeSet<_>>();
            if expected_values.len() != example.expected.entities.len() {
                return Err(ConditionalConstructionError::AmbiguousPreservedAttribute);
            }
            let mut selected = BTreeSet::new();
            let mut entities = Vec::new();
            for entity in &example.source.geometry.entities {
                if expected_values.contains(&entity.value) {
                    selected.insert(EntityId(entity.identity));
                    entities.push(entity.clone());
                    positive_bindings = positive_bindings
                        .checked_add(1)
                        .ok_or(ConditionalConstructionError::Capacity)?;
                } else {
                    negative_bindings = negative_bindings
                        .checked_add(1)
                        .ok_or(ConditionalConstructionError::Capacity)?;
                }
            }
            if selected.is_empty() || selected.len() != expected_values.len() {
                return Err(ConditionalConstructionError::AmbiguousPreservedAttribute);
            }
            demonstrations.push(EntitySelectionDemonstration {
                collection: example.source.relations.clone(),
                selected,
            });
            filtered.push(EntityCorrespondenceExample {
                source: EntityGeometryScene {
                    extents: example.source.geometry.extents.clone(),
                    background: example.source.geometry.background,
                    entities,
                },
                expected: example.expected.clone(),
            });
        }
        if positive_bindings == 0 || negative_bindings == 0 {
            return Err(ConditionalConstructionError::InsufficientEvidence);
        }
        let selector = RelationalEntitySelectorInventor
            .invent(
                &EntitySelectionProblem {
                    demonstrations,
                    prediction: examples[0].source.relations.clone(),
                },
                limits,
            )
            .map_err(ConditionalConstructionError::Selection)?;
        let construction = EntityConstructionProgramSynthesizer
            .synthesize_joint_unordered(&filtered, limits)
            .map_err(ConditionalConstructionError::Construction)?;
        let candidates_evaluated = selector
            .selector
            .proposal
            .candidates_evaluated
            .checked_add(construction.candidates_evaluated)
            .ok_or(ConditionalConstructionError::Capacity)?;
        let report = ConditionalConstructionReport {
            program: ConditionalEntityConstructionProgram {
                selector,
                construction: construction.program,
            },
            candidates_evaluated,
            examples: examples.len(),
            positive_bindings,
            negative_bindings,
        };
        for example in examples {
            if self
                .execute(&report.program, &example.source, ExecLimits::default())?
                .render
                != example.expected
            {
                return Err(ConditionalConstructionError::Prediction);
            }
        }
        Ok(report)
    }

    /// Rebind and execute a learned selector before executing learned geometry.
    ///
    /// # Errors
    ///
    /// Rejects malformed alignment, empty selection, selector execution, or
    /// construction failure.  The result remains an untrusted proposal.
    pub fn execute(
        &self,
        program: &ConditionalEntityConstructionProgram,
        source: &RelationalGeometryScene,
        limits: ExecLimits,
    ) -> Result<EntityConstruction, ConditionalConstructionError> {
        validate_alignment(source)?;
        let selected = RelationalEntitySelectorInventor
            .predict(&program.selector, &source.relations)
            .map_err(ConditionalConstructionError::Selection)?;
        if selected.is_empty() {
            return Err(ConditionalConstructionError::Prediction);
        }
        let geometry = EntityGeometryScene {
            extents: source.geometry.extents.clone(),
            background: source.geometry.background,
            entities: source
                .geometry
                .entities
                .iter()
                .filter(|entity| selected.contains(&EntityId(entity.identity)))
                .cloned()
                .collect(),
        };
        EntityConstructionProgramSynthesizer
            .execute_joint(&program.construction, &geometry, limits)
            .map_err(ConditionalConstructionError::Construction)
    }
}

fn validate_alignment(
    source: &RelationalGeometryScene,
) -> Result<(), ConditionalConstructionError> {
    let geometry = source
        .geometry
        .entities
        .iter()
        .map(|entity| EntityId(entity.identity))
        .collect::<BTreeSet<_>>();
    let relational = source
        .relations
        .entities
        .iter()
        .map(|entity| entity.id.clone())
        .collect::<BTreeSet<_>>();
    if geometry.is_empty()
        || geometry.len() != source.geometry.entities.len()
        || relational.len() != source.relations.entities.len()
        || geometry != relational
    {
        return Err(ConditionalConstructionError::InvalidScene);
    }
    Ok(())
}
