# ADR-0025: Learn collection correspondence with construction provenance

Status: accepted experiment freeze

Date: 2026-07-13

Owners: WP18/WP19/WP23/WP29/WP34 integration

## Context

The cardinality-layer mechanism passes non-ARC transfer but fails its causal ARC
accuracy gate. It flattens a construction into one set and cannot explain which
source entity and point caused each output entity and point. Existing relational
policies bind proof values, while the affine field sampler is task-local and
cell based. Neither is persistent collection-level geometric correspondence.

## Decision

Add a modality-neutral finite entity-geometry scene and learned
`EntityCorrespondenceMechanism`. A mechanism maps every source entity through
one bounded signed-axis correspondence and either preserves the observation
frame or normalizes to the collection's tight frame. Execution returns an
entity render plus point-level provenance mapping every emitted coordinate to
its source entity and coordinate. Conflicting provenance fails closed.

The learner exhaustively enumerates the bounded axis/frame space in canonical
order and accepts only one exact mechanism. The independent verifier
reimplements geometry, bounds, normalization, correspondence, provenance, and
replay. Persistent promotion requires two disjoint verified cohorts and records
explicit support geometry. Runtime memory remains proposal-only.

## Frozen non-ARC families

Before ARC access, independently learn and hold out:

1. tight-frame reflection of multiple differently shaped entities;
2. preserved-frame axis permutation with changed extents;
3. changed identities, values, order, count, positions, and distractor geometry;
4. symmetric ambiguity, collision, malformed coordinates, and capacity limits;
5. vector-to-categorical-field transfer using the unchanged mechanism.

Fresh learning versus persistent reuse must show fewer examples and candidate
executions with unchanged exactness and zero negative transfer.

## Compatibility

All interfaces are additive and use a new canonical marker. Existing layer,
world, proof, memory, verifier, and adapter semantics are unchanged.

## ARC quarantine and kill conditions

Do not open ARC until the source families, ablations, verification, promotion,
memory-off/on reduction, cross-domain transfer, documentation, and harness are
committed. Stop if IDs or values determine correspondence, truth enters runtime,
provenance is incomplete, ambiguity tie-breaks, negative transfer is nonzero,
or source reuse fails to reduce work. After freezing, require a strict ARC
memory-off/on accuracy gain; applicability or reordered attribution do not count.

## Rollback

The additive module can be removed without migration. Failed source gates kill
the experiment. Passed source gates with failed ARC transfer retain the general
capability but block benchmark-directed grammar growth.
