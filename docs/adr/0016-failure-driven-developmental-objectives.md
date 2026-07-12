# ADR-0016: Generate developmental objectives from redacted failures

Status: accepted

Date: 2026-07-12

Owners: WP34 integration

## Context

The restartable curriculum selected from a finite, generator-authored objective
set. Learned prerequisites affected eligibility, but the system could not turn
an unanticipated failure into a new experiment objective. Adding objectives
without recording when they appeared would also make checkpoint replay invalid:
a future objective could incorrectly influence an earlier reconstructed choice.

## Decision

Bump developmental curriculum semantics to 1.2.0. Add a deterministic
`FailureDrivenObjectiveGenerator` whose only inputs are a schema-closed,
content-free `LearningFailureCore` and a sorted set of active mechanism roles.
It classifies the unresolved need as latent context, variable cardinality,
relational structure, mechanism composition, or value law. It emits an opaque
canonical objective and mechanism identity, learning axis, bounded hypothesis
count, transfer estimate, cost, risk, and at most three role prerequisites.

The fingerprint covers every permitted structural failure field and the bound
prerequisite roles. Task names, observations, literal values, expected answers,
domains, corpus positions, and benchmark metadata are absent. Restricted or
unsupported failure cores fail closed.

Add `AutonomousCurriculum::add_objective` and journal each accepted addition as
`CurriculumEvent::ObjectiveAdded`. A checkpoint stores only the original
objective set plus the ordered event stream. Restore introduces each generated
objective at its historical event boundary before reproducing subsequent
selection. Admission while a decision is outstanding and duplicate objective
or produced-mechanism identities are rejected.

## Alternatives considered

### Rebuild with the final objective set

This lets later objectives alter earlier choices and breaks causal replay.
Rejected.

### Let the generator inspect failed observations

That could create task-shaped curricula and leak evaluation data. The generator
receives only the existing redacted structural core.

### Use random IDs

Random identity defeats deduplication and deterministic replay. Canonical
structural identity is used instead.

## Consequences

The controller can expand its objective frontier after deployment and retain
that expansion across restart. Objective identity is reproducible and support
roles, not source task constants, determine composition prerequisites.

This does not yet synthesize a concrete world, observation family, or action
sequence that exercises the generated objective. A separate environment or
curriculum materializer must produce candidate experiences, and their outcomes
remain subject to independent verification.

## Validation

Unit tests cover deterministic identity, prerequisite-order independence,
latent/cardinality/composition classification, restricted-content rejection,
live admission, outstanding-work rejection, and event-boundary replay.

In frozen non-ARC G6, four real internal mechanisms are learned first. A bounded
failed invention is reduced to a redacted core; no composition objective exists
in the initial set. The generator creates a composition objective using the
first two learned roles, persists its addition, reconstructs a fresh controller,
and that objective becomes the next selected experience.

Rollback or kill conditions: raw task content enters identity, objective order
changes with input ordering, a future objective affects an earlier replayed
choice, or generated objectives bypass verifier-controlled outcomes.

## References

- ADR-0015 replay developmental curriculum after restart
- ADR-0014 stateful autonomous developmental curriculum
- `docs/experiments/G6_AUTONOMOUS_RESTART_CURRICULUM.md`
