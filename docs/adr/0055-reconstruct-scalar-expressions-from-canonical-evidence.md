# ADR-0055: reconstruct scalar expressions from canonical evidence

Status: accepted

## Context

ADR-0054 introduced independently verified scalar-expression mechanisms and
in-process proposal memory. Process loss discarded the memory correctly, but
also discarded the evidence needed to reconstruct useful mechanisms. Serializing
an executable expression, verifier receipt, promotion certificate, or module
status would improperly restore authority.

## Decision

Introduce `cairn.memory.scalar-expression-cohort` schema version 1 with
semantics 1.0.0. The artifact contains sorted scalar traces, the committed
source-independent mechanism hash, and verifier-derived work accounting. It
does not contain the expression AST, RKL definition, channel bindings, receipt,
promotion certificate, or memory status.

After restart, load returns untrusted evidence only. The ordinary scalar learner
must rediscover the expression and bindings, the independent verifier must
replay every example and reconstruct accounting, and the consolidator must
receive two disjoint fresh receipts before a blank store becomes persistent.

## Compatibility and migration

This is a new additive artifact kind and does not change existing relation or
scalar mechanism identity. Unsupported schema or semantics versions fail
closed. Existing in-process scalar modules require no migration and acquire no
durability automatically.

## Validation

Persist two independently verified cohorts, destroy all process objects, reopen
and recover CAS, demonstrate zero restored authority, reject insufficient replay
capacity, freshly reconstruct both cohorts, promote once, and exactly cover a
changed future trace through the reconstructed mechanism.

## Rollback and kill conditions

Remove archive admission if loading restores executable state, if replay can be
skipped, if canonical identity changes across restart, if accounting is trusted
rather than recomputed, or if malformed lineage is accepted.

