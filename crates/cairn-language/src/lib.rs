//! Controlled-language compilation and verified technical response planning.
//!
//! This initial frontend deliberately covers a narrow technical-assistant
//! slice. It compiles text to explicit intent, constructs a typed Python AST,
//! verifies executable examples, and realizes a response. It is not a free-form
//! text generator and does not call an external model.

use cairn_core::Hash32;
use std::collections::{BTreeMap, BTreeSet};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const LANGUAGE_SEMANTICS_VERSION: &str = "1.1.0";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SpeechAct {
    Request,
    Correction,
    Answer,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GoalExpression {
    GenerateProgram {
        language: ProgrammingLanguage,
        operation: ProgramOperation,
    },
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProgrammingLanguage {
    Python,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProgramOperation {
    IllustrativeFunction,
    AddNumbers,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OutputSpecification {
    SourceCode,
    Clarification,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Ambiguity {
    pub field: String,
    pub alternatives: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserIntent {
    pub speech_act: SpeechAct,
    pub objective: GoalExpression,
    pub constraints: Vec<String>,
    pub referenced_entities: Vec<String>,
    pub requested_output: OutputSpecification,
    pub uncertainty: Vec<Ambiguity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageLesson {
    pub utterance: String,
    pub objective: GoalExpression,
    pub requested_output: OutputSpecification,
}

#[derive(Clone, Debug, Default)]
pub struct LanguageCompiler {
    lessons: BTreeMap<String, LanguageLesson>,
    structural_lessons: Vec<(BTreeSet<String>, LanguageLesson)>,
}

impl LanguageCompiler {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            lessons: BTreeMap::new(),
            structural_lessons: Vec::new(),
        }
    }

    pub fn install_lesson(&mut self, lesson: LanguageLesson) -> bool {
        let normalized = normalize(&lesson.utterance);
        let signature = semantic_signature(&normalized);
        if !signature.is_empty()
            && !self.structural_lessons.iter().any(|(existing, item)| {
                existing == &signature && item.objective == lesson.objective
            })
        {
            self.structural_lessons.push((signature, lesson.clone()));
        }
        self.lessons.insert(normalized, lesson).is_none()
    }

    #[must_use]
    pub fn compile(&self, prompt: &str) -> UserIntent {
        let normalized = normalize(prompt);
        if let Some(lesson) = self.lessons.get(&normalized) {
            return UserIntent {
                speech_act: SpeechAct::Request,
                objective: lesson.objective.clone(),
                constraints: Vec::new(),
                referenced_entities: Vec::new(),
                requested_output: lesson.requested_output.clone(),
                uncertainty: Vec::new(),
            };
        }
        let signature = semantic_signature(&normalized);
        if let Some((_, lesson)) = self
            .structural_lessons
            .iter()
            .filter(|(required, _)| required.is_subset(&signature))
            .max_by_key(|(required, _)| required.len())
        {
            return UserIntent {
                speech_act: SpeechAct::Request,
                objective: lesson.objective.clone(),
                constraints: Vec::new(),
                referenced_entities: Vec::new(),
                requested_output: lesson.requested_output.clone(),
                uncertainty: Vec::new(),
            };
        }
        let words = normalized.split_whitespace().collect::<BTreeSet<_>>();
        if words.contains("python") && words.contains("function") {
            return UserIntent {
                speech_act: SpeechAct::Request,
                objective: GoalExpression::GenerateProgram {
                    language: ProgrammingLanguage::Python,
                    operation: ProgramOperation::IllustrativeFunction,
                },
                constraints: vec!["basic".to_owned()],
                referenced_entities: Vec::new(),
                requested_output: OutputSpecification::SourceCode,
                uncertainty: vec![Ambiguity {
                    field: "function_purpose".to_owned(),
                    alternatives: vec!["ask".to_owned(), "illustrate".to_owned()],
                }],
            };
        }
        UserIntent {
            speech_act: SpeechAct::Request,
            objective: GoalExpression::Unknown,
            constraints: Vec::new(),
            referenced_entities: Vec::new(),
            requested_output: OutputSpecification::Clarification,
            uncertainty: vec![Ambiguity {
                field: "objective".to_owned(),
                alternatives: Vec::new(),
            }],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DialogueState {
    pub current_goal: Option<GoalExpression>,
    pub established_facts: Vec<String>,
    pub unresolved_questions: Vec<Ambiguity>,
    pub user_preferences: BTreeSet<String>,
    pub prior_artifacts: Vec<Hash32>,
}

impl DialogueState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current_goal: None,
            established_facts: Vec::new(),
            unresolved_questions: Vec::new(),
            user_preferences: BTreeSet::new(),
            prior_artifacts: Vec::new(),
        }
    }

    pub fn observe(&mut self, intent: &UserIntent) {
        if intent.objective != GoalExpression::Unknown {
            self.current_goal = Some(intent.objective.clone());
        }
        self.unresolved_questions.clone_from(&intent.uncertainty);
    }
}

impl Default for DialogueState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PythonType {
    Float,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PythonParameter {
    pub name: String,
    pub ty: PythonType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PythonExpression {
    Parameter(String),
    Add(Box<Self>, Box<Self>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PythonFunction {
    pub name: String,
    pub parameters: Vec<PythonParameter>,
    pub return_type: PythonType,
    pub docstring: String,
    pub body: PythonExpression,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionExample {
    pub inputs: Vec<f64>,
    pub output: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeError {
    InvalidIdentifier,
    DuplicateParameter,
    UnknownParameter,
    Arity,
    NonFinite,
    ExampleMismatch,
    SandboxUnavailable,
    SandboxTimeout,
    SandboxRejected,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PythonVerifier;

impl PythonVerifier {
    /// Check identifiers, bindings, arity, total numeric inputs, and every
    /// supplied executable example against the typed AST.
    ///
    /// # Errors
    ///
    /// Returns the first structural or behavioral mismatch.
    pub fn verify(
        &self,
        function: &PythonFunction,
        examples: &[FunctionExample],
    ) -> Result<(), CodeError> {
        if !identifier(&function.name) || function.parameters.iter().any(|p| !identifier(&p.name)) {
            return Err(CodeError::InvalidIdentifier);
        }
        let names = function
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<BTreeSet<_>>();
        if names.len() != function.parameters.len() {
            return Err(CodeError::DuplicateParameter);
        }
        check_expression(&function.body, &names)?;
        for example in examples {
            if example.inputs.len() != function.parameters.len() {
                return Err(CodeError::Arity);
            }
            if example.inputs.iter().any(|value| !value.is_finite()) || !example.output.is_finite()
            {
                return Err(CodeError::NonFinite);
            }
            let environment = function
                .parameters
                .iter()
                .zip(&example.inputs)
                .map(|(parameter, value)| (parameter.name.as_str(), *value))
                .collect::<BTreeMap<_, _>>();
            let actual = evaluate(&function.body, &environment)?;
            if actual.to_bits() != example.output.to_bits() {
                return Err(CodeError::ExampleMismatch);
            }
        }
        Ok(())
    }

    /// Execute the rendered typed AST in isolated Python mode with a cleared
    /// environment, closed stdin, CPU/address-space limits, and a wall timeout.
    /// Only ASTs that already pass `verify` reach this boundary.
    ///
    /// # Errors
    ///
    /// Returns structural verification, unavailable runtime, timeout, or
    /// nonzero sandbox result errors.
    pub fn verify_in_sandbox(
        &self,
        function: &PythonFunction,
        examples: &[FunctionExample],
    ) -> Result<(), CodeError> {
        self.verify(function, examples)?;
        let assertions = examples
            .iter()
            .map(|example| {
                let inputs = example
                    .inputs
                    .iter()
                    .map(|value| format!("{value:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("assert {}({inputs}) == {:?}", function.name, example.output)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let script = format!(
            "import resource\nresource.setrlimit(resource.RLIMIT_CPU, (1, 1))\nresource.setrlimit(resource.RLIMIT_AS, (134217728, 134217728))\n{}\n{}\n",
            render_function(function),
            assertions
        );
        let mut child = Command::new("python3")
            .args(["-I", "-S", "-c", &script])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| CodeError::SandboxUnavailable)?;
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(|_| CodeError::SandboxRejected)? {
                return status
                    .success()
                    .then_some(())
                    .ok_or(CodeError::SandboxRejected);
            }
            if started.elapsed() >= Duration::from_secs(2) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CodeError::SandboxTimeout);
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnswerPlan {
    CodeExample {
        introduction: String,
        function: PythonFunction,
        explanation: String,
    },
    Clarify(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatResponse {
    pub intent: UserIntent,
    pub plan: AnswerPlan,
    pub rendered: String,
}

#[derive(Clone, Debug, Default)]
pub struct TechnicalChatbot {
    compiler: LanguageCompiler,
    dialogue: DialogueState,
}

impl TechnicalChatbot {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            compiler: LanguageCompiler::new(),
            dialogue: DialogueState::new(),
        }
    }

    pub fn install_lesson(&mut self, lesson: LanguageLesson) -> bool {
        self.compiler.install_lesson(lesson)
    }

    #[must_use]
    pub const fn dialogue(&self) -> &DialogueState {
        &self.dialogue
    }

    /// Compile a prompt, construct and verify a typed answer artifact, then
    /// realize the controlled response plan.
    ///
    /// # Errors
    ///
    /// Returns a typed-code verification error; unverifiable code is never
    /// rendered as an accepted response.
    pub fn respond(&mut self, prompt: &str) -> Result<ChatResponse, CodeError> {
        let intent = self.compiler.compile(prompt);
        self.dialogue.observe(&intent);
        let plan = match intent.objective {
            GoalExpression::GenerateProgram {
                language: ProgrammingLanguage::Python,
                operation: ProgramOperation::IllustrativeFunction | ProgramOperation::AddNumbers,
            } => {
                let function = addition_function();
                let examples = [
                    FunctionExample {
                        inputs: vec![2.0, 3.0],
                        output: 5.0,
                    },
                    FunctionExample {
                        inputs: vec![-4.5, 1.5],
                        output: -3.0,
                    },
                ];
                PythonVerifier.verify_in_sandbox(&function, &examples)?;
                AnswerPlan::CodeExample {
                    introduction: "Here is a basic Python function that adds two numbers."
                        .to_owned(),
                    function,
                    explanation: "It accepts two values and returns their sum.".to_owned(),
                }
            }
            GoalExpression::Unknown => AnswerPlan::Clarify(
                "What would you like the program or explanation to do?".to_owned(),
            ),
        };
        let rendered = realize(&plan);
        Ok(ChatResponse {
            intent,
            plan,
            rendered,
        })
    }
}

fn addition_function() -> PythonFunction {
    PythonFunction {
        name: "add_numbers".to_owned(),
        parameters: vec![
            PythonParameter {
                name: "a".to_owned(),
                ty: PythonType::Float,
            },
            PythonParameter {
                name: "b".to_owned(),
                ty: PythonType::Float,
            },
        ],
        return_type: PythonType::Float,
        docstring: "Return the sum of two numbers.".to_owned(),
        body: PythonExpression::Add(
            Box::new(PythonExpression::Parameter("a".to_owned())),
            Box::new(PythonExpression::Parameter("b".to_owned())),
        ),
    }
}

fn realize(plan: &AnswerPlan) -> String {
    match plan {
        AnswerPlan::Clarify(question) => question.clone(),
        AnswerPlan::CodeExample {
            introduction,
            function,
            explanation,
        } => format!(
            "{introduction}\n\n```python\n{}\n```\n\n{explanation}",
            render_function(function)
        ),
    }
}

fn render_function(function: &PythonFunction) -> String {
    let parameters = function
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, render_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "def {}({parameters}) -> {}:\n    \"\"\"{}\"\"\"\n    return {}",
        function.name,
        render_type(&function.return_type),
        function.docstring,
        render_expression(&function.body)
    )
}

fn render_type(ty: &PythonType) -> &'static str {
    match ty {
        PythonType::Float => "float",
    }
}

fn render_expression(expression: &PythonExpression) -> String {
    match expression {
        PythonExpression::Parameter(name) => name.clone(),
        PythonExpression::Add(left, right) => {
            format!("{} + {}", render_expression(left), render_expression(right))
        }
    }
}

fn check_expression(
    expression: &PythonExpression,
    parameters: &BTreeSet<&str>,
) -> Result<(), CodeError> {
    match expression {
        PythonExpression::Parameter(name) if parameters.contains(name.as_str()) => Ok(()),
        PythonExpression::Parameter(_) => Err(CodeError::UnknownParameter),
        PythonExpression::Add(left, right) => {
            check_expression(left, parameters)?;
            check_expression(right, parameters)
        }
    }
}

fn evaluate(
    expression: &PythonExpression,
    environment: &BTreeMap<&str, f64>,
) -> Result<f64, CodeError> {
    match expression {
        PythonExpression::Parameter(name) => environment
            .get(name.as_str())
            .copied()
            .ok_or(CodeError::UnknownParameter),
        PythonExpression::Add(left, right) => {
            Ok(evaluate(left, environment)? + evaluate(right, environment)?)
        }
    }
}

fn identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn semantic_signature(normalized: &str) -> BTreeSet<String> {
    normalized
        .split_whitespace()
        .filter_map(|word| match word {
            "python" | "py" => Some("python"),
            "function" | "callable" | "method" => Some("function"),
            "add" | "sum" | "total" | "totals" | "adding" => Some("add"),
            "number" | "numbers" | "value" | "values" => Some("number"),
            _ => None,
        })
        .map(str::to_owned)
        .collect()
}
