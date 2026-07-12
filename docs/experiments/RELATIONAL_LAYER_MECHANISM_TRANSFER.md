# Relational layer mechanism transfer

Date: 2026-07-13

Status: non-ARC core measured; ARC evaluation frozen and pending

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

## Limits

This experiment does not yet learn arbitrary relation paths, affine geometry,
object correspondence under topology change, recursion, or perception from raw
pixels. It is evidence of narrow verified mechanism invention and cross-domain
reuse, not evidence of general intelligence.
