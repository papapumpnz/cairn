//! Online version-space maintenance; permanent promotion belongs elsewhere.

use cairn_core::{Hash32, LearningProblem, ObservationId, TaskId};
use cairn_invent::{InventionError, InventionProblem};
use cairn_memory::{
    MechanismStore, MemoryAssistedInventor, MemorySolveOutcome, ProceduralMechanismStore,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct CognitionRuntime {
    memory: MechanismStore,
    procedures: ProceduralMechanismStore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmittedExperience {
    pub task: TaskId,
    pub observations: usize,
    pub demonstrations: usize,
    pub requested_predictions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExperienceError {
    EmptyObservations,
    EmptyDemonstrations,
    EmptyPredictions,
    EmptyAttemptLimit,
    InvalidObservation(ObservationId),
    DuplicateObservation(ObservationId),
    UnknownReference(ObservationId),
}

impl CognitionRuntime {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
        }
    }

    #[must_use]
    pub const fn with_memory(memory: MechanismStore) -> Self {
        Self {
            memory,
            procedures: ProceduralMechanismStore::new(),
        }
    }

    #[must_use]
    pub const fn with_memories(
        memory: MechanismStore,
        procedures: ProceduralMechanismStore,
    ) -> Self {
        Self { memory, procedures }
    }

    #[must_use]
    pub const fn memory(&self) -> &MechanismStore {
        &self.memory
    }

    pub const fn memory_mut(&mut self) -> &mut MechanismStore {
        &mut self.memory
    }

    #[must_use]
    pub const fn procedural_memory(&self) -> &ProceduralMechanismStore {
        &self.procedures
    }

    pub const fn procedural_memory_mut(&mut self) -> &mut ProceduralMechanismStore {
        &mut self.procedures
    }

    /// Learn through the same task-local and memory-proposal path for every
    /// frontend lowered to the generic invention contract.
    ///
    /// # Errors
    ///
    /// Returns bounded invention, checking, replay, or resource failures.
    pub fn learn(&self, problem: &InventionProblem) -> Result<MemorySolveOutcome, InventionError> {
        MemoryAssistedInventor.invent(problem, &self.memory)
    }

    /// Admit one modality-neutral experience envelope before any proposal
    /// backend can inspect its payload.
    ///
    /// # Errors
    ///
    /// Rejects empty work, invalid observation identities, duplicates, and
    /// demonstration or prediction references outside the envelope.
    pub fn admit_experience(
        &self,
        problem: &LearningProblem,
    ) -> Result<AdmittedExperience, ExperienceError> {
        if problem.observations.is_empty() {
            return Err(ExperienceError::EmptyObservations);
        }
        if problem.demonstrations.is_empty() {
            return Err(ExperienceError::EmptyDemonstrations);
        }
        if problem.requested_predictions.is_empty() {
            return Err(ExperienceError::EmptyPredictions);
        }
        if problem.attempt_limit == 0 {
            return Err(ExperienceError::EmptyAttemptLimit);
        }
        let mut observations = BTreeSet::new();
        for observation in &problem.observations {
            if observation.verify().is_err() {
                return Err(ExperienceError::InvalidObservation(observation.id));
            }
            if !observations.insert(observation.id) {
                return Err(ExperienceError::DuplicateObservation(observation.id));
            }
        }
        for demonstration in &problem.demonstrations {
            check_references(&demonstration.context, &observations)?;
            check_references(&demonstration.outcome, &observations)?;
            if let Some(intervention) = &demonstration.intervention {
                check_references(&intervention.arguments, &observations)?;
            }
        }
        for request in &problem.requested_predictions {
            check_references(&request.context, &observations)?;
            if let Some(intervention) = &request.intervention {
                check_references(&intervention.arguments, &observations)?;
            }
        }
        Ok(AdmittedExperience {
            task: problem.id,
            observations: problem.observations.len(),
            demonstrations: problem.demonstrations.len(),
            requested_predictions: problem.requested_predictions.len(),
        })
    }
}

fn check_references(
    references: &[ObservationId],
    observations: &BTreeSet<ObservationId>,
) -> Result<(), ExperienceError> {
    for reference in references {
        if !observations.contains(reference) {
            return Err(ExperienceError::UnknownReference(*reference));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TheoryHypothesis {
    pub id: Hash32,
    pub predicted_outcomes: BTreeMap<Hash32, Hash32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscriminatingTest {
    pub intervention: Hash32,
    pub outcome_partitions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    EmptyVersionSpace,
    DuplicateTheory,
    UnknownIntervention,
    AllTheoriesRefuted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionSpace {
    theories: BTreeMap<Hash32, TheoryHypothesis>,
}

impl VersionSpace {
    /// Construct a nonempty deterministic version space.
    ///
    /// # Errors
    ///
    /// Rejects empty or duplicate hypothesis sets.
    pub fn new(theories: Vec<TheoryHypothesis>) -> Result<Self, RuntimeError> {
        if theories.is_empty() {
            return Err(RuntimeError::EmptyVersionSpace);
        }
        let count = theories.len();
        let theories = theories
            .into_iter()
            .map(|theory| (theory.id, theory))
            .collect::<BTreeMap<_, _>>();
        if theories.len() != count {
            return Err(RuntimeError::DuplicateTheory);
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

    /// Select the intervention producing the most distinct predicted outcomes.
    #[must_use]
    pub fn next_discriminating_test(&self) -> Option<DiscriminatingTest> {
        self.discriminating_tests().into_iter().max_by_key(|test| {
            (
                test.outcome_partitions,
                std::cmp::Reverse(test.intervention),
            )
        })
    }

    /// Expose all outcome partition counts to the separate curriculum runtime.
    #[must_use]
    pub fn discriminating_tests(&self) -> Vec<DiscriminatingTest> {
        let interventions = self
            .theories
            .values()
            .flat_map(|theory| theory.predicted_outcomes.keys().copied())
            .collect::<BTreeSet<_>>();
        interventions
            .into_iter()
            .map(|intervention| {
                let outcomes = self
                    .theories
                    .values()
                    .filter_map(|theory| theory.predicted_outcomes.get(&intervention))
                    .copied()
                    .collect::<BTreeSet<_>>();
                DiscriminatingTest {
                    intervention,
                    outcome_partitions: outcomes.len(),
                }
            })
            .collect()
    }

    /// Return the resolved theory identity only when one hypothesis remains.
    #[must_use]
    pub fn sole_hypothesis_id(&self) -> Option<Hash32> {
        if self.theories.len() == 1 {
            self.theories.keys().next().copied()
        } else {
            None
        }
    }

    /// Eliminate hypotheses contradicted by an observed intervention outcome.
    ///
    /// # Errors
    ///
    /// Rejects unknown evidence and preserves the old space if all theories fail.
    pub fn observe(&mut self, intervention: Hash32, outcome: Hash32) -> Result<(), RuntimeError> {
        if !self
            .theories
            .values()
            .any(|theory| theory.predicted_outcomes.contains_key(&intervention))
        {
            return Err(RuntimeError::UnknownIntervention);
        }
        let survivors = self
            .theories
            .iter()
            .filter(|(_, theory)| theory.predicted_outcomes.get(&intervention) == Some(&outcome))
            .map(|(id, theory)| (*id, theory.clone()))
            .collect::<BTreeMap<_, _>>();
        if survivors.is_empty() {
            return Err(RuntimeError::AllTheoriesRefuted);
        }
        self.theories = survivors;
        Ok(())
    }
}
