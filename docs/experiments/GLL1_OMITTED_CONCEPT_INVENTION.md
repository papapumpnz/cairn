# GLL-1 initial evidence: omitted-concept invention

Date: 2026-07-10  
Status: passed for the bounded typed-expression families described below

## Claim

CAIRN now demonstrates task-level symbolic learning in a bounded, generated
setting. Given only typed parameters, input/output demonstrations, a generic RKL
primitive basis, and resource limits, it discovers a previously absent
executable definition. No target expression, family identifier, adapter
identity, task identifier, or demonstration output literal is supplied to the
inventor.

This is the first measured evidence for Loop 1. It is not evidence that CAIRN is
a generally learning intelligence. GLL-2 through GLL-5 remain open, and WP20's
REG-driven predicate, role, recursive, and CEGIS-triggered invention scope is not
complete.

## Method

`cairn-invent` performs deterministic, cost-ordered enumeration of type-checked
RKL compositions. It evaluates candidates through the slow reference executor,
deduplicates task-local observational equivalence classes, and selects the first
minimum-cost behavior matching every demonstration. The proposed `Definition`
is then:

1. checked for type, bounds, grounding, and totality;
2. replayed against every demonstration by the AST reference interpreter;
3. independently compiled and replayed by the register-bytecode executor;
4. rejected if results or complete deterministic traces differ.

The frozen tests then execute the admitted definition on values unavailable to
the search process. Leave-one-demonstration-out runs repeat invention after
removing each answer, then predict that omitted answer.

## Frozen families and results

| Omitted concept | Synthesized executable body | Cost | Candidates | Observational classes | Varied exact predictions |
|---|---|---:|---:|---:|---:|
| signed magnitude | `max(reading, -reading)` | 4 | 44 | 21 | 21 |
| changed-bound clamping | `max(lower, min(value, upper))` | 5 | 480 | 182 | 4,961 |
| relational symbol selection | `if reading < threshold { below_symbol } else { other_symbol }` | 6 | 1,340 | 300 | 441 |
| collection cardinality adjustment | `offset + count(items)` | 4 | 22 | 15 | 187 |
| coordinate-tuple translation | `(origin_row + row_shift, origin_column + column_shift)` | 7 | 15,181 | 7,265 | 225 |

Total varied predictions: 5,835, all exact.

Variation covers signed numeric ranges, changed lower/upper limits, unseen
categorical atoms, collection counts from zero through the declared maximum,
offset changes, coordinate positions, and positive/negative translations.

All 29 leave-one-demonstration-out runs relearned a definition without the held
answer and predicted it exactly. Reducing the expression-cost limit below each
learned definition's cost eliminated the solve in all five families. One hundred
controlled unrelated-label problems produced zero false accepted definitions.

## Reproduction

```sh
cargo test --locked -p cairn-invent --test gll1_gate -- --nocapture
cargo test --locked -p cairn-invent --test learning
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo run --locked -p cairn-check -- check .
```

The frozen fixtures are in
`crates/cairn-invent/tests/gll1_gate.rs`. Generic production search and admission
code is in `crates/cairn-invent/src/lib.rs`; no fixture family is referenced
there.

## Claim boundary and remaining risks

- This is executable concept invention, not mere lookup or selection from five
  supplied composite operations. The learned bodies are RKL compositions
  produced during the test.
- The primitive basis and search grammar are developer supplied. Learning is
  currently bounded by that representational basis.
- Observational equivalence is task-local and minimum-description biased;
  insufficiently varied demonstrations can still select a spurious simple rule.
- Cross-implementation admission is implemented; persistent WP06 proof
  certificates and corruption replay are not yet complete.
- Continual improvement, learned-module composition, interactive causal world
  learning, and one runtime across three adapters have not yet passed their
  gates. Therefore CAIRN must not yet be described as a general-learning AI.
