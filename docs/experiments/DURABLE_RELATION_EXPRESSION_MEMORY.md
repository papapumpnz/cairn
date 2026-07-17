# Durable relation-expression memory

Status: measured `PASS_WITH_DECLARED_LIMITS`

## Question

Can a learned multi-operator relation expression survive complete process loss
without serializing authority, and still reduce future examples and expression
search after fresh invention and independent replay?

## Frozen comparison

Two disjoint non-ARC cohorts require the omitted expression
`minimum(add(p0,p1),p2)`. Each cohort is persisted only after independent reuse
verification. The CAS is closed, all process objects are dropped, and a blank
runtime is constructed before load and replay.

## Command

```text
cargo test --offline -p cairn-memory --test relation_expression_restart -- --nocapture
```

## Measured result

```text
artifacts: 2 distinct content-addressed cohorts
serialized authority restored: 0
cohorts freshly reinvented: 2
fresh independent replay receipts: 2
fresh promotions: 1
expression structures: 4 -> 1
source examples: 4 -> 0
future answer-hidden proposals: 1
future exact proposals: 1
insufficient-cap replay accepted: 0
```

The exact artifact IDs emitted by the deterministic test are
`blake3:e4edcfe347cac9233377f1bff66df4c62585836ce0ebb6a724eb869a2311d2cb`
and
`blake3:7bf6a4e99847b13a1fefae2fdf6760efaeec243210141735769d4aa40e88b410`.

## Negative result retained

ADR-0052 records that immediately widening the same frozen expression family
to every binary-tree association makes the explanation ambiguous. Durable
memory therefore preserves a verified bounded mechanism; it is not evidence
for arbitrary syntax invention or general intelligence.
