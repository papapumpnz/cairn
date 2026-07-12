# ADR-0018: Construct learned proof compositions from evidence frontiers

Status: accepted

Date: 2026-07-12

Owners: WP23/WP34 integration

## Context

ADR-0017 proved that distinct learned mechanisms could be rebound and jointly
verified inside changed proof hosts. The materializer nevertheless supplied the
complete host, including the learned operations and final result. Recognition
of already-present proof fragments is weaker than constructing a prediction
from evidence, and it cannot demonstrate that one verified experience changes
the next construction.

## Decision

Introduce `ProofFrontier`, an evidence-only typed DAG prefix with no learned
operation nodes, output node, or expected proof. The reference consolidator
sorts distinct compiled mechanisms by semantic identity, enumerates bounded
typed boundary/value bindings, expands each mechanism, and adds one fixed
domain-neutral joint-result node. It returns untrusted
`ConstructedInternalComposition` candidates with exact work accounting.

Introduce task-local `InternalConstructionPolicy`. After a caller has obtained
independent verification of a training construction, the consolidator records
only typed evidence ordinals for each mechanism parameter. Applying the policy
to a changed frontier rebinds those ordinals to new typed values and constructs
exactly one candidate. The policy contains no expected proof, task ID, answer,
or verifier authority and is not persistent mechanism memory.

Expose both operations through `CognitionRuntime`, restricted to persistent
internal mechanisms. Blank or non-persistent memory fails before construction.

Add `InternalMechanismVerifier::verify_constructed_composition`. The expected
proof is verifier-only input. The independent path validates frontier topology,
requires the candidate prefix and complete graph to equal held-out truth,
reconstructs every learned invocation, checks the fixed joint result, reruns
composition verification, and emits a canonical construction certificate.

Existing mechanism identities and persisted schemas are unchanged. These are
bounded proposal and verifier interfaces, not a new trusted serialization
format.

## Alternatives considered

### Continue matching complete proof hosts

Benefits: existing composition code already passed.

Costs: the runtime receives the structure it claims to predict.

Reason rejected: it does not establish evidence-to-construction learning.

### Persist the selected candidate as a composite mechanism

Benefits: trivial one-candidate reuse.

Costs: stores a host-specific answer and bypasses rebinding.

Reason rejected: violates the memory boundary and does not test transfer.

### Let the runtime compare candidates with expected proof

Benefits: simple candidate selection.

Costs: leaks held-out truth across the verifier boundary.

Reason rejected: only the independent verifier may consume expected proof.

## Consequences

Correctness: every counted construction equals verifier-only held-out truth and
all learned expansions replay independently.

Expressivity: typed scalar bindings and a fixed n-ary result join are supported;
relational binding induction, arbitrary schedules, and raw-world proof
construction remain open.

CPU cost: training construction is bounded exhaustive enumeration; learned
policy application emits one candidate.

Memory cost: the current task-local policy stores mechanism hashes and small
ordinal vectors only.

Determinism: mechanism order, binding order, candidate order, and certificates
have canonical identity/tie breaking.

Verifier impact: expected proofs remain exclusively in the independent path;
producer output cannot certify itself.

Downstream interfaces: `cairn-consolidator`, `cairn-runtime`,
`cairn-verifier`, and WP34 G6 integration gain additive APIs.

## Validation

Tests: producer tests cover evidence-only enumeration, changed-value ordinal
transfer, insufficient cardinality, duplicate mechanisms, capacity, and
malformed frontiers. Verifier tests cover exact reconstruction and changed
frontier, expected graph, joint result, and lineage failures.

Benchmark gate: frozen G6 learns the policy from one independently verified
construction and applies it once to changed values and geometry without
expected-proof input. The held-out candidate verifies exactly. Aggregate work
is 72-to-40 candidate windows, 16-to-one examples, and 112-to-16 proof units;
the runtime generates 16 training candidates and one held-out candidate.

Rollback trigger: expected proof reaches the producer, a policy embeds literal
answer structure, non-persistent memory is callable, changed typed cardinality
silently defaults, or independent replay cannot reproduce the result.

## Compatibility

Semantics version change: none for persisted mechanism or curriculum schemas;
new in-process proposal/certificate marker
`cairn.verified-internal-construction.v1`.

Migration: no persisted artifact migration. Consumers opt into the additive
construction APIs.

## References

- ADR-0017 generated proof-mechanism composition
- ADR-0007 self-compressing mechanism learner
- `docs/experiments/G6_AUTONOMOUS_RESTART_CURRICULUM.md`
