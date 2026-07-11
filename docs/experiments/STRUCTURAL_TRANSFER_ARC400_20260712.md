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
| Non-ARC structural memory | 33/400 (8.25%) | 367 | 0 |

The only gained task was `09629e4f`; no baseline exact solve was lost. Frozen
ARC-40 changed from 17/40 to 18/40. The complete structural run took 637.62
seconds. Retrieval checks the binding on every task, so total runtime did not
improve; typed structural indexing is now required.

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

80% ARC-1: **failed**. The measured result is 8.25%. This must not be described
as an ARC pass or general intelligence.

