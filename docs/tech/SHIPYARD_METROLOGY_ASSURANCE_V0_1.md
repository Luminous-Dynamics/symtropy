# Inter-pass metrology assurance oracle v0.1

Tracking: [SHIPYARD-METROLOGY-ASSURANCE-001 #5148](https://github.com/Luminous-Dynamics/symtropy/issues/5148).

## Design

The v0.1 library treats each uncertainty contribution as a deterministic, non-negative absolute-error bound. It sums contributions by checked worst-case addition (triangle inequality), not RSS. RSS is only appropriate when a defensible statistical model and covariance/independence assumptions are explicitly established.

A before/after geometry difference is:

`delta = after.value - before.value`

Its uncertainty is the conservative sum of the before budget, after budget, inter-scan transform budget and datum budget. The code rejects duplicate source IDs, so a source cannot silently enter twice under the same identity. It rejects mismatched units/scopes, expired or not-yet-valid evidence, unverified contributions, stale/future observations, mismatched workpiece/revision/frame, missing scan/registration digests, non-monotonic scan ordering and arithmetic overflow.

For a measured value x with bound U and specification [L,H], the guard-band result is:
- CONFORMING if [x-U,x+U] is entirely within [L,H].
- NONCONFORMING below/above if the full uncertainty interval lies entirely outside one side.
- INDETERMINATE if the interval intersects a specification boundary.

This is an engineering contract inspired by ISO 14253-1:2017, not a declaration of conformance to the complete standard or a weld acceptance authority.

## Frozen known-answer case

Before value 200, after value 260, before/after measurement bounds 2 and 3, transform bound 1 and datum bound 2, all in symbolic area units:

- delta = 60;
- conservative uncertainty = 2 + 3 + 1 + 2 = 8;
- interval = [52, 68];
- specification = [50, 70];
- result = CONFORMING.

A second case uses delta 55 and the same uncertainty; its interval [47,63] crosses the lower limit and therefore returns INDETERMINATE. It is not rounded or forced into acceptance.

## Baselines

- Manufacturer nominal accuracy only (insufficient by itself).
- Registration fit residual only (not independent target registration error).
- RSS combination under a separately declared independence/covariance model.
- Conservative worst-case bound sum (v0.1 reference).
- Point-estimate threshold vs uncertainty guard-band decision.

## Standards alignment

- ISO 10360-13:2021: acceptance/reverification for optical 3D CMS under its cooperative surface assumptions.
- ISO 14253-1:2017: conformity decisions with measurement uncertainty; current after 2023 confirmation.
- ISO/AWI 23247-8: VVUQ work item approved 23 October 2026 and still under development; alignment only, no compliance claim.
- Yu et al. (2026): reports a ±0.5 mm wire-tip calibration result in its specific robotic multi-pass welding context.
- Jiang et al. (2026): reports path extraction error ≤1.5 mm and RMS ≤0.8 mm in its specific robotic path extraction context. These values are not interchangeable with inter-pass measurement uncertainty.

## Claim ceiling

The 22 deterministic unit tests exercise symbolic arithmetic, identity, source validity, interval propagation and decision classes only. They do not qualify an optical scanner, establish millimetre accuracy, parse physical scans, validate weld geometry, prove distortion, accept a weld or authorize production release.
