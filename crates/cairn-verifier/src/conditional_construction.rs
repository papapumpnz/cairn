use crate::construction_program::reference_execute_joint;
use cairn_core::{CanonicalEncode, CanonicalValue, EntityId, Hash32};
use cairn_rkl::{
    ExecLimits, ReferenceExecutor, RuntimeEnvironment, TraceLevel, TypeChecker, TypeEnvironment,
    Value,
};
use cairn_theory::{
    ConditionalConstructionExample, ConditionalEntityConstructionProgram, EntityFeature,
    EntityGeometryScene,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedConditionalConstructionProgram {
    program: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_inputs: u64,
    replayed_outputs: u64,
    observed_input_cardinalities: BTreeSet<usize>,
    observed_output_cardinalities: BTreeSet<usize>,
    observed_relation_counts: BTreeSet<usize>,
}

impl VerifiedConditionalConstructionProgram {
    #[must_use]
    pub const fn program(&self) -> Hash32 {
        self.program
    }
    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }
    #[must_use]
    pub const fn replayed_inputs(&self) -> u64 {
        self.replayed_inputs
    }
    #[must_use]
    pub const fn replayed_outputs(&self) -> u64 {
        self.replayed_outputs
    }
    #[must_use]
    pub const fn observed_input_cardinalities(&self) -> &BTreeSet<usize> {
        &self.observed_input_cardinalities
    }
    #[must_use]
    pub const fn observed_output_cardinalities(&self) -> &BTreeSet<usize> {
        &self.observed_output_cardinalities
    }
    #[must_use]
    pub const fn observed_relation_counts(&self) -> &BTreeSet<usize> {
        &self.observed_relation_counts
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConditionalConstructionVerifyError {
    InsufficientEvidence,
    DuplicateExperience,
    InvalidAlignment,
    InvalidSelector,
    EmptySelection,
    Construction,
    ReplayMismatch,
    Capacity,
    Canonicalization,
}

/// Independent exact replay boundary for conditional construction programs.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConditionalConstructionVerifier;

impl ConditionalConstructionVerifier {
    /// Recompute relational features, execute the raw selector definition, and
    /// execute raw construction definitions without calling either learner.
    ///
    /// # Errors
    ///
    /// Rejects malformed alignment, duplicate evidence, definition failure,
    /// empty selection, resource failure, and exact render disagreement.
    pub fn verify(
        &self,
        program: &ConditionalEntityConstructionProgram,
        examples: &[ConditionalConstructionExample],
        limits: ExecLimits,
    ) -> Result<VerifiedConditionalConstructionProgram, ConditionalConstructionVerifyError> {
        if examples.len() < 2 {
            return Err(ConditionalConstructionVerifyError::InsufficientEvidence);
        }
        let mut receipt = VerifiedConditionalConstructionProgram {
            program: program
                .semantic_hash()
                .map_err(|_| ConditionalConstructionVerifyError::Canonicalization)?,
            experiences: BTreeSet::new(),
            replayed_inputs: 0,
            replayed_outputs: 0,
            observed_input_cardinalities: BTreeSet::new(),
            observed_output_cardinalities: BTreeSet::new(),
            observed_relation_counts: BTreeSet::new(),
        };
        for example in examples {
            validate_alignment(example)?;
            if !receipt.experiences.insert(experience_hash(example)?) {
                return Err(ConditionalConstructionVerifyError::DuplicateExperience);
            }
            let selected = select(program, example)?;
            if selected.is_empty() {
                return Err(ConditionalConstructionVerifyError::EmptySelection);
            }
            let geometry = EntityGeometryScene {
                extents: example.source.geometry.extents.clone(),
                background: example.source.geometry.background,
                entities: example
                    .source
                    .geometry
                    .entities
                    .iter()
                    .filter(|entity| selected.contains(&EntityId(entity.identity)))
                    .cloned()
                    .collect(),
            };
            let render = reference_execute_joint(&program.construction, &geometry, limits)
                .map_err(|_| ConditionalConstructionVerifyError::Construction)?;
            if render != example.expected {
                return Err(ConditionalConstructionVerifyError::ReplayMismatch);
            }
            receipt.replayed_inputs = receipt
                .replayed_inputs
                .checked_add(
                    u64::try_from(example.source.geometry.entities.len())
                        .map_err(|_| ConditionalConstructionVerifyError::Capacity)?,
                )
                .ok_or(ConditionalConstructionVerifyError::Capacity)?;
            receipt.replayed_outputs = receipt
                .replayed_outputs
                .checked_add(
                    u64::try_from(example.expected.entities.len())
                        .map_err(|_| ConditionalConstructionVerifyError::Capacity)?,
                )
                .ok_or(ConditionalConstructionVerifyError::Capacity)?;
            receipt
                .observed_input_cardinalities
                .insert(example.source.geometry.entities.len());
            receipt
                .observed_output_cardinalities
                .insert(example.expected.entities.len());
            receipt
                .observed_relation_counts
                .insert(example.source.relations.relations.len());
        }
        Ok(receipt)
    }
}

fn select(
    program: &ConditionalEntityConstructionProgram,
    example: &ConditionalConstructionExample,
) -> Result<BTreeSet<EntityId>, ConditionalConstructionVerifyError> {
    let definition = &program.selector.selector.proposal.definition;
    let mut types = TypeEnvironment::new();
    for parameter in &definition.parameters {
        types.insert(parameter.name.clone(), parameter.ty.clone());
    }
    let checked = TypeChecker::new(10_000)
        .check(&definition.body, &types)
        .map_err(|_| ConditionalConstructionVerifyError::InvalidSelector)?;
    let mut selected = BTreeSet::new();
    for entity in &example.source.relations.entities {
        let values = feature_values(example, &entity.id, &program.selector.features);
        if values.len() != definition.parameters.len() {
            return Err(ConditionalConstructionVerifyError::InvalidSelector);
        }
        let mut inputs = RuntimeEnvironment::new();
        for (parameter, value) in definition.parameters.iter().zip(values) {
            inputs.insert(parameter.name.clone(), Value::Int(value));
        }
        match ReferenceExecutor::without_definitions(ExecLimits::default(), TraceLevel::Off)
            .execute(&checked, &inputs)
            .map_err(|_| ConditionalConstructionVerifyError::InvalidSelector)?
            .result
        {
            Value::Bool(true) => {
                selected.insert(entity.id.clone());
            }
            Value::Bool(false) => {}
            _ => return Err(ConditionalConstructionVerifyError::InvalidSelector),
        }
    }
    Ok(selected)
}

fn feature_values(
    example: &ConditionalConstructionExample,
    id: &EntityId,
    features: &[EntityFeature],
) -> Vec<i64> {
    let entity = example
        .source
        .relations
        .entities
        .iter()
        .find(|entity| &entity.id == id);
    features
        .iter()
        .map(|feature| match feature {
            EntityFeature::Attribute(feature) => entity
                .and_then(|entity| entity.attributes.get(feature))
                .copied()
                .unwrap_or(0),
            EntityFeature::OutDegree(role) => i64::try_from(
                example
                    .source
                    .relations
                    .relations
                    .iter()
                    .filter(|relation| relation.role == *role && relation.subject == *id)
                    .count(),
            )
            .unwrap_or(i64::MAX),
            EntityFeature::InDegree(role) => i64::try_from(
                example
                    .source
                    .relations
                    .relations
                    .iter()
                    .filter(|relation| relation.role == *role && relation.object == *id)
                    .count(),
            )
            .unwrap_or(i64::MAX),
            EntityFeature::TwoHopOutDegree(first, second) => i64::try_from(
                example
                    .source
                    .relations
                    .relations
                    .iter()
                    .filter(|relation| relation.role == *first && relation.subject == *id)
                    .flat_map(|relation| {
                        example
                            .source
                            .relations
                            .relations
                            .iter()
                            .filter_map(move |next| {
                                (next.role == *second && next.subject == relation.object)
                                    .then_some(next.object.clone())
                            })
                    })
                    .collect::<BTreeSet<_>>()
                    .len(),
            )
            .unwrap_or(i64::MAX),
        })
        .collect()
}

fn validate_alignment(
    example: &ConditionalConstructionExample,
) -> Result<(), ConditionalConstructionVerifyError> {
    let geometry = example
        .source
        .geometry
        .entities
        .iter()
        .map(|entity| EntityId(entity.identity))
        .collect::<BTreeSet<_>>();
    let relational = example
        .source
        .relations
        .entities
        .iter()
        .map(|entity| entity.id.clone())
        .collect::<BTreeSet<_>>();
    if geometry.is_empty()
        || geometry.len() != example.source.geometry.entities.len()
        || relational.len() != example.source.relations.entities.len()
        || geometry != relational
        || example.source.relations.relations.iter().any(|relation| {
            !relational.contains(&relation.subject) || !relational.contains(&relation.object)
        })
    {
        return Err(ConditionalConstructionVerifyError::InvalidAlignment);
    }
    Ok(())
}

fn experience_hash(
    example: &ConditionalConstructionExample,
) -> Result<Hash32, ConditionalConstructionVerifyError> {
    let mut relational_entities = example.source.relations.entities.iter().collect::<Vec<_>>();
    relational_entities.sort_by_key(|entity| &entity.id);
    let entities = relational_entities
        .into_iter()
        .map(|entity| {
            CanonicalValue::Array(vec![
                CanonicalValue::Bytes((entity.id.0).as_bytes().to_vec()),
                CanonicalValue::Array(
                    entity
                        .attributes
                        .iter()
                        .map(|(key, value)| {
                            CanonicalValue::Array(vec![
                                CanonicalValue::Text(key.0.clone()),
                                signed(*value),
                            ])
                        })
                        .collect(),
                ),
            ])
        })
        .collect::<Vec<_>>();
    let mut relation_records = example
        .source
        .relations
        .relations
        .iter()
        .collect::<Vec<_>>();
    relation_records.sort_by_key(|relation| (&relation.role, &relation.subject, &relation.object));
    let relations = relation_records
        .into_iter()
        .map(|relation| {
            CanonicalValue::Array(vec![
                CanonicalValue::Text((relation.role.0).clone()),
                CanonicalValue::Bytes((relation.subject.0).as_bytes().to_vec()),
                CanonicalValue::Bytes((relation.object.0).as_bytes().to_vec()),
            ])
        })
        .collect::<Vec<_>>();
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.conditional-construction-experience.v1".to_owned()),
        CanonicalValue::Array(entities),
        CanonicalValue::Array(relations),
        geometry_value(&example.source.geometry),
        render_value(&example.expected),
    ])
    .encode_canonical()
    .map(|bytes| Hash32::digest(&bytes))
    .map_err(|_| ConditionalConstructionVerifyError::Canonicalization)
}

fn signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}

fn geometry_value(scene: &cairn_theory::EntityGeometryScene) -> CanonicalValue {
    let mut entities = scene.entities.iter().collect::<Vec<_>>();
    entities.sort_by_key(|entity| entity.identity);
    CanonicalValue::Array(vec![
        unsigneds(&scene.extents),
        signed(scene.background),
        CanonicalValue::Array(
            entities
                .into_iter()
                .map(|entity| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Bytes(entity.identity.as_bytes().to_vec()),
                        signed(entity.value),
                        points(&entity.points),
                    ])
                })
                .collect(),
        ),
    ])
}

fn render_value(render: &cairn_theory::EntityGeometryRender) -> CanonicalValue {
    let mut entities = render.entities.iter().collect::<Vec<_>>();
    entities.sort_by_key(|entity| (entity.value, &entity.points));
    CanonicalValue::Array(vec![
        unsigneds(&render.extents),
        signed(render.background),
        CanonicalValue::Array(
            entities
                .into_iter()
                .map(|entity| {
                    CanonicalValue::Array(vec![signed(entity.value), points(&entity.points)])
                })
                .collect(),
        ),
    ])
}

fn unsigneds(values: &[u32]) -> CanonicalValue {
    CanonicalValue::Array(
        values
            .iter()
            .map(|value| CanonicalValue::Unsigned(u64::from(*value)))
            .collect(),
    )
}

fn points(values: &BTreeSet<Vec<i64>>) -> CanonicalValue {
    CanonicalValue::Array(
        values
            .iter()
            .map(|point| CanonicalValue::Array(point.iter().copied().map(signed).collect()))
            .collect(),
    )
}
