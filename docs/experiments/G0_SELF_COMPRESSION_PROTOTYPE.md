# G0 self-compression prototype

Date: 2026-07-11

Status: historical narrow-domain prototype; superseded by the multifamily G0 pass

## Pre-registered question

Can CAIRN begin without a stored mechanism, independently synthesize the same
unnamed executable law in different episodes, compress their proof structure,
promote the abstraction through the strict ADR-0007 gate, and use it to learn
future variants with less search and fewer demonstrations?

The hidden behavior was unsigned magnitude over bounded integers. No magnitude
function or mechanism was present in memory. The task learner received only the
generic RKL substrate and demonstrations. The held-out task names, numeric
ranges, and values differed from both source episodes.

## Method

1. Run ten held-out variants with an empty mechanism store and an intentionally
   frozen one-cost/one-candidate initial budget.
2. Independently invent executable RKL definitions in two source episodes with
   different variable names, ranges, and examples.
3. Convert their learned definition bodies to proof-structure traces and
   anti-unify them in the separate consolidator.
4. Provisionally promote the equivalent definitions with source lineage.
5. Rebind the promoted definition to ten new names and ranges using one
   demonstration each; execute an additional unseen value in every task.
6. Measure the same tasks with fresh full search using six demonstrations.
7. Submit the Mechanism IR, support geometry, proof lineage, and measured
   invention/reuse evidence to the independent promotion verifier. The verifier
   issues a lineage-bound token; the store rejects direct persistent status and
   accepts the token exactly once.

## Result

```text
initial bounded success:       0 / 10
post-experience success:      10 / 10
fresh candidate executions: 440
memory candidate executions: 10
search reduction:             44x
fresh demonstrations/task:     6
memory demonstrations/task:    1
sample reduction:            83.3%
held-out executed predictions: 10 / 10
source-code changes:            0
actual reuse count:            10
necessary reuse count:         10
strict promotion result: persistent
```

Command:

```sh
cargo test --locked -p cairn-memory --test adr0007_g0_gate -- --nocapture
```

## Why this qualifies as bounded learning

The accepted mechanism was not inserted as a memory template or returned as an
answer lookup. Task-local synthesis produced executable definitions; repeated
learned proof structure was compressed; the definition rebound to new symbols
and ranges; current-task evidence re-admitted it; an unseen input executed
exactly; and ablation to the empty store caused all ten initial bounded attempts
to fail. Experience therefore changed future search and sample requirements.

## What it does not prove

This prototype alone did not vary collection cardinality or layout. The
successor G0_MULTIFAMILY_HIDDEN_LAW_PASS.md adds five unrelated families and
unseen cardinality/layout transfer. G1-G6 remain open, and no
general-intelligence claim is made.
