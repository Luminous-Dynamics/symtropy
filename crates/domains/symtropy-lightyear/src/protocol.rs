// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Candidate component reflection registration for future Lightyear replication.
//!
//! This module currently registers Bevy reflection metadata only. It does not
//! configure Lightyear replication, prediction/interpolation policies, wire
//! messages, or client/server plugins; those require explicit runtime wiring
//! and end-to-end qualification.

use crate::components::*;

/// Register candidate component types with Bevy reflection.
///
/// This only calls `App::register_type`; it does not register replication
/// rules or enable Lightyear networking. Call it during app setup only when
/// the application also needs the candidate reflection metadata.
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
