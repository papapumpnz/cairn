# Iterative mechanism promotion

Date: 2026-07-11

Status: verifier-gated persistent mechanism pass

## Protocol

The learner observes independent state traces containing an initial integer, a
step parameter, and successive states until stability. It is not given a
remainder, modulo, repeat-until, or trace-replay operator.

Task learning first synthesizes the one-step transition from 13 observed
transitions. The higher-order invention route then binds that learned step as
the body of the universal bounded-fixpoint substrate, with an explicit maximum
step count and decreasing progress measure. The complete mechanism is replayed
through both RKL executors.

Two independently generated trace families vary initial values, parameters,
and iteration lengths. Their complete fixpoint definitions are structurally
equivalent and pass strict verifier promotion. A third problem changes variable
names and supplies only one final-state example before held-out prediction.

## Invented program

Conceptually:

    step(current, parameter) :=
        current - if current < parameter then 0 else parameter

    iterate(initial, parameter) :=
        bounded_fixpoint(current = initial, step, progress = current)

The exact stored artifact is executable RKL, not the prose above.

## Result

    source episodes:                  6 across two families
    first-family transitions:        13
    fresh transition candidates: 13,539
    learned step cost:                 8
    complete mechanism cost:          11
    persistent mechanisms:             1
    memory candidates:                  1
    examples without/with memory:     3/1
    held-out iteration cases:         4/4 exact
    unrolled trace stored:              no

Command:

    cargo test --locked -p cairn-theory --test iterative_invention -- --nocapture

## Claim boundary

This demonstrates learned bounded iteration with a synthesized step function,
explicit progress, independent re-induction, promotion, and reuse. It does not
yet invent arbitrary recursion topology, mutually recursive definitions,
higher-order collection maps, or unbounded algorithms. The maximum execution
bound remains developer-supplied as part of the trusted resource substrate.
