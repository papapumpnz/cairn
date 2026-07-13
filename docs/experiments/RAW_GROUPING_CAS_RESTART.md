# Raw grouping CAS restart experiment

Status: `PASS_WITH_DECLARED_LIMITS`

## Frozen question

Can the relation-bound learner lose all process memory and reconstruct the same
verified capability solely from canonical raw-evidence artifacts, without
trusting serialized learning status?

## Method

Two disjoint raw relation-edge cohorts independently learn and verify the same
grouping/aggregate mechanism. Each cohort and its measured claim are stored in
the CAS with two source-evidence parents. The process then drops the CAS handle,
raw source arrays, learned mechanisms, and verifier receipts.

After reopening and recovering the CAS, a new empty store loads both untrusted
cohorts. CAIRN re-invents each mechanism and independently replays the raw
edges. Fresh receipts enter the consolidator, which issues a new promotion
certificate for a newly provisional store. A changed held-out topology is then
executed through the restored proposal.

## Measured result

```text
cohort artifacts: 2 distinct
serialized status restored: 0
cohorts re-invented after restart: 2
fresh receipts after restart: 2
mechanisms re-promoted: 1
fresh candidate executions / restored attempts: 187 / 1
fresh examples / restored examples: 4 / 0
held-out components after restart: 6 / 6 exact
missing-parent writes accepted: 0
wrong-kind loads accepted: 0
capped replays accepted: 0
```

Artifact identities:

```text
blake3:533cd69ff67b91a83de007b6d91060ecc0a0afa8fc0a658ed36170cf39408aa4
blake3:6ba8eb184aeb9e64f57c314353e41243f72b52356cf87cc7377b90523ac2ddbc
```

## Interpretation

This closes durable evidence replay for the bounded raw grouping mechanism. It
does not make learned status durable; authority is deliberately regenerated.
Primitive relation-predicate invention, generalized support geometry, and
adapter transfer remain open. This is not a general-intelligence or ARC-score
claim.
