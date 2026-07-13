# Output-driven pair-relation invention experiment

Status: `PASS_WITH_DECLARED_LIMITS`

## Frozen question

Can CAIRN invent the primitive pair relation that defines collections using
only generic entity features and downstream unordered outputs?

## Method

Four source episodes span vector, sequence, table, and workflow labels. Every
entity has a scalar payload and two anonymous integer feature slots. Equality
on one slot yields variable-size collections supporting one aggregate. Equality
on the distractor slot yields equal-count identical-evidence components and
cannot explain distinct outputs.

No pair relation, relation edge, component, selected feature, or pair label is
supplied. Two independent cohorts synthesize the same RKL pair predicate and
merge. The verifier independently executes both definitions. Graphics and
operations held-outs change identities, feature values, payloads, component
counts, cardinalities, and entity order.

## Measured result

```text
pair labels supplied: 0
relation edges supplied: 0
partitions supplied: 0
relation candidates evaluated: 2
fresh total candidates / learned reuse attempts: 187 / 1
fresh examples / reuse examples: 4 / 0
held-out components independently replayed: 13 / 13 exact
entity-order semantic changes: 0
feature-slot/definition ablation acceptances: 0
```

## Interpretation

This is output-driven invention of one executable primitive relation rather
than selection of supplied relation evidence. The current bounded grammar only
tests equality over corresponding generic feature slots. It does not yet invent
multi-feature conjunctions, inequalities, geometry-derived features, recursive
relations, or raw entities. Persistence and cross-adapter realization are also
open. No general-intelligence or ARC-score claim follows.
