# ADR-0043 — Synthesize observation feature programs from typed RKL

Status: accepted

Date: 2026-07-13

Owners: WP34 integration

## Context

ADR-0042 removed frontend-supplied endpoint normalization, but its learner chose
between exactly two developer-authored feature sets: raw observations and
minimum/maximum. That establishes bounded selection, not an expandable feature
language. The next general-learning gate requires the structure of a useful
feature expression to be constructed from the universal substrate.

## Decision

Replace the two feature templates with deterministic cost-ordered enumeration
of typed integer RKL expressions over anonymous observations. The initial
grammar contains variables, negation, addition, subtraction, multiplication,
minimum, and maximum. Generic literals remain a future grammar extension. It has
explicit expression-cost, candidate, feature-count, entity-count, and feature-
set caps.

Observationally equivalent programs are collapsed only within the current
training evidence. One or more feature programs are jointly selected with the
existing output-driven predicate and aggregate learner. Equal-cost distinct
exact mechanisms remain an ambiguity terminal. Persistent identity binds the
complete executable feature-program collection and downstream mechanism under
the v2 observation-predicate semantic marker.

The independent verifier receives executable definitions, never the producer's
search state, and separately type-checks and executes them before relation and
aggregate replay. Memory remains proposal-only.

## Alternatives considered

### Add more named feature templates

Benefits: smaller and faster search.

Costs: every capability remains developer-selected and the language cannot
expand compositionally.

Reason rejected: it repeats the architectural failure identified in ADR-0042.

### Expose private `cairn-invent` search internals

Benefits: less local enumeration code.

Costs: couples joint latent-feature search to an API designed for supervised
input/output mappings and changes a dependency-owned contract.

Reason rejected: the observation features have no supplied target values; they
must be selected by explanatory utility downstream.

## Consequences

Correctness: every retained feature is typed RKL and exact-executed.

Expressivity: nested arithmetic feature structure can be invented without a
named template, within explicit bounds.

CPU cost: candidate and feature-set enumeration grows combinatorially and must
terminate with `Capacity` at the declared caps.

Memory cost: bounded behavioral vectors are retained only for one invention
call.

Determinism: structural cost and canonical bytes define stable ordering.

Verifier impact: artifact identity changes to observation-predicate v2; replay
logic remains search-independent.

Downstream interfaces: feature definitions become a variable-length vector and
search metrics split candidate-program work from feature-set work.

## Validation

Tests: retain min/max enclosure invention; add an omitted addition-feature
family, capacity negatives, feature-body ablation, independent replay,
cross-adapter transfer, and proof-gated reuse.

Benchmark gate: a feature structure absent as a named template must transfer to
changed values, cardinalities, orientation, and another adapter with exact
replay. Memory must strictly reduce candidate and example work.

Rollback trigger: rollback if a domain label enters feature search, a frontend
supplies the selected derived feature, capacity is not typed, equal-cost
ambiguity is silently resolved, or verifier replay imports producer search.

## Compatibility

Semantics version change: observation-predicate semantic marker v1 to v2.

Migration: v1 learned artifacts fail identity matching and must be re-invented
from canonical evidence; no trusted promotion status is migrated.

## References

- ADR-0040
- ADR-0041
- ADR-0042
- `08_GENERAL_LEARNER_REQUIREMENTS.md`, G0, G1, G2, and G5
