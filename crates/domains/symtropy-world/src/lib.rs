// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Macro/micro world bridge for the Symtropy engine.
//!
//! Connects the multiworld-sim civilization model (monthly ticks) to real-time
//! gameplay (60fps) via threaded simulation with channel-based communication.
//!
//! # Architecture
//!
//! ```text
//! Sim Thread (monthly ticks)          Game Thread (60fps)
//! ┌──────────────────┐                ┌──────────────────┐
//! │ MultiWorld Sim   │  SimSnapshot   │ PhysicsWorld     │
//! │ - governance     │ ──────────────>│ - bodies         │
//! │ - economy        │                │ - consciousness  │
//! │ - factions       │  PlayerAction  │ - rendering      │
//! │ - demographics   │ <──────────────│ - input          │
//! └──────────────────┘                └──────────────────┘
//! ```
//!
//! The sim thread runs ahead, buffering snapshots. The game thread interpolates
//! between snapshots for smooth visualization. Player actions (votes, proposals)
//! are sent back via the reverse channel.

pub mod authority_view;
pub mod basin_ingest_receipt;
pub mod basin_state;
pub mod bridge;
pub mod environment_evidence;
pub mod fidelity;
pub mod grid;
pub mod living_watershed;
mod observation_bridge;
pub mod residency;
pub mod scale;
pub mod snapshot;
pub mod time_control;

pub use authority_view::{
    AuthorityViewError, BodyCellIdentity, ClimateCellSummary, DerivedDomainView,
    EcologyCellSummary, HydrologyCellSummary, PlanetCellAuthorityView, TerrainCellSummary,
};
pub use basin_ingest_receipt::{
    BASIN_ENVIRONMENT_INGEST_RECEIPT_DOMAIN, BASIN_ENVIRONMENT_INGEST_RECEIPT_SCHEMA_VERSION,
    BASIN_ENVIRONMENT_POLICY_DOMAIN_PREFIX, BasinEnvironmentalIngestError,
    BasinEnvironmentalIngestReceipt, BasinEnvironmentalObservation, BasinIngestEffect,
    EnvironmentalObservationRole,
};
pub use basin_state::{
    BASIN_STATE_DIGEST_DOMAIN, BASIN_STATE_SCHEMA_VERSION, BasinCausalStateIdentity,
    BasinStateDigestError,
};
pub use bridge::WorldBridge;
pub use environment_evidence::{EnvironmentalEvidenceBundle, EnvironmentalEvidenceError};
pub use fidelity::{
    FidelityDemand, FidelityError, FidelityScheduler, FidelitySelectionPlan, RefinementReason,
    RefinementRequest, ResolutionResult,
};
pub use grid::{
    BiomeKind, BodyHexGrid, BodyId, EarthH3CellRef, GridSystem, HexCellId, HydrologyState,
    PlanetCell, ProceduralBodyGrid, normalize_lon_deg,
};
pub use living_watershed::{
    FLOOD_MAX_SLOPE, FLOOD_MIN_FLOW_ACCUMULATION, FLOOD_SURFACE_WATER_M,
    LIVING_WATERSHED_POLICY_DOMAIN, LIVING_WATERSHED_POLICY_SCHEMA_VERSION,
    RIPARIAN_MAX_SALINITY, RIPARIAN_MAX_SLOPE, RIPARIAN_MAX_SURFACE_WATER_M,
    RIPARIAN_MAX_TEMPERATURE_K, RIPARIAN_MIN_SURFACE_WATER_M, RIPARIAN_MIN_TEMPERATURE_K,
    LivingWatershedEvaluation, LivingWatershedPolicyError, LivingWatershedPolicyV1,
    LivingWatershedProposal, LivingWatershedReason,
};
pub use residency::{
    ActiveRepresentation, RepresentationLease, RepresentationReleasePermit, ResidencyDecision,
    ResidencyError, ResidencyGate,
};
pub use scale::WorldScale;
pub use snapshot::{EconomySummary, GovernanceSummary, PlayerAction, SimSnapshot};
pub use time_control::TimeControl;
