# Multi-task policy aggregation protocol v0.1

## Purpose

A policy benchmark with one task cannot support a claim about general optimizer performance. Shape and flow categories differ, and per-task optimizer rankings can change. This protocol aggregates already evaluated, evidence-gated policy scorecards across a declared family of tasks while keeping the task—not each individual run—as the unit of generalization.

## Input contract

Input rows must come from the policy benchmark scorecard and carry task, replicate and policy IDs, one task-level normalized validated hypervolume fraction, a fixed budget cap used by every policy and replicate within that task, the original operation accounting and evidence level.

The aggregator does not authenticate artifact receipts, re-run CFD, or make a synthetic input into experimental evidence. All input task ledgers must use the declared common cost-unit basis. Within each task, budget caps must match for every policy and replicate. Across tasks, the normalized hypervolume fraction removes objective-box units; the macro-average weights each task equally rather than weighting tasks by their number of runs.

Require at least five independent task families and at least two replicates per task. This is a minimum software acceptance floor, not enough to claim robust SOTA superiority. The research target is at least 20 independent task families, preferably spanning 2D sections, 3D wings, delta-wing/transonic, blended-body, commercial-aircraft, and UAV-like design classes, with at least 5 independent seeds per task after the initial smoke stage.

## Aggregation

For each task and policy, take the median normalized validated hypervolume fraction over replicates. Compare every policy to the declared baseline with a paired task-level difference. Then report the macro mean and median of task-median normalized validated hypervolume; nearest-rank p10 and worst-task quality; mean and median paired delta to baseline; nearest-rank p10 and worst-task paired delta; tasks improved, tied and degraded; and a 95% percentile interval for the mean paired delta from a deterministic bootstrap resampling tasks as clusters.

Replicate rows from one task are collapsed before bootstrap resampling. They are not treated as independent task families. Policies must have complete, identical task, replicate and policy matrices. Missing, duplicate, non-finite, out-of-range or over-budget rows fail closed.

The deterministic bootstrap seed and sample count are recorded by the CLI; the independent oracle reimplements the XorShift64 resampling sequence and nearest-rank interval calculation.

## Interpretation

The score is only as meaningful as the upstream admission record and task split. Freeze benchmark tasks, objective directions, ideal/reference boxes, baseline, total-cost accounting and blind confirmation split before tuning. Do not include the holdout geometry or test regimes in policy design. Report task-family breakdowns and failures, not merely the macro mean.

A positive mean delta with a confidence interval crossing zero is inconclusive. A positive mean with a negative p10 or a large worst-task loss is a fragile policy, not a general improvement. A claim of better-than-SOTA requires a blind, independently audited comparison against the strongest applicable baselines under identical per-task cost caps, including random/space-filling, Bayesian, adjoint/gradient, evolutionary and other ShapeBench baselines where license and adapters permit.

## Synthetic known-answer

The committed synthetic fixture contains ten tasks, three replicates and three policies named uniform, voi and adjoint. The uniform policy is the baseline. The fixed input includes a mixed set of per-family improvements and degradations. The independent oracle recomputes all aggregate metrics and 10,000 deterministic task-cluster bootstrap draws. All rows are explicitly synthetic and the workflow asserts no solver evaluations occurred.

## Research targets

Minimum acceptance:
- complete task/replicate/policy matrix;
- at least five task families and two replicates each;
- deterministic aggregation and independent bootstrap agreement;
- paired task-cluster resampling;
- full task win/tie/loss and worst-task reporting;
- no false generalization claim from a single task.

Stretch:
- 20+ public benchmark tasks, 5+ seeds;
- blind geometry-family and flow-regime holdouts;
- pre-registered baseline hyperparameters and compute caps;
- independent source and receipt verification;
- non-inferiority on worst-decile performance plus at least 20% fewer expensive evaluations at equal validated Pareto quality.
