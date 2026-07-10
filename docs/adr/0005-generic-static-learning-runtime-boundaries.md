# ADR-0005: Generic static-learning runtime boundaries

Status: accepted

Date: 2026-07-11

Owners: integration owner

## Context

RKL previously admitted whole theories but only executed isolated expressions.
It also lacked declared inputs, making it impossible for a checked rule to bind
generic observations without embedding constants. The repository had invention,
memory, and world-learning experiments, but no connected static path from a
frontend observation through entity/event hypotheses, component synthesis,
whole-theory search, and independent replay.

## Decision

RKL 1.1 declares typed theory inputs and defines reference whole-theory
semantics for quantifiers, logical writes, state effects, all schedule forms,
explicit conflict policies, renderer layers, and bounded fixpoints. The bytecode
executor supplies an alternate expression backend to the same deterministic
schedule engine; differential tests compare externally visible theory results.

Generic learning contracts live in `cairn-core`. Frontends emit content-derived
observations with modality identifiers, canonical payloads, sources, logical
time, and explicit certainty. `cairn-entity`, `cairn-events`, `cairn-theory`,
and `cairn-cegis` consume environment-neutral evidence and RKL. `cairn-proof`
depends on no synthesis or memory crate and is the final acceptance boundary.

Finite categorical arrays are handled by `cairn-field`. ARC parsing, split
types, categorical range validation, answer encoding, and the attempt CLI live
only in `adapters/cairn-arc`. The adapter can call the generic learner; cognition
cannot import the adapter.

## Consequences

- Theories can now express observation-dependent rules without free variables.
- Persistent theory identity changed, requiring RKL semantics 1.1.
- Search output is never accepted from its score alone; replay certificates
  bind the checked theory, evidence, semantic versions, and canonical output.
- The first finite-field and ARC adapter path supports learned pointwise
  same-shape constructors only. Shape-changing, relational, entity-level, and
  multi-rule tasks remain future capability, not silently handled special cases.
- Whole-theory reference and bytecode paths share scheduling code. This catches
  expression compiler defects but is not yet the fully separate critical-effect
  reimplementation required for final WP06 acceptance.

## Validation

- one million generated scalar programs still match reference and bytecode;
- a typed-input whole theory matches across reference and bytecode output,
  state, firings, and fixpoint count;
- planted replay-certificate corruptions fail closed;
- entity, event, guard, constructor, schedule, and CEGIS fixtures pass;
- an omitted finite-field constructor transfers to changed shape and values;
- the quarantined ARC-format fixture is solved without an adapter operation
  label or test-output access.

## Compatibility

RKL semantic artifacts produced under 1.0 must not be treated as 1.1 artifacts.
No migration is inferred because theory input declarations and logical writes
change executable meaning.
