# ADR-0006: Finite-field schema induction frontier

Status: accepted

Date: 2026-07-11

Owners: integration owner

## Context

The first official ARC-AGI-1 development batch exposed that unrestricted
pointwise RKL expression search could neither construct changed output geometry
nor reach useful relational theories under its candidate cap. The initial ten
tasks required recurring mechanisms that are meaningful beyond ARC: bounded
tensor construction, enclosed-region filling, periodic continuation, entity
attraction, panel-relative comparison, span completion, component ranking,
boundary-anchored deformation, and template completion along directional rays.

Adding branches keyed by benchmark task or filename would violate ADR-0003 and
the ARC quarantine. Treating these mechanisms as accepted merely because they
fit one ARC task would also collapse CAIRN toward a handcrafted solver.

## Decision

`cairn-field` owns a bounded schema-induction frontier. A schema is an
environment-neutral, parameterized executable field mechanism. The learner:

1. derives candidate parameters from context/outcome demonstrations;
2. executes every candidate over every demonstration;
3. admits only exact full-field replay;
4. freezes the selected mechanism before receiving test outputs;
5. applies deterministic tie-breaking and explicit failure classes.

The initial frontier contains:

- tensor mask construction;
- enclosed-region filling;
- rectangular periodic tiling and diagonal-phase completion;
- directional gravity and attraction to a persistent entity;
- background-only spans and separator-defined panel spans;
- component-size ranking;
- panel intersection;
- periodic row extrapolation with categorical rebinding;
- boundary-anchored shear;
- template completion and repeated directional placement.

Every mechanism has a generated, adapter-free `cairn-field` test. ARC task IDs,
filenames, split names, and expected test outputs are absent from the schema API.

## Consequences

The frozen first ten ARC-AGI-1 training tasks now produce ten exact predictions
under the same 1,000-candidate fallback cap. This is a development-set result:
the mechanisms were added after analyzing those ten tasks. It proves execution,
induction, and quarantine behavior, not unseen ARC generalization.

A strictly untouched follow-on probe of tasks 11–20 produced zero exact
predictions: two unsupported shape changes and eight candidate-cap outcomes.
That negative result is retained. No broad ARC capability or general
intelligence claim follows from the first-ten pass.

Schema growth must remain governed by synthetic and non-ARC evidence. Before a
schema is promoted into a general cognition layer it requires cross-domain
representation, ablation, and memory-transfer evidence. `cairn-field` remains a
frontend/synthesis service, not an acceptance authority.

## Validation

- thirteen generated schema-learning tests cover the admitted frontier;
- every frozen first-ten candidate replays all demonstrations exactly;
- test outputs are stripped before learning and exposed only during scoring;
- the first-ten result is 10/10 exact with no incorrect or capped tasks;
- the untouched tasks 11–20 remain 0/10 and are recorded separately;
- the full workspace, policy checker, schema validator, and corpus hash manifest
  must pass before handoff.

## Compatibility

ARC adapter semantics advance from 1.0.0 to 1.1.0. RKL remains 1.1.0 because no
kernel syntax or execution semantics changed.
