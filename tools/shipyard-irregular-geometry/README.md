# Shipyard irregular-geometry prototype (V0)

An isolated, dependency-light geometry kernel for **simple, non-self-intersecting polygon outlines** representing the horizontal projection of irregular ship blocks. It provides bounded exact integer-coordinate validation, ear-clipping triangulation, positive-area polygon/polygon overlap, polygon/rectangle overlap, and polygon containment in rectangular yard bounds.

## Important integration boundary

This crate is a research layer and **does not yet extend or replace** the stable shipyard-space-time::Footprint enum. The currently verified space-time verifier remains rectangle-first and rejects unsupported footprint profiles. No existing plan can become accepted merely by adding this crate. Integration should happen only after the polygon oracle and geometry review pass, through an explicit SimplePolygon footprint variant and fail-closed updates to the baseline/recovery evaluators.

## Geometric contract

- Vertices are integer yard coordinates; each coordinate must be in ±1,000,000,000.
- Rings contain 3–128 unique vertices, have non-zero area and are simple. Self-crossings, non-adjacent touches and backtracking collinear edges fail validation.
- Collinear vertices that continue straight are allowed and removed for triangulation.
- Positive-area intersection is overlap. Shared polygon edges or a shared corner, on their own, are legal.
- An axis-aligned rectangular yard contains a simple polygon iff the valid rectangle contains all of the polygon's vertices; convexity of the rectangle keeps each edge within bounds.
- Arithmetic stays within the declared coordinate/vertex limits. Invalid or over-complex input returns a typed error instead of being treated as non-overlap.

This is deliberately a narrow 2D outline primitive. It does not represent holes, overhang/height, stacking clearance, support points, center-of-gravity constraints, swept-volume transport, crane reach, lift rigging, or uncertain/updated geometry.

## Run

```sh
cargo fmt --manifest-path tools/shipyard-irregular-geometry/Cargo.toml -- --check
cargo test --locked --offline --manifest-path tools/shipyard-irregular-geometry/Cargo.toml
cargo clippy --locked --offline --manifest-path tools/shipyard-irregular-geometry/Cargo.toml --all-targets -- -D warnings
```

## Oracle protocol

Eight unit tests cover concave outlines, arm/notch distinctions, crossing and containment (including a contained triangle whose vertices all lie on the outer edges), boundary-only contact, orientation/start-vertex invariance, self-intersection and coordinate-bound rejection, straight-line vertex normalization, and rectangle containment.

The integration test tests/rectangle_differential_oracle.rs generates 512 deterministic rectangle pairs, converts them to polygon rings, and compares the polygon kernel with the existing verifier's independent strict-area rectangle-overlap predicate (105 overlapping and 407 disjoint/touching targets). The same test file also generates 512 rectangles against a concave L outline and compares with a unit-cell-set reference derived directly from the L's orthogonal regions (157 overlap, 355 disjoint/touching targets). This adds a non-rectangular oracle without sharing production triangulation or point-in-polygon code. These remain exact-head test targets, not passed results or an exhaustive corpus of arbitrary simple polygons.

## Next validation gates

1. Exact-head Rust format, test and Clippy checks.
2. A frozen concave/simple-polygon corpus containing boundary-only contacts, vertex-on-edge contacts, near-collinear edges, narrow notches and nearly touching outlines.
3. An independent polygon intersection oracle that does not reuse production triangulation or orientation helpers.
4. Differential comparison against a mature computational-geometry library on a frozen coordinate corpus, with its numeric model and tolerance rules recorded.
5. Only then, an adapter into the schedule verifier, followed by new placement, exclusion-zone and corridor cases plus independent schedule-level oracles.

A geometry capability is not an industrial scheduling improvement by itself. The evaluation must subsequently compare area utilization, move feasibility, vehicle/crane travel, recovery time, unaffected-plan churn and false-feasible rate on the same industrial instances as the published baselines.
