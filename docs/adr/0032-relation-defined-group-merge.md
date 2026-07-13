# ADR-0032: relation-defined group merge

Status: accepted for bounded implementation

## Context

ADR-0031 learns one-to-many expansion. CAIRN still cannot learn many-to-one
construction without adding a named domain merge or extending RKL with a new
fold primitive before exact grouping semantics are established.

## Decision

Introduce a domain-neutral grouped observation contract. Each episode contains
finite non-empty groups of exact integers and only an unordered set of expected
outputs. Group/output correspondence is absent. The trusted substrate computes
bounded `count`, `sum`, `minimum`, and `maximum` values for each supplied group.
The ordinary RKL inventor synthesizes one output definition over those generic
aggregates.

The learned mechanism contains only executable RKL, support cardinalities, and
proof lineage. Independent verification recomputes aggregates and complete
output sets. Two disjoint cohorts are required for proposal-only persistence.

Grouping itself is evidence supplied by a relation frontend in this slice. A
later gate must invent the grouping relation jointly; this ADR does not treat
supplied groups as general perception.

## Validation and kill conditions

Freeze non-ARC families whose output is a composition of aggregates rather
than a copied aggregate. Vary values, group sizes, group counts, order, and
domain labels. Require held-out larger groups, exact two-cohort replay,
zero-example reuse, blank-memory failure, and wrong-grouping rejection. Stop if
outputs are paired to groups or a learned module bypasses current execution.
