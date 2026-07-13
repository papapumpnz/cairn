# ADR-0038 — Invent conjunctive pair relations from downstream explanation

Status: accepted

## Context

ADR-0037 can invent equality over one anonymous entity-feature slot. Many
collection concepts require several conditions simultaneously. Selecting a
single observed feature therefore leaves the learned ontology unable to express
even a compound key without frontend-authored relation edges.

## Decision

Extend the bounded pair-relation hole with canonical conjunctions of equality
atoms over corresponding anonymous feature slots. Enumerate candidates by
increasing conjunction width and lexicographic feature index, subject to
explicit `maximum_conjuncts` and `maximum_relation_candidates` caps. For every
candidate, execute its RKL body, build complete connected components, and admit
it only when an independently learnable aggregate explains all unordered
outputs. Rank exact theories by aggregate cost plus the complete relation AST
cost; equal minimum costs remain ambiguous.

The learned artifact remains `cairn.predicate-grouping-merge.v1` because its
semantics already bind arbitrary executable relation and merge definitions.
This changes proposal coverage, not accepted artifact semantics. The verifier
continues to execute the raw Boolean RKL body through its separate component
implementation and does not import candidate enumeration.

## Alternatives rejected

- Supply compound pair labels or partitions: this would leak the ontology
  answer into evidence.
- Persist feature-index tuples instead of RKL: this would retain configuration,
  not executable semantics.
- Add a named compound-key primitive: this would grow a domain operation list.
- Search arbitrary global RKL syntax: the event-first contract permits bounded
  typed-hole search, not global program enumeration.

## Compatibility and migration

`PredicateGroupingLimits` gains `maximum_conjuncts`; callers using `Default`
receive a bound of two. Existing v1 artifacts, equality-only mechanisms, and
verifier receipts remain valid. Explicit struct literals must select a bound.

## Validation

Use two independent non-ARC cohorts where equality on either relevant feature
merges unrelated entities and equality on a distractor splits valid groups.
Only the two-atom conjunction may explain the outputs. Require canonical
agreement, changed held-out values/cardinalities/identities, order invariance,
11/11 independent component replay, single-conjunct ablation rejection, and
typed capacity termination.

## Rollback and kill conditions

Revert if an atomic predicate passes the frozen family, candidate order changes
the learned identity, candidate caps are ignored, an ablated conjunct still
replays, or verifier acceptance depends on producer-created components.
