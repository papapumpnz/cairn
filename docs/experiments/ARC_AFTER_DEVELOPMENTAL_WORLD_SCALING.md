# ARC after developmental world-mechanism scaling

Date: 2026-07-13

Status: frozen no-regression pass; no new ARC accuracy

## Frozen boundary

General-core revision `64437f7fcbb56d57f03f9860e38653dbf4e6ba90` was committed
before opening the ARC adapter gate. It contains the five-structure world
curriculum, restart-safe evidence, sequential experiment synthesis, and
interactive-to-sequence/vector transfer. No core or adapter semantics changed
during either ARC run.

The new latent mechanism library is deliberately not translated into ARC
answers. ARC continues to receive only the previously frozen, independently
learned non-ARC structural bindings through its quarantined adapter.

## Commands

```sh
cargo test --locked -p cairn-arc --test structural_transfer_improvement \
  non_arc_partition_knowledge_adds_an_exact_arc40_solve -- --nocapture

cargo test --locked -p cairn-arc --test structural_transfer_improvement \
  complete_arc400_structural_transfer_ablation -- --ignored --nocapture
```

## Results

```text
ARC-1 training first 40:
  exact: 18/40
  incorrect: 0
  resource-limited: 22

ARC-1 complete training 400:
  exact: 36/400 (9.00%)
  incorrect: 0
  resource-limited: 364
  attempted: 36
  wall time: 930.62 seconds
  maximum candidates/task: 1,000
  maximum structural cost: 9
  generic observations: 3,020
  relational nodes: 386,563
  relational facts: 1,224,953
```

Raw per-case evidence:
`docs/experiments/evidence/arc1_training_400_wp34_v0.9.json`

Sanitized evidence SHA-256:
`108ac9b96d6de3801ec1bf478362fc862c238b82f8a5cae170ac8207e6a1d54e`

## Interpretation

The result is unchanged from the prior structural-transfer checkpoint. The new
world-learning capability improves unknown-world sample efficiency and transfers
to sequence/vector tasks, but its current finite state/action schema does not
match the entity, correspondence, constructor, and renderer failures dominating
the unsolved ARC tasks. Quarantine worked: unrelated learned knowledge caused
neither false accepts nor an artificial score increase.

The previously established non-ARC structural library still contributes four
solves over the 32/400 blank/distilled baseline, but this milestone itself adds
zero ARC solves. Therefore the 80% ARC target and general-intelligence goal
remain unmet. The next general work should learn relational correspondence and
rendering mechanisms from non-ARC graphics/manipulation curricula rather than
encode additional ARC transformations.
