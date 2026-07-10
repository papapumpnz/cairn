# CAIRN

CAIRN (Causal Analogical Induction over Relational Networks) is a planned
CPU-only, transformer-free general learning architecture for inducing executable
causal rewrite theories, retaining verified mechanisms, learning unknown worlds,
and composing skills from relational evidence.

ARC is one quarantined evaluation adapter. It is not the project's developmental
centre or sufficient evidence of general learning.

## Current implementation status

The repository has completed WP00, WP01, WP03, and WP04, and has working WP05
reference/bytecode execution plus an initial WP20 task-invention vertical slice.
Implemented today:

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
- a typed immutable Relational Evidence Graph with provenance, temporal/value
  indexes, deterministic joins, bounded causal paths, canonical replay, and
  measured performance above its initial CPU gates.
- an exact bounded RKL reference executor and typed register bytecode executor
  with deterministic traces, reusable workspaces, explicit fuel/progress caps,
  and zero mismatches over one million generated scalar programs;
- domain-neutral task-local definition invention from input/output
  demonstrations. The frozen initial GLL-1 suite autonomously synthesized five
  omitted executable concepts and replayed 5,835 varied predictions exactly,
  with leave-one-demonstration-out success, cost ablations, and zero accepted
  definitions on 100 controlled negative problems.
- proof-gated mechanism memory that promotes only definitions supported by
  multiple tasks and held-out rebinding. On the frozen initial GLL-2 curriculum,
  memory reduced median search work from 480 candidates to one verified proposal
  (99.79%), reduced median demonstrations from two to one, retained exact
  related/unrelated predictions, and fell back to fresh search when unrelated.
- generic composition of two independently promoted mechanisms by typed RKL
  substitution. The frozen initial GLL-3 suite composed magnitude with clamping,
  collection counting with clamping, and magnitude with categorical control,
  producing 510 exact held-out predictions with no coded composite operators.
- bounded interactive causal learning for fully observed deterministic worlds.
  Across 100 procedurally permuted unknown worlds, CAIRN inferred action laws,
  predicted 32,400 transitions exactly, inferred goals, constructed 100 shortest
  plans, saved 400 exploratory actions on repeated episodes, and transferred 100
  learned multi-step options into shifted layouts.

This establishes bounded task-level symbolic learning, not general learning.
Entity/event induction, full proof certificates, richer ontology invention,
continual memory, interactive world learning, planning, cross-domain runtime
integration, and knowledge learning remain targets; they are not claimed as
achieved. See the
[GLL-1 evidence report](docs/experiments/GLL1_OMITTED_CONCEPT_INVENTION.md).
The continual-learning result is documented in the
[GLL-2 evidence report](docs/experiments/GLL2_CONTINUAL_MECHANISM_MEMORY.md).
The initial composition result is in the
[GLL-3 evidence report](docs/experiments/GLL3_SKILL_COMPOSITION.md).
The initial unknown-world result is in the
[GLL-4 evidence report](docs/experiments/GLL4_UNKNOWN_WORLD_LEARNING.md).

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
bash scripts/wp03_acceptance.sh artifacts/wp03-acceptance
bash scripts/wp04_acceptance.sh artifacts/wp04-acceptance
```

Emit a runtime record with:

```sh
cargo run --locked -p cairn-check --bin cairn-attest -- \
  --out /tmp/cairn-attestation.json --path .
```

The governing contract is [AGENTS.md](AGENTS.md). The authored architecture
handoff is under [`docs/CAIRN_handoff`](docs/CAIRN_handoff/00_README.md).
