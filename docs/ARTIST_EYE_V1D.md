# ARTIST-EYE-v1D — Persistent Object & Motion Identity

Status: implementation + preregistration; **not qualified until the v1D qualification contract passes under Nix and live object-ID acquisition is separately validated**.

## Purpose

ARTIST-EYE-v1A measures 2D composition, v1B measures depth/occlusion structure,
and v1C measures temporal change. v1D adds a new question:

> Which persistent artistic entity produced which visible pixels, and how did
> that same entity change across time?

The design keeps four evidence planes separate:

1. **semantic identity** — stable `ArtEntityId` / `ArtSceneRecord` state;
2. **raster identity** — object-ID pixels saying which stable entity was visible;
3. **camera motion** — explicit stable-camera pose change;
4. **screen trajectory** — centroid/visible-area change in the object-ID raster.

No plane is an aesthetic score or mutation authority.

## Identity contract

A qualification/session lineage freezes an `ObjectIdRegistry` before outcomes
are inspected. Zero is background. Every non-zero raster label must resolve to
exactly one stable artistic ID. Unknown labels fail closed.

The registry digest is bound into every `ObjectIdObservation`; changing the
registry inside one confirmatory window invalidates the transition.

### Session-bound registry limitation

v1D intentionally treats the registry as frozen within one confirmatory
lineage. An open-ended studio that creates arbitrary new stable objects will
need a reviewed append-only registry-lineage protocol or a stable collision-
checked encoding before cross-epoch tracking is authorized. v1D does **not**
silently renumber identities during an experiment.

## Semantic frame integrity

`SemanticObjectFrame::validate()` reconstructs `ArtSceneRecord` values from the
retained semantic states and recomputes `stable_scene_hash`. A caller cannot
supply an arbitrary scene-hash string and have it accepted merely because the
raster copied the same string.

`ArtSceneRecord` currently stores the host-provided `Transform`. This may be a
local transform in a hierarchy, so v1D calls its deltas **semantic-transform
evidence**, not global/world motion.

## Visibility terminology

v1D deliberately distinguishes:

- `SemanticCreated` / `SemanticDestroyed`;
- `AuthoredVisibilityEnabled` / `AuthoredVisibilityDisabled`;
- `RasterVisibilityAcquired` / `RasterVisibilityLost`.

A raster loss is **not automatically called concealment** and a raster gain is
**not automatically called reveal**. Those causal labels require independent
occluder evidence. This prevents the eye from confusing creation, clipping,
camera motion, authoring visibility and physical occlusion.

## Conservative motion attribution

`attribute_transition_motion()` compares three independent changes:

- semantic-transform delta;
- camera-pose delta;
- object screen-centroid delta.

It can report:

- `NoTrackedMotion`;
- `SemanticTransformMotion`;
- `CameraMotionWithSemanticTransformStable`;
- `MixedCameraAndSemanticTransformMotion`;
- `UnattributedScreenMotion`;
- `NonScreenMotionEvidence`;
- `VisibilityTransition`.

`UnattributedScreenMotion` is important: a visible object can move on screen
without a recorded semantic transform or camera movement because of hierarchy
motion, deformation, animation omitted from the semantic plane, a render
artifact, or a measurement defect. v1D preserves that discrepancy instead of
forcing a convenient explanation.

## Persistent windows

A `PersistentObjectWindow` requires:

- at least two frames;
- one stable camera identity;
- strictly increasing frame numbers;
- a frozen maximum frame gap;
- the same registry lineage;
- valid semantic hashes at every frame.

It reports track evidence separately:

- semantic-present frame count;
- raster-visible frame count;
- raster visibility acquisitions/losses;
- semantic creations/destructions;
- authored visibility changes;
- cumulative semantic-transform displacement;
- cumulative screen path;
- maximum visible fraction;
- cumulative camera translation/rotation when camera poses are available.

There is no aggregate motion-quality or cinematic-quality scalar.

# VART-OBJ-001 — Persistent identity discrimination

## Goal

Confirm that stable semantic identities and object-ID raster identities remain
correctly bound under controlled scene changes.

## Freeze before execution

- exact qualified HEAD/TREE;
- object registry and registry digest;
- camera stable ID;
- resolution and object-ID target format;
- frame cadence and maximum frame gap;
- object set and scene seed;
- raster-label decoding rule;
- expected identity/event outcomes for each intervention.

## Conditions

1. **Static control** — same semantic IDs and same raster locations.
2. **Translate A** — only object A semantic transform and screen trajectory change.
3. **Translate camera** — object semantic transforms stable; screen locations change.
4. **Create B** — B produces `SemanticCreated`; raster acquisition is recorded separately.
5. **Destroy B** — B produces `SemanticDestroyed`; raster loss is recorded separately.
6. **Authored hide/show** — authored visibility events remain separate from create/destroy.
7. **Unknown raster label** — the unit must fail closed.
8. **Registry mutation** — cross-frame transition must be rejected.
9. **Forged semantic hash** — semantic validation must reject it.
10. **Row padding sentinel** — padding must never create a visible object.

## Confirmatory unit

A unit passes only if every preregistered stable identity, event family, raster
centroid direction and fail-closed control matches its frozen expectation.
Missing object-ID evidence invalidates the unit; it is not complete-case dropped.

# VART-MOT-001 — Motion-plane discrimination

## Goal

Test whether semantic-transform, camera-motion and screen-motion evidence remain
separable under controlled interventions.

## Conditions

- static object + static camera;
- semantic object translation + static camera;
- static semantic object + camera translation;
- semantic object translation + camera translation;
- semantic depth-axis change with negligible centroid change;
- forced screen displacement with no semantic/camera explanation;
- raster visibility loss/gain.

## Expected qualitative outcomes

The confirmatory study freezes thresholds before execution and verifies the
corresponding `ObjectMotionAttribution` category. No category authorizes an
artistic action.

# Not yet established by v1D

A v1D PASS does **not** establish:

- causal occluder identity;
- physical rigid-body motion;
- skeleton/vertex/deformation tracking;
- optical flow or GPU motion-vector qualification;
- object permanence under long unobserved intervals;
- learned object recognition from raw pixels;
- aesthetic competence;
- subjective experience;
- active-policy authority.

## Next technical extension

After v1D qualification, the strongest continuation is a real Bevy object-ID
render/readback adapter plus object-depth fusion. That would allow per-object
median depth, depth trajectory and candidate occluder testing. Only after those
are qualified should the event ontology advance from `RasterVisibilityLost`
toward evidence-bearing `ConcealedBy(id)` / `RevealedFromBehind(id)` claims.
