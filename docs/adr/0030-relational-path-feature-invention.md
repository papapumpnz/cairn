# ADR-0030: bounded relational-path feature invention

Status: accepted for WP34 implementation

## Context

The conditional selector introduced by ADR-0029 can use attributes and direct
relation-role degrees.  It cannot distinguish an entity whose neighbour has a
required causal relation from a hard negative having the same direct degree.
Supplying a named two-hop concept would move the missing intelligence into the
frontend.

## Decision

The generic selector feature grammar gains a bounded, executable
`TwoHopOutDegree(first_role, second_role)` form.  Its denotation is the number
of distinct endpoints reachable from the candidate entity by exactly one edge
of `first_role` followed by exactly one edge of `second_role`.  All ordered
role pairs observed in the current collection schema are candidates; no path
name, domain label, selected entity, or output value is supplied.

The existing bounded RKL contrast inventor chooses whether a path feature is
necessary.  Canonical program identity records both roles and order.
Independent verification recomputes the path denotation from relation records
without importing selector synthesis.

## Compatibility

The feature enum is additive. Existing serialized conditional programs retain
their explicit feature list and semantics. New selectors may have additional
unused parameters, but minimum-cost Boolean bodies and exact replay remain the
acceptance criterion.

## Validation and kill conditions

Freeze non-ARC families in which positives and negatives have identical
attributes and direct in/out degrees, but only positives complete the required
ordered two-edge path. Vary identities, order, cardinality, geometry, and
domain labels. Path deletion/reversal must fail, independent replay must pass,
and two disjoint cohorts must synthesize the same persistent program. Stop if a
one-edge feature separates the frozen hard negatives or if the verifier calls
the learner.
