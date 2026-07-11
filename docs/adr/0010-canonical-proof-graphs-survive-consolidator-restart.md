# ADR-0010: Canonical proof graphs survive consolidator restart

Status: accepted

Date: 2026-07-12

Owners: WP23 integration

## Context

ADR-0009 makes actual-reuse receipts durable, but `ConsolidationRuntime` still
retained its source `ProofGraph` values only in process memory. A restart could
therefore replay reuse evidence for an already known theory but could not
reconstruct the proof cohort from which a new reusable mechanism is discovered.
WP23 names canonical proof-graph persistence as unfinished.

## Decision

Introduce `cairn.consolidator.proof-graph` schema version 1, artifact semantics
1.0.0, and consolidator semantics 2.1.0. The canonical payload contains an
episode identity, a topologically ordered typed node sequence, opaque typed
argument hashes, and one output node. It never contains a complete observation,
answer, benchmark identity, or native executable code.

`ProofGraphArchive::persist` requires parent lineage and validates the complete
ancestral DAG before writing. `load` performs bounded canonical envelope and
payload decoding, schema/semantics negotiation, scope checking, type-code
checking, and another topology validation. A restarted consolidator must submit
the loaded graphs through its ordinary admission path. Its resulting proposal
remains untrusted until `CompressionVerifier` independently reconstructs all
source graphs and costs.

## Alternatives considered

### Serialize `ConsolidationRuntime`

Benefits: captures maps and limits in one snapshot.

Costs: persists private state, cohort decisions, and possibly future caches as
authority; obstructs independent source replay.

Reason rejected: source evidence should be immutable and runtime state should
be reconstructible.

### Persist compression proposals only

Benefits: smaller storage and faster restart.

Costs: removes the source graphs required for independent falsification and
cost reconstruction.

Reason rejected: a proposal cannot certify its own compression claim.

## Consequences

Correctness: malformed, cyclic, forward-referencing, empty, and invalid-output
graphs fail before consolidation.

Expressivity: the artifact carries all existing `ProofValueType` variants and
opaque values without adding a domain operation.

CPU cost: restart decodes and validates each graph, then repeats consolidation
and independent verifier replay.

Memory cost: payload loading is bounded to 8 MiB and 300,000 collection items;
runtime admission retains its stricter configured node and episode limits.

Determinism: canonical bytes and semantic IDs are independent of producer and
resource observations; restarted consolidation produces the same proposal hash.

Verifier impact: `cairn-verifier` gains no CAS production dependency. Its test
reloads sources through the public archive and uses the existing independent
compression verifier.

Downstream interfaces: WP23 adds `ProofGraphArchive`,
`ProofGraphArtifactMetadata`, `ProofGraphDecodeError`, and
`ProofGraphArchiveError`.

## Validation

Tests: canonical round trip, idempotent CAS publication, CAS reopen/recovery,
restart proposal equality, independent compression verification, missing
lineage, cyclic topology, and malformed payload.

Benchmark gate: no speed claim; exact restart equivalence is the gate.

Rollback trigger: changed proposal identity after restart, verifier disagreement,
or any loaded graph bypassing ordinary consolidation admission.

## Compatibility

Semantics version change: consolidator 2.0.0 to 2.1.0; proof-graph artifact
semantics begin at 1.0.0.

Migration: no previous persisted proof-graph schema exists. Process-local
graphs must be re-emitted through the archive to become restart evidence.

## References

- ADR-0007 self-compressing mechanism learner
- ADR-0009 persisted reuse evidence requires replay
- `docs/CAIRN_handoff/work_packages/WP23.md`
