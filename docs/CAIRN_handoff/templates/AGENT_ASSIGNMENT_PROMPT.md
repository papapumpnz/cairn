# Coding-agent assignment prompt

You own the work package attached below. Treat its public interfaces, acceptance criteria, forbidden shortcuts, and CPU-only constraints as binding.

Read before coding:

```text
00_README.md
02_CAIRN_MASTER_DESIGN.md
03_CAIRN_INTERFACES.md
06_CAIRN_AGENT_OPERATING_LAW.md
07_CAIRN_RISKS_AND_KILL_CRITERIA.md
this work package
all direct dependency handoffs
```

Operating rules:

1. Work only inside the package's owned scope unless an interface change is unavoidable.
2. Propose an ADR before changing a dependency's public contract.
3. Implement a simple, independently testable reference path before optimization.
4. Preserve exact provenance and typed terminal states.
5. Do not add task IDs, task hashes, literal evaluation examples, family-specific branches, opaque native concepts, transformer dependencies, GPU dependencies, or remote model calls.
6. A heuristic may order candidates but may not make a valid semantic candidate unreachable unless the package explicitly owns a hard resource cap.
7. Every optimized kernel requires differential tests against the reference implementation.
8. Every persistent artifact uses canonical serialization, a semantics version, producer identity, configuration hash, and content hash.
9. All claimed outputs must pass the independent verifier.
10. Deliver the completed `WORK_PACKAGE_HANDOFF.md`; a code commit without evidence is not complete.

Required first response to the integration agent:

```text
understood_scope:
public_symbols_to_implement:
direct_dependencies:
planned_reference_tests:
planned_acceptance_measurement:
interface_questions_or_none:
```

Required final response:

```text
commit:
interface_tag:
acceptance_status:
evidence_pack:
known_limits:
downstream_start_condition:
```
