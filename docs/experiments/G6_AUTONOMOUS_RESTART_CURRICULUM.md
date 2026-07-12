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
next unlocked objective: compose-first-two
learned mechanisms: 1 -> 2 -> 3 -> 4
candidate windows: 18 -> 9 in every family
examples: 4 -> 1 in every family
proof units: 28 -> 4 in every family
CAS restarts: 2
aggregate retained prior tasks: 6/6
unrelated negative transfer: 0/10
```

The composition objective is not selected until both prerequisite mechanism
roles are active. This experiment validates curriculum choice, not composition
execution; the separate G3/G6 composition suites validate execution and report
the latest same-task marginal reductions from 24 to 16 candidates and ten to
six examples.

## Counterexample behavior

One distinct independently verified counterexample quarantines its mechanism;
a second deprecates it. Duplicate, empty, or unverified evidence cannot change
status. Quarantined and deprecated mechanisms no longer satisfy curriculum
prerequisites. Persistence of deprecation state itself remains future work.

## Claim boundary

This replaces scripted curriculum order with bounded stateful selection and
demonstrates four learned mechanisms across two restarts. The objective set is
still finite and generator-authored. It does not establish open-ended experience
generation, RKL lowering of arbitrary proof operations, unrestricted
composition discovery, natural perception, broad general intelligence, or the
80% ARC target. ARC remains at the last measured 36/400.
