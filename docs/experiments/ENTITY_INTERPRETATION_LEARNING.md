# Explanation-driven entity interpretation learning

Status: `PASS_WITH_DECLARED_LIMITS`

## Frozen question

Can CAIRN select entity boundaries from competing raw relations only because
one interpretation supports a reusable executable mechanism, then transfer and
reuse that mechanism without retaining source relation names?

## Result

```text
supplied entity groups: 0
anonymous relation roles per episode: 2
cross-episode binding sets evaluated: 4
source domains: 2
independently renamed corroborating domains: 2
held-out domains: 2
held-out groups: 9 / 9 exact
binding ablation acceptances: 0
ambiguous duplicate interpretations accepted: 0
binding-cap violations accepted: 0
sequence -> vector adapter transfer: 9 / 9 exact
core changes for adapter transfer: 0
fresh feature candidates -> memory synthesis: 708 -> 0
source examples -> reuse examples: 4 -> 0
current relation bindings still attempted: 2
blank-memory proposals: 0
```

Commands:

```sh
cargo test --offline -p cairn-theory --test entity_observation -- --nocapture
cargo test --offline -p cairn-runtime --test entity_observation_memory -- --nocapture
cargo test --offline -p cairn-bench --test g5_entity_interpretation_transfer -- --nocapture
```

## Attribution

Every intended entity consists of three raw primitives, but no component list
is supplied. One anonymous relation partitions primitives coherently; a hard
negative relation creates equal-sized mixed components. The feature learner can
construct the required invariant only on the coherent components. Rebinding
uses different relation names, values, counts, ordering, and frontend types.

The verifier uses disjoint-set reconstruction rather than the producer's graph
traversal. Replacing every selected binding with the distractor causes replay to
fail. Duplicate equivalent relations cause `Ambiguous` rather than a hidden
tie-break.

## Claim boundary

This is bounded joint perception/explanation and continual reuse, not general
intelligence. Primitive scalars and candidate binary relations are still
frontend evidence; the system does not yet invent relations from raw raster,
audio, video, or text streams. Connected components and uniform observation
arity remain part of the minimal graph substrate and current experiment design.
