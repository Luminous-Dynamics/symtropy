# LOCO Contact Safety V0

Dependency-free Rust reference implementation for [LOCO-CONTACT-SAFETY-001 / #4010](https://github.com/Luminous-Dynamics/symtropy/issues/4010).

## Statistical contract

For five one-sided constraints, a candidate action supplies five predictive marginal distributions with calibrated Gaussian means and variances. The family-wise risk budget is alpha, split by Bonferroni into alpha/5 per constraint.

    z = Phi^-1(1 - alpha/5)

Upper constraints use mean + z * sd; support margin uses mean - z * sd. The candidate is admitted only if every bound satisfies its hard limit. By the union bound, when each calibrated marginal is valid for the declared stratum, the probability of any one of the five violations is no greater than alpha. This does not assume independence between the five constraints.

## Inputs

Every prediction must carry non-empty predictor, model digest, calibration ID and calibration-stratum ID, and be explicitly marked calibrated for that stratum. Otherwise the library returns an error and withdraws the probability guarantee.

Variances must be finite and non-negative; zero variance is a deterministic limit. Alpha must be finite and in the supported open interval (1e-12, 1). Physical units and the meaning of each constraint are fixed by the enum and its versioned caller-side model.

## Limits

This is a one-step marginal-Gaussian chance filter with Bonferroni correction. It is not a joint-distribution model, a CBF, a forward-invariance proof, a formal safety case or a robot certificate. Correctness depends on held-out calibration, predictor scope, action delivery, and the declared physical model. Heavy tails, distribution shift and calibration drift must cause abstention or require an appropriate non-Gaussian predictive model; the software does not validate calibration by itself.

## Validation

    cargo fmt --manifest-path research/loco-contact-safety-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-contact-safety-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-contact-safety-v0/Cargo.toml --locked --all-targets -- -D warnings
