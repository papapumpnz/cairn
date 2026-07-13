# ADR-0050 — Grow bounded relation language from verified experience and failure

Status: accepted

Date: 2026-07-14

Owners: WP34 integration

## Context

ADR-0049 proved that a configured ternary fold can be learned and reused, but
the experiment itself raised the relation-key arity and exposed the fold. That
is useful composition evidence, not autonomous language growth. A higher-arity
failure must request the missing structure, and prior experience must constrain
which new proposal family becomes reachable.

## Decision

Developmental curriculum semantics advance to 1.3.0.
`GeneratedDevelopmentalObjective` and `DevelopmentalExerciseRequest` carry the
content-free failure core's exact `minimum_dependency_arity_lower_bound`. The
field exposes no observation, task identity, expected output, or proof.

`RelationObservationLimits` remains closed by default: maximum key arity is two
and `enabled_associative_folds` is empty. Raising an arity cap alone cannot make
a fold reachable. `RelationFoldMask` separately gates the additive, minimum,
and maximum fold families.

`RelationLanguageExtensionConsolidator` is the only authority that may issue a
`VerifiedRelationLanguageExtension`. It requires:

- a source-independent binary relation mechanism;
- a verifier-issued persistent-promotion certificate covering at least two
  disjoint reuse cohorts;
- matching operator, arity, and mechanism identity;
- a content-free failure-core commitment requesting arity at least three; and
- an explicit maximum-arity safety cap.

Only an already supplied associative binary algebra may be closed over higher
arity: addition exposes sum, minimum exposes minimum-fold, and maximum exposes
maximum-fold. Subtraction and every unknown operator fail closed. Applying the
opaque certificate changes proposal reachability only. It grants neither
memory persistence nor answer acceptance.

Canonical evidence archives continue to store observations and commitments,
not language-extension authority. After process loss, base mechanisms must be
re-invented, independently replayed, freshly promoted, and then the extension
must be regenerated from the failure commitment.

## Consequences

The frozen additive experiment begins with no fold capability. A content-free
arity-three failure plus two promoted binary-addition cohorts unlocks only the
ternary sum family. Blank search finds no theory; extended search learns the
fold, obtains two new reuse receipts, promotes it as proposal-only memory, and
replays 9/9 changed held-out groups. Cohort binding work falls 128→16 and later
examples fall 4→0 with zero observed negative transfer.

A separate minimum family proves the gate is not hard-coded to addition:
verified binary-minimum experience unlocks only minimum-fold and independently
replays 9/9 changed held-out groups. Requests below arity three or above the
safety cap fail closed.

The additive extension also crosses actual unchanged adapters: sequence
evidence learns the ternary fold and vector evidence rebinds the same semantic
mechanism over 9/9 independently replayed groups.

This is a bounded form of experience-driven proposal-language growth. The
associativity laws, scalar channels, RKL binary operations, and safety cap are
developer-supplied. It is not unrestricted grammar invention and is not a
general-intelligence result.

## Compatibility and migration

Developmental checkpoints with semantics 1.2.0 fail closed under the 1.3.0
reader and must be reconstructed from their canonical event journals. Failure-
core and generated-objective semantic hashes are unchanged because dependency
arity already entered both commitments; 1.3.0 only propagates that committed
value to the materializer request.

Default relation candidate counts remain compatible with ADR-0048 because the
fold mask is empty and maximum arity remains two. ADR-0049 tests that manually
exercise a fold must now explicitly enable its family. No persistent relation
artifact schema changes.

## Validation

- Default and arity-only configurations cannot solve the omitted-fold family.
- Verified addition experience unlocks sum, never minimum-fold or maximum-fold.
- Verified minimum experience unlocks minimum-fold, never sum or maximum-fold.
- Arity two and arity nine under a safety cap of eight are rejected.
- The additive loop records 128→16 candidate bindings, 4→0 examples, 9/9
  held-out exact groups, and zero observed negative transfer.
- The minimum loop independently records 9/9 changed held-out exact groups.
- The same additive fold transfers sequence→vector over 9/9 exact groups with
  zero cognition-core adapter branches.
- Cold restart restores zero authority and regenerates one extension only after
  two cohorts are re-invented, replayed, and freshly promoted.

## Rollback or kill conditions

Stop if arity growth occurs without a content-free structural failure, if one
binary operator exposes another operator's fold, if stored status restores an
extension, if a certificate bypasses current binding or independent replay, if
non-associative structure is admitted without a new proof contract, or if this
bounded closure is described as general intelligence.
