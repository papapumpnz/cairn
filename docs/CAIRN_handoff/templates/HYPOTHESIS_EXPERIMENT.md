# Pre-registered hypothesis experiment

## Identity

Experiment ID:

Architecture commit:

Semantics version:

Registration timestamp UTC:

Owner:

## Hypothesis

One falsifiable sentence. It must predict an observable difference between the candidate mechanism and a named baseline.

## Mechanism under test

Describe the smallest architectural change being tested. State what is held fixed.

## Dataset and quarantine

Source or generator:

Split construction:

Development split:

Frozen validation split:

Evaluation split access policy:

Duplicate and near-duplicate checks:

## Baselines

Baseline A:

Baseline B:

Ablation:

## Metrics

Primary metric:

Secondary metrics:

Correctness verification:

CPU and memory metrics:

## Success gate

State an exact threshold before execution.

## Kill gate

State the result that causes the mechanism to be rejected or reduced to a diagnostic role.

## Compute contract

```text
cpu_only: true
physical_core_limit:
ram_limit_bytes:
per_case_work_limit:
per_case_wall_limit:
seed_list:
thread_policy:
```

## Procedure

1. Build from a clean checkout.
2. Run `cairn-check`.
3. Generate or verify the frozen split manifest.
4. Execute baselines and candidate under identical resource controls.
5. Verify every claimed solve independently.
6. Emit raw per-case records before computing aggregates.
7. Seal the artifact pack and hash it.

## Decision rule

Describe exactly how the result changes the architecture. Do not replace the registered rule after seeing results.

## Results

Leave empty until execution.
