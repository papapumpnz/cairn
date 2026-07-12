# ADR-0014: Stateful autonomous developmental curriculum

Status: accepted

Date: 2026-07-12

Owners: WP34 integration

## Context

The initial G6 developmental curve demonstrated growing capability and falling
composition cost, but its learning order was scripted by the benchmark test.
`FrontierCurriculumGenerator` could rank a supplied list once; it did not retain
learned support, unlock prerequisite-dependent objectives, distinguish terminal
states, or react to counterexamples. This was evidence for learning under a
curriculum, not autonomous curriculum control.

## Decision

Add developmental curriculum semantics 1.0.0 and
`AutonomousCurriculum`. Objectives expose only an opaque experience identity,
learning axis, produced mechanism role, prerequisite roles, hypothesis count,
expected transfer, action cost, and irreversible risk. They contain no task
payload, expected answer, domain label, or benchmark identity.

The controller deterministically selects the highest-valued eligible unresolved
objective. A composition objective is ineligible until every prerequisite is an
active independently verified mechanism. Stable experience identity breaks
ties. Completion, capacity, blocked prerequisites, and outstanding work are
distinct terminal states.

Only an independently verified outcome with held-out transfer, strict
memory-off candidate and sample reduction, complete retention, and zero
negative transfer updates support. One distinct verified counterexample
quarantines a mechanism; a second deprecates it. Unverified, duplicate, empty,
or mismatched evidence cannot change state.

`SustainedLearningGate` evaluates repeated phases at equal or independently
declared geometry. Every phase must reduce candidate, example, and proof work
against its own frozen memory-off baseline; capability and mechanism count must
not regress; prior tasks must be retained; negative transfer must remain below
2%; and at least one full restart must occur. Cross-phase cost comparisons are
not inferred from differing task sizes. The existing composition G6 remains the
separate evidence for decreasing marginal composition cost.

## Alternatives considered

### Keep benchmark-scripted order

This is reproducible but cannot show that learned prerequisites change what the
system attempts next. It was rejected as the developmental controller.

### Let objective IDs name solver families

This simplifies dispatch but risks domain logic entering cognition. Objective
IDs remain opaque to the controller; the separated environment generator owns
the mapping from a selected experience to observations.

### Require cross-stage candidate cost always to fall

This can be gamed by shortening later tasks. It was rejected. Each stage uses
its own memory-off ablation, while marginal composition cost remains a separate
same-task measurement.

## Consequences

Correctness: curriculum state changes only from verified summarized outcomes;
it cannot reconstruct an answer or observation.

Determinism: objective input order does not affect selection, and every tie has
a stable ID rule.

Safety: counterexamples remove mechanisms from prerequisite eligibility before
deprecation; stale or missing prerequisites produce an explicit blocked state.

Integration: the frozen test learns four distinct source-independent internal
mechanisms, persists their proof and reuse evidence, crosses two CAS restarts,
and then selects an unseen composition objective unlocked by the first two.

## Validation

The autonomous order is `family-a`, `family-b`, `family-c`, `family-d`, followed
by `compose-first-two`; the order emerges from uncertainty scores and
prerequisites. All four mechanisms use identical host geometry and independently
measure 18 to nine candidate windows, four to one examples, and 28 to four proof
units. Two restart reconstructions retain all mechanisms. Aggregate prior-task
retention is 6/6 and unrelated transfer is 0/10.

Unit tests cover reordered-objective determinism, outstanding work, capacity,
prerequisite deadlock, unverified outcomes, duplicate counterexamples,
quarantine, and deprecation.

Rollback trigger: input-order-dependent selection, unverified state change,
composition before prerequisites, retention failure, negative transfer at or
above 2%, or any learning claim derived from unequal hidden task sizes.

## Compatibility

The developmental curriculum API is additive. Existing one-shot `Curriculum`
and `DevelopmentalGate` behavior is unchanged.

## References

- ADR-0007 self-compressing mechanism learner
- ADR-0013 replay internal learning after restart
- `docs/experiments/G6_AUTONOMOUS_RESTART_CURRICULUM.md`
