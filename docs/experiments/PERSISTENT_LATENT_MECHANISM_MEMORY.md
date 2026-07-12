# Persistent latent-mechanism memory

Date: 2026-07-13

Status: bounded G2/G4 coupling pass

## Question

Can independently learned hidden-world laws become a source-free persistent
mechanism that reduces the evidence required in a changed world without
receiving hidden state or bypassing current replay?

## Frozen comparison

Two source worlds use the same unknown causal structure but change action IDs,
state bounds, and initial visible positions. Each source starts from ambiguous
visible histories and requires one learner-selected intervention. The resulting
complete theories are independently compiled, replayed, and recorded.

A third world changes all action IDs, bounds, and initial positions again. Its
initial visible episodes are identical for memory-on and memory-off conditions.

## Command

```sh
cargo test -p cairn-bench --test g2_latent_mechanism_memory -- --nocapture
```

## Result

```text
source worlds independently verified: 2
canonical mechanisms after source 1/2: 1 / 1
status after source 1: provisional
status after source 2: persistent
fresh third-world interventions: 1
memory third-world interventions: 0
third-world held-out predictions: 510/510 exact
blank-memory proposals: 0
duplicate evidence promotions: 0
```

The canonical mechanism hash is stable across source action identifiers,
coordinate bounds, support, episode content, and latent-state relabeling. The
runtime retrieves it only as a proposal, enumerates action-role bijections, and
requires exact replay of all current visible transitions. A separate verifier
reconstructs compilation and transfer.

## Attribution and limits

The reduction is caused by persistent mechanism memory: disabling memory forces
one active experiment on the same third-world evidence; enabling memory needs
none. Both paths retain identical exact held-out behavior.

This is one two-state, six-rule, one-dimensional deterministic family with
known episode resets and immediate visible effects. It is not evidence of noisy
belief learning, long conditional experimentation, broad world knowledge,
general intelligence, or 80% ARC. ARC remains quarantined and unchanged at
36/400.
