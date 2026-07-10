# G4 active causal-world pass

Date: 2026-07-11

Status: frozen initial G4 suite pass

## Architecture

For each world, cairn-runtime begins with all 24 possible bindings between four
opaque actions and four transition effects. Each candidate is an executable
outcome theory. The separate curriculum runtime receives current outcome
partition counts, action cost, and irreversible risk, then selects a
discriminating intervention. Contradicted theories are eliminated after every
outcome.

Three observations resolve the four-action permutation; the fourth law follows
by causal elimination. The resolved mapping is lowered into the generic world
learner and tested throughout the state space.

The latent-world learner is evaluated separately on 100 worlds whose invisible
mode reverses movement laws. It rejects every memoryless model, induces the
minimum two-state variable, resolves successor ambiguity with continuation
evidence, and predicts/plans over joint visible and latent state.

## Frozen results

Active causal identification:

    worlds:                              100
    competing theories/world:            24
    active exploratory actions:         300
    random exploratory actions:         791
    exploratory-action reduction:     62.1%
    exact next-state predictions: 32,400/32,400
    shifted-layout transfers:        100/100
    changed-action-law relearning:   100/100

Hidden-variable recovery:

    hidden worlds:                       100
    one-state models rejected:       100/100
    minimum two-state models:        100/100
    exact latent transitions: 12,600/12,600
    shortest learned-model plans:    100/100

Commands:

    cargo test --locked -p cairn-world --test adr0007_g4_active -- --nocapture
    cargo test --locked -p cairn-world --test hidden_state_gate -- --nocapture
    cargo test --locked -p cairn-world --test five_experiments -- --nocapture

## Gate judgment

Prediction exceeds 95%, active exploration uses more than 50% fewer actions
than random, the hidden variable is required by a failed one-state ablation,
and transfer succeeds after both layout shifts and action-law changes.
Therefore the frozen initial suite satisfies G4.

The environments remain deterministic bounded vectors. Object-rich worlds,
stochastic effects, uncertain sensors, and broad goal discovery remain outside
this pass. G5-G6 remain open.
