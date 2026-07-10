# CAIRN sub-agent handoff index

Each work package is intended for one primary coding sub-agent. Packages in the same lane may execute concurrently after dependencies are satisfied. One integration agent owns public API conflict resolution.

## Dependency lanes

```text
Lane A: governance and substrate
WP00 -> WP01
WP00 -> WP02
WP01 + WP02 -> WP03
WP01 -> WP04 -> WP05
WP03 + WP05 -> WP06

Lane B: static evidence
WP02 + WP03 + WP04 -> WP07
WP07 -> WP08
WP07 + WP08 -> WP09 -> WP10
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

Lane D: ARC and measurement
WP02 + WP06 -> WP22
WP06 + WP21 + WP22 -> WP33
WP22 + WP33 -> WP34

Lane E: memory and swarm
WP01 + WP06 + WP20 -> WP23
WP01 + WP06 + WP18 -> WP24
WP24 -> WP25

Lane F: interactive intelligence
WP03 + WP07 + WP22 -> WP26
WP19 + WP26 -> WP27
WP18 + WP27 -> WP28

Lane G: modality extension
WP03 + WP04 + WP06 -> WP29
WP29 -> WP30
WP29 + WP26 -> WP31
WP29 + WP23 -> WP32
```

## Work package catalog

| WP | Owner focus | Primary deliverable |
|---|---|---|
| WP00 | Governance | Repository, LAW, CPU/transformer checker, CI |
| WP01 | Storage | Canonical CBOR, BLAKE3 CAS, artifact envelopes |
| WP02 | Systems | Packed grids and CPU geometry kernels |
| WP03 | Data systems | Relational Evidence Graph and query indexes |
| WP04 | Languages | RKL AST, types, bounds, canonical form |
| WP05 | Runtime | Reference and optimized RKL executors |
| WP06 | Formal integrity | Proof DAGs, replay certificates, independent verifier |
| WP07 | Perception | Demand-driven entity algebra |
| WP08 | Vision geometry | Structure evidence and motif/panel detectors |
| WP09 | Abduction | Event and correspondence hypothesis generator |
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
| WP22 | ARC static | ARC-AGI-1/2 frontend and task runner |
| WP23 | Learning | Mechanism memory, promotion, deprecation |
| WP24 | Parallel systems | Local multi-process cognitive swarm |
| WP25 | Distributed systems | Networked swarm protocol and integrity |
| WP26 | Temporal perception | Interactive states, tracks, event history, ARC-AGI-3 adapter |
| WP27 | Causal learning | World theory, hidden state, intervention design, goal hypotheses |
| WP28 | Planning | A*, AO*, bounded MCTS, option induction |
| WP29 | Extensibility | Modality frontend SDK and graphics frontend |
| WP30 | Vision | CPU vision frontend spike |
| WP31 | Audio/video | Temporal media frontend spikes |
| WP32 | Text/knowledge | Transformer-free parser and knowledge-graph frontend spike |
| WP33 | Measurement | Benchmark harness, evidence packs, gated evaluation |
| WP34 | Integration | End-to-end solver, profiling, release configuration |


## Machine-readable coordination

`work_packages/index.json` contains each package's dependencies, owned scope, objective, deliverables, and acceptance criteria. Coding agents should consume that file for orchestration and the corresponding Markdown package for the normative assignment.

## Suggested parallel staffing

Initial wave:

```text
Agent 1: WP00
Agent 2: WP01
Agent 3: WP02
Agent 4: WP04
Agent 5: synthetic-suite generator for WP09/WP10 tests
Agent 6: benchmark and CI scaffolding for WP33 without dataset access
```

Second wave after public interfaces freeze:

```text
WP03, WP05, WP07, WP13, WP22
```

Third wave:

```text
WP06, WP08, WP09, WP12, WP24
```

Fourth wave:

```text
WP10, WP11, WP14, WP15, WP17
```

Fifth wave:

```text
WP16, WP18, WP19, WP20
```

Sixth wave:

```text
WP21, WP23, WP26, WP33, WP34
```

Optional extension wave only after static core gates:

```text
WP25, WP27, WP28, WP29, WP30, WP31, WP32
```

## Handoff rule

A downstream agent starts only from a tagged interface milestone, not from an uncommitted branch. Temporary mock implementations are permitted behind traits, but acceptance tests must run against the real dependency before the package is declared complete.
