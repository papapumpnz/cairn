# ADR-0044 — Reconstruct observation-feature memory from canonical evidence

Status: accepted

Date: 2026-07-13

Owners: WP34 integration

## Context

ADR-0043 demonstrated proof-gated process-local reuse of synthesized feature
programs. A process-local store is insufficient for continual learning, while
serializing module status, verifier receipts, promotion certificates, or learned
native objects would move untrusted historical authority across a restart.

## Decision

Add a content-addressed observation-predicate cohort artifact. It stores only
canonical primitive evidence, the claimed semantic commitment, replay metrics,
and mandatory parent lineage. It stores no learned definition, status, receipt,
certificate, or compiled implementation.

After restart, loading validates the envelope, schema, semantics, hash, scope,
lineage, dimensions, duplicate identities, and canonical ordering. Replay runs
the inventor from blank authority, requires the same semantic identity and work
accounting, and asks the independent verifier for a new reuse receipt. Two
disjoint replayed cohorts are required for a new consolidator certificate and
promotion.

`CognitionRuntime` owns the resulting proof-gated store. Retrieval executes a
persistent mechanism only as a proposal; current evidence still requires
independent acceptance replay.

## Consequences

Evidence survives process loss without restoring trust. Canonical entity and
episode reordering produces the same artifact identity. Corruption, absent
parents, unsupported semantics, capacity failure, changed invention identity,
or replay-work mismatch fail closed.

The restart path costs full re-invention and verification by design. A later
checkpoint may cache compiled definitions, but only as equivalence-checked
optimizations after fresh selection.

## Validation

Two independently learned cohorts are persisted, every authority-bearing value
and CAS handle is destroyed, the CAS is reopened and recovered, both mechanisms
are re-invented and independently replayed, and a new certificate re-promotes
one module. Changed held-out data is then solved exactly. Stored status restored
is zero.

## Compatibility

The initial archive used observation-predicate cohort schema version 1 and
feature semantics 2.0.0. ADR-0045 immediately supersedes its fixed-arity payload
with schema version 2 and semantics 3.0.0. Version 1 fails closed and must be
reconstructed from source evidence.

## Rollback or kill conditions

Stop if loading restores authority, replay imports producer search, identity is
not canonical under ordering, missing lineage is accepted, or the runtime can
accept a memory result without independent current-evidence verification.
