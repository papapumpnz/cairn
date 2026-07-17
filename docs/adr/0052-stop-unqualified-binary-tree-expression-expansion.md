# ADR-0052: stop unqualified binary-tree expression expansion

Status: accepted negative result

## Context

ADR-0051 deliberately bounded relation-expression invention to a
left-associated meta-grammar. The next hypothesis was that the same frozen
family would identify one mechanism if every ordered full binary-tree shape
over the promoted operators became reachable.

## Experiment and result

The producer was changed locally to enumerate both association shapes for
three parameters, and the verifier independently counted the enlarged language
using the Catalan recurrence. The unchanged frozen command was:

```text
cargo test --offline -p cairn-bench --test g6_relation_expression_growth -- --nocapture
```

It failed with `Entity(Ambiguous)` during expression invention. The enlarged
language exposed multiple complete explanations. No threshold, example, or
ambiguity policy was changed after observing the result.

## Decision

Stop this expansion path and retain the previous exact ambiguity rejection.
Do not make right-associated structures reachable until an independently
authored curriculum supplies discriminating positive and negative evidence.
The experimental code was removed; this ADR preserves the negative result.

## Alternatives rejected

- Prefer the cheaper or first expression: this would hide observational
  ambiguity behind proposal ordering.
- Add a demonstration to the frozen family after seeing the failure: this
  would invalidate the pre-registered measurement.
- Treat algebraic similarity as proof of equivalence: no universal equivalence
  proof exists for the mixed operators.

## Compatibility and verifier impact

There is no production schema or semantics change. Existing
`cairn.relation-expression.v1` identities remain unchanged. The verifier
continues to accept only the declared left-associated language and exact
definition replay.

## Restart condition

Reopen broader tree synthesis only with a separately frozen family that varies
association structure, contains hard negatives for competing trees, and keeps
producer and verifier enumeration independent.
