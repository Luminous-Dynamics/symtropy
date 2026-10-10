// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Signaling client — bounded WebSocket control and relay-data transport.
//!
//! Control messages (join/leave, SDP, ICE, membership, and errors) have a strict
//! 64 KiB serialized-envelope cap. Game data uses a separate relay_data variant
//! with a 1 MiB binary payload limit and a larger outer JSON envelope cap.
//!
//! Both command and event mailboxes are bounded by message count AND aggregate
//! serialized bytes. A single maximum-size relay message can occupy a byte budget;
//! it cannot be multiplied into dozens of queued copies. One message being read or
//! written may exist outside the queued-byte budget, and application-owned events
//! returned by poll_events are no longer mailbox memory.
//!
//! The relay-data variant is a protocol change. A live signaling server must
//! forward the new kind: relay_data shape opaquely before this path is usable.


#[cfg(feature = "webrtc")]
use std::sync::Arc;

#[cfg(feature = "webrtc")]
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};

use serde::{Deserialize, Serialize};

use crate::peer::PeerId;
use crate::transport::MAX_PEER_MESSAGE_BYTES;

pub(crate) const MAX_SIGNAL_CONTROL_BYTES: usize = 64 * 1024;
pub(crate) const MAX_SIGNALING_MESSAGE_BYTES: usize = (4 * MAX_PEER_MESSAGE_BYTES) + 16 * 1024;
#[cfg(feature = "webrtc")]
const MAX_SIGNAL_QUEUE_BYTES: usize = MAX_SIGNALING_MESSAGE_BYTES;
#[cfg(feature = "webrtc")]
const SIGNAL_COMMAND_QUEUE_CAPACITY: usize = 16;
#[cfg(feature = "webrtc")]
const SIGNAL_EVENT_QUEUE_CAPACITY: usize = 32;
#[cfg(feature = "webrtc")]
const MAX_SIGNAL_WRITE_BUFFER_BYTES: usize = MAX_SIGNALING_MESSAGE_BYTES + 256 * 1024;

/// Reliability lane for an explicit relayed game-data envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalChannel {
    Unreliable,
    Reliable,
}

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
    /// Send a control or relayed-data message to a specific peer.
    #[serde(rename = "signal")]
    Signal { to: u64, data: SignalData },
}

/// Messages received FROM the signaling server.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Signaling data exchanged between peers.
///
/// Control messages have their own small envelope cap. RelayData is an explicit
/// game-data variant, not an SDP field repurposed as a binary payload container.
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
    /// Binary game-data packet transported as a JSON byte array.
    #[serde(rename = "relay_data")]
    RelayData {
        channel: SignalChannel,
        payload: Vec<u8>,
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
    /// Received signaling data from another peer.
    Signal { from: PeerId, data: SignalData },
    /// Connection to signaling server lost.
    Disconnected,
    /// Error.
    Error(String),
}

fn serialized_outgoing(command: &SignalOutgoing) -> Result<String, String> {
    let (is_relay_data, largest_field) = match command {
        SignalOutgoing::Join { room } => (false, room.len()),
        SignalOutgoing::Leave => (false, 0),
        SignalOutgoing::Signal { data, .. } => match data {
            SignalData::Offer { sdp } | SignalData::Answer { sdp } => (false, sdp.len()),
            SignalData::IceCandidate {
                candidate, sdp_mid, ..
            } => (
                false,
                candidate
                    .len()
                    .saturating_add(sdp_mid.as_deref().map_or(0, str::len)),
            ),
            SignalData::RelayData { payload, .. } => (true, payload.len()),
        },
    };

    if is_relay_data {
        if largest_field > MAX_PEER_MESSAGE_BYTES {
            return Err(format!(
                "relay packet is {largest_field} bytes; maximum is {MAX_PEER_MESSAGE_BYTES} bytes"
            ));
        }
    } else if largest_field > MAX_SIGNAL_CONTROL_BYTES {
        return Err(format!(
            "signaling control field is {largest_field} bytes; maximum is {MAX_SIGNAL_CONTROL_BYTES} bytes"
        ));
    }

    // Serialize once so validation and queue-byte accounting use the exact same
    // representation that will be written to the WebSocket.
    let json = serde_json::to_string(command)
        .map_err(|error| format!("failed to serialize signaling command: {error}"))?;
    let limit = if is_relay_data {
        MAX_SIGNALING_MESSAGE_BYTES
    } else {
        MAX_SIGNAL_CONTROL_BYTES
    };
    if json.len() > limit {
        return Err(format!(
            "serialized signaling message is {} bytes; maximum is {limit} bytes",
            json.len()
        ));
    }
    Ok(json)
}

fn validate_incoming(message: &SignalIncoming, encoded_bytes: usize) -> Result<(), String> {
    if encoded_bytes > MAX_SIGNALING_MESSAGE_BYTES {
        return Err(format!(
            "incoming signaling envelope is {encoded_bytes} bytes; maximum is {MAX_SIGNALING_MESSAGE_BYTES} bytes"
        ));
    }

    match message {
        SignalIncoming::Signal {
            data: SignalData::RelayData { payload, .. },
            ..
        } => {
            if payload.len() > MAX_PEER_MESSAGE_BYTES {
                return Err(format!(
                    "incoming relay packet is {} bytes; maximum is {} bytes",
                    payload.len(),
                    MAX_PEER_MESSAGE_BYTES
                ));
            }
            Ok(())
        }
        _ if encoded_bytes > MAX_SIGNAL_CONTROL_BYTES => Err(format!(
            "incoming signaling control message is {encoded_bytes} bytes; maximum is {MAX_SIGNAL_CONTROL_BYTES} bytes"
        )),
        _ => Ok(()),
    }
}

fn parse_incoming(text: &str) -> Result<SignalIncoming, String> {
    if text.len() > MAX_SIGNALING_MESSAGE_BYTES {
        return Err(format!(
            "incoming WebSocket message is {} bytes; maximum is {} bytes",
            text.len(),
            MAX_SIGNALING_MESSAGE_BYTES
        ));
    }
    let message: SignalIncoming =
        serde_json::from_str(text).map_err(|error| format!("invalid signaling JSON: {error}"))?;
    validate_incoming(&message, text.len())?;
    Ok(message)
}

#[cfg(feature = "webrtc")]
struct QueuedSignalCommand {
    json: String,
    _byte_permit: OwnedSemaphorePermit,
}

#[cfg(feature = "webrtc")]
struct QueuedSignalingEvent {
    event: SignalingEvent,
    _byte_permit: OwnedSemaphorePermit,
}

#[cfg(feature = "webrtc")]
fn enqueue_command(
    tx: &mpsc::Sender<QueuedSignalCommand>,
    queued_bytes: &Arc<Semaphore>,
    command: SignalOutgoing,
) -> Result<(), String> {
    let json = serialized_outgoing(&command)?;
    let byte_count = u32::try_from(json.len())
        .map_err(|_| "serialized signaling message length exceeds accounting range".to_string())?;
    let byte_permit = queued_bytes
        .clone()
        .try_acquire_many_owned(byte_count)
        .map_err(|_| {
            format!(
                "signaling command queue byte budget exhausted ({} bytes); command rejected",
                MAX_SIGNAL_QUEUE_BYTES
            )
        })?;

    tx.try_send(QueuedSignalCommand {
        json,
        _byte_permit: byte_permit,
    })
    .map_err(|error| match error {
        mpsc::error::TrySendError::Full(_) => format!(
            "signaling command queue is full (capacity {SIGNAL_COMMAND_QUEUE_CAPACITY}); command rejected"
        ),
        mpsc::error::TrySendError::Closed(_) => "signaling command queue is closed".to_string(),
    })
}

#[cfg(feature = "webrtc")]
async fn reserve_queue_bytes(
    budget: &Arc<Semaphore>,
    bytes: usize,
    maximum: usize,
) -> Result<OwnedSemaphorePermit, String> {
    if bytes == 0 || bytes > maximum {
        return Err(format!(
            "queued item charge {bytes} bytes is outside the 1..={maximum} byte budget"
        ));
    }
    let permits = u32::try_from(bytes)
        .map_err(|_| "queued item charge exceeds byte accounting range".to_string())?;
    budget
        .clone()
        .acquire_many_owned(permits)
        .await
        .map_err(|_| "signaling byte budget closed".to_string())
}

#[cfg(feature = "webrtc")]
async fn enqueue_event_with_permit(
    tx: &mpsc::Sender<QueuedSignalingEvent>,
    event: SignalingEvent,
    byte_permit: OwnedSemaphorePermit,
) -> Result<(), String> {
    tx.send(QueuedSignalingEvent {
        event,
        _byte_permit: byte_permit,
    })
    .await
    .map_err(|_| "signaling event queue is closed".to_string())
}

#[cfg(feature = "webrtc")]
async fn enqueue_event(
    tx: &mpsc::Sender<QueuedSignalingEvent>,
    queued_bytes: &Arc<Semaphore>,
    event: SignalingEvent,
    charged_bytes: usize,
    maximum: usize,
) -> Result<(), String> {
    let permit = reserve_queue_bytes(queued_bytes, charged_bytes.max(1), maximum).await?;
    enqueue_event_with_permit(tx, event, permit).await
}

/// Signaling client handle — used by the transport to send/receive signaling messages.
#[cfg(feature = "webrtc")]
pub struct SignalingClient {
    tx: mpsc::Sender<QueuedSignalCommand>,
    rx: mpsc::Receiver<QueuedSignalingEvent>,
    command_bytes: Arc<Semaphore>,
    event_bytes: Arc<Semaphore>,
    /// Our peer ID (set after Welcome message).
    pub local_id: Option<PeerId>,
}

#[cfg(feature = "webrtc")]
impl SignalingClient {
    /// Connect to the signaling server and spawn the WebSocket task.
    pub async fn connect(url: &str) -> Result<Self, String> {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::{
            connect_async_with_config, tungstenite::protocol::WebSocketConfig,
        };

        // The write budget must be large enough for one maximum-size relay packet.
        let ws_config = WebSocketConfig {
            max_message_size: Some(MAX_SIGNALING_MESSAGE_BYTES),
            max_frame_size: Some(MAX_SIGNALING_MESSAGE_BYTES),
            max_write_buffer_size: MAX_SIGNAL_WRITE_BUFFER_BYTES,
            ..WebSocketConfig::default()
        };
        let (ws_stream, _) = connect_async_with_config(url, Some(ws_config), false)
            .await
            .map_err(|error| format!("Signaling connect failed: {error}"))?;

        let (mut ws_tx, mut ws_rx) = ws_stream.split();
        let (cmd_tx, mut cmd_rx) =
            mpsc::channel::<QueuedSignalCommand>(SIGNAL_COMMAND_QUEUE_CAPACITY);
        let (evt_tx, evt_rx) =
            mpsc::channel::<QueuedSignalingEvent>(SIGNAL_EVENT_QUEUE_CAPACITY);
        let command_bytes = Arc::new(Semaphore::new(MAX_SIGNAL_QUEUE_BYTES));
        let event_bytes = Arc::new(Semaphore::new(MAX_SIGNAL_QUEUE_BYTES));

        let task_event_tx = evt_tx.clone();
        let task_event_bytes = event_bytes.clone();
        tokio::spawn(async move {
            let send_event_tx = task_event_tx.clone();
            let send_event_bytes = task_event_bytes.clone();
            let recv_event_tx = task_event_tx.clone();
            let recv_event_bytes = task_event_bytes.clone();

            let send_task = async {
                while let Some(queued) = cmd_rx.recv().await {
                    let QueuedSignalCommand { json, _byte_permit } = queued;
                    if let Err(error) = ws_tx
                        .send(tokio_tungstenite::tungstenite::Message::Text(json))
                        .await
                    {
                        drop(_byte_permit);
                        let _ = enqueue_event(
                            &send_event_tx,
                            &send_event_bytes,
                            SignalingEvent::Error(format!(
                                "failed to write signaling WebSocket: {error}"
                            )),
                            512,
                            MAX_SIGNAL_QUEUE_BYTES,
                        )
                        .await;
                        return;
                    }
                    // Hold the command byte permit until the write completes.
                    drop(_byte_permit);
                }
            };

            let recv_task = async {
                while let Some(message_result) = ws_rx.next().await {
                    let message = match message_result {
                        Ok(message) => message,
                        Err(error) => {
                            let _ = enqueue_event(
                                &recv_event_tx,
                                &recv_event_bytes,
                                SignalingEvent::Error(format!(
                                    "failed to read signaling WebSocket: {error}"
                                )),
                                512,
                                MAX_SIGNAL_QUEUE_BYTES,
                            )
                            .await;
                            break;
                        }
                    };

                    match message {
                        tokio_tungstenite::tungstenite::Message::Text(text) => {
                            if text.len() > MAX_SIGNALING_MESSAGE_BYTES {
                                let _ = enqueue_event(
                                    &recv_event_tx,
                                    &recv_event_bytes,
                                    SignalingEvent::Error(format!(
                                        "incoming WebSocket message is {} bytes; maximum is {} bytes",
                                        text.len(),
                                        MAX_SIGNALING_MESSAGE_BYTES
                                    )),
                                    512,
                                    MAX_SIGNAL_QUEUE_BYTES,
                                )
                                .await;
                                continue;
                            }

                            // Reserve based on raw serialized bytes BEFORE JSON parsing.
                            // This stops a stalled consumer from accumulating parsed payloads
                            // beyond the mailbox's aggregate byte budget.
                            let raw_bytes = text.len().max(1);
                            let permit = match reserve_queue_bytes(
                                &recv_event_bytes,
                                raw_bytes,
                                MAX_SIGNAL_QUEUE_BYTES,
                            )
                            .await
                            {
                                Ok(permit) => permit,
                                Err(error) => {
                                    let _ = enqueue_event(
                                        &recv_event_tx,
                                        &recv_event_bytes,
                                        SignalingEvent::Error(error),
                                        512,
                                        MAX_SIGNAL_QUEUE_BYTES,
                                    )
                                    .await;
                                    continue;
                                }
                            };

                            let parsed = match parse_incoming(&text) {
                                Ok(message) => message,
                                Err(error) => {
                                    log::warn!("Signaling message rejected: {error}");
                                    let _ = enqueue_event_with_permit(
                                        &recv_event_tx,
                                        SignalingEvent::Error(error),
                                        permit,
                                    )
                                    .await;
                                    continue;
                                }
                            };

                            let event = match parsed {
                                SignalIncoming::Welcome { peer_id } => {
                                    SignalingEvent::Connected(PeerId(peer_id))
                                }
                                SignalIncoming::PeerJoined { peer_id } => {
                                    SignalingEvent::PeerJoined(PeerId(peer_id))
                                }
                                SignalIncoming::PeerLeft { peer_id } => {
                                    SignalingEvent::PeerLeft(PeerId(peer_id))
                                }
                                SignalIncoming::Signal { from, data } => {
                                    SignalingEvent::Signal {
                                        from: PeerId(from),
                                        data,
                                    }
                                }
                                SignalIncoming::Error { message } => {
                                    SignalingEvent::Error(message)
                                }
                            };

                            if enqueue_event_with_permit(&recv_event_tx, event, permit)
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                        tokio_tungstenite::tungstenite::Message::Close(_) => break,
                        tokio_tungstenite::tungstenite::Message::Binary(_) => {
                            let _ = enqueue_event(
                                &recv_event_tx,
                                &recv_event_bytes,
                                SignalingEvent::Error(
                                    "binary WebSocket messages are not part of the signaling protocol"
                                        .to_string(),
                                ),
                                256,
                                MAX_SIGNAL_QUEUE_BYTES,
                            )
                            .await;
                        }
                        _ => {}
                    }
                }
            };

            tokio::select! {
                _ = send_task => {},
                _ = recv_task => {},
            }

            // One terminal event follows either send-side or receive-side shutdown.
            // If the consumer is stalled, this awaits capacity rather than dropping it.
            let _ = enqueue_event(
                &task_event_tx,
                &task_event_bytes,
                SignalingEvent::Disconnected,
                256,
                MAX_SIGNAL_QUEUE_BYTES,
            )
            .await;
        });

        Ok(Self {
            tx: cmd_tx,
            rx: evt_rx,
            command_bytes,
            event_bytes,
            local_id: None,
        })
    }

    /// Join a room. A full count or byte budget returns an explicit error.
    pub fn join(&self, room: &str) -> Result<(), String> {
        enqueue_command(
            &self.tx,
            &self.command_bytes,
            SignalOutgoing::Join {
                room: room.to_string(),
            },
        )
    }

    /// Send signaling control data or an explicit relay-data packet to a peer.
    pub fn signal(&self, to: PeerId, data: SignalData) -> Result<(), String> {
        enqueue_command(
            &self.tx,
            &self.command_bytes,
            SignalOutgoing::Signal { to: to.0, data },
        )
    }

    /// Poll for events (non-blocking). Byte permits are released as events leave the
    /// internal mailbox; memory retained by the caller is outside the queue budget.
    pub fn poll_events(&mut self) -> Vec<SignalingEvent> {
        let mut events = Vec::new();
        while let Ok(queued) = self.rx.try_recv() {
            let QueuedSignalingEvent {
                event,
                _byte_permit,
            } = queued;
            if let SignalingEvent::Connected(id) = &event {
                self.local_id = Some(*id);
            }
            events.push(event);
            drop(_byte_permit);
        }
        events
    }
}

#[cfg(all(test, feature = "webrtc"))]
mod tests {
    use super::*;

    fn command_queue() -> (
        mpsc::Sender<QueuedSignalCommand>,
        mpsc::Receiver<QueuedSignalCommand>,
        Arc<Semaphore>,
    ) {
        let (tx, rx) = mpsc::channel(SIGNAL_COMMAND_QUEUE_CAPACITY);
        let bytes = Arc::new(Semaphore::new(MAX_SIGNAL_QUEUE_BYTES));
        (tx, rx, bytes)
    }

    #[test]
    fn control_message_exact_boundary_is_accepted_and_one_byte_over_is_rejected() {
        let empty = serialized_outgoing(&SignalOutgoing::Join {
            room: String::new(),
        })
        .expect("empty join");
        let room_bytes = MAX_SIGNAL_CONTROL_BYTES - empty.len();
        let exact = serialized_outgoing(&SignalOutgoing::Join {
            room: "x".repeat(room_bytes),
        })
        .expect("serialized control envelope at exact byte cap");
        assert_eq!(exact.len(), MAX_SIGNAL_CONTROL_BYTES);

        let error = serialized_outgoing(&SignalOutgoing::Join {
            room: "x".repeat(room_bytes + 1),
        })
        .expect_err("one byte over the control cap must fail");
        assert!(error.contains("maximum is"));
    }

    #[test]
    fn maximum_relay_packet_round_trips_in_the_outer_envelope() {
        let message = SignalIncoming::Signal {
            from: 7,
            data: SignalData::RelayData {
                channel: SignalChannel::Unreliable,
                payload: vec![u8::MAX; MAX_PEER_MESSAGE_BYTES],
            },
        };
        let encoded = serde_json::to_string(&message).expect("serialize maximum data envelope");
        assert!(encoded.len() <= MAX_SIGNALING_MESSAGE_BYTES);
        let decoded = parse_incoming(&encoded).expect("parse maximum data envelope");
        match decoded {
            SignalIncoming::Signal {
                from,
                data:
                    SignalData::RelayData {
                        channel,
                        payload,
                    },
            } => {
                assert_eq!(from, 7);
                assert_eq!(channel, SignalChannel::Unreliable);
                assert_eq!(payload.len(), MAX_PEER_MESSAGE_BYTES);
                assert!(payload.iter().all(|byte| *byte == u8::MAX));
            }
            _ => panic!("expected explicit relay-data envelope"),
        }
    }

    #[test]
    fn incoming_outer_envelope_rejects_one_byte_over_cap_before_parsing() {
        let oversized = " ".repeat(MAX_SIGNALING_MESSAGE_BYTES + 1);
        let error = parse_incoming(&oversized).expect_err("oversized envelope must fail closed");
        assert!(error.contains("maximum is"));
    }

    #[test]
    fn oversized_control_is_rejected_even_inside_large_outer_allowance() {
        let json = serde_json::to_string(&SignalIncoming::Signal {
            from: 2,
            data: SignalData::Offer {
                sdp: "x".repeat(MAX_SIGNAL_CONTROL_BYTES),
            },
        })
        .expect("serialize over-limit control envelope");
        assert!(json.len() < MAX_SIGNALING_MESSAGE_BYTES);
        let error = parse_incoming(&json).expect_err("SDP must use the small control cap");
        assert!(error.contains("control message"));
    }

    #[tokio::test]
    async fn command_queue_is_bounded_by_aggregate_bytes_as_well_as_count() {
        let (tx, _rx, bytes) = command_queue();
        enqueue_command(
            &tx,
            &bytes,
            SignalOutgoing::Signal {
                to: 1,
                data: SignalData::RelayData {
                    channel: SignalChannel::Reliable,
                    payload: vec![u8::MAX; MAX_PEER_MESSAGE_BYTES],
                },
            },
        )
        .expect("one maximum relay message fits the byte budget");

        let error = enqueue_command(
            &tx,
            &bytes,
            SignalOutgoing::Join {
                room: "x".repeat(MAX_SIGNAL_CONTROL_BYTES / 2),
            },
        )
        .expect_err("large second command exceeds aggregate byte budget");
        assert!(error.contains("byte budget"));
    }

    #[tokio::test]
    async fn stalled_event_consumer_backpressures_on_aggregate_bytes() {
        use tokio::task::yield_now;
        let (tx, mut rx) = mpsc::channel(SIGNAL_EVENT_QUEUE_CAPACITY);
        let bytes = Arc::new(Semaphore::new(10));

        enqueue_event(
            &tx,
            &bytes,
            SignalingEvent::Connected(PeerId(1)),
            6,
            10,
        )
        .await
        .expect("first event occupies six bytes");

        let pending_tx = tx.clone();
        let pending_bytes = bytes.clone();
        let pending = tokio::spawn(async move {
            enqueue_event(
                &pending_tx,
                &pending_bytes,
                SignalingEvent::PeerJoined(PeerId(2)),
                5,
                10,
            )
            .await
        });
        yield_now().await;
        assert!(!pending.is_finished(), "second event must wait for byte capacity");

        let first = rx.recv().await.expect("first event is queued");
        drop(first); // Dropping the mailbox item releases its byte permit.
        pending.await.expect("enqueue task should not panic").expect("released capacity");
        assert_eq!(bytes.available_permits(), 5);

        let second = rx.recv().await.expect("second event is queued");
        drop(second);
        assert_eq!(bytes.available_permits(), 10);
    }

    #[test]
    fn bounded_queue_item_count_constants_remain_explicit() {
        let (tx, mut rx) = mpsc::channel::<QueuedSignalCommand>(SIGNAL_COMMAND_QUEUE_CAPACITY);
        let bytes = Arc::new(Semaphore::new(MAX_SIGNAL_QUEUE_BYTES));
        for index in 0..SIGNAL_COMMAND_QUEUE_CAPACITY {
            enqueue_command(
                &tx,
                &bytes,
                SignalOutgoing::Join {
                    room: format!("room-{index}"),
                },
            )
            .expect("small command fits both bounds");
        }
        let error = enqueue_command(&tx, &bytes, SignalOutgoing::Leave)
            .expect_err("command count cap must reject the next entry");
        assert!(error.contains("queue is full"));
        assert_eq!(rx.max_capacity(), SIGNAL_COMMAND_QUEUE_CAPACITY);
    }
}
