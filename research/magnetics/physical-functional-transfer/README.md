# Physical field to sensor functional assurance (v0.1)

This research crate composes the whole-cell physical field envelope from `magnetic-physical-discrepancy` with a bounded two-channel sensor transfer. It returns independent shortest-arc phase and minimum-signal-amplitude margin enclosures.

## Why this increment exists

The field-discrepancy layer can enclose the physical 3D vector field over a spatial cell. The phasor-margin layer can certify a supplied phasor. A missing interface is the sensor's field-to-channel response: calibration gains, cross-axis coupling, offsets and bounded residual effects must be propagated rather than assumed away.

This crate does not replace the field-to-angle benchmark (#7411), the sensor transfer's physical calibration, or the phase definitions in #7682. It supplies one typed composition path between the physical field certificate and the functional decision.

## Transfer model

For a two-channel sensor:

`p = H B + o + e`

- `B`: physical field vector over the exact cell, supplied by the discrepancy certificate;
- `H`: interval 2x3 gain/cross-axis transfer matrix;
- `o`: interval channel offsets;
- `e`: whole-cell response residual envelope for declared nonlinearity, hysteresis, temperature/electronics effects not represented by `H`.

The matrix coefficient unit is `output_unit / field_unit`. Offset and response residual units match the output unit. The first implementation requires dimensionless phasor output. The intervals are component-wise; correlations are not encoded, so overestimation is possible.

The sensor calibration, transfer model, artifact, environment, frontend and exact upstream physical scope are identity-bound. The external evidence behind those identities is not cryptographically authenticated by this crate.

## Functional margins

For independently traceable nonzero reference phasor `q`, maximum-error cosine interval `C_max`, and nonnegative minimum squared amplitude `A_min²`:

`g_phase = (p · q)/(||p|| ||q||) - C_max`

`g_amplitude = px² + py² - A_min²`

The phase expression is the same branch-cut-free shortest-arc criterion as #7682. It does not use `atan2`, angle wrapping or runtime trigonometric functions. The reference phasor must come from an independent angular-reference record for the same rotor state and operating condition.

Phase and amplitude remain separate hard requirements. A measured phasor that may be zero leaves phase unresolved; it is never normalized through the singularity. The amplitude result remains independently available.

- **Safe:** complete margin interval is strictly above zero.
- **Failing:** complete margin interval is strictly below zero.
- **Unresolved:** interval touches or straddles zero, evidence is insufficient, or phase is undefined.

The combined result is safe only when both requirements are safe; any definitely failing requirement makes the combined result failing; all other combinations are unresolved.

## Scope

The initial API certifies one fixed rotor-state and operating-condition slice over an XYZ cell. Dynamic speed/bandwidth, angle sweeps and time-varying sensor effects require separate records and later extension.

The result is conditional on the correctness of the physical field, sensor transfer, calibration and angular-reference intervals. The crate checks scope joins, interval domains, units and digest syntax; it does not establish any of those external facts.

## Tests and benchmark targets

The initial test suite covers identity transfer, independent phase/amplitude failure, a branch-cut challenge, a potentially zero measured phasor, nonzero reference, uncertainty widening, exact cell scope, calibration digest, evidence-role and unit controls.

Research targets:
- zero false-safe classifications on preregistered analytic and physical holdouts;
- complete interval containment on analytic transfer known answers;
- widening an admitted uncertainty interval must not improve a certified margin;
- eventual at least 2x fewer physical observations than a uniform grid at the same false-safe criterion, compared with #7540 under identical conditions.

These are targets, not achieved results. Synthetic tests do not demonstrate hardware accuracy, calibration validity, manufacturing yield or industrial superiority.

## Dependencies

- #7655 / `magnetic-interval-ad`
- #7678 / `magnetic-physical-discrepancy`
- #7682 / branch-cut-free phase-margin definition
- #7411 / field-to-angle application benchmark
- #7334 / calibrated 3D field metrology
- #7476 / traceable angular-reference metrology
