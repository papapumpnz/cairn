# Five latent-learning experiments and ambiguity fix

Date: 2026-07-11  
Status: five experiments pass after preserving and resolving latent ambiguity

## Campaign rule

Five distinct experiments were run sequentially. The initial campaign stopped
when Experiment 3 failed, preserving the counterexample. The architecture was
then changed under ADR-0004, and the same five experiments were rerun unchanged
except that Experiment 3 executes the continuation requested by the learner.

## Results

| Experiment | Result |
|---|---|
| 1. Minimum three-state hidden cycle | Pass: rejected 1–2 states, recovered 3 states and 6 rules, held-out history exact |
| 2. Minimum four-state hidden cycle | Pass: rejected 1–3 states, recovered 4 states and 8 rules, held-out history exact |
| 3. Multiple reset episodes | Initial fail; fixed pass: detected one ambiguous terminal successor, requested action 5, re-induced 2 states/6 rules, seven held-out steps exact |
| 4. Partial goal | Pass: inferred one required dimension and ignored one irrelevant dimension; shortest plan length 4 |
| 5. Irreconcilable histories | Pass: rejected contradictory reset histories for every model through 8 latent states |

Search work was 28 nodes for the three-state theory, 174 for the four-state
theory, and 12 after Experiment 3's distinguishing continuation.

## Failure and correction

Before the fix, Experiment 3 produced the correct visible value `23` but latent
state `0` instead of `1`. The relevant rule had appeared only at an episode
boundary, so its successor was not identifiable.

The corrected learner reports terminal-only rule successors, compares candidate
latent states' action-effect signatures, and requests the lowest-ID action with
different visible outcomes. After that continuation is observed, re-induction
removes the ambiguity and the original held-out history passes.

No task-family branch or expected hidden-state value was added to production
code.

## Reproduction

```sh
cargo test --locked -p cairn-world --test five_experiments -- \
  --nocapture --test-threads=1
```

The campaign is `crates/cairn-world/tests/five_experiments.rs`. Generic ambiguity
and continuation selection are in `crates/cairn-world/src/latent.rs`.
