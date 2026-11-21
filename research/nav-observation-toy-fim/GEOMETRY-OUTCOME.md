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
