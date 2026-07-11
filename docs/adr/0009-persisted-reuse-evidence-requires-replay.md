# ADR-0009: Persisted reuse evidence requires independent replay

Status: accepted

Date: 2026-07-12

Owners: WP23 integration

## Context

The initial G1 suite records actual mechanism invocation, necessity under
ablation, candidate work, examples, and proof depth. Those records lived only
inside `MechanismStore`. Process loss therefore discarded the scientific
evidence, and a later consolidator could not distinguish a reconstructed result
from a caller-authored claim. WP23 explicitly lists canonical CAS persistence
as unfinished.

## Decision

Introduce schema `cairn.memory.reuse-evidence` version 1 and memory semantics
1.5.0. A reuse artifact binds the canonical theory hash, experience hash,
verified-case count, and frozen memory-off/on metrics. Its envelope is scoped to
the experience and must contain at least one parent evidence artifact.

Only an opaque `VerifiedProceduralReuse` may create the typed payload. Loading
checks CAS integrity, envelope identity, schema, semantics, lineage presence,
scope, and exact payload shape. Loading never restores trust. Before a receipt
may enter a ledger or promotion cohort, `PromotionVerifier` re-executes the
checked theory against the referenced problem and expected held-out result,
consumes freshly measured memory-off/on metrics, and requires every
reconstructed field to equal the persisted claim. Persisted metrics never serve
as their own replay input.

The consolidator continues accepting verifier receipts, not artifacts. This
keeps storage outside the verifier and makes restart behavior equivalent to a
fresh verification run.

## Alternatives considered

### Serialize the in-memory store

Benefits: minimal code and fast restart.

Costs: couples persistence to private layout, trusts stored status and counters,
and cannot independently reconstruct promotion evidence.

Reason rejected: it violates the verifier and immutable-artifact boundaries.

### Treat CAS identity as proof

Benefits: avoids replay cost.

Costs: content addressing proves immutability, not truth or verifier authorship.

Reason rejected: an untrusted producer can hash a false claim correctly.

## Consequences

Correctness: persisted evidence is proposal-only and must replay exactly.

Expressivity: no new mechanism or RKL construct is introduced.

CPU cost: restart incurs deterministic exact replay before promotion.

Memory cost: each payload is bounded to 2 KiB before envelope overhead.

Determinism: semantic artifact identity excludes producer, resource, and logical
time metadata and is stable for identical evidence and parents.

Verifier impact: no storage dependency enters `cairn-verifier`; it receives the
same checked theory and evidence as before.

Downstream interfaces: WP23 adds `ReuseEvidenceArtifact`,
`ReuseArtifactArchive`, `ReuseArtifactMetadata`, and `ReuseArchiveError`.

## Validation

Tests: idempotent persistence, CAS reopen/recovery, two-experience replay,
promotion after process-local receipt loss, missing lineage, malformed payload,
changed theory, and changed expected behavior.

Benchmark gate: no speed claim; the artifact is small and replay correctness is
the gate.

Rollback trigger: any path that promotes directly from loaded bytes, or any
semantic difference between fresh and restart consolidation.

## Compatibility

Semantics version change: `cairn-memory` 1.4.0 to 1.5.0; reuse artifact semantics
start at 1.0.0.

Migration: no prior persisted reuse schema exists. Existing process-local
ledgers remain usable but are not restart evidence until re-verified and
persisted.

## References

- `docs/experiments/G1_ACTUAL_REUSE_LEDGER_PASS.md`
- `docs/CAIRN_handoff/work_packages/WP23.md`
- ADR-0001 semantic artifact identity
- ADR-0007 self-compressing mechanism learner
