# Observation-derived feature invention and adapter transfer

Status: `PASS_WITH_DECLARED_LIMITS`

## Frozen question

Can CAIRN invent useful executable features from anonymous primitive
observations and reuse the resulting mechanism through a different frontend?

## Results

```text
supplied derived features: 0
typed feature candidates evaluated: 22
feature sets evaluated: 20
learned feature programs: minimum(observation_0, observation_1),
                          maximum(observation_0, observation_1)
learned relation: strict enclosure
source adapters: primitive + sequence
held-out adapter: vector
held-out groups: 11 / 11 exact
raw-feature ablation acceptances: 0
adapter-conditioned core branches: 0
```

## Interpretation

ADR-0043 now obtains the same min/max result through typed program enumeration,
not the historical two-template selector. This closes one supplied-feature
limitation and demonstrates real adapter-level transfer of learned feature and
relation programs. Primitive entity boundaries, the two-input feature grammar,
exact observations, and the projection axis are still frontend supplied. It is
not raw perception, open-ended representation learning, or general intelligence.
