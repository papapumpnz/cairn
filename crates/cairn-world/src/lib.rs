//! Bounded, modality-neutral action-conditioned world learning.

mod latent;

pub use latent::*;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

pub const WORLD_LEARNING_SEMANTICS_VERSION: &str = "1.0.0";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WorldState(pub Vec<i64>);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ActionId(pub u16);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSpace {
    lower: Vec<i64>,
    upper: Vec<i64>,
}

impl StateSpace {
    /// Construct exact inclusive bounds.
    ///
    /// # Errors
    ///
    /// Rejects empty, mismatched, or inverted dimensions.
    pub fn new(lower: Vec<i64>, upper: Vec<i64>) -> Result<Self, WorldError> {
        if lower.is_empty() {
            return Err(WorldError::EmptyDimensions);
        }
        if lower.len() != upper.len() {
            return Err(WorldError::DimensionMismatch);
        }
        if lower.iter().zip(&upper).any(|(lower, upper)| lower > upper) {
            return Err(WorldError::InvalidBounds);
        }
        Ok(Self { lower, upper })
    }

    #[must_use]
    pub fn dimensions(&self) -> usize {
        self.lower.len()
    }

    #[must_use]
    pub fn lower(&self) -> &[i64] {
        &self.lower
    }

    #[must_use]
    pub fn upper(&self) -> &[i64] {
        &self.upper
    }

    #[must_use]
    pub fn contains(&self, state: &WorldState) -> bool {
        state.0.len() == self.dimensions()
            && state
                .0
                .iter()
                .zip(self.lower.iter().zip(&self.upper))
                .all(|(value, (lower, upper))| lower <= value && value <= upper)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionObservation {
    pub before: WorldState,
    pub action: ActionId,
    pub after: WorldState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionRule {
    pub action: ActionId,
    pub delta: Vec<i64>,
    pub support: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionTheory {
    space: StateSpace,
    rules: BTreeMap<ActionId, TransitionRule>,
}

impl TransitionTheory {
    /// Predict an action outcome using the induced effect and declared bounds.
    ///
    /// # Errors
    ///
    /// Rejects unknown actions, invalid states, or arithmetic overflow.
    pub fn predict(&self, state: &WorldState, action: ActionId) -> Result<WorldState, WorldError> {
        if !self.space.contains(state) {
            return Err(WorldError::StateOutOfBounds);
        }
        let rule = self
            .rules
            .get(&action)
            .ok_or(WorldError::UnknownAction(action))?;
        let values = state
            .0
            .iter()
            .zip(&rule.delta)
            .zip(self.space.lower.iter().zip(&self.space.upper))
            .map(|((value, delta), (lower, upper))| {
                value
                    .checked_add(*delta)
                    .map(|next| next.clamp(*lower, *upper))
                    .ok_or(WorldError::ArithmeticOverflow)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(WorldState(values))
    }

    pub fn rules(&self) -> impl Iterator<Item = &TransitionRule> {
        self.rules.values()
    }

    #[must_use]
    pub const fn space(&self) -> &StateSpace {
        &self.space
    }

    /// Rebind learned action effects to a changed layout with the same number
    /// of state dimensions.
    ///
    /// # Errors
    ///
    /// Rejects dimensional changes; effects are otherwise replayed unchanged.
    pub fn rebind_space(&self, space: StateSpace) -> Result<Self, WorldError> {
        if space.dimensions() != self.space.dimensions() {
            return Err(WorldError::DimensionMismatch);
        }
        Ok(Self {
            space,
            rules: self.rules.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalObservation {
    pub state: WorldState,
    pub achieved: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalTheory {
    required_values: Vec<Option<i64>>,
    positive_support: u32,
    negative_support: u32,
}

impl GoalTheory {
    #[must_use]
    pub fn achieved(&self, state: &WorldState) -> bool {
        state.0.len() == self.required_values.len()
            && state
                .0
                .iter()
                .zip(&self.required_values)
                .all(|(value, required)| required.is_none_or(|required| *value == required))
    }

    #[must_use]
    pub fn required_values(&self) -> &[Option<i64>] {
        &self.required_values
    }

    #[must_use]
    pub const fn positive_support(&self) -> u32 {
        self.positive_support
    }

    #[must_use]
    pub const fn negative_support(&self) -> u32 {
        self.negative_support
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WorldLearner;

impl WorldLearner {
    /// Select the deterministic set of actions whose effects remain
    /// unidentified by current transition evidence.
    ///
    /// # Errors
    ///
    /// Rejects an empty or duplicate action space.
    pub fn propose_interventions(
        &self,
        actions: &[ActionId],
        observations: &[TransitionObservation],
    ) -> Result<Vec<ActionId>, WorldError> {
        let actions = validate_actions(actions)?;
        let observed = observations
            .iter()
            .map(|observation| observation.action)
            .collect::<BTreeSet<_>>();
        Ok(actions
            .into_iter()
            .filter(|action| !observed.contains(action))
            .collect())
    }

    /// Induce one deterministic bounded additive effect for every action.
    ///
    /// # Errors
    ///
    /// Returns missing support, inconsistent effects, invalid observations, or
    /// arithmetic failures explicitly.
    pub fn induce_transitions(
        &self,
        space: &StateSpace,
        actions: &[ActionId],
        observations: &[TransitionObservation],
    ) -> Result<TransitionTheory, WorldError> {
        let actions = validate_actions(actions)?;
        let mut rules = BTreeMap::new();
        for action in actions {
            let supporting = observations
                .iter()
                .filter(|observation| observation.action == action)
                .collect::<Vec<_>>();
            if supporting.is_empty() {
                return Err(WorldError::MissingActionEvidence(action));
            }
            let first = supporting[0];
            validate_observation(space, first)?;
            let delta = difference(&first.before, &first.after)?;
            for observation in supporting.iter().skip(1) {
                validate_observation(space, observation)?;
                if difference(&observation.before, &observation.after)? != delta {
                    return Err(WorldError::InconsistentActionEffect(action));
                }
            }
            let support = u32::try_from(supporting.len()).map_err(|_| WorldError::Capacity)?;
            rules.insert(
                action,
                TransitionRule {
                    action,
                    delta,
                    support,
                },
            );
        }
        let theory = TransitionTheory {
            space: space.clone(),
            rules,
        };
        for observation in observations {
            if theory.predict(&observation.before, observation.action)? != observation.after {
                return Err(WorldError::InconsistentActionEffect(observation.action));
            }
        }
        Ok(theory)
    }

    /// Infer the minimal conjunction of state-value equalities shared by every
    /// achieved observation and separating all negative observations.
    ///
    /// # Errors
    ///
    /// Rejects missing positives, invalid states, or inseparable evidence.
    pub fn induce_goal(
        &self,
        space: &StateSpace,
        observations: &[GoalObservation],
    ) -> Result<GoalTheory, WorldError> {
        let positives = observations
            .iter()
            .filter(|observation| observation.achieved)
            .collect::<Vec<_>>();
        if positives.is_empty() {
            return Err(WorldError::MissingPositiveGoal);
        }
        for observation in observations {
            if !space.contains(&observation.state) {
                return Err(WorldError::StateOutOfBounds);
            }
        }
        let mut required_values = Vec::with_capacity(space.dimensions());
        for dimension in 0..space.dimensions() {
            let first = positives[0].state.0[dimension];
            required_values.push(
                positives
                    .iter()
                    .all(|observation| observation.state.0[dimension] == first)
                    .then_some(first),
            );
        }
        if required_values.iter().all(Option::is_none) {
            return Err(WorldError::AmbiguousGoal);
        }
        let positive_support = u32::try_from(positives.len()).map_err(|_| WorldError::Capacity)?;
        let negative_support = u32::try_from(
            observations
                .len()
                .checked_sub(positives.len())
                .ok_or(WorldError::Capacity)?,
        )
        .map_err(|_| WorldError::Capacity)?;
        let theory = GoalTheory {
            required_values,
            positive_support,
            negative_support,
        };
        if observations
            .iter()
            .any(|observation| theory.achieved(&observation.state) != observation.achieved)
        {
            return Err(WorldError::AmbiguousGoal);
        }
        Ok(theory)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanLimits {
    pub maximum_visited_states: usize,
    pub maximum_plan_length: usize,
}

impl Default for PlanLimits {
    fn default() -> Self {
        Self {
            maximum_visited_states: 100_000,
            maximum_plan_length: 10_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plan {
    pub actions: Vec<ActionId>,
    pub visited_states: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ExactPlanner;

impl ExactPlanner {
    /// Find a shortest deterministic plan by bounded breadth-first search.
    ///
    /// # Errors
    ///
    /// Returns exact invalid-state, capacity, length, or unreachable outcomes.
    pub fn plan(
        &self,
        theory: &TransitionTheory,
        goal: &GoalTheory,
        initial: &WorldState,
        limits: PlanLimits,
    ) -> Result<Plan, WorldError> {
        if !theory.space.contains(initial) {
            return Err(WorldError::StateOutOfBounds);
        }
        if limits.maximum_visited_states == 0 || limits.maximum_plan_length == 0 {
            return Err(WorldError::EmptyBound);
        }
        if goal.achieved(initial) {
            return Ok(Plan {
                actions: Vec::new(),
                visited_states: 1,
            });
        }
        let actions = theory.rules.keys().copied().collect::<Vec<_>>();
        let mut frontier = VecDeque::from([initial.clone()]);
        let mut parents = BTreeMap::<WorldState, Option<(WorldState, ActionId)>>::new();
        parents.insert(initial.clone(), None);
        while let Some(state) = frontier.pop_front() {
            for action in &actions {
                let next = theory.predict(&state, *action)?;
                if next == state || parents.contains_key(&next) {
                    continue;
                }
                if parents.len() >= limits.maximum_visited_states {
                    return Err(WorldError::PlanningCapacity {
                        maximum: limits.maximum_visited_states,
                    });
                }
                parents.insert(next.clone(), Some((state.clone(), *action)));
                if goal.achieved(&next) {
                    let actions = reconstruct(initial, &next, &parents)?;
                    if actions.len() > limits.maximum_plan_length {
                        return Err(WorldError::PlanLength {
                            maximum: limits.maximum_plan_length,
                        });
                    }
                    return Ok(Plan {
                        actions,
                        visited_states: parents.len(),
                    });
                }
                frontier.push_back(next);
            }
        }
        Err(WorldError::GoalUnreachable)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptionSkill {
    actions: Vec<ActionId>,
    displacement: Vec<i64>,
}

impl OptionSkill {
    /// Induce a reusable multi-step option from a verified plan execution.
    ///
    /// # Errors
    ///
    /// Rejects empty plans or arithmetic/model failures.
    pub fn induce(
        initial: &WorldState,
        plan: &Plan,
        theory: &TransitionTheory,
    ) -> Result<Self, WorldError> {
        if plan.actions.is_empty() {
            return Err(WorldError::EmptyOption);
        }
        let terminal = execute_actions(initial, &plan.actions, theory)?;
        let displacement = terminal
            .0
            .iter()
            .zip(&initial.0)
            .map(|(terminal, initial)| {
                terminal
                    .checked_sub(*initial)
                    .ok_or(WorldError::ArithmeticOverflow)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            actions: plan.actions.clone(),
            displacement,
        })
    }

    /// Replay the option in a new state/layout under a rebound transition
    /// theory and verify its learned displacement.
    ///
    /// # Errors
    ///
    /// Returns model, bounds, arithmetic, or transfer mismatch errors.
    pub fn transfer(
        &self,
        initial: &WorldState,
        theory: &TransitionTheory,
    ) -> Result<WorldState, WorldError> {
        let terminal = execute_actions(initial, &self.actions, theory)?;
        let displacement = terminal
            .0
            .iter()
            .zip(&initial.0)
            .map(|(terminal, initial)| {
                terminal
                    .checked_sub(*initial)
                    .ok_or(WorldError::ArithmeticOverflow)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if displacement != self.displacement {
            return Err(WorldError::OptionTransferMismatch);
        }
        Ok(terminal)
    }

    #[must_use]
    pub fn actions(&self) -> &[ActionId] {
        &self.actions
    }

    #[must_use]
    pub fn displacement(&self) -> &[i64] {
        &self.displacement
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldError {
    EmptyDimensions,
    DimensionMismatch,
    InvalidBounds,
    EmptyActionSpace,
    DuplicateAction(ActionId),
    UnknownAction(ActionId),
    MissingActionEvidence(ActionId),
    InconsistentActionEffect(ActionId),
    StateOutOfBounds,
    MissingPositiveGoal,
    AmbiguousGoal,
    ArithmeticOverflow,
    Capacity,
    EmptyBound,
    PlanningCapacity { maximum: usize },
    PlanLength { maximum: usize },
    GoalUnreachable,
    InvalidPlanLineage,
    EmptyOption,
    OptionTransferMismatch,
}

impl fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for WorldError {}

fn validate_actions(actions: &[ActionId]) -> Result<BTreeSet<ActionId>, WorldError> {
    if actions.is_empty() {
        return Err(WorldError::EmptyActionSpace);
    }
    let mut unique = BTreeSet::new();
    for action in actions {
        if !unique.insert(*action) {
            return Err(WorldError::DuplicateAction(*action));
        }
    }
    Ok(unique)
}

fn validate_observation(
    space: &StateSpace,
    observation: &TransitionObservation,
) -> Result<(), WorldError> {
    if !space.contains(&observation.before) || !space.contains(&observation.after) {
        Err(WorldError::StateOutOfBounds)
    } else {
        Ok(())
    }
}

fn difference(before: &WorldState, after: &WorldState) -> Result<Vec<i64>, WorldError> {
    if before.0.len() != after.0.len() {
        return Err(WorldError::DimensionMismatch);
    }
    before
        .0
        .iter()
        .zip(&after.0)
        .map(|(before, after)| {
            after
                .checked_sub(*before)
                .ok_or(WorldError::ArithmeticOverflow)
        })
        .collect()
}

fn reconstruct(
    initial: &WorldState,
    terminal: &WorldState,
    parents: &BTreeMap<WorldState, Option<(WorldState, ActionId)>>,
) -> Result<Vec<ActionId>, WorldError> {
    let mut reversed = Vec::new();
    let mut current = terminal;
    while current != initial {
        let Some((parent, action)) = parents.get(current).ok_or(WorldError::InvalidPlanLineage)?
        else {
            return Err(WorldError::InvalidPlanLineage);
        };
        reversed.push(*action);
        current = parent;
    }
    reversed.reverse();
    Ok(reversed)
}

fn execute_actions(
    initial: &WorldState,
    actions: &[ActionId],
    theory: &TransitionTheory,
) -> Result<WorldState, WorldError> {
    actions.iter().try_fold(initial.clone(), |state, action| {
        theory.predict(&state, *action)
    })
}
