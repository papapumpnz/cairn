use super::{
    validate_actions, ActionId, GoalTheory, PlanLimits, StateSpace, TransitionObservation,
    WorldError, WorldState,
};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentEpisode {
    pub transitions: Vec<TransitionObservation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LatentLimits {
    pub maximum_latent_states: u8,
    pub maximum_search_nodes: u64,
}

impl Default for LatentLimits {
    fn default() -> Self {
        Self {
            maximum_latent_states: 8,
            maximum_search_nodes: 1_000_000,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BeliefState {
    pub visible: WorldState,
    pub latent: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentTransitionRule {
    pub latent: u8,
    pub action: ActionId,
    pub next_latent: u8,
    pub delta: Vec<i64>,
    pub support: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentTransitionTheory {
    space: StateSpace,
    latent_states: u8,
    rules: BTreeMap<(u8, ActionId), LatentTransitionRule>,
    unresolved_successors: std::collections::BTreeSet<(u8, ActionId)>,
}

impl LatentTransitionTheory {
    #[must_use]
    pub const fn latent_states(&self) -> u8 {
        self.latent_states
    }

    pub fn rules(&self) -> impl Iterator<Item = &LatentTransitionRule> {
        self.rules.values()
    }

    /// Predict visible and latent outcomes under an induced hidden-state rule.
    ///
    /// # Errors
    ///
    /// Rejects invalid visible/latent states, unknown rules, and overflow.
    pub fn predict(
        &self,
        state: &BeliefState,
        action: ActionId,
    ) -> Result<BeliefState, LatentError> {
        if !self.space.contains(&state.visible) {
            return Err(LatentError::World(WorldError::StateOutOfBounds));
        }
        if state.latent >= self.latent_states {
            return Err(LatentError::LatentOutOfBounds(state.latent));
        }
        if self.unresolved_successors.contains(&(state.latent, action)) {
            return Err(LatentError::AmbiguousSuccessor {
                latent: state.latent,
                action,
            });
        }
        let rule = self
            .rules
            .get(&(state.latent, action))
            .ok_or(LatentError::UnknownRule {
                latent: state.latent,
                action,
            })?;
        let visible = WorldState(
            state
                .visible
                .0
                .iter()
                .zip(&rule.delta)
                .zip(self.space.lower().iter().zip(self.space.upper()))
                .map(|((value, delta), (lower, upper))| {
                    value
                        .checked_add(*delta)
                        .map(|next| next.clamp(*lower, *upper))
                        .ok_or(LatentError::World(WorldError::ArithmeticOverflow))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        Ok(BeliefState {
            visible,
            latent: rule.next_latent,
        })
    }

    #[must_use]
    pub const fn space(&self) -> &StateSpace {
        &self.space
    }

    #[must_use]
    pub fn unresolved_successors(&self) -> usize {
        self.unresolved_successors.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentInductionReport {
    pub theory: LatentTransitionTheory,
    pub search_nodes: u64,
    pub rejected_state_counts: u8,
    pub successor_ambiguities: Vec<LatentSuccessorAmbiguity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentSuccessorAmbiguity {
    pub latent: u8,
    pub source_action: ActionId,
    pub terminal_episode: usize,
    /// An action whose immediate visible effects differ between at least two
    /// possible successor states. `None` means a longer experiment is needed.
    pub distinguishing_action: Option<ActionId>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LatentWorldLearner;

impl LatentWorldLearner {
    /// Find the smallest complete deterministic hidden-state transducer that
    /// explains all visible action/outcome histories.
    ///
    /// # Errors
    ///
    /// Returns exact episode, bound, capacity, or unrealizability failures.
    pub fn induce(
        &self,
        space: &StateSpace,
        actions: &[ActionId],
        episodes: &[LatentEpisode],
        limits: LatentLimits,
    ) -> Result<LatentInductionReport, LatentError> {
        let actions = validate_actions(actions).map_err(LatentError::World)?;
        if episodes.is_empty()
            || episodes
                .iter()
                .any(|episode| episode.transitions.is_empty())
        {
            return Err(LatentError::EmptyEpisodes);
        }
        if limits.maximum_latent_states == 0 || limits.maximum_search_nodes == 0 {
            return Err(LatentError::World(WorldError::EmptyBound));
        }
        validate_episodes(space, episodes)?;
        let actions = actions.into_iter().collect::<Vec<_>>();
        let mut search_nodes = 0_u64;
        for latent_states in 1..=limits.maximum_latent_states {
            let mut rules = BTreeMap::new();
            if search(
                episodes,
                &actions,
                latent_states,
                0,
                0,
                0,
                &mut rules,
                &mut search_nodes,
                limits.maximum_search_nodes,
            )? {
                let rules = supported_rules(episodes, latent_states, rules)?;
                let successor_ambiguities =
                    successor_ambiguities(episodes, latent_states, &actions, &rules)?;
                let unresolved_successors = successor_ambiguities
                    .iter()
                    .map(|ambiguity| (ambiguity.latent, ambiguity.source_action))
                    .collect();
                return Ok(LatentInductionReport {
                    theory: LatentTransitionTheory {
                        space: space.clone(),
                        latent_states,
                        rules,
                        unresolved_successors,
                    },
                    search_nodes,
                    rejected_state_counts: latent_states - 1,
                    successor_ambiguities,
                });
            }
        }
        Err(LatentError::NoTheory {
            maximum_latent_states: limits.maximum_latent_states,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentPlan {
    pub actions: Vec<ActionId>,
    pub visited_states: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LatentPlanner;

impl LatentPlanner {
    /// Find a shortest plan over the joint visible and induced latent state.
    ///
    /// # Errors
    ///
    /// Returns model, capacity, length, lineage, or unreachable failures.
    pub fn plan(
        &self,
        theory: &LatentTransitionTheory,
        goal: &GoalTheory,
        initial: &BeliefState,
        limits: PlanLimits,
    ) -> Result<LatentPlan, LatentError> {
        if !theory.space.contains(&initial.visible) {
            return Err(LatentError::World(WorldError::StateOutOfBounds));
        }
        if initial.latent >= theory.latent_states {
            return Err(LatentError::LatentOutOfBounds(initial.latent));
        }
        if limits.maximum_visited_states == 0 || limits.maximum_plan_length == 0 {
            return Err(LatentError::World(WorldError::EmptyBound));
        }
        if goal.achieved(&initial.visible) {
            return Ok(LatentPlan {
                actions: Vec::new(),
                visited_states: 1,
            });
        }
        let mut frontier = VecDeque::from([initial.clone()]);
        let mut parents = BTreeMap::<BeliefState, Option<(BeliefState, ActionId)>>::new();
        parents.insert(initial.clone(), None);
        while let Some(state) = frontier.pop_front() {
            let actions = theory
                .rules
                .range((state.latent, ActionId(0))..=(state.latent, ActionId(u16::MAX)))
                .map(|(_, rule)| rule.action)
                .collect::<Vec<_>>();
            for action in actions {
                let next = theory.predict(&state, action)?;
                if next == state || parents.contains_key(&next) {
                    continue;
                }
                if parents.len() >= limits.maximum_visited_states {
                    return Err(LatentError::World(WorldError::PlanningCapacity {
                        maximum: limits.maximum_visited_states,
                    }));
                }
                parents.insert(next.clone(), Some((state.clone(), action)));
                if goal.achieved(&next.visible) {
                    let actions = reconstruct(initial, &next, &parents)?;
                    if actions.len() > limits.maximum_plan_length {
                        return Err(LatentError::World(WorldError::PlanLength {
                            maximum: limits.maximum_plan_length,
                        }));
                    }
                    return Ok(LatentPlan {
                        actions,
                        visited_states: parents.len(),
                    });
                }
                frontier.push_back(next);
            }
        }
        Err(LatentError::World(WorldError::GoalUnreachable))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LatentError {
    EmptyEpisodes,
    BrokenEpisodeChain,
    UnknownEpisodeAction(ActionId),
    LatentOutOfBounds(u8),
    UnknownRule { latent: u8, action: ActionId },
    AmbiguousSuccessor { latent: u8, action: ActionId },
    SearchCapacity { maximum: u64 },
    NoTheory { maximum_latent_states: u8 },
    InvalidPlanLineage,
    Capacity,
    World(WorldError),
}

impl fmt::Display for LatentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for LatentError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DraftRule {
    next_latent: u8,
    delta: Vec<i64>,
}

#[allow(clippy::too_many_arguments)]
fn search(
    episodes: &[LatentEpisode],
    actions: &[ActionId],
    latent_states: u8,
    episode_index: usize,
    transition_index: usize,
    current_latent: u8,
    rules: &mut BTreeMap<(u8, ActionId), DraftRule>,
    search_nodes: &mut u64,
    maximum_search_nodes: u64,
) -> Result<bool, LatentError> {
    if episode_index == episodes.len() {
        let expected = usize::from(latent_states)
            .checked_mul(actions.len())
            .ok_or(LatentError::Capacity)?;
        return Ok(rules.len() == expected);
    }
    let episode = &episodes[episode_index];
    if transition_index == episode.transitions.len() {
        return search(
            episodes,
            actions,
            latent_states,
            episode_index + 1,
            0,
            0,
            rules,
            search_nodes,
            maximum_search_nodes,
        );
    }
    let transition = &episode.transitions[transition_index];
    if !actions.contains(&transition.action) {
        return Err(LatentError::UnknownEpisodeAction(transition.action));
    }
    let delta = difference(&transition.before, &transition.after)?;
    let key = (current_latent, transition.action);
    if let Some(rule) = rules.get(&key) {
        if rule.delta != delta {
            return Ok(false);
        }
        return search(
            episodes,
            actions,
            latent_states,
            episode_index,
            transition_index + 1,
            rule.next_latent,
            rules,
            search_nodes,
            maximum_search_nodes,
        );
    }
    for next_latent in 0..latent_states {
        *search_nodes = search_nodes.saturating_add(1);
        if *search_nodes > maximum_search_nodes {
            return Err(LatentError::SearchCapacity {
                maximum: maximum_search_nodes,
            });
        }
        rules.insert(
            key,
            DraftRule {
                next_latent,
                delta: delta.clone(),
            },
        );
        if search(
            episodes,
            actions,
            latent_states,
            episode_index,
            transition_index + 1,
            next_latent,
            rules,
            search_nodes,
            maximum_search_nodes,
        )? {
            return Ok(true);
        }
        rules.remove(&key);
    }
    Ok(false)
}

fn supported_rules(
    episodes: &[LatentEpisode],
    latent_states: u8,
    rules: BTreeMap<(u8, ActionId), DraftRule>,
) -> Result<BTreeMap<(u8, ActionId), LatentTransitionRule>, LatentError> {
    let mut support = BTreeMap::<(u8, ActionId), u32>::new();
    for episode in episodes {
        let mut latent = 0_u8;
        for transition in &episode.transitions {
            let key = (latent, transition.action);
            let rule = rules.get(&key).ok_or(LatentError::UnknownRule {
                latent,
                action: transition.action,
            })?;
            *support.entry(key).or_default() = support
                .get(&key)
                .copied()
                .unwrap_or_default()
                .checked_add(1)
                .ok_or(LatentError::Capacity)?;
            latent = rule.next_latent;
        }
    }
    rules
        .into_iter()
        .map(|((latent, action), rule)| {
            Ok((
                (latent, action),
                LatentTransitionRule {
                    latent,
                    action,
                    next_latent: rule.next_latent,
                    delta: rule.delta,
                    support: support.get(&(latent, action)).copied().unwrap_or_default(),
                },
            ))
        })
        .collect::<Result<_, LatentError>>()
        .and_then(|rules: BTreeMap<_, _>| {
            if rules.keys().any(|(latent, _)| *latent >= latent_states) {
                Err(LatentError::LatentOutOfBounds(latent_states))
            } else {
                Ok(rules)
            }
        })
}

fn successor_ambiguities(
    episodes: &[LatentEpisode],
    latent_states: u8,
    actions: &[ActionId],
    rules: &BTreeMap<(u8, ActionId), LatentTransitionRule>,
) -> Result<Vec<LatentSuccessorAmbiguity>, LatentError> {
    let mut occurrences = BTreeMap::<(u8, ActionId), (bool, usize)>::new();
    for (episode_index, episode) in episodes.iter().enumerate() {
        let mut latent = 0_u8;
        for (transition_index, transition) in episode.transitions.iter().enumerate() {
            let key = (latent, transition.action);
            let is_nonterminal = transition_index + 1 < episode.transitions.len();
            let occurrence = occurrences.entry(key).or_insert((false, episode_index));
            occurrence.0 |= is_nonterminal;
            if !is_nonterminal {
                occurrence.1 = occurrence.1.min(episode_index);
            }
            latent = rules
                .get(&key)
                .ok_or(LatentError::UnknownRule {
                    latent,
                    action: transition.action,
                })?
                .next_latent;
        }
    }
    occurrences
        .into_iter()
        .filter(|(_, (has_nonterminal_support, _))| !has_nonterminal_support)
        .map(|((latent, source_action), (_, terminal_episode))| {
            let distinguishing_action = actions.iter().copied().find(|action| {
                let effects = (0..latent_states)
                    .filter_map(|candidate| {
                        rules
                            .get(&(candidate, *action))
                            .map(|rule| rule.delta.clone())
                    })
                    .collect::<std::collections::BTreeSet<_>>();
                effects.len() > 1
            });
            Ok(LatentSuccessorAmbiguity {
                latent,
                source_action,
                terminal_episode,
                distinguishing_action,
            })
        })
        .collect()
}

fn validate_episodes(space: &StateSpace, episodes: &[LatentEpisode]) -> Result<(), LatentError> {
    for episode in episodes {
        for (index, transition) in episode.transitions.iter().enumerate() {
            if !space.contains(&transition.before) || !space.contains(&transition.after) {
                return Err(LatentError::World(WorldError::StateOutOfBounds));
            }
            if index > 0 && episode.transitions[index - 1].after != transition.before {
                return Err(LatentError::BrokenEpisodeChain);
            }
        }
    }
    Ok(())
}

fn difference(before: &WorldState, after: &WorldState) -> Result<Vec<i64>, LatentError> {
    if before.0.len() != after.0.len() {
        return Err(LatentError::World(WorldError::DimensionMismatch));
    }
    before
        .0
        .iter()
        .zip(&after.0)
        .map(|(before, after)| {
            after
                .checked_sub(*before)
                .ok_or(LatentError::World(WorldError::ArithmeticOverflow))
        })
        .collect()
}

fn reconstruct(
    initial: &BeliefState,
    terminal: &BeliefState,
    parents: &BTreeMap<BeliefState, Option<(BeliefState, ActionId)>>,
) -> Result<Vec<ActionId>, LatentError> {
    let mut reversed = Vec::new();
    let mut current = terminal;
    while current != initial {
        let Some((parent, action)) = parents
            .get(current)
            .ok_or(LatentError::InvalidPlanLineage)?
        else {
            return Err(LatentError::InvalidPlanLineage);
        };
        reversed.push(*action);
        current = parent;
    }
    reversed.reverse();
    Ok(reversed)
}
