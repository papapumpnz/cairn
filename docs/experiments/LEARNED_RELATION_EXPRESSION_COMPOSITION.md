# Learned relation-expression composition

Status: `PASS_WITH_DECLARED_LIMITS`

## Frozen question

Can independently promoted mechanisms become a vocabulary from which CAIRN
invents a new executable rule structure, rather than merely applying one
configured operator at higher arity?

## Measured result

```text
promoted source operators: addition/2, minimum/2
independent promotion cohorts per source: 2
dependency arity requested by redacted failure: 3
named composite templates supplied: 0
candidate nested expression structures: 4
selected expression: minimum(add(p0, p1), p2)
expression promotion receipts: 2
expression structure work per cohort: 4 -> 1
promotion expression evaluations saved: 6
source examples -> answer-hidden reuse examples: 4 -> 0
source frontend -> held-out frontend: sequence -> vector
answer-hidden held-out proposals: 2
held-out groups: 9 / 9 exact
blank-language exact theories: 0
verifier ablation acceptances: 0
```

Command:

```sh
cargo test --offline -p cairn-bench --test g6_relation_expression_growth -- --nocapture
```

## Attribution

The language certificate contains only two independently promoted operator
families and bounded resource information. It does not contain the nested AST,
attribute binding, entities, outputs, or task identity. Every simpler supplied
family is a hard negative. Two expression cohorts separately discover the same
program under changed values and channel permutations. Only verifier receipts
can produce the promotion certificate used before answer-hidden transfer.

## Claim boundary

This demonstrates bounded learner-constructed rule structure, verified reuse,
sample reduction, and cross-frontend transfer. It does not invent the binary
operators, the expression meta-grammar, perception channels, arbitrary trees,
recursive control, or its own verifier. CAIRN therefore remains a research
general learner, not a demonstrated general intelligence.
