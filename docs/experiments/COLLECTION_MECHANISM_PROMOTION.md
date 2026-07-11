# Higher-order collection mechanism promotion

Date: 2026-07-11

Status: canonical whole-theory persistence pass

## Protocol

Three independent collection traces contain aligned input/output elements with
cardinalities two, four, and five. The learner is given no task-specific map,
doubling, increment, or fixed-cardinality operator.

It first synthesizes the scalar element definition from all correspondences.
Because cardinality varies, the collection invention route lifts that definition
into an RKL rewrite rule with a `ForEach` binder over the runtime collection.
The complete theory includes its learned definition, quantified rule, schedule,
renderer, and resource-bounded execution semantics.

Two separately generated trace families induce the same checked theory. The
strict verifier binds promotion evidence to the canonical bytes of the whole
theory. `ProceduralMechanismStore` refuses theories whose canonical hash differs
from the verifier token, and `CognitionRuntime` now owns this procedural memory
alongside scalar definition memory.

## Invented program

The learned scalar body is equivalent to:

    transform(item) := item + item + 1

The learner then constructs:

    for each item in collection:
        emit transform(item)

The second line is a checked quantified rule, not a host-language loop or an
unrolled action sequence.

## Result

    observed cardinalities:          2, 4, 5
    scalar candidates evaluated:        395
    scalar structural cost:                5
    independent source families:           2
    canonical whole theories equal:       yes
    procedural mechanisms persisted:       1
    candidates without/with memory:    395/1
    examples without/with memory:        11/2
    held-out cardinality:                   7
    quantified rule firings:                7
    held-out elements exact:              7/7
    altered-theory token reuse:      rejected

Command:

    cargo test --locked -p cairn-theory --test collection_invention -- --nocapture

## Architectural change

Scalar `MechanismStore` remains available for compact RKL definitions.
`ProceduralMechanismStore` stores complete `CheckedTheory` values only under a
`VerifiedProceduralPromotion` token containing the theory's canonical hash,
source-proof set, verified case count, and actual reuse count. This prevents a
valid promotion token from being attached to an altered rule or schedule.

## Claim boundary

This demonstrates variable-cardinality higher-order application with learned
element semantics, exact quantified execution, canonical promotion, persistent
whole-theory memory, and tamper rejection. Element correspondences are currently
aligned by the frontend. Unordered correspondence invention, filtering, folds,
nested collections, variable output cardinality, and recovery branches remain
open.
