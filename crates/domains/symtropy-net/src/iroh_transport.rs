// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Iroh transport scaffold — an in-memory queue stub, not a QUIC transport.
//!
//! This module currently does not use Iroh, establish network connections,
//! perform NAT traversal, or provide a latency guarantee. Its injection and
//! drain methods support local tests and planned future integration only.
//!
//! A real Iroh adapter still needs endpoint creation, ALPN negotiation,
//! authenticated admission, bounded framing, lifecycle handling, and
//! multi-process qualification. Do not advertise this type as a working
//! desktop transport until those gates pass.
//!
//! # Intended Future Architecture (Not Implemented Here)
//!
//! ```text
//! IrohTransport
//!   └→ IrohBridgeHandle (from symthaea::swarm::iroh)
//!        └→ IrohBridgeActor (async tokio task)
//!             └→ Iroh Endpoint (QUIC + Magicsock)
//!                  ├→ Reliable streams (authority messages)
//!                  └→ Unreliable datagrams (physics state)
//! ```
//!
//! # Integration with Symthaea
//!
//! Symthaea's Iroh module (`symthaea/src/swarm/iroh/`) provides:
//! - `IrohBridgeHandle` — sync-safe handle for the game tick loop
//! - `IrohBridgeActor` — async actor managing connections
//! - `TensorStream` — bidirectional streaming (we reuse for physics)
//! - `HybridHandshake` — trust-gated peer verification
//!
//! Symtropy wraps this in the `Transport` trait so the game code
//! doesn't know or care whether it's using Iroh, WebRTC, or loopback.
//!
//! # Honest status (verified 2026-07-04): this is a stub, not a transport
//!
//! **Nothing in this file talks to the network.** There is no `iroh`
//! dependency in this crate's `Cargo.toml`, no QUIC endpoint, no
//! Magicsock, no NAT traversal, no discovery, and no bytes ever cross a
//! process boundary. `IrohTransport` is an in-process queue:
//! send and broadcast push into bounded in-memory queues, while poll drains
//! the inbound queue. These queues are not connected to any external actor
//! (the `inject_*`/`drain_outbox` methods exist for a future bridge actor
//! to call, but no such actor exists yet). `connect()` just flips a bool;
//! it never joins any swarm. Unit tests inject peer events/messages directly
//! and verify bounded local queue/lifecycle contracts, not P2P connectivity.
//!
//! What *does* exist, and is real: a substantial (~2,143 LOC across
//! `bridge.rs`/`mod.rs`/`streaming.rs`/`ticket.rs`) Iroh integration in
//! the separate `symthaea` crate at `symthaea/src/swarm/iroh/`, built on
//! a real, patched `iroh = "0.97"` dependency
//! (`symthaea/Cargo.toml:379,1912`, patch at
//! `symthaea/patches/iroh/iroh/`). It provides `IrohBridgeHandle` /
//! `IrohBridgeActor` and does real QUIC endpoint setup, connection
//! lifecycle, and streaming — but it lives inside `symthaea`'s `swarm`
//! module, gated behind `symthaea`'s own feature flags, and is not
//! exposed as a small, reusable library API that `symtropy-net` could
//! cleanly depend on today.
//!
//! Wiring this transport for real is **not** a small bolt-on:
//! - Depending on the full `symthaea` crate just to reach `swarm::iroh`
//!   would be architecturally wrong (a physics/game-networking crate
//!   pulling in an entire consciousness engine).
//! - The alternative — depending directly on `iroh` and writing a fresh
//!   endpoint/connect/accept/discovery loop against Iroh's real API,
//!   independent of `symthaea::swarm::iroh` — is genuine, non-trivial
//!   async networking work requiring real understanding of Iroh's
//!   `Endpoint`, ALPN protocol negotiation, `NodeAddr`/discovery, and
//!   connection lifecycle, plus real multi-process testing.
//! - Either path is scoped as its own follow-up task, not something to
//!   rush alongside unrelated fixes. A half-wired transport that *looks*
//!   connected while silently dropping every packet would be strictly
//!   worse than this honestly-labeled stub.
//!
//! Until that follow-up lands, `LoopbackTransport` (single-process,
//! in-memory) is the only transport in this crate proven to move bytes
//! between peers, and (with the `webrtc` feature, fixed 2026-07-04)
//! `RelayTransport` compiles and can reach a real WebSocket signaling
//! server, though it hasn't been validated end-to-end against one.

use crate::peer::PeerId;
use crate::transport::{Channel, PeerMessage, Transport, TransportEvent};
use std::collections::VecDeque;

/// Maximum payload size for one stub packet (1 MiB).
pub const MAX_STUB_PACKET_BYTES: usize = 1024 * 1024;
/// Maximum queued packets in each direction.
pub const MAX_STUB_QUEUE_PACKETS: usize = 256;
/// Maximum aggregate payload bytes held by each packet queue (1 MiB).
pub const MAX_STUB_QUEUE_BYTES: usize = 1024 * 1024;
/// Maximum remote peers represented by the stub.
pub const MAX_STUB_PEERS: usize = 256;
/// Bound lifecycle events even if the consumer stops polling.
pub const MAX_STUB_PENDING_EVENTS: usize = 512;

fn check_queue_admission(
    direction: &str,
    current_packets: usize,
    current_bytes: usize,
    add_packets: usize,
    packet_bytes: usize,
) -> Result<(), String> {
    if packet_bytes > MAX_STUB_PACKET_BYTES {
        return Err(format!(
            "{direction} packet of {packet_bytes} bytes exceeds {} byte limit",
            MAX_STUB_PACKET_BYTES
        ));
    }
    let packet_count = current_packets
        .checked_add(add_packets)
        .ok_or_else(|| format!("{direction} packet-count overflow"))?;
    if packet_count > MAX_STUB_QUEUE_PACKETS {
        return Err(format!(
            "{direction} queue would contain {packet_count} packets; limit is {MAX_STUB_QUEUE_PACKETS}"
        ));
    }
    let added_bytes = packet_bytes
        .checked_mul(add_packets)
        .ok_or_else(|| format!("{direction} queue byte multiplication overflow"))?;
    let bytes = current_bytes
        .checked_add(added_bytes)
        .ok_or_else(|| format!("{direction} queue byte accounting overflow"))?;
    if bytes > MAX_STUB_QUEUE_BYTES {
        return Err(format!(
            "{direction} queue would hold {bytes} bytes; limit is {MAX_STUB_QUEUE_BYTES}"
        ));
    }
    Ok(())
}

/// In-memory queue stub reserved for a future Iroh transport.
///
/// Every method currently touches only local queues. There is no Iroh endpoint,
/// QUIC connection, network discovery, or cross-process delivery. Queue bounds
/// protect this stub's own storage; a real adapter still needs wire framing,
/// authenticated admission, endpoint-level limits, and end-to-end backpressure.
pub struct IrohTransport {
    local_id: PeerId,
    peers: Vec<PeerId>,
    connected: bool,
    outbox: VecDeque<(PeerId, Channel, Vec<u8>)>,
    outbox_bytes: usize,
    inbox: VecDeque<PeerMessage>,
    inbox_bytes: usize,
    pending_events: VecDeque<TransportEvent>,
}

impl IrohTransport {
    /// Create a new in-memory transport stub.
    pub fn new(local_id: PeerId) -> Self {
        Self {
            local_id,
            peers: Vec::new(),
            connected: false,
            outbox: VecDeque::new(),
            outbox_bytes: 0,
            inbox: VecDeque::new(),
            inbox_bytes: 0,
            pending_events: VecDeque::new(),
        }
    }

    /// Inject an inbound message as a stand-in for a future transport actor.
    ///
    /// This is for local integration/testing only. It rejects disconnected,
    /// unknown-sender, oversized, and over-budget packets before queueing them.
    pub fn inject_message(&mut self, msg: PeerMessage) -> Result<(), String> {
        if !self.connected {
            return Err("Cannot inject inbound message while disconnected".into());
        }
        if !self.peers.contains(&msg.from) {
            return Err(format!(
                "Inbound sender {:?} is not connected/admitted",
                msg.from
            ));
        }
        check_queue_admission(
            "inbound",
            self.inbox.len(),
            self.inbox_bytes,
            1,
            msg.data.len(),
        )?;
        self.inbox_bytes += msg.data.len();
        self.inbox.push_back(msg);
        Ok(())
    }

    /// Inject a peer connection event as a stand-in for the future actor.
    ///
    /// Duplicate admissions are idempotent. The local identity is never
    /// considered a remote peer. Successful changes are visible from poll().
    pub fn inject_peer_connected(&mut self, peer: PeerId) -> Result<(), String> {
        if !self.connected {
            return Err("Cannot admit a peer while disconnected".into());
        }
        if peer == self.local_id {
            return Err("Cannot admit the local peer as a remote peer".into());
        }
        if self.peers.contains(&peer) {
            return Ok(());
        }
        if self.peers.len() >= MAX_STUB_PEERS {
            return Err(format!("Peer limit reached: {} admitted peers", MAX_STUB_PEERS));
        }
        if self.pending_events.len() >= MAX_STUB_PENDING_EVENTS {
            return Err("Transport lifecycle event queue is full".into());
        }
        self.peers.push(peer);
        self.pending_events.push_back(TransportEvent::PeerConnected(peer));
        Ok(())
    }

    /// Inject a disconnect and discard stale queued data for that peer.
    ///
    /// The operation is idempotent for unknown peers. If the bounded lifecycle
    /// event queue is full, it returns an error and leaves membership unchanged
    /// so the caller can retry after polling.
    pub fn inject_peer_disconnected(&mut self, peer: PeerId) -> Result<(), String> {
        if !self.peers.contains(&peer) {
            return Ok(());
        }
        if self.pending_events.len() >= MAX_STUB_PENDING_EVENTS {
            return Err("Transport lifecycle event queue is full".into());
        }

        self.peers.retain(|candidate| *candidate != peer);
        self.inbox.retain(|msg| msg.from != peer);
        self.inbox_bytes = self.inbox.iter().map(|msg| msg.data.len()).sum();
        self.outbox.retain(|(to, _, _)| *to != peer);
        self.outbox_bytes = self.outbox.iter().map(|(_, _, data)| data.len()).sum();
        self.pending_events.push_back(TransportEvent::PeerDisconnected(peer));
        Ok(())
    }

    /// Drain locally queued outbound packets for tests/future bridge adapters.
    ///
    /// Draining this stub queue is not a remote-delivery acknowledgement.
    pub fn drain_outbox(&mut self) -> Vec<(PeerId, Channel, Vec<u8>)> {
        self.outbox_bytes = 0;
        std::mem::take(&mut self.outbox).into_iter().collect()
    }

    /// Number of packets currently queued for outbound delivery.
    pub fn pending_outbound_packets(&self) -> usize {
        self.outbox.len()
    }

    /// Number of packets currently queued for inbound delivery.
    pub fn pending_inbound_packets(&self) -> usize {
        self.inbox.len()
    }
}

impl Transport for IrohTransport {
    fn connect(&mut self, _room_id: &str) -> Result<(), String> {
        // This deliberately models local stub state only; no server or swarm joins.
        if self.connected {
            return Err("Already connected; disconnect before starting a new session".into());
        }
        self.connected = true;
        Ok(())
    }

    fn disconnect(&mut self) {
        self.connected = false;
        self.peers.clear();
        self.outbox.clear();
        self.outbox_bytes = 0;
        self.inbox.clear();
        self.inbox_bytes = 0;
        self.pending_events.clear();
    }

    fn send(&mut self, to: PeerId, channel: Channel, data: &[u8]) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected".into());
        }
        if !self.peers.contains(&to) {
            return Err(format!("Peer {:?} is not connected/admitted", to));
        }
        check_queue_admission("outbound", self.outbox.len(), self.outbox_bytes, 1, data.len())?;
        self.outbox_bytes += data.len();
        self.outbox.push_back((to, channel, data.to_vec()));
        Ok(())
    }

    fn broadcast(&mut self, channel: Channel, data: &[u8]) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected".into());
        }
        let count = self.peers.len();
        check_queue_admission(
            "outbound broadcast",
            self.outbox.len(),
            self.outbox_bytes,
            count,
            data.len(),
        )?;
        let additional_bytes = data
            .len()
            .checked_mul(count)
            .ok_or_else(|| "outbound broadcast byte accounting overflow".to_string())?;

        // Capacity is checked for the full fanout before mutating state.
        for peer in &self.peers {
            self.outbox.push_back((*peer, channel, data.to_vec()));
        }
        self.outbox_bytes += additional_bytes;
        Ok(())
    }

    fn poll(&mut self) -> Vec<TransportEvent> {
        let mut events = Vec::with_capacity(self.pending_events.len() + self.inbox.len());
        events.extend(self.pending_events.drain(..));
        for msg in self.inbox.drain(..) {
            events.push(TransportEvent::Message(msg));
        }
        self.inbox_bytes = 0;
        events
    }

    fn peer_count(&self) -> usize {
        self.peers.len()
    }

    fn is_signaling_connected(&self) -> bool {
        self.connected
    }

    fn local_peer_id(&self) -> PeerId {
        self.local_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connected_transport() -> IrohTransport {
        let mut transport = IrohTransport::new(PeerId(1));
        transport.connect("test-room").unwrap();
        transport
    }

    #[test]
    fn connection_admission_is_reported_and_duplicate_is_idempotent() {
        let mut transport = connected_transport();
        transport.inject_peer_connected(PeerId(2)).unwrap();
        transport.inject_peer_connected(PeerId(2)).unwrap();
        assert_eq!(transport.peer_count(), 1);

        let events = transport.poll();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], TransportEvent::PeerConnected(peer) if *peer == PeerId(2)));
    }

    #[test]
    fn peer_admission_ceiling_is_enforced() {
        let mut transport = connected_transport();
        for raw_id in 2..=(MAX_STUB_PEERS as u64 + 1) {
            transport.inject_peer_connected(PeerId(raw_id)).unwrap();
        }

        let error = transport
            .inject_peer_connected(PeerId(MAX_STUB_PEERS as u64 + 2))
            .unwrap_err();
        assert!(error.contains("Peer limit reached"));
        assert_eq!(transport.peer_count(), MAX_STUB_PEERS);
    }

    #[test]
    fn lifecycle_event_queue_is_bounded_without_partial_admission() {
        let mut transport = connected_transport();
        for raw_id in 2..=(MAX_STUB_PEERS as u64 + 1) {
            let peer = PeerId(raw_id);
            transport.inject_peer_connected(peer).unwrap();
            transport.inject_peer_disconnected(peer).unwrap();
        }
        assert_eq!(transport.pending_events.len(), MAX_STUB_PENDING_EVENTS);
        assert_eq!(transport.peer_count(), 0);

        assert!(transport.inject_peer_connected(PeerId(900)).is_err());
        assert_eq!(transport.peer_count(), 0);
        assert_eq!(transport.pending_events.len(), MAX_STUB_PENDING_EVENTS);
    }

    #[test]
    fn transport_rejects_self_and_unknown_peer() {
        let mut transport = connected_transport();
        assert!(transport.inject_peer_connected(PeerId(1)).is_err());
        assert!(transport.send(PeerId(99), Channel::Reliable, b"authority").is_err());
        assert!(transport.drain_outbox().is_empty());
    }

    #[test]
    fn outbound_send_preserves_recipient_and_channel() {
        let mut transport = connected_transport();
        transport.inject_peer_connected(PeerId(2)).unwrap();
        transport.send(PeerId(2), Channel::Unreliable, b"physics").unwrap();

        assert_eq!(transport.pending_outbound_packets(), 1);
        let outbox = transport.drain_outbox();
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].0, PeerId(2));
        assert_eq!(outbox[0].1, Channel::Unreliable);
        assert_eq!(outbox[0].2.as_slice(), b"physics");
    }

    #[test]
    fn outbound_oversize_and_count_overflow_are_rejected_atomically() {
        let mut transport = connected_transport();
        transport.inject_peer_connected(PeerId(2)).unwrap();
        assert!(transport.send(
            PeerId(2),
            Channel::Reliable,
            &vec![0; MAX_STUB_PACKET_BYTES + 1]
        ).is_err());
        assert_eq!(transport.pending_outbound_packets(), 0);

        for _ in 0..MAX_STUB_QUEUE_PACKETS {
            transport.send(PeerId(2), Channel::Reliable, b"x").unwrap();
        }
        assert!(transport.send(PeerId(2), Channel::Reliable, b"overflow").is_err());
        assert_eq!(transport.pending_outbound_packets(), MAX_STUB_QUEUE_PACKETS);
    }

    #[test]
    fn broadcast_capacity_is_checked_before_any_recipient_is_queued() {
        let mut transport = connected_transport();
        transport.inject_peer_connected(PeerId(2)).unwrap();
        transport.inject_peer_connected(PeerId(3)).unwrap();

        transport.broadcast(Channel::Unreliable, b"state").unwrap();
        let outbox = transport.drain_outbox();
        assert_eq!(outbox.len(), 2);
        assert!(outbox.iter().any(|(peer, channel, _)| {
            *peer == PeerId(2) && *channel == Channel::Unreliable
        }));
        assert!(outbox.iter().any(|(peer, channel, _)| {
            *peer == PeerId(3) && *channel == Channel::Unreliable
        }));
    }

    #[test]
    fn broadcast_byte_overflow_is_atomic() {
        let mut transport = connected_transport();
        transport.inject_peer_connected(PeerId(2)).unwrap();
        transport.inject_peer_connected(PeerId(3)).unwrap();

        let payload = vec![0; 600_000];
        assert!(transport.broadcast(Channel::Reliable, &payload).is_err());
        assert_eq!(transport.pending_outbound_packets(), 0);
        assert!(transport.drain_outbox().is_empty());
    }

    #[test]
    fn inbound_rejects_unknown_sender_and_enforces_byte_limit() {
        let mut transport = connected_transport();
        assert!(transport.inject_message(PeerMessage {
            from: PeerId(2),
            channel: Channel::Reliable,
            data: b"forged".to_vec(),
        }).is_err());

        transport.inject_peer_connected(PeerId(2)).unwrap();
        transport.inject_message(PeerMessage {
            from: PeerId(2),
            channel: Channel::Unreliable,
            data: vec![0; MAX_STUB_QUEUE_BYTES],
        }).unwrap();
        assert!(transport.inject_message(PeerMessage {
            from: PeerId(2),
            channel: Channel::Unreliable,
            data: b"x".to_vec(),
        }).is_err());
        assert_eq!(transport.pending_inbound_packets(), 1);

        assert!(transport.poll().iter().any(|event| matches!(event, TransportEvent::Message(_))));
        assert_eq!(transport.pending_inbound_packets(), 0);
        transport.inject_message(PeerMessage {
            from: PeerId(2),
            channel: Channel::Unreliable,
            data: b"next tick".to_vec(),
        }).unwrap();
    }

    #[test]
    fn inbound_packet_count_is_bounded_even_for_empty_packets() {
        let mut transport = connected_transport();
        transport.inject_peer_connected(PeerId(2)).unwrap();
        for _ in 0..MAX_STUB_QUEUE_PACKETS {
            transport.inject_message(PeerMessage {
                from: PeerId(2),
                channel: Channel::Unreliable,
                data: Vec::new(),
            }).unwrap();
        }
        assert!(transport.inject_message(PeerMessage {
            from: PeerId(2),
            channel: Channel::Unreliable,
            data: Vec::new(),
        }).is_err());
        assert_eq!(transport.pending_inbound_packets(), MAX_STUB_QUEUE_PACKETS);
    }

    #[test]
    fn disconnect_discards_stale_messages_and_reports_lifecycle() {
        let mut transport = connected_transport();
        transport.inject_peer_connected(PeerId(2)).unwrap();
        transport.inject_message(PeerMessage {
            from: PeerId(2),
            channel: Channel::Reliable,
            data: b"old-session".to_vec(),
        }).unwrap();
        transport.send(PeerId(2), Channel::Reliable, b"old-outbound").unwrap();

        transport.inject_peer_disconnected(PeerId(2)).unwrap();
        assert_eq!(transport.peer_count(), 0);
        assert_eq!(transport.pending_inbound_packets(), 0);
        assert_eq!(transport.pending_outbound_packets(), 0);

        let events = transport.poll();
        assert!(events.iter().any(|event| {
            matches!(event, TransportEvent::PeerConnected(peer) if *peer == PeerId(2))
        }));
        assert!(events.iter().any(|event| {
            matches!(event, TransportEvent::PeerDisconnected(peer) if *peer == PeerId(2))
        }));
        assert!(!events.iter().any(|event| matches!(event, TransportEvent::Message(_))));
    }

    #[test]
    fn duplicate_connect_requires_explicit_disconnect() {
        let mut transport = connected_transport();
        transport.inject_peer_connected(PeerId(2)).unwrap();

        assert!(transport.connect("different-room").is_err());
        assert_eq!(transport.peer_count(), 1);

        transport.disconnect();
        transport.connect("different-room").unwrap();
        assert_eq!(transport.peer_count(), 0);
    }

    #[test]
    fn disconnect_clears_all_queues_and_reconnect_starts_fresh() {
        let mut transport = connected_transport();
        transport.inject_peer_connected(PeerId(2)).unwrap();
        transport.inject_message(PeerMessage {
            from: PeerId(2),
            channel: Channel::Reliable,
            data: b"old".to_vec(),
        }).unwrap();
        transport.send(PeerId(2), Channel::Reliable, b"old").unwrap();

        transport.disconnect();
        transport.connect("new-room").unwrap();
        assert_eq!(transport.peer_count(), 0);
        assert_eq!(transport.pending_inbound_packets(), 0);
        assert_eq!(transport.pending_outbound_packets(), 0);
        assert!(transport.poll().is_empty());
        assert!(transport.send(PeerId(2), Channel::Reliable, b"old").is_err());
    }
}
