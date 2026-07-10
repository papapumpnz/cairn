//! Adapter-neutral finite categorical fields and a first learned field model.

use cairn_core::{
    CanonicalValue, Hash32, ModalityId, Observation, ObservationCertainty, ObservationError,
    SourceReference,
};
use cairn_invent::{
    BindingExample, DefinitionInventor, DefinitionProposal, InventionError, InventionLimits,
    InventionProblem,
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
