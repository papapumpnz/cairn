# ADR-0042 — Invent observation features and transfer them across adapters

Status: superseded by ADR-0043

## Context

ADR-0040 invents enclosure after a frontend supplies normalized endpoint
features. General cognition must instead construct useful feature definitions
from less interpreted observations and carry them across modalities.

## Decision

Add `PrimitivePairEntity` and `PrimitivePairTrace`, containing opaque identity,
scalar payload, and two anonymous scalar observations. Within a bounded feature
hole, compare raw observation order with executable RKL minimum/maximum
definitions. For each feature hypothesis, run the ordinary output-driven
predicate/aggregate learner. Retain only a unique exact joint explanation.

The independent verifier separately executes both learned feature definitions,
constructs fresh attributed evidence, and invokes independent predicate replay.
The learned semantic identity binds both feature RKL definitions and the
predicate/aggregate identity.

Sequence and vector adapters may lower domain-native spans or projected line
segments to anonymous primitive pairs. They may not normalize endpoint order or
assign relational meaning. The same learned core object must transfer unchanged.

## Alternatives rejected

- Normalize endpoints in every frontend: hard-codes the feature and prevents
  learning evidence.
- Add a named interval entity or containment operator: supplies the target
  ontology.
- Store the selected hypothesis index: retains configuration rather than
  executable semantics.
- Claim cross-domain transfer from renamed synthetic labels alone: does not
  exercise adapter boundaries.

## Compatibility and migration

All contracts are additive. `cairn-sequence` adds a dependency on
`cairn-theory` solely for the generic primitive envelope already used by
cognition. Existing adapter methods and learned artifacts remain unchanged.

## Validation

Require inconsistent primitive-pair orientation, raw-order failure, learned
minimum/maximum definitions, two independent source cohorts, semantic identity
agreement, 11/11 held-out replay, feature-body ablation rejection, sequence-to-
vector transfer, and no adapter or core domain branch.

## Rollback and kill conditions

Revert if a frontend normalizes the observations, raw order explains the frozen
family, feature ablation still passes, the verifier imports producer lowering,
or adapter identity affects the learned semantic hash.
