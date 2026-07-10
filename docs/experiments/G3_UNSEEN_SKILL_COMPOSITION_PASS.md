# G3 unseen conditional-skill composition pass

Date: 2026-07-11

Status: frozen initial G3 suite pass

## Architecture

The new cairn-skill crate represents a learned skill with:

- typed parameters;
- initiation and goal conditions;
- an explicit control program;
- predicted add/remove effects;
- invariants and failure conditions;
- recovery branches;
- resource bounds;
- independent episode proof lineage.

Skill induction intersects independently successful traces to retain stable
causal preconditions while discarding varying surface facts. Stable effects and
the primitive control program must agree across traces.

The planner performs exact breadth-first effect chaining over learned skills and
emits every invocation with before/after state. It has no domain labels and no
composite key-door routine.

## Frozen curriculum

The independent benchmark generator supplies separate traces for:

1. acquire a visible key;
2. navigate from the start role to a barrier role;
3. open a barrier when at it and holding a key;
4. reach a target after the barrier is open.

No trace demonstrates their composition. One hundred test scenarios vary
irrelevant layout facts while retaining the same relational roles.

## Result

    independently induced skills:       4
    unseen composite tasks exact: 100/100
    explicit skill invocations:        400
    composite mechanisms supplied:       0
    ablated skill/task combinations:    400
    predictable no-plan failures:   400/400

Command:

    cargo test --locked -p cairn-bench --test g3_composition -- --nocapture

## Gate judgment

The suite exceeds G3's 80% exact threshold, records explicit learned-skill
calls, and shows predictable failure when any required skill is removed.

This is a finite propositional role-state environment. Parameter binding to
rich entities, learned recovery branches, stochastic effects, and hierarchical
planning remain future work. G4-G6 remain open.
