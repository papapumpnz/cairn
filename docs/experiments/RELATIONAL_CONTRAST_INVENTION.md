# Relational evidence and contrast-driven invention

Date: 2026-07-11

Status: bounded post-G6 capability checkpoint

## Relational evidence bridge

An exact finite-field observation now decodes into a provenance-bound
Relational Evidence Graph:

- every primitive element is a generic scalar node;
- row, column, and categorical value are typed attributes;
- orthogonal adjacency is an exact binary relation;
- same-value adjacency is a separate exact relation;
- the observation identity is the derivation root.

The bridge contains no benchmark task identity or transformation names. A
diagram fixture passes the graph to `EntityGenerator`, which proposes connected
components and attribute groups from relations rather than a supplied object
list. The official ARC first-20 adapter run emits 23,114 such nodes and 70,550
facts from 150 sealed observations, while retaining the 17/20 legacy baseline.

## Contrast-driven predicate invention

The new `ContrastPredicateInventor` receives only typed feature bindings and
positive/negative labels. It constructs an executable Boolean RKL definition
through the ordinary bounded inventor.

The frozen family varies categorical value and size. Selected entities are
both enclosed and associated with a marker; hard negatives violate either or
both conditions. No `eligible` predicate or selector template is supplied.

Result:

    candidates evaluated: 2,086
    structural cost:       5
    training bindings:     5
    held-out exact:         4/4
    invented body:         1 == min(enclosed, has_marker)

Held-out bindings vary values and sizes unseen during synthesis. Removing the
enclosure or marker feature changes the predicted class as expected.

Commands:

    cargo test --locked -p cairn-field --test relational_evidence
    cargo test --locked -p cairn-theory --test contrast_invention -- --nocapture

## Claim boundary

This is genuine task-local predicate invention over a developer-supplied typed
feature substrate. The follow-on frozen experiment now infers and promotes
strict bounding-box enclosure itself; see `INVENTED_ENCLOSURE_PROMOTION.md`.
Panels, correspondences, residual constructors, irregular enclosure, and the
remaining official tasks remain open.
