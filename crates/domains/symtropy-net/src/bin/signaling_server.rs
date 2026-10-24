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
    collections::{HashMap, HashSet, VecDeque},
    env, io,
    net::SocketAddr,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use futures_util::{SinkExt, StreamExt};
use symtropy_net::signaling::{
    MAX_SIGNAL_CONTROL_BYTES, MAX_SIGNALING_MESSAGE_BYTES, SignalData, SignalIncoming,
    SignalOutgoing, SignalingClient, SignalingEvent, parse_incoming, serialized_outgoing,
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
const MAX_CLIENTS: usize = 16;
const MAX_ROOMS: usize = 16;
const MAX_ROOM_PEERS: usize = 16;
const MAX_ROOM_ID_BYTES: usize = 128;
/// Shared budget for queued/in-flight game-data envelopes across every client.
const MAX_TOTAL_OUTBOUND_DATA_BYTES: usize = MAX_SIGNALING_MESSAGE_BYTES * 8;
/// Reserved global budget for non-membership protocol control envelopes.
const MAX_TOTAL_OUTBOUND_CONTROL_BYTES: usize = MAX_SIGNAL_CONTROL_BYTES * MAX_CLIENTS;
/// Separate reserve for Welcome/PeerJoined/PeerLeft so membership cleanup keeps
/// progressing even if data and SDP/ICE/error messages saturate their own budgets.
const MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES: usize = MAX_ROOM_PEERS * OUTBOUND_QUEUE_CAPACITY * 128;
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
    /// Shared budget for all queued/in-flight relay-data envelopes.
    global_data_bytes: Arc<Semaphore>,
    /// Shared budget for non-membership control envelopes.
    global_control_bytes: Arc<Semaphore>,
    /// Reserved shared budget for Welcome/PeerJoined/PeerLeft envelopes.
    global_membership_bytes: Arc<Semaphore>,
    /// Lets the room registry terminate a connection evicted for backpressure.
    shutdown_tx: watch::Sender<bool>,
}

struct QueuedFrame {
    frame: Message,
    _byte_permit: Option<OwnedSemaphorePermit>,
    _global_byte_permit: Option<OwnedSemaphorePermit>,
}

/// Queue reservation failures are typed so connection lifecycle decisions never depend
/// on error-message wording.
#[derive(Debug, PartialEq, Eq)]
enum QueueError {
    Serialize(String),
    InvalidWire(String),
    AccountingOverflow,
    RecipientByteBudgetFull,
    AggregateDataBudgetFull,
    AggregateMembershipBudgetFull,
    AggregateControlBudgetFull,
    RecipientQueueFull,
}

impl QueueError {
    fn is_recipient_backpressure(&self) -> bool {
        matches!(
            self,
            Self::RecipientByteBudgetFull | Self::RecipientQueueFull
        )
    }
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(error) => write!(f, "failed to serialize server message: {error}"),
            Self::InvalidWire(error) => write!(f, "server message violates wire limits: {error}"),
            Self::AccountingOverflow => {
                f.write_str("outbound message byte count exceeds accounting range")
            }
            Self::RecipientByteBudgetFull => f.write_str("recipient outbound byte budget is full"),
            Self::AggregateDataBudgetFull => {
                f.write_str("server aggregate outbound data budget is full")
            }
            Self::AggregateMembershipBudgetFull => {
                f.write_str("server aggregate outbound membership budget is full")
            }
            Self::AggregateControlBudgetFull => {
                f.write_str("server aggregate outbound control budget is full")
            }
            Self::RecipientQueueFull => {
                f.write_str("recipient outbound message-count queue is full")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum RoomForwardError {
    SenderNotMember,
    RecipientNotMember,
    RegistryUnavailable,
    Queue(QueueError),
}

/// Reserves both count capacity and byte capacity before changing any room state.
/// The reservation can be committed synchronously after all recipients pass preflight.
struct PreparedFrame {
    slot: mpsc::OwnedPermit<QueuedFrame>,
    frame: Message,
    byte_permit: Option<OwnedSemaphorePermit>,
    global_byte_permit: Option<OwnedSemaphorePermit>,
}

impl PreparedFrame {
    fn commit(self) {
        self.slot.send(QueuedFrame {
            frame: self.frame,
            _byte_permit: self.byte_permit,
            _global_byte_permit: self.global_byte_permit,
        });
    }
}

/// Per-connection rolling one-second limiter. Because successful events are bounded
/// by `MAX_MESSAGES_PER_SECOND`, the timestamp deque is bounded independently of
/// attacker-supplied payload sizes.
struct RateWindow {
    events: VecDeque<(Instant, usize)>,
    bytes: usize,
}

impl RateWindow {
    fn new() -> Self {
        Self {
            events: VecDeque::new(),
            bytes: 0,
        }
    }

    fn admit(&mut self, bytes: usize) -> bool {
        self.admit_at(Instant::now(), bytes)
    }

    fn admit_at(&mut self, now: Instant, bytes: usize) -> bool {
        const WINDOW: Duration = Duration::from_secs(1);

        while let Some((timestamp, expired_bytes)) = self.events.front().copied() {
            if now.saturating_duration_since(timestamp) < WINDOW {
                break;
            }
            self.events.pop_front();
            self.bytes = self.bytes.saturating_sub(expired_bytes);
        }

        if self.events.len() >= MAX_MESSAGES_PER_SECOND
            || self.bytes.saturating_add(bytes) > MAX_BYTES_PER_SECOND
        {
            return false;
        }

        self.events.push_back((now, bytes));
        self.bytes += bytes;
        true
    }
}

/// Return payload bytes for rate accounting for every client-originated frame type.
/// Control frames consume message budget too; otherwise Ping/Pong traffic could avoid
/// the per-connection rate limit applied to JSON text messages.
fn inbound_frame_payload_bytes(message: &Message) -> usize {
    match message {
        Message::Text(text) => text.len(),
        Message::Binary(payload) | Message::Ping(payload) | Message::Pong(payload) => payload.len(),
        Message::Close(_) => 0,
        _ => 0,
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
        let bind = env::var("SYMTROPY_RELAY_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_string());
        let allow_insecure_bind = env::var("SYMTROPY_RELAY_ALLOW_INSECURE_BIND")
            .ok()
            .as_deref()
            == Some("1");
        Self::from_values(token, &bind, allow_insecure_bind)
    }

    fn from_values(token: String, bind: &str, allow_insecure_bind: bool) -> Result<Self, String> {
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

        let bind: SocketAddr = bind.parse().map_err(|_| {
            "SYMTROPY_RELAY_BIND must be a socket address such as 127.0.0.1:9001".to_string()
        })?;
        if !bind.ip().is_loopback() && !allow_insecure_bind {
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
    let global_data_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_DATA_BYTES));
    let global_control_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_CONTROL_BYTES));
    let global_membership_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES));
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
                let global_data_bytes = global_data_bytes.clone();
                let global_control_bytes = global_control_bytes.clone();
                let global_membership_bytes = global_membership_bytes.clone();
                clients.spawn(async move {
                    let _client_permit = permit;
                    if let Err(error) = handle_connection(
                        stream,
                        token,
                        rooms,
                        next_peer_id,
                        global_data_bytes,
                        global_control_bytes,
                        global_membership_bytes,
                    )
                    .await
                    {
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

    let command: SignalOutgoing = serde_json::from_str(text)
        .map_err(|_| "invalid or unsupported signaling JSON".to_string())?;

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
    if room.is_empty() || room.len() > MAX_ROOM_ID_BYTES || room.chars().any(char::is_control) {
        return Err(format!(
            "room ID must contain 1..={MAX_ROOM_ID_BYTES} UTF-8 bytes and no control characters"
        ));
    }
    Ok(())
}

fn prepare_message(
    peer: &PeerHandle,
    message: &SignalIncoming,
) -> Result<PreparedFrame, QueueError> {
    let text =
        serde_json::to_string(message).map_err(|error| QueueError::Serialize(error.to_string()))?;
    parse_incoming(&text).map_err(|error| QueueError::InvalidWire(error.to_string()))?;

    let charged_bytes = u32::try_from(text.len()).map_err(|_| QueueError::AccountingOverflow)?;
    let byte_permit = peer
        .queued_bytes
        .clone()
        .try_acquire_many_owned(charged_bytes)
        .map_err(|_| QueueError::RecipientByteBudgetFull)?;
    let is_relay_data = matches!(
        message,
        &SignalIncoming::Signal {
            data: SignalData::RelayData { .. },
            ..
        }
    );
    let is_membership = matches!(
        message,
        &SignalIncoming::Welcome { .. }
            | &SignalIncoming::PeerJoined { .. }
            | &SignalIncoming::PeerLeft { .. }
    );
    let (global_budget, budget_error) = if is_relay_data {
        (&peer.global_data_bytes, QueueError::AggregateDataBudgetFull)
    } else if is_membership {
        (
            &peer.global_membership_bytes,
            QueueError::AggregateMembershipBudgetFull,
        )
    } else {
        (
            &peer.global_control_bytes,
            QueueError::AggregateControlBudgetFull,
        )
    };
    let global_byte_permit = global_budget
        .clone()
        .try_acquire_many_owned(charged_bytes)
        .map_err(|_| budget_error)?;
    let slot = peer
        .tx
        .clone()
        .try_reserve_owned()
        .map_err(|_| QueueError::RecipientQueueFull)?;

    Ok(PreparedFrame {
        slot,
        frame: Message::Text(text),
        byte_permit: Some(byte_permit),
        global_byte_permit: Some(global_byte_permit),
    })
}

fn enqueue_message(peer: &PeerHandle, message: &SignalIncoming) -> Result<(), QueueError> {
    prepare_message(peer, message)?.commit();
    Ok(())
}

/// Test-only primitive for queue/backpressure behavior. Production forwarding must use
/// `forward_room_message` so delivery is serialized with membership changes.
#[cfg(test)]
fn forward_message(recipient: &PeerHandle, message: &SignalIncoming) -> Result<(), QueueError> {
    match enqueue_message(recipient, message) {
        Ok(()) => Ok(()),
        Err(error) => {
            // Evict only when the recipient's own bounded mailbox is saturated.
            // Shared aggregate budgets are server-wide backpressure, not peer fault.
            if error.is_recipient_backpressure() {
                let _ = recipient.shutdown_tx.send(true);
            }
            Err(error)
        }
    }
}

/// Commits a prepared data frame only while both endpoints are still members of the
/// room. The room lock makes this commit linearizable with membership removal.
fn commit_room_forward(
    rooms: &SharedRooms,
    room_id: &str,
    sender_id: u64,
    recipient_id: u64,
    prepared: PreparedFrame,
) -> Result<(), RoomForwardError> {
    let state = rooms
        .lock()
        .map_err(|_| RoomForwardError::RegistryUnavailable)?;
    let Some(room) = state.rooms.get(room_id) else {
        return Err(RoomForwardError::SenderNotMember);
    };
    if !room.contains_key(&sender_id) {
        return Err(RoomForwardError::SenderNotMember);
    }
    if !room.contains_key(&recipient_id) {
        return Err(RoomForwardError::RecipientNotMember);
    }

    prepared.commit();
    Ok(())
}

/// Reserves the target's outbound capacity outside the registry lock, then
/// revalidates membership while holding the lock for the actual queue commit.
/// A leave that wins the lock first therefore cannot be followed by stale data.
fn forward_room_message(
    rooms: &SharedRooms,
    room_id: &str,
    sender_id: u64,
    recipient_id: u64,
    message: &SignalIncoming,
) -> Result<(), RoomForwardError> {
    let recipient = {
        let state = rooms
            .lock()
            .map_err(|_| RoomForwardError::RegistryUnavailable)?;
        let Some(room) = state.rooms.get(room_id) else {
            return Err(RoomForwardError::SenderNotMember);
        };
        if !room.contains_key(&sender_id) {
            return Err(RoomForwardError::SenderNotMember);
        }
        room.get(&recipient_id)
            .cloned()
            .ok_or(RoomForwardError::RecipientNotMember)?
    };

    let prepared = match prepare_message(&recipient, message) {
        Ok(prepared) => prepared,
        Err(error) => {
            if error.is_recipient_backpressure() {
                let state = rooms
                    .lock()
                    .map_err(|_| RoomForwardError::RegistryUnavailable)?;
                if let Some(current_recipient) = state
                    .rooms
                    .get(room_id)
                    .filter(|room| room.contains_key(&sender_id))
                    .and_then(|room| room.get(&recipient_id))
                {
                    let _ = current_recipient.shutdown_tx.send(true);
                }
            }
            return Err(RoomForwardError::Queue(error));
        }
    };

    commit_room_forward(rooms, room_id, sender_id, recipient_id, prepared)
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
        _global_byte_permit: None,
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
        .map(|room| room.iter().map(|(id, peer)| (*id, peer.clone())).collect())
        .unwrap_or_default();

    if existing.len() >= MAX_ROOM_PEERS {
        return Err(format!("room peer limit ({MAX_ROOM_PEERS}) reached"));
    }
    if !state.rooms.contains_key(room_id) && state.rooms.len() >= MAX_ROOMS {
        return Err(format!("room limit ({MAX_ROOMS}) reached"));
    }

    let mut staged = Vec::with_capacity(1 + existing.len() * 2);
    staged.push(
        prepare_message(new_peer, &SignalIncoming::Welcome { peer_id })
            .map_err(|error| error.to_string())?,
    );

    for (existing_id, existing_peer) in &existing {
        staged.push(
            prepare_message(
                new_peer,
                &SignalIncoming::PeerJoined {
                    peer_id: *existing_id,
                },
            )
            .map_err(|error| error.to_string())?,
        );
        staged.push(
            prepare_message(existing_peer, &SignalIncoming::PeerJoined { peer_id })
                .map_err(|error| error.to_string())?,
        );
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
/// bounded batch. A recipient-local queue failure evicts that slow peer and retries.
/// A shared/serialization failure closes the room instead of blaming healthy peers.
fn remove_peer_and_notify(rooms: &SharedRooms, room_id: &str, initial_peer_id: u64) {
    let Ok(mut state) = rooms.lock() else {
        eprintln!("relay room registry lock poisoned while removing peer");
        return;
    };

    let removed_initial = state
        .rooms
        .get_mut(room_id)
        .is_some_and(|room| room.remove(&initial_peer_id).is_some());
    if !removed_initial {
        let empty = state.rooms.get(room_id).is_some_and(|room| room.is_empty());
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
            .iter()
            .filter(|(id, _)| !removed_set.contains(id))
            .map(|(id, peer)| (*id, peer.clone()))
            .collect();

        let mut staged = Vec::new();
        let mut slow_peers = HashSet::new();
        let mut fatal_error = None;
        for removed_id in &removed {
            for (survivor_id, survivor) in &survivors {
                match prepare_message(
                    survivor,
                    &SignalIncoming::PeerLeft {
                        peer_id: *removed_id,
                    },
                ) {
                    Ok(message) => staged.push(message),
                    Err(error) if error.is_recipient_backpressure() => {
                        slow_peers.insert(*survivor_id);
                    }
                    Err(error) => {
                        fatal_error.get_or_insert(error);
                    }
                }
            }
        }

        if let Some(error) = fatal_error {
            // Membership delivery is essential for keeping client and server state
            // aligned. If the shared reserve cannot carry the batch, terminate the
            // room rather than misclassifying every survivor as a slow peer.
            drop(staged);
            eprintln!(
                "relay room {room_id:?} cannot publish membership cleanup: {error}; closing room"
            );
            if let Some(room) = state.rooms.remove(room_id) {
                for (_, peer) in room {
                    let _ = peer.shutdown_tx.send(true);
                }
            }
            return;
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
            if let Some(slow_peer) = room.remove(&slow_id) {
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
        room.retain(|id, _| !removed_set.contains(id));
        room.is_empty()
    } else {
        return;
    };
    if empty {
        state.rooms.remove(room_id);
    }
}

// Tungstenite fixes this callback's error type to `ErrorResponse` by value. The
// protocol API cannot be boxed locally, so keep the Clippy exception scoped here.
#[allow(clippy::result_large_err)]
async fn handle_connection(
    stream: TcpStream,
    token: Arc<String>,
    rooms: SharedRooms,
    next_peer_id: Arc<AtomicU64>,
    global_data_bytes: Arc<Semaphore>,
    global_control_bytes: Arc<Semaphore>,
    global_membership_bytes: Arc<Semaphore>,
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
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| "server peer-ID space exhausted".to_string())?;

    let (out_tx, mut out_rx) = mpsc::channel::<QueuedFrame>(OUTBOUND_QUEUE_CAPACITY);
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let mut writer_shutdown_rx = shutdown_tx.subscribe();
    let peer = PeerHandle {
        tx: out_tx,
        queued_bytes: Arc::new(Semaphore::new(MAX_SIGNALING_MESSAGE_BYTES)),
        global_data_bytes,
        global_control_bytes,
        global_membership_bytes,
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
        loop {
            let queued = tokio::select! {
                shutdown_changed = writer_shutdown_rx.changed() => {
                    if shutdown_changed.is_err() || *writer_shutdown_rx.borrow() {
                        break;
                    }
                    continue;
                }
                queued = out_rx.recv() => queued,
            };
            let Some(queued) = queued else {
                break;
            };
            let QueuedFrame {
                frame,
                _byte_permit,
                _global_byte_permit,
            } = queued;
            if let Err(error) = sink.send(frame).await {
                drop(_byte_permit);
                drop(_global_byte_permit);
                return Err(format!("WebSocket write failed: {error}"));
            }
            // Keep both peer-local and global byte permits charged until the queued
            // frame is fully written, not merely until it leaves the mpsc queue.
            drop(_byte_permit);
            drop(_global_byte_permit);
        }
        sink.close()
            .await
            .map_err(|error| format!("WebSocket close failed: {error}"))?;
        Ok::<(), String>(())
    });

    let mut writer_finished = false;
    let mut rate = RateWindow::new();
    loop {
        tokio::select! {
            shutdown_changed = shutdown_rx.changed() => {
                if shutdown_changed.is_err() || *shutdown_rx.borrow() {
                    break;
                }
            }
            writer_result = &mut writer => {
                writer_finished = true;
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
                if !rate.admit(inbound_frame_payload_bytes(&message)) {
                    let _ = enqueue_message(&peer, &SignalIncoming::Error {
                        message: "per-connection message or byte rate limit exceeded".to_string(),
                    });
                    break;
                }
                match message {
                    Message::Text(text) => {
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
                                let forwarded = SignalIncoming::Signal { from: peer_id, data };
                                match forward_room_message(
                                    &rooms,
                                    &room_id,
                                    peer_id,
                                    to,
                                    &forwarded,
                                ) {
                                    Ok(()) => {}
                                    Err(RoomForwardError::SenderNotMember) => break,
                                    Err(RoomForwardError::RegistryUnavailable) => {
                                        let _ = enqueue_message(&peer, &SignalIncoming::Error {
                                            message: "relay room registry unavailable".to_string(),
                                        });
                                        break;
                                    }
                                    Err(RoomForwardError::RecipientNotMember) => {
                                        let _ = enqueue_message(&peer, &SignalIncoming::Error {
                                            message: format!("recipient {to} is not a member of room {room_id:?}"),
                                        });
                                    }
                                    Err(RoomForwardError::Queue(error)) => {
                                        let _ = enqueue_message(&peer, &SignalIncoming::Error {
                                            message: format!("message not queued for recipient {to}: {error}"),
                                        });
                                    }
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
    if !writer_finished && timeout(WRITER_DRAIN_TIMEOUT, &mut writer).await.is_err() {
        writer.abort();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use symtropy_net::transport::MAX_PEER_MESSAGE_BYTES;
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{client::IntoClientRequest, http::HeaderValue},
    };

    const TEST_TOKEN: &str = "test_token_that_is_long_enough_for_local_tests_123";

    async fn connect_authenticated(
        url: &str,
    ) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>> {
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
                Message::Text(text) => {
                    return parse_incoming(&text).expect("valid server envelope");
                }
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
        socket
            .send(Message::Text(text))
            .await
            .expect("send client message");
    }

    #[test]
    fn startup_config_rejects_bad_tokens_and_public_plaintext_binds() {
        assert!(ServerConfig::from_values("short".to_string(), "127.0.0.1:9001", false).is_err());
        assert!(
            ServerConfig::from_values(
                "invalid token that has enough length to fail only character validation"
                    .to_string(),
                "127.0.0.1:9001",
                false
            )
            .is_err()
        );
        assert!(ServerConfig::from_values(TEST_TOKEN.to_string(), "not-a-socket", false).is_err());
        assert!(
            ServerConfig::from_values(TEST_TOKEN.to_string(), "0.0.0.0:9001", false).is_err(),
            "non-loopback plaintext binds must fail closed by default"
        );
        assert!(
            ServerConfig::from_values(TEST_TOKEN.to_string(), "127.0.0.1:9001", false).is_ok(),
            "loopback should be the safe default"
        );
        assert!(
            ServerConfig::from_values(TEST_TOKEN.to_string(), "0.0.0.0:9001", true).is_ok(),
            "the dangerous bind override should only be explicit"
        );
    }

    #[test]
    fn bearer_token_match_rejects_missing_wrong_and_prefix_confusion() {
        assert!(token_matches(TEST_TOKEN, TEST_TOKEN));
        assert!(!token_matches("", TEST_TOKEN));
        assert!(!token_matches("x", TEST_TOKEN));
        assert!(!token_matches("Bearer test", TEST_TOKEN));
        assert!(!token_matches(
            "test_token_that_is_long_enough_for_local_tests_124",
            TEST_TOKEN
        ));
    }

    #[test]
    fn recipient_queue_saturation_evicts_the_slow_peer() {
        let (tx, mut rx) = mpsc::channel::<QueuedFrame>(OUTBOUND_QUEUE_CAPACITY);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let peer = PeerHandle {
            tx,
            queued_bytes: Arc::new(Semaphore::new(MAX_SIGNALING_MESSAGE_BYTES)),
            global_data_bytes: Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_DATA_BYTES)),
            global_control_bytes: Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_CONTROL_BYTES)),
            global_membership_bytes: Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES)),
            shutdown_tx,
        };

        for peer_id in 0..OUTBOUND_QUEUE_CAPACITY as u64 {
            prepare_message(&peer, &SignalIncoming::PeerJoined { peer_id })
                .expect("small membership event fits before count cap")
                .commit();
        }

        let error = forward_message(
            &peer,
            &SignalIncoming::Signal {
                from: 9,
                data: SignalData::RelayData {
                    channel: symtropy_net::signaling::SignalChannel::Reliable,
                    payload: b"overloaded".to_vec(),
                },
            },
        )
        .expect_err("full recipient queue must not silently accept another message");
        assert_eq!(error, QueueError::RecipientQueueFull);
        assert!(
            *shutdown_rx.borrow(),
            "saturated peer must receive a shutdown signal"
        );
        assert_eq!(rx.len(), OUTBOUND_QUEUE_CAPACITY);
        drop(rx);
        assert_eq!(
            peer.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES,
            "dropping queued messages releases all byte permits"
        );
    }

    #[tokio::test]
    async fn room_peer_and_room_count_limits_fail_closed() {
        let rooms: SharedRooms = Arc::new(Mutex::new(Rooms::default()));
        let mut peers = Vec::new();

        for peer_id in 1..=MAX_ROOM_PEERS as u64 {
            let (peer, rx) = test_peer_handle();
            join_room(&rooms, "capacity-room", peer_id, &peer)
                .expect("room must admit peers up to the inclusive limit");
            peers.push((peer, rx));
        }
        let (overflow_peer, overflow_rx) = test_peer_handle();
        let overflow = join_room(
            &rooms,
            "capacity-room",
            (MAX_ROOM_PEERS + 1) as u64,
            &overflow_peer,
        );
        assert!(
            matches!(overflow, Err(ref error) if error.contains("room peer limit")),
            "the peer cap must reject the next join"
        );

        let room_registry: SharedRooms = Arc::new(Mutex::new(Rooms::default()));
        let mut separate_rooms = Vec::new();
        for index in 0..MAX_ROOMS {
            let (peer, rx) = test_peer_handle();
            let room_name = format!("room-{index}");
            join_room(&room_registry, &room_name, 10_000 + index as u64, &peer)
                .expect("room registry must admit up to its room limit");
            separate_rooms.push((peer, rx));
        }
        let (overflow_room_peer, overflow_room_rx) = test_peer_handle();
        let room_overflow = join_room(
            &room_registry,
            "one-room-too-many",
            99_999,
            &overflow_room_peer,
        );
        assert!(
            matches!(room_overflow, Err(ref error) if error.contains("room limit")),
            "the active room cap must reject a new room"
        );

        drop((
            peers,
            overflow_rx,
            overflow_peer,
            separate_rooms,
            overflow_room_rx,
            overflow_room_peer,
        ));
    }

    #[test]
    fn client_parser_rejects_unknown_kinds_and_oversized_control_or_relay_data() {
        assert!(parse_client_message(r#"{"type":"unknown"}"#).is_err());
        assert!(parse_client_message(
            r#"{"type":"signal","to":2,"data":{"kind":"relay_data","channel":"reliable","payload":[256]}}"#
        ).is_err());

        let oversized_control = serde_json::to_string(&SignalOutgoing::Signal {
            to: 2,
            data: SignalData::Offer {
                sdp: "x".repeat(MAX_SIGNAL_CONTROL_BYTES),
            },
        })
        .expect("serialize oversized control input");
        assert!(oversized_control.len() > MAX_SIGNAL_CONTROL_BYTES);
        assert!(
            matches!(parse_client_message(&oversized_control), Err(ref error) if error.contains("control envelope")),
            "oversized control messages must be rejected independently of the relay allowance"
        );

        let oversized_data = serde_json::json!({
            "type": "signal",
            "to": 2,
            "data": {
                "kind": "relay_data",
                "channel": "reliable",
                "payload": vec![0; symtropy_net::transport::MAX_PEER_MESSAGE_BYTES + 1]
            }
        });
        let oversized_data =
            serde_json::to_string(&oversized_data).expect("serialize over-limit relay payload");
        assert!(oversized_data.len() < MAX_SIGNALING_MESSAGE_BYTES);
        assert!(
            matches!(parse_client_message(&oversized_data), Err(ref error) if error.contains("relay packet")),
            "binary payload cap must fail before the larger outer-envelope cap"
        );
    }

    #[test]
    fn room_ids_are_bounded_and_reject_control_characters() {
        assert!(validate_room_id("room-1").is_ok());
        assert!(validate_room_id(&"🚀".repeat(32)).is_ok());
        assert!(validate_room_id("").is_err());
        assert!(validate_room_id("room\nforged").is_err());
        assert!(validate_room_id(&"x".repeat(MAX_ROOM_ID_BYTES + 1)).is_err());
    }

    #[test]
    fn inbound_rate_accounting_includes_websocket_control_frames() {
        let ping = Message::Ping(vec![0; 125]);
        let pong = Message::Pong(vec![0; 125]);
        let text = Message::Text("{}".to_string());

        assert_eq!(inbound_frame_payload_bytes(&ping), 125);
        assert_eq!(inbound_frame_payload_bytes(&pong), 125);
        assert_eq!(inbound_frame_payload_bytes(&text), 2);

        let mut limiter = RateWindow::new();
        for _ in 0..MAX_MESSAGES_PER_SECOND {
            assert!(
                limiter.admit(inbound_frame_payload_bytes(&ping)),
                "Ping frames must consume the same message budget as text frames"
            );
        }
        assert!(
            !limiter.admit(inbound_frame_payload_bytes(&pong)),
            "a Pong frame must not bypass the per-connection message ceiling"
        );
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

    #[test]
    fn rate_window_prevents_boundary_bursts() {
        let start = Instant::now();
        let before_boundary = start + Duration::from_millis(999);
        let boundary = start + Duration::from_secs(1);

        let mut message_limiter = RateWindow::new();
        assert!(message_limiter.admit_at(start, 0));
        for _ in 1..MAX_MESSAGES_PER_SECOND {
            assert!(message_limiter.admit_at(before_boundary, 0));
        }
        assert!(
            message_limiter.admit_at(boundary, 0),
            "only the event at the one-second boundary has expired"
        );
        assert!(
            !message_limiter.admit_at(boundary, 0),
            "events from 999 ms ago must still count against the message ceiling"
        );

        let mut byte_limiter = RateWindow::new();
        assert!(byte_limiter.admit_at(start, 0));
        assert!(byte_limiter.admit_at(before_boundary, MAX_BYTES_PER_SECOND));
        assert!(
            !byte_limiter.admit_at(boundary, 1),
            "bytes admitted 999 ms ago must remain charged for the full rolling window"
        );
    }

    fn test_peer_handle() -> (PeerHandle, mpsc::Receiver<QueuedFrame>) {
        let (tx, rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);
        test_peer_handle_with_global(
            tx,
            rx,
            shutdown_tx,
            Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_DATA_BYTES)),
            Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_CONTROL_BYTES)),
            Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES)),
        )
    }

    fn test_peer_handle_with_global(
        tx: mpsc::Sender<QueuedFrame>,
        rx: mpsc::Receiver<QueuedFrame>,
        shutdown_tx: watch::Sender<bool>,
        global_data_bytes: Arc<Semaphore>,
        global_control_bytes: Arc<Semaphore>,
        global_membership_bytes: Arc<Semaphore>,
    ) -> (PeerHandle, mpsc::Receiver<QueuedFrame>) {
        (
            PeerHandle {
                tx,
                queued_bytes: Arc::new(Semaphore::new(MAX_SIGNALING_MESSAGE_BYTES)),
                global_data_bytes,
                global_control_bytes,
                global_membership_bytes,
                shutdown_tx,
            },
            rx,
        )
    }

    #[test]
    fn aggregate_outbound_budgets_isolate_data_control_and_membership_capacity() {
        let data_message = SignalIncoming::Signal {
            from: 1,
            data: SignalData::RelayData {
                channel: symtropy_net::signaling::SignalChannel::Reliable,
                payload: b"data".to_vec(),
            },
        };
        let data_charge = serde_json::to_string(&data_message)
            .expect("serialize relay-data test envelope")
            .len();
        let control_message = SignalIncoming::Error {
            message: "test-error".to_string(),
        };
        let control_charge = serde_json::to_string(&control_message)
            .expect("serialize error envelope")
            .len();
        let membership_message = SignalIncoming::PeerLeft { peer_id: 1 };
        let membership_charge = serde_json::to_string(&membership_message)
            .expect("serialize membership envelope")
            .len();

        let global_data_bytes = Arc::new(Semaphore::new(data_charge));
        let global_control_bytes = Arc::new(Semaphore::new(control_charge));
        let global_membership_bytes = Arc::new(Semaphore::new(membership_charge));

        let (tx1, rx1) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (shutdown_tx1, _shutdown_rx1) = watch::channel(false);
        let (peer1, rx1) = test_peer_handle_with_global(
            tx1,
            rx1,
            shutdown_tx1,
            global_data_bytes.clone(),
            global_control_bytes.clone(),
            global_membership_bytes.clone(),
        );
        prepare_message(&peer1, &data_message)
            .expect("first peer uses the exact shared game-data budget")
            .commit();
        assert_eq!(global_data_bytes.available_permits(), 0);
        prepare_message(&peer1, &control_message)
            .expect("control envelope uses the separate reserved budget")
            .commit();
        assert_eq!(global_control_bytes.available_permits(), 0);

        let (tx2, rx2) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (shutdown_tx2, _shutdown_rx2) = watch::channel(false);
        let (peer2, rx2) = test_peer_handle_with_global(
            tx2,
            rx2,
            shutdown_tx2,
            global_data_bytes.clone(),
            global_control_bytes.clone(),
            global_membership_bytes.clone(),
        );
        let blocked_data = prepare_message(
            &peer2,
            &SignalIncoming::Signal {
                from: 2,
                data: SignalData::RelayData {
                    channel: symtropy_net::signaling::SignalChannel::Reliable,
                    payload: b"data".to_vec(),
                },
            },
        );
        assert!(
            matches!(blocked_data, Err(QueueError::AggregateDataBudgetFull)),
            "the global relay-data budget must span all peer mailboxes"
        );
        let blocked_control = prepare_message(&peer2, &control_message);
        assert!(
            matches!(blocked_control, Err(QueueError::AggregateControlBudgetFull)),
            "the non-membership control budget must also be aggregate-bounded"
        );

        // Membership cleanup remains queueable when both data and general control
        // quotas are exhausted. This avoids a full queue causing a disconnect cascade.
        prepare_message(&peer2, &membership_message)
            .expect("reserved global membership capacity remains available")
            .commit();
        assert_eq!(global_membership_bytes.available_permits(), 0);

        drop(rx1);
        assert_eq!(global_data_bytes.available_permits(), data_charge);
        assert_eq!(global_control_bytes.available_permits(), control_charge);
        prepare_message(
            &peer2,
            &SignalIncoming::Signal {
                from: 2,
                data: SignalData::RelayData {
                    channel: symtropy_net::signaling::SignalChannel::Reliable,
                    payload: b"data".to_vec(),
                },
            },
        )
        .expect("draining first peer returns the shared game-data capacity")
        .commit();
        assert_eq!(global_data_bytes.available_permits(), 0);

        drop(rx2);
        assert_eq!(global_data_bytes.available_permits(), data_charge);
        assert_eq!(global_control_bytes.available_permits(), control_charge);
        assert_eq!(
            global_membership_bytes.available_permits(),
            membership_charge,
            "dropping queued membership messages returns reserved permits"
        );
    }

    #[test]
    fn aggregate_data_backpressure_does_not_evict_recipient_or_block_membership() {
        let (tx, rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let global_data_bytes = Arc::new(Semaphore::new(0));
        let global_control_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_CONTROL_BYTES));
        let global_membership_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES));
        let (peer, mut outbound) = test_peer_handle_with_global(
            tx,
            rx,
            shutdown_tx,
            global_data_bytes.clone(),
            global_control_bytes,
            global_membership_bytes.clone(),
        );

        let data_message = SignalIncoming::Signal {
            from: 1,
            data: SignalData::RelayData {
                channel: symtropy_net::signaling::SignalChannel::Reliable,
                payload: b"temporary aggregate pressure".to_vec(),
            },
        };
        let error = forward_message(&peer, &data_message)
            .expect_err("an exhausted aggregate data budget must reject this send");
        assert_eq!(error, QueueError::AggregateDataBudgetFull);
        assert!(
            !*shutdown_rx.borrow(),
            "shared data-budget exhaustion must not evict a healthy recipient"
        );
        assert_eq!(
            peer.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES,
            "the local byte reservation must roll back when the shared reservation fails"
        );
        assert_eq!(global_data_bytes.available_permits(), 0);
        assert!(
            outbound.try_recv().is_err(),
            "a rejected relay envelope must not enter the recipient mailbox"
        );

        forward_message(&peer, &SignalIncoming::PeerLeft { peer_id: 99 })
            .expect("reserved membership capacity remains usable under data pressure");
        let membership = outbound
            .try_recv()
            .expect("membership notification should be queued");
        drop(membership);
        assert!(
            !*shutdown_rx.borrow(),
            "successful membership delivery must leave the recipient connected"
        );
        assert_eq!(
            peer.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES,
            "dropping the queued membership frame must release local permits"
        );
        assert_eq!(
            global_membership_bytes.available_permits(),
            MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES,
            "dropping the queued membership frame must release reserved global permits"
        );
    }

    #[test]
    fn peer_left_cleanup_survives_exhausted_data_and_general_control_budgets() {
        let rooms: SharedRooms = Arc::new(Mutex::new(Rooms::default()));
        let global_data_bytes = Arc::new(Semaphore::new(0));
        let global_control_bytes = Arc::new(Semaphore::new(0));
        let global_membership_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES));

        let (peer1_tx, peer1_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (peer1, mut rx1) = test_peer_handle_with_global(
            peer1_tx,
            peer1_rx,
            watch::channel(false).0,
            global_data_bytes.clone(),
            global_control_bytes.clone(),
            global_membership_bytes.clone(),
        );
        let (peer2_tx, peer2_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (peer2, mut rx2) = test_peer_handle_with_global(
            peer2_tx,
            peer2_rx,
            watch::channel(false).0,
            global_data_bytes,
            global_control_bytes,
            global_membership_bytes,
        );

        join_room(&rooms, "room", 1, &peer1).expect("admit first peer");
        join_room(&rooms, "room", 2, &peer2).expect("admit second peer");

        // Drain join notifications so this checks only whether the dedicated
        // membership budget can carry PeerLeft while both other budgets are zero.
        while let Ok(frame) = rx1.try_recv() {
            drop(frame);
        }
        while let Ok(frame) = rx2.try_recv() {
            drop(frame);
        }
        remove_peer_and_notify(&rooms, "room", 2);

        assert!(matches!(
            rx1.try_recv().expect("remaining peer must get PeerLeft").frame,
            Message::Text(text)
                if matches!(
                    parse_incoming(&text).expect("valid PeerLeft envelope"),
                    SignalIncoming::PeerLeft { peer_id: 2 }
                )
        ));
        let state = rooms.lock().expect("room registry lock");
        let members = state.rooms.get("room").expect("remaining room");
        assert_eq!(members.len(), 1);
        assert!(members.contains_key(&1));
        assert!(!members.contains_key(&2));
    }

    #[test]
    fn failed_room_join_rolls_back_every_staged_reservation() {
        let rooms: SharedRooms = Arc::new(Mutex::new(Rooms::default()));
        let welcome_bytes = serde_json::to_string(&SignalIncoming::Welcome { peer_id: 2 })
            .expect("serialize staged Welcome")
            .len();
        let joined_bytes = serde_json::to_string(&SignalIncoming::PeerJoined { peer_id: 1 })
            .expect("serialize staged PeerJoined")
            .len();
        // Exactly enough for Welcome and the new peer's first PeerJoined event,
        // so the following reservation for the existing peer fails deterministically.
        let initial_membership_budget = welcome_bytes + joined_bytes;
        let global_data_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_DATA_BYTES));
        let global_control_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_CONTROL_BYTES));
        let global_membership_bytes = Arc::new(Semaphore::new(initial_membership_budget));

        let (existing_tx, existing_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (existing_shutdown_tx, _existing_shutdown_rx) = watch::channel(false);
        let (existing, mut existing_outbound) = test_peer_handle_with_global(
            existing_tx,
            existing_rx,
            existing_shutdown_tx,
            global_data_bytes.clone(),
            global_control_bytes.clone(),
            global_membership_bytes.clone(),
        );
        let (new_tx, new_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (new_shutdown_tx, _new_shutdown_rx) = watch::channel(false);
        let (new_peer, mut new_outbound) = test_peer_handle_with_global(
            new_tx,
            new_rx,
            new_shutdown_tx,
            global_data_bytes,
            global_control_bytes,
            global_membership_bytes.clone(),
        );

        {
            let mut state = rooms.lock().expect("room registry lock");
            state
                .rooms
                .entry("room".to_string())
                .or_default()
                .insert(1, existing.clone());
        }

        let result = join_room(&rooms, "room", 2, &new_peer);
        assert!(
            matches!(
                &result,
                Err(error) if error.contains("aggregate outbound membership budget is full")
            ),
            "third staged reservation should fail on the shared membership budget: {result:?}"
        );

        {
            let state = rooms.lock().expect("room registry lock");
            let members = state
                .rooms
                .get("room")
                .expect("existing room survives failed admission");
            assert_eq!(members.len(), 1);
            assert!(members.contains_key(&1));
            assert!(
                !members.contains_key(&2),
                "failed join must never become visible"
            );
        }
        assert!(existing_outbound.try_recv().is_err());
        assert!(new_outbound.try_recv().is_err());
        assert_eq!(
            existing.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES,
            "failed admission must release all existing-peer byte reservations"
        );
        assert_eq!(
            new_peer.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES,
            "failed admission must release all joining-peer byte reservations"
        );
        assert_eq!(
            global_membership_bytes.available_permits(),
            initial_membership_budget,
            "all staged global membership permits must roll back after a later reservation fails"
        );
    }

    #[test]
    fn concurrent_join_and_leave_never_leave_stale_membership() {
        let rooms: SharedRooms = Arc::new(Mutex::new(Rooms::default()));
        let global_data_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_DATA_BYTES));
        let global_control_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_CONTROL_BYTES));
        let global_membership_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES));

        let (departing_tx, departing_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (departing_shutdown_tx, _departing_shutdown_rx) = watch::channel(false);
        let (departing, mut departing_outbound) = test_peer_handle_with_global(
            departing_tx,
            departing_rx,
            departing_shutdown_tx,
            global_data_bytes.clone(),
            global_control_bytes.clone(),
            global_membership_bytes.clone(),
        );
        let (joining_tx, joining_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (joining_shutdown_tx, _joining_shutdown_rx) = watch::channel(false);
        let (joining, mut joining_outbound) = test_peer_handle_with_global(
            joining_tx,
            joining_rx,
            joining_shutdown_tx,
            global_data_bytes,
            global_control_bytes,
            global_membership_bytes.clone(),
        );
        {
            let mut state = rooms.lock().expect("room registry lock");
            state
                .rooms
                .entry("room".to_string())
                .or_default()
                .insert(1, departing.clone());
        }

        let start = Arc::new(Barrier::new(3));
        let join_rooms = rooms.clone();
        let join_start = start.clone();
        let join_peer = joining.clone();
        let join_thread = std::thread::spawn(move || {
            join_start.wait();
            join_room(&join_rooms, "room", 2, &join_peer)
        });
        let leave_rooms = rooms.clone();
        let leave_start = start.clone();
        let leave_thread = std::thread::spawn(move || {
            leave_start.wait();
            remove_peer_and_notify(&leave_rooms, "room", 1);
        });
        start.wait();

        let join_result = join_thread.join().expect("join thread must not panic");
        assert!(
            join_result.is_ok(),
            "serialized join must complete regardless of whether leave takes the room lock first: {join_result:?}"
        );
        leave_thread.join().expect("leave thread must not panic");

        {
            let state = rooms.lock().expect("room registry lock");
            let members = state
                .rooms
                .get("room")
                .expect("joining peer's room remains");
            assert_eq!(members.len(), 1);
            assert!(members.contains_key(&2));
            assert!(!members.contains_key(&1));
        }

        let mut joining_events = Vec::new();
        while let Ok(frame) = joining_outbound.try_recv() {
            match frame.frame {
                Message::Text(text) => {
                    joining_events.push(parse_incoming(&text).expect("valid membership frame"));
                }
                other => panic!("expected a text membership frame, received {other:?}"),
            }
        }
        assert!(
            matches!(
                joining_events.first(),
                Some(SignalIncoming::Welcome { peer_id: 2 })
            ),
            "joining peer must always observe Welcome first"
        );
        let saw_old_peer_join = joining_events
            .iter()
            .position(|event| matches!(event, SignalIncoming::PeerJoined { peer_id: 1 }));
        let saw_old_peer_leave = joining_events
            .iter()
            .position(|event| matches!(event, SignalIncoming::PeerLeft { peer_id: 1 }));
        assert_eq!(
            saw_old_peer_join.is_some(),
            saw_old_peer_leave.is_some(),
            "if the joining peer saw peer 1 join, it must also see peer 1 leave"
        );
        if let (Some(joined), Some(left)) = (saw_old_peer_join, saw_old_peer_leave) {
            assert!(
                joined < left,
                "PeerJoined must precede PeerLeft for the same transition"
            );
        }
        assert!(
            joining_events.len() == 1 || joining_events.len() == 3,
            "unexpected membership event sequence: {joining_events:?}"
        );

        let mut departing_saw_join = false;
        while let Ok(frame) = departing_outbound.try_recv() {
            if let Message::Text(text) = frame.frame {
                departing_saw_join |= matches!(
                    parse_incoming(&text).expect("valid membership frame"),
                    SignalIncoming::PeerJoined { peer_id: 2 }
                );
            }
        }
        assert_eq!(
            departing_saw_join,
            saw_old_peer_join.is_some(),
            "the old peer and new peer must agree about whether the join preceded the leave"
        );
        assert_eq!(
            joining.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES
        );
        assert_eq!(
            departing.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES
        );
        assert_eq!(
            global_membership_bytes.available_permits(),
            MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES,
            "draining all race-generated events must release every shared membership permit"
        );
    }

    #[test]
    fn completed_join_before_leave_publishes_ordered_membership() {
        let rooms: SharedRooms = Arc::new(Mutex::new(Rooms::default()));
        let global_data_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_DATA_BYTES));
        let global_control_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_CONTROL_BYTES));
        let global_membership_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES));

        let (departing_tx, departing_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (departing_shutdown_tx, _departing_shutdown_rx) = watch::channel(false);
        let (departing, mut departing_outbound) = test_peer_handle_with_global(
            departing_tx,
            departing_rx,
            departing_shutdown_tx,
            global_data_bytes.clone(),
            global_control_bytes.clone(),
            global_membership_bytes.clone(),
        );
        let (joining_tx, joining_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (joining_shutdown_tx, _joining_shutdown_rx) = watch::channel(false);
        let (joining, mut joining_outbound) = test_peer_handle_with_global(
            joining_tx,
            joining_rx,
            joining_shutdown_tx,
            global_data_bytes,
            global_control_bytes,
            global_membership_bytes.clone(),
        );
        {
            let mut state = rooms.lock().expect("room registry lock");
            state
                .rooms
                .entry("room".to_string())
                .or_default()
                .insert(1, departing.clone());
        }

        join_room(&rooms, "room", 2, &joining).expect("join publishes both membership views");
        remove_peer_and_notify(&rooms, "room", 1);

        {
            let state = rooms.lock().expect("room registry lock");
            let members = state
                .rooms
                .get("room")
                .expect("room remains for the joining peer");
            assert_eq!(members.len(), 1);
            assert!(members.contains_key(&2));
            assert!(!members.contains_key(&1));
        }

        let mut joining_events = Vec::new();
        while let Ok(frame) = joining_outbound.try_recv() {
            match frame.frame {
                Message::Text(text) => {
                    joining_events.push(parse_incoming(&text).expect("valid membership frame"));
                }
                other => panic!("expected text membership event, got {other:?}"),
            }
        }
        assert!(
            matches!(
                joining_events.as_slice(),
                [
                    SignalIncoming::Welcome { peer_id: 2 },
                    SignalIncoming::PeerJoined { peer_id: 1 },
                    SignalIncoming::PeerLeft { peer_id: 1 }
                ]
            ),
            "join-first history must be Welcome, PeerJoined, PeerLeft; got {joining_events:?}"
        );

        let departed_event = departing_outbound
            .try_recv()
            .expect("old peer must receive PeerJoined before it leaves");
        assert!(matches!(
            departed_event.frame,
            Message::Text(ref text)
                if matches!(
                    parse_incoming(text).expect("valid PeerJoined"),
                    SignalIncoming::PeerJoined { peer_id: 2 }
                )
        ));
        assert!(departing_outbound.try_recv().is_err());
        drop(departed_event);

        assert_eq!(
            joining.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES
        );
        assert_eq!(
            departing.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES
        );
        assert_eq!(
            global_membership_bytes.available_permits(),
            MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES,
            "draining ordered join/leave events must recover shared membership capacity"
        );
    }

    #[test]
    fn prepared_relay_frame_is_discarded_if_recipient_leaves_before_commit() {
        let rooms: SharedRooms = Arc::new(Mutex::new(Rooms::default()));
        let global_data_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_DATA_BYTES));
        let global_control_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_CONTROL_BYTES));
        let global_membership_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES));

        let (sender_tx, sender_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (sender_shutdown_tx, _sender_shutdown_rx) = watch::channel(false);
        let (sender, sender_outbound) = test_peer_handle_with_global(
            sender_tx,
            sender_rx,
            sender_shutdown_tx,
            global_data_bytes.clone(),
            global_control_bytes.clone(),
            global_membership_bytes.clone(),
        );
        let (recipient_tx, recipient_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (recipient_shutdown_tx, _recipient_shutdown_rx) = watch::channel(false);
        let (recipient, mut recipient_outbound) = test_peer_handle_with_global(
            recipient_tx,
            recipient_rx,
            recipient_shutdown_tx,
            global_data_bytes.clone(),
            global_control_bytes,
            global_membership_bytes.clone(),
        );
        {
            let mut state = rooms.lock().expect("room registry lock");
            let members = state.rooms.entry("room".to_string()).or_default();
            members.insert(1, sender.clone());
            members.insert(2, recipient.clone());
        }

        let message = SignalIncoming::Signal {
            from: 1,
            data: SignalData::RelayData {
                channel: symtropy_net::signaling::SignalChannel::Reliable,
                payload: b"must-not-arrive-after-leave".to_vec(),
            },
        };
        let prepared = prepare_message(&recipient, &message)
            .expect("reserve the recipient mailbox before the leave wins");
        assert!(
            recipient.queued_bytes.available_permits() < MAX_SIGNALING_MESSAGE_BYTES,
            "staged relay data must hold its byte reservation until commit or drop"
        );

        // This deterministically models the race: a sender held a recipient handle
        // and reserved its queue before the recipient's leave acquired the registry.
        remove_peer_and_notify(&rooms, "room", 2);
        assert_eq!(
            commit_room_forward(&rooms, "room", 1, 2, prepared),
            Err(RoomForwardError::RecipientNotMember),
            "a completed leave must invalidate any pre-reserved data frame"
        );
        assert!(
            recipient_outbound.try_recv().is_err(),
            "a departed recipient must not receive the staged relay payload"
        );

        drop(sender_outbound);
        drop(recipient_outbound);
        assert_eq!(
            recipient.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES,
            "dropping a stale prepared frame must release recipient capacity"
        );
        assert_eq!(
            global_data_bytes.available_permits(),
            MAX_TOTAL_OUTBOUND_DATA_BYTES,
            "dropping a stale prepared frame must release global data capacity"
        );
        assert_eq!(
            global_membership_bytes.available_permits(),
            MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES,
            "dropping queued PeerLeft and stale reservations must restore membership capacity"
        );
    }

    #[test]
    fn completed_leave_before_join_never_publishes_stale_membership() {
        let rooms: SharedRooms = Arc::new(Mutex::new(Rooms::default()));
        let global_data_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_DATA_BYTES));
        let global_control_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_CONTROL_BYTES));
        let global_membership_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES));

        let (departing_tx, departing_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (departing_shutdown_tx, _departing_shutdown_rx) = watch::channel(false);
        let (departing, mut departing_outbound) = test_peer_handle_with_global(
            departing_tx,
            departing_rx,
            departing_shutdown_tx,
            global_data_bytes.clone(),
            global_control_bytes.clone(),
            global_membership_bytes.clone(),
        );
        let (joining_tx, joining_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (joining_shutdown_tx, _joining_shutdown_rx) = watch::channel(false);
        let (joining, mut joining_outbound) = test_peer_handle_with_global(
            joining_tx,
            joining_rx,
            joining_shutdown_tx,
            global_data_bytes,
            global_control_bytes,
            global_membership_bytes.clone(),
        );
        {
            let mut state = rooms.lock().expect("room registry lock");
            state
                .rooms
                .entry("room".to_string())
                .or_default()
                .insert(1, departing.clone());
        }

        // Force the leave to finish and remove the empty room before the new
        // peer joins. The new peer must not inherit a departed peer's identity.
        remove_peer_and_notify(&rooms, "room", 1);
        assert!(
            !rooms
                .lock()
                .expect("room registry lock")
                .rooms
                .contains_key("room"),
            "a room with no members must be removed"
        );
        join_room(&rooms, "room", 2, &joining).expect("new peer can create a fresh room");

        {
            let state = rooms.lock().expect("room registry lock");
            let members = state
                .rooms
                .get("room")
                .expect("fresh room must exist after the join");
            assert_eq!(members.len(), 1);
            assert!(members.contains_key(&2));
            assert!(!members.contains_key(&1));
        }
        assert!(departing_outbound.try_recv().is_err());

        let welcome = joining_outbound
            .try_recv()
            .expect("joining peer must receive Welcome");
        assert!(matches!(
            welcome.frame,
            Message::Text(ref text)
                if matches!(
                    parse_incoming(text).expect("valid Welcome"),
                    SignalIncoming::Welcome { peer_id: 2 }
                )
        ));
        assert!(
            joining_outbound.try_recv().is_err(),
            "new peer must not receive a PeerJoined for the peer that already left"
        );
        drop(welcome);

        assert_eq!(
            departing.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES
        );
        assert_eq!(
            joining.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES
        );
        assert_eq!(
            global_membership_bytes.available_permits(),
            MAX_TOTAL_OUTBOUND_MEMBERSHIP_BYTES,
            "fresh admission must release its Welcome permit once consumed"
        );
    }

    #[test]
    fn membership_budget_exhaustion_closes_room_without_blame_shifting() {
        let rooms: SharedRooms = Arc::new(Mutex::new(Rooms::default()));
        let global_data_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_DATA_BYTES));
        let global_control_bytes = Arc::new(Semaphore::new(MAX_TOTAL_OUTBOUND_CONTROL_BYTES));
        let global_membership_bytes = Arc::new(Semaphore::new(0));

        let (survivor_tx, survivor_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (survivor_shutdown_tx, mut survivor_shutdown_rx) = watch::channel(false);
        let (survivor, _survivor_outbound) = test_peer_handle_with_global(
            survivor_tx,
            survivor_rx,
            survivor_shutdown_tx,
            global_data_bytes.clone(),
            global_control_bytes.clone(),
            global_membership_bytes.clone(),
        );
        let (departing_tx, departing_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (departing_shutdown_tx, _departing_shutdown_rx) = watch::channel(false);
        let (departing, _departing_outbound) = test_peer_handle_with_global(
            departing_tx,
            departing_rx,
            departing_shutdown_tx,
            global_data_bytes,
            global_control_bytes,
            global_membership_bytes,
        );

        {
            let mut state = rooms.lock().expect("room registry lock");
            let members = state.rooms.entry("room".to_string()).or_default();
            members.insert(1, survivor);
            members.insert(2, departing);
        }

        remove_peer_and_notify(&rooms, "room", 2);

        assert!(
            *survivor_shutdown_rx.borrow(),
            "failed critical membership publication must close the affected room"
        );
        assert!(
            !rooms
                .lock()
                .expect("room registry lock")
                .rooms
                .contains_key("room"),
            "room closure prevents a stale client membership view after atomic notification failure"
        );
    }

    #[test]
    fn recipient_queue_byte_budget_accepts_one_maximum_envelope_then_backpressures() {
        let (peer, rx) = test_peer_handle();
        let message = SignalIncoming::Signal {
            from: 7,
            data: SignalData::RelayData {
                channel: symtropy_net::signaling::SignalChannel::Reliable,
                payload: vec![u8::MAX; MAX_PEER_MESSAGE_BYTES],
            },
        };
        let serialized_bytes = serde_json::to_string(&message)
            .expect("serialize maximum relay envelope")
            .len();
        assert!(serialized_bytes <= MAX_SIGNALING_MESSAGE_BYTES);

        prepare_message(&peer, &message)
            .expect("one maximum envelope fits the outbound budget")
            .commit();
        assert_eq!(
            peer.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES - serialized_bytes
        );
        let second = prepare_message(&peer, &message);
        assert!(
            matches!(second, Err(QueueError::RecipientByteBudgetFull)),
            "a second maximum envelope must not accumulate behind the first"
        );

        drop(rx);
        assert_eq!(
            peer.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES,
            "dropping the mailbox must release the queued envelope's permits"
        );
    }

    #[test]
    fn recipient_queue_count_budget_is_enforced_independently_of_bytes() {
        let (peer, rx) = test_peer_handle();
        for peer_id in 0..OUTBOUND_QUEUE_CAPACITY as u64 {
            prepare_message(&peer, &SignalIncoming::PeerJoined { peer_id })
                .expect("small membership message fits queue")
                .commit();
        }
        let next = prepare_message(&peer, &SignalIncoming::PeerJoined { peer_id: 999 });
        assert!(
            matches!(next, Err(QueueError::RecipientQueueFull)),
            "the item-count cap must reject the next small envelope"
        );
        drop(rx);
        assert_eq!(
            peer.queued_bytes.available_permits(),
            MAX_SIGNALING_MESSAGE_BYTES
        );
    }

    #[tokio::test]
    async fn library_signaling_client_authenticates_with_authorization_header() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fixture");
        let address = listener.local_addr().expect("fixture address");
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let server = tokio::spawn(run_server(listener, TEST_TOKEN.to_string(), shutdown_rx));
        let url = format!("ws://{address}");

        let mut client = SignalingClient::connect_with_token(&url, Some(TEST_TOKEN))
            .await
            .expect("library client must authenticate through the upgrade header");
        client.join("client-library-room").expect("queue Join");

        timeout(Duration::from_secs(3), async {
            loop {
                let events = client.poll_events();
                if events
                    .iter()
                    .any(|event| matches!(event, SignalingEvent::Connected(_)))
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("Welcome must be delivered through the library client's bounded mailbox");
        assert!(client.local_id.is_some());

        drop(client);
        let _ = shutdown_tx.send(true);
        timeout(Duration::from_secs(3), server)
            .await
            .expect("server shutdown timeout")
            .expect("server task join")
            .expect("server shutdown");
    }

    #[tokio::test]
    async fn unauthorized_handshake_is_rejected_before_join() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fixture");
        let address = listener.local_addr().expect("fixture address");
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let server = tokio::spawn(run_server(listener, TEST_TOKEN.to_string(), shutdown_rx));
        let url = format!("ws://{address}");
        let result = connect_async(url.clone()).await;
        assert!(result.is_err(), "server must require a bearer token");

        let mut wrong_request = url.into_client_request().expect("valid WebSocket URL");
        wrong_request.headers_mut().insert(
            AUTHORIZATION,
            HeaderValue::from_str("Bearer wrong_token_that_is_long_enough_123456")
                .expect("valid wrong-token header"),
        );
        let wrong_token_result = connect_async(wrong_request).await;
        assert!(
            wrong_token_result.is_err(),
            "server must reject a well-formed but incorrect bearer token"
        );
        let _ = shutdown_tx.send(true);
        timeout(Duration::from_secs(3), server)
            .await
            .expect("server shutdown timeout")
            .expect("server task join")
            .expect("server shutdown");
    }

    #[tokio::test]
    async fn room_membership_prevents_cross_room_forwarding() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fixture");
        let address = listener.local_addr().expect("fixture address");
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let server = tokio::spawn(run_server(listener, TEST_TOKEN.to_string(), shutdown_rx));
        let url = format!("ws://{address}");

        let mut alpha = connect_authenticated(&url).await;
        send_client_message(
            &mut alpha,
            SignalOutgoing::Join {
                room: "room-alpha".to_string(),
            },
        )
        .await;
        let alpha_id = match next_server_message(&mut alpha).await {
            SignalIncoming::Welcome { peer_id } => peer_id,
            other => panic!("alpha must receive Welcome, got {other:?}"),
        };

        let mut beta = connect_authenticated(&url).await;
        send_client_message(
            &mut beta,
            SignalOutgoing::Join {
                room: "room-beta".to_string(),
            },
        )
        .await;
        let beta_id = match next_server_message(&mut beta).await {
            SignalIncoming::Welcome { peer_id } => peer_id,
            other => panic!("beta must receive Welcome, got {other:?}"),
        };
        assert_ne!(alpha_id, beta_id);

        send_client_message(
            &mut alpha,
            SignalOutgoing::Signal {
                to: beta_id,
                data: SignalData::RelayData {
                    channel: symtropy_net::signaling::SignalChannel::Reliable,
                    payload: b"must-not-cross-room".to_vec(),
                },
            },
        )
        .await;
        assert!(matches!(
            next_server_message(&mut alpha).await,
            SignalIncoming::Error { message } if message.contains("not a member of room")
        ));
        assert!(
            timeout(Duration::from_millis(100), beta.next())
                .await
                .is_err(),
            "recipient in another room must receive no forwarded payload"
        );

        alpha.close(None).await.expect("close alpha");
        beta.close(None).await.expect("close beta");
        let _ = shutdown_tx.send(true);
        timeout(Duration::from_secs(3), server)
            .await
            .expect("server shutdown timeout")
            .expect("server task join")
            .expect("server shutdown");
    }

    #[tokio::test]
    async fn two_clients_exchange_relay_data_and_reconnect_with_authentication() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fixture");
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

        // A malicious client may try to include its own "from" field. The server
        // ignores that extra field and derives the forwarded sender ID from the
        // authenticated WebSocket session instead.
        let forged_sender_message = serde_json::json!({
            "type": "signal",
            "from": 999_999,
            "to": second_id,
            "data": {
                "kind": "relay_data",
                "channel": "reliable",
                "payload": [104, 101, 108, 108, 111]
            }
        })
        .to_string();
        first
            .send(Message::Text(forged_sender_message))
            .await
            .expect("send message with an untrusted extra sender field");
        assert!(matches!(
            next_server_message(&mut second).await,
            SignalIncoming::Signal {
                from,
                data: SignalData::RelayData {
                    channel: symtropy_net::signaling::SignalChannel::Reliable,
                    payload,
                },
            } if from == first_id && from != 999_999 && payload == b"hello"
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

        reconnected
            .close(None)
            .await
            .expect("close reconnected client");
        first.close(None).await.expect("close first client");
        let _ = shutdown_tx.send(true);
        timeout(Duration::from_secs(3), server)
            .await
            .expect("server shutdown timeout")
            .expect("server task join")
            .expect("server shutdown");
    }
}
