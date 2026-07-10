# G0 multifamily hidden-law pass

Date: 2026-07-11

Status: frozen initial G0 suite pass

## Scope

An independently owned cairn-bench generator produced five unrelated hidden
mechanism families. Family labels and expected probes remained benchmark
metadata and never entered learner selection:

1. unsigned magnitude over bounded scalar readings;
2. lower/upper clamping;
3. relational selection between arbitrary symbols;
4. collection cardinality combined with a varying adjustment;
5. two-axis coordinate translation.

The starting mechanism store was empty. The learner received only generic RKL
types, variables, literals, bounded composition, and demonstrations. Each
family had two independent source episodes and two held-out tasks.

## Protocol

- Freeze one-cost/one-candidate initial attempts.
- Invent an executable definition independently in both source episodes.
- Extract learned proof structure and anti-unify it in cairn-consolidator.
- Create only a provisional memory module.
- Rebind it on held-out names, bounds, symbols, counts, positions, and layouts
  using one demonstration.
- Execute one additional held-out probe per task.
- Compare with fresh full search on the identical held-out problem.
- Submit support geometry, compression, replay, work, sample, and reuse
  evidence to cairn-verifier.
- Require a lineage-matched single-use token before persistent status.

## Frozen result

    families:                         5
    independent source episodes:     10
    held-out tasks:                  10
    initial success:               0/10
    post-experience success:      10/10
    fresh candidate executions: 33,600
    memory candidate executions:    10
    candidate reduction:         3,360x
    memory demonstrations/task:       1
    held-out probes exact:         10/10
    persistent mechanisms:             5
    source changes during run:          0

Varied support dimensions:

    parameter names
    numeric ranges
    categorical values
    collection cardinalities
    positions
    layouts

Command:

    cargo test --locked -p cairn-memory --test adr0007_g0_multifamily -- --nocapture

## Gate judgment

This frozen suite satisfies the ADR-0007 G0 criteria: initial capability was
below 20%, post-experience capability exceeded 70%, no source changed, learned
definitions remained visible and executable, and transfer covered unseen
cardinalities and layouts.

This does not prove general intelligence. G1 requires durable actual-use
accounting over a continuing curriculum; G2 requires timed, equal-budget
continual-learning comparisons and negative transfer below 2%; G3-G6 remain
open. The present families also use a bounded expression substrate and do not
yet establish latent-variable, recursive, or residual-constructor invention.
