# Cross-frontend entity-role transfer

Date: 2026-07-12

## Hypothesis

A relational selector distilled from offline, non-ARC teaching episodes should
remain useful when rebound to a frontend that was absent from the teaching
packet. The learned predicate must remain unchanged; the new frontend may only
emit typed entity evidence and reconstruct a selected entity from the current
observation.

## Teacher and invention boundary

The existing offline Anthropic-shaped packet contains software-module,
document-section, and vector-diagram collections. CAIRN invents the conjunction
of maximum occupancy and maximum diversity from the first two domains and
checks it on the held-out third domain. The packet contains no grids, ARC task
identifiers, colours, coordinates, or answers.

This experiment adds an exact connected-entity frontend. It emits the same two
relative structural attributes for each connected entity. A verified selector
is rebound by feature contract, and a unique selected entity is rendered from
its current native bounding frame. The binding stores neither a field nor an
output. Zero or multiple selections fail closed.

## Causal ablation

On frozen ARC-AGI-1 training task `be94b721`, the blank cognition runtime does
not produce a theory within the fixed budget. Supplying only the offline
AI-distilled selector through the connected-entity frontend produces an exact
test prediction after all demonstrations replay. Removing the binding removes
the solve.

The complete applicability audit admitted four structural bindings:
`09629e4f`, `1cf80156`, `2dc579da`, and `be94b721`. All four predictions were
exact; there were no false acceptances.

## Complete frozen result

Corpus: 400 filename-sorted ARC-AGI-1 v1.0.2 training tasks. Candidate budget:
1,000 per task.

| Condition | Exact | Resource-limited | Incorrect |
|---|---:|---:|---:|
| Blank structural memory | 32/400 (8.00%) | 368 | 0 |
| Prior structural bindings | 35/400 (8.75%) | 365 | 0 |
| Plus cross-frontend entity-role rebinding | 36/400 (9.00%) | 364 | 0 |

The final run completed in 467.20 seconds on the current development machine.
It is not used as a speedup claim because execution conditions were not a
pre-registered paired timing experiment.

## Claim boundary

This is positive evidence for distilled cross-domain and cross-frontend reuse:
one predicate learned from three non-ARC domains caused an additional frozen
ARC solve through a fourth frontend. It is one narrow mechanism family, not
general intelligence. ARC-1 remains 284 exact tasks short of the 80% target.
