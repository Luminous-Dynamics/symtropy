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

        /// Enforce session membership before exposing a remote peer to NetworkSession.
        fn admit_peer(&mut self, peer_id: PeerId) -> Result<bool, String> {
            if !self.connected {
                return Err(format!(
                    "rejected peer {} before signaling-server Welcome",
                    peer_id.0
                ));
            }
            if peer_id == self.local_id {
                return Err(format!(
                    "rejected signaling peer {} because it matches the local identity",
                    peer_id.0
                ));
            }
            if self.peers.contains(&peer_id) {
                return Ok(false);
            }
            if self.peers.len() >= self.config.max_peers {
                return Err(format!(
                    "rejected peer {}: relay peer limit ({}) reached",
                    peer_id.0,
                    self.config.max_peers
                ));
            }
            self.peers.push(peer_id);
            Ok(true)
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
            let mut clear_signaling = false;

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
                        SignalingEvent::PeerJoined(id) => match self.admit_peer(id) {
                            Ok(true) => events.push(TransportEvent::PeerConnected(id)),
                            Ok(false) => {}
                            Err(error) => events.push(TransportEvent::Error(error)),
                        },
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
                            clear_signaling = true;
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

            // Drop the terminated client so an explicit later connect_async can create
            // a fresh websocket/command task instead of retaining a closed handle.
            if clear_signaling {
                self.signaling = None;
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

        #[test]
        fn relay_peer_admission_is_idempotent_and_respects_identity_and_limit() {
            let mut config = NetworkConfig::local_test();
            config.max_peers = 1;
            let mut transport = RelayTransport::new(config);
            transport.local_id = PeerId(100);
            transport.connected = true;

            assert!(transport.admit_peer(PeerId(1)).expect("first peer is admitted"));
            assert!(!transport.admit_peer(PeerId(1)).expect("duplicate is idempotent"));
            assert!(transport
                .admit_peer(PeerId(100))
                .expect_err("local identity must not be a remote peer")
                .contains("local identity"));
            assert!(transport
                .admit_peer(PeerId(2))
                .expect_err("peer limit must be enforced")
                .contains("peer limit"));
            assert_eq!(transport.peers.len(), 1);
        }

        #[test]
        fn peer_admission_fails_closed_before_welcome() {
            let mut transport = RelayTransport::new(NetworkConfig::local_test());
            let error = transport
                .admit_peer(PeerId(1))
                .expect_err("peer events before Welcome must not create membership");
            assert!(error.contains("before signaling-server Welcome"));
            assert!(transport.peers.is_empty());
        }

        #[test]
        fn rejects_oversized_game_packet_before_connection_or_payload_copy() {
            let mut transport = RelayTransport::new(NetworkConfig::local_test());
            let payload = vec![0; MAX_PEER_MESSAGE_BYTES + 1];

            let error = transport
                .send(PeerId(42), Channel::Reliable, &payload)
                .expect_err("relay transport must enforce packet cap before connection work");

            assert!(error.contains("maximum is"));
            assert!(!transport.is_signaling_connected());
            assert!(transport.signaling.is_none());
        }

        #[test]
        fn connected_relay_refuses_unadmitted_recipient() {
            let mut transport = RelayTransport::new(NetworkConfig::local_test());
            transport.connected = true;

            let error = transport
                .send(PeerId(42), Channel::Reliable, b"not admitted")
                .expect_err("an arbitrary peer ID must not become a relay destination");

            assert!(error.contains("non-admitted peer"));
        }

        #[cfg(not(target_arch = "wasm32"))]
        #[tokio::test]
        async fn localhost_websocket_fixture_roundtrips_relay_data_and_reconnects() {
            use futures_util::{SinkExt, StreamExt};
            use std::time::Duration;
            use tokio::net::TcpListener;
            use tokio::time::{sleep, timeout};
            use tokio_tungstenite::{accept_async, tungstenite::Message};

            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind local signaling fixture");
            let address = listener.local_addr().expect("fixture local address");

            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.expect("accept first client");
                let mut websocket = accept_async(stream).await.expect("accept first WebSocket");

                let join_text = match websocket.next().await {
                    Some(Ok(Message::Text(text))) => text,
                    other => panic!("expected first join command, received {other:?}"),
                };
                let join: serde_json::Value =
                    serde_json::from_str(&join_text).expect("parse first join command");
                assert_eq!(join["type"], "join");
                assert_eq!(join["room"], "fixture-room");

                websocket
                    .send(Message::Text(
                        serde_json::to_string(&SignalIncoming::Welcome { peer_id: 1 })
                            .expect("serialize first Welcome"),
                    ))
                    .await
                    .expect("send first Welcome");
                websocket
                    .send(Message::Text(
                        serde_json::to_string(&SignalIncoming::PeerJoined { peer_id: 2 })
                            .expect("serialize peer_joined"),
                    ))
                    .await
                    .expect("send peer_joined");

                // A control-plane SDP offer must not be treated as gameplay data.
                websocket
                    .send(Message::Text(
                        serde_json::to_string(&SignalIncoming::Signal {
                            from: 2,
                            data: SignalData::Offer {
                                sdp: "not-a-game-packet".to_string(),
                            },
                        })
                        .expect("serialize control-plane offer"),
                    ))
                    .await
                    .expect("send control-plane offer");

                let outgoing_text = match websocket.next().await {
                    Some(Ok(Message::Text(text))) => text,
                    other => panic!("expected relay-data command, received {other:?}"),
                };
                let outgoing: serde_json::Value =
                    serde_json::from_str(&outgoing_text).expect("parse relay-data command");
                assert_eq!(outgoing["type"], "signal");
                assert_eq!(outgoing["to"], 2);
                assert_eq!(outgoing["data"]["kind"], "relay_data");
                assert_eq!(outgoing["data"]["channel"], "reliable");
                assert_eq!(
                    outgoing["data"]["payload"],
                    serde_json::json!([104, 101, 108, 108, 111])
                );

                websocket
                    .send(Message::Text(
                        serde_json::to_string(&SignalIncoming::Signal {
                            from: 2,
                            data: SignalData::RelayData {
                                channel: SignalChannel::Unreliable,
                                payload: b"world".to_vec(),
                            },
                        })
                        .expect("serialize inbound relay data"),
                    ))
                    .await
                    .expect("send inbound relay data");
                websocket
                    .send(Message::Close(None))
                    .await
                    .expect("close first signaling connection");
                drop(websocket);

                // Accept a second explicit connection to prove the client discarded
                // the terminated websocket handle instead of retaining stale state.
                let (stream, _) = timeout(Duration::from_secs(3), listener.accept())
                    .await
                    .expect("client should reconnect to the same fixture")
                    .expect("accept second client");
                let mut websocket = accept_async(stream).await.expect("accept second WebSocket");
                let second_join_text = match timeout(Duration::from_secs(3), websocket.next())
                    .await
                    .expect("second Join should arrive")
                {
                    Some(Ok(Message::Text(text))) => text,
                    other => panic!("expected second join command, received {other:?}"),
                };
                let second_join: serde_json::Value =
                    serde_json::from_str(&second_join_text).expect("parse second join command");
                assert_eq!(second_join["type"], "join");
                assert_eq!(second_join["room"], "fixture-room-2");

                websocket
                    .send(Message::Text(
                        serde_json::to_string(&SignalIncoming::Welcome { peer_id: 3 })
                            .expect("serialize second Welcome"),
                    ))
                    .await
                    .expect("send second Welcome");

                let close = timeout(Duration::from_secs(3), websocket.next())
                    .await
                    .expect("client should close the second connection after the test");
                assert!(
                    matches!(&close, None | Some(Ok(Message::Close(_)))),
                    "expected second websocket to close cleanly, got {close:?}"
                );

                outgoing
            });

            let mut config = NetworkConfig::local_test();
            config.signal_url = format!("ws://{address}");
            let mut transport = RelayTransport::new(config);
            transport
                .connect_async("fixture-room")
                .await
                .expect("connect to local signaling fixture");

            let mut initial_events = Vec::new();
            let mut rejected_control_message = false;
            timeout(Duration::from_secs(3), async {
                loop {
                    let events = transport.poll();
                    rejected_control_message |= events.iter().any(|event| {
                        matches!(
                            event,
                            TransportEvent::Error(error)
                                if error.contains("non-relay signaling control message")
                        )
                    });
                    initial_events.extend(events);
                    if transport.is_signaling_connected()
                        && transport.peer_count() == 1
                        && rejected_control_message
                    {
                        break;
                    }
                    sleep(Duration::from_millis(1)).await;
                }
            })
            .await
            .expect("Welcome, peer admission and control/data separation must arrive");

            assert!(initial_events
                .iter()
                .any(|event| matches!(event, TransportEvent::SignalingConnected)));
            assert!(initial_events
                .iter()
                .any(|event| matches!(event, TransportEvent::PeerConnected(PeerId(2)))));
            assert!(
                rejected_control_message,
                "SDP/ICE control messages must never be promoted to game packets"
            );

            transport
                .send(PeerId(2), Channel::Reliable, b"hello")
                .expect("queue explicit relay_data command");

            let mut got_payload = false;
            let mut got_disconnect = false;
            timeout(Duration::from_secs(3), async {
                while !(got_payload && got_disconnect) {
                    for event in transport.poll() {
                        match event {
                            TransportEvent::Message(message) => {
                                assert_eq!(message.from, PeerId(2));
                                assert_eq!(message.channel, Channel::Unreliable);
                                assert_eq!(message.data, b"world");
                                got_payload = true;
                            }
                            TransportEvent::SignalingDisconnected => got_disconnect = true,
                            _ => {}
                        }
                    }
                    sleep(Duration::from_millis(1)).await;
                }
            })
            .await
            .expect("client must receive relay data and terminal disconnect");

            assert!(!transport.is_signaling_connected());
            assert_eq!(transport.peer_count(), 0);

            transport
                .connect_async("fixture-room-2")
                .await
                .expect("create fresh connection after terminal disconnect");
            let mut reconnected = false;
            timeout(Duration::from_secs(3), async {
                loop {
                    let events = transport.poll();
                    if events
                        .iter()
                        .any(|event| matches!(event, TransportEvent::SignalingConnected))
                    {
                        reconnected = true;
                    }
                    if reconnected {
                        break;
                    }
                    sleep(Duration::from_millis(1)).await;
                }
            })
            .await
            .expect("second Welcome must establish the new connection");
            assert_eq!(transport.local_peer_id(), PeerId(3));

            transport.disconnect();
            let _outgoing = timeout(Duration::from_secs(3), server)
                .await
                .expect("fixture should complete both sessions")
                .expect("fixture task should not panic");
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
