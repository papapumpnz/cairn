# ADR-0008: Verified chatbot, symbolic distillation, and swarm transfer

Status: accepted

## Context

The causal learner alone cannot provide broad language understanding, dialogue,
technical knowledge, typed code generation, or polished responses. Requiring a
blank CPU learner to rediscover programming and language conventions is also an
impractical bootstrap strategy. At the same time, a remote teacher in the
answer path would violate the transformer-free runtime and would make CAIRN an
adapter around another model rather than an autonomous learner.

## Decision

CAIRN targets a proof-carrying software-engineering chatbot first. Language,
dialogue, knowledge, tools, typed artifact synthesis, and response realization
surround the unchanged cognition and verifier boundaries.

Stronger teachers may create offline, schema-closed teaching packets. They are
untrusted proposals. CAIRN compiles them into native typed artifacts, verifies
executable behavior, anti-unifies multiple examples, generates challenges,
records support and failures, and admits only teacher-independent transfer.
Weights, logits, hidden reasoning, opaque confidence, raw answer memorisation,
and benchmark answers are not ingestion formats.

Swarm agents transfer immutable artifacts by value. Initial swarm acceptance
requires local verification, two distinct non-author peer verifiers, positive
changed-context transfer, retained provenance, and revocation support. Imported
knowledge remains proposal-only at every task.

ARC may retrieve clean non-ARC knowledge after the evaluation set is frozen.
Every mechanism must rebind and replay all current demonstrations. Memory-off
and causal ablation reports are mandatory.

## Consequences

The system can provide a normal chat surface before open-domain language is
solved, while being honest about its narrow controlled-language coverage.
Bootstrap competence becomes practical without permanent teacher dependence.
Additional attack surfaces—teacher error, code execution, artifact poisoning,
reputation bias, and revocation—become explicit verifier and experiment work.
