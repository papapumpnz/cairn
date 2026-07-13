# Joint grouping-relation and merge induction

Date: 2026-07-13  
Decision: ADR-0033  
Status: PASS_WITH_DECLARED_LIMITS

The learner receives two grouping relations per episode and no selected role.
Both have identical group counts. The distractor contains aggregate-identical
groups that would require different outputs, so no deterministic group
function can explain it. The structural grouping admits one RKL definition
across all episodes.

```text
candidate_relations: 2
selected_relations_supplied: 0
training_domains: 2
heldout_groups: 4
heldout_exact: 4/4
missing_relation: rejected
distractor_exact_theories: 0
```

This removes the single-trusted-grouping assumption from task learning. It does
not yet persist the relation-bound composition as one artifact, and candidate
relations still come from frontends. Raw relation/entity discovery remains
open.
