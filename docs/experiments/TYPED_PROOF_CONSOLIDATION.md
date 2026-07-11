# Typed proof-DAG consolidation

Date: 2026-07-11

Status: bounded end-to-end consolidation checkpoint

## Change

The original consolidator compared equal-length vectors of operation strings.
That was useful scaffolding but did not establish proof-subgraph compression.
The replacement represents each episode as a topologically ordered, immutable
proof DAG with fixed operations, dependency edges, output types, and typed
content-addressed arguments.

A separate offline `ConsolidationRuntime` now:

- admits independently identified proof graphs under explicit node/episode caps;
- selects the complete ancestral subgraph of each requested output;
- ignores unrelated perception or search branches;
- groups structurally compatible episodes without task or family labels;
- anti-unifies varying typed arguments into shared parameters;
- calculates uncompressed and parameterized representation costs;
- reconstructs every source subgraph exactly before proposal emission.

`GraphCompressionProposal` has private fields and is created only by this replay
path. `cairn-verifier` now separately reimplements source traversal, pattern
instantiation, and cost calculation. `PromotionRunner::promote_compressed`
requires that verifier token and requires the source episode set to equal the
semantic hashes of the independently invented definitions. The module retains
the proposal identity, costs, replay count, and compiled-definition identity.

## End-to-end frozen rerun

The G0 scalar hidden-law experiment independently invented the same omitted
mechanism with renamed variables and different numeric ranges. No trace family
was supplied to consolidation.

| Measure | Result |
|---|---:|
| Episodes compared | 2 |
| Automatically discovered cohorts | 1 |
| Compression proposals | 1 |
| Uncompressed proof units | 12 |
| Compressed proof units | 7 |
| Exact source reconstructions | 2/2 |
| Empty-memory held-out success | 0/10 |
| Post-learning held-out success | 10/10 |
| Fresh candidate executions | 440 |
| Verified memory proposals | 10 |

Negative tests reject changed topology, changed output types, malformed DAGs,
duplicate episode identities, one-node fragments, and proposals without a
positive compression gain. A mixed four-episode test finds the one supported
three-episode cohort and leaves the unrelated structure uncompressed.

Commands:

    cargo test --locked -p cairn-consolidator --all-targets
    cargo test --locked -p cairn-memory --test adr0007_g0_gate -- --nocapture

## Claim boundary

This is the first mainline route where CAIRN automatically compares typed proof
structure across experience and makes that compression a prerequisite of
memory promotion. The supported whole-definition cohort now compiles into a new
RKL mechanism, but this is not autonomous open-ended invention. Current cohorts
still require the same operation/type/dependency shape; arbitrary internal
subgraph extraction and graph persistence remain open.
