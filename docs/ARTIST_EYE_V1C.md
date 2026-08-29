# ARTIST-EYE-v1C — Linear Depth Acquisition + Temporal Vision

Status: implementation / qualification candidate

ARTIST-EYE-v1C closes two gaps left deliberately open by v1B:

1. obtain a real Bevy 0.19 3D camera depth plane without calling nonlinear
   device depth "meters";
2. turn aligned spatial/depth observations into bounded temporal evidence
   without inventing a cinematic-quality scalar.

## Evidence pipeline

```text
committed / preview scene
        |
        +--> color target --> GPU readback --> ARTIST-EYE-v1A spatial evidence
        |
        +--> ViewDepthTexture
                 |
            exact camera
            exact host frame
                 |
            Depth32Float copy
                 |
            async readback
                 |
       explicit projection provenance
                 |
       device depth -> linear meters
                 |
        ARTIST-EYE-v1B depth evidence
                 |
                 +-------------------+
                                     |
                              ARTIST-EYE-v1C
                              temporal transition
                                     |
                       +-------------+-------------+
                       |             |             |
                  focal motion  depth change  camera motion
                       |             |             |
                       +-------------+-------------+
                                     |
                           descriptive time window
```

Perception evidence remains separate from artistic intention, choice and commit
permission.

## Bevy depth acquisition contract

`ArtDepthReadbackPlugin` copies the `ViewDepthTexture` of a camera marked with
`ArtDepthCopyTarget` after the main 3D pass into a dedicated `Depth32Float`
image. The destination is not a continuously reused evidence surface.

`PreparedArtDepthCapture` is armed for one host render frame, then detached
before asynchronous readback. Its request must already declare the `Depth`
channel.

The adapter records an explicit `BevyDepthProjection`:

- `PerspectiveInfiniteReverseZ { near_meters, culling_far_meters }`
- `OrthographicReverseZ { near_meters, far_meters }`

Custom projections fail closed until they provide their own reviewed decoder.

For Bevy's standard infinite reverse-Z perspective path, positive linear
forward distance is reconstructed as:

```text
linear_distance = near / device_depth
```

Device depth zero is treated as missing/infinite background, not an arbitrary
finite distance.

## Temporal evidence contract

`ArtistTemporalFrame` binds:

- stable camera identity;
- one ARTIST-EYE-v1A spatial observation;
- optional aligned v1B depth observation;
- optional explicit camera pose.

A transition requires strictly increasing studio frames and a preregistered
maximum frame gap. Missing intermediate evidence is not silently interpolated.

The transition preserves separate channels:

- full v1A spatial consequence evidence;
- focal-region migration per pyramid level;
- optional v1B depth consequence evidence;
- camera translation and rotation;
- occupancy / negative-space change;
- optional depth-validity and occlusion-boundary change.

`ArtistTemporalWindow` may summarize descriptive change rates across multiple
transitions, but there is no aggregate cinematic-value, beauty, reward,
fitness, or policy signal.

## Important non-claim

v1C does **not** yet claim robust object-motion versus camera-motion
separation. Camera motion is measured explicitly, while scene/image/depth
change is measured independently. True object-level motion attribution should
wait for qualified object-ID / motion-vector evidence or persistent semantic
tracking.

## VART-DEPTH-LIVE-001 — depth reconstruction qualification

Freeze before the first confirmatory run:

- Bevy/WGPU/backend identity;
- GPU/driver;
- perspective or orthographic projection;
- exact near/far/culling parameters;
- render resolution;
- depth format and copy timing;
- acceptable absolute/relative metric reconstruction error;
- camera and scene seed.

Use analytically known planes/objects at several distances, including near,
middle and far placements. Include a background-only region to confirm that
perspective depth zero remains missing.

Required PASS properties:

1. no missing expected readback;
2. no depth completion-queue drop;
3. camera/revision/frame/scene identity preserved;
4. reconstructed metric depths within frozen tolerance;
5. monotonic near-to-far ordering preserved;
6. depth evidence stable under lighting-only intervention.

## VART-TEMP-001 — temporal evidence qualification

Prospective intervention family:

1. static camera + static scene;
2. camera-only lateral translation;
3. camera-only yaw;
4. object/form translation with fixed camera;
5. foreground occluder enters frame;
6. foreground occluder leaves frame;
7. focal high-contrast region migrates across screen;
8. deliberately missing/gapped capture negative control.

Expected directions are frozen before outcome inspection. Examples:

- static/static -> near-zero camera and perceptual change;
- camera translation -> nonzero camera translation evidence;
- focal migration -> nonzero focal migration at preregistered scales;
- occluder entry -> occupancy/depth-boundary evidence changes;
- missing/gapped capture -> fail closed rather than interpolate.

## Next milestone

After v1C qualifies, the next useful tranche is object/motion identity rather
than adding another scalar image statistic. Candidate work:

- object-ID render channel;
- persistent semantic object identity across frames;
- per-object screen/depth trajectory;
- motion-vector validation;
- camera-motion compensation;
- reveal/concealment events;
- shot continuity and recurrence evidence.
