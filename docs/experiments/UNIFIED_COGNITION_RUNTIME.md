# Unified cognition runtime

Date: 2026-07-11

Status: shared orchestration checkpoint

## Change

`CognitionRuntime` is now the common owner and entry point for:

- generic `LearningProblem` admission;
- task-local definition invention with memory proposals;
- verifier-gated scalar mechanism memory;
- canonical checked-theory procedural memory;
- action-conditioned transition induction;
- goal induction from positive and negative observations;
- exact bounded planning under the learned world and goal theories;
- conditional learned-skill composition.

The APIs use core, mechanism, world, and skill types. They contain no ARC,
vector, sequence, or gridworld adapter payloads.

## Unified test

One runtime instance:

1. learns an omitted scalar transformation from demonstrations;
2. induces two unknown action effects from transitions;
3. infers a goal predicate from achieved/non-achieved states;
4. produces the shortest three-action plan under those learned theories;
5. composes two independently induced conditional skills into a new behavior.

Separately, the higher-order collection test inserts a canonically verified
quantified theory into the same runtime's procedural memory and executes it on
unseen cardinality.

Commands:

    cargo test --locked -p cairn-runtime --test unified_learning_loops
    cargo test --locked -p cairn-theory --test collection_invention

## ARC regression

After runtime integration, the quarantined official checkpoint remains:

    exact:                  17/20
    resource limited:       3/20
    generic observations:    150
    relational nodes:     23,114
    relational facts:     70,550

No domain-specific operation was added to the runtime.

## Claim boundary

This establishes one orchestration surface and common memory owner. The current
world learner still assumes small bounded additive deterministic state effects,
and the skill planner operates over exact symbolic fluents. Rich noisy worlds,
belief-space skill policies, recovery learning, and learned perception remain
open. The ARC adapter still invokes its quarantined legacy backend after generic
admission and does not count as a shared-runtime solve.
