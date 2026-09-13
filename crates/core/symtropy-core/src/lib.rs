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

pub mod economic;
pub mod economic_resolution;
pub mod financial;
pub mod industrial_ecology;
pub mod settlement;

pub mod prelude {
    pub use crate::bevy_physics::{BevyPhysicsPlugin, NoCouplingResource, PhysicsBody};
    pub use crate::economic::{
        ActorId, AssetId, CausalId, CommoditySpecId, EconomicActor, EconomicActorKind,
        EconomicError, LocationId, LotId, StockDepletionCause, StockEvent, StockLedger,
        StockLedgerEntry, StockLot, StockOrigin,
    };
    pub use crate::economic_resolution::{
        AccountConservationRecord, CurrencyConservationRecord, DetailRetentionId,
        DetailRetentionRef, EconomicConservationManifest, EconomicDetailState,
        EconomicHistoryManifest, EconomicResolutionLedger, EconomicResolutionSnapshot,
        EconomicResolutionTier, EconomicSnapshotId, ExactEconomicStateRef,
        FinancialRegistrySnapshot, MonetaryHistoryIdentity, ResolutionError,
        ResolutionLedgerEntry, ResolutionTransition, StockConservationKey,
        StockConservationRecord, StockHistoryIdentity,
    };
    pub use crate::financial::{
        AccountNetBalance, AccountTotals, CurrencyDefinition, CurrencyId, FinancialAccount,
        FinancialAccountClass, FinancialAccountId, FinancialBook, FinancialError,
        JournalLedgerEntry, JournalTransaction, JournalTransactionId, MonetaryAuthorityId,
        MonetaryLedgerEntry, MonetarySupplyEvent, Posting, PostingSide,
    };
    pub use crate::industrial_ecology::{
        BlockedIndustrialFlow, DependencyShortage, IndustrialCapability, IndustrialDependencyState,
        IndustrialEcology, IndustrialEcologyError, IndustrialFlowKind, IndustrialFlowPrerequisite,
        IndustrialGovernance, IndustrialShock, IndustrialTickReport, IndustrialViabilityOutcome,
    };
    pub use crate::settlement::{
        MonetarySettlement, MonetarySettlementLedger, SettlementAuthorizationRef, SettlementError,
        SettlementId, SettlementLedgerEntry,
    };
    pub use crate::math::Point;
    pub use crate::physics::body::BodyHandle;
    pub use crate::physics::world::PhysicsWorld;
}
