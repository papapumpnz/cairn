# ADR-0040 — Invent order relations and normalize unordered pair direction

Status: accepted

## Context

Equality conjunctions cannot express relational geometry such as enclosure.
Directional inequalities can, but grouping evidence consists of unordered
entity pairs. A relation and its complete left/right reversal therefore denote
the same undirected adjacency and must not create false ambiguity.

## Decision

Optionally extend the bounded pair-relation atom set with integer less-than
comparisons between every anonymous left/right feature pair in either
direction. Keep this family disabled by default and enable it explicitly under
the existing conjunction/candidate caps.

For collection formation, execute a learned directional body in both entity
orders and join the pair when either execution is true. Canonicalize each body
against its complete left/right variable reversal and retain the smaller full
definition encoding. Deduplicate identical canonical definitions before
downstream explanation. Producer and verifier implement the two-order execution
separately.

## Alternatives rejected

- Name `contains` or `enclosed_by`: supplies the concept being tested.
- Treat left/right reversal as two answer modes: they have identical unordered
  denotation.
- Break equal-cost ties by enumeration order: makes evidence ambiguity look
  like learning.
- Enable order atoms universally: expands search where observations do not
  justify it.

## Compatibility and migration

`PredicateGroupingLimits` gains `include_order_comparisons`, defaulting to
false. Existing equality mechanisms retain their identity and behavior. The
unordered grouping semantics are now explicit for directional definitions;
no persisted directional predicate existed before this decision.

## Validation

Use anonymous interval endpoints with crossing hard negatives. No atomic
comparison or same-direction shortcut may explain all outputs. Require the
opposing-inequality enclosure body, orientation-normalized identity across two
cohorts, 31 unique bounded candidates, 11/11 changed held-out components, and
entity-order invariance through independent verification.

## Rollback and kill conditions

Revert if reversal changes semantic identity, candidate ordering changes the
result, a named geometry concept enters the learner, the candidate cap is
ignored, or producer-only component construction is trusted.
