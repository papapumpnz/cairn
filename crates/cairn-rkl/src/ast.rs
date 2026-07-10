use cairn_core::Hash32;
use std::collections::BTreeSet;

/// Inclusive exact integer range.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct IntRange {
    pub lower: i64,
    pub upper: i64,
}

impl IntRange {
    #[must_use]
    pub const fn exact(value: i64) -> Self {
        Self {
            lower: value,
            upper: value,
        }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.lower <= self.upper
    }
}

/// Closed RKL type universe. Every collection carries a finite maximum.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Type {
    Bool,
    Int(IntRange),
    /// An uninterpreted finite categorical value supplied by an adapter.
    Atom,
    Coord2,
    Vector2,
    Axis,
    Transform2,
    CellSet {
        maximum: u32,
    },
    Shape,
    Entity,
    EntitySet {
        maximum: u32,
    },
    Symbol,
    Action,
    State,
    /// A bounded finite observation field. Axis meaning belongs to its adapter.
    FiniteField {
        axes: u8,
        maximum_elements: u64,
    },
    Tuple(Vec<Self>),
    Option(Box<Self>),
    List {
        element: Box<Self>,
        maximum: u32,
    },
    Map {
        key: Box<Self>,
        value: Box<Self>,
        maximum: u32,
    },
    Relation {
        arguments: Vec<Self>,
        maximum: u32,
    },
    Operator {
        input: Box<Self>,
        output: Box<Self>,
    },
    Unit,
}

impl Type {
    #[must_use]
    pub const fn collection_maximum(&self) -> Option<u32> {
        match self {
            Self::CellSet { maximum }
            | Self::EntitySet { maximum }
            | Self::List { maximum, .. }
            | Self::Map { maximum, .. }
            | Self::Relation { maximum, .. } => Some(*maximum),
            _ => None,
        }
    }
}

/// Source/debug variable name. Bound names are removed from semantic identity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VarName(pub String);

/// Scalar and collection primitive operators only.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UnaryOp {
    Not,
    Negate,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Minimum,
    Maximum,
    And,
    Or,
    Equal,
    LessThan,
    Union,
    Intersection,
    Difference,
}

impl BinaryOp {
    #[must_use]
    pub const fn is_commutative(self) -> bool {
        matches!(
            self,
            Self::Add
                | Self::Multiply
                | Self::Minimum
                | Self::Maximum
                | Self::And
                | Self::Or
                | Self::Equal
                | Self::Union
                | Self::Intersection
        )
    }

    #[must_use]
    pub const fn is_associative(self) -> bool {
        matches!(
            self,
            Self::Add
                | Self::Multiply
                | Self::Minimum
                | Self::Maximum
                | Self::And
                | Self::Or
                | Self::Union
                | Self::Intersection
        )
    }
}

/// Validated-by-checker expression syntax.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Bool(bool),
    Int(i64),
    Atom(u16),
    Coord2 {
        row: i32,
        column: i32,
    },
    Var(VarName),
    Tuple(Vec<Self>),
    Let {
        binder: VarName,
        value: Box<Self>,
        body: Box<Self>,
    },
    If {
        condition: Box<Self>,
        then_branch: Box<Self>,
        else_branch: Box<Self>,
    },
    Unary {
        op: UnaryOp,
        argument: Box<Self>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Self>,
        right: Box<Self>,
    },
    EmptySet {
        element: Type,
        maximum: u32,
    },
    SingletonSet {
        value: Box<Self>,
        maximum: u32,
    },
    Count(Box<Self>),
    Call {
        function: String,
        arguments: Vec<Self>,
    },
    BoundedFixpoint {
        binder: VarName,
        initial: Box<Self>,
        step: Box<Self>,
        maximum_steps: u32,
        progress_measure: Box<Self>,
    },
}

/// Checked expression plus exact inferred type and non-semantic proof lineage.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedExpr {
    pub(crate) expression: Expr,
    pub(crate) ty: Type,
    pub(crate) free_variables: BTreeSet<VarName>,
    pub(crate) proof_annotations: Vec<Hash32>,
}

impl TypedExpr {
    #[must_use]
    pub const fn expression(&self) -> &Expr {
        &self.expression
    }

    #[must_use]
    pub const fn ty(&self) -> &Type {
        &self.ty
    }

    #[must_use]
    pub const fn free_variables(&self) -> &BTreeSet<VarName> {
        &self.free_variables
    }

    #[must_use]
    pub fn proof_annotations(&self) -> &[Hash32] {
        &self.proof_annotations
    }

    /// Attach non-semantic proof lineage without changing canonical identity.
    pub fn add_proof_annotation(&mut self, annotation: Hash32) {
        self.proof_annotations.push(annotation);
    }
}

/// Predicate wrapper constructible only from a checked Boolean expression.
#[derive(Clone, Debug, PartialEq)]
pub struct PredicateExpr(pub(crate) TypedExpr);

impl PredicateExpr {
    #[must_use]
    pub const fn typed(&self) -> &TypedExpr {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binder {
    pub name: VarName,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecursionBound {
    pub maximum_steps: u32,
    pub progress_measure: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    pub name: String,
    pub parameters: Vec<Binder>,
    pub return_type: Type,
    pub body: Expr,
    pub recursion: Option<RecursionBound>,
    pub proof_annotations: Vec<Hash32>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Quantifier {
    One,
    Unique,
    ForEach,
    Group,
    Exists,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuleBinder {
    pub variable: Binder,
    pub domain: Expr,
    pub quantifier: Quantifier,
}

/// Low-level effects; higher-level conveniences must lower to these forms.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Preserve(Expr),
    Delete(Expr),
    EmitCells(Expr),
    SetState { target: Expr, value: Expr },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RewriteRule {
    pub name: String,
    pub binders: Vec<RuleBinder>,
    pub guard: Expr,
    pub effects: Vec<Effect>,
    pub priority: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Schedule {
    Parallel,
    Sequence(Vec<String>),
    DependencyDag(Vec<(String, String)>),
    DecisionList(Vec<String>),
    BoundedFixpoint {
        rules: Vec<String>,
        maximum_steps: u32,
    },
    Hierarchical(Vec<Self>),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConflictPolicy {
    Reject,
    SameValueIdempotent,
    Priority,
    LastWrite,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Renderer {
    pub canvas: Expr,
    pub layers: Vec<Expr>,
    pub conflict_policy: ConflictPolicy,
    pub background: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Theory {
    pub definitions: Vec<Definition>,
    pub rules: Vec<RewriteRule>,
    pub schedule: Schedule,
    pub renderer: Renderer,
    pub assumptions: Vec<Expr>,
    pub proof_annotations: Vec<Hash32>,
}
