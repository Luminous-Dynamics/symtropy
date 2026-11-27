# Runtime spacecraft pose admission contract v0.1

This crate is a no-ground-truth runtime admission adapter for spacecraft pose predictions. It consumes a synchronized frame-bundle identity, a pose prediction and a separately qualified empirical calibration-profile reference. It does not estimate pose, select viewpoints, evaluate ground-truth error, prove physical calibration or authorize a maneuver.

## Architectural boundary

The offline evaluator in #7744 requires ground truth to compute residuals, Mahalanobis distance, empirical coverage, Gaussian NLL and false-confidence labels. Those values are unavailable at inference time. Its evaluation receipt must never be used as a runtime mission input.

This runtime contract is deliberately separate:
- RuntimePosePrediction contains only predicted pose, covariance, observability, source/frame identity and calibration-profile metadata.
- It contains no ground-truth translation or quaternion, residual, Mahalanobis outcome, empirical coverage outcome or false-confidence label.
- CalibrationProfile is an external, immutable, scope-bound artifact with an empirical held-out coverage lower confidence bound, sample count and split digests.
- A successful result is named CANDIDATE_STATISTICAL_REGION, never SAFE, TRUTHFUL, PASSIVE_SAFE or MANEUVER_AUTHORIZED.

## Required scope

The adapter verifies:
- exact sample/frame/target/dataset identity against a SYNCHRONIZED frame bundle;
- model weights/configuration and camera calibration revision/digest;
- sensor mode, frame convention, quaternion order, tangent convention and physical units;
- a positive upstream frame-source authentication claim; unauthenticated source frames remain ABSTAIN_UNCALIBRATED;
- calibration profile model/data/sensor/camera/target-population scope;
- calibration profile status, authentication, method, nominal confidence level, held-out sample count and declared lower confidence bound;
- finite unit quaternion and finite symmetric positive-definite 6x6 covariance;
- visibility, OOD, symmetry and support-mode observability gates.

The only supported calibration method in v0.1 is SPLIT_CONFORMAL_SE3-v1 at a 95% joint 6-DoF nominal region. The profile's lower confidence bound must meet its own declared minimum; the code does not invent a universal minimum coverage requirement.

The adapter requires an upstream frame_source_bytes_authenticated=true claim before producing a candidate region. It echoes the frame/profile authentication claims but does not verify signatures or authenticate bytes itself. Therefore the runtime receipt always sets source_authentication_verified_here=false and source_bytes_authenticated=false; a separate provenance/custody authority must independently verify those claims.

## Dispositions

- CANDIDATE_STATISTICAL_REGION: a scope-matched statistical region may be offered to a downstream planning/recourse component.
- ABSTAIN_UNCALIBRATED: frame bundle, profile freshness, sample support, held-out calibration limit or profile authentication is insufficient.
- ABSTAIN_UNOBSERVABLE: OOD, symmetry ambiguity, poor visibility or insufficient PnP support.
- OUT_OF_SCOPE: incompatible model, dataset, target population, sensor, frame, units or calibration scope.
- INVALID_INPUT: malformed identity, digest, quaternion or numeric input.
- INVALID_COVARIANCE: covariance is not finite, symmetric and positive definite.
- DUPLICATE_SAMPLE: a sample identity occurs more than once in the manifest.

Direct-regression pose models are not required to provide PnP correspondences; keypoint/dense/event PnP modes are.

## 50-case synthetic corpus

The frozen cases include:
- valid profile and frame bundle positive controls;
- model/calibration/dataset/sensor/frame/tangent/unit mismatch;
- frame bundle sample/frame/target/revision mismatch;
- unsynchronized/incomplete frame bundles;
- stale/revoked/unauthenticated/undersampled calibration profiles;
- weak empirical coverage lower confidence bound and wrong nominal level/method;
- target-population identity/digest mismatch;
- malformed model/configuration/camera/bundle/profile/split digests;
- invalid quaternion and non-SPD/asymmetric/nonfinite covariance;
- OOD, symmetry, low visibility, insufficient correspondences and poor geometry conditioning;
- direct-regression mode with no keypoint correspondences;
- explicit duplicate-sample rows;
- unauthenticated frame source abstention and an authenticated-upstream positive control whose bytes are still not locally authenticated by this adapter.

The independent standard-library Python oracle recomputes every disposition and complete receipt. It intentionally has no ground-truth input fields.

## Reproduce

    cargo fmt --manifest-path tools/runtime-pose-admission-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/runtime-pose-admission-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/runtime-pose-admission-bench/Cargo.toml --all-targets -- -D warnings
    CORPUS=tools/runtime-pose-admission-bench/fixtures/runtime-pose-admission-corpus-v0_1.csv
    DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/runtime-pose-admission-bench/Cargo.toml --bin runtime-pose-admission-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
    python3 tools/runtime-pose-admission-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"

## Integration

- Upstream frame identity: #7715 / #7713.
- Offline empirical evaluation and calibration evidence: #7744 / #7743.
- Truncation/OOD strata: #7719.
- Downstream decision/recourse remains owned by #7393.

The adapter emits only a candidate observation region and abstention status. It does not select the next view; #7393 remains the sole owner of mission-level action choice.

## Claim boundary

The result proves only typed scope/profile consistency for the declared runtime fields. It does not prove that the calibration profile transfers to the current physical target, that the sensor is calibrated in the physical world, that the model is correct on orbit, that the covariance is a deterministic enclosure, that the source bytes are authentic, or that the spacecraft is safe to maneuver. The receipt hard-codes local source authentication verification, physical calibration validation, deterministic safety bound and maneuver authority to false. Upstream authentication claims are carried as claims and never upgraded to locally verified status.
