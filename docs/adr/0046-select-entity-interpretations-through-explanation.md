# ADR-0046 — Select entity interpretations through executable explanation

Status: accepted

Date: 2026-07-13

Owners: WP34 integration

## Context

ADR-0045 learns features over variable-arity observations, but its primitive
entities are already segmented. This allows a frontend's grouping decision to
determine which mechanisms cognition can discover. The repository already has
bounded relation-defined component generation, but it was not composed with the
observation-feature learner.

## Decision

Add `RawEntityObservationEvidence`: opaque scalar primitives plus competing
anonymous binary relations. For every episode, the reference learner enumerates
one connected-component interpretation per relation role. It enumerates bounded
cross-episode binding sets and lowers each interpretation to the generic
variable-arity observation contract. Entity interpretation, typed feature
program, pair predicate, connected collection, and aggregate are selected only
when their complete composition exactly explains all episode outputs.

Relation names are task-local bindings and do not enter reusable mechanism
identity. The executable feature/predicate/aggregate identity therefore rebinds
to independently named relation roles in later domains. Equivalent exact entity
bindings remain `Ambiguous`; exhausted binding enumeration returns `Capacity`.

The independent verifier reconstructs selected components with a separate
disjoint-set implementation, regenerates canonical entity identities and bounded
surrogate scalar identities, then invokes independent feature, predicate, and
aggregate replay. Memory stores only the already proof-gated source-independent
mechanism. `CognitionRuntime` tries structurally compatible current relation
bindings and emits untrusted proposals; it cannot accept one.

Sequence and vector adapters expose the same raw entity-candidate evidence
contract. They do not select the relation role or resulting entity groups.

## Consequences

The learner can now discover one useful entity interpretation from competing
relation evidence rather than receiving the winning groups. Persistent reuse
removes repeated feature-program synthesis but still pays for current binding
attempts and independent replay.

This does not learn primitive relations from pixels, waveforms, or unrestricted
geometry. Connectedness remains a developer-supplied graph substrate, relation
candidates remain frontend evidence, components currently require uniform arity
within one problem, and canonical surrogate values are internal identities only.

## Validation

Frozen non-ARC episodes contain two anonymous relation roles and no supplied
entity groups. Four cross-episode bindings are evaluated. Only one binding
supports the complete executable mechanism. Independently named cohorts learn
the same reusable identity. Changed held-out domains replay 9/9 groups; replacing
the selected relation with the distractor fails independent verification.

The unchanged mechanism transfers through actual sequence and vector adapters
with zero core changes and 9/9 exact held-out groups. Proof-gated memory reduces
708 feature candidates/four source episodes to zero feature synthesis/zero new
episodes; two current relation bindings are still attempted. Blank memory emits
no proposal.

## Compatibility

This is an additive public interface. Observation-feature v3 and cohort schema
v2 remain unchanged because reusable identity already excludes task-local
binding evidence.

## Rollback or kill conditions

Stop if a domain label chooses an interpretation, selected groups enter the
input, relation bindings contaminate reusable identity, equivalent bindings are
silently tie-broken, memory accepts an answer, or verifier component logic
imports the producer traversal.
