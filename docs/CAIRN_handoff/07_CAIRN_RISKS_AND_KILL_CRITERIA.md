# CAIRN risks and kill criteria

## 1. Event covers do not reduce search enough

Signal:

- correspondence generation dominates runtime;
- top covers rarely contain the true event structure;
- forward enumeration performs similarly at equal CPU.

Mitigations:

- improve invariant signatures;
- learn demand scheduling;
- use stronger cover constraints;
- retain event ambiguity longer.

Kill criterion:

- Experiment E1 fails after one general correspondence revision and event search executes fewer than 1.5 times fewer candidates than forward search.

Consequence:

- reject event-first synthesis as the core and retain only its diagnostics.

## 2. Joint perception explodes combinatorially

Signal:

- entity e-graph cap reached on common tasks;
- most entities never participate in a candidate cover;
- fixed parse lattice is faster and equally accurate.

Mitigations:

- stronger demand gating;
- denotation hashing;
- value-vector novelty;
- per-hole entity generation;
- cost-based garbage collection.

Kill criterion:

- E2 fails to improve exact coverage by 5 points over parse lattice at equal CPU after one demand-policy revision.

## 3. Universal kernel is technically complete but practically unreachable

Signal:

- ontology expansion repeatedly proposes coordinate-level literal programs;
- useful high-level definitions are not discovered;
- invention median exceeds 30 s without general gains.

Mitigations:

- richer contrast features;
- constructor-specific abductive templates;
- anti-unification of partial constructions;
- reusable core-knowledge definition pack.

Kill criterion:

- T4 solves fewer than 40% of deliberately omitted-vocabulary cases after one invention revision.

## 4. Anti-unification selects superficial commonalities

Signal:

- aligned rules fit demonstrations but fail leave-one-out;
- variables generalize literals without grounding;
- relation preservation is weaker than appearance match.

Mitigations:

- hard grounding requirement;
- systematicity scoring;
- hard negatives;
- preserve multiple alignment modes.

Kill criterion:

- leave-one-out prediction is not better than MDL-only syntax search on A0.

## 5. Constraint encoding becomes the bottleneck

Signal:

- SAT variables grow faster than event ambiguity shrinks;
- encoding time exceeds execution time by an order of magnitude;
- unsat cores are too coarse to guide invention.

Mitigations:

- decompose rule clusters;
- lazy theory checking;
- custom finite-domain propagators;
- hybrid enumerative fill for small holes;
- incremental assumptions.

Kill criterion:

- p95 constraint time exceeds 80% of task budget on the 60-task diagnostic while fewer than 20% of those tasks solve.

## 6. Exact fit remains too ambiguous

Signal:

- many low-cost theories survive;
- leave-one-out is unavailable or non-discriminating;
- pass@2 remains near pass@1.

Mitigations:

- stronger causal constant penalties;
- applicable equivariance re-derivation;
- cross-task core-knowledge priors;
- distinct posterior-mode modeling.

Kill criterion:

- A1 fails to improve pass@2 by 5 points on the controlled ambiguity suite.

## 7. Symbolic interpretation is not identifiable

Signal:

- symbol maps overfit appearance;
- operator-valued meanings explode;
- unrelated small motifs are treated as controls.

Mitigations:

- causal covariance trigger;
- legend structure detector;
- map complexity priors;
- negative control symbols;
- mutual exclusivity constraints.

Kill criterion:

- T3 remains below 60% operator-valued held-out after one general symbol-map revision.

## 8. Dynamic invention recreates task-shaped code

Signal:

- promoted definitions contain source-specific constants;
- one definition supports one task only;
- names obscure literal grids or task IDs;
- transfer collapses under parameter changes.

Mitigations:

- task-local default;
- support geometry;
- held-out rebinding;
- static lint;
- description-cost penalty.

Kill criterion:

- any evaluation solve depends on a persistent module that fails current-task re-verification or contains prohibited source-specific content. Treat as invalid result.

## 9. Memory repeats SCEM's failure

Signal:

- memory improves training coverage but not held-out folds;
- exact solve rate falls when memory proposals dominate scheduling;
- modules transfer only within source families.

Mitigations:

- proposal-only boundary;
- broad/narrow variation tests;
- uncertainty on unexercised axes;
- memory-off ablation.

Kill criterion:

- two successive promotion cohorts add zero held-out solves and less than 10% speed benefit. Freeze semantic memory growth and retain procedural compilation only.

## 10. Swarm scales traffic rather than intelligence

Signal:

- duplicate theories dominate;
- verifier bottleneck;
- network overhead exceeds gains;
- heterogeneous agents have no unique solves.

Mitigations:

- novelty-aware broker;
- structural sharding;
- local value reconstruction;
- role contribution metrics.

Kill criterion:

- S1 shows less than 5% capability gain and S0 misses 3 times speedup at four workers. Simplify to task-level parallelism.

## 11. ARC-AGI-1 improves but ARC-AGI-2 collapses

Signal:

- strong global geometry but weak symbol, context, and composition scores;
- training gains concentrated in ARC-AGI-1-like families.

Mitigations:

- enforce family diagnostics before full sweeps;
- prioritize T0–T4 synthetic gates;
- prohibit geometry-only headline claims.

Kill criterion:

- after R1 mature target, ARC-AGI-2 diagnostic remains below 20% and no headline family exceeds 30%. The architecture is an ARC-AGI-1 specialist, not the requested system.

## 12. Interactive extension is not actually general

Signal:

- game-specific source branches;
- action policies work only with known goals;
- state models fail under changed layouts;
- raw history grows without bound.

Mitigations:

- executable goal hypotheses;
- hidden-state criteria;
- environment-agnostic action/event API;
- compressed event memory;
- held-out environment generators.

Kill criterion:

- I0 active policy does not beat random probing by 20% or I1 cannot infer goals above 60%. Do not claim ARC-AGI-3 capability.

## 13. CPU performance is incompatible with target accuracy

Signal:

- median static solve exceeds 60 s before mature accuracy;
- memory exceeds 16 GB;
- solver relies on impractical node counts.

Mitigations:

- profile before architecture expansion;
- batch value vectors;
- micro-op compilation;
- cap low-value alignments;
- use task-level swarm parallelism.

Kill criterion:

- mature 60-task diagnostic remains below 30 solves at 120 s despite correctness of component gates. The search architecture is not viable under the hard compute constraint.

## 14. Overall architecture kill criterion

Stop CAIRN as the primary path when all are true:

```text
R1 has completed at least two general taxonomy-driven iterations
full ARC-AGI-1 training remains below 120/400
ARC-AGI-2 diagnostic remains below 20%
event, joint-perception, and ontology-invention component gates have either failed or shown no unique contribution
no dominant implementation defect explains the result
```

A negative result should preserve reusable assets: RKL, proof checker, REG, event diagnostics, and benchmark infrastructure.
