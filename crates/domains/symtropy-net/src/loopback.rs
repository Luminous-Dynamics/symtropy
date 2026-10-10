// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Loopback transport — in-memory, zero-latency transport for testing.
//!
//! Two `LoopbackTransport` instances share a channel pair via `Arc<Mutex<>>`.
//! Messages sent by one appear instantly in the other's poll().
//! No signaling, no WebRTC — pure in-process testing.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::peer::PeerId;
use crate::transport::{Channel, PeerMessage, Transport, TransportEvent};

/// Shared message queue between two loopback endpoints.
type SharedQueue = Arc<Mutex<VecDeque<PeerMessage>>>;

#[derive(Debug)]
struct LoopbackConnectivity {
    connected: [bool; 2],
    announced: bool,
    events: [VecDeque<TransportEvent>; 2],
}

type SharedConnectivity = Arc<Mutex<LoopbackConnectivity>>;

/// Create a connected pair of loopback transports.
///
/// Messages sent by `a` appear in `b.poll()` and vice versa.
pub fn loopback_pair() -> (LoopbackTransport, LoopbackTransport) {
    let a_to_b: SharedQueue = Arc::new(Mutex::new(VecDeque::new()));
    let b_to_a: SharedQueue = Arc::new(Mutex::new(VecDeque::new()));
    let connectivity: SharedConnectivity = Arc::new(Mutex::new(LoopbackConnectivity {
        connected: [false, false],
        announced: false,
        events: std::array::from_fn(|_| VecDeque::new()),
    }));

    let a = LoopbackTransport {
        local_id: PeerId(0),
        remote_id: PeerId(1),
        outbox: a_to_b.clone(),
        inbox: b_to_a.clone(),
        connected: false,
        connectivity: connectivity.clone(),
        side: 0,
    };

    let b = LoopbackTransport {
        local_id: PeerId(1),
        remote_id: PeerId(0),
        outbox: b_to_a,
        inbox: a_to_b,
        connected: false,
        connectivity,
        side: 1,
    };

    (a, b)
}

/// In-memory transport for testing P2P sync without real networking.
pub struct LoopbackTransport {
    local_id: PeerId,
    remote_id: PeerId,
    outbox: SharedQueue,
    inbox: SharedQueue,
    connected: bool,
    connectivity: SharedConnectivity,
    side: usize,
}

impl Transport for LoopbackTransport {
    fn connect(&mut self, _room_id: &str) -> Result<(), String> {
        if self.connected {
            return Ok(());
        }

        self.connected = true;
        let mut connectivity = self.connectivity.lock().unwrap();
        connectivity.connected[self.side] = true;
        if connectivity.connected[0] && connectivity.connected[1] && !connectivity.announced {
            connectivity.announced = true;
            connectivity.events[0].push_back(TransportEvent::PeerConnected(PeerId(1)));
            connectivity.events[1].push_back(TransportEvent::PeerConnected(PeerId(0)));
        }
        Ok(())
    }

    fn disconnect(&mut self) {
        if !self.connected {
            return;
        }

        self.connected = false;
        let mut connectivity = self.connectivity.lock().unwrap();
        connectivity.connected[self.side] = false;
        if connectivity.announced {
            connectivity.announced = false;
            connectivity.events[0].push_back(TransportEvent::PeerDisconnected(PeerId(1)));
            connectivity.events[1].push_back(TransportEvent::PeerDisconnected(PeerId(0)));
        }
    }

    fn send(&mut self, _to: PeerId, channel: Channel, data: &[u8]) -> Result<(), String> {
        let peer_connected = self.connectivity.lock().unwrap().connected[1 - self.side];
        if !self.connected || !peer_connected {
            return Err("Not connected".into());
        }
        let msg = PeerMessage {
            from: self.local_id,
            channel,
            data: data.to_vec(),
        };
        self.outbox.lock().unwrap().push_back(msg);
        Ok(())
    }

    fn broadcast(&mut self, channel: Channel, data: &[u8]) -> Result<(), String> {
        self.send(self.remote_id, channel, data)
    }

    fn poll(&mut self) -> Vec<TransportEvent> {
        let mut events = {
            let mut connectivity = self.connectivity.lock().unwrap();
            connectivity.events[self.side].drain(..).collect::<Vec<_>>()
        };

        if self.connected {
            let mut inbox = self.inbox.lock().unwrap();
            while let Some(msg) = inbox.pop_front() {
                events.push(TransportEvent::Message(msg));
            }
        }

        events
    }

    fn peer_count(&self) -> usize {
        let connectivity = self.connectivity.lock().unwrap();
        if self.connected && connectivity.connected[1 - self.side] {
            1
        } else {
            0
        }
    }

    fn is_signaling_connected(&self) -> bool {
        self.connected
    }

    fn local_peer_id(&self) -> PeerId {
        self.local_id
    }
}

#[cfg(test)]
impl LoopbackTransport {
    /// Inject a synthetic packet for negative-path session tests.
    pub(crate) fn inject_message(&mut self, msg: PeerMessage) {
        self.inbox.lock().unwrap().push_back(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_send_receive() {
        let (mut a, mut b) = loopback_pair();
        a.connect("test").unwrap();
        b.connect("test").unwrap();

        a.send(PeerId(1), Channel::Reliable, b"hello").unwrap();

        let events = b.poll();
        assert_eq!(events.len(), 2, "peer admission arrives before payload");
        match &events[1] {
            TransportEvent::Message(msg) => {
                assert_eq!(msg.from, PeerId(0));
                assert_eq!(msg.channel, Channel::Reliable);
                assert_eq!(msg.data, b"hello");
            }
            _ => panic!("Expected Message event"),
        }
    }

    #[test]
    fn loopback_bidirectional() {
        let (mut a, mut b) = loopback_pair();
        a.connect("test").unwrap();
        b.connect("test").unwrap();

        a.broadcast(Channel::Unreliable, b"physics").unwrap();
        b.broadcast(Channel::Reliable, b"authority").unwrap();

        let b_events = b.poll();
        assert_eq!(b_events.len(), 2);

        let a_events = a.poll();
        assert_eq!(a_events.len(), 2);
        match &a_events[1] {
            TransportEvent::Message(msg) => {
                assert_eq!(msg.from, PeerId(1));
                assert_eq!(msg.data, b"authority");
            }
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn loopback_not_connected() {
        let (mut a, _b) = loopback_pair();
        assert!(a.send(PeerId(1), Channel::Reliable, b"fail").is_err());
    }

    #[test]
    fn loopback_peer_count() {
        let (mut a, mut b) = loopback_pair();
        assert_eq!(a.peer_count(), 0);
        a.connect("test").unwrap();
        assert_eq!(a.peer_count(), 0, "remote is not connected yet");
        b.connect("test").unwrap();
        assert_eq!(a.peer_count(), 1);
        a.disconnect();
        assert_eq!(a.peer_count(), 0);
        assert_eq!(
            b.poll().iter().filter(|event| matches!(event, TransportEvent::PeerDisconnected(PeerId(0)))).count(),
            1
        );
    }
}
