# ADR-0041 — Reconstruct invented predicate memory from canonical evidence

Status: accepted

## Context

ADR-0039 makes invented predicate/aggregate compositions persistent in one
process, but trusted status cannot survive restart. Continual learning requires
durable evidence while preserving verifier and consolidator authority.

## Decision

Persist attributed grouping cohorts under canonical schema
`cairn.memory.predicate-grouping-cohort` version 1, semantics 1.0.0. The payload
contains sorted raw traces, the claimed mechanism identity, replay count, and
counterfactual candidate measurements. It never contains a learned native
object, verifier receipt, promotion certificate, or module status.

After restart, load the artifact as untrusted, re-invent the complete executable
predicate/aggregate, independently replay it, regenerate reuse receipts, invoke
the separate consolidator, and insert a fresh provisional module before applying
the new certificate. Entity and episode ordering are non-semantic.

## Alternatives rejected

- Serialize a persistent store: restores authority without reproduction.
- Serialize verifier receipts: a receipt is a conclusion, not replayable source
  evidence.
- Store only the learned RKL definitions: cannot establish that they remain
  supported by source experience.

## Compatibility and migration

This adds a new schema and artifact kind. Existing process-local modules remain
valid but have no automatic durable migration without their source cohorts.

## Validation

Require two distinct canonical artifacts, entity-order identity invariance,
missing-parent rejection, full process-state loss, capped replay failure, two
fresh inventions, two fresh verifier receipts, a new consolidation certificate,
one fresh promotion, and exact zero-example held-out reuse.

## Rollback and kill conditions

Revert if loaded status becomes authoritative, reordered entities change
artifact identity, replay can be skipped, a single cohort promotes, or changed
work/replay measurements are accepted.
