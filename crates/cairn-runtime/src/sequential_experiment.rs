use cairn_core::Hash32;
use std::collections::{BTreeMap, BTreeSet};

/// One exact action-conditioned transition inside a competing causal theory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SequentialTransition {
    pub next_state: Hash32,
    pub visible_outcome: Hash32,
}

/// A finite executable world hypothesis. State identities are internal to the
/// hypothesis; only visible outcome traces are compared during experiments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequentialTheoryHypothesis {
    pub id: Hash32,
    pub initial_state: Hash32,
    pub transitions: BTreeMap<(Hash32, Hash32), SequentialTransition>,
}

/// Explicit CPU work bounds for deterministic experiment synthesis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SequentialExperimentLimits {
    pub maximum_depth: u16,
    pub maximum_plans: u64,
}

impl Default for SequentialExperimentLimits {
    fn default() -> Self {
        Self {
            maximum_depth: 4,
            maximum_plans: 100_000,
        }
    }
}

/// The shortest canonical action sequence that separates current theories.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequentialExperimentPlan {
    pub actions: Vec<Hash32>,
    pub outcome_partitions: usize,
    pub theories_compared: usize,
    pub plans_examined: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequentialExperimentError {
    EmptyVersionSpace,
    DuplicateTheory,
    EmptyActions,
    DuplicateAction,
    EmptyBound,
    Capacity { maximum: u64 },
    NoDistinguishingPlan,
    TraceLengthMismatch,
    AllTheoriesRefuted,
}

/// Exact version space that can invent a multi-action diagnostic experiment
/// when every individual action has the same visible prediction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequentialVersionSpace {
    theories: BTreeMap<Hash32, SequentialTheoryHypothesis>,
}

impl SequentialVersionSpace {
    /// Construct a deterministic, nonempty theory set.
    ///
    /// # Errors
    ///
    /// Rejects empty and duplicate hypothesis identities.
    pub fn new(
        theories: Vec<SequentialTheoryHypothesis>,
    ) -> Result<Self, SequentialExperimentError> {
        if theories.is_empty() {
            return Err(SequentialExperimentError::EmptyVersionSpace);
        }
        let count = theories.len();
        let theories = theories
            .into_iter()
            .map(|theory| (theory.id, theory))
            .collect::<BTreeMap<_, _>>();
        if theories.len() != count {
            return Err(SequentialExperimentError::DuplicateTheory);
        }
        Ok(Self { theories })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.theories.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.theories.is_empty()
    }

    /// Enumerate sequences by length and lexicographic action identity, then
    /// return the first maximum-partition plan at the shortest useful depth.
    ///
    /// # Errors
    ///
    /// Rejects malformed action sets, empty bounds, plan capacity, or a version
    /// space indistinguishable within the declared depth.
    pub fn next_discriminating_plan(
        &self,
        actions: &[Hash32],
        limits: SequentialExperimentLimits,
    ) -> Result<SequentialExperimentPlan, SequentialExperimentError> {
        let ordered_actions = actions.iter().copied().collect::<BTreeSet<_>>();
        if ordered_actions.is_empty() {
            return Err(SequentialExperimentError::EmptyActions);
        }
        if ordered_actions.len() != actions.len() {
            return Err(SequentialExperimentError::DuplicateAction);
        }
        if limits.maximum_depth == 0 || limits.maximum_plans == 0 {
            return Err(SequentialExperimentError::EmptyBound);
        }
        let ordered_actions = ordered_actions.into_iter().collect::<Vec<_>>();
        let mut frontier = vec![Vec::<Hash32>::new()];
        let mut examined = 0_u64;
        for _ in 0..limits.maximum_depth {
            let mut next = Vec::new();
            let mut best: Option<SequentialExperimentPlan> = None;
            for prefix in frontier {
                for action in &ordered_actions {
                    examined =
                        examined
                            .checked_add(1)
                            .ok_or(SequentialExperimentError::Capacity {
                                maximum: limits.maximum_plans,
                            })?;
                    if examined > limits.maximum_plans {
                        return Err(SequentialExperimentError::Capacity {
                            maximum: limits.maximum_plans,
                        });
                    }
                    let mut plan = prefix.clone();
                    plan.push(*action);
                    let traces = self
                        .theories
                        .values()
                        .map(|theory| execute_trace(theory, &plan))
                        .collect::<Option<BTreeSet<_>>>();
                    if let Some(traces) = traces {
                        if traces.len() > 1 {
                            let candidate = SequentialExperimentPlan {
                                actions: plan.clone(),
                                outcome_partitions: traces.len(),
                                theories_compared: self.theories.len(),
                                plans_examined: examined,
                            };
                            if best.as_ref().is_none_or(|current| {
                                candidate.outcome_partitions > current.outcome_partitions
                                    || (candidate.outcome_partitions == current.outcome_partitions
                                        && candidate.actions < current.actions)
                            }) {
                                best = Some(candidate);
                            }
                        }
                        next.push(plan);
                    }
                }
            }
            if let Some(best) = best {
                return Ok(best);
            }
            frontier = next;
        }
        Err(SequentialExperimentError::NoDistinguishingPlan)
    }

    /// Retain only theories whose exact visible trace matches an executed plan.
    /// The old space is preserved if the observation refutes every theory.
    ///
    /// # Errors
    ///
    /// Rejects trace length disagreement and evidence that matches no theory.
    pub fn observe_plan(
        &mut self,
        actions: &[Hash32],
        visible_outcomes: &[Hash32],
    ) -> Result<(), SequentialExperimentError> {
        if actions.len() != visible_outcomes.len() {
            return Err(SequentialExperimentError::TraceLengthMismatch);
        }
        let survivors = self
            .theories
            .iter()
            .filter(|(_, theory)| {
                execute_trace(theory, actions).as_deref() == Some(visible_outcomes)
            })
            .map(|(id, theory)| (*id, theory.clone()))
            .collect::<BTreeMap<_, _>>();
        if survivors.is_empty() {
            return Err(SequentialExperimentError::AllTheoriesRefuted);
        }
        self.theories = survivors;
        Ok(())
    }

    #[must_use]
    pub fn sole_hypothesis_id(&self) -> Option<Hash32> {
        (self.theories.len() == 1)
            .then(|| self.theories.keys().next().copied())
            .flatten()
    }
}

fn execute_trace(theory: &SequentialTheoryHypothesis, actions: &[Hash32]) -> Option<Vec<Hash32>> {
    let mut state = theory.initial_state;
    actions
        .iter()
        .map(|action| {
            let transition = theory.transitions.get(&(state, *action))?;
            state = transition.next_state;
            Some(transition.visible_outcome)
        })
        .collect()
}
