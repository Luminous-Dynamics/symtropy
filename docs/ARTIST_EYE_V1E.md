# ARTIST-EYE-v1E — Object-ID GPU Acquisition + Object/Depth Fusion

Status: implementation / preregistration, not empirically qualified

## Purpose

v1E closes the host gap left intentionally by v1D. It adds a real Bevy object-ID
render/readback adapter and joins the resulting persistent object labels to the
already-linearized metric depth plane.

The target causal stack is:

```text
committed Bevy scene
       |
       +-- color evidence
       +-- linear metric depth evidence
       +-- isolated object-ID proxy render
                     |
                     v
          exact stable object identity
                     +
               metric distance
                     +
        persistent semantic identity
                     +
             recorded camera pose
                     v
        conservative scene evidence
```

The word *conservative* is essential. This release does not define aesthetic
value and it does not infer physical causality merely from optical change.

## 1. Isolated object-ID GPU pass

The object-ID pass must never recolor or material-swap the committed artistic
scene. `ObjectIdGpuSource` snapshots the stable ID, mesh handle, host-computed
world transform, and visibility of each planned artistic mesh. The adapter then
builds one-frame proxy entities on a dedicated render layer and a dedicated
camera/target.

Each proxy receives the exact raster ID already frozen in
`ObjectIdRenderPlan`. IDs are encoded losslessly as four little-endian bytes in
an `Rgba8Unorm` render target. Tonemapping and MSAA are disabled for this
qualification path.

After exactly one host render frame, the evidence camera and all proxy entities
are despawned before asynchronous readback begins.

The committed scene therefore remains the authority-bearing world; the proxy
pass is only an evidence instrument.

## 2. Registry discipline

Object-ID zero is background. Non-zero IDs are assigned by the frozen
`ObjectIdRegistry` and are bound to the prospective plan before rendering.
Unknown non-zero IDs in a readback fail closed.

v1E retains the v1D limitation that registry membership is frozen within one
confirmatory/session lineage. Open-ended object creation needs a future
append-only registry-lineage protocol or another reviewed persistent encoding.

## 3. Object/depth fusion

`fuse_object_id_and_linear_depth` accepts only an object receipt and a depth
receipt that independently validate and match on:

- revision;
- studio frame;
- semantic scene hash;
- stable camera identity;
- width and height.

The depth samples must already be linear positive distance. Device-Z never
enters the fusion API as though it were meters.

The resulting `ObjectDepthFusionFrame` preserves a cognitive-resolution aligned
pixel plane and reports per-object:

- visible pixels;
- valid depth pixels / fraction;
- minimum depth;
- p10 depth;
- median depth;
- p90 depth;
- maximum depth.

There is no aggregate scene or artistic score.

## 4. Depth-takeover evidence

v1D intentionally used `RasterVisibilityLost` and `RasterVisibilityAcquired`
rather than `Concealed` and `Revealed`. v1E introduces the first mechanism by
which that evidence can be strengthened.

`assess_depth_takeover` can return `DepthTakeoverSupported` only when all of the
following are true:

1. the target exists semantically at both endpoints;
2. the event is not semantic creation/destruction or authored hide/show;
3. a genuine raster visibility loss/gain occurred;
4. target semantic-transform change remains within prospectively frozen bounds;
5. camera pose change remains within prospectively frozen bounds;
6. another registered object occupies enough of the target's corresponding
   pixel support in the opposing frame;
7. enough depth-comparable takeover pixels place that object prospectively far
   enough in front of the target.

The thresholds are deliberately not defined by a `Default` implementation.
Confirmatory values must be reviewed and frozen before seeing study outcomes.

Even after those gates pass, the scientific label is **depth-takeover support**.
A stronger `ConcealedBy(object)` / `RevealedFromBehind(object)` ontology should
be promoted only after VART-OCC-001 replication establishes that the mechanism
has acceptable false-positive behavior.

## 5. Perception / authority boundary

```text
object ID     metric depth      semantic history      camera pose
    \             |                  |                   /
     \            |                  |                  /
                   descriptive evidence
                           |
                     artistic cognition
                           |
                      proposal only
                           |
                  normal art-world authority
```

No v1E evidence type defines beauty, utility, reward, fitness, trust, punishment,
or mutation authority.

## 6. VART-OBJ-GPU-001 — live object-ID acquisition

Preregister a small known scene containing several non-overlapping and partially
overlapping stable mesh objects whose raster IDs are frozen before execution.

Primary gates:

- exact expected non-zero IDs and zero background only;
- zero unknown IDs;
- deterministic registry digest;
- byte-exact repeated static renders under the frozen GPU/backend configuration,
  subject to a prospectively declared exactness policy;
- zero readback drops;
- proxy/evidence render leaves the committed semantic scene hash unchanged;
- object centroids / areas agree with analytically or independently generated
  simple-scene expectations within prospectively frozen tolerances.

Adversarial controls must include wrong registry, wrong scene identity, unknown
raster ID, readback backpressure, and omitted planned proxy source.

## 7. VART-OCC-001 — depth-takeover mechanism

Use fresh scene seeds and a controlled target/occluder family. At minimum:

- static target + static camera + occluder enters in front;
- static target + static camera + occluder exits;
- target authored hidden with an apparent competing object;
- target semantically destroyed;
- target translates out of frame with no occluder;
- camera moves enough to violate the frozen static-camera bound;
- competing object appears behind the target;
- insufficient takeover area;
- sufficient area but insufficient depth margin;
- correct front-occluder control.

Freeze before execution:

- takeover fraction;
- closer-depth fraction;
- depth margin;
- target semantic-transform stability bounds;
- camera stability bounds;
- minimum independent run count;
- false-positive / false-negative acceptance criteria;
- GPU/backend and depth/object acquisition profiles.

The threshold template intentionally leaves these fields null until prospective
review.

## 8. What a v1E PASS would and would not mean

A clean v1E qualification would support the claim that, for the qualified Bevy
path, Symthaea can bind persistent object identities to metric depth and detect
a preregistered object-depth takeover mechanism across time.

It would not establish unrestricted object permanence, human-like vision,
physical causality in arbitrary scenes, aesthetic competence, subjective
experience, or active policy authority.
