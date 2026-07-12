# Conditional relational construction experiment

Date: 2026-07-13  
Decision: ADR-0029  
Status: PASS_WITH_DECLARED_LIMITS

## Result

```text
independent_cohorts: 2
verified_experiences: 4
observed_output_cardinalities: 1..4
held_out_output_cardinalities: 5..6
fresh_candidates: 140
memory_attempts: 2
memory_examples: 0
heldout_exact: 2/2
finite_field_adapter_transfer: 1 exact
changed_relation_truth_accepts: 0
blank_memory_proposals: 0
```

The learner invents a Boolean RKL selector over generic relation-role degree
and a v2 tuple-valued RKL point definition. Joint point invention preserves
coordinate pairing. The verifier separately recomputes relational features and
executes raw definitions. Two disjoint verified cohorts are required before
proposal-only persistence.

## ARC quarantine result

The program was learned only from non-ARC vector, sequence, diagram, table,
graph, and workflow examples. A field adapter derived the same generic
ordered-contact relation from current evidence.

```text
ARC first 10 blank:    10 exact, 0 incorrect
ARC first 10 transfer: 10 exact, 0 incorrect
ARC first 20 comparison: CAPACITY (600-second fixed command budget)
ARC first 40 comparison: stopped after an earlier unbounded run exposed the
                         need for an explicit command cap; no score recorded
```

The first ten are saturated and show no gain. No claim is made for the first
20, first 40, or complete corpus. Evaluation evidence did not change the
learned definition or core grammar.

## Limits

Entities, categorical values, and relation evidence are supplied by frontends.
The program filters/deletes and transforms preserved entities; it does not
invent attributes, create multiple outputs per input, merge, recurse, or
discover raw entities. This is bounded mechanism learning, not general
intelligence and not an 80% ARC system.
