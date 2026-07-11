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
The rendered typed AST is also executed by `python3 -I -S` with a cleared
environment, closed stdin, one-second CPU limit, 128 MiB address-space limit,
and two-second wall timeout before acceptance.

## Offline teacher packet

The frozen JSON fixture contains a paraphrase family, one sourced claim
candidate, a four-step dependency-checked procedure, three integer addition
examples, and an executable counterexample separating addition from
multiplication. It identifies itself as an offline teacher fixture and carries
an explicit uncertainty. The compiler creates structural language signatures,
typed provisional claims, a procedure graph, and a bounded
`InventionProblem`; dual RKL paths synthesize and replay the mechanism, then
execute the negative case. Claim structure is validated, while factual truth
remains provisional pending its named validator.

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
installs only the accepted lesson and answers unseen held-out wording using the
induced semantic signature (`python`, function/callable, add/sum,
number/value). A Python-class negative does not trigger the rule.

## Reproduction

```text
cargo test -p cairn-language -p cairn-distill
cargo run -p cairn-language --bin cairn-chat -- "Give me a basic Python function"
```

## Claim boundary

This proves the initial contracts, bounded structural paraphrase transfer, and
teacher removal. It does not establish broad natural-language understanding,
safe arbitrary Python, documentation-grounded factual truth, or distributed
transport. Those require later D1 gates.

## Initial clean transfer to ARC

A second offline packet uses only generic spatial-sensor examples. Two changed
families independently support `output := north`; no ARC grid, task identifier,
or answer appears in the packet. Local dual-executor replay, two separately
recompiled peer receipts, and 12 changed-context transfer cases make the packet
swarm accepted. Its two executable definitions then pass strict
`PromotionVerifier` evidence and enter `MechanismStore` as a persistent proposal
module.

On pinned official task `25ff71a9`, blank memory cannot solve under a frozen
one-candidate budget. The same budget with the non-ARC module succeeds exactly,
records `MemoryProposal`, and reports one proposal execution. Every official
training cell is replayed; expected test output is withheld until scoring.

This is an initial D2 compute/capability result under an intentionally tight
budget, not by itself a full-corpus accuracy increase.

## Complete 400-task D2 ablation

Adapter 1.15.0 ran the complete pinned corpus twice at the identical
1,000-candidate budget with a read-only runtime:

| Condition | Exact | Resource limited | Incorrect | Memory uses |
|---|---:|---:|---:|---:|
| Blank memory | 32/400 | 368 | 0 | 0 |
| Distilled persistent memory | 32/400 | 368 | 0 | 1 |

The single actual reuse is official task `25ff71a9`. Its trace changes from
`FreshSearch; search_work=2` to `MemoryProposal; search_work=1`. Removing memory
causes the expected fresh path. The other 399 tasks produce no incorrect
acceptance, so observed accuracy negative transfer is 0/400. Accuracy does not
increase at the ordinary budget because the law was already reachable in two
candidates.

The paired test took 878.33 seconds. Artifact timestamps imply roughly 420
seconds memory-off and 458 seconds memory-on; this split is derived, not an
embedded per-run CPU measurement. The roughly 38-second retrieval overhead is a
negative systems result: one narrow persistent mechanism is safe but not
globally cost-effective when offered broadly. Contextual retrieval must gate it
before a larger library is enabled.

Memory semantics 1.1.0 adds that gate for rebound single-variable mechanisms:
if any current demonstration contradicts the bound variable, retrieval rejects
the module before proposal compilation. A focused contrary-law test changes
rejected memory executions from one to zero without limiting fresh fallback.
The complete contextual rerun remains 32/400 exact with zero incorrect and
finishes in an embedded 390.85 seconds—67 seconds faster than unconditional
memory-on and about 29 seconds faster than the timestamp-derived memory-off
split. The latter comparison is indicative rather than a repeated paired timing.
`25ff71a9` still uses one memory proposal; `a79310a0` returns from 280 to its
memory-off 279 search work.

Raw reports:

- `full-400-distill-memory-off-v1.15.0.json`, SHA-256
  `d32703649a99ddcc4153028753f29f2719331925c2b628a2f9c11cc57bf52cd5`;
- `full-400-distill-memory-on-v1.15.0.json`, SHA-256
  `d6aa3aaf87d078c59fa4595ac48d25f17512ccb8f4b3f99c2207cff7c55ab072`.
- `full-400-distill-contextual-memory-v1.15.0-memory1.1.0.json`, SHA-256
  `9451e18190be37daa2edbf5a5c16fc938481418c4f4dee131322fcd2d15a0e18`.

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

- broad learned paraphrase generalization beyond the initial semantic-signature
  vocabulary;
- a broad Python AST, property testing, imports, APIs, or repository tools (the
  initial restricted AST does execute in a bounded CPython subprocess);
- semantic verification of typed factual claims, documentation checks, skill
  packets, or multi-example procedure/mechanism anti-unification;
- cross-implementation language verification, distributed transport, durable
  CAS/proof storage, signatures, reputation, or revocation propagation;
- richer context-gated retrieval for compound and composed mechanisms (the
  initial single-variable gate recovers the measured overhead);
- an accuracy increase at the ordinary ARC budget.

The next engineering gate is semantic claim validation, multi-family
anti-unification, and contextual retrieval, followed by a broader frozen
teacher-independence curriculum.

## Offline dual-provider follow-up

The provider boundary is now a network-free spool supporting GPT-compatible
Responses structured output and Anthropic-compatible Messages forced-tool
output. It exports request bytes and imports saved response bytes only. CAIRN
has no live teacher client and answers from accepted local knowledge.

An offline GPT-authored follow-up compiled 40 non-ARC lessons into 20 provisional
mechanisms, but the full frozen ARC rerun remained 32/400 exact with zero
incorrect, used none of those mechanisms in selected proofs, and took 476.50
seconds. This is not an ARC improvement and the library does not yet satisfy G1
actual-reuse evidence. See `OFFLINE_DUAL_TEACHER_ARC_TRANSFER.md`.
