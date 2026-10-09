// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Symtropy networking integration scaffold.
//!
//! This plugin does not yet enable end-to-end multiplayer: the Iroh transport
//! is a stub and Lightyear replication/Link wiring is incomplete.
//!
//! ```rust,ignore
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(SymtropyNetPlugin)
//!     .run();
//! ```

use bevy_app::prelude::*;

use crate::iroh_io::IrohIoPlugin;
use crate::protocol;

/// Networking scaffold for Symtropy; not a production multiplayer plugin yet.
///
/// Adds local stub-transport systems, reflected component type registration,
/// and placeholder spatial-zone labeling. It does not create a real connection,
/// configure Lightyear ClientPlugins/ServerPlugins, or register full replication
/// rules. Keep it explicitly unqualified until the integration gates in
/// `docs/tech/MULTIPLAYER_SCALE_AND_SOL_ATLAS.md` pass.
pub struct SymtropyNetPlugin;

impl Plugin for SymtropyNetPlugin {
    fn build(&self, app: &mut App) {
        // Register networked component types
        protocol::register(app);

        // Add Iroh transport IO systems
        app.add_plugins(IrohIoPlugin);

        // Spatial authority management (FixedUpdate)
        app.add_systems(
            FixedUpdate,
            (
                crate::components::update_spatial_authority::<2>,
                crate::components::update_spatial_authority::<3>,
            ),
        );

        bevy_log::warn!("Symtropy networking scaffold initialized; live transport and Lightyear replication are not yet wired");
    }
}
