# Primitive relation invention from anonymous attributes

Status: `PASS_WITH_DECLARED_LIMITS` (historical equality-only slice; extended by ADR-0048)

## Frozen question

Can CAIRN construct a useful executable binary relation when frontends supply
only anonymous primitive attributes, select entities through downstream
explanation, transfer the complete mechanism, and reuse it without answer
authority?

## Result

```text
supplied relations: 0
supplied entity groups: 0
relation program structures: 1
anonymous attributes per primitive: 3
cross-episode attribute bindings: 4 explanatory combinations
independent source domains: 2
held-out domains: 2
held-out groups: 9 / 9 exact
binding-ablation acceptances: 0
ambiguous duplicate-binding acceptances: 0
relation-cap acceptances: 0
sequence -> vector transfer: 9 / 9 exact
core changes for adapter transfer: 0
fresh feature candidates -> memory synthesis: 708 -> 0
source examples -> reuse examples: 4 -> 0
current attribute bindings attempted: 3
blank-memory proposals: 0
```

Commands:

```sh
cargo test --offline -p cairn-theory --test relation_observation -- --nocapture
cargo test --offline -p cairn-runtime --test raw_relation_observation_memory -- --nocapture
cargo test --offline -p cairn-bench --test g5_raw_relation_transfer -- --nocapture
```

## Attribution

The intended relation is not present in input evidence. One anonymous attribute
partitions primitives into explanatory entities; a second partitions them into
equal-sized mixed hard negatives; a third is unique. Attribute positions change
between episodes and adapters. The learned RKL equality structure and downstream
mechanism remain identical, while task-local bindings change.

The verifier independently executes relation RKL against raw attributes and
constructs its own edges before disjoint-set entity reconstruction. Replacing
every useful binding with the hard negative causes replay failure.

ADR-0048 retains this result as the lowest structural-cost phase and adds
composed relation-key programs, complete program promotion, evidence-only cold
restart, and failure-driven curriculum coupling. See
`COMPOSED_RELATION_PROGRAM_LEARNING.md` for the current result.

## Claim boundary

This is bounded relation construction and joint explanation, not general
intelligence. Equality is the sole initial relation-program structure, scalar
attributes remain frontend evidence, and the experiment does not learn sensory
features, nested entities, noisy correspondences, or an expanding relation
grammar. Memory removes downstream synthesis but still enumerates current
attribute bindings and requires independent current-evidence acceptance.
