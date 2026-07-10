# ADR-0003: General learning is the development centre

Status: accepted

Date: 2026-07-10

Owners: integration owner

## Context

CAIRN's representation includes task-local ontology invention, persistent
mechanism memory, action-conditioned world theories, intervention selection,
planning, and modality frontends. However, the original work order placed the
static ARC runner before continual learning, interactive causal learning, and
cross-domain integration. That ordering creates a strong incentive to optimize
one benchmark and postpone the mechanisms that would establish learning.

A universal language and a search procedure are not evidence of learning.
CAIRN qualifies as a general learner only if it discovers mechanisms not
supplied by developers, transfers them, becomes more sample- or compute-efficient
with experience, composes skills, learns unknown worlds, and uses the same
cognitive core outside ARC.

## Decision

CAIRN development is governed by three mandatory, co-equal learning loops:

1. task learning: infer a new executable theory from current observations;
2. continual learning: promote proof-backed mechanisms that measurably improve
   later learning without controlling acceptance;
3. interactive causal learning: discover entities, action effects, hidden
   state, goals, interventions, plans, and reusable skills online.

ARC is one quarantined evaluation adapter. It does not own core task, colour,
operation, training-pair, or test-grid semantics. Interactive temporal evidence,
memory, vector graphics, and general benchmark infrastructure move earlier in
the dependency graph. The release integration target must exercise grid
transformations, unknown interactive worlds, and vector graphics through the
same evidence, theory, invention, verification, memory, and planning contracts.

Six general-learning gates are mandatory: omitted-concept invention,
cross-task improvement, skill composition, unknown-world learning,
cross-domain representation, and ARC quarantine. ARC score alone is never a
general-learning acceptance result.

A proof-backed knowledge subsystem is also a first-class target. It stores
typed, temporally scoped, source-linked claims, contradictions, relations, and
executable schemas rather than opaque embeddings.

## Alternatives considered

### Complete the static ARC solver first

Benefits: short path to a familiar benchmark and simple deterministic fixtures.

Costs: encourages benchmark-specific mechanisms and makes continual,
interactive, and multimodal learning optional schedule casualties.

Reason rejected: it can produce a sophisticated symbolic application without
evidence of learning.

### Keep the architecture but add stronger wording

Benefits: minimal disruption.

Costs: dependency order and continuation gates would still reward ARC-first
work.

Reason rejected: incentives are set by gates and dependencies, not prose.

### Remove ARC entirely

Benefits: eliminates one source of benchmark bias.

Costs: loses a valuable exact induction benchmark and mature comparison data.

Reason rejected: ARC is useful as one adapter when quarantined from core design.

## Consequences

Correctness: independent replay remains mandatory; memory never becomes an
acceptance authority.

Expressivity: generic observation/transition/goal/skill/knowledge contracts
replace benchmark-shaped core interfaces.

CPU cost: all learning-loop comparisons use fixed CPU and sample budgets.

Memory cost: continual learning must report storage and retrieval costs and pass
memory-off ablations.

Determinism: exact semantic results remain deterministic; uncertain frontends
and beliefs record seeds and evidence.

Latent-state uncertainty: an observationally adequate hidden-state theory is
not treated as identified when episode boundaries leave successor states
unconstrained. Competing successors remain explicit and trigger a
discriminating continuation under ADR-0004 before deterministic promotion or
planning.

Verifier impact: modality hooks may verify source/render semantics, but core
theory acceptance remains independent of synthesis and memory.

Downstream interfaces: WP22 becomes an adapter; WP23, WP26-WP29, WP32-WP34 and
their dependencies are revised. WP02 becomes an optional spatial-frontend
service; WP03, WP05-WP07, and WP09 no longer depend on it. Core types must
remain environment-neutral.

## Validation

Tests: the six gates in `08_GENERAL_LEARNER_REQUIREMENTS.md` and the revised
experiment ladder.

Benchmark gate: fixed-set memory on/off, procedural unknown-world learning, and
same-core three-domain integration are release blockers.

Rollback trigger: none based on ARC score. Superseding this decision requires
evidence that a different development order better establishes transferable
learning across domains under fixed resources.

## Compatibility

Semantics version change: RKL categorical values and finite fields adopt
modality-neutral naming before the WP04 interface is tagged.

Migration: no released WP04 artifact exists. WP00/WP01 storage interfaces remain
compatible.

## References

- `08_GENERAL_LEARNER_REQUIREMENTS.md`
- `02_CAIRN_MASTER_DESIGN.md`
- `04_CAIRN_EXPERIMENT_LADDER.md`
- `HANDOFF_INDEX.md`
- `0004-latent-successor-ambiguity-requires-intervention.md`
