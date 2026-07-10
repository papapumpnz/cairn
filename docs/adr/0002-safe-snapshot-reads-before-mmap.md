# ADR-0002: Use verified shared snapshots before unsafe file mapping

Status: accepted

Date: 2026-07-10

Owners: WP01 integration owner

## Context

WP01 proposed memory-mapped reads for large packed values. Rust file-backed
mapping requires an unsafe boundary because another process can mutate or
truncate the backing file while references remain live. CAIRN treats local CAS
files as untrusted until hash validation, and repository law permits unsafe code
only after a measured performance need, a safe reference implementation,
differential tests, fuzzing, and an explicit allowlist.

The safe reference CAS already measures above the mandatory cached-read gate:
1.41 GB/s for verified repeated 1 MiB reads on the recorded baseline machine,
against a 1.00 GB/s threshold.

## Decision

WP01 returns immutable `Arc<[u8]>` snapshots. Slicing a loaded blob is zero-copy,
the file length is bounded before allocation, and BLAKE3 is checked before bytes
become an accepted `CasBlob`.

File-backed memory mapping is deferred. It may be introduced only by a later
measured optimization ADR that specifies filesystem immutability assumptions,
isolates the unsafe call, proves lifetime and mutation safety, differentially
tests against `Cas::get`, and shows a material improvement on a frozen workload.

## Alternatives considered

### Add `memmap2` immediately

Benefits: avoids the initial userspace copy and can reduce repeated allocation.

Costs: introduces an unsafe lifetime/mutation boundary despite the safe path
already exceeding acceptance performance.

Reason rejected: violates the measured-need order for unsafe optimization.

### Trust read-only path conventions

Benefits: no new API shape.

Costs: path naming and permissions do not prevent the file owner or a remote
artifact synchronizer from mutating an existing inode.

Reason rejected: convention is not a safety proof.

## Consequences

Correctness: consumers see a hash-verified immutable snapshot even if the
filesystem changes later.

CPU cost: each uncached `get` performs a bounded read and BLAKE3 verification.

Memory cost: one owned copy per independently loaded blob; clones and slices
share it.

Determinism: unchanged.

Verifier impact: the verifier can safely retain slices without trusting backing
file immutability.

Downstream interfaces: consumers use `CasBlob::as_slice` or `CasBlob::slice` and
must not rely on a memory-mapping implementation detail.

## Validation

Tests: corruption during storage state, capacity checks, shared-slice lifetime,
and repeated verified reads.

Benchmark gate: cached verified 1 MiB reads must remain above 1 GB/s.

Rollback trigger: a frozen representative workload demonstrates that snapshot
copying is a dominant bottleneck and an unsafe mapping design satisfies every
repository unsafe-code obligation.

## Compatibility

Semantics version change: none; storage access method is non-semantic.

Migration: none.

## References

- `AGENTS.md`, unsafe and performance-engineering policies
- `WP01.md`
- WP01 benchmark evidence
