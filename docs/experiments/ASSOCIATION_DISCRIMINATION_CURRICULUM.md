# Association-discrimination curriculum

Status: measured `PASS_WITH_DECLARED_LIMITS`

## Question

Can an observed ambiguity cause the learner to expand its bounded expression
grammar, invent a previously unreachable association structure, verify and
promote it, then use it on changed values without examples or source-channel
positions?

## Frozen family

Two primitive scalar operators, subtraction and minimum, are independently
learned and promoted from raw anonymous attributes. A content-free ambiguity
core declares only expression-association class, arity three, survivor count,
and verified prerequisite roles. It contains no values, operator order,
expression tree, expected output, task identifier, or benchmark content.

Four scalar traces in two disjoint cohorts vary values, offsets, scales, and
all four anonymous input-channel positions. The omitted target is structurally
right-associated. A fifth trace is held out from invention and promotion.

## Command

```text
cargo test --offline -p cairn-bench --test g6_expression_association_curriculum -- --nocapture
```

## Measured result

```text
left-associated exact theories: 0
tree shapes before/after request: 1 / 2
full-language expression structures: 8
surviving source-independent structures: 1
selected expression: subtract(p0, minimum(p1, p2))
independent reuse cohorts: 2
expression structures saved: 14
source examples / reuse examples: 4 / 0
source/target typed frontends: sequence / vector
promoted in-process memory modules: 1
held-out exact scalar cases: 8 / 8
current anonymous bindings attempted: 96
```

The earlier relation-only contrast is retained as a negative finding: equality
partitions can erase association, allowing a different tree and binding to
produce the same relation. Exact scalar evidence is necessary for this family.

## Attribution and limits

The gain is caused by a content-free curriculum request, full-tree search,
scalar-valued evidence, independent replay, and proof-gated memory. Removing
the extension leaves no exact theory. Sequence and vector adapters perform only
lossless lowering to the same trace; neither contains operators, expression
shapes, or output logic. No ARC case or answer was used.

This is bounded mechanism invention and continual reuse, not general
intelligence. Operators and integer channels remain developer-supplied, the
tree bound is finite, the held-out domains are still typed synthetic frontends,
and no sustained mixed
static/interactive
curriculum or ARC improvement is claimed.
