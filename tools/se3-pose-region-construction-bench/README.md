# Calibrated SE(3) pose-region construction v0.1

This standalone Rust research crate converts a no-ground-truth runtime pose receipt plus an immutable split-conformal calibration profile into an explicit **statistical** six-degree-of-freedom prediction region.

It is not a pose estimator, calibration-profile producer, physical metrology system, deterministic reachability solver, collision checker or maneuver authority.

## Why this slice exists

- #7753 admits a runtime prediction and checks the declared calibration profile scope, but carries the covariance and profile reference rather than a concrete geometric set.
- #7723 scores explicit translation intervals and an orientation angular cap.
- #7777 defines the missing conversion boundary.
- #7744 remains the offline owner for ground-truth residuals and empirical calibration evaluation.
- #7659 and the orbital safety owners remain responsible for deterministic continuous-time safety claims.

This crate composes those owners. It does not copy their data-acquisition, offline evaluation, scoring or physical-safety logic.

## Statistical construction

The calibration profile is produced elsewhere using the fixed nonconformity score

`s = \sqrt{\xi^T \Sigma^{-1} \xi}`

where `xi` is the body-local translation-XYZ / rotation-vector-XYZ residual and `Sigma` is the corresponding 6x6 covariance. The profile pins the score definition, tangent convention, right-composed SE(3) perturbation convention, calibration/test split digests, model and sensor scope, sample count, nominal level and one-based conformal order-statistic rank.

For a calibration sample count `n` and nominal coverage `1-alpha`, the rank is

`k = ceil((n + 1) * (1 - alpha))`.

The profile supplies the frozen score quantile `q` as unsigned Q32.32. This crate checks that the declared rank is exactly the finite-sample rank and that the quantile is representable. It does not read ground truth or recalculate the quantile.

The joint tangent-space prediction set is

`xi^T Sigma^{-1} xi <= q^2`.

Let `Sigma_tt` and `Sigma_rr` be the translation and rotation principal 3x3 covariance blocks. For each block, the maximum absolute Gershgorin row sum is a conservative upper bound on its largest eigenvalue. The crate constructs:

- `r_t = q * sqrt(lambda_upper(Sigma_tt))` metres;
- `r_R = min(pi, q * sqrt(lambda_upper(Sigma_rr)))` radians.

The result is a world-axis translation box centred at the predicted translation, with each endpoint rounded outward to integer micrometres, plus an orientation geodesic cap rounded outward to integer microradians.

The box/cap is an outer approximation of the **joint tangent ellipsoid**. It does not assert independence between translation and rotation or between axes. Using the declared right-composed body-local SE(3) exponential, the translational perturbation is `R * J(phi) * rho`; the SO(3) left Jacobian has operator norm no greater than one, so `||delta_t|| <= ||rho||`. Each world-axis translation error is therefore bounded by the translation-vector norm.

## Numerical and evidence contract

- Dependency-free Rust 2024 crate; isolated with its own Cargo workspace.
- The input runtime receipt must already be `CANDIDATE_STATISTICAL_REGION`; abstentions cannot be promoted.
- Profile, runtime receipt and calibration-score artifact identities are retained.
- The profile must be current, scope-matched, source-authentication-claimed, sufficiently sampled and above its declared held-out coverage lower-confidence-bound floor.
- The exact 95% split-conformal rank is recomputed using checked integer arithmetic.
- Covariance must be finite and exactly symmetric; outward-rounded interval Cholesky must prove every pivot strictly positive. Inconclusive or excessively ill-conditioned cases fail closed.
- Non-finite inputs, invalid conventions, authority escalation and arithmetic/representation overflow never produce a region.
- Outward rounding is explicit throughout the interval-Cholesky definiteness proof, spectral row bounds, square root, multiplication and integer interval conversion.
- Duplicate sample identities invalidate every occurrence.
- Output flags for locally verified source authentication, authenticated bytes, physical calibration, deterministic safety and maneuver authority remain false.

The profile's authentication booleans are upstream claims only. This crate does not verify digital signatures, source bytes, or a custody chain.

## Reproduce

```sh
cargo fmt --manifest-path tools/se3-pose-region-construction-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/se3-pose-region-construction-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/se3-pose-region-construction-bench/Cargo.toml --all-targets -- -D warnings

CORPUS=tools/se3-pose-region-construction-bench/fixtures/pose-region-corpus-v0_1.csv
DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
cargo run --quiet --locked --manifest-path tools/se3-pose-region-construction-bench/Cargo.toml --bin se3-pose-region-construction-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
python3 tools/se3-pose-region-construction-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"
```

The CLI emits a deterministic JSON receipt bound to the exact source commit and corpus SHA-256. The independent Python oracle verifies the corpus digest from the bytes it reads and recomputes the full result without importing the Rust implementation.

## Frozen corpus

The v0.1 CSV contains 35 rows, including:
- positive controls with diagonal, correlated and near-singular-but-positive-definite covariance;
- signed and near-zero translations;
- weak/stale/undersampled calibration profiles;
- rank, digest, score, tangent and perturbation mismatches;
- authentication and authority-escalation negative controls;
- invalid quaternion, asymmetric, singular, non-SPD and nonfinite covariance;
- arithmetic overflow, quaternion sign equivalence, orientation cap saturation and split identity mismatch;
- a deliberate duplicated sample pair.

Positive controls prevent the implementation from “passing” by rejecting all inputs.

## Promotion targets (not results)

- 100% Rust/Python complete-receipt parity on the frozen corpus.
- Zero under-enclosed samples in independent synthetic property tests.
- 100% coverage of the declared tangent ellipsoid by the constructed box/cap in analytical generated controls.
- No invalid profile, unsupported convention, invalid covariance or overflow may produce a candidate region.
- The same region must feed #7723 without scorecard metric duplication or changes.
- Empirical coverage must later be reproduced on target-identity-held-out and HIL data at declared sample sizes, with confidence intervals and group-level treatment.

## Claim ceiling

A passing synthetic run establishes only implementation correctness for this declared mathematical contract and frozen known-answer corpus. Split-conformal coverage is marginal under exchangeability; it does not guarantee conditional coverage for every target, spacecraft geometry, crop fraction, illumination state, sensor mode or flight domain. It does not validate the profile's source evidence, prove physical sensor calibration, turn a statistical region into a deterministic collision bound, establish passive safety or authorize a maneuver.

A downstream safety verifier must receive deterministic state/trajectory enclosures from qualified dynamics and geometry providers. A statistical pose region may inform inspection planning or request another observation, but cannot replace that proof.
