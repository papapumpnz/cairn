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

ADR-0029 now composes an invented relational selector with joint tuple-valued
RKL geometry, supports changing output cardinality, two-cohort promotion,
zero-example reuse, and finite-field transfer. The frozen ARC-10 ablation is
10/10 in both conditions; ARC-20 hit its 600-second command cap. Split/merge,
creation, raw perception, broad general intelligence, and the ARC target remain
open.

ADR-0030/0031 add learned ordered two-relation paths and unordered one-to-many
collection construction. Both now pass independent-cohort promotion and
zero-example reuse gates. Relation roles/entities remain supplied, and merge,
conditional multiplicity, raw perception, and broad general intelligence are
still open.

ADR-0032 adds learned relation-defined many-to-one aggregation. A composed RKL
merge promotes from independent cohorts and reuses with zero examples. Group
membership is still supplied, so joint grouping/entity discovery remains the
next general-learning gate.

ADR-0033 now selects a grouping relation jointly with its executable merge from
competing hypotheses. No selected grouping is supplied. Candidate relations
remain frontend proposals, so this is joint ontology selection rather than raw
perception.

ADR-0034 retains that relation/merge composition through proof-gated continual
memory. Two independent verifier cohorts and a separate consolidation
certificate are mandatory; later use falls from 187 candidates and four
examples to one proposal and zero examples over nine exact held-out groups.
Candidate-relation generation and durable restart remain open.

ADR-0035 now constructs candidate collection partitions directly from opaque
entities and unselected generic relation edges. Producer and verifier use
separate component paths. Two cohorts promote the relation-bound mechanism;
future raw-edge work falls from 187 candidates/four examples to one attempt/zero
examples over 13 exact held-out components. Relation predicates themselves are
still observed evidence rather than invented perception.

ADR-0036 makes that bounded learning restart-safe without trusting serialized
status. Canonical CAS cohorts survive complete process loss; a blank process
re-invents, independently replays, reconsolidates, and re-promotes the same
mechanism. Stored status restored is zero, while the 187-to-one candidate and
four-to-zero example reductions remain.

ADR-0037 removes observed relation edges for one bounded family. Given only
anonymous entity features and unordered outputs, CAIRN synthesizes the pairwise
RKL equality predicate that forms useful collections, then learns their
aggregate. Independent replay is exact over 13 changed held-out components;
multi-feature and geometry-derived relations remain open.

ADR-0038 removes the single-feature restriction. On a frozen family where all
atomic relations fail, CAIRN invents the executable conjunction of two anonymous
feature equalities, agrees across two independently named cohorts, and replays
11/11 changed held-out collections with zero examples. Equality atoms and
frontend-authored features remain decisive limits.

ADR-0039 carries that invented predicate/aggregate through proof-gated memory.
Two disjoint independent receipts and a consolidator certificate reduce later
work from 65 candidates/four examples to one proposal/zero examples on changed
entities. Memory remains proposal-only; durable restart and richer atoms remain
open.

ADR-0040 adds optional order-comparison atoms and canonical unordered-pair
orientation. From anonymous interval endpoints and outputs, CAIRN invents the
two-inequality enclosure relation and independently replays 11/11 changed
held-out groups. No named geometry primitive, edge, or pair label is supplied.

ADR-0041 makes invented predicate learning restart-safe through evidence rather
than serialized authority. Two CAS cohorts survive process loss, are re-invented
and independently replayed, and regain one promoted mechanism with the same
65-to-one candidate and four-to-zero example reduction.

ADR-0042 removes supplied normalized endpoint features. CAIRN selects executable
RKL minimum/maximum definitions from anonymous primitive pairs, composes them
with invented enclosure, and transfers the unchanged learned object from the
sequence adapter to the vector adapter over 11/11 held-out groups.

ADR-0043 removes the remaining two-template feature menu. A bounded,
cost-ordered typed RKL grammar constructs scalar feature programs and jointly
selects them through downstream explanatory adequacy. On a frozen non-ARC
family it synthesizes addition, transfers sequence-to-vector over 11/11 groups,
and proof-gated reuse reduces 91 candidates/four examples to one proposal/zero
examples. ADR-0044 persists evidence without trust and reconstructs the learned
mechanism after complete process loss. ADR-0045 replaces the fixed pair with
bounded variable-arity evidence, verifies every observation permutation, and
records tested support geometry. A three-observation mechanism passes 9/9
changed held-out groups and restart re-promotion. Entity boundaries and
projection axes remain supplied; this is bounded learning, not general
intelligence.

ADR-0046 composes entity hypothesis generation with that learner. From raw
scalars and two competing anonymous relations per episode, CAIRN selects the
only entity interpretation supporting a complete executable theory, rebinds
renamed roles from the sequence adapter to the vector adapter over 9/9 groups,
and removes 708 feature candidates/four examples during reuse. Candidate
relations are still frontend evidence, so this is not raw perception or a
general-intelligence claim.

ADR-0047 removes supplied relations for one bounded non-ARC family. From raw
primitives with anonymous attributes, CAIRN constructs executable equality RKL,
selects a task-local attribute binding only through complete explanation, and
independently regenerates relation edges before entity/mechanism replay. The
unchanged structure transfers sequence-to-vector over 9/9 groups and reuse
removes 708 feature candidates/four examples. Equality is the only current
relation structure and attributes remain frontend evidence, so general
intelligence remains unmet.

ADR-0048 removes the equality-only closure for the next bounded gate. The
cost-phased typed relation-key grammar considers projection, addition,
subtraction, minimum, and maximum. A family with zero explanatory atomic keys
forces addition; the complete executable mechanism transfers unchanged from
sequence to vector over 9/9 groups. Two independent verifier receipts promote
it as a proposal-only module, reducing binding search 15→3 and examples 4→0.
Canonical evidence survives process loss without authority and is freshly
re-invented and re-promoted. A content-free relation failure also drives one
autonomous curriculum objective to verified completion. Exact scalar channels
and the five grammar constructors are still supplied, so general intelligence
remains unmet.

ADR-0049 lifts relation keys beyond ordered pairs through a bounded associative
fold in the same RKL/verifier/memory path. With arity three enabled, a family
where every atomic and binary key fails learns a three-input sum, promotes it
from two independent cohorts, cuts current bindings 32→4 and examples 4→0,
and independently replays 9/9 changed groups. The fold operator and arity cap
remain developer-supplied, so this is higher-arity mechanism learning rather
than autonomous grammar invention or general intelligence.

ADR-0050 removes that experiment's manual fold-enablement assumption. The
default proposal language is closed. A redacted structural failure propagates
its committed dependency-arity lower bound, while a separate verifier-side
consolidator requires a promoted binary mechanism before exposing only its own
associative fold family. Addition/2→sum/3 reduces cohort binding work 128→16,
examples 4→0, and replays 9/9 changed groups; minimum/2→minimum-fold/3 also
replays 9/9. The learned additive fold also transfers sequence→vector over 9/9
groups through unchanged adapters. Cold restart regenerates the extension only
after re-invention and fresh promotion. Algebra laws and scalar channels remain
supplied, so this is bounded experience-driven language growth, not general
intelligence.

ADR-0051 moves beyond one-operator closure. Independently promoted addition and
minimum mechanisms become the vocabulary for a bounded expression search; the
certificate does not contain a final AST. Two disjoint cohorts invent and
promote `minimum(add(p0,p1),p2)`, reducing expression structures 4→1 per cohort
and examples 4→0. Answer-hidden sequence→vector reuse emits two proposals and
replays 9/9 groups. This is learner-constructed rule structure, but the binary
operators and left-associated meta-grammar remain supplied.

ADR-0052 preserves a decisive negative result: exposing both binary-tree
association shapes on the unchanged frozen family makes theory induction
ambiguous. CAIRN rejects the task rather than selecting by enumeration order;
broader grammar growth now requires independently frozen discriminating
evidence. ADR-0053 makes the validated bounded expression restart-safe without
serializing authority. Two canonical cohorts restore zero status, are freshly
re-invented and independently replayed, regain one promotion, and yield one
exact answer-hidden future proposal through the shared cognition runtime.

ADR-0054 reopens broader association only through a content-free ambiguity
curriculum and separately verified shape-extension certificate. A relation-only
contrast first confirmed that equality partitions erase association; CAIRN now
retains exact scalar results in a distinct evidence path. Two independent
cohorts invent and promote `subtract(p0,minimum(p1,p2))`, reduce bounded
expression structures 8→1 and examples 4→0, and cover 8/8 changed held-out
values through proposal-only runtime memory. Operators, scalar channels, and
tree bounds remain supplied, so this is bounded mechanism learning rather than
general intelligence.

ADR-0055 makes that scalar mechanism restart-safe without restoring authority.
Two canonical evidence cohorts reopen from CAS, freshly reconstruct the same
mechanism under independent replay, issue a new promotion, and cover 8/8 future
cases. No AST, definition, binding, receipt, certificate, or status is stored.

The reference implementation is Rust with small, audited C/C++ solver dependencies when justified. The baseline machine is one commodity 8-core CPU with 16 GB RAM. The system must also support multi-process and multi-node CPU swarms through content-addressed, proof-carrying artifacts.
