// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Two-process native Iroh connectivity witness.
//!
//! This executable proves endpoint identity, room binding, ordered-stream data,
//! QUIC datagrams, disconnect, and same-identity reconnect. It intentionally
//! does not claim Lightyear replication or rollback; those are separate gates.

use std::time::Duration;

use serde::Serialize;
use symtropy_net::native_actor::{
    NativeIrohActorHandle, NativeIrohCommand, NativeIrohConfig, NativeIrohEvent,
    spawn_native_iroh_actor,
};
use symtropy_net::transport::Channel;
use tokio::time::{sleep, timeout};

const ROOM: &str = "symtropy-native-witness-v1";
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Serialize)]
struct WitnessReport {
    role: &'static str,
    local_peer: u64,
    remote_peer: u64,
    reliable_round_trip: bool,
    unreliable_round_trip: bool,
    reconnect_succeeded: bool,
    endpoint_identity_stable: bool,
}

fn usage() -> ! {
    eprintln!(
        "usage:\n  symtropy-native-multiplayer-witness server\n  \
         symtropy-native-multiplayer-witness client '<endpoint-address-json>'"
    );
    std::process::exit(2);
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let role = args.next().unwrap_or_default();
    let result = match role.as_str() {
        "server" => run_server().await,
        "client" => {
            let encoded = args.next().unwrap_or_else(|| usage());
            run_client(&encoded).await
        }
        _ => usage(),
    };
    if let Err(error) = result {
        eprintln!("native witness failed: {error}");
        std::process::exit(1);
    }
}

async fn run_server() -> Result<(), String> {
    let mut config = NativeIrohConfig::internet(ROOM);
    config.secret_key = fixed_secret_from_env("SYMTROPY_WITNESS_SERVER_KEY")?;
    let actor = spawn_native_iroh_actor(config)
        .await
        .map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::json!({
            "event": "ready",
            "peer": actor.local_peer().0,
            "endpoint_addr": actor.endpoint_addr(),
        })
    );

    let mut remote_peer = None;
    let mut reliable = false;
    let mut unreliable = false;
    let mut reconnect = false;
    let mut connection_count = 0_u32;
    timeout(TIMEOUT.saturating_mul(4), async {
        loop {
            for event in actor.drain_events() {
                match event {
                    NativeIrohEvent::PeerConnected { peer, .. } => {
                        remote_peer = Some(peer);
                        connection_count = connection_count.saturating_add(1);
                        reconnect |= connection_count >= 2;
                    }
                    NativeIrohEvent::Message(message) => {
                        let response = match (message.channel, message.data.as_slice()) {
                            (Channel::Reliable, b"ping-reliable") => {
                                reliable = true;
                                Some((Channel::Reliable, b"pong-reliable".as_slice()))
                            }
                            (Channel::Unreliable, b"ping-unreliable") => {
                                unreliable = true;
                                Some((Channel::Unreliable, b"pong-unreliable".as_slice()))
                            }
                            (Channel::Reliable, b"ping-reconnect") => {
                                reconnect = true;
                                Some((Channel::Reliable, b"pong-reconnect".as_slice()))
                            }
                            _ => None,
                        };
                        if let Some((channel, payload)) = response {
                            actor
                                .command(NativeIrohCommand::Send {
                                    peer: message.from,
                                    channel,
                                    payload: payload.to_vec(),
                                })
                                .await
                                .map_err(|error| error.to_string())?;
                        }
                    }
                    NativeIrohEvent::Error(error) => return Err(error),
                    _ => {}
                }
            }
            if reliable && unreliable && reconnect {
                return Ok(());
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| "server witness timed out".to_owned())??;

    println!(
        "{}",
        serde_json::to_string(&WitnessReport {
            role: "server",
            local_peer: actor.local_peer().0,
            remote_peer: remote_peer.ok_or("server never observed a peer")?.0,
            reliable_round_trip: reliable,
            unreliable_round_trip: unreliable,
            reconnect_succeeded: reconnect,
            endpoint_identity_stable: true,
        })
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

async fn run_client(encoded_addr: &str) -> Result<(), String> {
    let remote_addr = serde_json::from_str(encoded_addr).map_err(|error| error.to_string())?;
    let mut config = NativeIrohConfig::internet(ROOM);
    config.secret_key = fixed_secret_from_env("SYMTROPY_WITNESS_CLIENT_KEY")?;
    let actor = spawn_native_iroh_actor(config)
        .await
        .map_err(|error| error.to_string())?;
    let initial_endpoint = actor.endpoint_id();
    actor
        .command(NativeIrohCommand::Dial {
            remote_addr: remote_addr.clone(),
        })
        .await
        .map_err(|error| error.to_string())?;
    let remote_peer = wait_connected(&actor).await?;

    send_and_wait(
        &actor,
        remote_peer,
        Channel::Reliable,
        b"ping-reliable",
        b"pong-reliable",
    )
    .await?;
    send_and_wait(
        &actor,
        remote_peer,
        Channel::Unreliable,
        b"ping-unreliable",
        b"pong-unreliable",
    )
    .await?;

    actor
        .command(NativeIrohCommand::Disconnect { peer: remote_peer })
        .await
        .map_err(|error| error.to_string())?;
    wait_disconnected(&actor, remote_peer).await?;
    actor
        .command(NativeIrohCommand::Dial { remote_addr })
        .await
        .map_err(|error| error.to_string())?;
    let reconnected_peer = wait_connected(&actor).await?;
    if reconnected_peer != remote_peer {
        return Err("remote identity changed across reconnect".into());
    }
    send_and_wait(
        &actor,
        remote_peer,
        Channel::Reliable,
        b"ping-reconnect",
        b"pong-reconnect",
    )
    .await?;

    println!(
        "{}",
        serde_json::to_string(&WitnessReport {
            role: "client",
            local_peer: actor.local_peer().0,
            remote_peer: remote_peer.0,
            reliable_round_trip: true,
            unreliable_round_trip: true,
            reconnect_succeeded: true,
            endpoint_identity_stable: actor.endpoint_id() == initial_endpoint,
        })
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

async fn wait_connected(actor: &NativeIrohActorHandle) -> Result<symtropy_net::PeerId, String> {
    timeout(TIMEOUT, async {
        loop {
            for event in actor.drain_events() {
                match event {
                    NativeIrohEvent::PeerConnected { peer, .. } => return Ok(peer),
                    NativeIrohEvent::Error(error) => return Err(error),
                    _ => {}
                }
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| "connection timed out".to_owned())?
}

async fn wait_disconnected(
    actor: &NativeIrohActorHandle,
    peer: symtropy_net::PeerId,
) -> Result<(), String> {
    timeout(TIMEOUT, async {
        loop {
            for event in actor.drain_events() {
                match event {
                    NativeIrohEvent::PeerDisconnected { peer: observed, .. }
                        if observed == peer => return Ok(()),
                    NativeIrohEvent::Error(error) => return Err(error),
                    _ => {}
                }
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| "disconnect timed out".to_owned())?
}

async fn send_and_wait(
    actor: &NativeIrohActorHandle,
    peer: symtropy_net::PeerId,
    channel: Channel,
    request: &[u8],
    response: &[u8],
) -> Result<(), String> {
    actor
        .command(NativeIrohCommand::Send {
            peer,
            channel,
            payload: request.to_vec(),
        })
        .await
        .map_err(|error| error.to_string())?;
    timeout(TIMEOUT, async {
        loop {
            for event in actor.drain_events() {
                match event {
                    NativeIrohEvent::Message(message)
                        if message.from == peer
                            && message.channel == channel
                            && message.data == response => return Ok(()),
                    NativeIrohEvent::Error(error) => return Err(error),
                    _ => {}
                }
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| "round trip timed out".to_owned())?
}

fn fixed_secret_from_env(name: &str) -> Result<Option<[u8; 32]>, String> {
    let Ok(value) = std::env::var(name) else {
        return Ok(None);
    };
    let bytes = value.as_bytes();
    if bytes.len() != 64 || !bytes.iter().all(u8::is_ascii_hexdigit) {
        return Err(format!("{name} must contain exactly 64 hexadecimal characters"));
    }
    let mut output = [0_u8; 32];
    for (index, pair) in bytes.chunks_exact(2).enumerate() {
        output[index] = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16)
            .map_err(|error| error.to_string())?;
    }
    Ok(Some(output))
}
