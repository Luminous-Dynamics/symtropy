// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Intended I/O adapter between Symtropy's transport abstraction and Lightyear.
//!
//! **Qualification status: scaffold only; not a live multiplayer transport.**
//! The current `IrohTransport` is an in-memory stub with no QUIC endpoint, and
//! these queues are not wired to Lightyear's actual `Link` buffers. This module
//! only hardens the adapter boundary: bounded queue admission, per-peer sends,
//! channel/sender metadata preservation, and visible failures. It does not prove
//! delivery, authentication, replication, or a latency bound.
//!
//! Next gate: connect real Lightyear Link buffers to a verified transport and
//! prove the path with two separate processes. See
//! `docs/tech/MULTIPLAYER_SCALE_AND_SOL_ATLAS.md` for the implementation gates.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use std::collections::VecDeque;

use symtropy_net::PeerId;
use symtropy_net::iroh_transport::IrohTransport;
use symtropy_net::transport::{Channel, PeerMessage, Transport, TransportEvent};

/// Maximum size of an individual packet at this adapter boundary (1 MiB).
pub const MAX_PACKET_BYTES: usize = 1024 * 1024;
/// Maximum number of pending packets in either direction.
pub const MAX_PENDING_PACKETS: usize = 256;
/// Maximum combined payload bytes held in either direction (1 MiB).
pub const MAX_PENDING_BYTES: usize = 1024 * 1024;

/// A received packet with its transport identity and reliability semantics intact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohInboundPacket {
    pub from: PeerId,
    pub channel: Channel,
    pub data: Bytes,
}

/// An outbound packet with an explicit recipient and reliability semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
struct IrohOutboundPacket {
    to: PeerId,
    channel: Channel,
    data: Bytes,
}

/// Observable local adapter failures. These counters do not imply remote delivery.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IrohIoStats {
    pub rejected_inbound_packets: u64,
    pub rejected_outbound_packets: u64,
    pub outbound_send_failures: u64,
    pub last_error: Option<String>,
}

/// Validate a queue admission without changing state.
fn validate_admission(
    direction: &str,
    queued_packets: usize,
    queued_bytes: usize,
    packet_bytes: usize,
) -> Result<(), String> {
    if packet_bytes > MAX_PACKET_BYTES {
        return Err(format!(
            "{direction} packet is {packet_bytes} bytes; limit is {MAX_PACKET_BYTES}"
        ));
    }
    if queued_packets >= MAX_PENDING_PACKETS {
        return Err(format!(
            "{direction} queue is full: {queued_packets} packets; limit is {MAX_PENDING_PACKETS}"
        ));
    }
    let total = queued_bytes
        .checked_add(packet_bytes)
        .ok_or_else(|| format!("{direction} queue byte accounting overflow"))?;
    if total > MAX_PENDING_BYTES {
        return Err(format!(
            "{direction} queue would hold {total} bytes; limit is {MAX_PENDING_BYTES}"
        ));
    }
    Ok(())
}

/// Transitional transport queues; not yet connected to Lightyear's actual `Link`.
///
/// Queue fields remain private so callers must pass admission checks rather than
/// growing the queues directly. A successful `queue_send` only means local queue
/// admission, never transport delivery or remote application acceptance.
#[derive(Component)]
pub struct IrohIo {
    /// The underlying Iroh transport stub.
    pub transport: IrohTransport,
    recv_buffer: VecDeque<IrohInboundPacket>,
    send_buffer: VecDeque<IrohOutboundPacket>,
    recv_buffer_bytes: usize,
    send_buffer_bytes: usize,
    /// Counters and last error for the adapter boundary.
    pub stats: IrohIoStats,
}

impl IrohIo {
    /// Create a new adapter for a local peer identity.
    pub fn new(local_id: PeerId) -> Self {
        Self {
            transport: IrohTransport::new(local_id),
            recv_buffer: VecDeque::new(),
            send_buffer: VecDeque::new(),
            recv_buffer_bytes: 0,
            send_buffer_bytes: 0,
            stats: IrohIoStats::default(),
        }
    }

    /// Connect to the in-memory transport stub.
    ///
    /// This is not a live network connection; see `IrohTransport`'s module docs.
    pub fn connect(&mut self, room_id: &str) -> Result<(), String> {
        self.transport.connect(room_id)
    }

    /// Queue one per-peer packet, retaining the requested channel.
    ///
    /// Errors are explicit and the queue is unchanged on rejection.
    pub fn queue_send(
        &mut self,
        to: PeerId,
        channel: Channel,
        data: Bytes,
    ) -> Result<(), String> {
        if let Err(error) = validate_admission(
            "outbound",
            self.send_buffer.len(),
            self.send_buffer_bytes,
            data.len(),
        ) {
            self.stats.rejected_outbound_packets =
                self.stats.rejected_outbound_packets.saturating_add(1);
            self.stats.last_error = Some(error.clone());
            return Err(error);
        }
        self.send_buffer_bytes += data.len();
        self.send_buffer.push_back(IrohOutboundPacket { to, channel, data });
        Ok(())
    }

    /// Take the next received packet, releasing its queue byte budget.
    pub fn try_recv(&mut self) -> Option<IrohInboundPacket> {
        let packet = self.recv_buffer.pop_front()?;
        self.recv_buffer_bytes = self.recv_buffer_bytes.saturating_sub(packet.data.len());
        Some(packet)
    }

    /// Current number of queued inbound packets.
    pub fn pending_inbound_packets(&self) -> usize {
        self.recv_buffer.len()
    }

    /// Current number of queued outbound packets.
    pub fn pending_outbound_packets(&self) -> usize {
        self.send_buffer.len()
    }

    fn accept_received(&mut self, msg: PeerMessage) -> Result<(), String> {
        if let Err(error) = validate_admission(
            "inbound",
            self.recv_buffer.len(),
            self.recv_buffer_bytes,
            msg.data.len(),
        ) {
            self.stats.rejected_inbound_packets =
                self.stats.rejected_inbound_packets.saturating_add(1);
            self.stats.last_error = Some(error.clone());
            return Err(error);
        }
        let data = Bytes::from(msg.data);
        self.recv_buffer_bytes += data.len();
        self.recv_buffer.push_back(IrohInboundPacket {
            from: msg.from,
            channel: msg.channel,
            data,
        });
        Ok(())
    }

    fn record_inbound_rejection(&mut self, error: String) {
        // The admission method already counted and retained this failure.
        bevy_log::warn!("Symtropy Lightyear adapter rejected inbound packet: {error}");
    }

    fn record_send_failure(&mut self, to: PeerId, error: String) {
        self.stats.outbound_send_failures = self.stats.outbound_send_failures.saturating_add(1);
        self.stats.last_error = Some(error.clone());
        bevy_log::warn!(
            "Symtropy Lightyear adapter local send to peer {:?} failed: {error}",
            to
        );
    }

    fn dispatch_queued_packet(&mut self, packet: IrohOutboundPacket) {
        self.send_buffer_bytes = self.send_buffer_bytes.saturating_sub(packet.data.len());
        if let Err(error) = self.transport.send(packet.to, packet.channel, &packet.data) {
            self.record_send_failure(packet.to, error);
        }
        // Transport::send returning Ok is only local acceptance, not remote receipt.
    }
}

/// Drain messages from the in-memory transport into the bounded adapter queue.
///
/// A full queue rejects the newly polled packet and records the loss explicitly.
/// The transport API has no receive-side pause/ack mechanism yet, so this is not
/// end-to-end backpressure; a real adapter must provide that separately.
pub fn iroh_io_recv(mut query: Query<&mut IrohIo>) {
    for mut io in query.iter_mut() {
        for event in io.transport.poll() {
            match event {
                TransportEvent::Message(msg) => {
                    if let Err(error) = io.accept_received(msg) {
                        io.record_inbound_rejection(error);
                    }
                }
                TransportEvent::Error(error) => {
                    io.stats.last_error = Some(error.clone());
                    bevy_log::warn!("Symtropy Iroh transport reported an error: {error}");
                }
                _ => {}
            }
        }
    }
}

/// Send queued packets to their intended peer using their original channel.
///
/// A transport success means local acceptance only, not remote receipt. Failures
/// are counted and logged rather than silently discarded; this scaffold currently
/// drops the failed packet instead of retrying across session lifetimes.
pub fn iroh_io_send(mut query: Query<&mut IrohIo>) {
    for mut io in query.iter_mut() {
        while let Some(packet) = io.send_buffer.pop_front() {
            io.dispatch_queued_packet(packet);
        }
    }
}

/// Plugin that adds the adapter's scaffold systems to the Bevy schedule.
pub struct IrohIoPlugin;

impl Plugin for IrohIoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, iroh_io_recv)
            .add_systems(PostUpdate, iroh_io_send);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outbound_admission_preserves_recipient_and_channel_and_obeys_byte_budget() {
        let mut io = IrohIo::new(PeerId(1));
        io.queue_send(PeerId(2), Channel::Unreliable, Bytes::from_static(b"physics"))
            .unwrap();

        assert_eq!(io.pending_outbound_packets(), 1);
        assert_eq!(io.send_buffer.front().map(|packet| packet.to), Some(PeerId(2)));
        assert_eq!(
            io.send_buffer.front().map(|packet| packet.channel),
            Some(Channel::Unreliable)
        );
        assert_eq!(io.send_buffer_bytes, b"physics".len());
    }

    #[test]
    fn outbound_oversize_packet_is_rejected_without_queue_mutation() {
        let mut io = IrohIo::new(PeerId(1));
        let error = io
            .queue_send(
                PeerId(2),
                Channel::Reliable,
                Bytes::from(vec![0; MAX_PACKET_BYTES + 1]),
            )
            .unwrap_err();

        assert!(error.contains("limit"));
        assert_eq!(io.pending_outbound_packets(), 0);
        assert_eq!(io.send_buffer_bytes, 0);
        assert_eq!(io.stats.rejected_outbound_packets, 1);
    }

    #[test]
    fn outbound_queue_packet_count_is_bounded() {
        let mut io = IrohIo::new(PeerId(1));
        for _ in 0..MAX_PENDING_PACKETS {
            io.queue_send(
                PeerId(2),
                Channel::Reliable,
                Bytes::from_static(b"x"),
            )
            .unwrap();
        }

        assert!(io
            .queue_send(
                PeerId(2),
                Channel::Reliable,
                Bytes::from_static(b"overflow"),
            )
            .is_err());
        assert_eq!(io.pending_outbound_packets(), MAX_PENDING_PACKETS);
        assert_eq!(io.stats.rejected_outbound_packets, 1);
    }

    #[test]
    fn inbound_packet_preserves_sender_channel_and_releases_byte_budget_on_pop() {
        let mut io = IrohIo::new(PeerId(1));
        io.accept_received(PeerMessage {
            from: PeerId(7),
            channel: Channel::Reliable,
            data: b"authority".to_vec(),
        })
        .unwrap();

        let packet = io.try_recv().unwrap();
        assert_eq!(packet.from, PeerId(7));
        assert_eq!(packet.channel, Channel::Reliable);
        assert_eq!(packet.data.as_ref(), b"authority");
        assert_eq!(io.recv_buffer_bytes, 0);
        assert_eq!(io.pending_inbound_packets(), 0);
    }

    #[test]
    fn inbound_queue_rejects_byte_budget_overflow() {
        let mut io = IrohIo::new(PeerId(1));
        io.accept_received(PeerMessage {
            from: PeerId(7),
            channel: Channel::Unreliable,
            data: vec![0; MAX_PENDING_BYTES],
        })
        .unwrap();

        let error = io
            .accept_received(PeerMessage {
                from: PeerId(8),
                channel: Channel::Unreliable,
                data: b"x".to_vec(),
            })
            .unwrap_err();

        assert!(error.contains("limit"));
        assert_eq!(io.pending_inbound_packets(), 1);
        assert_eq!(io.stats.rejected_inbound_packets, 1);
    }

    #[test]
    fn disconnected_send_failure_is_counted_and_visible() {
        let mut io = IrohIo::new(PeerId(1));
        io.queue_send(PeerId(2), Channel::Unreliable, Bytes::from_static(b"x"))
            .unwrap();

        // The transport is not connected; executing its send path must not
        // silently ignore the failure. Exercise the same accounting directly.
        io.record_send_failure(PeerId(2), "Not connected".into());
        assert_eq!(io.stats.outbound_send_failures, 1);
        assert_eq!(io.stats.last_error.as_deref(), Some("Not connected"));
    }
}
