# Orbital projective acquisition v0.1

## Intent

Extend [#7350](https://github.com/Luminous-Dynamics/symtropy/issues/7350)'s motion/time-aware acquisition contract beyond the deliberately narrow translational subset in [#7384](https://github.com/Luminous-Dynamics/symtropy/pull/7384).

A camera pose is not a single point observation. A feature traces a bounded image-space path during each exposure, and rolling-shutter rows observe different time slices. This benchmark treats the frame as a **projection tube** and bounds the perspective displacement of the declared 3D support.

## Contract

Given a qualified upper bound on relative camera-to-target translational speed (V_t), angular speed (Omega), and maximum point radius (R_{max}), use the conservative point speed:

[
V = V_t + Omega R_{max}.
]

For a declared integration interval (Delta t):

[
Delta P = lceil VDelta t ceil.
]

For the pinhole projection (u=f_x X/Z+c_x), (v=f_yY/Z+c_y), with support bounds (Zge Z_{min}>0), (|X|le X_{max}), (|Y|le Y_{max}), the derivative bound yields:

[
|Delta u|le f_xDelta Pleft({1over Z_{min}}+{X_{max}over Z_{min}^2}ight)
={f_xDelta P(Z_{min}+X_{max})over Z_{min}^2},
]

[
|Delta v|le {f_yDelta P(Z_{min}+Y_{max})over Z_{min}^2}.
]

Every division rounds outward. The complete coordinate, depth and radius envelopes must hold across the entire exposure/readout interval. The code computes exposure smear and first-to-last row midpoint variation independently and also reports their conservative sum.

## Bound derivation

For each supported point, its relative camera-frame velocity is bounded by translational speed plus angular speed times its radius. Each coordinate displacement is therefore no greater than (Delta P). Applying the quotient derivative to (X/Z):

[
left|{d(X/Z)over dt}ight|
le {|dot X|over Z_{min}}+
{X_{max}|dot Z|over Z_{min}^2}.
]

The implementation assumes (|dot X|,|dot Z|le V) and integrates that conservative bound over the full time separation. The Y projection is analogous. This is intentionally looser than a full interval SE(3) trajectory solution; its advantage is a small, auditable, fixed-point proof obligation.

## Scope and owners

- #7350 remains the acquisition-integrity umbrella.
- #7384 owns the translation-only support-plane/image-plane bound and timestamp/trigger/projector association.
- #7339 owns calibration scope/state validity.
- #5631 owns multi-device clock-map uncertainty.
- #7164 owns sampling density.
- #7222 owns photometric/material usability.
- #7043 owns feature identifiability.
- #6846 owns metrology covariance and conformity.
- #7388 co-designs finite speed/exposure/illumination/projector choices.
- This issue owns only the conservative perspective projection contribution from already-supplied, qualified relative motion and geometric support bounds.

No pose estimator, trajectory generator, hand-eye calibration, clock synchronizer, image corrector or metrology engine is duplicated here.

## Frozen benchmark strata

The initial ten cases exercise:
- static positive control;
- translational motion within and beyond budget;
- pure relative rotation;
- near-depth perspective sensitivity;
- rolling row-time displacement beyond budget;
- exposure and row-time individually within budget but combined frame bound beyond budget;
- unknown relative-motion state;
- failed support-envelope state;
- zero minimum-depth invalid input.

Rust tests also check increasing bound with increasing support radius, lower bounds at nearer depth, and correct separation/combination of exposure and row-time displacement. The independent standard-library Python oracle uses exact rational arithmetic and must match complete Rust receipts.

## Metrics and prospective gates

Report:
- upper-bound conservatism against high-resolution temporal projection sampling;
- horizontal/vertical exposure smear and row-time reprojection in micro-pixels;
- near/far depth sensitivity;
- per-source uncertainty and model validity status;
- false-valid and false-hold rates;
- combined-frame bound and channel-specific margins;
- fixed-point runtime, overflow rejection and exact replay.

Prospective software targets:
- 100% agreement with independent rational oracle on frozen cases;
- zero unknown, failed or unsupported upstream states accepted;
- zero over-budget exposure, row-time or combined-frame cases accepted;
- monotonic bounds with increased relative speed, angular speed, radius or duration, and with decreased minimum depth;
- exact equality with the translation-only formula when angular speed is zero and row-time span is zero.

These are synthetic contract targets, not camera performance or physical inspection results.

## Claim ceiling

The evaluator checks only the supplied synthetic bounds. It does not prove the supplied speed/depth/support envelope contains reality, nor validate motion capture, calibration, timing, shutter/readout, blur, feature observability, coverage, POD/NDE, measurement uncertainty, traceability, structural acceptance or flight readiness. Physical validation requires independent time-synchronized motion truth and qualified imaging references across depth, rotation, shutter modes, acceleration and repeated sessions.
