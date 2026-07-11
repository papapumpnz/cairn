//! Adapter-neutral finite categorical fields and a first learned field model.

mod schema;

pub use schema::{
    Direction, FieldSchemaError, FieldSchemaLearner, FieldTransformation, LearnedFieldTheory,
};

use cairn_core::{
    CanonicalValue, Hash32, ModalityId, Observation, ObservationCertainty, ObservationError,
    SourceReference,
};
use cairn_invent::{
    BindingExample, DefinitionInventor, DefinitionProposal, InventionError, InventionLimits,
    InventionProblem,
};
use cairn_reg::{
    AttrKey, AttrValue, Attribute, DerivationId, DerivationKind, DerivationRecord, EvidenceWeight,
    NodeConstraint, NodeId, NodeKind, RegError, RegLimits, RelGraphBuilder, RelGraphSnapshot,
    RelationDefinition, RelationId,
};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteField {
    pub shape: Vec<u32>,
    pub values: Vec<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldError {
    EmptyShape,
    EmptyAxis,
    Cardinality,
    Capacity,
    Observation(ObservationError),
    Invention(InventionError),
    Prediction,
    ObservationMismatch,
    Reg(RegError),
    NoResidual,
    ResidualConflict,
}

#[derive(Clone, Debug)]
pub struct FieldEvidence {
    pub graph: RelGraphSnapshot,
    pub cells: Vec<NodeId>,
    pub adjacent: RelationId,
    pub same_value_adjacent: RelationId,
    pub row_key: AttrKey,
    pub column_key: AttrKey,
    pub value_key: AttrKey,
}

impl FiniteField {
    /// Validate exact finite axes and row-major payload cardinality.
    ///
    /// # Errors
    ///
    /// Rejects empty/zero axes, overflow, or mismatched payload length.
    pub fn validate(&self) -> Result<(), FieldError> {
        if self.shape.is_empty() {
            return Err(FieldError::EmptyShape);
        }
        if self.shape.contains(&0) {
            return Err(FieldError::EmptyAxis);
        }
        let cardinality = self
            .shape
            .iter()
            .try_fold(1_usize, |total, extent| total.checked_mul(*extent as usize));
        if cardinality != Some(self.values.len()) {
            return Err(FieldError::Cardinality);
        }
        Ok(())
    }

    /// Lower the field into a generic, provenance-bound observation.
    ///
    /// # Errors
    ///
    /// Returns field validation or observation identity failures.
    pub fn observation(&self, source: SourceReference) -> Result<Observation, FieldError> {
        self.validate()?;
        let payload = CanonicalValue::Array(vec![
            CanonicalValue::Array(
                self.shape
                    .iter()
                    .map(|extent| CanonicalValue::Unsigned(u64::from(*extent)))
                    .collect(),
            ),
            CanonicalValue::Array(self.values.iter().map(|value| signed(*value)).collect()),
        ]);
        Observation::new(
            ModalityId("urn:cairn:finite-field:v1".to_owned()),
            None,
            source,
            payload,
            ObservationCertainty::Exact,
        )
        .map_err(FieldError::Observation)
    }

    /// Decode an exact field observation into modality-neutral scalar nodes
    /// and adjacency relations suitable for entity and theory invention.
    ///
    /// # Errors
    ///
    /// Rejects invalid fields, mismatched observation provenance/payload, and
    /// relational graph capacity or integrity failures.
    pub fn evidence_graph(
        &self,
        observation: &Observation,
        limits: RegLimits,
    ) -> Result<FieldEvidence, FieldError> {
        self.validate()?;
        observation.verify().map_err(FieldError::Observation)?;
        let expected = self.observation(observation.source.clone())?;
        if observation.logical_time.is_some() || expected.id != observation.id {
            return Err(FieldError::ObservationMismatch);
        }
        let mut builder = RelGraphBuilder::new(limits);
        let derivation = builder
            .add_derivation(DerivationRecord {
                kind: DerivationKind::Observed,
                operation: observation.id.0,
                inputs: Vec::new(),
                parameters: Vec::new(),
                semantics_version: "cairn.field.evidence/1".to_owned(),
            })
            .map_err(FieldError::Reg)?;
        let adjacent = builder
            .declare_relation(RelationDefinition {
                name: "field.adjacent".to_owned(),
                arguments: vec![
                    NodeConstraint::Kind(NodeKind::Scalar),
                    NodeConstraint::Kind(NodeKind::Scalar),
                ],
            })
            .map_err(FieldError::Reg)?;
        let same_value_adjacent = builder
            .declare_relation(RelationDefinition {
                name: "field.same_value_adjacent".to_owned(),
                arguments: vec![
                    NodeConstraint::Kind(NodeKind::Scalar),
                    NodeConstraint::Kind(NodeKind::Scalar),
                ],
            })
            .map_err(FieldError::Reg)?;
        let row_key = AttrKey(1);
        let column_key = AttrKey(2);
        let value_key = AttrKey(3);
        let columns = usize::try_from(self.shape[1]).map_err(|_| FieldError::Capacity)?;
        let cells = self
            .values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                builder
                    .add_node(
                        NodeKind::Scalar,
                        vec![
                            Attribute {
                                key: row_key,
                                value: AttrValue::Int(
                                    i64::try_from(index / columns)
                                        .map_err(|_| FieldError::Capacity)?,
                                ),
                            },
                            Attribute {
                                key: column_key,
                                value: AttrValue::Int(
                                    i64::try_from(index % columns)
                                        .map_err(|_| FieldError::Capacity)?,
                                ),
                            },
                            Attribute {
                                key: value_key,
                                value: AttrValue::Int(*value),
                            },
                        ],
                        derivation,
                    )
                    .map_err(FieldError::Reg)
            })
            .collect::<Result<Vec<_>, FieldError>>()?;
        let rows = usize::try_from(self.shape[0]).map_err(|_| FieldError::Capacity)?;
        add_adjacencies(
            &mut builder,
            &cells,
            &self.values,
            rows,
            columns,
            adjacent,
            same_value_adjacent,
            derivation,
        )?;
        Ok(FieldEvidence {
            graph: builder.publish(),
            cells,
            adjacent,
            same_value_adjacent,
            row_key,
            column_key,
            value_key,
        })
    }
}

#[allow(clippy::too_many_arguments)] // Explicit graph resources keep the frontend stateless.
fn add_adjacencies(
    builder: &mut RelGraphBuilder,
    cells: &[NodeId],
    values: &[i64],
    rows: usize,
    columns: usize,
    adjacent: RelationId,
    same_value_adjacent: RelationId,
    derivation: DerivationId,
) -> Result<(), FieldError> {
    for row in 0..rows {
        for column in 0..columns {
            let source = row * columns + column;
            for target in [
                (column + 1 < columns).then_some(source + 1),
                (row + 1 < rows).then_some(source + columns),
            ]
            .into_iter()
            .flatten()
            {
                builder
                    .add_fact(
                        adjacent,
                        vec![cells[source], cells[target]],
                        Vec::new(),
                        EvidenceWeight::Exact,
                        derivation,
                    )
                    .map_err(FieldError::Reg)?;
                if values[source] == values[target] {
                    builder
                        .add_fact(
                            same_value_adjacent,
                            vec![cells[source], cells[target]],
                            Vec::new(),
                            EvidenceWeight::Exact,
                            derivation,
                        )
                        .map_err(FieldError::Reg)?;
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedPointwiseFieldTheory {
    pub proposal: DefinitionProposal,
    pub training_cells: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PointwiseFieldLearner;

impl PointwiseFieldLearner {
    /// Invent one executable cell constructor from paired fields. The learner
    /// receives values, coordinates, and extents as ordinary typed bindings.
    ///
    /// # Errors
    ///
    /// Rejects invalid pairs or bounded invention exhaustion.
    pub fn learn(
        &self,
        pairs: &[(FiniteField, FiniteField)],
        limits: InventionLimits,
    ) -> Result<LearnedPointwiseFieldTheory, FieldError> {
        if pairs.is_empty() {
            return Err(FieldError::Invention(InventionError::EmptyDemonstrations));
        }
        let parameters = parameters();
        let mut demonstrations = Vec::new();
        for (input, output) in pairs {
            input.validate()?;
            output.validate()?;
            if input.shape != output.shape || input.shape.len() != 2 {
                return Err(FieldError::Cardinality);
            }
            for (index, (&input_value, &output_value)) in
                input.values.iter().zip(&output.values).enumerate()
            {
                demonstrations.push(BindingExample {
                    inputs: bindings(input, index, input_value)?,
                    output: Value::Int(output_value),
                });
            }
        }
        let outcome = DefinitionInventor
            .invent(&InventionProblem {
                definition_name: "field_cell".to_owned(),
                parameters,
                return_type: output_integer(),
                literals: (-4..=4).map(Value::Int).collect(),
                demonstrations,
                limits,
            })
            .map_err(FieldError::Invention)?;
        Ok(LearnedPointwiseFieldTheory {
            proposal: outcome.proposal,
            training_cells: outcome.admission.training_examples,
        })
    }

    /// Apply the frozen learned constructor to changed field geometry/content.
    ///
    /// # Errors
    ///
    /// Rejects invalid fields or non-integer/out-of-bound predictions.
    pub fn predict(
        &self,
        theory: &LearnedPointwiseFieldTheory,
        input: &FiniteField,
        limits: cairn_rkl::ExecLimits,
    ) -> Result<FiniteField, FieldError> {
        input.validate()?;
        if input.shape.len() != 2 {
            return Err(FieldError::Cardinality);
        }
        let values = input
            .values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                DefinitionInventor
                    .predict(&theory.proposal, &bindings(input, index, *value)?, limits)
                    .map_err(FieldError::Invention)
                    .and_then(|value| match value {
                        Value::Int(value) => Ok(value),
                        _ => Err(FieldError::Prediction),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FiniteField {
            shape: input.shape.clone(),
            values,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedResidualConstructor {
    pub proposal: DefinitionProposal,
    pub background: i64,
    pub paint: i64,
    pub training_cells: usize,
    pub residual_cells: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ResidualConstructorInventor;

impl ResidualConstructorInventor {
    /// Synthesize a Boolean emission condition from cells created in otherwise
    /// unexplained output residuals.
    ///
    /// # Errors
    ///
    /// Rejects empty/invalid evidence, mixed paints, deletion/update residuals,
    /// absent created cells, or bounded invention failure.
    pub fn invent(
        &self,
        pairs: &[(FiniteField, FiniteField)],
        background: i64,
        limits: InventionLimits,
    ) -> Result<LearnedResidualConstructor, FieldError> {
        if pairs.is_empty() {
            return Err(FieldError::NoResidual);
        }
        let parameters = residual_parameters();
        let mut demonstrations = Vec::new();
        let mut paint = None;
        let mut residual_cells = 0_usize;
        for (input, output) in pairs {
            input.validate()?;
            output.validate()?;
            if input.shape != output.shape || input.shape.len() != 2 {
                return Err(FieldError::ResidualConflict);
            }
            for (index, (&before, &after)) in input.values.iter().zip(&output.values).enumerate() {
                let created = before == background && after != background;
                if before != after && !created {
                    return Err(FieldError::ResidualConflict);
                }
                if created {
                    residual_cells = residual_cells.saturating_add(1);
                    if paint.replace(after).is_some_and(|current| current != after) {
                        return Err(FieldError::ResidualConflict);
                    }
                }
                demonstrations.push(BindingExample {
                    inputs: residual_bindings(input, index)?,
                    output: Value::Bool(created),
                });
            }
        }
        let paint = paint.ok_or(FieldError::NoResidual)?;
        let outcome = DefinitionInventor
            .invent(&InventionProblem {
                definition_name: "invented_residual_emission".to_owned(),
                parameters,
                return_type: Type::Bool,
                literals: vec![Value::Int(0), Value::Int(1)],
                demonstrations,
                limits,
            })
            .map_err(FieldError::Invention)?;
        Ok(LearnedResidualConstructor {
            training_cells: outcome.admission.training_examples,
            proposal: outcome.proposal,
            background,
            paint,
            residual_cells,
        })
    }

    /// Apply the frozen invented emission condition to changed geometry.
    ///
    /// # Errors
    ///
    /// Rejects invalid fields or non-Boolean/failed predicate execution.
    pub fn predict(
        &self,
        constructor: &LearnedResidualConstructor,
        input: &FiniteField,
        limits: cairn_rkl::ExecLimits,
    ) -> Result<FiniteField, FieldError> {
        input.validate()?;
        if input.shape.len() != 2 {
            return Err(FieldError::ResidualConflict);
        }
        let mut output = input.clone();
        for index in 0..input.values.len() {
            let emit = DefinitionInventor
                .predict(
                    &constructor.proposal,
                    &residual_bindings(input, index)?,
                    limits,
                )
                .map_err(FieldError::Invention)?;
            match emit {
                Value::Bool(true) if input.values[index] == constructor.background => {
                    output.values[index] = constructor.paint;
                }
                Value::Bool(_) => {}
                _ => return Err(FieldError::Prediction),
            }
        }
        Ok(output)
    }
}

fn residual_parameters() -> Vec<Binder> {
    ["row", "column", "rows", "columns"]
        .into_iter()
        .map(|name| Binder {
            name: VarName(name.to_owned()),
            ty: Type::Int(IntRange {
                lower: 0,
                upper: i64::from(u16::MAX),
            }),
        })
        .collect()
}

fn residual_bindings(field: &FiniteField, index: usize) -> Result<RuntimeEnvironment, FieldError> {
    let columns = usize::try_from(field.shape[1]).map_err(|_| FieldError::Capacity)?;
    let values = [
        i64::try_from(index / columns).map_err(|_| FieldError::Capacity)?,
        i64::try_from(index % columns).map_err(|_| FieldError::Capacity)?,
        i64::from(field.shape[0]),
        i64::from(field.shape[1]),
    ];
    let mut environment = RuntimeEnvironment::new();
    for (parameter, value) in residual_parameters().into_iter().zip(values) {
        environment.insert(parameter.name, Value::Int(value));
    }
    Ok(environment)
}

fn parameters() -> Vec<Binder> {
    ["value", "row", "column", "rows", "columns"]
        .into_iter()
        .map(|name| Binder {
            name: VarName(name.to_owned()),
            ty: input_integer(),
        })
        .collect()
}

fn input_integer() -> Type {
    Type::Int(IntRange {
        lower: -10_000,
        upper: 10_000,
    })
}

fn output_integer() -> Type {
    Type::Int(IntRange {
        lower: -1_000_000,
        upper: 1_000_000,
    })
}

fn bindings(
    field: &FiniteField,
    index: usize,
    value: i64,
) -> Result<RuntimeEnvironment, FieldError> {
    let columns = usize::try_from(field.shape[1]).map_err(|_| FieldError::Capacity)?;
    let row = index / columns;
    let column = index % columns;
    let values = [
        value,
        i64::try_from(row).map_err(|_| FieldError::Capacity)?,
        i64::try_from(column).map_err(|_| FieldError::Capacity)?,
        i64::from(field.shape[0]),
        i64::from(field.shape[1]),
    ];
    let mut environment = RuntimeEnvironment::new();
    for (parameter, value) in parameters().into_iter().zip(values) {
        environment.insert(parameter.name, Value::Int(value));
    }
    Ok(environment)
}

fn signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}

#[must_use]
pub fn generated_source(label: &str) -> SourceReference {
    SourceReference {
        locator: format!("generated:{label}"),
        content_hash: Hash32::digest(label.as_bytes()),
    }
}
