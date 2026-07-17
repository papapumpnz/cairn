# ADR-0054: use scalar evidence to discriminate expression association

Status: accepted

## Context

ADR-0052 stopped an unconditional expansion from left-associated expressions to
all bounded binary-tree shapes because the unchanged relation-grouping family
became ambiguous. A new content-free curriculum objective was then generated
from the ambiguity class and parameter arity. The first independently authored
contrast still failed to distinguish association: equality relations preserve
only a partition, and a different tree with a different anonymous-channel
binding reproduced the same partition exactly.

## Decision

Add an explicit scalar-expression evidence channel. Scalar examples retain
their exact integer result, source channels remain anonymous, and both the tree
and a task-local channel binding are learned. Full binary-tree enumeration is
reachable only through an opaque, content-free association-discrimination
request and a separately checked shape-extension certificate. Historical
left-associated defaults remain unchanged.

The learned scalar AST lowers to typed, bounded RKL. A separately implemented
verifier reconstructs the definition and exact integer evaluation. Two
disjoint verifier receipts are required before a consolidator can promote a
source-independent mechanism into proposal-only memory. Runtime memory may
rebind the mechanism and emit values, but cannot accept them.

## Alternatives rejected

- Select the cheapest relation tree: this would conceal exact observational
  equivalence rather than learn from distinguishing evidence.
- Add source-channel order to mechanism identity: this would prevent rebinding
  and cross-frontend transfer.
- Treat the curriculum request as an expression template: it may expose only a
  structural search class, never an operator tree or answer.
- Store a learned output function without RKL: this would violate executable
  ontology and independent replay requirements.

## Compatibility and migration

`RelationExpressionLimits` gains `RelationExpressionShapePolicy`, defaulting to
`LeftAssociated`; existing callers retain their prior semantics. Scalar
mechanisms use a separate canonical identity marker,
`cairn.scalar-expression.v1`, and do not change
`cairn.relation-expression.v1`. There is no durable scalar-memory schema in
this change, so no artifact migration is required.

## Validation and verifier impact

The frozen non-ARC scalar family must reject the left-only language, identify
one full-tree expression in two independent cohorts, pass independent replay,
reduce expression search, and predict changed held-out values through the
shared runtime. Wrong-arity and content-bearing requests fail closed. The
verifier independently counts full-tree structures using a Catalan recurrence
and never imports producer enumeration.

## Rollback and kill conditions

Revert the full-tree extension if it changes default reachability, if producer
and verifier counts diverge, if more than one exact source-independent scalar
expression survives the frozen contrasts, or if memory can emit an accepted
answer without current-task verification. Preserve any such result as a
negative experiment rather than introducing a tie-break.

