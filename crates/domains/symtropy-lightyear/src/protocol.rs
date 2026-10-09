// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Protocol definition — register components and messages with Lightyear.
//!
//! This tells Lightyear which components to replicate, predict, and
//! interpolate. Components must be registered in both client and server
//! (or both peers in P2P mode).

use crate::components::*;

/// Register all Symtropy networked components and messages with Lightyear.
///
/// Call this during app setup before adding Lightyear plugins.
///
/// ```rust,ignore
/// let mut app = App::new();
/// symtropy_lightyear::protocol::register(&mut app);
/// app.add_plugins(ClientPlugins::new(client_config));
/// ```
pub fn register(app: &mut bevy_app::App) {
    // Register replicated components.
    // Each component that crosses the network needs registration.
    //
    // This registers Bevy reflection metadata only. The Lightyear 0.28+
    // replication protocol, entity markers, prediction/interpolation policy,
    // server/client plugin setup, and actual Link transport still need explicit
    // integration and end-to-end tests.
    app.register_type::<NetPosition>();
    app.register_type::<NetVelocity>();
    app.register_type::<NetRotation>();
    app.register_type::<NetConsciousness>();
    app.register_type::<NetAuthority>();
    app.register_type::<SpatialZone>();
}
