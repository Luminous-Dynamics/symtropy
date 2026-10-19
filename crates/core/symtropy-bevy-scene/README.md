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
- `ReviewRoutePlayback::new(route)` validates a route once, then the scene plugin advances its Transform during Update; playback can be paused/resumed. Accessible-teleport routes do not auto-play and require user-selected waypoint poses.

Coordinates are metres in the model's declared frame, not georeferencing. The `ReviewAnnotation` types distinguish binding requirements, unresolved binding conflicts, required missing evidence, engineering evidence, non-binding guidance, unknown site facts, owner preferences, and code-reform proposals. An annotation must be pinned to the displayed design revision; binding findings require source/module provenance, owner preferences can be marked `UnfulfilledPreference` but cannot be styled as a binding failure, and guidance/reform proposals remain informational. The validator checks annotation semantics and digest syntax only—not legal truth or evidence bytes.

### Occupant feedback from virtual review

`OccupantReviewFeedback` captures a person's reaction to the design revision they actually reviewed: preferences, spatial comfort, circulation, accessibility concerns, sightlines/privacy, storage/ergonomics, daylight/outdoor connection, acoustics questions, or material/maintenance preferences. It can be pinned to a route waypoint or a stable room/component reference.

This feedback is **not** automatically converted into a code violation or engineering fact. Old-revision feedback cannot be silently applied to a new revision, route/waypoint references are checked, accepted design changes need a traceable change-request ID, and public case-study sharing requires explicit consent. The data model does not collect headset, gaze, body-pose, camera, or microphone data; a host application should keep recording opt-in and provide private/default local review.

The route sampler does not perform wall collision, visibility, accessibility-code, daylight, energy, or building-code checks. A production reviewer should pair it with geometry collision tests, a real model-revision ID, user-controlled speed/pause/comfort settings, a desktop fallback, and an independently implemented XR adapter. A pleasant walkthrough is design feedback, not proof that the proposed building is buildable.

### Try the synthetic-room example

From the workspace root, run:

```bash
cargo run -p symtropy-bevy-scene --example architectural_review_room
```

The example creates an illustrative room with a scripted route and a direct free-camera mode: Space pauses/resumes the route, R restarts it, F toggles free camera, hold right mouse to look, WASD to move horizontally, E/Q to rise/descend, and Shift to move faster. The direct camera pauses the route and preserves the currently displayed view orientation when enabled. Route controls are explicitly ordered before route advancement and free-camera movement. This is a visual prototype with synthetic geometry, not a real home model, collision-checked navigation, XR implementation, engineering simulation, or compliance check.

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
