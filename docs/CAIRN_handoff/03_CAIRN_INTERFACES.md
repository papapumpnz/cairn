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

pub struct PairEventCover {
    pub pair: PairId,
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
    pub pair: PairId,
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
    SetState(SetStateEffect),
}

pub struct Theory {
    pub version: TheoryVersion,
    pub definitions: Vec<Definition>,
    pub rules: Vec<RewriteRule>,
    pub schedule: Schedule,
    pub renderer: Renderer,
    pub assumptions: Vec<Assumption>,
}
```

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
    pub mismatch_region: CellSet,
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
