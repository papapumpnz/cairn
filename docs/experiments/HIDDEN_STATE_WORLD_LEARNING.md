# Major evidence: latent-state world-model invention

Date: 2026-07-11  
Status: passed for deterministic two-mode finite transducers

## Claim

CAIRN can now invent a state variable that is absent from visible observations
when no memoryless transition theory can explain action outcomes.

The frozen worlds contain a hidden mode that reverses two movement effects. A
third action changes only the hidden mode and has no visible immediate effect.
Action identifiers and their meanings are freshly permuted in every world.
CAIRN receives only visible before/action/after histories.

For every world, the learner rejects the one-state model as inconsistent, finds
the smallest complete deterministic latent-state transducer, and plans over the
joint visible/latent belief state.

## Induction method

The learner searches latent-state counts in increasing order. For a proposed
count, it assigns episode states and constructs rules of the form:

```text
(latent state, action) -> (next latent state, visible-state delta)
```

Assignments must explain every observed transition, preserve episode chains,
reset to the declared initial latent state, and supply a complete rule table for
every latent/action pair. The first accepted theory is therefore minimum-state
under the bounded grammar. Search-node and latent-state caps are explicit typed
terminal outcomes.

Planning uses bounded breadth-first search over the learned joint state. A
separate truth-model search in the frozen test checks shortest-plan length.

## Frozen procedural results

| Measure | Result |
|---|---:|
| Procedurally permuted hidden worlds | 100 |
| One-state theories rejected | 100 / 100 |
| Two-state theories induced | 100 / 100 |
| Complete latent/action rules | 600 / 600 |
| Exact held-out transition predictions | 12,600 / 12,600 |
| Learned-model plans reaching goals | 100 / 100 |
| Plans matching independent shortest length | 100 / 100 |
| Initial induction search nodes | 1,200 |
| Repeated-episode induction search nodes | 0 |

The one-state ablation returns `NoTheory`. A deliberately tiny search budget
returns `SearchCapacity`, rather than being mislabeled as inexpressibility.

## Reproduction

```sh
cargo test --locked -p cairn-world --test hidden_state_gate -- --nocapture
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p cairn-check -- check .
```

Generic induction and latent planning are in
`crates/cairn-world/src/latent.rs`. Hidden truth rules exist only in the frozen
test `crates/cairn-world/tests/hidden_state_gate.rs`.

## Claim boundary and remaining risks

- The tested hidden process is deterministic and has exactly two modes. The
  algorithm is bounded for more states, but larger models are not yet measured.
- The initial latent state is known to reset to state zero at each episode.
- Visible observations are exact. Sensor noise, uncertain state estimates,
  stochastic effects, aliasing beyond the latent mode, and belief probabilities
  are not implemented.
- Effects remain additive over bounded integer state vectors; entity/relation
  perception is not part of this slice.
- This strengthens interactive causal learning but does not establish one
  cognition runtime across three external domains.
