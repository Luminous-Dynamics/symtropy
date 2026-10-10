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
//!
//! Resource bounds in this implementation:
//! - 16 queued outgoing commands and 32 queued incoming events.
//! - 64 KiB maximum WebSocket message/frame and a bounded 256 KiB write buffer.
//! - Queue overflow is surfaced as an error; it is never reported as successful enqueue.

#[cfg(feature = "webrtc")]
use tokio::sync::mpsc;

use serde::{Deserialize, Serialize};

use crate::peer::PeerId;

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

#[cfg(feature = "webrtc")]
const SIGNAL_COMMAND_QUEUE_CAPACITY: usize = 16;
#[cfg(feature = "webrtc")]
const SIGNAL_EVENT_QUEUE_CAPACITY: usize = 32;
#[cfg(feature = "webrtc")]
const MAX_SIGNAL_MESSAGE_BYTES: usize = 64 * 1024;

#[cfg(feature = "webrtc")]
fn enqueue_command(
    tx: &mpsc::Sender<SignalOutgoing>,
    command: SignalOutgoing,
) -> Result<(), String> {
    // Reject obviously oversized caller-owned strings before serializing them.
    // JSON escaping can increase size, so perform an exact envelope-size check next.
    let largest_field = match &command {
        SignalOutgoing::Join { room } => room.len(),
        SignalOutgoing::Leave => 0,
        SignalOutgoing::Signal { data, .. } => match data {
            SignalData::Offer { sdp } | SignalData::Answer { sdp } => sdp.len(),
            SignalData::IceCandidate {
                candidate, sdp_mid, ..
            } => candidate
                .len()
                .saturating_add(sdp_mid.as_deref().map_or(0, str::len)),
        },
    };
    if largest_field > MAX_SIGNAL_MESSAGE_BYTES {
        return Err(format!(
            "signaling payload field is {largest_field} bytes; maximum is {MAX_SIGNAL_MESSAGE_BYTES} bytes"
        ));
    }

    let encoded = serde_json::to_vec(&command)
        .map_err(|error| format!("failed to serialize signaling command: {error}"))?;
    if encoded.len() > MAX_SIGNAL_MESSAGE_BYTES {
        return Err(format!(
            "signaling message is {} bytes; maximum is {} bytes",
            encoded.len(),
            MAX_SIGNAL_MESSAGE_BYTES
        ));
    }

    tx.try_send(command).map_err(|error| match error {
        mpsc::error::TrySendError::Full(_) => format!(
            "signaling command queue is full (capacity {SIGNAL_COMMAND_QUEUE_CAPACITY}); command rejected"
        ),
        mpsc::error::TrySendError::Closed(_) => "signaling command queue is closed".to_string(),
    })
}

/// Signaling client handle — used by the transport to send/receive signaling messages.
///
/// Both directions use bounded mailboxes. A full command queue returns an error to
/// the caller; a full event queue backpressures WebSocket reads instead of growing
/// memory without limit.

#[cfg(feature = "webrtc")]
pub struct SignalingClient {
    /// Send commands to the signaling task.
    pub tx: mpsc::Sender<SignalOutgoing>,
    /// Receive events from the signaling task.
    pub rx: mpsc::Receiver<SignalingEvent>,
    /// Our peer ID (set after Welcome message).
    pub local_id: Option<PeerId>,
}

#[cfg(all(test, feature = "webrtc"))]
mod tests {
    use super::*;

    #[test]
    fn bounded_command_queue_rejects_overflow_explicitly() {
        let (tx, _rx) = mpsc::channel(SIGNAL_COMMAND_QUEUE_CAPACITY);

        for index in 0..SIGNAL_COMMAND_QUEUE_CAPACITY {
            enqueue_command(
                &tx,
                SignalOutgoing::Join {
                    room: format!("room-{index}"),
                },
            )
            .expect("capacity permits queueing the command");
        }

        let error = enqueue_command(&tx, SignalOutgoing::Leave)
            .expect_err("full queue must reject instead of growing");

        assert!(error.contains("queue is full"));
    }

    #[test]
    fn oversized_outgoing_signal_is_rejected_before_enqueue() {
        let (tx, _rx) = mpsc::channel(SIGNAL_COMMAND_QUEUE_CAPACITY);
        let command = SignalOutgoing::Signal {
            to: 42,
            data: SignalData::Offer {
                sdp: "x".repeat(MAX_SIGNAL_MESSAGE_BYTES + 1),
            },
        };

        let error = enqueue_command(&tx, command)
            .expect_err("oversized signal must be rejected before enqueue");

        assert!(error.contains("maximum is"));
        assert_eq!(tx.capacity(), SIGNAL_COMMAND_QUEUE_CAPACITY);
    }

    #[test]
    fn bounded_event_queue_capacity_is_finite() {
        let (_tx, mut rx) = mpsc::channel::<SignalingEvent>(SIGNAL_EVENT_QUEUE_CAPACITY);
        assert_eq!(rx.max_capacity(), SIGNAL_EVENT_QUEUE_CAPACITY);
    }
}

#[cfg(feature = "webrtc")]
impl SignalingClient {
    /// Connect to the signaling server and spawn the WebSocket task.
    pub async fn connect(url: &str) -> Result<Self, String> {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::{
            connect_async_with_config, tungstenite::protocol::WebSocketConfig,
        };

        let ws_config = WebSocketConfig {
            max_message_size: Some(MAX_SIGNAL_MESSAGE_BYTES),
            max_frame_size: Some(MAX_SIGNAL_MESSAGE_BYTES),
            max_write_buffer_size: 256 * 1024,
            ..WebSocketConfig::default()
        };
        let (ws_stream, _) = connect_async_with_config(url, Some(ws_config), false)
            .await
            .map_err(|e| format!("Signaling connect failed: {e}"))?;

        let (mut ws_tx, mut ws_rx) = ws_stream.split();
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<SignalOutgoing>(SIGNAL_COMMAND_QUEUE_CAPACITY);
        let (evt_tx, evt_rx) = mpsc::channel::<SignalingEvent>(SIGNAL_EVENT_QUEUE_CAPACITY);

        // The event buffer is bounded; if the ECS caller falls behind, receive-side
        // sends await capacity and naturally apply backpressure to WebSocket reads.
        let task_event_tx = evt_tx.clone();
        tokio::spawn(async move {
            let send_event_tx = task_event_tx.clone();
            let recv_event_tx = task_event_tx.clone();

            let send_task = async {
                while let Some(cmd) = cmd_rx.recv().await {
                    let json = match serde_json::to_string(&cmd) {
                        Ok(json) => json,
                        Err(error) => {
                            if send_event_tx
                                .send(SignalingEvent::Error(format!(
                                    "failed to serialize signaling command: {error}"
                                )))
                                .await
                                .is_err()
                            {
                                return;
                            }
                            continue;
                        }
                    };

                    if let Err(error) = ws_tx
                        .send(tokio_tungstenite::tungstenite::Message::Text(json))
                        .await
                    {
                        let _ = send_event_tx
                            .send(SignalingEvent::Error(format!(
                                "failed to write signaling WebSocket: {error}"
                            )))
                            .await;
                        break;
                    }
                }
            };

            let recv_task = async {
                while let Some(message_result) = ws_rx.next().await {
                    let msg = match message_result {
                        Ok(msg) => msg,
                        Err(error) => {
                            if recv_event_tx
                                .send(SignalingEvent::Error(format!(
                                    "failed to read signaling WebSocket: {error}"
                                )))
                                .await
                                .is_err()
                            {
                                return;
                            }
                            break;
                        }
                    };

                    match msg {
                        tokio_tungstenite::tungstenite::Message::Text(text) => {
                            let event = match serde_json::from_str::<SignalIncoming>(&text) {
                                Ok(SignalIncoming::Welcome { peer_id }) => {
                                    Some(SignalingEvent::Connected(PeerId(peer_id)))
                                }
                                Ok(SignalIncoming::PeerJoined { peer_id }) => {
                                    Some(SignalingEvent::PeerJoined(PeerId(peer_id)))
                                }
                                Ok(SignalIncoming::PeerLeft { peer_id }) => {
                                    Some(SignalingEvent::PeerLeft(PeerId(peer_id)))
                                }
                                Ok(SignalIncoming::Signal { from, data }) => {
                                    Some(SignalingEvent::Signal {
                                        from: PeerId(from),
                                        data,
                                    })
                                }
                                Ok(SignalIncoming::Error { message }) => {
                                    Some(SignalingEvent::Error(message))
                                }
                                Err(error) => {
                                    log::warn!("Signaling parse error: {error}");
                                    None
                                }
                            };

                            if let Some(event) = event
                                && recv_event_tx.send(event).await.is_err()
                            {
                                return;
                            }
                        }
                        tokio_tungstenite::tungstenite::Message::Close(_) => break,
                        _ => {}
                    }
                }
            };

            tokio::select! {
                _ = send_task => {},
                _ = recv_task => {},
            }

            // Publish one terminal event for either socket direction closing.
            let _ = task_event_tx.send(SignalingEvent::Disconnected).await;
        });

        Ok(Self {
            tx: cmd_tx,
            rx: evt_rx,
            local_id: None,
        })
    }

    /// Join a room.
    ///
    /// Fails immediately if the bounded command queue is full or closed.
    pub fn join(&self, room: &str) -> Result<(), String> {
        enqueue_command(
            &self.tx,
            SignalOutgoing::Join {
                room: room.to_string(),
            },
        )
    }

    /// Send signaling data to a peer.
    ///
    /// Payloads above the configured signaling message limit and commands rejected
    /// by the bounded queue return an error instead of being buffered indefinitely.
    pub fn signal(&self, to: PeerId, data: SignalData) -> Result<(), String> {
        enqueue_command(&self.tx, SignalOutgoing::Signal { to: to.0, data })
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
