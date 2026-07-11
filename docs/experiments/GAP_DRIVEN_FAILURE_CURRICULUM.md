# Gap-driven failure curriculum

Status: redacted feedback loop and representation correction pass; ARC score
unchanged.

## Purpose

The earlier offline spatial curriculum compiled safely but was never used by an
ARC proof. This experiment replaces guessed teaching topics with structural
evidence from failed generic invention problems.

## Redaction contract

`FailureCoreExtractor` records:

- parameter and result type classes;
- demonstration, variation, and distinct-output counts;
- whether constant or direct projection explanations remain possible;
- whether any anonymous input alone determines the output;
- exact matches against bounded two- and three-role mechanism families;
- whether complete available inputs deterministically identify the output;
- the exhausted candidate or structural-cost bound.

It does not record task identifiers, variable names, domain labels, literal
values, observation payloads, examples, grids, or answers. Both offline teacher
formats receive the same redacted cores and the same teaching contract.

## Finding and correction

The first 40 frozen training tasks produced nine distinct local-invention
failure cores at 1,000 candidates. One core had 300 demonstrations and three
output classes but was non-deterministic under all 13 basic inputs. No amount of
formula enumeration can solve conflicting outcomes for identical inputs.

The field frontend previously escalated to long-range relational evidence only
when an output value was unavailable locally. It now also tests whether the
current observations are deterministic. Collisions cause evidence escalation
before synthesis; collisions that remain under relational evidence fail as
`NoCorrespondence` rather than wasting candidate search.

A synthetic non-benchmark family demonstrates the corrected behavior. Identical
local neighborhoods require different output values according to row-level
occupancy. The learner escalates, invents `output := row_occupied_count`, and
transfers exactly to changed geometry, positions, values, and counts.

After the correction, the non-deterministic core disappears. Eight deterministic
cores remain. None matches direct projection or the tested two-role arithmetic,
min/max, zero-selection, or three-role guarded families. The generated teaching
job therefore asks for cross-domain hierarchical relations, variable-arity
constructors, bounded recursion, and conditional role composition—not more
local scalar macros.

## Frozen ARC check

| Condition | Exact | Resource limited | Incorrect |
|---|---:|---:|---:|
| Prior first 40, 1,000 candidates | 17 | 23 | 0 |
| Determinism escalation, same budget | 17 | 23 | 0 |

This is a self-diagnosis and representation-correction result, not an ARC score
gain and not evidence that the remaining mechanisms have been learned.

## Offline artifacts

- failure cores SHA-256:
  `b1ba2e1d04eb5e6ef4411cd5e394bfb604e1051a81b0be2c40494064a4ba3f6d`;
- GPT-shaped request SHA-256:
  `a35b088a7755cb36d80a158517a60b9ef09b024a86339f90570060003fd36133`;
- Anthropic-shaped request SHA-256:
  `4cb516d7be27ea78045b7eadbe3d1608388faa93f77fbda50a28f83162b8a420`.

The requests are files only. No network call occurred.

The first schema extension and offline response experiment generated from this
direction is recorded in `VARIABLE_CARDINALITY_DISTILLATION.md`.

## Reproduction

```text
cargo test -p cairn-curriculum -p cairn-teacher-spool
cargo test -p cairn-field --test local_transition
cargo test -p cairn-arc --test gap_curriculum -- --nocapture
cargo run -p cairn-arc --bin cairn-arc-batch -- \
  datasets/arc-agi-1-v1.0.2/training 40 1000 /tmp/cairn-gap-first40.json
```
