// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Intended I/O adapter between Symtropy's transport abstraction and Lightyear.
//!
//! **Qualification status: scaffold only; not a live multiplayer transport.**
//! The current `IrohTransport` is an in-memory stub with no QUIC endpoint, and
//! these local buffers are not wired to Lightyear's actual `Link` send/receive
//! buffers. The send system currently queues via the stub on one channel and
//! discards send errors. Do not claim direct P2P, a latency bound, or completed
//! Lightyear replication from this module.
//!
//! Next gate: connect real Lightyear Link buffers to a verified transport,
//! preserve reliable/unreliable channel semantics, handle failures without
//! dropping them silently, and prove the path with two separate processes.
//! See [multiplayer scale and integration gates](../../../docs/tech/MULTIPLAYER_SCALE_AND_SOL_ATLAS.md).

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use std::collections::VecDeque;

use symtropy_net::PeerId;
use symtropy_net::iroh_transport::IrohTransport;
use symtropy_net::transport::{Channel, Transport, TransportEvent};

/// Transitional transport queues; not yet connected to Lightyear's actual `Link`.
///
/// The current systems move messages only between these local queues and the
/// in-memory `IrohTransport` stub. They do not carry real network packets.
#[derive(Component)]
pub struct IrohIo {
    /// The underlying Iroh transport.
    pub transport: IrohTransport,
    /// Inbound placeholder buffer (not currently drained by Lightyear).
    pub recv_buffer: VecDeque<Bytes>,
    /// Outbound placeholder buffer (not currently filled from Lightyear's Link).
    pub send_buffer: VecDeque<Bytes>,
}

impl IrohIo {
    /// Create a new IrohIo bridge for a peer.
    pub fn new(local_id: PeerId) -> Self {
        Self {
            transport: IrohTransport::new(local_id),
            recv_buffer: VecDeque::new(),
            send_buffer: VecDeque::new(),
        }
    }

    /// Connect to a room.
    pub fn connect(&mut self, room_id: &str) -> Result<(), String> {
        self.transport.connect(room_id)
    }
}

/// System: drain the stub transport's in-memory inbox into a local buffer.
///
/// This is not connected to Lightyear packet processing.
pub fn iroh_io_recv(mut query: Query<&mut IrohIo>) {
    for mut io in query.iter_mut() {
        let events = io.transport.poll();
        for event in events {
            if let TransportEvent::Message(msg) = event {
                io.recv_buffer.push_back(Bytes::from(msg.data));
            }
        }
    }
}

/// System: drain the local placeholder buffer into the stub transport (PostUpdate).
///
/// It is not currently connected to Lightyear outgoing packets or a live network.
pub fn iroh_io_send(mut query: Query<&mut IrohIo>) {
    for mut io in query.iter_mut() {
        while let Some(data) = io.send_buffer.pop_front() {
            // Broadcast to all connected peers on the reliable channel.
            // Lightyear handles its own reliability/ordering within the packet.
            let _ = io.transport.broadcast(Channel::Reliable, &data);
        }
    }
}

/// Plugin that adds IrohIo systems to the Bevy schedule.
pub struct IrohIoPlugin;

impl Plugin for IrohIoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, iroh_io_recv)
            .add_systems(PostUpdate, iroh_io_send);
    }
}
