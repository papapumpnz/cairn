# ADR-0007: Make verified mechanism growth the principal output

Status: accepted

Date: 2026-07-11

## Context

The first quarantined ARC development batch reached 10/10 and the next frozen
batch reached 17/20 while this decision was being prepared. That work is useful
diagnostic evidence, but it also demonstrates the architectural hazard: adding
synthetic-tested finite-field schemas can improve ARC coverage while leaving
the central question—whether experience autonomously expands a reusable
reasoning language—largely unanswered.

Task-local synthesis is learning, but it is not by itself continual mechanism
learning. Storing a function is also insufficient: the system must demonstrate
that the abstraction was synthesized from the universal substrate, compresses
independent explanations, predicts held-out situations, rebinds outside its
source episode, and measurably reduces later search, samples, or actions.

## Decision

CAIRN is a self-compressing causal theory learner. Its principal output is an
expanding, verified language of mechanisms. Answers and plans are consequences
of executing that language against current evidence.

The canonical developmental loop is:

```text
experience
  -> infer executable competing theories
  -> compare proof and execution graphs across experiences
  -> invent parameterized mechanisms by compression
  -> verify them on held-out and adversarial situations
  -> promote and compile supported mechanisms and skills
  -> measure whether future learning becomes cheaper
```

The central persistent semantic object is `Mechanism`, represented in the
universal bounded language and carrying a signature, typed variables, derived
entity definitions, preconditions, causal transition rules, invariants,
termination, effects, support geometry, and a derivation certificate. A static
transformation is a transition under an implicit action; an interactive action,
planning option, experimental procedure, and conditional skill use the same
representation.

Developers supply only the bounded substrate: types, sets, relations, variables,
arithmetic, graphs, time, actions, observations, conditionals, bounded
iteration/recursion, and composition. Named objects, containers, paths,
symmetries, collision laws, symbol meanings, world laws, and skills count as
learned only when synthesized as definitions over that substrate.

Three runtimes have separate authority:

- the online runtime maintains a version space, selects evidence or
  interventions, refines theories, and predicts or plans;
- the consolidator receives completed proof/execution graphs and proposes
  anti-unified predicates, constructors, latent variables, higher-order
  mechanisms, and conditional skills;
- the curriculum runtime selects or generates experiences with high expected
  information gain over weak support dimensions and novel compositions.

The independent verifier alone confirms replay. The consolidator cannot promote
its own proposal. Promotion follows candidate, local, provisional, validated,
persistent, compiled, and deprecated states. Persistent entries record tested,
invariant, unknown, and failed support dimensions plus actual reuse and ablation
metrics.

## Required invention routes

The invention system must independently support:

1. repeated proof-subgraph compression and anti-unification;
2. contrast-driven predicate invention from positive and hard-negative bindings;
3. residual-driven constructor invention from unexplained effects;
4. intervention-testable latent-variable invention;
5. bounded recursive and higher-order abstraction.

A route is not complete because a matching Rust function exists. Its hidden-law
generator must omit the target mechanism from the initial learner vocabulary.

## Revised implementation order

1. trusted observation, REG, Mechanism IR, RKL, executor, verifier, CAS, and CPU enforcement;
2. blind mechanism invention on independently authored hidden-law suites;
3. consolidation, support geometry, promotion, proof retrieval, typed probabilistic grammar, and negative transfer;
4. interactive version spaces, interventions, latent state, goals, and planning;
5. conditional skill induction, recovery, and unseen composition;
6. shared grid, sequence/table, interactive-world, and vector adapters;
7. quarantined ARC-AGI-1/2/3 adapters and evaluations.

ARC work may continue only as adapter evaluation. A failed ARC case cannot
justify adding a named transformation to a general core crate.

## Decisive gates

- G0 omitted-mechanism invention: below 20% initially and above 70% after
  experience, without source changes, with visible executable definitions and
  transfer to unseen cardinalities/layouts;
- G1 actual reuse: every counted persistent mechanism records independent
  invocations, necessity, depth removed, candidates saved, and sample savings;
- G2 continual improvement: at least 4x fewer candidate executions, 2x lower
  median CPU time, 30% fewer examples/interventions, no accuracy loss, and under
  2% negative transfer;
- G3 unseen skill composition: at least 80% exact completion with explicit
  learned-skill calls and predictable ablation failures;
- G4 unknown causal worlds: at least 95% next-state prediction, 50% fewer
  exploratory actions than random, testable hidden variables, and changed-law
  or changed-layout transfer;
- G5 cross-domain cognition: one unchanged runtime, at least three adapters, no
  domain branches in core, and measured cross-domain mechanism reuse;
- G6 developmental scaling: capability rises while marginal CPU, examples, and
  interventions per acquired mechanism fall without catastrophic forgetting.

## Consequences

Existing finite-field schemas remain quarantined development baselines and must
be migrated out of the cognition core or reconstructed as learner-invented RKL
mechanisms before they can count toward any learning claim. Existing bounded
GLL results remain valid component evidence but do not satisfy the stronger
gates above unless regenerated with invention provenance, reuse counters,
support geometry, and ablations.

Benchmark reports must include learning curves and mechanism-use evidence, not
only solve rates. The repository must not claim general intelligence until all
decisive gates pass on frozen suites.
