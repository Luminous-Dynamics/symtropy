# CivBench Task Evaluation Sidecar v0.1

## Purpose

Humanity Atlas owns simulation execution identity and the exact run artifact envelope. CivBench owns task-specific scientific evaluation.

The sidecar binds a benchmark result to a **canonical run-core digest** and a benchmark protocol digest. It is referenced from the existing Humanity Atlas Run v0.1 `artifacts[]` array using role `civbench_task_evaluation`. The run schema itself is not changed.

## Why the run-core projection exists

A full-envelope hash would create a cycle if:
1. the Humanity Atlas envelope hashes the sidecar in `artifacts[]`; and
2. the sidecar hashes the complete Humanity Atlas envelope.

Instead, `humanity-atlas-run-core.v1` hashes a fixed identity projection containing these fields in this exact set:

- `schema_version`
- `run_id`
- `scenario_id`
- `inputs`
- `engine`
- `run_status`
- `output_classification`
- `deterministic`
- `seed`
- `replicates`

The projection excludes `outputs`, `validation`, `warnings` and `artifacts`. Those excluded fields can contain the evaluation sidecar's own hash without changing the identity digest. Canonicalization is RFC 8785 JSON Canonicalization Scheme (JCS), then SHA-256.

This is an identity binding, not publisher authentication or scientific proof. Source identity, key custody, signed attestations and independent review are separate concerns.

## Separation of responsibility

| Humanity Atlas Run v0.1 | CivBench sidecar v0.1 |
|---|---|
| Scenario and input digests | Benchmark/task/protocol IDs |
| Engine/model/adapter identity | Scorer identity and scoring code |
| Run status, seed, replications | Dataset version, rights status and split semantics |
| Simple `outputs[]` points | Metric vectors, units, strata, sample unit and uncertainty |
| Content-addressed `artifacts[]` | Baseline comparisons, negative controls and missingness |
| Structural validation evidence | Claim ceiling, limitations and task-specific deviations |

A scalar metric can be duplicated in Humanity Atlas `outputs[]` for compact consumers. Distributional results, subgroup strata, trajectories, event traces and baseline comparisons belong in hashed sidecar artifacts. The sidecar lists those artifact IDs, roles, media types, relative references and SHA-256 digests; the Humanity Atlas envelope binds the same payload hashes in its own `artifacts[]`.

## Synthetic fixture

The fixture directory contains:
- the exact Humanity Atlas Run v0.1 schema snapshot from PR #1571 at commit `7f0128bca016f324faaf9041e9c99b3e3f65ec03`;
- a synthetic Humanity Atlas run envelope;
- a synthetic benchmark protocol document;
- a synthetic task-evaluation sidecar;
- a byte-pinned input CSV and a distributional metric CSV.

The fixture uses no historical, social-media, archaeological or human-subject data. It exists to validate the schema, the canonical run identity binding, the bidirectional artifact references, and failure-on-tampering controls.

## Reproduction

```sh
python3 -m pip install -r requirements-civbench-sidecar.txt
python3 scripts/research/validate_civbench_task_evaluation.py
```

The script validates both JSON Schemas, checks the pinned run schema's Git blob identity, recomputes all file hashes, verifies metric/baseline/split/stratum relationships and exercises five negative controls.

## Claim ceiling

A passing integration test establishes only that the contract and its synthetic fixture agree. It does not establish that a benchmark is empirically valid, that the model is calibrated, that the dataset is representative, or that the run producer is authentic. No cross-metric civilization score is defined or permitted.
