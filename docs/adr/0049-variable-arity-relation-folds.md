# ADR-0049 — Admit bounded variable-arity relation folds

Status: accepted

Date: 2026-07-13

Owners: WP34 integration

## Context

ADR-0048 learns reusable relation structure, but every non-projection key still
has exactly two inputs. A family whose invariant depends on three independently
varying channels cannot be expressed, even though RKL supports bounded finite
composition.

## Decision

Add a variable-arity associative `Sum` key node to the environment-neutral
relation grammar. `RelationObservationLimits::maximum_key_arity` bounds its
enumeration and defaults to two, preserving the ADR-0048 baseline. When raised,
the learner enumerates canonical attribute combinations from arity three to the
declared cap and evaluates them only after projection and binary phases fail.

ADR-0050 subsequently separates the arity cap from fold-family reachability.
The ADR-0049 experiment now explicitly enables the additive fold; default and
arity-only configurations expose no fold.

The RKL relation definition contains the complete left and right fold, parameter
arity, bounds, and equality test. `RawRelationMechanism` records `key_arity` in
addition to its operator, and verifier promotion support records the same value.
Memory retrieval must match both operator and arity before trying current
bindings. It cannot use a learned ternary fold as a binary macro.

## Consequences

A four-channel family in which no atomic or pair key is exact now learns a
three-input sum. Two disjoint cohorts promote it, current structural binding
work falls from 32 to 4, examples fall from four to zero, and 9/9 changed groups
replay independently.

This is bounded higher-arity composition, not self-generated grammar. Addition
and the fold law are developer-supplied RKL substrate, the arity cap is explicit,
and scalar channels remain frontend evidence.

## Compatibility and migration

The default maximum arity remains two, so ADR-0048 candidate counts and
identities are unchanged. New mechanisms retain the v2 identity marker; their
canonical RKL definition contains six parameters and cannot collide with a
binary definition. Promotion certificates and runtime applicability now include
`key_arity`. Pre-change in-memory certificates are experimental and must be
regenerated from evidence.

## Validation

- Projection and all binary key families explain zero cases.
- The cost-phased learner selects one three-input sum under permuted channels.
- Changed values, identifiers, cardinalities, and layouts replay 9/9 groups.
- Two independent receipts measure cohort binding work 128 to 16 and issue one
  certificate saving 112 bindings.
- Proposal-only runtime use measures 32 to 4 bindings and four to zero examples.
- Independent current-evidence replay accepts 9/9 groups.

## Rollback or kill conditions

Stop if the arity bound is implicit, attribute order changes semantic identity,
a certificate omits arity, runtime retrieval accepts a different arity, a
fold bypasses exact RKL replay, or this bounded node is described as autonomous
grammar invention.
