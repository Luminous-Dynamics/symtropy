// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Render one skinned human from a user-supplied glTF/GLB and switch its clips.
//!
//! Put a rigged character file under `assets/humans/person.glb`, or set
//! `SYMTROPY_HUMAN_AVATAR` to a different asset-relative path under `assets/`.
//!
//! Run: `cargo run -p symtropy-bevy --features human-animation --example human_avatar`
//!
//! Keys: 1 idle, 2 walk, 3 run, 4 talk/gesture when available.
//! Missing categories fall back to a documented available clip and are reported in logs.
//! This is an asset/animation smoke test, not a human-motion realism benchmark.

use bevy::{
    animation::{AnimationGraph, AnimationPlayer},
    light::CascadeShadowConfigBuilder,
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use symtropy_bevy::{
    HumanAnimationClips, HumanAnimationController, HumanAnimationPlugin, HumanLocomotionIntent,
    HumanMotion,
};

#[derive(Resource)]
struct HumanAsset(Handle<Gltf>);

#[derive(Component, Clone)]
struct PendingHumanAnimation {
    graph: Handle<AnimationGraph>,
    clips: HumanAnimationClips,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(HumanAnimationPlugin)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, load_human_when_ready)
        .add_systems(Update, keyboard_motion_control)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let path =
        std::env::var("SYMTROPY_HUMAN_AVATAR").unwrap_or_else(|_| "humans/person.glb".to_owned());
    info!(
        "Human avatar asset: assets/{path}. Put a rigged GLB there or set SYMTROPY_HUMAN_AVATAR."
    );
    commands.insert_resource(HumanAsset(asset_server.load(path)));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(2.8, 1.8, 4.2).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.28, 0.32, 0.29))),
    ));

    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.4, 0.0)),
        DirectionalLight {
            illuminance: 9_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 20.0,
            maximum_distance: 60.0,
            ..default()
        }
        .build(),
    ));

    commands.spawn((
        Text::new(concat!(
            "HUMAN AVATAR SMOKE TEST\n",
            "1: Idle   2: Walk   3: Run   4: Talk/Gesture\n",
            "Set SYMTROPY_HUMAN_AVATAR to an asset-relative GLB path",
        )),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn load_human_when_ready(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset: Res<HumanAsset>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut attempted: Local<bool>,
) {
    if *attempted {
        return;
    }
    if let Some(LoadState::Failed(err)) = asset_server.get_load_state(&asset.0) {
        error!("Failed to load human avatar asset: {err}");
        *attempted = true;
        return;
    }
    if !asset_server.is_loaded_with_dependencies(&asset.0) {
        return;
    }
    *attempted = true;

    let Some(gltf) = gltfs.get(&asset.0) else {
        error!("Human GLB finished loading but no Gltf asset was registered.");
        return;
    };

    if gltf.named_animations.is_empty() {
        error!(concat!(
            "Human GLB contains no named animation clips. ",
            "Use a rigged animated GLB for this smoke test."
        ));
        return;
    }

    let Some(default_scene) = gltf.default_scene.clone() else {
        error!("Human GLB has no default scene; cannot spawn its character.");
        return;
    };

    let mut named: Vec<(String, Handle<AnimationClip>)> = gltf
        .named_animations
        .iter()
        .map(|(name, handle)| (name.to_string(), handle.clone()))
        .collect();
    named.sort_by(|a, b| a.0.cmp(&b.0));

    let names: Vec<String> = named.iter().map(|(name, _)| name.clone()).collect();
    let clip_handles: Vec<Handle<AnimationClip>> =
        named.iter().map(|(_, handle)| handle.clone()).collect();
    let (graph, node_indices) = AnimationGraph::from_clips(clip_handles);
    let graph_handle = graphs.add(graph);

    let Some(idle) = find_clip_index(&names, &["idle", "stand", "survey", "rest"]) else {
        error!("No idle/standing clip found. Name one clip idle, stand, survey, or rest.");
        return;
    };
    let Some(walk) = find_clip_index(&names, &["relaxedwalk", "walkcycle", "walk"]) else {
        error!("No walk clip found. Name one clip relaxedwalk, walkcycle, or walk.");
        return;
    };
    let Some(run) = find_clip_index(&names, &["run", "jog", "sprint"]) else {
        error!("No run/jog/sprint clip found. Name one clip run, jog, or sprint.");
        return;
    };
    let talk = find_clip_index(&names, &["talk", "gesture", "conversation"]);
    if talk.is_none() {
        warn!("No talk/gesture clip found; talk requests will display the idle clip.");
    }

    info!("Human GLB animation clips: {names:?}");
    info!(
        "Mapped idle={}, walk={}, run={}, talk={:?}",
        names[idle],
        names[walk],
        names[run],
        talk.map(|i| names[i].as_str())
    );

    let clips = HumanAnimationClips {
        idle: node_indices[idle],
        walk: node_indices[walk],
        run: node_indices[run],
        talk: talk.map(|index| node_indices[index]),
    };

    commands
        .spawn((
            WorldAssetRoot(default_scene),
            PendingHumanAnimation {
                graph: graph_handle,
                clips,
            },
        ))
        .observe(bind_controller_to_rig);
}

fn normalize_clip_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn find_clip_index(names: &[String], aliases: &[&str]) -> Option<usize> {
    for alias in aliases {
        if let Some(index) = names
            .iter()
            .position(|name| normalize_clip_name(name).contains(alias))
        {
            return Some(index);
        }
    }
    None
}

fn bind_controller_to_rig(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    bindings: Query<&PendingHumanAnimation>,
    children: Query<&Children>,
    players: Query<Entity, With<AnimationPlayer>>,
) {
    let Ok(bindings) = bindings.get(ready.entity) else {
        warn!("Loaded human scene has no pending animation binding.");
        return;
    };

    let targets = std::iter::once(ready.entity).chain(children.iter_descendants(ready.entity));
    let mut bound = 0usize;
    for entity in targets {
        if players.get(entity).is_ok() {
            commands.entity(entity).insert((
                HumanAnimationController::new(
                    bindings.graph.clone(),
                    bindings.clips.clone(),
                    HumanMotion::Idle,
                ),
                HumanLocomotionIntent::default(),
            ));
            bound += 1;
        }
    }

    if bound == 0 {
        warn!("Human scene loaded, but no AnimationPlayer was found in its hierarchy.");
    } else {
        info!("Bound human animation controller to {bound} animation player(s).");
    }
}

fn keyboard_motion_control(
    keys: Res<ButtonInput<KeyCode>>,
    mut controllers: Query<(&mut HumanAnimationController, &mut HumanLocomotionIntent)>,
) {
    let requested = if keys.just_pressed(KeyCode::Digit1) {
        Some((HumanMotion::Idle, 0.0))
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some((HumanMotion::Walk, 0.8))
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some((HumanMotion::Run, 2.5))
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some((HumanMotion::Talk, 0.0))
    } else {
        None
    };

    if let Some((motion, speed_mps)) = requested {
        for (mut controller, mut intent) in &mut controllers {
            controller.motion = motion;
            controller.transition_seconds = 0.2;
            intent.desired_planar_velocity_mps = Vec2::new(0.0, speed_mps);
            intent.observed_planar_velocity_mps = None;
            intent.valid_until_sec = None;
            intent.motion_override = match motion {
                HumanMotion::Idle | HumanMotion::Talk => Some(motion),
                HumanMotion::Walk | HumanMotion::Run => None,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_name_normalization_ignores_spaces_and_punctuation() {
        assert_eq!(normalize_clip_name("Walk-Cycle 01"), "walkcycle01");
    }

    #[test]
    fn clip_name_lookup_uses_aliases_and_is_case_insensitive() {
        let names = vec![
            "Idle_Standing".to_owned(),
            "Fast Walk".to_owned(),
            "Jog".to_owned(),
        ];
        assert_eq!(find_clip_index(&names, &["idle"]), Some(0));
        assert_eq!(find_clip_index(&names, &["walk"]), Some(1));
        assert_eq!(find_clip_index(&names, &["run", "jog"]), Some(2));
        assert_eq!(find_clip_index(&names, &["gesture"]), None);
    }
}
