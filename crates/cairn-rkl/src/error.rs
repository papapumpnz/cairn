use crate::Type;
use std::fmt;

/// Deterministic path into a nested expression or theory.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExprPath(pub Vec<PathSegment>);

impl ExprPath {
    #[must_use]
    pub fn child(&self, segment: PathSegment) -> Self {
        let mut path = self.0.clone();
        path.push(segment);
        Self(path)
    }
}

impl fmt::Display for ExprPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("$")?;
        for segment in &self.0 {
            write!(formatter, ".{segment}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathSegment {
    Condition,
    Then,
    Else,
    Value,
    Body,
    Argument,
    Left,
    Right,
    Element(usize),
    CallArgument(usize),
    Initial,
    Step,
    Progress,
    Definition(usize),
    Rule(usize),
    Binder(usize),
    Domain,
    Guard,
    Effect(usize),
    Canvas,
    Layer(usize),
    Background,
    Assumption(usize),
    Schedule,
}

impl fmt::Display for PathSegment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Condition => formatter.write_str("condition"),
            Self::Then => formatter.write_str("then"),
            Self::Else => formatter.write_str("else"),
            Self::Value => formatter.write_str("value"),
            Self::Body => formatter.write_str("body"),
            Self::Argument => formatter.write_str("argument"),
            Self::Left => formatter.write_str("left"),
            Self::Right => formatter.write_str("right"),
            Self::Element(index) => write!(formatter, "element[{index}]"),
            Self::CallArgument(index) => write!(formatter, "argument[{index}]"),
            Self::Initial => formatter.write_str("initial"),
            Self::Step => formatter.write_str("step"),
            Self::Progress => formatter.write_str("progress"),
            Self::Definition(index) => write!(formatter, "definition[{index}]"),
            Self::Rule(index) => write!(formatter, "rule[{index}]"),
            Self::Binder(index) => write!(formatter, "binder[{index}]"),
            Self::Domain => formatter.write_str("domain"),
            Self::Guard => formatter.write_str("guard"),
            Self::Effect(index) => write!(formatter, "effect[{index}]"),
            Self::Canvas => formatter.write_str("canvas"),
            Self::Layer(index) => write!(formatter, "layer[{index}]"),
            Self::Background => formatter.write_str("background"),
            Self::Assumption(index) => write!(formatter, "assumption[{index}]"),
            Self::Schedule => formatter.write_str("schedule"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckErrorKind {
    UnknownVariable(String),
    UnknownFunction(String),
    TypeMismatch {
        expected: Type,
        actual: Type,
    },
    InvalidOperand {
        operation: &'static str,
        actual: Type,
    },
    BranchTypeMismatch {
        then_type: Type,
        else_type: Type,
    },
    ArityMismatch {
        expected: usize,
        actual: usize,
    },
    BoundOverflow,
    InvalidRange,
    EmptyBound,
    RecursiveCallWithoutBound(String),
    RecursiveBoundMissingProgress(String),
    DuplicateDefinition(String),
    DuplicateRule(String),
    DuplicateBinder(String),
    InvalidDomain(Type),
    UnknownRule(String),
    DuplicateScheduledRule(String),
    ScheduleCycle,
    EmptyTheory,
    MutualRecursionUnsupported(Vec<String>),
    CapacityCap {
        requested: usize,
        maximum: usize,
    },
    InternalInvariant(String),
}

/// Typed checker failure with a stable exact source path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckError {
    pub path: ExprPath,
    pub kind: CheckErrorKind,
}

impl CheckError {
    #[must_use]
    pub const fn new(path: ExprPath, kind: CheckErrorKind) -> Self {
        Self { path, kind }
    }
}

impl fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: ", self.path)?;
        match &self.kind {
            CheckErrorKind::UnknownVariable(name) => write!(formatter, "unknown variable `{name}`"),
            CheckErrorKind::UnknownFunction(name) => write!(formatter, "unknown function `{name}`"),
            CheckErrorKind::TypeMismatch { expected, actual } => {
                write!(formatter, "expected {expected:?}, found {actual:?}")
            }
            CheckErrorKind::InvalidOperand { operation, actual } => {
                write!(formatter, "invalid {operation} operand {actual:?}")
            }
            CheckErrorKind::BranchTypeMismatch {
                then_type,
                else_type,
            } => write!(
                formatter,
                "branch types differ: {then_type:?} and {else_type:?}"
            ),
            CheckErrorKind::ArityMismatch { expected, actual } => {
                write!(formatter, "expected {expected} arguments, found {actual}")
            }
            CheckErrorKind::BoundOverflow => formatter.write_str("inferred bound overflows i64"),
            CheckErrorKind::InvalidRange => formatter.write_str("integer range is invalid"),
            CheckErrorKind::EmptyBound => formatter.write_str("finite bound must be positive"),
            CheckErrorKind::RecursiveCallWithoutBound(name) => {
                write!(
                    formatter,
                    "recursive call to `{name}` has no explicit bound"
                )
            }
            CheckErrorKind::RecursiveBoundMissingProgress(name) => {
                write!(
                    formatter,
                    "recursive definition `{name}` has no integer progress measure"
                )
            }
            CheckErrorKind::DuplicateDefinition(name) => {
                write!(formatter, "duplicate definition `{name}`")
            }
            CheckErrorKind::DuplicateRule(name) => write!(formatter, "duplicate rule `{name}`"),
            CheckErrorKind::DuplicateBinder(name) => {
                write!(formatter, "duplicate binder `{name}`")
            }
            CheckErrorKind::InvalidDomain(actual) => {
                write!(
                    formatter,
                    "binder domain is not a compatible finite collection: {actual:?}"
                )
            }
            CheckErrorKind::UnknownRule(name) => write!(formatter, "unknown rule `{name}`"),
            CheckErrorKind::DuplicateScheduledRule(name) => {
                write!(formatter, "rule `{name}` occurs more than once in schedule")
            }
            CheckErrorKind::ScheduleCycle => formatter.write_str("schedule dependency cycle"),
            CheckErrorKind::EmptyTheory => {
                formatter.write_str("theory must contain a definition or rule")
            }
            CheckErrorKind::MutualRecursionUnsupported(names) => {
                write!(
                    formatter,
                    "mutual recursion is unsupported: {}",
                    names.join(", ")
                )
            }
            CheckErrorKind::CapacityCap { requested, maximum } => {
                write!(
                    formatter,
                    "checker needs {requested} nodes; cap is {maximum}"
                )
            }
            CheckErrorKind::InternalInvariant(message) => {
                write!(formatter, "internal checker invariant: {message}")
            }
        }
    }
}

impl std::error::Error for CheckError {}
