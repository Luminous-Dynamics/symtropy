// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ROOM-TWIN-01: deterministic indoor topology-identifiability fixture.
//!
//! The first two observations use a frozen corridor camera pose. A later
//! camera-rail segment traverses two aligned doorways and faces the far wall.
//! The only world-variant geometry is a panel at that far doorway. Shadows
//! are disabled so the hidden panel cannot alter the shared-prefix images
//! through shadow casting.

use bevy::prelude::*;
use std::env;
use symtropy_demo_capture::CapturePlugin;

const FIXTURE_ID: &str = "ROOM-TWIN-01";
const CAMERA_ID: &str = "room-twin-camera-v1";
const CAPTURE_PREFIX: f32 = 4.0;
const REVEAL_CAPTURE: f32 = 7.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorldVariant {
    AOpen,
    BClosed,
}

impl WorldVariant {
    fn from_env() -> Result<Self, String> {
        let value = env::var("SYMTROPY_ROOM_TWIN_VARIANT")
            .map_err(|_| "SYMTROPY_ROOM_TWIN_VARIANT must be set to A or B".to_owned())?;
        Self::from_value(&value).ok_or_else(|| {
            format!("SYMTROPY_ROOM_TWIN_VARIANT must be A or B, got {value:?}")
        })
    }

    fn has_far_door_panel(self) -> bool {
        matches!(self, Self::BClosed)
    }
}

#[derive(Resource, Clone, Copy)]
struct TwinWorld {
    variant: WorldVariant,
}

#[derive(Component)]
struct TwinCamera;

#[derive(Clone)]
struct CameraWaypoint {
    at_secs: f32,
    transform: Transform,
}

#[derive(Resource)]
struct CameraRail {
    waypoints: Vec<CameraWaypoint>,
}

fn camera_rail() -> CameraRail {
    let corridor_view =
        Transform::from_xyz(0.0, 1.65, -7.0).looking_at(Vec3::new(0.0, 1.65, 8.0), Vec3::Y);
    let corridor_exit =
        Transform::from_xyz(0.0, 1.65, 2.0).looking_at(Vec3::new(0.0, 1.65, 8.0), Vec3::Y);
    let inner_room =
        Transform::from_xyz(5.2, 1.65, 2.0).looking_at(Vec3::new(5.2, 1.65, 8.0), Vec3::Y);
    let reveal =
        Transform::from_xyz(5.2, 1.65, 2.0).looking_at(Vec3::new(13.0, 1.65, 2.0), Vec3::Y);

    CameraRail {
        waypoints: vec![
            CameraWaypoint {
                at_secs: 0.0,
                transform: corridor_view.clone(),
            },
            CameraWaypoint {
                at_secs: CAPTURE_PREFIX + 0.5,
                transform: corridor_view,
            },
            CameraWaypoint {
                at_secs: 5.7,
                transform: corridor_exit,
            },
            CameraWaypoint {
                at_secs: 6.4,
                transform: inner_room,
            },
            CameraWaypoint {
                at_secs: REVEAL_CAPTURE - 0.2,
                transform: reveal,
            },
        ],
    }
}

fn camera_pose_at(rail: &CameraRail, elapsed_secs: f32) -> Transform {
    let Some(first) = rail.waypoints.first() else {
        return Transform::IDENTITY;
    };
    if elapsed_secs <= first.at_secs {
        return first.transform.clone();
    }

    for pair in rail.waypoints.windows(2) {
        let start = &pair[0];
        let end = &pair[1];
        if elapsed_secs <= end.at_secs {
            let duration = (end.at_secs - start.at_secs).max(f32::EPSILON);
            let alpha = ((elapsed_secs - start.at_secs) / duration).clamp(0.0, 1.0);
            return Transform {
                translation: start
                    .transform
                    .translation
                    .lerp(end.transform.translation, alpha),
                rotation: start
                    .transform
                    .rotation
                    .slerp(end.transform.rotation, alpha),
                scale: Vec3::ONE,
            };
        }
    }

    rail.waypoints
        .last()
        .expect("non-empty rail")
        .transform
        .clone()
}

fn main() {
    let variant = match WorldVariant::from_env() {
        Ok(variant) => variant,
        Err(error) => {
            eprintln!("{FIXTURE_ID}: {error}");
            std::process::exit(2);
        }
    };

    let (width, height) = match env::var("SYMTROPY_ROOM_TWIN_PROFILE").as_deref() {
        Ok("low-power-720p") | Err(_) => (1280, 720),
        Ok("balanced-1080p") => (1920, 1080),
        Ok(other) => {
            eprintln!("{FIXTURE_ID}: unsupported profile {other:?}");
            std::process::exit(2);
        }
    };

    let window = Window {
        title: FIXTURE_ID.to_owned(),
        resolution: (width, height).into(),
        ..default()
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(window),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.050)))
        .insert_resource(bevy::light::GlobalAmbientLight {
            color: Color::srgb(0.86, 0.89, 0.96),
            brightness: 240.0,
            ..default()
        })
        .insert_resource(TwinWorld { variant })
        .insert_resource(camera_rail())
        .add_plugins(CapturePlugin::new("room-twin"))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, move_camera_on_rail)
        .run();
}

fn spawn_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    center: Vec3,
    size: Vec3,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(size))),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(center),
    ));
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<TwinWorld>,
) {
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.63, 0.68, 0.73),
        metallic: 0.0,
        perceptual_roughness: 0.96,
        ..default()
    });
    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.25, 0.28),
        metallic: 0.0,
        perceptual_roughness: 0.98,
        ..default()
    });
    let accent_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.24, 0.12),
        metallic: 0.0,
        perceptual_roughness: 0.92,
        ..default()
    });

    // A single continuous floor joins the corridor and both rooms.
    spawn_box(
        &mut commands,
        &mut meshes,
        &floor_material,
        Vec3::new(3.3, -0.12, 0.0),
        Vec3::new(20.0, 0.24, 24.0),
    );

    // Main corridor: the right wall has a doorway into the side-room wing.
    spawn_box(
        &mut commands,
        &mut meshes,
        &wall_material,
        Vec3::new(-2.2, 1.5, 0.5),
        Vec3::new(0.2, 3.0, 19.0),
    );
    spawn_box(
        &mut commands,
        &mut meshes,
        &wall_material,
        Vec3::new(2.2, 1.5, -4.25),
        Vec3::new(0.2, 3.0, 9.5),
    );
    spawn_box(
        &mut commands,
        &mut meshes,
        &wall_material,
        Vec3::new(2.2, 1.5, 6.75),
        Vec3::new(0.2, 3.0, 6.5),
    );
    spawn_box(
        &mut commands,
        &mut meshes,
        &wall_material,
        Vec3::new(0.0, 1.5, 10.0),
        Vec3::new(4.4, 3.0, 0.2),
    );

    // Side-room wing. The two aligned openings allow the camera rail to enter
    // the inner chamber only after the shared-prefix captures are complete.
    for z in [-2.5, 6.5] {
        spawn_box(
            &mut commands,
            &mut meshes,
            &wall_material,
            Vec3::new(7.85, 1.5, z),
            Vec3::new(11.3, 3.0, 0.2),
        );
    }

    // First interior divider: opening is shared by both hidden worlds.
    spawn_box(
        &mut commands,
        &mut meshes,
        &wall_material,
        Vec3::new(4.2, 1.5, -1.0),
        Vec3::new(0.2, 3.0, 3.0),
    );
    spawn_box(
        &mut commands,
        &mut meshes,
        &wall_material,
        Vec3::new(4.2, 1.5, 5.0),
        Vec3::new(0.2, 3.0, 3.0),
    );

    // Far wall has a fixed-width opening. World A leaves it open; World B
    // inserts one panel. This is the only variant-dependent geometry.
    spawn_box(
        &mut commands,
        &mut meshes,
        &wall_material,
        Vec3::new(9.2, 1.5, -1.0),
        Vec3::new(0.2, 3.0, 3.0),
    );
    spawn_box(
        &mut commands,
        &mut meshes,
        &wall_material,
        Vec3::new(9.2, 1.5, 5.0),
        Vec3::new(0.2, 3.0, 3.0),
    );
    if world.variant.has_far_door_panel() {
        spawn_box(
            &mut commands,
            &mut meshes,
            &accent_material,
            Vec3::new(9.2, 1.5, 2.0),
            Vec3::new(0.2, 3.0, 3.0),
        );
    }

    // A second, shared chamber beyond the far doorway makes the open/closed
    // distinction visually legible from the diagnostic waypoint.
    spawn_box(
        &mut commands,
        &mut meshes,
        &wall_material,
        Vec3::new(13.5, 1.5, 2.0),
        Vec3::new(0.2, 3.0, 9.0),
    );

    // No shadow maps: a hidden topology change must not alter the early
    // observations through shadow casting.
    commands.spawn((
        DirectionalLight {
            illuminance: 9_000.0,
            color: Color::linear_rgb(1.0, 0.98, 0.94),
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.72, -0.62, 0.0)),
    ));

    let initial = camera_rail()
        .waypoints
        .first()
        .expect("camera rail has an initial pose")
        .transform
        .clone();
    commands.spawn((Camera3d::default(), initial, TwinCamera));
}

fn move_camera_on_rail(
    time: Res<Time<Real>>,
    rail: Res<CameraRail>,
    mut cameras: Query<&mut Transform, With<TwinCamera>>,
) {
    let pose = camera_pose_at(&rail, time.elapsed_secs());
    for mut transform in &mut cameras {
        *transform = pose.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_two_capture_times_hold_the_same_camera_pose() {
        let rail = camera_rail();
        let first = camera_pose_at(&rail, 1.5);
        let second = camera_pose_at(&rail, CAPTURE_PREFIX);
        assert_eq!(first.translation, second.translation);
        assert_eq!(first.rotation, second.rotation);
    }

    #[test]
    fn camera_reaches_the_diagnostic_chamber_before_the_final_capture() {
        let rail = camera_rail();
        let pose = camera_pose_at(&rail, REVEAL_CAPTURE);
        assert!(pose.translation.x > 5.0);
        assert!((pose.translation.z - 2.0).abs() < f32::EPSILON);
        assert!(camera_pose_at(&rail, 6.8).rotation != camera_pose_at(&rail, 6.4).rotation);
    }

    #[test]
    fn world_variants_differ_only_by_the_far_door_panel_switch() {
        assert!(!WorldVariant::AOpen.has_far_door_panel());
        assert!(WorldVariant::BClosed.has_far_door_panel());
    }

    #[test]
    fn variant_parser_is_explicit_and_fail_closed() {
        assert_eq!(WorldVariant::from_value("A"), Some(WorldVariant::AOpen));
        assert_eq!(WorldVariant::from_value("B"), Some(WorldVariant::BClosed));
        assert_eq!(WorldVariant::from_value("C"), None);
    }
}

impl WorldVariant {
    fn from_value(value: &str) -> Option<Self> {
        match value {
            "A" => Some(Self::AOpen),
            "B" => Some(Self::BClosed),
            _ => None,
        }
    }
}
