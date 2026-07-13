# Variable-arity relation-fold learning

Status: `PASS_WITH_DECLARED_LIMITS`

## Frozen question

Can the same relation learner discover and reuse a bounded collection fold when
every atomic and binary key structure fails?

## Measured result

```text
maximum relation-key arity: 3
explicit fold capability: additive
anonymous attributes per primitive: 4
relation structure families: 6
atomic and pair exact theories: 0
selected structure: three-input sum
changed held-out groups: 9 / 9 exact
promotion receipts: 2
cohort binding work without -> with: 128 -> 16
promotion bindings saved: 112
runtime bindings without -> with: 32 -> 4
source examples -> reuse examples: 4 -> 0
independent runtime-held-out groups: 9 / 9 exact
```

Commands:

```sh
cargo test --offline -p cairn-theory --test relation_observation invents_a_variable_arity_relation_fold_beyond_pairs -- --nocapture
cargo test --offline -p cairn-runtime --test raw_relation_program_memory variable_arity_relation_fold_is_promoted_and_rebound -- --nocapture
```

## Attribution

The three useful channels vary independently within each entity and only their
sum is conserved. Every pair sum, difference, minimum, maximum, projection, and
fourth distractor channel cuts across the intended components. Attribute axes
are permuted across experiences. The complete fold is represented as typed RKL;
memory retains its source-independent operator and arity while rebinding only
current channel positions.

## Claim boundary

This demonstrates bounded n-ary composition and actual reuse. It does not
demonstrate recursive language growth, learner-invented algebraic operators,
raw sensory feature acquisition, nested entity discovery, or general
intelligence. ADR-0050 supersedes manual capability enablement for the
experience-driven gate; this experiment remains the direct fold-semantics and
memory baseline.
