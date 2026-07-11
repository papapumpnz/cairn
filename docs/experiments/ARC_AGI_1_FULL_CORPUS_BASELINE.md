# ARC-AGI-1 complete training-corpus baseline

Date: 2026-07-11

Status: threshold failed; general-constructor gain measured

## Corrected baseline

The complete pinned ARC-AGI-1 v1.0.2 training corpus contains 400 tasks. Under
the frozen 1,000-candidate budget, adapter 1.5.0 produced:

| Status | Tasks |
|---|---:|
| Exact | 27 |
| Unsupported shape change | 133 |
| Resource limited | 240 |
| Incorrect accepted theory | 0 |

Thus complete-corpus accuracy was 6.75%. The earlier 17/20 result described a
favorable filename-sorted prefix and was never evidence of broad ARC ability.

## Generic remapped constructor

The new `cairn-field::RemappedFieldLearner` contains no ARC task or operation
type. It learns output extents and source-coordinate definitions from generic
finite observations. A bounded typed affine grammar proposes rule structure;
complete demonstration replay determines whether a correspondence is valid.
Content bounds are hypotheses derived from the modal-value complement and only
gain semantic force when the executable theory replays all evidence.

Independent synthetic families establish:

- transpose transfer from 2×3 and 3×4 to unseen 4×5 geometry;
- content-relative extraction across changed canvas, origin, object dimensions,
  repeated values, and held-out cardinality;
- fail-closed rejection of uniform evidence with no identifying contrast.

## Full-corpus ablation

At the same 1,000-candidate budget, adapter 1.6.0 produced:

| Status | Tasks |
|---|---:|
| Exact | 29 |
| Resource limited | 371 |
| Incorrect accepted theory | 0 |

The two new exact tasks are `2dee498d` and `d10ecb37`. No legacy schema was
added or changed. A higher-budget diagnostic also solved `1cf80156` with learned
content extents and `output + content_origin` coordinates, but it is not counted
in the fixed-budget score.

Raw reports:

- `artifacts/arc-agi-1-v1.0.2/full-400-baseline-v1.5.0.json`
- `artifacts/arc-agi-1-v1.0.2/full-400-affine-v1.6.0.json`
- `artifacts/arc-agi-1-v1.0.2/full-400-residual-v1.7.0.json`

## Residual-constructor ablation

The already generic residual constructor was then connected to the shared
static path. It proposes each observed value as a background hypothesis,
synthesizes a coordinate-conditioned Boolean emission predicate, requires
single-paint creation and complete demonstration replay, and otherwise falls
through to general search. The complete adapter 1.7.0 run remained 29/400 exact
with zero incorrect accepted theories. This negative result shows that the
remaining gap is not explained by coordinate-only residual emission; the next
required capability is entity-conditioned selection and multi-entity
construction.

## Threshold and claim boundary

The requested 80% threshold is 320/400. Current exact coverage is 29/400, a gap
of 291 tasks. ARC-1 is not passed. The result demonstrates one transferable
constructor class and an honest complete-corpus measurement; it does not
demonstrate general intelligence.

## Entity and local-law experiments

An executable contrast predicate learned to select one connected entity frame
across unseen sizes, colors, and layouts. It added zero ARC solves, establishing
that entity selection alone does not cover the remaining relational
construction gap.

A bounded local causal learner then synthesized output values from typed
8-neighborhood observations. It added two fixed-budget exact tasks:

- `25ff71a9`: `output := north`;
- `a79310a0`: `output := min(north, 2)`.

The result is 31/400 exact, 369 resource-limited, and zero incorrect accepted
theories under adapter 1.10.0. An evidence-derived routing gate retained the
same accuracy while completing in 360.36 seconds, materially below the initial
unguided local run.

Additional raw reports:

- `artifacts/arc-agi-1-v1.0.2/full-400-entity-v1.8.0.json`;
- `artifacts/arc-agi-1-v1.0.2/full-400-local-v1.9.0.json`;
- `artifacts/arc-agi-1-v1.0.2/full-400-local-routed-v1.10.0.json`.

The revised gap to 80% is 289 tasks. The next required general mechanism is
multi-entity relational construction, not additional benchmark-specific
operators.

## Long-range relational evidence

Nearest-visible values along each axis and axis occupancy were added as lazy,
generic observation summaries. A non-ARC carry-forward family validates exact
transfer across changed geometry, values, and positions. The complete corpus
then gained one exact theory:

- `d037b0a7`: `output := max(center, visible_north)`.

Accuracy is now 32/400 (8.0%) with zero incorrect theories. The implementation
sequence also preserves negative performance evidence:

| Semantics | Exact | Elapsed seconds | Change |
|---|---:|---:|---|
| 1.11.0 | 32 | 446.27 | eager relational summaries |
| 1.12.0 | 32 | 444.53 | contextual grammar split |
| 1.13.0 | 32 | 452.81 | stricter ray-support route |
| 1.14.0 | 32 | 414.16 | lazy relational perception |

Raw reports use matching `full-400-*-v1.*.json` names under
`artifacts/arc-agi-1-v1.0.2/`. The remaining gap to 80% is 288 tasks.
