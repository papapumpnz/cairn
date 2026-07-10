# ARC-AGI-1 frozen first-ten schema pass

Date: 2026-07-11

Status: frozen development batch passed; unseen follow-on failed

## Protocol

The selection, corpus, and scoring protocol are unchanged from
`ARC_AGI_1_INITIAL_OFFICIAL_RUN.md`:

- official ARC-AGI-1 v1.0.2 training split;
- first ten files in bytewise filename order;
- expected test outputs removed before learning;
- exact complete-grid scoring after prediction;
- 1,000-candidate pointwise fallback cap;
- no task-ID or filename access inside learning.

ARC adapter semantics are 1.1.0. The new field schemas were first required to
pass generated adapter-free tests, then admitted only when one schema replayed
every training demonstration exactly.

## Frozen first-ten result

| Task | Learned mechanism | Exact |
|---|---|---:|
| `007bbfb7` | tensor mask | 1/1 |
| `00d62c1b` | enclosed-region fill | 1/1 |
| `017c7c7b` | periodic rows with categorical rebinding | 1/1 |
| `025d127b` | boundary-anchored shear | 1/1 |
| `045e512c` | template completion and directional repetition | 1/1 |
| `0520fde7` | panel intersection | 1/1 |
| `05269061` | diagonal-phase periodic completion | 1/1 |
| `05f2a901` | attraction to persistent entity | 1/1 |
| `06df4c85` | separator-defined panel span completion | 1/1 |
| `08ed6ac7` | component-size ranking | 1/1 |

Aggregate: 10 selected, 10 parsed, 10 attempted, 10 exact, zero incorrect,
zero unsupported, zero capped, and zero execution failures.

## Untouched follow-on probe

After the first-ten result was achieved, the same frozen implementation and
budget were run once on filename-sorted tasks 11–20. It achieved 0/10: two
shape-change mechanisms were unsupported and eight tasks exhausted the
pointwise fallback cap. No code was changed in response during this experiment.

This sharp result is important. The first-ten pass validates the mechanisms it
contains but does not demonstrate broad ARC transfer. CAIRN is still an early
symbolic learner with a small schema frontier, not an ARC solver or general AI.

## Reproduction

```sh
cargo run --locked -p cairn-arc --bin cairn-arc-batch -- \
  datasets/arc-agi-1-v1.0.2/training 10 1000 \
  artifacts/arc-agi-1-v1.0.2/schema-first-10.json
```

The committed result artifacts are:

- `artifacts/arc-agi-1-v1.0.2/schema-first-10.json`
- `artifacts/arc-agi-1-v1.0.2/schema-first-20-probe.json`
