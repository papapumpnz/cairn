# ADR-0012: Compile internal proof patterns into source-independent mechanisms

Status: accepted

Date: 2026-07-12

Owners: WP23 integration

## Context

ADR-0011 introduced boundary-aware internal compression, but its proposal
identity deliberately included source episodes and measured costs. The same
causal structure learned from a second evidence cohort therefore had a different
proposal identity. Such a proposal is evidence, not a stable reusable library
item. Runtime reuse also required manually assembling promotion evidence rather
than allowing verified experience to drive consolidation.

## Decision

Consolidator semantics 2.3.0 adds `CompiledInternalMechanism`. Its canonical
semantic identity contains only the typed internal pattern, typed boundary and
value-parameter signatures, and output. Source proposal identity remains
explicit provenance but is excluded from mechanism identity. The reference
executor rebinds the schema to a host proof and expands it using only explicit
boundary and value bindings.

`InternalMechanismVerifier` independently reconstructs the source-independent
identity, replays the source proposal, and validates every source and current
host invocation. It derives conservative fixed-work reuse metrics itself:
candidate windows at exactly the learned width for fresh mining versus compiled
matching, source examples versus one current host, and uncompressed proof units
versus one bound invocation. The caller cannot select a wider fresh-search range.

Memory semantics 1.6.0 adds `InternalMechanismStore`. It accepts only a compiled
mechanism with an opaque compilation token. Each runtime reuse must arrive as
an opaque verifier receipt. Independently verified proposals that compile to
the same semantic mechanism may be registered as additional lineage; receipts
remain invalid until their proposal lineage is registered. After the second
independent receipt, the store
automatically calls the separate `InternalPromotionConsolidator`; only its
certificate changes status from provisional to persistent. Runtime callers
cannot directly construct a certificate or set persistent status.

`CognitionRuntime` owns this internal-mechanism memory alongside definition,
whole-theory, and knowledge memory.

## Alternatives considered

### Use the compression-proposal hash as mechanism identity

This preserves provenance but makes identity depend on which examples happened
to be observed. It was rejected because equivalent independent cohorts must
converge on one semantic mechanism.

### Compile arbitrary proof operations directly into RKL

Proof operation names do not carry sufficient reference semantics to lower
arbitrary operations into RKL expressions. Inventing such semantics would be a
hidden native primitive. The accepted first compiler is an executable typed
proof schema; later RKL lowering requires explicit operation definitions.

### Let runtime promote after counting uses

This is simpler but allows use count to self-attest. It was rejected: invocation
and learning-cost reduction must be independently replayed, and consolidation
must remain a separate authority.

## Consequences

Correctness: compiled identity is canonical and source independent; lineage is
still checked before every receipt and certificate.

Learning: two verified changed-host invocations automatically cause a
consolidator-issued promotion certificate. Equivalent verified cohorts can
contribute jointly; duplicate and unregistered-proposal evidence fail closed.

Compute: the frozen family measures 18 fresh candidate windows versus nine and
five compiled matching windows. Examples fall from four to one and proof units
from 28 to four per invocation.

Persistence: persistent is currently a validated in-memory lifecycle state.
Canonical compiled bytes exist, but a dedicated CAS module archive remains
required for restart durability.

Verifier: the verifier duplicates mechanism identity, source replay, current
invocation expansion, and fixed-work counting rather than trusting producer
metrics.

## Validation

Two independent four-episode cohorts produce distinct evidence proposal hashes
but the identical compiled mechanism hash. Source replay covers all eight
episodes. Three changed hosts rebind in producer, verifier, and memory tests.
Two independent runtime receipts cause automatic promotion; one remains pending.
Duplicate experience, changed episode, changed type, missing boundary, and
unregistered proposal lineage are rejected. After independent compilation is
registered, two different proposal cohorts safely contribute to one certificate.

Rollback trigger: cohort-dependent compiled identity, verifier disagreement,
promotion after one experience, acceptance of mixed lineage, or any missing
strict learning-cost reduction.

## Compatibility

Consolidator semantics: 2.2.0 to 2.3.0.

Memory semantics: 1.5.0 to 1.6.0.

Existing whole-graph and internal compression proposals remain unchanged.
Consumers opt into the compiled mechanism and automatic promotion interfaces.

## References

- ADR-0007 self-compressing mechanism learner
- ADR-0011 boundary-aware internal proof compression
- `docs/experiments/COMPILED_INTERNAL_MECHANISM_REUSE.md`
