# ARC legacy schema quarantine

Date: 2026-07-11

Status: physical core/adapter isolation pass

## Change

The historical named finite-field transformation catalog and all twenty of its
tests moved from `crates/cairn-field` to `adapters/cairn-arc`.

The generic field crate now contains only:

- exact finite observations and provenance;
- generic relational evidence emission;
- task-local pointwise RKL invention;
- residual-driven constructor invention.

Named legacy transformations such as marker broadcast, rotational completion,
and template repetition are compiled only as part of the quarantined adapter.
They cannot enter scalar or procedural mechanism memory and do not count as
general-learning evidence.

An automated source test asserts that representative legacy names occur in the
adapter implementation and not in the general field crate.

## Regression

Adapter semantics advanced to 1.5.0 for the architectural move. Behavior is
unchanged:

    official first ten:       10/10 exact
    official first twenty:    17/20 exact
    resource limited:          3/20
    legacy schema tests:      20/20 pass

## Claim boundary

This removes ARC-oriented named operations from the general package graph; it
does not convert the 17 legacy successes into learned-mechanism successes. The
legacy backend remains a quarantined regression oracle until generic relational
theory induction replaces it.
