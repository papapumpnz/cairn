# ADR-0017: Verify completion of generated proof-mechanism compositions

Status: accepted

Date: 2026-07-12

Owners: WP23/WP34 integration

## Context

ADR-0016 generated and selected a new composition objective but stopped before
materialization or execution. Existing internal mechanisms could bind and replay
one proof fragment at a time. There was no runtime contract for proposing that
several learned mechanisms jointly explain one result, and no independent gate
proving that every proposed invocation was distinct, necessary, exact, and
cheaper than relearning.

## Decision

Add a content-free `DevelopmentalExerciseRequest` derived from a generated
objective. It carries only objective identity, developmental need, prerequisite
mechanism roles, failure-core identity, and a minimum independent-variation
count. A separately governed materializer, never the curriculum controller,
maps this request to concrete exercises.

Add a bounded reference composition proposer for compiled internal mechanisms.
It sorts distinct mechanism identities, enumerates occurrence bindings with a
fixed work cap, requires pairwise-disjoint selected proof nodes, and requires
each invocation output to be an ancestor of the host proof result. The proposer
returns bindings only and cannot certify itself.

Expose that proposer through the shared `CognitionRuntime`, restricted to
persistent internal mechanism modules. No adapter-specific path or composite
operator is added.

Add `InternalMechanismVerifier::verify_composition`. The independent path
reconstructs every compilation and current invocation, rejects duplicate,
overlapping, irrelevant, mistyped, or changed bindings, and derives aggregate
memory-off/on candidate, sample, and proof work. A canonical opaque certificate
binds the source proposals, mechanism identities, selected proof nodes,
experience, and recomputed metrics.

The curriculum may record success only from independently verified composition
certificates over at least the requested changed variations. It stores the
certificate identity as support evidence; it does not store a host-specific
action sequence or permit the learned support record to emit an answer.

Existing consolidator mechanism semantics remain 2.3.0 because compilation,
matching, canonical mechanism bodies, and their identities are unchanged. The
new composition plan is a non-persistent proposal object and the verifier
certificate has its own versioned canonical marker.

## Alternatives considered

### Treat two successful matches as composition

This permits overlapping fragments and matches unrelated to the selected
result. Rejected.

### Let the runtime accept its own plan

This crosses the verifier trust boundary. Rejected.

### Store a fixed composite macro

That would create the exact task-specific operator the experiment is meant to
avoid. The generic runtime binds the independently learned mechanisms anew.

## Consequences

Generated curriculum objectives can now reach a verifier-controlled terminal
result at the proof-mechanism layer. Changed host geometry is supported because
bindings are occurrence-local and mechanism identity excludes source nodes.

The current materializer supplies a complete proof host. CAIRN recognizes,
expands, and verifies the learned causal fragments but does not yet construct
that entire proof from raw environment observations. This result therefore does
not establish autonomous raw-world curriculum generation.

## Validation

Producer tests cover two distinct learned mechanisms, changed placement,
duplicate identity, irrelevant output, and work capacity. Verifier tests cover
order independence, exact joint contribution, overlap, irrelevant invocation,
lineage replay, and aggregate reductions.

Frozen G6 learns four mechanisms, generates its own composition objective,
materializes two changed proof-host exercises, explicitly invokes the first two
learned mechanisms twice each, and independently verifies both. Aggregate work
falls from 72 to 46 candidate windows, 16 to two examples, and 112 to 16 proof
units. The outcome is journaled and survives a fourth controller reconstruction.

Rollback or kill conditions: a non-persistent module is callable, an invocation
need not affect the result, overlap is accepted, producer output becomes verifier
authority, or the measured reduction cannot be independently reproduced.

## References

- ADR-0016 failure-driven developmental objectives
- ADR-0013 replay internal learning after restart
- `docs/experiments/G6_AUTONOMOUS_RESTART_CURRICULUM.md`
