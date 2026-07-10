# G5 cross-domain cognition pass

Date: 2026-07-11

Status: frozen initial G5 suite pass

## Architecture

Three independent frontend crates lower their own domain observations to the
same typed invention contract:

- cairn-vector: point and displacement relations;
- cairn-gridworld: agent state and observed action effect;
- cairn-sequence: segment endpoints and endpoint shifts.

The adapter-neutral CognitionRuntime owns mechanism memory and invokes the same
task learner for every lowered problem. It has no dependency on any adapter.
The cross-domain test also scans runtime source for adapter/domain labels.

## Protocol

Two independent vector-scene tasks first induce the same executable coordinate
translation definition. Their proof-backed provisional module is installed in
one cognition runtime.

The unchanged runtime then receives one-demonstration tasks from vector,
gridworld, and sequence adapters. Every rebound definition is checked and
replayed against current evidence, then tested on a held-out domain observation.
Only after all three transfers does the independent verifier issue a
lineage-bound persistent-promotion token whose support geometry names all three
domains.

## Result

    adapters:                         3
    cognition runtime revisions:     1
    held-out domains exact:        3/3
    shared mechanism reuses:         3
    fresh candidate executions: 36,180
    memory proposals:                 3
    domain checks in runtime:         0
    persistent promotion:          pass

Command:

    cargo test --locked -p cairn-bench --test g5_cross_domain -- --nocapture

## Gate judgment

One unchanged runtime operates through three adapters, cognition contains no
domain branches, and one learned mechanism is measurably reused across vector,
interactive-world, and sequence domains. The frozen initial suite therefore
satisfies G5.

The adapters intentionally cover small exact relational subsets, not rich
graphics, obstacles, or arbitrary sequences. G6 remains open, and no general
intelligence claim is made.
