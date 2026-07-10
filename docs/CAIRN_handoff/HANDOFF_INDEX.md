# CAIRN work-package handoff index

Each work package has one primary owner. ADR-0007 supersedes the historical
wave ordering below: trusted universal semantics are followed by blind
invention, consolidation, interactive learning, skills, cross-domain adapters,
and only then ARC adapters. The online runtime, consolidator, verifier, and
curriculum runtime are separate authority boundaries.

## Dependency lanes

```text
Lane A: governance and substrate
WP00 -> WP01
WP00 -> WP02
WP01 -> WP03
WP01 -> WP04
WP03 + WP04 -> WP05
WP01 + WP03 + WP04 + WP05 -> WP06

Lane B: generic evidence and spatial frontend
WP03 + WP04 -> WP07
WP02 + WP03 + WP07 -> WP08
WP03 + WP07 -> WP09 -> WP10
WP10 + WP03 -> WP11

Lane C: synthesis
WP04 + WP05 -> WP12
WP04 + WP01 -> WP13
WP11 + WP12 + WP13 -> WP14
WP09 + WP12 + WP13 -> WP15
WP11 + WP14 + WP15 -> WP16
WP10 + WP13 + WP15 -> WP17
WP14 + WP15 + WP16 + WP17 -> WP18
WP06 + WP18 -> WP19
WP12 + WP13 + WP19 -> WP20
WP18 + WP19 + WP20 -> WP21

Lane D: continual learning and swarm
WP01 + WP04 + WP06 -> WP23
WP01 + WP06 + WP18 -> WP24
WP24 -> WP25

Lane F: interactive intelligence
WP03 + WP04 -> WP29
WP03 + WP07 + WP29 -> WP26
WP05 + WP13 + WP26 -> WP27
WP05 + WP27 -> WP28

Lane G: modalities and knowledge
WP29 -> WP30
WP29 + WP26 -> WP31
WP03 + WP04 + WP06 + WP23 + WP29 -> WP32

Lane H: adapters, measurement, and integration
WP06 + WP23 + WP28 + WP29 -> WP33
WP18 + WP19 + WP20 + WP21 + WP23 + WP24 + WP28 + WP29 + WP32 + WP33 -> WP34
WP34 + WP02 + WP06 + WP29 -> WP22
```

## Work package catalog

| WP | Owner focus | Primary deliverable |
|---|---|---|
| WP00 | Governance | Repository, LAW, CPU/transformer checker, CI |
| WP01 | Storage | Canonical CBOR, BLAKE3 CAS, artifact envelopes |
| WP02 | Spatial frontend systems | Packed finite fields and CPU geometry kernels |
| WP03 | Data systems | Relational Evidence Graph and query indexes |
| WP04 | Languages | RKL AST, types, bounds, canonical form |
| WP05 | Runtime | Reference and optimized RKL executors |
| WP06 | Formal integrity | Proof DAGs, replay certificates, independent verifier |
| WP07 | Generic perception | Relation-defined demand-driven entity algebra |
| WP08 | Spatial frontend | Structure evidence lowered to generic relations |
| WP09 | Generic abduction | Cross-domain event and correspondence hypotheses |
| WP10 | Optimization | Exact pair event-cover solver |
| WP11 | Analogy | Cross-pair role alignment and anti-unification |
| WP12 | Symbolic optimization | Typed e-graph and goal-directed equality saturation |
| WP13 | Solvers | Incremental finite-domain SAT/MaxSAT backend |
| WP14 | ILP | Contextual guard and selector induction |
| WP15 | Program synthesis | Constructor and coordinate expression synthesis |
| WP16 | Semantics | Operator-valued symbol interpretation |
| WP17 | Composition | Schedule, dependency DAG, canvas, and renderer synthesis |
| WP18 | Architecture core | Theory weaver and belief-mode frontier |
| WP19 | CEGIS | Exact refutation and failure-derived constraints |
| WP20 | Invention | Dynamic predicates, roles, constructors, recursion |
| WP21 | Arbitration | Falsification, ranking, ambiguity, pass@2 |
| WP22 | ARC adapter | Quarantined ARC-AGI-1/2 translation and task runner |
| WP23 | Continual learning | Measured mechanism transfer, promotion, deprecation |
| WP24 | Parallel systems | Local multi-process cognitive swarm |
| WP25 | Distributed systems | Networked swarm protocol and integrity |
| WP26 | Temporal perception | Generic interactive evidence, tracks, and adapter SDK |
| WP27 | Causal learning | World theory, hidden state, intervention design, goal hypotheses |
| WP28 | Planning and composition | A*, AO*, bounded MCTS, composable skill induction |
| WP29 | Cross-domain boundary | Early modality SDK and vector-graphics frontend |
| WP30 | Vision | CPU vision frontend spike |
| WP31 | Audio/video | Temporal media frontend spikes |
| WP32 | Knowledge learning | Proof-backed claim store and transformer-free text frontend |
| WP33 | Measurement | Three-loop benchmark harness and GLL evidence packs |
| WP34 | Integration | Three-domain general-learning runtime and release configuration |


## Machine-readable coordination

`work_packages/index.json` contains each package's dependencies, owned scope, objective, deliverables, and acceptance criteria. Coding agents should consume that file for orchestration and the corresponding Markdown package for the normative assignment.

## Revised developmental stages

```text
Stage 0: WP00-WP06 plus universal Mechanism IR and generic observation protocol
Stage 1: WP09-WP21 blind invention on hidden-law suites
Stage 2: WP23 consolidation, support geometry, promotion, and learned search prior
Stage 3: WP26-WP27 interactive version spaces, interventions, goals, and latent state
Stage 4: WP28 conditional skill induction and unseen composition
Stage 5: WP29-WP32 cross-domain adapters and knowledge acquisition
Stage 6: WP33 learning-curve evidence, WP34 shared runtime, then WP22 ARC adapters
```

An ARC result cannot move Stage 1-5 APIs. Existing ARC code remains quarantined
and may run as a regression adapter while these stages are built.

## Historical suggested parallel staffing

Initial and foundation wave:

```text
Agent 1: WP00
Agent 2: WP01
Agent 3: WP02
Agent 4: WP04
Agent 5: synthetic-suite generator for WP09/WP10 tests
Agent 6: generic benchmark and GLL evidence-pack scaffolding without dataset access
```

Second wave after foundation interfaces freeze:

```text
WP03, WP13, WP23
```

Third wave:

```text
WP05, WP07, WP29
```

Fourth wave:

```text
WP06, WP08, WP09, WP12, WP24, WP26
```

Fifth wave:

```text
WP10, WP11, WP14, WP15, WP17, WP27
```

Sixth wave:

```text
WP16, WP18, WP19, WP20, WP28
```

Seventh wave:

```text
WP21, WP22, WP30, WP31, WP32, WP33
```

Integration wave:

```text
WP25, WP34
```

## Handoff rule

A downstream agent starts only from a tagged interface milestone, not from an uncommitted branch. Temporary mock implementations are permitted behind traits, but acceptance tests must run against the real dependency before the package is declared complete.
