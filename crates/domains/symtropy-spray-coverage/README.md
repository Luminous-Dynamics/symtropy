# symtropy-spray-coverage

Deterministic, low-cost geometric spray-cone visibility for early dishwasher design.

## What it computes

Given:
- a target-surface triangle mesh,
- an occlusion mesh containing all opaque geometry (target surfaces, dishes, racks and walls),
- idealized nozzle origin, axis, cone half-angle and range,

the crate samples target surfaces and estimates the area-weighted fraction inside the cone with unobstructed line of sight. It separately reports the absolute surface-normal incidence cosine (1 = normal, 0 = grazing), but does not infer impact pressure or cleaning effectiveness.

The implementation provides:
- Deterministic Halton sampling over triangles.
- Möller–Trumbore ray/triangle intersection.
- Separation of counted target area from non-target occluder geometry.
- Per-triangle and aggregate geometrically visible area estimates.
- Area-weighted absolute incidence cosine and a configurable favorable-incidence area fraction, reported separately from visibility.
- Input validation and tests for cone boundaries, occlusion, range, grazing versus normal incidence, degenerate geometry and deterministic output.

## What it does not compute

This is not CFD, droplet transport, spray breakup, atomization, momentum transfer, wetting, wall-film dynamics, impingement, soil removal, drying, hygiene or sanitization. A geometrically visible surface is not necessarily wetted or cleaned. A geometrically occluded surface may still receive splash or secondary droplets in a physical test.

Use it to screen nozzle placement and dish geometry, then compare promising candidates with measured spray-pattern tests and higher-fidelity models.

## Reproducibility

Inputs are deterministic. Preserve target-mesh digest, occlusion-mesh digest, nozzle parameters, sampling count, epsilon, crate version and code commit with every output. Increase sample counts and run convergence checks before relying on small differences.

## Build

~~~sh
cargo test -p symtropy-spray-coverage
cargo clippy -p symtropy-spray-coverage --all-targets -- -D warnings
~~~
