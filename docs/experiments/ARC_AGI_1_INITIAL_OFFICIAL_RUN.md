# ARC-AGI-1 initial official-corpus run

Date: 2026-07-11

Status: completed negative baseline; development training split only

## Corpus provenance and quarantine

The source is the official
[`fchollet/ARC-AGI`](https://github.com/fchollet/ARC-AGI) repository, release
`v1.0.2`, commit `aa922be204204ec148a1137fe6ed4d34ddde812b`.

CAIRN imports all 400 files from `data/training`, retains the Apache-2.0
license, and records SHA-256 for each file. The upstream evaluation split is not
imported. This follows the upstream instruction that evaluation tasks must not
be repeatedly used as development feedback.

Task filenames are adapter reporting metadata only. They are not passed to the
generic learner. For each selected task, the runner removes expected test
outputs before calling learning and uses them only after predictions return.

## Frozen initial protocol

- Selection: first 10 training files in bytewise filename order.
- Adapter semantics: `1.0.0`.
- RKL semantics: `1.1.0`.
- Learner: generic pointwise finite-field constructor invention.
- Maximum expression cost: 9.
- Maximum candidates per task: 1,000.
- Scoring: exact complete output for every test case.
- Attempts: one generated prediction per test input; no retry tuning.

Command:

```sh
cargo run --locked -p cairn-arc --bin cairn-arc-batch -- \
  datasets/arc-agi-1-v1.0.2/training 10 1000 \
  artifacts/arc-agi-1-v1.0.2/initial-10.json
```

## Result

| Result class | Tasks |
|---|---:|
| Parsed | 10 |
| Exact | 0 |
| Incorrect prediction | 0 |
| Unsupported output-shape change | 3 |
| Candidate cap reached | 7 |
| Invalid input | 0 |
| Execution failure | 0 |

The result is not an ARC score. No selected task reached a generated test
prediction: the current frontend either rejected a shape change it cannot
represent or exhausted the bounded pointwise expression search.

A secondary deterministic breadth probe used the first 100 files with a cap of
100 candidates. It parsed all 100, classified 27 as shape-changing, and reached
the candidate cap on 73. This confirms the runner and quarantine at larger
scale; its smaller budget is not compared as an accuracy result.

## Interpretation

This is valuable failure evidence. The synthetic finite-field success did not
translate into immediate official ARC coverage. The current learner needs:

1. learned canvas/output-shape construction;
2. entity- and relation-conditioned guards connected to field evidence;
3. variable-arity writes and copy/create/delete event weaving;
4. observational deduplication and staged search so moderate candidate budgets
   do useful work before materializing large expression frontiers;
5. per-task process isolation or incremental checkpointing for larger searches.

No ARC-specific operation will be added to address these failures. Each new
mechanism requires an environment-neutral synthetic family and a non-ARC case
before this frozen batch is rerun.

## Follow-up

The frozen batch subsequently reached 10/10 after adding generic, synthetic-
tested finite-field schemas. The untouched tasks 11–20 remained 0/10. See
`ARC_AGI_1_FROZEN_TEN_PASS.md`; the baseline in this document remains preserved.

## Artifacts

- `datasets/arc-agi-1-v1.0.2/SOURCE.md`
- `datasets/arc-agi-1-v1.0.2/SHA256SUMS`
- `artifacts/arc-agi-1-v1.0.2/initial-10.json`
- `artifacts/arc-agi-1-v1.0.2/frozen-first-100-budget-100.json`
