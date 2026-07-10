# Worked interactive solve trace

This synthetic trace specifies how the same CAIRN theory machinery extends from static grid transformations to interactive reasoning. It is not an ARC-AGI-3 result.

## 1. Environment sketch

A small grid world contains:

```text
avatar
movable crate
floor switch
closed door
key
exit tile
walls
```

The agent receives only frames and an action set:

```text
MoveNorth
MoveSouth
MoveWest
MoveEast
Interact
```

No natural-language goal is supplied. A positive terminal signal occurs only after reaching the exit under the correct latent conditions.

## 2. Initial relational state

The temporal frontend converts frame 0 into a `RelationalEvidenceGraph`.

Entities:

```text
A0 avatar
C0 crate
S0 switch
D0 door
K0 key
X0 exit
W[*] walls
```

Relations:

```text
At(A0, p_a)
At(C0, p_c)
At(S0, p_s)
At(D0, p_d)
At(K0, p_k)
At(X0, p_x)
BlockedBy paths and wall adjacency
```

Object identities are provisional tracks. Track confidence is revised when movement evidence arrives.

## 3. First safe action and event extraction

The planner chooses a low-cost movement into an empty neighboring cell.

Observed transition:

```text
At(A, p) -> At(A, p + east)
all other visible entities preserved
```

Event cover:

```text
Move(A, East, one cell)
Preserve(C, S, D, K, X, W)
```

Anti-unification with later free movement induces:

```text
R_move:
  when destination is traversable
  Move(avatar, direction, one cell)
```

A failed movement into a wall adds the negative guard:

```text
not OccupiedByWall(destination)
```

## 4. Competing door theories

After approaching the closed door, `MoveEast` produces no avatar movement.

Initial theory modes:

```text
H1: door is always impassable
H2: carrying the key makes the door passable
H3: depressing the switch makes the door passable
H4: Interact adjacent to the door opens it
H5: crate position controls the door through an unseen mechanism
```

These are executable hypotheses, not text labels. Each predicts observations for candidate actions.

## 5. Goal hypotheses

Before any reward, the goal learner proposes low-complexity candidates from salient structure:

```text
G1: reach exit tile
G2: collect key
G3: open door
G4: place crate on switch
```

The system treats these as a hierarchy rather than mutually exclusive final goals. `collect key`, `open door`, and `place crate on switch` may be instrumental subgoals for `reach exit`.

Goal evidence includes:

```text
unique terminal-like tile
regions inaccessible behind door
key and switch affordance motifs
reset or positive-signal observations when eventually available
progress features that persist
```

## 6. Intervention selection

The system compares possible actions by simulating the current top world-theory modes.

For action `Interact` beside the door:

```text
H1 predicts no change
H2 predicts no change without key
H3 predicts no change without switch
H4 predicts door opens
H5 predicts no change
```

For action sequence “push crate onto switch”:

```text
H1 predicts door unchanged
H2 predicts door unchanged
H3 predicts door opens
H4 predicts door unchanged
H5 predicts a door effect dependent on crate placement
```

For action sequence “collect key”:

```text
H1 predicts key disappears but door remains blocked
H2 predicts key carried and door becomes passable
H3 predicts key carried but door remains blocked
H4 predicts key carried but door still requires Interact
H5 predicts no door effect
```

CAIRN evaluates:

```text
EVI(action) =
  expected belief entropy reduction
  + expected goal progress
  - action cost
  - irreversible-risk penalty
```

The entropy term uses belief-mass-weighted observation partitions. Counting distinct outcomes without their probabilities is forbidden.

If collecting the key is short, safe, and useful under multiple hypotheses, it can outrank a pure information probe even when both split the theory set equally.

## 7. Belief update after key collection

Observed transition:

```text
avatar enters key cell
key disappears from frame
inventory-like persistent effect is not directly visible
```

The event model introduces a latent state only because a persistent cause is needed to explain later behavior:

```text
HasKey : Bool

R_collect:
  when avatar enters key cell
  SetLatentState(HasKey, true)
  Delete(key)
```

The latent variable is grounded by a visible causal event and future predictions. The system may not assign an arbitrary latent ID to every time step.

## 8. Counterexample and theory revision

The agent returns to the door and attempts to cross. Suppose the door remains closed.

This is an exact counterexample to the simple version of H2:

```text
HasKey = true
avatar adjacent to door
attempted transition through door
predicted passable
observed no movement
```

The CEGIS learner rules out:

```text
Passable(door) := HasKey
```

but preserves refinements such as:

```text
Passable(door) := HasKey and DoorActivated
Passable(door) := HasKey and action == Interact
```

The failed prediction also increases the value of testing the switch or interacting with the door.

## 9. Switch intervention and causal rule

The agent pushes the crate onto the floor switch. The next frame shows the door changed to an open appearance.

Event cover:

```text
Move(crate, onto switch)
UpdateAttribute(door, closed -> open)
Preserve(other entities)
```

Candidate causal links are ranked:

```text
crate moved
crate occupies switch
avatar moved near switch
time elapsed
```

Across a confirming removal/replacement sequence, only occupancy of the switch tracks the door state. Anti-unification yields:

```text
R_switch_door:
  DoorOpen := Exists(crate, At(crate, switch_position))
```

If door openness persists after the crate moves away, a latent latch is introduced instead:

```text
when crate first occupies switch:
  DoorActivated := true

door open iff DoorActivated
```

The distinction is established by intervention, not guessed from naming.

## 10. Planning with a learned world theory

The current theory includes:

```text
movement and collision rules
crate push rules
key collection rule
switch-door causal rule
door traversal rule
candidate terminal goal Reach(exit)
```

The planner first searches over induced options:

```text
AcquireKey
PlaceCrateOnSwitch
OpenDoor
TraverseDoor
ReachExit
```

Each option has:

```text
initiation predicate
policy or subplanner
termination predicate
predicted effects
failure conditions
cost distribution
```

An option-level A* search finds a candidate sequence. Primitive-level A* validates navigation and crate pushes under current obstacles. Bounded MCTS is reserved for residual uncertainty or stochastic-looking transitions.

## 11. Execution monitoring

Before every action, the expected next-state relation delta is recorded.

```text
expected:
  avatar position changes
  door remains open
  crate and switch relation preserved

observed:
  exact match
```

A mismatch triggers immediate belief revision and local replanning. The system does not continue a stale open-loop plan.

## 12. Goal confirmation

Upon entering the exit tile, a terminal positive signal occurs.

The goal learner updates:

```text
P(final goal = Reach(exit)) -> dominant
P(key collection = final goal) -> low
P(open door = final goal) -> low
```

It records the latter as instrumental predicates in the successful plan proof.

## 13. Context compression

The agent need not retain every raw frame indefinitely. It stores:

```text
current relational state
tracked entities and confidence
learned transition rules
latent-state belief
goal-hypothesis belief
counterexamples
option graph
selected high-value snapshots
proof links to replay data
```

Raw replay frames remain in the content-addressed store for independent audit.

## 14. Swarm execution

On a multi-process CPU swarm:

```text
one agent tracks entities and event covers
one agent induces movement and collision rules
one agent explores object-affordance hypotheses
one agent evaluates goal hypotheses
one agent simulates intervention partitions
one agent searches plans
one independent verifier replays predictions
```

Agents exchange canonical artifacts rather than mutable shared objects. The broker deduplicates equivalent theories by semantic hashes.

Only the action arbiter may commit an environment action. It selects from verified proposals under the current safety, cost, and information-value policy.

## 15. Required interactive metrics

```text
actions to first correct goal hypothesis
actions to stable transition model
prediction accuracy by event kind
belief entropy over time
information gain per action
irreversible failures
replans after prediction error
option reuse
primitive and option-level search nodes
total environment actions
CPU time per decision
peak memory
```

The important metric is not merely eventual success. It is skill-acquisition and action efficiency under novel dynamics.
