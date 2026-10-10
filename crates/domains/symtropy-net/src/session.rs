// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Network session — ties transport, authority, and state sync together.
//!
//! The game loop calls `session.tick()` each frame, which:
//! 1. Polls the transport for incoming messages
//! 2. Applies received physics state to remote bodies
//! 3. Sends local authority bodies' state to peers
//! 4. Handles authority claim/release messages

use std::collections::{HashMap, HashSet};

use crate::authority::SpatialAuthority;
use crate::config::NetworkConfig;
use crate::peer::{PeerId, PeerState};
use crate::transport::{
    AuthorityMessage, BodyStateUpdate, Channel, PhysicsSync, Transport, TransportEvent,
    MAX_AUTHORITY_BODY_IDS, MAX_AUTHORITY_REASON_BYTES, MAX_PEER_MESSAGE_BYTES,
};

/// Reject non-finite components and duplicate body IDs before updating session
/// freshness state. Domain-specific bounds remain the authoritative simulation's job.
fn is_valid_physics_sync(sync: &PhysicsSync) -> bool {
    let mut body_ids = HashSet::with_capacity(sync.bodies.len());
    sync.bodies.iter().all(|body| {
        let finite_components = body
            .position
            .iter()
            .chain(body.velocity.iter())
            .chain(body.rotation.iter())
            .chain(body.angular_velocity.iter())
            .all(|component| component.is_finite());

        finite_components && body_ids.insert(body.body_id)
    })
}

/// Validate resource bounds and structural invariants for authority messages.
///
/// The session intentionally does not decide whether the sender is authorized to
/// claim/release these bodies; it only bounds parsing and preserves sender identity
/// for the application-level permission check.
fn authority_message_validation_error(message: &AuthorityMessage) -> Option<String> {
    let (body_ids, reason): (&[u32], Option<&str>) = match message {
        AuthorityMessage::Claim { body_ids, .. } | AuthorityMessage::Release { body_ids } => {
            (body_ids, None)
        }
        AuthorityMessage::RequestTransfer { body_ids, reason } => (body_ids, Some(reason.as_str())),
    };

    if body_ids.len() > MAX_AUTHORITY_BODY_IDS {
        return Some(format!(
            "authority message contains {} body IDs; maximum is {}",
            body_ids.len(),
            MAX_AUTHORITY_BODY_IDS
        ));
    }
    if let Some(reason) = reason.filter(|reason| reason.len() > MAX_AUTHORITY_REASON_BYTES) {
        return Some(format!(
            "authority-transfer reason is {} bytes; maximum is {}",
            reason.len(),
            MAX_AUTHORITY_REASON_BYTES
        ));
    }

    let mut unique_ids = HashSet::with_capacity(body_ids.len());
    if !body_ids.iter().all(|body_id| unique_ids.insert(*body_id)) {
        return Some("authority message contains duplicate body IDs".to_string());
    }

    None
}

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
/// This reports local transport acceptance, not remote receipt or application.
/// Partial acceptance is possible: accepted_by_transport may be non-zero while
/// failures is non-empty. Reliable authority state still needs protocol-level
/// acknowledgement if the caller requires proof that a remote peer applied it.
#[derive(Debug, Clone, Default)]
pub struct OutboundSendReport {
    pub attempted_peers: usize,
    pub accepted_by_transport: usize,
    pub failures: Vec<OutboundSendFailure>,
    pub validation_error: Option<String>,
    pub serialization_error: Option<String>,
}

impl OutboundSendReport {
    /// Whether all intended local send attempts were accepted without errors.
    ///
    /// This is not a remote-delivery acknowledgement. A report with no admitted
    /// recipients is a successful no-op with zero attempted sends.
    pub fn is_complete(&self) -> bool {
        self.validation_error.is_none()
            && self.serialization_error.is_none()
            && self.failures.is_empty()
            && self.attempted_peers == self.accepted_by_transport
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
    /// Most recent accepted remote physics snapshot tick, scoped to the current peer session.
    latest_physics_tick: HashMap<PeerId, u64>,
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
            latest_physics_tick: HashMap::new(),
            tick: 0,
            incoming_physics: Vec::new(),
            incoming_authority: Vec::new(),
        }
    }

    /// Join a room using the synchronous path.
    ///
    /// Async transports fail closed here. Use `join_async` for transports
    /// that require network I/O and a remote handshake.
    pub fn join(&mut self, room_id: &str) -> Result<(), String> {
        self.transport.connect(room_id)
    }

    /// Start joining a room through the transport's async connection path.
    ///
    /// A successful return means the transport's connection/join request was
    /// accepted by its local async API. For asynchronous transports, call `tick()`
    /// and then inspect `is_connected()`; returning from this method does not
    /// prove that a remote server has completed its Welcome handshake.
    pub async fn join_async(&mut self, room_id: &str) -> Result<(), String> {
        self.transport.connect_async(room_id).await
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
        self.latest_physics_tick.remove(&peer_id);
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
        self.latest_physics_tick.clear();
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
                                if sync.authority != msg.from.0 || !is_valid_physics_sync(&sync) {
                                    continue;
                                }

                                // Unreliable snapshots may be delayed or reordered.
                                // Never let an older or duplicate tick overwrite newer state.
                                if self
                                    .latest_physics_tick
                                    .get(&msg.from)
                                    .is_some_and(|last_tick| sync.tick <= *last_tick)
                                {
                                    continue;
                                }
                                self.latest_physics_tick.insert(msg.from, sync.tick);

                                if let Some(peer) = self.peers.get_mut(&msg.from) {
                                    peer.mark_seen(self.tick);
                                }
                                self.incoming_physics.push(sync);
                            }
                        }
                        Channel::Reliable => {
                            if let Ok(auth) = rmp_serde::from_slice::<AuthorityMessage>(&msg.data) {
                                // Bound operation size and reject ambiguous duplicate body IDs
                                // before marking this payload as valid peer activity.
                                if authority_message_validation_error(&auth).is_some() {
                                    continue;
                                }

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

    /// Send local physics state to all admitted peers.
    ///
    /// Call after computing physics for bodies the local peer has authority over.
    /// Returns a per-peer send report; acceptance by the local transport is not proof of remote receipt. Over-budget payloads are rejected before fanout.
    pub fn send_physics(&mut self, bodies: Vec<BodyStateUpdate>) -> OutboundSendReport {
        if bodies.is_empty() || self.peers.is_empty() {
            return OutboundSendReport::default();
        }

        let sync = PhysicsSync {
            tick: self.tick,
            authority: self.transport.local_peer_id().0,
            bodies,
        };

        // Apply the same structural invariants locally that receivers enforce.
        // Do not emit packets every compliant peer will discard.
        if !is_valid_physics_sync(&sync) {
            return OutboundSendReport {
                validation_error: Some(
                    "physics payload contains non-finite components or duplicate body IDs"
                        .to_string(),
                ),
                ..OutboundSendReport::default()
            };
        }

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

        // Bound resource use and keep authority payload shape deterministic before
        // serializing or fanning out to any admitted peer.
        if let Some(error) = authority_message_validation_error(msg) {
            return OutboundSendReport {
                validation_error: Some(error),
                ..OutboundSendReport::default()
            };
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

    fn send_to_admitted_peers(&mut self, channel: Channel, data: &[u8]) -> OutboundSendReport {
        // Never use transport-wide broadcast here: transports may have links for
        // peers this session refused to admit (for example, over max_peers).
        let admitted_peers: Vec<PeerId> = self.peers.keys().copied().collect();
        let mut report = OutboundSendReport {
            attempted_peers: admitted_peers.len(),
            ..OutboundSendReport::default()
        };

        for peer in admitted_peers {
            match self.transport.send(peer, channel, data) {
                Ok(()) => report.accepted_by_transport += 1,
                Err(error) => report.failures.push(OutboundSendFailure { peer, error }),
            }
        }

        report
    }

    /// Number of connected peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Whether the underlying transport reports its connection/control-plane state as established.
    ///
    /// This is distinct from `is_multiplayer()` and remote delivery. The exact state
    /// is transport-specific: relay transports require Welcome to be processed by
    /// `tick()`, while an in-memory transport may report its local endpoint connected.
    /// This is not a gameplay-authorization signal.
    pub fn is_connected(&self) -> bool {
        self.transport.is_signaling_connected()
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
    fn session_connection_state_is_separate_from_peer_admission() {
        let (transport, _peer) = loopback_pair();
        let mut session = NetworkSession::new(transport, NetworkConfig::local_test());

        assert!(!session.is_connected());
        assert!(!session.is_multiplayer());

        session.join("test-room").expect("connect local transport");
        assert!(session.is_connected());
        assert!(
            !session.is_multiplayer(),
            "a local transport connection is not proof that a remote peer was admitted"
        );
    }

    #[test]
    fn session_async_join_supports_synchronous_transports() {
        let (transport, _peer) = loopback_pair();
        let mut session = NetworkSession::new(transport, NetworkConfig::local_test());

        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(session.join_async("test-room"))
            .expect("async join delegates to sync transport");

        assert!(session.is_connected());
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
        let mut session = NetworkSession::new(transport, NetworkConfig::local_test());
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
        let mut session = NetworkSession::new(transport, NetworkConfig::local_test());
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
        assert_eq!(report.accepted_by_transport, 0);
        assert!(report.failures.is_empty());
        assert!(report.validation_error.unwrap().contains("refusing authority Claim"));
    }

    #[test]
    fn outbound_physics_payload_is_bounded_before_fanout() {
        let (transport, _other) = loopback_pair();
        let mut session = NetworkSession::new(transport, NetworkConfig::local_test());
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
        assert_eq!(report.accepted_by_transport, 0);
        assert!(report.validation_error.unwrap().contains("maximum is"));
    }

    #[test]
    fn outbound_physics_rejects_nonfinite_and_duplicate_body_updates_before_fanout() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();
        session_a.tick(); // admit the receiver before attempting outbound delivery

        let body = BodyStateUpdate {
            body_id: 7,
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
            angular_velocity: [0.0, 0.0, 0.0],
        };
        let mut nonfinite = body.clone();
        nonfinite.position[0] = f64::NAN;

        let nonfinite_report = session_a.send_physics(vec![nonfinite]);
        assert!(!nonfinite_report.is_complete());
        assert_eq!(nonfinite_report.attempted_peers, 0);
        assert_eq!(nonfinite_report.accepted_by_transport, 0);
        assert!(nonfinite_report
            .validation_error
            .as_deref()
            .is_some_and(|error| error.contains("non-finite")));

        let duplicate_report = session_a.send_physics(vec![body.clone(), body]);
        assert!(!duplicate_report.is_complete());
        assert_eq!(duplicate_report.attempted_peers, 0);
        assert_eq!(duplicate_report.accepted_by_transport, 0);
        assert!(duplicate_report
            .validation_error
            .as_deref()
            .is_some_and(|error| error.contains("duplicate body IDs")));

        // The peer may observe its admission event, but neither invalid state
        // packet must cross the transport boundary.
        let events = session_b.transport.poll();
        assert!(!events
            .iter()
            .any(|event| matches!(event, TransportEvent::Message(_))));
    }

    #[test]
    fn per_peer_send_failures_are_returned_in_delivery_report() {
        let (transport, _other) = loopback_pair();
        let mut session = NetworkSession::new(transport, NetworkConfig::local_test());
        let unexpected = PeerId(999);
        session.peers.insert(
            unexpected,
            PeerState::remote(unexpected, "not-the-loopback-peer".to_string(), 0),
        );

        let report = session.send_authority(&AuthorityMessage::Release {
            body_ids: vec![7],
        });

        assert_eq!(report.attempted_peers, 1);
        assert_eq!(report.accepted_by_transport, 0);
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
    fn session_drops_duplicate_and_out_of_order_physics_snapshots() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();
        session_b.tick(); // admit peer before receiving its state

        for remote_tick in [12, 10, 12] {
            let payload = rmp_serde::to_vec(&PhysicsSync {
                tick: remote_tick,
                authority: 0,
                bodies: vec![BodyStateUpdate {
                    body_id: 1,
                    position: [remote_tick as f64, 0.0, 0.0],
                    velocity: [0.0, 0.0, 0.0],
                    rotation: [1.0, 0.0, 0.0, 0.0],
                    angular_velocity: [0.0, 0.0, 0.0],
                }],
            })
            .expect("serialize physics snapshot");
            session_b
                .transport
                .inject_message(crate::transport::PeerMessage {
                    from: PeerId(0),
                    channel: Channel::Unreliable,
                    data: payload,
                });
        }

        session_b.tick();

        assert_eq!(session_b.incoming_physics.len(), 1);
        assert_eq!(session_b.incoming_physics[0].tick, 12);
        assert_eq!(session_b.incoming_physics[0].bodies[0].position[0], 12.0);
    }

    #[test]
    fn session_rejects_nonfinite_and_duplicate_body_updates_without_advancing_tick() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();
        session_b.tick();

        let body = |body_id: u32, position: [f64; 3]| BodyStateUpdate {
            body_id,
            position,
            velocity: [0.0, 0.0, 0.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
            angular_velocity: [0.0, 0.0, 0.0],
        };
        let snapshots = vec![
            PhysicsSync {
                tick: 7,
                authority: 0,
                bodies: vec![body(1, [f64::NAN, 0.0, 0.0])],
            },
            PhysicsSync {
                tick: 7,
                authority: 0,
                bodies: vec![body(1, [1.0, 0.0, 0.0])],
            },
            PhysicsSync {
                tick: 8,
                authority: 0,
                bodies: vec![body(2, [1.0, 0.0, 0.0]), body(2, [2.0, 0.0, 0.0])],
            },
            PhysicsSync {
                tick: 8,
                authority: 0,
                bodies: vec![body(2, [2.0, 0.0, 0.0])],
            },
        ];

        for sync in snapshots {
            let data = rmp_serde::to_vec(&sync).expect("serialize physics snapshot");
            session_b.transport.inject_message(crate::transport::PeerMessage {
                from: PeerId(0),
                channel: Channel::Unreliable,
                data,
            });
        }
        session_b.tick();

        assert_eq!(session_b.incoming_physics.len(), 2);
        assert_eq!(
            session_b
                .incoming_physics
                .iter()
                .map(|sync| sync.tick)
                .collect::<Vec<_>>(),
            vec![7, 8]
        );
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
    fn session_drops_authority_message_over_body_id_limit() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();

        let payload = rmp_serde::to_vec(&AuthorityMessage::Release {
            body_ids: (0..=MAX_AUTHORITY_BODY_IDS as u32).collect(),
        })
        .expect("serialize oversized authority operation");
        assert!(payload.len() <= MAX_PEER_MESSAGE_BYTES);

        session_b.transport.inject_message(crate::transport::PeerMessage {
            from: PeerId(0),
            channel: Channel::Reliable,
            data: payload,
        });

        session_b.tick();
        assert!(
            session_b.incoming_authority.is_empty(),
            "oversized authority operation must not reach the application"
        );
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
    fn authority_message_validation_bounds_ids_reason_and_duplicates() {
        assert!(authority_message_validation_error(&AuthorityMessage::Claim {
            body_ids: (0..=MAX_AUTHORITY_BODY_IDS as u32).collect(),
            peer_id: 1,
        })
        .is_some());

        assert!(authority_message_validation_error(&AuthorityMessage::Release {
            body_ids: vec![7, 7],
        })
        .is_some());

        assert!(authority_message_validation_error(&AuthorityMessage::RequestTransfer {
            body_ids: vec![1],
            reason: "r".repeat(MAX_AUTHORITY_REASON_BYTES + 1),
        })
        .is_some());

        assert!(authority_message_validation_error(&AuthorityMessage::Claim {
            body_ids: (0..MAX_AUTHORITY_BODY_IDS as u32).collect(),
            peer_id: 1,
        })
        .is_none());

        assert!(authority_message_validation_error(&AuthorityMessage::RequestTransfer {
            body_ids: vec![1, 2, 3],
            reason: "r".repeat(MAX_AUTHORITY_REASON_BYTES),
        })
        .is_none());
    }

    #[test]
    fn session_rejects_invalid_authority_shape_before_fanout() {
        let (a_transport, b_transport) = loopback_pair();
        let config = NetworkConfig::local_test();
        let mut session_a = NetworkSession::new(a_transport, config.clone());
        let mut session_b = NetworkSession::new(b_transport, config);

        session_a.join("test").unwrap();
        session_b.join("test").unwrap();
        session_a.tick(); // Admit the loopback peer before testing the outbound guard.

        let report = session_a.send_authority(&AuthorityMessage::Release {
            body_ids: vec![9, 9],
        });
        assert_eq!(report.attempted_peers, 0);
        assert_eq!(report.accepted_by_transport, 0);
        assert!(report.failures.is_empty());
        assert!(report
            .validation_error
            .as_deref()
            .is_some_and(|message| message.contains("duplicate body IDs")));
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
