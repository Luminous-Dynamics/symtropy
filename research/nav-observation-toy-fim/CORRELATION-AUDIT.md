# Held-out correlation-model audit

This module tests a dangerous assumption in robust fusion and active sensing: that the finite set of declared correlation hypotheses actually covers the residual dependence observed on held-out data.

The planner in the correlation-envelope module is robust only with respect to its scenario set. It cannot detect that every supplied scenario is wrong. The audit compares empirical pairwise residual correlations against the union interval of the frozen scenario family.

## Method

For each source pair:

1. Collect only blocks containing finite residuals for both sources.
2. Compute the sample Pearson correlation.
3. Transform it using Fisher's z transform.
4. Compute a two-sided confidence interval using the number of paired blocks.
5. Adjust the per-pair critical value with Bonferroni so the declared confidence is familywise across all source pairs.
6. Compare the interval with the pair's minimum-to-maximum declared rho across the scenario set.

The output is deliberately three-way:

- ConsistentWithEnvelope: the complete confidence interval lies within the declared interval. This is evidence of compatibility at the declared confidence, not proof of model truth.
- ModelMisspecificationDetected: the confidence interval is disjoint from the declared interval.
- Indeterminate: the interval overlaps without being fully contained, there are too few paired blocks, or a source has zero residual variance.

An aggregate audit is OutOfModel if any pair shows misspecification, ConsistentAtDeclaredConfidence only if every pair is fully contained, and otherwise Indeterminate.

## Input contract

Residual blocks are assumed to be independent, non-overlapping evaluation blocks. This assumption must be justified by the experiment design; the software only checks unique block IDs and finite values. For autocorrelated time series, first construct statistically defensible block summaries or use an effective sample size methodology validated for that process. The audit must not treat adjacent high-rate samples as independent evidence.

Missing source residuals are omitted pairwise and reduce sample count. They are never imputed with zero. Non-finite values, unknown sources and repeated block IDs fail closed.

## Reproducible synthetic cases

Run the isolated Rust crate's correlation_audit_demo binary using Cargo. It exercises:
- a hidden 0.95-like shared residual dependence against a declared [-0.2, 0.2] envelope (OUT_OF_MODEL);
- an orthogonal-residual example inside a declared envelope (CONSISTENT);
- a three-block case (INDETERMINATE).

## Limits

- Pearson/Fisher-z assumes the block pairs are suitable for this approximation; it is not robust to arbitrary heavy tails, nonlinear dependencies or serial correlation.
- Pairwise correlation is not the full joint dependence structure. A full correlation matrix must still be positive definite, as checked in the scenario evaluator.
- The union of marginal pairwise intervals does not prove that any one joint scenario simultaneously fits every pair.
- This audit does not estimate a new model, authorize independence, create a trust score, or supply a navigation protection level.
- Mycelix owns source/failure-domain evidence; the experiment owner owns the validity of holdout blocks and truth labels.


## Fail-closed planning composition

The correlation_planning module exposes a single composition function that:
1. audits held-out residual blocks against the full declared scenario family;
2. proceeds only when the aggregate result is ConsistentAtDeclaredConfidence;
3. otherwise returns ModelNotQualified(OutOfModel or Indeterminate) and no selected sensor subset;
4. after a successful audit, performs the exact worst-case robust subset selection.

The regression suite verifies all three paths: valid holdout enables a full-rank budget-feasible selection; a hidden shared correlation blocks the plan; too few blocks block the plan. This is an offline qualification gate. Callers remain responsible for the holdout's freshness, independence and representativeness for the current environment.
