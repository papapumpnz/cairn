# GLL-3 initial evidence: learned mechanism composition

Date: 2026-07-11  
Status: passed for the bounded arithmetic, collection, and control families below

## Claim

CAIRN can now combine two mechanisms learned independently on earlier tasks into
a new executable behavior without a developer-coded composite operator.

Four reusable mechanisms—signed magnitude, clamping, collection-cardinality
adjustment, and relational symbol selection—were each learned from
demonstrations on two independent source tasks and promoted through the GLL-2
memory boundary. Later tasks required behaviors that no single stored module
could express. The runtime constructed typed inner-to-outer substitutions,
replayed each composed definition against the current task, and admitted only
cross-implementation-exact results.

This demonstrates bounded symbolic skill composition. It does not yet satisfy
the full entity-relation composition target involving independently learned
containment and ordering, because demand-driven entity algebra and relational
guard induction are not implemented.

## Generic composition mechanism

Production code contains no branch for any tested composition family. For each
ordered pair of distinct active modules, it:

1. checks whether the inner return type can fill an outer parameter;
2. enumerates injective, type-compatible bindings to current-task parameters;
3. alpha-rebinds the inner executable RKL body;
4. substitutes that body into the selected outer parameter;
5. records outer module ID, inner module ID, and substituted parameter;
6. type/totality checks and independently replays the result through both RKL
   executors against all current demonstrations.

Composition cannot emit an answer directly, and failure cannot veto fresh
search. The two module IDs must differ.

## Frozen results

| Independently learned mechanisms | New synthesized behavior | Composition proposals | Changed held-out predictions |
|---|---|---:|---:|
| magnitude → clamp | `max(minimum, min(max(x, -x), maximum))` | 7 | 369 exact |
| count adjustment → clamp | `max(floor, min(offset + count(items), ceiling))` | 1 | 18 exact |
| magnitude → symbol selection | `if max(x, -x) < limit { inside } else { outside }` | 1 | 123 exact |

Total: 510/510 exact held-out predictions.

For every family:

- no stored single module fit the demonstrations;
- bounded memory-off invention did not produce exact held-out behavior;
- the accepted definition carried two distinct module IDs;
- quarantining the used inner module disabled memory composition;
- the composed definition passed current-task reference/bytecode agreement.

## Reproduction

```sh
cargo test --locked -p cairn-memory --test gll3_gate -- --nocapture
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p cairn-check -- check .
```

The frozen suite is `crates/cairn-memory/tests/gll3_gate.rs`. Generic typed
composition and proof records are in `crates/cairn-memory/src/lib.rs`.

## Claim boundary and remaining risks

- Composition is currently one inner module substituted into one outer module.
  Deeper DAG composition and shared subexpressions remain future work.
- The current families exercise numeric, collection, and categorical-control
  mechanisms, not learned entity relations, planning options, or action skills.
- Search uses type shape and exhaustive parameter bindings. Rich relational
  modules will need REG-context retrieval and grounding constraints.
- Persistent proof certificates and crash-safe module storage remain incomplete.
- Unknown-world interaction and cross-domain runtime gates remain open, so CAIRN
  must not yet be described as a generally learning AI.
