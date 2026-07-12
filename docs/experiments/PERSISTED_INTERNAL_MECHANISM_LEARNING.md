# Persisted internal mechanism learning

Date: 2026-07-12

Status: measured restart/replay result

## Question

Can CAIRN lose all process-local internal-learning authority, restart from
canonical evidence, and independently reconstruct the same learned mechanism,
reuse evidence, and promotion outcome?

## Procedure

The frozen source cohort contains four differently shaped proof hosts. Two
changed hosts supply runtime reuse. CAIRN persists all six proof graphs and two
receipt commitments, closes the CAS, and abandons the process-local proposal,
compiled schema, bindings, verifier receipts, and store.

After recovery it reloads proof graphs as untrusted input, remines, recompiles,
reverifies, rematches both held-out hosts, and independently regenerates every
receipt field. Only reconstructed opaque receipts enter a fresh store.

## Command

```sh
cargo test -p cairn-memory --test persisted_internal_reuse -- --nocapture
```

## Results

```text
mechanism: blake3:a8438381973c8205112a6017ff7fd747009154990fe1e461ff67ec9a3321d781
proposal: blake3:f6464e545db3f71f658badc05cdd45df56e7bf9ba5338651769d2dfd00466739
proof_artifacts: 6
receipt_artifacts: 2
loaded_receipts_replayed: 2/2
promotion_after_first_replay: false
promotion_after_second_replay: true
candidate_windows: 18 -> 9, 18 -> 5
examples: 4 -> 1
proof_units: 28 -> 4
```

Receipt artifact identities:

```text
67bda5fb5325cb98723f2a30f0eba08b61bfe6c2417baacee98c010b8b50f180
d5b188a73702203c3c1331e3e350ca02ab2d9148b6d0c0affa56d3ad97852d04
```

Repeated publication is idempotent. Changing host-run identity, observed CPU
time, or logical creation time preserves semantic artifact identity.

## Negative evidence

Missing parent lineage, changed held-out episode, changed source proof content,
changed counterfactual metrics, malformed payloads, mismatched task scope, and
unsupported schema or semantics versions all fail closed. Loaded fields are
comparison targets, never verifier measurements.

## Claim boundary

This makes one bounded compiled-mechanism learning lifecycle reconstructable
after restart and preserves its measured continual-learning effect. It does not
yet persist an optimized compiled implementation, handle arbitrary graph edits,
lower proof operations into RKL world semantics, demonstrate long-curriculum
retention, or prove broad G0-G6/general intelligence. ARC is untouched and
remains at the last measured 36/400.
