//! Adapter-neutral finite categorical fields and a first learned field model.

use cairn_core::{
    CanonicalValue, CollectionDemonstration, CollectionProcedureProblem, EntityId, EntityRecord,
    EntitySelectionDemonstration, EntitySelectionProblem, FeatureId, Hash32, ModalityId,
    Observation, ObservationCertainty, ObservationError, RelationalEntityCollection,
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
use cairn_theory::{
    ContrastExample, ContrastPredicateInventor, CoordinateLayer, LayerMechanism,
    LayerMechanismLearner, LayerRender, LayerScene, LearnedEntitySelector,
    RelationalEntitySelectorInventor,
};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteField {
    pub shape: Vec<u32>,
    pub values: Vec<i64>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FieldValueCollectionAdapter;

impl FieldValueCollectionAdapter {
    /// Lower distinct categorical values, never coordinates or benchmark
    /// concepts, into the shared collection-procedure contract.
    ///
    /// # Errors
    ///
    /// Rejects invalid fields, missing training evidence, shape mismatch, and
    /// cardinalities outside the bounded generic contract.
    pub fn lower(
        &self,
        training: &[(FiniteField, FiniteField)],
        prediction: &FiniteField,
    ) -> Result<CollectionProcedureProblem, FieldError> {
        prediction.validate()?;
        if training.is_empty() {
            return Err(FieldError::NoCorrespondence);
        }
        let demonstrations = training
            .iter()
            .map(|(input, output)| {
                input.validate()?;
                output.validate()?;
                if input.shape != output.shape {
                    return Err(FieldError::ObservationMismatch);
                }
                Ok(CollectionDemonstration {
                    input: input
                        .values
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                    output: output.values.iter().copied().collect(),
                })
            })
            .collect::<Result<Vec<_>, FieldError>>()?;
        let maximum = training
            .iter()
            .map(|(input, _)| input.values.iter().collect::<BTreeSet<_>>().len())
            .chain(std::iter::once(
                prediction.values.iter().collect::<BTreeSet<_>>().len(),
            ))
            .max()
            .ok_or(FieldError::Capacity)?;
        Ok(CollectionProcedureProblem {
            maximum_cardinality: u32::try_from(maximum).map_err(|_| FieldError::Capacity)?,
            demonstrations,
            prediction_input: prediction
                .values
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            maximum_predictions: 16,
        })
    }

    /// Reconstruct a field from an independently verified value binding.
    ///
    /// # Errors
    ///
    /// Rejects an invalid input field or a missing categorical binding.
    pub fn reconstruct(
        &self,
        input: &FiniteField,
        bindings: &BTreeMap<i64, i64>,
    ) -> Result<FiniteField, FieldError> {
        input.validate()?;
        let values = input
            .values
            .iter()
            .map(|value| {
                bindings
                    .get(value)
                    .copied()
                    .ok_or(FieldError::NoCorrespondence)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FiniteField {
            shape: input.shape.clone(),
            values,
        })
    }
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
    NoCorrespondence,
    OutOfBounds,
    NoEntity,
    AmbiguousEntity,
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
pub struct LearnedLocalFieldTheory {
    pub proposal: DefinitionProposal,
    pub training_cells: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LocalFieldLearner;

impl LocalFieldLearner {
    /// Lower demonstrations to the domain-neutral executable-definition
    /// contract. The returned problem can be learned either by fresh search or
    /// by the same verifier-gated mechanism memory used by other frontends.
    ///
    /// # Errors
    ///
    /// Rejects invalid geometry or observations outside the supported local
    /// and long-range relational evidence basis.
    pub fn learning_problem(
        &self,
        pairs: &[(FiniteField, FiniteField)],
        limits: InventionLimits,
    ) -> Result<InventionProblem, FieldError> {
        if pairs.is_empty() {
            return Err(FieldError::Invention(InventionError::EmptyDemonstrations));
        }
        for (input, output) in pairs {
            input.validate()?;
            output.validate()?;
            if input.shape != output.shape || input.shape.len() != 2 {
                return Err(FieldError::Cardinality);
            }
        }
        let (basic_demonstrations, changed, basic_supported) =
            collect_local_demonstrations(pairs, false)?;
        if !changed {
            return Err(FieldError::NoCorrespondence);
        }
        let basic_deterministic =
            demonstrations_are_deterministic(&basic_demonstrations, &basic_local_parameters());
        let (demonstrations, parameters) = if basic_supported && basic_deterministic {
            (basic_demonstrations, basic_local_parameters())
        } else {
            let (relational, _, relational_supported) = collect_local_demonstrations(pairs, true)?;
            if !relational_supported
                || !demonstrations_are_deterministic(&relational, &local_parameters())
            {
                return Err(FieldError::NoCorrespondence);
            }
            (relational, local_parameters())
        };
        Ok(InventionProblem {
            definition_name: "invented_local_transition".to_owned(),
            parameters,
            return_type: output_integer(),
            literals: (-2..=2).map(Value::Int).collect(),
            demonstrations,
            limits,
        })
    }

    /// Wrap an independently admitted invention result as an executable field
    /// theory. Admission already contains exact reference and optimized replay
    /// counts; no frontend-specific trust path is introduced here.
    #[must_use]
    pub fn from_invention(
        &self,
        outcome: cairn_invent::InventionOutcome,
    ) -> LearnedLocalFieldTheory {
        LearnedLocalFieldTheory {
            proposal: outcome.proposal,
            training_cells: outcome.admission.training_examples,
        }
    }

    /// Invent one local transition law from a cell, its bounded 8-neighborhood,
    /// coordinates, and extents. Out-of-bounds observations use the current
    /// modal-value hypothesis and complete replay remains mandatory.
    ///
    /// # Errors
    ///
    /// Rejects invalid geometry or bounded RKL invention failure.
    pub fn learn(
        &self,
        pairs: &[(FiniteField, FiniteField)],
        limits: InventionLimits,
    ) -> Result<LearnedLocalFieldTheory, FieldError> {
        let problem = self.learning_problem(pairs, limits)?;
        let outcome = DefinitionInventor
            .invent(&problem)
            .map_err(FieldError::Invention)?;
        Ok(self.from_invention(outcome))
    }

    /// Execute the frozen local law over a changed finite observation.
    ///
    /// # Errors
    ///
    /// Rejects invalid inputs or non-integer/failed predictions.
    pub fn predict(
        &self,
        theory: &LearnedLocalFieldTheory,
        input: &FiniteField,
        limits: cairn_rkl::ExecLimits,
    ) -> Result<FiniteField, FieldError> {
        input.validate()?;
        let background = modal_value(input);
        let relational = theory
            .proposal
            .definition
            .parameters
            .iter()
            .any(|parameter| parameter.name.0.starts_with("visible_"));
        let values = (0..input.values.len())
            .map(|index| {
                match DefinitionInventor
                    .predict(
                        &theory.proposal,
                        &local_bindings(input, index, background, relational)?,
                        limits,
                    )
                    .map_err(FieldError::Invention)?
                {
                    Value::Int(value) => Ok(value),
                    _ => Err(FieldError::Prediction),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FiniteField {
            shape: input.shape.clone(),
            values,
        })
    }
}

fn demonstrations_are_deterministic(
    demonstrations: &[BindingExample],
    parameters: &[Binder],
) -> bool {
    let mut outcomes = BTreeMap::<Vec<Value>, Value>::new();
    demonstrations.iter().all(|example| {
        let Some(inputs) = parameters
            .iter()
            .map(|parameter| example.inputs.get(&parameter.name).cloned())
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        if let Some(known) = outcomes.get(&inputs) {
            known == &example.output
        } else {
            outcomes.insert(inputs, example.output.clone());
            true
        }
    })
}

fn local_parameters() -> Vec<Binder> {
    [
        "center",
        "north",
        "south",
        "west",
        "east",
        "north_west",
        "north_east",
        "south_west",
        "south_east",
        "visible_north",
        "visible_south",
        "visible_west",
        "visible_east",
        "row_occupied_count",
        "column_occupied_count",
        "row",
        "column",
        "rows",
        "columns",
    ]
    .into_iter()
    .map(|name| Binder {
        name: VarName(name.to_owned()),
        ty: input_integer(),
    })
    .collect()
}

fn basic_local_parameters() -> Vec<Binder> {
    local_parameters()
        .into_iter()
        .filter(|parameter| {
            !matches!(
                parameter.name.0.as_str(),
                "visible_north"
                    | "visible_south"
                    | "visible_west"
                    | "visible_east"
                    | "row_occupied_count"
                    | "column_occupied_count"
            )
        })
        .collect()
}

fn collect_local_demonstrations(
    pairs: &[(FiniteField, FiniteField)],
    relational: bool,
) -> Result<(Vec<BindingExample>, bool, bool), FieldError> {
    let mut demonstrations = Vec::new();
    let mut changed = false;
    let mut supported = true;
    let support_count = if relational { 13 } else { 9 };
    let parameters = if relational {
        local_parameters()
    } else {
        basic_local_parameters()
    };
    for (input, output) in pairs {
        let background = modal_value(input);
        for (index, output_value) in output.values.iter().enumerate() {
            let inputs = local_bindings(input, index, background, relational)?;
            let observed = local_parameters()
                .into_iter()
                .take(support_count)
                .filter_map(|parameter| match inputs.get(&parameter.name) {
                    Some(Value::Int(value)) => Some(*value),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            changed |= input.values[index] != *output_value;
            supported &= observed.contains(output_value) || (-2..=2).contains(output_value);
            let mut filtered = RuntimeEnvironment::new();
            for parameter in &parameters {
                if let Some(value) = inputs.get(&parameter.name) {
                    filtered.insert(parameter.name.clone(), value.clone());
                }
            }
            demonstrations.push(BindingExample {
                inputs: filtered,
                output: Value::Int(*output_value),
            });
        }
    }
    Ok((demonstrations, changed, supported))
}

fn local_bindings(
    field: &FiniteField,
    index: usize,
    background: i64,
    relational: bool,
) -> Result<RuntimeEnvironment, FieldError> {
    let rows = usize::try_from(field.shape[0]).map_err(|_| FieldError::Capacity)?;
    let columns = usize::try_from(field.shape[1]).map_err(|_| FieldError::Capacity)?;
    let row = index / columns;
    let column = index % columns;
    let sample = |row_offset: isize, column_offset: isize| {
        let candidate_row = row.checked_add_signed(row_offset);
        let candidate_column = column.checked_add_signed(column_offset);
        match (candidate_row, candidate_column) {
            (Some(candidate_row), Some(candidate_column))
                if candidate_row < rows && candidate_column < columns =>
            {
                field.values[candidate_row * columns + candidate_column]
            }
            _ => background,
        }
    };
    let visible = |row_step: isize, column_step: isize| {
        let mut candidate_row = row;
        let mut candidate_column = column;
        loop {
            let Some(next_row) = candidate_row.checked_add_signed(row_step) else {
                return background;
            };
            let Some(next_column) = candidate_column.checked_add_signed(column_step) else {
                return background;
            };
            if next_row >= rows || next_column >= columns {
                return background;
            }
            candidate_row = next_row;
            candidate_column = next_column;
            let value = field.values[candidate_row * columns + candidate_column];
            if value != background {
                return value;
            }
        }
    };
    let basic_values = [
        field.values[index],
        sample(-1, 0),
        sample(1, 0),
        sample(0, -1),
        sample(0, 1),
        sample(-1, -1),
        sample(-1, 1),
        sample(1, -1),
        sample(1, 1),
        i64::try_from(row).map_err(|_| FieldError::Capacity)?,
        i64::try_from(column).map_err(|_| FieldError::Capacity)?,
        i64::from(field.shape[0]),
        i64::from(field.shape[1]),
    ];
    let mut environment = RuntimeEnvironment::new();
    for (parameter, value) in basic_local_parameters().into_iter().zip(basic_values) {
        environment.insert(parameter.name, Value::Int(value));
    }
    if relational {
        let row_occupied = field.values[row * columns..(row + 1) * columns]
            .iter()
            .filter(|value| **value != background)
            .count();
        let column_occupied = (0..rows)
            .filter(|candidate_row| field.values[candidate_row * columns + column] != background)
            .count();
        for (name, value) in [
            ("visible_north", visible(-1, 0)),
            ("visible_south", visible(1, 0)),
            ("visible_west", visible(0, -1)),
            ("visible_east", visible(0, 1)),
            (
                "row_occupied_count",
                i64::try_from(row_occupied).map_err(|_| FieldError::Capacity)?,
            ),
            (
                "column_occupied_count",
                i64::try_from(column_occupied).map_err(|_| FieldError::Capacity)?,
            ),
        ] {
            environment.insert(VarName(name.to_owned()), Value::Int(value));
        }
    }
    Ok(environment)
}

/// A learned output constructor expressed as four ordinary RKL scalar
/// definitions: two infer output extents and two map each output coordinate to
/// a source coordinate. No transform label or field-operation enum is stored.
#[derive(Clone, Debug, PartialEq)]
pub struct LearnedRenderBinding {
    pub output_extents: [DefinitionProposal; 2],
    pub source_coordinates: [DefinitionProposal; 2],
    pub training_correspondences: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearnedPartitionBroadcastBinding {
    pub controller_role: PartitionControllerRole,
    pub observed_partition_grids: BTreeSet<(usize, usize)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PartitionControllerRole {
    FixedIndex(usize),
    OmitsSharedMarker,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedPartitionExtractionBinding {
    pub selector: LearnedEntitySelector,
}

/// A domain-neutral entity selector rebound to connected field entities. The
/// selector is learned elsewhere; this binding only supplies exact structural
/// evidence and renders the selected entity in its native bounding frame.
#[derive(Clone, Debug, PartialEq)]
pub struct LearnedEntityFrameExtractionBinding {
    pub selector: LearnedEntitySelector,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LearnedStructuralFieldBinding {
    PartitionBroadcast(LearnedPartitionBroadcastBinding),
    PartitionExtraction(LearnedPartitionExtractionBinding),
    EntityFrameExtraction(LearnedEntityFrameExtractionBinding),
    LayerMechanism(LayerMechanism),
}

impl LearnedStructuralFieldBinding {
    /// Execute one learned structural rendering procedure.
    ///
    /// # Errors
    ///
    /// Rejects invalid geometry, failed role rebinding, or ambiguous rendering.
    pub fn predict(&self, input: &FiniteField) -> Result<FiniteField, FieldError> {
        match self {
            Self::PartitionBroadcast(binding) => PartitionBroadcastLearner.predict(binding, input),
            Self::PartitionExtraction(binding) => {
                PartitionExtractionLearner.predict(binding, input)
            }
            Self::EntityFrameExtraction(binding) => {
                EntityFrameExtractionAdapter.predict(binding, input)
            }
            Self::LayerMechanism(mechanism) => LayerFieldAdapter.predict(mechanism, input, 65_536),
        }
    }
}

/// Exact finite-field lowering and realization for modality-neutral learned
/// coordinate-layer mechanisms.
#[derive(Clone, Copy, Debug, Default)]
pub struct LayerFieldAdapter;

impl LayerFieldAdapter {
    /// Convert categorical non-background values into coordinate layers.
    ///
    /// # Errors
    ///
    /// Rejects malformed or over-capacity fields and coordinate conversion.
    pub fn lower(&self, input: &FiniteField) -> Result<LayerScene, FieldError> {
        input.validate()?;
        let background = modal_value(input);
        let mut layers = BTreeMap::<i64, BTreeSet<Vec<i64>>>::new();
        for (index, value) in input.values.iter().copied().enumerate() {
            if value == background {
                continue;
            }
            layers
                .entry(value)
                .or_default()
                .insert(index_coordinates(index, &input.shape)?);
        }
        if layers.is_empty() {
            return Err(FieldError::NoEntity);
        }
        Ok(LayerScene {
            extents: input.shape.clone(),
            background,
            layers: layers
                .into_iter()
                .map(|(value, points)| CoordinateLayer { value, points })
                .collect(),
        })
    }

    /// Realize a verified coordinate render as an exact categorical field.
    ///
    /// # Errors
    ///
    /// Rejects invalid extents, out-of-bounds points, and cardinality overflow.
    pub fn realize(&self, render: &LayerRender) -> Result<FiniteField, FieldError> {
        let cardinality = render.extents.iter().try_fold(1_usize, |total, extent| {
            total.checked_mul(usize::try_from(*extent).ok()?)
        });
        let mut values = vec![render.background; cardinality.ok_or(FieldError::Capacity)?];
        for point in &render.points {
            let index = coordinates_index(point, &render.extents)?;
            let target = values.get_mut(index).ok_or(FieldError::OutOfBounds)?;
            if *target != render.background && *target != render.foreground {
                return Err(FieldError::ResidualConflict);
            }
            *target = render.foreground;
        }
        let field = FiniteField {
            shape: render.extents.clone(),
            values,
        };
        field.validate()?;
        Ok(field)
    }

    /// Rebind and execute a learned layer mechanism on a current field.
    ///
    /// # Errors
    ///
    /// Rejects lowering, mechanism applicability, or realization failures.
    pub fn predict(
        &self,
        mechanism: &LayerMechanism,
        input: &FiniteField,
        maximum_points: usize,
    ) -> Result<FiniteField, FieldError> {
        let scene = self.lower(input)?;
        let render = LayerMechanismLearner
            .execute(mechanism, &scene, maximum_points)
            .map_err(|_| FieldError::Prediction)?;
        self.realize(&render)
    }
}

fn index_coordinates(mut index: usize, extents: &[u32]) -> Result<Vec<i64>, FieldError> {
    let mut coordinates = vec![0_i64; extents.len()];
    for axis in (0..extents.len()).rev() {
        let extent = usize::try_from(extents[axis]).map_err(|_| FieldError::Capacity)?;
        if extent == 0 {
            return Err(FieldError::EmptyAxis);
        }
        coordinates[axis] = i64::try_from(index % extent).map_err(|_| FieldError::Capacity)?;
        index /= extent;
    }
    if index != 0 {
        return Err(FieldError::OutOfBounds);
    }
    Ok(coordinates)
}

fn coordinates_index(point: &[i64], extents: &[u32]) -> Result<usize, FieldError> {
    if point.len() != extents.len() {
        return Err(FieldError::Cardinality);
    }
    point
        .iter()
        .zip(extents)
        .try_fold(0_usize, |index, (coordinate, extent)| {
            let coordinate = usize::try_from(*coordinate).map_err(|_| FieldError::OutOfBounds)?;
            let extent = usize::try_from(*extent).map_err(|_| FieldError::Capacity)?;
            if coordinate >= extent {
                return Err(FieldError::OutOfBounds);
            }
            index
                .checked_mul(extent)
                .and_then(|value| value.checked_add(coordinate))
                .ok_or(FieldError::Capacity)
        })
}

/// Exact frontend rebinding for selectors learned over generic relational
/// entity collections. This adapter does not invent or name a selection rule.
#[derive(Clone, Copy, Debug, Default)]
pub struct EntityFrameExtractionAdapter;

impl EntityFrameExtractionAdapter {
    /// Rebind a learned selector to connected field entities and render the
    /// uniquely selected entity's native frame.
    ///
    /// # Errors
    ///
    /// Rejects malformed fields, selector execution failure, or ambiguous
    /// selection. The selected result is always reconstructed from the current
    /// input and never stored in the learned binding.
    pub fn predict(
        &self,
        binding: &LearnedEntityFrameExtractionBinding,
        input: &FiniteField,
    ) -> Result<FiniteField, FieldError> {
        input.validate()?;
        let frames = entity_frames(input)?;
        let collection = frame_entity_collection(&frames)?;
        let selected = RelationalEntitySelectorInventor
            .predict(&binding.selector, &collection)
            .map_err(|_| FieldError::Prediction)?;
        let selected_frames = frames
            .iter()
            .enumerate()
            .filter_map(|(index, frame)| {
                selected.contains(&frame_entity_id(index)).then_some(frame)
            })
            .collect::<Vec<_>>();
        if selected_frames.len() != 1 {
            return Err(FieldError::AmbiguousEntity);
        }
        render_frame(input, selected_frames[0])
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PartitionExtractionLearner;

impl PartitionExtractionLearner {
    /// Learn an executable predicate selecting one structural partition whose
    /// native contents exactly explain each demonstrated output.
    ///
    /// # Errors
    ///
    /// Rejects insufficient, ambiguous, or malformed partition evidence and
    /// bounded relational-selector failure.
    pub fn learn(
        &self,
        pairs: &[(FiniteField, FiniteField)],
        limits: InventionLimits,
    ) -> Result<LearnedPartitionExtractionBinding, FieldError> {
        if pairs.len() < 2 {
            return Err(FieldError::NoCorrespondence);
        }
        let demonstrations = pairs
            .iter()
            .enumerate()
            .map(|(episode, (input, output))| {
                let (collection, panels) = panel_entities(input, episode)?;
                let matches = panels
                    .iter()
                    .enumerate()
                    .filter_map(|(index, panel)| (panel == output).then_some(index))
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return Err(FieldError::AmbiguousEntity);
                }
                Ok(EntitySelectionDemonstration {
                    collection,
                    selected: BTreeSet::from([panel_id(episode, matches[0])]),
                })
            })
            .collect::<Result<Vec<_>, FieldError>>()?;
        let prediction = demonstrations[0].collection.clone();
        let selector = RelationalEntitySelectorInventor
            .invent(
                &EntitySelectionProblem {
                    demonstrations,
                    prediction,
                },
                limits,
            )
            .map_err(|_| FieldError::NoEntity)?;
        Ok(LearnedPartitionExtractionBinding { selector })
    }

    /// Rebind the learned panel predicate and render the unique selection.
    ///
    /// # Errors
    ///
    /// Rejects malformed partitions, selector failure, or non-unique selection.
    pub fn predict(
        &self,
        binding: &LearnedPartitionExtractionBinding,
        input: &FiniteField,
    ) -> Result<FiniteField, FieldError> {
        let (collection, panels) = panel_entities(input, 0)?;
        let selected = RelationalEntitySelectorInventor
            .predict(&binding.selector, &collection)
            .map_err(|_| FieldError::Prediction)?;
        let matches = panels
            .iter()
            .enumerate()
            .filter_map(|(index, panel)| {
                selected
                    .contains(&panel_id(0, index))
                    .then_some(panel.clone())
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(FieldError::AmbiguousEntity);
        }
        Ok(matches[0].clone())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PartitionBroadcastLearner;

impl PartitionBroadcastLearner {
    /// Learn which structural partition acts as a cellwise controller for a
    /// grid of equally shaped payload partitions.
    ///
    /// # Errors
    ///
    /// Rejects absent/ambiguous partitions, incompatible geometry, or failure
    /// to replay every source output exactly.
    pub fn learn(
        &self,
        pairs: &[(FiniteField, FiniteField)],
    ) -> Result<LearnedPartitionBroadcastBinding, FieldError> {
        if pairs.len() < 2 {
            return Err(FieldError::NoCorrespondence);
        }
        let layouts = pairs
            .iter()
            .map(|(input, output)| {
                input.validate()?;
                output.validate()?;
                if input.shape != output.shape {
                    return Err(FieldError::ObservationMismatch);
                }
                partition_layout(input)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let maximum = layouts
            .iter()
            .map(|layout| layout.row_ranges.len() * layout.column_ranges.len())
            .min()
            .ok_or(FieldError::NoCorrespondence)?;
        let candidates = (0..maximum)
            .map(PartitionControllerRole::FixedIndex)
            .chain(std::iter::once(PartitionControllerRole::OmitsSharedMarker))
            .filter(|role| {
                pairs.iter().all(|(input, output)| {
                    render_partition_broadcast(input, role).as_ref() == Ok(output)
                })
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(FieldError::NoCorrespondence);
        }
        Ok(LearnedPartitionBroadcastBinding {
            controller_role: candidates[0].clone(),
            observed_partition_grids: layouts
                .iter()
                .map(|layout| (layout.row_ranges.len(), layout.column_ranges.len()))
                .collect(),
        })
    }

    /// Rebind the learned controller role to newly inferred partitions and
    /// render the resulting finite field.
    ///
    /// # Errors
    ///
    /// Rejects malformed partition geometry, out-of-range controller roles, or
    /// invalid fields.
    pub fn predict(
        &self,
        binding: &LearnedPartitionBroadcastBinding,
        input: &FiniteField,
    ) -> Result<FiniteField, FieldError> {
        render_partition_broadcast(input, &binding.controller_role)
    }
}

#[derive(Clone, Debug)]
struct PartitionLayout {
    separator: i64,
    separator_rows: BTreeSet<usize>,
    separator_columns: BTreeSet<usize>,
    row_ranges: Vec<(usize, usize)>,
    column_ranges: Vec<(usize, usize)>,
}

fn partition_layout(field: &FiniteField) -> Result<PartitionLayout, FieldError> {
    field.validate()?;
    if field.shape.len() != 2 {
        return Err(FieldError::Cardinality);
    }
    let rows = usize::try_from(field.shape[0]).map_err(|_| FieldError::Capacity)?;
    let columns = usize::try_from(field.shape[1]).map_err(|_| FieldError::Capacity)?;
    let separator_rows = (0..rows)
        .filter(|row| {
            let first = field.values[row * columns];
            field.values[row * columns..(row + 1) * columns]
                .iter()
                .all(|value| *value == first)
        })
        .collect::<BTreeSet<_>>();
    let separator_columns = (0..columns)
        .filter(|column| {
            let first = field.values[*column];
            (0..rows).all(|row| field.values[row * columns + column] == first)
        })
        .collect::<BTreeSet<_>>();
    let separator = separator_rows
        .iter()
        .map(|row| field.values[row * columns])
        .chain(separator_columns.iter().map(|column| field.values[*column]))
        .next()
        .ok_or(FieldError::NoCorrespondence)?;
    if separator_rows
        .iter()
        .any(|row| field.values[row * columns] != separator)
        || separator_columns
            .iter()
            .any(|column| field.values[*column] != separator)
    {
        return Err(FieldError::NoCorrespondence);
    }
    let row_ranges = nonseparator_ranges(rows, &separator_rows);
    let column_ranges = nonseparator_ranges(columns, &separator_columns);
    let row_width = row_ranges.first().map(|(start, end)| end - start);
    let column_width = column_ranges.first().map(|(start, end)| end - start);
    if row_width.is_none()
        || column_width.is_none()
        || row_ranges
            .iter()
            .any(|(start, end)| Some(end - start) != row_width)
        || column_ranges
            .iter()
            .any(|(start, end)| Some(end - start) != column_width)
    {
        return Err(FieldError::NoCorrespondence);
    }
    Ok(PartitionLayout {
        separator,
        separator_rows,
        separator_columns,
        row_ranges,
        column_ranges,
    })
}

fn nonseparator_ranges(extent: usize, separators: &BTreeSet<usize>) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = None;
    for index in 0..=extent {
        if index < extent && !separators.contains(&index) {
            start.get_or_insert(index);
        } else if let Some(start) = start.take() {
            ranges.push((start, index));
        }
    }
    ranges
}

fn render_partition_broadcast(
    input: &FiniteField,
    controller_role: &PartitionControllerRole,
) -> Result<FiniteField, FieldError> {
    let layout = partition_layout(input)?;
    let columns = usize::try_from(input.shape[1]).map_err(|_| FieldError::Capacity)?;
    let partition_columns = layout.column_ranges.len();
    let controller_height = layout.row_ranges[0].1 - layout.row_ranges[0].0;
    let controller_width = layout.column_ranges[0].1 - layout.column_ranges[0].0;
    if controller_height != layout.row_ranges.len()
        || controller_width != layout.column_ranges.len()
    {
        return Err(FieldError::NoCorrespondence);
    }
    let controller_partition = match controller_role {
        PartitionControllerRole::FixedIndex(index) => *index,
        PartitionControllerRole::OmitsSharedMarker => {
            omitted_shared_marker_partition(input, &layout)?
        }
    };
    let controller_row = controller_partition / partition_columns;
    let controller_column = controller_partition % partition_columns;
    let Some((controller_row_start, _)) = layout.row_ranges.get(controller_row) else {
        return Err(FieldError::NoCorrespondence);
    };
    let Some((controller_column_start, _)) = layout.column_ranges.get(controller_column) else {
        return Err(FieldError::NoCorrespondence);
    };
    let mut values = vec![layout.separator; input.values.len()];
    for (panel_row, (row_start, row_end)) in layout.row_ranges.iter().enumerate() {
        for (panel_column, (column_start, column_end)) in layout.column_ranges.iter().enumerate() {
            let control = input.values[(controller_row_start + panel_row) * columns
                + controller_column_start
                + panel_column];
            for row in *row_start..*row_end {
                for column in *column_start..*column_end {
                    values[row * columns + column] = control;
                }
            }
        }
    }
    for row in layout.separator_rows {
        for column in 0..columns {
            values[row * columns + column] = layout.separator;
        }
    }
    for column in layout.separator_columns {
        for row in 0..usize::try_from(input.shape[0]).map_err(|_| FieldError::Capacity)? {
            values[row * columns + column] = layout.separator;
        }
    }
    Ok(FiniteField {
        shape: input.shape.clone(),
        values,
    })
}

fn panel_entities(
    input: &FiniteField,
    episode: usize,
) -> Result<(RelationalEntityCollection, Vec<FiniteField>), FieldError> {
    let layout = partition_layout(input)?;
    let columns = usize::try_from(input.shape[1]).map_err(|_| FieldError::Capacity)?;
    let background = modal_value(input);
    let mut entities = Vec::new();
    let mut panels = Vec::new();
    for (panel_row, (row_start, row_end)) in layout.row_ranges.iter().enumerate() {
        for (panel_column, (column_start, column_end)) in layout.column_ranges.iter().enumerate() {
            let values = (*row_start..*row_end)
                .flat_map(|row| {
                    (*column_start..*column_end)
                        .map(move |column| input.values[row * columns + column])
                })
                .collect::<Vec<_>>();
            let index = panel_row * layout.column_ranges.len() + panel_column;
            entities.push(EntityRecord {
                id: panel_id(episode, index),
                attributes: BTreeMap::from([
                    (
                        FeatureId("occupied_count".to_owned()),
                        i64::try_from(values.iter().filter(|value| **value != background).count())
                            .map_err(|_| FieldError::Capacity)?,
                    ),
                    (
                        FeatureId("distinct_count".to_owned()),
                        i64::try_from(values.iter().copied().collect::<BTreeSet<_>>().len())
                            .map_err(|_| FieldError::Capacity)?,
                    ),
                    (
                        FeatureId("panel_row".to_owned()),
                        i64::try_from(panel_row).map_err(|_| FieldError::Capacity)?,
                    ),
                    (
                        FeatureId("panel_column".to_owned()),
                        i64::try_from(panel_column).map_err(|_| FieldError::Capacity)?,
                    ),
                ]),
            });
            panels.push(FiniteField {
                shape: vec![
                    u32::try_from(row_end - row_start).map_err(|_| FieldError::Capacity)?,
                    u32::try_from(column_end - column_start).map_err(|_| FieldError::Capacity)?,
                ],
                values,
            });
        }
    }
    let occupied_key = FeatureId("occupied_count".to_owned());
    let distinct_key = FeatureId("distinct_count".to_owned());
    let maximum_occupied = entities
        .iter()
        .filter_map(|entity| entity.attributes.get(&occupied_key))
        .copied()
        .max()
        .ok_or(FieldError::NoEntity)?;
    let maximum_distinct = entities
        .iter()
        .filter_map(|entity| entity.attributes.get(&distinct_key))
        .copied()
        .max()
        .ok_or(FieldError::NoEntity)?;
    for entity in &mut entities {
        entity.attributes.insert(
            FeatureId("has_maximum_occupied".to_owned()),
            i64::from(entity.attributes[&occupied_key] == maximum_occupied),
        );
        entity.attributes.insert(
            FeatureId("has_maximum_distinct".to_owned()),
            i64::from(entity.attributes[&distinct_key] == maximum_distinct),
        );
    }
    Ok((
        RelationalEntityCollection {
            entities,
            relations: Vec::new(),
        },
        panels,
    ))
}

fn panel_id(episode: usize, index: usize) -> EntityId {
    EntityId(Hash32::digest(
        format!("panel:{episode}:{index}").as_bytes(),
    ))
}

fn omitted_shared_marker_partition(
    input: &FiniteField,
    layout: &PartitionLayout,
) -> Result<usize, FieldError> {
    let columns = usize::try_from(input.shape[1]).map_err(|_| FieldError::Capacity)?;
    let panels = layout
        .row_ranges
        .iter()
        .flat_map(|(row_start, row_end)| {
            layout
                .column_ranges
                .iter()
                .map(move |(column_start, column_end)| {
                    (*row_start..*row_end)
                        .flat_map(|row| {
                            (*column_start..*column_end)
                                .map(move |column| input.values[row * columns + column])
                        })
                        .collect::<BTreeSet<_>>()
                })
        })
        .collect::<Vec<_>>();
    let all_values = panels.iter().flatten().copied().collect::<BTreeSet<_>>();
    let missing = all_values
        .iter()
        .filter_map(|value| {
            let absent = panels
                .iter()
                .enumerate()
                .filter_map(|(index, panel)| (!panel.contains(value)).then_some(index))
                .collect::<Vec<_>>();
            if absent.len() == 1 {
                Some(absent[0])
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    if missing.len() != 1 {
        return Err(FieldError::AmbiguousEntity);
    }
    missing.into_iter().next().ok_or(FieldError::NoEntity)
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedRemappedFieldTheory {
    pub binding: LearnedRenderBinding,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RemappedFieldLearner;

impl RemappedFieldLearner {
    /// Infer output geometry and a coordinate-conditioned source sampler from
    /// unambiguous value correspondences, then replay the complete outputs.
    ///
    /// # Errors
    ///
    /// Rejects non-2D data, absent unambiguous correspondences, bounded RKL
    /// invention failure, out-of-range sampling, or incomplete replay.
    pub fn learn(
        &self,
        pairs: &[(FiniteField, FiniteField)],
        limits: InventionLimits,
    ) -> Result<LearnedRemappedFieldTheory, FieldError> {
        if pairs.is_empty() {
            return Err(FieldError::Invention(InventionError::EmptyDemonstrations));
        }
        for (input, output) in pairs {
            input.validate()?;
            output.validate()?;
            if input.shape.len() != 2 || output.shape.len() != 2 {
                return Err(FieldError::Cardinality);
            }
        }
        let output_rows = invent_extent(pairs, 0, limits)?;
        let output_columns = invent_extent(pairs, 1, limits)?;
        let (source_row, source_column, correspondence_count) =
            invent_affine_sampler(pairs, limits)?;
        let theory = LearnedRemappedFieldTheory {
            binding: LearnedRenderBinding {
                output_extents: [output_rows, output_columns],
                source_coordinates: [source_row, source_column],
                training_correspondences: correspondence_count,
            },
        };
        for (input, output) in pairs {
            if self.predict(&theory, input, limits.execution)? != *output {
                return Err(FieldError::Prediction);
            }
        }
        Ok(theory)
    }

    /// Execute the learned shape and coordinate definitions on a new field.
    ///
    /// # Errors
    ///
    /// Rejects invalid inputs, non-integer dimensions/coordinates, capacity,
    /// or sampling outside the observed field.
    pub fn predict(
        &self,
        theory: &LearnedRemappedFieldTheory,
        input: &FiniteField,
        limits: cairn_rkl::ExecLimits,
    ) -> Result<FiniteField, FieldError> {
        input.validate()?;
        if input.shape.len() != 2 {
            return Err(FieldError::Cardinality);
        }
        let shape_inputs = shape_bindings(input);
        let output_rows =
            predict_nonnegative(&theory.binding.output_extents[0], &shape_inputs, limits)?;
        let output_columns =
            predict_nonnegative(&theory.binding.output_extents[1], &shape_inputs, limits)?;
        if output_rows == 0 || output_columns == 0 {
            return Err(FieldError::EmptyAxis);
        }
        let cardinality = output_rows
            .checked_mul(output_columns)
            .ok_or(FieldError::Capacity)?;
        let mut values = Vec::with_capacity(cardinality);
        for row in 0..output_rows {
            for column in 0..output_columns {
                let bindings =
                    coordinate_bindings(input, row, column, output_rows, output_columns)?;
                let source_row =
                    predict_nonnegative(&theory.binding.source_coordinates[0], &bindings, limits)?;
                let source_column =
                    predict_nonnegative(&theory.binding.source_coordinates[1], &bindings, limits)?;
                let input_columns =
                    usize::try_from(input.shape[1]).map_err(|_| FieldError::Capacity)?;
                if source_row
                    >= usize::try_from(input.shape[0]).map_err(|_| FieldError::Capacity)?
                    || source_column >= input_columns
                {
                    return Err(FieldError::OutOfBounds);
                }
                values.push(input.values[source_row * input_columns + source_column]);
            }
        }
        Ok(FiniteField {
            shape: vec![
                u32::try_from(output_rows).map_err(|_| FieldError::Capacity)?,
                u32::try_from(output_columns).map_err(|_| FieldError::Capacity)?,
            ],
            values,
        })
    }
}

fn invent_extent(
    pairs: &[(FiniteField, FiniteField)],
    axis: usize,
    limits: InventionLimits,
) -> Result<DefinitionProposal, FieldError> {
    let demonstrations = pairs
        .iter()
        .map(|(input, output)| {
            Ok(BindingExample {
                inputs: shape_bindings(input),
                output: Value::Int(i64::from(output.shape[axis])),
            })
        })
        .collect::<Result<Vec<_>, FieldError>>()?;
    DefinitionInventor
        .invent(&InventionProblem {
            definition_name: format!("invented_output_extent_{axis}"),
            parameters: shape_parameters(),
            return_type: coordinate_integer(),
            literals: (-2..=2).map(Value::Int).collect(),
            demonstrations,
            limits,
        })
        .map(|outcome| outcome.proposal)
        .map_err(FieldError::Invention)
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AffineExpression {
    coefficients: Vec<i8>,
    constant: i8,
}

#[derive(Clone)]
struct SamplerLabel {
    inputs: RuntimeEnvironment,
    source_row: i64,
    source_column: i64,
}

fn invent_affine_sampler(
    pairs: &[(FiniteField, FiniteField)],
    limits: InventionLimits,
) -> Result<(DefinitionProposal, DefinitionProposal, usize), FieldError> {
    let observed_values = pairs
        .iter()
        .flat_map(|(input, _)| input.values.iter().copied())
        .collect::<BTreeSet<_>>();
    if observed_values.len() < 2 {
        return Err(FieldError::NoCorrespondence);
    }
    let rows = affine_candidates(0);
    let columns = affine_candidates(1);
    let mut evaluated = 0_u64;
    for diagonal in 0..rows.len().saturating_add(columns.len()) {
        for (row_index, row) in rows
            .iter()
            .enumerate()
            .take(diagonal.min(rows.len().saturating_sub(1)) + 1)
        {
            let Some(column_index) = diagonal.checked_sub(row_index) else {
                continue;
            };
            if column_index >= columns.len() {
                continue;
            }
            evaluated = evaluated.saturating_add(1);
            if evaluated > limits.maximum_candidates {
                return Err(FieldError::Invention(InventionError::CandidateCapacity {
                    maximum: limits.maximum_candidates,
                }));
            }
            let Some(labels) = sampler_labels(pairs, row, &columns[column_index])? else {
                continue;
            };
            let row = admit_affine_coordinate(row, &labels, 0, evaluated, limits)?;
            let column =
                admit_affine_coordinate(&columns[column_index], &labels, 1, evaluated, limits)?;
            return Ok((row, column, labels.len()));
        }
    }
    Err(FieldError::NoCorrespondence)
}

fn affine_candidates(axis: usize) -> Vec<AffineExpression> {
    let width = coordinate_parameters().len();
    let mut candidates = BTreeSet::new();
    for constant in -1_i8..=1 {
        candidates.insert(AffineExpression {
            coefficients: vec![0; width],
            constant,
        });
    }
    for first in 0..width {
        add_affine_terms(&mut candidates, width, first, None);
        for second in first + 1..width {
            add_affine_terms(&mut candidates, width, first, Some(second));
        }
    }
    let mut candidates = candidates.into_iter().collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| affine_rank(candidate, axis));
    candidates
}

fn add_affine_terms(
    candidates: &mut BTreeSet<AffineExpression>,
    width: usize,
    first: usize,
    second: Option<usize>,
) {
    for first_coefficient in [-1_i8, 1] {
        for second_coefficient in if second.is_some() {
            vec![-1_i8, 1]
        } else {
            vec![0]
        } {
            for constant in -1_i8..=1 {
                let mut coefficients = vec![0; width];
                coefficients[first] = first_coefficient;
                if let Some(second) = second {
                    coefficients[second] = second_coefficient;
                }
                candidates.insert(AffineExpression {
                    coefficients,
                    constant,
                });
            }
        }
    }
}

fn affine_rank(candidate: &AffineExpression, axis: usize) -> (u32, u32, u32, Vec<i8>, i8) {
    let active = u32::try_from(
        candidate
            .coefficients
            .iter()
            .filter(|coefficient| **coefficient != 0)
            .count(),
    )
    .unwrap_or(u32::MAX);
    let cross_axis = u32::try_from(
        candidate
            .coefficients
            .iter()
            .enumerate()
            .filter(|(index, coefficient)| **coefficient != 0 && index % 2 != axis)
            .count(),
    )
    .unwrap_or(u32::MAX);
    let role_order = [
        axis,
        4 + axis,
        2 + axis,
        6 + axis,
        8 + axis,
        1 - axis,
        5 - axis,
        3 - axis,
        7 - axis,
        9 - axis,
    ];
    let role_penalty = candidate
        .coefficients
        .iter()
        .enumerate()
        .filter(|(_, coefficient)| **coefficient != 0)
        .map(|(index, _)| {
            u32::try_from(
                role_order
                    .iter()
                    .position(|candidate| *candidate == index)
                    .unwrap_or(role_order.len()),
            )
            .unwrap_or(u32::MAX)
        })
        .sum();
    (
        active
            .saturating_mul(4)
            .saturating_add(u32::from(candidate.constant.unsigned_abs())),
        cross_axis,
        role_penalty,
        candidate.coefficients.clone(),
        candidate.constant,
    )
}

fn sampler_labels(
    pairs: &[(FiniteField, FiniteField)],
    row_expression: &AffineExpression,
    column_expression: &AffineExpression,
) -> Result<Option<Vec<SamplerLabel>>, FieldError> {
    let mut labels = Vec::new();
    for (input, output) in pairs {
        let output_rows = usize::try_from(output.shape[0]).map_err(|_| FieldError::Capacity)?;
        let output_columns = usize::try_from(output.shape[1]).map_err(|_| FieldError::Capacity)?;
        for output_row in 0..output_rows {
            for output_column in 0..output_columns {
                let inputs = coordinate_bindings(
                    input,
                    output_row,
                    output_column,
                    output_rows,
                    output_columns,
                )?;
                let source_row = evaluate_affine(row_expression, &inputs)?;
                let source_column = evaluate_affine(column_expression, &inputs)?;
                let Some(source) = source_index(input, source_row, source_column)? else {
                    return Ok(None);
                };
                let target = output_row * output_columns + output_column;
                if input.values[source] != output.values[target] {
                    return Ok(None);
                }
                labels.push(SamplerLabel {
                    inputs,
                    source_row,
                    source_column,
                });
            }
        }
    }
    Ok(Some(labels))
}

fn source_index(input: &FiniteField, row: i64, column: i64) -> Result<Option<usize>, FieldError> {
    if row < 0
        || column < 0
        || row >= i64::from(input.shape[0])
        || column >= i64::from(input.shape[1])
    {
        return Ok(None);
    }
    let columns = usize::try_from(input.shape[1]).map_err(|_| FieldError::Capacity)?;
    Ok(Some(
        usize::try_from(row).map_err(|_| FieldError::Capacity)? * columns
            + usize::try_from(column).map_err(|_| FieldError::Capacity)?,
    ))
}

fn admit_affine_coordinate(
    expression: &AffineExpression,
    labels: &[SamplerLabel],
    axis: usize,
    evaluated: u64,
    limits: InventionLimits,
) -> Result<DefinitionProposal, FieldError> {
    let problem = InventionProblem {
        definition_name: format!("invented_source_coordinate_{axis}"),
        parameters: coordinate_parameters(),
        return_type: scalar_integer(),
        literals: Vec::new(),
        demonstrations: labels
            .iter()
            .map(|label| BindingExample {
                inputs: label.inputs.clone(),
                output: Value::Int(if axis == 0 {
                    label.source_row
                } else {
                    label.source_column
                }),
            })
            .collect(),
        limits,
    };
    DefinitionInventor
        .admit_definition(
            &problem,
            cairn_rkl::Definition {
                name: problem.definition_name.clone(),
                parameters: problem.parameters.clone(),
                return_type: problem.return_type.clone(),
                body: affine_rkl_expression(expression),
                recursion: None,
                proof_annotations: Vec::new(),
            },
            affine_cost(expression),
            evaluated,
        )
        .map(|outcome| outcome.proposal)
        .map_err(FieldError::Invention)
}

fn evaluate_affine(
    expression: &AffineExpression,
    inputs: &RuntimeEnvironment,
) -> Result<i64, FieldError> {
    coordinate_parameters()
        .iter()
        .zip(&expression.coefficients)
        .try_fold(
            i64::from(expression.constant),
            |total, (parameter, coefficient)| {
                let Some(Value::Int(value)) = inputs.get(&parameter.name) else {
                    return Err(FieldError::Prediction);
                };
                total
                    .checked_add(value.saturating_mul(i64::from(*coefficient)))
                    .ok_or(FieldError::Capacity)
            },
        )
}

fn affine_rkl_expression(expression: &AffineExpression) -> cairn_rkl::Expr {
    let mut terms = coordinate_parameters()
        .into_iter()
        .zip(&expression.coefficients)
        .filter_map(|(parameter, coefficient)| match coefficient {
            1 => Some(cairn_rkl::Expr::Var(parameter.name)),
            -1 => Some(cairn_rkl::Expr::Unary {
                op: cairn_rkl::UnaryOp::Negate,
                argument: Box::new(cairn_rkl::Expr::Var(parameter.name)),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    if expression.constant != 0 || terms.is_empty() {
        terms.push(cairn_rkl::Expr::Int(i64::from(expression.constant)));
    }
    terms
        .into_iter()
        .reduce(|left, right| cairn_rkl::Expr::Binary {
            op: cairn_rkl::BinaryOp::Add,
            left: Box::new(left),
            right: Box::new(right),
        })
        .unwrap_or(cairn_rkl::Expr::Int(0))
}

fn affine_cost(expression: &AffineExpression) -> u16 {
    let terms = expression
        .coefficients
        .iter()
        .filter(|coefficient| **coefficient != 0)
        .count()
        + usize::from(expression.constant != 0);
    u16::try_from(terms.saturating_mul(2).saturating_sub(1)).unwrap_or(u16::MAX)
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearnedEntityFrameTheory {
    pub selector: DefinitionProposal,
    pub positive_frames: usize,
    pub negative_frames: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EntityFrameLearner;

impl EntityFrameLearner {
    /// Invent a predicate selecting one connected entity frame, requiring its
    /// bounded rendering to replay every demonstrated output exactly.
    ///
    /// # Errors
    ///
    /// Rejects missing/ambiguous target frames, absent contrast, bounded
    /// predicate invention failure, or non-unique selection at prediction.
    pub fn learn(
        &self,
        pairs: &[(FiniteField, FiniteField)],
        limits: InventionLimits,
    ) -> Result<LearnedEntityFrameTheory, FieldError> {
        let mut examples = Vec::new();
        let mut positives = 0_usize;
        let mut negatives = 0_usize;
        for (input, output) in pairs {
            input.validate()?;
            output.validate()?;
            let frames = entity_frames(input)?;
            let matching = frames
                .iter()
                .filter(|frame| render_frame(input, frame).as_ref() == Ok(output))
                .count();
            if matching == 0 {
                return Err(FieldError::NoEntity);
            }
            if matching != 1 {
                return Err(FieldError::AmbiguousEntity);
            }
            for frame in frames {
                let selected = render_frame(input, &frame).as_ref() == Ok(output);
                positives += usize::from(selected);
                negatives += usize::from(!selected);
                examples.push(ContrastExample {
                    features: frame_features(&frame),
                    selected,
                });
            }
        }
        if positives == 0 || negatives == 0 {
            return Err(FieldError::NoEntity);
        }
        let selector = ContrastPredicateInventor
            .invent(
                "invented_entity_selector",
                &frame_feature_types(),
                &examples,
                vec![Value::Int(0), Value::Int(1)],
                limits,
            )
            .map_err(|_| FieldError::NoEntity)?;
        Ok(LearnedEntityFrameTheory {
            selector: selector.proposal,
            positive_frames: positives,
            negative_frames: negatives,
        })
    }

    /// Apply the invented selector to new entity hypotheses and render the
    /// uniquely selected bounded observation.
    ///
    /// # Errors
    ///
    /// Rejects invalid evidence, selector execution failure, or zero/multiple
    /// selected frames.
    pub fn predict(
        &self,
        theory: &LearnedEntityFrameTheory,
        input: &FiniteField,
        limits: cairn_rkl::ExecLimits,
    ) -> Result<FiniteField, FieldError> {
        input.validate()?;
        let selected = entity_frames(input)?
            .into_iter()
            .filter_map(|frame| {
                let mut environment = RuntimeEnvironment::new();
                for (name, value) in frame_features(&frame) {
                    environment.insert(name, value);
                }
                match DefinitionInventor.predict(&theory.selector, &environment, limits) {
                    Ok(Value::Bool(true)) => Some(Ok(frame)),
                    Ok(Value::Bool(false)) => None,
                    _ => Some(Err(FieldError::Prediction)),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        if selected.len() != 1 {
            return Err(FieldError::AmbiguousEntity);
        }
        render_frame(input, &selected[0])
    }
}

#[derive(Clone, Debug)]
struct EntityFrame {
    min_row: usize,
    min_column: usize,
    rows: usize,
    columns: usize,
    cell_count: usize,
    distinct_values: usize,
    touches_boundary: bool,
}

fn entity_frames(field: &FiniteField) -> Result<Vec<EntityFrame>, FieldError> {
    if field.shape.len() != 2 {
        return Err(FieldError::Cardinality);
    }
    let rows = usize::try_from(field.shape[0]).map_err(|_| FieldError::Capacity)?;
    let columns = usize::try_from(field.shape[1]).map_err(|_| FieldError::Capacity)?;
    let background = modal_value(field);
    let mut unseen = field
        .values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (*value != background).then_some(index))
        .collect::<BTreeSet<_>>();
    let mut frames = Vec::new();
    while let Some(seed) = unseen.pop_first() {
        let mut members = BTreeSet::from([seed]);
        let mut queue = VecDeque::from([seed]);
        while let Some(index) = queue.pop_front() {
            let row = index / columns;
            let column = index % columns;
            for neighbor in [
                (row > 0).then(|| index - columns),
                (row + 1 < rows).then(|| index + columns),
                (column > 0).then(|| index - 1),
                (column + 1 < columns).then(|| index + 1),
            ]
            .into_iter()
            .flatten()
            {
                if unseen.remove(&neighbor) {
                    members.insert(neighbor);
                    queue.push_back(neighbor);
                }
            }
        }
        let min_row = members
            .iter()
            .map(|index| index / columns)
            .min()
            .unwrap_or(0);
        let max_row = members
            .iter()
            .map(|index| index / columns)
            .max()
            .unwrap_or(0);
        let min_column = members
            .iter()
            .map(|index| index % columns)
            .min()
            .unwrap_or(0);
        let max_column = members
            .iter()
            .map(|index| index % columns)
            .max()
            .unwrap_or(0);
        frames.push(EntityFrame {
            min_row,
            min_column,
            rows: max_row - min_row + 1,
            columns: max_column - min_column + 1,
            cell_count: members.len(),
            distinct_values: members
                .iter()
                .map(|index| field.values[*index])
                .collect::<BTreeSet<_>>()
                .len(),
            touches_boundary: min_row == 0
                || min_column == 0
                || max_row + 1 == rows
                || max_column + 1 == columns,
        });
    }
    Ok(frames)
}

fn frame_entity_collection(
    frames: &[EntityFrame],
) -> Result<RelationalEntityCollection, FieldError> {
    let maximum_occupied = frames
        .iter()
        .map(|frame| frame.cell_count)
        .max()
        .ok_or(FieldError::NoEntity)?;
    let maximum_distinct = frames
        .iter()
        .map(|frame| frame.distinct_values)
        .max()
        .ok_or(FieldError::NoEntity)?;
    let entities = frames
        .iter()
        .enumerate()
        .map(|(index, frame)| EntityRecord {
            id: frame_entity_id(index),
            attributes: BTreeMap::from([
                (
                    FeatureId("has_maximum_occupied".to_owned()),
                    i64::from(frame.cell_count == maximum_occupied),
                ),
                (
                    FeatureId("has_maximum_distinct".to_owned()),
                    i64::from(frame.distinct_values == maximum_distinct),
                ),
            ]),
        })
        .collect();
    Ok(RelationalEntityCollection {
        entities,
        relations: Vec::new(),
    })
}

fn frame_entity_id(index: usize) -> EntityId {
    EntityId(Hash32::digest(
        format!("connected-entity:{index}").as_bytes(),
    ))
}

fn render_frame(field: &FiniteField, frame: &EntityFrame) -> Result<FiniteField, FieldError> {
    let columns = usize::try_from(field.shape[1]).map_err(|_| FieldError::Capacity)?;
    let mut values = Vec::with_capacity(frame.rows.saturating_mul(frame.columns));
    for row in frame.min_row..frame.min_row + frame.rows {
        for column in frame.min_column..frame.min_column + frame.columns {
            values.push(field.values[row * columns + column]);
        }
    }
    Ok(FiniteField {
        shape: vec![
            u32::try_from(frame.rows).map_err(|_| FieldError::Capacity)?,
            u32::try_from(frame.columns).map_err(|_| FieldError::Capacity)?,
        ],
        values,
    })
}

fn frame_feature_types() -> Vec<(VarName, Type)> {
    [
        "min_row",
        "min_column",
        "rows",
        "columns",
        "cell_count",
        "distinct_values",
        "touches_boundary",
    ]
    .into_iter()
    .map(|name| (VarName(name.to_owned()), coordinate_integer()))
    .collect()
}

fn frame_features(frame: &EntityFrame) -> BTreeMap<VarName, Value> {
    let values = [
        frame.min_row,
        frame.min_column,
        frame.rows,
        frame.columns,
        frame.cell_count,
        frame.distinct_values,
        usize::from(frame.touches_boundary),
    ];
    frame_feature_types()
        .into_iter()
        .zip(values)
        .map(|((name, _), value)| (name, Value::Int(i64::try_from(value).unwrap_or(i64::MAX))))
        .collect()
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

fn coordinate_integer() -> Type {
    Type::Int(IntRange {
        lower: 0,
        upper: i64::from(u16::MAX),
    })
}

fn scalar_integer() -> Type {
    Type::Int(IntRange {
        lower: i64::from(i32::MIN),
        upper: i64::from(i32::MAX),
    })
}

fn shape_parameters() -> Vec<Binder> {
    [
        "input_rows",
        "input_columns",
        "content_min_row",
        "content_min_column",
        "content_rows",
        "content_columns",
    ]
    .into_iter()
    .map(|name| Binder {
        name: VarName(name.to_owned()),
        ty: coordinate_integer(),
    })
    .collect()
}

fn coordinate_parameters() -> Vec<Binder> {
    [
        "output_row",
        "output_column",
        "input_rows",
        "input_columns",
        "content_min_row",
        "content_min_column",
        "content_rows",
        "content_columns",
        "output_rows",
        "output_columns",
    ]
    .into_iter()
    .map(|name| Binder {
        name: VarName(name.to_owned()),
        ty: coordinate_integer(),
    })
    .collect()
}

fn shape_bindings(field: &FiniteField) -> RuntimeEnvironment {
    let mut environment = RuntimeEnvironment::new();
    let bounds = content_bounds(field);
    let extents = [
        field.shape[0],
        field.shape[1],
        bounds[0],
        bounds[1],
        bounds[2],
        bounds[3],
    ];
    for (parameter, extent) in shape_parameters().into_iter().zip(extents) {
        environment.insert(parameter.name, Value::Int(i64::from(extent)));
    }
    environment
}

fn coordinate_bindings(
    input: &FiniteField,
    output_row: usize,
    output_column: usize,
    output_rows: usize,
    output_columns: usize,
) -> Result<RuntimeEnvironment, FieldError> {
    let bounds = content_bounds(input);
    let values = [
        i64::try_from(output_row).map_err(|_| FieldError::Capacity)?,
        i64::try_from(output_column).map_err(|_| FieldError::Capacity)?,
        i64::from(input.shape[0]),
        i64::from(input.shape[1]),
        i64::from(bounds[0]),
        i64::from(bounds[1]),
        i64::from(bounds[2]),
        i64::from(bounds[3]),
        i64::try_from(output_rows).map_err(|_| FieldError::Capacity)?,
        i64::try_from(output_columns).map_err(|_| FieldError::Capacity)?,
    ];
    let mut environment = RuntimeEnvironment::new();
    for (parameter, value) in coordinate_parameters().into_iter().zip(values) {
        environment.insert(parameter.name, Value::Int(value));
    }
    Ok(environment)
}

fn modal_value(field: &FiniteField) -> i64 {
    let mut frequencies = BTreeMap::new();
    for value in &field.values {
        *frequencies.entry(*value).or_insert(0_usize) += 1;
    }
    frequencies
        .iter()
        .max_by_key(|(value, count)| (**count, Reverse(**value)))
        .map_or(0, |(value, _)| *value)
}

fn content_bounds(field: &FiniteField) -> [u32; 4] {
    let mut frequencies = BTreeMap::new();
    for value in &field.values {
        *frequencies.entry(*value).or_insert(0_usize) += 1;
    }
    let background = frequencies
        .iter()
        .max_by_key(|(value, count)| (**count, Reverse(**value)))
        .map_or(0, |(value, _)| *value);
    let columns = usize::try_from(field.shape[1]).unwrap_or(usize::MAX);
    let positions = field
        .values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (*value != background).then_some(index))
        .collect::<Vec<_>>();
    if positions.is_empty() || columns == usize::MAX {
        return [0, 0, field.shape[0], field.shape[1]];
    }
    let min_row = positions
        .iter()
        .map(|index| index / columns)
        .min()
        .unwrap_or(0);
    let max_row = positions
        .iter()
        .map(|index| index / columns)
        .max()
        .unwrap_or(0);
    let min_column = positions
        .iter()
        .map(|index| index % columns)
        .min()
        .unwrap_or(0);
    let max_column = positions
        .iter()
        .map(|index| index % columns)
        .max()
        .unwrap_or(0);
    [
        u32::try_from(min_row).unwrap_or(u32::MAX),
        u32::try_from(min_column).unwrap_or(u32::MAX),
        u32::try_from(max_row - min_row + 1).unwrap_or(u32::MAX),
        u32::try_from(max_column - min_column + 1).unwrap_or(u32::MAX),
    ]
}

fn predict_nonnegative(
    proposal: &DefinitionProposal,
    inputs: &RuntimeEnvironment,
    limits: cairn_rkl::ExecLimits,
) -> Result<usize, FieldError> {
    match DefinitionInventor
        .predict(proposal, inputs, limits)
        .map_err(FieldError::Invention)?
    {
        Value::Int(value) if value >= 0 => usize::try_from(value).map_err(|_| FieldError::Capacity),
        _ => Err(FieldError::Prediction),
    }
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
