# ARTIST-EYE-v1B — Depth, Occlusion, and Spatial Layering

Status: **implementation tranche; not yet Rust/Nix or live-GPU qualified**

ARTIST-EYE-v1B extends the deterministic v1A spatial eye with explicit depth evidence. It does not replace v1A and it does not define artistic value.

## Scientific boundary

The renderer may expose a hardware depth attachment, a dedicated linear-depth material pass, or another host-specific depth source. The evidence layer accepts only an explicitly declared depth encoding and converts it into linear camera-space distance before analysis.

Raw device-Z values must **not** be passed as `Linear01` unless the host has already linearized them. Perspective device depth is generally non-linear. A host qualification receipt must record how the plane was linearized, the camera near/far parameters when applicable, reverse-Z convention, source format, and projection identity.

Depth evidence remains distinct from:

- artistic preference;
- salience or attention;
- visual-semantic recognition;
- a beauty/utility/reward/fitness objective;
- scene mutation authority.

## Evidence surface

For every depth plane v1B retains separate dimensions:

- valid-sample fraction;
- clipped-far fraction;
- minimum / p10 / median / p90 / maximum linear depth;
- depth span;
- near / middle / far occupancy fractions under frozen metric boundaries;
- normalized centroid of the nearest depth quartile;
- normalized centroid of the farthest non-clipped quartile;
- horizontal depth-discontinuity fraction and mean delta;
- vertical depth-discontinuity fraction and mean delta;
- combined occlusion-boundary fraction.

No aggregate depth quality score is defined.

## Four-ghost integration

Depth may be rendered in a separate pass from color. `FourGhostArtistDepthEvidenceSet` therefore requires a dedicated depth capture receipt for each candidate. Before computing any proposal-minus-baseline consequence it checks that the depth receipt matches the candidate's:

- base revision;
- studio frame;
- rendered semantic scene hash;
- stable camera identity;
- width and height;
- declared depth channel.

The baseline receives no artificial consequence vector. Each proposal receives a descriptive candidate-minus-baseline depth consequence vector.

## VART-DEPTH-001 — deterministic depth evidence

Purpose: establish that the measurement layer responds in the prospectively expected direction to simple, known geometric interventions.

Use a frozen camera and a simple scene family with fresh scene seeds. Before execution, freeze practical tolerances for each expected direction.

Interventions:

1. **Planar control** — one fronto-parallel plane at a constant metric depth.
   - expected: near-zero depth span;
   - expected: near-zero discontinuity fraction.

2. **Near/far split** — left half near, right half far.
   - expected: non-zero horizontal-neighbor depth boundary evidence;
   - expected: near and far occupancy both increase relative to the planar middle-depth control.

3. **Object translation toward camera** — move one isolated object closer without changing camera.
   - expected: median or lower quantile depth decreases where the object occupies substantial area;
   - expected: near-depth centroid follows the object's screen-space motion.

4. **Occluder insertion** — insert a foreground occluder in front of a background object.
   - expected: depth-discontinuity evidence increases;
   - expected: near-layer occupancy increases.

5. **Camera-only dolly** — translate the camera toward a static scene.
   - expected: absolute metric depths change coherently;
   - semantic scene identity must still reflect the camera state used by the art-world host contract.

6. **Lighting-only control** — alter light while preserving geometry and camera.
   - expected: v1A color/value evidence may change;
   - expected: v1B metric depth evidence remains within preregistered tolerance.

The independent unit is the scene seed, not individual pixels.

## VART-DEPTH-002 — four-ghost depth consequences

Run one baseline plus exactly three proposal ghosts with synchronized color and depth passes. A confirmatory episode requires all four color captures and all four depth captures. Any missing, evicted, mismatched, or unlinearized required depth plane invalidates the episode.

A useful first proposal family is:

- A: move the form laterally at constant depth;
- B: move the form toward the camera;
- C: insert an occluder;
- baseline: abstain.

Expected qualitative separation:

- A changes v1A spatial placement strongly but should leave global metric-depth distribution comparatively stable;
- B changes depth distribution and near-layer evidence;
- C changes depth-discontinuity / occlusion-boundary evidence;
- baseline defines zero consequence by construction.

This is a causal discrimination study, not an aesthetic ranking study.

## Next tranche

After v1B qualifies, ARTIST-EYE-v1C should add temporal evidence: motion fields, focal migration, reveal/concealment, persistence, shot continuity, and temporal rhythm. Depth and motion should then be combined to distinguish camera motion, object motion, and genuine reveal/occlusion events.
