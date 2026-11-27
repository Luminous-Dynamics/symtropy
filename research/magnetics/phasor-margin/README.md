# Branch-cut-free phasor phase margin (v0.1)

A small Rust reference for robust encoder phase acceptance without evaluating a wrapped angle difference.

## Mathematical reformulation

For nonzero measured and reference phase vectors

    p = (px, py)
    q = (qx, qy)

their normalized dot product is

    s = (p · q) / (||p|| ||q||) = cos(Δθ)

where Δθ is the shortest angular separation on the unit circle. For a maximum allowed phase error δ_max in [0, π), the scalar margin

    g = s - cos(δ_max)

is nonnegative exactly when the shortest-arc phase difference is within tolerance.

This is continuous across the conventional atan2 branch cut. Two vectors near the negative real axis can have reported angles near -π and +π while remaining physically close; their normalized dot product remains close to +1.

The threshold-cosine interval is a supplied, provenance-bound requirement. This crate does not compute it using platform libm.

## Interval expression

The crate evaluates the following expression using the interval forward-mode AD and centered mean-value enclosure in the sibling magnetic-interval-ad crate:

    r_p² = px² + py²
    r_q² = qx² + qy²
    s = (px*qx + py*qy) / sqrt(r_p²*r_q²)
    g = s - C_max

It returns a value enclosure and a whole-box derivative enclosure over five dimensionless inputs: measured X/Y, reference X/Y, and the threshold cosine.

## Spatial derivative handoff

`certify_spatial_phasor_phase_margin` accepts a pure expression evaluator over interval dual XYZ coordinates. The evaluator supplies the measured phasor, reference phasor and threshold cosine as expressions of the exact cell coordinates. The function checks the nonzero norm bounds and threshold domain, evaluates the centered interval phase-margin expression, and maps the resulting XYZ derivative suprema through `map_spatial_derivative_suprema`.

The returned `SpatialDerivativeSupremumEvidence` is suitable for the spatial certifier's derivative-bound fields when cell/frame/condition/model/solver/geometry/material/requirement identities match. It contains the phase margin's derivative suprema in dimensionless-per-metre units.

**This does not create the spatial certifier's centre margin.** The centre measurement, measurement uncertainty and centre-margin interval must still be independently supplied at the exact box centre. The whole-cell value enclosure is not substituted for that sample. A separate minimum-amplitude margin is also required.

## Fail-closed contract

- Parameter identity/order, units, expression ID, requirement and interval-domain provenance are explicit.
- Threshold cosine must lie within (-1, 1]; this corresponds to a maximum shortest-arc error in [0, π).
- Both measured and reference phasor squared-magnitude intervals must have a strictly positive lower bound. If either vector may vanish, phase is undefined and no finite phase certificate is issued.
- The interval AD derivative enclosure must remain finite over the full box.
- No atan2, phase wrapping or runtime trigonometric function is used in the acceptance expression.
- This scalar similarity margin certifies only absolute shortest-arc error. A signed lead/lag requirement needs a separate cross-product/sine margin and an explicit sector contract.
- Minimum measured signal amplitude remains a separate functional requirement; normalization must not hide low-amplitude/noisy vectors.

## Tests

The analytic test suite covers:
- a phase pair straddling the conventional -π/+π cut but physically close;
- a phase pair outside the permitted angular tolerance;
- exact 90-degree common-rotation invariance;
- positive scale invariance;
- a measured phasor box containing the origin fails closed;
- reference phasor zero possibility fails closed;
- invalid threshold-cosine domains are rejected;
- whole-box derivative enclosures remain finite for a non-singular box.

## Independent minimum-amplitude reserve

The spatial API now supports two separate requirements over the same exact XYZ cell:

- phase: normalized phasor dot product minus the supplied minimum cosine;
- amplitude: measured phasor norm squared minus the supplied minimum amplitude squared.

The amplitude threshold must be a finite, nonnegative squared magnitude. The amplitude margin has its own expression identity, requirement ID, value enclosure and XYZ derivative-supremum record. The joint `certify_spatial_phasor_requirements` API verifies that both requirements share the same cell bounds, axes, frame, operating condition, model/solver, geometry/material and build provenance, while preserving their distinct requirement IDs.

The two margins are not collapsed into one score. A phase pass cannot hide a low-amplitude signal; an amplitude pass cannot establish phase accuracy. Downstream spatial certification must still bind each derivative record to its own independently observed centre-margin interval and uncertainty. Neither whole-cell expression enclosure is relabelled as a centre sample.

## Claim ceiling

This is a coordinate reformulation and interval arithmetic reference, not a novel trigonometric theorem. It certifies only the supplied phasor components and threshold-cosine interval. It does not validate quadrature calibration, magnetic sensor response, physical field, amplitude reserve, model discrepancy, or installed encoder accuracy. Physical measurement and discrepancy evidence remain separate.
