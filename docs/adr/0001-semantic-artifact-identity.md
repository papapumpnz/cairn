# ADR-0001: Separate semantic artifact identity from observation metadata

Status: accepted

Date: 2026-07-10

Owners: WP01 integration owner

## Context

The first-pass interface sketch says an artifact ID hashes the canonical
envelope without its signature. That envelope also contains producer, consumed
budget, and logical creation metadata. The repository law separately requires
identical semantic values to have identical canonical bytes and IDs, and marks
creation metadata as non-semantic. Hashing observation metadata into identity
would make the same proven theory acquire different IDs when independently
replayed, defeating deterministic swarm deduplication.

## Decision

CAIRN defines two hashes:

- `payload_hash` is BLAKE3 over the canonical CBOR payload bytes.
- `artifact_id` is BLAKE3 over a canonical semantic identity tuple containing
  protocol version, schema name and version, semantics version, artifact kind,
  sorted unique parent IDs, optional task scope, payload hash, and payload
  length.

Producer, actual resource consumption, logical creation sequence, annotations,
and signature are authenticated envelope metadata but are excluded from
semantic identity. Multiple observations of the same artifact may therefore
share one artifact ID while retaining distinct signed metadata envelopes.

## Alternatives considered

### Hash every envelope field

Benefits: literal reading of the early Rust sketch.

Costs: destroys identity across independent replay and makes timestamps or
worker identity semantic.

Reason rejected: conflicts with deterministic deduplication and the repository
artifact law.

### Identify artifacts by payload hash alone

Benefits: simplest identity rule.

Costs: loses schema, semantics version, parent lineage, kind, and task scope.

Reason rejected: identical bytes can have different typed meaning or lineage.

## Consequences

Correctness: identity is stable across producers while remaining scoped by
schema, semantics, kind, lineage, task, and payload length.

Expressivity: no reduction.

CPU cost: one additional small canonical tuple hash per envelope.

Memory cost: two 32-byte hashes per envelope.

Determinism: improved across processes and worker counts.

Verifier impact: verifiers reconstruct both hashes and validate metadata
separately.

Downstream interfaces: WP03 and later artifacts must not use producer or time as
semantic keys.

## Validation

Tests: cross-producer identity equality, metadata tampering diagnostics, parent
order invariance, payload corruption, and schema/version rejection.

Benchmark gate: 10 million small canonical round trips and CAS throughput gate.

Rollback trigger: a verifier requirement demonstrates that excluded metadata
changes executable artifact meaning. Such a change requires a semantics-version
migration and a superseding ADR.

## Compatibility

Semantics version change: establishes artifact identity semantics `1.0.0`.

Migration: none; no prior implementation artifacts exist.

## References

- `03_CAIRN_INTERFACES.md`, section 2
- `AGENTS.md`, sections 3.9 and 11
- `WP01.md`

