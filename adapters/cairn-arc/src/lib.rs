//! ARC-specific parsing and encoding quarantined from CAIRN cognition crates.

mod legacy_schema;

pub use legacy_schema::{
    Direction, FieldSchemaError, FieldSchemaLearner, FieldTransformation, LearnedFieldTheory,
};

use cairn_core::{
    Demonstration, Hash32, Intervention, LearningProblem, PredictionRequest, SourceReference,
    TaskId,
};
use cairn_field::{
    FieldError, FieldEvidence, FiniteField, PointwiseFieldLearner, RemappedFieldLearner,
    ResidualConstructorInventor,
};
use cairn_invent::InventionLimits;
use cairn_reg::RegLimits;
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionRuntime, ExperienceError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const ARC_ADAPTER_SEMANTICS_VERSION: &str = "1.7.0";

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
    Schema(FieldSchemaError),
    Runtime(ExperienceError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArcAttempt {
    pub predictions: Vec<ArcGrid>,
    pub evaluated_tests: usize,
    pub exact_tests: usize,
    pub training_cells: usize,
    pub generic_observations: usize,
    pub relational_nodes: usize,
    pub relational_facts: usize,
    pub learned_expression: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcTaskStatus {
    Exact,
    Incorrect,
    UnsupportedShapeChange,
    NoTheoryWithinBounds,
    ResourceLimit,
    InvalidTask,
    ExecutionFailure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArcTaskReport {
    pub task: String,
    pub status: ArcTaskStatus,
    pub evaluated_tests: usize,
    pub exact_tests: usize,
    pub training_cells: usize,
    pub generic_observations: usize,
    pub relational_nodes: usize,
    pub relational_facts: usize,
    pub learned_expression: Option<String>,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ArcExperience {
    pub problem: LearningProblem,
    pub evidence: Vec<FieldEvidence>,
}

#[derive(Clone, Copy)]
struct AttemptEvidence {
    observations: usize,
    relational_nodes: usize,
    relational_facts: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArcBatchReport {
    pub adapter_semantics: String,
    pub rkl_semantics: String,
    pub corpus: String,
    pub maximum_tasks: usize,
    pub maximum_cost: u16,
    pub maximum_candidates: u64,
    pub selected_tasks: usize,
    pub parsed_tasks: usize,
    pub attempted_tasks: usize,
    pub exact_tasks: usize,
    pub incorrect_tasks: usize,
    pub unsupported_tasks: usize,
    pub no_theory_tasks: usize,
    pub resource_limited_tasks: usize,
    pub invalid_tasks: usize,
    pub execution_failures: usize,
    pub generic_observations: usize,
    pub relational_nodes: usize,
    pub relational_facts: usize,
    pub reports: Vec<ArcTaskReport>,
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

/// Lower a static task to the universal observation/demonstration protocol.
/// Test outputs are deliberately excluded from the returned envelope.
///
/// # Errors
///
/// Rejects an invalid finite field or observation identity.
pub fn lower_learning_problem(
    task: &ArcTask,
    source_locator: &str,
) -> Result<LearningProblem, ArcAdapterError> {
    let mut observations = Vec::new();
    let mut demonstrations = Vec::new();
    for (index, pair) in task.train.iter().enumerate() {
        let input = lower(&pair.input)?
            .observation(source(
                source_locator,
                &format!("train/{index}/input"),
                &pair.input,
            ))
            .map_err(ArcAdapterError::Field)?;
        let output = lower(&pair.output)?
            .observation(source(
                source_locator,
                &format!("train/{index}/output"),
                &pair.output,
            ))
            .map_err(ArcAdapterError::Field)?;
        demonstrations.push(Demonstration {
            context: vec![input.id],
            intervention: Some(implicit_transform()),
            outcome: vec![output.id],
        });
        observations.extend([input, output]);
    }
    let mut requested_predictions = Vec::new();
    for (index, case) in task.test.iter().enumerate() {
        let input = lower(&case.input)?
            .observation(source(
                source_locator,
                &format!("test/{index}/input"),
                &case.input,
            ))
            .map_err(ArcAdapterError::Field)?;
        requested_predictions.push(PredictionRequest {
            context: vec![input.id],
            intervention: Some(implicit_transform()),
        });
        observations.push(input);
    }
    let mut identity = Vec::new();
    for observation in &observations {
        identity.extend_from_slice(observation.id.0.as_bytes());
    }
    Ok(LearningProblem {
        id: TaskId(Hash32::digest(&identity)),
        observations,
        demonstrations,
        requested_predictions,
        attempt_limit: 3,
    })
}

/// Lower and independently decode every carried observation into relational
/// evidence. Adapter records remain outside the returned graph semantics.
///
/// # Errors
///
/// Rejects generic lowering, field validation, provenance mismatches, or REG
/// capacity failures.
pub fn lower_relational_experience(
    task: &ArcTask,
    source_locator: &str,
    limits: RegLimits,
) -> Result<ArcExperience, ArcAdapterError> {
    let problem = lower_learning_problem(task, source_locator)?;
    let fields = task
        .train
        .iter()
        .flat_map(|pair| [&pair.input, &pair.output])
        .chain(task.test.iter().map(|case| &case.input))
        .map(lower)
        .collect::<Result<Vec<_>, _>>()?;
    let evidence = fields
        .iter()
        .zip(&problem.observations)
        .map(|(field, observation)| {
            field
                .evidence_graph(observation, limits)
                .map_err(ArcAdapterError::Field)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ArcExperience { problem, evidence })
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
    let experience =
        lower_relational_experience(task, "urn:cairn:adapter-task:sealed", RegLimits::default())?;
    let admitted = CognitionRuntime::new()
        .admit_experience(&experience.problem)
        .map_err(ArcAdapterError::Runtime)?;
    let relational_nodes = experience
        .evidence
        .iter()
        .map(|item| item.graph.graph().node_count())
        .sum();
    let relational_facts = experience
        .evidence
        .iter()
        .map(|item| item.graph.graph().fact_count())
        .sum();
    let evidence = AttemptEvidence {
        observations: admitted.observations,
        relational_nodes,
        relational_facts,
    };
    let training = task
        .train
        .iter()
        .map(|pair| {
            let input = lower(&pair.input)?;
            let output = lower(&pair.output)?;
            Ok((input, output))
        })
        .collect::<Result<Vec<_>, ArcAdapterError>>()?;
    match FieldSchemaLearner.learn(&training) {
        Ok(learned) => {
            let predictions = task
                .test
                .iter()
                .map(|case| {
                    let input = lower(&case.input)?;
                    let prediction = FieldSchemaLearner
                        .predict(&learned, &input)
                        .map_err(ArcAdapterError::Schema)?;
                    raise(&prediction)
                })
                .collect::<Result<Vec<_>, ArcAdapterError>>()?;
            let evaluated_tests = task
                .test
                .iter()
                .filter(|case| case.output.is_some())
                .count();
            let exact_tests = predictions
                .iter()
                .zip(&task.test)
                .filter(|(prediction, case)| case.output.as_ref() == Some(*prediction))
                .count();
            return Ok(ArcAttempt {
                predictions,
                evaluated_tests,
                exact_tests,
                training_cells: training.iter().map(|(input, _)| input.values.len()).sum(),
                generic_observations: admitted.observations,
                relational_nodes,
                relational_facts,
                learned_expression: format!("{:?}", learned.transformation),
            });
        }
        Err(FieldSchemaError::NoTheory) => {}
        Err(error) => return Err(ArcAdapterError::Schema(error)),
    }
    if training
        .iter()
        .any(|(input, output)| input.shape != output.shape)
    {
        return attempt_remapped(task, &training, invention, execution, evidence);
    }
    if let Some(attempt) = attempt_residual(task, &training, invention, execution, evidence)? {
        return Ok(attempt);
    }
    attempt_pointwise(task, &training, invention, execution, evidence)
}

fn attempt_residual(
    task: &ArcTask,
    training: &[(FiniteField, FiniteField)],
    invention: InventionLimits,
    execution: ExecLimits,
    evidence: AttemptEvidence,
) -> Result<Option<ArcAttempt>, ArcAdapterError> {
    let backgrounds = training
        .iter()
        .flat_map(|(input, _)| input.values.iter().copied())
        .collect::<std::collections::BTreeSet<_>>();
    for background in backgrounds {
        let Ok(learned) = ResidualConstructorInventor.invent(training, background, invention)
        else {
            continue;
        };
        let predictions = task
            .test
            .iter()
            .map(|case| {
                let input = lower(&case.input)?;
                ResidualConstructorInventor
                    .predict(&learned, &input, execution)
                    .map_err(ArcAdapterError::Field)
                    .and_then(|prediction| raise(&prediction))
            })
            .collect::<Result<Vec<_>, ArcAdapterError>>()?;
        let evaluated_tests = task
            .test
            .iter()
            .filter(|case| case.output.is_some())
            .count();
        let exact_tests = predictions
            .iter()
            .zip(&task.test)
            .filter(|(prediction, case)| case.output.as_ref() == Some(*prediction))
            .count();
        return Ok(Some(ArcAttempt {
            predictions,
            evaluated_tests,
            exact_tests,
            training_cells: learned.training_cells,
            generic_observations: evidence.observations,
            relational_nodes: evidence.relational_nodes,
            relational_facts: evidence.relational_facts,
            learned_expression: format!(
                "residual background={} paint={} predicate={:?}",
                learned.background, learned.paint, learned.proposal.definition.body
            ),
        }));
    }
    Ok(None)
}

fn attempt_pointwise(
    task: &ArcTask,
    training: &[(FiniteField, FiniteField)],
    invention: InventionLimits,
    execution: ExecLimits,
    evidence: AttemptEvidence,
) -> Result<ArcAttempt, ArcAdapterError> {
    let learned = PointwiseFieldLearner
        .learn(training, invention)
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
        generic_observations: evidence.observations,
        relational_nodes: evidence.relational_nodes,
        relational_facts: evidence.relational_facts,
        learned_expression: format!("{:?}", learned.proposal.definition.body),
    })
}

fn attempt_remapped(
    task: &ArcTask,
    training: &[(FiniteField, FiniteField)],
    invention: InventionLimits,
    execution: ExecLimits,
    evidence: AttemptEvidence,
) -> Result<ArcAttempt, ArcAdapterError> {
    let learned = match RemappedFieldLearner.learn(training, invention) {
        Ok(learned) => learned,
        Err(FieldError::Invention(error)) => {
            return Err(ArcAdapterError::Field(FieldError::Invention(error)));
        }
        Err(_) => return Err(ArcAdapterError::ShapeChangeUnsupported),
    };
    let predictions = task
        .test
        .iter()
        .map(|case| {
            let input = lower(&case.input)?;
            RemappedFieldLearner
                .predict(&learned, &input, execution)
                .map_err(ArcAdapterError::Field)
                .and_then(|prediction| raise(&prediction))
        })
        .collect::<Result<Vec<_>, ArcAdapterError>>()?;
    let evaluated_tests = task
        .test
        .iter()
        .filter(|case| case.output.is_some())
        .count();
    let exact_tests = predictions
        .iter()
        .zip(&task.test)
        .filter(|(prediction, case)| case.output.as_ref() == Some(*prediction))
        .count();
    Ok(ArcAttempt {
        predictions,
        evaluated_tests,
        exact_tests,
        training_cells: learned.training_correspondences,
        generic_observations: evidence.observations,
        relational_nodes: evidence.relational_nodes,
        relational_facts: evidence.relational_facts,
        learned_expression: format!(
            "shape=({:?},{:?}); source=({:?},{:?})",
            learned.output_rows.definition.body,
            learned.output_columns.definition.body,
            learned.source_row.definition.body,
            learned.source_column.definition.body,
        ),
    })
}

/// Run a deterministic filename-sorted development batch. Expected test
/// outputs are removed from the task passed to learning and used only after
/// predictions return.
///
/// # Errors
///
/// Returns an I/O diagnostic if the corpus directory cannot be enumerated.
pub fn run_training_batch(
    directory: &Path,
    maximum_tasks: usize,
    invention: InventionLimits,
    execution: ExecLimits,
) -> Result<ArcBatchReport, String> {
    let mut paths = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.truncate(maximum_tasks);
    let mut batch = ArcBatchReport {
        adapter_semantics: ARC_ADAPTER_SEMANTICS_VERSION.to_owned(),
        rkl_semantics: cairn_rkl::RKL_SEMANTICS_VERSION.to_owned(),
        corpus: directory.display().to_string(),
        maximum_tasks,
        maximum_cost: invention.maximum_cost,
        maximum_candidates: invention.maximum_candidates,
        selected_tasks: paths.len(),
        parsed_tasks: 0,
        attempted_tasks: 0,
        exact_tasks: 0,
        incorrect_tasks: 0,
        unsupported_tasks: 0,
        no_theory_tasks: 0,
        resource_limited_tasks: 0,
        invalid_tasks: 0,
        execution_failures: 0,
        generic_observations: 0,
        relational_nodes: 0,
        relational_facts: 0,
        reports: Vec::with_capacity(paths.len()),
    };
    for path in paths {
        let task_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("invalid-filename")
            .to_owned();
        let task = fs::read(&path)
            .map_err(|_| ArcAdapterError::Json)
            .and_then(|bytes| parse_task(&bytes));
        let Ok(task) = task else {
            record_report(
                &mut batch,
                ArcTaskReport {
                    task: task_name,
                    status: ArcTaskStatus::InvalidTask,
                    evaluated_tests: 0,
                    exact_tests: 0,
                    training_cells: 0,
                    generic_observations: 0,
                    relational_nodes: 0,
                    relational_facts: 0,
                    learned_expression: None,
                    diagnostic: Some("parse or input validation failed".to_owned()),
                },
            );
            continue;
        };
        batch.parsed_tasks += 1;
        let Ok((generic_observations, relational_nodes, relational_facts)) =
            admit_batch_boundary(&task, &task_name)
        else {
            record_report(
                &mut batch,
                ArcTaskReport {
                    task: task_name,
                    status: ArcTaskStatus::ExecutionFailure,
                    evaluated_tests: 0,
                    exact_tests: 0,
                    training_cells: 0,
                    generic_observations: 0,
                    relational_nodes: 0,
                    relational_facts: 0,
                    learned_expression: None,
                    diagnostic: Some("generic experience admission failed".to_owned()),
                },
            );
            continue;
        };
        batch.generic_observations = batch
            .generic_observations
            .saturating_add(generic_observations);
        batch.relational_nodes = batch.relational_nodes.saturating_add(relational_nodes);
        batch.relational_facts = batch.relational_facts.saturating_add(relational_facts);
        let report = evaluate_task(task_name, task, invention, execution);
        record_report(&mut batch, report);
    }
    Ok(batch)
}

fn admit_batch_boundary(
    task: &ArcTask,
    task_name: &str,
) -> Result<(usize, usize, usize), ArcAdapterError> {
    let experience = lower_relational_experience(
        task,
        &format!("urn:cairn:corpus-task:{task_name}"),
        RegLimits::default(),
    )?;
    CognitionRuntime::new()
        .admit_experience(&experience.problem)
        .map_err(ArcAdapterError::Runtime)?;
    let nodes = experience
        .evidence
        .iter()
        .map(|item| item.graph.graph().node_count())
        .sum();
    let facts = experience
        .evidence
        .iter()
        .map(|item| item.graph.graph().fact_count())
        .sum();
    Ok((experience.problem.observations.len(), nodes, facts))
}

fn evaluate_task(
    task_name: String,
    task: ArcTask,
    invention: InventionLimits,
    execution: ExecLimits,
) -> ArcTaskReport {
    let expected = task
        .test
        .iter()
        .map(|case| case.output.clone())
        .collect::<Vec<_>>();
    let sealed = ArcTask {
        train: task.train,
        test: task
            .test
            .into_iter()
            .map(|case| ArcTestCase {
                input: case.input,
                output: None,
            })
            .collect(),
    };
    match attempt_task(&sealed, invention, execution) {
        Ok(attempt) => {
            let evaluated = expected.iter().filter(|item| item.is_some()).count();
            let exact = attempt
                .predictions
                .iter()
                .zip(&expected)
                .filter(|(prediction, expected)| expected.as_ref() == Some(*prediction))
                .count();
            ArcTaskReport {
                task: task_name,
                status: if evaluated > 0 && exact == evaluated {
                    ArcTaskStatus::Exact
                } else {
                    ArcTaskStatus::Incorrect
                },
                evaluated_tests: evaluated,
                exact_tests: exact,
                training_cells: attempt.training_cells,
                generic_observations: attempt.generic_observations,
                relational_nodes: attempt.relational_nodes,
                relational_facts: attempt.relational_facts,
                learned_expression: Some(attempt.learned_expression),
                diagnostic: None,
            }
        }
        Err(error) => {
            let (status, diagnostic) = classify_error(&error);
            ArcTaskReport {
                task: task_name,
                status,
                evaluated_tests: 0,
                exact_tests: 0,
                training_cells: 0,
                generic_observations: 0,
                relational_nodes: 0,
                relational_facts: 0,
                learned_expression: None,
                diagnostic: Some(diagnostic),
            }
        }
    }
}

fn record_report(batch: &mut ArcBatchReport, report: ArcTaskReport) {
    match report.status {
        ArcTaskStatus::Exact => {
            batch.attempted_tasks += 1;
            batch.exact_tasks += 1;
        }
        ArcTaskStatus::Incorrect => {
            batch.attempted_tasks += 1;
            batch.incorrect_tasks += 1;
        }
        ArcTaskStatus::UnsupportedShapeChange => batch.unsupported_tasks += 1,
        ArcTaskStatus::NoTheoryWithinBounds => batch.no_theory_tasks += 1,
        ArcTaskStatus::ResourceLimit => batch.resource_limited_tasks += 1,
        ArcTaskStatus::InvalidTask => batch.invalid_tasks += 1,
        ArcTaskStatus::ExecutionFailure => batch.execution_failures += 1,
    }
    batch.reports.push(report);
}

fn classify_error(error: &ArcAdapterError) -> (ArcTaskStatus, String) {
    let status = match error {
        ArcAdapterError::ShapeChangeUnsupported => ArcTaskStatus::UnsupportedShapeChange,
        ArcAdapterError::Field(FieldError::Invention(
            cairn_invent::InventionError::NoDefinition { .. },
        ))
        | ArcAdapterError::Schema(FieldSchemaError::NoTheory) => {
            ArcTaskStatus::NoTheoryWithinBounds
        }
        ArcAdapterError::Field(FieldError::Invention(
            cairn_invent::InventionError::CandidateCapacity { .. },
        ))
        | ArcAdapterError::Schema(FieldSchemaError::Capacity) => ArcTaskStatus::ResourceLimit,
        ArcAdapterError::Json
        | ArcAdapterError::EmptyTraining
        | ArcAdapterError::EmptyTest
        | ArcAdapterError::EmptyGrid
        | ArcAdapterError::RaggedGrid
        | ArcAdapterError::InvalidCategoricalValue(_) => ArcTaskStatus::InvalidTask,
        ArcAdapterError::Field(_) | ArcAdapterError::Schema(_) | ArcAdapterError::Runtime(_) => {
            ArcTaskStatus::ExecutionFailure
        }
    };
    (status, format!("{error:?}"))
}

fn implicit_transform() -> Intervention {
    Intervention {
        action: "urn:cairn:implicit-transform:v1".to_owned(),
        arguments: Vec::new(),
    }
}

fn source(locator: &str, role: &str, grid: &ArcGrid) -> SourceReference {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(role.as_bytes());
    for row in &grid.rows {
        bytes.extend_from_slice(&u64::try_from(row.len()).unwrap_or(u64::MAX).to_be_bytes());
        bytes.extend_from_slice(row);
    }
    SourceReference {
        locator: format!("{locator}#{role}"),
        content_hash: Hash32::digest(&bytes),
    }
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
