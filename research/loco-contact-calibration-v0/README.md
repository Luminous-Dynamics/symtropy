# LOCO Contact Calibration V0

Dependency-free Rust reference for exact fixed-point Brier, expected calibration error (ECE), maximum calibration error (MCE) and reliability-bin summaries for a continuous contact-reliability estimator.

Research parent: [LOCO-CONTACT-CALIBRATION-001 / #4148](https://github.com/Luminous-Dynamics/symtropy/issues/4148).

## One invocation, one declared stratum

Every evaluation run is bound to one exact:
- sensing channel (D1 interface fields, D2 near-foot wrench/vibro-acoustic, or D3 proprioceptive residual);
- target (contact presence, no-slip kinematic-constraint validity, or traction feasibility);
- estimator/model digest;
- terrain/operating-domain digest;
- evaluation split digest.

Mixed strata are rejected. To produce per-channel or per-target reports, run the evaluator separately for each stratum. Do not pool them into one headline score.

Each observation also has an episode ID and independent session/trial unit ID. Hundreds of correlated timestamps can contribute to the observation-level score, but they do not inflate the independent-unit count. Use session/trial units for statistical confidence intervals and clustered analysis.

## Exact integer metrics

Predicted probabilities are integer parts per million, in [0, 1,000,000]. For n observations and binary target y_i in {0,1}, V0 computes:

    Brier = sum((p_i - y_i * 1,000,000)^2) / (n * 1,000,000^2)

The result is returned as an exact numerator/denominator pair.

For equal-width reliability bins, the in-bin absolute calibration gap is:

    gap_b_ppm = abs(sum(p_i) - positive_count_b * 1,000,000) / count_b

Expected calibration error in ppm is returned exactly as:

    ECE_ppm = sum_b(abs(sum(p_i) - positive_count_b * 1,000,000)) / n

MCE reports the largest exact per-bin gap and the index/count of that bin. Empty bins are preserved. The exact reliability-bin accumulators accompany every aggregate; ECE/MCE must never be quoted without the binning configuration and reliability diagram.

All accumulators and cross-products use checked u128 arithmetic and fail closed on overflow. Input order does not affect the report.

## Split and identity controls

- Duplicate observation IDs are rejected.
- Fit/test-reserved observation IDs cannot enter the evaluation input.
- An episode cannot be mapped to more than one independent session/trial unit.
- Model, domain, calibration split, sensing channel and target must match the declared dataset stratum.
- Probabilities outside [0,1] and invalid bin counts are rejected.

The supplied binary outcomes are assumed to be independently adjudicated. V0 does not prove label correctness.

## Implemented tests

Fifteen deterministic fixtures cover perfect 0/1 forecasts, exact quarter Brier score, overconfident wrong forecasts, bin boundaries, empty bins, all-positive/all-negative strata, correlated observations within one session, duplicate IDs, episode/session identity, fit/test leakage, mixed strata, invalid inputs, permutation invariance and accumulator overflow.

## Commands

    cargo fmt --manifest-path research/loco-contact-calibration-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-contact-calibration-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-contact-calibration-v0/Cargo.toml --locked --all-targets -- -D warnings

Dedicated exact-head workflow: .github/workflows/loco-contact-calibration.yml.

## Research baselines

- Zhao et al. (Sensors 26(19), 6041; 23 September 2026): continuous contact reliability plus neural residual Kalman filtering, https://doi.org/10.3390/s26196041
- Cheng et al. (ICRA 2026): simultaneous covariance and kinematics calibration, https://www.leggedai.com/publication/2026icra_calibration/
- IUCA (IEEE Journal of Selected Areas in Sensors, 7 September 2026): model-agnostic correlated-state uncertainty calibration, https://doi.org/10.1109/JSAS.2026.3731931
- Jia et al. (Artificial Intelligence Review, 2026): contact sensing channels and controller-facing feedback targets, https://doi.org/10.1007/s10462-026-11644-0

## Claim ceiling

V0 computes descriptive calibration metrics from supplied probabilities and binary labels. Brier is a proper scoring rule but combines calibration, resolution and uncertainty; ECE/MCE depend on bin count and edges. These numbers are not a safety probability, a distribution-free coverage statement, proof of estimator correctness, or a physical contact/traction certificate. Statistical uncertainty must respect session/trial clustering; correlated timestamps are not independent Bernoulli trials.