# Internal subgraph mechanism discovery

Date: 2026-07-12

Status: measured bounded reference result

## Question

Can CAIRN discover repeated causal structure inside differently shaped proof
hosts without a family label, hidden domain operation, or whole-graph match?

## Frozen family

Four source episodes contain a four-node typed fragment:

```text
select-role -> filter-relation -> map-effect -> render-effect
```

Each occurrence receives an external `EntitySet` boundary. Role and effect
values vary and must become shared parameters. Host prefixes and suffixes vary
in shape and operation. A fifth host changes both surrounding structure and
parameter values and is held out from mining.

Hard negatives alter the internal operation, a fixed relation value, a boundary
edge, episode identity, or source content. Candidate-window capacity is tested
independently.

## Commands

```sh
cargo test -p cairn-consolidator --test internal_subgraph -- --nocapture
cargo test -p cairn-verifier --test internal_compression_verifier -- --nocapture
```

## Result

```text
proposal: blake3:f6464e545db3f71f658badc05cdd45df56e7bf9ba5338651769d2dfd00466739
source_hosts: 4
held_out_hosts: 1
internal_nodes: 4
boundary_parameters: 1
value_parameters: 2
raw_proof_units: 28
compressed_proof_units: 20
whole_graph_compression: rejected as incompatible
held_out_rebindings: 1 exact occurrence
independent_source_replays: 4/4
negative_cases: all rejected
```

Reversing source order produces the same proposal identity. The independent
verifier reconstructs selected windows and boundary edges and recomputes the
identity and both costs.

## Attribution

The causal contribution is the boundary-aware internal miner: the pre-existing
whole-graph consolidator cannot cohort these hosts. No ARC data or adapter
concept appears in the family or implementation.

## Claim boundary

This passes one bounded proof-subgraph-compression test and supplies an exact
held-out rebinding. It does not establish arbitrary non-contiguous graph mining,
recursive mechanism invention, RKL compilation, persistent promotion, broad
cross-domain transfer, autonomous curriculum growth, general intelligence, or
an ARC accuracy improvement. The last measured frozen ARC result remains
36/400; this mechanism has not entered that answer path.
