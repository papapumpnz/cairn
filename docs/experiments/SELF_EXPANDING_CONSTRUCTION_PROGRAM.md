# Self-expanding entity-construction program

Date: 2026-07-13

Status: initial non-ARC program-synthesis slice passed

## Question

Can CAIRN replace a dedicated signed-axis representation with executable rule
structure synthesized from generic RKL variables and verified experience?

## Program substrate

Developer supplied:

```text
bounded map over entities
bounded map over points
variables, integer arithmetic, condition-free exact emission
source extents and collection coordinate bounds
```

Learner synthesized:

```text
each output-extent expression
each output-coordinate expression
the dependency of output geometry on source coordinates and bounds
```

The program has no reflection, rotation, crop, axis map, frame, ARC, grid, or
modality operation.

## Commands and results

```sh
cargo test --offline -p cairn-theory --test construction_program -- --nocapture
cargo test --offline -p cairn-bench --test g0_construction_program_invention -- --nocapture
```

```text
independently verified training traces: 4
point binding examples: 30
RKL candidates evaluated: 5,986
held-out program-only cases: 2 exact
held-out v1 runtime use: 0
independent verifier path: exact
vector-to-field transfer: 1 exact
changed truth: rejected
learned-definition ablation: capability removed
```

The first two-trace attempt fit training but failed independent
changed-cardinality replay. That result was not weakened or hidden. Training
expanded across counts 2–5; counts 6–7 remained frozen held-outs.

## Interpretation

This is materially closer to mechanism invention: the retained executable rule
is an RKL program synthesized from examples, not a selected Rust transform
variant. Nevertheless traces currently originate from a v1 teacher mechanism.
CAIRN has not yet discovered point correspondences directly from source/output
evidence, promoted programs from independent cohorts, or invented conditional
and non-bijective control structure.

No ARC evaluation is permitted for this checkpoint.

## Next gate

Maintain competing entity/point correspondences, synthesize programs under
those alternatives, resolve them with held-out replay, promote equivalent
programs from independent cohorts, and execute from program memory without v1
trace generation.
