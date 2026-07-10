# CAIRN master design

**Causal Analogical Induction over Relational Networks**

Version: 1.0 architecture handoff

Status: implementable research design; no benchmark result is claimed by this document

## 0. Executive decision

CAIRN is a CPU-only, transformer-free architecture whose central inference object is a **causal rewrite theory**, not an action sequence, a fixed ARC program, or a vector prediction.

For a static induction problem, the system must explain demonstrations with a compact set of quantified graph-rewrite rules. It discovers the rules by aligning observed events, solving unknown predicates and constructors, executing the resulting theory, and using exact mismatches as counterexamples. Perception is induced jointly with the theory: an entity, motif, region, symbol, or track exists in the working model only because its definition helps explain observations. ARC grid pairs are one adapter representation of this generic problem.

For an interactive task, the same theory language models state transitions, observations, action effects, hidden variables, and goals. The system maintains a posterior over possible world theories, chooses interventions by expected information gain plus goal value, and plans through the surviving theories.

The core design is summarized by this loop:

```text
observe
build relational evidence
propose event correspondences
weave a minimal causal theory
execute
refute with exact counterexamples
invent missing relations or constructors when needed
compress and verify
predict or plan
```

The architecture deliberately avoids two failed patterns:

```text
SCEM: learn reusable surface programs from previously solved traces
MOSAIC: enumerate a fixed high-level DSL over precomputed parse worlds
```

CAIRN instead uses a small universal relational kernel and induces high-level semantics as task-local, proof-carrying definitions.

### 0.1 General-learning claim boundary

CAIRN is not considered a general learner merely because it can represent or
search for programs. General learning requires measured evidence from three
co-developed loops:

```text
task learning: infer a previously unstored theory
continual learning: verified experience improves later learning
interactive causal learning: learn unknown dynamics, goals, and skills online
```

ARC is one quarantined adapter and benchmark. It may exercise exact induction,
but it does not define core APIs, development order, or the general-learning
claim. The six mandatory gates are specified in
`08_GENERAL_LEARNER_REQUIREMENTS.md`.

## 1. Requirements

### 1.1 Hard constraints

- CPU only for training, inference, evaluation, and swarm execution.
- No transformer architecture or transformer dependency.
- No GPU runtime, GPU kernel, GPU library, remote GPU service, or GPU-generated answer path.
- Deterministic replay for all exact-domain claims.
- Commodity baseline: 8 physical CPU cores, 16 GB RAM.
- Static-task solver is anytime and must return at most two semantically distinct attempts.
- Interactive solver maintains bounded compressed state rather than raw unbounded frame history.
- All persisted learned mechanisms are proposal sources only; correctness is established by current-task evidence and an independent verifier.
- Evaluation tasks never enter training, debugging, prompt construction, primitive design, or search-policy tuning.

### 1.2 Capability requirements

The static solver must natively express and induce:

- identity and global geometric transforms;
- recoloring by literal, role, relation, or symbol;
- object selection by relational context;
- variable-arity mapping over entities;
- creation, deletion, movement, copying, merging, splitting, and replacement;
- crop, padding, scaling, tiling, panel extraction, and canvas construction;
- multicolor shape construction;
- rules that interact through dependency ordering;
- parallel, sequential, conditional, recursive, and bounded-fixpoint behavior;
- symbolic control, legends, keys, and operator-valued symbols;
- output dimensions and coordinates derived from input structure;
- ambiguity management and pass@2 diversity.

The interactive solver must add:

- action-conditioned transition induction;
- hidden-state and partial-observation handling;
- online goal inference;
- active experimentation;
- long-horizon planning;
- option or skill induction with learned termination conditions;
- memory compression and retrieval.

The continual learner must add:

- proof-backed mechanism promotion and deprecation;
- explicit support geometry and unsupported-axis uncertainty;
- fixed-budget memory-on versus memory-off improvement;
- skill and mechanism composition without coded composite operators;
- negative-memory reuse without making valid candidates unreachable;
- knowledge claims with source, time, contradictions, and provenance.

The same cognition runtime must operate on finite-field transformations,
interactive worlds, and vector graphics or diagrams before a general-learning
claim is permitted.

### 1.3 Performance requirements

Engineering targets, to be validated rather than assumed:

- grid kernel throughput: at least 20 million primitive cell or bitset operations per core per second;
- static perception and evidence construction median: below 50 ms per task;
- first valid theory median: below 2 s on ARC-AGI-1 training after optimization;
- full static solve median: below 8 s, p95 below 60 s, hard cap 120 s on the baseline machine;
- common interactive state update: below 100 ms;
- ordinary interactive action choice: below 500 ms, with explicit longer deliberation only when action timing permits;
- memory below 2 GB per static task and below 4 GB per interactive episode on the baseline machine;
- local 8-worker speedup target: at least 4.5 times over one worker on the theory-search portion.

These are gates in the experiment plan, not reported achievements.

## 2. Research position and architectural choice

### 2.1 What current ARC suites require

ARC-AGI-2 explicitly emphasizes symbolic interpretation, compositional reasoning, and contextual rule application. A solver therefore needs more than geometric operator enumeration. It must bind visual symbols to semantics, infer multiple interacting rules, and condition behavior on relationships rather than surface appearance.

ARC-AGI-3 changes the task from static induction to online skill acquisition. It requires exploration, goal inference, world-model construction, planning, and efficient use of interaction history. A static puzzle harness with game-specific procedures is not an acceptable general solution.

### 2.2 Useful prior techniques

CAIRN adopts ideas, not architectures, from several established lines of work:

- object-centric graph abstraction and constraint acquisition;
- generalized planning with induced programs;
- relational analogy and systematic structure mapping;
- anti-unification for extracting shared relational form;
- counterexample-guided inductive synthesis;
- learning from failures, where failed hypotheses generate pruning constraints;
- equality saturation for compactly maintaining equivalent expressions;
- active causal learning with expected information gain.

Each technique addresses one layer. None alone resolves the complete problem. CAIRN combines them around a different central object: a jointly induced perceptual and causal rewrite theory.

### 2.3 Rejected alternatives

#### Larger fixed ARC DSL

Rejected because MOSAIC's measured frontier was inexpressible, not compute-limited. Expanding a catalog of operations creates another closure boundary and encourages task-shaped patches.

#### Pure brute-force universal program search

Rejected because a universal language without evidence-derived sketches is computationally unusable. Universality is needed for closure; blind enumeration is not the search algorithm.

#### Stored macro or concept library as the main intelligence

Rejected because SCEM measured near-zero evaluation transfer from three learned artifact classes. Stored mechanisms remain useful only as hypotheses that must be rebound and verified.

#### End-to-end non-transformer neural prediction

Rejected as the primary path because it would still need large training distributions, would not naturally produce exact causal certificates, and would make CPU-only test-time adaptation expensive. Small non-transformer statistical models are permitted for noisy frontends or search ordering, never as the sole static-task reasoner.

#### Evolutionary program search

Rejected as the core because it spends large budgets rediscovering local syntax, has weak failure-derived pruning, and makes exact attribution difficult. Evolutionary search may be used as a bounded secondary proposer for derived features.

## 3. Core hypothesis

CAIRN's central hypothesis is:

> Few-shot abstract tasks become tractable when inference begins with the observed causal event structure and searches only for theories that can account for those events, while allowing the system to invent task-local relational definitions inside a universal executable kernel.

This decomposes into six testable claims.

### H1: Event correspondence is a stronger search prior than program syntax

The input-output difference constrains which entities were preserved, transformed, consumed, or created. Constructing a small set of plausible event covers should reduce search more than forward enumeration by program size.

### H2: Perception should be justified by explanation

An entity definition that makes a transformation simple is preferable to a segmentation selected without reference to the output. Joint abduction should recover useful multicolor objects, panels, motifs, and roles that a fixed parse lattice misses.

### H3: A theory of multiple quantified rules is the correct unit of composition

Many tasks are not one program chain. They are a set of context-dependent rules with explicit dependencies, concurrency, and rendering conflicts. Inducing the dependency graph should handle composition more naturally than increasing term depth.

### H4: Dynamic derived definitions can remove the fixed-DSL ceiling

The kernel remains fixed and universal, but the search vocabulary can expand during a task. New predicates, entity definitions, operator-valued symbol maps, constructors, and recursive abbreviations are accepted only when they have executable kernel definitions and improve exact explanatory compression.

### H5: Counterexamples should change both search constraints and ontology

A failed candidate is not merely discarded. Its mismatch cells, failed bindings, and unsatisfied explanation core generate constraints. Repeated failures that cannot be separated by existing features trigger invention of a new relation or role.

### H6: The same theory representation can support static induction and interactive intelligence

A static input-output transformation is a one-step transition. An interactive world is a sequence of such transitions conditioned on actions. The same rewrite theory, belief manager, and planner should therefore extend to ARC-AGI-3 and other modalities.

## 4. System overview

```mermaid
flowchart TD
    O[Observations] --> F[Frontend normalization]
    F --> E[Relational evidence store]
    E --> C[Correspondence and event-cover agents]
    C --> A[Analogical role alignment]
    A --> S[Theory sketch construction]
    S --> Q[Incremental constraint solver]
    Q --> T[Candidate causal rewrite theories]
    T --> X[Exact executor]
    X --> V[Independent verifier]
    V -->|mismatch or ambiguity| R[Refutation and unsat-core analysis]
    R --> I[Predicate, role, constructor, or schedule invention]
    I --> S
    V -->|verified| B[Theory belief set and arbiter]
    B --> P[Prediction or planner]
    P --> M[Proof-carrying memory]
    M --> S
```

### 4.1 Major subsystems

1. **Frontend normalization** converts grids, frames, audio events, vector graphics, text facts, or environment records into observations with explicit coordinates, channels, and uncertainty.
2. **Relational evidence store** lazily materializes cells, sets, entities, motifs, relations, and temporal events.
3. **Event-cover engine** builds alternative causal explanations for each observed transition.
4. **Analogical aligner** maps roles and relations across demonstrations while ignoring irrelevant appearance differences.
5. **Theory weaver** anti-unifies aligned events into rule sketches with typed holes.
6. **Constraint synthesizer** jointly solves selectors, constructors, symbol meanings, dependencies, and output rendering.
7. **Executor and verifier** produce exact outputs and proof certificates.
8. **Refuter and inventor** learn constraints from failure and create new task-local definitions when the current ontology is inadequate.
9. **Belief manager and arbiter** preserve distinct explanatory modes and choose two diverse attempts.
10. **Temporal model and planner** extend the theory to action-conditioned sequences.
11. **Memory system** stores proof-backed mechanisms, counterexamples, and compiled definitions.
12. **Swarm runtime** coordinates heterogeneous agents through immutable artifacts.

### 4.2 No privileged monolithic controller

The coordinator schedules jobs and maintains budgets. It does not contain domain logic. Every substantive proposal is an artifact that can be inspected, replayed, refuted, and reproduced by another process.

## 5. Foundational representation

### 5.1 Observation

An observation is a finite typed field plus metadata:

```rust
pub struct Observation {
    pub id: ObservationId,
    pub modality: Modality,
    pub axes: Vec<AxisSpec>,
    pub channels: Vec<ChannelSpec>,
    pub values: ValueStore,
    pub uncertainty: Option<UncertaintyStore>,
    pub timestamp: Option<LogicalTime>,
}
```

For ARC-AGI-1/2, `values` is a 2D color grid. For ARC-AGI-3, it is a frame plus action and episode metadata. Frontends may attach uncertain detections, but the raw observation always remains available.

### 5.2 Relational Evidence Graph

The **Relational Evidence Graph**, abbreviated REG, is a typed attributed hypergraph.

Nodes can represent:

- cells;
- coordinates;
- colors or channel values;
- cell sets;
- candidate entities;
- boundaries, contours, holes, axes, and regions;
- panels, motifs, repetitions, and symbol tokens;
- actions, states, observations, rewards, and temporal events;
- latent roles and derived definitions.

Hyperedges represent typed relations such as:

- membership;
- adjacency and connectivity;
- containment and overlap;
- relative direction and displacement;
- alignment, symmetry, congruence, and similarity;
- count, rank, and uniqueness;
- source-to-target correspondence;
- causal production;
- temporal precedence and action conditioning.

```rust
pub struct RelGraph {
    pub nodes: NodeArena,
    pub edges: EdgeArena,
    pub indexes: RelationIndexes,
    pub derivations: DerivationArena,
}

pub struct HyperEdge {
    pub relation: RelationId,
    pub args: SmallVec<[NodeId; 4]>,
    pub attrs: AttrMap,
    pub confidence: EvidenceWeight,
    pub derivation: DerivationId,
}
```

The implementation is structure-of-arrays and index-driven. It is not a general database. Common relations have specialized bitset indexes.

### 5.3 Entity algebra

CAIRN does not enumerate complete parse worlds. It defines entities lazily through an **entity algebra**.

Representative constructors:

```text
CellsWhere(predicate)
Connected(seed_or_mask, connectivity)
ColorComponent(color_role, connectivity)
MulticolorComponent(boundary_rule, connectivity)
Run(axis, predicate)
Contour(entity)
Holes(entity)
BoundingBox(entity)
RectangleFrom(corners_or_extent)
SymmetryOrbit(entity, group)
RepeatGroup(seed, displacement_relation)
Panel(region_or_separator_rule)
Motif(equivalence_relation)
DifferenceRegion(before, after)
Track(entity_across_time)
```

An entity expression has a cost, a denotation, and a derivation. Equivalent denotations are merged in a typed e-graph. Entity definitions are requested by theory holes and event explanations rather than generated as a Cartesian product of segmentation settings.

### 5.4 Roles

A role is a variable whose meaning is relational, not a fixed label.

Examples:

```text
source_object
selected_object
reference_object
anchor
marker
payload
delimiter
legend_key
legend_value
control_symbol
target_region
obstacle
goal
agent
```

A role hypothesis is defined by a clause over entities and relations. The same role can be bound to different colors, shapes, sizes, or positions in different demonstrations.

### 5.5 Provenance is mandatory

Every derived entity, event, output cell, learned definition, and memory item carries a derivation DAG. Provenance distinguishes:

- observed fact;
- deterministic derived fact;
- hypothesis;
- selected model choice;
- external or uncertain frontend evidence;
- persisted prior.

No output cell may be marked verified without a chain from current observations through executable theory steps.

## 6. Relational Kernel Language

### 6.1 Purpose

The **Relational Kernel Language**, RKL, is the fixed computational substrate. It is intentionally lower-level and more general than an ARC DSL. It supplies finite sets, relations, bounded arithmetic, coordinate geometry, graph matching, rendering, conditionals, and bounded iteration.

High-level operations such as “copy each enclosed object to its matching panel” are not kernel primitives. They are induced definitions over RKL.

### 6.2 Type system

```text
Bool
Int[lo, hi]
Atom
Coord2
Vector2
Axis
Transform2
CellSet
Shape
Entity
EntitySet
Symbol
Action
State
FiniteField<axes, max-elements>
Relation<A...>
Option<T>
List<T, max>
Map<K, V, max>
Operator<A, B>
```

Ranges and collection bounds are explicit. Finite-field axis meanings and atom
interpretations belong to adapters, not RKL. Static programs are total under
observation-derived bounds. Interactive programs consume a fuel budget and
expose any nontermination as verification failure.

### 6.3 Expression forms

```text
Literal
Variable
Tuple
Let
If
Match
Call
RelationQuery
SetComprehension
Map
Filter
Fold
Count
Any
All
Unique
ArgMin
ArgMax
Rank
GroupBy
Join
TransitiveClosure with bound
BoundedFixpoint
```

Geometry and rendering are expressed through verified intrinsics that are themselves specified against reference semantics:

```text
translate
apply_affine_d4
scale_integer
crop
pad
rasterize
outline
fill
line_segment
ray
bbox
centroid_exact_or_rational
project_to_axis
```

### 6.4 Theory structure

A theory contains definitions, rules, a schedule, and a renderer.

```rust
pub struct Theory {
    pub inputs: Vec<Binder>,
    pub definitions: Vec<Definition>,
    pub rules: Vec<RewriteRule>,
    pub schedule: Schedule,
    pub renderer: Renderer,
    pub assumptions: Vec<Assumption>,
    pub score: TheoryScore,
}

pub struct RewriteRule {
    pub binders: Vec<Binder>,
    pub guard: PredicateExpr,
    pub effects: Vec<Effect>,
    pub priority: PriorityExpr,
}
```

Theory inputs are typed generic observation/request bindings. They are in scope
for rule domains and guards, renderer expressions, and assumptions. Adapter
types never enter this declaration.

Rule binders support one entity, tuples, groups, variable-arity sets, and operator-valued symbols.

Effects include:

```text
Preserve
Delete
UpdateAttribute
Move
Copy
Transform
Split
Merge
EmitShape
Emit
Paint
Erase
CreateRelation
SetLatentState
Write
EmitRewardHypothesis
```

All convenience effects lower to a small cell/entity mutation calculus before execution.

### 6.5 Schedule semantics

```rust
pub enum Schedule {
    Parallel,
    Sequence(Vec<RuleId>),
    DependencyDag(DependencyDag),
    DecisionList(Vec<RuleId>),
    BoundedFixpoint { rules: Vec<RuleId>, max_steps: BoundExpr },
    Hierarchical(Vec<Schedule>),
}
```

The schedule is inferred. It is not assumed that all rules run once or that a task is a fixed-depth chain.

### 6.6 Renderer

The renderer makes canvas construction explicit:

```rust
pub struct Renderer {
    pub canvas: CanvasExpr,
    pub layers: Vec<LayerExpr>,
    pub conflict_policy: ConflictPolicy,
    pub background: Expr,
}
```

Canvas expressions may derive dimensions from input, selected panels, entity extents, counts, repetition periods, or rule outputs. Every output write records its source and layer. Conflicting writes are either ruled out by constraints or resolved by an explicit inferred policy.

### 6.7 Expressive closure

RKL is designed to express any bounded finite observation transformation by
quantifying over finite domains and emitting values, entities, relations, or
state effects. Spatial adapters may lower a field to coordinates and elements,
but those axes are not RKL semantics. Literal member-by-member solutions carry
prohibitive description cost and cannot generalize; they exist only to establish
substrate completeness and diagnose search failures.

The practical search language is the set of low-cost definitions and theory sketches induced from evidence. Dynamic invention changes that language without adding opaque, unverifiable semantics.

## 7. Demonstration inference: evidence to event covers

### 7.1 Transition framing

Each demonstration is treated as one observed transition:

```text
context O_i  --unknown mechanism/intervention-->  outcome O'_i
```

The first inference problem is not “which program generated H_i?” It is:

> which minimal set of causal events could account for every preserved, changed, deleted, and created entity, relation, or value in the outcome relative to its context?

### 7.2 Candidate entity demand

The event engine asks the entity algebra for candidate source and target entities only when they can participate in a low-cost explanation. Demand sources include:

- unchanged relation-defined groups;
- maximal mismatch evidence;
- repeated structures;
- candidate containers;
- entities whose categorical, structural, or topological evidence matches across context and outcome;
- entities related by low-complexity transforms supplied as evidence;
- symbols whose variation correlates with an effect;
- unmatched output regions requiring construction.

This keeps perception coupled to explanatory need.

### 7.3 Correspondence hyperedges

A correspondence can map zero, one, or many source entities to zero, one, or many target entities.

```rust
pub enum EventKind {
    Preserve,
    Update,
    Move,
    Transform,
    Recolor,
    Copy,
    Delete,
    Create,
    Split,
    Merge,
    Aggregate,
    Distribute,
    RenderPanel,
    InterpretSymbol,
}

pub struct EventHypothesis {
    pub kind: EventKind,
    pub sources: SmallVec<[EntityId; 4]>,
    pub targets: SmallVec<[EntityId; 4]>,
    pub parameter_sketch: ExprSketch,
    pub required_relations: Vec<RelationLiteral>,
    pub covered_input_cells: CellSetId,
    pub covered_output_cells: CellSetId,
    pub cost: Cost,
    pub provenance: DerivationId,
}
```

Events may overlap only when the renderer permits layered writes. Candidate generation uses invariant signatures and bounded matching:

- exact shape and color signature;
- shape under D4 transform;
- scaled or tiled signatures;
- color-normalized shape;
- topology and contour signatures;
- relative-coordinate signatures;
- multiset signatures for groups;
- geometric construction signatures for lines, rectangles, fills, and repetition.

### 7.4 Event-cover optimization

For each pair, choose a set of events that explains the transition.

Hard constraints:

- every output cell is produced exactly once unless an explicit layer relation permits multiple writes;
- every changed input cell is consumed, updated, hidden by a justified layer, or declared irrelevant by canvas selection;
- event parameters are internally consistent;
- source and target membership constraints hold;
- the rendered result equals the output exactly.

Objective, lexicographically:

1. exact coverage;
2. minimum unexplained structure;
3. minimum event description length;
4. maximum reuse of the same event form;
5. minimum literal constants;
6. minimum entity-definition cost.

The engine keeps the top `K_cover` structurally distinct covers, not only the cheapest cover. Structural distinction is based on event-kind multiset, source-target arity, and causal dependency shape.

### 7.5 Pair-local counterexamples

When no exact cover exists, the solver emits a typed explanation core rather than `inexpressible` alone:

```text
unmatched_target_region
source_correspondence_missing
constructor_missing
entity_definition_missing
color_function_missing
coordinate_expression_missing
layer_conflict
variable_arity_required
merge_or_split_required
symbol_semantics_required
schedule_required
```

The core is the input to ontology expansion in Section 11.

## 8. Cross-example analogical alignment

### 8.1 Why correspondence must precede rule search

Demonstrations normally instantiate the same abstract rule with different colors, counts, sizes, and positions. A syntactic program search repeatedly rediscovers those bindings. CAIRN first maps the relational roles across examples.

### 8.2 Structural signatures

Each event participant receives a signature containing:

- intrinsic attributes: area, shape, colors, topology;
- relational neighborhood: containment, alignment, adjacency, rank, uniqueness;
- causal role in the pair cover;
- relations to unchanged context;
- relations to control symbols or panels;
- source-target transform parameters expressed relative to other entities.

Appearance features are marked as either potentially semantic or potentially incidental. No color is automatically treated as a label or as a literal.

### 8.3 Structure mapping

The analogical aligner builds candidate maps between pair event graphs under these preferences:

1. preserve relation structure;
2. preserve causal role;
3. prefer one-to-one mappings unless the event arity requires a group;
4. prefer systematic connected mappings over isolated feature matches;
5. allow intrinsic attributes to differ when a common relational role explains them;
6. reject mappings that force incompatible event semantics.

The implementation uses a beam search over consistent partial mappings with upper bounds from unmatched relation weight. Small subgraphs may use exact maximum common subgraph or VF2-style matching.

### 8.4 Anti-unification

Aligned event instances are generalized into a `RuleSketch`.

Example concrete instances:

```text
pair 1: copy blue 2x2 object right of red marker to the marker's left side
pair 2: copy green L-shape below yellow marker to the marker's upper side
```

Possible anti-unified sketch:

```text
bind marker M by relational role
bind payload P by relation adjacent_in_direction(P, M, D)
emit transform(P, T) at opposite_side(M, D)
```

The unknowns are typed holes:

```text
selector(M)
selector(P)
relation(P, M, D)
operator T
placement expression
paint function
```

Anti-unification generalizes constants only when a variable can be grounded by current observations. A value that never varies is retained as a competing literal and role-derived alternative; it is not automatically fixed or generalized.

### 8.5 Alignment uncertainty

The top `K_align` alignments remain separate belief modes. They are not merged into one inconsistent sketch. Later exact execution, leave-one-example-out tests, and description length decide between them.

## 9. Theory weaving

### 9.1 Rule-sketch inventory

Aligned event clusters yield candidate rule sketches. The theory weaver must decide:

- number of rules;
- which event instances belong to each rule;
- binders and quantifiers;
- selector and context clauses;
- constructors and attribute functions;
- symbol mappings;
- rule dependencies;
- renderer and canvas;
- whether a repeated event is parallel mapping, sequence, or fixpoint.

### 9.2 Positive and negative examples for a rule

For a candidate event cluster:

- positive bindings are entities or groups that underwent the event;
- negative bindings are comparable entities that remained unchanged or underwent a different event;
- hard negatives come from the same relational context whenever possible;
- cross-pair negatives prevent a selector from exploiting one pair's accidental feature.

This converts contextual rule application into relational concept induction with explicit negatives.

### 9.3 Predicate language

Guard literals are generated in cost tiers.

Tier 0:

```text
type and modality
color role equality
area, width, height, count
boundary contact
connectedness
containment
```

Tier 1:

```text
relative direction
overlap
alignment
same shape under transform
rank by attribute
unique argmin or argmax
same or different palette role
```

Tier 2:

```text
relation paths of length two or three
count of related entities
parity and modular relations
symmetry orbit membership
panel correspondence
nearest or farthest under graph distance
```

Tier 3:

```text
derived predicates invented from contrast
latent roles linked to symbols
recursive reachability
higher-order set relations
```

Predicates are clauses over variables. The default guard form is a conjunction; decision lists and small disjunctions are introduced only when exact evidence requires them.

### 9.4 Constructor language

A constructor is inferred from the target's provenance and can produce cells, shapes, entities, groups, or layers.

Candidate expression families include:

```text
copy or transform a source entity
recolor by constant, role, map, rank, or symbol
place by source-relative displacement
place relative to a reference entity
place on a panel-local coordinate system
span line or rectangle between derived anchors
repeat along a derived vector until a boundary or relation
scale by count or extent ratio
compose parts from multiple sources
render a set comprehension
```

Coordinate formulas use bounded affine arithmetic, min/max, extents, ranks, counts, and relation-derived vectors. Nonlinear formulas are introduced only through verified derived definitions.

### 9.5 Symbolic interpretation

CAIRN represents a visual symbol as an entity signature plus relational context. Its meaning may be a value or an operator.

```rust
pub enum SymbolMeaning {
    Atom(AtomExpr),
    Direction(DirectionExpr),
    Transform(TransformExpr),
    Predicate(PredicateExpr),
    Constructor(ConstructorExpr),
    ScheduleChoice(ScheduleExpr),
    RoleBinding(RoleExpr),
}
```

A symbol map is synthesized jointly with the rule it controls. Candidate evidence includes:

- a small object or panel changes while the main effect changes;
- a legend pairs tokens with exemplars;
- spatial orientation of a token corresponds to a transform direction;
- token color selects a target color or object role;
- a token is consumed as instruction rather than rendered.

The solver permits bijective, many-to-one, compositional, and structurally derived maps. A symbol's pixel appearance is not assumed to directly encode its semantics.

### 9.6 Compositional scheduling

Rules are connected by data and write dependencies:

- rule B reads an entity created by rule A;
- rule B selects cells changed by rule A;
- two rules write overlapping cells;
- one rule creates the canvas used by another;
- an event repeats until no new match exists.

The scheduler infers the smallest dependency structure consistent with all demonstrations. Independent rules run in parallel. Cycles require a bounded fixpoint with a monotonicity or fuel proof.

### 9.7 Joint constraint model

The preferred backend is incremental SAT or SMT with finite-domain encodings. Variables include:

```text
choose event cover per pair
choose alignment
assign event instance to rule
choose binder arity and quantifier
choose guard literals
choose constructor expression from an e-class
choose symbol meaning
choose rule dependency edges
choose canvas and layer policy
```

Hard constraints enforce exact execution semantics for the current finite examples. Soft clauses encode theory cost. Solving is lexicographic or weighted MaxSAT with exact rechecking by the executor.

The solver backend is an optimization aid, not the semantic authority. Every candidate is independently executed.

## 10. Counterexample-guided theory refinement

### 10.1 Main CEGIS loop

```rust
pub fn solve_problem(problem: &LearningProblem, budget: Budget) -> SolveOutcome {
    let evidence = build_initial_evidence(problem);
    let mut frontier = seed_theory_sketches(&evidence);
    let mut constraints = ConstraintState::new(problem);
    let mut beliefs = TheoryBeliefSet::new();

    while let Some(job) = frontier.next(budget.remaining()) {
        let candidate = synthesize_candidate(job, &constraints, &evidence)?;
        let replay = execute_on_all_demonstrations(&candidate, problem);

        match replay {
            Replay::Exact(certificate) => {
                let tests = falsify_candidate(&candidate, problem, &evidence);
                beliefs.insert(candidate, certificate, tests);
            }
            Replay::Mismatch(counterexample) => {
                constraints.learn_from(counterexample.clone());
                frontier.refine(counterexample.clone());

                if counterexample.indicates_missing_ontology() {
                    frontier.extend(invent_definitions(counterexample, &evidence));
                }
            }
        }

        if beliefs.has_stable_two_attempt_answer() || budget.exhausted() {
            break;
        }
    }

    arbitrate_and_emit(beliefs, problem)
}
```

### 10.2 Counterexample forms

A counterexample is structured:

```rust
pub struct Counterexample {
    pub theory: TheoryId,
    pub demonstration: DemonstrationId,
    pub mismatch: EvidenceSlice,
    pub missing: EvidenceSlice,
    pub extra: EvidenceSlice,
    pub wrong_values: Vec<ValueMismatch>,
    pub failed_bindings: Vec<BindingFailure>,
    pub conflicting_rules: Vec<RuleConflict>,
    pub unsatisfied_core: ExplanationCore,
}
```

### 10.3 Learning from failures

Failure-derived constraints include:

- selector too general: prune guards weaker than the failed guard;
- selector too specific: prune strict specializations that exclude a required positive;
- constructor family impossible: block all expressions in the same semantic e-class;
- literal constant invalid across pairs: require a grounded role-derived expression;
- wrong schedule: block dependency orientations that recreate the conflict;
- missing variable arity: require a set binder;
- symbol map collision: add disequality or structural mapping constraints;
- renderer conflict: require layer ordering or disjoint writes.

Constraints are attached to canonical semantic patterns so equivalent syntax is pruned together.

### 10.4 Unrealizability within a sketch

The solver must distinguish:

```text
candidate not yet found
current sketch proven impossible under present definitions
present ontology insufficient
budget exhausted
```

A proof that a sketch is unrealizable is a valuable artifact. It prevents agents from repeating the same search and is a trigger for controlled invention.

## 11. Dynamic ontology and primitive invention

### 11.1 Objective

CAIRN must expand its practical hypothesis language without adding task-shaped opaque code. Every invented item is a definition in RKL with an exact denotation.

Inventable items:

- entity definition;
- relational predicate;
- role definition;
- numeric or geometric feature;
- constructor;
- operator-valued symbol map;
- recursive or fixpoint abbreviation;
- temporally extended option.

### 11.2 Triggers

Invention is triggered when one of these conditions holds:

1. positive and hard-negative bindings are indistinguishable under current predicates;
2. unmatched output structure repeats across pairs but no current constructor explains it;
3. the same multi-step derivation appears repeatedly and exceeds the search cost limit;
4. an unsat core contains incompatible literals that suggest a hidden conditioning variable;
5. a candidate requires a relation path or aggregate not currently materialized;
6. interactive hypotheses disagree but available observations cannot distinguish them without intervention.

### 11.3 Contrast-driven feature invention

Given positive set `P` and negative set `N`, the feature inventor searches for low-cost relational expressions whose values separate them.

Search order:

```text
single observed attributes
aggregates over one relation
relation paths
group-level statistics
symmetry and topology features
comparisons between two role-bound entities
small compositions of existing features
latent variable linked to a control entity
```

Candidate features are canonicalized in the typed e-graph and evaluated in columnar batches. The score combines separation, stability across pairs, cost, and grounding.

### 11.4 Constructor invention from unmatched regions

For each unmatched target region:

1. enumerate plausible source sets by shape, palette-normalized shape, geometric relation, and group correspondence;
2. derive anchor candidates from target extrema, source extrema, reference entities, panels, and intersections;
3. fit low-cost coordinate and color expressions;
4. anti-unify successful partial constructions across pairs;
5. compile the result into a named local definition;
6. return the definition to the theory solver as a new constructor family.

The definition remains task-local until it passes memory promotion criteria.

### 11.5 Hidden-role invention

When no observed predicate separates rule cases, introduce a latent role only if its value can be bound from another observed structure.

Valid example:

```text
control symbol S determines which transform applies to payload P
```

Invalid example:

```text
latent bit L is chosen independently for each pair solely to fit the answer
```

Latents must reduce total description length after accounting for their binder and must predict a held-out demonstration in leave-one-out testing.

### 11.6 Recursive abbreviation invention

Repeated event chains are generalized into a bounded recursive definition when:

- each step has the same relational form;
- a progress measure decreases or a monotone set grows;
- termination is derivable from grid or episode bounds;
- the definition compresses at least two independent instances;
- an unrolled reference execution agrees on all demonstrations.

This handles propagation, repeated tiling, flood-like completion, iterative movement, and interactive skills.

### 11.7 Acceptance gate for an invented definition

An invented definition is admitted to the task-local language only when all are true:

- well-typed RKL body;
- total under declared bounds;
- deterministic in exact domains;
- exact replay improvement or a proven reduction of an unsatisfied core;
- positive description-length gain after including the definition body;
- no new failure on existing demonstrations;
- independent verifier agreement;
- canonical content hash and derivation record.

Task-local admission is not persistent promotion.

## 12. Equality saturation and semantic canonicalization

### 12.1 Purpose

Syntactically different expressions often denote the same feature, selector, coordinate formula, or constructor. A typed e-graph stores these equivalences and prevents redundant search.

### 12.2 E-class analyses

Each e-class tracks:

- type;
- value vector on all current bindings;
- value vector on all training inputs;
- cost lower bound;
- free variables and required roles;
- totality and bound proof;
- symmetry behavior;
- provenance;
- whether a literal constant is grounded;
- compiler availability.

Expressions with identical observational vectors are merged provisionally. Algebraic rewrites establish stronger semantic equality. Observational merges are scoped to the current task and never promoted as universal identities.

### 12.3 Rewrite families

Verified rewrite families include:

```text
set algebra normalization
boolean simplification
arithmetic normalization under bounded ranges
coordinate transform composition
inverse transform cancellation
map/filter fusion
relation join reassociation
rank and argmin equivalences when uniqueness is proven
renderer layer simplification
```

### 12.4 Resource limits

Equality saturation is bounded by:

- e-node cap per type;
- rewrite fuel;
- extraction cost floor;
- value-vector novelty;
- per-agent memory budget.

The system does not seek global saturation. It uses goal-directed saturation around active theory holes.

## 13. Candidate scoring, falsification, and pass@2

### 13.1 Hard acceptance

A deterministic candidate is eligible only if it exactly reproduces every
demonstration outcome under the frontend's declared equality semantics.

An interactive transition model may be probabilistic because observations can be partial. Exact deterministic transitions remain hard constraints where observed.

### 13.2 Structural score

After exact fit, candidates are ordered by a lexicographic score:

1. leave-one-demonstration-out predictive success;
2. falsification survival;
3. total description length of entity definitions, theory, bindings, and renderer;
4. number and cost of literal constants not derived from input roles;
5. number of independent rules and latent variables;
6. causal reuse across event instances;
7. proof complexity;
8. execution cost.

### 13.3 Static falsification suite

Static tasks do not permit arbitrary oracle queries. CAIRN therefore uses non-oracular tests as ranking evidence, never as a substitute for demonstrations.

#### Leave-one-demonstration-out

Refit or rebind the theory on all but one demonstration and predict the held-out output. This is the strongest available task-internal generalization test.

#### Equivariance re-derivation

Apply applicable coordinate and color renamings to the whole task, re-derive or rebind the theory, and compare semantic classes after inverse transformation. Symmetries are tested only when they preserve the observed task semantics; fixed semantic colors are not blindly permuted.

#### Irrelevance perturbation

Modify features that the candidate claims are irrelevant, execute the candidate, and measure whether minor changes cause discontinuous rule changes. Because no oracle output exists, this is a simplicity and stability signal, not a correctness test.

#### Decomposition consistency

When the candidate claims independent rules, execute components separately and jointly. Their composition must match the declared schedule and renderer.

#### Counterfactual discrimination

Generate synthetic inputs on which top candidate theories disagree. This does not reveal which is correct, but measures how distinct the hypotheses are and guides pass@2 diversity.

### 13.4 Posterior modes

Candidates are clustered by predicted requested observations and by semantic theory signature. Near-identical syntax is one mode. The arbiter retains calibrated mass over modes rather than collapsing immediately to one minimum-cost theory.

### 13.5 Two attempts

Attempt one is the highest-scoring mode.

Attempt two is selected to maximize expected coverage under uncertainty:

```text
mode score
multiplied by disagreement with attempt one
multiplied by falsification survival
subject to exact training fit
```

Two theories that differ internally but produce the same requested prediction
count as one attempt. The runtime emits distinct predictions only when credible
semantic modes remain and the adapter permits multiple attempts.

## 14. ARC-AGI-1 and ARC-AGI-2 solve pipeline

### 14.1 Input contract

```rust
pub struct ArcStaticTask {
    pub id: TaskId,
    pub train: Vec<ArcGridPair>,
    pub test: Vec<ArcGrid>,
    pub max_attempts_per_test: u8,
}
```

This is an ARC-adapter contract, lowered losslessly into generic
`LearningProblem`, `Observation`, and `PredictionRequest` values before cognition.
The solver sees all training pairs and all test inputs in the task. It may induce
one shared theory with test-specific bindings, or a theory that branches on input
context. It may not inspect expected test outputs.

### 14.2 Staged anytime schedule

#### Stage A: evidence fingerprinting

Budget target: 10–30 ms.

Compute:

- dimensions and palettes;
- changed-cell masks;
- global transform matches;
- panel and separator evidence;
- connected and multicolor component indices;
- repetition and symmetry fingerprints;
- candidate output-canvas relations;
- rough task family probabilities used only for scheduling.

#### Stage B: event covers

Budget target: 50–300 ms.

Construct low-cost pair-local covers and unmatched-region cores. Run several event-cover agents with different entity demand policies.

#### Stage C: analogical theory sketches

Budget target: 100 ms–1 s.

Align covers across pairs, form rule sketches, and instantiate the simplest theory shapes:

```text
one quantified rule
one rule plus renderer
multiple independent rules
decision list
operator-valued symbol map
bounded fixpoint
```

#### Stage D: exact joint synthesis

Budget target: 1–10 s.

Solve selectors, constructors, mappings, dependencies, and canvas jointly. Execute every candidate and learn from failures.

#### Stage E: ontology expansion

Budget target: 5–60 s.

Run contrast-driven predicate invention, constructor synthesis from unmatched regions, hidden-role proposals, and recursive abbreviation mining.

#### Stage F: deep ambiguity resolution

Budget target: remaining time to 120 s.

Perform leave-one-out refits, applicable augmentations, additional alignments, and second-mode search.

The solver can stop early when one output mode has overwhelming structural evidence and no credible alternative remains.

### 14.3 ARC-AGI-2-specific mechanisms

#### Symbolic interpretation lane

Always activate when:

- small motifs or panels vary across demonstrations;
- the main transformation family varies with them;
- a legend-like pairing structure exists;
- output behavior cannot be explained by geometry alone.

This lane synthesizes operator-valued maps and can bind a symbol to a selector, transform, constructor, or schedule choice.

#### Multi-rule composition lane

Always activate when the changed-cell mask has separable components with distinct transformation signatures or when a single rule leaves systematic residuals. It induces a dependency DAG rather than merely extending term depth.

#### Contextual selection lane

Always construct positive and hard-negative object sets. Selectors are learned as clauses over relations, ranks, group membership, and symbol context. Surface color or position is penalized unless the same literal is stable and causally supported across demonstrations.

### 14.4 Output certificate

```rust
pub struct ArcStaticAnswerCertificate {
    pub task_hash: Hash,
    pub theory_hash: Hash,
    pub theory: Theory,
    pub training_replays: Vec<PairReplayCertificate>,
    pub test_bindings: Vec<TestBindingCertificate>,
    pub predictions: Vec<Vec<ArcGrid>>,
    pub falsification_report: FalsificationReport,
    pub resource_report: ResourceReport,
}
```

The independent checker re-executes the theory from canonical input bytes and verifies hashes, bounds, and predictions.

## 15. Interactive world-model learning

### 15.1 State-transition formulation

For an interactive environment, observations form a history:

```text
o_0, a_0, o_1, a_1, ... , o_t
```

CAIRN induces a belief over theories containing:

- observation-to-entity definitions;
- latent state variables;
- action preconditions;
- action-conditioned rewrite rules;
- stochastic or hidden effects when required;
- reward, progress, terminal, and reset hypotheses;
- temporally extended options.

```rust
pub struct WorldTheory {
    pub state_schema: StateSchema,
    pub observation_model: ObservationModel,
    pub transition_theory: Theory,
    pub goal_hypotheses: Vec<GoalHypothesis>,
    pub options: Vec<OptionTheory>,
}
```

### 15.2 Event extraction across time

Frame differences are not enough. The temporal frontend tracks entities and emits events such as:

```text
agent moved
object appeared or disappeared
tile changed state
counter changed
collision occurred
region became reachable
reward or progress signal changed
level reset
camera or viewport moved
```

Alternative tracks remain as hypotheses until transition rules resolve them.

### 15.3 Hidden state

Introduce a latent state variable only when two histories with observationally similar current frames produce different action outcomes, or when visible changes require a persistent unobserved cause.

Latent values must be updated by explicit transition rules and should compress multiple observations. Arbitrary per-timestep latent labels are prohibited.

### 15.4 Goal inference

Goal hypotheses arise from:

- explicit reward or completion signals;
- transitions followed by reset or new level;
- irreversible progress indicators;
- recurring approach to a state feature;
- conserved resource or target count;
- human-core-prior candidates such as reach, collect, avoid, arrange, unlock, or complete.

Goal hypotheses are executable predicates over state, not natural-language descriptions.

### 15.5 Active intervention selection

The action selector evaluates candidate interventions using a belief-aware objective:

```text
expected reduction in theory entropy
plus expected progress toward likely goals
plus expected discovery of new controllable state
minus action cost
minus risk of irreversible failure
```

Formally, for action or short action sequence `a`:

```text
EVI(a) = H(B) - sum_o P(o | a, B) H(B | a, o)
Utility(a) = EVI(a) + beta * E[goal_value] - gamma * cost(a) - rho * risk(a)
```

Predicted outcomes are clustered by observation signature. Partition probability, not merely the number of distinct outputs, determines information gain. This directly avoids the tie-degenerate experiment selector measured in SCEM H3.1.

### 15.6 Planning

The planner uses a hierarchy:

1. deterministic shortest-path or A* when one theory dominates;
2. contingent AO* over a small belief tree when outcomes distinguish theories;
3. bounded Monte Carlo tree search when the model is stochastic or large;
4. option-level planning after reusable skills are induced.

Planning nodes hold compressed symbolic states and theory-belief summaries, not full raw frame histories.

### 15.7 Option induction

An option is a proof-backed temporally extended mechanism:

```rust
pub struct OptionTheory {
    pub initiation: PredicateExpr,
    pub policy: PolicyProgram,
    pub termination: PredicateExpr,
    pub expected_effect: EffectSummary,
    pub failure_modes: Vec<PredicateExpr>,
}
```

Options are induced from repeated successful action-event patterns, then replayed and falsified in distinct contexts. A raw action sequence without an initiation condition, termination condition, and effect model remains an episode fragment, not a skill.

### 15.8 Context compression

The interactive working memory stores:

- current REG;
- active world-theory posterior;
- event summaries;
- unresolved discrepancies;
- option progress;
- selected landmark states;
- compact replay links to raw frames.

Raw observations remain in a disk-backed episode store but are not repeatedly copied into planning state. This addresses long-horizon context growth.

## 16. Memory and learning

### 16.1 Memory layers

#### Episodic memory

Immutable observations, actions, events, theories considered, counterexamples, and outcomes.

#### Semantic memory

Proof-backed definitions of relations, roles, causal schemas, symbol-map structures, and domain facts.

#### Procedural memory

Compiled RKL kernels, successful solver tactics, option theories, and search schedules.

#### Negative memory

Unrealizable sketches, counterexample families, failed promotion attempts, and domain boundaries.

### 16.2 Persisted mechanism format

```rust
pub struct MechanismModule {
    pub id: ModuleId,
    pub signature: TypedSignature,
    pub definition: RklDefinition,
    pub binder: BinderTheory,
    pub verifier: VerifierSpec,
    pub support: SupportGeometry,
    pub counterexamples: Vec<CounterexampleId>,
    pub transfer_metrics: TransferMetrics,
    pub provenance: DerivationId,
    pub status: ModuleStatus,
}
```

`SupportGeometry` records which variables, parameter ranges, counts, contexts, and relation patterns were actually exercised. An unvaried dimension remains explicitly uncertain; it is not silently fixed or treated as general.

### 16.3 Promotion criteria

A task-local definition becomes a persistent module only if it:

- has a complete RKL definition;
- has support from multiple independent tasks or environments;
- demonstrates held-out rebinding or intervention prediction;
- exercises every declared latent dimension or marks the unexercised dimensions uncertain;
- improves solve coverage, solve time, or theory compression at fixed correctness;
- survives stored counterexamples and generated negative tests;
- can be disabled without invalidating replay of prior answers;
- does not contain task identifiers, literal full grids, or source-task-only constants.

### 16.4 How memory influences solving

Memory may:

- propose an entity definition;
- propose a rule sketch;
- provide a derived predicate;
- initialize a symbol-map family;
- choose an agent schedule;
- provide a compiled implementation for an already selected definition.

Memory may not:

- accept a candidate without current-task replay;
- veto all alternatives;
- directly emit an answer grid;
- hide opaque neural state as a proof substitute.

### 16.5 Learning search strategy

A small CPU-friendly contextual bandit or Bayesian table may learn which agent or sketch family to schedule from evidence fingerprints. It may reorder work only. The architecture does not require neural guidance.

Features include:

```text
dimension relation
palette relation
changed-cell topology
panel likelihood
symbol-control likelihood
number of unmatched regions
event-cover entropy
selector separability
constructor-core types
```

Rewards are exact solve, reduction in unsatisfied core, information gain, and CPU time saved.

## 17. General modality extension

### 17.1 Stable core contract

A frontend must emit:

```text
raw observation reference
candidate entities or primitive elements
relations with uncertainty
temporal correspondences when applicable
modality-specific verifier hooks
```

The core consumes only the normalized observation and REG APIs.

### 17.2 Vision frontend

Initial CPU-only capabilities:

- color and grayscale segmentation;
- edges, contours, regions, lines, corners, and keypoints;
- template and shape matching;
- optical flow or feature tracking;
- depth or stereo facts when supplied;
- scene-graph hypotheses.

Classical algorithms are preferred for exact geometry. Compact non-transformer CPU models such as small convolutional, recurrent, reservoir, or state-space models may propose uncertain detections. Their outputs remain evidence with likelihoods, not verified facts.

### 17.3 Graphics frontend

Vector graphics map naturally to entities, paths, groups, layers, transforms, constraints, and render events. This is the lowest-risk non-ARC extension and should precede unrestricted natural images.

### 17.4 Video frontend

Video adds tracks, temporal relations, action segmentation, and latent-state induction. It shares most machinery with ARC-AGI-3.

### 17.5 Audio frontend

Initial representation:

- waveform segments;
- spectral peaks and bands;
- onset, offset, rhythm, pitch, timbre, and source hypotheses;
- temporal grouping and repetition;
- action or event correspondence.

The theory engine can then induce transformations, sequences, causal effects, or symbolic patterns in the event graph.

### 17.6 Text and knowledge frontend

A transformer is not required by the architecture. A text frontend may combine deterministic parsing, finite-state morphology, chart parsing, grammar induction, entity linking, and domain-specific semantic parsers. Compact non-transformer sequence models may rank parses on CPU.

Knowledge is stored as typed propositions, relations, procedures, source references, and confidence. Retrieval supplies facts to theory formation but does not replace reasoning or verification.

### 17.7 Honest boundary

A common relational backend makes extension possible; it does not make raw perception solved. Every modality requires its own measured frontend. No modality is declared supported until it clears a benchmark and verifier gate.

## 18. Swarm architecture

### 18.1 Purpose

The swarm must extend capability as well as speed. Heterogeneous agents search different explanation spaces, invent different definitions, and falsify each other's theories.

### 18.2 Agent classes

Core classes:

1. evidence indexer;
2. entity-demand explorer;
3. pair correspondence solver;
4. event-cover optimizer;
5. analogical aligner;
6. guard and context inducer;
7. constructor synthesizer;
8. symbol interpreter;
9. schedule synthesizer;
10. joint theory solver;
11. equality-saturation reducer;
12. counterexample analyst;
13. predicate and role inventor;
14. recursive-definition inventor;
15. static falsifier;
16. transition-model learner;
17. goal inference agent;
18. intervention designer;
19. planner;
20. independent verifier;
21. compiler and performance agent;
22. memory curator;
23. resource broker.

### 18.3 Blackboard

Agents communicate through an append-only content-addressed artifact store. They do not mutate shared hypotheses in place.

Key artifact types:

```text
ObservationBundle
EvidenceFragment
EntityDefinition
EventHypothesis
DemonstrationEventCover
Alignment
RuleSketch
ConstraintDelta
TheoryCandidate
Counterexample
UnrealizabilityCertificate
DefinitionProposal
FalsificationReport
AnswerCertificate
WorldTheoryCandidate
PlanCandidate
MechanismModule
```

### 18.4 Work allocation

The resource broker treats each job as an arm with:

```text
estimated probability of useful result
expected reduction in uncertainty or unsatisfied core
estimated CPU and memory cost
novelty relative to completed jobs
critical-path priority
```

Use a deterministic upper-confidence or Thompson-style scheduler in normal mode and a fixed schedule in reproducibility mode.

### 18.5 Capability growth through agents

New agents may add:

- a new entity algebra constructor;
- a new relational feature generator;
- a new exact constructor family;
- a new solver backend;
- a new falsification test;
- a new frontend;
- a new planning algorithm;
- a new compiled kernel.

They must publish artifacts through existing contracts. Core verifier semantics cannot be bypassed.

### 18.6 Local and distributed operation

Local workers use work stealing and shared read-only memory maps for observations and indexes. Distributed workers exchange only canonical artifacts and referenced blobs. Large value vectors are reconstructed locally when cheaper than transfer.

The first implementation target is local multi-process reliability. Network distribution is a later work package and must not delay the single-node solver.

## 19. CPU performance engineering

### 19.1 Data layout

- fixed-size bitsets for grids up to 32 by 32 and 64 by 64;
- dynamic tiled bitsets for larger modalities;
- one bitplane per color or binary predicate;
- structure-of-arrays arenas for nodes and relations;
- small-vector storage for common hyperedges;
- stable integer IDs rather than pointer-rich graphs;
- memory-mapped immutable artifact blobs;
- arena reset between tasks.

### 19.2 Specialized kernels

Hand-optimized CPU kernels are permitted for:

- connected components;
- D4 transforms;
- translations and overlays;
- shape signatures;
- set operations;
- neighborhood expansion;
- distance transforms;
- line and rectangle rasterization;
- relation joins;
- value-vector evaluation.

Every optimized kernel has a naive reference implementation and property tests.

### 19.3 Batching

Evaluate predicate and expression candidates over all bindings and demonstrations in batches. Store boolean value vectors as packed bits. This turns guard induction and observational equivalence into bit operations rather than interpreter loops.

### 19.4 Incremental solving

Reuse:

- SAT clauses across CEGIS iterations;
- event candidates across theory sketches;
- e-classes across related holes;
- entity denotations across agents;
- compiled definition kernels across candidate theories.

### 19.5 Compilation

RKL initially executes in a typed bytecode interpreter. Hot definitions are lowered to a compact micro-op plan specialized to grid dimensions and relation indexes. Native JIT is optional and may be introduced only after the bytecode executor is correct and profiling proves a need.

### 19.6 Search caps

Every search structure has explicit caps and emits a reason when capped:

```text
entity e-nodes
candidate event hyperedges
event covers per pair
alignments per task
rule sketches
SAT variables and clauses
candidate theories
falsification variants
belief-tree nodes
```

A cap hit is not reported as inexpressibility.

### 19.7 Determinism

- stable iteration order;
- deterministic hashers for semantic ordering;
- canonical tie breaks by content hash;
- fixed work quanta;
- no dependence on thread completion order for final ranking;
- deterministic replay mode with the same artifact hashes across worker counts.

## 20. Security, integrity, and CPU-only enforcement

### 20.1 Static dependency policy

The repository checker rejects production dependencies or source references associated with:

```text
CUDA
ROCm
Metal compute
OpenCL compute
Vulkan compute
TensorRT
GPU-enabled ONNX execution
transformer model libraries
remote model APIs in the answer path
```

A library with both CPU and GPU support requires an explicit feature audit and GPU features disabled in the lockfile.

### 20.2 Runtime attestation

Every benchmark record includes:

- CPU model and core count;
- RAM;
- operating system and kernel;
- process tree;
- loaded native libraries;
- environment variables relevant to accelerators;
- network status;
- wall and CPU time;
- peak resident memory;
- git commit and lockfile hash.

### 20.3 Evaluation quarantine

- training and evaluation roots are separate capabilities;
- ordinary binaries cannot enumerate evaluation directories;
- gated evaluation binary requires a signed or explicit sanction file;
- per-task evaluation diagnostics are sealed until the run report is committed;
- no automatic memory promotion from evaluation tasks;
- public task identifiers are forbidden in production source and configuration.

## 21. Repository architecture

```text
cairn/
  LAW.md
  Cargo.toml
  crates/
    cairn-check
    cairn-core
    cairn-cas
    cairn-spatial
    cairn-reg
    cairn-entity
    cairn-rkl
    cairn-bytecode
    cairn-proof
    cairn-events
    cairn-cover
    cairn-analogy
    cairn-egraph
    cairn-constraints
    cairn-guards
    cairn-construct
    cairn-symbols
    cairn-schedule
    cairn-theory
    cairn-cegis
    cairn-invent
    cairn-falsify
    cairn-belief
    cairn-memory
    cairn-knowledge
    cairn-swarm
    cairn-plan
    cairn-frontend-sdk
    cairn-bench
    cairn-eval
  adapters/
    arc12
    arc3
    gridworld
  frontends/
    graphics
    vision
    video
    audio
    text
  schemas/
  suites/
  experiments/
  tools/
```

Crates lower in this list may depend only on explicitly declared layers. In particular:

- the proof checker does not depend on the synthesizer;
- frontends do not alter RKL semantics;
- memory does not enter verifier acceptance logic;
- swarm transport does not enter theory semantics;
- benchmark and evaluation crates do not enter production inference crates.
- adapters translate benchmark/modality records into generic observations and
  requested predictions; adapter types never enter cognition crates;
- `cairn-spatial` is an optional frontend service; `cairn-reg`, `cairn-rkl`,
  `cairn-entity`, `cairn-events`, and `cairn-proof` cannot depend on it;
- temporal evidence, memory, planning, and graphics are core development lanes,
  not post-ARC extensions;
- knowledge retrieval may propose claims or schemas but cannot enter verifier
  acceptance without source and executable proof obligations.

## 22. Integration with legacy assets

### 22.1 Reusable infrastructure

The following prior assets may be reused after isolated property testing:

- bitgrid and geometry kernels;
- canonical serialization and BLAKE3 content addressing;
- deterministic replay infrastructure;
- benchmark report schemas;
- split quarantine and static checker patterns;
- selected structure detectors;
- version-set serialization utilities.

### 22.2 Forbidden architectural inheritance

Do not import as core design:

- SCEM atom, schema, or relational-concept registries;
- task-level trace-mining promotion logic;
- MOSAIC fixed top-level DSL search;
- complete precomputed parse worlds;
- first-consistent-candidate arbitration;
- task-family special cases.

### 22.3 Legacy regression bridge

A one-way translator may lower known SCEM and MOSAIC programs into RKL for regression tests. It exists to prove RKL expressivity and executor correctness. The CAIRN synthesizer must not call the legacy solvers or use their task solutions as search hints.

## 23. Observable failure taxonomy

Every unsolved task ends with a ranked taxonomy:

```text
observation_invalid
entity_demand_cap
no_pair_event_cover
event_cover_ambiguous
cross_pair_alignment_failure
selector_not_separable
constructor_not_synthesized
symbol_semantics_ambiguous
schedule_unrealizable
renderer_unrealizable
ontology_expansion_exhausted
constraint_backend_cap
egraph_cap
theory_exact_but_test_ambiguous
falsification_rejected_all
wall_budget
memory_budget
internal_verification_failure
```

Each category includes:

- deepest completed stage;
- unsatisfied core;
- caps reached;
- top partial theory;
- unexplained cells and entities;
- recommended next general mechanism;
- whether more compute is expected to help.

## 24. Success criteria

### 24.1 Architecture-level success

The architecture is supported only if experiments show all of these:

- event-derived sketches solve tasks that equivalent fixed-size forward enumeration misses at the same CPU budget;
- joint perception and theory induction adds exact pair coverage beyond independent parse selection;
- ontology invention converts previously unrealizable general task families without task-specific code;
- learned guards solve contextual selection with hard negatives;
- symbol mapping solves held-out operator-valued control tasks;
- multi-rule scheduling solves interacting-rule tasks;
- the same theory engine learns useful ARC-AGI-3 transition models and reduces action count versus reactive baselines;
- swarm agents add both speed and unique solves.
- omitted predicates and constructors are invented across unrelated families;
- memory reduces median search work by at least 50% at fixed CPU budget without
  reducing exact accuracy;
- independently learned mechanisms compose on a held-out problem without a new
  composite primitive;
- unknown transition laws and goals are learned online with improving action
  efficiency and option transfer;
- one evidence/theory/invention/verifier/memory/planning core works across
  finite-field transformations, interactive worlds, and vector graphics.

### 24.2 Benchmark north stars

- General learning: pass GLL-1 through GLL-6 before using the phrase generally
  learning system.
- ARC-AGI-1: exceed strong classical symbolic baselines with CPU-only pass@2.
- ARC-AGI-2: staged climb toward the 85% clean private-evaluation target.
- ARC-AGI-3: complete novel environments with action efficiency approaching human baselines, without environment-specific harness logic.
- Vector graphics/diagrams: learn and verify held-out transformations through
  the same cognition runtime without graphics-specific core effects.

### 24.3 No guarantee by design document

A design can be falsifiable and implementable; it cannot honestly certify a future private score. The experiment ladder is constructed to expose failure quickly and to prevent another long build whose core hypothesis is not measured until the end.

## 25. One-paragraph implementation brief

Build a typed relational evidence engine, universal bounded rewrite kernel,
independent verifier, temporal observation path, memory boundary, and frontend SDK
as co-developed foundations. Infer theories from observations, execute and refute
them, invent missing task-local definitions, and preserve multiple explanatory
modes. In parallel, learn action-conditioned worlds and goals, promote only
proof-backed mechanisms through fixed-budget transfer tests, and validate the
same cognition contracts on finite-field, interactive, and vector-graphics
domains. ARC remains an adapter and quarantined benchmark. Coordinate work
through proof-carrying artifacts while enforcing CPU-only execution,
deterministic replay, source-linked knowledge, and independent acceptance.
