# Compiled internal mechanism reuse and automatic promotion

Date: 2026-07-12

Status: measured bounded continual-learning result

## Question

Can independently discovered internal proof structure become one stable
executable mechanism, reduce later learning work, and be promoted automatically
from independently verified runtime reuse?

## Frozen design

Two disjoint four-episode cohorts contain the same typed four-node causal
structure inside different host prefixes and suffixes. Episode identities,
role values, effect values, host operations, and surrounding topology differ.
Two further hosts are reserved for runtime reuse.

The producer mines each cohort separately and compiles each proposal. The
verifier independently reconstructs mechanism identity and expands every source
and held-out binding. The conservative fixed-work comparison counts every
source window at exactly the learned mechanism width and every window considered
by compiled matching. No caller-supplied width range can inflate the baseline.

## Commands

```sh
cargo test -p cairn-consolidator --test internal_subgraph -- --nocapture
cargo test -p cairn-verifier --test internal_compression_verifier -- --nocapture
cargo test -p cairn-memory --test internal_auto_promotion -- --nocapture
```

## Results

```text
compiled_mechanism: blake3:a8438381973c8205112a6017ff7fd747009154990fe1e461ff67ec9a3321d781
first_source_proposal: blake3:f6464e545db3f71f658badc05cdd45df56e7bf9ba5338651769d2dfd00466739
independent_source_cohorts: 2
source_episodes: 8
compiled_identity_equal_across_cohorts: true
exact_source_replays: 8/8
runtime_experiences: 2
verified_cases_in_certificate: 10
candidate_windows: 18 -> 9, 18 -> 5
examples_required: 4 -> 1
proof_units: 28 -> 4
promotion_after_one_receipt: false
promotion_after_two_receipts: true
unregistered_proposal_receipt: rejected
registered_equivalent_proposals_in_one_certificate: 2
duplicate_experience: rejected
```

The evidence proposals retain different hashes because they bind different
episodes. Their compiled canonical bytes and semantic mechanism hashes are
identical because evidence identity and cost are not mechanism semantics.

## Attribution

Without compiled memory, the reference learner must enumerate the frozen
four-example source cohort and candidate windows. With compiled memory, it
matches one current host at the learned width and supplies only explicit
boundary and value bindings. The verifier, not the producer or memory store,
derives both work counts. The store cannot promote directly: the second opaque
receipt triggers `InternalPromotionConsolidator` and its certificate changes
the module state. A second independently verified cohort can be registered
against the same semantic mechanism and then contributes receipts to the same
certificate; its evidence is rejected before registration.

## Claim boundary

This is a genuine bounded continual-learning result: independently obtained
evidence converges on one mechanism, and experience measurably reduces later
candidate, sample, and proof work. The mechanism is an executable proof schema,
not yet an RKL causal theory; operations still require separately defined
semantics for world execution. Mining remains contiguous-window only. The
validated module state is not yet durable across restart. No ARC adapter or
benchmark data participates, and the result does not change the last measured
36/400 ARC score or establish broad G0-G6/general intelligence.
