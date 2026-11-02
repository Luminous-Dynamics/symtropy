# symtropy-spray-coverage

Deterministic, low-cost geometric spray-cone visibility for early dishwasher design.

## What it computes

Given a triangulated dish/target mesh and idealized nozzle origin, axis, cone half-angle and range, the crate samples target surfaces and estimates the area-weighted portion that is inside the cone and has unobstructed line of sight.

The implementation provides:
- Deterministic Halton sampling over triangles.
- Ray/triangle intersection for occlusion.
- Per-triangle and aggregate geometrically visible area estimates.
- Input validation and unit tests for cone boundaries, occlusion, range and degenerate geometry.

## What it does not compute

This is not CFD, a droplet-transport model, spray breakup, atomization, momentum transfer, wetting, wall-film dynamics, impingement, soil removal, drying, hygiene or sanitization. A geometrically visible surface is not necessarily wetted or cleaned. A geometrically occluded surface may still receive splash or secondary droplets in a physical test.

Use it to screen nozzle placement and dish geometry, then compare promising candidates with measured spray-pattern tests and higher-fidelity models.

## Reproducibility

Inputs are deterministic. Preserve the mesh digest, nozzle parameters, sampling count, epsilon, crate version and code commit with every output. Increase sample counts and run convergence checks before relying on small differences.

## Build

~~~sh
cargo test -p symtropy-spray-coverage
cargo clippy -p symtropy-spray-coverage --all-targets -- -D warnings
~~~
