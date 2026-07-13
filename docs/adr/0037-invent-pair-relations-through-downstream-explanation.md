# ADR-0037 — Invent pair relations through downstream explanation

Status: accepted

## Context

ADR-0035 constructs collections from observed relation edges. The primitive
relation predicates and pair labels remain supplied, so the learner cannot
decide which entity attribute defines collection membership.

## Decision

Add attributed grouping evidence containing opaque entity identity, scalar
payload, and a bounded vector of generic integer features. It contains no pair
labels, relation edges, partitions, or selected feature.

Inside the typed pairwise-relation hole, enumerate executable RKL equality
definitions over corresponding feature slots. For each definition:

1. execute it on every unordered entity pair;
2. construct the complete connected components;
3. invoke ordinary unordered group-merge invention; and
4. retain only a minimum-cost unambiguous relation/merge theory explaining all
   outputs.

The learned identity binds both executable definitions. The independent
verifier separately executes the pair predicate, reconstructs components, and
replays the aggregate. Enumeration remains local to a typed hole and does not
become global syntax search.

## Alternatives rejected

- Derive pair labels from the expected grouping: would supply the ontology
  answer indirectly.
- Persist a selected feature index as the mechanism: stores a configuration,
  not an executable semantic definition.
- Add named grouping operations: creates an ARC-like operation catalogue.

## Compatibility and migration

The additive canonical mechanism marker is
`cairn.predicate-grouping-merge.v1`. Existing raw-edge mechanisms remain valid.
No automatic migration exists because they do not contain the evidence needed
to infer an underlying predicate.

## Validation

Require two independently named cohorts, distractor attributes, changed feature
values and entity identities, variable component count/cardinality, independent
replay, entity-order invariance, feature-slot ablation, and exact cross-domain
held-outs. No relation edge or pair label may enter the test learner.

## Rollback and kill conditions

Revert if a feature/task name affects selection, if an altered relation body
retains identity, if entity ordering changes semantics, or if the verifier uses
producer-generated components.
