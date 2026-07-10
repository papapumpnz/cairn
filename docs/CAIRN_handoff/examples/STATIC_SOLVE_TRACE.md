# Worked static solve trace

This is a synthetic, non-benchmark example. Its purpose is to specify the expected information flow and artifact boundaries for a compositional task. It is not an ARC score claim.

## 1. Task concept

Each input has two panels separated by a uniform divider.

The legend panel contains examples that assign meanings to two otherwise arbitrary tokens:

```text
token 2  means  mirror the payload across its vertical axis
token 3  means  rotate the payload clockwise by 90 degrees
```

The scene panel contains:

```text
one control token, either 2 or 3
several multicolor objects
one rectangular frame whose border color identifies the selected object
```

The output is the scene panel only. The object inside the frame is transformed by the operator denoted by the control token, recolored with the frame-border color, and written back at the same anchor. The frame and control token disappear. Other objects are preserved.

This task requires all of the following:

```text
panel and legend discovery
operator-valued symbol interpretation
context-dependent object selection
shape transform followed by recoloring
explicit deletion
output-canvas cropping
multi-rule scheduling
```

A one-operation global transform cannot express it.

## 2. Observation artifact

The frontend emits an immutable `ObservationSet` containing all demonstrations and the test input.

```text
ObservationSet
  train[0]: input grid I0, output grid O0
  train[1]: input grid I1, output grid O1
  train[2]: input grid I2, output grid O2
  test[0]:  input grid T0
```

Every grid receives:

```text
shape
color histogram
row and column run indexes
bitplanes by color
change masks for training pairs
stable content hash
```

No object segmentation is committed at this point.

## 3. Evidence fingerprinting

The evidence agent discovers a full-height single-color divider in each input. Removing it yields two maximal rectangular regions with repeated relative layout.

Candidate evidence:

```text
E_divider: maximal uniform separator
E_panel_legend: rectangle left of E_divider
E_panel_scene: rectangle right of E_divider
E_output_shape: equal to E_panel_scene shape in every demonstration
E_output_palette: subset of scene colors plus frame-border color
```

A renderer hypothesis is opened early:

```text
canvas := dimensions(E_panel_scene)
source coordinate frame := local coordinates of E_panel_scene
```

This avoids forcing a full-input canvas theory and lets event analysis ignore legend cells that are outside the output canvas while still retaining them as causal evidence.

## 4. Demand-driven entity generation

The event engine asks for entities that could explain preserved, deleted, or changed output regions.

The entity algebra produces, lazily:

```text
Panel(legend)
Panel(scene)
ConnectedComponent under 4-connectivity
MulticolorComponent under non-background connectivity
RectangularContour
InteriorOf(contour)
ContainedBy(entity, contour)
Token(singleton or compact monochrome glyph)
RepeatedMotifPair inside legend rows
```

The following entities are retained because they participate in low-cost candidate correspondences:

```text
L2, L3                 legend tokens
LA_in, LA_out          legend payload before and after token 2
LB_in, LB_out          legend payload before and after token 3
C                     scene control token
F                     scene frame
P_selected             multicolor object contained by F
P_other[*]              remaining scene objects
```

Alternative worlds remain available, including color components and whole-frame-plus-payload entities. They are not all expanded to closure.

## 5. Pair-local event hypotheses

For each training pair, source and target entities are compared under color-normalized and transformed shape signatures.

High-scoring event hypotheses include:

```text
Preserve(P_other[k] -> P_other'[k])
Delete(C)
Delete(F.border)
Transform(P_selected -> P_selected', operator = unknown)
Recolor(P_selected' -> frame_color(F))
Crop(scene_panel -> output_canvas)
```

The transform event's parameter is not yet a literal. Its candidate domain is:

```text
identity
D4 transforms
small integer scale transforms when dimension evidence permits
operator denoted by an observed symbol
```

The selected payload is not defined as “the first object” or “the object of color x.” Candidate binders include:

```text
contained_by(P, F)
inside(interior(F), cells(P))
unique object whose bounding box is inside F
nearest object to C
matching color to F
```

## 6. Event-cover optimization

The event-cover solver chooses an exact cover for each demonstration.

Hard checks:

```text
every output cell has one justified writer
every changed input scene cell is consumed or deleted
every preserved object is mapped exactly
no unmodeled frame cell survives
all transformed payload cells land inside the output canvas
rendered grid equals the observed output byte-for-byte
```

A representative winning cover is:

```text
1. crop the scene panel into a fresh canvas
2. preserve all scene cells except control, frame, and selected payload
3. delete control
4. delete frame border
5. replace selected payload with transformed, recolored payload
```

The cover solver keeps other exact covers when their causal decomposition differs, for example “erase then paint” versus “replace entity.” These become separate structural modes.

## 7. Legend interpretation

Within the legend panel, the engine finds two local input-output motif relations:

```text
normalize_color(LA_in) --MirrorVertical--> normalize_color(LA_out)
normalize_color(LB_in) --Rotate90CW-----> normalize_color(LB_out)
```

The tokens adjacent to those motif relations covary with which transform is demonstrated. A symbol-map sketch is created:

```text
meaning : Symbol -> Transform2
meaning(L2) = MirrorVertical
meaning(L3) = Rotate90CW
```

The map is grounded in observable legend structure. It is not a global convention about colors 2 and 3. On another task, the same colors may mean something else.

Competing interpretations are retained briefly:

```text
token means transform
token means output orientation
token means placement direction
```

Only the transform interpretation composes with all scene event covers without extra constants.

## 8. Cross-demonstration role alignment

The analogical aligner maps event participants by relational role rather than by appearance.

```text
Demo 0 frame F0  <-> Demo 1 frame F1  <-> Demo 2 frame F2
Demo 0 payload P0 <-> Demo 1 payload P1 <-> Demo 2 payload P2
Demo 0 control C0 <-> Demo 1 control C1 <-> Demo 2 control C2
```

Preserved relations:

```text
ContainedBy(Pi, Fi)
UniqueControlToken(Ci, ScenePaneli)
Meaning(Ci) = operator used by Transform event
PaintColor = BorderColor(Fi)
Anchor(output payload) = Anchor(input payload)
```

Color, absolute coordinate, payload area, and shape differ across demonstrations and are therefore treated as role-bound variables.

## 9. Anti-unified rule sketch

The aligned covers produce the following sketch:

```text
role scene := panel whose dimensions match output
role control := unique scene token whose value is in legend symbol domain
role frame := unique rectangular contour containing a multicolor entity
role payload := unique entity contained by frame
role op := interpret(control, legend)
role paint := border_color(frame)

rule R_transform:
  for payload P
  when selected_by_frame(P, frame)
  replace P with recolor(apply(op, P), paint) at anchor(P)

rule R_delete_control:
  delete control

rule R_delete_frame:
  delete frame.border

rule R_preserve:
  preserve all remaining scene cells

schedule:
  construct fresh scene-sized canvas
  preserve background and unaffected entities
  execute R_transform
  omit control and frame
```

Typed holes remain for the exact frame selector, symbol-map extraction, anchor convention, and render conflict policy.

## 10. Joint constraint problem

The theory weaver sends one joint finite-domain problem rather than solving each hole independently.

Representative variables:

```text
panel_scene_selector
frame_predicate
payload_predicate
control_predicate
symbol_map_mode
transform_anchor
paint_function
rule_schedule
conflict_policy
```

Representative constraints:

```text
for every demonstration there is exactly one scene panel
for every demonstration there is exactly one bound frame
for every demonstration there is exactly one payload contained by that frame
control token occurs in the legend symbol domain
symbol meaning equals the observed payload transform
recolored payload cells equal frame border color
all unaffected cells equal their input-scene value
control and frame cells are absent
final dimensions equal output dimensions
```

An incremental SAT or finite-domain backend returns a model. The executor lowers it to cell mutations and compares every output cell.

## 11. First counterexample and CEGIS refinement

Suppose the first candidate uses:

```text
frame := largest rectangular contour
```

It fits demonstrations 0 and 1 but fails demonstration 2, which contains a larger decorative rectangle not enclosing an object.

The verifier emits a minimized counterexample:

```text
candidate binding F_decorative satisfies Rectangle and Largest
F_decorative contains no payload
observed changed region lies inside F_small
```

The constraint learner adds:

```text
frame must contain exactly one eligible multicolor payload
```

This removes all theories using size alone without banning `ArgMax` globally.

The revised binder is:

```text
Unique(
  Filter(
    RectangularContours(scene),
    lambda F: Count(EligiblePayloadsInside(F)) == 1
  )
)
```

## 12. Ontology invention case

Assume the base predicate vocabulary contains `Inside`, `Touches`, and `BBox`, but not “strictly enclosed without sharing border cells.” Two candidate objects may both have bounding boxes inside the frame, while only one is topologically enclosed.

The failure core is:

```text
positive and negative candidates are indistinguishable under current selector features
```

The predicate inventor searches relational paths and aggregates and proposes:

```text
StrictlyEnclosed(P, F) :=
  Inside(Cells(P), Interior(F))
  and not Any(cell in Cells(P), Adjacent(cell, Outside(F)))
```

Admission requires:

```text
separates every positive and negative binding
uses only observable entities and RKL relations
is total on bounded grids
reduces the unsatisfied core
improves description length over literal coordinates
survives leave-one-demonstration-out reconstruction
```

The definition is task-local. It is not persisted as a trusted global primitive until cross-task promotion gates are met.

## 13. Exact theory execution

The accepted theory executes as follows on each demonstration and then the test input:

```text
1. identify legend and scene panels
2. derive the task-local Symbol -> Transform2 map
3. bind scene control, frame, and payload
4. allocate scene-sized output canvas
5. copy unaffected scene cells
6. apply the interpreted transform about the payload-local anchor
7. recolor transformed cells with frame border color
8. paint transformed payload
9. leave control and frame unpainted
10. verify bounds, conflicts, and totality
```

Every painted cell records:

```text
rule ID
source entity or constructor
source coordinate when applicable
transform
paint function
layer
proof node
```

## 14. Falsification

The theory must pass more than training replay.

### Leave-one-demonstration-out

For each held-out training pair, derive the theory from the remaining pairs and predict the held-out output. A theory that relies on a constant unique to the held-out pair fails.

### Equivariance re-derivation

Apply a task-valid translation or color permutation to all demonstrations, re-run induction, invert the output transform, and compare. The test is applied only when the transformation preserves task semantics.

### Irrelevance perturbation

Insert a distant object that does not satisfy the payload selector. The selected payload and operator mapping should remain stable. This is a ranking test, not fabricated evidence.

### Decomposition consistency

Re-render the output from the event cover and from the final theory independently. Both cell-provenance maps must agree or have a formally equivalent layering explanation.

## 15. Posterior modes and two attempts

After exact fit and falsification, two semantic modes may survive:

```text
Mode A: transform around the payload bounding-box center
Mode B: transform around the frame interior center
```

If all demonstrations make those anchors coincide but the test does not, the system must not return two syntactic variants of the same mode. It emits one answer from each distinct semantic mode, ordered by structural score and cross-task core-knowledge prior.

Each attempt has an independently replayable certificate.

## 16. Required trace metrics

A compliant implementation emits at least:

```text
entities requested and retained by family
event hypotheses by kind
exact and partial event covers
alignment modes and relation-preservation scores
theory sketches generated
constraint variables, clauses, conflicts, and models
executed theories and semantic dedup rate
counterexample classes
invented definitions and admission reports
falsification scores
wall time, CPU time, peak RSS, and work units by stage
```

A terminal result distinguishes:

```text
solved
ambiguous
unsupported semantics
capacity stop
timeout
verification defect
```
