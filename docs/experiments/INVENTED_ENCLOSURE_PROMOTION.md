# Invented enclosure promotion

Date: 2026-07-11

Status: verifier-gated persistent mechanism pass

## Protocol

The learner receives pairs of entities described only by generic minimum and
maximum coordinates. The substrate supplies ordinary typed variables, equality,
less-than, and conjunction. It does not supply containment, enclosure, spatial
roles, a grid type, or a relation template.

Positive pairs are strictly enclosed. Hard negatives independently violate the
top, bottom, left, or right boundary, including equality cases. The inventor
enumerates 48 ordinary cross-entity comparison atoms and searches conjunctions
that separate all evidence.

Two source families use different variable names and coordinate ranges. Their
admitted executable definitions are anti-unified by structural equivalence and
passed to the strict promotion verifier. A third family changes names, ranges,
positions, and layout values and supplies only one positive example before
held-out prediction.

## Invented mechanism

    outer_min_row    < inner_min_row
    inner_max_row    < outer_max_row
    outer_min_column < inner_min_column
    inner_max_column < outer_max_column

The four atoms are joined by executable RKL conjunctions and replayed through
both the reference and bytecode executors.

## Result

    generic comparison atoms:       48
    conjunction candidates:         236
    selected conjuncts:              4
    structural cost:                12
    independent source families:     2
    persistent mechanisms:           1
    examples without/with memory:  9/1
    candidates without/with memory: 236/1
    held-out renamed geometries:    5/5 exact

Command:

    cargo test --locked -p cairn-theory --test relation_promotion -- --nocapture

## Claim boundary

This is a learned and reused symbolic relation under the repository's strict
mechanical criteria: it was absent as a named primitive, synthesized from the
minimal comparison substrate, independently re-invented, verifier-promoted,
rebound, and necessary for the one-shot result.

It is not open-ended spatial intelligence. Bounding-box features are currently
computed by the frontend; overlap, nesting, irregular enclosure, occlusion, and
three-dimensional relations remain untested. Persistent support must retain
those unknown boundaries.
