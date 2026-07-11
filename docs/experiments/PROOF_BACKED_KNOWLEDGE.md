# Proof-backed structured knowledge

Date: 2026-07-11

Status: structured knowledge subsystem checkpoint

## Representation

The new `cairn-knowledge` evidence crate stores:

- source observations whose content identities are independently verified;
- stable entity and claim identifiers;
- attribute, causal, taxonomic, temporal, procedure, and custom predicates;
- entity, integer, text, and verified-mechanism terms;
- inclusive temporal validity intervals;
- fixed-point confidence in parts per million;
- complete source-observation provenance;
- subject/predicate/time retrieval indexes;
- explicit contradiction pairs without destructive overwrite;
- reversible entity-merge proposals.

`CognitionRuntime` owns this knowledge base alongside scalar and procedural
mechanism memory.

## Frozen test

Two independently admitted structured-document observations assert different
operating modes for the same entity over overlapping time intervals. Both
claims remain retrievable at the overlap instant and one contradiction pair is
recorded. A third claim links the entity to a verified executable stabilization
mechanism. An alias merge is proposed with two-source provenance but is not
silently applied.

Fail-closed validation covers invalid observation identities, empty predicates,
confidence outside 0..1,000,000, inverted validity, missing/unknown sources,
duplicate claims, self-merges, and unsupported merge evidence.

Command:

    cargo test --locked -p cairn-knowledge --all-targets

## Claim boundary

This is practical proof-backed acquisition from already structured evidence.
It is not natural-language understanding. Deterministic lexing/parsing,
coreference, grammar induction, entity extraction from prose, source-quality
models, and large-corpus scaling remain open and must be measured separately.
