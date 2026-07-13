# ADR-0039 — Promote invented predicate/aggregate compositions by reuse proof

Status: accepted

## Context

ADR-0038 invents a compound pair predicate, but task-local rediscovery does not
establish continual learning. Persistence must cover the executable predicate
and aggregate together, require independent current-task replay, and remain a
proposal source in the shared runtime.

## Decision

Add opaque predicate-grouping reuse receipts, a separate consolidation token,
and `PredicateGroupingStore`. A receipt binds mechanism identity, disjoint
experience identities, replayed groups, and strict candidate-execution
reduction. Two or more non-overlapping receipts are required. The store becomes
persistent only when the consolidator token exactly covers its complete local
ledger.

`CognitionRuntime` owns the store and may execute persistent definitions over
current attributed entities only as untrusted proposals. It has no acceptance
authority; current-task verification remains separate. Duplicate or overlapping
evidence cannot satisfy promotion.

## Alternatives rejected

- Promote after successful synthesis: confuses task learning with transfer.
- Reuse the relation-role store by manufacturing a role: loses the invented
  executable predicate identity.
- Let memory emit accepted outputs: violates the memory and verifier boundary.
- Count repeated replay of one cohort twice: supplies no independent evidence.

## Compatibility and migration

The change is additive. Existing stores and artifacts are unchanged. Predicate
memory is initially process-local; no serialized status or migration is added.

## Validation

Require two independently learned conjunctive cohorts, identical canonical
mechanism identity, two disjoint verifier receipts, rejection with one receipt,
duplicate-receipt rejection, exact certificate/ledger coverage, runtime
proposal-only use, blank-memory failure, and a 65-to-one candidate/four-to-zero
example reduction on changed held-out entities.

## Rollback and kill conditions

Revert if one cohort promotes, duplicate evidence counts twice, a certificate
for incomplete evidence applies, blank memory proposes, or the runtime accepts
without a separate verifier boundary.
