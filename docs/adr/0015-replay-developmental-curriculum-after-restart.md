# ADR-0015: Replay developmental curriculum after restart

Status: accepted

Date: 2026-07-12

Owners: WP34 integration

## Context

ADR-0014 introduced autonomous developmental selection, but the controller's
history, prerequisite state, outstanding decision, quarantine, and deprecation
were process-local. Mechanism proofs survived CAS restart while curriculum
authority did not. Serializing the resulting maps and statuses as trusted state
would violate CAIRN's replay boundary and make a forged promotion or omission
indistinguishable from learned history.

## Decision

Bump developmental curriculum semantics to 1.1.0 and define canonical schema
`cairn.curriculum.checkpoint` version 1. A checkpoint contains the ordered,
domain-neutral objective definitions, a fixed decision budget, an immutable
semantic event journal, and an optional outstanding decision. Events are either
an accepted decision/outcome pair or a distinct verified counterexample.

The checkpoint contains no observation, answer, task identifier, verifier
token, serialized learned-status map, cache, or process pointer. Persistence
requires nonempty parent lineage and a successful local replay. Loading bounds
and validates the envelope and payload, reconstructs a fresh controller, calls
ordinary selection and admission methods for every event, and checks every
recomputed decision exactly. Quarantine, deprecation, reuse counts, completed
objectives, and prerequisite eligibility are consequences of replay, not
trusted serialized authority.

An outstanding decision is deterministically reissued during restore and must
match the journal's decision exactly. Unsupported semantics, malformed fields,
missing lineage, task-scoped checkpoints, capacity divergence, and changed
selection results fail closed.

## Alternatives considered

### Serialize controller maps directly

This is faster to load but would trust the learner's own status claims and make
counterexample omission difficult to detect. Rejected.

### Persist only completed objective IDs

This cannot independently reconstruct measured reductions, mechanism evidence,
counterexample state, verified reuse counts, or the decision sequence. Rejected.

### Recompute from raw tasks

That would put domain observations inside the curriculum boundary and could
leak benchmark content. The controller replays content-free verified summaries
instead.

## Consequences

Correctness: restart state is derived through the same fail-closed API as live
state. A forged recorded decision is rejected when deterministic selection
does not reproduce it.

Determinism: canonical objective order and event order fix identity; metadata
does not enter semantic identity.

Cost: restore is linear in the number of bounded journal events and repeats
selection. Checkpoint size is capped at 1 MiB and decision count remains bounded.

Compatibility: one-shot curriculum APIs remain unchanged. Consumers of
developmental semantics 1.0.0 must explicitly migrate by creating a new 1.1.0
checkpoint from verified source evidence; old process snapshots are not
silently accepted.

## Validation

Unit tests replay learned outcomes, quarantine/deprecation, and an outstanding
decision. CAS tests cover idempotence, reopen/recovery, missing lineage,
malformed payload, and a forged journal decision. The frozen G6 integration
persists and reloads the curriculum at both existing mechanism-memory restart
boundaries and recovers a deprecated mechanism before selecting the eligible
composition objective.

Rollback or kill conditions: restored selection differs from live selection,
status can be changed without replayable events, metadata changes semantic
identity, task-scoped content enters the checkpoint, or corruption is accepted.

## References

- ADR-0014 stateful autonomous developmental curriculum
- ADR-0013 replay internal learning after restart
- `docs/experiments/G6_AUTONOMOUS_RESTART_CURRICULUM.md`
