# Variable-cardinality offline distillation

Status: bounded non-scalar invention and transfer pass; downstream ARC reuse
open.

## Motivation

The gap-driven failure cores ruled out the tested scalar one-, two-, and
three-role families. The teacher packet previously had only scalar mechanism
lessons, so it could not express evidence for variable-arity behavior without
collapsing it into prose.

Distillation semantics 1.2.0 adds a separate collection lesson channel. A
lesson supplies named episodes containing aligned input and output collections,
plus a maximum cardinality. It does not supply an RKL definition, loop, or
`ForEach` template instance.

## Offline lesson

The offline GPT-authored, non-benchmark packet contains three episodes:

| Episode cardinality | Input/output behavior |
|---:|---|
| 2 | varied signed integers |
| 4 | changed values including zero |
| 5 | wider unseen values |

CAIRN flattens the aligned evidence only for element-law invention, discovers
`output := input * 2 + 1`, and lifts the result into a checked theory with a
dynamic collection binder and `ForEach` quantifier. The complete theory is then
executed against every source episode.

Measured evidence:

- 3 independent source episodes;
- observed cardinalities 2, 4, and 5;
- 11 exact source-element replays;
- two independent peer recompilation/replay receipts;
- exact transfer to an unseen seven-element collection;
- seven quantified rule firings on the held-out collection.

No online teacher call occurs. The accepted swarm packet contains the learned
checked theory and provenance, while the raw teacher is absent during transfer.

## Fail-closed boundary

The compiler rejects:

- fewer than two episodes;
- only one observed cardinality;
- empty or misaligned collections;
- cardinalities exceeding the declared bound;
- duplicate episode identifiers;
- duplicate input or output values under current set semantics;
- values outside the checked integer range;
- any source replay mismatch.

This channel currently models set-like aligned maps. It does not yet learn
ordering, filtering, grouping, nesting, variable output cardinality, relational
joins, or a higher-order operation parameter. It is provisional and has no ARC
proof-trace use.

## Reproduction

```text
cargo test -p cairn-distill --test collection_distillation -- --nocapture
cargo test -p cairn-distill --test bootstrap
```
