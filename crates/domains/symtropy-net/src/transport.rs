// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Transport trait — the abstraction over how peers exchange data.
//!
//! Symtropy uses two channels per peer connection:
//! - **Unreliable**: Physics state (positions, velocities). Dropped packets
//!   are fine — next tick's data supersedes. Low latency critical.
//! - **Reliable**: Authority changes, governance actions, chat.
//!   Must arrive in order, no drops.
//!
//! Current implementations and qualification status:
//! - `RelayTransport` (feature `webrtc`) — experimental WebSocket signaling/data relay;
//!   not a WebRTC data channel and not qualified against a live signaling server.
//! - `IrohTransport` — in-memory scaffold only; it does not open an Iroh/QUIC endpoint.
//! - `LoopbackTransport` — in-memory test transport.
//!
//! No production-qualified network transport is currently provided by this crate.

use crate::peer::PeerId;
use serde::{Deserialize, Serialize};

/// Maximum encoded game packet accepted by the session and relay envelope.
pub(crate) const MAX_PEER_MESSAGE_BYTES: usize = 1024 * 1024;

/// Maximum number of body identifiers in one authority message.
///
/// Larger authority transitions must be chunked into explicit protocol messages
/// instead of forcing receivers to allocate an unbounded collection from one packet.
pub(crate) const MAX_AUTHORITY_BODY_IDS: usize = 4096;

/// Maximum UTF-8 byte length for an authority-transfer reason.
pub(crate) const MAX_AUTHORITY_REASON_BYTES: usize = 4 * 1024;

/// Channel reliability mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Unreliable, unordered. For physics state (positions, velocities).
    /// Dropped packets accepted — next tick supersedes.
    Unreliable,
    /// Reliable, ordered. For authority changes, governance, chat.
    Reliable,
}

/// A message received from a peer.
#[derive(Debug, Clone)]
pub struct PeerMessage {
    /// Which peer sent this.
    pub from: PeerId,
    /// Which channel it arrived on.
    pub channel: Channel,
    /// Raw payload bytes (MessagePack-encoded).
    pub data: Vec<u8>,
}

/// Events emitted by the transport layer.
#[derive(Debug, Clone)]
pub enum TransportEvent {
    /// A new peer connected.
    PeerConnected(PeerId),
    /// A peer disconnected.
    PeerDisconnected(PeerId),
    /// A message was received.
    Message(PeerMessage),
    /// Connection to signaling server established.
    SignalingConnected,
    /// Connection to signaling server lost.
    SignalingDisconnected,
    /// Error occurred.
    Error(String),
}

/// The transport trait — the byte/message boundary between peers.
///
/// Concrete transports own connection setup, framing, peer membership, and their
/// own reliability semantics. Some backends use a signaling relay; this trait
/// does not imply ICE negotiation, direct peer-to-peer connectivity, or data-channel
/// support. The game loop calls `poll()` each tick and `send()` to distribute state.
pub trait Transport {
    /// Connect to the signaling server and join a room.
    /// This begins the peer discovery process.
    fn connect(&mut self, room_id: &str) -> Result<(), String>;

    /// Connect using the transport's asynchronous path when one is required.
    ///
    /// Synchronous transports inherit this default implementation. Async
    /// transports should override it and must not report signaling connectivity
    /// until the remote handshake has actually been observed.
    fn connect_async<'a>(
        &'a mut self,
        room_id: &'a str,
    ) -> impl std::future::Future<Output = Result<(), String>> + 'a {
        async move { self.connect(room_id) }
    }

    /// Disconnect from all peers and the signaling server.
    fn disconnect(&mut self);

    /// Send data to a specific peer on the given channel.
    fn send(&mut self, to: PeerId, channel: Channel, data: &[u8]) -> Result<(), String>;

    /// Broadcast data to all connected peers on the given channel.
    fn broadcast(&mut self, channel: Channel, data: &[u8]) -> Result<(), String>;

    /// Poll for events. Call once per game tick.
    /// Returns all events that occurred since the last poll.
    fn poll(&mut self) -> Vec<TransportEvent>;

    /// Number of currently connected peers.
    fn peer_count(&self) -> usize;

    /// Whether we're connected to the signaling server.
    fn is_signaling_connected(&self) -> bool;

    /// Our own peer ID (assigned by signaling server or self-generated).
    fn local_peer_id(&self) -> PeerId;
}

/// Physics state snapshot for a single body — sent on the unreliable channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyStateUpdate {
    /// Body handle identifier.
    pub body_id: u32,
    /// Position (x, y, z) — or (x, y) for 2D.
    pub position: [f64; 3],
    /// Linear velocity.
    pub velocity: [f64; 3],
    /// Rotation (quaternion: w, x, y, z).
    pub rotation: [f64; 4],
    /// Angular velocity.
    pub angular_velocity: [f64; 3],
}

/// A batch of body state updates — one per tick from the authority peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsSync {
    /// Tick number this snapshot is from.
    pub tick: u64,
    /// The peer that computed this physics step.
    pub authority: u64,
    /// Body states (only bodies this peer has authority over).
    pub bodies: Vec<BodyStateUpdate>,
}

/// Authority change message — sent on the reliable channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorityMessage {
    /// Claim authority over bodies.
    Claim { body_ids: Vec<u32>, peer_id: u64 },
    /// Release authority over bodies.
    Release { body_ids: Vec<u32> },
    /// Request authority transfer (negotiation).
    RequestTransfer { body_ids: Vec<u32>, reason: String },
}
