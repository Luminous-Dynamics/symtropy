# EHD PIV reference-aware covariance decomposition

This standalone Rust crate implements the covariance identity required to distinguish PIV measurement error from reference-instrument and shared-cause error:

`C_residual = C_piv + C_reference - C_piv,reference - C_reference,piv`

It never assumes that PIV and the reference are independent. For a qualified reference assessment and a quantified cross-covariance, it derives:

`C_piv = C_residual - C_reference + C_piv,reference + C_piv,referenceᵀ`

## Run

    cargo fmt --manifest-path tools/ehd-piv-reference-uq/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-piv-reference-uq/Cargo.toml
    cargo clippy --manifest-path tools/ehd-piv-reference-uq/Cargo.toml --all-targets -- -D warnings

The crate is a standalone Cargo workspace and has no third-party dependencies.

## Qualification behavior

- If reference uncertainty, traceability, bandwidth, timing, registration, drift, shared causes, or PIV/reference cross-covariance are missing or unqualified, the evaluator returns `ResidualCovarianceOnly` and leaves the PIV-only covariance absent.
- If evidence is qualified, it checks the residual and reference covariance, derives the PIV covariance, and checks the complete 6×6 joint PIV/reference covariance block.
- The full joint covariance check prevents mathematically impossible marginal/cross-covariance combinations from being accepted.
- Covariance tests operate in a dimensionless correlation representation, preserving PSD decisions under independent component-unit scaling.
- No covariance is clipped, projected, regularized or repaired.

## Evidence boundary

The evidence fields are declarations supplied by an external assessment workflow. This crate does not verify signatures, raw asset bytes, custody, instrument certificates, calibration validity, or physical independence. SHA-256 strings are required to bind identities, but the crate does not rehash source files. A digest establishes byte identity against a declared digest, not authorship or physical truth.

## Scope

The kernel handles one 3-component velocity sample covariance (3×3) and its 6×6 joint PIV/reference covariance. It does not fit spatial/temporal covariance, process raw images, infer a cross-covariance from paired data, qualify a force balance, or validate EHD thrust.
