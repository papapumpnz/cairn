# CAIRN experiment ladder

The ladder is designed to test the central architecture before expensive full-suite runs. Thresholds are hypotheses to pre-register before implementation of each measured stage. A failed gate is recorded; it is not weakened retroactively.

## 0. Dataset policy

### Development partitions

- Synthetic mechanism suites authored independently from the solver implementation.
- ARC-AGI-1 training split.
- ARC-AGI-2 training split.
- Public ARC-AGI-3 development environments under the benchmark's permitted rules.

### Quarantined partitions

- ARC-AGI-1 evaluation.
- ARC-AGI-2 public evaluation after the development protocol is frozen.
- ARC-AGI-2 semi-private and private evaluation.
- ARC-AGI-3 hidden environments.

No persistent module may be promoted from a quarantined task.

## 1. Measurement law

Every experiment directory contains:

```text
hypothesis.md
config.toml
run_log.md
benchmark_result.json
per_task_results.jsonl or per_case_results.jsonl
evidence.md
next_experiment_suggestion.md
```

Before a measured run:

- hypothesis and thresholds committed;
- suite frozen;
- seeds frozen;
- code commit frozen;
- baseline frozen;
- machine idle policy declared;
- expected failure taxonomy declared.

After a measured run:

- run once unless repeats were pre-registered;
- all negatives retained;
- no task-shaped patch before reporting;
- all certificates replayed by the independent checker.

## 2. Phase F: foundations

### F0 — CPU and transformer exclusion

Question: can prohibited execution paths be detected mechanically?

Gate:

- checker catches planted CUDA, ROCm, OpenCL, transformer-library, remote-model, and task-ID violations;
- runtime attestation detects a planted loaded accelerator library;
- clean workspace passes.

### F1 — grid and relation kernels

Question: are low-level semantics correct and fast?

Gate:

- exhaustive equivalence to naive reference on all grids up to 5 by 5 for defined kernels;
- at least 100,000 randomized cases up to 30 by 30;
- at least 20 million benchmarked primitive operations per core per second on baseline hardware;
- no undefined behavior under sanitizer and Miri-compatible subsets.

### F2 — RKL totality and proof replay

Question: can arbitrary typed bounded programs execute safely and replay identically?

Gate:

- type soundness property tests;
- random typed program fuzzing with no panic;
- bound checker rejects planted non-total definitions;
- independent verifier agrees with optimized executor on at least one million random programs over small grids.

## 3. Phase E: event induction

### E0 — correspondence family suite

Independent generator emits preserve, move, transform, recolor, copy, delete, split, merge, create, aggregate, and distribute transitions. Solver receives raw grids only.

Gate:

- correct event kind present in top five hypotheses for at least 98% of single-event cases;
- exact cover present in top eight for at least 95% of two-event cases;
- false exact covers below 1% on adversarial distractor cases;
- median below 20 ms per pair.

### E1 — event-cover versus forward enumeration

Compare event-derived sketches with a size-matched forward RKL enumerator under identical CPU time.

Gate:

- event method solves at least twice as many multi-event held-out cases;
- at least 5 times fewer executed complete programs per solve;
- no accuracy regression on single-event cases.

Failure falsifies H1.

### E2 — joint perception ablation

Compare:

```text
A: fixed top-ranked segmentation
B: precomputed parse lattice
C: demand-driven joint entity and theory induction
```

Suite includes multicolor objects, deceptive backgrounds, panels, nested regions, and motifs.

Gate:

- C improves exact pair coverage by at least 10 percentage points over A;
- C improves at least 5 points over B at equal CPU budget;
- median entity materialization remains below 100 ms.

Failure falsifies H2 or its implementation.

## 4. Phase T: theory formation

### T0 — contextual guard induction

Generate tasks with identical object appearance but different relational roles. Include hard negatives.

Gate:

- at least 95% held-out selector accuracy;
- zero accepted theory that changes a hard negative on demonstrations;
- literal-color and literal-position ablations show relational clauses are causally used.

### T1 — variable-arity construction

Suites vary object counts, sizes, positions, and transforms while preserving mechanism.

Gate:

- at least 95% held-out exact outputs;
- count range in held-out exceeds training range;
- no fixed-slot fallback accepted;
- output provenance accounts for every created entity.

### T2 — multi-rule composition

Suites contain independent, dependent, conflicting, and fixpoint rules.

Gate:

- at least 90% held-out exact output overall;
- at least 85% on interacting-rule cases;
- schedule recovered up to semantic equivalence in at least 90% of solved cases;
- one-rule ablation loses at least 30 points, confirming the suite is nontrivial.

### T3 — symbolic interpretation

Symbols control colors, directions, transforms, selectors, constructors, and schedule branches. Visual appearance is permuted independently of semantics across task families.

Gate:

- at least 90% held-out exact output;
- at least 80% on operator-valued meanings;
- no more than 2% false acceptance on unrelated symbols;
- symbol-map removal causes the expected collapse.

### T4 — ontology invention

The base search vocabulary deliberately omits derived relations and constructors needed by the suite, while RKL remains capable of defining them.

Gate:

- invention solves at least 70% of previously unrealizable cases;
- no task-specific primitive names or IDs;
- invented definitions transfer to held-out instances with changed counts and geometry;
- exact negative suite false positives below 2%;
- median invention overhead below 10 s.

Failure falsifies H4.

### T5 — learning from failures ablation

Compare full CEGIS constraints against candidate rejection without learned constraints.

Gate:

- at least 3 times fewer candidate executions on median;
- at least 2 times lower p95 solve time;
- same or higher exact solve rate.

Failure weakens H5.

## 5. Phase A: ambiguity and generalization

### A0 — leave-one-demonstration-out prediction

On training-only tasks with at least three demonstrations, measure whether ranking by leave-one-out improves held-out demonstration prediction versus MDL alone.

Gate:

- at least 8 percentage-point improvement on ambiguous tasks;
- no more than 2-point regression overall.

### A1 — pass@2 diversity

Synthetic tasks are generated from two plausible rule families that agree on demonstrations and differ on test inputs, with a known prior distribution.

Gate:

- pass@2 exceeds best single-mode by at least 10 points;
- second attempt duplicates first on fewer than 1% of ambiguous cases;
- diversity does not reduce pass@1.

### A2 — memory promotion

Train mechanism modules on broad and narrow variation regimes.

Gate:

- broad modules transfer at least 90%;
- narrow modules explicitly retain uncertainty and do not overclaim unsupported dimensions;
- disabling memory preserves correctness but increases cost;
- memory causes zero unverified accepts.

## 6. Phase R1: ARC-AGI-1 training

### R1.0 — frozen 60-task diagnostic

Select 60 ARC-AGI-1 training tasks before engine integration, stratified by:

```text
global transform
object edit
variable-arity construction
crop and canvas
symbol or key
multi-rule
contextual selector
iterative pattern
```

Do not choose only tasks known to fit the architecture.

Initial gate:

- at least 36/60 exact at 30 s;
- at least one solve in every family;
- fewer than half of failures may be `no_pair_event_cover`;
- no family with 0/5 or worse unless diagnosed as a named architectural gap.

The threshold is lower than MOSAIC's failed 50/60 because this suite must be broader and the first run tests integration. A later 50/60 gate follows after one taxonomy-driven general iteration.

### R1.1 — full 400 training sweep

Uniform scheduling, no learned cross-task memory except verified core definitions.

Gate 1:

- at least 120/400 exact pass@2;
- median below 15 s;
- at least 70% of solved tasks pass@1;
- failure taxonomy and ablations committed.

### R1.2 — one general taxonomy-driven iteration

One architectural change may be chosen from the dominant failure class. It must be a general mechanism with a synthetic pre-test.

Gate 2:

- at least 180/400;
- gain at least 30 tasks outside the mechanism's development subset;
- zero task-ID code;
- same config rerun within measured noise.

### R1.3 — mature static engine

After no more than three general iterations:

Target:

- at least 240/400 training;
- p95 below 120 s;
- no more than 20% failures classified as kernel or ontology closure.

Kill criterion:

- two consecutive general iterations add fewer than 10 tasks and do not shift taxonomy.

## 7. Phase R2: ARC-AGI-2 training

ARC-AGI-2 training is used for mechanism development, not public evaluation tuning.

### R2.0 — family diagnostics

Measure separate subsets for symbolic interpretation, composition, and context.

Gate:

- at least 50% on a human-labeled or independently classified 120-task training diagnostic;
- no headline family below 35%;
- symbol and composition ablations materially reduce relevant scores.

### R2.1 — full 1,000-task training sweep

Initial target:

- at least 250/1,000 exact pass@2;
- median below 20 s;
- at least 50 unique solves not attributable to ARC-AGI-1 legacy mechanisms.

### R2.2 — architecture continuation gate

Continue toward evaluation only if:

- at least 350/1,000 after general iterations;
- learning curves are not saturated;
- held-out training folds show less than 5-point degradation;
- no evidence of task-family lookup.

## 8. Gated evaluations

### G1 — ARC-AGI-1 evaluation

One sanctioned run after R1.3.

Report:

- pass@1 and pass@2;
- Wilson confidence intervals;
- CPU cost;
- overlap with SCEM and MOSAIC solves where records permit;
- memory-disabled and swarm-disabled ablations on a predeclared subset or full set if affordable.

No threshold is used to rewrite the architecture. The result is evidence.

### G2 — ARC-AGI-2 public evaluation

One sanctioned run after R2.2.

Initial continuation bars:

- 10% pass@2: architecture produces meaningful signal;
- 20%: continue optimization;
- 35%: strong symbolic CPU system;
- 50%: begin private-evaluation campaign;
- 85%: benchmark pass objective.

These bars do not imply expected scores.

### G3 — semi-private and private

Only after the public protocol, software, and resource budget are frozen. Official evaluation procedures take precedence.

## 9. Phase I: interactive intelligence

### I0 — controlled transition induction

Hidden gridworlds vary action effects, obstacles, keys, switches, and goals.

Gate:

- transition prediction at least 95% after bounded exploration;
- active policy uses at least 30% fewer actions than random probing;
- hidden-state variables correspond to independently testable effects.

### I1 — goal inference

No goal label is given. Environments expose sparse progress or terminal changes.

Gate:

- correct executable goal predicate in top three for at least 90%;
- at least 80% episode completion;
- goal ablation causes expected collapse.

### I2 — option induction

Gate:

- learned options reduce planning-node expansions by at least 5 times on held-out layouts;
- option success at least 90% where initiation condition holds;
- false initiation below 5%.

### I3 — ARC-AGI-3 public development environments

Report completion, action efficiency, theory quality, and failure taxonomy. No environment-specific source code is allowed.

Continuation gate:

- meaningful progress on every development environment;
- complete at least half with one general configuration;
- action count improves across repeated levels as theories and options are learned online.

## 10. Swarm experiments

### S0 — local speed scaling

Compare 1, 2, 4, and 8 workers.

Gate:

- at least 1.7 times at 2 workers;
- at least 3.0 times at 4 workers;
- at least 4.5 times at 8 workers on search-heavy tasks;
- deterministic semantic results across worker counts.

### S1 — capability scaling

Compare homogeneous theory-solver workers with the full heterogeneous portfolio at equal CPU time.

Gate:

- heterogeneous swarm solves at least 10% more tasks on the diagnostic suite;
- unique gains attributable to at least three non-solver roles;
- falsifier or inventor artifacts demonstrably change successful theories.

### S2 — distributed integrity

Gate:

- forged, corrupted, version-mismatched, and resource-misreported artifacts rejected;
- node loss recovers without semantic corruption;
- network overhead below 15% on a four-node task-sharded workload.

## 11. Required ablations

Every major benchmark report includes, where applicable:

```text
fixed perception versus joint perception
event-derived sketches versus forward enumeration
failure constraints on versus off
ontology invention on versus off
symbol interpreter on versus off
multi-rule scheduler on versus one-rule
memory on versus off
uniform scheduler versus learned scheduler
single process versus swarm
MDL only versus full falsification ranking
pass@1 versus pass@2
```

## 12. Final pass definition

Static pass:

```text
at least 85% exact pass@2 on a clean ARC-AGI-2 private evaluation
CPU-only
transformer-free
reproducible resource report
open, replayable method under the applicable competition rules
```

Interactive pass:

```text
complete the official environment set
with action efficiency at or near the benchmark's human baseline
without environment-specific harness logic
using the same general-purpose CAIRN API and memory system
```
