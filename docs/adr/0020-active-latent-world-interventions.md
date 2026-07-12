# ADR-0020: Actively resolve latent action-law ambiguity

Status: accepted

Date: 2026-07-12

Owners: WP28/WP34 integration

## Context

The latent world learner can infer the smallest deterministic hidden-state
transducer and can identify terminal successor ambiguities with a one-step
distinguishing action. Existing tests manually inspect that request, author a
longer episode, and invoke induction again. The cognition runtime therefore
does not yet close observe -> hypothesize -> intervene -> revise by itself.

The relational-world transfer checkpoint also receives complete visible
transition sets. To advance interactive causal learning, experiment selection
must become a bounded production contract and its learned latent theory must
feed the same relational mechanism path.

## Decision

Add `LatentExperiment` as an environment boundary. Given an episode index, its
immutable visible history, and a selected action, it returns exactly one visible
transition. It cannot expose latent state to the learner.

Add `ActiveLatentWorldLearner::induce`. It repeatedly:

1. invokes the exact bounded `LatentWorldLearner`;
2. orders unresolved successor ambiguities canonically;
3. selects the first available one-step distinguishing action;
4. requests one continuation from `LatentExperiment`;
5. verifies action, before-state continuity, bounds, and episode integrity;
6. appends the observation and re-induces from all evidence.

The loop terminates with an exact theory containing no unresolved successors or
with a typed no-distinguishing-action, intervention-capacity, malformed
observation, cancellation-equivalent resource, or underlying induction error.
It records every selected intervention and the final augmented episodes.

Expose this through the unchanged `CognitionRuntime`; the runtime coordinates
but does not contain environment laws. A benchmark environment independently
owns truth, exposes only visible outcomes, and compiles final learned latent
rules into canonical relational observations. Existing persistent mechanisms
then consume those observations without a latent-world-specific core path.

## Alternatives considered

### Keep intervention logic in benchmark tests

Benefits: no public API.

Costs: interactive learning remains authored orchestration rather than system
behavior.

Reason rejected: it cannot satisfy the mandatory online causal loop.

### Reveal latent state through the environment interface

Benefits: trivial learning and verification.

Costs: removes the variable that must be inferred.

Reason rejected: invalidates latent-state learning.

### Generate arbitrary action sequences globally

Benefits: broad search.

Costs: expensive and ignores the exact ambiguity certificate already produced.

Reason rejected: bounded discriminating continuations are the trustworthy
reference path; longer experiment search remains future work.

## Consequences

Correctness: each appended transition is continuity- and bounds-checked before
re-induction; final theories retain exact replay semantics.

Expressivity: one-step distinguishing continuations are automated. Ambiguities
requiring longer conditional experiments fail explicitly.

CPU cost: each intervention reruns bounded exact induction. Incremental search
is deferred until profiling proves need.

Memory cost: augmented visible episodes and a small intervention journal.

Determinism: ambiguity and action tie-breaking use canonical episode/action
order; environment arrival order cannot alter selection.

Verifier impact: held-out prediction and relational proof verification remain
independent. The experiment oracle is never trusted as a verifier.

Downstream interfaces: additive `cairn-world`, `cairn-runtime`, and WP34/G4-G6
integration APIs; no ARC adapter change.

## Validation

Tests: automatic resolution of the existing two-state terminal ambiguity,
exact held-out histories, action/before/out-of-bounds corruption, absent
one-step discriminator, and intervention cap. Compare selected interventions
with a frozen non-informed action order.

Benchmark gate: recover the exact two-state/six-rule theory, use fewer
continuations than the non-informed baseline, predict all held-out transitions,
plan through latent state, lower learned rules to relational evidence, and
verify unchanged persistent mechanisms on the transferred construction.

Rollback trigger: the learner sees latent truth, malformed continuation is
accepted, selection depends on map/scheduler order, an unresolved theory is
reported complete, or ARC-specific concepts enter core code.

## Compatibility

Semantics version change: additive in-process interfaces; existing latent-world
semantics 1.1.0 and persisted formats are unchanged.

Migration: none.

## References

- ADR-0019 relational observation lowering
- `docs/experiments/FIVE_LATENT_EXPERIMENTS.md`
- `08_GENERAL_LEARNER_REQUIREMENTS.md`
