# CAIRN agent operating law

This file is binding for every coding, research, benchmark, and swarm sub-agent.

## 1. Prime directive

Build general mechanisms and measure them honestly. Do not add code whose primary purpose is to flip one known task.

## 2. Hard prohibitions

- No GPU use, GPU dependency, remote GPU service, or hidden accelerator path.
- No transformer model, transformer library, or transformer-derived runtime component.
- No production source containing an ARC task identifier.
- No branch on full-grid hash, task filename, evaluation index, or memorized answer.
- No evaluation data in tests, tuning, failure analysis, primitive design, memory, or prompts.
- No acceptance of a theory without exact current-task replay.
- No learned module directly emitting an answer.
- No opaque invented primitive without an RKL definition.
- No weakening a pre-registered threshold after seeing a result.
- No deleting or rewriting a negative result.
- No claiming transfer from training-only refill or synthetic self-alignment.
- No parking a sub-agent waiting on a long benchmark; launch it with logs and return the run handle.

## 3. General-mechanism test

Before adding a production mechanism, the author must answer:

```text
What class of observations triggers it?
What task-independent semantics does it implement?
What synthetic family tests it?
What negative cases prevent overfitting?
Which failure taxonomy bucket should move?
How will an ablation prove causal contribution?
```

A mechanism without clear answers remains an experiment branch.

## 4. Evidence standard

A performance claim requires:

- committed config;
- exact code revision;
- machine record;
- raw per-case results;
- replayed certificates;
- baseline under comparable budget;
- uncertainty or repeat policy;
- attribution or an explicit statement that attribution is unknown.

## 5. Definition standard

Every RKL definition must have:

- typed signature;
- totality or fuel bound;
- reference semantics;
- property tests;
- cost;
- provenance;
- canonical serialization;
- no hidden mutable global state.

## 6. Solver standard

Every solver component must distinguish:

```text
unsatisfiable
not found within current sketch
ontology missing
capacity cap
wall timeout
internal error
```

Do not collapse these into `failed` or `inexpressible`.

## 7. Perception standard

An entity detector may propose an entity. Only a theory or deterministic observation rule justifies its use. Do not assign semantic names from appearance alone.

## 8. Memory standard

- Task-local definitions are temporary by default.
- Promotion is offline and evidence-gated.
- Support geometry records exercised dimensions.
- Counterexamples are first-class and never discarded to improve a score.
- A module that harms transfer is deprecated, not silently retuned on evaluation.

## 9. Swarm standard

- All jobs idempotent.
- All artifacts immutable.
- Producer is not verifier for final acceptance.
- Arrival order cannot change semantic answer in deterministic mode.
- Remote artifacts are untrusted until replayed.
- Resource use is measured by role.

## 10. Code quality

- Rust stable toolchain pinned by repository policy.
- `cargo fmt`, `clippy -D warnings`, unit tests, property tests, and repository checker pass.
- Unsafe code isolated, documented, fuzzed, and benchmark-justified.
- Public APIs documented with invariants and failure modes.
- No silent fallback from exact to approximate semantics.
- Approximate frontends label uncertainty explicitly.

## 11. Benchmark hygiene

- Development suites frozen before implementation of the measured mechanism.
- Synthetic generator and solver authored in separate modules; ideally separate agents.
- Generator internals unavailable to solver runtime.
- Held-out dimensions and structural combinations declared before runs.
- Public evaluation touched only by sanctioned gate.
- Private scores never reverse-engineered through repeated submission.

## 12. Commit and handoff format

Each work package handoff contains:

```text
scope completed
files changed
public APIs
commands run
acceptance criteria status
benchmarks and machine
known defects
artifacts and hashes
next dependency
```

## 13. Architecture changes

A change to these requires an architecture decision record:

- RKL semantics;
- artifact hash semantics;
- verifier trust boundary;
- memory acceptance boundary;
- evaluation quarantine;
- CPU or transformer constraint;
- theory score ordering;
- public schema compatibility.

## 14. Honest stopping rule

When a pre-registered kill criterion fires, stop that path, commit the evidence, and escalate the architectural decision. Do not bury a failed hypothesis under implementation activity.
