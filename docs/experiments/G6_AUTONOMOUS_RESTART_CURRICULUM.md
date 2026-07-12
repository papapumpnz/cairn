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
3c98b2f528fb6c798869ff1517fe30ab57ecccd15ecfcfcec827427451af7749
```

No composition objective exists in the initial curriculum. After all four
families, a bounded failed invention is reduced to a content-free core. The
failure-driven generator classifies it as a composition gap, binds the first two
active mechanism roles as prerequisites, assigns a canonical identity, and
journals the objective. A third curriculum reconstruction reproduces the
objective-addition boundary before selecting it.

The generated request then crosses a separate materializer boundary. Two
evidence-only proof frontiers vary prefix geometry and typed parameter values;
neither contains learned operation nodes, an output node, or expected truth.
The shared cognition runtime retrieves the two persistent prerequisite
mechanisms and enumerates typed constructions for the first frontier. After the
independent verifier accepts one training construction, the consolidator learns
a task-local typed ordinal correspondence policy. Applying it to the changed
frontier produces exactly one held-out candidate without expected-proof input.
The independent verifier alone receives both expected proofs, reconstructs both
compilations and all four invocations, and reports:

```text
explicit learned invocations: 2 mechanisms x 2 changed exercises
runtime candidates: 16 training + 1 held-out
candidate windows: 72 -> 40
examples: 16 -> 1
proof units: 112 -> 16
composition evidence: 3aae7022b583b25378eab43f140d68e52a219fc18852bc67fee38184ef806c59
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

This replaces scripted order, the authored composition objective, and supplied
complete proof hosts with bounded failure-driven frontier expansion plus
experience-dependent proof construction. The materializer still supplies a
typed proof-evidence frontier rather than raw generic observations; the learned
policy is ordinal rather than relational. It does not establish open-ended
environment generation, arbitrary RKL composition, natural perception, broad
general intelligence, or the 80% ARC target. ARC remains at the last measured
36/400.
