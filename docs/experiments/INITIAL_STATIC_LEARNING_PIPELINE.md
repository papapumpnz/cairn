# Initial generic static-learning pipeline

Date: 2026-07-11

Status: passing initial vertical slice; not work-package acceptance

## Result

CAIRN now has a connected bounded path for static learning:

```text
generic Observation
  -> relation-defined EntityHypothesis
  -> generic EventHypothesis
  -> guard / constructor / schedule induction
  -> checked RKL Theory candidates
  -> counterexample-guided replay
  -> synthesis-free verification certificate
  -> adapter output
```

ARC types exist only after the final boundary. The same finite-field learner can
be called without the ARC adapter.

## Experiments run

1. Whole-theory interpreter/bytecode agreement with typed observation inputs.
2. Certificate corruption: output, evidence, and semantics-version mutations.
3. Relation-defined components and attribute-defined entity grouping.
4. Generic update/create/delete/split event generation.
5. Two-predicate guard conjunction induction against hard negatives.
6. Executable constructor induction and cross-executor replay.
7. Dependency schedule induction plus ambiguous-write rejection.
8. CEGIS rejection of a cheaper wrong theory before verified acceptance.
9. Omitted finite-field constructor invention with changed held-out geometry,
   positions, counts, and values.
10. Quarantined ARC-format parsing and exact held-out adapter prediction.

All workspace tests and strict Clippy checks passed after these experiments.

## What was learned

The finite-field experiment did not select a named transformation. It searched
typed RKL expressions from a generic literal and variable basis, found an
executable arithmetic constructor consistent with seven demonstrated cells,
and applied the frozen definition to twelve unseen cells in a different shape.
The ARC-format fixture used the same learner and did not expose its expected
test output during invention.

This is bounded task-level symbolic learning. It is not evidence of autonomous
self-improvement, open-ended ontology discovery, general intelligence, or broad
ARC solving.

## Known limitations and next gates

- entity and event generators implement only their first generic hypothesis
  families and do not yet meet WP07/WP09 coverage or performance criteria;
- guards are conjunction-only; constructors are the existing bounded RKL
  expression search; symbol meanings and joint component solving are absent;
- whole-theory CEGIS currently ranks caller-supplied complete candidates rather
  than weaving them from all component streams;
- verifier certificates are in-memory types and critical effects still share
  the RKL scheduling implementation;
- finite-field prediction is pointwise and same-shape;
- no official ARC dataset is present in the workspace, so the ARC CLI has not
  yet produced a benchmark result;
- the required shared vector-graphics domain runtime remains absent.
