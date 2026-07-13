# ADR-0035 — Invent grouping hypotheses from raw relation edges

Status: accepted

## Context

ADR-0034 persists a relation-bound grouping mechanism, but the online learner
still receives complete candidate partitions. That representation hides the
decision that relation edges form collections and prevents isolated entities,
malformed endpoints, and edge ordering from being tested at the theory boundary.

## Decision

Add a bounded environment-neutral input containing opaque `EntityId` scalar
evidence and unselected `RelationRecord` edges. For every observed
`RelationRole`, the exact reference proposer constructs weakly connected
components, includes isolated entities as singleton components, and lowers the
result to the existing joint grouping/theory selector. Entity, edge, and role
limits are explicit. Unknown endpoints and duplicate edges fail closed.

The independent verifier separately validates the raw envelope, reconstructs
the selected components without invoking the producer, recomputes aggregate
evidence, and executes raw RKL. Runtime memory accepts raw edges and emits only
untrusted proposals.

## Alternatives rejected

- Continue supplying partitions: does not test relation-to-collection
  construction.
- Treat edge direction as grouping semantics: makes incidental frontend order
  alter entity membership.
- Import the producer's component builder into the verifier: violates the
  independent replay boundary.

## Compatibility and migration

The change is additive. Existing partition-based interfaces remain valid. Raw
evidence lowers into them through a deterministic reference path. The learned
mechanism identity remains `cairn.grouping-merge.v1`; source topology is not
embedded in persistent identity.

## Validation

Require edge-order and direction invariance, isolated-entity coverage, two hard
competing roles, two independent learning cohorts, consolidator promotion,
changed topology/cardinality transfer, blank-memory failure, and independent
rejection of unknown, duplicate, and capped evidence.

## Rollback and kill conditions

Revert if component construction drops or duplicates an entity, if ordering
changes semantic output, if the verifier trusts producer partitions, or if any
frontend/benchmark role is introduced into core logic.
