# RKL and Theory IR examples

These examples are normative sketches for semantics and tests. Concrete parser syntax may change through an ADR, but the type, grounding, provenance, and totality requirements may not.

## 1. Contextual predicate invention

Objective: select an object strictly enclosed by a contour.

```text
def StrictlyEnclosed(p: Entity, frame: Entity) -> Bool =
  IsRectangularContour(frame)
  and Subset(Cells(p), InteriorCells(frame))
  and not Any(
    c in Cells(p),
    Adjacent4(c, Complement(InteriorCells(frame)))
  )
```

Admission obligations:

```text
all referenced cells derive from the current observation
InteriorCells is total for all bounded entities
positive and negative bindings are separated
no absolute coordinate or task identifier occurs
proof records the evidence core this definition resolves
```

## 2. Variable-arity constructor

Objective: repeat a motif once for every marker, preserving marker order along an inferred axis.

```text
def EmitForMarkers(
  motif: Entity,
  markers: EntitySet,
  axis: Axis,
  gap: Int
) -> CellSet =
  Fold(
    SortBy(markers, Project(Centroid, axis)),
    EmptyCellSet,
    lambda acc, marker, index:
      Union(
        acc,
        Translate(
          NormalizeToOrigin(Cells(motif)),
          PlacementFrom(marker, index, axis, gap)
        )
      )
  )
```

The constructor is variable-arity because the fold domain is input-derived. Its bound is `Count(markers)`, which is finite and observable.

## 3. Operator-valued symbol map

Objective: infer a task-local map from legend tokens to geometric transforms.

```text
def InterpretTransform(
  token: Symbol,
  legend_rows: EntitySet
) -> Option<Transform2> =
  Unique(
    Map(
      Filter(legend_rows, lambda row: RowToken(row) == token),
      lambda row:
        UniqueTransform(
          NormalizeColor(RowSourceMotif(row)),
          NormalizeColor(RowTargetMotif(row))
        )
    )
  )
```

`UniqueTransform` returns `None` when no transform or multiple transforms fit. Ambiguity remains explicit and cannot silently default to identity.

## 4. Multi-rule theory with dependency schedule

```text
theory LegendControlledRewrite {
  roles {
    scene   := Unique(ScenePanelCandidates(input))
    legend  := Unique(LegendPanelCandidates(input))
    control := Unique(ControlTokens(scene, SymbolDomain(legend)))
    frame   := Unique(Filter(RectangularContours(scene), ContainsEligiblePayload))
    payload := Unique(Filter(EntitiesInside(frame), EligiblePayload))
    op      := Require(InterpretTransform(control, LegendRows(legend)))
    paint   := BorderColor(frame)
  }

  rule erase_control {
    guard true
    effects Erase(Cells(control))
  }

  rule erase_frame {
    guard true
    effects Erase(BorderCells(frame))
  }

  rule rewrite_payload {
    guard StrictlyEnclosed(payload, frame)
    effects Replace(
      Cells(payload),
      Recolor(
        ApplyTransformLocal(Cells(payload), op, BBoxAnchor(payload)),
        paint
      )
    )
  }

  schedule DependencyDag {
    allocate_scene_canvas -> copy_unaffected
    copy_unaffected -> erase_control
    copy_unaffected -> erase_frame
    erase_frame -> rewrite_payload
  }

  renderer {
    canvas Dimensions(scene)
    layers [copied_scene, rewritten_payload]
    conflict_policy reject
    background BackgroundColor(scene)
  }
}
```

## 5. Bounded recursive abbreviation

Objective: extend a ray until it reaches the first obstacle or boundary.

```text
def RayUntilBlocked(
  start: Coord2,
  direction: Vector2,
  blockers: CellSet,
  bounds: Rect
) -> CellSet =
  BoundedFixpoint(
    state = { tip: start, cells: EmptyCellSet },
    max_steps = Area(bounds),
    step = lambda state:
      let next = state.tip + direction in
      if not InBounds(next, bounds) or Contains(blockers, next)
      then Stop(state)
      else Continue({ tip: next, cells: Insert(state.cells, next) }),
    progress_measure = Count(state.cells)
  ).cells
```

The progress measure increases monotonically and the step bound is derived from the canvas area. The verifier also compares the optimized implementation against an unrolled reference.

## 6. Event-cover constraint sketch

```text
variable use_event[e] : Bool

for each output cell q:
  Sum(use_event[e] for e where q in e.covered_output_cells) == 1

for each changed input cell p:
  Sum(use_event[e] for e where p in e.covered_input_cells) >= 1

for each incompatible pair e1, e2:
  not (use_event[e1] and use_event[e2])

objective lexicographic:
  minimize unexplained_regions
  minimize total_description_cost
  minimize literal_constants
  maximize repeated_event_form_reuse
```

Layered writers replace the equality with an explicit renderer-order constraint; they are never accepted as accidental overlap.

## 7. Failure-derived constraint

Candidate failure:

```text
selector chooses the largest rectangle, but the changed target lies in a smaller rectangle containing a payload
```

Learned constraint at the current sketch level:

```text
SelectedFrame must satisfy Exists(EligiblePayloadInside(SelectedFrame))
```

Forbidden overgeneralization:

```text
never choose the largest rectangle
```

The former removes the failed causal explanation. The latter incorrectly prunes future tasks where size is the true selector.

## 8. Support geometry for memory promotion

A reusable definition records which input dimensions were exercised.

```text
support_geometry {
  varied:
    payload_color
    frame_color
    payload_shape
    payload_area
    absolute_position
    frame_size
  held_constant:
    connectivity = four
  unobserved:
    multiple_nested_frames
    touching_frame_border
    empty_frame
}
```

At reuse time, predictions along `unobserved` axes receive uncertainty or trigger task-local re-derivation. They are not treated as proven universal behavior.

## 9. World transition rule

```text
rule PushCrate {
  bind avatar A
  bind crate C
  bind direction D

  guard
    AdjacentInDirection(A, C, D)
    and Traversable(Position(C) + D)

  effects
    Move(C, D)
    Move(A, D)

  priority 0
}
```

A failed push into a wall supplies a negative binding for `Traversable`, while a successful push supplies positive bindings. The rule remains grounded in visible entities and action history.

## 10. Hidden-state rule

```text
state DoorActivated : Bool = false

rule ActivateDoor {
  guard Occupies(crate, switch)
  effects SetLatentState(DoorActivated, true)
}

rule DoorAppearance {
  guard DoorActivated
  effects UpdateAttribute(door, appearance, open)
}
```

The latent state is admissible only when persistence after switch release is observed or required by a verified prediction. A visible-state rule is preferred when it explains the same evidence.

## 11. Pass@2 semantic separation

Two candidate theories are different posterior modes only when they can produce different outputs on some bounded observation satisfying their shared assumptions.

```text
Mode A anchor := BBoxCenter(payload)
Mode B anchor := InteriorCenter(frame)
```

Two ASTs differing only by algebraic normalization are merged in the e-graph and cannot consume both attempts.

## 12. Terminal-state examples

```text
Solved {
  attempts: [verified_output_0, optional_verified_output_1]
}

Ambiguous {
  surviving_modes: 7
  emitted_attempts: 2
  discrimination_gap: 0.13
}

UnsupportedSemantics {
  core: constructor_missing
  minimized_evidence: artifact_id
}

CapacityStop {
  cap: max_egraph_nodes
  consumed: 1000000
  frontier_checkpoint: artifact_id
}

Timeout {
  wall_ms: 120000
  work_units: 48299312
  frontier_checkpoint: artifact_id
}
```

Capacity and timeout are not reported as proof that no theory exists.
