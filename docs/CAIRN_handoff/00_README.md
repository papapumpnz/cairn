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
8. `08_GENERAL_LEARNER_REQUIREMENTS.md` — mandatory learning loops, gates, core/adapter boundary, and honest claim rule.
9. `HANDOFF_INDEX.md` — work-package ownership and dependency graph.
10. `work_packages/` — one self-contained assignment per coding sub-agent.
11. `schemas/`, `templates/`, and `examples/` — artifact contracts and worked traces.
12. `BIBLIOGRAPHY.md` — official benchmark sources and primary research basis.
13. `SOURCE_ARCHIVE_RECORD.md` — hashes and review scope for the supplied attempts.
14. `VALIDATION_REPORT.md` — specification-package consistency checks.
15. `manifest.json` — file inventory and integrity hashes for this bundle.

## North-star acceptance objective

The north star is a verifier-controlled system that learns new task theories,
improves later learning from proof-backed experience, discovers unknown
action-conditioned worlds, composes skills, and reuses one cognitive core across
at least three domains while remaining CPU-only and transformer-free.

The eventual objective of at least 85% pass@2 on a clean ARC-AGI-2 private
evaluation remains one benchmark objective, not the project definition and not a
claimed result. It cannot substitute for the general-learning gates.

Interactive learning, continual memory, and non-ARC vector-graphics integration
are developed alongside static induction rather than after it.

## Ground-up distinction

SCEM persisted surface programs and labels. MOSAIC enumerated a fixed compositional language. CAIRN does neither.

CAIRN starts from observed differences and relational correspondences, constructs event explanations, anti-unifies those events into causal rule sketches, solves the remaining unknowns with incremental constraints, and grows task-local derived predicates or constructors from unsatisfied explanation cores. High-level operations are induced definitions over a small universal relational kernel rather than an expanding catalog of ARC operations.

## Implementation language and baseline hardware

The reference implementation is Rust with small, audited C/C++ solver dependencies when justified. The baseline machine is one commodity 8-core CPU with 16 GB RAM. The system must also support multi-process and multi-node CPU swarms through content-addressed, proof-carrying artifacts.
