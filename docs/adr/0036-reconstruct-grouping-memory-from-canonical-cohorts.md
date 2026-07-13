# ADR-0036 — Reconstruct grouping memory from canonical raw cohorts

Status: accepted

## Context

ADR-0035 learns and promotes relation-bound grouping mechanisms from raw edges,
but the store is process-local. Serializing a module's persistent status,
verifier receipts, or promotion certificate would let stale authority survive
without current semantics, lineage, and independent replay.

## Decision

Persist canonical `cairn.memory.raw-grouping-cohort.v1` artifacts in the
append-only CAS. Each artifact contains sorted raw traces, expected outputs,
the claimed semantic mechanism identity, replay count, and counterfactual work
measurements. It carries explicit parent lineage and a cohort-scoped hash.
Producer identity, host run, resource observations, and logical creation time
remain non-semantic metadata.

Loaded cohorts are untrusted. Restart reconstruction must:

1. schema-, kind-, semantics-, hash-, scope-, bound-, and lineage-check them;
2. re-invent the grouping and RKL merge from their raw traces;
3. independently reconstruct relation components and replay outputs;
4. compare mechanism identity and all measurements to the commitment;
5. generate fresh opaque receipts;
6. reconsolidate disjoint cohorts; and
7. create a new provisional store before applying the new certificate.

No status, receipt, certificate, or executable native state is deserialized.

## Alternatives rejected

- Serialize `GroupingMergeStore`: trusts lifecycle state across semantics and
  verifier changes.
- Serialize only executable RKL: loses evidence needed to reproduce learning.
- Restore a stored promotion certificate: bypasses fresh independent replay.

## Compatibility and migration

This adds schema `cairn.memory.raw-grouping-cohort` version 1 with semantics
1.0.0. Existing in-memory modules have no automatic migration because their
source evidence may be unavailable. They must be relearned and archived.

## Validation

Require content identity under edge reordering and non-semantic metadata
changes, physical idempotence under identical writes, complete process-state
loss, CAS recovery, two-cohort reinvention, fresh receipts, repromotion,
held-out transfer, and rejection of missing parents, wrong kind, caps, scope,
hash, schema, or semantics.

## Rollback and kill conditions

Revert the archive if any persistent status survives directly, if loaded bytes
can propose without reinvention/replay, if metadata changes semantic identity,
or if malformed lineage or unsupported semantics fail open.
