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

## Claim ceiling

This is a coordinate reformulation and interval arithmetic reference, not a novel trigonometric theorem. It certifies only the supplied phasor components and threshold-cosine interval. It does not validate quadrature calibration, magnetic sensor response, physical field, amplitude reserve, model discrepancy, or installed encoder accuracy. Physical measurement and discrepancy evidence remain separate.
