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

“Offline” is a hard capability boundary, not a deployment preference. The
repository may serialize a GPT-compatible Responses request or an
Anthropic-compatible Messages tool request and may import a saved response, but
it has no HTTP client, socket, credential reader, provider SDK, or subprocess
transport for either path. Moving a spool artifact to or from a teacher is an
external, separately governed action. CAIRN never calls a teacher while
answering and may answer only from its current observations and locally
accepted knowledge.

Swarm agents transfer immutable artifacts by value. Initial swarm acceptance
requires local verification, two distinct non-author peer verifiers, positive
changed-context transfer, retained provenance, and revocation support. Imported
knowledge remains proposal-only at every task.

ARC may retrieve clean non-ARC knowledge after the evaluation set is frozen.
Every mechanism must rebind and replay all current demonstrations. Memory-off
and causal ablation reports are mandatory.

Provider-shaped wire formats share one `TeacherPacket` compiler and trust
lifecycle. Provider identity changes provenance only; it never changes
verification strength or promotion policy.

Gap-driven jobs may contain only redacted `LearningFailureCore` summaries:
type classes, demonstration and variation counts, deterministic-collision
status, exact anonymous mechanism-family matches, and exhausted bounds. Task
identifiers, variable names, observation payloads, literal values, expected
answers, and complete examples are forbidden. If identical available evidence
has conflicting outcomes, curriculum generation must target missing causal
context before proposing more formula search.

Teacher packets have separate scalar and variable-cardinality collection lesson
channels. Collection lessons are accepted only with multiple independent
episodes and at least two cardinalities. CAIRN must infer the element definition,
lift it into a checked quantified theory, replay every element, and transfer to
an unseen cardinality. Fixed-length traces and duplicate values under the
current set semantics fail closed. Collection knowledge remains provisional
until actual downstream reuse supplies promotion evidence.

## Consequences

The system can provide a normal chat surface before open-domain language is
solved, while being honest about its narrow controlled-language coverage.
Bootstrap competence becomes practical without permanent teacher dependence.
Additional attack surfaces—teacher error, code execution, artifact poisoning,
reputation bias, and revocation—become explicit verifier and experiment work.
The offline spool prevents accidental teacher dependence and makes missing
knowledge visible as a capability gap rather than silently outsourcing an
answer.
