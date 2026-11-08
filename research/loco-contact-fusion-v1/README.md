# LOCO Contact Fusion V1

Dependency-free Rust reference for joint Gaussian fusion of synchronized, correlated legged-contact channels.

Parent: [LOCO-CONTACT-FUSION-001 / #4017](https://github.com/Luminous-Dynamics/symtropy/issues/4017).

This is a V1 extension to the scalar/action-qualified contact-information work in [#4008](https://github.com/Luminous-Dynamics/symtropy/pull/4008). The V0 shared conditional-noise-independence contract remains intact. V1 supplies a separate joint-observation path for a calibrated full cross-channel covariance; it does not assert independence where it has not been established.

## Model

For a three-dimensional declared latent contact state and m synchronized channels (1 <= m <= 3):

    x ~ N(mu, P)
    y = H x + epsilon
    epsilon ~ N(0, R)

R is a calibrated, symmetric-positive-definite m-by-m covariance containing cross-channel terms. The joint update uses:

    S = H P H^T + R
    K = P H^T S^-1
    mu_post = mu + K (y - H mu)
    P_post = P - K H P
    EIG = 0.5 * (logdet(S) - logdet(R))

The implementation uses Cholesky substitutions to solve S K^T = (P H^T)^T; it does not explicitly form S inverse. Log determinants are computed from Cholesky factors.

The diagonal-R case is tested against the independent scalar-update reference. Non-zero off-diagonal R is processed jointly and must not be treated as independent measurements.

## Evidence and alignment

Every joint group binds to:
- exact contact-event and foot identity;
- stable evidence, sensor, model and channel identities;
- one declared action and an upstream action-execution receipt;
- one source clock and verified clock-mapping receipt;
- one common corrected acquisition timestamp;
- arrival time and a bounded sample age;
- per-channel coordinate frame, units and calibration identity;
- observation-model digest;
- covariance-calibration ID and canonical channel-order manifest.

Input sample vectors can arrive in any order: samples are mapped by stable sensor ID into the model's canonical channel order. The model rows, R rows/columns and channel manifest share one explicit order. A mismatched manifest, wrong sensor/channel mapping, duplicate/replayed evidence, absent channel, wrong action/actuator-model receipt, unverified clock mapping, or asynchronous acquisition time is rejected.

A sample that arrives late but still falls inside the declared freshness bound is allowed. A sample whose source acquisition time differs from another channel is not converted into a simultaneous vector; it is rejected. No interpolation, zero imputation or timestamp substitution occurs.

## Implemented fixtures

Eighteen deterministic tests cover:
- scalar one-channel known-answer posterior and EIG;
- diagonal-R equivalence to independent scalar updates;
- a deliberately correlated same-signal pair showing that a diagonal-independence assumption is overconfident;
- sample-order invariance;
- asymmetric, singular and non-positive covariance rejection;
- missing-channel non-imputation;
- asynchronous sample rejection even when arrivals are close;
- wrong clock-mapping receipt and unverified clock rejection;
- stale samples and arrival-before-acquisition rejection;
- action-execution receipt and actuator-model binding;
- covariance/channel manifest mismatch;
- model/calibration digest mismatch;
- duplicate sensor/evidence IDs;
- required covariance-calibration identity;
- inclusive sample-age and clock-mapping validity boundaries;
- non-finite values and invalid prior covariance;
- Cholesky solve and log-determinant known answers.

The fixtures are synthetic and validate the declared equations and evidence checks only.

## Commands

    cargo fmt --manifest-path research/loco-contact-fusion-v1/Cargo.toml -- --check
    cargo test --manifest-path research/loco-contact-fusion-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-contact-fusion-v1/Cargo.toml --locked --all-targets -- -D warnings

Dedicated exact-head workflow: `.github/workflows/loco-contact-fusion.yml`.

## SOTA and fair comparison

Relevant baselines:
- Zhao et al. (Sensors, 23 September 2026): continuous contact reliability, neural residual state/observation corrections and Kalman covariance regulation. The authors' hardware study is indoor level-ground walking/turning; the paper identifies complex terrain as future validation. https://doi.org/10.3390/s26196041
- Latent Regime Mixture-of-Experts quadruped odometry (IEEE RA-L, 28 August 2026): reported 14.2% velocity-RMSE reduction on ANYmal, 28.2% drift reduction on Unitree Go1 and over 240 Hz on Jetson. This is a state-estimation baseline, not evidence of contact-safety certification. https://doi.org/10.1109/LRA.2026.3728379
- SafeMind (IEEE Access 2026): probabilistic CBFs with uncertainty-aware barriers, reported on Unitree A1 and ANYmal C at 200 Hz. Its reported violation/energy improvements require matched benchmarking rather than direct comparison with a different morphology or scenario. https://doi.org/10.1109/ACCESS.2026.3679520
- Jia et al. (Artificial Intelligence Review, 2026): D1/D2/D3 contact sensing and F1/F2/F3 controller-facing products. https://doi.org/10.1007/s10462-026-11644-0

The V1 research claim is intentionally narrow: calibrated cross-channel covariance can be used for a joint update without falsely multiplying correlated scalar likelihoods. It is not a new Kalman filter class, contact sensor, terrain estimator or safety controller.

## Claim ceiling

This crate checks arithmetic for supplied H, R and source-qualified evidence identities. It does not fit H/R, prove covariance calibration, authenticate upstream receipts cryptographically, infer physical friction, establish conditional/ood coverage, certify support/traction, or validate a real robot. Hardware correlation claims require measured cross-channel calibration; biological claims require anatomy-grounded channels and source-qualified animal measurements.