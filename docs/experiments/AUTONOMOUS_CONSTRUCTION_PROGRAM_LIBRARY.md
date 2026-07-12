# Autonomous construction-program library

Date: 2026-07-13

Status: non-ARC autonomous learning gate passed

## Question

Can CAIRN discover correspondence directly from unordered observations,
synthesize the same executable program independently, promote it, and learn
later cases with less compute and fewer examples—without v1 traces?

## Frozen evidence

Two independent cohorts each contain four entity scenes. Source and expected
entities preserve a generic categorical attribute, but point sets are unordered
and no point pairing is provided. IDs, values, extents, positions, entity counts,
and point cardinalities vary.

## Commands

```sh
cargo test --offline -p cairn-invent --test unordered_mapping -- --nocapture
cargo test --offline -p cairn-theory --test construction_program -- --nocapture
cargo test --offline -p cairn-bench --test g2_autonomous_construction_program_memory -- --nocapture
```

## Results

```text
independent cohorts: 2
verified experiences: 8
v1 training traces: 0
canonical promoted programs: 1
fresh synthesis candidates: 5,986
memory attempts: 2
fresh examples: 4
memory examples: 0
changed held-outs: 2/2 exact
field transfer: 1 exact
blank-memory proposals: 0
wrong-family verifier accepts: 0
unordered reorder invariance: exact
missing-output/candidate-cap negatives: rejected
```

## Interpretation

This is the first checkpoint satisfying the local mechanical learning claim
without teacher correspondence: executable RKL structure is invented,
independent experience converges on the same program, verified memory promotes
it, and experience reduces later search and sample cost.

The capability is narrow. Entity boundaries and the preserved categorical
attribute are supplied. Every entity maps once and retains cardinality. There is
no learned filter, conditional branch, split, merge, recursive emitter, or
uncertain perception.

No ARC evaluation was run or authorized.

## Next gate

Learn non-ARC families where relational roles determine whether an entity is
preserved, transformed, split, merged, or used as a constructor. Program
structure—not a new Rust enum—must introduce filter/group/conditional/emit
composition with complete provenance.
