# Review findings: SCEM and MOSAIC

## 1. Evidence reviewed

The review used the supplied archives and, in particular, the following committed artifacts:

- `scem/experiments/FINAL_EVAL.md`
- `scem/docs/scem_failure_review_refactor_plan.md`
- `scem/docs/h2_deep_failure_review_and_h3_refactor.md`
- `scem/experiments/H2-synth-transfer/`
- `scem/experiments/H2-synth-discovery/`
- `scem/experiments/H3.0-active-vs-passive/`
- `scem/experiments/H3.1-axis-invention/`
- `scem/experiments/H3.2-primitive-invention/`
- `scem/experiments/R3.6-substrate-headroom/`
- `mosaic_docs/docs/MOSAIC_DESIGN.md`
- `mosaic_experiments/experiments/WP3/`
- `mosaic_experiments/experiments/WP3-R1/`
- `mosaic_experiments/experiments/WP5/`
- `mosaic_experiments/experiments/X1/`

All numerical claims below come directly from those artifacts.

## 2. SCEM: what worked

SCEM produced a real deterministic CPU inference engine.

- ARC-AGI-1 training: 65/400.
- ARC-AGI-1 evaluation after five gated runs: 21/400.
- Exact replay verification was enforced.
- Pre-registration, split quarantine, content-addressed artifacts, negative-result reporting, and attribution discipline were strong.
- Engineered mechanisms transferred: 20 of the 21 evaluation solves were attributed to general inference machinery.
- In controlled synthetic settings, a correctly structured generative concept could bind task-local parameters and transfer perfectly.
- Active intervention resolved a hidden causal axis that passive data left ambiguous.
- A derived primitive improved effective search reach by compressing a long base program into one operation.

These are assets to retain: exact execution, causal testing where an environment is available, task-local binding, rigorous measurement, and proof-carrying artifacts.

## 3. SCEM: why the architecture failed

### 3.1 Stored content did not generalize

Measured evaluation transfer was:

- concrete mined atoms: 0;
- op-sequence schemas: 0;
- relational concepts: 1, within the measured noise band.

The learned artifacts mostly compressed the system's own solved traces. They did not create a new inference process at deployment.

### 3.2 Concepts became labels rather than executable causal models

The promoted relational concepts named patterns already reachable by the hand-built inducer. They did not alter binding, construction, search, or verification. A concept that cannot cause the solver to form a new candidate explanation is metadata, not intelligence.

### 3.3 Discovery only generalized along observed variation

The controlled H2 discovery experiment was decisive:

- broad training that varied every latent dimension: 38/38 held-out;
- narrow training that held critical dimensions constant: 0/38 held-out.

The miner inferred which existing fields were variable. It did not invent a missing variable or operator from an underdetermined distribution.

### 3.4 Fixed templates hit an expressivity ceiling

Among 335 unsolved training tasks, the largest reported buckets were:

- `inexpressible_pair`: 143;
- `created_unbindable`: 95;
- `selector_intersection_empty`: 34;
- `parameter_binding`: 28;
- `canvas_underivable`: 22.

Within `created_unbindable`, 78 tasks required variable-arity construction and the apparently simpler 16 fixed-arity candidates still had zero observably bindable templates. The missing behavior was an input-dependent derivation, not one more fixed offset or selector.

### 3.5 Primitive invention remained inside the base closure

H3.2 correctly showed useful invention: `PaintToEdge` compressed 174 base operations into 33 corpus-cost units and changed held-out within-budget performance from 0/40 to 40/40. It also correctly identified the boundary: this was a macro over existing primitives. It improved tractability, not expressive closure.

### 3.6 Active learning is only available when the environment can answer

H3.0 and H3.1 showed that actual interventions can resolve ambiguity. Static ARC-AGI-1/2 tasks do not provide an oracle for arbitrary synthetic inputs. Counterfactual tests can rank static-task hypotheses, but cannot manufacture missing evidence. The architecture therefore needs a strong structural prior and exact relational induction for static tasks, while using genuine intervention only for interactive domains.

## 4. MOSAIC: what worked

MOSAIC corrected several SCEM defects in its design and infrastructure.

- Multiple parse worlds instead of one committed segmentation.
- Constructive provenance.
- Variable-arity program forms in the design.
- Typed execution, deterministic certificates, and fixed-work search quanta.
- A sound version-space implementation.
- Strong CPU performance in low-level components.

Measured implementation results included:

- WP3 exact pair replay: 1,047/1,302, or 80.41%;
- WP3-R1 exact pair replay: 1,073/1,302, or 82.41%;
- WP5 version-space algebra acceptance: passed;
- X1: 11/60 tasks solved, with all generated candidates exhausted and no capacity limits reached.

The X1 result was not a compute failure. Median candidate frontier was 617 and the largest was 1,322. No task exhausted wall time, bank capacity, or goal capacity.

## 5. MOSAIC: why the implemented system failed

### 5.1 It still searched a fixed semantic closure

MOSAIC replaced SCEM's learned atoms with a fixed DSL and more sophisticated enumeration. That improves completeness inside the DSL but does not solve missing ontology, missing role variables, or missing constructors.

### 5.2 Perception and transformation were separated too early

The parse lattice generated worlds before knowing which causal theory had to be explained. This can produce many locally plausible objects while missing the entity definition required by the transformation. A segmentation should be justified by its explanatory role, not selected independently and then handed to synthesis.

### 5.3 Pair explanation did not become a multi-rule theory

WP3-R1 left 229 raw pair failures; every best world had exactly one unexplained output object. The dominant remaining categories were transform-plus-recolor patterns and multicolor construction. More importantly, X1 did not lower constructive provenance into multi-action canvas programs. It supported one global geometry with optional recolor, or one selector/action form.

The result was:

- all 7 pure D8 cases solved;
- 0/10 same-canvas object edits;
- 0/10 same-canvas palette-preserving tasks;
- 0/7 crop-recolor tasks;
- 0/3 expansion tasks.

This exposed the gap between explaining one pair locally and inducing a reusable, interacting rule system across examples.

### 5.4 More search would not have fixed X1

X1 exhaustively evaluated its implemented frontier. The failure state was predominantly `inexpressible`, not `timeout`. The next architecture must expand and restructure the hypothesis space during inference rather than only search it faster.

### 5.5 Cross-task learning was intentionally reduced to ordering

MOSAIC allowed learned policy weights to reorder search but not form new semantic candidates. This was safe, yet it could not accumulate new causal abstractions or improve the representational process itself.

## 6. Combined design requirements

A successor must satisfy all of the following at once:

1. Begin from observed causal effects rather than forward enumeration.
2. Infer perception and transformation jointly.
3. Represent variable-arity creation, merging, splitting, iteration, and interacting rules natively.
4. Support operator-valued latent variables for symbolic interpretation.
5. Learn contextual selectors from changed and unchanged entities.
6. Invent task-local predicates, roles, and constructors from failed explanations.
7. Ground every invented abstraction in a small universal kernel so it is executable and verifiable.
8. Use failed hypotheses and unsatisfied cores to prune or expand search.
9. Preserve exact replay and split discipline.
10. Extend to temporal world-model learning and active intervention without replacing the core.
11. Scale through heterogeneous agents that contribute different kinds of evidence, not merely copies of the same enumerator.
12. Remain fast on commodity CPUs.

CAIRN is designed around those requirements.
