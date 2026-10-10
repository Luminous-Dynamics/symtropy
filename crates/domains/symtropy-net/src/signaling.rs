// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Signaling client — WebSocket connection for WebRTC session negotiation.
//!
//! The signaling server is a simple WebSocket relay that forwards messages
//! between peers in a room. It doesn't need to understand the content —
//! it just routes messages by peer ID.
//!
//! Protocol:
//! - Client sends: `{"type": "join", "room": "room-id"}`
//! - Server sends: `{"type": "peer_joined", "peer_id": 42}`
//! - Client sends: `{"type": "signal", "to": 42, "data": {...}}`
//! - Server forwards to peer 42: `{"type": "signal", "from": 1, "data": {...}}`

#[cfg(feature = "webrtc")]
use tokio::sync::mpsc;

use serde::{Deserialize, Serialize};

use crate::peer::PeerId;
use crate::transport::MAX_PEER_MESSAGE_BYTES;

/// Maximum WebSocket signaling envelope size. The relay serializes the binary
/// packet as a JSON byte array (up to four characters per byte) and then nests
/// that JSON in an outer signaling message. Keep headroom for envelope metadata.
const MAX_SIGNALING_MESSAGE_BYTES: usize = (4 * MAX_PEER_MESSAGE_BYTES) + 16 * 1024;

/// Messages sent TO the signaling server.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SignalOutgoing {
    /// Join a room.
    #[serde(rename = "join")]
    Join { room: String },
    /// Leave the room.
    #[serde(rename = "leave")]
    Leave,
    /// Send a signaling message to a specific peer.
    #[serde(rename = "signal")]
    Signal { to: u64, data: SignalData },
}

/// Messages received FROM the signaling server.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum SignalIncoming {
    /// Server assigned us an ID.
    #[serde(rename = "welcome")]
    Welcome { peer_id: u64 },
    /// A peer joined the room.
    #[serde(rename = "peer_joined")]
    PeerJoined { peer_id: u64 },
    /// A peer left the room.
    #[serde(rename = "peer_left")]
    PeerLeft { peer_id: u64 },
    /// A signaling message from another peer.
    #[serde(rename = "signal")]
    Signal { from: u64, data: SignalData },
    /// Error from the server.
    #[serde(rename = "error")]
    Error { message: String },
}

/// Signaling data exchanged between peers for WebRTC negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SignalData {
    /// SDP offer.
    #[serde(rename = "offer")]
    Offer { sdp: String },
    /// SDP answer.
    #[serde(rename = "answer")]
    Answer { sdp: String },
    /// ICE candidate.
    #[serde(rename = "ice")]
    IceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    },
}

/// Events produced by the signaling client for the transport layer.
#[derive(Debug, Clone)]
pub enum SignalingEvent {
    /// We received our peer ID from the server.
    Connected(PeerId),
    /// A peer joined our room — we should initiate a WebRTC connection.
    PeerJoined(PeerId),
    /// A peer left our room — clean up their connection.
    PeerLeft(PeerId),
    /// Received signaling data from a peer (offer/answer/ICE).
    Signal { from: PeerId, data: SignalData },
    /// Connection to signaling server lost.
    Disconnected,
    /// Error.
    Error(String),
}

/// Bound WebSocket allocation before signaling JSON is parsed.
#[cfg(feature = "webrtc")]
fn bounded_websocket_config() -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
    tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(MAX_SIGNALING_MESSAGE_BYTES))
        .max_frame_size(Some(MAX_SIGNALING_MESSAGE_BYTES))
}

/// Signaling client handle — used by the transport to send/receive signaling messages.
///
/// The actual WebSocket connection runs in a tokio task.
/// This handle provides a channel-based interface.
#[cfg(feature = "webrtc")]
pub struct SignalingClient {
    /// Send commands to the signaling task.
    pub tx: mpsc::UnboundedSender<SignalOutgoing>,
    /// Receive events from the signaling task.
    pub rx: mpsc::UnboundedReceiver<SignalingEvent>,
    /// Our peer ID (set after Welcome message).
    pub local_id: Option<PeerId>,
}

#[cfg(feature = "webrtc")]
impl SignalingClient {
    /// Connect to the signaling server and spawn the WebSocket task.
    pub async fn connect(url: &str) -> Result<Self, String> {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::connect_async_with_config;

        let (ws_stream, _) = connect_async_with_config(url, Some(bounded_websocket_config()), false)
            .await
            .map_err(|e| format!("Signaling connect failed: {e}"))?;

        let (mut ws_tx, mut ws_rx) = ws_stream.split();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SignalOutgoing>();
        let (evt_tx, evt_rx) = mpsc::unbounded_channel::<SignalingEvent>();

        // Spawn task: forward outgoing commands to WebSocket
        let evt_tx_clone = evt_tx.clone();
        tokio::spawn(async move {
            // Forward commands to WebSocket
            let send_task = async {
                while let Some(cmd) = cmd_rx.recv().await {
                    let json = serde_json::to_string(&cmd).unwrap();
                    if ws_tx
                        .send(tokio_tungstenite::tungstenite::Message::Text(json))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            };

            // Receive from WebSocket and parse
            let recv_task = async {
                while let Some(Ok(msg)) = ws_rx.next().await {
                    if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                        // Defense in depth in addition to tungstenite's frame/message limits.
                        if text.len() > MAX_SIGNALING_MESSAGE_BYTES {
                            log::warn!(
                                "Signaling message rejected: {} bytes exceeds limit {}",
                                text.len(),
                                MAX_SIGNALING_MESSAGE_BYTES
                            );
                            continue;
                        }
                        match serde_json::from_str::<SignalIncoming>(&text) {
                            Ok(SignalIncoming::Welcome { peer_id }) => {
                                let _ =
                                    evt_tx_clone.send(SignalingEvent::Connected(PeerId(peer_id)));
                            }
                            Ok(SignalIncoming::PeerJoined { peer_id }) => {
                                let _ =
                                    evt_tx_clone.send(SignalingEvent::PeerJoined(PeerId(peer_id)));
                            }
                            Ok(SignalIncoming::PeerLeft { peer_id }) => {
                                let _ =
                                    evt_tx_clone.send(SignalingEvent::PeerLeft(PeerId(peer_id)));
                            }
                            Ok(SignalIncoming::Signal { from, data }) => {
                                let _ = evt_tx_clone.send(SignalingEvent::Signal {
                                    from: PeerId(from),
                                    data,
                                });
                            }
                            Ok(SignalIncoming::Error { message }) => {
                                let _ = evt_tx_clone.send(SignalingEvent::Error(message));
                            }
                            Err(e) => {
                                log::warn!("Signaling parse error: {e}");
                            }
                        }
                    }
                }
                let _ = evt_tx_clone.send(SignalingEvent::Disconnected);
            };

            tokio::select! {
                _ = send_task => {},
                _ = recv_task => {},
            }
        });

        Ok(Self {
            tx: cmd_tx,
            rx: evt_rx,
            local_id: None,
        })
    }

    /// Join a room.
    pub fn join(&self, room: &str) -> Result<(), String> {
        self.tx
            .send(SignalOutgoing::Join {
                room: room.to_string(),
            })
            .map_err(|e| format!("Send failed: {e}"))
    }

    /// Send signaling data to a peer.
    pub fn signal(&self, to: PeerId, data: SignalData) -> Result<(), String> {
        self.tx
            .send(SignalOutgoing::Signal { to: to.0, data })
            .map_err(|e| format!("Send failed: {e}"))
    }

    /// Poll for events (non-blocking).
    pub fn poll_events(&mut self) -> Vec<SignalingEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.rx.try_recv() {
            if let SignalingEvent::Connected(id) = &evt {
                self.local_id = Some(*id);
            }
            events.push(evt);
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct RelayEnvelope {
        channel: u8,
        payload: Vec<u8>,
    }

    #[test]
    fn maximum_peer_payload_fits_the_bounded_nested_json_envelope() {
        let relay_json = serde_json::to_string(&RelayEnvelope {
            channel: 0,
            payload: vec![u8::MAX; MAX_PEER_MESSAGE_BYTES],
        })
        .expect("serialize relay envelope");
        let outgoing = SignalOutgoing::Signal {
            to: 1,
            data: SignalData::Offer { sdp: relay_json },
        };
        let signaling_json =
            serde_json::to_string(&outgoing).expect("serialize outer signaling message");

        assert!(
            signaling_json.len() <= MAX_SIGNALING_MESSAGE_BYTES,
            "maximum valid peer packet expanded to {} bytes, over the {} byte signaling cap",
            signaling_json.len(),
            MAX_SIGNALING_MESSAGE_BYTES
        );
    }

    #[cfg(feature = "webrtc")]
    #[test]
    fn websocket_config_enforces_frame_and_message_limits() {
        let config = bounded_websocket_config();
        assert_eq!(config.max_message_size, Some(MAX_SIGNALING_MESSAGE_BYTES));
        assert_eq!(config.max_frame_size, Some(MAX_SIGNALING_MESSAGE_BYTES));
    }
}
