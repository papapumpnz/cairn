# Entity correspondence and construction-provenance transfer

Date: 2026-07-13

Status: source-domain gates passed; ARC gate frozen and pending

## Frozen question

Can CAIRN infer one source-independent mapping for an entire entity collection,
reconstruct every output entity with point-level causal lineage, reuse it with
less search and fewer examples, and transfer it across frontends?

## Mechanism substrate

```text
output axis -> unique source axis
optional reversal per output axis
frame = preserve observation | tight collection
map every entity and point
record source entity and source point for every output point
```

No task ID, source identity, categorical value, modality, named rotation or
reflection, expected answer, or benchmark family enters mechanism identity.

## Commands and source results

```sh
cargo test --offline -p cairn-vector --test entity_correspondence -- --nocapture
cargo test --offline -p cairn-bench --test g5_entity_correspondence_transfer -- --nocapture
```

```text
mechanisms independently learned: tight reflection; preserved axis permutation
promotion: 2 disjoint cohorts / 4 experiences
fresh held-out work: 16 candidates / 2 examples
memory held-out work: 2 attempts / 0 new examples
held-out exact: 2/2
vector-to-field transfer: 1 exact
wrong-family verifier accepts: 0
point provenance: complete for every held-out source point
ambiguity/collision/cap/changed-truth/duplicate-evidence: rejected
```

## Frozen ARC gate

The first required comparison is blank versus correspondence-only memory over
the frozen first 40:

```sh
cargo test --locked -p cairn-arc --test structural_transfer_improvement \
  frozen_arc40_entity_correspondence_causal_ablation -- --ignored --nocapture
```

Strictly more exact tasks and zero incorrect are required. If the assertion
fails, preserve the result and stop before full-400. If it passes, run the
already frozen all-structural first-40 and full-400 tests without changing the
mechanism, order, budget, or threshold.

## Limits

The current mechanism is a bounded signed-axis bijection, not arbitrary affine,
topological, recursive, or relation-conditioned construction. Entities are
frontend-provided exact hypotheses. This checkpoint cannot support a general
intelligence claim.
