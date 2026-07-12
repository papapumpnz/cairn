# ADR-0028: Invent mappings from unordered relational evidence

Status: accepted experiment freeze

Date: 2026-07-13

Owners: WP16/WP18/WP34 integration

## Context

The v0.12 program synthesizer learns executable RKL structure but still receives
point-to-point provenance from v1. Static observations ordinarily provide
unordered source and output entity-point sets, not correspondence labels.
Enumerating explicit bijections scales factorially and risks making a hidden
matching template the real solver.

## Decision

Extend the generic definition inventor with `UnorderedMappingProblem`. Each
episode provides multiple input environments and one expected output set. A
candidate expression is executed once per input; acceptance compares the
resulting set with the expected set. No input-to-output pairing is supplied.

The ordinary typed expression grammar, cost order, canonicalization, CPU caps,
reference executor, optimized executor, and exact cross-implementation admission
remain unchanged. Only the observational equivalence criterion changes from an
ordered value vector to per-episode set equality.

Construction-program learning uses this mode independently for every output
coordinate and extent definition, then jointly replays complete entity renders.
Point provenance is reconstructed by program execution after learning.

## Validation

- input and expected order permutations produce the same definition;
- changed values and identities cannot affect program identity;
- ambiguity and candidate caps are explicit;
- reference and optimized outputs agree for every environment;
- independently generated cohorts synthesize the same program;
- no v1 trace or point pairing enters the learner.

## Compatibility

This is an additive invention interface. Existing ordered invention and
semantics remain unchanged. No persisted artifact is reinterpreted.

## Kill conditions

Stop if an expected output is paired to a particular input, iteration order
changes the learned definition, matching code uses modality/task labels, or
unordered admission bypasses either exact executor.
