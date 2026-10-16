// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Experimental multiplayer networking primitives for Symtropy.
//!
//! Current transport reality: in-memory loopback is the only qualified byte-moving
//! backend. The WebSocket relay requires a compatible service and is not live-qualified;
//! the Iroh transport is an in-memory scaffold. No Holochain DHT state sync is implemented.

pub mod config;
pub mod iroh_transport;
pub mod lockstep;
pub mod loopback;
pub mod peer;
pub mod relay_transport;
pub mod session;
pub mod signaling;
pub mod transport;

pub use peer::PeerState;

// Re-export core types
pub use symtropy_net_core::{
    authority,
    peer::{PeerId, PeerStateCore},
    state,
};

pub use authority::SpatialAuthority;
pub use state::{StateAuthority, SyncableState};
