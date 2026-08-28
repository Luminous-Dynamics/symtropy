# Symthaea Real-Time Art Studio v2 — Bevy Execution Plan

Status: RT1 substrate implemented; GPU capture/preview-world mutation intentionally deferred.

## Goal

Make Bevy a living artistic medium that Symthaea can inhabit continuously rather than a one-shot renderer. The studio must support real-time 3D art, cinematic camera planning, interactive worlds, counterfactual previews, embodied technique learning, and persistent artistic history without coupling art observation to scene-mutation authority.

## Implemented RT1 substrate

`crates/bridges/symthaea-bevy-brain` now contains:

- `art_timeline`: deterministic studio frame identity and bounded frame-pacing receipts;
- `art_capture`: exact revision/frame capture requests, receipts, and bounded backpressure;
- `art_counterfactual`: revision-isolated preview branch lineage with no scene mutation handle;
- `art_cinema`: shot/sequence plans, camera paths, multidimensional evidence, and cinematic history;
- `art_runtime`: opt-in `RealtimeArtStudioPlugin` that initializes studio time, capture queue, pacing ledger, and cinematic history.

The existing `ArtPort` remains the proposal/authority boundary. None of the new modules can mutate the Bevy scene by themselves.

## Minimal host setup

```rust,ignore
use bevy::prelude::*;
use symthaea_bevy_brain::{
    RealtimeArtStudioPlugin, StudioFrameRate, SymthaeaBrainPlugin,
};

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(SymthaeaBrainPlugin::default())
    .add_plugins(RealtimeArtStudioPlugin::new(
        StudioFrameRate::new(24, 1).unwrap(),
    ))
    .run();
```

The studio plugin is explicit and opt-in so existing Symtropy experiences do not acquire art timing/capture behavior by merely depending on the brain crate.

## Timing model

The artistic frame coordinate is deterministic and independent of wall-clock duration. That allows expensive analysis to be deferred while all evidence still refers to an exact artistic frame.

```text
FixedUpdate
   |
   +--> StudioClock frame N
   |
   +--> normal world simulation
   |
   `--> cheap capture request bookkeeping

render/GPU readback
   |
   `--> bounded capture receipt

background/deferred
   |
   +--> Vision/scene analysis
   +--> candidate comparison
   `--> portfolio consolidation
```

Frame-pacing evidence records wall/simulation/capture duration separately and is bounded. Dropped pacing samples are counted explicitly.

## Capture contract

A valid capture binds:

```text
capture_id
revision_id
studio frame
semantic scene hash
camera stable id
resolution
purpose
render channels
```

The receipt repeats revision, frame, and scene hash. Cross-frame or cross-revision pixels fail closed.

The queue is bounded with explicit `RejectNewest` or `EvictOldest` behavior. Real-time stability is more important than pretending every requested observation was collected.

## Counterfactual isolation

`CounterfactualRegistry` deliberately stores only branch identity and preview evidence. It has no `Commands`, `World`, `Assets`, or mutable scene handle.

```text
committed revision R
      |
      +--> preview branch A
      +--> preview branch B
      +--> preview branch C
      `--> abstain baseline

preview creation/render/disposal != committed mutation
```

A real host commit is observed only after a separately authorized adapter has already advanced the scene. Observing that new revision disposes stale previews.

## Cinematic direction

A shot plan is revision-bound and carries:

- start/end artistic frames;
- stable camera identity;
- ordered camera keyframes;
- artistic intention reference;
- scheduled proposal IDs;
- notes/evidence.

A sequence validates that shots share one base revision, have unique IDs, and do not overlap.

Candidate evidence remains multidimensional. There is no cinematic or beauty score field.

## RT2 — GPU render observation

Next implementation should add a dedicated adapter, not logic inside `ArtPort`:

1. create a render target `Image` for one explicit studio camera;
2. associate camera/render-target state with stable art IDs;
3. enqueue `ArtCaptureRequest` at the end of committed scene preparation;
4. perform asynchronous GPU readback;
5. hash/locate the resulting artifact;
6. emit `ArtCaptureReceipt` with the original revision/frame/scene hash;
7. drop/evict explicitly when readback cannot keep pace.

Recommended first observation resolution is intentionally modest (for example 320x180 or 512x288) so whole-scene cognition can run frequently while portfolio/export renders use a separate fidelity class.

Do not compare preview and committed evidence without recording render fidelity.

## RT3 — preview world / proposal ghosts

Use one of two strategies, evaluated empirically:

### A. Dedicated preview layer

Clone only tagged art entities into a preview namespace/render layer. Apply a proposal there, render it, then despawn it.

Advantages: cheap, integrates with normal Bevy renderer.

Risk: accidental shared mutable assets/resources.

### B. Secondary Bevy `World`

Construct a small isolated preview world from canonical `ArtSceneRecord` data.

Advantages: stronger mutation isolation and deterministic replay.

Risk: more adapter code and asset synchronization.

The acceptance gate is the same: N previews followed by disposal must leave the committed scene semantic hash unchanged relative to a no-preview control.

## RT4 — whole-scene artistic eye

Each committed/counterfactual render should be perceived at multiple scales:

- global value/color distribution;
- negative-space and silhouette structure;
- focal hierarchy;
- edge/texture density;
- depth layering;
- motion/optical-flow structure;
- repetition and temporal recurrence;
- semantic scene relationships.

Pixel evidence and semantic scene evidence must share revision/frame identity.

The perception system predicts consequences; it does not decide universal aesthetic quality.

## RT5 — physical artistic hand

Introduce one learnable physical medium before many tools. Recommended first target: deformable clay/sculpting because 3D geometry, contact, force, and silhouette consequences are easy to observe.

Then add:

- brush-on-surface;
- palette knife / impasto;
- charcoal/eraser;
- camera dolly/gimbal body;
- lighting rig;
- procedural field controls.

Research loop:

```text
intention
 -> motor trajectory
 -> tool contact
 -> material consequence
 -> re-observation
 -> prediction residual
 -> technique memory
```

Compare direct semantic API control against embodied practice and a hybrid artist that can transfer learned concepts between them.

## RT6 — real-time cinema and video

Bevy should render and direct the living world. Encoding stays downstream:

```text
Bevy frame/audio stream
   +--> FFmpeg/GStreamer --> archival video
   `--> WebRTC -----------> live performance
```

Camera planning, world simulation, temporal motifs, lighting changes, agent behavior, and music coupling remain inside the studio's artistic/causal model.

## RT7 — impossible media

Symtropy gives the studio materials normal film tools do not:

- reaction-diffusion pigments;
- physically evolving/destructible sculpture;
- audio-responsive material fields;
- gravitational/field sculpture;
- living mycelial/ecological media;
- topology-changing surfaces;
- higher-dimensional geometry projected into 3D;
- autonomous agents that become part of the work;
- persistent installations that evolve over days/months.

The research question is not whether these effects are flashy. It is whether a persistent artist can learn their causal affordances and use them intentionally.

## Determinism and provenance

Every committed artistic scene should retain:

```text
world/revision hash
studio frame
artistic question / intention
proposal/action IDs
preview branch lineage
capture receipts
camera/shot plan
selected and rejected candidates
physical interaction receipts where applicable
render fidelity
export/video artifact digest
```

Do not use Bevy runtime `Entity` IDs as long-term artistic identity.

## Performance budgets

Treat workloads differently:

| Class | Examples | Rule |
|---|---|---|
| Frame-critical | physics, camera transforms, rendering | never wait for cognition |
| Cycle-critical | proposal execution, cheap capture enqueue | bounded work only |
| Deferred | visual analysis, counterfactual batches, critique | asynchronous/budgeted |

Backpressure must be observable. Queue depth, dropped capture IDs, frame pacing, and preview latency are evidence—not reasons to silently skip provenance.

## Qualification gates

Current RT1 code:

```bash
cargo fmt --all -- --check
cargo check -p symthaea-bevy-brain --all-targets
cargo test -p symthaea-bevy-brain
cargo clippy -p symthaea-bevy-brain --all-targets -- -D warnings
```

Specific invariants:

- `RealtimeArtStudioPlugin` is opt-in;
- existing cognitive behavior is unchanged when the plugin is absent;
- studio frame identity does not depend on wall time;
- capture queues are bounded and report exact drops/evictions;
- capture receipts reject revision/frame/hash mismatch;
- preview branch creation never advances committed revision;
- stale preview bases fail closed;
- camera values are finite and keyframes strictly ordered;
- sequence shots cannot cross revision or overlap;
- cinematic candidate evidence has no scalar score.

## Research program

- **VART-RT-001** temporal/revision integrity;
- **VART-RT-002** counterfactual isolation;
- **VART-RT-003** held-out consequence prediction;
- **VART-RT-004** embodied technique acquisition;
- **VART-RT-005** long-horizon artistic continuity;
- **VART-RT-006** interactive direction under viewer/world perturbation.

The north-star is not a prettier single frame. It is a system that can originate an artistic question, direct a causal world over time, learn its tools, revise itself, abstain, preserve history, and transfer what it discovers into other studios.
