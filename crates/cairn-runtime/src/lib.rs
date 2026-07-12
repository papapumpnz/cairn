//! Online version-space maintenance; permanent promotion belongs elsewhere.

mod sequential_experiment;

pub use sequential_experiment::*;

use cairn_consolidator::{
    ConsolidationError, Consolidator, ConstructedInternalComposition, InternalCompositionPlan,
    InternalConstructionLimits, InternalConstructionPolicy, LoweredObservationFrontier,
    ObservationFrontierLimits, ProofFrontier, ProofGraph, RelationalConstructionPolicy,
};
use cairn_core::{
    CollectionProcedureProblem, Hash32, LearningProblem, ObservationId, RelationalObservation,
    TaskId,
};
use cairn_invent::{InventionError, InventionProblem};
use cairn_knowledge::KnowledgeBase;
use cairn_memory::{
    InternalMechanismStore, LatentMechanismStore, MechanismStore, MemoryAssistedInventor,
    MemorySolveOutcome, ModuleStatus, ProceduralMechanismModule, ProceduralMechanismStore,
};
use cairn_rkl::{
    ReferenceTheoryExecutor, RuntimeEnvironment, TheoryExecLimits, TheoryExecutionRequest, Type,
    TypeEnvironment, Value,
};
use cairn_skill::{Condition, Skill, SkillPlan, SkillPlanError, SkillPlanner};
use cairn_world::{
    ActionId, ActiveLatentInductionReport, ActiveLatentLimits, ActiveLatentWorldLearner,
    BeliefState, ExactPlanner, GoalObservation, GoalTheory, LatentEpisode, LatentError,
    LatentExperiment, LatentPlan, LatentPlanner, LatentSchemaBinding, LatentSchemaCompiler,
    LatentSchemaError, LatentSchemaLimits, LatentTransitionTheory, Plan, PlanLimits, StateSpace,
    TransitionObservation, TransitionTheory, WorldError, WorldLearner, WorldState,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct CognitionRuntime {
    memory: MechanismStore,
    procedures: ProceduralMechanismStore,
    internal_mechanisms: InternalMechanismStore,
    latent_mechanisms: LatentMechanismStore,
    knowledge: KnowledgeBase,
}

fn procedure_schedules(
    module_count: usize,
    minimum_depth: usize,
    maximum_depth: usize,
    maximum_schedules: u64,
) -> Vec<Vec<usize>> {
    fn extend(
        module_count: usize,
        target: usize,
        maximum: u64,
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
    ) {
        if u64::try_from(output.len()).unwrap_or(u64::MAX) >= maximum {
            return;
        }
        if current.len() == target {
            output.push(current.clone());
            return;
        }
        for index in 0..module_count {
            if current.contains(&index) {
                continue;
            }
            current.push(index);
            extend(module_count, target, maximum, current, output);
            current.pop();
        }
    }
    let mut output = Vec::new();
    for depth in minimum_depth..=maximum_depth {
        extend(
            module_count,
            depth,
            maximum_schedules,
            &mut Vec::new(),
            &mut output,
        );
    }
    output
}

fn execute_schedule(
    modules: &[&ProceduralMechanismModule],
    schedule: &[usize],
    input: &[i64],
) -> Option<BTreeSet<i64>> {
    let mut values = input.iter().copied().collect::<BTreeSet<_>>();
    for index in schedule {
        values = execute_collection(
            modules.get(*index)?.theory(),
            &values.iter().copied().collect::<Vec<_>>(),
        )?;
    }
    Some(values)
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
pub struct LatentPlanningOutcome {
    pub goal_theory: GoalTheory,
    pub plan: LatentPlan,
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
pub struct CompositionLimits {
    pub minimum_depth: usize,
    pub maximum_depth: usize,
    pub maximum_schedules: u64,
}

impl Default for CompositionLimits {
    fn default() -> Self {
        Self {
            minimum_depth: 2,
            maximum_depth: 4,
            maximum_schedules: 10_000,
        }
    }
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
    LatentWorld(LatentError),
    Skill(SkillPlanError),
    InternalComposition(InternalCompositionRuntimeError),
    LatentMemory(LatentMemoryRuntimeError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatentMemoryRuntimeError {
    Schema(LatentSchemaError),
    NoVerifiedProposal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentMemoryOutcome {
    pub bindings: Vec<LatentSchemaBinding>,
    pub proposals: Vec<LatentMemoryProposal>,
    pub persistent_schemas_attempted: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatentMemoryProposal {
    pub schema: Hash32,
    pub binding: LatentSchemaBinding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InternalCompositionRuntimeError {
    UnavailableMechanism(Hash32),
    Proposal(ConsolidationError),
}

impl CognitionRuntime {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    #[must_use]
    pub const fn with_memory(memory: MechanismStore) -> Self {
        Self {
            memory,
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
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
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    /// Construct a runtime with an already proof-gated latent mechanism
    /// library. All other memories begin empty.
    #[must_use]
    pub const fn with_latent_mechanism_memory(latent_mechanisms: LatentMechanismStore) -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms,
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
    pub const fn internal_mechanism_memory(&self) -> &InternalMechanismStore {
        &self.internal_mechanisms
    }

    pub const fn internal_mechanism_memory_mut(&mut self) -> &mut InternalMechanismStore {
        &mut self.internal_mechanisms
    }

    #[must_use]
    pub const fn latent_mechanism_memory(&self) -> &LatentMechanismStore {
        &self.latent_mechanisms
    }

    pub const fn latent_mechanism_memory_mut(&mut self) -> &mut LatentMechanismStore {
        &mut self.latent_mechanisms
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
        self.solve_collection_composed(
            problem,
            CompositionLimits {
                minimum_depth: 2,
                maximum_depth: 2,
                maximum_schedules: 10_000,
            },
        )
    }

    /// Enumerate bounded schedules of independently stored procedures. Every
    /// complete schedule is replayed on every current demonstration.
    ///
    /// # Errors
    ///
    /// Rejects invalid bounds, malformed evidence, exhausted schedule capacity,
    /// or the absence of an exactly replayed composition.
    pub fn solve_collection_composed(
        &self,
        problem: &CollectionProcedureProblem,
        limits: CompositionLimits,
    ) -> Result<CollectionCompositionOutcome, ProceduralSolveError> {
        validate_collection_problem(problem)?;
        if limits.minimum_depth < 2
            || limits.maximum_depth < limits.minimum_depth
            || limits.maximum_schedules == 0
        {
            return Err(ProceduralSolveError::EmptyBound);
        }
        let modules = self.procedures.reusable_modules().collect::<Vec<_>>();
        let mut schedules_attempted = 0_u64;
        let mut demonstration_replays = 0_u64;
        let mut predictions = Vec::new();
        let mut observed_outputs = BTreeSet::new();
        let schedules = procedure_schedules(
            modules.len(),
            limits.minimum_depth,
            limits.maximum_depth.min(modules.len()),
            limits.maximum_schedules,
        );
        for schedule in schedules {
            schedules_attempted = schedules_attempted.saturating_add(1);
            let mut exact = true;
            for demonstration in &problem.demonstrations {
                demonstration_replays = demonstration_replays.saturating_add(1);
                if execute_schedule(&modules, &schedule, &demonstration.input)
                    != Some(demonstration.output.clone())
                {
                    exact = false;
                    break;
                }
            }
            if !exact {
                continue;
            }
            let Some(output) = execute_schedule(&modules, &schedule, &problem.prediction_input)
            else {
                continue;
            };
            if observed_outputs.insert(output.clone()) {
                predictions.push(CollectionCompositionPrediction {
                    modules: schedule.iter().map(|index| modules[*index].id()).collect(),
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

    /// Close the interactive latent-learning loop by selecting and validating
    /// distinguishing continuations through an environment-neutral experiment
    /// boundary.
    ///
    /// # Errors
    ///
    /// Returns exact latent induction, intervention, observation-integrity, or
    /// resource failures.
    pub fn learn_latent_world_actively<E: LatentExperiment>(
        &self,
        space: &StateSpace,
        actions: &[ActionId],
        episodes: &[LatentEpisode],
        limits: ActiveLatentLimits,
        experiment: &mut E,
    ) -> Result<ActiveLatentInductionReport, CognitionError> {
        ActiveLatentWorldLearner
            .induce(space, actions, episodes, limits, experiment)
            .map_err(CognitionError::LatentWorld)
    }

    /// Rebind persistent source-independent world mechanisms to current
    /// actions and retain only proposals that exactly replay current episodes.
    /// The returned theories remain proposals until independent verification.
    ///
    /// # Errors
    ///
    /// Returns bounded schema failures when no persistent mechanism can be
    /// rebound and replayed under the declared limits.
    pub fn propose_latent_world_from_memory(
        &self,
        space: &StateSpace,
        actions: &[ActionId],
        episodes: &[LatentEpisode],
        limits: LatentSchemaLimits,
    ) -> Result<LatentMemoryOutcome, CognitionError> {
        let mut persistent_schemas_attempted = 0_u64;
        let mut bindings = Vec::new();
        let mut proposals = Vec::new();
        for schema in self.latent_mechanisms.persistent_schemas() {
            persistent_schemas_attempted = persistent_schemas_attempted.saturating_add(1);
            match LatentSchemaCompiler.bind(schema, space, actions, episodes, limits) {
                Ok(mut rebound) => {
                    proposals.extend(rebound.iter().cloned().map(|binding| LatentMemoryProposal {
                        schema: schema.semantic_hash(),
                        binding,
                    }));
                    bindings.append(&mut rebound);
                }
                Err(
                    LatentSchemaError::NoBinding
                    | LatentSchemaError::DimensionMismatch
                    | LatentSchemaError::ActionCountMismatch,
                ) => {}
                Err(error) => {
                    return Err(CognitionError::LatentMemory(
                        LatentMemoryRuntimeError::Schema(error),
                    ));
                }
            }
        }
        if bindings.is_empty() {
            return Err(CognitionError::LatentMemory(
                LatentMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(LatentMemoryOutcome {
            bindings,
            proposals,
            persistent_schemas_attempted,
        })
    }

    /// Infer a visible goal and plan over a previously learned joint
    /// visible/latent transition theory.
    ///
    /// # Errors
    ///
    /// Returns exact goal induction or latent planning failures.
    pub fn infer_goal_and_plan_latent_world(
        &self,
        space: &StateSpace,
        theory: &LatentTransitionTheory,
        goals: &[GoalObservation],
        initial: &BeliefState,
        limits: PlanLimits,
    ) -> Result<LatentPlanningOutcome, CognitionError> {
        let goal_theory = WorldLearner
            .induce_goal(space, goals)
            .map_err(CognitionError::World)?;
        let plan = LatentPlanner
            .plan(theory, &goal_theory, initial, limits)
            .map_err(CognitionError::LatentWorld)?;
        Ok(LatentPlanningOutcome { goal_theory, plan })
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

    /// Propose a bounded joint proof using distinct persistent internal
    /// mechanisms. This runtime path cannot verify its own proposal.
    ///
    /// # Errors
    ///
    /// Rejects missing/non-persistent mechanisms and bounded composition
    /// failures from the reference proposer.
    pub fn plan_internal_composition(
        &self,
        mechanism_ids: &[Hash32],
        host: &ProofGraph,
        maximum_windows_per_mechanism: usize,
        maximum_combinations: u64,
    ) -> Result<InternalCompositionPlan, CognitionError> {
        let mechanisms = mechanism_ids
            .iter()
            .map(|id| {
                let module = self
                    .internal_mechanisms
                    .get(*id)
                    .filter(|module| module.status() == ModuleStatus::Persistent)
                    .ok_or(InternalCompositionRuntimeError::UnavailableMechanism(*id))?;
                Ok(module.mechanism().clone())
            })
            .collect::<Result<Vec<_>, InternalCompositionRuntimeError>>()
            .map_err(CognitionError::InternalComposition)?;
        Consolidator
            .compose_compiled_internal_mechanisms(
                &mechanisms,
                host,
                maximum_windows_per_mechanism,
                maximum_combinations,
            )
            .map_err(|error| {
                CognitionError::InternalComposition(InternalCompositionRuntimeError::Proposal(
                    error,
                ))
            })
    }

    /// Construct bounded proof candidates from evidence-only nodes by applying
    /// persistent learned mechanisms. Expected proof structure is not an input.
    ///
    /// # Errors
    ///
    /// Rejects missing/non-persistent mechanisms and bounded construction
    /// failures from the reference proposer.
    pub fn construct_internal_compositions(
        &self,
        mechanism_ids: &[Hash32],
        frontier: &ProofFrontier,
        limits: InternalConstructionLimits,
    ) -> Result<Vec<ConstructedInternalComposition>, CognitionError> {
        let mechanisms = mechanism_ids
            .iter()
            .map(|id| {
                let module = self
                    .internal_mechanisms
                    .get(*id)
                    .filter(|module| module.status() == ModuleStatus::Persistent)
                    .ok_or(InternalCompositionRuntimeError::UnavailableMechanism(*id))?;
                Ok(module.mechanism().clone())
            })
            .collect::<Result<Vec<_>, InternalCompositionRuntimeError>>()
            .map_err(CognitionError::InternalComposition)?;
        Consolidator
            .construct_compiled_internal_compositions(&mechanisms, frontier, limits)
            .map_err(|error| {
                CognitionError::InternalComposition(InternalCompositionRuntimeError::Proposal(
                    error,
                ))
            })
    }

    /// Deterministically lower modality-neutral relational evidence into an
    /// untrusted proof frontier. No memory or expected output is consulted.
    ///
    /// # Errors
    ///
    /// Rejects invalid observation evidence, capacity excess, or canonical
    /// lowering failure.
    pub fn lower_relational_observation(
        &self,
        observation: &RelationalObservation,
        limits: ObservationFrontierLimits,
    ) -> Result<LoweredObservationFrontier, CognitionError> {
        Consolidator
            .lower_relational_observation(observation, limits)
            .map_err(|error| {
                CognitionError::InternalComposition(InternalCompositionRuntimeError::Proposal(
                    error,
                ))
            })
    }

    /// Apply a separately validated task-local typed correspondence policy to
    /// a changed evidence frontier and construct one proof candidate.
    ///
    /// # Errors
    ///
    /// Rejects missing/non-persistent mechanisms and policy/evidence mismatch.
    pub fn apply_internal_construction_policy(
        &self,
        mechanism_ids: &[Hash32],
        frontier: &ProofFrontier,
        policy: &InternalConstructionPolicy,
    ) -> Result<ConstructedInternalComposition, CognitionError> {
        let mechanisms = mechanism_ids
            .iter()
            .map(|id| {
                let module = self
                    .internal_mechanisms
                    .get(*id)
                    .filter(|module| module.status() == ModuleStatus::Persistent)
                    .ok_or(InternalCompositionRuntimeError::UnavailableMechanism(*id))?;
                Ok(module.mechanism().clone())
            })
            .collect::<Result<Vec<_>, InternalCompositionRuntimeError>>()
            .map_err(CognitionError::InternalComposition)?;
        Consolidator
            .apply_internal_construction_policy(&mechanisms, frontier, policy)
            .map_err(|error| {
                CognitionError::InternalComposition(InternalCompositionRuntimeError::Proposal(
                    error,
                ))
            })
    }

    /// Apply learned relational correspondence to a changed authenticated
    /// observation frontier using only persistent mechanisms.
    ///
    /// # Errors
    ///
    /// Rejects missing/non-persistent mechanisms, invalid observation lowering,
    /// or missing/ambiguous structural correspondence.
    pub fn apply_relational_construction_policy(
        &self,
        mechanism_ids: &[Hash32],
        observation: &RelationalObservation,
        lowering: &LoweredObservationFrontier,
        policy: &RelationalConstructionPolicy,
        limits: ObservationFrontierLimits,
    ) -> Result<ConstructedInternalComposition, CognitionError> {
        let mechanisms = mechanism_ids
            .iter()
            .map(|id| {
                let module = self
                    .internal_mechanisms
                    .get(*id)
                    .filter(|module| module.status() == ModuleStatus::Persistent)
                    .ok_or(InternalCompositionRuntimeError::UnavailableMechanism(*id))?;
                Ok(module.mechanism().clone())
            })
            .collect::<Result<Vec<_>, InternalCompositionRuntimeError>>()
            .map_err(CognitionError::InternalComposition)?;
        Consolidator
            .apply_relational_construction_policy(
                &mechanisms,
                observation,
                lowering,
                policy,
                limits,
            )
            .map_err(|error| {
                CognitionError::InternalComposition(InternalCompositionRuntimeError::Proposal(
                    error,
                ))
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
