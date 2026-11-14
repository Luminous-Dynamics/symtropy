# Hierarchical policy aggregation protocol v0.2

## Purpose

A policy benchmark with one task cannot support a claim about general optimizer performance. Moreover, multiple benchmark tasks may share a geometry or flow environment and therefore are not independent samples of generalization. This protocol first aggregates seeds within tasks, then tasks within explicitly declared families, and bootstraps at the family/environment level.

## Frozen task-family mapping

The caller supplies an exact task_id -> task_family_id mapping before comparing policies. Every scorecard task must appear exactly once; missing or additional mappings fail closed. A family should represent the independent geometry/environment or physical regime that defines the generalization unit, not a post-hoc grouping chosen to improve a policy's score.

This supports ShapeBench's hierarchy of many individual optimization tasks nested inside a smaller number of geometry/environment categories. It prevents dozens of closely related tasks from being mistaken for dozens of independent generalization trials.

## Input contract

Input rows come from the policy benchmark scorecard and carry task, replicate and policy IDs, a normalized validated hypervolume fraction, one fixed budget cap across all policies/replicates within each task, and the original operation accounting/evidence level.

The aggregator does not authenticate artifact receipts, re-run CFD, or turn synthetic inputs into experimental evidence. All task ledgers must use the declared common cost-unit basis. Every family receives equal weight after within-family task medians are calculated.

Require at least five independent families and at least two replicates for every task. These are software acceptance floors, not sufficient evidence for a SOTA claim. Research targets are at least eight independent environments and 20+ public tasks, with at least five seeds per task and complete blinded geometry/condition holdouts.

## Aggregation

1. Check complete, identical task/replicate/policy matrices and matched task budget caps.
2. For each task and policy, take the median normalized validated hypervolume fraction across independent replicates.
3. For each family and policy, take the median of that family's task medians.
4. Compare each policy to a declared baseline with a paired family-level difference.
5. Report the macro mean and median of family-median scores; nearest-rank p10 and worst-family score; mean/median/p10/worst-family paired deltas; families improved, tied and degraded.
6. Compute a deterministic percentile interval for the mean paired delta by resampling entire family clusters, not tasks or replicates.

The seed, sample count, cost-unit basis and canonical task-family mapping must be retained. The aggregate report carries the SHA-256 of the exact sorted UTF-8 mapping bytes, encoded as one task ID, a tab, a family ID and a newline per row. The independent oracle reimplements the XorShift64 sequence, mapping digest and nearest-rank interval calculation.

## Interpretation

A positive mean delta with a family-clustered confidence interval crossing zero is inconclusive. A positive mean with a negative p10 or substantial worst-family regression is not robust generalization. Do not state statistical superiority merely because the average is positive. Report all family results and all failed/omitted tasks.

A fair better-than-SOTA comparison requires frozen task/environment splits, identical per-task total-cost caps, pre-registered hyperparameters, and the strongest applicable random/space-filling, Bayesian, adjoint/gradient, evolutionary and ShapeBench baselines. Count training, tuning, failed solver attempts, mesh generation, acquisition and audit cost. Do not include blind holdout geometries or regimes during policy development.

## Synthetic known-answer

The workflow's synthetic fixture contains ten tasks, grouped into five families, three replicates and three policies (uniform, VOI and adjoint). Uniform is the declared baseline. The family-level input intentionally mixes positive and negative policy deltas. The independent Python oracle recomputes all family medians, macro metrics, win/tie/loss counts and 10,000 deterministic family-cluster bootstrap samples. No CFD or solver evaluations are performed.

## Targets

Minimum software acceptance:
- complete task/replicate/policy matrix;
- exact task-family mapping;
- at least five families, two replicates per task;
- deterministic outputs and independent oracle agreement;
- family-level bootstrap, wins/losses, p10 and worst-family disclosure;
- explicit evidence ceiling and no false generalization claim.

Research stretch:
- eight or more independent benchmark environments and 20+ tasks;
- five or more seeds per task;
- blind geometry-family and flow-regime holdouts;
- pre-registered baselines and total-cost caps;
- non-inferiority on worst-decile/family performance plus at least 20% fewer high-fidelity evaluations at equal validated Pareto quality.
