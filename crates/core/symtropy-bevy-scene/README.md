# symtropy-bevy-scene

Opinionated scene/camera/light scaffolding for Bevy 0.19 apps. Eliminates the
camera-light-clear-color boilerplate every Symtropy demo otherwise repeats.

```toml
[dependencies]
symtropy-bevy-scene = "0.1"
```

```rust
use bevy::prelude::*;
use symtropy_bevy_scene::{SymtropyScenePlugin, fixed_camera};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SymtropyScenePlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(fixed_camera(Vec3::new(0.0, 1.5, 6.0), Vec3::ZERO));
}
```

## What it does

- `ClearColor` — dark cool indigo background
- `GlobalAmbientLight` — cool tint, brightness 200 cd/m² so shadowed sides
  aren't fully black
- One `DirectionalLight` ("sun") at a stage-lighting angle (upper-front-right),
  illuminance 8000 lux

Everything is overridable via `SymtropyScenePlugin::with_config(...)`.

## Helpers

- `fixed_camera(position: Vec3, target: Vec3)` — `Camera3d + Transform`
  one-liner.
- `fixed_light(illuminance, color, euler_xyz)` — secondary directional
  light spawn.

## Architectural review routes and annotations

The crate exposes validated routes for scripted review and a direct desktop floating camera, plus revision-pinned annotations that distinguish binding rules, evidence, guidance, site unknowns, preferences, and reform proposals. These APIs prepare a future pre-construction review experience; they do not make legal or engineering determinations.

- `ReviewRig::FloatingCamera` for a free inspection camera and `GuidedWalkthrough` for a human-height path.
- `AccessibleTeleport` for user-confirmed waypoint-to-waypoint movement.
- `HeadTrackedVrReference` for a future XR adapter to consume route reference poses; this enum does **not** mean an OpenXR/WebXR session or headset input path is implemented.
- `validate_review_route` checks stable waypoint IDs, finite poses, view direction, dwell/timeline constraints, and the route's explicit translation-speed ceiling.
- `sample_review_route` returns a deterministic Bevy `Transform` at a given elapsed time. Invalid routes return `None`, allowing the application to preserve a static camera.
- `ReviewRoutePlayback::new(route)` validates a route once and starts paused. `ReviewRoutePlayback::new_with_comfort_profile(route, profile)` binds explicit motion/consent preferences to playback; automatic start occurs only with `OptInAutoStart`. The profile is stored in the revisioned `DesignReviewSession` manifest (schema v2). Accessible-teleport routes do not auto-play and require user-selected waypoint poses.

Coordinates are metres in the model's declared frame, not georeferencing. The `DesignReviewSession` manifest pins the owner-brief revision, design/model artifact and digest, viewer build, snapshot manifest, comfort profile, review mode, annotations, and occupant feedback. A `ReviewJurisdictionContext` can additionally pin the jurisdiction capability/profile ID and the exact composed-rule snapshot artifact/digest. Session schema v3 also requires a `ReviewModelContext` that declares source format, source kind, fidelity level, source linear units, coordinate frame, and known limitations. `scale_point_to_review_metres` converts metre/millimetre/foot/inch values safely to review-space metres; it does not convert coordinate frames, axis orientation, handedness, or datum, which remain explicit importer responsibilities. The `fixtures/review-model-context-v1.schema.json` publishes that context contract for other clients, and `fixtures/synthetic-review-model-context-v1.json` is the explicit low-fidelity example parsed by a Rust regression test. `validate_design_review_session` requires the route (when used), annotations and feedback to reference the same design revision; static-plan review is supported without a route. Its digest checks validate metadata only, not actual bytes or compliance. The manifest has an explicit schema version and uses `review_session_manifest_bytes` to encode compact deterministic JSON with its own snapshot digest field blanked. A trusted evidence store should hash and archive those bytes; this crate does not hash or authenticate them, and the encoding contract does not claim RFC 8785 canonical JSON conformance. If any annotation is a binding requirement, binding conflict, or required evidence, the session must include a valid `ReviewJurisdictionContext`; a purely creative/static exploration session may omit it. This prevents legally framed overlays from appearing without an identified jurisdiction profile and a specific rules snapshot. The digest is metadata only: this viewer does not hash the source bytes, verify legal applicability, or authenticate the rule registry. The `ReviewAnnotation` types distinguish binding requirements, unresolved binding conflicts, required missing evidence, engineering evidence, non-binding guidance, unknown site facts, owner preferences, and code-reform proposals. An annotation must be pinned to the displayed design revision; binding findings require source/module provenance and result evidence; binding-conflict overlays must show at least two distinct source/clause pairs; owner preferences can be marked `UnfulfilledPreference` but cannot be styled as a binding failure, and guidance/reform proposals remain informational. The validator checks annotation semantics, requires an uncertainty note for unknown site facts/engineering results, and checks digest syntax only—not legal truth or evidence bytes.

### Occupant feedback from virtual review

`OccupantReviewFeedback` captures a person's reaction to the design revision they actually reviewed: preferences, spatial comfort, circulation, accessibility concerns, sightlines/privacy, storage/ergonomics, daylight/outdoor connection, acoustics questions, or material/maintenance preferences. It can be pinned to a route waypoint or a stable room/component reference.

This feedback is **not** automatically converted into a code violation or engineering fact. Old-revision feedback cannot be silently applied to a new revision, route/waypoint references are checked, accepted design changes need a traceable change-request ID, and public case-study sharing requires explicit consent. The data model does not collect headset, gaze, body-pose, camera, or microphone data; a host application should keep recording opt-in and provide private/default local review. The new `FeedbackResolutionRecord` closes the iteration loop: each resolution pins the original revision and current review revision; implemented changes need a change-request ID and evidence artifact, accessibility/circulation concerns also need an independent assessment reference, and owner withdrawal needs explicit confirmation. A preference may be left unchanged with a rationale without being reclassified as a violation.

The route sampler does not perform wall collision, visibility, accessibility-code, daylight, energy, or building-code checks. A production reviewer should pair it with geometry collision tests, a real model-revision ID, user-controlled speed/pause/comfort settings, a desktop fallback, and an independently implemented XR adapter. A pleasant walkthrough is design feedback, not proof that the proposed building is buildable.

### Try the synthetic-room example

From the workspace root, run:

```bash
cargo run -p symtropy-bevy-scene --example architectural_review_room
```

The example creates an illustrative room with a scripted route and a direct free-camera mode. It starts paused. Space starts/pauses, R restarts, F toggles free camera, right mouse looks, and WASD moves. The default profile limits the route to half its authored speed, uses a slower free-camera speed, disables the Shift boost and vertical flight, and records manual start rather than autoplay. Direct free-camera mode pauses the route and preserves the current view orientation. These are changeable user settings, not universal comfort limits. The scene remains synthetic and is not a real model, collision-checked navigation, XR implementation, engineering simulation, or compliance check.

## Owner-led design alternatives

The `design_intent` module distinguishes jurisdiction-pinned binding requirements, owner-declared hard constraints, preferences, exploration goals, engineering assumptions, and unverified site constraints. Each candidate needs an evaluation for every declared intent.

There is deliberately **no overall scalar score** that tries to declare one home best across privacy, cultural expression, daylight, affordability, maintenance, spatial character, and other unlike objectives. A candidate can be marked `ReadyForTechnicalAssessment` only when its hard constraints have known, satisfied or justified-not-applicable outcomes; this remains a path to independent assessment, not code compliance or approval. An owner can explicitly decline an alternative without misrepresenting that personal choice as a code violation. A `BlockedByHardConstraint` disposition needs an evidence-linked unsatisfied hard constraint, and binding requirements require exact rule-source/module provenance.

These types validate the comparison contract; they do not author building geometry, evaluate laws, prove source authenticity, or assess the truth of a claimed measurement.

### Pareto comparison without a single happiness score

The `IntentEvaluation.preference_rating` field is optional and only valid for `OwnerPreference` intents. It is an owner-provided 0–100 rating, not an objective measurement. `compute_owner_preference_frontier` compares alternatives independently on those dimensions:

- a candidate is preference-dominated only when another is at least as good on every comparable, owner-rated preference and strictly better on one;
- trade-offs such as "more daylight, less privacy" stay on the frontier instead of being crushed into one arbitrary weighted total;
- asymmetrically missing ratings make a pair incomparable; the system never guesses or imputes a rating;
- alternatives with known failed or unresolved binding/owner hard constraints are excluded from the preference frontier with an explicit reason, not silently traded against aesthetic preference;
- owner-declined and explicitly blocked alternatives remain recorded with their separate disposition.

This is an exploration aid: it does not prove a candidate is compliant, safe, affordable, or buildable. Hard constraints and independent professional/authority assessments remain separate.

### Try the Pareto example without a renderer

From the workspace root, run:

```bash
cargo run -p symtropy-bevy-scene --example owner_led_home_alternatives
```

The example compares four synthetic home concepts using made-up owner ratings for daylight and privacy. It retains genuine trade-offs, leaves a candidate with unknown preference ratings incomparable, and keeps the egress requirement outside the preference score. It does not generate a house, evaluate building codes, or claim that any candidate is buildable.

## License

Apache-2.0 OR MIT (permissive). No AGPL deps.

## Relationship to symtropy-bevy

`symtropy-bevy-scene` is the *visual* sibling of `symtropy-bevy-core`
(physics-only). They compose:

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(symtropy_bevy_scene::SymtropyScenePlugin::default())
    .add_plugins(symtropy_bevy::SymtropyPhysicsPlugin::<3>::with_gravity([0.0, -9.81, 0.0]))
    .run();
```
