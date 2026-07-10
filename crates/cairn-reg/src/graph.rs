use crate::{
    normalize_attributes, AttrKey, AttrValue, Attribute, DerivationId, DerivationRecord,
    EvidenceWeight, FactId, NodeConstraint, NodeId, NodeKind, RegError, RegLimits,
    RelationDefinition, RelationId, REG_SEMANTICS_VERSION,
};
use cairn_core::{CanonicalEncode, CanonicalError, CanonicalValue};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
struct NodeRecord {
    kind: NodeKind,
    attributes: Vec<Attribute>,
    derivation: DerivationId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FactRecord {
    relation: RelationId,
    arguments: Vec<NodeId>,
    attributes: Vec<Attribute>,
    weight: EvidenceWeight,
    derivation: DerivationId,
}

/// Deterministic per-relation lookup by argument position and node.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RelationIndex {
    facts: Vec<FactId>,
    by_argument: Vec<BTreeMap<NodeId, Vec<FactId>>>,
}

impl RelationIndex {
    #[must_use]
    pub fn all(&self) -> &[FactId] {
        &self.facts
    }

    #[must_use]
    pub fn at(&self, position: usize, node: NodeId) -> &[FactId] {
        self.by_argument
            .get(position)
            .and_then(|index| index.get(&node))
            .map_or(&[], Vec::as_slice)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NodeRef<'a> {
    pub id: NodeId,
    pub kind: NodeKind,
    pub attributes: &'a [Attribute],
    pub derivation: DerivationId,
}

#[derive(Clone, Copy, Debug)]
pub struct FactRef<'a> {
    pub id: FactId,
    pub relation: RelationId,
    pub arguments: &'a [NodeId],
    pub attributes: &'a [Attribute],
    pub weight: EvidenceWeight,
    pub derivation: DerivationId,
}

/// Immutable typed evidence graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelGraph {
    nodes: Vec<NodeRecord>,
    relations: Vec<RelationDefinition>,
    facts: Vec<FactRecord>,
    derivations: Vec<DerivationRecord>,
    relation_indexes: Vec<RelationIndex>,
    attribute_index: BTreeMap<(AttrKey, AttrValue), Vec<NodeId>>,
}

impl RelGraph {
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn fact_count(&self) -> usize {
        self.facts.len()
    }

    #[must_use]
    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }

    #[must_use]
    pub fn derivation_count(&self) -> usize {
        self.derivations.len()
    }

    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<NodeRef<'_>> {
        self.nodes.get(id.0 as usize).map(|node| NodeRef {
            id,
            kind: node.kind,
            attributes: &node.attributes,
            derivation: node.derivation,
        })
    }

    #[must_use]
    pub fn fact(&self, id: FactId) -> Option<FactRef<'_>> {
        self.facts.get(id.0 as usize).map(|fact| FactRef {
            id,
            relation: fact.relation,
            arguments: &fact.arguments,
            attributes: &fact.attributes,
            weight: fact.weight,
            derivation: fact.derivation,
        })
    }

    #[must_use]
    pub fn relation(&self, id: RelationId) -> Option<&RelationDefinition> {
        self.relations.get(id.0 as usize)
    }

    #[must_use]
    pub fn derivation(&self, id: DerivationId) -> Option<&DerivationRecord> {
        self.derivations.get(id.0 as usize)
    }

    #[must_use]
    pub fn relation_index(&self, id: RelationId) -> Option<&RelationIndex> {
        self.relation_indexes.get(id.0 as usize)
    }

    #[must_use]
    pub fn nodes_with_attribute(&self, key: AttrKey, value: &AttrValue) -> &[NodeId] {
        self.attribute_index
            .get(&(key, value.clone()))
            .map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn snapshot(self) -> RelGraphSnapshot {
        RelGraphSnapshot(Arc::new(self))
    }
}

impl CanonicalEncode for RelGraph {
    fn encode_canonical(&self) -> Result<Vec<u8>, CanonicalError> {
        graph_value(self).encode_canonical()
    }
}

/// Cheap immutable graph handle for concurrent readers.
#[derive(Clone, Debug)]
pub struct RelGraphSnapshot(Arc<RelGraph>);

impl RelGraphSnapshot {
    #[must_use]
    pub fn graph(&self) -> &RelGraph {
        &self.0
    }
}

impl CanonicalEncode for RelGraphSnapshot {
    fn encode_canonical(&self) -> Result<Vec<u8>, CanonicalError> {
        self.0.encode_canonical()
    }
}

/// Append-only graph constructor. Publishing consumes the builder.
#[derive(Clone, Debug)]
pub struct RelGraphBuilder {
    limits: RegLimits,
    graph: RelGraph,
    relation_names: BTreeSet<String>,
}

impl RelGraphBuilder {
    #[must_use]
    pub fn new(limits: RegLimits) -> Self {
        Self {
            limits,
            graph: RelGraph {
                nodes: Vec::new(),
                relations: Vec::new(),
                facts: Vec::new(),
                derivations: Vec::new(),
                relation_indexes: Vec::new(),
                attribute_index: BTreeMap::new(),
            },
            relation_names: BTreeSet::new(),
        }
    }

    /// Append a derivation whose inputs already exist.
    ///
    /// # Errors
    ///
    /// Rejects forward references, duplicate attributes, and capacity excess.
    pub fn add_derivation(
        &mut self,
        mut record: DerivationRecord,
    ) -> Result<DerivationId, RegError> {
        ensure_capacity(
            "derivations",
            self.graph.derivations.len(),
            self.limits.maximum_derivations,
        )?;
        for input in &record.inputs {
            if input.0 as usize >= self.graph.derivations.len() {
                return Err(RegError::UnknownDerivation(*input));
            }
        }
        record.parameters = normalize_attributes(record.parameters, self.limits)?;
        let id = DerivationId(to_u32(self.graph.derivations.len(), "derivations")?);
        self.graph.derivations.push(record);
        Ok(id)
    }

    /// Declare a uniquely named typed relation.
    ///
    /// # Errors
    ///
    /// Rejects duplicate/empty names, excessive arity, and capacity excess.
    pub fn declare_relation(
        &mut self,
        definition: RelationDefinition,
    ) -> Result<RelationId, RegError> {
        ensure_capacity(
            "relations",
            self.graph.relations.len(),
            self.limits.maximum_relations,
        )?;
        if definition.name.is_empty() {
            return Err(RegError::EmptyRelationName);
        }
        if !self.relation_names.insert(definition.name.clone()) {
            return Err(RegError::DuplicateRelationName(definition.name));
        }
        if definition.arguments.len() > usize::from(self.limits.maximum_relation_arity) {
            return Err(RegError::Capacity {
                resource: "relation arity",
                requested: definition.arguments.len() as u64,
                maximum: u64::from(self.limits.maximum_relation_arity),
            });
        }
        let id = RelationId(to_u32(self.graph.relations.len(), "relations")?);
        self.graph.relation_indexes.push(RelationIndex {
            facts: Vec::new(),
            by_argument: vec![BTreeMap::new(); definition.arguments.len()],
        });
        self.graph.relations.push(definition);
        Ok(id)
    }

    /// Append a node with canonicalized attributes and existing provenance.
    ///
    /// # Errors
    ///
    /// Rejects unknown derivations, duplicate attributes, and capacity excess.
    pub fn add_node(
        &mut self,
        kind: NodeKind,
        attributes: Vec<Attribute>,
        derivation: DerivationId,
    ) -> Result<NodeId, RegError> {
        ensure_capacity("nodes", self.graph.nodes.len(), self.limits.maximum_nodes)?;
        self.require_derivation(derivation)?;
        let attributes = normalize_attributes(attributes, self.limits)?;
        let id = NodeId(to_u32(self.graph.nodes.len(), "nodes")?);
        for attribute in &attributes {
            self.graph
                .attribute_index
                .entry((attribute.key, attribute.value.clone()))
                .or_default()
                .push(id);
        }
        self.graph.nodes.push(NodeRecord {
            kind,
            attributes,
            derivation,
        });
        Ok(id)
    }

    /// Append a typed relation fact and update deterministic indexes.
    ///
    /// # Errors
    ///
    /// Rejects unknown IDs, wrong arity/kinds, duplicate attributes, and caps.
    pub fn add_fact(
        &mut self,
        relation: RelationId,
        arguments: Vec<NodeId>,
        attributes: Vec<Attribute>,
        weight: EvidenceWeight,
        derivation: DerivationId,
    ) -> Result<FactId, RegError> {
        ensure_capacity("facts", self.graph.facts.len(), self.limits.maximum_facts)?;
        self.require_derivation(derivation)?;
        let definition = self
            .graph
            .relations
            .get(relation.0 as usize)
            .ok_or(RegError::UnknownRelation(relation))?;
        if definition.arguments.len() != arguments.len() {
            return Err(RegError::InvalidArity {
                relation,
                expected: definition.arguments.len(),
                actual: arguments.len(),
            });
        }
        for (position, (constraint, node)) in
            definition.arguments.iter().zip(&arguments).enumerate()
        {
            let actual = self
                .graph
                .nodes
                .get(node.0 as usize)
                .ok_or(RegError::UnknownNode(*node))?
                .kind;
            if let NodeConstraint::Kind(expected) = constraint {
                if *expected != actual {
                    return Err(RegError::NodeKindMismatch {
                        relation,
                        position,
                        expected: *expected,
                        actual,
                    });
                }
            }
        }
        let attributes = normalize_attributes(attributes, self.limits)?;
        let id = FactId(to_u32(self.graph.facts.len(), "facts")?);
        let index = &mut self.graph.relation_indexes[relation.0 as usize];
        index.facts.push(id);
        for (position, node) in arguments.iter().enumerate() {
            index.by_argument[position]
                .entry(*node)
                .or_default()
                .push(id);
        }
        self.graph.facts.push(FactRecord {
            relation,
            arguments,
            attributes,
            weight,
            derivation,
        });
        Ok(id)
    }

    #[must_use]
    pub fn publish(self) -> RelGraphSnapshot {
        self.graph.snapshot()
    }

    fn require_derivation(&self, id: DerivationId) -> Result<(), RegError> {
        if id.0 as usize >= self.graph.derivations.len() {
            Err(RegError::UnknownDerivation(id))
        } else {
            Ok(())
        }
    }
}

fn ensure_capacity(resource: &'static str, current: usize, maximum: u32) -> Result<(), RegError> {
    if current >= maximum as usize {
        Err(RegError::Capacity {
            resource,
            requested: current as u64 + 1,
            maximum: u64::from(maximum),
        })
    } else {
        Ok(())
    }
}

fn to_u32(value: usize, resource: &'static str) -> Result<u32, RegError> {
    u32::try_from(value).map_err(|_| RegError::Capacity {
        resource,
        requested: value as u64,
        maximum: u64::from(u32::MAX),
    })
}

fn graph_value(graph: &RelGraph) -> CanonicalValue {
    tagged(
        "rel_graph",
        vec![
            CanonicalValue::Text(REG_SEMANTICS_VERSION.to_owned()),
            CanonicalValue::Array(graph.relations.iter().map(relation_value).collect()),
            CanonicalValue::Array(graph.derivations.iter().map(derivation_value).collect()),
            CanonicalValue::Array(graph.nodes.iter().map(node_value).collect()),
            CanonicalValue::Array(graph.facts.iter().map(fact_value).collect()),
        ],
    )
}

fn relation_value(value: &RelationDefinition) -> CanonicalValue {
    tagged(
        "relation_definition",
        vec![
            CanonicalValue::Text(value.name.clone()),
            CanonicalValue::Array(
                value
                    .arguments
                    .iter()
                    .map(|constraint| match constraint {
                        NodeConstraint::Any => tagged("any", Vec::new()),
                        NodeConstraint::Kind(kind) => node_kind_value(*kind),
                    })
                    .collect(),
            ),
        ],
    )
}

fn derivation_value(value: &DerivationRecord) -> CanonicalValue {
    tagged(
        "derivation",
        vec![
            CanonicalValue::Unsigned(value.kind as u64),
            CanonicalValue::Bytes(value.operation.as_bytes().to_vec()),
            CanonicalValue::Array(
                value
                    .inputs
                    .iter()
                    .map(|id| CanonicalValue::Unsigned(u64::from(id.0)))
                    .collect(),
            ),
            attributes_value(&value.parameters),
            CanonicalValue::Text(value.semantics_version.clone()),
        ],
    )
}

fn node_value(value: &NodeRecord) -> CanonicalValue {
    tagged(
        "node",
        vec![
            node_kind_value(value.kind),
            attributes_value(&value.attributes),
            CanonicalValue::Unsigned(u64::from(value.derivation.0)),
        ],
    )
}

fn fact_value(value: &FactRecord) -> CanonicalValue {
    tagged(
        "fact",
        vec![
            CanonicalValue::Unsigned(u64::from(value.relation.0)),
            CanonicalValue::Array(
                value
                    .arguments
                    .iter()
                    .map(|id| CanonicalValue::Unsigned(u64::from(id.0)))
                    .collect(),
            ),
            attributes_value(&value.attributes),
            match value.weight {
                EvidenceWeight::Exact => tagged("exact", Vec::new()),
                EvidenceWeight::LogMilli(weight) => {
                    tagged("log_milli", vec![signed(i64::from(weight))])
                }
            },
            CanonicalValue::Unsigned(u64::from(value.derivation.0)),
        ],
    )
}

fn attributes_value(attributes: &[Attribute]) -> CanonicalValue {
    CanonicalValue::Array(
        attributes
            .iter()
            .map(|attribute| {
                tagged(
                    "attribute",
                    vec![
                        CanonicalValue::Unsigned(u64::from(attribute.key.0)),
                        attr_value(&attribute.value),
                    ],
                )
            })
            .collect(),
    )
}

fn attr_value(value: &AttrValue) -> CanonicalValue {
    match value {
        AttrValue::Bool(value) => tagged("bool", vec![CanonicalValue::Bool(*value)]),
        AttrValue::Int(value) => tagged("int", vec![signed(*value)]),
        AttrValue::Atom(value) => tagged("atom", vec![CanonicalValue::Unsigned(u64::from(*value))]),
        AttrValue::Text(value) => tagged("text", vec![CanonicalValue::Text(value.clone())]),
        AttrValue::Bytes(value) => tagged("bytes", vec![CanonicalValue::Bytes(value.clone())]),
        AttrValue::Hash(value) => tagged(
            "hash",
            vec![CanonicalValue::Bytes(value.as_bytes().to_vec())],
        ),
    }
}

fn node_kind_value(kind: NodeKind) -> CanonicalValue {
    match kind {
        NodeKind::Custom(value) => tagged(
            "custom_kind",
            vec![CanonicalValue::Unsigned(u64::from(value))],
        ),
        other => tagged(
            "node_kind",
            vec![CanonicalValue::Unsigned(node_kind_code(other))],
        ),
    }
}

const fn node_kind_code(kind: NodeKind) -> u64 {
    match kind {
        NodeKind::Observation => 0,
        NodeKind::Scalar => 1,
        NodeKind::Atom => 2,
        NodeKind::EntityHypothesis => 3,
        NodeKind::EntitySet => 4,
        NodeKind::RelationValue => 5,
        NodeKind::Action => 6,
        NodeKind::State => 7,
        NodeKind::Event => 8,
        NodeKind::Role => 9,
        NodeKind::Latent => 10,
        NodeKind::Definition => 11,
        NodeKind::Goal => 12,
        NodeKind::Skill => 13,
        NodeKind::KnowledgeClaim => 14,
        NodeKind::Custom(_) => 15,
    }
}

fn signed(value: i64) -> CanonicalValue {
    if value >= 0 {
        CanonicalValue::Unsigned(value.unsigned_abs())
    } else {
        CanonicalValue::Negative(value)
    }
}

fn tagged(name: &str, values: Vec<CanonicalValue>) -> CanonicalValue {
    let mut output = Vec::with_capacity(values.len() + 1);
    output.push(CanonicalValue::Text(name.to_owned()));
    output.extend(values);
    CanonicalValue::Array(output)
}
