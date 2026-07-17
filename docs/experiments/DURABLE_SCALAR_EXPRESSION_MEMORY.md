# Durable scalar-expression memory

Status: measured `PASS_WITH_DECLARED_LIMITS`

## Question

Can a learned association-sensitive scalar mechanism survive complete process
loss while persistence stores evidence rather than authority?

## Command

```text
cargo test --offline -p cairn-memory --test scalar_expression_restart -- --nocapture
```

## Measured result

```text
canonical cohort artifacts: 2
serialized authority restored: 0
cohorts freshly re-invented: 2
fresh independent receipts: 2
fresh promotions: 1
expression structures: 8 -> 1
source examples: 4 -> 0
future exact scalar cases: 8 / 8
insufficient-cap replay accepted: 0
```

Artifact IDs:

```text
blake3:9ba9787149a5178fdbe11480e45a4d3d848ab451e3cc6fb5879dae7f8fe0d184
blake3:4d9c53e4da0585ce6f09fada3daab01ba94f05cf6f761136b18b44c4451cc894
```

The archive excludes the learned AST, RKL definition, channel bindings,
receipts, promotion certificate, and module status. The future trace changes
values, scale, offset, and channel permutation.

This establishes durable bounded continual learning, not general intelligence.
The scalar family remains synthetic, base operators are supplied, and broad
autonomous static/interactive curriculum transfer remains incomplete.

