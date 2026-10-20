# RENDER-PROJECTION-ERROR-002 — Project geometric error into screen space

**Status:** Draft research kernel. Seven unit tests are authored; no compile or CI pass is claimed yet.

## Why this is the next step

The semantic LOD allocator in PR #2398 accepts an error estimate in pixels, but the host needs a principled path from world-space geometric deviation to an estimate in the current camera view. Distance-only heuristics are not enough for off-axis or high-FOV geometry, and triangle count is not a geometric-error bound.

This tranche adds a pure helper to symtropy-render-bridge for orthographic and pinhole-perspective camera profiles. The host provides the geometric deviation bound and the camera-space extent bounds. The helper does not infer a mesh's error bound, read a Bevy camera, perform frustum clipping, or switch assets.

## Analytical model

For an orthographic projection with viewport height H and vertical world-space span S, a Euclidean geometry-deviation bound delta projects to at most:

    pixel_error = delta * H / S

For a pinhole perspective camera, let:

- H be the viewport height in render pixels;
- theta be vertical FOV in radians;
- z_min be a positive lower bound on camera-space depth for all affected source points;
- rho_max be an upper bound on radial distance from the optical axis;
- delta be the Euclidean source-to-approximation displacement bound, with delta < z_min;
- f = H / (2 tan(theta / 2)) be focal length in pixels.

The analytic upper bound implemented is:

    pixel_error <= f * delta * (z_min + rho_max) / (z_min * (z_min - delta))

The code evaluates the equivalent normalized expression to reduce avoidable overflow. Non-finite or invalid inputs fail closed, as do intermediate/final results that cannot be represented. A nonzero world error that underflows to a zero pixel error is not silently accepted.

This formula is a pinhole-camera analytical bound conditional on the caller supplying correct bounds for the complete affected geometry. It is not a mesh simplification certificate. Ordinary f64 arithmetic does not provide directed-rounding interval guarantees, so it is not yet suitable as standalone safety certification. A certification-grade hard gate should add outward-rounded evaluation and an independently implemented oracle before being described as formally conservative.

## Authored tests

- Known-answer orthographic scale.
- Known-answer perspective projection.
- Monotonicity with larger geometric error and off-axis radius.
- Viewport-resolution scaling.
- Rejection of negative/non-finite geometry error and zero-height viewports.
- Rejection of invalid FOV, depth, radial bounds, and error reaching the camera plane.
- Rejection of invalid orthographic span and unrepresentable results.

## Acceptance and next work

1. Pass format, strict Clippy, and focused render-bridge tests on the exact branch head.
2. Add independent rational/high-precision or interval reference cases for the projection formula and adversarial floating-point values.
3. Define how geometry providers prove or source the actual geometric-deviation bound, including asset/source digest and representation identity.
4. Integrate this helper into PR #2398's LOD candidate preparation only after its assumptions and test corpus are qualified.
5. Keep hard screen-error requirements separate from cost allocation. If a required bound or geometric input is unavailable, return not-assessable; do not substitute a guessed value.

This is stacked after PR #2398 on purpose. It should not land independently of the LOD policy it feeds.
