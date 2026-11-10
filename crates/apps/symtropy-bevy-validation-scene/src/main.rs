// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable smoke scene for the permissive Symtropy Bevy distribution.
//!
//! Default mode is Bevy's conventional raster renderer. Build with
//! `--features solari-preview` to render the same procedural material scene
//! using Bevy 0.20's experimental realtime ray-traced lighting.

use bevy::prelude::*;
use symtropy_bevy_scene::{SymtropySceneConfig, SymtropyScenePlugin};
use symtropy_core::prelude::BevyPhysicsPlugin;

#[cfg(feature = "solari-preview")]
use bevy::{
    camera::CameraMainTextureUsages,
    render::render_resource::TextureUsages,
    solari::prelude::{RaytracingMesh3d, SolariLighting, SolariPlugins},
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    #[cfg(feature = "solari-preview")]
    {
        // Solari replaces conventional shadow maps; leave them disabled so
        // the comparison isolates the experimental ray-traced lighting path.
        app.add_plugins(SymtropyScenePlugin::with_config(SymtropySceneConfig {
            sun_shadow_maps_enabled: false,
            ..default()
        }));
        app.add_plugins(SolariPlugins);
    }

    #[cfg(not(feature = "solari-preview"))]
    app.add_plugins(SymtropyScenePlugin::default());

    app.add_plugins(BevyPhysicsPlugin::<3>::with_gravity([0.0, -9.81, 0.0]))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = meshes.add(
        Cuboid::default()
            .mesh()
            .build()
            .with_generated_tangents()
            .expect("cube tangents should generate"),
    );
    let floor = meshes.add(
        Plane3d::default()
            .mesh()
            .size(12.0, 12.0)
            .build()
            .with_generated_tangents()
            .expect("floor tangents should generate"),
    );

    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.21, 0.24),
        perceptual_roughness: 0.92,
        ..default()
    });
    let floor_bundle = (
        Mesh3d(floor.clone()),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, -0.05, 0.0),
    );
    #[cfg(feature = "solari-preview")]
    commands.spawn((floor_bundle, RaytracingMesh3d(floor.clone())));
    #[cfg(not(feature = "solari-preview"))]
    commands.spawn(floor_bundle);

    let material_profiles = [
        StandardMaterial {
            base_color: Color::srgb(0.82, 0.12, 0.10),
            metallic: 0.02,
            perceptual_roughness: 0.82,
            ..default()
        },
        StandardMaterial {
            base_color: Color::srgb(0.08, 0.28, 0.84),
            metallic: 0.88,
            perceptual_roughness: 0.18,
            ..default()
        },
        StandardMaterial {
            base_color: Color::srgb(0.12, 0.65, 0.32),
            metallic: 0.35,
            perceptual_roughness: 0.42,
            ..default()
        },
        StandardMaterial {
            base_color: Color::srgb(0.92, 0.58, 0.08),
            metallic: 0.04,
            perceptual_roughness: 0.28,
            ..default()
        },
    ];

    for (index, material) in material_profiles.into_iter().enumerate() {
        let x = index as f32 * 1.8 - 2.7;
        let handle = materials.add(material);
        let transform = Transform::from_xyz(x, 0.55, 0.0)
            .with_scale(Vec3::splat(1.1))
            .with_rotation(Quat::from_rotation_y(index as f32 * 0.18));
        let bundle = (
            Mesh3d(cube.clone()),
            MeshMaterial3d(handle),
            transform,
        );
        #[cfg(feature = "solari-preview")]
        commands.spawn((bundle, RaytracingMesh3d(cube.clone())));
        #[cfg(not(feature = "solari-preview"))]
        commands.spawn(bundle);
    }

    #[cfg(feature = "solari-preview")]
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
        Msaa::Off,
        SolariLighting::default(),
        Transform::from_xyz(7.0, 5.5, 9.0).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
    ));

    #[cfg(not(feature = "solari-preview"))]
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(7.0, 5.5, 9.0).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
    ));
}
