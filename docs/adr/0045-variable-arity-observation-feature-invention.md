# ADR-0045 — Variable-arity observation-feature invention

Status: accepted

Date: 2026-07-13

Owners: WP34 integration

## Context

ADR-0043 synthesized executable features but fixed every entity to exactly two
observations and checked only left/right reversal. That fixed interface prevents
the same invention path from describing higher-arity measurements, relation
tuples, or collections of anonymous evidence.

## Decision

Replace the fixed pair payload with a bounded vector of anonymous scalar
observations while retaining source aliases for the former pair type names.
All entities in one invention problem must have one arity between two and the
declared maximum. Feature parameters and variable seeds are generated from that
arity. The complete arity is part of the learned semantic identity.

For the current unordered-observation contract, the producer enumerates every
index permutation within an explicit factorial cap. Behavioral deduplication
and selected feature collections must be invariant under all permutations. The
independent verifier separately generates the permutations and replays the
complete learned mechanism for each one.

Reuse receipts and promotion certificates carry exact tested support geometry:
observation arity, entity-count range, and scalar range. The runtime uses arity
as a hard structural applicability check. Numeric and count ranges remain
reported evidence, not a claim of universal validity.

## Consequences

The same typed arithmetic grammar can now synthesize higher-arity invariant
features. Search and verification grow factorially with observation arity and
combinatorially with expression depth; observation, permutation, expression,
candidate, feature-set, and entity caps all terminate explicitly with
`Capacity`.

The semantic marker changes from observation-predicate v2 to v3. The evidence
archive changes to schema version 2 and semantics 3.0.0. Earlier artifacts fail
closed and require evidence reconstruction.

## Validation

A frozen three-observation family varies the position of every component across
all six permutations. No aggregate-of-three template is supplied. The learner
constructs a permutation-invariant typed feature, transfers it to changed
cardinalities, values, and independently named domains, and the verifier replays
all six permutations. The variable-arity mechanism also passes complete CAS
loss, re-invention, new receipt, re-promotion, and held-out reuse.

## Alternatives considered

Keeping a growing set of pairwise features was rejected because it makes tuple
structure frontend-authored. Treating observation order as silently semantic
was rejected because existing adapters deliberately provide unordered endpoint
evidence. A future interface may add an explicit ordered/symmetry-group contract
rather than assuming every observation vector is unordered.

## Rollback or kill conditions

Stop if arity is inferred from a domain label, a frontend supplies the selected
aggregate, a permutation is skipped silently, incompatible arities share an
identity, support ranges are presented as proven universality, or verifier
permutation logic imports the producer implementation.
