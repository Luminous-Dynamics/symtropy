# Correlation-aware active sensing: prototype v0.1

**Status:** research implementation slice; not compiled or physically validated.  
**Parent:** NAV-OBSERVATION-001 #6438.  
**Independent-noise prototype:** draft PR #6471.

## Paradigm shift: source count is not information count

A set of measurements can have many IDs and still contain little independent information. Two receivers may share a clock, antenna, correction service, RF front-end, calibration, power source, anchor survey or mapping prior. Treating every observation as independent inflates the Fisher information matrix and makes an active-sensing planner overvalue redundant actions.

This prototype introduces an explicit, narrow common-mode model:

`R = diag(\sigma_{private,i}^2) + \sum_g \tau_g^2 1_g 1_g^T`

`J = H^T R^{-1} H`

The covariance is Cholesky-factorized; the implementation does not form a matrix inverse. It compares a naïve independent-noise model against the declared shared scalar error model.

## What is implemented

- Correlated scalar range actions with explicit shared-error group and magnitude.
- Strict validation of IDs, bearings, variances, group identities, within-group parameter consistency and resource cost.
- Cholesky-based solution of `R x = H`, followed by exact local linearized `J = H^T R^-1 H`.
- Exhaustive subset oracle (up to 16 candidate actions) and greedy baseline, with D/A/E-optimal criteria.
- Eight authored tests for repeated-observation overcounting, cross-axis correlation, common-mode saturation, distinct groups, permutation invariance, inconsistent declarations, selector consistency and fail-closed inputs.

## Known-answer result

For two identical same-bearing observations, each with independent standard deviation 1 m and a shared common-mode standard deviation 1 m:

- Naïve independence: information (J_{xx}=2).
- Shared-error model: (R=[[2,1],[1,2]]), so (J_{xx}=2/3).
- The naïve model overstates this directional information by a factor of 3.

For n same-bearing observations with private sigma 0.1 m and common-mode sigma 1 m, information is (n/(0.1^2+n)), so it approaches 1 rather than growing without limit.

These are analytical known-answer checks and test expectations, not test-run results.

## Claim boundaries

- One scalar additive common-mode error per explicitly declared group is not a complete sensor-correlation model.
- No inference of independence is made from distinct IDs or modalities; group identity and sigma are explicit inputs requiring upstream evidence.
- This is a local linearized 2D range model. It omits general block correlation, time-varying dependencies, nonlinear relinearization, NLOS mixture distributions, anchor survey error, clock-state augmentation and physical reachability.
- Fisher information is not a calibrated posterior covariance, protection level or integrity certificate.
- No physical sensing, movement or actuation is included.
- The code and tests are authored but not compiled/executed in this environment.

## Next research step

Replace the single shared additive-error group with a validated multi-source dependency graph and a general structured covariance or latent-variable model. Compare that model with conservative fusion when cross-correlations are only bounded or unknown. Keep the provenance/common-mode authority in Mycelix; this package consumes a pre-qualified dependency model rather than becoming another provenance ledger.
