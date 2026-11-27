# Cross-modal spacecraft pose consistency gate v0.1

A standalone, dependency-free Rust gate for RGB and event-camera pose estimates. It does not estimate or fuse pose. It checks identity, time, coordinate conventions, pose disagreement and declared common dependencies before a downstream system may consider a two-modal candidate.

This composes with #7713 (synchronization), #7743 (pose trustworthiness), #7782 (statistical SE(3) regions), #7801 (independent-unit calibration) and #7808 (translated geometry preservation).

## Core idea

Agreement is not independence and is not proof of correctness. RGB and event estimates may share clock, optics, extrinsic calibration, target geometry, renderer, training data, annotations or translator lineage. A common bias can make two sensors agree and still be wrong.

The gate has distinct dispositions:
- CANDIDATE_CONSISTENT: exact frame/target/time/convention identity matches, both estimates are candidates, observability is adequate, pose disagreement is within the declared gate, and no dependency digest is shared or missing.
- DISAGREEMENT: pose disagreement exceeds the declared physical consistency tolerance, even if each estimator says its own uncertainty is low.
- DEPENDENCE_UNKNOWN: a dependency digest is shared or dependency declarations are incomplete.
- OUT_OF_SCOPE: frame, target, coordinate convention or timestamp scope does not match.
- ABSTAIN_MODALITY_UNAVAILABLE / ABSTAIN_LOW_OBSERVABILITY: do not silently fall back to one sensor and label it as fused evidence.

The gate never averages estimates or reports a fused covariance. Original RGB and event estimates remain intact.

## Numeric distance

Translation disagreement is the outward-rounded Euclidean norm in integer micrometres.

Orientation uses a sign-invariant Q30 quaternion chord metric:

d_chord = min(||q_rgb - q_event||, ||q_rgb + q_event||)

For unit quaternions, this is related monotonically to SO(3) geodesic angle by d_chord = 2 sin(theta/4). The v0.1 gate compares the exact integer Q30 chord metric to a declared threshold instead of using floating acos; q and -q therefore produce zero disagreement. It does not report an angle in radians. A future version may introduce a separately qualified outward-rounded geodesic conversion.

Each quaternion is validated against a declared 0.1% squared-norm tolerance. This is a syntactic/numeric check, not a camera calibration result.

## Common dependencies

The gate compares paired digests for clock, optics, extrinsics, model, training data, target geometry, renderer, annotation and translator.

Any exact shared digest is reported and blocks CANDIDATE_CONSISTENT under the v0.1 fail-closed policy. Incomplete dependency declarations also block promotion.

Different digests do not prove independence or exclude unrecorded common causes. A shared dependency does not prove the two errors are perfectly correlated; it only means independence cannot be assumed from the available evidence. The correct disposition is DEPENDENCE_UNKNOWN, not DISAGREEMENT.

## Reproduce

    cargo fmt --manifest-path tools/cross-modal-pose-consistency-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/cross-modal-pose-consistency-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/cross-modal-pose-consistency-bench/Cargo.toml --all-targets -- -D warnings

    MANIFEST=tools/cross-modal-pose-consistency-bench/fixtures/cross-modal-common-cause-v0_1.tsv
    DIGEST=$(sha256sum "$MANIFEST" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/cross-modal-pose-consistency-bench/Cargo.toml -- "$MANIFEST" "$GIT_COMMIT" "$DIGEST"
    python3 tools/cross-modal-pose-consistency-bench/tests/reference_oracle.py "$MANIFEST" "$GIT_COMMIT" "$DIGEST"

The separate Python oracle hashes raw bytes, parses the TSV independently, recomputes distances and dispositions, and emits a complete receipt. The exact-head workflow compares complete JSON output.

## Frozen cases

The v0.1 corpus covers distinct dependencies and consistent pose; shared clock; shared training and target-geometry provenance; common-mode biased but mutually agreeing pose; explicit translation conflict; q/-q equivalent rotation; stale timestamp; frame identity mismatch; coordinate-frame convention mismatch; event OOD; low event observability; incomplete dependency declarations.

These are synthetic contract cases, not measured spacecraft results.

## Claim ceiling

CANDIDATE_CONSISTENT means only that the declared checks passed and no shared dependency digest was declared. The receipt always states independence_proven=false, physical_truth_established=false, deterministic_safety_bound=false and maneuver_authority_granted=false. The tool does not authenticate upstream bytes, prove sensor independence, discover every hidden common cause, establish conditional or simultaneous coverage, or certify spacecraft autonomy. It is not a covariance-intersection or Lie-group fusion implementation.
