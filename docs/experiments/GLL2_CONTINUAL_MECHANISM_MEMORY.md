# GLL-2 initial evidence: continual mechanism memory

Date: 2026-07-10  
Status: passed for the bounded five-family curriculum described below

## Claim

Past task experience now measurably improves later task learning. CAIRN promotes
verified executable definitions into mechanism memory, retrieves them by typed
signature, alpha-rebinds their parameters, and treats each retrieval only as a
current-task proposal. A memory module cannot emit a prediction or bypass exact
replay.

On the frozen curriculum, memory reduced median search work by 99.79%, from 480
fresh candidates to one memory proposal. Median demonstrations required for
exact held-out behavior fell from two to one. Exact accuracy did not fall on the
five related families or the unrelated control.

This is bounded continual symbolic learning. It is not yet a claim of general
learning: persistent CAS storage, full WP06 certificates, broad unrelated
corpora, skill composition, and interactive world learning remain open.

## Promotion and retrieval boundary

A module is promoted only when supplied with:

- at least two independently learned, structurally equivalent RKL definitions;
- at least one held-out rebinding result;
- explicit varied support dimensions;
- the semantic hashes of all supporting proposals.

The module identity includes its RKL definition, memory-semantics version,
sorted lineage, support counts, and variation dimensions. Active modules are
retrieved deterministically. Quarantined and deprecated modules are excluded.

For a new task, memory enumerates type-compatible parameter bijections and
rewrites the old free variables to current binders. Every rebound definition is
then type/totality checked and replayed by both the reference interpreter and
bytecode executor against the current demonstrations. Rejected proposals cannot
block ordinary invention; unrelated problems fall back to fresh search.

## Frozen curriculum

Ten independent source tasks—two for each mechanism—produce five promoted
modules:

1. signed magnitude;
2. changed-bound clamping;
3. relational categorical-symbol selection;
4. collection cardinality adjustment;
5. coordinate-tuple translation.

Source pairs vary parameter names and applicable numeric ranges, categorical
values, counts, positions, and coordinate-axis/tuple layouts. Five later tasks
change those surfaces again without exposing source answers.

## Results

| Measure | Memory disabled | Memory enabled |
|---|---:|---:|
| Median search work | 480 candidates | 1 proposal |
| Median demonstrations for exact held-out behavior | 2 | 1 |
| Related-family exactness | 5/5 | 5/5 |
| Unrelated-control exactness | exact | exact |
| Direct memory acceptance on unrelated control | n/a | 0 |

Search reduction is 99.79%, above the required 50% threshold. Every accepted
reuse passed current-task cross-implementation replay. The unrelated square-law
control rejected the superficially compatible magnitude module, ran fresh
search, and matched the memory-disabled held-out result. A one-task promotion
was rejected, and quarantining an active module removed it from retrieval.

## Reproduction

```sh
cargo test --locked -p cairn-memory --test gll2_gate -- --nocapture
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p cairn-check -- check .
```

The frozen curriculum is
`crates/cairn-memory/tests/gll2_gate.rs`. Generic promotion, typed rebinding,
status enforcement, fallback, and metrics are in
`crates/cairn-memory/src/lib.rs`.

## Claim boundary and remaining risks

- The measured curriculum is small and generated. Larger related and unrelated
  corpora may expose signature collisions or ambiguous rebindings.
- Support geometry is explicit, but the initial promotion runner consumes
  admitted proposal lineage rather than persistent WP06 replay certificates.
- The store is currently an in-process deterministic store; crash-safe CAS
  persistence and corruption replay are not implemented.
- Retrieval currently uses typed signature and parameter bijections, not REG
  relation-pattern indexing.
- Memory improves task learning but has not yet demonstrated composition of two
  independently learned modules. That is GLL-3.
