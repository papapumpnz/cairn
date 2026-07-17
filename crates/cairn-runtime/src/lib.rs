//! Online version-space maintenance; permanent promotion belongs elsewhere.

mod sequential_experiment;

pub use sequential_experiment::*;

use cairn_consolidator::{
    ConsolidationError, Consolidator, ConstructedInternalComposition, InternalCompositionPlan,
    InternalConstructionLimits, InternalConstructionPolicy, LoweredObservationFrontier,
    ObservationFrontierLimits, ProofFrontier, ProofGraph, RelationalConstructionPolicy,
};
use cairn_core::{
    CollectionProcedureProblem, Hash32, LearningProblem, ObservationId, RelationRole,
    RelationalObservation, TaskId,
};
use cairn_invent::{InventionError, InventionProblem};
use cairn_knowledge::KnowledgeBase;
use cairn_memory::{
    ConditionalConstructionStore, ConstructionProgramStore, EntityCorrespondenceStore,
    ExpansionStore, GroupMergeStore, GroupingMergeStore, InternalMechanismStore,
    LatentMechanismStore, LayerMechanismStore, MechanismStore, MemoryAssistedInventor,
    MemorySolveOutcome, ModuleStatus, ObservationPredicateStore, PredicateGroupingStore,
    ProceduralMechanismModule, ProceduralMechanismStore, RawRelationStore, RelationExpressionStore,
    ScalarExpressionStore,
};
use cairn_rkl::{
    ExecLimits, ReferenceTheoryExecutor, RuntimeEnvironment, TheoryExecLimits,
    TheoryExecutionRequest, Type, TypeEnvironment, Value,
};
use cairn_skill::{Condition, Skill, SkillPlan, SkillPlanError, SkillPlanner};
use cairn_theory::{
    AttributedGroupingEntity, CollectionInventionError, CollectionMechanismInventor,
    ConditionalConstructionError, ConditionalConstructionSynthesizer, EntityConstruction,
    EntityConstructionProgramError, EntityConstructionProgramSynthesizer,
    EntityCorrespondenceError, EntityCorrespondenceLearner, EntityCorrespondenceLimits,
    EntityGeometryScene, EntityObservationError, EntityObservationInventor,
    EntityObservationLimits, GroupMergeError, GroupMergeInventor, GroupingRelationInventor,
    GroupingSelectionError, LayerMechanismError, LayerMechanismLearner, LayerRender, LayerScene,
    ObservationPredicateError, ObservationPredicateInventor, ObservationPredicateLimits,
    PredicateGroupingError, PredicateGroupingInventor, PredicateGroupingLimits,
    PrimitivePairEntity, RawEntityObservationTrace, RawGroupingError, RawGroupingEvidence,
    RawGroupingLimits, RawRelationGroupingInventor, RawRelationObservationInventor,
    RawRelationObservationTrace, RelationAttributeBinding, RelationExpressionBinding,
    RelationExpressionError, RelationExpressionInventor, RelationExpressionLimits,
    RelationObservationError, RelationObservationLimits, RelationalGeometryScene,
    ScalarExpressionError, ScalarExpressionInventor,
};
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
    layer_mechanisms: LayerMechanismStore,
    entity_correspondences: EntityCorrespondenceStore,
    construction_programs: ConstructionProgramStore,
    conditional_constructions: ConditionalConstructionStore,
    expansions: ExpansionStore,
    group_merges: GroupMergeStore,
    grouping_merges: GroupingMergeStore,
    predicate_groupings: PredicateGroupingStore,
    observation_predicates: ObservationPredicateStore,
    raw_relations: RawRelationStore,
    relation_expressions: RelationExpressionStore,
    scalar_expressions: ScalarExpressionStore,
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
    LayerMemory(LayerMemoryRuntimeError),
    EntityCorrespondenceMemory(EntityCorrespondenceMemoryRuntimeError),
    ConstructionProgramMemory(ConstructionProgramMemoryRuntimeError),
    ConditionalConstructionMemory(ConditionalConstructionMemoryRuntimeError),
    ExpansionMemory(ExpansionMemoryRuntimeError),
    GroupMergeMemory(GroupMergeMemoryRuntimeError),
    GroupingMergeMemory(GroupingMergeMemoryRuntimeError),
    PredicateGroupingMemory(PredicateGroupingMemoryRuntimeError),
    ObservationPredicateMemory(ObservationPredicateMemoryRuntimeError),
    EntityObservationMemory(EntityObservationMemoryRuntimeError),
    RawRelationObservationMemory(RawRelationObservationMemoryRuntimeError),
    RelationExpressionMemory(RelationExpressionMemoryRuntimeError),
    ScalarExpressionMemory(ScalarExpressionMemoryRuntimeError),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayerMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerMemoryProposal {
    pub mechanism: Hash32,
    pub render: LayerRender,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerMemoryOutcome {
    pub proposals: Vec<LayerMemoryProposal>,
    pub persistent_mechanisms_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityCorrespondenceMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityCorrespondenceMemoryProposal {
    pub mechanism: Hash32,
    pub construction: EntityConstruction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityCorrespondenceMemoryOutcome {
    pub proposals: Vec<EntityCorrespondenceMemoryProposal>,
    pub persistent_mechanisms_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructionProgramMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructionProgramMemoryProposal {
    pub program: Hash32,
    pub construction: EntityConstruction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructionProgramMemoryOutcome {
    pub proposals: Vec<ConstructionProgramMemoryProposal>,
    pub persistent_programs_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConditionalConstructionMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalConstructionMemoryProposal {
    pub program: Hash32,
    pub construction: EntityConstruction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalConstructionMemoryOutcome {
    pub proposals: Vec<ConditionalConstructionMemoryProposal>,
    pub persistent_programs_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpansionMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpansionMemoryOutcome {
    pub proposals: Vec<BTreeSet<i64>>,
    pub persistent_mechanisms_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupMergeMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupMergeMemoryOutcome {
    pub proposals: Vec<BTreeSet<i64>>,
    pub persistent_mechanisms_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupingMergeMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupingMergeMemoryOutcome {
    pub proposals: Vec<BTreeSet<i64>>,
    pub persistent_mechanisms_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateGroupingMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateGroupingMemoryOutcome {
    pub proposals: Vec<BTreeSet<i64>>,
    pub persistent_mechanisms_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationPredicateMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
    Execution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationPredicateMemoryOutcome {
    pub proposals: Vec<BTreeSet<i64>>,
    pub persistent_mechanisms_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityObservationMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityObservationMemoryProposal {
    pub mechanism: Hash32,
    pub relation: RelationRole,
    pub output: BTreeSet<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityObservationMemoryOutcome {
    pub proposals: Vec<EntityObservationMemoryProposal>,
    pub persistent_mechanisms_considered: u64,
    pub entity_bindings_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawRelationObservationMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
    InvalidEvidence,
}

fn map_raw_relation_memory_error(error: &RelationObservationError) -> CognitionError {
    let terminal = match error {
        RelationObservationError::Capacity
        | RelationObservationError::Entity(EntityObservationError::Capacity) => {
            RawRelationObservationMemoryRuntimeError::Capacity
        }
        _ => RawRelationObservationMemoryRuntimeError::InvalidEvidence,
    };
    CognitionError::RawRelationObservationMemory(terminal)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawRelationObservationMemoryProposal {
    pub mechanism: Hash32,
    pub binding: RelationAttributeBinding,
    pub output: BTreeSet<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawRelationObservationMemoryOutcome {
    pub proposals: Vec<RawRelationObservationMemoryProposal>,
    pub persistent_mechanisms_considered: u64,
    pub relation_bindings_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationExpressionMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
    InvalidEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationExpressionMemoryProposal {
    pub mechanism: Hash32,
    pub binding: RelationExpressionBinding,
    pub output: BTreeSet<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationExpressionMemoryOutcome {
    pub proposals: Vec<RelationExpressionMemoryProposal>,
    pub persistent_mechanisms_considered: u64,
    pub relation_bindings_attempted: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarExpressionMemoryRuntimeError {
    NoVerifiedProposal,
    Capacity,
    InvalidEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarExpressionMemoryProposal {
    pub mechanism: Hash32,
    pub binding: RelationExpressionBinding,
    pub output: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarExpressionMemoryOutcome {
    pub proposals: Vec<ScalarExpressionMemoryProposal>,
    pub persistent_mechanisms_considered: u64,
    pub bindings_attempted: u64,
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
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions: ExpansionStore::new(),
            group_merges: GroupMergeStore::new(),
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
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
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions: ExpansionStore::new(),
            group_merges: GroupMergeStore::new(),
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
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
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions: ExpansionStore::new(),
            group_merges: GroupMergeStore::new(),
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
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
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions: ExpansionStore::new(),
            group_merges: GroupMergeStore::new(),
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    /// Construct a runtime with proof-gated graphics layer mechanisms and all
    /// other memories empty.
    #[must_use]
    pub const fn with_layer_mechanism_memory(layer_mechanisms: LayerMechanismStore) -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
            layer_mechanisms,
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions: ExpansionStore::new(),
            group_merges: GroupMergeStore::new(),
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    /// Construct a runtime with proof-gated collection correspondence memory.
    #[must_use]
    pub const fn with_entity_correspondence_memory(
        entity_correspondences: EntityCorrespondenceStore,
    ) -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences,
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions: ExpansionStore::new(),
            group_merges: GroupMergeStore::new(),
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    /// Construct a runtime with proof-gated synthesized program memory.
    #[must_use]
    pub const fn with_construction_program_memory(
        construction_programs: ConstructionProgramStore,
    ) -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs,
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions: ExpansionStore::new(),
            group_merges: GroupMergeStore::new(),
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    /// Construct a runtime with proof-gated conditional program memory.
    #[must_use]
    pub const fn with_conditional_construction_memory(
        conditional_constructions: ConditionalConstructionStore,
    ) -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions,
            expansions: ExpansionStore::new(),
            group_merges: GroupMergeStore::new(),
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    /// Construct a runtime with proof-gated one-to-many expansion memory.
    #[must_use]
    pub const fn with_expansion_memory(expansions: ExpansionStore) -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions,
            group_merges: GroupMergeStore::new(),
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    /// Construct a runtime with proof-gated many-to-one merge memory.
    #[must_use]
    pub const fn with_group_merge_memory(group_merges: GroupMergeStore) -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions: ExpansionStore::new(),
            group_merges,
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    /// Construct a runtime with a proof-gated relation-bound merge library.
    #[must_use]
    pub const fn with_grouping_merge_memory(grouping_merges: GroupingMergeStore) -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions: ExpansionStore::new(),
            group_merges: GroupMergeStore::new(),
            grouping_merges,
            predicate_groupings: PredicateGroupingStore::new(),
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    /// Construct a runtime with proof-gated invented predicate/merge memory.
    #[must_use]
    pub const fn with_predicate_grouping_memory(
        predicate_groupings: PredicateGroupingStore,
    ) -> Self {
        Self {
            memory: MechanismStore::new(),
            procedures: ProceduralMechanismStore::new(),
            internal_mechanisms: InternalMechanismStore::new(),
            latent_mechanisms: LatentMechanismStore::new(),
            layer_mechanisms: LayerMechanismStore::new(),
            entity_correspondences: EntityCorrespondenceStore::new(),
            construction_programs: ConstructionProgramStore::new(),
            conditional_constructions: ConditionalConstructionStore::new(),
            expansions: ExpansionStore::new(),
            group_merges: GroupMergeStore::new(),
            grouping_merges: GroupingMergeStore::new(),
            predicate_groupings,
            observation_predicates: ObservationPredicateStore::new(),
            raw_relations: RawRelationStore::new(),
            relation_expressions: RelationExpressionStore::new(),
            scalar_expressions: ScalarExpressionStore::new(),
            knowledge: KnowledgeBase::new(),
        }
    }

    /// Construct a runtime with proof-gated synthesized observation-feature
    /// memory. The mechanisms remain proposal sources and require current-task
    /// verification before acceptance.
    #[must_use]
    pub fn with_observation_predicate_memory(
        observation_predicates: ObservationPredicateStore,
    ) -> Self {
        Self {
            observation_predicates,
            ..Self::new()
        }
    }

    /// Construct a runtime with proof-gated complete primitive-relation
    /// mechanisms. Modules remain proposal-only.
    #[must_use]
    pub fn with_raw_relation_memory(raw_relations: RawRelationStore) -> Self {
        Self {
            raw_relations,
            ..Self::new()
        }
    }

    /// Construct a runtime with proof-gated complete relation-expression
    /// mechanisms. Modules remain proposal-only.
    #[must_use]
    pub fn with_relation_expression_memory(relation_expressions: RelationExpressionStore) -> Self {
        Self {
            relation_expressions,
            ..Self::new()
        }
    }

    /// Construct a runtime with proof-gated scalar-expression mechanisms.
    /// Modules remain proposal-only and must bind to current input channels.
    #[must_use]
    pub fn with_scalar_expression_memory(scalar_expressions: ScalarExpressionStore) -> Self {
        Self {
            scalar_expressions,
            ..Self::new()
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
    pub const fn layer_mechanism_memory(&self) -> &LayerMechanismStore {
        &self.layer_mechanisms
    }

    pub const fn layer_mechanism_memory_mut(&mut self) -> &mut LayerMechanismStore {
        &mut self.layer_mechanisms
    }

    #[must_use]
    pub const fn entity_correspondence_memory(&self) -> &EntityCorrespondenceStore {
        &self.entity_correspondences
    }

    pub const fn entity_correspondence_memory_mut(&mut self) -> &mut EntityCorrespondenceStore {
        &mut self.entity_correspondences
    }

    #[must_use]
    pub const fn construction_program_memory(&self) -> &ConstructionProgramStore {
        &self.construction_programs
    }

    pub const fn construction_program_memory_mut(&mut self) -> &mut ConstructionProgramStore {
        &mut self.construction_programs
    }

    #[must_use]
    pub const fn conditional_construction_memory(&self) -> &ConditionalConstructionStore {
        &self.conditional_constructions
    }

    pub const fn conditional_construction_memory_mut(
        &mut self,
    ) -> &mut ConditionalConstructionStore {
        &mut self.conditional_constructions
    }

    #[must_use]
    pub const fn expansion_memory(&self) -> &ExpansionStore {
        &self.expansions
    }

    pub const fn expansion_memory_mut(&mut self) -> &mut ExpansionStore {
        &mut self.expansions
    }

    #[must_use]
    pub const fn group_merge_memory(&self) -> &GroupMergeStore {
        &self.group_merges
    }
    pub const fn group_merge_memory_mut(&mut self) -> &mut GroupMergeStore {
        &mut self.group_merges
    }

    #[must_use]
    pub const fn grouping_merge_memory(&self) -> &GroupingMergeStore {
        &self.grouping_merges
    }
    pub const fn grouping_merge_memory_mut(&mut self) -> &mut GroupingMergeStore {
        &mut self.grouping_merges
    }

    #[must_use]
    pub const fn predicate_grouping_memory(&self) -> &PredicateGroupingStore {
        &self.predicate_groupings
    }
    pub const fn predicate_grouping_memory_mut(&mut self) -> &mut PredicateGroupingStore {
        &mut self.predicate_groupings
    }

    #[must_use]
    pub const fn observation_predicate_memory(&self) -> &ObservationPredicateStore {
        &self.observation_predicates
    }
    pub const fn observation_predicate_memory_mut(&mut self) -> &mut ObservationPredicateStore {
        &mut self.observation_predicates
    }

    #[must_use]
    pub const fn raw_relation_memory(&self) -> &RawRelationStore {
        &self.raw_relations
    }
    pub const fn raw_relation_memory_mut(&mut self) -> &mut RawRelationStore {
        &mut self.raw_relations
    }

    #[must_use]
    pub const fn relation_expression_memory(&self) -> &RelationExpressionStore {
        &self.relation_expressions
    }

    pub const fn relation_expression_memory_mut(&mut self) -> &mut RelationExpressionStore {
        &mut self.relation_expressions
    }

    #[must_use]
    pub const fn scalar_expression_memory(&self) -> &ScalarExpressionStore {
        &self.scalar_expressions
    }

    pub const fn scalar_expression_memory_mut(&mut self) -> &mut ScalarExpressionStore {
        &mut self.scalar_expressions
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

    /// Execute every persistent coordinate-layer mechanism as a proposal on a
    /// current exact scene. Acceptance still requires independent replay.
    ///
    /// # Errors
    ///
    /// Returns explicit absence or capacity failures. Inapplicable mechanisms
    /// are rejected locally and cannot emit a render.
    pub fn propose_layer_renders_from_memory(
        &self,
        scene: &LayerScene,
        maximum_points: usize,
    ) -> Result<LayerMemoryOutcome, CognitionError> {
        if maximum_points == 0 {
            return Err(CognitionError::LayerMemory(
                LayerMemoryRuntimeError::Capacity,
            ));
        }
        let mut persistent_mechanisms_attempted = 0_u64;
        let mut proposals = Vec::new();
        for mechanism in self.layer_mechanisms.persistent_mechanisms() {
            persistent_mechanisms_attempted = persistent_mechanisms_attempted.saturating_add(1);
            match LayerMechanismLearner.execute(mechanism, scene, maximum_points) {
                Ok(render) => proposals.push(LayerMemoryProposal {
                    mechanism: mechanism.semantic_hash().map_err(|_| {
                        CognitionError::LayerMemory(LayerMemoryRuntimeError::Capacity)
                    })?,
                    render,
                }),
                Err(
                    LayerMechanismError::AmbiguousRole
                    | LayerMechanismError::EmptyConstruction
                    | LayerMechanismError::InvalidScene
                    | LayerMechanismError::DimensionMismatch
                    | LayerMechanismError::PointOutOfBounds
                    | LayerMechanismError::DuplicateLayerValue,
                ) => {}
                Err(_) => {
                    return Err(CognitionError::LayerMemory(
                        LayerMemoryRuntimeError::Capacity,
                    ));
                }
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::LayerMemory(
                LayerMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(LayerMemoryOutcome {
            proposals,
            persistent_mechanisms_attempted,
        })
    }

    /// Rebind every persistent entity correspondence to a current scene and
    /// return untrusted constructions with complete producer provenance.
    ///
    /// # Errors
    ///
    /// Returns explicit absence or capacity failure. Acceptance remains the
    /// responsibility of the independent verifier.
    pub fn propose_entity_constructions_from_memory(
        &self,
        scene: &EntityGeometryScene,
        limits: EntityCorrespondenceLimits,
    ) -> Result<EntityCorrespondenceMemoryOutcome, CognitionError> {
        if limits.maximum_candidates == 0 || limits.maximum_points == 0 {
            return Err(CognitionError::EntityCorrespondenceMemory(
                EntityCorrespondenceMemoryRuntimeError::Capacity,
            ));
        }
        let mut persistent_mechanisms_attempted = 0_u64;
        let mut proposals = Vec::new();
        for mechanism in self.entity_correspondences.persistent_mechanisms() {
            persistent_mechanisms_attempted = persistent_mechanisms_attempted.saturating_add(1);
            match EntityCorrespondenceLearner.execute(mechanism, scene, limits) {
                Ok(construction) => proposals.push(EntityCorrespondenceMemoryProposal {
                    mechanism: mechanism.semantic_hash().map_err(|_| {
                        CognitionError::EntityCorrespondenceMemory(
                            EntityCorrespondenceMemoryRuntimeError::Capacity,
                        )
                    })?,
                    construction,
                }),
                Err(
                    EntityCorrespondenceError::InvalidScene
                    | EntityCorrespondenceError::InvalidRender
                    | EntityCorrespondenceError::DimensionCapacity
                    | EntityCorrespondenceError::ConstructionConflict,
                ) => {}
                Err(_) => {
                    return Err(CognitionError::EntityCorrespondenceMemory(
                        EntityCorrespondenceMemoryRuntimeError::Capacity,
                    ));
                }
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::EntityCorrespondenceMemory(
                EntityCorrespondenceMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(EntityCorrespondenceMemoryOutcome {
            proposals,
            persistent_mechanisms_attempted,
        })
    }

    /// Execute every persistent synthesized program on a current scene as an
    /// untrusted proposal. No v1 correspondence or expected output is used.
    ///
    /// # Errors
    ///
    /// Returns typed absence/capacity failures; independent replay still owns
    /// acceptance.
    pub fn propose_construction_programs_from_memory(
        &self,
        scene: &EntityGeometryScene,
        limits: ExecLimits,
    ) -> Result<ConstructionProgramMemoryOutcome, CognitionError> {
        let mut persistent_programs_attempted = 0_u64;
        let mut proposals = Vec::new();
        for program in self.construction_programs.persistent_programs() {
            persistent_programs_attempted = persistent_programs_attempted.saturating_add(1);
            match EntityConstructionProgramSynthesizer.execute(program, scene, limits) {
                Ok(construction) => proposals.push(ConstructionProgramMemoryProposal {
                    program: program.semantic_hash().map_err(|_| {
                        CognitionError::ConstructionProgramMemory(
                            ConstructionProgramMemoryRuntimeError::Capacity,
                        )
                    })?,
                    construction,
                }),
                Err(
                    EntityConstructionProgramError::InvalidTrace
                    | EntityConstructionProgramError::UnsupportedDimensions
                    | EntityConstructionProgramError::ConstructionConflict
                    | EntityConstructionProgramError::OutOfBounds,
                ) => {}
                Err(_) => {
                    return Err(CognitionError::ConstructionProgramMemory(
                        ConstructionProgramMemoryRuntimeError::Capacity,
                    ));
                }
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::ConstructionProgramMemory(
                ConstructionProgramMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(ConstructionProgramMemoryOutcome {
            proposals,
            persistent_programs_attempted,
        })
    }

    /// Execute persistent conditional programs as untrusted current-scene
    /// proposals. Neither selected entities nor expected output are supplied.
    ///
    /// # Errors
    ///
    /// Returns typed absence or capacity failure. Independent replay remains
    /// the sole acceptance authority.
    pub fn propose_conditional_constructions_from_memory(
        &self,
        scene: &RelationalGeometryScene,
        limits: ExecLimits,
    ) -> Result<ConditionalConstructionMemoryOutcome, CognitionError> {
        let mut persistent_programs_attempted = 0_u64;
        let mut proposals = Vec::new();
        for program in self.conditional_constructions.persistent_programs() {
            persistent_programs_attempted = persistent_programs_attempted.saturating_add(1);
            match ConditionalConstructionSynthesizer.execute(program, scene, limits) {
                Ok(construction) => proposals.push(ConditionalConstructionMemoryProposal {
                    program: program.semantic_hash().map_err(|_| {
                        CognitionError::ConditionalConstructionMemory(
                            ConditionalConstructionMemoryRuntimeError::Capacity,
                        )
                    })?,
                    construction,
                }),
                Err(
                    ConditionalConstructionError::InvalidScene
                    | ConditionalConstructionError::AmbiguousPreservedAttribute
                    | ConditionalConstructionError::Prediction,
                ) => {}
                Err(_) => {
                    return Err(CognitionError::ConditionalConstructionMemory(
                        ConditionalConstructionMemoryRuntimeError::Capacity,
                    ));
                }
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::ConditionalConstructionMemory(
                ConditionalConstructionMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(ConditionalConstructionMemoryOutcome {
            proposals,
            persistent_programs_attempted,
        })
    }

    /// Execute every persistent one-to-many constructor as an untrusted
    /// proposal over current values.
    ///
    /// # Errors
    ///
    /// Returns typed absence or exact execution failure.
    pub fn propose_expansions_from_memory(
        &self,
        inputs: &[i64],
    ) -> Result<ExpansionMemoryOutcome, CognitionError> {
        let mut persistent_mechanisms_attempted = 0_u64;
        let mut proposals = Vec::new();
        for mechanism in self.expansions.persistent_mechanisms() {
            persistent_mechanisms_attempted = persistent_mechanisms_attempted.saturating_add(1);
            match CollectionMechanismInventor.expand(mechanism, inputs) {
                Ok(output) => proposals.push(output),
                Err(CollectionInventionError::Invention(_)) => {
                    return Err(CognitionError::ExpansionMemory(
                        ExpansionMemoryRuntimeError::Capacity,
                    ));
                }
                Err(_) => {}
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::ExpansionMemory(
                ExpansionMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(ExpansionMemoryOutcome {
            proposals,
            persistent_mechanisms_attempted,
        })
    }

    /// Execute persistent many-to-one mechanisms as untrusted proposals.
    ///
    /// # Errors
    ///
    /// Returns typed absence or exact execution failure.
    pub fn propose_group_merges_from_memory(
        &self,
        groups: &[BTreeSet<i64>],
    ) -> Result<GroupMergeMemoryOutcome, CognitionError> {
        let mut persistent_mechanisms_attempted = 0_u64;
        let mut proposals = Vec::new();
        for mechanism in self.group_merges.persistent_mechanisms() {
            persistent_mechanisms_attempted = persistent_mechanisms_attempted.saturating_add(1);
            match GroupMergeInventor.merge(mechanism, groups) {
                Ok(output) => proposals.push(output),
                Err(GroupMergeError::Invention(_) | GroupMergeError::Capacity) => {
                    return Err(CognitionError::GroupMergeMemory(
                        GroupMergeMemoryRuntimeError::Capacity,
                    ));
                }
                Err(_) => {}
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::GroupMergeMemory(
                GroupMergeMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(GroupMergeMemoryOutcome {
            proposals,
            persistent_mechanisms_attempted,
        })
    }

    /// Rebind persistent relation-bound merges as untrusted proposals.
    ///
    /// # Errors
    ///
    /// Returns typed absence or exact execution failure. Current-task evidence
    /// must still be verified by a separate authority before acceptance.
    pub fn propose_grouping_merges_from_memory(
        &self,
        candidates: &BTreeMap<RelationRole, Vec<BTreeSet<i64>>>,
    ) -> Result<GroupingMergeMemoryOutcome, CognitionError> {
        let mut persistent_mechanisms_attempted = 0_u64;
        let mut proposals = Vec::new();
        for mechanism in self.grouping_merges.persistent_mechanisms() {
            persistent_mechanisms_attempted = persistent_mechanisms_attempted.saturating_add(1);
            match GroupingRelationInventor.predict(mechanism, candidates) {
                Ok(output) => proposals.push(output),
                Err(GroupingSelectionError::Merge(
                    GroupMergeError::Invention(_) | GroupMergeError::Capacity,
                )) => {
                    return Err(CognitionError::GroupingMergeMemory(
                        GroupingMergeMemoryRuntimeError::Capacity,
                    ));
                }
                Err(_) => {}
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::GroupingMergeMemory(
                GroupingMergeMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(GroupingMergeMemoryOutcome {
            proposals,
            persistent_mechanisms_attempted,
        })
    }

    /// Construct relation partitions from raw edges and apply persistent
    /// relation-bound merges as untrusted proposals.
    ///
    /// # Errors
    ///
    /// Returns typed absence or bounded/exact lowering failure.
    pub fn propose_raw_grouping_merges_from_memory(
        &self,
        evidence: &RawGroupingEvidence,
        limits: RawGroupingLimits,
    ) -> Result<GroupingMergeMemoryOutcome, CognitionError> {
        let mut persistent_mechanisms_attempted = 0_u64;
        let mut proposals = Vec::new();
        for mechanism in self.grouping_merges.persistent_mechanisms() {
            persistent_mechanisms_attempted = persistent_mechanisms_attempted.saturating_add(1);
            match RawRelationGroupingInventor.predict(mechanism, evidence, limits) {
                Ok(output) => proposals.push(output),
                Err(
                    RawGroupingError::Capacity
                    | RawGroupingError::Selection(GroupingSelectionError::Merge(
                        GroupMergeError::Invention(_) | GroupMergeError::Capacity,
                    )),
                ) => {
                    return Err(CognitionError::GroupingMergeMemory(
                        GroupingMergeMemoryRuntimeError::Capacity,
                    ));
                }
                Err(_) => {}
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::GroupingMergeMemory(
                GroupingMergeMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(GroupingMergeMemoryOutcome {
            proposals,
            persistent_mechanisms_attempted,
        })
    }

    /// Execute persistent invented pair predicates and aggregates as untrusted
    /// proposals over current attributed entities.
    ///
    /// # Errors
    ///
    /// Returns typed absence or bounded/exact execution failure. A separate
    /// verifier must replay current evidence before any proposal is accepted.
    pub fn propose_predicate_groupings_from_memory(
        &self,
        entities: &[AttributedGroupingEntity],
        limits: PredicateGroupingLimits,
    ) -> Result<PredicateGroupingMemoryOutcome, CognitionError> {
        let mut persistent_mechanisms_attempted = 0_u64;
        let mut proposals = Vec::new();
        for mechanism in self.predicate_groupings.persistent_mechanisms() {
            persistent_mechanisms_attempted = persistent_mechanisms_attempted.saturating_add(1);
            match PredicateGroupingInventor.predict(mechanism, entities, limits) {
                Ok(output) => proposals.push(output),
                Err(
                    PredicateGroupingError::Capacity
                    | PredicateGroupingError::RelationExecution
                    | PredicateGroupingError::Merge(GroupMergeError::Capacity),
                ) => {
                    return Err(CognitionError::PredicateGroupingMemory(
                        PredicateGroupingMemoryRuntimeError::Capacity,
                    ));
                }
                Err(_) => {}
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::PredicateGroupingMemory(
                PredicateGroupingMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(PredicateGroupingMemoryOutcome {
            proposals,
            persistent_mechanisms_attempted,
        })
    }

    /// Execute persistent synthesized observation-feature programs and their
    /// learned predicate/aggregate compositions as untrusted proposals.
    ///
    /// This method does not accept an answer: the caller must independently
    /// replay the selected mechanism over complete current evidence.
    ///
    /// # Errors
    ///
    /// Returns typed absence, resource capacity, or exact execution failure.
    pub fn propose_observation_predicates_from_memory(
        &self,
        entities: &[PrimitivePairEntity],
        limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
    ) -> Result<ObservationPredicateMemoryOutcome, CognitionError> {
        let mut persistent_mechanisms_attempted = 0_u64;
        let mut proposals = Vec::new();
        for module in self.observation_predicates.persistent_modules() {
            if !module
                .support()
                .is_some_and(|support| support.structurally_applies(entities))
            {
                continue;
            }
            persistent_mechanisms_attempted = persistent_mechanisms_attempted.saturating_add(1);
            match ObservationPredicateInventor.predict(
                module.mechanism(),
                entities,
                limits,
                predicate_limits,
            ) {
                Ok(output) => proposals.push(output),
                Err(ObservationPredicateError::Capacity) => {
                    return Err(CognitionError::ObservationPredicateMemory(
                        ObservationPredicateMemoryRuntimeError::Capacity,
                    ));
                }
                Err(ObservationPredicateError::FeatureExecution) => {
                    return Err(CognitionError::ObservationPredicateMemory(
                        ObservationPredicateMemoryRuntimeError::Execution,
                    ));
                }
                Err(_) => {}
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::ObservationPredicateMemory(
                ObservationPredicateMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(ObservationPredicateMemoryOutcome {
            proposals,
            persistent_mechanisms_attempted,
        })
    }

    /// Rebind persistent source-independent feature mechanisms across every
    /// anonymous relation role in one raw episode. Returned interpretations
    /// remain untrusted proposals requiring multi-episode verifier replay.
    ///
    /// # Errors
    ///
    /// Returns typed absence or explicit entity/feature capacity exhaustion.
    pub fn propose_entity_observations_from_memory(
        &self,
        trace: &RawEntityObservationTrace,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
    ) -> Result<EntityObservationMemoryOutcome, CognitionError> {
        let roles = trace
            .evidence
            .relations
            .iter()
            .map(|relation| relation.role.clone())
            .collect::<BTreeSet<_>>();
        if roles.len() > entity_limits.maximum_roles_per_episode {
            return Err(CognitionError::EntityObservationMemory(
                EntityObservationMemoryRuntimeError::Capacity,
            ));
        }
        let mut proposals = Vec::new();
        let mut persistent_mechanisms_considered = 0_u64;
        let mut entity_bindings_attempted = 0_u64;
        for module in self.observation_predicates.persistent_modules() {
            persistent_mechanisms_considered = persistent_mechanisms_considered.saturating_add(1);
            for relation in &roles {
                entity_bindings_attempted = entity_bindings_attempted.saturating_add(1);
                let lowered = match EntityObservationInventor.lower(
                    trace,
                    relation,
                    entity_limits,
                    observation_limits,
                ) {
                    Ok(lowered) => lowered,
                    Err(EntityObservationError::Capacity) => {
                        return Err(CognitionError::EntityObservationMemory(
                            EntityObservationMemoryRuntimeError::Capacity,
                        ));
                    }
                    Err(_) => continue,
                };
                if !module
                    .support()
                    .is_some_and(|support| support.structurally_applies(&lowered.entities))
                {
                    continue;
                }
                match ObservationPredicateInventor.predict(
                    module.mechanism(),
                    &lowered.entities,
                    observation_limits,
                    predicate_limits,
                ) {
                    Ok(output) => proposals.push(EntityObservationMemoryProposal {
                        mechanism: module.id(),
                        relation: relation.clone(),
                        output,
                    }),
                    Err(ObservationPredicateError::Capacity) => {
                        return Err(CognitionError::EntityObservationMemory(
                            EntityObservationMemoryRuntimeError::Capacity,
                        ));
                    }
                    Err(_) => {}
                }
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::EntityObservationMemory(
                EntityObservationMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(EntityObservationMemoryOutcome {
            proposals,
            persistent_mechanisms_considered,
            entity_bindings_attempted,
        })
    }

    /// Bind persistent source-independent mechanisms across anonymous raw
    /// primitive attributes. Equality relations and entities are reconstructed
    /// for each binding, but every result remains an untrusted proposal.
    ///
    /// # Errors
    ///
    /// Returns typed absence or explicit relation/entity/feature capacity.
    #[allow(clippy::too_many_arguments)]
    pub fn propose_raw_relation_observations_from_memory(
        &self,
        trace: &RawRelationObservationTrace,
        relation_limits: RelationObservationLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
    ) -> Result<RawRelationObservationMemoryOutcome, CognitionError> {
        let bindings = RawRelationObservationInventor
            .candidate_bindings(trace, relation_limits)
            .map_err(|error| map_raw_relation_memory_error(&error))?;
        let mut proposals = Vec::new();
        let mut persistent_mechanisms_considered = 0_u64;
        let mut relation_bindings_attempted = 0_u64;
        for module in self.observation_predicates.persistent_modules() {
            persistent_mechanisms_considered = persistent_mechanisms_considered.saturating_add(1);
            for binding in &bindings {
                relation_bindings_attempted = relation_bindings_attempted.saturating_add(1);
                let lowered = match RawRelationObservationInventor.lower(
                    trace,
                    binding,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                ) {
                    Ok(lowered) => lowered,
                    Err(
                        RelationObservationError::Capacity
                        | RelationObservationError::Entity(EntityObservationError::Capacity),
                    ) => {
                        return Err(CognitionError::RawRelationObservationMemory(
                            RawRelationObservationMemoryRuntimeError::Capacity,
                        ));
                    }
                    Err(_) => continue,
                };
                if !module
                    .support()
                    .is_some_and(|support| support.structurally_applies(&lowered.entities))
                {
                    continue;
                }
                match ObservationPredicateInventor.predict(
                    module.mechanism(),
                    &lowered.entities,
                    observation_limits,
                    predicate_limits,
                ) {
                    Ok(output) => proposals.push(RawRelationObservationMemoryProposal {
                        mechanism: module.id(),
                        binding: binding.clone(),
                        output,
                    }),
                    Err(ObservationPredicateError::Capacity) => {
                        return Err(CognitionError::RawRelationObservationMemory(
                            RawRelationObservationMemoryRuntimeError::Capacity,
                        ));
                    }
                    Err(_) => {}
                }
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::RawRelationObservationMemory(
                RawRelationObservationMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(RawRelationObservationMemoryOutcome {
            proposals,
            persistent_mechanisms_considered,
            relation_bindings_attempted,
        })
    }

    /// Rebind proof-gated multi-operator expressions to current anonymous
    /// attributes and emit untrusted answer-hidden proposals.
    ///
    /// # Errors
    ///
    /// Returns typed absence, malformed evidence, or an explicit capacity
    /// terminal. This path has no answer-acceptance authority.
    #[allow(clippy::too_many_arguments)]
    pub fn propose_relation_expressions_from_memory(
        &self,
        trace: &RawRelationObservationTrace,
        expression_limits: &RelationExpressionLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
    ) -> Result<RelationExpressionMemoryOutcome, CognitionError> {
        let mut proposals = Vec::new();
        let mut persistent_mechanisms_considered = 0_u64;
        let mut relation_bindings_attempted = 0_u64;
        for module in self.relation_expressions.persistent_modules() {
            persistent_mechanisms_considered = persistent_mechanisms_considered.saturating_add(1);
            match RelationExpressionInventor.propose(
                module.mechanism(),
                trace,
                expression_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
            ) {
                Ok(current) => {
                    relation_bindings_attempted = relation_bindings_attempted
                        .saturating_add(u64::try_from(current.len()).unwrap_or(u64::MAX));
                    proposals.extend(current.into_iter().map(|proposal| {
                        RelationExpressionMemoryProposal {
                            mechanism: module.id(),
                            binding: proposal.binding,
                            output: proposal.output,
                        }
                    }));
                }
                Err(
                    RelationExpressionError::Capacity
                    | RelationExpressionError::Entity(EntityObservationError::Capacity),
                ) => {
                    return Err(CognitionError::RelationExpressionMemory(
                        RelationExpressionMemoryRuntimeError::Capacity,
                    ));
                }
                Err(RelationExpressionError::InvalidEvidence) => {
                    return Err(CognitionError::RelationExpressionMemory(
                        RelationExpressionMemoryRuntimeError::InvalidEvidence,
                    ));
                }
                Err(_) => {}
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::RelationExpressionMemory(
                RelationExpressionMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(RelationExpressionMemoryOutcome {
            proposals,
            persistent_mechanisms_considered,
            relation_bindings_attempted,
        })
    }

    /// Rebind persistent scalar expressions to current anonymous channels and
    /// emit untrusted value proposals with no answer-acceptance authority.
    ///
    /// # Errors
    ///
    /// Returns typed absence, malformed evidence, or explicit capacity.
    pub fn propose_scalar_expressions_from_memory(
        &self,
        inputs: &[i64],
        expression_limits: &RelationExpressionLimits,
    ) -> Result<ScalarExpressionMemoryOutcome, CognitionError> {
        let mut proposals = Vec::new();
        let mut persistent_mechanisms_considered = 0_u64;
        let mut bindings_attempted = 0_u64;
        for module in self.scalar_expressions.persistent_modules() {
            persistent_mechanisms_considered = persistent_mechanisms_considered.saturating_add(1);
            match ScalarExpressionInventor.propose(module.mechanism(), inputs, expression_limits) {
                Ok(current) => {
                    bindings_attempted = bindings_attempted
                        .saturating_add(u64::try_from(current.len()).unwrap_or(u64::MAX));
                    proposals.extend(current.into_iter().map(|(binding, output)| {
                        ScalarExpressionMemoryProposal {
                            mechanism: module.id(),
                            binding,
                            output,
                        }
                    }));
                }
                Err(ScalarExpressionError::Capacity) => {
                    return Err(CognitionError::ScalarExpressionMemory(
                        ScalarExpressionMemoryRuntimeError::Capacity,
                    ));
                }
                Err(ScalarExpressionError::InvalidEvidence) => {
                    return Err(CognitionError::ScalarExpressionMemory(
                        ScalarExpressionMemoryRuntimeError::InvalidEvidence,
                    ));
                }
                Err(_) => {}
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::ScalarExpressionMemory(
                ScalarExpressionMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(ScalarExpressionMemoryOutcome {
            proposals,
            persistent_mechanisms_considered,
            bindings_attempted,
        })
    }

    /// Rebind proof-gated complete relation programs, pruning every relation
    /// structure other than the learned operator while retaining current
    /// current attribute enumeration and independent acceptance.
    ///
    /// # Errors
    ///
    /// Returns typed absence or explicit relation/entity/feature capacity.
    #[allow(clippy::too_many_arguments)]
    pub fn propose_raw_relation_programs_from_memory(
        &self,
        trace: &RawRelationObservationTrace,
        relation_limits: RelationObservationLimits,
        entity_limits: EntityObservationLimits,
        observation_limits: ObservationPredicateLimits,
        predicate_limits: PredicateGroupingLimits,
    ) -> Result<RawRelationObservationMemoryOutcome, CognitionError> {
        let bindings = RawRelationObservationInventor
            .candidate_bindings(trace, relation_limits)
            .map_err(|error| map_raw_relation_memory_error(&error))?;
        let attribute_count = trace
            .primitives
            .values()
            .next()
            .map_or(0, |primitive| primitive.attributes.len());
        let mut proposals = Vec::new();
        let mut persistent_mechanisms_considered = 0_u64;
        let mut relation_bindings_attempted = 0_u64;
        for module in self.raw_relations.persistent_modules() {
            let Some(support) = module.support() else {
                continue;
            };
            let Ok(attribute_count) = u32::try_from(attribute_count) else {
                return Err(CognitionError::RawRelationObservationMemory(
                    RawRelationObservationMemoryRuntimeError::Capacity,
                ));
            };
            if attribute_count < support.minimum_attributes
                || attribute_count > support.maximum_attributes
            {
                continue;
            }
            persistent_mechanisms_considered = persistent_mechanisms_considered.saturating_add(1);
            for binding in bindings.iter().filter(|binding| {
                binding.operator == module.mechanism().operator
                    && u32::try_from(binding.attributes.len())
                        .is_ok_and(|arity| arity == module.mechanism().key_arity)
            }) {
                relation_bindings_attempted = relation_bindings_attempted.saturating_add(1);
                let lowered = match RawRelationObservationInventor.lower(
                    trace,
                    binding,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                ) {
                    Ok(lowered) => lowered,
                    Err(
                        RelationObservationError::Capacity
                        | RelationObservationError::Entity(EntityObservationError::Capacity),
                    ) => {
                        return Err(CognitionError::RawRelationObservationMemory(
                            RawRelationObservationMemoryRuntimeError::Capacity,
                        ));
                    }
                    Err(_) => continue,
                };
                match ObservationPredicateInventor.predict(
                    &module.mechanism().observation_mechanism,
                    &lowered.entities,
                    observation_limits,
                    predicate_limits,
                ) {
                    Ok(output) => proposals.push(RawRelationObservationMemoryProposal {
                        mechanism: module.id(),
                        binding: binding.clone(),
                        output,
                    }),
                    Err(ObservationPredicateError::Capacity) => {
                        return Err(CognitionError::RawRelationObservationMemory(
                            RawRelationObservationMemoryRuntimeError::Capacity,
                        ));
                    }
                    Err(_) => {}
                }
            }
        }
        if proposals.is_empty() {
            return Err(CognitionError::RawRelationObservationMemory(
                RawRelationObservationMemoryRuntimeError::NoVerifiedProposal,
            ));
        }
        Ok(RawRelationObservationMemoryOutcome {
            proposals,
            persistent_mechanisms_considered,
            relation_bindings_attempted,
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
