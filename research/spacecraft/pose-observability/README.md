# Pose Observability FIM Benchmark v0.1

Issue: [#7929 — SE(3) Fisher-information and geometric observability](https://github.com/Luminous-Dynamics/symtropy/issues/7929)

## Contract

This crate computes a local pinhole-camera SE(3) Fisher-information matrix from caller-supplied 3D camera-frame landmarks, 2D measured pixels, focal lengths, principal point and isotropic pixel-noise standard deviations.

Twist order is explicitly fixed to `left_camera_frame_scaled_xyz_then_rx_ry_rz`:
`[dx/L, dy/L, dz/L, d_rx, d_ry, d_rz]`, where `L` is the declared characteristic target length in metres. This scales translation and rotation perturbation coordinates into dimensionless local parameters before computing the six-dimensional spectrum.

For each landmark the implementation uses the left-perturbation point Jacobian and pinhole projection Jacobian, then accumulates `Jᵀ Σ⁻¹ J`. A fixed-budget symmetric Jacobi solver produces the ordered eigenspectrum and weakest eigenvector.

## Reported diagnostics

- 6×6 FIM and all six eigenvalues ascending;
- numerical rank under frozen absolute/relative thresholds;
- minimum and maximum eigenvalues and condition number (only when full rank);
- log determinant (only when full rank) and participation ratio;
- weakest-mode eigenvector and translation/rotation subspace norms;
- reprojection error mean/P95/max;
- non-authoritative claim flags.

The CLI refuses input files larger than 32 MiB and refuses to overwrite existing output files.

## Known-answer and negative controls

- Diverse non-coplanar landmarks should produce full-rank information.
- Repeated points on the optical axis leave a rotational mode unobservable.
- Collinear landmarks expose rank deficiency.
- A common 2D correspondence bias can leave the FIM full-rank while producing very high reprojection residual. This proves that FIM is not a truth or correspondence-authentication certificate.
- Invalid depth, noise, intrinsics, duplicate IDs and malformed values reject.
- Translation scale changes the FIM and is explicitly committed to the request.

Run:

```sh
cargo fmt --manifest-path tools/pose-observability-bench/Cargo.toml -- --check
cargo test --manifest-path tools/pose-observability-bench/Cargo.toml
cargo clippy --manifest-path tools/pose-observability-bench/Cargo.toml --all-targets -- -D warnings
```

## Limits

The v0.1 FIM is a single-frame local linearization, assumes correct feature correspondences and supplied isotropic pixel noise, uses an ideal pinhole model with no distortion, and is conditional on caller-supplied 3D camera-frame points. Sequence stacking, covariance transport, distortion models, full anisotropic covariance and robust correspondence estimation are future increments. This does not estimate pose, authenticate landmarks, prove physical observability over a trajectory, calibrate posterior pose uncertainty or authorize an action.
