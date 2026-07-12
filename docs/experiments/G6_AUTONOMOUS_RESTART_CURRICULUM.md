# G6 autonomous restart curriculum

Date: 2026-07-12

Status: bounded autonomous curriculum pass

## Question

Can CAIRN choose its own next developmental experience from support gaps, grow
several independently verified mechanisms, retain them across restart, reject
unrelated transfer, and unlock a novel composition objective from learned
prerequisites?

## Separation

The curriculum sees opaque objective IDs, learning axes, prerequisite roles,
hypothesis counts, transfer estimates, and costs. A separate generator maps the
chosen objective to proof observations. The learner never sees generator family
metadata. No ARC adapter or corpus is linked to the test.

Four generators contain different hidden four-node causal structures. Source
and held-out host geometry is identical across families, preventing later task
size from manufacturing a learning curve.

## Commands

```sh
cargo test -p cairn-curriculum --test autonomous -- --nocapture
cargo test -p cairn-bench --test g6_autonomous_internal_curriculum -- --nocapture
cargo test -p cairn-bench --test g6_developmental_scaling -- --nocapture
```

## Autonomous result

```text
selected: family-a, family-b, family-c, family-d
next objective: generated composition from a redacted failed-search core
learned mechanisms: 1 -> 2 -> 3 -> 4
candidate windows: 18 -> 9 in every family
examples: 4 -> 1 in every family
proof units: 28 -> 4 in every family
CAS restarts: 2
curriculum checkpoint restores: 4
restored lifecycle state: target-d deprecated from two verified counterexamples
aggregate retained prior tasks: 6/6
unrelated negative transfer: 0/10
```

Mechanism identities:

```text
45f0ef9aa5f769980c1e41142530cc26b691b48c1bed887e7832af5c5f97fe4b
ea4db297cdd1b3968a8b5d76e693d357e7f4c47f20c7354c020f092174d9af8c
36142f8b0bb6b8516280d790479ee26f8a923f3aaa479db0dc1bd90ec3d11cec
510dc7b9134bd7f30dd9b62ce960d2ad9727f4f992a8b0a4ee4a2a84df6ea684
```

Curriculum checkpoint identities:

```text
deff68108d34dcd1856618f70b25512180a978ab44e9d8d6d1fbb139e07c5942
4fb39e7cd27f09571a15f939d096a538dce350b6ddfc3a3a66ffcfd3501be3cb
622a5fd163f323272b529ab9f04c27f6f35af37b60d8752ef6aecf4824781d39
a2a8314cc30512ad7981d8db730b29977d12f0686bf1b47f773a379710465c56
```

No composition objective exists in the initial curriculum. After all four
families, a bounded failed invention is reduced to a content-free core. The
failure-driven generator classifies it as a composition gap, binds the first two
active mechanism roles as prerequisites, assigns a canonical identity, and
journals the objective. A third curriculum reconstruction reproduces the
objective-addition boundary before selecting it.

The generated request then crosses a separate materializer boundary as two
canonical `RelationalObservation` values, not proof nodes. The producer and
independent verifier separately lower opaque typed elements and relations into
identical proof frontiers with explicit provenance.

The first observation has static vector-like provenance. After exact training
construction, the consolidator derives two-round relational neighbourhood
selectors and the verifier binds them to that training certificate. The second
observation is built from two action effects learned by the unchanged
unknown-world runtime from four transitions. Both effects predict unseen states
and support a one-step inferred-goal plan.

Held-out modality, element IDs, values, canonical order, and cardinality change,
and an unrelated entity/effect pair is added. The ordinal-policy ablation emits
the wrong graph. Relational policy application produces one candidate without
expected-proof input. The independent verifier alone receives truth, replays
lowering, policy induction, both compilations, and all four invocations, and
reports:

```text
explicit learned invocations: 2 mechanisms x 2 changed exercises
runtime candidates: 16 training + 1 held-out
unknown-world laws: 2
unseen world predictions: 2/2
inferred-goal plan: 1 action
ordinal ablation: failed exact held-out graph
candidate windows: 72 -> 35
examples: 16 -> 1
proof units: 112 -> 16
composition evidence: 7a7cbc0a7e44a06ad480a015b26c1b0a95b00b8e0991f83d448d07e1bb484f8a
```

Overlap, duplicate mechanisms, irrelevant invocations, and exhausted bounds
fail closed. The verified outcome is recorded, checkpointed, reconstructed a
fourth time, and the five-objective curriculum terminates complete.

## Restart and counterexample behavior

One distinct independently verified counterexample quarantines its mechanism;
a second deprecates it. Duplicate, empty, or unverified evidence cannot change
status. Quarantined and deprecated mechanisms no longer satisfy curriculum
prerequisites.

At both CAS boundaries the controller writes a canonical, parent-bound event
journal, loses its process state, and reconstructs a fresh controller by
replaying ordinary selection and admission. The final restart reproduces the
two counterexamples and the deprecated status of `target-d`; it does not trust
a serialized status flag. Malformed payloads, missing lineage, and a forged
recorded choice fail closed.

## Claim boundary

This replaces scripted order, the authored composition objective, supplied
complete proof hosts, manual frontier assembly, and ordinal transfer with
bounded failure-driven relational-observation learning. Entity/relation
evidence remains generator-supplied rather than induced from raw sensors, and
the world laws are small deterministic deltas without latent state. It does not
establish open-ended environment generation, arbitrary RKL composition, natural
perception, broad general intelligence, or the 80% ARC target. ARC remains at
the last measured 36/400.
