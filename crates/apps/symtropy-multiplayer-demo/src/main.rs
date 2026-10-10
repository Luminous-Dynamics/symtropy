// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use symtropy_lightyear::SymtropyNetPlugin;
use symtropy_lightyear::components::*;

/// This demo currently runs locally only. The networking plugin is a scaffold;
/// the host flag is retained for command-line compatibility, not server startup.
const DEMO_PLAYER_COUNT: u64 = 8;

#[derive(Resource, Clone, Copy, Debug)]
struct LocalDemoPlayerId(u64);

#[derive(Component)]
struct LocallyControlledPlayer;

// --- Candidate protocol ---

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct PlayerController {
    pub id: u64,
}

// --- Demo Plugin ---

pub struct MultiplayerDemoPlugin;

impl Plugin for MultiplayerDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_scene);
        app.add_systems(
            Update,
            (
                handle_input,
                sync_physics_to_render,
                interest_management_debug,
            ),
        );
    }
}

fn parse_player_id(args: &[String]) -> Result<u64, String> {
    let mut requested: Option<&str> = None;

    for (index, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix("--player-id=") {
            requested = Some(value);
            break;
        }
        if arg == "--player-id" {
            requested = Some(
                args.get(index + 1)
                    .map(String::as_str)
                    .ok_or_else(|| "missing value after --player-id".to_string())?,
            );
            break;
        }
    }

    let Some(value) = requested else {
        return Ok(0);
    };
    let id = value
        .parse::<u64>()
        .map_err(|_| format!("invalid player ID {value:?}; expected an integer"))?;
    if id >= DEMO_PLAYER_COUNT {
        return Err(format!(
            "player ID {id} is outside this local preview's supported range 0..{}",
            DEMO_PLAYER_COUNT - 1
        ));
    }
    Ok(id)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_host_flag = args.iter().any(|arg| arg == "--host");
    let player_id = match parse_player_id(&args) {
        Ok(id) => id,
        Err(error) => {
            eprintln!("{error}; selecting local preview player 0");
            0
        }
    };

    eprintln!(
        "LOCAL PREVIEW ONLY: no multiplayer server, peer connection, or Lightyear replication is active."
    );
    if is_host_flag {
        eprintln!("The --host flag is compatibility-only; it does not start a server.");
    }

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: if is_host_flag {
                format!("Symtropy Local Preview - host flag only (P{player_id})")
            } else {
                format!("Symtropy Local Preview (P{player_id})")
            },
            ..default()
        }),
        ..default()
    }));

    // SymtropyNetPlugin performs candidate component reflection registration.
    // Do not call protocol::register separately; doing so would register them twice.
    app.register_type::<PlayerController>();
    app.insert_resource(LocalDemoPlayerId(player_id));
    app.add_plugins(SymtropyNetPlugin);
    app.add_plugins(MultiplayerDemoPlugin);
    app.run();
}

fn setup_scene(
    mut commands: Commands,
    local_player: Res<LocalDemoPlayerId>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Setup camera.
    commands
        .spawn(Camera3d::default())
        .insert(Transform::from_xyz(0.0, 50.0, 75.0).looking_at(Vec3::ZERO, Vec3::Y));

    // Setup light.
    commands.spawn(DirectionalLight {
        illuminance: 10000.0,
        shadow_maps_enabled: true,
        ..default()
    });

    // Spawn floor.
    let floor_mesh = meshes.add(Plane3d::default().mesh().size(200.0, 200.0));
    let floor_material = materials.add(Color::srgb(0.1, 0.12, 0.1));
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Spawn local-preview placeholders; these are not remote/networked players.
    let sphere_mesh = meshes.add(Sphere::new(0.5).mesh().uv(32, 16));

    for i in 0..DEMO_PLAYER_COUNT {
        let angle = (i as f32 / DEMO_PLAYER_COUNT as f32) * std::f32::consts::TAU;
        let radius = 30.0;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;

        let color = Color::hsl(i as f32 * 45.0, 0.9, 0.6);
        let material = materials.add(color);

        let mut player = commands.spawn((
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(x, 0.5, z),
            NetPosition {
                x: x as f64,
                y: 0.5,
                z: z as f64,
            },
            NetAuthority {
                peer_id: i,
                transferable: true,
            },
            SpatialZone(0),
            PlayerController { id: i },
        ));

        if i == local_player.0 {
            player.insert(LocallyControlledPlayer);
        }
    }
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut NetPosition, With<LocallyControlledPlayer>>,
) {
    let mut move_vec = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        move_vec.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        move_vec.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        move_vec.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        move_vec.x += 1.0;
    }

    if move_vec.length_squared() > 0.0 {
        move_vec = move_vec.normalize() * 0.5;
        for mut pos in query.iter_mut() {
            pos.x += move_vec.x as f64;
            pos.z += move_vec.z as f64;
        }
    }
}

fn sync_physics_to_render(mut query: Query<(&NetPosition, &mut Transform)>) {
    for (net_pos, mut transform) in query.iter_mut() {
        transform.translation.x = net_pos.x as f32;
        transform.translation.y = net_pos.y as f32;
        transform.translation.z = net_pos.z as f32;
    }
}

fn interest_management_debug(query: Query<(&PlayerController, &SpatialZone)>) {
    for (_controller, _zone) in &query {
        // Placeholder only: no network interest/visibility policy is active yet.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_id_defaults_to_zero() {
        assert_eq!(parse_player_id(&["demo".into()]).unwrap(), 0);
    }

    #[test]
    fn player_id_supports_separate_and_equals_forms() {
        assert_eq!(
            parse_player_id(&["demo".into(), "--player-id".into(), "3".into()]).unwrap(),
            3
        );
        assert_eq!(
            parse_player_id(&["demo".into(), "--player-id=4".into()]).unwrap(),
            4
        );
    }

    #[test]
    fn player_id_rejects_out_of_range_and_missing_values() {
        assert!(parse_player_id(&["demo".into(), "--player-id".into(), "8".into()]).is_err());
        assert!(parse_player_id(&["demo".into(), "--player-id".into()]).is_err());
        assert!(parse_player_id(&["demo".into(), "--player-id=x".into()]).is_err());
    }
}
