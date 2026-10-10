// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Symtropy's Lightyear integration scaffold.
//!
//! **Qualification status: not end-to-end multiplayer.**
//!
//! This crate depends on Lightyear, but the current plugin does not configure
//! Lightyear ClientPlugins/ServerPlugins, register actual replication rules,
//! or connect its adapter queues to Lightyear's `Link` buffers. The
//! `symtropy_net::iroh_transport::IrohTransport` used by the adapter is an
//! in-memory stub with no QUIC endpoint. Reflected components and passing local
//! queue tests do not establish replication, P2P connectivity, or latency.
//!
//! `SymtropyNetPlugin` currently registers candidate component reflection
//! metadata and installs scaffold I/O and spatial-zone systems. Keep capability
//! claims bounded until exact-head CI and multi-process real-transport tests
//! demonstrate the end-to-end path.

pub mod components;
pub mod iroh_io;
pub mod plugin;
pub mod protocol;

pub use plugin::SymtropyNetPlugin;
