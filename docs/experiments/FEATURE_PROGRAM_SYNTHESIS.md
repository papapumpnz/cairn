# Typed observation-feature program synthesis

Status: `PASS_WITH_DECLARED_LIMITS`

## Frozen question

Can CAIRN construct a useful executable observation feature from the generic
RKL arithmetic substrate, rather than select it from named feature templates,
then transfer and retain that mechanism?

## Frozen result

```text
named feature templates: 0
primitive observations per entity: 2 and 3
typed feature candidates evaluated (binary addition family): 217
feature sets evaluated before exact addition theory: 6
invented feature body: observation_0 + observation_1
independent invention cohorts: 2
source adapter: sequence
held-out adapter: vector
held-out cross-adapter groups: 11 / 11 exact
feature-body ablation acceptances: 0
candidate-cap violations accepted: 0
verified reuse receipts: 2
fresh candidate work -> memory proposals: 286 -> 1
fresh examples -> reuse examples: 4 -> 0
memory held-out groups: 5 / 5 exact
three-observation permutations checked: 6 / 6
three-observation feature candidates / sets: 640 / 5
three-observation held-out groups: 9 / 9 exact
restart authority restored directly: 0
restart cohorts re-invented / re-promoted: 2 / 1
restart fresh candidate work -> proposal: 708 -> 1
```

Commands:

```sh
cargo test --offline -p cairn-theory --test observation_predicate -- --nocapture
cargo test --offline -p cairn-bench --test g5_observation_feature_transfer -- --nocapture
cargo test --offline -p cairn-memory --test observation_predicate_memory -- --nocapture
cargo test --offline -p cairn-memory --test observation_predicate_restart -- --nocapture
cargo test --offline -p cairn-runtime --test observation_predicate_memory -- --nocapture
```

## Causal attribution

Members of a collection share neither raw observation, minimum, nor maximum.
Their anonymous observations can also arrive in either order. Only the
synthesized addition program supplies the equality invariant needed by the
downstream learned grouping relation. Replacing that body with a raw observation
causes independent replay to fail.

The feature search receives no sequence/vector identity. The sequence and
vector adapters emit the same primitive-pair contract, and the unchanged learned
feature/relation/aggregate object replays vector evidence exactly.

The independent verifier additionally enumerates every observation-index
permutation within the declared cap and requires the complete prediction to
remain exact. Reuse work is derived from
the learned candidate/set/relation/constructor reports; callers cannot supply a
smaller baseline to manufacture a promotion receipt.

## Claim boundary

This is genuine bounded task and continual learning: executable rule structure
absent from a named feature menu is synthesized, independently reproduced,
transferred, promoted from disjoint verifier receipts, and later reduces search
and examples. Evidence now survives process loss without restoring authority,
and the same grammar handles three observations with exact tested support
geometry. It is not general intelligence. Entity boundaries, clean outputs, and
the vector projection axis remain frontend supplied. Default arity is capped at
four, permutation search at 24, and feature cost at five. Ordered observation
semantics, raw perception, and open-ended grammar/curriculum growth are not
demonstrated.
