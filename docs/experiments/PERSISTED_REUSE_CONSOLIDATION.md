# Persisted reuse evidence and restart consolidation

Date: 2026-07-12

Status: WP23 persistence checkpoint pass

## Question

Can CAIRN preserve evidence that a learned mechanism reduced future learning
cost, discard all process-local verifier receipts, restart, and still promote
the mechanism without trusting stored counters?

## Architecture

Memory semantics 1.5.0 introduces canonical schema
`cairn.memory.reuse-evidence` version 1. Each artifact binds:

- canonical checked-theory hash;
- experience hash and experience-scoped envelope;
- parent evidence artifact IDs;
- independently verified case count;
- memory-off/on candidate executions;
- memory-off/on examples;
- memory-off/on proof depth.

Only an opaque verifier receipt can create the typed payload. CAS loading checks
content, envelope, schema, semantics, scope, lineage presence, and exact payload
shape, but does not restore trust. Restart replay receives the checked theory,
referenced problem, held-out result, and freshly measured counterfactual metrics.
The independent verifier re-executes behavior and returns a new opaque receipt.
Every new field must match the persisted claim before consolidation.

Persisted metrics are never passed back as their own replay measurements.

## Frozen restart experiment

Two independent collection experiences use the same checked quantified theory.
The run persists both receipts, verifies idempotent publication and semantic-ID
stability under changed producer/resource metadata, discards the receipts,
reopens and recovers the CAS, reloads both artifacts, replays both experiences,
and promotes a newly reconstructed provisional module.

```text
theory: ec2041abcb36451e65baf9226d3b6732f4ef7198f147cd8dc24b3df5c1482bc8
artifact 1: 62ffff008f482f99a16c3718e3ce8cc4fd2938df35edd9d8d1f6810b6e57cc1c
artifact 2: fd19bd3ef768c5e38e21dfcbd9ea4f2898c85c6302379e3f4e66eeeb4c7b42e4
persisted receipts replayed: 2/2
promotion after restart: 1
candidate executions: 200 -> 2
examples: 8 -> 2
proof nodes: 40 -> 6
negative transfers: 0
```

The cost values are a deterministic synthetic gate, not a wall-clock speed
claim.

## Negative tests

- missing parent lineage rejects before persistence;
- malformed payload marker rejects on load;
- changed checked-theory identity rejects before replay;
- changed held-out result fails verifier replay;
- changed freshly measured metrics fail artifact comparison;
- duplicate byte publication converges on the same blob;
- changed non-semantic producer/resource metadata preserves semantic artifact
  identity.

## Claim boundary

This closes the process-local reuse-ledger limitation identified by the initial
G1 experiment for procedural mechanisms. It makes one self-compression evidence
path durable and independently reconstructable. It does not yet persist proof
DAGs themselves, extract arbitrary internal proof subgraphs, automatically
deprecate mechanisms from counterexamples, or establish open-ended general
intelligence.
