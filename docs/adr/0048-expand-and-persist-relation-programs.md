# ADR-0048 — Expand and persist relation-key programs

Status: accepted

Date: 2026-07-13

Owners: WP34 integration

## Context

ADR-0047 can invent only equality over one anonymous attribute. It therefore
cannot discover a relation whose key is absent from every atomic channel, and
its memory stores only the downstream observation mechanism rather than the
complete invented relation program. That is evidence of binding selection, but
not of reusable relation-structure learning.

## Decision

Replace the single projection key with a bounded, typed, cost-phased relation
key grammar. A binary relation connects primitives when independently evaluated
keys are equal. Key programs are projection, addition, subtraction, minimum,
and maximum over task-local anonymous attribute bindings. Projection candidates
are tested first; binary structures become reachable only when that cheaper
phase has no exact theory. Equal-cost exact alternatives fail as ambiguity.

Persist the source-independent complete `RawRelationMechanism`: canonical RKL
relation definition, key operator, key arity, and downstream executable observation
mechanism. Attribute indexes, episode identities, generated relation roles,
outputs, receipts, status, and certificates are excluded. Its semantic marker
is `cairn.raw-relation-observation.v2` and explicitly commits to operator and
arity as well as definition bytes; v1 values are not treated as equivalent.

An independent verifier type-checks and executes relation RKL over raw
attributes, reconstructs edges and entities through a separate path, and
derives binding-search reduction. A separate consolidator requires two
disjoint exact reuse receipts before issuing promotion. Runtime memory filters
the current binding grammar by the learned operator, but its outputs remain
untrusted proposals requiring current-evidence verification.

Canonical CAS artifacts use schema `cairn.memory.raw-relation-cohort`, version
1, semantics 2.0.0. They contain raw evidence and commitments only. After
process loss, the learner must re-invent, independently replay, reconsolidate,
and freshly promote; loaded data restores no authority.

## Consequences

One frozen family has no explanatory atomic key. The learner invents addition,
rebinds permuted attribute axes, and transfers unchanged from sequence to
vector evidence. Once promoted, the operator reduces current structural
binding search from 15 to 3 while examples fall from four to zero. This is
actual reuse of an invented executable structure, not answer lookup.

The grammar is still deliberately small and bounded. It does not invent its
own grammar productions, learn scalar channels from sensory input, construct
nested or overlapping entity systems, or establish general intelligence.

ADR-0049 extends the same cost-phased and proof-gated contract with a bounded
variable-arity addition fold. The five-structure binary measurements here
remain the default-arity baseline.

## Compatibility and migration

This is an experimental semantic-version change for raw-relation identities.
Historical equality-only evidence can be replayed through the new learner, but
old semantic identities and any hypothetical v1 cache entries must be treated
as separate artifacts. No verifier, memory, ARC adapter, or accepted-answer
authority is widened.

## Validation

- Atomic projection candidates explain zero cases in the composed-key family.
- Addition is selected across independently permuted three-channel episodes.
- Changed cardinalities, values, identifiers, and layouts replay 9/9 groups.
- Sequence-to-vector transfer through unchanged adapters replays 9/9 groups.
- Two disjoint verifier receipts reduce binding work 60 to 12 and issue one
  promotion certificate; runtime search is 15 to 3 and examples 4 to 0.
- Two canonical artifacts survive complete process-memory loss only as
  evidence; two cohorts are re-invented and one module is freshly promoted.
- A content-free relational failure generates and completes an autonomous
  curriculum objective with two independent variations and restart retention.

## Rollback or kill conditions

Stop if task-local attribute indexes enter mechanism identity, a loaded artifact
restores status or authority, producer edges enter verifier replay, memory emits
an accepted answer, binary structures eclipse a cheaper exact projection, or
the bounded grammar is described as open-ended relation learning.
