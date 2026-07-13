# ADR-0031: variable-multiplicity collection construction

Status: accepted for bounded task-local implementation

## Context

Existing collection programs map or filter each input once. They cannot invent
one-to-many construction, so apparent split/create behavior must be supplied by
a developer mechanism.

## Decision

Add a bounded expansion learner over the universal substrate of collections,
integer variables, bounded iteration, and executable RKL definitions. For each
candidate multiplicity from one to a declared cap, the learner constructs
unordered `(item, slot)` environments and asks the ordinary RKL inventor for
one output expression. A candidate is accepted only when the complete output
set exactly matches every episode under reference and optimized replay.

The learned object records the invented constructor, multiplicity, source
episodes, observed input/output cardinalities, and a checked nested-`ForEach`
theory whose slot collection is an explicit bounded input. The multiplicity is
not inferred from a task label or output values; inconsistent ratios fail.

## Limits and promotion

This checkpoint is task-local. Persistent promotion is prohibited until a
separate verifier reconstructs the complete expanded set and two disjoint
cohorts agree on canonical constructor and multiplicity.

## Validation and kill conditions

Vary input values, counts, order, and domain adapters; hold out larger counts.
Require exact one-to-many output, order invariance, ablation failure, and cap
failures. Stop if outputs are paired to inputs by the generator, if literal
answers enter the basis, or if a learned expansion is promoted before the
independent verifier exists.
