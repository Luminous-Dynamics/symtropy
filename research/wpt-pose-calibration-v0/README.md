# WPT contact-conditioned pose-set calibration v0

**Status:** exact finite-sample split-conformal threshold and candidate-set filter over upstream-supplied joint pose nonconformity scores. This is not a pose estimator, sensor model, mechanical solver, shift detector, safety assessor or permit.

## Research positioning

Conformal calibration itself is established prior art. Required baselines are:
- IUCA (IEEE JSAS, Sep 2026): in-situ model-agnostic calibration of correlated robot-pose uncertainty.
- LCP (ICRA 2026): contextual nonconformity scoring for robotic perception and planning.
- CaPTURe (PMLR 329, 2026): contact-aware particle-set calibration in stratified configuration spaces.
- RSS 2026 polytopic UQ for SLAM: deterministic set propagation combined with conformal measurement calibration.
- Conformal Policy Control (ICML 2026): calibrated policy-risk regulation against a safe reference.

The potential contribution is WPT/aircraft-specific composition across mechanical capture, joint 6-DOF pose scores, receiver revisions, contact regimes, motion/model validity and energy-service consequences.

## Mathematical contract

For n independently sampled calibration episodes and target miscoverage alpha, use the non-randomized split-conformal upper order statistic:

    k = ceil((n + 1) * (1 - alpha))
    q = k-th smallest calibration score

If k > n, no finite threshold exists and the evaluator abstains. It never substitutes the maximum available score.

At inference, retain every pose/model/fault tuple with score <= q. The resulting set has nominal **marginal** coverage only under exchangeability of calibration and test episodes. A singleton set is not a conditional probability of correctness.

## Input contracts

Calibration file header:

    calibrator_id evidence_class stratum_id sampling_unit partition alpha_ppm metric_sha256 predictor_sha256 hardware_sha256 environment_sha256 calibration_episode_id score_micros valid_until_us

Inference file header:

    inference_id calibrator_id evidence_class stratum_id sampling_unit partition decision_time_us alpha_ppm metric_sha256 predictor_sha256 hardware_sha256 environment_sha256 shift_status observation_sha256 pose_id model_id fault_id score_micros valid_until_us

The score is upstream supplied as a fixed-point joint pose nonconformity value. metric_sha256 identifies the score normalization (including translation/attitude combination and modality-specific measurement semantics). This crate does not calculate physical distances or sensor residuals.

One row is one independent episode, not a frame or timestep. The parser verifies the declared sampling-unit token and unique calibration IDs, but cannot prove statistical independence.

## Stratum and shift rules

Calibration must be bound to exactly one stratum and matching metric, predictor, hardware, environment, evidence class and alpha. Inference must match those identifiers and occur before the minimum calibration validity horizon.

The inference case carries an upstream shift disposition:
- SUPPORTED: apply the locked calibrator;
- SHIFT_DETECTED: abstain;
- UNKNOWN: abstain.

This crate does not detect shift. Ordinary split conformal does not guarantee coverage under arbitrary distribution shift. Weighted/online conformal, calibration transfer and context-adaptive score methods need separate assumptions and benchmarks.

## Outputs

- CALIBRATED_SINGLE_POSE_SET_MARGINAL_COVERAGE_ONLY
- CALIBRATED_MULTI_POSE_SET
- EMPTY_CALIBRATED_SET
- ABSTAIN_INSUFFICIENT_CALIBRATION
- ABSTAIN_SHIFT_DETECTED
- ABSTAIN_SHIFT_UNKNOWN

A finite threshold at alpha=0.0001 requires at least 9,999 independent calibration episodes in this non-randomized construction. Repeated frames or correlated samples from the same flight do not count as independent episodes.

## Run

    cargo test --locked --manifest-path tools/wpt-pose-calibration/Cargo.toml
    cargo run --locked --manifest-path tools/wpt-pose-calibration/Cargo.toml -- research/wpt-pose-calibration-v0/v-guide-calibration.tsv research/wpt-pose-calibration-v0/v-guide-inference.tsv

## Claim ceiling

A pass validates order-statistic arithmetic, data integrity and finite candidate filtering only. It does not validate score quality, exchangeability, stratum support, model/fault completeness, physical localization, contact mechanics, electrical transfer, safety or operational authority.
