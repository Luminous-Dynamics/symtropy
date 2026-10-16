// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Planned Iroh direct-P2P backend — currently an in-memory scaffold.
//!
//! **This module does not open an Iroh/QUIC endpoint.** Its current methods only
//! manipulate local queues. The Magicsock, NAT traversal, peer discovery,
//! streaming, and datagram descriptions below are future integration notes, not
//! capabilities of this implementation.
//!
//! # Intended future architecture (not implemented here)
//!
//! ```text
//! IrohTransport
//!   └→ IrohBridgeHandle (possible reuse from symthaea::swarm::iroh)
//!        └→ IrohBridgeActor (future async adapter)
//!             └→ Iroh Endpoint (future QUIC + Magicsock endpoint)
//!                  ├→ Reliable streams (planned authority messages)
//!                  └→ Unreliable datagrams (planned physics state)
//! ```
//!
//! # Possible integration with Symthaea
//!
//! Symthaea's separate `symthaea/src/swarm/iroh/` tree contains candidate
//! components such as `IrohBridgeHandle`, `IrohBridgeActor`, `TensorStream`,
//! and `HybridHandshake`. This crate does not depend on that module today.
//!
//! # Current source status (2026-10-11): scaffold, not a network transport
//!
//! There is no `iroh` dependency in this crate's `Cargo.toml`, no QUIC endpoint,
//! no Magicsock, no NAT traversal, no peer discovery, and no bytes cross a process
//! boundary. `send()`/`broadcast()` only append to an in-memory `Vec`; `poll()`
//! drains another in-memory `Vec`; `connect()` only changes a Boolean. The unit
//! tests exercise this queue plumbing through injected events and payloads, not P2P
//! connectivity, latency, NAT traversal, or trust-gated handshakes.
//!
//! A substantial Iroh integration exists in the separate `symthaea` crate, but
//! it is not exposed as a small reusable API that this crate can depend on without
//! pulling in unrelated engine dependencies. A proper integration remains a
//! separate task requiring connection lifecycle, protocol negotiation, discovery,
//! authentication/peer verification, and multi-process testing.
//!
//! Until that work lands, `LoopbackTransport` is the only transport here proven
//! to move bytes between peers within one process. `RelayTransport` (feature
//! `ws-relay`; legacy alias `webrtc`) is an experimental custom JSON WebSocket
//! relay that requires a compatible service and is not live-server-qualified.

use crate::peer::PeerId;
use crate::transport::{Channel, PeerMessage, Transport, TransportEvent};

/// Planned Iroh transport API. The current implementation is an in-memory
/// test scaffold; it does not send over QUIC or cross a process boundary.
/// See the module-level status notes before treating it as a network transport.
pub struct IrohTransport {
    local_id: PeerId,
    peers: Vec<PeerId>,
    connected: bool,
    /// Outbound message queue (drained by the bridge actor).
    outbox: Vec<(PeerId, Channel, Vec<u8>)>,
    /// Inbound message queue (filled by the bridge actor).
    inbox: Vec<PeerMessage>,
}

impl IrohTransport {
    /// Create a new Iroh transport.
    ///
    /// In the full integration, this takes an `IrohBridgeHandle` parameter.
    /// For now, it creates a standalone instance that queues messages
    /// until the bridge is wired.
    pub fn new(local_id: PeerId) -> Self {
        Self {
            local_id,
            peers: Vec::new(),
            connected: false,
            outbox: Vec::new(),
            inbox: Vec::new(),
        }
    }

    /// Inject a received message (called by the bridge actor adapter).
    pub fn inject_message(&mut self, msg: PeerMessage) {
        self.inbox.push(msg);
    }

    /// Inject a peer connection event.
    pub fn inject_peer_connected(&mut self, peer: PeerId) {
        if !self.peers.contains(&peer) {
            self.peers.push(peer);
        }
    }

    /// Inject a peer disconnection event.
    pub fn inject_peer_disconnected(&mut self, peer: PeerId) {
        self.peers.retain(|p| *p != peer);
    }

    /// Drain outbound messages (consumed by the bridge actor adapter).
    pub fn drain_outbox(&mut self) -> Vec<(PeerId, Channel, Vec<u8>)> {
        std::mem::take(&mut self.outbox)
    }
}

impl Transport for IrohTransport {
    fn connect(&mut self, _room_id: &str) -> Result<(), String> {
        // In the full integration, this joins the Iroh swarm.
        // The room_id maps to a topic/document hash.
        self.connected = true;
        Ok(())
    }

    fn disconnect(&mut self) {
        self.connected = false;
        self.peers.clear();
        self.outbox.clear();
        self.inbox.clear();
    }

    fn send(&mut self, to: PeerId, channel: Channel, data: &[u8]) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected".into());
        }
        self.outbox.push((to, channel, data.to_vec()));
        Ok(())
    }

    fn broadcast(&mut self, channel: Channel, data: &[u8]) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected".into());
        }
        for peer in &self.peers {
            self.outbox.push((*peer, channel, data.to_vec()));
        }
        Ok(())
    }

    fn poll(&mut self) -> Vec<TransportEvent> {
        let mut events = Vec::new();
        for msg in self.inbox.drain(..) {
            events.push(TransportEvent::Message(msg));
        }
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

    #[test]
    fn iroh_transport_basic() {
        let mut transport = IrohTransport::new(PeerId(1));
        transport.connect("test-room").unwrap();
        assert!(transport.is_signaling_connected());
        assert_eq!(transport.peer_count(), 0);

        // Simulate peer joining
        transport.inject_peer_connected(PeerId(2));
        assert_eq!(transport.peer_count(), 1);

        // Send a message
        transport
            .send(PeerId(2), Channel::Unreliable, b"physics")
            .unwrap();
        let outbox = transport.drain_outbox();
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].0, PeerId(2));
        assert_eq!(outbox[0].2, b"physics");
    }

    #[test]
    fn iroh_transport_receive() {
        let mut transport = IrohTransport::new(PeerId(1));
        transport.connect("test").unwrap();

        transport.inject_message(PeerMessage {
            from: PeerId(2),
            channel: Channel::Reliable,
            data: b"authority".to_vec(),
        });

        let events = transport.poll();
        assert_eq!(events.len(), 1);
        match &events[0] {
            TransportEvent::Message(msg) => {
                assert_eq!(msg.from, PeerId(2));
                assert_eq!(msg.data, b"authority");
            }
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn iroh_transport_broadcast() {
        let mut transport = IrohTransport::new(PeerId(0));
        transport.connect("test").unwrap();
        transport.inject_peer_connected(PeerId(1));
        transport.inject_peer_connected(PeerId(2));

        transport.broadcast(Channel::Unreliable, b"state").unwrap();
        let outbox = transport.drain_outbox();
        assert_eq!(outbox.len(), 2);
    }

    #[test]
    fn iroh_transport_disconnect() {
        let mut transport = IrohTransport::new(PeerId(0));
        transport.connect("test").unwrap();
        transport.inject_peer_connected(PeerId(1));

        transport.disconnect();
        assert!(!transport.is_signaling_connected());
        assert_eq!(transport.peer_count(), 0);
    }
}
