# Relation-defined group merge memory

Date: 2026-07-13  
Decision: ADR-0032  
Status: PASS_WITH_DECLARED_LIMITS

CAIRN receives finite groups but no group/output correspondence. It computes
exact generic aggregate evidence and invents an executable RKL definition. The
frozen family requires `maximum - minimum`; copying one aggregate does not
solve it.

```text
independent_cohorts: 2
verified_experiences: 4
fresh_candidates: 185
memory_attempts: 1
memory_examples: 0
heldout_groups: 6
heldout_exact: 6/6
wrong_family_accepts: 0
blank_memory_proposals: 0
```

The independent verifier recomputes every aggregate and executes raw RKL. Two
disjoint cohorts are required for persistence; reuse remains proposal-only.

This is bounded many-to-one aggregation, not learned grouping. Groups are
supplied. Recursive aggregation, noisy grouping, raw entity discovery, and
open-ended general intelligence remain open.
