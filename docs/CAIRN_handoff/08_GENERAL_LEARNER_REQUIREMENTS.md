# CAIRN general-learner requirements

Status: normative redesign, adopted by ADR-0003

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
- GLL-6 structure is enforced, but no official ARC corpus has been attempted;
- the verifier is synthesis-free but does not yet independently reimplement
  every critical rule/effect/renderer semantic required by WP06.

See `docs/adr/0005-generic-static-learning-runtime-boundaries.md` and
`docs/experiments/INITIAL_STATIC_LEARNING_PIPELINE.md`.
