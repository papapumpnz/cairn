# Basic technical chat and symbolic distillation

Status: initial bounded pass; broad language, broad Python synthesis, and
open-domain chat remain open.

## Chat path

Input: `Give me a basic Python function`

The controlled frontend emits `GenerateProgram(Python,
IllustrativeFunction)`, records the unresolved purpose ambiguity, chooses the
documented illustrative-example policy, constructs a typed two-parameter
addition AST, verifies positive and negative numeric examples, and realizes a
code block. Unknown objectives produce a clarification instead of guessed code.

## Offline teacher packet

The frozen JSON fixture contains a paraphrase lesson and three integer examples
of addition. It identifies itself as an offline teacher fixture and carries an
explicit uncertainty. The strict parser rejects unknown fields. The compiler
creates a language lesson and a bounded `InventionProblem`; the local dual RKL
paths independently synthesize and replay the executable mechanism.

The packet then passes through:

```text
local verification
-> replay-agent peer verification
-> falsifier-agent peer verification
-> two changed-context transfer cases
-> swarm accepted
```

The publication author is rejected as its own peer. After acceptance, the raw
packet and compiler are removed from the execution path. A new chatbot instance
installs only the accepted lesson and answers the normalized teacher-provided
wording with locally generated, verified typed code.

## Reproduction

```text
cargo test -p cairn-language -p cairn-distill
cargo run -p cairn-language --bin cairn-chat -- "Give me a basic Python function"
```

## Claim boundary

This proves the initial contracts and teacher removal, not learned paraphrase
generalization: the first language lesson is an exact normalized mapping. It
also does not yet prove consolidation across multiple programming mechanisms,
safe arbitrary Python execution, documentation-grounded factual ingestion,
distributed transport, or an ARC benefit. Those require the D1 and D2 gates.

## Initial clean transfer to ARC

A second offline packet uses only generic spatial-sensor examples. Two changed
families independently support `output := north`; no ARC grid, task identifier,
or answer appears in the packet. Local dual-executor replay, two separately
recompiled peer receipts, and 12 changed-context transfer cases make the packet
swarm accepted. Its two executable definitions form a provisional general
memory module.

On pinned official task `25ff71a9`, blank memory cannot solve under a frozen
one-candidate budget. The same budget with the non-ARC module succeeds exactly,
records `MemoryProposal`, and reports one proposal execution. Every official
training cell is replayed; expected test output is withheld until scoring.

This is an initial D2 compute/capability result under an intentionally tight
budget, not a full-corpus accuracy increase. The ordinary 1,000-candidate first
20 official regression remains 17/20 exact with zero incorrect accepts. Full
adapter 1.15.0 remains to be rerun over all 400 tasks before its headline score
can change.

## Engineering review

What is now real:

- an executable chat CLI with explicit intent, dialogue goal state, typed code
  artifact, deterministic AST evaluation, and controlled realization;
- a closed, content-addressed teacher packet whose hash covers executable
  evidence;
- local RKL invention with reference/bytecode replay;
- peer receipts created by separately recompiling and replaying the packet;
- author self-review rejection, transfer gating, accepted-artifact lookup, and
  revocation state;
- caller-supplied general memory in the ARC adapter, with current-task replay;
- one clean non-ARC-to-ARC capability result.

What is not yet real:

- learned paraphrase generalization beyond exact normalized lessons;
- a broad Python AST, actual sandboxed CPython execution, property testing,
  imports, APIs, or repository tools;
- typed factual claims, documentation checks, procedure graphs, skill packets,
  counterexample compilation, or multi-example anti-unification in the packet
  compiler;
- cross-implementation language verification, distributed transport, durable
  CAS/proof storage, signatures, reputation, or revocation propagation;
- strict persistent `MechanismStore` promotion of the teacher-derived spatial
  law (the swarm-accepted packet currently supplies a provisional proposal that
  is still fully replayed on the ARC task);
- a complete 400-task adapter 1.15.0 memory-on/off run or an accuracy increase
  at the ordinary budget.

The next engineering gate is typed procedure, knowledge, and counterexample
packets plus durable proof-backed promotion, followed by a frozen multi-family
teacher-independence curriculum and a complete ARC memory ablation.
