# G6 developmental scaling pass

Date: 2026-07-11

Status: frozen initial G6 suite pass

## Protocol

The learner receives three independently induced unary mechanisms over the
generic RKL substrate: increment, doubling, and squaring. The frozen evaluation
requires mechanisms of increasing executable depth:

1. square after increment;
2. double the first composition;
3. increment the second composition.

No composite source template is supplied. A strict one-node/one-candidate
evaluation bound makes fresh synthesis impossible, so new capability must come
from the growing mechanism library. Two rebound source tasks are used to
consolidate each new composition before the next stage.

The `DevelopmentalGate` rejects solve-rate-only reports. It also requires
growth in available mechanisms and composition depth, falling marginal search,
CPU, examples and interventions, complete replay of prior behavior, and zero
negative transfer.

## Result

    bounded capability:                 1/3 -> 2/3 -> 3/3
    available mechanisms:                 3 -> 4 -> 5
    executable composition depth:         1 -> 2 -> 4
    base candidates per mechanism:                 24
    final composition candidates:                  20
    base examples per mechanism:                   10
    final composition examples:                     6
    observed fresh CPU median:          2,283,765,571 ns
    observed composition CPU median:          915,116 ns
    retained base behavior:                       3/3
    unrelated negative transfers:                   0
    primitive skill traces:                         2
    new interventions for four-skill composition:   0

The CPU figures are observed medians from nine same-process repetitions and
are reported as evidence, not as portable performance constants. Deterministic
candidate counts are the primary compute invariant.

Command:

    cargo test --locked -p cairn-bench --test g6_developmental_scaling -- --nocapture

## Gate judgment

Capability rises across a frozen curriculum while marginal candidate work,
observed CPU, demonstrations, and interventions fall. Earlier mechanisms are
replayed after consolidation and an unrelated law is rejected by memory and
solved through fresh search. The initial G6 suite therefore passes.

This is bounded synthetic developmental learning. It does not establish broad
general intelligence, open-ended ontology growth, natural perception, or
unrestricted knowledge acquisition. Those claim boundaries remain explicit.
