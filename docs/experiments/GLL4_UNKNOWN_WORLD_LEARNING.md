# GLL-4 initial evidence: unknown-world causal learning

Date: 2026-07-11  
Status: passed for bounded, fully observed, deterministic additive worlds

## Claim

CAIRN can now learn action-conditioned transition laws in previously unseen
worlds, infer an executable goal from outcome evidence, plan through the learned
model, reduce intervention cost on repeated episodes, and transfer an induced
multi-step option into a shifted layout.

The meanings of all action identifiers are permuted procedurally after the
learner is compiled. Production code contains no directional action IDs or
world-specific transition table.

This is bounded interactive causal learning. It is not yet the complete GLL-4
claim for hidden, conditional, stochastic, object-rich, or partially observed
environments.

## Learning loop

For each world, the generic learner:

1. identifies action effects missing causal evidence;
2. proposes one safe intervention for each unidentified action;
3. induces a bounded deterministic transition rule from observed
   state/action/outcome triples;
4. infers the minimal conjunction of state-value equalities shared by positive
   goal evidence and separating negative evidence;
5. constructs a shortest plan by bounded deterministic breadth-first search;
6. induces an option from the verified multi-step execution;
7. retains the learned action laws, so later episodes omit redundant
   interventions;
8. rebinds the world theory and option to a coordinate-shifted layout and checks
   the expected displacement.

Inconsistent effects, missing evidence, invalid bounds, planning capacity,
unreachable goals, and option-transfer mismatches are typed failures.

## Frozen procedural suite

The suite generates 100 two-dimensional finite worlds. Each has four action
identifiers mapped through an independently shuffled transition table to the
four unit coordinate effects. Starts and goals vary procedurally. The learner
receives no action labels or target transition law.

| Measure | Result |
|---|---:|
| Unknown worlds | 100 |
| Exact action-outcome predictions | 32,400 / 32,400 |
| Exact inferred-goal classifications | 8,100 / 8,100 |
| Goals reached by learned-model plans | 100 / 100 |
| Plans proven shortest | 100 / 100 |
| Shifted-layout option transfers | 100 / 100 |
| First-episode actions | 1,030 |
| Repeated-episode actions | 630 |
| Exploratory actions saved | 400 |

The repeated-episode reduction is exactly one avoided intervention per action
per world. Transition predictions include boundary states, testing that learned
effects interact correctly with declared world bounds.

## Reproduction

```sh
cargo test --locked -p cairn-world --test gll4_gate -- --nocapture
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p cairn-check -- check .
```

Generic learning, planning, goal, and option code is in
`crates/cairn-world/src/lib.rs`. Procedural truth generation exists only in
`crates/cairn-world/tests/gll4_gate.rs`.

## Claim boundary and remaining risks

- State is a bounded vector of exact integers. Entities and relations are not
  induced from raw observations in this slice.
- Transition rules are deterministic additive effects. Conditional effects,
  action preconditions, stochasticity, and hidden state are not learned.
- The goal learner receives labeled positive and negative outcome states; it
  does not yet discover an unobserved reward source through exploration.
- Intervention selection currently identifies unsupported actions; it does not
  optimize information gain between multiple competing structured theories.
- Option transfer covers coordinate-shifted layouts with invariant dynamics,
  not obstacles or changed action laws.
- One cognition runtime across three adapters remains unimplemented. Therefore
  CAIRN must not yet be described as a generally learning AI.
