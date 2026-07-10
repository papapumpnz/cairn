# CAIRN

CAIRN (Causal Analogical Induction over Relational Networks) is a CPU-only,
transformer-free self-compressing causal theory learner. Its principal output is
an expanding, independently verified language of executable mechanisms; answers
and plans are consequences of applying those mechanisms to current evidence.

ARC is one quarantined evaluation adapter. It is not the project's developmental
centre or sufficient evidence of general learning.

ADR-0007 changes the implementation order: universal Mechanism IR, blind
invention, consolidation and support-aware memory, interactive causal learning,
and conditional skills precede further ARC-driven core development.

## Current implementation status

The repository has completed WP00, WP01, WP03, and WP04, and now has an initial
end-to-end static-learning vertical slice spanning WP05-WP07, WP09,
WP14-WP15, WP17-WP19, and WP22. These are functional starts, not full work
package acceptance.
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
- complete checked-theory execution for quantifiers, effects, schedules,
  conflicts, state, and rendering; typed observation inputs replay through both
  reference and bytecode expression paths with matching external semantics;
- content-addressed generic `Observation`, `Demonstration`, `Intervention`,
  `PredictionRequest`, and `LearningProblem` contracts with source provenance
  and explicit certainty;
- synthesis-free replay certificates binding theory, evidence, semantics
  versions, and canonical outputs; planted output/evidence/version corruptions
  fail closed;
- demand-driven relation-defined entity hypotheses and generic preserve,
  update, create, delete, split, and merge event hypotheses;
- bounded guard-conjunction induction, executable constructor invention,
  dependency-DAG schedule induction, and counterexample-guided whole-theory
  selection with final independent replay;
- a generic finite-field frontend whose initial gate invents a missing
  pointwise constructor and transfers it exactly to changed geometry, values,
  counts, and positions;
- a quarantined `adapters/cairn-arc` JSON adapter, single-task CLI, and bounded
  batch runner. The official ARC-AGI-1 v1.0.2 training corpus is provenance-
  pinned under `datasets/`; its evaluation split is intentionally absent. The
  first frozen official batch produced honest unsupported/resource-limit
  results and no exact solves. After adding generic, synthetic-tested field
  schemas, that same frozen batch reached 10/10 exact. During the next frozen
  ten-task development pass, generic field schemas reached 7/10 (17/20
  cumulative) before ARC expansion was frozen by ADR-0007. This is diagnostic
  adapter progress, not mechanism-library learning or broad ARC capability;
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
- minimum hidden-state-machine induction from visible action/outcome histories.
  In 100 further worlds, the one-state theory was rejected, a two-state latent
  mode was invented, 12,600 hidden-mode transitions were predicted exactly, and
  100 shortest plans were constructed through the inferred latent dynamics.
- explicit latent-successor ambiguity and active continuation selection. A
  five-experiment campaign exposed an unidentifiable terminal successor; CAIRN
  now preserves that uncertainty, requests a distinguishing action, and only
  permits deterministic reuse after the continuation resolves it.

This establishes bounded task-level symbolic learning, not general learning.
Under ADR-0007, the first self-compression prototype has also passed in one
narrow scalar family: empty-memory bounded success was 0/10; after two
independent inventions and proof compression it was 10/10, with candidate
executions falling from 440 to 10 and demonstrations from six to one. This is
bounded continual mechanism learning, not a full G0 or general-intelligence
pass. See
[G0_SELF_COMPRESSION_PROTOTYPE.md](docs/experiments/G0_SELF_COMPRESSION_PROTOTYPE.md).
The new entity/event, proof, component-induction, CEGIS, and ARC paths are
initial bounded implementations. Full generator coverage, certificate decoding
and proof DAGs, richer ontology invention, one shared orchestrating runtime,
vector-domain integration, ARC evaluation beyond the development training
split, and knowledge learning
remain targets. See the
[GLL-1 evidence report](docs/experiments/GLL1_OMITTED_CONCEPT_INVENTION.md).
The continual-learning result is documented in the
[GLL-2 evidence report](docs/experiments/GLL2_CONTINUAL_MECHANISM_MEMORY.md).
The initial composition result is in the
[GLL-3 evidence report](docs/experiments/GLL3_SKILL_COMPOSITION.md).
The initial unknown-world result is in the
[GLL-4 evidence report](docs/experiments/GLL4_UNKNOWN_WORLD_LEARNING.md).
The latent-state extension is documented in
[HIDDEN_STATE_WORLD_LEARNING.md](docs/experiments/HIDDEN_STATE_WORLD_LEARNING.md).
The failure, architecture correction, and five passing reruns are recorded in
[FIVE_LATENT_EXPERIMENTS.md](docs/experiments/FIVE_LATENT_EXPERIMENTS.md).
The new static-learning slice and its limitations are recorded in
[INITIAL_STATIC_LEARNING_PIPELINE.md](docs/experiments/INITIAL_STATIC_LEARNING_PIPELINE.md).
The first official-corpus run is documented in
[ARC_AGI_1_INITIAL_OFFICIAL_RUN.md](docs/experiments/ARC_AGI_1_INITIAL_OFFICIAL_RUN.md).
The frozen first-ten pass and untouched follow-on failure are documented in
[ARC_AGI_1_FROZEN_TEN_PASS.md](docs/experiments/ARC_AGI_1_FROZEN_TEN_PASS.md).

The normative target and claim boundary are in
[`08_GENERAL_LEARNER_REQUIREMENTS.md`](docs/CAIRN_handoff/08_GENERAL_LEARNER_REQUIREMENTS.md).

## Developer checks

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets --all-features
cargo test --locked --workspace --doc
cargo run --locked -p cairn-check -- check .
cargo run --locked -p cairn-arc --bin cairn-arc-batch -- \
  datasets/arc-agi-1-v1.0.2/training 10 1000 \
  artifacts/arc-agi-1-v1.0.2/initial-10.json
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
