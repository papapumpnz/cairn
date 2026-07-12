# ADR-0027: Synthesize generic construction programs from verified traces

Status: accepted experiment freeze

Date: 2026-07-13

Owners: WP18/WP19/WP23/WP34 integration

## Context

The v0.11 entity learner chooses among developer-authored signed-axis/frame
configurations. That is bounded induction and reuse, but the learned mechanism's
structure already exists in its Rust type. Adding more variants would grow a
catalogue rather than a self-expanding executable language.

## Decision

Introduce an additive `EntityConstructionProgram` whose semantics are generic
bounded map-each execution over entities and points. Output extents and each
output coordinate are ordinary learned RKL definitions over only:

- source point coordinates;
- source observation extents;
- collection coordinate minima and maxima;
- small universal integer literals and arithmetic.

The program contains no reflection, rotation, crop, frame, axis-map, ARC, grid,
or modality enum. It maps every source entity and point, preserves a generic
entity attribute, and emits complete source entity/point provenance. These map
and bounded-iteration operations are developer-supplied substrate; their
coordinate and extent rule structures are learner-synthesized.

Initially, independently verified v1 executions may provide training traces.
They are teacher evidence only. A program counts as acquired only after v1 is
removed and the independent verifier replays the synthesized definitions on
changed held-out scenes. Future routes must derive traces directly from
observation/output correspondence version spaces.

## Trust boundary

The synthesizer returns an untrusted program. The independent verifier checks
definition types and totality, executes through the RKL reference path rather
than the inventor, reconstructs every output and provenance edge, and compares
held-out truth. Persistent promotion remains a separate future step after two
independent program-synthesis cohorts.

## Frozen gates

1. Synthesize a tight horizontal-reflection program without a named transform.
2. Synthesize a preserved axis-permutation program through the same API.
3. Remove the v1 mechanism and replay changed IDs, values, extents, positions,
   entity counts, and point cardinalities exactly.
4. Show source program text contains only generic variables/arithmetic.
5. Reject missing/duplicated provenance, ambiguous examples, caps, changed
   truth, and unsupported dimensions.
6. Transfer the synthesized program from vector traces to a categorical field.
7. Ablating learned definitions must remove the capability.

No ARC access is permitted in this slice.

## Compatibility

The v1 correspondence mechanism remains readable but is deprecated as a
persistent learning claim. New program semantics use a distinct canonical
marker. No existing artifact is reinterpreted.

## Kill conditions

Stop if program identity contains v1 configuration fields, named transforms,
source IDs/values, expected outputs, or modality; if verifier imports the
synthesizer; or if held-out execution still requires the v1 mechanism.
