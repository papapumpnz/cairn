# ADR-0026: Stop signed-axis collection correspondence as an ARC path

Status: accepted research stop

Date: 2026-07-13

Owners: WP34 integration

## Context

ADR-0025 froze a generic entity-collection correspondence experiment before ARC
access. It learns from non-ARC vector scenes, retains complete point provenance,
promotes from independent cohorts, reduces held-out work from 16 candidates/two
examples to two attempts/zero examples, transfers to fields, and accepts zero
wrong-family predictions.

At frozen commit `a597bfc`, blank and correspondence-only ARC first-40 both
score 17 exact, zero incorrect, and 23 limited. The pre-registered strict-gain
assertion fails.

## Decision

Stop expanding the global signed-axis/frame grammar for ARC accuracy. Do not run
full-400, reorder proposals, weaken the threshold, or add evaluation-derived
expressions. Retain the implementation as verified general collection
correspondence and provenance infrastructure.

The next mechanism must learn from non-ARC families where different entities
receive different operations or construct variable numbers of outputs according
to relational roles. Candidate research directions are learned bipartite role
matching, conditional per-entity transforms, non-bijective split/merge
provenance, and relation-path-conditioned placement. These are hypotheses, not
approved primitives.

## Alternatives rejected

- Running full-400 after the first gate failed violates pre-registration.
- Counting source success as ARC gain conflates learning with benchmark coverage.
- Adding constants or operations named from ARC cases violates quarantine.
- Deleting the mechanism discards verified cross-domain learning because one
  benchmark transfer failed.

## Compatibility and evidence

No interface or migration changes result from this stop. Evidence is
`docs/experiments/evidence/arc1_first40_entity_correspondence_ablation_wp34_v0.11.json`,
SHA-256
`ec2b42cbe363a77d3610feb77d648a56503e4d1de54c2f208addb87558cb14fe`.

## Reopen condition

Reopen only after a frozen non-ARC curriculum demonstrates a materially richer
relation-conditioned mechanism with exact verifier lineage, strict memory-off/on
source improvement, changed-structure transfer, and zero negative acceptance.
