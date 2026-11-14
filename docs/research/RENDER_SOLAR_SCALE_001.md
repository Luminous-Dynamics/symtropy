# RENDER-SOLAR-SCALE-001 — Coordinate precision before more visual effects

**Status:** Draft foundation. The canonical position type and tests are authored; this is not yet a complete solar-system renderer or a validated frame-time result.

## Objective

Enable a continuous rendering experience that can move between detailed local engineering, a planet's surface, planetary orbit, interplanetary distances, and the scale of the solar system without defining every body in one absolute f32 coordinate system.

The core change is not “use doubles everywhere.” It is to separate canonical spatial truth from camera-relative presentation.

## Coordinate contract added

The render bridge now has a pure Rust large-world position type, LargeWorldPosition:

- Canonical position = signed 64-bit cell address per axis plus normalized f64 local offset inside that cell.
- Cell width = 2^20 metres (1,048,576 m), fixed and globally aligned.
- Each position carries an opaque CoordinateFrameId. Relative transforms fail closed when frame IDs differ. A caller must use a verified frame/ephemeris service to convert between planet-fixed and inertial/barycentric frames.
- Camera-relative coordinates subtract integer cell addresses before conversion to floating point. The canonical position does not move when the camera origin changes.
- A three-part GPU-relative representation (high/middle/low f32) preserves the residual between coarse astronomical displacement and sub-cell offsets. A shader should transform each part separately and combine transformed results; adding the three position parts in f32 first defeats the split.
- A convenience f32 conversion checks the actual per-axis round-trip error against a caller-declared metre budget and rejects a conversion when the budget is exceeded.
- Construction from one absolute f64 coordinate is explicitly restricted when a reliable cell-local remainder cannot be recovered. For precise astronomical sources, callers should supply integer cell address and local offset separately.

The fixed grid supports much more range than a solar system requires while keeping near-field offsets representable independently of global position. It does not, by itself, give a graphics engine the ability to display a solar system: shader integration, scene hierarchy, projection/depth strategy, streaming, culling and scale transitions are separate obligations.

## Why not a single double or a floating origin alone?

A double-precision absolute coordinate helps, and is a valid reference calculation, but its spacing grows with coordinate magnitude. A single 32-bit position is substantially worse: at astronomical distances, centimetre or metre details vanish when world-space values are cast to f32.

Floating-origin rebasing alone moves the error elsewhere if canonical simulation coordinates are first stored in a giant float vector. In this contract, world identity remains as integer cells plus local offsets; choosing a camera origin only changes the presentation transform.

Unreal's Large World Coordinates likewise expands core world data to double precision and has specialized position handling for particles. That is an instructive precedent, not a claim of parity. Sources:
- https://dev.epicgames.com/documentation/unreal-engine/large-world-coordinates-in-unreal-engine-5
- https://dev.epicgames.com/documentation/en-us/unreal-engine/georeferencing-a-level-in-unreal-engine

## Scientific frame integrity

Solar-system visuals must not invent a single unnamed coordinate system. The new frame ID is deliberately opaque: this change does not ship an ephemeris model or declare arbitrary frame IDs equivalent.

The system-level design should establish a narrow frame-service boundary for:
- a documented inertial reference frame and epoch/time scale;
- body-fixed frames for surface detail;
- time-dependent transforms for body orientation and orbit positions;
- source, epoch, frame and digest provenance for every scientific ephemeris input;
- explicit approximation labels where visuals are illustrative rather than ephemeris-backed.

NASA/JPL's SPICE system exists to supply observation geometry, reference frames and time/frame transformations. We should use its conventions and vetted kernels rather than inventing astronomical frame semantics:
- https://naif.jpl.nasa.gov/
- https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/FORTRAN/req/frames.html
- https://hesperia.gsfc.nasa.gov/ssw/packages/sunspice/icy/doc/html/req/time.html

The Rust application should keep the coordinate kernel and scene contract Rust-first. If a SPICE integration requires an external/native boundary, isolate and version that adapter; do not silently substitute an approximate home-grown model in a scientific view.

## Rendering architecture required on top

### 1. Hierarchical scene representation

Use a semantic hierarchy, not a flat list containing every detailed mesh:

- System catalog: star, major bodies, moons, orbital paths and labels.
- Body proxy: sphere/ellipsoid with physically or explicitly illustratively derived radius, orientation and orbit state.
- Body surface: tiled terrain, imagery, atmosphere, rings and local assets; refine only when appropriate for projected error and task.
- Local engineering scene: high-detail structures, robotics and contacts anchored to a body-fixed or local frame.
- Stellar background: direction/apparent-magnitude-driven points or billboards, not millions of tiny full meshes when they are subpixel.

Identity, selection and source provenance must survive proxy/tile swaps. A proxy is a render representation, not a replacement physical body.

### 2. Scale-aware view policy

Define view profiles with distinct camera navigation, content and precision requirements:
- close inspection / human-scale operations;
- planetary surface;
- low orbit;
- system overview;
- interplanetary navigation;
- heliocentric overview.

The profile changes representation and optional effects, not authoritative positions, physical time, or simulation outputs. For scale transitions, preserve selected-body and task-critical content even when the normal hierarchy would collapse it.

### 3. Screen-space hierarchy and temporal stability

Build on the semantic LOD work in PR #2398. Use conservative screen-space geometric-error bounds where available, but also account for apparent angular size, planetary limb/horizon silhouette, labels, selected-body identity and task importance. Add hysteresis to prevent visible oscillation when crossing tile/detail boundaries. Never claim a geometric bound if its source mesh error or camera assumptions cannot be established.

The OGC 3D Tiles specification provides a mature reference for hierarchy, bounding volumes and screen-space geometric error. It is a reference for concepts, not a reason to force all astronomical data into a geospatial tile format:
- https://docs.ogc.org/cs/18-053r2/18-053r2.html
- https://github.com/CesiumGS/3d-tiles

Gaia Sky is another relevant comparison for catalog-backed star rendering and solar-system visualization. It uses apparent-magnitude-related pseudo-size rather than assuming every star should become a conventional geometric sphere at every zoom level:
- https://www.cosmos.esa.int/en/web/gaia/gaiadr2_gaiasky
- https://gaia.ari.uni-heidelberg.de/gaiasky/docs/master/Star-rendering.html

### 4. Depth and near/far composition

A solar-system view spans vastly different depth scales. A single conventional depth buffer and a single fixed clipping range may not serve surface geometry, a planet limb and distant orbital bodies equally well.

The follow-on should benchmark, not prejudge, combinations of:
- reverse-Z / carefully selected near-far ranges for local and planetary scenes;
- separate near-field and far-field composition passes where justified;
- logarithmic or otherwise scale-aware depth only after quantifying depth precision, shader cost, occlusion interactions and visual discontinuities;
- camera-relative GPU transforms;
- separate sky/starfield rendering that does not share the same depth semantics as terrain or structures.

Do not add logarithmic depth just because the scene is large; prove which precision problem it solves and test interference with shadows, depth prepasses, temporal effects and multi-pass materials.

## Authored regression tests

The new module includes tests for:
- normalized local-coordinate validity and non-finite input rejection;
- negative/positive normalization from moderate absolute coordinates;
- camera-relative subtraction when the camera and object share a cell;
- preserving small local offsets while applying an astronomical-unit-scale displacement;
- reconstruction of an AU-scale coordinate from the GPU high/middle/low split, compared with one f32;
- rejection when a requested f32 error budget is exceeded;
- refusal to compare positions whose frame identities differ;
- coordinates at Neptune-like solar-system distances supplied as integer cell addresses;
- rejection of absolute-f64 input where reliable cell-local reconstruction is no longer supported;
- signed cell-address overflow.

These tests are authored, not yet compiled or executed at the time this note was written.

## Frozen fixture now checked in

The machine-readable `benchmarks/render/SOLAR-SCALE-01.json` fixture defines the current planned qualification ladder: local 1 m/1 km, planetary-radius context, Earth-Moon distance, 1 AU, Neptune semimajor-axis scale, 100 AU, apparent-size formula cases, and a seven-stage forward-and-return traversal. It includes the required invariants, reference-value provenance, and report schema.

NASA/JPL's planetary parameters list Earth's mean radius at 6,371.0084 km and Neptune's semimajor axis at about 4.515 billion km; NASA/NSSDC's Sun facts list a mean radius of 695,700 km. The fixture uses the conventional astronomical unit value 149,597,870,700 m. These values anchor workload scale; a runtime scientific scene must still carry exact source/kernel identity, reference frame, time scale, and epoch rather than infer ephemerides from these single constants.

## Required benchmarks

### Coordinate precision and cost

Measure the following on CPU and target GPU:
- relative-position round-trip error for local offsets (millimetre, centimetre and metre cases) at 1 m, 1 km, planetary radius, Earth-Moon distance, 1 AU, Neptune distance, and 100 AU;
- high/middle/low reconstruction error and shader projection error compared with f64 reference calculations;
- rejection rates for invalid or under-specified frame/precision inputs;
- cell-relative transform cost across 1, 1,000, 100,000 and 1,000,000 positions;
- CPU time to update a moving camera origin without mutating canonical entity positions;
- temporal capture stability while crossing scale bands in both directions.

Use exact, machine-readable fixtures for positions and camera poses. Include source revision, frame ID, cell size, viewport, camera model, GPU adapter/driver/backend, shader revision and error budget. Keep f64 reference math separate from the optimized path to make it a useful oracle.

### Solar-system visual fixture ladder

1. Earth-scale: selected surface feature, horizon, atmosphere and orbital spacecraft.
2. Earth-Moon: preserve planet/modeled body positions while zooming between surface detail and the Earth-Moon system.
3. Inner solar system: Sun, terrestrial planets, orbit paths and spacecraft.
4. Outer solar system: Jupiter through Neptune, moons and ring systems, with explicit apparent-size policy.
5. Dense background: catalog stars with apparent magnitude, clustering/aggregation, labels and selection.
6. Camera traversal: one continuous fly-through from local inspection to solar-system overview and back, checking for popping, lost identity, origin discontinuities and depth fighting.

For each fixture report frame interval p50/p95/p99, GPU timestamps where supported, memory/asset residency, number of visible proxies/tiles/instances, representation counts, projected screen error and selected-object continuity. A CPU coordinate microbenchmark is not evidence of renderer scalability.

## Initial targets (proposed, not achieved)

- No unhandled cell-address overflow or frame-mismatch acceptance.
- No protected/selected body or engineering feature silently disappears during a scale transition.
- GPU-relative split round-trip error below 2 cm at a representative 1 AU test point on the CPU-side reconstruction fixture; tighten according to measured shader error and scientific use.
- Zero unreported coordinate-origin jumps in deterministic camera traversal captures.
- Separate performance targets by device tier and view profile; use the existing Symtropy PBR timing protocol and RTX 2070 Mobile baseline only for comparable local measurements.
- Every result binds to exact source, shader, camera, assets, settings, coordinate frame/epoch and hardware/backend. No pass based on a queued job, stale report or missing measurement.

The two-centimetre CPU reconstruction target is an initial engineering guard, not an assertion that an entire astronomical renderer is accurate to two centimetres.

## Execution order

1. Compile, format, Clippy and run the exact-head large-world coordinate tests.
2. Add independent numerical known-answer fixtures for camera-relative high/middle/low transforms and adversarial boundaries.
3. Add a canonical frame/epoch/ephemeris data contract, keeping coordinates and source data separate.
4. Implement body proxies and a hierarchical tile/LOD policy that consumes these coordinates.
5. Add a depth strategy probe comparing ordinary depth, reverse-Z ranges and optional scale-aware composition.
6. Integrate with the existing frozen PBR render receipt workflow and publish comparable precision/performance curves.
7. Complete a continuous camera traversal across surface/orbit/inner/outer solar-system scenes before making whole-solar-system usability claims.

## Claim ceiling

This tranche adds a precision-aware coordinate primitive and tests. It does not yet render a solar system, ingest authoritative ephemerides, provide a log-depth renderer, or establish performance against Unreal, Unity, Gaia Sky, Cesium, or other industry/scientific engines.
