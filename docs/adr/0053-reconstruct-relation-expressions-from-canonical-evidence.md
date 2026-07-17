# ADR-0053: reconstruct relation expressions from canonical evidence

Status: accepted

## Context

ADR-0051 learned and promoted a composite relation expression, but its
proposal-only library existed only in process memory. Serializing the learned
AST, receipt, promotion certificate, or status would let persistence restore
authority without repeating invention and independent verification.

## Decision

Add canonical schema `cairn.memory.relation-expression-cohort` version 1 with
semantics 1.0.0. Each artifact stores only sorted raw relation observations,
the committed mechanism hash, and verifier-derived replay/work accounting.
It stores no expression AST, RKL definition, receipt, promotion certificate,
or module status.

`RelationExpressionCohortArchive::load` returns untrusted evidence. `replay`
invokes the ordinary expression inventor and the independent expression
verifier, then checks semantic identity and accounting. A fresh
`RelationExpressionStore` requires two disjoint replay receipts and a new
`RelationExpressionPromotionConsolidator` certificate before it exposes the
mechanism as a proposal source. `CognitionRuntime` rebinds persistent
expressions to current answer-hidden evidence but cannot accept an answer.

## Compatibility and migration

This is an additive artifact kind and runtime memory field. No existing schema,
semantic identity, or verifier certificate changes. There is no migration from
opaque process state; existing in-memory expressions must be re-observed and
persisted through the verified cohort API.

## Validation

The restart integration persists two disjoint cohorts, closes and recovers the
CAS, proves a blank store has zero persistent authority, rejects a replay under
an insufficient work cap, re-invents both cohorts, issues two fresh verifier
receipts and one new promotion, and emits one exact answer-hidden future
proposal. Expression work remains 4→1 and examples 4→0.

## Rollback or kill conditions

Remove the archive path if loaded fields can restore status, if replay imports
producer authority, if altered lineage/schema/hash is accepted, if a
single-cohort certificate promotes, or if runtime memory can accept an output
without independent current-task verification.

