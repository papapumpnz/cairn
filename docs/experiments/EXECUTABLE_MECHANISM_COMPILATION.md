# Executable mechanism compilation

Date: 2026-07-11

Status: bounded executable library-invention checkpoint

## Result

The typed proof-DAG consolidator no longer stops at a compression record. On the
main scalar G0 route, two independently synthesized episode-local definitions
are compiled into a third, newly named executable RKL definition.

The compiler:

- requires compression lineage to equal the independent source identities;
- rejects incompatible executable structures and unsupported recursion;
- removes episode-local binder names through alpha normalization;
- widens observed integer and bounded-collection type support;
- derives an `invented_mechanism_<hash>` name from the compression;
- runs RKL definition type and totality checks;
- reconstructs the normalized body of every source definition;
- emits a semantic identity different from both source definitions.

The promoted module stores this compiled abstraction, not either representative
episode definition. It rebinds through ordinary memory proposal machinery, so
current-task evidence and dual execution remain mandatory.

## Independent acceptance

`CompressionVerifier` is implemented in `cairn-verifier`, not the consolidator
or compiler. From source graphs and the opaque proposal it independently:

1. validates source identities and topological order;
2. extracts each selected ancestral subgraph;
3. instantiates fixed and parameterized pattern arguments;
4. compares every reconstructed node exactly;
5. recalculates uncompressed and compressed representation costs;
6. issues a private token binding proposal, sources, costs, and replay count.

Memory promotion requires that token. Tests alter source content, identity, and
topology and confirm fail-closed rejection.

## Frozen measurements

| Measure | Result |
|---|---:|
| Independent source inventions | 2 |
| Newly compiled definitions | 1 |
| Exact structural source replays | 2/2 |
| Independent proof-graph replays | 2/2 |
| Proof compression | 12 → 7 units |
| Empty-memory held-out success | 0/10 |
| Compiled-memory held-out success | 10/10 |
| Candidate executions | 440 → 10 |

Commands:

    cargo test --locked -p cairn-verifier --all-targets
    cargo test --locked -p cairn-memory --test adr0007_g0_gate -- --nocapture

## Claim boundary

This mechanically demonstrates a learned, executable library item whose
identity and support are derived from multiple experiences and whose use lowers
future search. It remains a bounded same-shape whole-definition compiler. It
does not yet extract an arbitrary repeated internal proof subgraph, reconcile
different graph shapes, or constitute general intelligence.
