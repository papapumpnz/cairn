use crate::{
    EntityConstruction, EntityConstructionProvenance, EntityCorrespondenceExample,
    EntityGeometryRender, EntityGeometryScene, PointConstructionProvenance, RenderedEntity,
};
use cairn_core::{CanonicalEncode, CanonicalValue, Hash32};
use cairn_invent::{
    BindingExample, DefinitionInventor, DefinitionProposal, InventionError, InventionLimits,
    InventionProblem, UnorderedMappingEpisode, UnorderedMappingProblem,
};
use cairn_rkl::{Binder, ExecLimits, IntRange, RuntimeEnvironment, Type, Value, VarName};
use std::collections::{BTreeMap, BTreeSet};

/// Generic map-each construction program. The learned RKL definitions—not a
/// transform enum—determine output geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityConstructionProgram {
    pub output_extents: Vec<DefinitionProposal>,
    pub output_coordinates: Vec<DefinitionProposal>,
}

/// V2 program whose point mapping is invented and observed jointly, preserving
/// coordinate correspondence that independent per-axis sets cannot establish.
#[derive(Clone, Debug, PartialEq)]
pub struct JointEntityConstructionProgram {
    pub output_extents: Vec<DefinitionProposal>,
    pub output_point: DefinitionProposal,
}

impl JointEntityConstructionProgram {
    /// Return the canonical identity of executable v2 semantics.
    ///
    /// # Errors
    ///
    /// Returns canonical encoding failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        self.encode_canonical().map(|bytes| Hash32::digest(&bytes))
    }
}

impl CanonicalEncode for JointEntityConstructionProgram {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.entity-construction-program.v2".to_owned()),
            CanonicalValue::Text("map-each-entity-point-joint".to_owned()),
            CanonicalValue::Array(
                self.output_extents
                    .iter()
                    .map(|item| {
                        item.definition
                            .encode_canonical()
                            .map(CanonicalValue::Bytes)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            CanonicalValue::Bytes(self.output_point.definition.encode_canonical()?),
        ])
        .encode_canonical()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct JointConstructionProgramReport {
    pub program: JointEntityConstructionProgram,
    pub candidates_evaluated: u64,
    pub example_count: usize,
    pub point_examples: usize,
}

impl EntityConstructionProgram {
    /// Canonical identity includes only executable definitions and generic
    /// map-each semantics.
    ///
    /// # Errors
    ///
    /// Returns canonicalization failure.
    pub fn semantic_hash(&self) -> Result<Hash32, cairn_core::CanonicalError> {
        self.encode_canonical().map(|bytes| Hash32::digest(&bytes))
    }
}

impl CanonicalEncode for EntityConstructionProgram {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        let definitions = |items: &[DefinitionProposal]| {
            items
                .iter()
                .map(|item| {
                    item.definition
                        .encode_canonical()
                        .map(CanonicalValue::Bytes)
                })
                .collect::<Result<Vec<_>, _>>()
        };
        CanonicalValue::Array(vec![
            CanonicalValue::Text("cairn.entity-construction-program.v1".to_owned()),
            CanonicalValue::Text("map-each-entity-point".to_owned()),
            CanonicalValue::Array(definitions(&self.output_extents)?),
            CanonicalValue::Array(definitions(&self.output_coordinates)?),
        ])
        .encode_canonical()
    }
}

/// Training evidence containing a complete source/output point correspondence.
/// It is an untrusted producer input until independent program replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityConstructionTrace {
    pub source: EntityGeometryScene,
    pub expected: EntityGeometryRender,
    pub provenance: Vec<EntityConstructionProvenance>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityConstructionProgramReport {
    pub program: EntityConstructionProgram,
    pub candidates_evaluated: u64,
    pub trace_count: usize,
    pub point_examples: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityConstructionProgramError {
    EmptyTraces,
    InvalidTrace,
    UnsupportedDimensions,
    IncompleteProvenance,
    ConstructionConflict,
    OutOfBounds,
    Capacity,
    Invention(InventionError),
    Prediction,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EntityConstructionProgramSynthesizer;

impl EntityConstructionProgramSynthesizer {
    /// Invent one joint tuple-valued coordinate mapping from unordered entity
    /// point sets, preventing false cross-axis pairings.
    ///
    /// # Errors
    ///
    /// Rejects malformed observations, unsupported dimensions, bounded search
    /// failure, or joint replay mismatch.
    #[allow(clippy::too_many_lines)]
    pub fn synthesize_joint_unordered(
        &self,
        examples: &[EntityCorrespondenceExample],
        limits: InventionLimits,
    ) -> Result<JointConstructionProgramReport, EntityConstructionProgramError> {
        if examples.is_empty() {
            return Err(EntityConstructionProgramError::EmptyTraces);
        }
        let dimensions = examples[0].source.extents.len();
        if dimensions == 0
            || examples.iter().any(|example| {
                example.source.extents.len() != dimensions
                    || example.expected.extents.len() != dimensions
            })
        {
            return Err(EntityConstructionProgramError::UnsupportedDimensions);
        }
        for example in examples {
            validate_unordered_example(example)?;
        }
        let mut output_extents = Vec::new();
        let mut candidates = 0_u64;
        for axis in 0..dimensions {
            let demonstrations = examples
                .iter()
                .map(|example| {
                    Ok(BindingExample {
                        inputs: scene_environment(&example.source, false)?,
                        output: Value::Int(i64::from(example.expected.extents[axis])),
                    })
                })
                .collect::<Result<Vec<_>, EntityConstructionProgramError>>()?;
            let outcome = DefinitionInventor
                .invent(&InventionProblem {
                    definition_name: format!("invented_output_extent_{axis}"),
                    parameters: extent_parameters(dimensions),
                    return_type: scalar_type(),
                    literals: vec![Value::Int(0), Value::Int(1)],
                    demonstrations,
                    limits,
                })
                .map_err(EntityConstructionProgramError::Invention)?;
            candidates = candidates
                .checked_add(outcome.proposal.candidates_evaluated)
                .ok_or(EntityConstructionProgramError::Capacity)?;
            output_extents.push(outcome.proposal);
        }
        let mut episodes = Vec::new();
        let mut point_examples = 0_usize;
        for example in examples {
            let expected = example
                .expected
                .entities
                .iter()
                .map(|entity| (entity.value, entity))
                .collect::<BTreeMap<_, _>>();
            for entity in &example.source.entities {
                let target = expected
                    .get(&entity.value)
                    .ok_or(EntityConstructionProgramError::InvalidTrace)?;
                point_examples = point_examples
                    .checked_add(entity.points.len())
                    .ok_or(EntityConstructionProgramError::Capacity)?;
                episodes.push(UnorderedMappingEpisode {
                    inputs: entity
                        .points
                        .iter()
                        .map(|point| point_environment(&example.source, point))
                        .collect::<Result<Vec<_>, _>>()?,
                    expected_outputs: target
                        .points
                        .iter()
                        .map(|point| Value::Tuple(point.iter().copied().map(Value::Int).collect()))
                        .collect(),
                });
            }
        }
        let outcome = DefinitionInventor
            .invent_unordered_mapping(&UnorderedMappingProblem {
                definition_name: "invented_output_point".to_owned(),
                parameters: point_parameters(dimensions),
                return_type: Type::Tuple(vec![joint_scalar_type(); dimensions]),
                literals: vec![Value::Int(0), Value::Int(1)],
                episodes,
                limits,
            })
            .map_err(EntityConstructionProgramError::Invention)?;
        candidates = candidates
            .checked_add(outcome.proposal.candidates_evaluated)
            .ok_or(EntityConstructionProgramError::Capacity)?;
        let report = JointConstructionProgramReport {
            program: JointEntityConstructionProgram {
                output_extents,
                output_point: outcome.proposal,
            },
            candidates_evaluated: candidates,
            example_count: examples.len(),
            point_examples,
        };
        for example in examples {
            if self
                .execute_joint(&report.program, &example.source, ExecLimits::default())?
                .render
                != example.expected
            {
                return Err(EntityConstructionProgramError::Prediction);
            }
        }
        Ok(report)
    }

    /// Execute a tuple-valued joint coordinate program.
    ///
    /// # Errors
    ///
    /// Rejects malformed scenes/programs, definition failure, bounds, or
    /// conflicting constructions.
    pub fn execute_joint(
        &self,
        program: &JointEntityConstructionProgram,
        source: &EntityGeometryScene,
        limits: ExecLimits,
    ) -> Result<EntityConstruction, EntityConstructionProgramError> {
        validate_source(source)?;
        let dimensions = source.extents.len();
        if program.output_extents.len() != dimensions {
            return Err(EntityConstructionProgramError::UnsupportedDimensions);
        }
        let extents = program
            .output_extents
            .iter()
            .map(|definition| {
                predict_integer(definition, &scene_environment(source, false)?, limits)
            })
            .map(|result| {
                result.and_then(|value| {
                    u32::try_from(value)
                        .ok()
                        .filter(|value| *value > 0)
                        .ok_or(EntityConstructionProgramError::OutOfBounds)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut occupied = BTreeMap::new();
        let mut rendered = Vec::new();
        let mut provenance = Vec::new();
        for entity in &source.entities {
            let mut output_points = BTreeSet::new();
            let mut point_provenance = Vec::new();
            for point in &entity.points {
                let output = match DefinitionInventor
                    .predict(
                        &program.output_point,
                        &point_environment(source, point)?,
                        limits,
                    )
                    .map_err(EntityConstructionProgramError::Invention)?
                {
                    Value::Tuple(values) if values.len() == dimensions => values
                        .into_iter()
                        .map(|value| match value {
                            Value::Int(value) => Ok(value),
                            _ => Err(EntityConstructionProgramError::Prediction),
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    _ => return Err(EntityConstructionProgramError::Prediction),
                };
                if output.iter().zip(&extents).any(|(value, extent)| {
                    *value < 0
                        || u64::try_from(*value).map_or(true, |value| value >= u64::from(*extent))
                }) {
                    return Err(EntityConstructionProgramError::OutOfBounds);
                }
                if occupied
                    .insert(output.clone(), entity.identity)
                    .is_some_and(|owner| owner != entity.identity)
                {
                    return Err(EntityConstructionProgramError::ConstructionConflict);
                }
                output_points.insert(output.clone());
                point_provenance.push(PointConstructionProvenance {
                    output,
                    source: point.clone(),
                });
            }
            point_provenance.sort_by(|left, right| left.output.cmp(&right.output));
            rendered.push(RenderedEntity {
                value: entity.value,
                points: output_points,
            });
            provenance.push(EntityConstructionProvenance {
                source_entity: entity.identity,
                points: point_provenance,
            });
        }
        rendered.sort_by(|left, right| {
            left.value
                .cmp(&right.value)
                .then_with(|| left.points.cmp(&right.points))
        });
        provenance.sort_by_key(|item| item.source_entity);
        Ok(EntityConstruction {
            render: EntityGeometryRender {
                extents,
                background: source.background,
                entities: rendered,
            },
            provenance,
        })
    }
    /// Invent directly from unordered source/output entity-point sets. Entity
    /// values establish only the generic preserved-attribute relation; no point
    /// correspondence or v1 mechanism is supplied.
    ///
    /// # Errors
    ///
    /// Rejects malformed/ambiguous entities, bounded invention failure, and any
    /// jointly reconstructed render mismatch.
    #[allow(clippy::too_many_lines)] // Keeps the bounded reference pipeline explicit.
    pub fn synthesize_unordered(
        &self,
        examples: &[EntityCorrespondenceExample],
        limits: InventionLimits,
    ) -> Result<EntityConstructionProgramReport, EntityConstructionProgramError> {
        if examples.is_empty() {
            return Err(EntityConstructionProgramError::EmptyTraces);
        }
        let dimensions = examples[0].source.extents.len();
        if dimensions == 0
            || examples.iter().any(|example| {
                example.source.extents.len() != dimensions
                    || example.expected.extents.len() != dimensions
            })
        {
            return Err(EntityConstructionProgramError::UnsupportedDimensions);
        }
        for example in examples {
            validate_unordered_example(example)?;
        }
        let extent_parameters = extent_parameters(dimensions);
        let mut output_extents = Vec::with_capacity(dimensions);
        let mut candidates_evaluated = 0_u64;
        for axis in 0..dimensions {
            let demonstrations = examples
                .iter()
                .map(|example| {
                    Ok(BindingExample {
                        inputs: scene_environment(&example.source, false)?,
                        output: Value::Int(i64::from(example.expected.extents[axis])),
                    })
                })
                .collect::<Result<Vec<_>, EntityConstructionProgramError>>()?;
            let outcome = DefinitionInventor
                .invent(&InventionProblem {
                    definition_name: format!("invented_output_extent_{axis}"),
                    parameters: extent_parameters.clone(),
                    return_type: scalar_type(),
                    literals: vec![Value::Int(0), Value::Int(1)],
                    demonstrations,
                    limits,
                })
                .map_err(EntityConstructionProgramError::Invention)?;
            candidates_evaluated = candidates_evaluated
                .checked_add(outcome.proposal.candidates_evaluated)
                .ok_or(EntityConstructionProgramError::Capacity)?;
            output_extents.push(outcome.proposal);
        }
        let mut output_coordinates = Vec::with_capacity(dimensions);
        let mut point_examples = 0_usize;
        for axis in 0..dimensions {
            let mut episodes = Vec::new();
            for example in examples {
                let expected = example
                    .expected
                    .entities
                    .iter()
                    .map(|entity| (entity.value, entity))
                    .collect::<BTreeMap<_, _>>();
                for entity in &example.source.entities {
                    let target = expected
                        .get(&entity.value)
                        .ok_or(EntityConstructionProgramError::InvalidTrace)?;
                    if axis == 0 {
                        point_examples = point_examples
                            .checked_add(entity.points.len())
                            .ok_or(EntityConstructionProgramError::Capacity)?;
                    }
                    episodes.push(UnorderedMappingEpisode {
                        inputs: entity
                            .points
                            .iter()
                            .map(|point| point_environment(&example.source, point))
                            .collect::<Result<Vec<_>, _>>()?,
                        expected_outputs: target
                            .points
                            .iter()
                            .map(|point| Value::Int(point[axis]))
                            .collect(),
                    });
                }
            }
            let outcome = DefinitionInventor
                .invent_unordered_mapping(&UnorderedMappingProblem {
                    definition_name: format!("invented_output_coordinate_{axis}"),
                    parameters: point_parameters(dimensions),
                    return_type: scalar_type(),
                    literals: vec![Value::Int(0), Value::Int(1)],
                    episodes,
                    limits,
                })
                .map_err(EntityConstructionProgramError::Invention)?;
            candidates_evaluated = candidates_evaluated
                .checked_add(outcome.proposal.candidates_evaluated)
                .ok_or(EntityConstructionProgramError::Capacity)?;
            output_coordinates.push(outcome.proposal);
        }
        let report = EntityConstructionProgramReport {
            program: EntityConstructionProgram {
                output_extents,
                output_coordinates,
            },
            candidates_evaluated,
            trace_count: examples.len(),
            point_examples,
        };
        for example in examples {
            if self
                .execute(&report.program, &example.source, ExecLimits::default())?
                .render
                != example.expected
            {
                return Err(EntityConstructionProgramError::Prediction);
            }
        }
        Ok(report)
    }

    /// Invent output extent and coordinate definitions from complete execution
    /// traces using only generic scalar RKL variables and arithmetic.
    ///
    /// # Errors
    ///
    /// Rejects malformed/incomplete traces, mixed dimensions, bounded invention
    /// failure, or a synthesized program that does not replay every trace.
    pub fn synthesize(
        &self,
        traces: &[EntityConstructionTrace],
        limits: InventionLimits,
    ) -> Result<EntityConstructionProgramReport, EntityConstructionProgramError> {
        if traces.is_empty() {
            return Err(EntityConstructionProgramError::EmptyTraces);
        }
        let dimensions = traces[0].source.extents.len();
        if dimensions == 0
            || traces.iter().any(|trace| {
                trace.source.extents.len() != dimensions
                    || trace.expected.extents.len() != dimensions
            })
        {
            return Err(EntityConstructionProgramError::UnsupportedDimensions);
        }
        for trace in traces {
            validate_trace(trace)?;
        }
        let extent_parameters = extent_parameters(dimensions);
        let point_parameters = point_parameters(dimensions);
        let extent_inputs = traces
            .iter()
            .map(|trace| scene_environment(&trace.source, false))
            .collect::<Result<Vec<_>, _>>()?;
        let mut output_extents = Vec::with_capacity(dimensions);
        let mut candidates_evaluated = 0_u64;
        for axis in 0..dimensions {
            let demonstrations = traces
                .iter()
                .zip(&extent_inputs)
                .map(|(trace, inputs)| BindingExample {
                    inputs: inputs.clone(),
                    output: Value::Int(i64::from(trace.expected.extents[axis])),
                })
                .collect();
            let outcome = DefinitionInventor
                .invent(&InventionProblem {
                    definition_name: format!("invented_output_extent_{axis}"),
                    parameters: extent_parameters.clone(),
                    return_type: scalar_type(),
                    literals: vec![Value::Int(0), Value::Int(1)],
                    demonstrations,
                    limits,
                })
                .map_err(EntityConstructionProgramError::Invention)?;
            candidates_evaluated = candidates_evaluated
                .checked_add(outcome.proposal.candidates_evaluated)
                .ok_or(EntityConstructionProgramError::Capacity)?;
            output_extents.push(outcome.proposal);
        }
        let point_rows = trace_point_rows(traces)?;
        let mut output_coordinates = Vec::with_capacity(dimensions);
        for axis in 0..dimensions {
            let demonstrations = point_rows
                .iter()
                .map(|row| BindingExample {
                    inputs: row.inputs.clone(),
                    output: Value::Int(row.output[axis]),
                })
                .collect();
            let outcome = DefinitionInventor
                .invent(&InventionProblem {
                    definition_name: format!("invented_output_coordinate_{axis}"),
                    parameters: point_parameters.clone(),
                    return_type: scalar_type(),
                    literals: vec![Value::Int(0), Value::Int(1)],
                    demonstrations,
                    limits,
                })
                .map_err(EntityConstructionProgramError::Invention)?;
            candidates_evaluated = candidates_evaluated
                .checked_add(outcome.proposal.candidates_evaluated)
                .ok_or(EntityConstructionProgramError::Capacity)?;
            output_coordinates.push(outcome.proposal);
        }
        let report = EntityConstructionProgramReport {
            program: EntityConstructionProgram {
                output_extents,
                output_coordinates,
            },
            candidates_evaluated,
            trace_count: traces.len(),
            point_examples: point_rows.len(),
        };
        for trace in traces {
            if self
                .execute(&report.program, &trace.source, ExecLimits::default())?
                .render
                != trace.expected
            {
                return Err(EntityConstructionProgramError::Prediction);
            }
        }
        Ok(report)
    }

    /// Execute generic map-each semantics using only the synthesized RKL
    /// definitions.
    ///
    /// # Errors
    ///
    /// Rejects malformed programs/scenes, definition failures, out-of-bounds
    /// construction, and conflicting entity output.
    pub fn execute(
        &self,
        program: &EntityConstructionProgram,
        source: &EntityGeometryScene,
        limits: ExecLimits,
    ) -> Result<EntityConstruction, EntityConstructionProgramError> {
        let dimensions = source.extents.len();
        validate_source(source)?;
        if program.output_extents.len() != dimensions
            || program.output_coordinates.len() != dimensions
        {
            return Err(EntityConstructionProgramError::UnsupportedDimensions);
        }
        let extent_inputs = scene_environment(source, false)?;
        let extents = program
            .output_extents
            .iter()
            .map(|definition| predict_integer(definition, &extent_inputs, limits))
            .map(|value| {
                value.and_then(|value| {
                    u32::try_from(value)
                        .ok()
                        .filter(|value| *value > 0)
                        .ok_or(EntityConstructionProgramError::OutOfBounds)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut occupied = BTreeMap::new();
        let mut rendered = Vec::with_capacity(source.entities.len());
        let mut provenance = Vec::with_capacity(source.entities.len());
        for entity in &source.entities {
            let mut output_points = BTreeSet::new();
            let mut point_provenance = Vec::with_capacity(entity.points.len());
            for point in &entity.points {
                let inputs = point_environment(source, point)?;
                let output = program
                    .output_coordinates
                    .iter()
                    .map(|definition| predict_integer(definition, &inputs, limits))
                    .collect::<Result<Vec<_>, _>>()?;
                if output.iter().zip(&extents).any(|(value, extent)| {
                    *value < 0
                        || u64::try_from(*value).map_or(true, |value| value >= u64::from(*extent))
                }) {
                    return Err(EntityConstructionProgramError::OutOfBounds);
                }
                if occupied
                    .insert(output.clone(), entity.identity)
                    .is_some_and(|owner| owner != entity.identity)
                {
                    return Err(EntityConstructionProgramError::ConstructionConflict);
                }
                output_points.insert(output.clone());
                point_provenance.push(PointConstructionProvenance {
                    output,
                    source: point.clone(),
                });
            }
            point_provenance.sort_by(|left, right| left.output.cmp(&right.output));
            rendered.push(RenderedEntity {
                value: entity.value,
                points: output_points,
            });
            provenance.push(EntityConstructionProvenance {
                source_entity: entity.identity,
                points: point_provenance,
            });
        }
        rendered.sort_by(|left, right| {
            left.value
                .cmp(&right.value)
                .then_with(|| left.points.cmp(&right.points))
        });
        provenance.sort_by_key(|item| item.source_entity);
        Ok(EntityConstruction {
            render: EntityGeometryRender {
                extents,
                background: source.background,
                entities: rendered,
            },
            provenance,
        })
    }
}

#[derive(Clone)]
struct PointRow {
    inputs: RuntimeEnvironment,
    output: Vec<i64>,
}

fn trace_point_rows(
    traces: &[EntityConstructionTrace],
) -> Result<Vec<PointRow>, EntityConstructionProgramError> {
    let mut rows = Vec::new();
    for trace in traces {
        for item in &trace.provenance {
            for point in &item.points {
                rows.push(PointRow {
                    inputs: point_environment(&trace.source, &point.source)?,
                    output: point.output.clone(),
                });
            }
        }
    }
    Ok(rows)
}

fn validate_trace(trace: &EntityConstructionTrace) -> Result<(), EntityConstructionProgramError> {
    validate_source(&trace.source)?;
    if trace.provenance.len() != trace.source.entities.len() {
        return Err(EntityConstructionProgramError::IncompleteProvenance);
    }
    let expected_by_value = trace
        .expected
        .entities
        .iter()
        .map(|entity| (entity.value, &entity.points))
        .collect::<BTreeMap<_, _>>();
    if expected_by_value.len() != trace.expected.entities.len() {
        return Err(EntityConstructionProgramError::InvalidTrace);
    }
    for entity in &trace.source.entities {
        let item = trace
            .provenance
            .iter()
            .find(|item| item.source_entity == entity.identity)
            .ok_or(EntityConstructionProgramError::IncompleteProvenance)?;
        if item.points.len() != entity.points.len()
            || item
                .points
                .iter()
                .map(|point| point.source.clone())
                .collect::<BTreeSet<_>>()
                != entity.points
            || item
                .points
                .iter()
                .map(|point| point.output.clone())
                .collect::<BTreeSet<_>>()
                != **expected_by_value
                    .get(&entity.value)
                    .ok_or(EntityConstructionProgramError::InvalidTrace)?
        {
            return Err(EntityConstructionProgramError::IncompleteProvenance);
        }
    }
    Ok(())
}

fn validate_unordered_example(
    example: &EntityCorrespondenceExample,
) -> Result<(), EntityConstructionProgramError> {
    validate_source(&example.source)?;
    if example.expected.extents.is_empty()
        || example.expected.extents.contains(&0)
        || example.expected.entities.len() != example.source.entities.len()
    {
        return Err(EntityConstructionProgramError::InvalidTrace);
    }
    let source_values = example
        .source
        .entities
        .iter()
        .map(|entity| entity.value)
        .collect::<BTreeSet<_>>();
    let expected_values = example
        .expected
        .entities
        .iter()
        .map(|entity| entity.value)
        .collect::<BTreeSet<_>>();
    if source_values.len() != example.source.entities.len()
        || expected_values.len() != example.expected.entities.len()
        || source_values != expected_values
        || example.expected.entities.iter().any(|entity| {
            entity.points.is_empty()
                || entity.points.iter().any(|point| {
                    point.len() != example.expected.extents.len()
                        || point
                            .iter()
                            .zip(&example.expected.extents)
                            .any(|(value, extent)| {
                                *value < 0
                                    || u64::try_from(*value)
                                        .map_or(true, |value| value >= u64::from(*extent))
                            })
                })
        })
    {
        return Err(EntityConstructionProgramError::InvalidTrace);
    }
    Ok(())
}

fn validate_source(source: &EntityGeometryScene) -> Result<(), EntityConstructionProgramError> {
    if source.extents.is_empty() || source.extents.contains(&0) || source.entities.is_empty() {
        return Err(EntityConstructionProgramError::InvalidTrace);
    }
    let mut identities = BTreeSet::new();
    for entity in &source.entities {
        if !identities.insert(entity.identity) || entity.points.is_empty() {
            return Err(EntityConstructionProgramError::InvalidTrace);
        }
        if entity.points.iter().any(|point| {
            point.len() != source.extents.len()
                || point.iter().zip(&source.extents).any(|(value, extent)| {
                    *value < 0
                        || u64::try_from(*value).map_or(true, |value| value >= u64::from(*extent))
                })
        }) {
            return Err(EntityConstructionProgramError::InvalidTrace);
        }
    }
    Ok(())
}

fn point_parameters(dimensions: usize) -> Vec<Binder> {
    (0..dimensions)
        .map(|axis| binder(&format!("source_coordinate_{axis}")))
        .chain(extent_parameters(dimensions))
        .collect()
}

fn extent_parameters(dimensions: usize) -> Vec<Binder> {
    (0..dimensions)
        .flat_map(|axis| {
            [
                binder(&format!("source_extent_{axis}")),
                binder(&format!("collection_minimum_{axis}")),
                binder(&format!("collection_maximum_{axis}")),
            ]
        })
        .collect()
}

fn binder(name: &str) -> Binder {
    Binder {
        name: VarName(name.to_owned()),
        ty: scalar_type(),
    }
}

fn scalar_type() -> Type {
    Type::Int(IntRange {
        lower: -1_000_000,
        upper: 1_000_000,
    })
}

fn joint_scalar_type() -> Type {
    Type::Int(IntRange {
        lower: -2_000_000,
        upper: 2_000_000,
    })
}

fn scene_environment(
    source: &EntityGeometryScene,
    include_points: bool,
) -> Result<RuntimeEnvironment, EntityConstructionProgramError> {
    let mut environment = RuntimeEnvironment::new();
    for axis in 0..source.extents.len() {
        let minimum = source
            .entities
            .iter()
            .flat_map(|entity| &entity.points)
            .map(|point| point[axis])
            .min()
            .ok_or(EntityConstructionProgramError::InvalidTrace)?;
        let maximum = source
            .entities
            .iter()
            .flat_map(|entity| &entity.points)
            .map(|point| point[axis])
            .max()
            .ok_or(EntityConstructionProgramError::InvalidTrace)?;
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
        if include_points {
            environment.insert(VarName(format!("source_coordinate_{axis}")), Value::Int(0));
        }
    }
    Ok(environment)
}

fn point_environment(
    source: &EntityGeometryScene,
    point: &[i64],
) -> Result<RuntimeEnvironment, EntityConstructionProgramError> {
    if point.len() != source.extents.len() {
        return Err(EntityConstructionProgramError::InvalidTrace);
    }
    let mut environment = scene_environment(source, false)?;
    for (axis, value) in point.iter().copied().enumerate() {
        environment.insert(
            VarName(format!("source_coordinate_{axis}")),
            Value::Int(value),
        );
    }
    Ok(environment)
}

fn predict_integer(
    definition: &DefinitionProposal,
    inputs: &RuntimeEnvironment,
    limits: ExecLimits,
) -> Result<i64, EntityConstructionProgramError> {
    match DefinitionInventor.predict(definition, inputs, limits) {
        Ok(Value::Int(value)) => Ok(value),
        _ => Err(EntityConstructionProgramError::Prediction),
    }
}
