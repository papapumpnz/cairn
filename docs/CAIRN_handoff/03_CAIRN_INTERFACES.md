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

## 26. Active latent-world experiment contracts

ADR-0020 adds `LatentExperiment`, which receives an episode index, immutable
visible transition history, and one selected `ActionId`. It returns one visible
`TransitionObservation`; hidden state is absent from both sides.

`ActiveLatentWorldLearner::induce` repeatedly runs exact minimal-transducer
induction, sorts unresolved successor ambiguities canonically, selects an
available one-step distinguishing action, validates the returned action,
predecessor continuity and state bounds, appends evidence, and re-induces.
`ActiveLatentInductionReport` contains the exact final theory, augmented visible
episodes, and `SelectedLatentIntervention` journal.

`ActiveLatentLimits` separately bounds exact induction and intervention count.
No available one-step discriminator, cap exhaustion, malformed observation, and
underlying search failures are distinct terminals. `CognitionRuntime` exposes
this loop through `learn_latent_world_actively` and exposes exact goal/latent
planning through `infer_goal_and_plan_latent_world`; it contains no environment
truth or adapter logic.

## 27. Persistent latent-mechanism contracts

ADR-0021 adds `LatentMechanismSchema`, a canonical finite hidden-state
mechanism containing only visible dimensionality, canonical latent-state count,
and sorted action-role rule tables. Source action IDs, state bounds, episodes,
support counts, and latent label choices are non-semantic. Compilation is
bounded by `LatentSchemaLimits`.

`LatentSchemaCompiler::bind` enumerates bounded action-role bijections and
retains only theories that exactly replay complete current visible episodes.
It returns all surviving semantic bindings rather than selecting silently.
`LatentSchemaVerifier` separately reconstructs alpha-normalization and current
transition replay and emits opaque compilation and transfer receipts.

`LatentMechanismStore::record_verified_world` accepts only matching receipt
pairs. One world is provisional; two distinct experience hashes promote the
schema automatically. Duplicate evidence cannot satisfy the threshold.
`CognitionRuntime::propose_latent_world_from_memory` retrieves only persistent
schemas and performs bounded current rebinding. Returned theories remain
proposals and require independent transfer verification before acceptance.

## 28. Developmental world scaling and restart contracts

`LatentSupportGeometry` reports only the visible dimensions, latent-state
count, action-role count, and independently verified world count actually
established for a schema. It does not infer untested ranges.

`LatentMechanismSchema` implements canonical encoding and strict bounded
decoding. `LatentWorldEvidenceArtifact` commits the schema, experience identity,
and independent replay counts. `LatentWorldEvidenceArchive` persists it with
parent lineage, loads it as untrusted, and reconstructs fresh compilation and
transfer receipts through `LatentWorldReplay`. Serialized module status never
restores authority.

`SequentialVersionSpace` contains executable finite transition hypotheses.
`next_discriminating_plan` enumerates action sequences in deterministic
length/identity order under `SequentialExperimentLimits`; it selects the
shortest depth with maximum visible-outcome partitioning. `observe_plan`
retains exact trace matches and preserves the old space if every hypothesis is
refuted.

`LatentMemoryOutcome::proposals` binds every current proposal to its schema
hash, preventing cross-schema attribution ambiguity. Sequence and vector
frontends expose adapter-only transition lowering into `LatentEpisode`; core
cognition contains no modality checks.

## 28. Relational layer mechanism contracts

`cairn-theory` exposes `CoordinateLayer`, `LayerScene`, `LayerSetExpression`,
`LayerRenderFrame`, `LayerMechanism`, `LayerLearningExample`, and
`LayerMechanismLearner`. The canonical mechanism marker is
`cairn.layer-mechanism.v1`; semantic identity excludes source modality and
example content. `LayerLearningLimits` bounds candidate and point work, and
ambiguous cardinality roles fail closed.

`cairn-verifier::LayerMechanismVerifier` is the separately implemented exact
reference path. Its opaque `VerifiedLayerMechanism` receipt commits to the
mechanism and distinct experiences and exposes only replay measurements and
support observations. It does not import the learner or runtime.

`cairn-memory::LayerMechanismStore` requires two non-overlapping verifier
cohorts before `Persistent`. `LayerSupportGeometry` accumulates observed
dimensions, extents, layer cardinalities, background values, and rendered
foreground values. `CognitionRuntime::propose_layer_renders_from_memory`
returns untrusted proposals with mechanism identity; blank or inapplicable
memory returns a typed `LayerMemoryRuntimeError`.

`cairn-vector::VectorAdapter::learn_layer_mechanism` and
`cairn-field::LayerFieldAdapter` are frontend bindings to the same theory. The
finite-field adapter lowers non-background categorical values to disjoint
coordinate layers and realizes an exact categorical output. It cannot represent
overlapping source layers, so overlap-dependent mechanisms remain outside that
adapter's support rather than being approximated.

## 29. Entity correspondence and provenance contracts

`cairn-theory` exposes `GeometricEntity`, `EntityGeometryScene`,
`RenderedEntity`, `EntityGeometryRender`, `CorrespondenceFrame`,
`EntityCorrespondenceMechanism`, `EntityConstruction`, and
`EntityCorrespondenceLearner`. Canonical identity uses
`cairn.entity-correspondence.v1`. The initial exact language is an N-dimensional
axis bijection, per-output-axis reversal, and preserved/tight frame.

`EntityConstructionProvenance` names the opaque source entity and contains one
`PointConstructionProvenance` for every source/output coordinate pair.
Construction conflicts, malformed permutations, ambiguous mechanisms, and
resource exhaustion are typed failures.

`EntityCorrespondenceVerifier` independently returns a
`VerifiedEntityCorrespondence` receipt containing experience identities,
replayed entities/points, and empirical support. `EntityCorrespondenceStore`
requires two disjoint cohorts before persistence. `CognitionRuntime` exposes
proposal-only `propose_entity_constructions_from_memory`.

`VectorAdapter::learn_entity_correspondence` supplies no operation label.
`EntityCorrespondenceFieldAdapter` lowers non-background categories as disjoint
entities and exactly realizes verified renders; categorical overlap fails rather
than being approximated.

## 30. Generic entity-construction program contracts

`EntityConstructionProgram` contains learned `output_extents` and
`output_coordinates` definitions. Canonical marker
`cairn.entity-construction-program.v1` includes executable RKL and the generic
`map-each-entity-point` contract, excluding proposal statistics and source data.

`EntityConstructionTrace` carries source geometry, expected render, and complete
entity/point provenance. The synthesizer validates trace closure, creates
binding examples, invokes RKL invention, and exactly replays the program.

`EntityConstructionProgramVerifier` independently type-checks raw definitions
and executes through `TypeChecker` and `ReferenceExecutor`. Its receipt commits
to program identity, experiences, and replay counts.
`EntityConstructionProgramFieldAdapter` executes the same program after field
lowering.

## 31. Unordered invention and program-memory contracts

`UnorderedMappingEpisode` carries multiple input environments and one output
set. `UnorderedMappingProblem` otherwise uses ordinary binders, return type,
literals, and limits. `invent_unordered_mapping` returns the normal admitted
outcome after exact reference and optimized replay.

`synthesize_unordered` groups entities by a unique preserved generic value,
invents coordinate definitions from unordered point sets, and jointly replays
the render. `ConstructionProgramStore` requires two disjoint verified cohorts.
`propose_construction_programs_from_memory` executes persistent programs as
untrusted proposals without v1 or expected output.

## 32. Conditional construction program

`ConditionalEntityConstructionProgram` canonically composes a
`LearnedEntitySelector` with `JointEntityConstructionProgram`. The latter has
per-axis extent definitions and one tuple-valued point definition. Training
uses `ConditionalConstructionExample`; held-out execution uses only
`RelationalGeometryScene`. `ConditionalConstructionVerifier` independently
recomputes features and executes definitions. `ConditionalConstructionStore`
requires two disjoint receipts; runtime retrieval returns proposals only.

## 33. Path and expansion contracts

`EntityFeature::TwoHopOutDegree(first, second)` counts distinct endpoints of an
exact ordered two-edge path. `LearnedExpansionMechanism` contains an admitted
constructor, bounded multiplicity, checked nested-`ForEach` theory, source
episodes, and observed cardinalities. `ExpansionMechanismVerifier` independently
reconstructs output sets. `ExpansionStore` requires two disjoint cohorts;
`propose_expansions_from_memory` returns untrusted sets only.

## 34. Group merge contracts

`GroupMergeTrace` contains finite groups and unordered outputs.
`LearnedGroupMerge` stores admitted RKL and empirical support.
`GroupMergeVerifier` independently recomputes aggregates and output sets.
`GroupMergeStore` requires two disjoint cohorts, and
`propose_group_merges_from_memory` returns untrusted proposals.

## 35. Grouping hypothesis contracts

`GroupingHypothesisTrace` maps generic `RelationRole` values to candidate finite
partitions and carries unordered outputs. `LearnedGroupingMerge` records the
selected role, learned merge, and evaluated count. Missing roles and equal-cost
explanations fail closed.

## 36. Relation-bound promotion contracts

`LearnedGroupingMerge::semantic_hash` uses marker
`cairn.grouping-merge.v1`, `RelationRole`, and canonical RKL definition bytes.
`GroupingMergeVerifier::verify_reuse` independently returns
`VerifiedGroupingReuse`; callers cannot construct its fields.
`GroupingPromotionConsolidator::consolidate` requires at least two disjoint
same-mechanism receipts with strict candidate reduction and returns
`VerifiedGroupingPromotion`.

`GroupingMergeStore` admits task-local mechanisms provisionally, records opaque
receipts, and applies only a certificate covering its complete ledger.
`CognitionRuntime::propose_grouping_merges_from_memory` returns untrusted output
sets and attempt counts. It has no answer-acceptance authority.

## 37. Raw grouping-evidence contracts

`RawGroupingEvidence` maps opaque `EntityId` values to scalar evidence and
carries unselected `RelationRecord` edges. `RawGroupingTrace` adds episode and
unordered expected outputs. `RawGroupingLimits` caps entities, edges, and roles.

`RawRelationGroupingInventor::lower` constructs weak connected components per
role with deterministic entity ordering and complete singleton coverage.
`invent` delegates only the resulting generic hypotheses to joint theory
selection. `GroupingMergeVerifier::verify_raw_reuse` separately reconstructs
the selected components. `propose_raw_grouping_merges_from_memory` exposes
proposal-only raw-edge reuse through the shared runtime.

## 38. Raw grouping cohort archive

`RawGroupingCohortArtifact` is canonical schema
`cairn.memory.raw-grouping-cohort` version 1, semantics 1.0.0. It contains
sorted `RawGroupingTrace` evidence, claimed mechanism identity, replay count,
and candidate measurements. Parent IDs and cohort task scope are mandatory.

`RawGroupingCohortArchive::persist` checks learned/receipt identity before CAS
write. `load` validates schema, kind, semantics, bounds, hash, lineage, and
scope but returns an untrusted artifact. `replay` re-invents and independently
re-verifies it, returning `ReplayedGroupingCohort`. Callers must create a fresh
store and consolidator certificate; no loaded field directly restores status.

## 39. Predicate grouping contracts

`AttributedGroupingEntity` carries opaque identity, scalar payload, and generic
integer feature vector. `AttributedGroupingTrace` adds episode and unordered
outputs. Neither type contains pair labels or relation edges.

`PredicateGroupingInventor::invent` enumerates bounded executable pair
definitions, constructs components, and induces an aggregate.
`LearnedPredicateGroupingMerge` canonically binds relation and merge RKL under
`cairn.predicate-grouping-merge.v1`. `PredicateGroupingVerifier` independently
returns `VerifiedPredicateGrouping` only after raw definition execution,
complete component reconstruction, and exact output replay.

`PredicateGroupingLimits::maximum_conjuncts` bounds canonical conjunctions of
corresponding feature-equality atoms. Enumeration terminates with `Capacity`
before exceeding `maximum_relation_candidates`. Exact candidate ranking charges
three cost units per equality atom and one per Boolean conjunction. Existing
`cairn.predicate-grouping-merge.v1` artifacts remain compatible because their
identity already contains the complete executable RKL definitions.

`VerifiedPredicateGroupingReuse` is opaque evidence for one exact cohort and a
strict candidate reduction. `PredicateGroupingPromotionConsolidator` requires
two disjoint receipts and produces `VerifiedPredicateGroupingPromotion`.
`PredicateGroupingStore` applies a token only when it covers the complete local
ledger. `CognitionRuntime::propose_predicate_groupings_from_memory` returns
untrusted unordered outputs and attempt counts; it cannot accept an answer.

`PredicateGroupingLimits::include_order_comparisons` enables bounded less-than
atoms between anonymous feature slots. It defaults to false. Directional bodies
are evaluated in both entity orders for unordered component semantics and are
canonicalized against complete left/right reversal before candidate
deduplication. Producer and verifier retain separate execution paths.

`PredicateGroupingCohortArtifact` uses schema
`cairn.memory.predicate-grouping-cohort` version 1 and semantics 1.0.0.
`PredicateGroupingCohortArchive::load` returns untrusted evidence;
`replay` returns a freshly invented mechanism and newly issued verifier receipt.
No artifact field restores `ModuleStatus` or promotion authority.

`PrimitiveObservationEntity` contains opaque identity, scalar evidence, and a
bounded vector of anonymous scalar observations. `PrimitivePairEntity` remains
a source alias, not a fixed-arity semantic contract. Every task has one checked
arity. `LearnedObservationPredicateGrouping` binds that arity and a bounded
variable-length vector of executable feature `Definition` values to a
predicate/aggregate mechanism. Its v3 semantic identity includes the arity,
complete ordered definition vector, and downstream mechanism.
`ObservationPredicateVerifier` independently lowers current primitives.
`SequenceAdapter::lower_span_observations` and
`VectorAdapter::lower_segment_observations` emit the same envelope without
normalizing endpoints or assigning relation semantics.

`ObservationPredicateLimits` independently caps entities, observation arity,
observation permutations, exposed features, feature expression cost, typed
candidates, and feature sets. Search constructs
integer RKL from variables, negation, minimum, maximum, addition, subtraction,
and multiplication. `feature_candidates_evaluated` and
`feature_sets_evaluated` report producer work. `ObservationPredicateStore`
requires two disjoint `VerifiedObservationPredicateReuse` receipts plus a
complete-ledger `VerifiedObservationPredicatePromotion`; persistent entries
remain proposals and require current-evidence replay.
The verifier independently enumerates every observation-index permutation under
the declared cap and replays the complete mechanism. Reuse and promotion
certificates report tested arity, entity-count range, and scalar range; only
arity is a hard runtime applicability constraint. It derives fresh candidate
work from the learned reports;
the receipt API does not accept caller-authored reduction metrics.

`ObservationPredicateCohortArtifact` uses schema
`cairn.memory.observation-predicate-cohort` version 2 and semantics 3.0.0. It
contains evidence and commitments only. `ObservationPredicateCohortArchive`
re-invents and re-verifies after restart; no module status, executable learned
object, receipt, or certificate is deserialized. `CognitionRuntime` owns the
re-promoted store and emits proposals only.

`RawEntityObservationEvidence` contains opaque scalar primitives and unselected
binary `RelationRecord` candidates. `EntityObservationInventor` enumerates one
connected-component interpretation per role and bounded cross-episode binding
sets. `LearnedEntityObservationGrouping::selected_relations` is task-local
lineage; `mechanism_hash` excludes those source names and identifies only the
executable observation-feature/predicate/aggregate composition.

`EntityObservationVerifier` independently reconstructs components through a
disjoint-set reference path before ordinary observation-mechanism replay.
`CognitionRuntime::propose_entity_observations_from_memory` tries each current
role against structurally compatible persistent modules and returns mechanism,
binding, output, and work counts as proposals only. `SequenceAdapter` and
`VectorAdapter` lower frontend primitives to this same evidence contract without
selecting groups.

`RawRelationObservationTrace` contains opaque attributed primitives and complete
outputs but no `RelationRecord` values or entity groups.
`RawRelationObservationInventor` constructs a typed RKL equality definition,
enumerates bounded task-local attribute bindings, materializes candidate graphs,
and invokes explanation-driven entity/mechanism induction.
`LearnedRawRelationObservationGrouping::mechanism_hash` binds canonical relation
RKL and the downstream executable mechanism while excluding episode attribute
indexes and generated roles.

`RawRelationObservationVerifier` independently type-checks and executes relation
RKL against raw attributes, creates verifier-local edges, and invokes the
disjoint-set entity verifier.
`CognitionRuntime::propose_raw_relation_observations_from_memory` tries every
bounded current attribute binding against proof-gated persistent observation
mechanisms and returns untrusted proposals plus work counts. Actual sequence and
vector adapters expose unrelated primitives without constructing relations.

`RelationKeyOperator` is the source-independent key-program node set:
`Projection`, `Add`, `Subtract`, `Minimum`, `Maximum`, `Sum`, `MinimumFold`,
and `MaximumFold`.
`RelationAttributeBinding` binds operator arguments to current anonymous
attribute positions and is task-local. `RawRelationMechanism` stores relation
RKL, operator, key arity, and the downstream executable mechanism under semantic
identity marker `cairn.raw-relation-observation.v2`; all four enter identity.

`RawRelationObservationVerifier::verify_reuse` returns opaque exact-replay and
binding-reduction evidence. `RawRelationObservationPromotionConsolidator`
requires two disjoint receipts and returns a complete-ledger promotion.
`RawRelationStore` stores the full mechanism as a proposal source.
`CognitionRuntime::propose_raw_relation_programs_from_memory` filters the
current candidate grammar by the learned operator, then rebinds and executes it;
it has no acceptance authority.

`RawRelationCohortArtifact` uses schema
`cairn.memory.raw-relation-cohort`, version 1, semantics 2.0.0. The archive
persists evidence and commitments only. `load` is untrusted and `replay`
performs fresh invention plus independent verification. Status, receipts,
certificates, and learned native objects are not serialized.

`RelationObservationLimits::maximum_key_arity` bounds variable-arity key
enumeration and defaults to two. `enabled_associative_folds` is a separate
`RelationFoldMask` and defaults to empty; raising arity alone cannot expose a
fold. `Sum`, `MinimumFold`, and `MaximumFold` accept canonical attribute
combinations of arity three through the cap and lower to typed left-associated
RKL folds. `RawRelationMechanism::key_arity` and
`RawRelationSupportGeometry::key_arity` are mandatory applicability fields.
Verifier receipts derive their arity from the learned definition and selected
bindings; runtime retrieval matches both operator and arity before proposal.

Developmental semantics 1.3.0 propagates
`minimum_dependency_arity_lower_bound` through
`GeneratedDevelopmentalObjective` and `DevelopmentalExerciseRequest`. The value
already participates in the content-free failure commitment and exposes no task
content. `GeneratedDevelopmentalObjective::relation_language_growth_request`
also issues an opaque `RelationLanguageGrowthRequest` binding the commitment to
that arity; external callers cannot construct it. The ordinary exercise request
remains a materializer message and grants no language authority.

`RelationLanguageExtensionConsolidator::consolidate` accepts a binary
`RawRelationMechanism`, its opaque
`VerifiedRawRelationObservationPromotion`, the opaque growth request, and
`RelationLanguageExtensionLimits`. It returns an opaque
`VerifiedRelationLanguageExtension` only for the corresponding associative fold
under the declared cap. `apply` modifies proposal limits only. It cannot persist
a mechanism or verify an answer, and no extension certificate is restored from
the evidence archive.

`RelationKeyExpression` is a source-independent AST over `Parameter` and
`Binary` nodes. `RelationExpressionBinding` maps its logical parameters to
current anonymous attributes and is excluded from semantic identity.
`RelationExpressionMechanism` binds the expression, exact RKL definition, and
downstream executable observation mechanism under marker
`cairn.relation-expression.v1`.

`RelationExpressionLanguageConsolidator` consumes at least two pairs of
`RawRelationMechanism` and opaque promotion certificates plus an opaque
`RelationLanguageGrowthRequest`. Its opaque
`VerifiedRelationExpressionLanguage` exposes only the promoted operator set,
requested arity, binary-node bound, failure commitment, and source mechanism
identities. Applying it changes `RelationExpressionLimits` proposal
reachability; it cannot supply an expression or accept output.

`RelationExpressionInventor` constructs bounded left-associated ASTs, binds
current attributes, and uses complete downstream explanation.
`RelationExpressionVerifier` reconstructs RKL and relation graphs through a
separate path. `VerifiedRelationExpressionReuse` derives structure work from
the certified language. `RelationExpressionPromotionConsolidator` requires two
disjoint same-mechanism receipts and returns
`VerifiedRelationExpressionPromotion`. `propose` accepts answer-hidden current
evidence and emits untrusted binding/output alternatives only.
