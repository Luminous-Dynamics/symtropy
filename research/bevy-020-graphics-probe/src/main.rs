// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Isolated Bevy 0.20 renderer/API probe.
//!
//! This package intentionally does not depend on Symtropy 0.19 crates. It is
//! a migration test surface while the production workspace remains on one
//! Bevy version.

use bevy::{
    camera_controller::pan_orbit_camera::{DefaultPanOrbitCameraPlugins, prelude::PanOrbitCamera},
    prelude::*,
};
use std::env;

const MAX_OBJECTS: usize = 10_000;
const GRID_SPACING: f32 = 1.1;

#[derive(Resource, Clone, Debug)]
struct ProbeConfig {
    object_count: usize,
    shadow_maps_enabled: bool,
}

impl ProbeConfig {
    fn from_env() -> Result<Self, String> {
        let object_count = env::var("SYMTROPY_BEVY020_PROBE_OBJECTS")
            .ok()
            .map(|value| {
                value
                    .parse::<usize>()
                    .map_err(|_| "SYMTROPY_BEVY020_PROBE_OBJECTS must be an integer".to_owned())
            })
            .transpose()?
            .unwrap_or(64);
        if !(1..=MAX_OBJECTS).contains(&object_count) {
            return Err(format!("object count must be in 1..={MAX_OBJECTS}"));
        }

        let shadow_maps_enabled = match env::var("SYMTROPY_BEVY020_PROBE_SHADOWS") {
            Ok(value) => match value.to_ascii_lowercase().as_str() {
                "1" | "true" | "on" => true,
                "0" | "false" | "off" => false,
                _ => return Err("SYMTROPY_BEVY020_PROBE_SHADOWS must be on/off".to_owned()),
            },
            Err(_) => true,
        };
        Ok(Self {
            object_count,
            shadow_maps_enabled,
        })
    }
}

fn main() {
    let config = match ProbeConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("symtropy-bevy-020-graphics-probe: {error}");
            std::process::exit(2);
        }
    };

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultPanOrbitCameraPlugins)
        .insert_resource(ClearColor(Color::srgb(0.035, 0.045, 0.065)))
        .insert_resource(bevy::light::GlobalAmbientLight {
            color: Color::srgb(0.82, 0.86, 0.96),
            brightness: 180.0,
            ..default()
        })
        .insert_resource(config)
        .add_systems(Startup, setup_probe)
        .run();
}

fn setup_probe(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<ProbeConfig>,
) {
    let (columns, rows) = grid_dimensions(config.object_count);
    let grid_width = (columns.saturating_sub(1) as f32 * GRID_SPACING) + 0.8;
    let grid_depth = (rows.saturating_sub(1) as f32 * GRID_SPACING) + 0.8;
    let view_distance = grid_width.max(grid_depth) * 1.6 + 6.0;

    commands.spawn((
        Camera3d::default(),
        PanOrbitCamera::default(),
        Transform::from_xyz(view_distance * 0.72, view_distance * 0.84, view_distance)
            .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            color: Color::linear_rgb(1.0, 0.97, 0.91),
            shadow_maps_enabled: config.shadow_maps_enabled,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.72, -0.62, 0.0)),
    ));

    let cube_mesh = meshes.add(Cuboid::new(0.8, 0.8, 0.8));
    let palette = [
        Color::srgb(0.82, 0.12, 0.10),
        Color::srgb(0.08, 0.26, 0.86),
        Color::srgb(0.12, 0.66, 0.34),
        Color::srgb(0.92, 0.58, 0.08),
        Color::srgb(0.08, 0.67, 0.73),
        Color::srgb(0.60, 0.16, 0.76),
        Color::srgb(0.72, 0.75, 0.80),
        Color::srgb(0.16, 0.18, 0.22),
    ];
    let material_handles: Vec<Handle<StandardMaterial>> = palette
        .into_iter()
        .enumerate()
        .map(|(index, base_color)| {
            materials.add(StandardMaterial {
                base_color,
                metallic: if index % 2 == 0 { 0.05 } else { 0.82 },
                perceptual_roughness: if index % 2 == 0 { 0.78 } else { 0.24 },
                ..default()
            })
        })
        .collect();

    let floor_mesh = meshes.add(Cuboid::new(grid_width + 5.0, 0.2, grid_depth + 5.0));
    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.11, 0.13, 0.16),
        perceptual_roughness: 0.94,
        ..default()
    });
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, -0.12, 0.0),
    ));

    for index in 0..config.object_count {
        let column = index % columns;
        let row = index / columns;
        let x =
            column as f32 * GRID_SPACING - (columns.saturating_sub(1) as f32 * GRID_SPACING * 0.5);
        let z = row as f32 * GRID_SPACING - (rows.saturating_sub(1) as f32 * GRID_SPACING * 0.5);
        let material = material_handles[index % material_handles.len()].clone();
        let scale_y = if index % 11 == 0 { 1.45 } else { 1.0 };

        commands.spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0.4 * scale_y, z).with_scale(Vec3::new(1.0, scale_y, 1.0)),
        ));
    }

    commands.spawn((
        Text::new(format!(
            "BEVY-020-PROBE | {} objects | shadows {} | drag to orbit / pan / zoom",
            config.object_count,
            if config.shadow_maps_enabled {
                "on"
            } else {
                "off"
            }
        )),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(14.0),
            ..default()
        },
    ));
}

fn grid_dimensions(object_count: usize) -> (usize, usize) {
    let columns = (object_count as f64).sqrt().ceil() as usize;
    let rows = object_count.div_ceil(columns);
    (columns, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_framing_is_defined_for_reference_counts() {
        assert_eq!(grid_dimensions(1), (1, 1));
        assert_eq!(grid_dimensions(13), (4, 4));
        assert_eq!(grid_dimensions(64), (8, 8));
        assert_eq!(grid_dimensions(256), (16, 16));
        assert_eq!(grid_dimensions(MAX_OBJECTS), (100, 100));
    }

    #[test]
    fn object_limit_is_explicit() {
        assert!((1..=MAX_OBJECTS).contains(&1));
        assert!((1..=MAX_OBJECTS).contains(&MAX_OBJECTS));
        assert!(!(1..=MAX_OBJECTS).contains(&(MAX_OBJECTS + 1)));
    }
}
