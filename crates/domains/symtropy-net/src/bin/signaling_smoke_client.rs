// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Process-level smoke client for the experimental Symtropy JSON relay.
//!
//! The CI shell harness starts two independent copies of this binary against the
//! actual signaling-server executable. This checks local cross-process delivery;
//! it does not qualify public-service interoperability or production security.

use std::{collections::VecDeque, env, time::Duration};

use symtropy_net::{
    peer::PeerId,
    signaling::{SignalChannel, SignalData, SignalingClient, SignalingEvent},
};
use tokio::time::{sleep, timeout};

const ALPHA_TO_BETA: &[u8] = b"symtropy-process-smoke-alpha-to-beta-v1";
const BETA_TO_ALPHA: &[u8] = b"symtropy-process-smoke-beta-to-alpha-v1";

type PendingSignal = (PeerId, SignalChannel, Vec<u8>);

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let role = env::var("SYMTROPY_SMOKE_ROLE")
        .map_err(|_| "SYMTROPY_SMOKE_ROLE must be alpha or beta".to_string())?;
    if role != "alpha" && role != "beta" {
        return Err("SYMTROPY_SMOKE_ROLE must be alpha or beta".to_string());
    }

    let url = env::var("SYMTROPY_SIGNAL_URL")
        .map_err(|_| "SYMTROPY_SIGNAL_URL is required".to_string())?;
    let room = env::var("SYMTROPY_SMOKE_ROOM")
        .map_err(|_| "SYMTROPY_SMOKE_ROOM is required".to_string())?;

    let mut client = SignalingClient::connect(&url).await?;
    client.join(&room)?;
    let (local_id, remote_id, mut pending) = wait_for_peer(&mut client).await?;
    if local_id == remote_id {
        return Err("server assigned the same peer ID to both clients".to_string());
    }
    println!(
        "CONNECTED role={role} local_peer={} remote_peer={} room={room}",
        local_id.0, remote_id.0
    );

    if role == "alpha" {
        client.signal(
            remote_id,
            SignalData::RelayData {
                channel: SignalChannel::Reliable,
                payload: ALPHA_TO_BETA.to_vec(),
            },
        )?;
        println!(
            "LOCAL_SEND_ACCEPTED role=alpha payload_bytes={}",
            ALPHA_TO_BETA.len()
        );
        wait_for_payload(&mut client, remote_id, BETA_TO_ALPHA, &mut pending).await?;
        println!("REMOTE_PAYLOAD_OBSERVED role=alpha direction=beta-to-alpha");
    } else {
        wait_for_payload(&mut client, remote_id, ALPHA_TO_BETA, &mut pending).await?;
        println!("REMOTE_PAYLOAD_OBSERVED role=beta direction=alpha-to-beta");
        client.signal(
            remote_id,
            SignalData::RelayData {
                channel: SignalChannel::Reliable,
                payload: BETA_TO_ALPHA.to_vec(),
            },
        )?;
        println!(
            "LOCAL_SEND_ACCEPTED role=beta payload_bytes={}",
            BETA_TO_ALPHA.len()
        );
    }

    println!("PASS role={role} bidirectional_relay_round_trip=true");
    Ok(())
}

async fn wait_for_peer(
    client: &mut SignalingClient,
) -> Result<(PeerId, PeerId, VecDeque<PendingSignal>), String> {
    timeout(Duration::from_secs(15), async {
        let mut local_id: Option<PeerId> = None;
        let mut remote_id: Option<PeerId> = None;
        let mut pending = VecDeque::new();

        loop {
            for event in client.poll_events() {
                match event {
                    SignalingEvent::Connected(id) => {
                        if let Some(previous) = local_id {
                            if previous != id {
                                return Err(format!(
                                    "server changed assigned local ID from {} to {}",
                                    previous.0, id.0
                                ));
                            }
                        } else {
                            local_id = Some(id);
                        }
                    }
                    SignalingEvent::PeerJoined(id) => {
                        if local_id == Some(id) {
                            return Err("server announced the local peer as a remote peer".to_string());
                        }
                        if let Some(previous) = remote_id {
                            if previous != id {
                                return Err(format!(
                                    "smoke room unexpectedly admitted more than two peers: {} and {}",
                                    previous.0, id.0
                                ));
                            }
                        } else {
                            remote_id = Some(id);
                        }
                    }
                    SignalingEvent::PeerLeft(id) => {
                        if remote_id == Some(id) {
                            return Err(format!("remote peer {} left during admission", id.0));
                        }
                    }
                    SignalingEvent::Signal { from, data } => match data {
                        SignalData::RelayData { channel, payload } => {
                            pending.push_back((from, channel, payload));
                        }
                        _ => return Err("received a signaling control message during the relay smoke".to_string()),
                    },
                    SignalingEvent::Disconnected => {
                        return Err("signaling server disconnected during admission".to_string());
                    }
                    SignalingEvent::Error(message) => {
                        return Err(format!("signaling server error during admission: {message}"));
                    }
                }
            }

            if let (Some(local), Some(remote)) = (local_id, remote_id) {
                return Ok((local, remote, pending));
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| "timed out waiting for Welcome and PeerJoined".to_string())?
}

async fn wait_for_payload(
    client: &mut SignalingClient,
    expected_from: PeerId,
    expected_payload: &[u8],
    pending: &mut VecDeque<PendingSignal>,
) -> Result<(), String> {
    timeout(Duration::from_secs(15), async {
        loop {
            if let Some((from, channel, payload)) = pending.pop_front() {
                return validate_payload(from, channel, &payload, expected_from, expected_payload);
            }

            for event in client.poll_events() {
                match event {
                    SignalingEvent::Signal { from, data } => match data {
                        SignalData::RelayData { channel, payload } => {
                            return validate_payload(
                                from,
                                channel,
                                &payload,
                                expected_from,
                                expected_payload,
                            );
                        }
                        _ => {
                            return Err(
                                "received a signaling control message instead of relay data"
                                    .to_string(),
                            );
                        }
                    },
                    SignalingEvent::Disconnected => {
                        return Err(
                            "signaling server disconnected before the expected payload".to_string()
                        );
                    }
                    SignalingEvent::Error(message) => {
                        return Err(format!(
                            "signaling server error before the expected payload: {message}"
                        ));
                    }
                    SignalingEvent::PeerLeft(id) if id == expected_from => {
                        return Err(format!("expected sender {} disconnected", id.0));
                    }
                    _ => {}
                }
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| {
        format!(
            "timed out waiting for expected payload from peer {}",
            expected_from.0
        )
    })?
}

fn validate_payload(
    from: PeerId,
    channel: SignalChannel,
    payload: &[u8],
    expected_from: PeerId,
    expected_payload: &[u8],
) -> Result<(), String> {
    if from != expected_from {
        return Err(format!(
            "payload sender mismatch: expected peer {}, received peer {}",
            expected_from.0, from.0
        ));
    }
    if channel != SignalChannel::Reliable {
        return Err("payload arrived on an unexpected relay channel label".to_string());
    }
    if payload != expected_payload {
        return Err(format!(
            "payload mismatch from peer {}: received {} bytes, expected {} bytes",
            from.0,
            payload.len(),
            expected_payload.len()
        ));
    }
    Ok(())
}
