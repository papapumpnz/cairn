# CAIRN general-learner requirements

Status: normative redesign, adopted by ADR-0003 and strengthened by ADR-0007

## 1. Honest claim boundary

CAIRN is a hypothesis for a learning architecture. It is not evidence that a
generally learning system has been achieved.

The architecture genuinely intends to learn task-local entity definitions,
causal roles, selectors, guards, variable-arity constructors, schedules, symbol
meanings, latent state, executable world models, predicates, and abstractions.
Across tasks it may retain verified relational predicates, causal schemas,
constructors, symbol-binding structures, planning options, inference tactics,
negative constraints, and scheduling policies. Through interaction it may learn
action effects, persistent entities, hidden state, goals, preconditions, effects,
skills, and discriminating experiments.

These mechanisms qualify as learning only after measured discovery and transfer.
A universal language plus search is insufficient. ARC performance alone proves
only ARC-solving capability.

## 2. Mandatory learning loops

### Loop 1: task learning

Given observations or demonstrations, infer a theory that was not previously
stored.

Measure:

- performance on unseen instances;
- leave-one-demonstration-out prediction;
- omitted-predicate and omitted-constructor invention;
- stability under irrelevant surface changes;
- exact replay and independent verification.

### Loop 2: continual learning

After multiple tasks or episodes, extract reusable proof-backed mechanisms that
improve later learning.

Measure:

- demonstrations required;
- CPU work and wall time;
- hypotheses executed;
- exploratory actions;
- structurally related unseen-task success;
- regression on unrelated tasks;
- memory enabled versus disabled on the same frozen set and budget.

Memory proposes only. It cannot emit accepted answers, hide unsupported axes,
or bypass current-problem binding and verification.

### Loop 3: interactive causal learning

In unknown environments, discover entities, action effects, hidden state, goals,
interventions, plans, and reusable options. This loop is developed alongside
static induction, not after its completion.

Measure transition prediction, goal-hypothesis rank, information gained per
action, action efficiency, repeated-episode improvement, option transfer, and
recovery from incorrect world theories.

## 3. Mandatory continuation gates

The decisive gates are now G0 through G6 below. Earlier GLL-1 through GLL-6
reports remain component evidence and aliases for related concerns, but do not
establish completion without mechanism provenance, measured reuse, support
geometry, and learning-cost curves.

### G0: omitted-mechanism invention

The target mechanism is absent from named primitives and templates. Initial
success must be below 20%; after experience it must exceed 70% without source
changes. Invented definitions must be visible, executable, independently
verified, and transfer to unseen cardinalities and layouts.

### G1: actual reuse

Every counted persistent mechanism records independent invocations, necessity
under ablation, proof depth removed, candidate executions saved, and examples or
actions saved. An unused stored function is not learned persistent knowledge.

### G2: continual improvement

On identical frozen tasks, semantic mechanism memory must produce at least 4x
fewer candidate executions, 2x lower median CPU time, and 30% fewer
demonstrations/interventions without accuracy loss; negative transfer stays
below 2%.

### G3: unseen composition

Separately learned conditional skills must compose on undemonstrated composite
problems with at least 80% exact completion. Plans explicitly invoke learned
skills and ablating one produces the predicted failure.

### G4: unknown causal worlds

After bounded active exploration, next-state prediction reaches 95%, exploration
uses at least 50% fewer actions than random, latent variables are intervention-
testable, and mechanisms transfer after layout or action-law changes.

### G5: cross-domain cognition

One unchanged cognition runtime serves at least three adapters, contains no
domain checks, and demonstrates measurable reuse of at least one learned
mechanism across domains.

### G6: developmental scaling

Across an increasing curriculum, capability rises while marginal CPU,
examples, and interventions per acquired mechanism fall. Reports include
library reuse, program-depth compression, composition depth, support expansion,
negative transfer, and catastrophic forgetting.

### GLL-1: omitted-concept invention

Across multiple unrelated generated families, omit a required relation or
constructor from the initial vocabulary. CAIRN must invent an executable RKL
definition and solve held-out changes in geometry, count, categorical values,
and position. Passing one fixed family is insufficient.

### GLL-2: cross-task improvement

On a frozen structurally related test set and equal CPU budget:

- memory reduces median search work by at least 50%;
- required demonstrations decrease on a pre-registered curriculum;
- exact accuracy does not fall;
- definitions rebind across categorical values, sizes, counts, and layouts;
- unrelated-task accuracy does not regress beyond the pre-registered tolerance.

### GLL-3: skill composition

Learn two mechanisms independently, then solve held-out problems requiring their
composition without a newly coded composite operator. The initial suite includes
containment learned separately from ordering, followed by ordered operations over
contained entities.

### GLL-4: unknown-world learning

In procedurally generated environments with unseen transition laws, predict
action outcomes, infer executable goals, plan, improve action efficiency over
episodes, and transfer an induced option to changed layouts.

### GLL-5: cross-domain representation

Run one cognition runtime on at least three domains:

1. finite-field or grid transformations;
2. interactive worlds;
3. vector graphics or diagram manipulation.

Frontends differ. Evidence graph, theory representation, ontology invention,
verifier, memory promotion, and planning contracts remain shared.

### GLL-6: ARC quarantine

No general-core change enters solely because it solves an ARC task. Every core
mechanism first requires an environment-neutral definition, a synthetic family
varying every claimed axis, a non-ARC case where applicable, negative cases, and
an ablation attributing the gain to the mechanism.

## 4. Core and adapter boundary

General core vocabulary:

```text
Observation
EntityHypothesis
Relation
Event
Theory
Transition
Goal
Intervention
Skill
MemoryModule
KnowledgeClaim
VerificationResult
```

Adapter vocabulary may include benchmark or modality details. ARC task, training
pair, test grid, ARC colour, and ARC operation types belong only to `cairn-arc12`.
Gridworld, vector graphics, audio-event, video-event, and text/knowledge adapters
connect through the same frontend and cognition contracts.

Packed finite spatial fields and geometry kernels are an optional frontend
service. The evidence graph, RKL, entity algebra, event induction, and verifier
must compile and operate without that service. Spatial facts enter cognition as
ordinary sourced entities, relations, attributes, and events.

Implementation checkpoint (2026-07-11): `FiniteField::evidence_graph` emits
provenance-bound scalar nodes, typed attributes, and exact adjacency relations.
The demand-driven entity generator consumes those facts without field or ARC
types. `ContrastPredicateInventor` then constructs Boolean RKL definitions from
positive and hard-negative typed bindings. On the frozen initial family it
invented the omitted enclosure-plus-marker selector after 2,086 candidates and
predicted four unseen value/size combinations exactly. This establishes the
route, not broad relational ontology learning.

The next frozen relation family removes the supplied `enclosed` feature. From
48 ordinary cross-entity coordinate comparisons, the learner selects four
strict inequalities defining enclosure. A second independently shifted and
renamed family induces the same executable definition. The independent
verifier promotes it to persistent memory; a third renamed family reuses it
from one example, reducing 236 conjunction candidates to one verified proposal
while preserving 5/5 held-out geometry. See
docs/experiments/INVENTED_ENCLOSURE_PROMOTION.md.

Residual-driven constructor invention is now independently exercised. The
frontend computes created-cell residuals but supplies no shape operator. The
learner synthesizes the Boolean emission definition `row == column`, rejects
mixed-effect residuals, re-invents the rule across changed sizes and categorical
values, and passes verifier-gated promotion. A third 7x7 construction reuses the
predicate from one example with a new paint value: candidates fall 29 to one,
examples 34 to one, and 49/49 held-out cells are exact. See
docs/experiments/RESIDUAL_CONSTRUCTOR_PROMOTION.md.

Bounded recursive abstraction is now exercised from traces. The learner first
induces a conditional state-transition definition from observed steps, then
lifts the definition into an RKL `BoundedFixpoint` with explicit progress and
resource bounds. It does not retain the unrolled trace. Two independent trace
families produce the same mechanism; strict promotion succeeds, and a renamed
one-example problem reduces 13,539 transition candidates to one memory proposal
with 4/4 unseen iteration cases exact. See
docs/experiments/ITERATIVE_MECHANISM_PROMOTION.md.

Higher-order collection lifting and whole-theory persistence are now exercised.
From aligned collections of cardinalities two, four, and five, the learner
induces an element transform and constructs a checked `ForEach` rewrite rule.
On an unseen cardinality seven, the rule fires seven times and is 7/7 exact.
Two independent families yield the same canonical checked theory. A procedural
promotion token binds definitions, quantified rules, schedule, and renderer;
`CognitionRuntime` persists and replays the complete theory, while a modified
theory is rejected under the original token. See
docs/experiments/COLLECTION_MECHANISM_PROMOTION.md.

The common runtime now exposes all three mandatory loops through one instance:
task-local definition learning; persistent definition and canonical procedural
memory; action-conditioned transition and goal induction followed by planning;
and conditional learned-skill composition. A unified test executes all three
paths without adapter types. The official ARC regression remains 17/20 after
the integration, demonstrating that orchestration did not alter the quarantined
adapter baseline. See docs/experiments/UNIFIED_COGNITION_RUNTIME.md.

The named historical finite-field schema catalog is now physically located
inside `adapters/cairn-arc`, together with its twenty regression tests. The
general `cairn-field` crate contains observation, relational-evidence, pointwise
invention, and residual-invention facilities only. A source audit prevents
representative legacy transformation names from returning to the general crate.
The unchanged 17/20 legacy score remains non-learning adapter evidence. See
docs/experiments/ARC_LEGACY_SCHEMA_QUARANTINE.md.

## 5. Proof-backed knowledge learning

The knowledge subsystem stores:

- source observations and spans;
- resolved or explicitly ambiguous entities;
- typed claims and procedures;
- temporal validity intervals;
- confidence and evidence class;
- contradictions without destructive overwrite;
- causal and taxonomic relations;
- retrieval indexes and full provenance;
- executable domain schemas where available.

Agents may parse structured documents, extract claims, compare sources, propose
entity merges, identify contradictions, and construct domain models. Retrieval
confidence is not truth. Unrestricted prose understanding is not claimed without
a separately measured CPU frontend.

Implementation checkpoint (2026-07-11): `cairn-knowledge` implements admitted
source observations, typed relational claims, temporal intervals, fixed-point
confidence, subject/predicate/time retrieval, first-class contradiction pairs,
reversible entity-merge proposals, and executable mechanism references.
`CognitionRuntime` owns the knowledge base. Structured acquisition passes; no
unrestricted prose or language-understanding claim is made. See
docs/experiments/PROOF_BACKED_KNOWLEDGE.md.

## 6. Kill and claim rule

Do not describe CAIRN as a generally learning AI until it has passed all six
gates. If it solves ARC but fails transfer, continual improvement, unknown-world
learning, skill composition, or cross-domain reuse, report it as a symbolic ARC
solver and preserve the negative evidence.

## 7. Implementation checkpoint: 2026-07-11

The initial static-learning vertical slice now includes typed theory inputs,
whole-theory reference/bytecode execution, generic observation contracts,
synthesis-free replay certificates, first relation-defined entity and generic
event hypotheses, guard/constructor/schedule induction, CEGIS over complete
candidates, a finite-field invention gate, and a quarantined ARC adapter.

This checkpoint does not mark any new general-learning gate complete beyond the
previously documented bounded initial GLL-1 through GLL-4 results. In particular:

- the finite-field gate is one pointwise constructor family, not the multiple
  unrelated families required for final GLL-1;
- cross-domain GLL-5 still lacks vector graphics through one orchestrator;
- GLL-6 structure is enforced. ARC-AGI-1 v1.0.2 training data is now pinned and
  an initial frozen ten-task batch has been run; it yielded zero exact solves,
  three explicit shape-change capability gaps, and seven search-cap results;
- the verifier is synthesis-free but does not yet independently reimplement
  every critical rule/effect/renderer semantic required by WP06.

See `docs/adr/0005-generic-static-learning-runtime-boundaries.md` and
`docs/experiments/INITIAL_STATIC_LEARNING_PIPELINE.md`.

The same frozen first-ten ARC batch later reached 10/10 after generic field
schemas were added under ADR-0006. An untouched tasks 11–20 probe remained 0/10.
This is a development capability pass, not evidence that GLL-1, GLL-5, broad
ARC generalization, or general intelligence is complete.

## 8. ADR-0007 checkpoint: initial G0 pass

The independently owned frozen hidden-law suite now passes across five
unrelated mechanisms. Initial bounded success was 0/10 and post-experience
success was 10/10 with no source changes. Ten independently replayed held-out
probes varied parameter names, ranges, symbols, collection cardinalities,
positions, and layouts. Candidate executions fell from 33,600 under fresh
search to 10 verified memory proposals. All five mechanisms required
proof-subgraph compression and a lineage-bound verifier token before persistent
status.

This satisfies G0 on the frozen initial suite. It does not satisfy G1-G6 or the
general-learning pass. See
docs/experiments/G0_MULTIFAMILY_HIDDEN_LAW_PASS.md.

## 9. ADR-0007 checkpoint: initial G1 pass

Every mechanism counted by the frozen G0 suite now carries task-distinct,
independently verified reuse records. Across five persistent mechanisms and ten
held-out tasks, the ledger records ten actual and necessary invocations,
33,600 versus 10 candidate executions, 48 versus 10 demonstrations, and 52
versus 10 proof nodes. Duplicate, unverified, and pre-promotion claims fail.

This satisfies G1 on the frozen initial suite. The ledger is not yet a
canonical CAS artifact, and G2-G6 remain open. See
docs/experiments/G1_ACTUAL_REUSE_LEDGER_PASS.md.

## 10. ADR-0007 checkpoint: initial G2 pass

On the frozen equal-budget related suite, semantic memory reduced median
candidate executions from 480 to one, demonstrations from two to one, and
observed median CPU time from 272.6 ms to 0.553 ms while retaining 5/5 exact
held-out behavior. On 100 unrelated generated laws, memory rejected all
inapplicable proposals, fell back to fresh search, and preserved 100/100
exactness with zero measured negative transfer.

This exceeds the G2 thresholds on the frozen initial suite. G3-G6 remain open
under ADR-0007. See docs/experiments/G2_CONTINUAL_IMPROVEMENT_PASS.md.

## 11. ADR-0007 checkpoint: initial G3 pass

Four conditional skills were induced from separate successful trace families;
no composite trace or operator was supplied. On 100 held-out compositions the
generic effect-chain planner completed 100/100 and emitted 400 explicit learned
skill invocations. Removing any required skill caused the predicted no-plan
failure in all 400 skill/task ablations.

This exceeds the G3 threshold on the frozen initial suite. G4-G6 remain open.
See docs/experiments/G3_UNSEEN_SKILL_COMPOSITION_PASS.md.

## 12. ADR-0007 checkpoint: initial G4 pass

Across 100 unknown action-law worlds, the online version space and separate
curriculum resolved 24 competing theories using 300 interventions versus 791
random actions, predicted 32,400/32,400 transitions, transferred to 100 shifted
layouts, and relearned 100 changed action laws. In 100 hidden-mode worlds the
learner rejected one-state models, induced the required two-state variable,
predicted 12,600/12,600 latent transitions, and constructed 100 independently
shortest plans.

This satisfies G4 on the frozen initial suites. G5-G6 remain open. See
docs/experiments/G4_ACTIVE_CAUSAL_WORLD_PASS.md.

## 13. ADR-0007 checkpoint: initial G5 pass

One unchanged CognitionRuntime now serves independent vector-scene,
interactive-gridworld, and sequence-relation adapters. A mechanism learned from
vector tasks rebound and predicted held-out observations exactly in all three
domains, reducing 36,180 fresh candidates to three verified memory proposals.
An automated audit finds no adapter or domain labels in runtime semantics.

This satisfies G5 on the frozen initial suite. G6 remains open. See
docs/experiments/G5_CROSS_DOMAIN_COGNITION_PASS.md.

## 14. ADR-0007 checkpoint: initial G6 pass

A frozen staged curriculum starts with three independently induced mechanisms,
then constructs and consolidates successively deeper mechanisms by verified
composition. Under a bound that prevents fresh synthesis, capability rises
from 1/3 to 2/3 to 3/3 while the available library grows from three to five
mechanisms and executable composition depth rises from one to four. Marginal
candidate work per acquisition falls from 24 to 20, demonstrations from ten to
six, and observed same-task median CPU time from 2.284 seconds to 0.916
milliseconds. Previously learned behavior remains 3/3 exact; an unrelated
nonlinear law falls back to fresh search with zero measured negative transfer.
The skill curriculum separately confirms that four induced skills compose with
zero new interventions after two traces per primitive skill.

This satisfies G6 on the frozen initial suite. G0-G6 passing is a major
architectural checkpoint, not a general-intelligence claim: the suites remain
small, exact, synthetic, and bounded. Independent replications, richer
mechanism classes, longer curricula, durable compiled memory, and broad
real-world knowledge learning remain required. See
docs/experiments/G6_DEVELOPMENTAL_SCALING_PASS.md.

## 15. ARC generic-experience boundary checkpoint

The quarantined ARC adapter now lowers sealed tasks to core `LearningProblem`,
`Observation`, `Demonstration`, and `PredictionRequest` values. Every
observation uses the finite-field modality, carries a source/content identity,
and is admitted by `CognitionRuntime`, which rejects invalid identities,
duplicates, and references outside the experience envelope without inspecting
adapter semantics. The official first-20 checkpoint remains 17/20 after 150
generic observations emit 23,114 scalar evidence nodes and 70,550 exact
adjacency facts, with the same three resource-limited tasks.

This completes the generic input boundary but not the shared-solver migration.
The successful ARC tasks still use the quarantined legacy finite-field schema
backend after admission. Those named schemas do not count as learned
mechanisms, may not enter general memory, and must ultimately be replaced by
the mechanism learner or remain a non-claiming regression baseline. See
docs/experiments/ARC_GENERIC_EXPERIENCE_BOUNDARY.md.

## 16. Typed proof-DAG consolidation checkpoint

The initial string-trace compression prototype has been replaced on the main
G0 scalar path by typed immutable proof DAGs and a separate offline
`ConsolidationRuntime`. The runtime admits bounded topologically valid episode
graphs, automatically groups compatible ancestral subgraphs without task-family
labels, anti-unifies typed arguments, and exactly reconstructs each source
subgraph before returning a proposal. Memory's `promote_compressed` path binds
that opaque proposal to the exact set of independently invented definitions.

In the frozen end-to-end rerun, two independent inventions formed one cohort,
compressed from 12 proof units to seven with two exact reconstruction replays,
then supported a persistent mechanism that solved 10/10 held-out tasks from one
example each. Search was 440 candidates without memory versus ten verified
memory proposals. Topology and type changes, malformed DAGs, duplicate episode
identities, unrelated branches, and non-compressing cohorts are covered by
negative tests.

This is real automatic structural comparison across experience, but not yet a
complete self-compressing learner. It currently requires matching operation and
dependency shapes, retains graphs in process memory, and does not yet turn an
arbitrary compressed graph directly into a new executable RKL definition. See
docs/experiments/TYPED_PROOF_CONSOLIDATION.md.

## 17. Executable mechanism compilation checkpoint

The compressed-promotion route now creates a new executable definition rather
than retaining either episode-local source. `MechanismCompiler` requires the
opaque compression lineage, verifies equivalent executable structure,
alpha-normalizes binders, unions observed integer and bounded-collection type
support, gives the abstraction a content-derived `invented_mechanism_<hash>`
name, passes RKL totality/type checking, and exactly replays every normalized
source definition. Its semantic hash differs from both source definitions.

Acceptance authority is separate. `cairn-verifier::CompressionVerifier`
independently traverses each ancestral source graph, reconstructs it from the
pattern and episode binding, recalculates raw and compressed costs, and returns
an opaque token. `promote_compressed` requires that token and checks its proposal
identity, lineage, costs, and replay count. Changed source content, identity,
topology, missing binding, cost mismatch, and non-compression fail closed.

The compiled abstraction remains exact on the frozen 10-task held-out transfer
suite and retains the 440-to-10 candidate reduction. This meets the bounded
meaning of an expanding executable mechanism language for equal-shape scalar
cohorts. It does not yet compile arbitrary repeated internal subgraphs,
generalize different graph shapes, or verify semantic equivalence by exhaustive
execution over every bounded input. See
docs/experiments/EXECUTABLE_MECHANISM_COMPILATION.md.

## 18. Complete ARC-AGI-1 training audit and generic remapping

The first complete pinned 400-task training audit records 27 exact tasks
(6.75%), 133 unsupported shape changes, and 240 pointwise search caps under
adapter 1.5.0. This supersedes any impression that the favorable 17/20 sorted
prefix represented broad ARC coverage.

An adapter-neutral `RemappedFieldLearner` now invents four executable RKL
definitions: output rows, output columns, source row, and source column. Output
geometry can bind to generic content-bound hypotheses; joint affine search
resolves repeated-value correspondences by complete output replay. Synthetic
transpose and content-relative extraction families transfer to unseen geometry,
layout, cardinality, and values, while uniform ambiguous evidence fails closed.

Without adding a named transform or changing the quarantined schema catalog,
the fixed 1,000-candidate full audit rises to 29/400 exact (7.25%) with zero
incorrect accepted theories. The 80% criterion requires 320 exact tasks, so it
is emphatically not passed. See
docs/experiments/ARC_AGI_1_FULL_CORPUS_BASELINE.md.

## 19. Entity selection and local causal-law checkpoint

Generic component perception now proposes connected frames from modal-complement
evidence. Across independent finite-diagram families, positive and negative
frames drive synthesis of an executable selector that transfers to unseen
component size, color, and layout. The selector route adds no fixed-budget ARC
solve, a retained negative result showing that selection without subsequent
relational construction is insufficient.

`LocalFieldLearner` observes a cell, its bounded 8-neighborhood, coordinates,
and extents, then synthesizes one RKL transition law with exact replay. It
learns a horizontal propagation law outside ARC and transfers across new
geometry, values, and positions. On the complete pinned corpus it adds two exact
tasks through `north` and `min(north, 2)`, moving the score from 29/400 to
31/400 with zero incorrect accepted theories.

Unguided local search was too costly. A demonstration-derived support gate now
runs it only when outputs are grounded by observed neighborhood values or the
tiny arithmetic literal basis. Accuracy remains 31/400 while the complete run
takes 360.36 seconds rather than roughly eleven minutes. This is measurable
search-breadth learning/routing, but 7.75% remains far below the 80% threshold.

## 20. Long-range relational observation checkpoint

The local learner now has optional nearest-visible values in four directions
and row/column occupancy aggregates. On a non-ARC table family it invents a
long-range carry-forward law and transfers across unseen dimensions, values,
and positions. No propagation, ray, or fill operator is supplied.

On the complete ARC training audit, the new evidence supports one additional
exact law, `max(center, visible_north)`, reaching 32/400 (8.0%) with zero
incorrect accepted theories. Eager relational expansion took 446.27 seconds.
Two routing refinements retained accuracy but did not reduce synthesis cost;
lazy relational perception finally reduced elapsed time to 414.16 seconds.
This is still slower than the 360.36-second 31-task vocabulary, so the next
cost improvement must come from learned proposal ordering or reusable proof
fragments rather than additional frontend summaries.

## 21. Continual local-law integration checkpoint

`LocalFieldLearner` now lowers observations to the same domain-neutral
`InventionProblem` consumed by `CognitionRuntime` and
`MemoryAssistedInventor`. Fresh invention remains available, but it is no
longer the frontend's only learning route. Independently admitted outcomes are
wrapped as executable field theories only after common dual-executor replay.

The first high-arity experiment found that reusable-module retrieval eagerly
constructed all type-compatible parameter permutations before attempting any
proposal. Thirteen integer inputs imply 13! possible bindings, so this was a
general continual-learning blocker. Retrieval now prioritizes matching semantic
parameter names and bounds direct and composed rebound proposals with a
1,024-proposal hard ceiling. The cap limits proposal generation;
it does not weaken replay or verifier admission.

Two independent changed-geometry/value tasks invented the same previously
unnamed right-shift law, `output := west`. After strict promotion, a frozen
related task required four fresh candidate executions and 21 examples. The
persistent mechanism solved a three-example rebinding in one proposal under a
budget where fresh search fails, then predicted all 35 cells of a changed
held-out field exactly. This is a concrete continual-learning result for a
mechanism used by the live static frontend, not an ARC-specific operation. It
does not change the official ARC score by itself and is not evidence of general
intelligence.

## 22. Chatbot, distillation, and swarm-transfer capability

The product target now includes a normal conversational interface, initially a
structured technical and coding assistant. This does not relax the general
learning claim. Language understanding, dialogue state, declarative knowledge,
typed code/tool synthesis, response planning, and controlled realization are
new mandatory lanes around the same causal cognition runtime.

External-model bootstrap is behavioral and symbolic, never weight or logit
distillation. Offline structured packets may propose language mappings, typed
claims, procedures, mechanisms, skills, curricula, tests, alternatives, and
counterexamples. CAIRN must compile, verify, decompose, anti-unify, challenge,
transfer-test, and revoke them using its own representations. The external
teacher is absent from production and from the teacher-independence evaluation.
GPT and Anthropic integrations are network-free spool codecs only. They may
export an API-shaped JSON request and import a response saved by a separately
governed external process. They may not own HTTP, sockets, credentials, provider
SDKs, or subprocess transport. CAIRN answers solely from current evidence and
accepted local knowledge; absence of knowledge must remain an explicit failure
or clarification, never an implicit online teacher call.

Swarm learning is a first-class continual loop: discoveries and failures become
immutable proof-carrying artifacts; specialist peers replay, falsify, transfer
test, consolidate, and publish them; other agents retrieve by typed need and
rebind locally. Conclusions never transfer as truth. Shared facts, procedures,
mechanisms, skills, negative constraints, and search policies retain distinct
validators and support boundaries.

Verified non-ARC knowledge is available to ARC only as a proposal source. The
decisive transfer experiment freezes ARC first, learns from unrelated domains,
then measures exactness, CPU, candidate count, actual proof-trace use, ablation,
and negative transfer. A score increase without causal use of clean transferred
knowledge is not counted.

## 23. Initial chatbot/distillation/D2 result

The controlled technical chat path now compiles a structural intent signature,
retains dialogue state, builds a typed Python AST, checks it internally, and
executes it under isolated CPython resource limits before realization. One
offline packet carries held-out paraphrases, a structurally validated sourced
claim candidate, a dependency-checked procedure graph, executable positive
examples, and a negative counterexample. Two separately replayed peer receipts
plus transfer evidence gate swarm acceptance; the teacher is absent during use.

Two non-ARC spatial lessons produce a strictly persistent `output := north`
mechanism. On the complete pinned ARC corpus at 1,000 candidates, blank and
distilled-memory conditions both score 32/400 with zero incorrect. One proof
trace actually reuses memory and reduces task-local search from two candidates
to one. The full memory-on run is approximately 38 seconds slower, so the
current unconditional retrieval policy fails the systems-efficiency objective
despite zero accuracy negative transfer. D2 has an initial safe reuse result,
not an ARC accuracy pass or broad teacher-independence pass.

The first correction uses current demonstration bindings to refute impossible
single-variable rebindings before compilation. The complete rerun remains
32/400 with zero incorrect and falls from 458 derived seconds to an embedded
390.85 seconds, while matching memory still removes one candidate. This clears
the initial narrow retrieval-overhead regression; contextual retrieval for
compound and composed mechanisms remains open.

The first larger offline GPT-authored curriculum supplied 40 independently
varied, non-ARC spatial-sensor lessons. CAIRN synthesized and admitted 20
executable provisional mechanisms, then ran the frozen 400-task corpus. The
result was
32 exact, 368 resource-limited, zero incorrect, 476.50 seconds, and zero
selected proofs using those mechanisms. This passes offline compilation and
admission but fails D2 transfer and compute improvement. The mechanisms do not
count as useful learned persistent knowledge under G1 until actual reuse is
shown. No Anthropic knowledge was ingested; only its offline spool contract is
implemented and tested.

Gap-driven distillation now requires a content-free `LearningFailureCore`.
Permitted fields describe only type classes, counts, determinism, anonymous
mechanism-family matches, and exhausted resource bounds. Names, raw values,
observations, task identifiers, and answers are prohibited. A causally
insufficient core must trigger representation or latent-context curriculum work
before wider search.

On the first 40 frozen training tasks, nine distinct cores were extracted. One
showed different outcomes under identical 13-feature observations. Adding a
generic determinism gate caused the field frontend to escalate to existing
relational evidence, and that impossible core disappeared. A synthetic
non-benchmark row-occupancy family verifies changed-geometry transfer. The ARC
result remains 17/40 exact, 23 resource-limited, and zero incorrect at the same
1,000-candidate budget. The remaining eight cores match no tested simple
one-/two-/three-role family, so the next offline curriculum is explicitly
focused on hierarchical relations, variable arity, recursion, and role
composition.

The first response-side extension is variable-cardinality collection
distillation. A valid packet must vary cardinality across independent episodes;
CAIRN must invent the element rule and lift it into a checked `ForEach` theory.
The initial non-ARC GPT-authored packet learns from sizes 2, 4, and 5, verifies
11 source elements, and transfers to size 7 after two peer replays. It remains
provisional, supports unique set-like elements only, and does not satisfy the
hierarchical or ARC-transfer gate by itself.

The learned collection theory now crosses the runtime boundary as a provisional
procedural proposal under memory semantics 1.2.0. On a new sequence/set task,
two changed demonstrations are replayed before an unseen eight-element
prediction is accepted. Blank-memory ablation has no proposal; an alternative
`input + 1` task rejects the imported theory. This supplies one measured actual
reuse event without granting persistence. ARC reuse still requires a generic
entity-collection binding and render contract.
