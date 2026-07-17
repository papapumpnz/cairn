# Frozen-core autonomy audit

Status: `FAIL`

## Purpose

Test whether CAIRN can keep acquiring broadly useful capabilities without
developers adding representations or mechanisms after observing failures.

## Frozen cognition revision

```text
6eb8468578a31390bda6ef3ea67e7fcbc7e72db1
```

## Mandatory gates

1. No cognition, learner, memory, verifier, or adapter changes during the run.
2. Three post-freeze domains authored independently without solver access.
3. No family labels, required mechanisms, operator lists, or curriculum order.
4. Objectives generated and selected from content-free failures.
5. Ten consecutive acquisitions spanning static, interactive, and constructive
   problems.
6. Rising held-out success with falling examples, candidates, and interventions.
7. No source change needed for an individual domain.
8. A second independent suite confirms transfer with negative transfer below
   2%.

Any mandatory failure fails the audit.

## Result

```text
frozen source: PASS
independent post-freeze domains: FAIL
no supplied family/mechanism structure: FAIL
self-generated objectives: PARTIAL
ten consecutive cross-domain acquisitions: FAIL
sustained falling marginal cost: PARTIAL
no source changes per new domain: NOT TESTABLE
independent confirmation: FAIL
overall: FAIL
```

The full frozen workspace passes its bounded learning suites. Those generators
are separated from runtime modules, but they were co-developed inside this
project and the architecture was repeatedly extended after their failures.
Several successful families also begin with supplied scalar channels, operator
families, typed objectives, or bounded grammars. This is valid bounded learning,
not evidence of open developmental autonomy.

## Consequence

CAIRN is not on a demonstrated path to general intelligence. Internal mechanism
milestones cannot change this audit result. A rerun requires externally authored
sealed environments, unchanged cognition, and no target concepts, family labels,
solutions, or curriculum order. At least one domain must be interactive and one
must use a non-grid structured representation.

