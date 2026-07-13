# ADR-0051 — Compose promoted operators into learned relation expressions

Status: accepted

Date: 2026-07-14

Owners: WP34 integration

## Context

ADR-0050 lets experience expose the associative closure of one promoted binary
operator. It cannot create a new rule structure containing different learned
operators. A self-compressing learner must use independently acquired
mechanisms as typed building blocks and discover their composition from
evidence, without receiving a named composite operator.

## Decision

Add an environment-neutral `RelationKeyExpression` IR with parameter leaves and
bounded binary nodes. Logical parameter slots enter semantic identity; current
anonymous attribute positions remain task-local `RelationExpressionBinding`
values. Expressions lower to ordinary typed RKL equality definitions and retain
the complete downstream entity, feature, predicate, collection, and aggregate
mechanism.

`RelationExpressionLanguageConsolidator` may expose a bounded expression search
only when:

- at least two distinct source-independent binary relation mechanisms exist;
- every source has a matching two-cohort verifier promotion;
- source mechanisms and operators are distinct;
- the operators are exact scalar RKL operations;
- an opaque content-free failure request requires at least three dependencies;
- parameter, node, source-operator, candidate, and execution caps pass.

The certificate supplies operator vocabulary and bounds, not a final expression
shape. `RelationExpressionInventor` deterministically enumerates bounded
left-associated typed expression structures, separately binds anonymous current
attributes, constructs relation graphs, and retains a structure only through
complete downstream explanation.

`RelationExpressionVerifier` reconstructs the RKL definition and executes the
expression through a separate evaluator before independent entity/mechanism
replay. `verify_reuse` derives structure reduction rather than accepting caller
metrics. `RelationExpressionPromotionConsolidator` requires two disjoint exact
receipts before proposal-only persistent use may count. Answer-hidden proposal
generation skips inapplicable bindings but grants no acceptance authority.

## Consequences

Separately promoted addition and minimum mechanisms produce a four-expression
search language. A frozen family has no exact projection, binary mechanism,
pure sum, or pure minimum explanation. The learner invents:

```text
minimum(add(parameter_0, parameter_1), parameter_2)
```

No top-level operator or template with that structure is supplied. Two disjoint
cohorts independently learn the same semantic expression. Promotion reduces
expression-structure work from four to one per cohort and records six saved
evaluations. On answer-hidden vector observations, the promoted expression
rebinds from sequence experience with two proposals, zero new examples, and
9/9 independently replayed groups. Corrupting the expression while retaining
the old RKL definition fails verification.

This is bounded rule-structure invention from learned vocabulary. The scalar
operators, left-associated expression meta-grammar, parameter arity, and raw
attribute channels remain supplied. It is not open-ended program-language
invention and is not general intelligence.

## Compatibility and migration

The expression path is additive and does not change raw-relation v2 artifacts,
ADR-0050 fold certificates, RKL semantics, or existing runtime behavior. New
mechanisms use semantic marker `cairn.relation-expression.v1`. No expression
artifact schema is introduced in this checkpoint; cold CAS reconstruction and
a shared memory store remain mandatory before durable status can be claimed.

## Validation

- Blank expression language returns `EmptyLanguage`.
- One promoted operator cannot produce a composition certificate.
- A dependency-arity request above the safety cap is rejected.
- Four composed structures are searched; exactly one explains both cohorts.
- Two independent receipts promote one semantic expression and record six
  saved structure evaluations.
- Sequence→vector transfer replays 9/9 changed groups exactly.
- Answer-hidden reuse needs zero demonstrations and emits two applicable
  proposals across two held-out traces.
- Expression/definition mismatch is rejected by independent verification.

## Rollback or kill conditions

Stop if the certificate supplies the final AST, if one source mechanism can
authorize a multi-operator expression, if task-local attribute positions enter
semantic identity, if proposal memory accepts an answer, if verifier lowering
imports producer search, or if this bounded result is called general
intelligence.
