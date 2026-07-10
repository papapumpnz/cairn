use crate::{
    AttrKey, AttrValue, Attribute, DerivationId, DerivationKind, DerivationRecord, EvidenceWeight,
    LogicalTime, NodeConstraint, NodeId, NodeKind, RegDecodeError, RegLimits, RelGraphBuilder,
    RelGraphSnapshot, RelationDefinition, RelationId, REG_SEMANTICS_VERSION,
};
use cairn_core::{decode_canonical, CanonicalEncode, CanonicalLimits, CanonicalValue, Hash32};

impl RelGraphSnapshot {
    /// Decode, validate, and re-index a canonical REG artifact.
    ///
    /// # Errors
    ///
    /// Rejects malformed/non-canonical CBOR, unsupported semantics, invalid
    /// typed references, non-canonical semantic ordering, and resource excess.
    pub fn decode_canonical(
        bytes: &[u8],
        canonical_limits: CanonicalLimits,
        graph_limits: RegLimits,
    ) -> Result<Self, RegDecodeError> {
        let value = decode_canonical(bytes, canonical_limits)?;
        let fields = take_tag(value, "rel_graph", 5)?;
        let mut fields = fields.into_iter();
        let version = take_text(next(&mut fields)?)?;
        if version != REG_SEMANTICS_VERSION {
            return Err(RegDecodeError::UnsupportedSemantics(version));
        }
        let relations = take_array(next(&mut fields)?)?;
        let derivations = take_array(next(&mut fields)?)?;
        let nodes = take_array(next(&mut fields)?)?;
        let facts = take_array(next(&mut fields)?)?;

        let mut builder = RelGraphBuilder::new(graph_limits);
        for relation in relations {
            builder.declare_relation(parse_relation(relation)?)?;
        }
        for derivation in derivations {
            builder.add_derivation(parse_derivation(derivation)?)?;
        }
        for node in nodes {
            let (kind, attributes, derivation, time) = parse_node(node)?;
            builder.add_node_at(kind, attributes, derivation, time)?;
        }
        for fact in facts {
            let (relation, arguments, attributes, weight, derivation, time) = parse_fact(fact)?;
            builder.add_fact_at(relation, arguments, attributes, weight, derivation, time)?;
        }
        let snapshot = builder.publish();
        if snapshot.encode_canonical()?.as_slice() != bytes {
            return Err(RegDecodeError::InvalidShape(
                "semantic fields are not in canonical order",
            ));
        }
        Ok(snapshot)
    }
}

fn parse_relation(value: CanonicalValue) -> Result<RelationDefinition, RegDecodeError> {
    let fields = take_tag(value, "relation_definition", 2)?;
    let mut fields = fields.into_iter();
    let name = take_text(next(&mut fields)?)?;
    let arguments = take_array(next(&mut fields)?)?
        .into_iter()
        .map(parse_constraint)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RelationDefinition { name, arguments })
}

fn parse_constraint(value: CanonicalValue) -> Result<NodeConstraint, RegDecodeError> {
    match tag_name(&value)? {
        "any" => {
            take_tag(value, "any", 0)?;
            Ok(NodeConstraint::Any)
        }
        "node_kind" | "custom_kind" => parse_node_kind(value).map(NodeConstraint::Kind),
        _ => Err(RegDecodeError::InvalidShape("unknown node constraint")),
    }
}

fn parse_derivation(value: CanonicalValue) -> Result<DerivationRecord, RegDecodeError> {
    let fields = take_tag(value, "derivation", 5)?;
    let mut fields = fields.into_iter();
    let kind = parse_derivation_kind(take_u64(&next(&mut fields)?)?)?;
    let operation = take_hash(next(&mut fields)?)?;
    let inputs = take_array(next(&mut fields)?)?
        .into_iter()
        .map(|value| take_u32(&value).map(DerivationId))
        .collect::<Result<Vec<_>, _>>()?;
    let parameters = parse_attributes(next(&mut fields)?)?;
    let semantics_version = take_text(next(&mut fields)?)?;
    Ok(DerivationRecord {
        kind,
        operation,
        inputs,
        parameters,
        semantics_version,
    })
}

fn parse_node(
    value: CanonicalValue,
) -> Result<(NodeKind, Vec<Attribute>, DerivationId, Option<LogicalTime>), RegDecodeError> {
    let fields = take_tag(value, "node", 4)?;
    let mut fields = fields.into_iter();
    Ok((
        parse_node_kind(next(&mut fields)?)?,
        parse_attributes(next(&mut fields)?)?,
        DerivationId(take_u32(&next(&mut fields)?)?),
        parse_time(&next(&mut fields)?)?,
    ))
}

type ParsedFact = (
    RelationId,
    Vec<NodeId>,
    Vec<Attribute>,
    EvidenceWeight,
    DerivationId,
    Option<LogicalTime>,
);

fn parse_fact(value: CanonicalValue) -> Result<ParsedFact, RegDecodeError> {
    let fields = take_tag(value, "fact", 6)?;
    let mut fields = fields.into_iter();
    let relation = RelationId(take_u32(&next(&mut fields)?)?);
    let arguments = take_array(next(&mut fields)?)?
        .into_iter()
        .map(|value| take_u32(&value).map(NodeId))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        relation,
        arguments,
        parse_attributes(next(&mut fields)?)?,
        parse_weight(next(&mut fields)?)?,
        DerivationId(take_u32(&next(&mut fields)?)?),
        parse_time(&next(&mut fields)?)?,
    ))
}

fn parse_attributes(value: CanonicalValue) -> Result<Vec<Attribute>, RegDecodeError> {
    take_array(value)?
        .into_iter()
        .map(|value| {
            let fields = take_tag(value, "attribute", 2)?;
            let mut fields = fields.into_iter();
            Ok(Attribute {
                key: AttrKey(take_u32(&next(&mut fields)?)?),
                value: parse_attr_value(next(&mut fields)?)?,
            })
        })
        .collect()
}

fn parse_attr_value(value: CanonicalValue) -> Result<AttrValue, RegDecodeError> {
    let tag = tag_name(&value)?.to_owned();
    let fields = take_tag(value, &tag, 1)?;
    let mut fields = fields.into_iter();
    let value = next(&mut fields)?;
    match tag.as_str() {
        "bool" => take_bool(&value).map(AttrValue::Bool),
        "int" => take_i64(&value).map(AttrValue::Int),
        "atom" => take_u32(&value).map(AttrValue::Atom),
        "text" => take_text(value).map(AttrValue::Text),
        "bytes" => take_bytes(value).map(AttrValue::Bytes),
        "hash" => take_hash(value).map(AttrValue::Hash),
        _ => Err(RegDecodeError::InvalidShape("unknown attribute value")),
    }
}

fn parse_weight(value: CanonicalValue) -> Result<EvidenceWeight, RegDecodeError> {
    match tag_name(&value)? {
        "exact" => {
            take_tag(value, "exact", 0)?;
            Ok(EvidenceWeight::Exact)
        }
        "log_milli" => {
            let fields = take_tag(value, "log_milli", 1)?;
            let weight_value = fields
                .into_iter()
                .next()
                .ok_or(RegDecodeError::InvalidShape("missing log weight"))?;
            let weight = i32::try_from(take_i64(&weight_value)?)
                .map_err(|_| RegDecodeError::IntegerOutOfRange)?;
            Ok(EvidenceWeight::LogMilli(weight))
        }
        _ => Err(RegDecodeError::InvalidShape("unknown evidence weight")),
    }
}

fn parse_node_kind(value: CanonicalValue) -> Result<NodeKind, RegDecodeError> {
    match tag_name(&value)? {
        "custom_kind" => {
            let fields = take_tag(value, "custom_kind", 1)?;
            let custom_value = fields
                .into_iter()
                .next()
                .ok_or(RegDecodeError::InvalidShape("missing custom kind"))?;
            let custom = u16::try_from(take_u64(&custom_value)?)
                .map_err(|_| RegDecodeError::IntegerOutOfRange)?;
            Ok(NodeKind::Custom(custom))
        }
        "node_kind" => {
            let fields = take_tag(value, "node_kind", 1)?;
            let code_value = fields
                .into_iter()
                .next()
                .ok_or(RegDecodeError::InvalidShape("missing node kind"))?;
            let code = take_u64(&code_value)?;
            match code {
                0 => Ok(NodeKind::Observation),
                1 => Ok(NodeKind::Scalar),
                2 => Ok(NodeKind::Atom),
                3 => Ok(NodeKind::EntityHypothesis),
                4 => Ok(NodeKind::EntitySet),
                5 => Ok(NodeKind::RelationValue),
                6 => Ok(NodeKind::Action),
                7 => Ok(NodeKind::State),
                8 => Ok(NodeKind::Event),
                9 => Ok(NodeKind::Role),
                10 => Ok(NodeKind::Latent),
                11 => Ok(NodeKind::Definition),
                12 => Ok(NodeKind::Goal),
                13 => Ok(NodeKind::Skill),
                14 => Ok(NodeKind::KnowledgeClaim),
                _ => Err(RegDecodeError::InvalidShape("unknown node kind")),
            }
        }
        _ => Err(RegDecodeError::InvalidShape("invalid node kind tag")),
    }
}

fn parse_derivation_kind(value: u64) -> Result<DerivationKind, RegDecodeError> {
    match value {
        0 => Ok(DerivationKind::Observed),
        1 => Ok(DerivationKind::DeterministicKernel),
        2 => Ok(DerivationKind::AlgebraicRewrite),
        3 => Ok(DerivationKind::Hypothesis),
        4 => Ok(DerivationKind::ConstraintChoice),
        5 => Ok(DerivationKind::MemoryProposal),
        6 => Ok(DerivationKind::ExternalEvidence),
        7 => Ok(DerivationKind::Verification),
        _ => Err(RegDecodeError::InvalidShape("unknown derivation kind")),
    }
}

fn parse_time(value: &CanonicalValue) -> Result<Option<LogicalTime>, RegDecodeError> {
    match value {
        CanonicalValue::Null => Ok(None),
        CanonicalValue::Unsigned(value) => Ok(Some(LogicalTime(*value))),
        _ => Err(RegDecodeError::InvalidShape("invalid logical time")),
    }
}

fn take_tag(
    value: CanonicalValue,
    expected: &str,
    field_count: usize,
) -> Result<Vec<CanonicalValue>, RegDecodeError> {
    let mut values = take_array(value)?.into_iter();
    let actual = take_text(next(&mut values)?)?;
    if actual != expected {
        return Err(RegDecodeError::InvalidShape("unexpected semantic tag"));
    }
    let fields = values.collect::<Vec<_>>();
    if fields.len() != field_count {
        return Err(RegDecodeError::InvalidShape("wrong semantic field count"));
    }
    Ok(fields)
}

fn tag_name(value: &CanonicalValue) -> Result<&str, RegDecodeError> {
    match value {
        CanonicalValue::Array(values) => match values.first() {
            Some(CanonicalValue::Text(tag)) => Ok(tag),
            _ => Err(RegDecodeError::InvalidShape("semantic tag is not text")),
        },
        _ => Err(RegDecodeError::InvalidShape("tagged value is not an array")),
    }
}

fn next(
    values: &mut impl Iterator<Item = CanonicalValue>,
) -> Result<CanonicalValue, RegDecodeError> {
    values
        .next()
        .ok_or(RegDecodeError::InvalidShape("missing semantic field"))
}

fn take_array(value: CanonicalValue) -> Result<Vec<CanonicalValue>, RegDecodeError> {
    if let CanonicalValue::Array(values) = value {
        Ok(values)
    } else {
        Err(RegDecodeError::InvalidShape("expected array"))
    }
}

fn take_text(value: CanonicalValue) -> Result<String, RegDecodeError> {
    if let CanonicalValue::Text(value) = value {
        Ok(value)
    } else {
        Err(RegDecodeError::InvalidShape("expected text"))
    }
}

fn take_bytes(value: CanonicalValue) -> Result<Vec<u8>, RegDecodeError> {
    if let CanonicalValue::Bytes(value) = value {
        Ok(value)
    } else {
        Err(RegDecodeError::InvalidShape("expected bytes"))
    }
}

fn take_bool(value: &CanonicalValue) -> Result<bool, RegDecodeError> {
    if let CanonicalValue::Bool(value) = value {
        Ok(*value)
    } else {
        Err(RegDecodeError::InvalidShape("expected Boolean"))
    }
}

fn take_u64(value: &CanonicalValue) -> Result<u64, RegDecodeError> {
    if let CanonicalValue::Unsigned(value) = value {
        Ok(*value)
    } else {
        Err(RegDecodeError::InvalidShape("expected unsigned integer"))
    }
}

fn take_u32(value: &CanonicalValue) -> Result<u32, RegDecodeError> {
    u32::try_from(take_u64(value)?).map_err(|_| RegDecodeError::IntegerOutOfRange)
}

fn take_i64(value: &CanonicalValue) -> Result<i64, RegDecodeError> {
    match value {
        CanonicalValue::Unsigned(value) => {
            i64::try_from(*value).map_err(|_| RegDecodeError::IntegerOutOfRange)
        }
        CanonicalValue::Negative(value) => Ok(*value),
        _ => Err(RegDecodeError::InvalidShape("expected integer")),
    }
}

fn take_hash(value: CanonicalValue) -> Result<Hash32, RegDecodeError> {
    let bytes = take_bytes(value)?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| RegDecodeError::InvalidShape("hash must contain 32 bytes"))?;
    Ok(Hash32::from_bytes(bytes))
}
