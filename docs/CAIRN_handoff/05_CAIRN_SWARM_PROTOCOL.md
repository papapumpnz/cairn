# CAIRN swarm protocol

Protocol name: `cairn-gossip/1`

The protocol coordinates specialized CPU agents. It is content-addressed, immutable, verifier-first, and transport-independent.

## 1. Design principles

- Agents exchange evidence and proofs, not mutable objects.
- The same artifact is processed at most once per semantic version and budget class.
- Correctness never depends on trusting a producer.
- Large artifacts are referenced by hash and fetched on demand.
- Semantic determinism is independent of arrival order.
- The coordinator is replaceable and contains no ARC rules.
- A swarm can add a new capability by publishing a new artifact kind or producer capability through a versioned extension.
- Roles operate on generic observations, theories, transitions, goals, skills, and claims; benchmark adapter records never enter job semantics.
- Parallel search is not evidence of continual learning. Memory benefit is measured separately on frozen future tasks.

## 2. Node roles

### Coordinator

Maintains task frontier, job dependencies, budgets, and termination conditions.

### Artifact store

Content-addressed blob storage with canonical envelope validation.

### Resource broker

Chooses jobs from expected utility per CPU and memory unit.

### Worker

Advertises capabilities and executes idempotent jobs.

### Verifier

Replays exact artifacts and signs verification results. Verifier code is isolated from proposer code.

### Curator

Runs offline memory promotion and deprecation workflows.

A physical process may host several roles, but role identity remains explicit in artifacts.

## 3. Capability advertisement

```rust
pub struct WorkerHello {
    pub protocol: ProtocolVersion,
    pub worker: WorkerId,
    pub software_hash: Hash32,
    pub semantics_versions: Vec<SemanticsVersion>,
    pub capabilities: CapabilitySet,
    pub cpu: CpuDescription,
    pub memory_limit_bytes: u64,
    pub concurrency: u16,
}
```

Capabilities are fine-grained:

```text
entity.connected
entity.panel
entity.motif
event.correspondence
event.cover
analogy.align
guard.induce
constructor.synthesize
symbol.interpret
schedule.synthesize
theory.solve
egraph.reduce
failure.analyze
ontology.invent
verify.static
world.transition
world.goal
intervention.design
plan.astar
plan.ao
plan.mcts
compile.bytecode
memory.curate
memory.transfer_test
knowledge.resolve_entity
knowledge.extract_claim
knowledge.detect_conflict
knowledge.compile_schema
skill.induce
skill.compose
verify.transition
verify.knowledge_provenance
```

## 4. Job lifecycle

```text
proposed
ready
leased
running
completed
verified
accepted or rejected
expired
```

A lease includes a logical deadline and work budget. Workers checkpoint by publishing partial artifacts, not by mutating coordinator state.

## 5. Job classes

### Evidence jobs

Produce immutable facts, indexes, or entity definitions.

### Proposal jobs

Produce event covers, alignments, sketches, theories, definitions, or plans.

### Refutation jobs

Produce counterexamples, unsat cores, or falsification reports.

### Verification jobs

Replay and certify artifacts.

### Compilation jobs

Produce an optimized implementation plus an equivalence certificate against reference semantics.

### Memory jobs

Evaluate transfer, promotion, deprecation, and counterexample impact.

### Interaction jobs

Propose transition theories, discriminating interventions, executable goals,
plans, and reusable skills from observation-action-outcome histories.

### Knowledge jobs

Resolve entities, extract typed claims, retain contradictions, link sources, and
compile proof-backed domain schemas. Extractors cannot certify their own claims.

## 6. Priority vector

```rust
pub struct PriorityVector {
    pub critical_path: i32,
    pub expected_information_gain: OrderedFloat<f64>,
    pub expected_solve_gain: OrderedFloat<f64>,
    pub novelty: OrderedFloat<f64>,
    pub estimated_cpu_ms: u64,
    pub estimated_memory_bytes: u64,
    pub deterministic_tiebreak: Hash32,
}
```

The broker can implement:

```text
utility = weighted gain / estimated cost
```

Weights are configuration, recorded in the run manifest. Reproducibility mode uses a frozen deterministic queue.

## 7. Artifact trust levels

```text
unverified proposal
locally replayed
independently replayed
cross-implementation replayed
promotion-tested
```

Only independently replayed theories may emit final static answers. No proposal from a remote node is trusted based on signature alone.

## 8. Merge semantics

### Evidence fragments

Union by fact hash. Conflicting exact facts reject the fragment. Uncertain facts may coexist with distinct provenance.

### Event covers

Deduplicate by structural signature and replay digest. Retain top cost variants per signature.

### Alignments

Deduplicate by abstract role map up to canonical variable renaming.

### Theories

Deduplicate by normalized semantic hash. Search metadata and scores merge separately.

### Constraints

Monotonic hard constraints are unioned when their scope matches. Heuristic soft constraints retain source and confidence.

### Counterexamples

Union by mismatch and semantic blocked-choice signature.

### Beliefs

Recomputed centrally from evidence terms. Agents do not send authoritative normalized posterior probabilities.

### Knowledge claims

Deduplicate only when typed assertion, temporal extent, and source derivation are
identical. Conflicting claims coexist and produce a conflict artifact; merge
order cannot erase provenance.

## 9. Task sharding

Preferred sharding order:

1. distinct event-cover policies;
2. distinct alignments;
3. distinct theory shapes;
4. distinct ontology-invention families;
5. distinct falsification methods;
6. distinct output modes.

Avoid sharding individual tiny expression evaluations across the network.

For batch benchmarks, shard across tasks first. Within-task distributed search is reserved for hard tasks after local capacity is saturated.

## 10. Work cancellation

A verified answer does not immediately cancel all work. The coordinator retains enough budget to:

- search for a credible second mode;
- independently verify the first mode;
- finish active jobs likely to reveal a contradiction;
- collect terminal diagnostics.

Once stopping criteria hold, leases are revoked. Completed late artifacts may be stored for research but cannot alter the committed answer after deadline.

## 11. Corruption tests

The protocol test suite injects:

```text
bit-flipped payload
wrong payload hash
forged parent
unknown semantics version
replayed artifact under another task
inflated verification claim
incorrect CPU budget
cyclic derivation
invalid signature
protocol downgrade
```

All must fail loudly and produce audit events.

## 12. Local implementation

Phase one uses:

- one coordinator process;
- Unix domain sockets or loopback TCP;
- memory-mapped CAS;
- one worker process per physical core or role bundle;
- bounded channels;
- crash-safe append-only job journal.

Threads may be used inside a worker for bitset or solver parallelism, but process-level isolation is preferred for fault containment and CPU accounting.

## 13. Network implementation

Phase two uses a framed binary protocol over authenticated QUIC or TCP with:

- explicit maximum frame size;
- hash-before-decode for large blobs;
- backpressure;
- resumable blob transfer;
- protocol and semantics negotiation;
- optional signatures;
- no remote code execution.

Artifacts contain data and RKL definitions only. Workers do not load arbitrary native plugins received over the network.

## 14. Swarm contribution accounting

Every accepted artifact records producer lineage. Benchmark reports attribute:

- first event cover leading to solve;
- alignment used;
- theory proposal origin;
- decisive counterexample;
- invented definition;
- verifier;
- CPU consumed by role.

A capability claim requires unique solved tasks, time reduction, core reduction, or falsification value. Message count is not capability.
