# Offline AI relational distillation

Date: 2026-07-12

This iteration replaces the handwritten partition-extraction curriculum with an
offline Anthropic-shaped AI-teacher packet. The packet contains software-graph,
document-structure, and vector-diagram entity collections. It contains no ARC
grids, colours, task IDs, panel coordinates, or benchmark answers.

CAIRN compiles the packet into typed entity and relation evidence, invents an
executable selector from two domains, and independently checks the third held-out
domain. The first packet was rejected because its contrasts did not distinguish
occupancy from diversity. A revised packet independently varied both features
and passed held-out replay.

The verified selector is then rebound to finite-field entities. Full-corpus
applicability remains exactly `09629e4f`, `1cf80156`, and `2dc579da`, with no
false acceptance. Thus the 35/400 result is preserved while the extraction
knowledge now originates from general cross-domain AI-teacher evidence rather
than an ARC-shaped synthetic curriculum.

A subsequent frontend-rebinding experiment applies the unchanged verified
selector to connected entity collections. The new frontend supplies relative
occupancy and diversity evidence and reconstructs a uniquely selected native
frame. This adds one exact frozen solve and raises the complete result to
36/400 with zero incorrect tasks. The causal ablation and claim boundary are in
`CROSS_FRONTEND_ENTITY_ROLE_TRANSFER.md`.

Teacher orchestration and import are file-only. CAIRN has no online provider
access and treats the teacher as a proposal source, never as a verifier.
