# Spacecraft pose region scorecard v0.1

A benchmark evaluator for pose uncertainty regions, not a pose estimator. It scores five nested nominal levels (50/80/90/95/99%) and reports translation and orientation separately, plus joint pose coverage and region sharpness.

## Input semantics

Each row binds dataset sample, target identity, split, crop stratum, visible-part group, sensor modality, OOD group, calibration revision, frame and explicit pose semantics.

Predicted and truth translation coordinates use integer micrometres in the declared frame. Quaternions use an explicit XYZW or WXYZ ordering. Accepted predictions carry five nested regions:
- an axis-aligned 3D translation box (lower/upper micrometres for x/y/z);
- an angular cap radius about the predicted orientation (microradians).

The evaluator validates both quaternions, normalizes them within a declared tolerance, and computes the orientation geodesic error as 2*acos(abs(dot(q_pred,q_truth))). The absolute dot makes q and -q rotation-equivalent. The angular error is rounded upward to microradians for coverage tests.

This v0.1 representation is intentionally a simple common scorecard region, not a covariance matrix or a general SE(3) credible set. A covariance-to-region calibration adapter must be separately versioned and verified. Stochastic prediction regions and deterministic geometric enclosures are different claims; this scorecard only evaluates empirical containment of labelled truth.

## Metrics

For each nominal level:
- translation x/y/z marginal coverage;
- translation joint coverage (all three axes);
- orientation geodesic coverage;
- joint pose coverage (translation joint and orientation);
- false-confident count (accepted pose outside the declared joint region);
- average width numerators for x/y/z;
- translation box volume proxy and orientation radius sums.

Coverage denominators include accepted predictions only. Abstained, missing, invalid, duplicate and split-leaking rows are retained as separate counts and grouped results; they are never silently dropped. The scorecard reports target/crop/modality/OOD groups to expose worst-group failures.

No single weighted score is emitted. Coverage, sharpness, false-confidence and usable-prediction rate remain separate dimensions. Large regions can achieve high coverage but will be visibly less sharp; abstention can reduce false-confidence but lowers usable coverage.

## Frozen 42-case corpus

The synthetic cases include:
- exact truth and q/-q equivalence;
- translation-only, orientation-only and joint misses;
- multiple crop fractions, visible-part groups, modalities and OOD strata;
- five nested levels and malformed/non-nested regions;
- predicted centre outside its own region;
- zero-width regions, maximum angular radius and volume overflow;
- invalid quaternion and unsupported units/split/status;
- abstained and missing predictions retained separately;
- target leakage across train/test and duplicate sample IDs.

Positive controls are mandatory. A checker that rejects all predictions is not a useful scorecard.

## Reproduce

    cargo fmt --manifest-path tools/spacecraft-pose-region-scorecard-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/spacecraft-pose-region-scorecard-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/spacecraft-pose-region-scorecard-bench/Cargo.toml --all-targets -- -D warnings
    CORPUS=tools/spacecraft-pose-region-scorecard-bench/fixtures/pose-region-corpus-v0_1.csv
    DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/spacecraft-pose-region-scorecard-bench/Cargo.toml --bin spacecraft-pose-region-scorecard-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
    python3 tools/spacecraft-pose-region-scorecard-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"

The dedicated workflow compares complete JSON receipts from Rust and an independently implemented Python oracle.

## Claim boundary

A passing result establishes only scoring correctness on the declared fixture/corpus. It does not prove that model uncertainty is calibrated outside the evaluated distribution, that simulated labels are physical truth, that a prediction region is a deterministic set enclosure, or that any pose is safe for spacecraft operation.

Receipt flags keep claim_ready, source_bytes_authenticated, physical_validation and authority_granted false.

Research parent: #7720. Truncation/OOD challenge: #7719. Dataset/geometry integrity: #7712 and #7715. Overall programme: #7700.
