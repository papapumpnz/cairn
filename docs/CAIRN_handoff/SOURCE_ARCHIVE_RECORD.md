# Supplied source archive record

This record identifies the exact inputs reviewed for the redesign. Hashes use SHA-256 over the uploaded ZIP bytes.

| Uploaded archive | SHA-256 | Entries |
|---|---|---:|
| `mosasic_experiments.zip` | `6fa6b89f57c5726dc3da81ab96747c0520745341d66ea8ad7e059466113eb8e2` | 104 |
| `mosaic_docs.zip` | `1b5a65ed98a3e5f3931871bb7ab345f32021df098c858a2f2f438becc20c47d8` | 2 |
| `scem_experiments_docs.zip` | `b4187a8306a4ff64267921ca66c1fbcaaefee47298a6628c3a627471b235d3e2` | 4,875 |

The first filename contains the supplied spelling `mosasic`; its contents identify the project as MOSAIC.

## Highest-weight reviewed artifacts

```text
SCEM
  experiments/FINAL_EVAL.md
  docs/scem_failure_review_refactor_plan.md
  docs/h2_deep_failure_review_and_h3_refactor.md
  experiments/H2-synth-transfer/
  experiments/H2-synth-discovery/
  experiments/H3.0-active-vs-passive/
  experiments/H3.1-axis-invention/
  experiments/H3.2-primitive-invention/
  experiments/R3.6-substrate-headroom/

MOSAIC
  docs/MOSAIC_DESIGN.md
  experiments/WP3/
  experiments/WP3-R1/
  experiments/WP5/
  experiments/X1/
```

## Interpretation rule

The redesign separates three categories:

```text
measured architectural failure
measured implementation-frontier failure
mechanism that worked and should be retained
```

MOSAIC X1 is treated specifically as a failure of the concrete implemented frontier, not a proof that every mechanism in the larger design document was invalid. The redesign nevertheless removes the fixed semantic-closure assumption because SCEM and MOSAIC both produced independent evidence that missing concepts, role variables, constructors, and multi-rule composition cannot be repaired by search scheduling alone.
