//! Online version-space maintenance; permanent promotion belongs elsewhere.

use cairn_core::{CollectionProcedureProblem, Hash32, LearningProblem, ObservationId, TaskId};
use cairn_invent::{InventionError, InventionProblem};
use cairn_knowledge::KnowledgeBase;
use cairn_memory::{
    MechanismStore, MemoryAssistedInventor, MemorySolveOutcome, ProceduralMechanismStore,
};
use cairn_rkl::{
    ReferenceTheoryExecutor, RuntimeEnvironment, TheoryExecLimits, TheoryExecutionRequest, Type,
    TypeEnvironment, Value,
};
use cairn_skill::{Condition, Skill, SkillPlan, SkillPlanError, SkillPlanner};
use cairn_world::{
    ActionId, ExactPlanner, GoalObservation, GoalTheory, Plan, PlanLimits, StateSpace,
    TransitionObservation, TransitionTheory, WorldError, WorldLearner, WorldState,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct CognitionRuntime {
    memory: MechanismStore,
    procedures: ProceduralMechanismStore,
    knowledge: KnowledgeBase,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveLearningOutcome {
    pub transition_theory: TransitionTheory,
    pub goal_theory: GoalTheory,
    pub plan: Plan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionPrediction {
    pub module: Hash32,
    pub output: BTreeSet<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionSolveOutcome {
    pub predictions: Vec<CollectionPrediction>,
    pub proposals_attempted: u64,
    pub demonstration_replays: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionCompositionPrediction {
    pub modules: Vec<Hash32>,
    pub output: BTreeSet<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionCompositionOutcome {
    pub predictions: Vec<CollectionCompositionPrediction>,
    pub schedules_attempted: u64,
    pub demonstration_replays: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProceduralSolveError {
    EmptyDemonstrations,
    EmptyPrediction,
    EmptyBound,
    DuplicateInput,
    NoVerifiedProposal,
    UnknownProcedure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CognitionError {
    World(WorldError),
    Skill(SkillPlanError),
}

impl CognitionRuntime {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    #[must_use]
    pub const fn with_memory(memory: MechanismStore) -> Self {
        Self {
            memory,
            procedures: ProceduralMechanismStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    #[must_use]
    pub const fn with_memories(
        memory: MechanismStore,
        procedures: ProceduralMechanismStore,
    ) -> Self {
        Self {
            memory,
            procedures,
            knowledge: KnowledgeBase::new(),
        }
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

    #[must_use]
    pub const fn knowledge(&self) -> &KnowledgeBase {
        &self.knowledge
    }

    pub const fn knowledge_mut(&mut self) -> &mut KnowledgeBase {
        &mut self.knowledge
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

    /// Replay provisional or persistent whole-theory proposals against every
    /// current collection demonstration before producing bounded predictions.
    ///
    /// # Errors
    ///
    /// Rejects malformed collection evidence, empty bounds, or absence of a
    /// proposal that exactly replays all current demonstrations.
    pub fn solve_collection(
        &self,
        problem: &CollectionProcedureProblem,
    ) -> Result<CollectionSolveOutcome, ProceduralSolveError> {
        validate_collection_problem(problem)?;
        let mut proposals_attempted = 0_u64;
        let mut demonstration_replays = 0_u64;
        let mut predictions = Vec::new();
        let mut observed_outputs = BTreeSet::new();
        for module in self.procedures.reusable_modules() {
            proposals_attempted = proposals_attempted.saturating_add(1);
            let mut exact = true;
            for demonstration in &problem.demonstrations {
                demonstration_replays = demonstration_replays.saturating_add(1);
                let Some(output) = execute_collection(module.theory(), &demonstration.input) else {
                    exact = false;
                    break;
                };
                if output != demonstration.output {
                    exact = false;
                    break;
                }
            }
            if !exact {
                continue;
            }
            let Some(output) = execute_collection(module.theory(), &problem.prediction_input)
            else {
                continue;
            };
            if observed_outputs.insert(output.clone()) {
                predictions.push(CollectionPrediction {
                    module: module.id(),
                    output,
                });
            }
            if predictions.len() == problem.maximum_predictions {
                break;
            }
        }
        if predictions.is_empty() {
            return Err(ProceduralSolveError::NoVerifiedProposal);
        }
        Ok(CollectionSolveOutcome {
            predictions,
            proposals_attempted,
            demonstration_replays,
        })
    }

    /// Enumerate ordered pairs of independently stored procedures. No composite
    /// mechanism is supplied: every schedule is replayed end-to-end against
    /// the current demonstrations before it may predict.
    ///
    /// # Errors
    ///
    /// Rejects malformed evidence or the absence of an exactly replayed pair.
    pub fn solve_collection_pair(
        &self,
        problem: &CollectionProcedureProblem,
    ) -> Result<CollectionCompositionOutcome, ProceduralSolveError> {
        validate_collection_problem(problem)?;
        let modules = self.procedures.reusable_modules().collect::<Vec<_>>();
        let mut schedules_attempted = 0_u64;
        let mut demonstration_replays = 0_u64;
        let mut predictions = Vec::new();
        let mut observed_outputs = BTreeSet::new();
        for first in &modules {
            for second in &modules {
                if first.id() == second.id() {
                    continue;
                }
                schedules_attempted = schedules_attempted.saturating_add(1);
                let mut exact = true;
                for demonstration in &problem.demonstrations {
                    demonstration_replays = demonstration_replays.saturating_add(1);
                    let Some(intermediate) =
                        execute_collection(first.theory(), &demonstration.input)
                    else {
                        exact = false;
                        break;
                    };
                    let input = intermediate.iter().copied().collect::<Vec<_>>();
                    let Some(output) = execute_collection(second.theory(), &input) else {
                        exact = false;
                        break;
                    };
                    if output != demonstration.output {
                        exact = false;
                        break;
                    }
                }
                if !exact {
                    continue;
                }
                let Some(intermediate) =
                    execute_collection(first.theory(), &problem.prediction_input)
                else {
                    continue;
                };
                let input = intermediate.iter().copied().collect::<Vec<_>>();
                let Some(output) = execute_collection(second.theory(), &input) else {
                    continue;
                };
                if observed_outputs.insert(output.clone()) {
                    predictions.push(CollectionCompositionPrediction {
                        modules: vec![first.id(), second.id()],
                        output,
                    });
                }
                if predictions.len() == problem.maximum_predictions {
                    break;
                }
            }
        }
        if predictions.is_empty() {
            return Err(ProceduralSolveError::NoVerifiedProposal);
        }
        Ok(CollectionCompositionOutcome {
            predictions,
            schedules_attempted,
            demonstration_replays,
        })
    }

    /// Execute one checked procedural module by content identity. Callers
    /// remain responsible for current-task replay before trusting the result.
    ///
    /// # Errors
    ///
    /// Rejects an unknown module or bounded execution failure.
    pub fn apply_collection_module(
        &self,
        module: Hash32,
        input: &[i64],
    ) -> Result<BTreeSet<i64>, ProceduralSolveError> {
        let module = self
            .procedures
            .reusable_modules()
            .find(|candidate| candidate.id() == module)
            .ok_or(ProceduralSolveError::UnknownProcedure)?;
        execute_collection(module.theory(), input).ok_or(ProceduralSolveError::NoVerifiedProposal)
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

    /// Learn action effects and a goal from observations, then plan under the
    /// induced theories through the same adapter-neutral runtime.
    ///
    /// # Errors
    ///
    /// Returns exact world induction, goal inference, or planning failures.
    pub fn learn_and_plan_world(
        &self,
        space: &StateSpace,
        actions: &[ActionId],
        transitions: &[TransitionObservation],
        goals: &[GoalObservation],
        initial: &WorldState,
        limits: PlanLimits,
    ) -> Result<InteractiveLearningOutcome, CognitionError> {
        let transition_theory = WorldLearner
            .induce_transitions(space, actions, transitions)
            .map_err(CognitionError::World)?;
        let goal_theory = WorldLearner
            .induce_goal(space, goals)
            .map_err(CognitionError::World)?;
        let plan = ExactPlanner
            .plan(&transition_theory, &goal_theory, initial, limits)
            .map_err(CognitionError::World)?;
        Ok(InteractiveLearningOutcome {
            transition_theory,
            goal_theory,
            plan,
        })
    }

    /// Compose conditional learned skills using their initiation conditions
    /// and effects rather than replaying a recorded action sequence.
    ///
    /// # Errors
    ///
    /// Returns exact malformed-library, capacity, or no-plan failures.
    pub fn plan_with_skills(
        &self,
        initial: &BTreeSet<cairn_skill::Fluent>,
        goal: &Condition,
        skills: &[Skill],
        maximum_expansions: u64,
    ) -> Result<SkillPlan, CognitionError> {
        SkillPlanner
            .plan(initial, goal, skills, maximum_expansions)
            .map_err(CognitionError::Skill)
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

fn validate_collection_problem(
    problem: &CollectionProcedureProblem,
) -> Result<(), ProceduralSolveError> {
    if problem.demonstrations.is_empty() {
        return Err(ProceduralSolveError::EmptyDemonstrations);
    }
    if problem.prediction_input.is_empty() {
        return Err(ProceduralSolveError::EmptyPrediction);
    }
    if problem.maximum_cardinality == 0 || problem.maximum_predictions == 0 {
        return Err(ProceduralSolveError::EmptyBound);
    }
    let valid = |values: &[i64]| {
        !values.is_empty()
            && values.len() <= problem.maximum_cardinality as usize
            && values.iter().copied().collect::<BTreeSet<_>>().len() == values.len()
    };
    if !valid(&problem.prediction_input)
        || problem.demonstrations.iter().any(|demonstration| {
            !valid(&demonstration.input)
                || demonstration.output.is_empty()
                || demonstration.output.len() > problem.maximum_cardinality as usize
        })
    {
        return Err(ProceduralSolveError::DuplicateInput);
    }
    Ok(())
}

fn execute_collection(theory: &cairn_rkl::CheckedTheory, input: &[i64]) -> Option<BTreeSet<i64>> {
    let binder = theory.theory().inputs.first()?;
    if theory.theory().inputs.len() != 1 {
        return None;
    }
    let Type::List { element, maximum } = &binder.ty else {
        return None;
    };
    if !matches!(element.as_ref(), Type::Int(_)) || input.len() > *maximum as usize {
        return None;
    }
    let mut types = TypeEnvironment::new();
    types.insert(binder.name.clone(), binder.ty.clone());
    let mut values = RuntimeEnvironment::new();
    values.insert(
        binder.name.clone(),
        Value::Set {
            element_type: (**element).clone(),
            maximum: *maximum,
            values: input.iter().copied().map(Value::Int).collect(),
        },
    );
    let trace = ReferenceTheoryExecutor
        .execute(
            theory,
            &TheoryExecutionRequest {
                types,
                values,
                initial_facts: BTreeSet::new(),
                initial_state: BTreeMap::new(),
            },
            TheoryExecLimits {
                expression: cairn_rkl::ExecLimits::default(),
                maximum_bindings: u64::from(*maximum),
                maximum_rule_firings: u64::from(*maximum),
            },
        )
        .ok()?;
    trace
        .rendered
        .facts
        .into_iter()
        .map(|value| match value {
            Value::Int(value) => Some(value),
            _ => None,
        })
        .collect()
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
