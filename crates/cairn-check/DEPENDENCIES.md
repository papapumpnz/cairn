# Dependency audit

All `cairn-check` dependencies are CPU-only data or tooling libraries with
default features disabled where supported.

| Dependency | Purpose | Enabled features | Determinism and policy notes |
|---|---|---|---|
| `blake3` | Lockfile digest in runtime attestations | `std` | Pure CPU hash; no accelerator or model path |
| `cargo_metadata` | Resolve actual Cargo packages and activated features | none | Invokes local Cargo; output is sorted before reporting |
| `indexmap` | Pin `toml`'s transitive map implementation to Rust 1.83-compatible code | none | Not used as semantic ordering; direct pin prevents incompatible lockfile drift |
| `serde` | Stable structured policy and attestation types | `derive`, `std` | Serialization support only |
| `serde_json` | Machine-readable reports and attestations | `std` | JSON map construction uses ordered structs/maps |
| `toml` | Parse Cargo and architecture policy manifests | `parse` | Read-only parsing; semantic results are explicitly sorted |

The lockfile is part of the checked repository boundary. Any dependency or
feature change requires this audit to be updated and the planted policy tests
to remain green.
