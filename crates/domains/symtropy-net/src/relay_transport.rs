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
    use crate::signaling::{SignalData, SignalingClient, SignalingEvent};
    use crate::transport::{Channel, PeerMessage, Transport, TransportEvent};

    /// Game data message relayed through the signaling server.
    #[derive(serde::Serialize, serde::Deserialize)]
    struct RelayedData {
        channel: u8, // 0 = unreliable, 1 = reliable
        payload: Vec<u8>,
    }

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
                return Err("RelayTransport already has a signaling connection or pending join".into());
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
            Err("RelayTransport requires async connection setup; use NetworkSession::join_async(...).await".into())
        }

        fn connect_async<'a>(
            &'a mut self,
            room_id: &'a str,
        ) -> impl std::future::Future<Output = Result<(), String>> + 'a {
            async move { RelayTransport::connect_async(self, room_id).await }
        }

        fn disconnect(&mut self) {
            self.signaling = None;
            self.peers.clear();
            self.pending_events.clear();
            self.connected = false;
        }

        fn send(&mut self, to: PeerId, channel: Channel, data: &[u8]) -> Result<(), String> {
            if !self.connected {
                return Err("Not connected: waiting for signaling-server Welcome".into());
            }
            let signaling = self.signaling.as_ref().ok_or("Not connected")?;

            let relayed = RelayedData {
                channel: match channel {
                    Channel::Unreliable => 0,
                    Channel::Reliable => 1,
                },
                payload: data.to_vec(),
            };

            let json = serde_json::to_string(&relayed).map_err(|e| format!("Serialize: {e}"))?;

            signaling
                .signal(to, SignalData::Offer { sdp: json })
                .map_err(|e| format!("Send: {e}"))
        }

        fn broadcast(&mut self, channel: Channel, data: &[u8]) -> Result<(), String> {
            let peers: Vec<PeerId> = self.peers.clone();
            for peer in peers {
                self.send(peer, channel, data)?;
            }
            Ok(())
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
                            if !self.peers.contains(&id) {
                                self.peers.push(id);
                                events.push(TransportEvent::PeerConnected(id));
                            }
                        }
                        SignalingEvent::PeerLeft(id) => {
                            self.peers.retain(|p| *p != id);
                            events.push(TransportEvent::PeerDisconnected(id));
                        }
                        SignalingEvent::Signal { from, data } => {
                            // Decode relayed game data
                            if let SignalData::Offer { sdp } = data
                                && let Ok(relayed) = serde_json::from_str::<RelayedData>(&sdp)
                            {
                                let channel = if relayed.channel == 0 {
                                    Channel::Unreliable
                                } else {
                                    Channel::Reliable
                                };
                                events.push(TransportEvent::Message(PeerMessage {
                                    from,
                                    channel,
                                    data: relayed.payload,
                                }));
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
