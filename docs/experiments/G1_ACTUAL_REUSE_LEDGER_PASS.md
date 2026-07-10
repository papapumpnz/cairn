# G1 actual-reuse ledger pass

Date: 2026-07-11

Status: frozen initial G1 suite pass

## Question

Does every mechanism counted as persistent have independently verified,
task-distinct evidence that it was actually invoked, necessary under ablation,
and responsible for lower search, sample, and proof depth?

## Implementation

Each mechanism module now owns a task-keyed reuse ledger. A record contains:

- an immutable experience hash;
- independent-verification status;
- necessity under a same-budget ablation;
- candidate executions with and without the mechanism;
- demonstrations with and without it;
- proof nodes with and without the compiled call.

The store accepts records only after lineage-gated persistent promotion.
Duplicate experience hashes cannot inflate counts. Unverified records and
pre-promotion records fail. Negative transfer is retained as a metric rather
than discarded.

## Frozen result

Across the five G0 mechanisms and ten held-out tasks:

    persistent mechanisms:          5
    distinct verified invocations: 10
    necessary under ablation:       10
    candidates with memory:         10
    candidates without memory:  33,600
    examples with memory:           10
    examples without memory:        48
    proof nodes with mechanism:     10
    proof nodes without:            52
    negative transfers:              0
    duplicate-record rejection:   pass
    unverified-record rejection:  pass
    pre-promotion rejection:      pass

Command:

    cargo test --locked -p cairn-memory --test adr0007_g0_multifamily -- --nocapture

## Gate judgment

The frozen initial suite satisfies G1: no stored-but-unused definition is
counted, and every counted mechanism has direct actual-use, necessity, search,
sample, and proof-depth evidence.

The ledger is currently part of the in-memory mechanism store and is not yet a
canonical CAS artifact. G2 remains open until equal-budget wall-clock runs,
unrelated-task negative transfer, and persistence/reload are measured.
