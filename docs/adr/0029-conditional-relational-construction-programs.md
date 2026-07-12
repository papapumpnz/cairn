# ADR-0029: conditional relational construction programs

Status: accepted for WP34 implementation

## Context

ADR-0028 learns a source-independent coordinate program from unordered
source/output point sets, but its executor maps every supplied entity exactly
once.  CAIRN already learns executable predicates over environment-neutral
entity attributes and relation-role degrees.  Leaving selection and
construction disconnected prevents learned theories from explaining changing
output cardinality and makes reuse appear more general than it is.

## Decision

Introduce a separate, versioned conditional construction artifact that
composes:

1. an invented Boolean RKL selector and its explicit relational feature
   contract; and
2. a v2 executable construction program with one tuple-valued point mapping.

Training observations contain a generic relational entity collection aligned
by content identity with a geometric entity collection plus an unordered
expected render.  The learner infers selected source entities from preserved
categorical evidence, invents the selector from positive and negative
bindings, filters each source collection, and invents geometry only from the
selected collections.  Held-out execution receives no selected set and no
expected output.

The joint point definition is inferred against unordered complete point tuples,
avoiding false coordinate combinations admitted by independent axis sets. The
v1 unconditional program and its canonical identity remain unchanged.
Conditional programs use a new canonical semantic identity. Independent
verification re-evaluates relational features and executes the raw Boolean and
coordinate definitions without calling either inventor.  Persistent memory
requires two disjoint independently verified cohorts and remains
proposal-only.

## Alternatives

- Adding an optional selector to the v1 program was rejected because it would
  silently change existing canonical semantics.
- A named `filter` or geometry-specific selection operator was rejected because
  it would close the learned language around a developer-supplied task family.
- Accepting supplied selected IDs at held-out execution was rejected because it
  would not demonstrate learned conditional control.

## Compatibility and migration

Existing v1 programs, receipts, memory modules, and hashes remain valid.  The
new artifact is additive and cannot be decoded as v1.  No ARC adapter or
benchmark concept enters the contract.

## Validation

The frozen non-ARC family varies domains, entity order, identities, categorical
values, relation graph size, cardinality, layout, and geometry.  Acceptance
requires exact two-cohort replay, zero-example held-out reuse, lower candidate
and sample work, cross-domain rebinding, blank-memory failure, selector
ablation failure, and rejection of changed relational truth.

## Rollback and kill conditions

Stop this path if the selected set is encoded by identity or fixed value, if
the verifier imports synthesis, if output cardinality does not vary, if
relation removal does not cause the predicted failure, or if a frozen
hard-negative family is accepted.  Preserve the negative result and do not run
the ARC contribution gate.
