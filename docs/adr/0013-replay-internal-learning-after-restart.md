# ADR-0013: Replay internal learning authority after restart

Status: accepted

Date: 2026-07-12

Owners: WP23 integration

## Context

ADR-0012 made compiled internal mechanisms source independent and allowed
verified runtime reuse to cause automatic promotion. The compiled mechanism,
compilation token, reuse receipts, and promotion certificate were process-local.
A restart therefore lost the learned lifecycle even though canonical source
proof graphs already existed.

Serializing verifier tokens or a promoted store snapshot would preserve status
without preserving the authority needed to justify it. CAS integrity proves
immutability, not semantic correctness.

## Decision

Memory semantics 1.7.0 adds schema
`cairn.memory.internal-reuse-evidence` version 1 with artifact semantics 1.0.0.
The canonical payload commits to mechanism, source-proposal, and experience
identities; verified-case count; and candidate-window, example, and proof-unit
counterfactual measurements. Its envelope is experience scoped and requires
parent lineage.

The payload is always untrusted after loading. To regain authority, restart
must load canonical source and held-out proof graphs; remine the proposal;
recompile the mechanism; independently reverify compilation; rematch the host;
rederive the reuse receipt and metrics; compare every field with the persisted
commitment; and submit only the reconstructed opaque receipt to ordinary memory.
The ordinary separate consolidator then reissues promotion.

No compiled mechanism snapshot, verifier token, receipt object, certificate, or
module status is deserialized as authority. The compiled mechanism is rebuilt
deterministically from its canonical proof sources.

## Alternatives considered

### Persist the promoted store directly

This gives fast startup but allows status and counters to regain authority
without semantic replay. It was rejected.

### Persist opaque verifier tokens

Tokens encode trust-boundary capability, not portable evidence. Authority must
be reconstructed from current implementations and source evidence.

### Persist only source proof graphs

This loses which changed experiences demonstrated actual reuse and the frozen
counterfactual claims that must be reproduced. Discovery and actual-use
evidence are distinct.

## Consequences

Correctness: altered mechanism, proposal, experience, proof content, metric,
schema, semantics, scope, lineage, or payload marker fails closed.

Determinism: non-semantic producer, runtime, resource, and logical-time metadata
do not change artifact identity. Reconstructed proposal and mechanism identities
match their pre-restart values.

CPU cost: restart intentionally repeats mining, compilation, invocation replay,
and fixed-work measurement.

Storage: six proof artifacts and two small receipt artifacts reproduce the
frozen two-experience lifecycle. The compiled schema remains a deterministic
derivative rather than a second stored authority.

## Validation

The reference experiment persists four source and two held-out proof graphs plus
two verifier receipts, closes and recovers the CAS, and abandons process-local
learning objects. Reloaded sources reproduce the proposal and mechanism hashes.
Both commitments pass fresh independent replay. The ordinary store remains
provisional after one receipt and automatically becomes persistent after the
second consolidator-issued certificate.

Missing parents, changed host identity, changed source proof content, changed
metrics, malformed payload, wrong task scope, unsupported schema, and unsupported
semantics are rejected.

Rollback trigger: any loaded claim affecting status without replay, changed
semantic identity after restart, verifier disagreement, or promotion from fewer
than two reconstructed experiences.

## Compatibility

Memory semantics: 1.6.0 to 1.7.0.

The schema is new and has no earlier persisted representation. Existing
procedural reuse schema v1 remains unchanged.

## References

- ADR-0010 canonical proof graphs survive consolidator restart
- ADR-0012 source-independent internal mechanisms
- `docs/experiments/PERSISTED_INTERNAL_MECHANISM_LEARNING.md`
