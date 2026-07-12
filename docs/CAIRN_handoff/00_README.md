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

A normal chatbot interface is now an explicit product and research target. The
first target is a CPU-only, continually learning, proof-carrying software
engineering chatbot: controlled language compilation, dialogue state, typed
code synthesis, local execution, knowledge provenance, tools, and response
realization surround the same cognition runtime. Broad open-domain conversation
remains a later research target and is not implied by the initial interface.

Bootstrap knowledge may originate from a stronger external teacher only as
offline structured teaching packets. CAIRN compiles these to its own typed
claims, procedures, language mappings, mechanisms, skills, tests, and
counterexamples; independent validators and swarm peers determine acceptance.
The deployed answer path remains transformer-free and CPU-only. Verified
non-ARC knowledge may be retrieved during ARC evaluation, but must be rebound
and replayed on current demonstrations and may never encode benchmark answers.

The current interactive checkpoint also learns across worlds: two independently
verified hidden-state theories alpha-normalize to one action-ID-free mechanism,
which is automatically promoted and rebound in a third changed world. Exact
current replay preserves 510/510 held-out predictions while reducing selected
interventions from one to zero. This is bounded continual causal learning, not
a general-intelligence or ARC-score claim.

The next developmental checkpoint broadens this to five different finite
causal structures spanning one to four latent states, two to four actions, and
one or two visible dimensions. Ten source worlds promote five schemas; five
changed worlds reduce required visible transitions from 42 to 10 with exact
prediction and zero negative transfer. Evidence survives CAS restart, a bounded
planner invents a necessary two-action diagnostic, and one interactive schema
rebinds through sequence and vector adapters. These remain controlled finite
experiments, not a general-intelligence claim.

The quarantined ARC rerun after freezing that core remains 18/40 and 36/400
exact with zero incorrect. This is an honest flat result: the new world
mechanisms transfer to sequence/vector observations but do not yet address
ARC's dominant relational correspondence and rendering gaps.

The next frozen core checkpoint learns coordinate-layer set construction and
rendering from non-ARC vector-graphics examples. It represents complete entity
collections, learns union, intersection, and ordered difference, binds roles by
relational cardinality rather than literal values, and learns preserve-source
versus tight-content rendering. A union mechanism promoted from two disjoint
graphics cohorts transfers unchanged to categorical finite fields. This is a
measured cross-domain mechanism-transfer result. Its frozen ARC evaluation is
negative: combined accuracy remains 36/400, and layer-only memory ties blank
memory at 17/40. The cardinality/set path is stopped for ARC-directed expansion;
general correspondence and rendering learning remain open.

The subsequent frozen checkpoint addresses that gap at collection level. It
learns a signed-axis correspondence and explicit frame from non-ARC vector
scenes, reconstructs each output entity, and records the exact source entity and
point behind every emitted point. Two verified cohorts promote the mechanism;
held-out reuse reduces 16 fresh candidates/two examples to two memory attempts
and zero examples, transfers unchanged to categorical fields, and accepts zero
wrong-family predictions. Its frozen ARC gate is negative: correspondence-only
and blank memory both score 17/40, so the complete 400 run was correctly not
opened. The general capability remains; benchmark-directed signed-axis growth
is stopped.

The bounded developmental controller now persists only a canonical,
content-free event journal. After process loss, a fresh controller must
reproduce every choice and evidence transition before learned prerequisites,
quarantine, or deprecation regain authority. This improves restart integrity;
it is not open-ended self-curriculum or a general-intelligence claim.

## Ground-up distinction

SCEM persisted surface programs and labels. MOSAIC enumerated a fixed compositional language. CAIRN does neither.

CAIRN starts from observed differences and relational correspondences, constructs event explanations, anti-unifies those events into causal rule sketches, solves the remaining unknowns with incremental constraints, and grows task-local derived predicates or constructors from unsatisfied explanation cores. High-level operations are induced definitions over a small universal relational kernel rather than an expanding catalog of ARC operations.

## Implementation language and baseline hardware

The first self-expanding-language pivot no longer persists the signed-axis
configuration. Four independently verified executions compile into generic RKL
definitions over source coordinates, extents, and collection bounds. After
compilation v1 is removed: the synthesized program alone replays two changed
held-outs, transfers to a field, and is independently executed by the verifier.
Retained executable rule structure is now synthesized rather than selected from
a named transform representation. Autonomous trace discovery and program
promotion remain open.

That next gate now passes: unordered invention learns coordinate definitions
without point-pair labels or v1 traces. Two independent cohorts synthesize the
same canonical program, independent receipts promote it, and reuse solves two
larger held-outs with two attempts and zero examples instead of 5,986 synthesis
candidates. This is autonomous bounded program-library learning; conditional
control, non-bijective construction, and raw perception remain open.

The reference implementation is Rust with small, audited C/C++ solver dependencies when justified. The baseline machine is one commodity 8-core CPU with 16 GB RAM. The system must also support multi-process and multi-node CPU swarms through content-addressed, proof-carrying artifacts.
