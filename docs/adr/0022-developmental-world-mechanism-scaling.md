# ADR-0022: Scale persistent world mechanisms through developmental curricula

Status: accepted

Date: 2026-07-13

Owners: WP23/WP27/WP33/WP34 integration

## Context

ADR-0021 demonstrates one exact two-state, three-action, one-dimensional
mechanism transferring across three worlds and reducing interventions from one
to zero. That result is real but structurally narrow and process-local. It
cannot establish a general learner unless mechanism structure varies, accepted
knowledge survives restart, experiments may contain several actions, the
curriculum selects support gaps, and the resulting abstractions transfer beyond
their source world and into quarantined evaluation only through generic
observations.

## Decision

Extend the existing source-independent `LatentMechanismSchema`; do not add
named world families or adapter operations. Its existing finite role tables are
the reference representation for varied latent-state counts, action counts,
visible dimensions, causal graphs, and additive effect vectors. Record explicit
support geometry for observed dimensions, state/action cardinalities, and
validated structural signatures. Unsupported structure remains unknown.

Persist schema proposals and verifier evidence as canonical, parent-bound CAS
artifacts. Loaded artifacts are untrusted. A fresh runtime must reconstruct the
schema, reload referenced visible episodes, independently replay compilation
and transfer, and reproduce promotion. Serialized status or opaque in-process
receipts never restore authority by themselves.

Extend active causal learning with bounded multi-step discriminating plans.
The reference planner enumerates action sequences in deterministic
length-then-lexicographic order over the current exact theory, selecting the
shortest sequence whose predicted visible trace separates at least two
candidate successor modes. Only one action is executed at a time; every visible
outcome is validated and exact induction restarts before the next action.
Maximum plan depth and enumeration work are explicit caps.

Connect structural support gaps to the existing autonomous curriculum. The
curriculum receives content-free dimensions and verifier-derived learning
metrics, never hidden laws or expected answers. It must select experiences that
expand weak support, and promotion evidence must demonstrate retained exactness
and reduced candidates, examples, or interventions.

Cross-domain transfer uses generic relational observations and current binding.
No ARC concept enters core crates. ARC runs only after non-ARC structural
transfer is frozen and compares blank memory with the exact same verified
memory under identical budgets. Imported knowledge remains proposal-only.

## Alternatives considered

### Add one solver per hidden-world family

Rejected because family labels would move generator knowledge into cognition.

### Serialize the in-memory store directly

Rejected because trusted status and opaque verifier receipts cannot survive a
process boundary without replay.

### Execute an authored diagnostic action sequence

Rejected because it does not demonstrate experiment selection.

### Tune mechanisms after each ARC result

Rejected because evaluation-derived core design violates ARC quarantine.

## Compatibility and migration

All changes are additive to `cairn.latent-mechanism-schema.v1`. New archive and
support schemas receive distinct versioned markers. Existing v0.8 schemas
remain valid proposals but acquire only the support proven by their evidence.

## Validation

The frozen developmental suite must contain at least five structurally distinct
mechanisms and changed third-cohort worlds. After CAS restart, memory must
retain exact prediction, reduce aggregate interventions or examples, and keep
negative transfer below two percent. At least one world must require a
learner-selected plan longer than one action. At least one accepted mechanism
must improve a non-source adapter task through generic relational binding.

Only after those gates pass may the existing frozen ARC first-40 and 400-task
training evaluations run. ARC accuracy changes must be causally ablated to
non-ARC learned artifacts; unchanged accuracy remains an honest negative result.

## Rollback or kill conditions

Stop the path if serialized status bypasses replay, hidden law labels reach the
learner, multi-step planning depends on environment truth, one source promotes,
negative transfer reaches two percent, memory reduces exactness, or an ARC
result changes general-core semantics.
