// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! ND physics → Bevy rendering bridge.
//!
//! Syncs `PhysicsWorld<D>` state to Bevy `Transform` components and provides
//! projection from ND to screen coordinates.
//!
//! # Modes
//! - **2D** (D=2): Direct 1:1 mapping to Bevy 2D sprites
//! - **3D** (D=3): Maps to Bevy 3D transforms (when `bevy_pbr` is available)
//! - **4D** (D=4): Cross-section slicing — slice 4D world with 3D hyperplane,
//!   render the intersection

pub mod galaxy_spatial;
pub mod hierarchy_select;
#[cfg(feature = "inspector")]
pub mod inspector;
pub mod large_world;
pub mod material;
pub mod projection;
pub mod scale_policy;
pub mod sync;
pub mod telemetry_ssbo;

pub use galaxy_spatial::{
    GALACTIC_COORDINATE_QUANTUM_MICROPC, GALACTIC_DOMAIN_HALF_WIDTH_PC,
    GALAXY_MAX_CELL_DEPTH, GalacticPosition, GalaxyCellId, GalaxyIndexError,
    GalaxyStreamingBudget, GalaxyTileCandidate, GalaxyTilePlan, plan_galaxy_tiles,
};
pub use hierarchy_select::{
    ContentAvailability, HeldParent, HierarchyHoldReason, HierarchyNode, HierarchyPolicy,
    HierarchyRefinement, HierarchySelection, HierarchySelectionError, RepresentationNodeId,
    SelectedRepresentation, select_hierarchy,
};
#[cfg(feature = "inspector")]
pub use inspector::SymtropyInspectorPlugin;
pub use large_world::{
    CoordinateFrameId, GpuRelativePosition, LargeWorldError, LargeWorldPosition,
    WORLD_CELL_SIZE_METERS,
};
pub use material::{
    NdSlicingMaterial, NdSlicingPlugin, NdSlicingSettings, TelemetryExtension, TelemetryMaterial,
    TelemetryMaterialPlugin, TelemetrySettings,
};
pub use projection::{Projector, Projector2D, Projector3D, Projector4D};
pub use scale_policy::{
    ApparentScaleError, ApparentSizeBand, ApparentSizeProfile, ApparentSizeProjection,
    ScaleBandLatch, projected_sphere_diameter_px,
};
pub use sync::{PhysicsBody, sync_physics_2d, sync_physics_3d, sync_physics_4d};
