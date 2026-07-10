# CAIRN handoff bundle

CAIRN stands for **Causal Analogical Induction over Relational Networks**.

It is a CPU-only, transformer-free cognitive architecture designed around one central operation:

> infer the smallest falsifiable causal rewrite theory that explains the observations, then use that theory to predict or plan.

The architecture is a ground-up replacement for SCEM and MOSAIC. It retains their measurement discipline and low-level CPU engineering, but not their learned-atom model or fixed-DSL search model.

## Read order

1. `01_REVIEW_FINDINGS.md` — evidence-based autopsy of SCEM and MOSAIC.
2. `02_CAIRN_MASTER_DESIGN.md` — complete architecture and algorithms.
3. `03_CAIRN_INTERFACES.md` — Rust-facing types and crate boundaries.
4. `04_CAIRN_EXPERIMENT_LADDER.md` — pre-registered implementation and benchmark gates.
5. `05_CAIRN_SWARM_PROTOCOL.md` — local and distributed agent design.
6. `06_CAIRN_AGENT_OPERATING_LAW.md` — non-negotiable engineering rules.
7. `07_CAIRN_RISKS_AND_KILL_CRITERIA.md` — falsification plan.
8. `HANDOFF_INDEX.md` — work-package ownership and dependency graph.
9. `work_packages/` — one self-contained assignment per coding sub-agent.
10. `schemas/`, `templates/`, and `examples/` — artifact contracts and worked traces.
11. `BIBLIOGRAPHY.md` — official benchmark sources and primary research basis.
12. `SOURCE_ARCHIVE_RECORD.md` — hashes and review scope for the supplied attempts.
13. `VALIDATION_REPORT.md` — specification-package consistency checks.
14. `manifest.json` — file inventory and integrity hashes for this bundle.

## North-star acceptance objective

The eventual benchmark objective is at least 85% pass@2 on a clean ARC-AGI-2 private evaluation while remaining CPU-only and transformer-free. This is an objective, not a claimed result. Intermediate gates are deliberately severe enough to invalidate the architecture early if its central mechanisms do not work.

For ARC-AGI-3 and later interactive suites, the same core must learn transition and goal theories online, plan under uncertainty, and be scored on action efficiency rather than merely completing a known game.

## Ground-up distinction

SCEM persisted surface programs and labels. MOSAIC enumerated a fixed compositional language. CAIRN does neither.

CAIRN starts from observed differences and relational correspondences, constructs event explanations, anti-unifies those events into causal rule sketches, solves the remaining unknowns with incremental constraints, and grows task-local derived predicates or constructors from unsatisfied explanation cores. High-level operations are induced definitions over a small universal relational kernel rather than an expanding catalog of ARC operations.

## Implementation language and baseline hardware

The reference implementation is Rust with small, audited C/C++ solver dependencies when justified. The baseline machine is one commodity 8-core CPU with 16 GB RAM. The system must also support multi-process and multi-node CPU swarms through content-addressed, proof-carrying artifacts.
