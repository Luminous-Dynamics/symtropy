# RENDER-APPARENT-SCALE-002 — Select planetary-to-system representations by projected size

**Status:** Draft policy kernel. Twelve unit tests are authored; the latest exact-head check is queued, and no rendered solar-system result is claimed yet.

## Purpose

A viewer can stay at roughly the same physical camera distance while changing viewport size or field of view; a planetary body can also remain physically identical while moving from a surface-level scene to an orbital or solar-system view. A scale policy should therefore use projected size, not just world distance.

This increment adds a pure Rust apparent-size and band-transition policy to the render bridge. It is intended to sit above the cell/local coordinate foundation in PR #2432 and feed a later hierarchical scene/LOD renderer.

## Projected apparent diameter

For a spherical body with radius r and camera-centre distance d, with d greater than r, a pinhole perspective projection has projected diameter:

    D_px = H * (r/d) / (tan(vertical_fov/2) * sqrt(1 - (r/d)^2))

where H is the physical render viewport height. The ratio form avoids squaring very large astronomical distances.

For an orthographic projection with vertical span S:

    D_px = 2 * r * H / S

The helper fails closed for invalid radii, viewport dimensions, FOV, camera distances and orthographic spans, or unrepresentable output. A perspective camera inside or on the sphere is not assigned an ordinary apparent diameter.

This is a sphere-limb approximation. It does not include ellipsoids, rings, atmosphere, occlusion, terrain geometric error, star apparent magnitude, or mesh simplification bounds. Those remain separate parts of the host renderer's data and appearance contract.

## Apparent-size bands

A validated profile classifies projected diameter into six presentation bands:

- Subpixel
- Point source
- Resolved disc
- Whole-body proxy
- Regional detail
- Surface detail

The proposed default thresholds are 0.5, 1, 8, 64 and 256 pixels. They are starting hypotheses, not standards. Benchmark captures and task-specific readability tests should tune the thresholds, and different content families may need different profiles. Star rendering also needs apparent-magnitude and catalog semantics; a star should not be converted into a physical sphere merely because it is selected in a view.

A ScaleBandLatch adds symmetric hysteresis: promoting to more detailed representation requires crossing the threshold times (1+h), and degrading requires crossing the threshold times (1-h). Large camera moves can cross multiple bands in one update; small oscillations around a threshold should not cause frame-to-frame chatter.

Band classification never deletes the underlying entity and does not override selected/critical-feature protection. The presentation host must preserve body/feature identity, selection remapping and source provenance across proxy/tile changes.

## Frozen whole-scale workload

The companion fixture `benchmarks/render/SOLAR-SCALE-01.json` defines seven traversal stages from local engineering detail through planetary surface, low orbit, Earth-Moon, inner system, outer system and return-to-local. It also lists seven coordinate precision scales and two analytic apparent-size cases. The manifest is a planned workload contract, not evidence of a rendered solar-system scene.

## Authored tests

Nine unit tests cover:
- analytic known-answer perspective and orthographic projections;
- monotonicity with radius, viewport resolution, field of view and distance;
- invalid domains and camera-inside-body rejection;
- threshold-order validation and classification;
- boundary hysteresis and multi-band transitions;
- invariance when radius, distance and orthographic span are expressed in common units scaled from 10^-12 to 10^12;
- finite, monotonic perspective projection across distance decades;
- invalid hysteresis fractions and projected diameters.

These tests are authored. The exact-head Large World Coordinate workflow must pass before the twelve-test set is claimed as verified; the current amended head has a queued check.

## Next implementation steps

1. Pass exact-head format, focused tests and strict Clippy.
2. Connect LargeWorldPosition to a camera state that derives relative position without changing canonical coordinates.
3. Extend apparent sizing to oblate bodies, rings, atmospheres and uncertainty-aware star catalog rendering.
4. Bind bands to body proxies, planetary surface tiles and scale-aware labels, with deterministic refinement and identity retention.
5. Add fixed-camera captures at 1 km/local, planetary surface, low orbit, Earth-Moon, 1 AU and Neptune distance; record projected diameter, representation band, label/selection continuity, screen error, frame interval and GPU time separately.
6. Exercise one continuous camera journey from local inspection to system overview and back. Test origin changes and repeated threshold crossings in both directions.
7. Run the same frozen capture sequence on the reference renderer and Symtropy, using the same camera path, resolution, geometry/detail budget and output frames. Compare pixel-space error, selected-entity continuity, transitions, CPU/GPU frame-time distributions and peak resident memory; keep ephemeris-backed positions distinct from illustrative positions.

## Proposed acceptance ladder (targets, not results)

- **Mathematical kernel:** all analytic known-answer tests pass; common-unit rescaling changes apparent diameter by no more than 10^-12 relative on the tested finite domain; no non-finite projection result is accepted.
- **Semantic continuity:** zero lost selected/task-critical entities and zero canonical-position changes under camera-origin rebasing across the frozen local-to-Neptune-and-back path.
- **Representation quality:** zero unlabelled replacement gaps. For any transition with a validated geometric-error bound, remain within the declared screen-space error ceiling; do not declare a bound for illustrative proxies without a conservative mesh/camera error model.
- **Runtime (provisional):** target p95 frame time <=16.67 ms at 1080p on a pinned reference machine; report p50/p95/p99 CPU and GPU timings separately. Treat 4K and mobile-class hardware as separate profiles rather than extrapolating from one GPU.
- **Reproducibility:** every reported run binds source SHA, fixture/config digest, camera path, output resolution, graphics backend/driver and reference-frame/epoch metadata. Compare only matched workloads; a faster frame that drops selected content or violates error/identity invariants is a failure, not a win.

## Research/industry context

The OGC 3D Tiles specification uses geometric error in hierarchical levels of detail and screen-space error to decide refinement. That is a useful model for surface/tile hierarchy, although celestial object presentation also needs body identity and apparent-size rules:
- https://docs.ogc.org/cs/18-053r2/18-053r2.html
- https://github.com/CesiumGS/3d-tiles

Gaia Sky's documentation describes magnitude-dependent star pseudo-size. That is a better reference for the stellar background than representing every star as an ordinary mesh:
- https://gaia.ari.uni-heidelberg.de/gaiasky/docs/master/Star-rendering.html
- https://www.cosmos.esa.int/en/web/gaia/gaiadr2_gaiasky

## Claim ceiling

This policy computes apparent size and stabilizes presentation bands. It does not implement hierarchical astronomical content, ingest authoritative ephemerides, choose depth buffering, bind shader transforms, or render a solar system. It does not demonstrate superiority to Gaia Sky, Cesium, Unreal, Unity or other engines.
