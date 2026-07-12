# CAIRN interfaces and contracts

This document fixes the first-pass data contracts. Sub-agents may extend fields through schema-versioned additions, but must not change semantics without an architecture decision record.

## 1. Common identifiers

```rust
#[repr(transparent)]
pub struct Hash32(pub [u8; 32]);

#[repr(transparent)]
pub struct TaskId(pub Hash32);

#[repr(transparent)]
pub struct ObservationId(pub Hash32);

#[repr(transparent)]
pub struct NodeId(pub u32);

#[repr(transparent)]
pub struct EdgeId(pub u32);

#[repr(transparent)]
pub struct EntityId(pub u32);

#[repr(transparent)]
pub struct DefinitionId(pub Hash32);

#[repr(transparent)]
pub struct TheoryId(pub Hash32);

#[repr(transparent)]
pub struct ArtifactId(pub Hash32);
```

All persistent IDs except arena-local integers are BLAKE3 hashes of canonical CBOR. Human task names are metadata and never semantic keys.

## 1.1 Universal mechanism contract

```rust
pub struct Mechanism {
    pub signature: Signature,
    pub variables: Vec<TypedVariable>,
    pub entity_definitions: Vec<DerivedDefinition>,
    pub preconditions: Formula,
    pub transition_rules: Vec<CausalRule>,
    pub invariants: Vec<Formula>,
    pub termination: TerminationCondition,
    pub effects: EffectModel,
    pub support: SupportGeometry,
    pub proof: DerivationCertificate,
}

pub struct SupportGeometry {
    pub observed_domains: Set<DomainClass>,
    pub observed_cardinalities: RangeSet,
    pub varied_variables: Set<VariableId>,
    pub invariant_variables: Set<VariableId>,
    pub unknown_variables: Set<VariableId>,
    pub validated_compositions: Set<CompositionId>,
    pub failed_conditions: Vec<Formula>,
}
```

Every field is represented by canonical, typed RKL or content-addressed
references to it. Static prediction uses an implicit transform intervention;
interactive transitions, procedures, and planning options use explicit actions.
No adapter operation enum is a `Mechanism`.

Mechanisms move through `candidate`, `local`, `provisional`, `validated`,
`persistent`, `compiled`, and `deprecated`. Promotion records held-out replay,
independent-task count, rebinding dimensions, negative transfer, candidate work
saved, samples/actions saved, and independent verification. Storage without
measured invocation and ablation is not library learning.

## 2. Artifact envelope

```rust
pub struct ArtifactEnvelope<T> {
    pub protocol: ProtocolVersion,
    pub schema: SchemaId,
    pub id: ArtifactId,
    pub parents: Vec<ArtifactId>,
    pub task: Option<TaskId>,
    pub producer: ProducerRecord,
    pub budget: BudgetRecord,
    pub created_at_logical: u64,
    pub payload: T,
    pub payload_hash: Hash32,
    pub signature: Option<ArtifactSignature>,
}
```

Required invariants:

- `id` is the hash of the canonical envelope without `signature`;
- parent IDs are sorted and duplicate-free;
- payload contains no process-local pointers;
- budget records actual consumed work, not requested work;
- verifier artifacts identify the exact input and executable semantics version.

## 3. Learning problems and observations

```rust
pub struct Observation {
    pub id: ObservationId,
    pub modality: ModalityId,
    pub logical_time: Option<u64>,
    pub source: SourceReference,
    pub payload: CanonicalValue,
    pub certainty: ObservationCertainty,
}

pub struct RelationalObservation {
    pub observation: Observation,
    pub elements: Vec<EvidenceElement>,
    pub relations: Vec<EvidenceRelation>,
}

pub struct EvidenceElement {
    pub id: EvidenceElementId,
    pub value_type: EvidenceValueType,
    pub value: Hash32,
}

pub struct EvidenceRelation {
    pub predicate: Hash32,
    pub participants: Vec<EvidenceElementId>,
}

pub struct Demonstration {
    pub context: Vec<ObservationId>,
    pub intervention: Option<Intervention>,
    pub outcome: Vec<ObservationId>,
}

pub struct LearningProblem {
    pub id: TaskId,
    pub observations: Vec<Observation>,
    pub demonstrations: Vec<Demonstration>,
    pub requested_predictions: Vec<PredictionRequest>,
    pub action_space: Option<ActionSpace>,
    pub attempt_limit: u8,
}
```

`Observation.payload` is schema-dispatched by `modality`; core cognition consumes
normalized evidence and source references rather than branching on a frontend.
Frontends may attach exact or uncertain observations, but uncertainty must remain
explicit through derivation and verification.

ADR-0019 adds `cairn.relational-observation.v1`. Element/relation input order is
non-semantic and canonicalized; IDs, typed values, predicates, ordered
participants, source, time, modality, and certainty are identity-bound.
Duplicate elements/relations, empty relations, unknown participants, capacity
excess, or payload tampering fail closed. Predicates and values are opaque;
their names are not interpreted by cognition.

An ARC adapter may define `Grid`, `GridPair`, and `ArcTask` in `cairn-arc12`.
Those types are not core contracts. Its normalization rules are:

- categorical cell values normalized to integer atoms without assigning semantics;
- exact dimensions preserved;
- serialization row-major and endian-independent;
- empty dimensions rejected;
- parser verifies all rows have equal width.

Gridworld, vector-graphics, audio-event, video-event, and text/knowledge adapters
use the same `LearningProblem` boundary. Adapter metadata cannot affect theory
semantics unless represented as evidence with provenance.

## 4. Evidence graph

```rust
pub enum NodeKind {
    Cell,
    Coordinate,
    Scalar,
    Atom,
    CellSet,
    Entity,
    EntitySet,
    Region,
    Boundary,
    Axis,
    Symbol,
    Action,
    State,
    Event,
    Role,
    Latent,
    Definition,
}

pub struct NodeRecord {
    pub kind: NodeKind,
    pub attrs: AttrSlice,
    pub derivation: DerivationId,
}

pub struct RelationRecord {
    pub relation: RelationId,
    pub arguments: ArgSlice,
    pub attrs: AttrSlice,
    pub weight: EvidenceWeight,
    pub derivation: DerivationId,
}
```

`EvidenceWeight` is exact for deterministic facts and log-likelihood-like for uncertain frontends. Exact adapter facts use exact weights only.

## 5. Derivations

```rust
pub enum DerivationKind {
    Observed,
    DeterministicKernel,
    AlgebraicRewrite,
    Hypothesis,
    ConstraintChoice,
    MemoryProposal,
    ExternalEvidence,
    Verification,
}

pub struct DerivationNode {
    pub kind: DerivationKind,
    pub operation: OperationId,
    pub inputs: SmallVec<[DerivationId; 4]>,
    pub parameters: CanonicalValue,
    pub semantics_version: SemanticsVersion,
}
```

A verification derivation may depend on hypotheses, but its result is only “this exact theory produces this exact output,” not “this theory is the true rule.”

## 6. Entity expressions

```rust
pub enum EntityExpr {
    ObservedCells(CellSetExpr),
    Connected {
        mask: CellSetExpr,
        connectivity: Connectivity,
    },
    Run {
        axis: AxisExpr,
        predicate: PredicateExpr,
    },
    Contour(Box<EntityExpr>),
    Holes(Box<EntityExpr>),
    BoundingBox(Box<EntityExpr>),
    Panel {
        region: RegionExpr,
        local_frame: FrameExpr,
    },
    RepeatGroup {
        seed: Box<EntityExpr>,
        displacement: VectorExpr,
    },
    MotifClass {
        domain: EntitySetExpr,
        equivalence: PredicateExpr,
    },
    DerivedCall {
        definition: DefinitionId,
        args: Vec<Expr>,
    },
}
```

Every expression exposes:

```rust
pub trait Denotable {
    type Output;

    fn infer_type(&self, env: &TypeEnv) -> Result<Type, TypeError>;
    fn evaluate(&self, ctx: &EvalContext) -> Result<Self::Output, EvalError>;
    fn lower_bound_cost(&self) -> Cost;
    fn free_variables(&self) -> VarSet;
}
```

## 7. Events and covers

```rust
pub struct EventHypothesis {
    pub id: EventId,
    pub kind: EventKind,
    pub sources: Vec<EntityRef>,
    pub targets: Vec<EntityRef>,
    pub parameters: ExprSketch,
    pub preconditions: Vec<RelationLiteral>,
    pub input_coverage: CellSet,
    pub output_coverage: CellSet,
    pub cost: CostVector,
    pub derivation: DerivationId,
}

pub struct DemonstrationEventCover {
    pub demonstration: DemonstrationId,
    pub events: Vec<EventId>,
    pub layer_constraints: Vec<LayerConstraint>,
    pub exact_replay: ReplayDigest,
    pub score: CostVector,
    pub structural_signature: Hash32,
}
```

A cover artifact is invalid unless the event set can be lowered to a replay plan that exactly produces the pair output.

## 8. Analogical alignments

```rust
pub struct RoleMapEntry {
    pub demonstration: DemonstrationId,
    pub local: LocalRoleRef,
    pub abstract_role: RoleVar,
}

pub struct Alignment {
    pub covers: Vec<CoverId>,
    pub role_map: Vec<RoleMapEntry>,
    pub event_clusters: Vec<EventCluster>,
    pub preserved_relations: Vec<RelationCorrespondence>,
    pub conflicts: Vec<AlignmentConflict>,
    pub score: AlignmentScore,
}
```

Alignment score is a structured vector. It must retain relation-preservation, causal-role, and intrinsic-feature terms separately for diagnostics.

## 9. RKL AST

```rust
pub enum Expr {
    Bool(bool),
    Int(i32),
    Atom(u16),
    Coord(Coord2),
    Var(VarId),
    Tuple(Vec<Expr>),
    Let {
        var: VarId,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    If {
        condition: Box<PredicateExpr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Call {
        function: FunctionId,
        args: Vec<Expr>,
    },
    SetComprehension(SetComprehension),
    Fold(FoldExpr),
    Derived {
        definition: DefinitionId,
        args: Vec<Expr>,
    },
}
```

The exact AST is split by type-specific enums in implementation to reduce runtime type checks. Serialization uses a stable tagged representation.

## 10. Rule and theory IR

```rust
pub struct Binder {
    pub variable: VarId,
    pub ty: Type,
    pub domain: Expr,
    pub quantifier: Quantifier,
}

pub enum Quantifier {
    One,
    Unique,
    ForEach,
    Group,
    Exists,
}

pub struct RewriteRule {
    pub id: RuleId,
    pub binders: Vec<Binder>,
    pub guard: PredicateExpr,
    pub effects: Vec<Effect>,
    pub priority: PriorityExpr,
}

pub enum Effect {
    Preserve(EntityExpr),
    Delete(EntityExpr),
    Update(UpdateEffect),
    Emit(EmitEffect),
    CreateRelation(CreateRelationEffect),
    Write { target: Expr, value: Expr },
    SetState(SetStateEffect),
}

pub struct Theory {
    pub version: TheoryVersion,
    pub inputs: Vec<Binder>,
    pub definitions: Vec<Definition>,
    pub rules: Vec<RewriteRule>,
    pub schedule: Schedule,
    pub renderer: Renderer,
    pub assumptions: Vec<Assumption>,
}
```

`inputs` are the only observation/request variables visible before rule-local
binders. Admission checks them once; execution requires matching runtime types
and values. This prevents unchecked free variables and avoids embedding one
observation into a supposedly reusable theory.

No synthesis score is included in the semantic hash. Score and search metadata live in `TheoryCandidate`.

## 11. Theory candidates and belief set

```rust
pub struct TheoryCandidate {
    pub theory: Theory,
    pub theory_id: TheoryId,
    pub origin: CandidateOrigin,
    pub exact_replay: Option<TrainingReplayCertificate>,
    pub falsification: Option<FalsificationReport>,
    pub cost: TheoryCost,
    pub predicted_outputs: Option<Vec<PredictedObservation>>,
}

pub struct TheoryBelief {
    pub theory_id: TheoryId,
    pub mode_id: ModeId,
    pub log_weight: f64,
    pub evidence_terms: EvidenceTerms,
}

pub struct TheoryBeliefSet {
    pub beliefs: Vec<TheoryBelief>,
    pub output_modes: Vec<OutputMode>,
}
```

Weights are heuristic model evidence, not calibrated probabilities until calibration experiments demonstrate otherwise.

## 12. Counterexamples and cores

```rust
pub enum CoreKind {
    MissingEntityDefinition,
    MissingCorrespondence,
    MissingSelectorFeature,
    MissingConstructor,
    MissingSymbolMeaning,
    MissingSchedule,
    MissingCanvas,
    LayerConflict,
    VariableArity,
    TypeConflict,
    BoundConflict,
    BackendCap,
}

pub struct ExplanationCore {
    pub kind: CoreKind,
    pub facts: Vec<FactRef>,
    pub blocked_choices: Vec<SemanticChoice>,
    pub mismatch_evidence: EvidenceSlice,
    pub minimality: CoreMinimality,
}
```

A core should be minimized when the backend supports it. Non-minimal cores must be labeled.

## 13. Definition proposals

```rust
pub enum DefinitionKind {
    Entity,
    Predicate,
    Feature,
    Constructor,
    SymbolMap,
    RecursiveAbbreviation,
    Option,
}

pub struct DefinitionProposal {
    pub kind: DefinitionKind,
    pub signature: TypedSignature,
    pub body: Expr,
    pub trigger: ExplanationCoreId,
    pub positive_bindings: BindingSet,
    pub negative_bindings: BindingSet,
    pub separation_report: SeparationReport,
    pub compression_report: CompressionReport,
    pub totality_proof: BoundProof,
}
```

## 14. Solver backend interface

```rust
pub trait ConstraintBackend {
    fn new_problem(&mut self, header: ProblemHeader) -> ProblemHandle;
    fn add_hard(&mut self, problem: ProblemHandle, constraint: HardConstraint);
    fn add_soft(&mut self, problem: ProblemHandle, constraint: SoftConstraint);
    fn solve(&mut self, problem: ProblemHandle, budget: SolverBudget) -> SolveResult;
    fn block_model(&mut self, problem: ProblemHandle, model: &Model, scope: BlockScope);
    fn explain_unsat(&mut self, problem: ProblemHandle) -> Option<UnsatCore>;
}
```

Required first backend:

- finite-domain Boolean encoding;
- incremental clauses;
- assumptions for cheap model blocking;
- deterministic tie-breaking at the CAIRN layer;
- optional MaxSAT optimization via repeated SAT or native backend.

## 15. E-graph interface

```rust
pub trait TypedEGraph {
    fn add(&mut self, expr: TypedExpr) -> EClassId;
    fn union(&mut self, a: EClassId, b: EClassId, proof: EqualityProof);
    fn run_goal_directed(&mut self, roots: &[EClassId], budget: EGraphBudget);
    fn analyze(&self, class: EClassId) -> &EClassAnalysis;
    fn extract_k(&self, class: EClassId, cost: &dyn CostModel, k: usize) -> Vec<TypedExpr>;
}
```

Universal equalities require algebraic proof. Task-local observational equality is stored with scope and cannot be exported as a universal rewrite.

## 16. Executor and verifier

```rust
pub trait TheoryExecutor {
    fn execute_prediction(
        &self,
        theory: &Theory,
        problem: &LearningProblem,
        request: &PredictionRequest,
        limits: ExecLimits,
    ) -> Result<ExecutionTrace, ExecError>;

    fn execute_transition(
        &self,
        theory: &WorldTheory,
        state: &BeliefState,
        action: &Action,
        limits: ExecLimits,
    ) -> Result<TransitionPrediction, ExecError>;
}

pub trait IndependentVerifier {
    fn verify_demonstrations(
        &self,
        problem: &LearningProblem,
        theory: &Theory,
    ) -> Result<TrainingReplayCertificate, VerifyError>;

    fn verify_prediction(
        &self,
        problem: &LearningProblem,
        certificate: &PredictionCertificate,
    ) -> Result<(), VerifyError>;
}
```

The verifier must use a separately implemented reference execution path for critical effects and observation-renderer semantics. An adapter may additionally verify lossless lowering and domain-specific output encoding; it may not replace core replay.

## 17. Swarm job interface

```rust
pub struct Job {
    pub id: JobId,
    pub task: TaskId,
    pub kind: JobKind,
    pub inputs: Vec<ArtifactId>,
    pub requirements: CapabilitySet,
    pub budget: JobBudget,
    pub priority: PriorityVector,
    pub seed: u64,
}

pub struct JobResult {
    pub job: JobId,
    pub status: JobStatus,
    pub artifacts: Vec<ArtifactId>,
    pub metrics: JobMetrics,
    pub followups: Vec<JobProposal>,
}
```

Jobs are idempotent. Re-running a deterministic job with the same inputs, semantic version, seed, and budget must produce the same semantic artifacts.

## 18. Memory interfaces

```rust
pub trait MechanismStore {
    fn propose(&self, query: MechanismQuery, limit: usize) -> Vec<ModuleId>;
    fn load(&self, id: ModuleId) -> Result<MechanismModule, StoreError>;
    fn submit_for_promotion(&mut self, candidate: PromotionCandidate) -> PromotionResult;
    fn record_counterexample(&mut self, id: ModuleId, counterexample: CounterexampleId);
    fn deprecate(&mut self, id: ModuleId, reason: DeprecationReason);
}
```

Promotion is a separate executable and evidence pipeline. The online solver may create task-local definitions but cannot mark them persistent by itself.

Memory-enabled and memory-disabled runs use the same frozen problems, verifier,
CPU budget, and answer policy. The store is successful only when it improves
future sample or search efficiency without reducing exact accuracy.

Actual-use evidence crosses process boundaries through
`cairn.memory.reuse-evidence` version 1. The canonical payload binds theory and
experience hashes, verified-case count, and counterfactual search, sample, and
proof-depth metrics. Its envelope is experience-scoped and requires parent
lineage. A loaded payload remains untrusted: `ReuseArtifactArchive::replay`
must reconstruct an opaque verifier receipt from the referenced checked theory
and evidence plus freshly measured counterfactual metrics before the store or
consolidator may count it. Persisted metrics are comparison targets, never
replay inputs. CAS identity proves immutability, not correctness.

Consolidation source evidence crosses process boundaries through
`cairn.consolidator.proof-graph` version 1. The canonical DAG payload contains
only typed operation structure, opaque argument hashes, an episode identity,
and an output node. `ProofGraphArchive::load` checks the envelope, bounds,
scope, type codes, and topology; `ConsolidationRuntime::submit` re-admits it.
Compression remains a proposal until `CompressionVerifier` independently
reconstructs every source and cost. Persisted proposals cannot replace their
source DAGs.

Consolidator semantics 2.2.0 adds `InternalGraphCompressionProposal`. Its
pattern contains typed nodes whose inputs are local node references or explicit
boundary-parameter references. `mine_internal_proof_subgraphs` exhaustively
considers bounded contiguous topological windows, anti-unifies typed values,
exactly reconstructs every source, and emits only positive-cost proposals.
`match_internal_proof_subgraph` rebinds the pattern to an unseen host.
`InternalCompressionVerifier` independently checks topology, selections,
boundary edges and types, replay, costs, and semantic identity. This interface
does not denote arbitrary non-contiguous subgraphs or an RKL definition.

Consolidator semantics 2.3.0 adds `CompiledInternalMechanism`. Its canonical
identity excludes episodes, proposal costs, and occurrence bindings; those
remain provenance. `InternalMechanismVerifier` independently verifies
compilation and current-host expansion. `InternalReuseMetrics` are derived by
the verifier from bounded fresh-mining and compiled-matching work rather than
accepted from runtime. Under memory semantics 1.6.0,
`InternalMechanismStore::record_runtime_reuse` retains opaque receipts and
automatically calls `InternalPromotionConsolidator` after two independent
experiences. Only the returned `VerifiedInternalPromotion` may change the
module from provisional to persistent. `register_compilation` permits another
independently verified proposal to support the same semantic mechanism, but a
receipt from that lineage fails until registration. This status is not yet a
directly serialized authority.

Memory semantics 1.7.0 adds
`cairn.memory.internal-reuse-evidence` version 1. Its canonical payload commits
to mechanism, proposal, and experience identities plus verified-case and
fixed-work measurements. `InternalReuseArtifactArchive::load` returns an
untrusted value. `replay` requires reconstructed proof sources, proposal,
compiled mechanism, compilation verification, current host, and binding; it
rederives every identity and metric before returning a new opaque receipt. A
fresh `InternalMechanismStore` and ordinary consolidation must reproduce
promotion. Serialized tokens and module status never regain authority.

## 19. Planner interfaces

```rust
pub struct BeliefState {
    pub current: RelGraphSnapshot,
    pub latent_beliefs: Vec<LatentBelief>,
    pub theory_beliefs: Vec<WorldTheoryBelief>,
    pub history_summary: HistorySummary,
}

pub trait InterventionPolicy {
    fn propose(
        &self,
        belief: &BeliefState,
        actions: &ActionSpace,
        budget: PlanBudget,
    ) -> Vec<ActionCandidate>;
}

pub trait Planner {
    fn plan(
        &self,
        belief: &BeliefState,
        goals: &[GoalHypothesis],
        budget: PlanBudget,
    ) -> PlanResult;
}
```

## 20. Proof-backed knowledge

```rust
pub struct KnowledgeClaim {
    pub subject: EntityRef,
    pub predicate: RelationId,
    pub object: CanonicalValue,
    pub valid_time: TemporalExtent,
    pub confidence: EvidenceWeight,
    pub sources: Vec<SourceReference>,
    pub derivation: DerivationId,
}

pub struct KnowledgeConflict {
    pub claims: Vec<KnowledgeClaimId>,
    pub conflict_kind: ConflictKind,
    pub resolution: Option<ResolutionProposal>,
}

pub trait KnowledgeStore {
    fn resolve_entity(&self, proposal: EntityResolution) -> Vec<EntityCandidate>;
    fn query(&self, query: KnowledgeQuery, limit: usize) -> Vec<KnowledgeClaimId>;
    fn submit(&mut self, claim: KnowledgeClaim) -> Result<KnowledgeClaimId, StoreError>;
    fn conflicts(&self, claim: KnowledgeClaimId) -> Vec<KnowledgeConflict>;
}
```

Claims remain typed, temporal, contradictory when evidence conflicts, and linked
to their source observations. Retrieval is never proof that a claim is true.

## 21. Metrics contract

Every task result records at minimum:

```text
solved
attempt count
wall time
CPU time
peak memory
agent CPU breakdown
entity expressions generated and retained
event hypotheses and covers
alignments
rule sketches
constraint variables, clauses, and solves
e-graph nodes and merges
theories executed
counterexamples learned
definitions invented
demonstrations consumed
interventions attempted
transition predictions and errors
skills learned, reused, and composed
memory proposals, rebindings, and verifier rejects
knowledge claims, source coverage, and unresolved contradictions
falsification tests
terminal taxonomy
certificate hash
```

This metrics schema is mandatory before any full benchmark sweep. Reports must
separate task learning, continual learning, and interactive causal learning so
an improvement in one loop cannot conceal the absence of another.

## 22. Language, dialogue, and response contracts

```rust
pub struct UserIntent {
    pub speech_act: SpeechAct,
    pub objective: GoalExpression,
    pub constraints: Vec<Constraint>,
    pub referenced_entities: Vec<EntityReference>,
    pub requested_output: OutputSpecification,
    pub uncertainty: Vec<Ambiguity>,
}

pub struct DialogueState {
    pub current_goal: Option<GoalExpression>,
    pub established_facts: Vec<GroundedFact>,
    pub unresolved_questions: Vec<Ambiguity>,
    pub user_preferences: Vec<Preference>,
    pub prior_artifacts: Vec<ArtifactReference>,
}

pub struct CognitiveRequest {
    pub intent: UserIntent,
    pub dialogue_context: DialogueContext,
    pub available_tools: Vec<ToolDefinition>,
    pub relevant_knowledge: Vec<KnowledgeClaim>,
}

pub struct CognitiveResponse {
    pub answer_plan: AnswerPlan,
    pub artifacts: Vec<GeneratedArtifact>,
    pub tool_requests: Vec<ToolInvocation>,
    pub unresolved_ambiguities: Vec<Ambiguity>,
    pub confidence: Rational,
    pub proof: Option<DerivationCertificate>,
}
```

The implemented initial slice lives in `cairn-language`: controlled intent,
dialogue goal retention, typed Python AST construction, executable examples,
and deterministic response realization. Logical runtime boundaries will split
as their contracts grow; no language crate may depend on an ARC adapter.

## 23. Structured distillation and swarm transfer contracts

```rust
pub struct DistillationJob {
    pub domain: Domain,
    pub learning_objective: LearningObjective,
    pub difficulty: Difficulty,
    pub required_variations: Vec<VariationAxis>,
    pub required_artifacts: Vec<ArtifactKind>,
    pub validation_plan: ValidationPlan,
    pub seed: u64,
}

pub struct KnowledgeArtifact {
    pub artifact_id: ContentHash,
    pub artifact_type: ArtifactType,
    pub semantics_version: SemanticsVersion,
    pub payload: ArtifactPayload,
    pub author_agent: AgentId,
    pub parent_artifacts: Vec<ContentHash>,
    pub evidence: Vec<EvidenceReference>,
    pub proof: Option<VerificationCertificate>,
    pub support: SupportGeometry,
    pub applicability: Formula,
    pub known_failures: Vec<Counterexample>,
    pub resource_profile: ResourceProfile,
    pub trust_state: TrustState,
    pub origin: KnowledgeOrigin,
}
```

Trust progresses through teacher-proposed, locally verified, peer verified,
transfer tested, swarm accepted, compiled, deprecated, and revoked states.
`cairn-distill` implements the first closed packet, compiler, local verifier,
two-peer gate, transfer gate, and revocation-capable registry. Provider calls and
batch orchestration remain outside the trusted runtime.

Metrics add teacher packets accepted/rejected, claims and procedures verified,
mechanisms anti-unified, teacher-independent cases, cross-agent and cross-domain
reuse, duplicate work avoided, propagation latency, revocations, and negative
transfer. Origin is permanent metadata.

## 24. Autonomous developmental curriculum contracts

`AutonomousCurriculum` owns only content-free developmental state. Each
`DevelopmentalObjective` declares an opaque ID, learning axis, produced
mechanism role, prerequisite roles, hypothesis count, transfer estimate, action
cost, and risk. `select_next` uses deterministic score and ID tie-breaking and
returns explicit complete, capacity, blocked-prerequisite, and outstanding-work
states.

`DevelopmentalOutcome` contains verifier status, mechanism evidence identity,
held-out count, memory-off/on candidate and example work, retention, and
negative transfer. It contains no observation or answer. Only verified,
transfer-tested, reducing outcomes update active support. One distinct verified
counterexample quarantines; two deprecate. `SustainedLearningGate` requires at
least three growing phases, strict within-phase candidate/sample/proof
reduction, capability non-regression, complete retention, negative transfer
below 2%, and restart evidence.

Developmental semantics 1.1.0 adds `CurriculumCheckpoint`, `CurriculumEvent`,
and `CurriculumCheckpointArchive`. The canonical checkpoint contains the
ordered objective contract, fixed decision budget, accepted event journal, and
optional outstanding decision. It deliberately omits derived mechanism maps
and lifecycle statuses. Loading validates bounded schema/kind/semantics/scope
and nonempty lineage, then reconstructs an empty `AutonomousCurriculum` and
replays ordinary selection, admission, and counterexample operations. Every
recorded decision must equal the deterministic recomputation. Restored state is
usable authority only after this replay succeeds.

Developmental semantics 1.2.0 adds `FailureDrivenObjectiveGenerator`,
`GeneratedDevelopmentalObjective`, and `DevelopmentalNeed`. The generator sees
only `LearningFailureCore` plus active mechanism-role identities and derives a
canonical objective. `AutonomousCurriculum::add_objective` records the addition
as an ordered `ObjectiveAdded` event. The checkpoint's objective list is the
initial set; generated additions replay at their original boundary so a future
objective cannot alter a past choice. The generator does not create task
payloads and cannot report a successful learning outcome.

## 25. Generated exercise and internal composition contracts

`DevelopmentalExerciseRequest` is the only curriculum-to-materializer message.
It contains objective/need/failure identities, prerequisite mechanism roles,
and the minimum independent variations; no task payload or expected answer is
present.

`InternalCompositionPlan` is an untrusted runtime proposal containing distinct
compiled-mechanism identities and occurrence-local bindings. The reference
proposer exhaustively searches bounded combinations, disallows overlap, and
requires every invocation output to occur in the selected result's ancestry.
`CognitionRuntime::plan_internal_composition` exposes this path only for
persistent internal modules.

`InternalCompositionEvidence` carries current-process source proposals,
compilations, sources, mechanisms, and bindings to the independent verifier.
`VerifiedInternalComposition` is returned only after separate reconstruction,
joint-contribution checking, and strict aggregate candidate/sample/proof
reduction. Its certificate is evidence, not an answer or a reusable verifier
token after restart.

ADR-0018 adds `ProofFrontier`, whose typed nodes are evidence only and cannot
contain `join-learned-results`. `construct_compiled_internal_compositions`
exhaustively expands persistent compiled mechanisms over bounded typed evidence
bindings and returns untrusted `ConstructedInternalComposition` candidates.
`CognitionRuntime::construct_internal_compositions` preserves the persistent
memory boundary.

After independent verification of one training candidate,
`induce_internal_construction_policy` may derive a task-local
`InternalConstructionPolicy` containing only typed binding ordinals and
mechanism identities. `apply_internal_construction_policy` rebinds those
ordinals to a changed frontier and emits exactly one candidate; unsupported
cardinality fails closed. The policy is not a persisted mechanism, proof, or
verification token.

`InternalMechanismVerifier::verify_constructed_composition` alone receives the
expected proof. It requires exact frontier prefix and complete-graph equality,
reconstructs current invocations through the independent composition verifier,
checks the fixed n-ary joint result, and returns
`VerifiedInternalConstruction`. Producer construction and expected truth never
share an interface.

ADR-0019 adds `LoweredObservationFrontier` with exact node provenance and a
canonical commitment. `lower_relational_observation` emits one root, one typed
node per element, and one ordered-input relation node. The independent verifier
reconstructs this through `verify_observation_frontier`; producer lowering code
is not imported into the replay path.

`RelationalConstructionPolicy` replaces value ordinals with typed two-round
relation-neighbourhood fingerprints. Fingerprints cover types, opaque
predicates, argument positions, and neighbouring fingerprints; they exclude
values, IDs, modality, source, task identity, order, and expected proof.
Missing or non-unique matches fail closed.

`verify_relational_policy_induction` independently reconstructs the complete
policy and binds it to a prior `VerifiedInternalConstruction`. Held-out
`verify_relational_constructed_composition` requires that opaque receipt,
replays the policy and learned mechanisms, and derives direct construction work
rather than unrelated sliding-window matching cost. The runtime accepts only
persistent mechanisms and never receives expected proof.
