# Realized-outcome check for sensitivity-aware geometry

This companion evaluates the expected squared position error of a two-observation linearized estimator when it uses nominal bearings but the measurement data are generated with scenario-perturbed bearings.

It decomposes expected error into:
- squared model-mismatch bias;
- propagated measurement-noise variance, including any explicitly declared shared common-mode scalar error;
- their sum, the expected squared position error.

The estimator uses the inverse of its nominal 2x2 observation matrix. This is exact for a full-rank square two-observation linear model, not a general nonlinear range solver.

## Frozen example

Truth: [10 m, -5 m]. Fragile east/north geometry has ±0.35 rad bearing errors. Stable 45°/135° geometry has ±0.01 rad bearing errors. Their nominal A-optimal traces differ by only about 5%, while their expected realized squared errors should differ by far more because the fragile geometry acquires a substantial position bias.

The Rust tests compute the closed-form expected error and assert the stable geometry's expected squared error is less than one percent of the fragile geometry's under the declared perturbation. A separate test verifies that common-mode noise increases propagated noise variance relative to the independent-noise model.

This is an analytical synthetic outcome check, not Monte Carlo sampling, a measured sensor result or a protection-level claim. It deliberately limits itself to two observations and a local linear model.


## Executed exact-head result

The targeted isolated Rust workflow on PR #6500 passed both outcome tests and all five robust-selection tests. For the frozen truth [10 m, -5 m] and adverse bearing perturbations:

| Geometry | Nominal determinant | Bias² (m²) | Noise variance (m²) | Expected squared error (m²) | RMSE (m) |
|---|---:|---:|---:|---:|---:|
| Fragile east/north, sigma 0.20 m, bearing errors ±0.35 rad | 625 | 19.3146 | 0.0800 | 19.3946 | 4.4039 |
| Stable 45°/135°, sigma 0.205 m, bearing errors ±0.01 rad | 566.22 | 0.01257 | 0.08405 | 0.09662 | 0.3108 |

The stable geometry retains 90.60% of nominal D-optimal determinant yet has about 200.7 times lower expected squared position error in this declared model. This result is due to the much smaller bearing-mismatch bias; the stable geometry's nominal noise variance is slightly larger.

This is a closed-form expected outcome for one synthetic linearized two-observation case. It is not Monte Carlo, measured sensor performance, a nonlinear range-localization result, protection-level validation, or evidence of a general 200× improvement.
