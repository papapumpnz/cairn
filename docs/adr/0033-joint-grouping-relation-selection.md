# ADR-0033: joint grouping-relation selection

Status: accepted for bounded implementation

## Context

ADR-0032 learns a merge only after one grouping is supplied as truth. This
places a central ontology decision outside learning.

## Decision

Each episode may provide several immutable relation-defined grouping
hypotheses. The learner evaluates every role shared across episodes, attempts
ordinary unordered group-merge invention, and retains the minimum-cost exact
explanation under canonical role/hash tie-breaking. A role is accepted only if
the same executable mechanism explains every episode. Held-out execution
rebinds the retained role to current groups and re-executes the mechanism.

No role label has built-in semantics. Frontends still propose candidate
relations; raw relation discovery remains a later gate.

## Validation

Hard alternatives have the same group counts but contain aggregate-equivalent
groups requiring different outputs, making any deterministic group function
impossible. Vary role names only across independent families where semantic
binding is explicit; within a learned mechanism the selected role is part of
canonical applicability. Missing-role and ambiguous evidence fail closed.
