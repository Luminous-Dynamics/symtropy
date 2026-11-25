# Orbital time-varying acquisition v0.1

## Intent

Extend [#7397](https://github.com/Luminous-Dynamics/symtropy/issues/7397) and [#7399](https://github.com/Luminous-Dynamics/symtropy/pull/7399) from a constant whole-interval point-speed ceiling to a bounded time-varying speed envelope. The purpose is to reduce false holds caused by applying one peak-speed value to every instant, without allowing any false-valid acquisition.

## Supplied kinematic envelope

At the first-row exposure midpoint t=0, the model supplies whole-support point-speed magnitude V0, point-acceleration magnitude A0, and point-jerk magnitude J. With |a(t)| <= A0 + J|t|, speed is bounded by:

`V(t) <= V0 + A0*|t| + 0.5*J*t^2`.

This is a support-wide point-motion bound, not a camera-origin-only bound. An upstream source must include relative translation, rotation, centripetal/Coriolis effects, target motion and flexure as appropriate. This evaluator checks only the arithmetic given the supplied envelope.

## Exposure and row-time integration

Let E be the maximum per-row exposure duration and T be the first-to-last row exposure-midpoint span.

The first-to-last row midpoint path bound is:

`Drow <= V0*T + A0*T^2/2 + J*T^3/6`.

For the latest row's exposure window centred at T, integrate the speed envelope over [T-E/2, T+E/2]:

`Dexp <= V0*E + A0*I1 + J*I2/2`

where:
- I1 = T*E when T >= E/2;
- I1 = T^2 + E^2/4 when T < E/2;
- I2 = E*T^2 + E^3/12.

The latest row maximizes the integral for this non-decreasing absolute-time speed envelope. The negative-time first-row half-exposure is included in the symmetric T=0 case.

## Complete-frame integral: do not add two rounded maxima

The complete sampled frame spans [-E/2, T+E/2]. Its path bound is integrated directly:

`Dframe <= V0*(T+E) + A0*I1frame + J*I2frame/2`

with:

- I1frame = (E^2 + (2*T+E)^2)/8;
- I2frame = ((2*T+E)^3 + E^3)/24.

The tempting shortcut Drow + Dexp is not a valid general upper bound for T < E/2. It may under-bound the integral of the full acquisition interval even though each diagnostic channel is separately rounded upward. Frozen case TV-016 has E=1 ms, T=0.25 ms and A0=32,000,000 µm/s²: Drow rounds to 1 µm and Dexp to 10 µm, but the complete-frame integral is 13 µm. The direct integral is therefore required.

## Arithmetic

Production evaluator uses Rust fixed-point integers, checked multiplications/additions and ceiling division. Speeds are in µm/s, acceleration in µm/s², jerk in µm/s³, times in ns, and pixel bounds in micro-pixels. Each non-negative rational contribution is independently rounded upward before summing. Inputs beyond declared limits, conversion overflow, zero minimum depth and invalid source digests produce INVALID_INPUT. FAIL gates produce BLOCKED; UNKNOWN gates produce INDETERMINATE.

## Integration

- #7350 owns acquisition integrity.
- #7384 owns translational/time-association acquisition checks.
- #7397/#7399 own constant-speed projective bounds.
- #7420 owns time-varying point-speed integration under supplied acceleration/jerk bounds.
- #7339 owns calibration validity.
- #5631 owns clock-map uncertainty.
- #7164, #7222, #7043 and #6846 own sampling, photometry, feature identifiability and metrology respectively.
- #7388 owns finite acquisition schedule co-design.

No trajectory estimator, camera calibration, robot dynamics engine, clock synchronizer, image correction or metrology implementation is duplicated.

## Frozen corpus

The 17 cases cover:
- constant-speed equivalence with #7399;
- acceleration and jerk growth;
- global shutter;
- T < E/2, T = E/2, and late-row exposure;
- separate exposure / row / direct full-frame budgets;
- the TV-016 complete-frame under-bound counterexample;
- negative-time first-row exposure;
- exact inclusive pixel budget and one-unit-over negative control;
- UNKNOWN kinematics and FAIL support/calibration gates;
- zero depth, digest validation and projection overflow.

Rust tests add monotonicity, exact piecewise integral checks and complete-window boundary behavior. The independent standard-library Python Fraction oracle independently derives the formulas and must match the entire Rust receipt.

## Metrics and prospective targets

Compare constant worst-case speed from #7399, the acceleration/jerk integral, and a time-synchronized motion-capture/encoder/IMU temporal reference. Report bound conservatism, false-valid rate, false-hold rate, avoided unnecessary holds, exposure/row/frame margins, runtime and replay parity.

Prospective targets:
- zero false-valid admissions against the declared reference envelope;
- 100% agreement with independent Fraction oracle on frozen cases;
- exact constant-speed reduction when A0=J=0;
- monotone increase with speed, acceleration, jerk or time span;
- zero unknown/stale/unqualified motion or calibration evidence treated as valid;
- zero overflow or under-bound counterexamples accepted.

A tighter bound that underestimates motion is not an improvement. Synthetic success does not validate real motion envelopes or physical inspection.
