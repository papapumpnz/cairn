# ADR-0034 — Persist relation-bound grouping mechanisms by verified reuse

Status: accepted

## Context

ADR-0033 lets the online learner choose a relation-defined partition jointly
with an executable group aggregate, but only the aggregate previously crossed
the memory boundary. Discarding the selected relation forces later tasks to
repeat ontology selection and prevents evidence-backed attribution of reuse.

## Decision

The canonical `cairn.grouping-merge.v1` identity binds the environment-neutral
`RelationRole` and exact RKL definition. Source episodes and search telemetry
remain non-semantic. The online learner may insert this composition only as a
provisional proposal.

An independent verifier lowers the selected relation from fresh evidence,
recomputes aggregate inputs, executes raw RKL, checks complete unordered
outputs, and issues an opaque reuse receipt. A separate consolidator requires
at least two disjoint receipts for the same semantic identity and strict
candidate-execution reduction. Only its certificate can make the memory module
persistent. Runtime retrieval rebinds the relation and emits an untrusted
proposal; it cannot accept an answer.

## Alternatives rejected

- Persist only the aggregate: loses the learned ontology decision.
- Let the store promote after receipt count: collapses consolidation authority
  into memory and cannot certify measured learning reduction.
- Serialize source candidate maps into identity: memorizes experience rather
  than a reusable mechanism.

## Compatibility and migration

This is an additive public contract. Existing `cairn.group-merge.v1` modules
remain valid but do not imply a grouping relation. No automatic migration is
permitted. The new in-memory lifecycle is not yet a durable CAS schema.

## Validation

Two independently learned cross-domain cohorts must have the same canonical
identity. Two verifier receipts must be necessary for promotion. Changed
values, counts, cardinalities, and task identities must reuse with zero new
examples; blank memory, duplicate evidence, and a changed relation identity
must fail closed.

## Rollback and kill conditions

Revert the new store and runtime entry point if the verifier imports synthesis,
if a relation change preserves identity, if promotion succeeds without two
disjoint receipts, or if memory directly accepts a prediction.
