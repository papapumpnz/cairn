# Handoff-bundle validation report

Validation date: 2026-07-10 UTC

## Completed checks

```text
source ZIP files readable: pass
source ZIP SHA-256 recorded: pass
core Markdown documents present: pass
work-package Markdown files: 35/35
machine-readable work-package index: pass
work-package dependency references: pass
work-package dependency graph acyclic: pass
JSON syntax for all schemas: pass
JSON Schema Draft 2020-12 meta-validation: pass
example TOML syntax: pass
cpu_only = true: pass
allow_gpu_runtime = false: pass
allow_remote_inference = false: pass
worked static trace present: pass
worked interactive trace present: pass
artifact manifest generation: performed after this report
```

## Scope of validation

This validates the architecture handoff as a coherent specification package. It does not validate a CAIRN implementation, prove that the architecture will solve ARC-AGI, or report a benchmark result. Those claims are deliberately reserved for the experiment ladder and independent replay verifier.

## Normative precedence

When documents appear to conflict, use this order:

```text
06_CAIRN_AGENT_OPERATING_LAW.md
03_CAIRN_INTERFACES.md and schemas/
02_CAIRN_MASTER_DESIGN.md
assigned work package
accepted architecture decision records
examples and templates
```

An accepted ADR may supersede an interface or design detail only when it records the semantics-version change and migration path. It may not waive the CPU-only, transformer-free, evaluation-quarantine, provenance, or independent-verification requirements.
