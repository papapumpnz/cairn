# CAIRN

CAIRN (Causal Analogical Induction over Relational Networks) is a CPU-only,
transformer-free self-compressing causal theory learner. Its principal output is
an expanding, independently verified language of executable mechanisms; answers
and plans are consequences of applying those mechanisms to current evidence.

The product target now also includes a normal technical-chat interface. The
initial slice compiles prompts to explicit intent, retains dialogue goals,
constructs a typed Python AST, executes examples, and only then realizes code
and explanation. A closed offline teacher-packet pipeline can bootstrap
language mappings and executable lessons, but the teacher is never in the
answer path. Both GPT-shaped Responses packets and Anthropic-shaped Messages
tool packets cross a network-free filesystem spool only; CAIRN contains no
provider client and answers solely from its accepted local knowledge. Local
verification, independent swarm peers, changed-context transfer, provenance,
and revocation gate shared knowledge.

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

- `cairn-chat`, a controlled CPU-only technical chat CLI whose initial Python
  response is built from a typed AST and executable examples;
- `cairn-distill`, a closed offline teacher-packet compiler with local replay,
  two-peer receipts, transfer gating, provenance, and revocation state;
- `cairn-teacher-spool`, a transport-incapable dual-protocol request/export and
  saved-response/import boundary for GPT and Anthropic teacher packets;

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
Under ADR-0007, the frozen initial G0 hidden-law suite now passes across five
unrelated mechanism families. Empty-memory bounded success was 0/10; after
independent invention, proof compression, rebinding, and lineage-gated
promotion it was 10/10. Candidate executions fell from 33,600 to 10 while
transfer varied names, ranges, symbols, collection cardinalities, positions,
and layouts. This is bounded continual mechanism learning, not general
intelligence. See
[G0_MULTIFAMILY_HIDDEN_LAW_PASS.md](docs/experiments/G0_MULTIFAMILY_HIDDEN_LAW_PASS.md).
The initial G1 suite also passes: all five persistent mechanisms have
task-distinct verified invocation and necessity records, covering 33,600 versus
10 candidate executions, 48 versus 10 demonstrations, and 52 versus 10 proof
nodes. See
[G1_ACTUAL_REUSE_LEDGER_PASS.md](docs/experiments/G1_ACTUAL_REUSE_LEDGER_PASS.md).
The frozen G2 equal-budget audit passes as well: median candidates are 480
versus one, demonstrations two versus one, and observed median CPU time
272.6 ms versus 0.553 ms. A 100-task unrelated audit remained 100% exact with
zero direct memory acceptance and zero negative transfer. See
[G2_CONTINUAL_IMPROVEMENT_PASS.md](docs/experiments/G2_CONTINUAL_IMPROVEMENT_PASS.md).
The regenerated G3 suite induces four conditional skills independently and
composes them on 100 never-demonstrated tasks with 100% exact completion and
400 explicit skill calls. All 400 skill/task ablations fail predictably. See
[G3_UNSEEN_SKILL_COMPOSITION_PASS.md](docs/experiments/G3_UNSEEN_SKILL_COMPOSITION_PASS.md).
The regenerated G4 suite maintains competing action-law theories, uses 62.1%
fewer exploratory actions than random, predicts 32,400/32,400 transitions, and
transfers after 100 layout shifts and 100 changed action laws. The latent suite
rejects memoryless explanations and recovers the required hidden two-state
model in 100/100 worlds. See
[G4_ACTIVE_CAUSAL_WORLD_PASS.md](docs/experiments/G4_ACTIVE_CAUSAL_WORLD_PASS.md).
The frozen G5 suite runs one unchanged CognitionRuntime through vector,
interactive-gridworld, and sequence adapters. One learned translation
mechanism transfers exactly across all three, reducing 36,180 fresh candidates
to three verified proposals, while a source audit finds no domain labels in the
runtime. See
[G5_CROSS_DOMAIN_COGNITION_PASS.md](docs/experiments/G5_CROSS_DOMAIN_COGNITION_PASS.md).
The frozen G6 curriculum now grows bounded composition capability from 1/3 to
3/3 as its library expands from three to five mechanisms. Marginal candidate
work falls from 24 to 20, examples from 10 to six, and the same-task observed
CPU median from 2.284 seconds to 0.916 milliseconds; all base behaviors remain
exact and the unrelated-law audit records no negative transfer. See
[G6_DEVELOPMENTAL_SCALING_PASS.md](docs/experiments/G6_DEVELOPMENTAL_SCALING_PASS.md).
The official ARC adapter now seals test answers, lowers every task to the same
provenance-bound `LearningProblem` protocol, and passes that envelope through
`CognitionRuntime` reference admission before invoking its quarantined legacy
backend. The frozen first-20 score remains 17/20 after 150 generic observations
become 23,114 evidence nodes and 70,550 exact relation facts; the three misses
remain resource-limited. This is a boundary-conformance
checkpoint, not shared-runtime ARC learning. See
[ARC_GENERIC_EXPERIENCE_BOUNDARY.md](docs/experiments/ARC_GENERIC_EXPERIENCE_BOUNDARY.md).
The first post-G6 invention expansion now turns exact finite observations into
generic relation graphs and synthesizes an omitted eligibility predicate from
positive and hard-negative entity bindings. The learned RKL predicate was
`1 == min(enclosed, has_marker)`, found after 2,086 candidates, and transferred
across unseen values and sizes without an `eligible` primitive. See
[RELATIONAL_CONTRAST_INVENTION.md](docs/experiments/RELATIONAL_CONTRAST_INVENTION.md).
Enclosure is no longer supplied to the follow-on experiment. From generic
bounding coordinates, two independent families invent the same four-clause
relation; the verifier promotes it, and a third renamed family reuses it from
one example. Search falls from 236 candidates to one proposal and five unseen
geometries remain exact. See
[INVENTED_ENCLOSURE_PROMOTION.md](docs/experiments/INVENTED_ENCLOSURE_PROMOTION.md).
The residual route now learns a constructor condition directly from unexplained
created cells. It invented `row == column`, was independently re-induced and
verifier-promoted, then rebound on a 7x7 field with a new paint value from one
example. Search fell 29 to one and all 49 held-out cells were exact. See
[RESIDUAL_CONSTRUCTOR_PROMOTION.md](docs/experiments/RESIDUAL_CONSTRUCTOR_PROMOTION.md).
The bounded-recursion route now learns a transition rule from traces and lifts
it into a reusable `BoundedFixpoint`, rather than storing an unrolled sequence.
After independent re-induction and verifier promotion, renamed one-example
tasks reduce search from 13,539 candidates to one and remain 4/4 exact. See
[ITERATIVE_MECHANISM_PROMOTION.md](docs/experiments/ITERATIVE_MECHANISM_PROMOTION.md).
The higher-order collection route learns an element definition from collections
of sizes 2, 4, and 5, lifts it into a quantified `ForEach` rule, and executes
7/7 unseen elements exactly. Whole checked theories now receive canonical
verifier tokens and persist in `CognitionRuntime` procedural memory; an altered
theory is rejected. See
[COLLECTION_MECHANISM_PROMOTION.md](docs/experiments/COLLECTION_MECHANISM_PROMOTION.md).
One `CognitionRuntime` now directly serves task-local induction, persistent
definition and whole-theory memory, action-conditioned world/goal learning,
exact planning, and conditional learned-skill composition. The official ARC
regression remains 17/20 after this integration. See
[UNIFIED_COGNITION_RUNTIME.md](docs/experiments/UNIFIED_COGNITION_RUNTIME.md).
The historical named field-schema catalog has moved physically out of the
general `cairn-field` crate and into the quarantined ARC adapter. Its 20 tests
and 17/20 official regression remain intact, but none of those results count as
mechanism learning. See
[ARC_LEGACY_SCHEMA_QUARANTINE.md](docs/experiments/ARC_LEGACY_SCHEMA_QUARANTINE.md).
The common runtime now also owns a proof-backed relational knowledge base with
source-bound typed claims, temporal validity, confidence, contradiction
retention, reversible entity-merge proposals, indexed retrieval, and executable
mechanism references. This accepts structured evidence only. See
[PROOF_BACKED_KNOWLEDGE.md](docs/experiments/PROOF_BACKED_KNOWLEDGE.md).
The consolidation boundary now compares typed immutable proof DAGs rather than
equal-length operation strings. A separate offline runtime automatically groups
structurally compatible episodes without family labels, anti-unifies typed
arguments, discards unrelated graph branches, exactly reconstructs every
source subgraph, and binds the resulting compression evidence into memory
promotion. In the first end-to-end rerun, two independent inventions compressed
from 12 proof units to seven and the promoted mechanism then reduced 440 fresh
candidates to ten verified proposals over ten held-out tasks. See
[TYPED_PROOF_CONSOLIDATION.md](docs/experiments/TYPED_PROOF_CONSOLIDATION.md).
That compression now produces a genuinely new executable RKL library item.
`MechanismCompiler` alpha-normalizes the learned structure, generalizes observed
type bounds, assigns a content-derived `invented_mechanism_*` identity, checks
the definition, and replays it against every source. A separately implemented
`CompressionVerifier` reconstructs proof DAGs and recalculates both costs;
memory rejects promotion without its opaque token. The compiled abstraction,
not either source definition, produced the same 10/10 held-out result. See
[EXECUTABLE_MECHANISM_COMPILATION.md](docs/experiments/EXECUTABLE_MECHANISM_COMPILATION.md).
The first complete pinned ARC-AGI-1 training-corpus audit corrects the earlier
prefix impression: the quarantined baseline is 27/400 exact (6.75%), not 85%.
A new generic remapped-field learner now invents output extents and affine
source-coordinate programs from replay evidence. It transfers across non-ARC
transpose and content-relative crop families and adds two exact ARC solves at
the fixed budget with no incorrect accepted theories, bringing the full result
to 29/400 (7.25%). The required 80% threshold is 320/400 and is not close. See
[ARC_AGI_1_FULL_CORPUS_BASELINE.md](docs/experiments/ARC_AGI_1_FULL_CORPUS_BASELINE.md).
Generic connected-frame hypotheses and contrast-driven entity selection now
transfer across changed component size, color, and layout, although they add no
fixed-budget ARC solve by themselves. A separate local-law learner observes a
bounded 8-neighborhood and synthesizes an RKL transition. It adds two exact
full-corpus solves (`north` and `min(north, 2)`), reaching 31/400 with zero
incorrect theories. An evidence routing gate preserves those gains while
reducing the full run to 360 seconds. This remains 7.75%, not an ARC pass.
Relational summaries now expose nearest visible values and axis occupancy only
when basic local evidence is insufficient. The learner transfers a long-range
carry-forward law outside ARC and adds one official exact theory,
`max(center, visible_north)`, reaching 32/400 with zero incorrect theories.
Lazy relational perception reduces the expanded run from 446 to 414 seconds,
but does not yet regain the earlier smaller-vocabulary cost envelope.
The local-law frontend now routes through the shared invention contract, so
verified memory can propose a learned law without owning the answer path. This
integration uncovered and removed factorial eager parameter rebinding in
memory retrieval. On a frozen non-ARC field family, persistent `output := west`
memory reduced four fresh candidate executions to one replayed proposal and 21
examples to three, then predicted 35/35 cells after geometry, value, and
position changes. This is bounded continual reuse, not a general-intelligence
claim and not an ARC score increase. See
[CONTINUAL_LOCAL_LAW_REUSE.md](docs/experiments/CONTINUAL_LOCAL_LAW_REUSE.md).
The first controlled chatbot and offline distillation slice now passes as well:
the exact prompt “Give me a basic Python function” produces a verified typed
addition function, an offline lesson survives teacher removal and two-peer
swarm transfer, and a clean non-ARC spatial lesson lets one pinned ARC task solve
under a one-candidate budget where blank memory fails. This is a tight-budget
transfer result, not a new 400-task score. See
[BASIC_CHAT_DISTILLATION.md](docs/experiments/BASIC_CHAT_DISTILLATION.md).
The complete 400-task distilled-memory ablation is now finished. Accuracy stays
32/400 with zero incorrect in both conditions; one official task genuinely
reuses the non-ARC law and reduces its search from two candidates to one. The
memory-on pass is roughly 38 seconds slower overall, so this is safe causal
transfer but also clear evidence that unconditional retrieval will not scale.
Memory semantics 1.1.0 now refutes directly contradicted variable bindings
before compilation. The complete contextual rerun keeps 32/400 and zero
incorrect, retains the useful one-candidate reuse, removes the known rejected
proposal, and completes in 390.85 seconds versus roughly 458 seconds before.
An offline GPT-authored, non-ARC curriculum subsequently induced 20 provisional
spatial mechanisms from 40 independently varied lessons. The frozen 400-task
ARC rerun stayed at 32/400 exact, 368 resource-limited, and zero incorrect; no
distilled mechanism entered a selected proof and elapsed time rose to 476.50
seconds. This is a successful distillation/admission test but a failed ARC
improvement experiment. It rules out broad guessed spatial priors as the next
route and motivates failure-core-driven curricula and stricter retrieval. See
[OFFLINE_DUAL_TEACHER_ARC_TRANSFER.md](docs/experiments/OFFLINE_DUAL_TEACHER_ARC_TRANSFER.md).
The next gap-driven slice now emits content-free learning failure cores and
offline request spools for both teacher formats. A first-40 analysis found one
causally insufficient local observation core; the generic field frontend now
detects such collisions and escalates to richer relational evidence before
search. That core disappears after the change, while ARC remains 17/40 under
the same 1,000-candidate budget. The remaining eight cores reject direct and
the tested two-/three-role scalar families, focusing the next curriculum on
hierarchical relations and composition rather than more guessed local laws.
Distillation semantics 1.2.0 now accepts bounded variable-cardinality collection
episodes as a separate typed channel. An offline GPT-authored non-benchmark
packet teaches three sizes; CAIRN invents the element law, lifts it into a
quantified `ForEach` theory, replays all 11 source elements, survives two peer
recompilations, and transfers to an unseen seven-element collection. This is
non-scalar learned structure, but not yet an ARC improvement or arbitrary
hierarchical mechanism induction.
Memory semantics 1.2.0 now carries those checked theories as provisional
procedural proposals. The general runtime replays every current collection
demonstration before prediction. A separate sequence/set adapter reuses the
offline theory on changed examples and an unseen eight-element input in one
proposal; blank memory fails and a different collection law is correctly
rejected. No ARC claim is made because ARC does not yet expose a valid entity
collection binding for this theory.
That binding now exists generically: the same whole theory replays through
sequence, vector, and finite-field frontends; contrast evidence can invent a
variable-output selector; and runtime pair search composes independently learned
map and filter theories when neither works alone. The field bridge reconstructs
all training cells before ARC use. The frozen first-20 ARC checkpoint remains 17
exact, 3 resource-limited, and 0 incorrect, so no ARC score gain is claimed. See
[PROCEDURAL_REUSE_COMPOSITION_FIELD_TRANSFER.md](docs/experiments/PROCEDURAL_REUSE_COMPOSITION_FIELD_TRANSFER.md).
The subsequent structural-transfer ablation produced the first full-corpus
accuracy gain from non-ARC learned knowledge. A controller/render binding learned
from generated dashboard fields improves frozen ARC-40 from 17 to 18 exact and
the complete ARC-1 training corpus from 32/400 to 33/400 with no lost solve or
incorrect output. This is 8.25%, not an ARC pass or evidence of general
intelligence. See
[STRUCTURAL_TRANSFER_ARC400_20260712.md](docs/experiments/STRUCTURAL_TRANSFER_ARC400_20260712.md).
The new entity/event, proof, component-induction, CEGIS, and ARC paths are
initial bounded implementations. Full generator coverage, certificate decoding
and durable proof-DAG artifact storage, arbitrary internal-subgraph compilation,
richer ontology invention, broad
cross-domain curricula, ARC evaluation beyond the development training split,
and knowledge frontends
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
