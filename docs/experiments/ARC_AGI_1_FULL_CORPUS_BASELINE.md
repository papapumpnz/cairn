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

## Threshold and claim boundary

The requested 80% threshold is 320/400. Current exact coverage is 29/400, a gap
of 291 tasks. ARC-1 is not passed. The result demonstrates one transferable
constructor class and an honest complete-corpus measurement; it does not
demonstrate general intelligence.
