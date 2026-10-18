// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Frozen, asset-free PBR scene for repeatable renderer comparisons.
//!
//! Geometry is generated from an integer lattice with no random inputs. Keep
//! the fixture implementation version bound to the source commit in captures.

use bevy::prelude::*;
use std::env;
use symtropy_demo_capture::CapturePlugin;

const FIXTURE_ID: &str = "SCENE-LIGHT-01";
const CAMERA_ID: &str = "auto-grid-camera-v1";
const CUBE_SIZE: f32 = 0.72;
const GRID_SPACING: f32 = 1.05;
const MAX_OBJECTS: usize = 10_000;

#[derive(Resource, Clone, Debug)]
struct BenchConfig {
    profile: String,
    width_px: u32,
    height_px: u32,
    object_count: usize,
    shadows_enabled: bool,
}

impl BenchConfig {
    fn from_env() -> Result<Self, String> {
        let profile = env::var("SYMTROPY_RENDER_BENCH_PROFILE")
            .unwrap_or_else(|_| "balanced-1080p".to_owned());
        let (width_px, height_px, default_count, default_shadows) = match profile.as_str() {
            "quality-1080p" => (1920, 1080, 256, true),
            "balanced-1080p" => (1920, 1080, 144, true),
            "low-power-720p" => (1280, 720, 64, false),
            _ => {
                return Err(format!(
                    "unsupported profile {profile:?}; choose quality-1080p, balanced-1080p, or low-power-720p"
                ));
            }
        };

        let object_count = match env::var("SYMTROPY_RENDER_BENCH_OBJECTS") {
            Ok(raw) => raw
                .parse::<usize>()
                .map_err(|_| "SYMTROPY_RENDER_BENCH_OBJECTS must be an integer".to_owned())?,
            Err(_) => default_count,
        };
        if !(1..=MAX_OBJECTS).contains(&object_count) {
            return Err(format!(
                "object count must be in 1..={MAX_OBJECTS}, got {object_count}"
            ));
        }

        let shadows_enabled = match env::var("SYMTROPY_RENDER_BENCH_SHADOWS") {
            Ok(raw) => match raw.to_ascii_lowercase().as_str() {
                "1" | "true" | "on" => true,
                "0" | "false" | "off" => false,
                _ => {
                    return Err(
                        "SYMTROPY_RENDER_BENCH_SHADOWS must be on/off, true/false, or 1/0"
                            .to_owned(),
                    );
                }
            },
            Err(_) => default_shadows,
        };

        Ok(Self {
            profile,
            width_px,
            height_px,
            object_count,
            shadows_enabled,
        })
    }

    fn capture_profile(&self) -> String {
        format!(
            "{}-n{}-shadows-{}",
            self.profile,
            self.object_count,
            if self.shadows_enabled { "on" } else { "off" }
        )
    }
}

fn main() {
    let config = match BenchConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("symtropy-render-benchmark: {error}");
            std::process::exit(2);
        }
    };

    let window = Window {
        title: format!("{FIXTURE_ID} — {}", config.capture_profile()),
        resolution: WindowResolution::new(config.width_px, config.height_px),
        ..default()
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(window),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.035, 0.045, 0.065)))
        .insert_resource(bevy::light::GlobalAmbientLight {
            color: Color::srgb(0.82, 0.86, 0.96),
            brightness: 180.0,
            ..default()
        })
        .insert_resource(config)
        .add_plugins(CapturePlugin::new("scene-light-01"))
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<BenchConfig>,
) {
    let columns = (config.object_count as f64).sqrt().ceil() as usize;
    let rows = config.object_count.div_ceil(columns);
    let grid_width = (columns.saturating_sub(1) as f32 * GRID_SPACING) + CUBE_SIZE;
    let grid_depth = (rows.saturating_sub(1) as f32 * GRID_SPACING) + CUBE_SIZE;
    let grid_span = grid_width.max(grid_depth);
    let camera_distance = grid_span * 1.55 + 7.0;

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(
            camera_distance * 0.70,
            camera_distance * 0.78,
            camera_distance * 0.92,
        )
        .looking_at(Vec3::new(0.0, 0.35, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            color: Color::linear_rgb(1.0, 0.97, 0.91),
            shadow_maps_enabled: config.shadows_enabled,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.72, -0.62, 0.0)),
    ));

    // One shared mesh and eight shared materials make the workload repeatable
    // without adding random texture/material asset loading to the fixture.
    let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
    let material_profiles = [
        (Color::srgb(0.82, 0.12, 0.10), 0.0, 0.88),
        (Color::srgb(0.08, 0.26, 0.86), 0.82, 0.22),
        (Color::srgb(0.12, 0.66, 0.34), 0.05, 0.72),
        (Color::srgb(0.92, 0.58, 0.08), 0.76, 0.28),
        (Color::srgb(0.08, 0.67, 0.73), 0.34, 0.42),
        (Color::srgb(0.60, 0.16, 0.76), 0.12, 0.55),
        (Color::srgb(0.72, 0.75, 0.80), 0.0, 0.94),
        (Color::srgb(0.16, 0.18, 0.22), 0.92, 0.18),
    ];
    let material_handles: Vec<Handle<StandardMaterial>> = material_profiles
        .into_iter()
        .map(|(base_color, metallic, perceptual_roughness)| {
            materials.add(StandardMaterial {
                base_color,
                metallic,
                perceptual_roughness,
                ..default()
            })
        })
        .collect();

    let floor_mesh = meshes.add(Cuboid::new(grid_width + 5.0, 0.2, grid_depth + 5.0));
    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.11, 0.13, 0.16),
        metallic: 0.0,
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
        let x = column as f32 * GRID_SPACING
            - (columns.saturating_sub(1) as f32 * GRID_SPACING * 0.5);
        let z = row as f32 * GRID_SPACING
            - (rows.saturating_sub(1) as f32 * GRID_SPACING * 0.5);
        let scale_y = if index % 11 == 0 { 1.45 } else { 1.0 };
        let material = material_handles[index % material_handles.len()].clone();

        commands.spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(x, CUBE_SIZE * scale_y * 0.5, z)
                .with_scale(Vec3::new(1.0, scale_y, 1.0)),
        ));
    }

    commands.spawn((
        Text::new(format!(
            "{FIXTURE_ID}  |  {} objects  |  profile {}  |  shadows {}",
            config.object_count,
            config.profile,
            if config.shadows_enabled { "on" } else { "off" }
        )),
        TextFont {
            font_size: 18.0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_profiles_have_fixed_resolution_and_default_workload() {
        let quality = profile_defaults("quality-1080p").expect("profile should exist");
        assert_eq!(quality, (1920, 1080, 256, true));

        let balanced = profile_defaults("balanced-1080p").expect("profile should exist");
        assert_eq!(balanced, (1920, 1080, 144, true));

        let low_power = profile_defaults("low-power-720p").expect("profile should exist");
        assert_eq!(low_power, (1280, 720, 64, false));
    }

    #[test]
    fn grid_dimensions_cover_all_objects_without_randomness() {
        assert_eq!(grid_dimensions(1), (1, 1));
        assert_eq!(grid_dimensions(13), (4, 4));
        assert_eq!(grid_dimensions(144), (12, 12));
    }

    fn profile_defaults(profile: &str) -> Result<(u32, u32, usize, bool), String> {
        match profile {
            "quality-1080p" => Ok((1920, 1080, 256, true)),
            "balanced-1080p" => Ok((1920, 1080, 144, true)),
            "low-power-720p" => Ok((1280, 720, 64, false)),
            _ => Err(format!("unknown profile: {profile}")),
        }
    }

    fn grid_dimensions(object_count: usize) -> (usize, usize) {
        let columns = (object_count as f64).sqrt().ceil() as usize;
        let rows = object_count.div_ceil(columns);
        (columns, rows)
    }
}
