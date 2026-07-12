# Developmental world-mechanism scaling

Date: 2026-07-13

Status: bounded structural G2/G4/G5 checkpoint

## Question

Does persistent world learning extend beyond one latent mechanism, survive a
process restart, select multi-action experiments, and transfer across domains
while reducing future evidence?

## Frozen mechanisms

The generator varies five structural signatures independently of cognition:

| latent states | actions | visible dimensions |
|---:|---:|---:|
| 1 | 2 | 1 |
| 2 | 3 | 1 |
| 3 | 3 | 1 |
| 2 | 4 | 2 |
| 4 | 3 | 2 |

Each mechanism is learned in two source worlds with renamed actions and shifted
bounds. A content-free frontier scores only support gaps and selects the order.

## Commands

```sh
cargo test -p cairn-bench --test g2_structural_latent_curriculum -- --nocapture
cargo test -p cairn-memory --test latent_restart -- --nocapture
cargo test -p cairn-runtime --test sequential_experiment -- --nocapture
cargo test -p cairn-bench --test g5_persistent_world_cross_domain -- --nocapture
```

## Results

```text
curriculum order: [4, 2, 3, 1, 0]
source worlds: 10
persistent mechanisms: 5
changed held-out worlds: 5
fresh induction search nodes: 1,225
visible transitions, fresh -> memory: 42 -> 10
exact memory bindings: 5/5
wrong-schema accepts: 0
CAS evidence artifacts replayed: 2/2
promotion reconstructed after restart: yes
shortest discriminating experiment: 2 actions
one-action ablation: no discriminating plan
cross-domain targets: sequence and vector
memory predictions: 2
blank-memory predictions: 0
```

## Claim boundary

The suite demonstrates finite executable world-theory learning, structural
memory, restart-safe replay, multi-step diagnostic selection, and adapter-neutral
reuse. Observations and candidate theory graphs remain exact and structured;
there is no raw perception, stochastic belief, unrestricted action language, or
open-ended environmental generation. General intelligence and 80% ARC remain
unmet. The general core is frozen before ARC is run again.
