// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Synthetic room to try the floating-camera route before a real house model is
//! connected. Units are illustrative metres, not a measured or permitted plan.
//!
//! Run from the workspace root:
//!   cargo run -p symtropy-bevy-scene --example architectural_review_room
//!
//! Route starts paused; press Space to start/pause and R to restart. This demo has no collision adapter,
//! IFC loader, engineering simulation, or VR runtime.

use bevy::prelude::*;
use symtropy_bevy_scene::{
    ArchitecturalReviewRoute, ReviewComfortProfile, ReviewFreeCameraController, ReviewRig,
    ReviewRoutePlayback, ReviewWaypoint, SymtropyScenePlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SymtropyScenePlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn spawn_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    size: Vec3,
    position: Vec3,
    color: Color,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_translation(position),
    ));
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // A simple 6 m × 8 m room with a deliberately visible entry opening.
    spawn_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(6.0, 0.1, 8.0),
        Vec3::new(0.0, -0.05, 0.0),
        Color::srgb(0.72, 0.70, 0.65),
    );
    let wall = Color::srgb(0.82, 0.81, 0.76);
    spawn_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(0.15, 3.0, 8.0),
        Vec3::new(-3.0, 1.5, 0.0),
        wall,
    );
    spawn_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(0.15, 3.0, 8.0),
        Vec3::new(3.0, 1.5, 0.0),
        wall,
    );
    spawn_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(6.0, 3.0, 0.15),
        Vec3::new(0.0, 1.5, -4.0),
        wall,
    );
    // Front wall segments create a 1.6 m opening at the centre.
    for x in [-2.05, 2.05] {
        spawn_box(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(1.9, 3.0, 0.15),
            Vec3::new(x, 1.5, 4.0),
            wall,
        );
    }
    spawn_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.2, 0.35, 0.15),
        Vec3::new(0.0, 2.825, 4.0),
        wall,
    );

    // Counter and seating are visual scale cues, not manufacturable assets.
    spawn_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.0, 0.9, 0.7),
        Vec3::new(1.75, 0.45, -0.3),
        Color::srgb(0.36, 0.24, 0.16),
    );
    spawn_box(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(1.5, 0.45, 0.8),
        Vec3::new(-1.6, 0.225, -2.4),
        Color::srgb(0.18, 0.42, 0.46),
    );

    let route = ArchitecturalReviewRoute {
        route_id: "synthetic-room:floating-review-v1".into(),
        design_revision_id: "synthetic-room:model-v1".into(),
        rig: ReviewRig::FloatingCamera,
        max_translation_speed_mps: 1.8,
        waypoints: vec![
            waypoint(
                "entry",
                "Entry overview",
                [0.0, 1.6, 3.0],
                [0.0, 1.4, 0.0],
                0.0,
                2.0,
            ),
            waypoint(
                "kitchen",
                "Kitchen work zone",
                [0.0, 1.6, 0.0],
                [2.0, 1.1, -0.3],
                4.0,
                1.5,
            ),
            waypoint(
                "living",
                "Living area",
                [-1.1, 1.6, -1.0],
                [-1.4, 1.2, -2.4],
                7.5,
                2.0,
            ),
        ],
    };

    let camera_pose =
        Transform::from_xyz(0.0, 1.6, 3.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y);
    let comfort_profile = ReviewComfortProfile::default();
    let playback = ReviewRoutePlayback::new_with_comfort_profile(route, comfort_profile.clone())
        .expect("the example route and comfort profile must validate");
    let mut free_camera = ReviewFreeCameraController::from_transform(&camera_pose);
    free_camera
        .apply_comfort_profile(&comfort_profile)
        .expect("the example comfort profile must validate");
    commands.spawn((Camera3d::default(), camera_pose, playback, free_camera));

    info!("Synthetic architectural review scene; not a real plan or compliance model.");
    info!("Route starts paused. Space: start/pause. R: restart. F: toggle free camera.");
    info!(
        "Free camera: hold right mouse to look, WASD to move, E/Q to rise/descend, Shift to move faster."
    );
}

fn waypoint(
    waypoint_id: &str,
    label: &str,
    position_m: [f32; 3],
    look_at_m: [f32; 3],
    arrival_seconds: f32,
    dwell_seconds: f32,
) -> ReviewWaypoint {
    ReviewWaypoint {
        waypoint_id: waypoint_id.into(),
        label: label.into(),
        position_m,
        look_at_m,
        arrival_seconds,
        dwell_seconds,
    }
}
