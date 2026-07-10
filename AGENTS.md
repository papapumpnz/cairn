# AGENTS.md — CAIRN repository operating contract

This file governs every coding, research, benchmark, integration, and swarm agent working in the CAIRN repository.

CAIRN means **Causal Analogical Induction over Relational Networks**. The system is a CPU-only, transformer-free architecture that infers executable causal rewrite theories from relational evidence, verifies them independently, and uses them to predict or plan.

The objective is not to produce an ARC-specific trick collection. The objective is to build and falsify a general learning architecture through task learning, continual learning, and interactive causal learning developed together from the beginning. ARC is one quarantined adapter, not the developmental centre.

## 1. Authority and precedence

Read the following before changing code:

1. `00_README.md`
2. `02_CAIRN_MASTER_DESIGN.md`
3. `03_CAIRN_INTERFACES.md`
4. `04_CAIRN_EXPERIMENT_LADDER.md`
5. `05_CAIRN_SWARM_PROTOCOL.md`
6. `06_CAIRN_AGENT_OPERATING_LAW.md`
7. `07_CAIRN_RISKS_AND_KILL_CRITERIA.md`
8. `08_GENERAL_LEARNER_REQUIREMENTS.md`
9. `HANDOFF_INDEX.md`
10. `work_packages/index.json`
11. The assigned `work_packages/WPxx.md`
12. Every direct dependency handoff and tagged public interface

When instructions conflict, use this precedence:

1. Evaluation quarantine, CPU-only, transformer-free, verifier, and integrity rules
2. `06_CAIRN_AGENT_OPERATING_LAW.md`
3. Stable schemas and public semantics in `03_CAIRN_INTERFACES.md`
4. Architecture decisions recorded in accepted ADRs
5. The assigned work package and dependency handoffs
6. This file
7. Local implementation notes

A nested `AGENTS.md` may add stricter local rules. It may not weaken this root contract.

## 2. Prime directive

Build general mechanisms and measure them honestly.

Every production mechanism must answer all of the following:

```text
What observation class triggers it?
What task-independent semantics does it implement?
What synthetic family exercises it?
Which non-ARC domain exercises it where applicable?
What negative cases constrain it?
Which failure-taxonomy bucket should change?
Which ablation establishes causal contribution?
Which learning loop becomes measurably better?
```

A mechanism without defensible answers remains experimental and cannot enter the verified solve path.

## 3. Non-negotiable architecture invariants

### 3.1 Compute boundary

The complete CAIRN runtime must use CPUs only.

Forbidden in production, tests, benchmarks, build scripts, optional features, and transitive runtime paths:

- CUDA
- ROCm or HIP
- OpenCL
- Metal compute
- Vulkan compute
- GPU kernels or GPU-specific libraries
- accelerator-backed inference services
- remote GPU execution
- hidden fallback to an accelerator

A dependency that can load an accelerator through an enabled default feature is prohibited unless that feature is disabled and the resulting dependency graph is verified.

The release process must emit a runtime attestation showing that no prohibited accelerator library is linked or loaded.

### 3.2 Transformer boundary

Forbidden:

- transformer architectures
- transformer runtimes
- transformer model weights
- transformer tokenizers coupled to a model runtime
- remote transformer APIs
- generated-answer paths using an external language or vision model

Compact non-transformer statistical components are permitted only for uncertain frontend evidence or search ordering. They cannot be the sole reasoner, cannot directly emit an accepted answer, and cannot replace exact replay.

### 3.3 Central inference object

The accepted semantic object is an executable **causal rewrite theory** containing explicit definitions, quantified rules, schedule, renderer, assumptions, and proof lineage.

Do not reduce the architecture to:

- one fixed action sequence
- one global image transform
- a growing catalogue of ARC operations
- a nearest-task lookup
- a memorized macro that emits an answer
- an opaque learned vector state used as proof

### 3.4 Event-first inference

Inference begins from observed correspondences, changes, preserved structure, created regions, and candidate causal events.

Program syntax enumeration may be used inside a typed hole or bounded secondary proposer. It must not become the global search strategy.

### 3.5 Joint perception and explanation

Entities are hypotheses justified by deterministic evidence or explanatory utility.

Do not assign semantic meaning from appearance alone. A detector may propose a region, motif, panel, contour, symbol, track, or object. A deterministic rule or candidate theory must justify its semantic role.

### 3.6 Dynamic ontology boundary

A task-local predicate, role, constructor, aggregate, coordinate expression, or recursive abbreviation is valid only when it has:

- a typed RKL definition
- explicit bounds or a totality proof
- exact reference semantics
- provenance
- canonical serialization
- a measured cost
- positive and negative evidence
- no task identifier, hidden native behavior, or source-specific answer content

Opaque invented primitives are prohibited.

### 3.7 Exactness and independent verification

Exact-domain claims require exact execution and independent replay.

The producer of a candidate cannot be the final authority accepting it. The independent verifier must use a separately implemented reference path for critical effects, bounds, renderer behavior, and answer reconstruction.

A heuristic score, learned ranker, solver model, or cached memory item cannot substitute for verification.

### 3.8 Memory boundary

Persisted mechanisms are proposal sources only.

Memory may propose:

- entity definitions
- rule sketches
- predicates
- constructors
- symbol-map families
- schedules
- compiled implementations of already selected definitions

Memory may not:

- emit an accepted answer directly
- bypass current-task binding
- bypass exact replay
- veto every non-memory alternative
- hide unsupported generalization axes

Task-local definitions remain temporary unless they pass the promotion gates in the architecture and experiment documents.

### 3.9 Swarm boundary

The coordinator schedules work and manages budgets. It must not contain hidden domain logic.

All substantive work moves through immutable, content-addressed artifacts. Jobs are idempotent. Arrival order must not change the semantic result in deterministic mode. Remote artifacts remain untrusted until locally validated or replayed.

### 3.10 ARC answer policy

Static solving is anytime and may return no more than two semantically distinct answer modes.

Two syntactically different theories that produce the same semantic prediction mode do not count as diverse attempts.

### 3.11 General-learning boundary

The three mandatory learning loops are:

```text
task learning
continual learning
interactive causal learning
```

They are co-equal release requirements. Interactive, memory, knowledge, and
non-ARC frontend work may not be deferred until a static ARC solver is mature.

General core crates use environment-neutral concepts such as `Observation`,
`EntityHypothesis`, `Relation`, `Event`, `Theory`, `Transition`, `Goal`,
`Intervention`, `Skill`, `MemoryModule`, `KnowledgeClaim`, and
`VerificationResult`.

Benchmark-shaped concepts such as ARC task, training pair, test grid, ARC
colour, ARC operation, corpus split, and answer formatting belong only in the
corresponding adapter or quarantined benchmark package.

Do not claim general learning until all gates in
`08_GENERAL_LEARNER_REQUIREMENTS.md` pass. An ARC score alone cannot satisfy a
general-learning gate.

## 4. Prohibited shortcuts and leakage

Never add any of the following:

- ARC task identifiers in production source
- full-grid hashes or filenames used as semantic branches
- branches on evaluation index or corpus position
- literal evaluation examples in prompts, tests, fixtures, comments, or generated tables
- memorized output grids
- source-task-only constants disguised by names or encoding
- evaluation-derived primitive design
- evaluation-derived search-policy tuning
- private-score hill climbing
- task-family labels supplied from hidden benchmark metadata
- a learned module that directly returns an answer
- a silent fallback from exact to approximate semantics
- a weakened threshold after observing a result
- deletion or rewriting of a negative result
- a headline result without raw per-case evidence and verifier status

The ordinary development binary must not be able to enumerate quarantined evaluation roots.

Evaluation access must be capability-separated and available only through a sanctioned gate owned by the benchmark package.

## 5. Work-package ownership

Each agent owns one work package unless the integration agent explicitly assigns a cross-package change.

Before implementation:

1. Read the assigned package.
2. Confirm its direct dependencies are complete at a tagged interface milestone.
3. Read dependency handoffs and public contracts.
4. Identify every file and crate inside the package's owned scope.
5. Record any required interface question before modifying dependency-owned code.

Agents must not casually edit another package's scope.

When a dependency contract must change:

1. Stop dependent implementation at the affected boundary.
2. Write an ADR using `templates/ARCHITECTURE_DECISION_RECORD.md`.
3. Describe compatibility, migration, verifier, schema, and benchmark effects.
4. Obtain integration ownership of the change.
5. Update contracts before relying on the new behavior.

Temporary mocks are allowed behind public traits. A package cannot be declared complete until acceptance tests run against the real dependencies.

## 6. Dependency and parallel-work rules

Use `HANDOFF_INDEX.md` and `work_packages/index.json` as the dependency DAG.

A downstream package starts only when:

- every direct dependency has a reviewed handoff
- required public symbols are tagged
- schema and semantics versions are declared
- dependency acceptance tests needed by the downstream package pass

Parallel agents must not modify the same owned files.

Cross-cutting files such as workspace manifests, global schemas, CI, protocol versions, and shared identifiers are integration-owned unless the assigned package explicitly owns them.

Do not resolve merge conflicts by discarding another agent's semantic changes. Escalate conflicting contracts to the integration agent.

## 7. Required agent workflow

### 7.1 Initial response

Before coding, report:

```text
work_package:
understood_scope:
owned_paths:
public_symbols_to_implement:
direct_dependencies:
dependency_interface_tags:
reference_path_plan:
planned_tests:
planned_acceptance_measurement:
architecture_or_interface_questions_or_none:
```

### 7.2 Implementation sequence

Use this order unless the work package explicitly requires another:

1. Define or confirm public types and invariants.
2. Implement the simplest exact reference path.
3. Add unit and property tests for the reference path.
4. Add typed errors and terminal states.
5. Add canonical artifact serialization and provenance.
6. Implement the optimized path.
7. Differential-test optimized behavior against the reference path.
8. Add resource accounting, cancellation, and capacity handling.
9. Run package acceptance measurements.
10. Run relevant workspace integration checks.
11. Produce the work-package handoff and evidence pack.

Optimization before a trustworthy reference implementation is prohibited unless the package is itself only an optimization layer over an existing reference implementation.

### 7.3 Completion response

At handoff, report:

```text
work_package:
commit:
interface_tag:
scope_completed:
files_changed:
public_apis:
commands_run:
acceptance_status:
benchmark_machine:
benchmark_results:
evidence_pack:
artifact_hashes:
known_defects:
known_limits:
next_dependency:
downstream_start_condition:
```

A code commit without reproducible evidence is incomplete.

## 8. Git and change discipline

Use one branch per work package or integration task.

Recommended branch form:

```text
agent/WPxx-short-description
```

Commit messages should begin with the work-package identifier:

```text
WP07: add demand-scoped entity denotation cache
```

Rules:

- keep commits logically reviewable
- do not rewrite shared history
- do not combine unrelated cleanup with semantic changes
- do not commit generated benchmark corpora unless the package owns and documents them
- do not commit build outputs, local CAS contents, raw private datasets, or machine-specific paths
- include migration notes for every schema or semantics version change
- tag stable public interfaces before downstream use

## 9. Rust implementation standards

### 9.1 Toolchain and workspace

- Use the pinned stable Rust toolchain established by WP00.
- Workspace code must pass formatting, Clippy with warnings denied, tests, policy checks, and schema validation.
- Avoid unnecessary dependencies. Record why each new dependency is required, its feature set, license, CPU behavior, and determinism implications.
- Disable unused default features.
- Do not introduce a dependency with a prohibited accelerator or transformer path.

### 9.2 Public APIs

Every public type, trait, function, and error must document:

- semantic purpose
- invariants
- totality or resource bounds
- deterministic behavior
- failure modes
- artifact or version implications
- thread-safety expectations where relevant

Public semantics cannot change without an ADR and semantics-version review.

### 9.3 Error handling

Use typed errors.

Library code must not use `unwrap`, `expect`, uncontrolled `panic!`, process exit, or silent defaulting on data-dependent paths.

Panics are acceptable only for unreachable internal invariants already established by construction, and must include a debug assertion or documented proof obligation. Prefer returning a typed internal-invariant error at trust boundaries.

Every solver component must distinguish at least:

```text
unsatisfiable
not_found_within_current_sketch
ontology_missing
capacity_cap
wall_timeout
cancelled
invalid_artifact
verification_failure
internal_error
```

Do not collapse these into `failed`, `unknown`, or `inexpressible`.

### 9.4 Determinism

Semantic output must not depend on:

- hash-map iteration order
- thread scheduling
- process count
- worker arrival order
- wall-clock timing
- memory address
- filesystem enumeration order
- locale
- random seed not recorded in the job

Requirements:

- sort before canonical serialization
- use canonical tie-breaking at the CAIRN layer
- record seeds, budgets, semantics versions, and configuration hashes
- use fixed-work quanta for deterministic scheduling where applicable
- make checkpoint/resume byte-identical for the same semantic state
- test one-worker and multi-worker semantic equivalence

Hash maps may be used internally for performance, but their iteration order cannot escape into semantic identity or selected results.

### 9.5 Exact arithmetic and floating point

Observation semantics, bindings, bounds, proof identities, canonical hashes, verifier decisions, and exact reasoning must not depend on non-canonical floating-point behavior.

Floating point may be used for uncertain frontend weights, heuristic ordering, telemetry, or statistical reports when:

- it cannot make a valid exact candidate unreachable except through an explicit resource cap
- ordering has a deterministic final tie-break
- values are excluded from universal equality claims unless formally canonicalized
- platform-sensitive behavior is tested and documented

Use checked arithmetic at external and artifact boundaries. Overflow must become a typed error or be ruled out by an explicit bound proof.

### 9.6 Unsafe code

Unsafe code is exceptional.

It must be:

- isolated in an explicitly permitted low-level module
- accompanied by a `SAFETY` argument for each unsafe block
- differentially tested against safe reference code
- fuzzed where input bytes or indexes are involved
- justified by a measured performance result
- excluded from verifier trust when a safe reference path is feasible

No unsafe code may be added merely to satisfy an unmeasured performance assumption.

### 9.7 Concurrency and cancellation

Long-running loops must support bounded cancellation checks and explicit work budgets.

Concurrency code must:

- preserve idempotency
- avoid hidden shared mutable semantic state
- journal accepted artifacts before acknowledging completion
- recover from worker loss without semantic corruption
- report CPU time and peak memory by role
- terminate with a typed capacity or cancellation state

## 10. RKL and theory rules

Every RKL construct or derived definition requires:

- typed signature
- reference semantics
- finite collection and iteration bounds
- totality or fuel proof
- canonical form
- semantic hash behavior
- cost-model contribution
- property tests
- provenance
- verifier support

Universal rewrites require algebraic proof.

Task-local observational equality must carry task scope and must never be exported as a universal rewrite.

A recursive definition requires:

- a progress measure
- a finite bound
- a reference execution proof
- a failure mode when the bound is exceeded

A candidate theory is invalid unless every event cover lowers to an exact replay plan and every emitted output cell has source or bounded-constructor provenance.

## 11. Artifact and schema requirements

Persistent artifacts are immutable and content addressed.

Unless a normative schema states otherwise, every persisted artifact must identify:

- protocol version
- schema version
- semantics version
- artifact kind
- canonical content hash
- parent artifact IDs
- producer identity and build revision
- configuration hash
- seed when randomness is used
- resource budget
- creation metadata that is non-semantic
- optional signature

Persistent IDs are BLAKE3 hashes of canonical CBOR. Human names, task filenames, and local paths are metadata only and cannot be semantic keys.

Artifact rules:

- identical semantic values must produce identical canonical bytes and IDs
- writes are append-only and idempotent
- readers validate kind, version, length, hash, and bounds before use
- unknown required fields or unsupported semantics versions fail closed
- remote artifacts are untrusted until validation and required replay
- corruption diagnostics identify the failed boundary precisely
- schema extensions are versioned and backward-compatibility behavior is explicit

Never mutate an accepted artifact in place. Produce a new artifact with parent lineage.

## 12. Verifier trust boundary

The independent verifier must not import synthesis, ranking, memory retrieval, or search-policy logic.

It may consume only:

- canonical observations
- the candidate theory and definitions
- declared bindings
- replay certificates
- exact semantics versions
- declared resource and bound proofs

The verifier must:

- reconstruct semantic IDs
- validate artifact lineage
- re-execute critical semantics through a separate reference implementation
- validate totality and resource bounds
- validate renderer conflicts and layer order
- validate every training replay
- reconstruct test predictions
- reject mismatched hashes, hidden dependencies, unsupported versions, and unexplained cells

A solved count may include only independently verified answers.

## 13. Testing standard

Every package must select the applicable test classes below.

### 13.1 Unit tests

Test local invariants, boundary values, invalid inputs, and typed terminal states.

### 13.2 Property tests

Required for canonicalization, serialization, geometry, bounds, type checking, executor semantics, e-graph rewrites, solver encodings, and proof reconstruction.

Record seeds for failures and preserve minimized counterexamples.

### 13.3 Differential tests

Every optimized path must be compared against a simple reference implementation.

Examples:

- SIMD grid operations versus scalar operations
- bytecode executor versus RKL interpreter
- incremental solver encoding versus exhaustive small-domain enumeration
- verifier replay versus reference executor
- indexed REG queries versus naive scans

Zero unexplained differential mismatch is acceptable.

### 13.4 Negative and corruption tests

Test that the system rejects:

- malformed canonical encodings
- invalid bounds
- unsupported semantics versions
- hash mismatches
- altered parents
- missing provenance
- cyclic derivations
- renderer conflicts
- unverified memory modules
- remote artifact corruption
- accelerator and transformer dependencies
- task identifiers and evaluation leakage

### 13.5 Synthetic-family tests

Synthetic families must vary the dimensions the mechanism claims to generalize across.

For each family, define:

- train dimensions
- held-out dimensions
- held-out structural combinations
- hard negatives
- distractors
- expected failure taxonomy
- random seed policy

The generator and solver must live in separate modules. Prefer separate authors or agents. Generator internals cannot be available to the solver runtime.

### 13.6 Integration tests

Integration tests must exercise public contracts through artifact boundaries, not private implementation shortcuts.

At minimum, relevant packages must test:

- canonical write/read/replay
- checkpoint/resume
- one-worker versus multi-worker equivalence
- crash recovery
- independent verification
- resource-cap termination
- unsupported-version rejection

### 13.7 Fuzzing

Fuzz parsers, canonical decoders, artifact validators, grid dimensions, RKL AST decoding, proof certificates, framed transport, and any unsafe indexing boundary.

Persist minimized crashing or divergent inputs as regression fixtures when they contain no quarantined data.

## 14. Standard validation commands

Use repository-provided scripts once WP00 establishes them. Do not replace a canonical script with an easier local subset.

Expected baseline checks are equivalent to:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc
cargo run --locked -p cairn-check -- check .
```

For package work, also run focused tests and benchmarks, for example:

```sh
cargo test --locked -p <owned-crate>
cargo test --locked -p <owned-crate> --test <integration-test>
cargo bench --locked -p <owned-crate> --bench <benchmark>
```

When the repository defines `just`, `make`, or scripts as the canonical entry point, use those commands and record the exact invocation.

Never report a check as passing unless it was run against the final commit or an identified equivalent tree.

## 15. Benchmark and experiment hygiene

No benchmark claim is valid without:

- frozen config
- exact code revision
- machine and OS record
- CPU model, physical-core count, logical-core count, RAM, and compiler version
- raw per-case result records
- verifier status
- resource budget
- comparable baseline
- repeat or uncertainty policy
- attribution or an explicit statement that attribution is unknown

Development suites must be frozen before implementing the measured mechanism.

Do not tune on a gated evaluation result.

A benchmark result must preserve:

- solved and unsolved cases
- terminal state for every case
- CPU time
- wall time
- peak memory
- candidate counts
- agent-role CPU breakdown
- falsification counts
- verifier failures
- configuration and artifact hashes

Negative results are first-class artifacts. Do not delete, rewrite, or hide them.

When a pre-registered kill criterion fires:

1. stop that path
2. preserve the evidence
3. classify the failure
4. write an ADR or research decision
5. do not bury the result beneath further implementation

## 16. Heuristic and learning policy

Heuristics may order candidates, schedule agents, estimate promise, or allocate budget.

A heuristic may not silently make a semantically valid candidate unreachable unless the owning package defines an explicit hard cap and reports that cap as the terminal cause.

Search-policy learning must be ablated against a deterministic baseline.

A learned component must report:

- input features
- output role
- training partition
- update rule
- deterministic seed policy
- effect on reachability
- memory and CPU cost
- exact downstream verifier boundary

No learned component may claim correctness by itself.

## 17. Performance engineering policy

Optimize measured bottlenecks only.

Required order:

1. establish correctness with reference code
2. profile under a frozen representative workload
3. identify the dominant cost
4. state the optimization hypothesis
5. implement the smallest optimization
6. differential-test it
7. remeasure on the same workload
8. report accuracy, latency, memory, and complexity effects

Prefer:

- packed grids and bitplanes
- structure-of-arrays indexes
- bounded arenas
- denotation and value-vector hashing
- compiled hot RKL definitions
- copy-on-write canvases
- incremental solving
- semantic deduplication
- fixed-work scheduling

Do not trade exact semantics for speed without a separate approximate layer whose uncertainty is explicit and whose output remains subject to exact reasoning and verification.

## 18. Security and input handling

Treat corpus files, network artifacts, plugins, and frontend observations as untrusted input.

Required protections:

- size and dimension limits before allocation
- depth and node limits for recursive formats
- checked offsets and lengths
- schema and semantics-version checks
- bounded decompression
- content-hash validation
- cancellation and time budgets
- no executable code deserialized from artifacts
- no dynamic plugin loading inside the verifier trust boundary without an approved interface and sandbox

Do not log private dataset contents or complete answer grids unless the sanctioned evidence format explicitly permits it.

## 19. Documentation requirements

Code and architecture documentation must distinguish:

- implemented behavior
- proposed behavior
- measured result
- engineering target
- benchmark objective
- unresolved hypothesis

Do not phrase a target as an achievement.

Every substantial algorithm module should document:

- problem statement
- data model
- algorithm
- complexity
- bounds
- deterministic tie-breaking
- failure taxonomy
- verifier relationship
- benchmarks
- known limitations

Update affected architecture, interface, schema, and handoff documents in the same change when semantics change.

## 20. Architecture decision records

An ADR is mandatory for changes to:

- RKL semantics
- artifact hashing or canonical serialization
- public schema compatibility
- verifier trust boundary
- memory acceptance boundary
- evaluation quarantine
- CPU-only or transformer-free rules
- theory score ordering
- deterministic tie-breaking
- output-attempt semantics
- dependency ownership
- a pre-registered experiment or kill threshold

An ADR must include context, decision, alternatives, compatibility impact, migration, validation, and rollback or kill conditions.

## 21. Work-package handoff evidence

Complete `templates/WORK_PACKAGE_HANDOFF.md`.

The evidence pack must contain or reference:

- final commit and interface tag
- scope completed
- changed files
- public APIs and invariants
- exact commands run
- test summaries
- raw benchmark outputs
- machine record
- acceptance-criterion matrix
- verifier results where applicable
- configuration and artifact hashes
- known defects and limitations
- downstream start condition

Use these acceptance states:

```text
PASS
PASS_WITH_DECLARED_LIMITS
FAIL
BLOCKED_BY_DEPENDENCY
KILL_CRITERION_TRIGGERED
```

`PASS_WITH_DECLARED_LIMITS` cannot conceal a failed mandatory threshold. Use `FAIL` when a required criterion is not met.

## 22. Review checklist

Before requesting review, verify all applicable items.

### Architecture

- The change implements a general mechanism.
- It preserves CPU-only and transformer-free operation.
- It does not introduce a fixed-DSL closure disguised as convenience code.
- It preserves exact current-task replay and independent verification.
- It does not let memory or a learned component emit accepted answers.

### Scope

- Changes remain inside owned scope.
- Dependency interface changes have an accepted ADR.
- Public semantics and versions are updated consistently.
- No parallel agent owns the same modified files.

### Correctness

- A reference path exists.
- Optimized paths match the reference path.
- Errors and capacity states are typed.
- Determinism is tested.
- Bounds and totality are explicit.
- Artifact lineage and provenance are complete.

### Testing

- Unit, property, differential, negative, and integration tests are present where applicable.
- Seeds and minimized counterexamples are retained.
- No evaluation data appears in fixtures.
- Corruption paths fail closed.

### Measurement

- Acceptance criteria were measured rather than inferred.
- Baselines use comparable budgets.
- Raw per-case results are preserved.
- Negative results are included.
- Claims distinguish targets from achievements.

### Delivery

- Formatting, Clippy, tests, policy checks, and schemas pass.
- The handoff file is complete.
- Artifact hashes are recorded.
- Known limitations are explicit.
- The downstream start condition is unambiguous.

## 23. Stop and escalate conditions

Stop implementation and escalate when:

- a requested change violates CPU-only or transformer-free constraints
- an interface change affects another active package
- evaluation data appears in development scope
- the verifier would need to trust synthesis or memory code
- exact semantics cannot be specified
- an optimization changes results nondeterministically
- a persistent definition lacks an executable RKL body
- a mandatory benchmark threshold fails
- a kill criterion is triggered
- a dependency handoff is inconsistent with its tagged interface
- an artifact or schema migration cannot preserve replay

Do not work around these conditions silently.

## 24. Definition of done

A work package is done only when:

- its owned deliverables exist
- public contracts are documented and versioned
- reference and optimized behavior are tested as applicable
- all mandatory acceptance criteria are measured
- exact artifacts and raw evidence are preserved
- required answers pass independent verification
- policy, formatting, lint, test, and schema checks pass
- the handoff is complete
- downstream agents have a stable interface tag and explicit start condition

Passing compilation alone is not completion.

## 25. Final governing principle

CAIRN must become more capable by discovering and validating reusable causal structure, not by accumulating hidden task answers.

When forced to choose between a higher score with weak attribution and a lower score with exact evidence, preserve the exact evidence. It is the only foundation on which the architecture can be improved reliably.
