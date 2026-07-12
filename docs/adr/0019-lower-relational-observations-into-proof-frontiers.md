# ADR-0019: Lower relational observations into learned proof construction

Status: accepted

Date: 2026-07-12

Owners: WP02/WP23/WP34 integration

## Context

ADR-0018 removed completed proof hosts from learned composition, but its
materializer still assembled `ProofFrontier` nodes directly. The stable
`Observation` contract authenticates an opaque canonical payload and therefore
does not yet provide a modality-neutral, verifier-replayable route from observed
entities and relations to proof evidence. Its first correspondence policy also
retains typed ordinals, which can transfer across changed values but not changed
element order or relational organization.

The next experiment must connect structured static evidence and a newly learned
action-conditioned world law to the same cognition constructor without adding
adapter checks or allowing expected proof into the producer.

## Decision

Add `RelationalObservation` to `cairn-core`. It wraps an ordinary authenticated
`Observation` whose payload is canonically derived from sorted typed evidence
elements and ordered-arity relation evidence. Elements carry opaque identities,
typed opaque values, and no domain semantics. Relations carry opaque predicates
and participant identities. Construction validates uniqueness, arity,
referential integrity, certainty, source provenance, and canonical identity.

Add a bounded reference lowerer in `cairn-consolidator`. It converts every
validated relational observation to a deterministic `ProofFrontier`: one
observation root, one typed element node per canonical element, and one relation
node whose ordered inputs reference its participants. It returns explicit node
provenance and a canonical lowering commitment. No expected output or learned
operation enters this path.

Add an independent verifier lowerer. It reconstructs the frontier and
commitment without calling producer lowering code and emits an opaque
`VerifiedObservationFrontier` only on exact equality.

Add `RelationalConstructionPolicy`. From one independently verified training
construction, the consolidator locates each bound value in observation evidence
and records its type plus a bounded two-round relational neighbourhood
fingerprint. The fingerprint uses element types, relation predicates, argument
positions, and neighbouring fingerprints; it excludes element IDs, values,
modality, source, node order, expected proof, and task identity. Application
requires each selector to match exactly one changed element and emits one
untrusted construction candidate. Ambiguous or missing matches fail closed.

The shared `CognitionRuntime` exposes validation/lowering and relational-policy
application while retaining the persistent-mechanism gate. Static and
action-conditioned frontends may create relational observations, but no domain
name or operation enters cognition.

## Alternatives considered

### Decode each modality payload in cognition

Benefits: no new observation schema.

Costs: adapter branches enter the core and cannot share exact semantics.

Reason rejected: violates the cross-domain cognition boundary.

### Continue using typed ordinals

Benefits: already implemented and inexpensive.

Costs: element reorderings and distractors alter meaning.

Reason rejected: it does not learn relational correspondence.

### Store natural-language role names

Benefits: easy debugging.

Costs: supplied semantic labels become the reason transfer succeeds.

Reason rejected: predicates remain opaque hashes and structural evidence must
determine applicability.

## Consequences

Correctness: the observation, lowering, construction, and held-out proof each
have separate validation boundaries.

Expressivity: exact structured entities, values, and finite relations are
supported. Raw pixels/audio/text, uncertain entity competition, and recursive
relation paths remain open.

CPU cost: canonical sorting is `O((E + R) log(E + R))`; two-round structural
fingerprinting is bounded by evidence size and relation arity.

Memory cost: observations store exact finite evidence; policies store hashes and
small selector vectors, never source values or answer graphs.

Determinism: canonical element/relation order and hash/tie rules are explicit.

Verifier impact: the verifier independently reconstructs lowering before any
constructed proof can count.

Downstream interfaces: additive core, consolidator, runtime, verifier, and WP34
experiment APIs. Existing observation and persisted mechanism semantics remain
valid.

## Validation

Tests: canonical order invariance, duplicate/missing participant/empty relation
rejection, changed payload detection, independent lowering replay, altered
frontier rejection, relational reorder/distractor transfer, ambiguous/missing
fingerprint rejection, blank memory, and expected-proof isolation.

Benchmark gate: learn correspondence on one non-interactive structured
observation; learn previously unknown action effects through `WorldLearner`;
materialize those verified effects as a changed relational observation; apply
the same persistent mechanisms and relational policy once; independently verify
the exact constructed proof. Memory/policy ablations must fail or require more
work, and no ARC package may be linked.

Rollback trigger: producer lowering consumes modality names, values/IDs enter a
selector fingerprint, ambiguous selection silently tie-breaks, expected proof
crosses into runtime, or the verifier cannot reproduce lowering independently.

## Compatibility

Semantics version change: additive `cairn.relational-observation.v1` payload and
`cairn.verified-observation-frontier.v1` certificate markers; no existing schema
is reinterpreted.

Migration: none. Existing opaque observations remain valid but are not eligible
for this lowering path unless explicitly compiled into the new envelope.

## References

- ADR-0018 evidence-frontier construction
- ADR-0007 self-compressing mechanism learner
- `08_GENERAL_LEARNER_REQUIREMENTS.md`
