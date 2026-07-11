# Persisted proof-graph consolidation

Date: 2026-07-12

Status: WP23 proof-source persistence checkpoint pass

## Question

Can CAIRN discard process-local proof DAGs, restart, and discover exactly the
same compressed mechanism proposal from canonical artifacts while retaining an
independent verifier boundary?

## Frozen experiment

Three independent typed proof DAGs share this causal structure:

```text
observe entities
→ filter by a relation
→ map an effect
→ render
```

Entity role and effect value differ across episodes. Each graph is encoded,
decoded, stored with source-certificate lineage, and published idempotently.
The CAS is closed, reopened, and recovered. A new `ConsolidationRuntime` loads
the three graphs as untrusted artifacts, admits them through normal topology
checks, and consolidates them without family labels.

```text
proof artifact 1: f0dba935efb9617bfea0d81efcce24bca487a3b2072ab7a4dd1a67c680d01b13
proof artifact 2: 40f56a99fff95e941074e6ac28c1dd0308c9c224c1f241ce1a47dc1cbe9f21ec
proof artifact 3: 2cc3dc16cf4445f4aab689ed6b0fbe7a77d06bf97a2636aad8866482d85bcb94
fresh proposal:     1c2edf4bdea47cd31d30e967f2742804ee51fa7ca711d6908967e603af0f4eca
restart proposal:   1c2edf4bdea47cd31d30e967f2742804ee51fa7ca711d6908967e603af0f4eca
source replays: 3/3
```

`CompressionVerifier`, which does not import synthesis or memory, independently
reconstructs the proposal against the reloaded source DAGs and recalculates its
costs.

## Negative evidence

- missing parent lineage rejects before persistence;
- self/forward references reject as invalid topology;
- malformed payload markers reject during bounded decoding;
- existing verifier tests reject changed source content and episode identity;
- duplicate publication converges on the same blob.

## Claim boundary

This makes the proof-source side of bounded self-compression restartable. It
does not yet extract arbitrary repeated internal subgraphs from differently
shaped proofs, persist compiled module lifecycle state, or demonstrate a
long-running autonomous curriculum that continually grows the library.
