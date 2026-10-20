# RENDER-SOLAR-VIEWER-005 — First continuous solar-scale traversal prototype

**Status:** Draft runnable Bevy prototype. It is not a scientific ephemeris renderer, a qualified GPU result, or an industry-comparable benchmark.

## Goal

Exercise the scale transition as one scene, not as unrelated screenshots. The prototype lets a user navigate through local engineering detail, a planetary surface proxy, low orbit, the Earth-Moon system, an inner-system overview, an outer-system overview, and back to the same local feature.

The viewer uses the render bridge's large-world coordinate and apparent-size primitives. Canonical body positions and radii are stored separately from render scaling and presentation. The frame is a single illustrative coordinate frame; the orbital configurations are deliberately illustrative and must not be interpreted as physical positions at a real epoch.

## Controls

- Number keys 1–7 select the corresponding view stage.
- Space advances to the next stage.
- The camera interpolates between waypoints; each update derives render transforms relative to the current camera origin rather than moving canonical positions.
- The HUD reports the active stage, camera transition state, visible bodies, rejected precision conversions, apparent-size bands, and the largest visual-radius exaggeration.

The HUD explicitly labels the orbits as illustrative, not ephemeris.

## What is implemented

- A static catalog of the Sun, Mercury, Venus, Earth, Moon, Mars, Jupiter, Saturn, Uranus and Neptune, using reference radii and approximate distance scales as fixture anchors.
- Deterministic circular guide paths for the eight planet orbits and the Moon, rendered as camera-relative segments. Path geometry and line thickness are presentation guides, not integrated orbital states.
- A deterministic synthetic skyfield for background texture. These are angular markers attached to a camera-relative background shell, not stars with physical positions or a catalogue.
- Canonical LargeWorldPosition values represented by cell addresses and local offsets.
- Camera-relative transform updates and explicit precision-budget checks.
- Perspective apparent-diameter estimates and apparent-size bands.
- Presentation-only minimum-size exaggeration for subpixel distant objects, recorded in the HUD and never written back to physical radius.
- A surface proxy stage with a local feature, plus system and planet proxy views.
- A slowly interpolated camera/near-far profile transition.
- Deterministic circular guide paths for the eight planets and the Moon, split into reusable cuboid segments, positioned relative to the camera and hidden in local/surface views. The guides are illustrative and use a presentation-only thickness for overview readability.

## What is not implemented

- No NASA/JPL SPICE integration, epoch, timescale, body-fixed orientation, or dynamically computed ephemeris.
- No surface imagery/DEM tiles, terrain LOD, atmospheres, rings, eclipses, orbital dynamics or spacecraft trails. Orbit guides are static circular geometry, not integrated paths; the synthetic stars are not an astronomical catalogue.
- No high/middle/low GPU vertex shader path; this prototype uses budget-checked camera-relative f32 transforms. Shader-space precision still needs separate validation.
- No qualified depth strategy. The stage-dependent near/far ranges are fixture inputs, not a recommendation for the final renderer.
- No GPU timestamp receipt or matched comparison against Gaia Sky, Cesium, Unreal or Unity.

## Acceptance path

1. Formatting, viewer unit tests, strict Clippy and a locked build must pass on the exact head.
2. Run a fixed traversal twice and compare selected body/feature identity, position/origin continuity, stage transitions, visible-body bands and visual exaggeration.
3. Capture each waypoint with source revision, fixture digest, camera settings, adapter, driver and backend.
4. Compare 32-bit camera-relative transforms with the high/middle/low CPU reference and then with the actual vertex shader once integrated.
5. Measure frame intervals separately from GPU execution time. Record memory/residency, visible instances, triangles and temporal popping.
6. Replace the illustrative catalog with a versioned SPICE/ephemeris adapter only after reference frame, epoch/time scale, kernel digest and provenance contracts are established.

The initial goal is navigation and precision stress testing, not claiming a scientifically correct solar system.