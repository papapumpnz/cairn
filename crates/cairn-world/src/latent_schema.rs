use crate::{
    ActionId, LatentEpisode, LatentTransitionRule, LatentTransitionTheory, StateSpace, WorldState,
};
use cairn_core::{decode_canonical, CanonicalEncode, CanonicalLimits, CanonicalValue, Hash32};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LatentSchemaRule {
    pub next_latent: u8,
    pub delta: Vec<i64>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LatentActionRole {
    pub rules: Vec<LatentSchemaRule>,
}

/// Source-independent finite hidden-state mechanism. Action IDs, state bounds,
/// support counts, source episodes, and noncanonical latent labels are absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentMechanismSchema {
    dimensions: u16,
    latent_states: u8,
    roles: Vec<LatentActionRole>,
    semantic_hash: Hash32,
}

impl LatentMechanismSchema {
    #[must_use]
    pub const fn dimensions(&self) -> u16 {
        self.dimensions
    }

    #[must_use]
    pub const fn latent_states(&self) -> u8 {
        self.latent_states
    }

    #[must_use]
    pub fn roles(&self) -> &[LatentActionRole] {
        &self.roles
    }

    #[must_use]
    pub const fn semantic_hash(&self) -> Hash32 {
        self.semantic_hash
    }

    /// Decode and fully validate a canonical source-independent schema.
    ///
    /// # Errors
    ///
    /// Rejects malformed canonical data, unsupported markers, invalid rule
    /// shapes, latent targets, and declared decode limits.
    pub fn decode_canonical(
        bytes: &[u8],
        limits: CanonicalLimits,
    ) -> Result<Self, LatentSchemaError> {
        let value =
            decode_canonical(bytes, limits).map_err(|_| LatentSchemaError::InvalidEncoding)?;
        let CanonicalValue::Array(fields) = value else {
            return Err(LatentSchemaError::InvalidEncoding);
        };
        if fields.len() != 4
            || fields.first()
                != Some(&CanonicalValue::Text(
                    "cairn.latent-mechanism-schema.v1".to_owned(),
                ))
        {
            return Err(LatentSchemaError::InvalidEncoding);
        }
        let dimensions = schema_u16(&fields[1])?;
        let latent_states = schema_u8(&fields[2])?;
        let CanonicalValue::Array(role_values) = &fields[3] else {
            return Err(LatentSchemaError::InvalidEncoding);
        };
        if dimensions == 0 || latent_states == 0 || role_values.is_empty() {
            return Err(LatentSchemaError::InvalidEncoding);
        }
        let roles = role_values
            .iter()
            .map(|role_value| decode_role(role_value, latent_states, dimensions))
            .collect::<Result<Vec<_>, _>>()?;
        if !roles.windows(2).all(|pair| pair[0] <= pair[1]) {
            return Err(LatentSchemaError::InvalidEncoding);
        }
        let canonical = schema_value(dimensions, latent_states, &roles)
            .encode_canonical()
            .map_err(|_| LatentSchemaError::Canonicalization)?;
        Ok(Self {
            dimensions,
            latent_states,
            roles,
            semantic_hash: Hash32::digest(&canonical),
        })
    }
}

impl CanonicalEncode for LatentMechanismSchema {
    fn encode_canonical(&self) -> Result<Vec<u8>, cairn_core::CanonicalError> {
        schema_value(self.dimensions, self.latent_states, &self.roles).encode_canonical()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LatentSchemaLimits {
    pub maximum_latent_permutations: u64,
    pub maximum_action_mappings: u64,
}

impl Default for LatentSchemaLimits {
    fn default() -> Self {
        Self {
            maximum_latent_permutations: 100_000,
            maximum_action_mappings: 100_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentSchemaBinding {
    pub action_roles: BTreeMap<ActionId, u16>,
    pub theory: LatentTransitionTheory,
    pub replayed_transitions: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatentSchemaError {
    EmptyTheory,
    IncompleteTheory,
    UnresolvedTheory,
    DimensionOverflow,
    DimensionMismatch,
    DuplicateAction,
    ActionCountMismatch,
    Capacity,
    NoBinding,
    Canonicalization,
    InvalidEncoding,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LatentSchemaCompiler;

impl LatentSchemaCompiler {
    /// Remove source action IDs, state bounds, support, and latent-label
    /// permutations from one complete exact theory.
    ///
    /// # Errors
    ///
    /// Rejects incomplete/unresolved theories, dimension overflow, permutation
    /// capacity, and canonical encoding failure.
    pub fn compile(
        &self,
        theory: &LatentTransitionTheory,
        limits: LatentSchemaLimits,
    ) -> Result<LatentMechanismSchema, LatentSchemaError> {
        if theory.latent_states == 0 || theory.rules.is_empty() {
            return Err(LatentSchemaError::EmptyTheory);
        }
        if !theory.unresolved_successors.is_empty() {
            return Err(LatentSchemaError::UnresolvedTheory);
        }
        let actions = theory
            .rules
            .keys()
            .map(|(_, action)| *action)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let expected = usize::from(theory.latent_states)
            .checked_mul(actions.len())
            .ok_or(LatentSchemaError::Capacity)?;
        if theory.rules.len() != expected {
            return Err(LatentSchemaError::IncompleteTheory);
        }
        let dimensions = u16::try_from(theory.space.dimensions())
            .map_err(|_| LatentSchemaError::DimensionOverflow)?;
        let permutations =
            latent_permutations(theory.latent_states, limits.maximum_latent_permutations)?;
        let mut best: Option<(Vec<u8>, Vec<LatentActionRole>)> = None;
        for old_to_new in permutations {
            let mut roles = actions
                .iter()
                .map(|action| canonical_role(theory, *action, &old_to_new))
                .collect::<Result<Vec<_>, _>>()?;
            roles.sort_unstable();
            let bytes = schema_value(dimensions, theory.latent_states, &roles)
                .encode_canonical()
                .map_err(|_| LatentSchemaError::Canonicalization)?;
            if best.as_ref().is_none_or(|(current, _)| bytes < *current) {
                best = Some((bytes, roles));
            }
        }
        let (bytes, roles) = best.ok_or(LatentSchemaError::Capacity)?;
        Ok(LatentMechanismSchema {
            dimensions,
            latent_states: theory.latent_states,
            roles,
            semantic_hash: Hash32::digest(&bytes),
        })
    }

    /// Enumerate bounded action-role bijections and retain only theories that
    /// exactly replay all current visible episodes.
    ///
    /// # Errors
    ///
    /// Rejects action/dimension mismatch, duplicates, capacity exhaustion, or
    /// absence of an exact current-world binding.
    pub fn bind(
        &self,
        schema: &LatentMechanismSchema,
        space: &StateSpace,
        actions: &[ActionId],
        episodes: &[LatentEpisode],
        limits: LatentSchemaLimits,
    ) -> Result<Vec<LatentSchemaBinding>, LatentSchemaError> {
        if usize::from(schema.dimensions) != space.dimensions() {
            return Err(LatentSchemaError::DimensionMismatch);
        }
        let mut ordered_actions = actions.to_vec();
        ordered_actions.sort_unstable();
        ordered_actions.dedup();
        if ordered_actions.len() != actions.len() {
            return Err(LatentSchemaError::DuplicateAction);
        }
        if ordered_actions.len() != schema.roles.len() {
            return Err(LatentSchemaError::ActionCountMismatch);
        }
        let mappings = index_permutations(schema.roles.len(), limits.maximum_action_mappings)?;
        let mut bindings = Vec::new();
        for role_order in mappings {
            let action_roles = ordered_actions
                .iter()
                .copied()
                .zip(role_order.iter().map(|role| u16::try_from(*role)))
                .map(|(action, role)| {
                    role.map(|role| (action, role))
                        .map_err(|_| LatentSchemaError::Capacity)
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            if let Some(binding) = replay_binding(schema, space, episodes, action_roles)? {
                bindings.push(binding);
            }
        }
        if bindings.is_empty() {
            return Err(LatentSchemaError::NoBinding);
        }
        Ok(bindings)
    }
}

fn canonical_role(
    theory: &LatentTransitionTheory,
    action: ActionId,
    old_to_new: &[u8],
) -> Result<LatentActionRole, LatentSchemaError> {
    let mut new_to_old = vec![0_u8; old_to_new.len()];
    for (old, new) in old_to_new.iter().copied().enumerate() {
        new_to_old[usize::from(new)] =
            u8::try_from(old).map_err(|_| LatentSchemaError::Capacity)?;
    }
    let rules = new_to_old
        .into_iter()
        .map(|old_latent| {
            let rule = theory
                .rules
                .get(&(old_latent, action))
                .ok_or(LatentSchemaError::IncompleteTheory)?;
            Ok(LatentSchemaRule {
                next_latent: *old_to_new
                    .get(usize::from(rule.next_latent))
                    .ok_or(LatentSchemaError::IncompleteTheory)?,
                delta: rule.delta.clone(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LatentActionRole { rules })
}

fn replay_binding(
    schema: &LatentMechanismSchema,
    space: &StateSpace,
    episodes: &[LatentEpisode],
    action_roles: BTreeMap<ActionId, u16>,
) -> Result<Option<LatentSchemaBinding>, LatentSchemaError> {
    let mut support = BTreeMap::<(u8, ActionId), u32>::new();
    let mut replayed = 0_u64;
    for episode in episodes {
        let mut latent = 0_u8;
        let mut prior: Option<&WorldState> = None;
        for transition in &episode.transitions {
            if !space.contains(&transition.before)
                || !space.contains(&transition.after)
                || prior.is_some_and(|after| after != &transition.before)
            {
                return Ok(None);
            }
            let role_schema = action_roles
                .get(&transition.action)
                .and_then(|role| schema.roles.get(usize::from(*role)));
            let Some(rule) = role_schema.and_then(|role| role.rules.get(usize::from(latent)))
            else {
                return Ok(None);
            };
            let predicted = apply_delta(space, &transition.before, &rule.delta)?;
            if predicted != transition.after {
                return Ok(None);
            }
            let entry = support.entry((latent, transition.action)).or_default();
            *entry = entry.checked_add(1).ok_or(LatentSchemaError::Capacity)?;
            latent = rule.next_latent;
            prior = Some(&transition.after);
            replayed = replayed.checked_add(1).ok_or(LatentSchemaError::Capacity)?;
        }
    }
    let rules = action_roles
        .iter()
        .flat_map(|(action, role)| {
            schema.roles[usize::from(*role)]
                .rules
                .iter()
                .enumerate()
                .map(move |(latent, rule)| (*action, latent, rule))
        })
        .map(|(action, latent, rule)| {
            let latent = u8::try_from(latent).map_err(|_| LatentSchemaError::Capacity)?;
            Ok((
                (latent, action),
                LatentTransitionRule {
                    latent,
                    action,
                    next_latent: rule.next_latent,
                    delta: rule.delta.clone(),
                    support: support.get(&(latent, action)).copied().unwrap_or_default(),
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, LatentSchemaError>>()?;
    Ok(Some(LatentSchemaBinding {
        action_roles,
        theory: LatentTransitionTheory {
            space: space.clone(),
            latent_states: schema.latent_states,
            rules,
            unresolved_successors: BTreeSet::new(),
        },
        replayed_transitions: replayed,
    }))
}

fn apply_delta(
    space: &StateSpace,
    before: &WorldState,
    delta: &[i64],
) -> Result<WorldState, LatentSchemaError> {
    if before.0.len() != delta.len() {
        return Err(LatentSchemaError::NoBinding);
    }
    before
        .0
        .iter()
        .zip(delta)
        .zip(space.lower().iter().zip(space.upper()))
        .map(|((value, delta), (lower, upper))| {
            value
                .checked_add(*delta)
                .map(|next| next.clamp(*lower, *upper))
                .ok_or(LatentSchemaError::Capacity)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(WorldState)
}

fn latent_permutations(latent_states: u8, maximum: u64) -> Result<Vec<Vec<u8>>, LatentSchemaError> {
    if maximum == 0 {
        return Err(LatentSchemaError::Capacity);
    }
    let tail = (1..latent_states).map(usize::from).collect::<Vec<_>>();
    let orders = permutations(tail, maximum)?;
    orders
        .into_iter()
        .map(|order| {
            let mut mapping = vec![0_u8; usize::from(latent_states)];
            for (new, old) in order.into_iter().enumerate() {
                mapping[old] = u8::try_from(new + 1).map_err(|_| LatentSchemaError::Capacity)?;
            }
            Ok(mapping)
        })
        .collect()
}

fn index_permutations(length: usize, maximum: u64) -> Result<Vec<Vec<usize>>, LatentSchemaError> {
    permutations((0..length).collect(), maximum)
}

fn permutations(values: Vec<usize>, maximum: u64) -> Result<Vec<Vec<usize>>, LatentSchemaError> {
    fn extend(
        remaining: &mut Vec<usize>,
        prefix: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
        maximum: u64,
    ) -> Result<(), LatentSchemaError> {
        if remaining.is_empty() {
            if u64::try_from(output.len()).map_err(|_| LatentSchemaError::Capacity)? >= maximum {
                return Err(LatentSchemaError::Capacity);
            }
            output.push(prefix.clone());
            return Ok(());
        }
        for index in 0..remaining.len() {
            let value = remaining.remove(index);
            prefix.push(value);
            extend(remaining, prefix, output, maximum)?;
            prefix.pop();
            remaining.insert(index, value);
        }
        Ok(())
    }
    let mut remaining = values;
    let mut output = Vec::new();
    extend(&mut remaining, &mut Vec::new(), &mut output, maximum)?;
    Ok(output)
}

fn schema_value(dimensions: u16, latent_states: u8, roles: &[LatentActionRole]) -> CanonicalValue {
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.latent-mechanism-schema.v1".to_owned()),
        CanonicalValue::Unsigned(u64::from(dimensions)),
        CanonicalValue::Unsigned(u64::from(latent_states)),
        CanonicalValue::Array(
            roles
                .iter()
                .map(|role| {
                    CanonicalValue::Array(
                        role.rules
                            .iter()
                            .map(|rule| {
                                CanonicalValue::Array(vec![
                                    CanonicalValue::Unsigned(u64::from(rule.next_latent)),
                                    CanonicalValue::Array(
                                        rule.delta
                                            .iter()
                                            .map(|value| {
                                                if *value >= 0 {
                                                    CanonicalValue::Unsigned(value.unsigned_abs())
                                                } else {
                                                    CanonicalValue::Negative(*value)
                                                }
                                            })
                                            .collect(),
                                    ),
                                ])
                            })
                            .collect(),
                    )
                })
                .collect(),
        ),
    ])
}

fn decode_role(
    value: &CanonicalValue,
    latent_states: u8,
    dimensions: u16,
) -> Result<LatentActionRole, LatentSchemaError> {
    let CanonicalValue::Array(rule_values) = value else {
        return Err(LatentSchemaError::InvalidEncoding);
    };
    if rule_values.len() != usize::from(latent_states) {
        return Err(LatentSchemaError::InvalidEncoding);
    }
    let rules = rule_values
        .iter()
        .map(|rule_value| {
            let CanonicalValue::Array(fields) = rule_value else {
                return Err(LatentSchemaError::InvalidEncoding);
            };
            if fields.len() != 2 {
                return Err(LatentSchemaError::InvalidEncoding);
            }
            let next_latent = schema_u8(&fields[0])?;
            if next_latent >= latent_states {
                return Err(LatentSchemaError::InvalidEncoding);
            }
            let CanonicalValue::Array(delta_values) = &fields[1] else {
                return Err(LatentSchemaError::InvalidEncoding);
            };
            if delta_values.len() != usize::from(dimensions) {
                return Err(LatentSchemaError::InvalidEncoding);
            }
            let delta = delta_values
                .iter()
                .map(schema_i64)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(LatentSchemaRule { next_latent, delta })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LatentActionRole { rules })
}

fn schema_u16(value: &CanonicalValue) -> Result<u16, LatentSchemaError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(LatentSchemaError::InvalidEncoding);
    };
    u16::try_from(*value).map_err(|_| LatentSchemaError::InvalidEncoding)
}

fn schema_u8(value: &CanonicalValue) -> Result<u8, LatentSchemaError> {
    let CanonicalValue::Unsigned(value) = value else {
        return Err(LatentSchemaError::InvalidEncoding);
    };
    u8::try_from(*value).map_err(|_| LatentSchemaError::InvalidEncoding)
}

fn schema_i64(value: &CanonicalValue) -> Result<i64, LatentSchemaError> {
    match value {
        CanonicalValue::Unsigned(value) => {
            i64::try_from(*value).map_err(|_| LatentSchemaError::InvalidEncoding)
        }
        CanonicalValue::Negative(value) if *value < 0 => Ok(*value),
        _ => Err(LatentSchemaError::InvalidEncoding),
    }
}
