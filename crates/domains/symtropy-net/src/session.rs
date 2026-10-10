// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Network session — ties transport, authority, and state sync together.
//!
//! The game loop calls `session.tick()` each frame, which:
//! 1. Polls the transport for incoming messages
//! 2. Applies received physics state to remote bodies
//! 3. Sends local authority bodies' state to peers
//! 4. Handles authority claim/release messages

use std::collections::HashMap;

use crate::authority::SpatialAuthority;
use crate::config::NetworkConfig;
use crate::peer::{PeerId, PeerState};
use crate::transport::{
    AuthorityMessage, BodyStateUpdate, Channel, PhysicsSync, Transport, TransportEvent,
};

/// Hard upper bound for serialized or decoded peer messages. Concrete transports
/// should enforce their own frame limits too; this is the session-level protocol cap.
const MAX_PEER_MESSAGE_BYTES: usize = 1024 * 1024;

/// An authority message received from a peer, retaining the transport sender identity.
///
/// This is provenance metadata, not proof that the peer is authorized to perform the
/// requested operation. Consumers must validate permissions before changing authority.
#[derive(Debug, Clone)]
pub struct IncomingAuthorityMessage {
    pub from: PeerId,
    pub message: AuthorityMessage,
}

/// A failed send to a session-admitted peer.
#[derive(Debug, Clone)]
pub struct OutboundSendFailure {
    pub peer: PeerId,
    pub error: String,
}

/// Observable outcome of one session-level outbound operation.
///
/// A report can describe partial delivery: delivered_peers may be non-zero even
/// when failures is non-empty. Callers should inspect the report before treating
/// reliable authority changes as delivered to every admitted peer.
#[derive(Debug, Clone, Default)]
pub struct OutboundSendReport {
    pub attempted_peers: usize,
    pub delivered_peers: usize,
    pub failures: Vec<OutboundSendFailure>,
    pub validation_error: Option<String>,
    pub serialization_error: Option<String>,
}

impl OutboundSendReport {
    /// Whether the operation had no validation, serialization, or per-peer send errors.
    pub fn is_complete(&self) -> bool {
        self.validation_error.is_none()
            && self.serialization_error.is_none()
            && self.failures.is_empty()
    }
}

/// A multiplayer network session.
pub struct NetworkSession<T: Transport> {
    /// The underlying transport (WebRTC, loopback, etc.).
    pub transport: T,
    /// Network configuration.
    pub config: NetworkConfig,
    /// Spatial authority system.
    pub authority: SpatialAuthority,
    /// Known peers.
    pub peers: HashMap<PeerId, PeerState>,
    /// Current tick.
    pub tick: u64,
    /// Received physics updates from remote peers (consumed by game loop).
    pub incoming_physics: Vec<PhysicsSync>,
    /// Received authority intents with source identity preserved for game-layer validation.
    pub incoming_authority: Vec<IncomingAuthorityMessage>,
}

impl<T: Transport> NetworkSession<T> {
    /// Create a new session with the given transport and config.
    pub fn new(transport: T, config: NetworkConfig) -> Self {
        let local_id = transport.local_peer_id();
        Self {
            transport,
            config,
            authority: SpatialAuthority::new(local_id, 100.0),
            peers: HashMap::new(),
            tick: 0,
            incoming_physics: Vec::new(),
            incoming_authority: Vec::new(),
        }
    }

    /// Join a room and start peer discovery.
    pub fn join(&mut self, room_id: &str) -> Result<(), String> {
        self.transport.connect(room_id)
    }

    /// Leave the session and release ownership claims held by remote peers.
    pub fn leave(&mut self) {
        self.transport.disconnect();
        self.clear_peers();
    }

    fn remove_peer(&mut self, peer_id: PeerId) {
        // A malformed disconnect event for the local identity must never release
        // locally owned bodies; only remote identities belong to this cleanup path.
        if peer_id == self.transport.local_peer_id() {
            return;
        }

        self.peers.remove(&peer_id);
        self.authority.release_peer(peer_id);
        self.incoming_physics
            .retain(|sync| sync.authority != peer_id.0);
        self.incoming_authority
            .retain(|message| message.from != peer_id);
    }

    fn clear_peers(&mut self) {
        let peers: Vec<PeerId> = self.peers.keys().copied().collect();
        for peer_id in peers {
            self.remove_peer(peer_id);
        }
        // The session/control connection is gone; no queued remote snapshot or
        // authority intent remains valid for the next session.
        self.incoming_physics.clear();
        self.incoming_authority.clear();
    }

    /// Process one tick of networking.
    ///
    /// Call this once per game tick. It:
    /// 1. Polls transport for events
    /// 2. Handles peer connect/disconnect
    /// 3. Deserializes incoming physics and authority messages
    /// 4. Clears stale data from previous tick
    pub fn tick(&mut self) {
        self.tick += 1;
        self.incoming_physics.clear();
        self.incoming_authority.clear();

        let events = self.transport.poll();
        for event in events {
            match event {
                TransportEvent::PeerConnected(peer_id) => {
                    // A transport must never admit the local identity as a remote peer.
                    if peer_id == self.transport.local_peer_id() {
                        #[cfg(feature = "logging")]
                        eprintln!(
                            "[symtropy-net] rejected local peer identity {} from PeerConnected",
                            peer_id.0
                        );
                        continue;
                    }

                    // Duplicate connection events must not reset liveness state.
                    if self.peers.contains_key(&peer_id) {
                        continue;
                    }

                    // max_peers is a session admission limit, not a transport-level
                    // socket limit. Excess peers remain unadmitted and their messages
                    // are rejected by the membership check below.
                    if self.peers.len() >= self.config.max_peers {
                        #[cfg(feature = "logging")]
                        eprintln!(
                            "[symtropy-net] rejected peer {}: session peer limit ({}) reached",
                            peer_id.0,
                            self.config.max_peers
                        );
                        continue;
                    }

                    self.peers.insert(
                        peer_id,
                        PeerState::remote(peer_id, format!("Peer-{}", peer_id.0), 0),
                    );
                }
                TransportEvent::PeerDisconnected(peer_id) => {
                    self.remove_peer(peer_id);
                }
                TransportEvent::Message(msg) => {
                    // Membership is established only by a transport PeerConnected event.
                    // Drop payloads from unknown peers and oversized frames before parsing.
                    if !self.peers.contains_key(&msg.from)
                        || msg.data.len() > MAX_PEER_MESSAGE_BYTES
                    {
                        continue;
                    }

                    match msg.channel {
                        Channel::Unreliable => {
                            if let Ok(sync) = rmp_serde::from_slice::<PhysicsSync>(&msg.data) {
                                // The payload's claimed authority must agree with the
                                // transport-authenticated sender identity.
                                if sync.authority != msg.from.0 {
                                    continue;
                                }
                                if let Some(peer) = self.peers.get_mut(&msg.from) {
                                    peer.mark_seen(self.tick);
                                }
                                self.incoming_physics.push(sync);
                            }
                        }
                        Channel::Reliable => {
                            if let Ok(auth) = rmp_serde::from_slice::<AuthorityMessage>(&msg.data) {
                                // A claim must identify the admitted peer that sent it.
                                // Other authority operations remain intents until the
                                // application validates their source and permissions.
                                if matches!(
                                    &auth,
                                    AuthorityMessage::Claim { peer_id, .. }
                                        if *peer_id != msg.from.0
                                ) {
                                    continue;
                                }

                                if let Some(peer) = self.peers.get_mut(&msg.from) {
                                    peer.mark_seen(self.tick);
                                }
                                self.incoming_authority.push(IncomingAuthorityMessage {
                                    from: msg.from,
                                    message: auth,
                                });
                            }
                        }
                    }
                }
                TransportEvent::SignalingConnected => {
                    // Some transports receive their authoritative local peer ID
                    // only after the remote handshake. Keep spatial authority in
                    // sync with that assigned identity before accepting gameplay.
                    self.authority
                        .reidentify_local_peer(self.transport.local_peer_id());
                }
                TransportEvent::SignalingDisconnected => {
                    // A lost signaling/control connection invalidates all admitted
                    // remote peers and their authority leases.
                    self.clear_peers();
                }
                TransportEvent::Error(e) => {
                    // Log but don't crash — graceful degradation
                    #[cfg(feature = "logging")]
                    eprintln!("[symtropy-net] Transport error: {e}");
                    let _ = e;
                }
            }
        }

        // Expire stale peers
        // Normalize a zero send rate to 1 Hz for timeout accounting so malformed
        // configuration cannot panic on integer division by zero.
        let effective_send_rate_hz = self.config.send_rate_hz.max(1) as u64;
        let timeout = self
            .config
            .authority_timeout_ms
            .saturating_mul(effective_send_rate_hz)
            .div_ceil(1000);
        let stale: Vec<PeerId> = self
            .peers
            .iter()
            .filter(|(_, p)| !p.is_alive(self.tick, timeout))
            .map(|(id, _)| *id)
            .collect();
        for id in stale {
            self.remove_peer(id);
        }
    }

    /// Send local physics state to all peers.
    ///
    /// Call after computing physics for bodies the local peer has authority over.
    pub fn send_physics(&mut self, bodies: Vec<BodyStateUpdate>) -> OutboundSendReport {
        if bodies.is_empty() || self.peers.is_empty() {
            return OutboundSendReport::default();
        }

        let sync = PhysicsSync {
            tick: self.tick,
            authority: self.transport.local_peer_id().0,
            bodies,
        };

        let data = match rmp_serde::to_vec(&sync) {
            Ok(data) => data,
            Err(error) => {
                return OutboundSendReport {
                    serialization_error: Some(error.to_string()),
                    ..OutboundSendReport::default()
                };
            }
        };
        if data.len() > MAX_PEER_MESSAGE_BYTES {
            return OutboundSendReport {
                validation_error: Some(format!(
                    "serialized physics payload is {} bytes; maximum is {} bytes",
                    data.len(),
                    MAX_PEER_MESSAGE_BYTES
                )),
                ..OutboundSendReport::default()
            };
        }

        self.send_to_admitted_peers(Channel::Unreliable, &data)
    }

    /// Send an authority message to admitted peers and report partial failures.
    pub fn send_authority(&mut self, msg: &AuthorityMessage) -> OutboundSendReport {
        if self.peers.is_empty() {
            return OutboundSendReport::default();
        }

        // A peer can only advertise its own claim. Receivers independently verify
        // this against the transport sender, but reject it here before fanout too.
        if let AuthorityMessage::Claim { peer_id, .. } = msg {
            let local_peer_id = self.transport.local_peer_id().0;
            if *peer_id != local_peer_id {
                return OutboundSendReport {
                    validation_error: Some(format!(
                        "refusing authority Claim for peer {peer_id}; local peer is {local_peer_id}"
                    )),
                    ..OutboundSendReport::default()
                };
            }
        }

        let data = match rmp_serde::to_vec(msg) {
            Ok(data) => data,
            Err(error) => {
                return OutboundSendReport {
                    serialization_error: Some(error.to_string()),
                    ..OutboundSendReport::default()
                };
            }
        };
        if data.len() > MAX_PEER_MESSAGE_BYTES {
            return OutboundSendReport {
                validation_error: Some(format!(
                    "serialized authority payload is {} bytes; maximum is {} bytes",
                    data.len(),
                    MAX_PEER_MESSAGE_BYTES
                )),
                ..OutboundSendReport::default()
            };
        }

        self.send_to_admitted_peers(Channel::Reliable, &data)
    }

    fn send_to_admitted_peers(
        &mut self,
        channel: Channel,
        data: &[u8],
    ) -> OutboundSendReport {
        // Never use transport-wide broadcast here: transports may have links for
        // peers this session refused to admit (for example, over max_peers).
        let admitted_peers: Vec<PeerId> = self.peers.keys().copied().collect();
        let mut report = OutboundSendReport {
            attempted_peers: admitted_peers.len(),
            ..OutboundSendReport::default()
        };

        for peer in admitted_peers {
            match self.transport.send(peer, channel, data) {
                Ok(()) => report.delivered_peers += 1,
                Err(error) => report.failures.push(OutboundSendFailure { peer, error }),
            }
        }

        report
    }

    /// Number of connected peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Whether we're in a multiplayer session.
    pub fn is_multiplayer(&self) -> bool {
        !self.peers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loopback::loopback_pair;

    #[test]
    fn session_join_and_tick() {
        let (a_transport, _b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session = NetworkSession::new(a_transport, config);

        session.join("test-room").unwrap();
        session.tick();
        assert_eq!(session.tick, 1);
    }

    #[test]
    fn session_rejects_peer_admission_after_max_peers_is_reached() {
        let (transport, _other) = loopback_pair();
        let mut config = NetworkConfig::local_test();
        config.max_peers = 1;
        let mut session = NetworkSession::new(transport, config);
        session.join("test").unwrap();

        session
            .transport
            .inject_event(TransportEvent::PeerConnected(PeerId(10)));
        session
            .transport
            .inject_event(TransportEvent::PeerConnected(PeerId(11)));

        // Even a syntactically valid update from the rejected peer must remain
        // outside the session's admitted-member boundary.
        let payload = rmp_serde::to_vec(&PhysicsSync {
            tick: 1,
            authority: 11,
            bodies: Vec::new(),
        })
        .expect("serialize over-limit peer update");
        session
            .transport
            .inject_message(crate::transport::PeerMessage {
                from: PeerId(11),
                channel: Channel::Unreliable,
                data: payload,
            });
        session.tick();

        assert_eq!(session.peer_count(), 1);
        assert!(session.peers.contains_key(&PeerId(10)));
        assert!(
            !session.peers.contains_key(&PeerId(11)),
            "peers over the configured session admission limit must not be admitted"
        );
        assert!(
            session.incoming_physics.is_empty(),
            "updates from peers rejected at admission must not reach the game"
        );
    }

    #[test]
    fn session_sends_only_to_admitted_peers_not_all_transport_links() {
        use crate::iroh_transport::IrohTransport;

        let mut transport = IrohTransport::new(PeerId(0));
        transport.connect("test").unwrap();
        transport.inject_peer_connected(PeerId(10));
        transport.inject_peer_connected(PeerId(11));

        let mut config = NetworkConfig::local_test();
        config.max_peers = 1;
        let mut session = NetworkSession::new(transport, config);
        assert!(
            !session.is_multiplayer(),
            "raw transport links do not make an unadmitted session multiplayer"
        );
        session.peers.insert(
            PeerId(10),
            PeerState::remote(PeerId(10), "admitted".to_string(), 0),
        );

        session.send_physics(vec![BodyStateUpdate {
            body_id: 7,
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
            angular_velocity: [0.0, 0.0, 0.0],
        }]);
        session.send_authority(&AuthorityMessage::Claim {
            body_ids: vec![7],
            peer_id: 0,
        });

        let outbox = session.transport.drain_outbox();
        assert_eq!(outbox.len(), 2, "one physics and one authority packet");
        assert!(
            outbox.iter().all(|(to, _, _)| *to == PeerId(10)),
            "both channels must target admitted peers only"
        );
        assert!(session.is_multiplayer());
    }

    #[test]
    fn disconnect_event_for_local_identity_preserves_local_authority() {
        use symtropy_physics::body::BodyHandle;

        let (transport, _other) = loopback_pair();
        let local_id = transport.local_peer_id();
        let mut session =
            NetworkSession::new(transport, NetworkConfig::local_test());
        let body = BodyHandle(77);
        session.authority.claim(body, local_id);

        session
            .transport
            .inject_event(TransportEvent::PeerDisconnected(local_id));
        session.tick();

        assert_eq!(session.authority.authority_of(body), Some(local_id));
        assert!(session.authority.is_local(body));
    }

    #[test]
    fn send_authority_rejects_claim_for_another_peer() {
        let (transport, _other) = loopback_pair();
        let mut session =
            NetworkSession::new(transport, NetworkConfig::local_test());
        session.peers.insert(
            PeerId(1),
            PeerState::remote(PeerId(1), "remote".to_string(), 0),
        );

        let report = session.send_authority(&AuthorityMessage::Claim {
            body_ids: vec![7],
            peer_id: 999,
        });

        assert!(!report.is_complete());
        assert_eq!(report.attempted_peers, 0);
        assert_eq!(report.delivered_peers, 0);
        assert!(report.failures.is_empty());
        assert!(report.validation_error.unwrap().contains("refusing authority Claim"));
    }

    #[test]
    fn outbound_physics_payload_is_bounded_before_fanout() {
        let (transport, _other) = loopback_pair();
        let mut session =
            NetworkSession::new(transport, NetworkConfig::local_test());
        session.peers.insert(
            PeerId(1),
            PeerState::remote(PeerId(1), "remote".to_string(), 0),
        );

        let body = BodyStateUpdate {
            body_id: 1,
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
            angular_velocity: [0.0, 0.0, 0.0],
        };
        let report = session.send_physics(vec![body; 12_000]);

        assert!(!report.is_complete());
        assert_eq!(report.attempted_peers, 0);
        assert_eq!(report.delivered_peers, 0);
        assert!(report.validation_error.unwrap().contains("maximum is"));
    }

    #[test]
    fn per_peer_send_failures_are_returned_in_delivery_report() {
        let (transport, _other) = loopback_pair();
        let mut session =
            NetworkSession::new(transport, NetworkConfig::local_test());
        let unexpected = PeerId(999);
        session.peers.insert(
            unexpected,
            PeerState::remote(unexpected, "not-the-loopback-peer".to_string(), 0),
        );

        let report = session.send_authority(&AuthorityMessage::Release {
            body_ids: vec![7],
        });

        assert_eq!(report.attempted_peers, 1);
        assert_eq!(report.delivered_peers, 0);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].peer, unexpected);
        assert!(report.failures[0].error.contains("Unknown loopback target"));
    }

    #[test]
    fn session_rejects_local_identity_as_remote_peer() {
        let (transport, _other) = loopback_pair();
        let local_id = transport.local_peer_id();
        let config = NetworkConfig::local_test();
        let mut session = NetworkSession::new(transport, config);

        session
            .transport
            .inject_event(TransportEvent::PeerConnected(local_id));
        session.tick();

        assert!(!session.peers.contains_key(&local_id));
        assert_eq!(session.peer_count(), 0);
    }

    #[test]
    fn zero_max_peers_rejects_all_remote_admissions() {
        let (transport, _other) = loopback_pair();
        let mut config = NetworkConfig::local_test();
        config.max_peers = 0;
        let mut session = NetworkSession::new(transport, config);

        session
            .transport
            .inject_event(TransportEvent::PeerConnected(PeerId(10)));
        session.tick();

        assert_eq!(session.peer_count(), 0);
    }

    #[test]
    fn duplicate_peer_connected_event_preserves_liveness_state() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();
        session_b.tick();

        session_b
            .peers
            .get_mut(&PeerId(0))
            .expect("peer admitted")
            .mark_seen(session_b.tick);
        let last_seen = session_b.peers[&PeerId(0)].core.last_seen_tick;

        session_b
            .transport
            .inject_event(TransportEvent::PeerConnected(PeerId(0)));
        session_b.tick();

        assert_eq!(
            session_b.peers[&PeerId(0)].core.last_seen_tick,
            last_seen,
            "duplicate admission must not reset peer liveness"
        );
    }

    #[test]
    fn session_tick_tolerates_zero_configured_send_rate() {
        let (transport, _peer) = loopback_pair();
        let mut config = NetworkConfig::local_test();
        config.send_rate_hz = 0;
        let mut session = NetworkSession::new(transport, config);

        session.join("test").unwrap();
        session.tick();

        assert_eq!(session.tick, 1);
    }

    #[test]
    fn session_releases_remote_authority_on_peer_disconnect() {
        use symtropy_physics::body::BodyHandle;

        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();
        session_b.tick();
        assert!(session_b.peers.contains_key(&PeerId(0)));

        let remote_body = BodyHandle(42);
        session_b.authority.claim(remote_body, PeerId(0));
        assert_eq!(session_b.authority.authority_of(remote_body), Some(PeerId(0)));

        session_a.leave();
        session_b.tick();

        assert!(!session_b.peers.contains_key(&PeerId(0)));
        assert_eq!(session_b.authority.authority_of(remote_body), None);
    }

    #[test]
    fn session_drops_messages_from_unknown_peers() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();

        let payload = rmp_serde::to_vec(&PhysicsSync {
            tick: 1,
            authority: 99,
            bodies: vec![BodyStateUpdate {
                body_id: 1,
                position: [1.0, 2.0, 3.0],
                velocity: [0.0, 0.0, 0.0],
                rotation: [1.0, 0.0, 0.0, 0.0],
                angular_velocity: [0.0, 0.0, 0.0],
            }],
        })
        .expect("serialize physics sync");
        session_b.transport.inject_message(crate::transport::PeerMessage {
            from: PeerId(99),
            channel: Channel::Unreliable,
            data: payload,
        });

        session_b.tick();
        assert!(session_b.incoming_physics.is_empty());
    }

    #[test]
    fn session_drops_physics_sync_with_mismatched_authority() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();

        let payload = rmp_serde::to_vec(&PhysicsSync {
            tick: 1,
            authority: 99,
            bodies: vec![BodyStateUpdate {
                body_id: 1,
                position: [1.0, 2.0, 3.0],
                velocity: [0.0, 0.0, 0.0],
                rotation: [1.0, 0.0, 0.0, 0.0],
                angular_velocity: [0.0, 0.0, 0.0],
            }],
        })
        .expect("serialize physics sync");
        session_b.transport.inject_message(crate::transport::PeerMessage {
            from: PeerId(0),
            channel: Channel::Unreliable,
            data: payload,
        });

        session_b.tick();
        assert!(session_b.incoming_physics.is_empty());
    }

    #[test]
    fn session_drops_oversized_peer_payload_before_decode() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();

        session_b.transport.inject_message(crate::transport::PeerMessage {
            from: PeerId(0),
            channel: Channel::Unreliable,
            data: vec![0; MAX_PEER_MESSAGE_BYTES + 1],
        });

        session_b.tick();
        assert!(session_b.incoming_physics.is_empty());
    }

    #[test]
    fn session_send_receive_physics() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();

        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();
        session_a.tick(); // process peer admission before sending to session members

        // A sends physics
        session_a.send_physics(vec![BodyStateUpdate {
            body_id: 1,
            position: [10.0, 20.0, 0.0],
            velocity: [1.0, 0.0, 0.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
            angular_velocity: [0.0, 0.0, 0.0],
        }]);

        // B receives on next tick
        session_b.tick();
        assert_eq!(session_b.incoming_physics.len(), 1);
        assert_eq!(session_b.incoming_physics[0].bodies[0].body_id, 1);
        assert_eq!(session_b.incoming_physics[0].bodies[0].position[0], 10.0);
    }

    #[test]
    fn session_drops_authority_claim_that_impersonates_another_peer() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();

        let payload = rmp_serde::to_vec(&AuthorityMessage::Claim {
            body_ids: vec![42],
            peer_id: 99,
        })
        .expect("serialize spoofed claim");
        session_b.transport.inject_message(crate::transport::PeerMessage {
            from: PeerId(0),
            channel: Channel::Reliable,
            data: payload,
        });

        session_b.tick();
        assert!(session_b.incoming_authority.is_empty());
    }

    #[test]
    fn session_send_receive_authority() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();

        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();
        session_a.tick(); // process peer admission before sending to session members

        // A claims authority
        session_a.send_authority(&AuthorityMessage::Claim {
            body_ids: vec![1, 2, 3],
            peer_id: 0,
        });

        // B receives
        session_b.tick();
        assert_eq!(session_b.incoming_authority.len(), 1);
        assert_eq!(session_b.incoming_authority[0].from, PeerId(0));
        match &session_b.incoming_authority[0].message {
            AuthorityMessage::Claim { body_ids, peer_id } => {
                assert_eq!(body_ids, &[1, 2, 3]);
                assert_eq!(*peer_id, 0);
            }
            _ => panic!("Expected Claim"),
        }
    }
}
