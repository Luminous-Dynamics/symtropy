// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Experimental self-hosted server for the Symtropy JSON WebSocket relay.
//!
//! This is an initial conformance prototype, not a production-hardened service.
//! It requires one shared bearer token for every client, assigns ephemeral numeric
//! peer IDs per process, and must be placed behind a TLS-terminating reverse proxy.
//! The shared token grants access; it does not establish an individual user identity.
//!
//! Run with:
//!   SYMTROPY_RELAY_TOKEN=<32+ base64url-safe characters> \
//!     cargo run -p symtropy-net --no-default-features --features ws-relay --bin symtropy-signaling-server
//!
//! The default bind is 127.0.0.1:9001. Do not expose the plaintext listener publicly.

use std::{
    collections::{HashMap, HashSet},
    env,
    io,
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use futures_util::{SinkExt, StreamExt};
use symtropy_net::{
    signaling::{
        MAX_SIGNAL_CONTROL_BYTES, MAX_SIGNALING_MESSAGE_BYTES, SignalData, SignalIncoming,
        SignalOutgoing, parse_incoming, serialized_outgoing,
    },
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{OwnedSemaphorePermit, Semaphore, mpsc, watch},
    task::JoinSet,
    time::timeout,
};
use tokio_tungstenite::{
    accept_hdr_async_with_config,
    tungstenite::{
        Message,
        handshake::server::{ErrorResponse, Request, Response},
        http::{StatusCode, header::AUTHORIZATION},
        protocol::WebSocketConfig,
    },
};

const DEFAULT_BIND: &str = "127.0.0.1:9001";
const MAX_CLIENTS: usize = 32;
const MAX_ROOMS: usize = 32;
const MAX_ROOM_PEERS: usize = 32;
const MAX_ROOM_ID_BYTES: usize = 128;
const OUTBOUND_QUEUE_CAPACITY: usize = 32;
const MAX_MESSAGES_PER_SECOND: usize = 120;
const MAX_BYTES_PER_SECOND: usize = 8 * 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const WRITER_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);

type SharedRooms = Arc<Mutex<Rooms>>;

#[derive(Default)]
struct Rooms {
    rooms: HashMap<String, HashMap<u64, PeerHandle>>,
}

#[derive(Clone)]
struct PeerHandle {
    tx: mpsc::Sender<QueuedFrame>,
    queued_bytes: Arc<Semaphore>,
    /// Lets the room registry terminate a connection evicted for backpressure.
    shutdown_tx: watch::Sender<bool>,
}

struct QueuedFrame {
    frame: Message,
    _byte_permit: Option<OwnedSemaphorePermit>,
}

/// Reserves both count capacity and byte capacity before changing any room state.
/// The reservation can be committed synchronously after all recipients pass preflight.
struct PreparedFrame {
    slot: mpsc::OwnedPermit<QueuedFrame>,
    frame: Message,
    byte_permit: Option<OwnedSemaphorePermit>,
}

impl PreparedFrame {
    fn commit(self) {
        self.slot.send(QueuedFrame {
            frame: self.frame,
            _byte_permit: self.byte_permit,
        });
    }
}

struct RateWindow {
    started: Instant,
    messages: usize,
    bytes: usize,
}

impl RateWindow {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            messages: 0,
            bytes: 0,
        }
    }

    fn admit(&mut self, bytes: usize) -> bool {
        if self.started.elapsed() >= Duration::from_secs(1) {
            self.started = Instant::now();
            self.messages = 0;
            self.bytes = 0;
        }
        if self.messages >= MAX_MESSAGES_PER_SECOND
            || self.bytes.saturating_add(bytes) > MAX_BYTES_PER_SECOND
        {
            return false;
        }
        self.messages += 1;
        self.bytes += bytes;
        true
    }
}

struct ServerConfig {
    bind: SocketAddr,
    token: String,
}

impl ServerConfig {
    fn from_env() -> Result<Self, String> {
        let token = env::var("SYMTROPY_RELAY_TOKEN")
            .map_err(|_| "SYMTROPY_RELAY_TOKEN is required; generate a random 32+ character base64url-safe token".to_string())?;
        if !(32..=128).contains(&token.len())
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(
                "SYMTROPY_RELAY_TOKEN must be 32..=128 ASCII characters using only [A-Za-z0-9_-]"
                    .to_string(),
            );
        }

        let bind = env::var("SYMTROPY_RELAY_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_string());
        let bind: SocketAddr = bind
            .parse()
            .map_err(|_| "SYMTROPY_RELAY_BIND must be a socket address such as 127.0.0.1:9001".to_string())?;
        if !bind.ip().is_loopback()
            && env::var("SYMTROPY_RELAY_ALLOW_INSECURE_BIND").ok().as_deref() != Some("1")
        {
            return Err(
                "refusing a non-loopback plaintext bind; use a TLS-terminating reverse proxy or explicitly set SYMTROPY_RELAY_ALLOW_INSECURE_BIND=1 only in a controlled test environment"
                    .to_string(),
            );
        }

        Ok(Self { bind, token })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::from_env().map_err(io::Error::other)?;
    let listener = TcpListener::bind(config.bind).await?;
    eprintln!(
        "symtropy signaling relay listening on {} (bearer auth required; TLS must terminate upstream)",
        config.bind
    );

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = shutdown_tx.send(true);
    });

    run_server(listener, config.token, shutdown_rx).await?;
    Ok(())
}

async fn run_server(
    listener: TcpListener,
    token: String,
    mut shutdown: watch::Receiver<bool>,
) -> io::Result<()> {
    let token = Arc::new(token);
    let rooms: SharedRooms = Arc::new(Mutex::new(Rooms::default()));
    let client_slots = Arc::new(Semaphore::new(MAX_CLIENTS));
    let next_peer_id = Arc::new(AtomicU64::new(1));
    let mut clients = JoinSet::new();

    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            accepted = listener.accept() => {
                let (stream, address) = accepted?;
                let permit = match client_slots.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(_) => {
                        eprintln!("rejected connection from {address}: client capacity reached");
                        drop(stream);
                        continue;
                    }
                };
                let token = token.clone();
                let rooms = rooms.clone();
                let next_peer_id = next_peer_id.clone();
                clients.spawn(async move {
                    let _client_permit = permit;
                    if let Err(error) = handle_connection(stream, token, rooms, next_peer_id).await {
                        eprintln!("relay client {address} ended: {error}");
                    }
                });
            }
            completed = clients.join_next(), if !clients.is_empty() => {
                if let Some(Err(error)) = completed {
                    eprintln!("relay connection task failed: {error}");
                }
            }
        }
    }

    clients.abort_all();
    while clients.join_next().await.is_some() {}
    Ok(())
}

fn websocket_config() -> WebSocketConfig {
    WebSocketConfig {
        max_message_size: Some(MAX_SIGNALING_MESSAGE_BYTES),
        max_frame_size: Some(MAX_SIGNALING_MESSAGE_BYTES),
        max_write_buffer_size: MAX_SIGNALING_MESSAGE_BYTES + 256 * 1024,
        ..Default::default()
    }
}

fn token_matches(provided: &str, expected: &str) -> bool {
    let left = provided.as_bytes();
    let right = expected.as_bytes();
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        difference |= usize::from(left_byte ^ right_byte);
    }
    difference == 0
}

fn request_is_authorized(request: &Request, token: &str) -> bool {
    request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|provided| token_matches(provided, token))
}

fn unauthorized_response() -> ErrorResponse {
    tokio_tungstenite::tungstenite::http::Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(Some("relay authorization required".to_string()))
        .expect("static unauthorized HTTP response")
}

fn parse_client_message(text: &str) -> Result<SignalOutgoing, String> {
    if text.len() > MAX_SIGNALING_MESSAGE_BYTES {
        return Err(format!(
            "client envelope is {} bytes; maximum is {}",
            text.len(),
            MAX_SIGNALING_MESSAGE_BYTES
        ));
    }

    let command: SignalOutgoing =
        serde_json::from_str(text).map_err(|_| "invalid or unsupported signaling JSON".to_string())?;

    let is_relay_data = matches!(
        &command,
        SignalOutgoing::Signal {
            data: SignalData::RelayData { .. },
            ..
        }
    );
    if !is_relay_data && text.len() > MAX_SIGNAL_CONTROL_BYTES {
        return Err(format!(
            "raw signaling control envelope is {} bytes; maximum is {}",
            text.len(),
            MAX_SIGNAL_CONTROL_BYTES
        ));
    }

    // Shared serializer validates field caps and canonical serialized size, keeping
    // server parsing aligned with the client's version-0 wire contract.
    serialized_outgoing(&command)?;

    if let SignalOutgoing::Join { room } = &command {
        validate_room_id(room)?;
    }

    Ok(command)
}

fn validate_room_id(room: &str) -> Result<(), String> {
    if room.is_empty()
        || room.len() > MAX_ROOM_ID_BYTES
        || room.chars().any(char::is_control)
    {
        return Err(format!(
            "room ID must contain 1..={MAX_ROOM_ID_BYTES} UTF-8 bytes and no control characters"
        ));
    }
    Ok(())
}

fn prepare_message(peer: &PeerHandle, message: &SignalIncoming) -> Result<PreparedFrame, String> {
    let text = serde_json::to_string(message)
        .map_err(|error| format!("failed to serialize server message: {error}"))?;
    parse_incoming(&text).map_err(|error| format!("server message violates wire limits: {error}"))?;

    let charged_bytes = u32::try_from(text.len())
        .map_err(|_| "outbound message byte count exceeds accounting range".to_string())?;
    let byte_permit = peer
        .queued_bytes
        .clone()
        .try_acquire_many_owned(charged_bytes)
        .map_err(|_| "recipient outbound byte budget is full".to_string())?;
    let slot = peer
        .tx
        .clone()
        .try_reserve_owned()
        .map_err(|_| "recipient outbound message-count queue is full".to_string())?;

    Ok(PreparedFrame {
        slot,
        frame: Message::Text(text),
        byte_permit: Some(byte_permit),
    })
}

fn enqueue_message(peer: &PeerHandle, message: &SignalIncoming) -> Result<(), String> {
    prepare_message(peer, message)?.commit();
    Ok(())
}

fn enqueue_control_frame(peer: &PeerHandle, frame: Message) -> Result<(), String> {
    let slot = peer
        .tx
        .clone()
        .try_reserve_owned()
        .map_err(|_| "recipient outbound message-count queue is full".to_string())?;
    slot.send(QueuedFrame {
        frame,
        _byte_permit: None,
    });
    Ok(())
}

/// Performs an atomic room join: all required queue slots and byte permits for the
/// Welcome and every peer-joined event are reserved before any event is published.
fn join_room(
    rooms: &SharedRooms,
    room_id: &str,
    peer_id: u64,
    new_peer: &PeerHandle,
) -> Result<(), String> {
    let mut state = rooms
        .lock()
        .map_err(|_| "relay room registry lock poisoned".to_string())?;
    let existing: Vec<(u64, PeerHandle)> = state
        .rooms
        .get(room_id)
        .map(|room| {
            room.iter()
                .map(|(id, peer)| (*id, peer.clone()))
                .collect()
        })
        .unwrap_or_default();

    if existing.len() >= MAX_ROOM_PEERS {
        return Err(format!("room peer limit ({MAX_ROOM_PEERS}) reached"));
    }
    if !state.rooms.contains_key(room_id) && state.rooms.len() >= MAX_ROOMS {
        return Err(format!("room limit ({MAX_ROOMS}) reached"));
    }

    let mut staged = Vec::with_capacity(1 + existing.len() * 2);
    staged.push(prepare_message(
        new_peer,
        &SignalIncoming::Welcome { peer_id },
    )?);

    for (existing_id, existing_peer) in &existing {
        staged.push(prepare_message(
            new_peer,
            &SignalIncoming::PeerJoined {
                peer_id: *existing_id,
            },
        )?);
        staged.push(prepare_message(
            existing_peer,
            &SignalIncoming::PeerJoined { peer_id },
        )?);
    }

    state
        .rooms
        .entry(room_id.to_string())
        .or_default()
        .insert(peer_id, new_peer.clone());

    // Welcome is staged first, so the connection's first server event is always
    // its identity assignment. The membership events are committed only after all
    // recipients have capacity; no peer can observe a half-completed join.
    for message in staged {
        message.commit();
    }
    Ok(())
}

/// Removes a peer and publishes all required PeerLeft notifications as an atomic
/// bounded batch. If a recipient cannot reserve queue capacity, that slow recipient
/// is disconnected too; the surviving peers then receive events for every removal.
fn remove_peer_and_notify(rooms: &SharedRooms, room_id: &str, initial_peer_id: u64) {
    let Ok(mut state) = rooms.lock() else {
        eprintln!("relay room registry lock poisoned while removing peer");
        return;
    };

    let removed_initial = state
        .rooms
        .get_mut(room_id)
        .is_some_and(|room| room.peers.remove(&initial_peer_id).is_some());
    if !removed_initial {
        let empty = state.rooms.get(room_id).is_some_and(|room| room.peers.is_empty());
        if empty {
            state.rooms.remove(room_id);
        }
        return;
    }

    let mut removed = vec![initial_peer_id];
    let mut removed_set = HashSet::from([initial_peer_id]);

    loop {
        let Some(room) = state.rooms.get(room_id) else {
            return;
        };
        let survivors: Vec<(u64, PeerHandle)> = room
            .peers
            .iter()
            .filter(|(id, _)| !removed_set.contains(id))
            .map(|(id, peer)| (*id, peer.clone()))
            .collect();

        let mut staged = Vec::new();
        let mut slow_peers = HashSet::new();
        for removed_id in &removed {
            for (survivor_id, survivor) in &survivors {
                match prepare_message(
                    survivor,
                    &SignalIncoming::PeerLeft {
                        peer_id: *removed_id,
                    },
                ) {
                    Ok(message) => staged.push(message),
                    Err(_) => {
                        slow_peers.insert(*survivor_id);
                    }
                }
            }
        }

        if slow_peers.is_empty() {
            for message in staged {
                message.commit();
            }
            break;
        }

        // No notification is committed until every survivor can queue the full
        // removal batch. Disconnect saturated recipients, then retry for survivors.
        drop(staged);
        let Some(room) = state.rooms.get_mut(room_id) else {
            return;
        };
        let mut added = false;
        for slow_id in slow_peers {
            if let Some(slow_peer) = room.peers.remove(&slow_id) {
                let _ = slow_peer.shutdown_tx.send(true);
                if removed_set.insert(slow_id) {
                    removed.push(slow_id);
                    added = true;
                }
            }
        }
        if !added {
            break;
        }
    }

    let empty = if let Some(room) = state.rooms.get_mut(room_id) {
        room.peers.retain(|id, _| !removed_set.contains(id));
        room.peers.is_empty()
    } else {
        return;
    };
    if empty {
        state.rooms.remove(room_id);
    }
}

async fn handle_connection(
    stream: TcpStream,
    token: Arc<String>,
    rooms: SharedRooms,
    next_peer_id: Arc<AtomicU64>,
) -> Result<(), String> {
    let expected = token.clone();
    let callback = move |request: &Request, response: Response| {
        if request_is_authorized(request, expected.as_str()) {
            Ok(response)
        } else {
            Err(unauthorized_response())
        }
    };
    let websocket = timeout(
        CONNECT_TIMEOUT,
        accept_hdr_async_with_config(stream, callback, Some(websocket_config())),
    )
    .await
    .map_err(|_| "WebSocket upgrade timed out".to_string())?
    .map_err(|error| format!("WebSocket handshake rejected: {error}"))?;
    let mut websocket = websocket;

    let first = timeout(CONNECT_TIMEOUT, websocket.next())
        .await
        .map_err(|_| "client did not send Join within the connection timeout".to_string())?
        .ok_or_else(|| "client closed before sending Join".to_string())?
        .map_err(|error| format!("failed to read initial Join: {error}"))?;

    let first_text = match first {
        Message::Text(text) => text,
        Message::Close(_) => return Ok(()),
        _ => {
            let _ = websocket
                .send(Message::Text(
                    serde_json::to_string(&SignalIncoming::Error {
                        message: "first message must be a JSON Join text envelope".to_string(),
                    })
                    .expect("serialize static protocol error"),
                ))
                .await;
            let _ = websocket.close(None).await;
            return Ok(());
        }
    };

    let first_command = match parse_client_message(&first_text) {
        Ok(command) => command,
        Err(error) => {
            let _ = websocket
                .send(Message::Text(
                    serde_json::to_string(&SignalIncoming::Error { message: error })
                        .expect("serialize validation error"),
                ))
                .await;
            let _ = websocket.close(None).await;
            return Ok(());
        }
    };
    let room_id = match first_command {
        SignalOutgoing::Join { room } => room,
        _ => {
            let _ = websocket
                .send(Message::Text(
                    serde_json::to_string(&SignalIncoming::Error {
                        message: "first signaling message must be Join".to_string(),
                    })
                    .expect("serialize Join-required error"),
                ))
                .await;
            let _ = websocket.close(None).await;
            return Ok(());
        }
    };

    let peer_id = next_peer_id
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| current.checked_add(1))
        .map_err(|_| "server peer-ID space exhausted".to_string())?;

    let (out_tx, mut out_rx) = mpsc::channel::<QueuedFrame>(OUTBOUND_QUEUE_CAPACITY);
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let peer = PeerHandle {
        tx: out_tx,
        queued_bytes: Arc::new(Semaphore::new(MAX_SIGNALING_MESSAGE_BYTES)),
        shutdown_tx,
    };

    if let Err(error) = join_room(&rooms, &room_id, peer_id, &peer) {
        let _ = websocket
            .send(Message::Text(
                serde_json::to_string(&SignalIncoming::Error { message: error })
                    .expect("serialize admission error"),
            ))
            .await;
        let _ = websocket.close(None).await;
        return Ok(());
    }

    let (mut sink, mut input) = websocket.split();
    let mut writer = tokio::spawn(async move {
        while let Some(queued) = out_rx.recv().await {
            let QueuedFrame {
                frame,
                _byte_permit,
            } = queued;
            if let Err(error) = sink.send(frame).await {
                drop(_byte_permit);
                return Err(format!("WebSocket write failed: {error}"));
            }
            // Keep the permit charged until the queued frame is fully written.
            drop(_byte_permit);
        }
        sink.close()
            .await
            .map_err(|error| format!("WebSocket close failed: {error}"))?;
        Ok::<(), String>(())
    });

    let mut rate = RateWindow::new();
    loop {
        tokio::select! {
            shutdown_changed = shutdown_rx.changed() => {
                if shutdown_changed.is_err() || *shutdown_rx.borrow() {
                    break;
                }
            }
            writer_result = &mut writer => {
                match writer_result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => eprintln!("peer {peer_id} writer stopped: {error}"),
                    Err(error) => eprintln!("peer {peer_id} writer task failed: {error}"),
                }
                break;
            }
            incoming = timeout(IDLE_TIMEOUT, input.next()) => {
                let incoming = match incoming {
                    Ok(Some(incoming)) => incoming,
                    Ok(None) => break,
                    Err(_) => {
                        let _ = enqueue_message(&peer, &SignalIncoming::Error {
                            message: "connection idle timeout".to_string(),
                        });
                        break;
                    }
                };
                let message = match incoming {
                    Ok(message) => message,
                    Err(error) => {
                        eprintln!("peer {peer_id} read failed: {error}");
                        break;
                    }
                };
                match message {
                    Message::Text(text) => {
                        if !rate.admit(text.len()) {
                            let _ = enqueue_message(&peer, &SignalIncoming::Error {
                                message: "per-connection message or byte rate limit exceeded".to_string(),
                            });
                            break;
                        }
                        let command = match parse_client_message(&text) {
                            Ok(command) => command,
                            Err(error) => {
                                let _ = enqueue_message(&peer, &SignalIncoming::Error { message: error });
                                break;
                            }
                        };
                        // Do not retain the incoming raw JSON beside its decoded payload
                        // while serializing/enqueuing the forwarded envelope.
                        drop(text);
                        match command {
                            SignalOutgoing::Leave => break,
                            SignalOutgoing::Join { .. } => {
                                let _ = enqueue_message(&peer, &SignalIncoming::Error {
                                    message: "connection has already joined one room".to_string(),
                                });
                                break;
                            }
                            SignalOutgoing::Signal { to, data } => {
                                if to == peer_id {
                                    let _ = enqueue_message(&peer, &SignalIncoming::Error {
                                        message: "self-targeted signaling messages are not allowed".to_string(),
                                    });
                                    continue;
                                }
                                let (sender_is_member, recipient) = match rooms.lock() {
                                    Ok(state) => match state.rooms.get(&room_id) {
                                        Some(room) => (
                                            room.contains_key(&peer_id),
                                            room.get(&to).cloned(),
                                        ),
                                        None => (false, None),
                                    },
                                    Err(_) => {
                                        let _ = enqueue_message(&peer, &SignalIncoming::Error {
                                            message: "relay room registry unavailable".to_string(),
                                        });
                                        break;
                                    }
                                };
                                // A peer evicted for backpressure may race with the
                                // shutdown watch notification; membership is the authority.
                                if !sender_is_member {
                                    break;
                                }
                                let Some(recipient) = recipient else {
                                    let _ = enqueue_message(&peer, &SignalIncoming::Error {
                                        message: format!("recipient {to} is not a member of room {room_id:?}"),
                                    });
                                    continue;
                                };
                                let forwarded = SignalIncoming::Signal { from: peer_id, data };
                                if let Err(error) = enqueue_message(&recipient, &forwarded) {
                                    let _ = enqueue_message(&peer, &SignalIncoming::Error {
                                        message: format!("message not queued for recipient {to}: {error}"),
                                    });
                                }
                            }
                        }
                    }
                    Message::Ping(payload) => {
                        if enqueue_control_frame(&peer, Message::Pong(payload)).is_err() {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    Message::Binary(_) => {
                        let _ = enqueue_message(&peer, &SignalIncoming::Error {
                            message: "binary WebSocket messages are not supported by the V0 JSON protocol".to_string(),
                        });
                        break;
                    }
                    Message::Pong(_) => {}
                    _ => {}
                }
            }
        }
    }

    remove_peer_and_notify(&rooms, &room_id, peer_id);
    drop(peer);
    if timeout(WRITER_DRAIN_TIMEOUT, &mut writer).await.is_err() {
        writer.abort();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{client::IntoClientRequest, http::HeaderValue},
    };

    const TEST_TOKEN: &str = "test_token_that_is_long_enough_for_local_tests_123";

    async fn connect_authenticated(url: &str) -> tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<TcpStream>,
    > {
        let mut request = url.into_client_request().expect("valid WebSocket URL");
        request.headers_mut().insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {TEST_TOKEN}")).expect("valid token header"),
        );
        connect_async(request)
            .await
            .expect("authenticated WebSocket connection")
            .0
    }

    async fn next_server_message(
        socket: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<TcpStream>,
        >,
    ) -> SignalIncoming {
        loop {
            match timeout(Duration::from_secs(3), socket.next())
                .await
                .expect("server response timeout")
                .expect("server closed unexpectedly")
                .expect("read server frame")
            {
                Message::Text(text) => return parse_incoming(&text).expect("valid server envelope"),
                Message::Ping(payload) => socket.send(Message::Pong(payload)).await.expect("pong"),
                other => panic!("expected server text envelope, received {other:?}"),
            }
        }
    }

    async fn send_client_message(
        socket: &mut tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<TcpStream>,
        >,
        message: SignalOutgoing,
    ) {
        let text = serialized_outgoing(&message).expect("valid client message");
        socket.send(Message::Text(text)).await.expect("send client message");
    }

    #[test]
    fn bearer_token_match_rejects_missing_wrong_and_prefix_confusion() {
        assert!(token_matches(TEST_TOKEN, TEST_TOKEN));
        assert!(!token_matches("", TEST_TOKEN));
        assert!(!token_matches("x", TEST_TOKEN));
        assert!(!token_matches("Bearer test", TEST_TOKEN));
        assert!(!token_matches("test_token_that_is_long_enough_for_local_tests_124", TEST_TOKEN));
    }

    #[test]
    fn room_ids_are_bounded_and_reject_control_characters() {
        assert!(validate_room_id("room-1").is_ok());
        assert!(validate_room_id("🚀".repeat(32)).is_ok());
        assert!(validate_room_id("").is_err());
        assert!(validate_room_id("room\nforged").is_err());
        assert!(validate_room_id("x".repeat(MAX_ROOM_ID_BYTES + 1)).is_err());
    }

    #[test]
    fn rate_window_has_explicit_message_and_byte_ceiling() {
        let mut limiter = RateWindow::new();
        for _ in 0..MAX_MESSAGES_PER_SECOND {
            assert!(limiter.admit(1));
        }
        assert!(!limiter.admit(1));
        let mut byte_limiter = RateWindow::new();
        assert!(byte_limiter.admit(MAX_BYTES_PER_SECOND));
        assert!(!byte_limiter.admit(1));
    }

    #[tokio::test]
    async fn unauthorized_handshake_is_rejected_before_join() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind fixture");
        let address = listener.local_addr().expect("fixture address");
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let server = tokio::spawn(run_server(listener, TEST_TOKEN.to_string(), shutdown_rx));
        let url = format!("ws://{address}");
        let result = connect_async(url).await;
        assert!(result.is_err(), "server must require a bearer token");
        let _ = shutdown_tx.send(true);
        timeout(Duration::from_secs(3), server)
            .await
            .expect("server shutdown timeout")
            .expect("server task join")
            .expect("server shutdown");
    }

    #[tokio::test]
    async fn two_clients_exchange_relay_data_and_reconnect_with_authentication() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind fixture");
        let address = listener.local_addr().expect("fixture address");
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let server = tokio::spawn(run_server(listener, TEST_TOKEN.to_string(), shutdown_rx));
        let url = format!("ws://{address}");

        let mut first = connect_authenticated(&url).await;
        send_client_message(
            &mut first,
            SignalOutgoing::Join {
                room: "integration-room".to_string(),
            },
        )
        .await;
        let first_id = match next_server_message(&mut first).await {
            SignalIncoming::Welcome { peer_id } => peer_id,
            other => panic!("first event must be Welcome, received {other:?}"),
        };

        let mut second = connect_authenticated(&url).await;
        send_client_message(
            &mut second,
            SignalOutgoing::Join {
                room: "integration-room".to_string(),
            },
        )
        .await;
        let second_id = match next_server_message(&mut second).await {
            SignalIncoming::Welcome { peer_id } => peer_id,
            other => panic!("second client's first event must be Welcome, received {other:?}"),
        };
        assert_ne!(first_id, second_id);
        assert!(matches!(
            next_server_message(&mut second).await,
            SignalIncoming::PeerJoined { peer_id } if peer_id == first_id
        ));
        assert!(matches!(
            next_server_message(&mut first).await,
            SignalIncoming::PeerJoined { peer_id } if peer_id == second_id
        ));

        send_client_message(
            &mut first,
            SignalOutgoing::Signal {
                to: second_id,
                data: SignalData::RelayData {
                    channel: symtropy_net::signaling::SignalChannel::Reliable,
                    payload: b"hello".to_vec(),
                },
            },
        )
        .await;
        assert!(matches!(
            next_server_message(&mut second).await,
            SignalIncoming::Signal {
                from,
                data: SignalData::RelayData {
                    channel: symtropy_net::signaling::SignalChannel::Reliable,
                    payload,
                },
            } if from == first_id && payload == b"hello"
        ));

        send_client_message(
            &mut second,
            SignalOutgoing::Signal {
                to: first_id,
                data: SignalData::RelayData {
                    channel: symtropy_net::signaling::SignalChannel::Unreliable,
                    payload: b"world".to_vec(),
                },
            },
        )
        .await;
        assert!(matches!(
            next_server_message(&mut first).await,
            SignalIncoming::Signal {
                from,
                data: SignalData::RelayData {
                    channel: symtropy_net::signaling::SignalChannel::Unreliable,
                    payload,
                },
            } if from == second_id && payload == b"world"
        ));

        second.close(None).await.expect("close second client");
        assert!(matches!(
            next_server_message(&mut first).await,
            SignalIncoming::PeerLeft { peer_id } if peer_id == second_id
        ));

        let mut reconnected = connect_authenticated(&url).await;
        send_client_message(
            &mut reconnected,
            SignalOutgoing::Join {
                room: "integration-room".to_string(),
            },
        )
        .await;
        let replacement_id = match next_server_message(&mut reconnected).await {
            SignalIncoming::Welcome { peer_id } => peer_id,
            other => panic!("reconnected client's first event must be Welcome, received {other:?}"),
        };
        assert_ne!(replacement_id, second_id);
        assert!(matches!(
            next_server_message(&mut reconnected).await,
            SignalIncoming::PeerJoined { peer_id } if peer_id == first_id
        ));
        assert!(matches!(
            next_server_message(&mut first).await,
            SignalIncoming::PeerJoined { peer_id } if peer_id == replacement_id
        ));

        reconnected.close(None).await.expect("close reconnected client");
        first.close(None).await.expect("close first client");
        let _ = shutdown_tx.send(true);
        timeout(Duration::from_secs(3), server)
            .await
            .expect("server shutdown timeout")
            .expect("server task join")
            .expect("server shutdown");
    }
}
