# ARTIST-EYE-v1A — Multi-Scale Spatial Vision

Status: **preregistered implementation tranche; Rust/Nix and live-GPU qualification pending**.

ARTIST-EYE-v1A upgrades the live four-ghost loop from cheap whole-frame scalar measurements to a deterministic hierarchy of spatial evidence. It remains a perception substrate, not an aesthetic judge.

## Scientific boundary

The following must remain distinct:

```text
pixel / scene evidence
        !=
visual attention
        !=
artistic intention
        !=
aesthetic judgment
        !=
choice
        !=
commit authority
```

No ARTIST-EYE-v1A field is a reward, beauty score, utility, fitness value, or direct policy-control signal.

## Evidence layers

Each GPU readback is converted into a luminance pyramid. At every retained scale the eye records:

- value masses: dark / middle / light fractions;
- border-referenced silhouette occupancy and negative-space fraction;
- connected-component counts and largest-component fractions for occupied and negative space;
- occupied border contact;
- edge-orientation energy across horizontal, vertical, and two diagonal families;
- mean gradient magnitude;
- left-right and top-bottom reflection mismatch;
- a bounded set of coarse focal regions carrying separate value separation, local contrast, and local edge energy;
- descriptive focal separation/concentration evidence.

The border-derived silhouette is explicitly a deterministic *measurement heuristic*. It is not semantic object segmentation and must not be reported as such.

## Multi-scale rule

The source image is level 0. Subsequent levels use deterministic 2x box reduction. A level is identified by `(level, width, height)` and candidate-minus-baseline comparison refuses mismatched pyramid shapes.

This allows evidence such as:

```text
fine scale:   many small edges / texture
medium scale: two major value masses
coarse scale: one dominant spatial split
```

without collapsing those observations into one number.

## Four-ghost binding

`FourGhostArtistEyeEvidenceSet` is valid only when all four observations correspond exactly to the candidate capture receipts already frozen by `FourGhostRenderSet`:

- same base revision;
- same studio frame;
- exact candidate capture ID;
- exact rendered semantic scene hash;
- exactly one abstention baseline + three proposal candidates.

Proposal evidence is stored as proposal minus baseline for every pyramid level. The abstention baseline carries no consequence vector.

Missing, duplicated, or misbound evidence invalidates the set.

## VART-EYE-001 — deterministic spatial sensitivity

### Goal

Establish that ARTIST-EYE-v1A produces stable, causally interpretable changes for simple scene interventions before it is used by artistic choice or VisionManifold.

### Frozen scene family

Use simple high-contrast scenes whose expected spatial consequences can be stated before rendering. At minimum include:

1. centered single form on approximately uniform background;
2. form translated laterally without changing material;
3. one connected form split into two disconnected forms;
4. background value shift with form held fixed;
5. dominant vertical boundary rotated to horizontal;
6. a focal high-contrast region moved from center toward a corner.

The host must save the exact scene seed, camera, render fidelity, revision, frame, GPU/driver identity, feature flags, and all capture receipts.

### Confirmatory invariants

The run is invalid if any required GPU readback is dropped, truncated, misaligned, or silently substituted.

For identical input bytes, the complete ARTIST-EYE evidence structure must be exactly deterministic on the same qualified binary/environment.

The preregistered scene manipulations should produce the corresponding directional evidence without relying on post-hoc threshold tuning. Examples:

- splitting one form should not reduce occupied connected-component count when the split is visually resolved at the tested scale;
- moving a focal region from center to corner should reduce center-localized focal evidence at a scale where the region is resolved;
- rotating a dominant edge family by 90 degrees should exchange horizontal/vertical orientation dominance;
- a pure background-value intervention should be distinguishable from a form-geometry intervention in at least one separate evidence channel.

Numeric tolerances and minimum effect magnitudes must be frozen before the first confirmatory VART-EYE-001 run.

## VART-EYE-002 — four-ghost consequence attribution

After VART-EYE-001 passes, run one full live four-ghost episode where the three proposals intentionally target different spatial dimensions, for example:

```text
A  move the form
B  split / duplicate the form
C  change lighting or value structure
```

Success requires all four real GPU readbacks, valid multi-scale evidence for all four candidates, and distinct consequence records bound to the exact proposal preview hashes. Selection remains outside this measurement layer.

## VisionManifold bridge — deferred

ARTIST-EYE-v1A should not directly replace Symthaea's VisionManifold. The later bridge should preserve two evidence planes:

```text
deterministic artist-eye geometry
            +
learned / HDC visual manifold representation
```

Top-down attention may influence what is sampled or emphasized later, but must not rewrite the underlying captured evidence.

## Next tranches

- **v1B Depth & Occlusion**: depth-buffer evidence, foreground/midground/background structure, occlusion boundaries, depth discontinuities.
- **v1C Temporal Eye**: frame-to-frame motion, focal migration, reveal/occlusion events, shot continuity and temporal rhythm.
- **v1D VisionManifold bridge**: causal pairing of deterministic spatial evidence with learned HDC representations and top-down attention.
