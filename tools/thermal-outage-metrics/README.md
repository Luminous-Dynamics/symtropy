# Thermal outage metrics — reference arithmetic oracle

This crate is a small, standalone, dependency-free Rust reference implementation for generic threshold-exposure arithmetic. It does not simulate heat transfer, set comfort thresholds, qualify sensors, estimate health risk, or own the Symtropy resilience trajectory.

## Scope

Given a declared temperature series, observation window, maximum admissible sample gap, threshold and side:

- cold-side exposure integrates the positive part of (floor minus temperature);
- hot-side exposure integrates the positive part of (temperature minus ceiling);
- piecewise-linear interpolation is used only between two valid adjacent samples;
- missing values and intervals larger than the declared maximum gap are omitted from the observed-interval integral and lower coverage;
- no extrapolation, zero-fill or implicit gap imputation occurs;
- the first observed breach is distinguished from a whole-window first breach, which is only reported when coverage is complete.

Exposure is reported in °C·h (equivalent to K·h for temperature differences). Breach duration is reported in hours. All times are seconds relative to an externally defined origin.

## Important distinctions

1. A caller must supply the threshold. This crate does not decide what is comfortable, safe, medically acceptable, or legally compliant.
2. Air temperature is not operative temperature. The input measurement method must be recorded by the calling evidence contract.
3. The interpolation model is a declared arithmetic assumption, not evidence that the true indoor temperature followed a straight line between samples.
4. Partial coverage yields only covered-interval metrics. It cannot be presented as a complete event result.
5. The generic hot/cold metrics here are not the paper-specific Degree Hour Ratio (DHR).

## Published DHR comparator

Mitchell, Stephens & Heidarinejad, Measurement and modeling of residential thermal resilience during a simulated outage, Energy and Buildings 359 (2026), 117221, DOI: https://doi.org/10.1016/j.enbuild.2026.117221.

The article proposes a Degree Hour Ratio for its calibrated residential heating-outage methodology. This crate does not implement or claim to reproduce that ratio. The full formula, normalization, target construction and test boundary must be independently extracted from the publication before an implementation can use the name DHR.

## Non-goals

- EnergyPlus or other building-physics solving.
- Thermal model calibration or validation.
- Resilience timelines, resource conservation, decision ranking or adaptive control.
- South African weather qualification or comfort criteria.
- Fire, structural, electrical, HVAC, water, medical or life-safety assurance.
- Device commands or occupied-house outage testing.

See Symtropy issue #4507 for the broader metrology programme.
