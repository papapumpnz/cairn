# ADR-0004: Latent-successor ambiguity requires intervention

Status: accepted

Date: 2026-07-11

Owners: interactive-learning owner

## Context

The five-experiment latent-learning campaign exposed an identifiability failure.
Two training episodes constrained the immediate visible effect of a terminal
transition but supplied no later observation that could identify its next hidden
state. The first implementation selected the lowest-numbered consistent latent
successor. On a held-out continuation it predicted the visible value correctly
but tracked latent state zero instead of the true state one.

This was not a search defect: multiple minimum-state machines were
observationally equivalent on the supplied histories. A deterministic successor
claim was therefore unsupported.

## Decision

Latent induction must distinguish exact replay from state-machine
identification.

For every inferred `(latent state, action)` rule, CAIRN records whether its next
latent state is constrained by at least one subsequent observation. A rule seen
only as an episode terminal carries a `LatentSuccessorAmbiguity`; its chosen
successor is not treated as established knowledge.

When possible, the learner selects a continuation action whose immediate visible
effects differ across candidate successor states. The environment executes that
continuation in the same episode, and induction is repeated. Deterministic
promotion and planning require the relevant successor ambiguities to be empty.
If no single action distinguishes the candidates, the ambiguity remains explicit
and a longer belief-contingent experiment is required.

## Alternatives considered

### Keep the canonical lowest-numbered successor

Benefits: deterministic and simple.

Costs: converts missing evidence into false knowledge and can corrupt later
latent tracking while preserving one-step visible accuracy.

Reason rejected: deterministic tie-breaking is not evidence.

### Reject every theory containing a terminal transition

Benefits: fail-closed semantics.

Costs: discards useful visible-effect knowledge and prevents the learner from
requesting the exact evidence it lacks.

Reason rejected: explicit ambiguity plus intervention is more informative.

### Retain all complete machines without intervention

Benefits: fully preserves belief alternatives.

Costs: belief sets may grow unnecessarily when one cheap continuation separates
the modes.

Reason deferred: full belief-set planning is still required when no bounded
one-step discriminator exists.

## Consequences

Correctness: terminal successor choices cannot silently enter verified world
models.

Expressivity: induction reports now distinguish replay-complete rules from
identified latent transitions.

CPU cost: action signatures are compared across candidate latent states; a
follow-up induction occurs only when new evidence is collected.

Memory cost: ambiguity records store a rule key, terminal episode, and optional
distinguishing action.

Determinism: ambiguity and continuation selection are canonically ordered by
latent state and action ID.

Verifier impact: a certificate claiming deterministic latent replay must show
nonterminal support or a completed disambiguating continuation for every relied
upon successor.

Downstream interfaces: `LatentInductionReport` gains
`successor_ambiguities`; world-learning semantics advances from 1.0.0 to 1.1.0.

## Validation

The original Experiment 3 counterexample must first produce exactly one
ambiguity. The selected continuation must separate the candidate states,
re-induction must produce no remaining ambiguity, and the original seven-step
held-out history must then match both visible and latent state exactly.

The complete five-experiment campaign and all prior hidden-world suites remain
mandatory regressions.

Rollback trigger: any accepted deterministic latent successor without
continuation evidence, or any reappearance of the original held-out mismatch.

## Compatibility

Semantics version change: `WORLD_LEARNING_SEMANTICS_VERSION` is 1.1.0.

No persistent world-theory artifact has been released. Callers consuming
induction reports must inspect `successor_ambiguities` before deterministic use.

## References

- `../experiments/FIVE_LATENT_EXPERIMENTS.md`
- `../experiments/HIDDEN_STATE_WORLD_LEARNING.md`
- `0003-general-learning-is-the-development-center.md`
