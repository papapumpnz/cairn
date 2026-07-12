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
a8438381973c8205112a6017ff7fd747009154990fe1e461ff67ec9a3321d781
a8d9b6ed78b077561c009877df357e79ae0b521ecea6375a41a29532a3cbc918
487f762058cb06e0373243cbfff4a3d30d1ad0c09a0aaadda235468805e05528
a0c3bb5a8f164ee25b4c9658ff7879d090fa527006a919fed1c0122dc402c00e
```

Curriculum checkpoint identities:

```text
107d19a230dd1d5e681cdacf75dc25dfff2cd07b56856fd38ad7cd98420685c9
03291a681ebe3c59225a15447456500db91e62fe60f80465f5115f6da303ab97
0271e0c8c5146a578a60340d8ca74e39c0c1a1d99cb2c9bb438464231d142897
1b24355fd6d822812a9a4c388786acb84622d1b14eff02daa191c27f967daede
```

No composition objective exists in the initial curriculum. After all four
families, a bounded failed invention is reduced to a content-free core. The
failure-driven generator classifies it as a composition gap, binds the first two
active mechanism roles as prerequisites, assigns a canonical identity, and
journals the objective. A third curriculum reconstruction reproduces the
objective-addition boundary before selecting it.

The generated request then crosses a separate materializer boundary. Two proof
hosts vary prefix/suffix geometry and parameter values. The shared cognition
runtime retrieves the two persistent prerequisite mechanisms and proposes two
pairwise-disjoint, result-ancestral invocations in each host. The independent
verifier reconstructs both compilations and all four invocations and reports:

```text
explicit learned invocations: 2 mechanisms x 2 changed exercises
candidate windows: 72 -> 46
examples: 16 -> 2
proof units: 112 -> 16
composition evidence: e48a1d839abdd757daf4ea02009a578535b7876d65d6266e3ab34c775f59d60f
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

This replaces scripted order and the authored composition objective with
bounded failure-driven frontier expansion and independently verified completion
at the proof-mechanism layer. The materializer still supplies complete proof
hosts rather than raw observations from which CAIRN constructs the entire
proof. It does not establish open-ended environment generation, arbitrary RKL
composition, natural perception, broad general intelligence, or the 80% ARC
target. ARC remains at the last measured 36/400.
