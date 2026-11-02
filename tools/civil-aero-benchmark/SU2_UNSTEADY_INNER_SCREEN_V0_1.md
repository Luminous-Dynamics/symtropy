# SU2 unsteady inner-step screen v0.1

## Motivation

SU2's time-domain driver checks the inner solver convergence flag or whether the configured maximum number of inner iterations has been reached before evaluating windowed time-averaged convergence. A time-window success message therefore does not independently prove that the inner residual criterion passed at every time step.

The relevant pinned SU2 source is available at:
- https://github.com/su2code/SU2/blob/6c0e5c5c720936b175c4004e57332884443437f5/SU2_CFD/src/output/COutput.cpp
- https://github.com/su2code/SU2/blob/6c0e5c5c720936b175c4004e57332884443437f5/SU2_CFD/src/drivers/CSinglezoneDriver.cpp

In `COutput::MonitorTimeConvergence`, the inner-loop predicate includes both `GetConvergence()` and the `INNER_ITER` cap condition. The time-window Cauchy average is then evaluated on `TAVG_*` coefficients. This is why the native time-window exit message and per-step inner convergence are different claims.

## Supported contract

`screen_su2_unsteady_inner_convergence` is a strict v0.1 qualification for unsteady cases with:

- `TIME_DOMAIN=YES`.
- `WINDOW_CAUCHY_CRIT=YES`.
- At least one residual-based `CONV_FIELD` value beginning with `RMS_` or `REL_RMS_`.
- An explicit finite `CONV_RESIDUAL_MINVAL`.
- `TIME_ITER` as the imported history iteration column.
- `INNER_ITER` imported with unit `iterations`.
- Each configured inner residual imported with its exact declared log10 residual unit.

It rejects coefficient-based `CONV_FIELD` values. Those require a separate adapter that can reconstruct the actual inner-loop Cauchy history; a final outer time-step history row is not enough evidence.

## Temporal coverage rules

The report requires:

1. At least two saved time steps.
2. Strictly contiguous `TIME_ITER` values. A missing saved time step is rejected.
3. The first recorded time step is no later than `WINDOW_START_ITER`.
4. The last recorded time step reaches at least `WINDOW_START_ITER + CONV_WINDOW_STARTITER + CONV_WINDOW_CAUCHY_ELEMS`. The delay before monitoring and the complete Cauchy-element window are sequential requirements, so their coverage offsets are added rather than collapsed with a maximum.
5. Every imported `INNER_ITER` is a positive integer.
6. Every configured residual is finite and at or below `CONV_RESIDUAL_MINVAL` at every saved time step.

The screen reports the worst observed residual and its time iteration, the number of contiguous saved steps, and the count of threshold failures. The evidence is hashed and bound to the exact production receipt, executable, raw history bytes, import digest and preflight digest.

This is deliberately fail-closed: sparse history output cannot silently be interpreted as full temporal coverage.

## What it does not establish

- It does not reconstruct every inner iteration's residual trace. It screens the final inner residual recorded for each saved outer time step.
- It does not prove the inner solver used the same residual field in every coupled zone or multizone subsystem.
- It does not replace SU2's native stopping-condition summary.
- It does not prove time-step refinement, periodic stationarity, sufficient statistical sampling, absence of aliasing, grid convergence or experimental validation.
- It does not assert that the declared residual threshold is scientifically adequate for every flow or quantity of interest.

For cases where inner-loop history is not emitted at every time step, the screen rejects rather than infer continuity.

## Relationship to signal and time-window diagnostics

The unsteady inner-step screen is separate from:
- Native windowed Cauchy convergence for `TAVG_DRAG`, `TAVG_LIFT`, etc.
- Welch PSD and dominant-frequency estimation in `unsteady_spectrum.rs`.
- Window-to-window stationarity checks.
- Temporal discretization refinement and experimental comparison.

These layers answer different questions: whether each saved inner solve met its residual criterion; whether the averaged engineering quantities stabilized; whether the observed time series has stable statistical/spectral properties; whether the time discretization is adequate; and whether predictions agree with physical measurements.

## Synthetic qualification

The grid-family fixture creates a fake executable and synthetic history only. It exercises:

- A passing contiguous per-time-step residual screen.
- Rejection when one time-step residual exceeds the configured threshold.
- Rejection when a time-step record is missing.
- Independent Python replay of the raw-history values, units, time coverage and canonical digest.
- Binding the screen digest into the verified run bundle for every grid-family member.

These tests verify software contracts. They are not real SU2 runs or evidence of CFD accuracy.
