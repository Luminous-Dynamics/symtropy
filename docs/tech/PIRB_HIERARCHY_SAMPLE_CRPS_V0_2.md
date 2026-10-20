---
title: PIRB hierarchy finite-sample CRPS evaluation
version: 0.2
status: research-stage scoring contract
authority: MeasurementOnly
---

# PIRB hierarchical finite-sample scoring contract v0.2

## Purpose and separation of claims

The hierarchy-coherence contract v0.1 checks whether parent and child counts agree, including for each joint predictive draw. Coherence does not imply that those counts are close to the observed outcome. This v0.2 scoring contract pins a separate outcome snapshot and measures the marginal predictive-distribution quality of the raw and reconciled count samples against the same mature outcome rows.

The scoring command is `pirb-hierarchy-sample-score`. It does not replace `pirb-hierarchy-coherence-audit`: CI runs both tools so coherence and predictive quality remain separate measurements.

## Empirical CRPS

For samples (x_1,ldots,x_m) from a univariate predictive distribution and observed count (y), the empirical continuous ranked probability score is

[
\operatorname{CRPS}(F,y)=\frac{1}{m}\sum_{i=1}^{m}|x_i-y|
-\frac{1}{2m^2}\sum_{i=1}^{m}\sum_{j=1}^{m}|x_i-x_j|.
]

Lower is better. The first term penalizes sample draws far from the observation. The pairwise term rewards useful spread rather than treating all samples as independent point predictions. The implementation uses an equivalent sorted-sample identity for the pairwise term, with (O(m\log m)) complexity instead of a quadratic pair loop. Unit tests compare it to an independent quadratic oracle over golden cases, including ties and nontrivial samples.

CRPS here is **marginal**: it scores each forecast node's sample distribution against that node's observed count. It does not evaluate dependency/correlation structure across all nodes in the joint sample. PIRB must not claim multivariate predictive superiority from the mean of marginal CRPS values. Joint distribution comparisons require a separate appropriate score (for example, energy or variogram score) with a preregistered protocol; in the probabilistic-reconciliation literature, log score is not a valid universal choice for comparing coherent and incoherent predictive distributions. Reference: Panagiotelis et al. (2023), https://doi.org/10.1016/j.ejor.2022.07.040.

## Frozen outcomes and eligibility

The v0.2 registration includes an `outcome_snapshot` with release ID/digest, source/target-definition/license references, evaluation-as-of index and one explicit row for every declared forecast ID. Each row is either:
- `matured`: a nonnegative integer observed count and an availability index strictly after the target period and no later than evaluation-as-of; or
- `unknown`: both observed count and availability index are null.

Missing, duplicated or dangling rows fail closed. Unknown outcomes are not zeros: their scores are null and they are excluded from both raw and reconciled CRPS means. Raw/reconciled comparisons use the identical list of mature forecast IDs and the same frozen outcome release.

## Receipt and forecast/outcome separation

The receipt includes:
- raw schema and input hashes;
- a forecast-inventory commitment over the provider/release descriptor, target/index policy, reconciliation policy, hierarchy constraints, forecast rows and raw/reconciled sample bytes;
- an independent digest of the outcome snapshot;
- outcome release ID, upstream release digest and evaluation-as-of index;
- per-forecast raw CRPS, reconciled CRPS and raw-minus-reconciled improvement;
- eligible/matured denominator, unknown outcome count, mean raw/reconciled CRPS and mean paired difference;
- exact source commit, dependency lock digest, Rust/Cargo versions and a receipt commitment over the entire evaluation identity.

An outcome-only revision must leave the forecast-inventory commitment unchanged while changing the outcome snapshot, full input and receipt commitments. This makes outcome-vintage sensitivity explicit instead of silently treating a revised label as a different forecast.

## Synthetic golden fixture

The checked-in `docs/fixtures/pirb-hierarchy-sample-score-synthetic-v0.2.json` contains invented counts, two finite sample draws, a placeholder outcome-release digest and seven invented matured outcome counts plus one unknown outcome. No real ACLED or other provider output is included.

Expected fixture output:
- 8 forecast inventory nodes, 2 draws;
- 7 mature scored rows, 1 unknown row;
- mean raw empirical CRPS = (2/7);
- mean reconciled empirical CRPS = (1.5/7);
- mean raw-minus-reconciled CRPS = (0.5/7).

These values test formula plumbing only. They are not benchmark evidence of predictive skill, calibration, uncertainty coverage or superiority. A positive synthetic paired CRPS difference does not establish a real model improvement.

## Required next benchmark stage

For authorized mature historical data, pre-register rolling-origin folds, outcome-vintage policy, estimand and common-row policy, score both raw/reconciled samples on the exact same mature IDs, and report horizon/level-specific CRPS with dependence-aware uncertainty. Keep marginal calibration, interval coverage, joint dependence scores, adjustment magnitude and coherence diagnostics separate. Treat incomplete hierarchy constraints as not-assessable, not as zeros. Do not translate a CRPS difference into an operational claim without evaluating the warning/response policy at fixed capacity and known costs.

## Claim ceiling

Passing the v0.2 contract means the input was valid and descriptive marginal CRPS was calculated reproducibly for the declared synthetic or authorized outcome snapshot. It does not prove data authority, population representativeness, model skill over a baseline, nominal calibration, causal peace impact, harm reduction or deployment safety.