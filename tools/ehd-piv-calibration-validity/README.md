# EHD-PIV calibration validity supervisor

Dependency-free Rust reference kernel for a revocable, scope-bound PIV uncertainty-calibration capability.

## What it does

- Binds the admitted claim to predictor, preprocessing, acquisition, instrument, calibration, clock, reference-graph and observer-state fingerprints.
- Checks a frozen feature envelope and certificate age.
- Rejects calibration/test leakage by dependence-cluster identity.
- Suspends the certificate on hard configuration, reference, observer or conditional-residual invalidators.
- Allows a covariate-shift warning only when a weighted-covariate-shift method was declared, an external conditional-law stability evidence ID is supplied, and weight-support evidence meets the frozen ESS, maximum-weight and overlap policy.
- Prevents automatic recovery using the same certificate; a new certificate and declared-disjoint calibration and sealed-test clusters are required.

## What it does not do

This is V0 policy/state-machine code. It does not fit a PIV model, calculate conformal quantiles, estimate density ratios, detect drift, prove exchangeability, authenticate digests or receipts, run a camera, validate a physical reference, or establish EHD performance. The external_evidence_verified field is an assertion from the caller, not a cryptographic proof.

A density-ratio estimate alone does not establish the conditional-law stability assumption for weighted conformal. The conditional-law evidence ID is an external witness reference, not a verified theorem or proof. Static split-conformal, weighted covariate-shift conformal, subgroup-conditional conformal and online adaptive conformal are distinct statistical methods. This crate does not collapse their assumptions into a universal coverage guarantee.

## Run

    cargo fmt --manifest-path tools/ehd-piv-calibration-validity/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-piv-calibration-validity/Cargo.toml
    cargo clippy --manifest-path tools/ehd-piv-calibration-validity/Cargo.toml --all-targets -- -D warnings

All claims remain limited to deterministic policy behavior under manufactured evidence inputs.
