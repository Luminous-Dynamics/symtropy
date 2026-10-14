// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Candidate ECS components for future Lightyear replication.
//!
//! These mirror part of Symtropy's physics/agent state, but they are not
//! currently wired to Lightyear replication rules. Defining/reflect-registering
//! a component does not make it replicated, predicted, or interpolated.

use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

/// Candidate 3D position component; not replicated until protocol wiring exists.
#[derive(Component, Clone, Debug, Serialize, Deserialize, PartialEq, Reflect)]
pub struct NetPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Candidate velocity component; prediction policy is not wired yet.
#[derive(Component, Clone, Debug, Serialize, Deserialize, PartialEq, Reflect)]
pub struct NetVelocity {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Candidate replicated rotation (quaternion); not wired yet.
#[derive(Component, Clone, Debug, Serialize, Deserialize, PartialEq, Reflect)]
pub struct NetRotation {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Candidate consciousness-state component; not replicated yet.
#[derive(Component, Clone, Debug, Serialize, Deserialize, PartialEq, Reflect)]
pub struct NetConsciousness {
    /// Phi score (0.0 - 1.0).
    pub phi: f64,
    /// Energy budget remaining (Joules).
    pub energy: f64,
    /// Safety tier (0=Green, 1=Yellow, 2=Orange, 3=Red).
    pub safety_tier: u8,
    /// Whether this entity is in a sanctuary zone.
    pub in_sanctuary: bool,
}

/// Replicated player input (sent from client to authority peer).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlayerInput {
    /// Movement direction (normalized).
    pub move_x: f64,
    pub move_y: f64,
    /// Whether the player is applying consciousness focus.
    pub focus: bool,
    /// Whether the player is attempting a harmony action.
    pub harmony: bool,
    /// Tick this input was generated on.
    pub tick: u64,
}

/// Candidate authority metadata; this component alone does not enforce ownership.
#[derive(Component, Clone, Debug, Serialize, Deserialize, PartialEq, Reflect)]
pub struct NetAuthority {
    /// Peer ID of the authority.
    pub peer_id: u64,
    /// Whether authority can be transferred (e.g., proximity-based).
    pub transferable: bool,
}

/// Component identifying the spatial zone of an entity (Morton prefix).
#[derive(Component, Clone, Debug, Serialize, Deserialize, PartialEq, Reflect)]
pub struct SpatialZone(pub u64);

/// Update coarse spatial-zone labels using Morton codes; does not assign peer authority.
pub fn update_spatial_authority<const D: usize>(
    mut query: Query<(&NetPosition, &NetAuthority, Option<&mut SpatialZone>)>,
) {
    // 1. Define world bounds (demo-fixed for now)
    let min = nalgebra::SVector::<f64, D>::from_element(-100.0);
    let max = nalgebra::SVector::<f64, D>::from_element(100.0);

    for (pos, _auth, zone) in query.iter_mut() {
        let mut p = nalgebra::SVector::<f64, D>::zeros();
        p[0] = pos.x;
        if D >= 2 {
            p[1] = pos.y;
        }
        if D >= 3 {
            p[2] = pos.z;
        }

        // 2. Compute Morton code
        let code = symtropy_physics::broadphase::morton_encode::<D>(&p, &min, &max);

        // 3. Extract 4-bit prefix for coarse spatial partitioning
        let prefix = symtropy_physics::broadphase::morton_prefix(code, 4);

        if let Some(mut z) = zone {
            z.0 = prefix;
        }

        // This computes a zone label only. A real authority protocol must be explicit,
        // versioned, authenticated, and protected against simultaneous ownership.
    }
}
