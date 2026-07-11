# Structural transfer and ARC-1 full-corpus ablation

Date: 2026-07-12

## Objective

Test whether a structural render binding learned outside ARC can improve a
frozen ARC-AGI-1 evaluation without benchmark answer memory, task identifiers,
colour constants, or an ARC-specific cognitive primitive.

## Non-ARC curriculum

Generated dashboard/table families contained equal partitions separated by
different categorical values. One partition controlled cellwise broadcast into
the others. Controller position, separator, values, and geometry varied. The
learned invariant controller role was the unique partition omitting a marker
shared by every other partition.

The ARC adapter must rebind that role and replay every training output exactly.
Any mismatch rejects the binding and uses the unchanged learner. Test outputs
remain sealed until scoring.

## Frozen results

Candidate budget: 1,000 per task. Corpus: all 400 sorted ARC-AGI-1 v1.0.2
training tasks.

| Condition | Exact | Resource-limited | Incorrect |
|---|---:|---:|---:|
| Blank structural memory | 32/400 (8.00%) | 368 | 0 |
| Non-ARC structural memory, controller only | 33/400 (8.25%) | 367 | 0 |
| Controller plus relational extraction | 35/400 (8.75%) | 365 | 0 |
| Plus connected-entity frontend rebinding | 36/400 (9.00%) | 364 | 0 |

Gained tasks were `09629e4f`, `1cf80156`, and `2dc579da`; no baseline exact
solve was lost. Frozen ARC-40 changed from 17/40 to 18/40. The controller-only
run took 637.62 seconds and the combined run took 692.42 seconds. Retrieval
checks bindings on every task, so total runtime did not improve; typed
structural indexing is now required.

The later connected-entity run adds `be94b721` through the unchanged offline
AI-distilled selector and completed in 467.20 seconds. Because this was not a
paired timing experiment, the lower elapsed value is not claimed as a speedup.

## Other evidence

- composition generalized to an order-sensitive depth-three schedule;
- verifier replays now become reuse receipts and an automatically consolidated
  promotion certificate;
- typed relational entity selectors transferred to an unseen diagram domain;
- frontier gaps now generate causal curriculum exercises automatically;
- 100 unknown action-law worlds produced 32,400 exact predictions using 300
  active interventions versus 791 random interventions;
- frozen developmental capability rose from 1/3 to 3/3, examples fell from 10
  to 6, and median composed acquisition was about 1.60 ms versus 3.93 s fresh,
  with zero measured negative transfer.

## Gate decision

Cross-domain positive transfer: **pass on one mechanism family**.

General intelligence: **not established**. The demonstrated domains remain
small, discrete, deterministic, and bounded.

80% ARC-1: **failed**. The latest measured result is 9.00%. This must not be described
as an ARC pass or general intelligence.
