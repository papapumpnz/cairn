# Work-package completion handoff

## Identity

Package: `WP__`

Owner:

Repository commit:

Public interface tag:

Semantics version:

Configuration hash:

## Scope delivered

State exactly which required deliverables are complete and which are not. Reference paths and public symbols.

## Interface changes

For every added or changed public type, trait, file format, or error variant, record:

```text
symbol_or_schema:
old_contract:
new_contract:
reason:
downstream_packages_affected:
compatibility_action:
```

## Build and test evidence

```text
rust_toolchain:
target_triple:
cpu_model:
physical_cores:
logical_cores:
ram_bytes:
os:
compiler_flags:
```

Commands:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p cairn-check -- audit .
```

Results:

```text
unit_tests:
property_tests:
differential_tests:
fuzz_or_metamorphic_tests:
integration_tests:
```

## Acceptance table

| Requirement | Threshold | Measured | Evidence artifact | Status |
|---|---:|---:|---|---|
| | | | | |

## Benchmark protocol

Dataset or generator version:

Seed set:

Warm-up policy:

Measurement repetitions:

Baseline commit:

Candidate commit:

CPU affinity:

Memory cap:

Timeout or fixed-work cap:

Raw artifact hash:

## Correctness and determinism

Reference implementation comparison:

Checkpoint/resume comparison:

Cross-process deterministic replay:

Canonical serialization round trip:

Independent verifier result:

## Resource profile

```text
median_wall_ns:
p95_wall_ns:
max_wall_ns:
median_cpu_ns:
peak_rss_bytes:
allocations:
solver_conflicts_or_search_nodes:
```

## Known limits

Record exact unsupported semantics, capacity ceilings, and adversarial inputs. Do not classify a capacity stop as an ordinary no-solution result.

## Deferred work

Each item must identify an owning package or issue. Avoid unowned TODOs.

## Downstream start condition

State the exact tag and tests a dependent agent must consume before beginning.
