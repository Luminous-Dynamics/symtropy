# HBR-PGD reproduction kernels: HQRM and SRSC

**Status:** two deterministic reference kernels implemented; exact-head CI and paper/data reproduction pending. No physical data has been admitted or processed by this increment.

**Reproduction work package:** [#6932 — HBR-PGD on Bregar with integrity extensions](https://github.com/Luminous-Dynamics/symtropy/issues/6932).

**Primary paper:** Yu, Liu and Qian, *Heteroscedastic Bias-Robust Projected Gradient Descent for UWB Localization in Complex Indoor Environments*, Sensors 26(14), 4578 (19 July 2026), DOI 10.3390/s26144578. Full text: https://pmc.ncbi.nlm.nih.gov/articles/PMC13418692/.

## Scope and source limits

The paper reports 2D single-tag localization with eight fixed anchors, six channels, DW1000/DWM1000 hardware and ADS-TWR, on Bregar environment0, environment1 and environment2. The paper's reported RMSE is 0.107 m, 0.068 m and 0.204 m; its reported P95 is 0.181 m, 0.118 m and 0.341 m, respectively. These are paper-reported outcomes, not results of these Rust kernels.

The article references Bregar's 2023 dataset DOI 10.1038/s41597-023-02639-5. It does not clearly identify which Zenodo payload version was consumed. This programme must not silently equate the paper's snapshot with the currently registered Bregar v2.0 candidate. Rights, exact release, preprocessing and calibration/test split must be reconciled before reproducing any number.

The paper's HBR-HQRM calibration weights are grid-searched on a calibration set. The available paper text does not fully pin all quality-feature weights, logistic slopes and calibration/test partition details. The code therefore accepts frozen caller-supplied parameters and refuses to invent paper values.

## Module 1: HBR-HQRM packet-quality mapping

Source: paper equations (5)–(8).

For packet quality vector q, calibration feature means mu, positive standard deviations s, and frozen weights w:

`q_tilde = sum_j w_j * ((q_j - mu_j) / s_j)`

Then:

`sigma = sigma_min + (sigma_max - sigma_min) * sigmoid(-k_sigma * q_tilde)`

`alpha = alpha_min + (alpha_max - alpha_min) * sigmoid(k_alpha * q_tilde)`

`delta_hat = delta_max * sigmoid(-k_delta * q_tilde)`

The kernel:
- validates exact feature/mean/scale/weight dimensionality;
- validates finite inputs, positive standard deviations and ordered parameter bounds;
- uses a numerically stable sigmoid;
- validates nonzero calibration and feature-schema SHA-256 commitments;
- returns the quality score, residual scale, robust-loss shape and nonnegative correction prior with both commitments.

The mapping convention assumes larger q_tilde means better quality. The actual feature weights and scaling determine that convention and must come from a frozen calibration artifact. This module does not estimate those weights or look at LOS/NLOS labels.

The paper table includes sigma bounds 0.052–0.200 m and alpha bounds [-1.5, 1.8]. Those may be used as a paper-reference configuration only after exact feature and calibration semantics are recovered. They are not silently installed as a default model.

## Module 2: HBR-SRSC anchor-channel soft gate

Source: paper equations (10)–(13).

For each declared time/anchor/channel group, using the initial geometric residuals r_p and packet-level nonnegative priors delta_hat_p:

- residual centre = median(r_p);
- residual spread = raw median absolute deviation, MAD(r_p);
- group correction prior = median(delta_hat_p);
- anomaly score = beta1*abs(centre) + beta2*spread + beta3*abs(median_prior);
- gate = sigmoid(gamma*(anomaly_score - threshold)).

The raw MAD is used as stated in the paper; no 1.4826 Gaussian-consistency scaling is silently introduced.

The gate is precomputed and frozen before the main trajectory/bias optimization. It is not an optimizable state. Each result carries source-observation-manifest and quality-mapping commitments. Groups are returned in canonical identity order.

## Module 3: HBR-BCTR objective, gradient and projected step

The companion `hbr_bctr_objective.rs` implements the local 2D objective and analytic gradients for the paper's trajectory state X and anchor-channel structural bias B.

For each packet:

`e = z - ||x_t - s_a|| - b_tac - m_tac * delta_hat_tacp`

`u = e / (sigma_tacp + epsilon)`

The data term is the generalized adaptive robust loss `rho_alpha(u)`. The current reference supports alpha below 2, including the logarithmic alpha=0 limit. It returns the loss and derivative needed for analytic position and bias gradients.

The remaining terms are:
- velocity smoothness: `lambda_v * sum ||x_t - x_(t-1)||^2`;
- acceleration smoothness: `lambda_a * sum ||x_t - 2*x_(t-1) + x_(t-2)||^2`;
- bias amplitude: `lambda_b * sum b_tac^2`;
- bias temporal drift: `lambda_b * eta * sum (b_tac - b_(t-1),ac)^2`.

The objective evaluator returns total cost and analytic gradients for the 2D position and flattened (time, anchor-channel group) bias states. A separate projected step applies caller-supplied position/bias step sizes, constrains positions to a declared spatial box and clamps structural bias to [0, b_max].

Finite-difference tests compare analytic position and bias gradients against objective perturbations. Other tests exercise the alpha=0 and near-zero robust-loss limits, projection bounds, invalid states and arithmetic overflow.

This is **not yet the full HBR-PGD reproduction**. It does not create overlapping windows, choose window overlap, implement the paper's curvature/trace-based fusion, derive the initial geometric trajectory, reconstruct the complete feature and split protocol, or implement the authors' exact stopping/line-search details. The objective and gradient are a numerical reference to build and compare; no RMSE, runtime or physical performance result is claimed.

## What is not yet implemented

These modules are only the two upstream precomputation blocks. They do not implement:
- overlapping-window construction and curvature-weighted fusion;
- the complete paper-faithful optimization/termination pipeline beyond the current local-window objective, analytic gradients and projected step;
- overlapping-window information-weighted fusion;
- author-specific missing/invalid row policy;
- exact paper feature selection, calibration weights, logistic slopes, gate parameters or train/test partition;
- controller/evaluator process isolation.

Those belong in later increments after source and protocol pinning.

## Reproduction protocol to preserve

The paper uses three selected Bregar environments:
- environment0: residential apartment, 85 points, 126,480 raw and 57,120 valid records;
- environment1: small apartment, 80 points, 119,040 raw and 53,760 valid records;
- environment2: workshop/industrial, 84 points, 124,992 raw and 49,392 valid records.

The paper describes removing invalid/unavailable observations according to dataset labels and evaluation protocol. Rebuild this selection independently and record every row and reason. The controller-visible event sequence must remain invariant under evaluator-truth mutation. NLOS labels may be used for evaluator-only stratification, never as controller-side quality inputs or selection oracles.

A fair reproduction requires exact source version, parser, trajectory order, calibration/validation/test partition, anchors, channels, accepted ranges, feature standardization, solver parameters and stopping rules. No random row split is acceptable; preserve trajectory/position groups and use held-out environments as a separate shift study.

## Assurance extensions after reproduction

Keep the conventional HBR output intact and compare:
1. HBR-PGD alone;
2. HBR-PGD plus the independent range-set posterior audit;
3. HBR-PGD plus the group-calibrated positive NLOS-bias interval;
4. HBR-PGD plus common-cause/fault-hypothesis outer envelope;
5. HBR-PGD plus active hypothesis-discriminating acquisition (simulated/conditional at first).

Do not change the paper's published objective while claiming faithful reproduction. Report estimator accuracy, uncertainty coverage, false-safe exposure, availability, false alerts, compute and energy on separate axes.

No HBR-PGD baseline or SOTA reproduction result is currently claimed by Luminous Dynamics.
