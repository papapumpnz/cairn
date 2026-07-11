# Offline dual-teacher orchestration and ARC transfer

Status: orchestration pass; first GPT curriculum transfer failed to improve ARC.

## Boundary tested

`cairn-teacher-spool` implements two provider-shaped formats over one neutral
job and packet contract:

- GPT-compatible Responses structured output;
- Anthropic-compatible Messages forced-tool output.

Both implementations only export JSON bytes and import previously saved JSON
bytes. The crate has no HTTP, socket, credential, provider SDK, or process
dependency. Moving bytes to a teacher is outside CAIRN. Neither chat nor ARC can
call a teacher; runtime answers are derived only from observations and accepted
knowledge.

Saved responses remain untrusted. Import performs closed packet parsing, after
which the ordinary compiler, dual executor, peer replay, transfer gate,
promotion verifier, support geometry, and revocation rules still apply.

## Offline GPT curriculum

This development session authored 40 non-ARC scalar lessons covering 20 generic
spatial-sensor laws. Each law had two independently varied lesson families. The
learner was given examples and a minimal scalar substrate, not final RKL
definitions. It synthesized the formulas, replayed them locally, survived two
independent recompilation/replay receipts and 64 declared transfer cases, and
admitted 20 separate provisional modules. None may become persistent until
measured task reuse supplies the missing promotion evidence.

The packet contains no ARC task identifier, grid, expected answer, or benchmark
lookup. Its SHA-256 is
`34742f2641bc6fea96b84ef3befc86391f8ba2ed10733c8f7380ddba218b00c3`.

## Frozen ARC-1 result

The official 400 training tasks were rerun with adapter 1.15.0 and the same
1,000-candidate task budget used by the prior contextual-memory audit.

| Metric | Result |
|---|---:|
| Exact | 32/400 |
| Resource limited | 368/400 |
| Incorrect | 0/400 |
| Selected proofs using new memory | 0 |
| Elapsed | 476.50 s |
| Provisional teacher mechanisms | 20 |

The report SHA-256 is
`d32703649a99ddcc4153028753f29f2719331925c2b628a2f9c11cc57bf52cd5`.

## Review

The dual offline orchestration and strict learning pipeline passed. The transfer
hypothesis did not: accuracy did not rise, no imported mechanism was causally
used, and runtime regressed relative to the 390.85-second contextual single-law
run. These 20 items therefore remain verified provisional mechanisms but do not
satisfy G1 useful-persistent-knowledge evidence.

The next distillation job must be gap-driven. It should summarize generic,
non-benchmark failure cores from unsolved observations, request examples and
counterexamples for missing representational structures, then require actual
proof-trace use and a memory-off ablation before promotion is credited. Adding
more guessed geometric laws is rejected as the next strategy.

This next step is implemented in `GAP_DRIVEN_FAILURE_CURRICULUM.md`.

## Reproduction

```text
cargo test -p cairn-teacher-spool
cargo test -p cairn-arc --test distilled_transfer \
  gpt_teacher_curriculum_compiles_to_provisional_non_arc_library
cargo test -p cairn-arc --test distilled_transfer \
  complete_gpt_teacher_arc_run -- --ignored --nocapture
```
