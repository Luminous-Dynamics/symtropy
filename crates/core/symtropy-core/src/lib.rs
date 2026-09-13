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

pub mod carrier_physics;
pub mod commodity;
pub mod economic;
pub mod economic_partition;
pub mod economic_repartition;
pub mod economic_resolution;
pub mod encumbrance_handoff;
pub mod financial;
pub mod freight_load;
pub mod industrial_ecology;
pub mod reservation_reconciliation;
pub mod reserved_lot_isolation;
pub mod settlement;
pub mod stock_reservation;

pub mod prelude {
    pub use crate::bevy_physics::{BevyPhysicsPlugin, NoCouplingResource, PhysicsBody};
    pub use crate::carrier_physics::{
        resolve_live_carrier, CarrierPhysicsError, CarrierPhysicsRef,
    };
    pub use crate::commodity::{
        BatchId, CommodityError, CommodityGradeId, CommodityIdentityRegistry,
        CommodityLotMetadata, CommoditySpecDefinition, CommodityTrackingMode, LotTrackingIdentity,
        MaterialAuthorityId, PhysicalMaterialId, PhysicalMaterialReference, QualitySpecificationId,
        QuantityDimensionId, QuantityUnitDefinition, QuantityUnitId,
    };
    pub use crate::economic::{
        ActorId, AssetId, CausalId, CommoditySpecId, EconomicActor, EconomicActorKind,
        EconomicError, LocationId, LotId, StockDepletionCause, StockEvent, StockLedger,
        StockLedgerEntry, StockLot, StockOrigin,
    };
    pub use crate::economic_partition::{
        EconomicPartitionId, EconomicPartitionManifest, EconomicPartitionPlan,
        EconomicPartitionSet, PartitionError, SharedGlobalEconomicState,
    };
    pub use crate::economic_repartition::{
        repartition_authority, AccountAuthorityMove, EconomicRepartitionId,
        EconomicRepartitionReceipt, RepartitionError, StockAuthorityMove,
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
    pub use crate::encumbrance_handoff::{
        handoff_full_reservation, EncumbranceHandoffError, EncumbranceHandoffReceipt,
    };
    pub use crate::financial::{
        AccountNetBalance, AccountTotals, CurrencyDefinition, CurrencyId, FinancialAccount,
        FinancialAccountClass, FinancialAccountId, FinancialBook, FinancialError,
        JournalLedgerEntry, JournalTransaction, JournalTransactionId, MonetaryAuthorityId,
        MonetaryLedgerEntry, MonetarySupplyEvent, Posting, PostingSide,
    };
    pub use crate::freight_load::{
        load_full_transport_reservation, CarrierCargoBinding, FreightLoadError, FreightLoadReceipt,
    };
    pub use crate::industrial_ecology::{
        BlockedIndustrialFlow, DependencyShortage, IndustrialCapability, IndustrialDependencyState,
        IndustrialEcology, IndustrialEcologyError, IndustrialFlowKind, IndustrialFlowPrerequisite,
        IndustrialGovernance, IndustrialShock, IndustrialTickReport, IndustrialViabilityOutcome,
    };
    pub use crate::reservation_reconciliation::{
        reconcile_reservation_repartition, ReservationConservationManifest,
        ReservationPartitionManifest, ReservationPartitionSet, ReservationReconciliationError,
        ReservationResolutionBinding,
    };
    pub use crate::reserved_lot_isolation::{
        isolate_reserved_lot, ReservedLotIsolationError, ReservedLotIsolationReceipt,
    };
    pub use crate::settlement::{
        MonetarySettlement, MonetarySettlementLedger, SettlementAuthorizationRef, SettlementError,
        SettlementId, SettlementLedgerEntry,
    };
    pub use crate::stock_reservation::{
        ReservationError, StockReservation, StockReservationEvent, StockReservationId,
        StockReservationLedger, StockReservationLedgerEntry,
    };
    pub use crate::math::Point;
    pub use crate::physics::body::{BodyHandle, NetId};
    pub use crate::physics::world::PhysicsWorld;
}
