# Relation-path and one-to-many expansion experiment

Date: 2026-07-13  
Decisions: ADR-0030, ADR-0031  
Status: PASS_WITH_DECLARED_LIMITS

## Relation-path result

The frozen hard negatives have identical attributes and identical direct
in/out relation degrees. Only positives complete an ordered two-edge path.

```text
path_length: 2
independent_promotion_cohorts: 2
verified_experiences: 4
fresh_candidates: 110
memory_attempts: 2
memory_examples: 0
training_output_cardinalities: 1..4
held_out_output_cardinalities: 5..6
held_out_exact: 2/2
finite_field_path_transfer: 1 exact
changed_truth_accepts: 0
path_ablation: capability removed
```

The selector feature grammar enumerates observed ordered role pairs. The RKL
contrast learner chooses the required path; no path name or selected binding is
provided. The independent verifier recomputes distinct reachable endpoints.

## Expansion result

```text
invented_multiplicity: 2
invented_constructor: executable RKL over item and bounded slot
input/output_pairing: absent
independent_promotion_cohorts: 2
verified_experiences: 4
fresh_candidates: 522
memory_attempts: 1
memory_examples: 0
held_out_inputs_outputs: 6/12 exact
wrong_family_accepts: 0
order_invariance: exact
```

The learner tries bounded multiplicities, creates unordered `(item, slot)`
environments, and invents the constructor from the ordinary substrate. It does
not receive a `split`, `duplicate`, or expansion template.

## Interpretation and limits

These are mechanically demonstrated learning gains: absent executable
mechanisms are synthesized, independently replayed, promoted from disjoint
experience, and reduce future search and examples. They remain narrow bounded
families. Relation roles and entities are frontend supplied. Expansion is
scalar and fixed-multiplicity; merge, conditional multiplicity, raw perception,
open-ended curricula, and broad general intelligence remain unproved.

## ARC regression after the general mechanisms froze

```text
conditional transfer first 10: blank 10/10, memory 10/10, incorrect 0/0
official frozen first 20: 17/20
```

The new general mechanisms do not improve this saturated first-ten slice, and
the expansion library is not yet bound to spatial entity creation. No ARC gain
is claimed and no core grammar was changed from these results.
