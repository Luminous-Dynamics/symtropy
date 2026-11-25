# Magnetic encoder function benchmark — 25 November 2026

## Research thesis

The first application benchmark should measure the mapping from rotor angle to sensor-plane field and inferred electrical phase. A useful magnet is not necessarily the one with the largest remanence or largest local B component; it is the one that produces the required signal with low angle error, adequate signal margin and repeatable performance over the declared environment.

The executable v0.1 primitive lives at research/magnetics/encoder-function-benchmark/.

## Model boundary

For a ring with p pole pairs, use the independently known mechanical reference angle theta and a phase-zero offset phi0 and calibration-record ID obtained from a separate calibration record:

phase_error(theta) = wrap_to_pi(atan2(By, Bx) - (p * theta + phi0))

This is an ideal two-axis vector-field phase oracle. It is not a complete sensor model. Sensor gain mismatch, offsets, cross-axis sensitivity, quantization, ASIC linearization, speed/bandwidth, thermal drift and reference encoder uncertainty must enter a later system-level evaluation.

## Primary metrics

Keep these quantities separate:

1. Peak, RMS and nearest-rank p95 absolute electrical phase error.
2. Minimum / mean / maximum in-plane field amplitude and amplitude ripple.
3. RMS out-of-plane to in-plane field ratio.
4. Samples below the declared minimum field threshold.
5. Worst-case metrics across a finite, explicitly identified set of traces.

No synthetic scenario count is interpreted as probability. No yield or CVaR is calculated without a defensible physical sampling design and repeated builds/lots. A physical build and a synthetic scenario are separate evidence classes and cannot be combined in one set. The new operating-condition tournament allows distinct condition IDs under one fixed frame/grid and requirements, preserves each scenario's condition identity, and reports worst-case results without treating the finite set as a probability sample.

## Matched baseline tournament

Compare the same target geometry and sensor arrangement against:
- conventional injection-moulded or extruded bonded multipole ring;
- conventional sintered or segmented ring where application-appropriate;
- optimized conventional segmented assembly;
- reproduction of published patterned multi-material SLS only when its true field and process inputs are available;
- manufacturing-aware inverse-designed candidate.

Freeze pole-pair count, mechanical datum, sensor, sensor-plane position, air gap, frame, sample grid, phase calibration, external field, temperature and processing. Use the same reference encoder and sensor frontend for all candidates.

A literature geometry anchor is TI's 20-pole bonded NdFeB ring example (25.5 mm outer diameter, 21.5 mm inner diameter) and its documented sensor/air-gap context. This is a reference geometry, not an established universal optimum.

The trace contract also binds an operating-condition identity (sensor plane, air gap, temperature, eccentricity/tilt, sensor registration and background-field setup) and signed electrical phase progression. Distinct operating-condition groups are scored separately rather than pooled. A commercial on-axis absolute encoder such as Infineon's AS5048A is a useful system-level reference, but it uses a different magnet/sensor architecture. Its accuracy claims must be compared only with the stated sensor conditions and calibration process, never directly with a raw magnetic field-map metric.

## Challenge target

After characterizing the reference instrument's repeatability and uncertainty, target at least a 2x reduction in peak electrical phase error against the strongest matched conventional ring while preserving minimum field margin and the environmental operating envelope. This is an aspirational research target, not a demonstrated result or a prevalidated absolute tolerance.

## Next experimental sequence

1. Reproduce an existing commercial multipole ring geometry and establish independent rotary-angle truth.
2. Calibrate the 3-axis Hall probe and coordinate registration; quantify position, orientation, standoff, temperature and sensor uncertainties.
3. Record full-turn 3D field traces at multiple air gaps and eccentricity/tilt conditions.
4. Freeze calibration and evaluation data splits; keep zero-offset fitting separate from holdout traces.
5. Compare a conventional ring and a first patterned candidate using the same sensor frontend and acceptance limits.
6. Repeat across independent builds and material lots before making any yield or accepted-part-cost claim.

## Evidence status

- Executable evaluator: authored, pending exact-head CI.
- Unit tests: authored, not yet executed in this environment.
- Synthetic field traces: software controls only.
- Independent Magpylib parity: not yet performed.
- Physical encoder fabrication or measurement: not performed.
- Industrial superiority: not demonstrated.


## Application-level uncertainty propagation

The encoder benchmark branch now includes a first-order covariance-to-phase function in the isolated Rust crate. It consumes an exact identity-matched trace of 2x2 (Bx, By) field covariance samples and an independent reference-uncertainty record.

For the measured vector `B=(Bx,By)`, local phase gradient is `(-By/|B|^2, Bx/|B|^2)`; field covariance propagates through this Jacobian. Mechanical angle reference standard uncertainty is multiplied by pole-pair count; phase-zero calibration standard uncertainty remains a separate additive variance term. The phase-zero calibration record ID must match the independent calibration ID used by the deterministic phase requirements.

This keeps three contributions visible:
- field-map measurement uncertainty;
- mechanical reference uncertainty, scaled by pole-pair count;
- phase-zero calibration uncertainty.

The function reports per-sample and aggregate standard-uncertainty summaries plus descriptive residual / standard-uncertainty ratios. It does not estimate covariance, establish coverage or model joint cross-sample common-mode correlation. It does not interpret normalized residuals as p-values. Physical calibration, covariance estimation and the independent reference instrument still need separate evidence.

The implementation has synthetic known-answer, inverse-field-scaling, rotational covariance metamorphism, rank-one PSD, indefinite/NaN covariance, identity mismatch, calibration mismatch, weak-field and zero-uncertainty negative controls. They are authored tests, not yet a reported runner pass.
