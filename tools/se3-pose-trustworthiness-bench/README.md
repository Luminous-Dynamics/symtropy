# SE(3) pose trustworthiness benchmark v0.1

A dependency-free Rust evaluator for pose error, joint 6-DoF covariance coverage, false-confidence, abstention and selective risk. It is designed to compare PnP, covariance-weighted PnP, dense-correspondence uncertainty, event/RGB fusion and direct-regression models under one output contract.

## Mathematical contract

The translation residual is expressed in the predicted body frame:

    dt_body = R_pred^T (t_gt - t_pred)

The orientation residual is the shortest-arc quaternion logarithm:

    dtheta = Log(q_pred^-1 * q_gt)

The joint residual is:

    e = [dt_body_x, dt_body_y, dt_body_z, dtheta_x, dtheta_y, dtheta_z]

The declared covariance is a 6x6 SPD matrix in that same tangent order:

    BODY_LOCAL_TRANSLATION_XYZ_ROTATION_VECTOR_XYZ
    units: m, m, m, rad, rad, rad

The covariance must be finite, symmetric and positive definite. The Mahalanobis distance is computed by Cholesky solve, not explicit matrix inversion:

    d2 = e^T Sigma^-1 e

For six degrees of freedom the frozen chi-square thresholds are 50%, 80%, 90%, 95% and 99%. This is a Gaussian ellipsoid coverage hypothesis: the evaluator tests it empirically and does not assume EDL/NIG outputs are Gaussian or calibrated. The evaluator reports empirical joint-region coverage separately from translation and orientation point error. It also reports covariance log-determinant and 95% prediction-region log-volume so a model cannot improve coverage merely by inflating uncertainty. It rejects non-finite, asymmetric, singular and non-positive-definite covariance matrices.

## Decision and false-confidence contract

A model emits ACCEPT or ABSTAIN, an abstention reason, and a confidence score (parts per million). The initial admission level is fixed to 95%.

An ACCEPT is flagged false-confident when either:
- the ground-truth pose lies outside the declared 95% joint covariance region; or
- the prediction is flagged OOD, symmetry-ambiguous, below 5% visible support, below six correspondence points, or above the frozen 1e8 geometry-condition threshold.

These are benchmark policy thresholds, not universal spacecraft safety constants. A downstream system may choose a stricter policy. ABSTAIN remains an evaluated sample and is counted against accepted-pose coverage; it is not counted as a false-confidence acceptance.

The receipt explicitly keeps:
- source_bytes_authenticated=false;
- physical_calibration_validated=false;
- maneuver_authority_granted=false.

A covariance is stochastic, not a deterministic geometric enclosure. This evaluator does not convert one into the other.

## Risk-coverage frontier

The benchmark sweeps confidence-score thresholds from 0 to 1,000,000 ppm over ACCEPT decisions only, reporting accepted-pose coverage, false-confidence count and selective risk. ABSTAIN decisions remain in the all-valid denominator and in the separate abstention count; they are never counted as false-confident accepts. It also reports accepted-pose coverage, empirical covariance coverage, accepted translation p95, accepted rotation p95, median covariance log-determinant and median 95% pose-region log-volume.

A model is not rewarded for rejecting every difficult sample: the risk-coverage frontier exposes the accuracy/coverage tradeoff. Results must always be stratified by spacecraft identity, view, truncation, visibility, sensor mode, range and OOD regime in full model experiments.

## 42-case synthetic corpus

The frozen corpus includes:
- identity and non-zero translation/rotation controls;
- q/-q equivalence and rotated body-frame residuals;
- accepted poses inside and outside the joint 95% covariance region;
- OOD, symmetry, low-visibility, insufficient-correspondence and ill-conditioned acceptance;
- valid ABSTAIN with an explicit reason;
- SPD, non-SPD, singular, asymmetric and NaN covariance;
- invalid predicted/ground-truth quaternion;
- wrong tangent, quaternion order, frame, unit and sensor mode;
- malformed dataset/model/calibration digests;
- invalid confidence score/level and compute measurement state;
- correlated covariance controls;
- direct-regression positive controls with no explicit correspondences;
- direct-regression low-visibility rejection and invalid support-mode controls;
- duplicate samples and target identity leakage across splits;
- deterministic receipt replay and independent Python parity.

Synthetic corpus values are not hardware measurements. Compute fields are marked NOT_MEASURED; the benchmark does not infer Jetson performance from fixture values.

## Reproduce

    cargo fmt --manifest-path tools/se3-pose-trustworthiness-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/se3-pose-trustworthiness-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/se3-pose-trustworthiness-bench/Cargo.toml --all-targets -- -D warnings
    CORPUS=tools/se3-pose-trustworthiness-bench/fixtures/pose-trust-corpus-v0_1.csv
    DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/se3-pose-trustworthiness-bench/Cargo.toml --bin se3-pose-trustworthiness-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
    python3 tools/se3-pose-trustworthiness-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"

## External comparators

- FastPose-ViT (WACV 2026): direct-regression plus geometric correction; Jetson Orin Nano 75 ms/frame and up to 33 FPS under a pipelined execution regime. Compare only on matched hardware, crop, resolution and end-to-end timing.
- UE-Pose (2026): per-keypoint RGB/event evidential uncertainty, topology-aware fusion and multi-candidate pose solving. Add RGB-only and RGB/event ablations separately.
- STRD truncation-robust dense correspondence: NIG uncertainty propagated through uncertainty-aware PnP into pose covariance. Retain its separate target/split and metric definitions.
- SPEED/SPEED+, SHIRT, SDKP and SpaceSense-Bench are separate data lanes; do not pool their metrics or sensor modalities.

## Claim ceiling

This component evaluates the consistency of supplied pose/covariance predictions on a declared test corpus. It does not train or calibrate an estimator, prove source data truth, authenticate model weights, validate camera calibration, establish on-orbit performance, produce deterministic collision enclosures, prove passive safety or authorize maneuvers.

Research parent: Symtropy #7743. Projection contract: #7712. Temporal frame-bundle contract: #7715. Truncation challenge: #7719. Strategy parent: #7700.
