# ADR-0011: Boundary-aware internal proof compression

Status: accepted

Date: 2026-07-12

Owners: WP23 integration

## Context

The initial consolidation runtime could compress only compatible whole proof
graphs. Repeated causal structure embedded inside different prefixes and
suffixes was therefore invisible, even when every occurrence had the same typed
internal computation and differed only in its external inputs and values.

## Decision

Under consolidator semantics 2.2.0, add bounded exhaustive discovery of
contiguous topological proof windows. A candidate is a connected induced
subgraph, is smaller than its host, and represents every incoming edge from
outside the window as an explicit typed boundary parameter. Typed opaque values
are anti-unified independently from boundary bindings. Candidates must occur in
the configured number of independent episodes, reconstruct every source
exactly, and reduce the declared proof-unit cost. Maximum savings wins, with the
semantic hash as deterministic tie-break.

`InternalCompressionVerifier` separately validates source topology, selected
windows, boundary edges and types, pattern instantiation, exact source replay,
costs, and proposal identity. A proposal remains untrusted and cannot enter
memory merely because the consolidator emitted it.

The initial search is deliberately limited to contiguous topological windows.
It is not an arbitrary graph-edit miner and does not compile the pattern into
RKL.

## Alternatives considered

### Compare all arbitrary node subsets

This could discover non-contiguous induced subgraphs, but introduces exponential
subset enumeration and more complex boundary semantics. It is deferred until
the exact reference interface demonstrates a measured need.

### Ignore external inputs

This simplifies structural hashing but creates helpers with hidden dependencies
and unverifiable transfer bindings. Every crossing dependency remains explicit.

### Let the consolidator certify its proposal

This avoids duplicate code but violates the independent-verifier boundary.
Compression identity and replay remain independently implemented claims.

## Consequences

Correctness: every selected occurrence and boundary is reconstructable from its
source proof graph; malformed or altered bindings fail closed.

Expressivity: internal fragments can be found in differently shaped hosts when
their nodes form a contiguous topological window.

CPU cost: discovery is bounded by nodes per graph and candidate-window count.
Exhausting either bound returns a typed capacity terminal.

Determinism: source order does not affect identity; selection uses fixed cost
and semantic-hash ordering.

Verifier impact: `cairn-verifier` contains an independent reconstruction and
identity implementation and imports no mining decisions.

Downstream impact: consumers may inspect and match proposals, but compilation,
promotion, and persistence require later versioned interfaces.

## Validation

Four independent episodes contain the same four-node causal fragment inside
different host prefixes and suffixes. Whole-graph compression rejects the
cohort. Internal mining compresses 28 proof units to 20 with one typed boundary
and two value parameters. A fifth changed host rebinds the fragment. Altered
operations, fixed values, boundaries, episodes, source content, and small work
caps are rejected or terminate explicitly.

Rollback trigger: verifier disagreement, hidden external dependency, source
order changing identity, or an unreproducible cost claim.

## Compatibility

Semantics version change: consolidator 2.1.0 to 2.2.0.

Migration: whole-graph proposals remain unchanged. Internal proposals are a new
public type and have no previous persisted schema.

## References

- ADR-0007 self-compressing mechanism learner
- ADR-0010 canonical proof graphs survive consolidator restart
- `docs/experiments/INTERNAL_SUBGRAPH_MECHANISM_DISCOVERY.md`
