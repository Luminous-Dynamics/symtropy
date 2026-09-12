// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! # symtropy-core
//!
//! The permissive Symtropy distribution: core Bevy physics bundle and deterministic
//! simulation primitives without AGPL dependencies.

pub use symtropy_bevy_core as bevy_physics;
pub use symtropy_bevy_scene as scene;
pub use symtropy_devconsole as devconsole;
pub use symtropy_math as math;
pub use symtropy_physics as physics;

pub mod industrial_ecology;
pub mod industrial_watch;
pub mod industrial_watch_trace;

pub mod prelude {
    pub use crate::bevy_physics::{BevyPhysicsPlugin, NoCouplingResource, PhysicsBody};
    pub use crate::industrial_ecology::{
        BlockedIndustrialFlow, DependencyShortage, IndustrialCapability, IndustrialDependencyState,
        IndustrialEcology, IndustrialEcologyError, IndustrialFlowKind, IndustrialFlowPrerequisite,
        IndustrialGovernance, IndustrialShock, IndustrialTickReport, IndustrialViabilityOutcome,
    };
    pub use crate::industrial_watch::{
        IndustrialCapabilityWatch, IndustrialCapabilityWatchError, IndustrialCapabilityWatchReport,
        assess_industrial_capability_watches,
    };
    pub use crate::industrial_watch_trace::{
        IndustrialCapabilityWatchTrace, IndustrialCapabilityWatchTraceError,
        IndustrialCapabilityWatchTransition, record_industrial_capability_watch_reports,
    };
    pub use crate::math::Point;
    pub use crate::physics::body::BodyHandle;
    pub use crate::physics::world::PhysicsWorld;
}
