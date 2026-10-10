// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use bevy::prelude::*;
use bevy::log::{Level, LogPlugin};
use bevy::state::app::StatesPlugin;
use lightyear::connection::client::Connected;
use lightyear::connection::server::Start;
use lightyear::netcode::Key;
use lightyear::prelude::client::{
    Authentication, ClientPlugins, NetcodeClient,
    NetcodeConfig as ClientNetcodeConfig, UdpIo,
};
use lightyear::prelude::server::{
    NetcodeConfig as ServerNetcodeConfig, NetcodeServer, ServerPlugins, ServerUdpIo,
};
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use symtropy_lightyear::SymtropyNetPlugin;
use symtropy_lightyear::components::*;

/// This demo currently runs locally only. The networking plugin is a scaffold;
/// the host flag is retained for command-line compatibility, not server startup.
const DEMO_PLAYER_COUNT: u64 = 8;
const NETWORK_TICK_HZ: f64 = 30.0;
const NETWORK_SERVER_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5000);

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

    // This mode uses real Lightyear 0.28 UDP/Netcode sockets on localhost.
    // It is separate from the Iroh stub and is an experimental smoke path.
    if args.iter().any(|arg| arg == "--network-server") {
        eprintln!(
            "LIGHTYEAR UDP SMOKE: starting localhost server on {NETWORK_SERVER_ADDR}"
        );
        run_network_server();
        return;
    }
    if args.iter().any(|arg| arg == "--network-client") {
        let player_id = match parse_player_id(&args) {
            Ok(id) => id,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        };
        eprintln!(
            "LIGHTYEAR UDP SMOKE: connecting client {player_id} to {NETWORK_SERVER_ADDR}"
        );
        run_network_client(player_id);
        return;
    }

    let is_host_flag = args.iter().any(|arg| arg == "--host");
    let player_id = match parse_player_id(&args) {
        Ok(id) => id,
        Err(error) => {
            eprintln!("{error}; selecting local preview player 0");
            0
        }
    };

    eprintln!("LOCAL PREVIEW ONLY: no multiplayer server or peer connection is active.");
    eprintln!("Lightyear replication is not configured.");
    if is_host_flag {
        eprintln!("The --host flag is compatibility-only; it does not start a server.");
    }
    eprintln!("Experimental localhost UDP smoke commands:");
    eprintln!("  server: --network-server");
    eprintln!("  client: --network-client --player-id 1");

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

fn build_headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        LogPlugin {
            level: Level::INFO,
            ..default()
        },
        StatesPlugin,
    ));
    app
}

fn run_network_server() {
    let mut app = build_headless_app();
    app.add_plugins(ServerPlugins {
        tick_duration: Duration::from_secs_f64(1.0 / NETWORK_TICK_HZ),
    });
    app.component::<NetPosition>().replicate();
    app.insert_resource(ReplicationMetadata::new(Duration::from_millis(100)));
    app.add_systems(
        Startup,
        (start_network_server, spawn_server_replicated_state),
    );
    app.add_systems(Update, advance_server_replicated_state);
    app.add_observer(attach_replication_sender);
    app.run();
}

fn start_network_server(mut commands: Commands) -> Result {
    let server = commands
        .spawn((
            NetcodeServer::new(ServerNetcodeConfig::default()),
            LocalAddr(NETWORK_SERVER_ADDR),
            ServerUdpIo::default(),
            Name::new("Symtropy Lightyear UDP Smoke Server"),
        ))
        .id();
    commands.trigger(Start { entity: server });
    info!(
        "LIGHTYEAR_SMOKE server_start_requested addr={NETWORK_SERVER_ADDR}"
    );
    Ok(())
}

fn spawn_server_replicated_state(mut commands: Commands) {
    commands.spawn((
        NetPosition {
            x: 0.0,
            y: 0.5,
            z: 0.0,
        },
        Replicate::to_clients(NetworkTarget::All),
        Name::new("Symtropy Authoritative Smoke Entity"),
    ));
    info!("LIGHTYEAR_SMOKE authoritative_entity_spawned");
}

fn attach_replication_sender(
    trigger: On<Add, Connected>,
    mut commands: Commands,
) {
    commands.entity(trigger.entity).insert(ReplicationSender);
    info!(
        "LIGHTYEAR_SMOKE client_connected link={:?}",
        trigger.entity
    );
}

fn advance_server_replicated_state(mut positions: Query<&mut NetPosition>) {
    for mut position in &mut positions {
        position.x += 0.05;
        position.z += 0.025;
    }
}

fn run_network_client(player_id: u64) {
    let mut app = build_headless_app();
    app.add_plugins(ClientPlugins {
        tick_duration: Duration::from_secs_f64(1.0 / NETWORK_TICK_HZ),
    });
    app.component::<NetPosition>().replicate();
    app.insert_resource(LocalDemoPlayerId(player_id));
    app.add_systems(Startup, start_network_client);
    app.add_systems(Update, report_replicated_state);
    app.run();
}

fn start_network_client(
    mut commands: Commands,
    player_id: Res<LocalDemoPlayerId>,
) -> Result {
    let client_addr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        4000 + player_id.0 as u16,
    );
    let auth = Authentication::Manual {
        server_addr: NETWORK_SERVER_ADDR,
        client_id: player_id.0,
        private_key: Key::default(),
        protocol_id: 0,
    };
    let client = commands
        .spawn((
            Client::default(),
            LocalAddr(client_addr),
            PeerAddr(NETWORK_SERVER_ADDR),
            Link::new(None),
            ReplicationReceiver,
            NetcodeClient::new(auth, ClientNetcodeConfig::default())?,
            UdpIo::default(),
            Name::new(format!("Symtropy Lightyear UDP Smoke Client P{}", player_id.0)),
        ))
        .id();
    commands.trigger(Connect { entity: client });
    info!(
        "LIGHTYEAR_SMOKE client_connect_requested id={} addr={} server={}",
        player_id.0,
        client_addr,
        NETWORK_SERVER_ADDR
    );
    Ok(())
}

fn report_replicated_state(
    query: Query<(Entity, &NetPosition), Changed<NetPosition>>,
) {
    for (entity, position) in &query {
        info!(
            "LIGHTYEAR_SMOKE replication_update entity={entity:?} position=({:.3}, {:.3}, {:.3})",
            position.x,
            position.y,
            position.z,
        );
    }
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
