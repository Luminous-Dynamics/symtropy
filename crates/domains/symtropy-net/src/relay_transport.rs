// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Relay transport — an experimental WebSocket-based signaling/data relay.
//!
//! **Not production-qualified.** This transport has no live-server integration
//! test; it tunnels game payloads through a signaling envelope and needs protocol,
//! backpressure, size-limit, and interoperability qualification before release.
//!
//! Connection lifecycle is async. The synchronous Transport::connect method
//! fails closed; use NetworkSession::join_async (or the inherent
//! RelayTransport::connect_async) and wait for a server Welcome event before
//! treating the session as connected.
//!
//! This is not a WebRTC data channel and does not implement direct P2P.

#[cfg(feature = "webrtc")]
mod implementation {
    use crate::config::NetworkConfig;
    use crate::peer::PeerId;
    use crate::signaling::{
        SignalChannel, SignalData, SignalingClient, SignalingEvent,
    };
    use crate::transport::{
        Channel, PeerMessage, Transport, TransportEvent, MAX_PEER_MESSAGE_BYTES,
    };

    /// Transport that uses the signaling WebSocket as a data relay.
    pub struct RelayTransport {
        config: NetworkConfig,
        signaling: Option<SignalingClient>,
        local_id: PeerId,
        peers: Vec<PeerId>,
        pending_events: Vec<TransportEvent>,
        connected: bool,
    }

    impl RelayTransport {
        /// Create a new relay transport with the given config.
        pub fn new(config: NetworkConfig) -> Self {
            Self {
                config,
                signaling: None,
                local_id: PeerId(rand_id()),
                peers: Vec::new(),
                pending_events: Vec::new(),
                connected: false,
            }
        }

        /// Open the signaling WebSocket and enqueue a room join.
        ///
        /// This returns after the local WebSocket/command setup succeeds. The
        /// transport remains disconnected until poll() observes the server's
        /// Welcome event and emits TransportEvent::SignalingConnected.
        pub async fn connect_async(&mut self, room_id: &str) -> Result<(), String> {
            if self.signaling.is_some() {
                return Err(
                    "RelayTransport already has a signaling connection or pending join".into(),
                );
            }

            let client = SignalingClient::connect(&self.config.signal_url).await?;
            client.join(room_id)?;
            self.peers.clear();
            self.pending_events.clear();
            self.connected = false;
            self.signaling = Some(client);
            Ok(())
        }
    }

    impl Transport for RelayTransport {
        fn connect(&mut self, _room_id: &str) -> Result<(), String> {
            Err(
                "RelayTransport requires async connection setup; use NetworkSession::join_async(...).await"
                    .into(),
            )
        }

        fn connect_async<'a>(
            &'a mut self,
            room_id: &'a str,
        ) -> impl std::future::Future<Output = Result<(), String>> + 'a {
            RelayTransport::connect_async(self, room_id)
        }

        fn disconnect(&mut self) {
            self.signaling = None;
            self.peers.clear();
            self.pending_events.clear();
            self.connected = false;
        }

        fn send(&mut self, to: PeerId, channel: Channel, data: &[u8]) -> Result<(), String> {
            if data.len() > MAX_PEER_MESSAGE_BYTES {
                return Err(format!(
                    "relay packet is {} bytes; maximum is {} bytes",
                    data.len(),
                    MAX_PEER_MESSAGE_BYTES
                ));
            }
            if !self.connected {
                return Err("Not connected: waiting for signaling-server Welcome".into());
            }
            if !self.peers.contains(&to) {
                return Err(format!("Refusing relay send to non-admitted peer {}", to.0));
            }
            let signaling = self.signaling.as_ref().ok_or("Not connected")?;
            let channel = match channel {
                Channel::Unreliable => SignalChannel::Unreliable,
                Channel::Reliable => SignalChannel::Reliable,
            };

            signaling
                .signal(
                    to,
                    SignalData::RelayData {
                        channel,
                        payload: data.to_vec(),
                    },
                )
                .map_err(|e| format!("Send: {e}"))
        }

        fn broadcast(&mut self, channel: Channel, data: &[u8]) -> Result<(), String> {
            // Validate deterministic packet limits before the first peer send so a bad
            // packet cannot produce a predictable partial broadcast.
            if data.len() > MAX_PEER_MESSAGE_BYTES {
                return Err(format!(
                    "relay packet is {} bytes; maximum is {} bytes",
                    data.len(),
                    MAX_PEER_MESSAGE_BYTES
                ));
            }
            let peers: Vec<PeerId> = self.peers.clone();
            let mut failures = Vec::new();
            for peer in peers.iter().copied() {
                if let Err(error) = self.send(peer, channel, data) {
                    failures.push(format!("peer {}: {error}", peer.0));
                }
            }
            if failures.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "relay broadcast had {} local send failure(s) across {} admitted peer(s): {}",
                    failures.len(),
                    peers.len(),
                    failures.join("; ")
                ))
            }
        }

        fn poll(&mut self) -> Vec<TransportEvent> {
            let mut events: Vec<TransportEvent> = self.pending_events.drain(..).collect();

            if let Some(ref mut signaling) = self.signaling {
                for evt in signaling.poll_events() {
                    match evt {
                        SignalingEvent::Connected(id) => {
                            self.local_id = id;
                            if !self.connected {
                                self.connected = true;
                                events.push(TransportEvent::SignalingConnected);
                            }
                        }
                        SignalingEvent::PeerJoined(id) => {
                            if self.peers.contains(&id) {
                                continue;
                            }
                            if self.peers.len() >= self.config.max_peers {
                                events.push(TransportEvent::Error(format!(
                                    "rejected peer {}: relay peer limit ({}) reached",
                                    id.0,
                                    self.config.max_peers
                                )));
                                continue;
                            }
                            self.peers.push(id);
                            events.push(TransportEvent::PeerConnected(id));
                        }
                        SignalingEvent::PeerLeft(id) => {
                            let was_admitted = self.peers.contains(&id);
                            self.peers.retain(|p| *p != id);
                            if was_admitted {
                                events.push(TransportEvent::PeerDisconnected(id));
                            }
                        }
                        SignalingEvent::Signal { from, data } => {
                            // The signaling client parses an explicit relay_data
                            // variant. SDP/ICE control messages are not game packets.
                            if !self.peers.contains(&from) {
                                events.push(TransportEvent::Error(format!(
                                    "ignored relay payload from non-admitted peer {}",
                                    from.0
                                )));
                                continue;
                            }
                            match data {
                                SignalData::RelayData { channel, payload } => {
                                    if payload.len() > MAX_PEER_MESSAGE_BYTES {
                                        events.push(TransportEvent::Error(format!(
                                            "ignored oversized relay packet from peer {}",
                                            from.0
                                        )));
                                        continue;
                                    }
                                    let channel = match channel {
                                        SignalChannel::Unreliable => Channel::Unreliable,
                                        SignalChannel::Reliable => Channel::Reliable,
                                    };
                                    events.push(TransportEvent::Message(PeerMessage {
                                        from,
                                        channel,
                                        data: payload,
                                    }));
                                }
                                _ => events.push(TransportEvent::Error(format!(
                                    "ignored non-relay signaling control message from peer {}",
                                    from.0
                                ))),
                            }
                        }
                        SignalingEvent::Disconnected => {
                            self.connected = false;
                            // A dead signaling connection invalidates the peer set;
                            // do not leave the session looking multiplayer-connected.
                            for peer in self.peers.drain(..) {
                                events.push(TransportEvent::PeerDisconnected(peer));
                            }
                            events.push(TransportEvent::SignalingDisconnected);
                        }
                        SignalingEvent::Error(e) => {
                            events.push(TransportEvent::Error(e));
                        }
                    }
                }
            }

            events
        }

        fn peer_count(&self) -> usize {
            if self.connected { self.peers.len() } else { 0 }
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
        use crate::config::NetworkConfig;

        #[test]
        fn synchronous_connect_fails_closed_without_claiming_connection() {
            let mut transport = RelayTransport::new(NetworkConfig::local_test());

            let error = transport
                .connect("test-room")
                .expect_err("sync connect must not pretend async setup succeeded");

            assert!(error.contains("join_async"));
            assert!(!transport.is_signaling_connected());
            assert!(transport.signaling.is_none());
            assert!(transport.pending_events.is_empty());
        }

        #[test]
        fn sends_are_rejected_until_server_welcome() {
            let mut transport = RelayTransport::new(NetworkConfig::local_test());

            let error = transport
                .send(PeerId(42), Channel::Reliable, b"premature")
                .expect_err("game data must not be sent before Welcome");

            assert!(error.contains("waiting for signaling-server Welcome"));
            assert!(!transport.is_signaling_connected());
        }

        #[test]
        fn new_relay_transport_is_not_connected_before_server_welcome() {
            let transport = RelayTransport::new(NetworkConfig::local_test());

            assert!(!transport.is_signaling_connected());
            assert_eq!(transport.peer_count(), 0);
            assert!(transport.pending_events.is_empty());
        }
    }

    /// Generate a random peer ID (used before server assigns one).
    fn rand_id() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        t.as_nanos() as u64 ^ (t.as_secs() << 32)
    }
}

#[cfg(feature = "webrtc")]
pub use implementation::RelayTransport;
