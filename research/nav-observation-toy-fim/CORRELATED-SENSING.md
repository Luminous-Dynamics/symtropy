# Correlation-aware active sensing: prototype v0.1

**Status:** research implementation slice; not compiled or physically validated.  
**Parent:** NAV-OBSERVATION-001 #6438.  
**Independent-noise implementation:** stacked on clean-base FIM PR #6471.  
**Ownership:** consumes explicit dependency facts; Mycelix remains the source-provenance/common-mode owner.

## Paradigm shift: source count is not information count

A set of measurements can have many IDs and still contain little independent information. Two receivers may share a clock, antenna, correction service, RF front-end, calibration, power source, anchor survey or mapping prior. Treating every observation as independent inflates the Fisher information matrix and makes an active-sensing planner overvalue redundant actions.

The prototype introduces a deliberately narrow, explicit common-mode additive error model:

`R = diag(\sigma_{private,i}^2) + \sum_g \tau_g^2 1_g 1_g^T`

`J = H^T R^{-1} H`

The covariance is Cholesky-factorized; the implementation does not form a matrix inverse. It compares a naïve independent-noise model against the declared shared-error model.

## Implemented

- Correlated scalar range actions with explicit shared-error group and magnitude.
- Fail-closed validation of IDs, bearings, private variances, group IDs, within-group common-mode sigma consistency and resource costs.
- Cholesky solution of `R x = H`, followed by local linearized `J = H^T R^{-1} H`.
- Exhaustive subset oracle (up to 16 actions) and greedy baseline using D/A/E-optimal criteria.
- Eight authored tests: repeated-observation overcounting, cross-axis correlation, common-mode information saturation, independent groups, permutation invariance, inconsistent group parameters, selection behavior and fail-closed missing parameters.

## Known-answer result

For two identical same-bearing observations, each with private standard deviation 1 m and shared common-mode standard deviation 1 m:

- Naïve independence: (J_{xx}=2).
- Shared-error model: (R=\begin{bmatrix}2&1\\1&2\end{bmatrix}), yielding (J_{xx}=2/3).
- The naïve model overstates directional information by a factor of 3.

For (n) same-bearing observations with private sigma 0.1 m and common-mode sigma 1 m:

[
J_{xx}=\frac{n}{0.1^2+n}
]

Information approaches 1 rather than growing without limit.

These are analytical known-answer checks and test expectations, not test-run results.

## Direct research comparators

1. **Robust Sensor Geometry Design via Sensitivity-Based Information Analysis**, IEEE Signal Processing Letters 33 (10 June 2026), DOI [10.1109/LSP.2026.3702588](https://doi.org/10.1109/LSP.2026.3702588). Uses a unified FIM for bearing/range/range-difference measurements and chooses lower-sensitivity geometries within an information-retention region; the abstract reports retaining around 90% of maximum information while reducing perturbation sensitivity in its three-sensor numerical example. This suggests adding sensitivity-to-bearing, sigma, anchor-position and common-mode perturbations as a separate design objective instead of maximizing nominal FIM alone.

2. **Data-driven sensor placement for predictive applications: a Correlation-Assisted Attribution Framework (CAAF)**, Communications AI & Computing (5 August 2026), [article](https://www.nature.com/articles/s44488-026-00011-1). Clusters correlated candidate sensor locations before feature attribution to reduce redundancy. It is an adjacent learned sensor-placement baseline; compare against explicit covariance modelling on identical candidates, costs and held-out configurations.

3. **Difference-of-convex algorithm for UWB cooperative localization using multiplicative factors in harsh indoor environments**, Measurement 290 Part C (15 November 2026), DOI [10.1016/j.measurement.2026.122957](https://doi.org/10.1016/j.measurement.2026.122957). The abstract describes sparse/rank-constrained multiplicative NLOS factor estimation and reports most-tag errors within 10 cm at 30% NLOS in its field tests. Exact data, baselines, hardware and license remain to be audited.

4. **A decoupled backend optimizer for UWB localization in NLOS environments**, Measurement 276 (2 June 2026), DOI [10.1016/j.measurement.2026.121494](https://doi.org/10.1016/j.measurement.2026.121494). Uses front-end estimates, ranging observations and anchor geometry for reusable posterior refinement, reporting numerical and offline gains under its tested static occlusion and poor-layout conditions. Reproduce LS, RICT and Shen baselines as described in the paper where artifacts permit.

5. **Multi-sensor fusion localization framework using UKF-LSTM with Multiple Model Adaptive Estimation**, Robotics and Autonomous Systems 202 (August 2026), DOI [10.1016/j.robot.2026.105491](https://doi.org/10.1016/j.robot.2026.105491). This is a learned/adaptive fusion comparator to include in the full localization track, not a substitute for explicit common-mode accounting.

These papers are method comparators, not results reproduced by Luminous. Match sensor privileges, calibration, routes, train/test separation, truth source, tuning and failure accounting before comparison.

## Limitations

- One scalar additive common-mode error per declared group is not a complete sensor-correlation model.
- No independence is inferred from IDs or modalities. Group membership and sigma are explicit modelling inputs requiring upstream provenance evidence.
- The toy omits arbitrary block correlations, time-varying dependencies, nonlinear relinearization, NLOS mixture distributions, anchor survey error, explicit clock-state augmentation and physical reachability.
- Fisher information is not calibrated posterior covariance, a protection level or an integrity certificate.
- No physical sensing, movement or actuation is included.
- Rust code and eight tests are authored but have not been compiled or executed in this environment.

## Next research step

Replace the scalar shared-error groups with a validated dependency graph and structured latent-variable/covariance models. Compare known-correlation FIM against conservative CI or bounded-correlation methods when correlations are unknown. Add sensitivity-aware D/A/E design and evaluate whether the selected observation actually reduces position error, false-safe exposure or recovery time.

The provenance/common-mode authority remains in Mycelix; this package consumes an explicit model and must not become another provenance ledger.
