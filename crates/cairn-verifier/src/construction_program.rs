use cairn_core::{CanonicalEncode, CanonicalValue, Hash32};
use cairn_rkl::{
    ExecLimits, ReferenceExecutor, RuntimeEnvironment, TraceLevel, TypeChecker, TypeEnvironment,
    Value, VarName,
};
use cairn_theory::{
    EntityConstructionProgram, EntityCorrespondenceExample, EntityGeometryRender,
    EntityGeometryScene, RenderedEntity,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedEntityConstructionProgram {
    program: Hash32,
    experiences: BTreeSet<Hash32>,
    replayed_entities: u64,
    replayed_points: u64,
}

impl VerifiedEntityConstructionProgram {
    #[must_use]
    pub const fn program(&self) -> Hash32 {
        self.program
    }
    #[must_use]
    pub const fn experiences(&self) -> &BTreeSet<Hash32> {
        &self.experiences
    }
    #[must_use]
    pub const fn replayed_entities(&self) -> u64 {
        self.replayed_entities
    }
    #[must_use]
    pub const fn replayed_points(&self) -> u64 {
        self.replayed_points
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityConstructionProgramVerifyError {
    InsufficientEvidence,
    DuplicateExperience,
    InvalidScene,
    InvalidProgram,
    DefinitionFailure,
    OutOfBounds,
    ConstructionConflict,
    ReplayMismatch,
    Capacity,
    Canonicalization,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EntityConstructionProgramVerifier;

impl EntityConstructionProgramVerifier {
    /// Independently type-check and execute learned RKL definitions over
    /// changed scenes. This path does not call the program synthesizer.
    ///
    /// # Errors
    ///
    /// Rejects insufficient/malformed evidence, definition or bound failure,
    /// collisions, duplicate experience, and any exact replay disagreement.
    pub fn verify(
        &self,
        program: &EntityConstructionProgram,
        examples: &[EntityCorrespondenceExample],
        limits: ExecLimits,
    ) -> Result<VerifiedEntityConstructionProgram, EntityConstructionProgramVerifyError> {
        if examples.len() < 2 {
            return Err(EntityConstructionProgramVerifyError::InsufficientEvidence);
        }
        let mut receipt = VerifiedEntityConstructionProgram {
            program: program
                .semantic_hash()
                .map_err(|_| EntityConstructionProgramVerifyError::Canonicalization)?,
            experiences: BTreeSet::new(),
            replayed_entities: 0,
            replayed_points: 0,
        };
        for example in examples {
            if !receipt.experiences.insert(experience_hash(example)?) {
                return Err(EntityConstructionProgramVerifyError::DuplicateExperience);
            }
            let render = reference_execute(program, &example.source, limits)?;
            if render != example.expected {
                return Err(EntityConstructionProgramVerifyError::ReplayMismatch);
            }
            receipt.replayed_entities = receipt
                .replayed_entities
                .checked_add(
                    u64::try_from(example.source.entities.len())
                        .map_err(|_| EntityConstructionProgramVerifyError::Capacity)?,
                )
                .ok_or(EntityConstructionProgramVerifyError::Capacity)?;
            receipt.replayed_points = receipt
                .replayed_points
                .checked_add(
                    u64::try_from(
                        example
                            .source
                            .entities
                            .iter()
                            .map(|entity| entity.points.len())
                            .sum::<usize>(),
                    )
                    .map_err(|_| EntityConstructionProgramVerifyError::Capacity)?,
                )
                .ok_or(EntityConstructionProgramVerifyError::Capacity)?;
        }
        Ok(receipt)
    }
}

fn reference_execute(
    program: &EntityConstructionProgram,
    source: &EntityGeometryScene,
    limits: ExecLimits,
) -> Result<EntityGeometryRender, EntityConstructionProgramVerifyError> {
    validate_source(source)?;
    let dimensions = source.extents.len();
    if program.output_extents.len() != dimensions || program.output_coordinates.len() != dimensions
    {
        return Err(EntityConstructionProgramVerifyError::InvalidProgram);
    }
    let extent_environment = scene_environment(source)?;
    let extents = program
        .output_extents
        .iter()
        .map(|proposal| execute_integer(&proposal.definition, &extent_environment, limits))
        .map(|result| {
            result.and_then(|value| {
                u32::try_from(value)
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or(EntityConstructionProgramVerifyError::OutOfBounds)
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut occupied = BTreeMap::<Vec<i64>, Hash32>::new();
    let mut entities = Vec::new();
    for entity in &source.entities {
        let mut points = BTreeSet::new();
        for source_point in &entity.points {
            let mut environment = scene_environment(source)?;
            for (axis, value) in source_point.iter().copied().enumerate() {
                environment.insert(
                    VarName(format!("source_coordinate_{axis}")),
                    Value::Int(value),
                );
            }
            let output = program
                .output_coordinates
                .iter()
                .map(|proposal| execute_integer(&proposal.definition, &environment, limits))
                .collect::<Result<Vec<_>, _>>()?;
            if output.iter().zip(&extents).any(|(value, extent)| {
                *value < 0
                    || u64::try_from(*value).map_or(true, |value| value >= u64::from(*extent))
            }) {
                return Err(EntityConstructionProgramVerifyError::OutOfBounds);
            }
            if occupied
                .insert(output.clone(), entity.identity)
                .is_some_and(|owner| owner != entity.identity)
            {
                return Err(EntityConstructionProgramVerifyError::ConstructionConflict);
            }
            points.insert(output);
        }
        entities.push(RenderedEntity {
            value: entity.value,
            points,
        });
    }
    entities.sort_by(|left, right| {
        left.value
            .cmp(&right.value)
            .then_with(|| left.points.cmp(&right.points))
    });
    Ok(EntityGeometryRender {
        extents,
        background: source.background,
        entities,
    })
}

fn execute_integer(
    definition: &cairn_rkl::Definition,
    inputs: &RuntimeEnvironment,
    limits: ExecLimits,
) -> Result<i64, EntityConstructionProgramVerifyError> {
    let mut types = TypeEnvironment::new();
    for parameter in &definition.parameters {
        types.insert(parameter.name.clone(), parameter.ty.clone());
    }
    let checked = TypeChecker::new(10_000)
        .check(&definition.body, &types)
        .map_err(|_| EntityConstructionProgramVerifyError::DefinitionFailure)?;
    match ReferenceExecutor::without_definitions(limits, TraceLevel::Off)
        .execute(&checked, inputs)
        .map_err(|_| EntityConstructionProgramVerifyError::DefinitionFailure)?
        .result
    {
        Value::Int(value) => Ok(value),
        _ => Err(EntityConstructionProgramVerifyError::DefinitionFailure),
    }
}

fn scene_environment(
    source: &EntityGeometryScene,
) -> Result<RuntimeEnvironment, EntityConstructionProgramVerifyError> {
    let mut environment = RuntimeEnvironment::new();
    for axis in 0..source.extents.len() {
        let minimum = source
            .entities
            .iter()
            .flat_map(|entity| &entity.points)
            .map(|point| point[axis])
            .min()
            .ok_or(EntityConstructionProgramVerifyError::InvalidScene)?;
        let maximum = source
            .entities
            .iter()
            .flat_map(|entity| &entity.points)
            .map(|point| point[axis])
            .max()
            .ok_or(EntityConstructionProgramVerifyError::InvalidScene)?;
        for (name, value) in [
            (
                format!("source_extent_{axis}"),
                i64::from(source.extents[axis]),
            ),
            (format!("collection_minimum_{axis}"), minimum),
            (format!("collection_maximum_{axis}"), maximum),
        ] {
            environment.insert(VarName(name), Value::Int(value));
        }
    }
    Ok(environment)
}

fn validate_source(
    source: &EntityGeometryScene,
) -> Result<(), EntityConstructionProgramVerifyError> {
    if source.extents.is_empty() || source.extents.contains(&0) || source.entities.is_empty() {
        return Err(EntityConstructionProgramVerifyError::InvalidScene);
    }
    let mut identities = BTreeSet::new();
    for entity in &source.entities {
        if !identities.insert(entity.identity) || entity.points.is_empty() {
            return Err(EntityConstructionProgramVerifyError::InvalidScene);
        }
        if entity.points.iter().any(|point| {
            point.len() != source.extents.len()
                || point.iter().zip(&source.extents).any(|(value, extent)| {
                    *value < 0
                        || u64::try_from(*value).map_or(true, |value| value >= u64::from(*extent))
                })
        }) {
            return Err(EntityConstructionProgramVerifyError::InvalidScene);
        }
    }
    Ok(())
}

fn experience_hash(
    example: &EntityCorrespondenceExample,
) -> Result<Hash32, EntityConstructionProgramVerifyError> {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.entity-program-experience.v1".to_owned()),
        unsigneds(&example.source.extents),
        signed(example.source.background),
        CanonicalValue::Array(
            example
                .source
                .entities
                .iter()
                .map(|entity| {
                    CanonicalValue::Array(vec![
                        CanonicalValue::Bytes(entity.identity.as_bytes().to_vec()),
                        signed(entity.value),
                        point_set(&entity.points),
                    ])
                })
                .collect(),
        ),
        unsigneds(&example.expected.extents),
        signed(example.expected.background),
        CanonicalValue::Array(
            example
                .expected
                .entities
                .iter()
                .map(|entity| {
                    CanonicalValue::Array(vec![signed(entity.value), point_set(&entity.points)])
                })
                .collect(),
        ),
    ])
    .encode_canonical()
    .map(|bytes| Hash32::digest(&bytes))
    .map_err(|_| EntityConstructionProgramVerifyError::Canonicalization)
}

fn unsigneds(values: &[u32]) -> CanonicalValue {
    CanonicalValue::Array(
        values
            .iter()
            .map(|value| CanonicalValue::Unsigned(u64::from(*value)))
            .collect(),
    )
}
fn point_set(points: &BTreeSet<Vec<i64>>) -> CanonicalValue {
    CanonicalValue::Array(
        points
            .iter()
            .map(|point| CanonicalValue::Array(point.iter().copied().map(signed).collect()))
            .collect(),
    )
}
fn signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}
