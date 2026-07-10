# ARC-AGI-1 training corpus provenance

- Upstream: `https://github.com/fchollet/ARC-AGI`
- Release tag: `v1.0.2`
- Upstream commit: `aa922be204204ec148a1137fe6ed4d34ddde812b`
- Imported split: `data/training` only (400 JSON tasks)
- Imported on: 2026-07-11
- License: Apache-2.0; the upstream `LICENSE` is retained alongside the data.
- Integrity: `SHA256SUMS` contains a digest for every imported task.

The upstream evaluation split is intentionally not present. During development,
CAIRN may use training-task test outputs only for post-prediction scoring. Test
outputs must not enter theory induction, candidate ordering, memory promotion,
or task-specific code changes.

The repository path and task filename are adapter metadata. They are prohibited
from entering generic observations or cognition semantics.
