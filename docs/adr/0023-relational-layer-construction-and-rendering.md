# ADR-0023: Learn relational layer construction and rendering from graphics

Status: accepted

Date: 2026-07-13

Owners: WP16/WP18/WP19/WP23/WP29/WP34 integration

## Context

The v0.9 world learner transfers finite action/state laws but does not improve
static ARC. The largest remaining architectural gap is not another benchmark
operation: cognition lacks a persistent, modality-neutral way to select
relational layers, construct a new coordinate set, and bind that construction
to an explicit output frame. This capability must be learned and falsified in
non-ARC graphics before the ARC adapter may retrieve it.

## Decision

Add a finite exact `LayerScene` representation to `cairn-theory`. A scene has
bounded integer-coordinate layers, categorical attributes, extents, and an
explicit background. No raster, colour name, ARC identifier, or output answer
is part of mechanism identity.

The learner synthesizes a `LayerMechanism` from the universal set substrate:

- select layers by relational cardinality roles, with ties failing closed;
- construct coordinates through input, union, intersection, or ordered set
  difference expressions;
- select the rendered categorical attribute by a rebound layer role;
- choose either source-frame preservation or tight content bounds;
- normalize tight-frame coordinates explicitly.

These are grammar nodes over sets, relations, ordering, and construction—not
named benchmark transformations. Candidate order is deterministic and bounded.
Every training output must replay exactly. Support records varied extents,
values, cardinalities, positions, and expression partners.

The independent verifier reimplements selection, set construction, frame
normalization, and rendering. Memory promotion requires distinct verified
graphics families and held-out rebinding. Runtime retrieval returns mechanisms
only as proposals.

The vector adapter supplies the source curriculum. The finite-field adapter may
lower categorical layers into the same `LayerScene` and reconstruct a field
only after every current demonstration replays. ARC receives this frozen
adapter proposal after non-ARC gates pass; evaluation cannot change the grammar
or ranking.

## Frozen non-ARC families

Before ARC access, test at least:

1. intersection of differently valued overlapping layers with tight rendering;
2. ordered difference under changed positions and cardinalities;
3. union with preserved source frame;
4. hard negatives with selector ties;
5. renderer conflict and changed categorical values.

## Compatibility

All interfaces are additive. `LayerMechanism` receives a new canonical
semantics marker. Existing world, proof, memory, vector, field, and ARC schemas
are not reinterpreted.

## Validation and kill conditions

Non-ARC held-outs must replay exactly with blank-memory and expression/renderer
ablations failing predictably. Cross-domain field demonstrations must all replay
before prediction. False acceptance must remain zero.

After the core is committed, run frozen ARC first-40 and full-400 comparisons.
An unchanged score is retained as a negative result. Stop this path if task IDs,
literal grids, corpus ordering, output lookup, unverified memory, or ARC-derived
grammar changes enter the mechanism.
