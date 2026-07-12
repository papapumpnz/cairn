# ADR-0024: Stop cardinality-layer expansion as an ARC transfer path

Status: accepted research stop

Date: 2026-07-13

Owners: WP34 integration

## Context

ADR-0023 pre-registered a modality-neutral coordinate-layer learner and required
its ARC contribution to be measured only after the non-ARC core and adapter
harness were committed. The learner passes its non-ARC invention,
verification, promotion, support, renderer-ablation, and vector-to-field
transfer gates.

The frozen complete ARC run remains 36/400 exact with zero incorrect. The
first-40 all-memory run is 19/40, but the additional solve over the older
partition-only measurement belongs to an existing partition-extraction
mechanism. A direct layer-only versus blank-memory causal ablation is 17/40
versus 17/40. Its required strict accuracy improvement assertion fails.

## Decision

Stop expanding the minimum/maximum-cardinality layer-set grammar for the purpose
of raising ARC accuracy. Retain the generic implementation because it has
independently verified non-ARC cross-domain utility. Do not add new roles,
expressions, renderer modes, ordering preferences, or adapter exceptions based
on the observed ARC cases.

The next research proposal must originate from frozen non-ARC graphics or
manipulation families and learn correspondence across changed geometry,
including entity-to-entity mappings and construction provenance. Before any ARC
run, it must demonstrate held-out transfer, negative cases, independent replay,
promotion from disjoint cohorts, and a memory-off/on causal ablation in the
source domains.

## Alternatives rejected

- Reordering the layer proposal ahead of existing bindings would change
  attribution but not exact coverage.
- Adding task-derived roles or geometry operations would violate quarantine.
- Weakening the causal gate to count applicability would confuse redundant
  explanation with added capability.
- Removing the generic layer implementation would discard a verified non-ARC
  mechanism solely because it did not improve one benchmark.

## Compatibility and migration

No runtime interface or artifact semantics change. The failing ignored ARC
ablation remains reproducible evidence. Existing layer mechanisms remain valid
proposal-only memory artifacts.

## Validation and rollback

Evidence is recorded in
`docs/experiments/evidence/arc1_first40_layer_ablation_wp34_v0.10.json` and
`docs/experiments/evidence/arc1_training_400_wp34_v0.10.json`. Reopen this path
only if an independently authored non-ARC curriculum demonstrates a materially
richer general mechanism and its frozen causal ablations pass without changing
evaluation-derived thresholds.
