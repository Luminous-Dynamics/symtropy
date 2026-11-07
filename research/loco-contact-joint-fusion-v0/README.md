# LOCO Joint Contact Fusion V0

Standalone, dependency-free Rust reference implementation for [LOCO-CONTACT-FUSION-001 / #4017](https://github.com/Luminous-Dynamics/symtropy/issues/4017).

## Joint observation model

For a three-dimensional latent contact state:

    x ~ Normal(mu, P)
    y = H x + epsilon
    epsilon ~ Normal(0, R)

V0 supports one to three channels in a single verified contact event. The full, calibrated covariance R includes cross-channel correlation. Its channel order is bound explicitly through `noise_covariance_sensor_ids`.

    S = H P H^T + R
    K = P H^T S^-1
    mu_post = mu + K (y - H mu)
    P_post = P - K H P
    EIG = 0.5 * (logdet(S) - logdet(R))

The implementation uses Cholesky factorization/solves and log determinants; it does not form an unchecked matrix inverse.

## Evidence contract

- Samples must share one contact event, foot, verified clock and exact acquisition timestamp.
- Sample arrival and age are checked against the caller's current time and maximum age.
- Sensor IDs, evidence IDs, measurement-model IDs, coordinate-frame IDs, units and calibration IDs are required.
- The covariance matrix row/column order is explicitly mapped to sensor IDs; sample reordering is handled by canonical sensor ordering and a consistent covariance permutation.
- Duplicate evidence/sensor IDs, mismatched covariance manifests, stale data, asynchronous samples and non-SPD covariance are rejected.

Asynchronous data must first be propagated to a common time by a separately declared/qualified dynamics model. V0 does not interpolate or silently fuse different timestamps.

## Limits

This is a small fixed-dimensional Gaussian reference oracle. It validates the supplied observation and covariance model; it does not estimate sensor covariance, calibrate units/frames, or prove that an experimental sensor model is valid. The state coordinates are generic and must be declared by the benchmark fixture.

## Validation

    cargo fmt --manifest-path research/loco-contact-joint-fusion-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-contact-joint-fusion-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-contact-joint-fusion-v0/Cargo.toml --locked --all-targets -- -D warnings
