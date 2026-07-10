# CAIRN

CAIRN (Causal Analogical Induction over Relational Networks) is a planned
CPU-only, transformer-free general learning architecture for inducing executable
causal rewrite theories, retaining verified mechanisms, learning unknown worlds,
and composing skills from relational evidence.

ARC is one quarantined evaluation adapter. It is not the project's developmental
centre or sufficient evidence of general learning.

## Current implementation status

The repository has completed WP00, WP01, and WP04. Implemented today:

- a pinned Rust workspace;
- the repository operating law and architecture-layer manifest;
- `cairn-check`, which returns structured policy violations from Cargo
  metadata, lockfiles, manifests, build/configuration files, and production
  source;
- `cairn-attest`, which records the local CPU/runtime boundary and identifies
  prohibited loaded accelerator libraries;
- planted negative tests, schema checks, and CI wiring.
- canonical CBOR values with strict bounded decoding and stable BLAKE3 IDs;
- versioned artifact envelopes, explicit additive schema readers, and semantic
  identity independent of producer/time metadata;
- crash-safe append-only content-addressed storage with thread/process
  idempotency, corruption checks, and recovery.
- a modality-neutral bounded RKL with expression and whole-theory admission,
  exact range/collection checks, alpha-stable canonical hashes, schedule-cycle
  rejection, and 100,000 generated-term checks.

Entity/event induction, verification, ontology invention, continual memory,
interactive world learning, planning, cross-domain integration, and knowledge
learning remain targets; they are not claimed as achieved.

The normative target and claim boundary are in
[`08_GENERAL_LEARNER_REQUIREMENTS.md`](docs/CAIRN_handoff/08_GENERAL_LEARNER_REQUIREMENTS.md).

## Developer checks

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets --all-features
cargo test --locked --workspace --doc
cargo run --locked -p cairn-check -- check .
bash scripts/validate_schemas.sh
bash scripts/wp00_acceptance.sh artifacts/wp00-acceptance
bash scripts/wp04_acceptance.sh artifacts/wp04-acceptance
```

Emit a runtime record with:

```sh
cargo run --locked -p cairn-check --bin cairn-attest -- \
  --out /tmp/cairn-attestation.json --path .
```

The governing contract is [AGENTS.md](AGENTS.md). The authored architecture
handoff is under [`docs/CAIRN_handoff`](docs/CAIRN_handoff/00_README.md).
