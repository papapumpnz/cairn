# ADR-0021: Persist verified latent-world mechanism schemas

Status: accepted

Date: 2026-07-13

Owners: WP19/WP23/WP28/WP34 integration

## Context

ADR-0020 actively resolves one hidden-world ambiguity, but every new world
starts from blank causal memory. The resulting theory embeds action IDs, state
bounds, and latent labels, so storing it directly would not establish reusable
mechanism learning. A valid continual-learning result must extract a
source-independent law, verify it across independent worlds, rebind it to new
actions/layouts, replay current evidence, and measurably reduce interventions.

## Decision

Add a canonical `LatentMechanismSchema` in `cairn-world`. Compilation removes:

- action identifiers by sorting complete per-action causal signatures;
- latent-label permutations by fixing initial state zero and exhaustively
  choosing the lexicographically least bounded relabeling;
- state bounds and source episodes, while retaining state dimensionality;
- support counts, while retaining every delta and next-latent transition.

The schema is an executable proposal containing a finite role-indexed
transducer. It cannot produce an accepted prediction directly.

Add bounded rebinding. Given current action IDs, state bounds, and visible
episodes, enumerate action-to-role bijections, instantiate a candidate latent
theory, and retain only exact episode replays. Ambiguous bindings remain
alternatives; no arbitrary tie-break is accepted. Capacity is explicit.

Add an independent verifier path in `cairn-verifier`. It separately
canonicalizes source theories, checks schema identity, reconstructs current
bindings, replays every transition, and emits opaque compilation and transfer
receipts. Producer hashes or replay claims are not accepted as authority.

Add `LatentMechanismStore` in `cairn-memory`. A schema begins provisional. It
becomes persistent only after two independently verified source worlds with
distinct evidence identities compile to the same semantic schema. Runtime
retrieval returns proposals only. A proposed binding must still replay current
evidence and receive a fresh transfer receipt before planning or prediction may
count.

The frozen continual experiment uses two changed worlds for promotion and a
third world with changed action IDs, initial values, and state bounds. Compare
fresh active induction against persistent-schema rebinding under identical
visible evidence. Exact predictions must not fall; memory must reduce selected
interventions.

## Alternatives considered

### Store complete `LatentTransitionTheory`

Benefits: trivial replay in the source world.

Costs: action IDs, bounds, latent labels, and task-specific support prevent
structural transfer.

Reason rejected: this is episodic caching, not mechanism learning.

### Promote after one world

Benefits: earlier reuse.

Costs: one task cannot establish claimed action/layout invariance.

Reason rejected: violates the multi-experience promotion boundary.

### Allow memory to select an answer directly

Benefits: lowest apparent work.

Costs: bypasses current evidence and verifier replay.

Reason rejected: violates the memory and exactness boundaries.

## Consequences

Correctness: every source and target transition is independently replayed.

Expressivity: finite deterministic latent transducers with bounded state/action
cardinality and additive integer effects. Stochastic, continuous, and
variable-dimensional transfer remain open.

CPU cost: canonical latent relabeling is factorial in at most the declared
latent bound; action rebinding is factorial in action count and work-capped.

Memory cost: schemas store one compact rule table plus verifier receipt IDs.

Determinism: canonical bytes select latent relabeling and role order; candidate
bindings sort by action/role vectors.

Verifier impact: new independent compilation and transfer receipts; existing
world execution remains unchanged.

Downstream interfaces: additive `cairn-world`, `cairn-verifier`, `cairn-memory`,
`cairn-runtime`, and G2/G4/G6 integration APIs. ARC remains untouched.

## Validation

Tests: action-ID and bound invariance, changed-layout exact rebinding,
action-mapping capacity, dimension mismatch, verifier binding tampering,
duplicate source evidence, premature persistence, and blank-memory rejection.

Benchmark gate: two independent source worlds promote one schema; on a third
changed world, persistent memory binds without the active continuation required
by a fresh learner and predicts all held-out transitions exactly.

Rollback trigger: source IDs affect schema identity, one receipt promotes,
memory bypasses current replay, ambiguous binding silently resolves, transfer
reduces accuracy, or verifier and producer canonicalization disagree.

## Compatibility

Semantics version change: additive
`cairn.latent-mechanism-schema.v1`; existing world and memory artifacts are not
reinterpreted.

Migration: none.

## References

- ADR-0020 active latent-world interventions
- ADR-0007 self-compressing mechanism learner
- `08_GENERAL_LEARNER_REQUIREMENTS.md`
