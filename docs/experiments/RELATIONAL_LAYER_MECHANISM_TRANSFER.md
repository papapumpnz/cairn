# Relational layer mechanism transfer

Date: 2026-07-13

Status: non-ARC transfer passed; ARC accuracy hypothesis failed

## Question

Can CAIRN learn collection-valued correspondence, construction, and rendering
from non-ARC graphics, promote the result through independent evidence, and
reuse the same mechanism in another frontend before it is exposed to ARC?

## Frozen design

The reference learner enumerates a bounded universal finite-set grammar over
unique minimum/maximum-cardinality layer roles:

```text
input(role)
union(role, role)
intersection(role, role)
difference(role, role)
```

It jointly learns the foreground role and one of two explicit render frames:
preserve the source extent or normalize to the tight content frame. The learner
receives no modality, task family, ARC identifier, benchmark answer, or
operation label.

Promotion requires two disjoint cohorts replayed by the independent verifier.
Memory remains a proposal source. The finite-field frontend can lower disjoint
categorical layers; overlapping-layer transfer is deliberately unsupported.

## Non-ARC commands

```sh
cargo test --offline -p cairn-vector --test layer_learning -- --nocapture
cargo test --offline -p cairn-bench --test g5_layer_mechanism_transfer -- --nocapture
```

## Measured non-ARC result

```text
set constructions learned: union, intersection, ordered difference
render frames learned: preserve source, tight content
promotion cohorts/experiences: 2/4
held-out vector replays: 2 exact
finite-field cross-domain transfers: 1 exact
blank-memory proposals: 0
wrong-renderer ablation: changed output
changed-expected verifier ablation: rejected
role-tie/candidate-cap negatives: rejected
```

The promoted union/preserve mechanism has empirical support across four source
extents and four foreground values in two dimensions. These observations are a
support boundary, not proof of universality.

## Frozen ARC commands

Run only after the implementation, this document, and the harness have been
committed:

```sh
cargo test --locked -p cairn-arc --test structural_transfer_improvement \
  frozen_arc40_all_structural_mechanisms -- --ignored --nocapture

cargo test --locked -p cairn-arc --test structural_transfer_improvement \
  complete_arc400_structural_transfer_ablation -- --ignored --nocapture
```

The pre-registered baselines are 18/40 and 36/400 exact, both with zero
incorrect acceptance. The grammar and thresholds must not change after either
result is observed. Raw per-case evidence and a flat or negative result are
mandatory outputs.

## ARC results

The implementation and harness were frozen in commit
`64e7a1e32d643e0027d09a3aa3734cd2dc30ffe9` before ARC execution.

```text
all structural memory, first 40: 19 exact, 0 incorrect, 21 limited
all structural memory, full 400: 36 exact, 0 incorrect, 364 limited
full-400 wall time: 588.93 seconds
layer-only first-40 ablation: 17 exact, 0 incorrect, 23 limited
blank first-40 ablation: 17 exact, 0 incorrect, 23 limited
```

The 19/40 all-memory number is not attributable to the new layer mechanism:
the extra result over the older partition-only first-40 test is supplied by an
already-existing partition-extraction binding. The complete score remains
36/400. A post-result causal ablation then compared blank memory and layer-only
memory over the identical first 40 and failed its precondition that exact solves
increase: both scored 17/40. The failing assertion is retained as a disabled
experiment gate rather than weakened.

Evidence:

- `evidence/arc1_training_400_wp34_v0.10.json` — SHA-256
  `6fd7cc6d0a7c22a85a5bcd66c4ab32e343377e0492b558d6a014af1f558caff6`
- `evidence/arc1_first40_layer_ablation_wp34_v0.10.json` — SHA-256
  `ec2b42cbe363a77d3610feb77d648a56503e4d1de54c2f208addb87558cb14fe`

The learned mechanism remains a valid cross-domain vector-to-field capability,
but this bounded cardinality/set grammar has no demonstrated causal contribution
to ARC accuracy. ADR-0024 stops ARC-directed expansion of this path.

## Limits

This experiment does not yet learn arbitrary relation paths, affine geometry,
object correspondence under topology change, recursion, or perception from raw
pixels. It is evidence of narrow verified mechanism invention and cross-domain
reuse, not evidence of general intelligence.
