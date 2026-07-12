# Active latent-world learning

Date: 2026-07-12

Status: bounded active-causal-learning pass

## Question

Can the unchanged cognition runtime detect that visible state is insufficient,
select a discriminating intervention, recover a hidden-state transition theory,
plan through it, and transfer learned mode-conditioned laws into the same
relational mechanism constructor used by static evidence?

## Separation

The environment owns a two-state/six-rule truth table and each episode's hidden
initial mode. `LatentExperiment` exposes only the episode index, immutable
visible history, selected action, and the next visible transition. The learner
never receives hidden modes or truth rules.

The active controller invokes exact latent induction, reads only its unresolved
successor ambiguity, selects a canonical one-step distinguishing action,
validates the returned visible transition, appends it, and re-induces. The
benchmark separately checks the resulting theory against hidden truth.

## Commands

```sh
cargo test -p cairn-world --test active_latent -- --nocapture
cargo test -p cairn-bench --test g6_autonomous_internal_curriculum -- --nocapture
```

## Result

```text
initial visible episodes: 2
initial unresolved terminal successors: 1
learner-selected interventions: 1
uninformed action coverage: 3
latent states recovered: 2
transition rules recovered: 6
held-out latent prediction cases: 150/150
inferred-goal plan length: 2
relational transfer: static vector-like -> active latent world
ordinal correspondence ablation: wrong held-out graph
relational held-out constructions: 1/1 exact
candidate work: 72 -> 35
examples: 16 -> 1
proof units: 112 -> 16
```

An intervention cap of zero terminates explicitly. An environment response with
the wrong visible predecessor is rejected before it can modify evidence. Blank
persistent mechanism memory, changed relational fingerprints, ordinal transfer,
and expected-proof leakage remain rejected by the existing construction gates.

## Attribution

The active learner, not the benchmark, chooses the continuation action. Its
choice is derived from differences in predicted visible effects between
possible latent successors. One selected intervention replaces exhaustive
coverage of all three actions.

The held-out relational observation is constructed from two opposing
mode-conditioned effects of one learned action. These values are not authored
static transform labels. The previously verified relational policy rebinds them
without seeing modality, IDs, values, ordering, cardinality, or expected proof.

## Claim boundary

This is a small deterministic, fully observable-output hidden-state transducer.
It automates only one-step distinguishing continuations; ambiguities needing
longer conditional experiments fail explicitly. Entity/relation evidence is
still supplied structurally, the learned relational policy remains task-local,
and no persistent world-model consolidation is demonstrated. This does not
establish open-ended exploration, noisy belief-state inference, general
intelligence, or the 80% ARC target. ARC remains 36/400 and was not executed or
modified by this experiment.
