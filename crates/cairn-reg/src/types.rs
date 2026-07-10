use cairn_core::Hash32;
use std::fmt;

macro_rules! local_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name(pub u32);
    };
}

local_id!(NodeId);
local_id!(FactId);
local_id!(RelationId);
local_id!(DerivationId);
local_id!(AttrKey);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct LogicalTime(pub u64);

/// Environment-neutral evidence node categories.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NodeKind {
    Observation,
    Scalar,
    Atom,
    EntityHypothesis,
    EntitySet,
    RelationValue,
    Action,
    State,
    Event,
    Role,
    Latent,
    Definition,
    Goal,
    Skill,
    KnowledgeClaim,
    Custom(u16),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AttrValue {
    Bool(bool),
    Int(i64),
    Atom(u32),
    Text(String),
    Bytes(Vec<u8>),
    Hash(Hash32),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Attribute {
    pub key: AttrKey,
    pub value: AttrValue,
}

/// Exact evidence or explicitly approximate fixed-point log weight.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EvidenceWeight {
    Exact,
    LogMilli(i32),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DerivationKind {
    Observed,
    DeterministicKernel,
    AlgebraicRewrite,
    Hypothesis,
    ConstraintChoice,
    MemoryProposal,
    ExternalEvidence,
    Verification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationRecord {
    pub kind: DerivationKind,
    pub operation: Hash32,
    pub inputs: Vec<DerivationId>,
    pub parameters: Vec<Attribute>,
    pub semantics_version: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NodeConstraint {
    Any,
    Kind(NodeKind),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationDefinition {
    pub name: String,
    pub arguments: Vec<NodeConstraint>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegLimits {
    pub maximum_nodes: u32,
    pub maximum_facts: u32,
    pub maximum_relations: u32,
    pub maximum_derivations: u32,
    pub maximum_relation_arity: u16,
    pub maximum_attributes_per_record: u16,
    pub maximum_path_hops: u16,
    pub maximum_path_visited: u32,
}

impl Default for RegLimits {
    fn default() -> Self {
        Self {
            maximum_nodes: 1_000_000,
            maximum_facts: 4_000_000,
            maximum_relations: 65_536,
            maximum_derivations: 4_000_000,
            maximum_relation_arity: 32,
            maximum_attributes_per_record: 256,
            maximum_path_hops: 1_024,
            maximum_path_visited: 1_000_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegError {
    Capacity {
        resource: &'static str,
        requested: u64,
        maximum: u64,
    },
    UnknownNode(NodeId),
    UnknownRelation(RelationId),
    UnknownDerivation(DerivationId),
    InvalidArity {
        relation: RelationId,
        expected: usize,
        actual: usize,
    },
    NodeKindMismatch {
        relation: RelationId,
        position: usize,
        expected: NodeKind,
        actual: NodeKind,
    },
    DuplicateAttribute(AttrKey),
    DuplicateRelationName(String),
    EmptyRelationName,
    QueryVariableOutOfRange(u16),
    QueryArity {
        relation: RelationId,
        expected: usize,
        actual: usize,
    },
    QueryUnboundVariable(u16),
    PathRequiresBinaryRelation(RelationId),
    EmptyPathBound,
    ResultColumnOutOfRange(usize),
}

impl fmt::Display for RegError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capacity {
                resource,
                requested,
                maximum,
            } => write!(
                formatter,
                "{resource} capacity exceeded: requested {requested}, maximum {maximum}"
            ),
            Self::UnknownNode(id) => write!(formatter, "unknown node {}", id.0),
            Self::UnknownRelation(id) => write!(formatter, "unknown relation {}", id.0),
            Self::UnknownDerivation(id) => write!(formatter, "unknown derivation {}", id.0),
            Self::InvalidArity {
                relation,
                expected,
                actual,
            }
            | Self::QueryArity {
                relation,
                expected,
                actual,
            } => write!(
                formatter,
                "relation {} expects {expected} arguments, found {actual}",
                relation.0
            ),
            Self::NodeKindMismatch {
                relation,
                position,
                expected,
                actual,
            } => write!(
                formatter,
                "relation {} argument {position} expects {expected:?}, found {actual:?}",
                relation.0
            ),
            Self::DuplicateAttribute(key) => write!(formatter, "duplicate attribute key {}", key.0),
            Self::DuplicateRelationName(name) => {
                write!(formatter, "duplicate relation name `{name}`")
            }
            Self::EmptyRelationName => formatter.write_str("relation name must not be empty"),
            Self::QueryVariableOutOfRange(variable) => {
                write!(
                    formatter,
                    "query variable {variable} exceeds declared width"
                )
            }
            Self::QueryUnboundVariable(variable) => {
                write!(formatter, "query variable {variable} is unbound")
            }
            Self::PathRequiresBinaryRelation(relation) => {
                write!(formatter, "path relation {} is not binary", relation.0)
            }
            Self::EmptyPathBound => formatter.write_str("path hop bound must be positive"),
            Self::ResultColumnOutOfRange(column) => {
                write!(formatter, "result column {column} is out of range")
            }
        }
    }
}

impl std::error::Error for RegError {}

pub(crate) fn normalize_attributes(
    mut attributes: Vec<Attribute>,
    limits: RegLimits,
) -> Result<Vec<Attribute>, RegError> {
    if attributes.len() > usize::from(limits.maximum_attributes_per_record) {
        return Err(RegError::Capacity {
            resource: "attributes",
            requested: attributes.len() as u64,
            maximum: u64::from(limits.maximum_attributes_per_record),
        });
    }
    attributes.sort();
    if let Some(pair) = attributes
        .windows(2)
        .find(|pair| pair[0].key == pair[1].key)
    {
        return Err(RegError::DuplicateAttribute(pair[0].key));
    }
    Ok(attributes)
}
