# CAIRN operating law

CAIRN builds general, falsifiable mechanisms and measures them honestly. This
law applies to production code, tests, benchmarks, build tooling, optional
features, and generated code.

## Hard boundaries

- The complete runtime is CPU-only. CUDA, ROCm/HIP, OpenCL, Metal compute,
  Vulkan compute, accelerator-backed services, and accelerator-capable default
  features are prohibited.
- Transformer architectures, runtimes, model weights, coupled tokenizers, and
  remote model answer paths are prohibited.
- Evaluation data is capability-separated. Production code may not contain
  ARC task identifiers, output grids, full-grid hashes, evaluation paths, or
  branches on corpus position, filename, task identity, or hidden metadata.
- Exact claims require deterministic execution and independent replay. A
  proposer, heuristic, learned component, or memory record cannot accept its
  own answer.
- Persisted artifacts are immutable, content-addressed, versioned, bounded,
  and validated before use. Unknown required fields and unsupported semantics
  versions fail closed.
- Unsafe Rust is denied unless an architecture-approved policy allowlist names
  the narrow module and the implementation has a documented safety argument,
  reference comparison, fuzzing, and measured justification.

## Engineering law

Public semantics are typed, bounded, deterministic, documented, and changed
only through an accepted ADR. Data-dependent failures use typed errors; they do
not panic or silently fall back. Search limits report capacity, timeout, or
cancellation distinctly from unsatisfiability and missing ontology.

Every mechanism states its trigger, task-independent semantics, synthetic
family, hard negatives, expected failure-taxonomy effect, and causal ablation.
Every performance claim preserves its committed configuration, code revision,
machine record, raw cases, replay status, comparable baseline, and negative
results.

## Learning law

Task learning, continual learning, and interactive causal learning are
mandatory co-developed loops. ARC is one adapter and cannot define core
semantics or the project schedule.

A general-learning claim requires omitted-concept invention, fixed-budget
memory-on versus memory-off improvement, composition of separately learned
skills, unknown-world learning, and one cognition runtime across at least three
domains. Until those gates pass, describe implemented results narrowly.

Persistent memory proposes mechanisms only. Knowledge claims retain source,
time, confidence, contradictions, and provenance. Neither memory nor knowledge
retrieval may become an acceptance oracle.

The normative detail remains in [AGENTS.md](AGENTS.md) and the versioned
architecture handoff under `docs/CAIRN_handoff/`.
