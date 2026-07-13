# ADR-0047 — Invent primitive relations from anonymous attributes

Status: accepted; equality-only grammar superseded by ADR-0048

Date: 2026-07-13

Owners: WP34 integration

## Context

ADR-0046 selects an entity interpretation through explanatory adequacy, but a
frontend still supplies every candidate binary relation. That leaves a decisive
ontology decision outside cognition and permits useful entities to remain
unreachable when no frontend relation names them.

## Decision

Add `RawRelationObservationTrace`: opaque primitives carrying one observation
and a bounded vector of anonymous scalar attributes, complete outputs, and no
binary relations or entity groups. The reference learner constructs a typed RKL
binary equality definition from the universal substrate, evaluates it over
each task-local attribute binding, materializes the resulting relation graphs,
and delegates joint entity/feature/predicate/aggregate selection to the
explanation-driven learner.

Attribute positions and generated relation roles are task-local bindings. They
do not enter reusable mechanism identity. The reusable identity contains the
canonical executable relation definition and the complete downstream mechanism.
Ambiguous exact bindings and relation-evaluation exhaustion fail explicitly.

The verifier does not consume producer-generated edges. It type-checks and
executes the learned RKL relation directly over raw attributes, regenerates a
verifier-local relation graph, and then invokes the independent disjoint-set
entity and mechanism replay path. Runtime memory retains proposal-only
authority: it must try current attribute bindings and cannot accept outputs.

Sequence and vector adapters expose unrelated anonymous primitives through the
same core trace. They do not construct a relation or select an entity.

## Consequences

One bounded family no longer requires supplied relations or entity boundaries.
The same relation/mechanism structure rebinds after attribute axes, values,
counts, identifiers, and frontend domains change. Prior experience removes
feature-program synthesis and examples, while current relation binding and
independent replay remain visible costs.

The initial relation grammar in this historical slice contains equality only. Anonymous attributes are
still exact frontend evidence; their extraction from pixels, geometry, audio,
video, or language is not learned. Connected components, clean demonstrations,
uniform three-primitive entities in the frozen family, and bounded exhaustive
binding remain important assumptions. This is not unrestricted relation
invention, raw perception, or general intelligence.

## Validation

Frozen non-ARC families expose three anonymous attributes per primitive: one
supports the explanatory grouping, one creates equal-sized mixed hard-negative
groups, and one is unique. The useful axis changes independently per episode.
No relation or entity membership is supplied. Two independent source cohorts
and sequence/vector adapters converge on one reusable identity. Changed
held-out episodes replay 9/9 groups exactly. Binding ablation, duplicate useful
axes, and relation-evaluation cap violations accept zero.

Proof-gated downstream memory reduces 708 feature candidates/four source
episodes to zero synthesis/zero new examples. Three current attribute bindings
remain explicit; a blank runtime emits no proposal.

## Compatibility

This was an additive experimental interface. ADR-0048 changes the complete
mechanism identity marker to v2 and adds a canonical evidence-only cohort
schema for complete relation programs. Historical v1 identities are not
silently upgraded.

## Rollback or kill conditions

Stop if adapters construct the winning relation, task-local attribute indexes
enter reusable identity, producer edges enter the verifier, an equivalent
binding is silently selected, memory accepts an output, or the equality
substrate is described as broad/open-ended relation invention.
