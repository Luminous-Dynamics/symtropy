# Purpose-Built Multidimensional Views — v0.1

Status: draft view contract and projection helpers. No local Rust execution or passing CI result is claimed.

## Design principle

A view has two orthogonal choices:

1. **Purpose** — strategic, operational, tactical, scientific, or diagnostic.
2. **Dimensional representation** — planar 2D, oblique 2.5D, spatial 3D, or a 3D slice through a 4D world.

A tactical view is therefore not forced to be 3D, and a strategic view is not forced to be 2D. The same source world can be explored in multiple views without changing physics state, object identity, measurements, or simulation time.

These presets are a host-renderer contract. A UI must apply them; declaring a profile alone does not create cameras, perform clustering, cull objects, or render overlays.

## Purpose profiles

| Purpose | Default detail policy | Information priority | Deliberately de-emphasized |
| --- | --- | --- | --- |
| Strategic | Aggregated | Terrain, infrastructure, networks, routes, system/resource distribution | Individual motion vectors and contact glyphs |
| Operational | Clustered | Regional routes, flows, infrastructure dependencies, changing activity | Unnecessary per-entity micro-detail at distance |
| Tactical | Per entity | Local geometry, selected entities, routes, contacts, velocity vectors, motion trails | Broad network summaries that obscure local constraints |
| Scientific | Per sample | Scalar/vector fields, uncertainty, provenance, legends, contextual geometry | Unqualified aggregation of measurement samples |
| Diagnostic | Per entity | Contacts, velocities, trajectories, field layers and validation metadata | Visual decoration that competes with debugging evidence |

Aggregate views must preserve member identities and provide a way to inspect the members. Focused/selected objects are intended to bypass ordinary draw-budget thinning. A draw budget is never a data-deletion limit.

## Navigation policies\n\n- Planar 2D uses pan/zoom.\n- Oblique 2.5D uses pan/orbit around an oblique overview.\n- Strategic/operational 3D use overview orbit; tactical 3D uses focused orbit around a selected entity/group; scientific/diagnostic 3D use inspection orbit.\n- 4D uses slice-position scrubbing plus pan/orbit inside the current slice.\n\nThese are interaction contracts only; the host application must wire input, camera transforms, focus and selection synchronization.\n\n## Dimensional representations

- **2D:** a true planar plan, map, circuit layout, network diagram, or slice. It does not pretend to encode elevation.
- **2.5D:** an oblique orthographic basis that separates horizontal position, ground depth, and height. Use it for city/infrastructure overviews, terrain, facility layouts and resource flows where height context helps but unrestricted orbiting is unnecessary.
- **3D:** full spatial geometry using a host-managed perspective or orthographic camera. Use it for assembly, robotics, collision review, human movement, architectural interiors and local inspection.
- **4D slice:** a 3D view of points projected to an arbitrary hyperplane in a 4D state space. The slice normal, offset and half-thickness must be explicit. The first implementation is a point/slice projection and does not claim exact continuous mesh or collider intersections.

A 2.5D projection helper is a coordinate transform, not a camera controller. The host camera must be aligned to its basis rather than applying an unrelated perspective transform on top of the projected coordinates.

## Validation invariants

- View purpose and dimensionality are independent.
- Invalid scales, angles, slice normals and thicknesses fail at construction.
- Four-dimensional profiles require a finite, positive slice window.
- Field layers require legends and provenance to remain available.
- Aggregation cannot discard the identities of source members.
- Projectors reject non-finite or unrepresentable coordinates for visibility.
- Projection and visibility checks are read-only; they do not alter simulation data.

## Benchmark protocol

Targets are proposed acceptance criteria, not measured results.

1. **Projection math:** compare 2.5D coordinates against hand-computed fixtures at overhead, oblique and near-horizon elevations; test center translation, azimuth and height.
2. **4D geometry:** test the axis-aligned W slice, at least two oblique normals, nonzero offsets, exact boundary behavior, degenerate normals and non-finite coordinates. Check basis orthonormality and preservation of distances within each view plane.
3. **Purpose behavior:** ensure strategic profiles prefer aggregation, operational profiles preserve route/network context, tactical profiles do not aggregate selected local entities, and scientific profiles keep field legends/provenance visible.
4. **Visual regression:** hold scene, simulation tick, camera parameters, field data, resolution and render settings fixed. Compare image differences only across compatible receipts; compare different purposes against their own approved reference images rather than treating a different composition as a regression.
5. **Scale and performance:** benchmark 100, 1,000, 10,000 and (where feasible) 100,000 entities/samples; report p50/p95/p99 frame interval, memory, draw count, culling/aggregation time and input workload.
6. **Semantic correctness:** toggling view layers or changing projection must not mutate physics state, measurement values, source identities, or simulation tick. Any aggregate must retain a recoverable mapping to its source members.

## Integration boundaries

- These helpers do not yet wire interactive camera controls, scale-dependent entity clustering, map generalization, cross-view selection state, rendering budgets, or automated screenshot tests into a host application.
- The host application must provide camera framing, viewport-aware visibility, legends, selection synchronization and layer UI.
- Continuous 4D surfaces require an explicit mesh/collider intersection pipeline; projecting body centers is not a substitute.
- Strategic/operational/tactical are display intent labels, not authoritative simulation modes, permissions, or changes to the underlying world state.

## Reference points

- OGC I3S Level of Detail describes thinning, clustering, generalization and identity-preserving levels of detail: https://docs.ogc.org/cs/17-014r5/17-014r5.html
- USGS multiscale representation describes matching feature detail to the map extent and display purpose: https://www.usgs.gov/centers/cegis/science/multiscale-representation
- Bevy orthographic view example provides a reference for isometric-look camera configuration: https://bevy.org/examples/3d-rendering/orthographic/
- NIST visualization work describes quantitative evaluation and uncertainty-aware scientific visualization: https://www.nist.gov/programs-projects/visualization

