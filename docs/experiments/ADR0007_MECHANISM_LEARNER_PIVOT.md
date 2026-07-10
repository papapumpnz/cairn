# ADR-0007 mechanism-learner pivot checkpoint

Date: 2026-07-11

Status: architecture and first refactor slice implemented; decisive learning
gates not yet passed

## Why the ARC pass stopped

The frozen official ARC-AGI-1 training prefix reached 17/20 exact after adding
seven generic finite-field schema classes with synthetic tests. Three cases
remained resource-limited. Further case-driven schema expansion was stopped
because it would strengthen a catalogue of field transformations without
testing whether CAIRN autonomously invents, reuses, and composes mechanisms.

The 17/20 checkpoint is preserved as adapter regression evidence. It is not a
general-learning result and none of those field schemas count as persistent
learned mechanisms under ADR-0007.

## Refactor delivered

- `cairn-mechanism`: universal Mechanism IR, support geometry, derivation
  certificate, lifecycle states, and strict persistent-admission evidence;
- `cairn-consolidator`: online-solver-independent proof-trace anti-unification
  with explicit compression accounting;
- `cairn-runtime`: a modality-neutral theory version space that chooses a
  discriminating intervention and eliminates contradicted theories;
- `cairn-curriculum`: deterministic information/transfer/underuse selection
  penalized by action cost and irreversible risk;
- `cairn-verifier`: separate promotion boundary re-exporting independent replay
  and enforcing learning-evidence admission.

Persistent admission now fails unless the target was absent from the named
vocabulary, source code remained unchanged, multiple experiences support it,
initial/post capability crosses the G0 bounds, the abstraction compresses,
held-out cases verify, later search and examples fall, false acceptance does not
rise, and actual plus necessary reuse are observed.

## Frozen ARC result

Command:

```sh
cargo run --locked -p cairn-arc --bin cairn-arc-batch -- \
  datasets/arc-agi-1-v1.0.2/training 20 1000 \
  artifacts/arc-agi-1-v1.0.2/adr0007-pivot-first-20.json
```

Expected summary under adapter semantics 1.2.0:

```text
selected: 20
exact: 17
resource limited: 3
incorrect: 0
unsupported: 0
```

## Honest status

The code establishes enforceable representation and authority boundaries. It
does not yet show self-directed mechanism acquisition. G0 next requires an
independently generated hidden-law curriculum, an intentionally impoverished
starting vocabulary, and a no-source-change pre/post run. G1-G6 remain open.
