//! ARC-specific parsing and encoding quarantined from CAIRN cognition crates.

use cairn_field::{FieldError, FiniteField, PointwiseFieldLearner};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use serde::Deserialize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArcGrid {
    pub rows: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArcPair {
    pub input: ArcGrid,
    pub output: ArcGrid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArcTestCase {
    pub input: ArcGrid,
    pub output: Option<ArcGrid>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArcTask {
    pub train: Vec<ArcPair>,
    pub test: Vec<ArcTestCase>,
}

#[derive(Deserialize)]
struct RawTask {
    train: Vec<RawPair>,
    test: Vec<RawTest>,
}

#[derive(Deserialize)]
struct RawPair {
    input: Vec<Vec<u8>>,
    output: Vec<Vec<u8>>,
}

#[derive(Deserialize)]
struct RawTest {
    input: Vec<Vec<u8>>,
    #[serde(default)]
    output: Option<Vec<Vec<u8>>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArcAdapterError {
    Json,
    EmptyTraining,
    EmptyTest,
    EmptyGrid,
    RaggedGrid,
    InvalidCategoricalValue(u8),
    ShapeChangeUnsupported,
    Field(FieldError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArcAttempt {
    pub predictions: Vec<ArcGrid>,
    pub evaluated_tests: usize,
    pub exact_tests: usize,
    pub training_cells: usize,
    pub learned_expression: String,
}

/// Parse canonical ARC-style JSON while rejecting malformed categorical grids.
///
/// # Errors
///
/// Rejects JSON, empty splits, ragged/empty grids, and values outside 0..=9.
pub fn parse_task(bytes: &[u8]) -> Result<ArcTask, ArcAdapterError> {
    let raw: RawTask = serde_json::from_slice(bytes).map_err(|_| ArcAdapterError::Json)?;
    if raw.train.is_empty() {
        return Err(ArcAdapterError::EmptyTraining);
    }
    if raw.test.is_empty() {
        return Err(ArcAdapterError::EmptyTest);
    }
    let train = raw
        .train
        .into_iter()
        .map(|pair| {
            Ok(ArcPair {
                input: grid(pair.input)?,
                output: grid(pair.output)?,
            })
        })
        .collect::<Result<Vec<_>, ArcAdapterError>>()?;
    let test = raw
        .test
        .into_iter()
        .map(|case| {
            Ok(ArcTestCase {
                input: grid(case.input)?,
                output: case.output.map(grid).transpose()?,
            })
        })
        .collect::<Result<Vec<_>, ArcAdapterError>>()?;
    Ok(ArcTask { train, test })
}

/// Attempt an ARC task solely through the generic pointwise field learner.
/// No task identifier, operation label, or test output enters learning.
///
/// # Errors
///
/// Returns unsupported shape changes or bounded generic learning failures.
pub fn attempt_task(
    task: &ArcTask,
    invention: InventionLimits,
    execution: ExecLimits,
) -> Result<ArcAttempt, ArcAdapterError> {
    let training = task
        .train
        .iter()
        .map(|pair| {
            let input = lower(&pair.input)?;
            let output = lower(&pair.output)?;
            if input.shape != output.shape {
                return Err(ArcAdapterError::ShapeChangeUnsupported);
            }
            Ok((input, output))
        })
        .collect::<Result<Vec<_>, ArcAdapterError>>()?;
    let learned = PointwiseFieldLearner
        .learn(&training, invention)
        .map_err(ArcAdapterError::Field)?;
    let mut predictions = Vec::with_capacity(task.test.len());
    let mut evaluated = 0_usize;
    let mut exact = 0_usize;
    for case in &task.test {
        let input = lower(&case.input)?;
        let prediction = PointwiseFieldLearner
            .predict(&learned, &input, execution)
            .map_err(ArcAdapterError::Field)?;
        let prediction = raise(&prediction)?;
        if let Some(expected) = &case.output {
            evaluated += 1;
            exact += usize::from(&prediction == expected);
        }
        predictions.push(prediction);
    }
    Ok(ArcAttempt {
        predictions,
        evaluated_tests: evaluated,
        exact_tests: exact,
        training_cells: learned.training_cells,
        learned_expression: format!("{:?}", learned.proposal.definition.body),
    })
}

fn grid(rows: Vec<Vec<u8>>) -> Result<ArcGrid, ArcAdapterError> {
    let Some(columns) = rows.first().map(Vec::len) else {
        return Err(ArcAdapterError::EmptyGrid);
    };
    if columns == 0 {
        return Err(ArcAdapterError::EmptyGrid);
    }
    if rows.iter().any(|row| row.len() != columns) {
        return Err(ArcAdapterError::RaggedGrid);
    }
    if let Some(value) = rows.iter().flatten().find(|value| **value > 9) {
        return Err(ArcAdapterError::InvalidCategoricalValue(*value));
    }
    Ok(ArcGrid { rows })
}

fn lower(grid: &ArcGrid) -> Result<FiniteField, ArcAdapterError> {
    let rows = u32::try_from(grid.rows.len()).map_err(|_| ArcAdapterError::EmptyGrid)?;
    let columns = u32::try_from(grid.rows[0].len()).map_err(|_| ArcAdapterError::EmptyGrid)?;
    let field = FiniteField {
        shape: vec![rows, columns],
        values: grid
            .rows
            .iter()
            .flatten()
            .map(|value| i64::from(*value))
            .collect(),
    };
    field.validate().map_err(ArcAdapterError::Field)?;
    Ok(field)
}

fn raise(field: &FiniteField) -> Result<ArcGrid, ArcAdapterError> {
    field.validate().map_err(ArcAdapterError::Field)?;
    let columns = usize::try_from(field.shape[1]).map_err(|_| ArcAdapterError::EmptyGrid)?;
    let rows = field
        .values
        .chunks(columns)
        .map(|row| {
            row.iter()
                .map(|value| {
                    u8::try_from(*value)
                        .ok()
                        .filter(|value| *value <= 9)
                        .ok_or(ArcAdapterError::InvalidCategoricalValue(u8::MAX))
                })
                .collect()
        })
        .collect::<Result<Vec<Vec<u8>>, ArcAdapterError>>()?;
    Ok(ArcGrid { rows })
}
