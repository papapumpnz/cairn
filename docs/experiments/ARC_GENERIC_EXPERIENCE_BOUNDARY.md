# ARC generic experience boundary

Date: 2026-07-11

Status: adapter-boundary checkpoint; shared-solver migration incomplete

## Change

The ARC adapter now translates every sealed task into the same core protocol
used by modality-neutral cognition:

- training inputs and outcomes become provenance-bound `Observation` values;
- each static transformation becomes an implicit-transform intervention;
- training pairs become `Demonstration` values;
- test inputs become `PredictionRequest` values;
- expected test outputs are excluded before lowering;
- `CognitionRuntime` verifies observation identities and reference closure.

Each admitted finite-field observation is then decoded without benchmark
labels into a relational evidence graph. Cells are scalar nodes with row,
column, and value attributes; exact orthogonal adjacency and same-value
adjacency facts provide the substrate consumed by the existing demand-driven
entity generator. A non-ARC diagram fixture confirms that connected components
and value groups are proposed from these relations.

No ARC type or vocabulary was added to the runtime. The admission path accepts
only generic `LearningProblem` values.

## Frozen official result

    corpus:                    ARC-AGI-1 v1.0.2 training prefix
    selected tasks:           20
    generic observations:     150
    relational evidence nodes: 23,114
    exact relation facts:      70,550
    exact tasks:              17
    resource-limited tasks:   3
    incorrect tasks:          0
    leaked test outcomes:     0

The remaining tasks are `09629e4f`, `0962bcdd`, and `0e206a2e`. Increasing the
enumerative pointwise bound to one million candidates did not promptly produce
a result and was terminated; inspection shows relational panel selection and
object reconstruction behavior outside the pointwise learner's useful basis.
No ARC-specific schema was added in response.

Artifact:

    artifacts/arc-agi-1-v1.0.2/generic-envelope-first-20.json

## Claim boundary

This refactor proves input-boundary conformance and preserves the official
baseline. It does not prove that the shared mechanism runtime solved these ARC
tasks: after generic admission, the 17 successes still come from the
quarantined legacy finite-field schema backend. Named legacy schemas are not
eligible for mechanism promotion or general-learning evidence.

The next migration step is to decode admitted finite-field observations into
generic relation/entity evidence and make the learned `Mechanism` executor the
prediction backend. The three misses should be addressed only through
environment-neutral invention families with non-ARC transfer evidence.
