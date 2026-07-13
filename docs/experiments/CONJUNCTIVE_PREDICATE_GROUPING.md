# Conjunctive output-driven pair-relation invention

Status: `PASS_WITH_DECLARED_LIMITS`

## Frozen question

Can CAIRN invent a multi-condition relation from anonymous entity attributes
and downstream outputs when every atomic relation is provably inadequate?

## Method

Vector, sequence, table, and workflow source episodes contain entities with two
key attributes and one distractor. Valid groups share both key attributes.
Sharing only the first key or only the second key joins unrelated groups;
sharing the distractor separates members. Group cardinality varies from two to
five in training.

No pair labels, edges, partitions, selected attributes, aggregate pairing, or
relation name enters the learner. It enumerates six bounded RKL candidates:
three atomic equalities and three two-atom conjunctions. A graphics and an
operations held-out change identities, key values, payload values, group count,
cardinality, and entity order.

## Measured result

```text
atomic exact theories: 0
relation candidates evaluated: 6
learned relation: equal(feature_0) AND equal(feature_1)
independent source cohorts: 2
fresh examples / reuse examples: 4 / 0
held-out components independently replayed: 11 / 11 exact
entity-order semantic changes: 0
single-conjunct ablation acceptances: 0
candidate-cap violations accepted: 0
```

## Interpretation

This is a genuine increase in the learned relation language: the required
compound predicate was neither named nor supplied, is executable RKL, agrees
across independent domains, and remains necessary on held-out data. It is still
a narrow symbolic result. Atoms are equality-only, feature extraction is
supplied, the learned predicate is not yet independently promoted to durable
memory, and no raw geometry, recursion, noisy evidence, or open-domain concept
learning is demonstrated. It does not establish general intelligence or ARC
progress.
