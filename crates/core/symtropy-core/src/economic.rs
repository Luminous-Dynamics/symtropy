// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Deterministic economic identity, authority, and physical-stock accounting.
//!
//! This module is intentionally narrower than a market or finance simulator. It
//! records who owns and possesses qualified stock, where that stock is, and every
//! quantity-changing event that reaches the economic boundary.
//!
//! It does **not** qualify industrial capability, manufacture goods, discover
//! recipes, set prices, create money, or claim mass balance between unlike
//! commodity units. Industrial and physical systems remain authoritative for those
//! facts. The economic kernel accepts explicitly caused stock events and guarantees
//! that stock cannot silently appear, disappear, move, or change authority inside
//! this ledger.

use std::collections::BTreeMap;
use std::fmt;

const MAX_ID_LEN: usize = 256;
const MAX_STOCK_LOTS: usize = 65_536;
const MAX_STOCK_EVENTS: usize = 262_144;

macro_rules! id_type {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, EconomicError> {
                let value = value.into();
                validate_id($kind, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(ActorId, "actor");
id_type!(AssetId, "asset");
id_type!(CommoditySpecId, "commodity-spec");
id_type!(LotId, "lot");
id_type!(LocationId, "location");
id_type!(CausalId, "causal-reference");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EconomicActorKind {
    Person,
    Household,
    Firm,
    Cooperative,
    PublicBody,
    Commons,
    Other,
}

/// An economic principal. Identity here grants no implicit permission; mutation
/// APIs still require the expected current owner/custodian where relevant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicActor {
    pub actor_id: ActorId,
    pub kind: EconomicActorKind,
}

/// One homogeneous economic lot.
///
/// `quantity` is expressed in the unit defined by `commodity_spec_id`; quantities
/// from different commodity specifications are never summed by this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockLot {
    lot_id: LotId,
    commodity_spec_id: CommoditySpecId,
    quantity: u64,
    owner_id: ActorId,
    custodian_id: ActorId,
    location_id: LocationId,
}

impl StockLot {
    pub fn new(
        lot_id: LotId,
        commodity_spec_id: CommoditySpecId,
        quantity: u64,
        owner_id: ActorId,
        custodian_id: ActorId,
        location_id: LocationId,
    ) -> Result<Self, EconomicError> {
        if quantity == 0 {
            return Err(EconomicError::ZeroQuantity);
        }
        Ok(Self {
            lot_id,
            commodity_spec_id,
            quantity,
            owner_id,
            custodian_id,
            location_id,
        })
    }

    pub fn lot_id(&self) -> &LotId {
        &self.lot_id
    }

    pub fn commodity_spec_id(&self) -> &CommoditySpecId {
        &self.commodity_spec_id
    }

    pub const fn quantity(&self) -> u64 {
        self.quantity
    }

    pub fn owner_id(&self) -> &ActorId {
        &self.owner_id
    }

    pub fn custodian_id(&self) -> &ActorId {
        &self.custodian_id
    }

    pub fn location_id(&self) -> &LocationId {
        &self.location_id
    }
}

/// Why a new lot entered economic stock.
///
/// These variants are accounting provenance, not authority to assert that the
/// referenced production, recycling, salvage, or initial qualification is valid.
/// The originating subsystem remains responsible for that evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StockOrigin {
    QualifiedInitial {
        evidence_id: CausalId,
    },
    IndustrialProduction {
        dependency_id: String,
        tick: u64,
        evidence_id: CausalId,
    },
    IndustrialRecycling {
        dependency_id: String,
        tick: u64,
        evidence_id: CausalId,
    },
    Salvage {
        source_asset_id: AssetId,
        evidence_id: CausalId,
    },
}

impl StockOrigin {
    fn validate(&self) -> Result<(), EconomicError> {
        match self {
            Self::IndustrialProduction { dependency_id, .. }
            | Self::IndustrialRecycling { dependency_id, .. } => {
                validate_id("industrial-dependency", dependency_id)
            }
            Self::QualifiedInitial { .. } | Self::Salvage { .. } => Ok(()),
        }
    }
}

/// Why quantity left a lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StockDepletionCause {
    IndustrialDemand {
        dependency_id: String,
        tick: u64,
        evidence_id: CausalId,
    },
    TransformationInput {
        process_id: CausalId,
    },
    HouseholdConsumption {
        activity_id: CausalId,
    },
    Loss {
        incident_id: CausalId,
    },
    Disposal {
        record_id: CausalId,
    },
}

impl StockDepletionCause {
    fn validate(&self) -> Result<(), EconomicError> {
        match self {
            Self::IndustrialDemand { dependency_id, .. } => {
                validate_id("industrial-dependency", dependency_id)
            }
            Self::TransformationInput { .. }
            | Self::HouseholdConsumption { .. }
            | Self::Loss { .. }
            | Self::Disposal { .. } => Ok(()),
        }
    }
}

/// Complete mutation grammar for the stock ledger.
///
/// There is deliberately no generic "set quantity/owner/location" operation.
/// Every state transition must select one of these auditable meanings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StockEvent {
    Establish {
        lot: StockLot,
        origin: StockOrigin,
    },
    Split {
        parent_lot_id: LotId,
        child_lot_id: LotId,
        child_quantity: u64,
        cause_id: CausalId,
    },
    TransferOwnership {
        lot_id: LotId,
        expected_owner_id: ActorId,
        new_owner_id: ActorId,
        cause_id: CausalId,
    },
    TransferCustody {
        lot_id: LotId,
        expected_custodian_id: ActorId,
        new_custodian_id: ActorId,
        cause_id: CausalId,
    },
    Relocate {
        lot_id: LotId,
        expected_location_id: LocationId,
        authorized_custodian_id: ActorId,
        new_location_id: LocationId,
        cause_id: CausalId,
    },
    Deplete {
        lot_id: LotId,
        quantity: u64,
        cause: StockDepletionCause,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockLedgerEntry {
    pub sequence: u64,
    pub event: StockEvent,
}

/// Append-only, replayable stock state.
///
/// Current state is a projection of `entries`. `validate()` independently replays
/// the complete history and requires byte-for-byte semantic equality with the
/// materialized lot map, making accidental state/history divergence detectable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockLedger {
    lots: BTreeMap<LotId, StockLot>,
    entries: Vec<StockLedgerEntry>,
}

impl Default for StockLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl StockLedger {
    pub const fn new() -> Self {
        Self {
            lots: BTreeMap::new(),
            entries: Vec::new(),
        }
    }

    /// Reconstruct materialized state from an append-only history.
    pub fn from_entries(entries: Vec<StockLedgerEntry>) -> Result<Self, EconomicError> {
        if entries.len() > MAX_STOCK_EVENTS {
            return Err(EconomicError::ModelTooLarge);
        }
        let lots = replay_entries(&entries)?;
        Ok(Self { lots, entries })
    }

    pub fn validate(&self) -> Result<(), EconomicError> {
        if self.lots.len() > MAX_STOCK_LOTS || self.entries.len() > MAX_STOCK_EVENTS {
            return Err(EconomicError::ModelTooLarge);
        }
        let replayed = replay_entries(&self.entries)?;
        if replayed != self.lots {
            return Err(EconomicError::LedgerHistoryMismatch);
        }
        Ok(())
    }

    pub fn lot(&self, lot_id: &LotId) -> Option<&StockLot> {
        self.lots.get(lot_id)
    }

    pub fn lots(&self) -> impl ExactSizeIterator<Item = &StockLot> {
        self.lots.values()
    }

    pub fn entries(&self) -> &[StockLedgerEntry] {
        &self.entries
    }

    pub fn quantity_by_spec(
        &self,
        commodity_spec_id: &CommoditySpecId,
    ) -> Result<u64, EconomicError> {
        self.lots
            .values()
            .filter(|lot| &lot.commodity_spec_id == commodity_spec_id)
            .try_fold(0_u64, |total, lot| {
                total
                    .checked_add(lot.quantity)
                    .ok_or(EconomicError::ArithmeticOverflow)
            })
    }

    pub fn establish(&mut self, lot: StockLot, origin: StockOrigin) -> Result<(), EconomicError> {
        self.transact(vec![StockEvent::Establish { lot, origin }])
    }

    /// Split a homogeneous lot without changing aggregate quantity, ownership,
    /// custody, specification, or location.
    pub fn split(
        &mut self,
        parent_lot_id: LotId,
        child_lot_id: LotId,
        child_quantity: u64,
        cause_id: CausalId,
    ) -> Result<(), EconomicError> {
        self.transact(vec![StockEvent::Split {
            parent_lot_id,
            child_lot_id,
            child_quantity,
            cause_id,
        }])
    }

    pub fn transfer_ownership(
        &mut self,
        lot_id: LotId,
        expected_owner_id: ActorId,
        new_owner_id: ActorId,
        cause_id: CausalId,
    ) -> Result<(), EconomicError> {
        self.transact(vec![StockEvent::TransferOwnership {
            lot_id,
            expected_owner_id,
            new_owner_id,
            cause_id,
        }])
    }

    pub fn transfer_custody(
        &mut self,
        lot_id: LotId,
        expected_custodian_id: ActorId,
        new_custodian_id: ActorId,
        cause_id: CausalId,
    ) -> Result<(), EconomicError> {
        self.transact(vec![StockEvent::TransferCustody {
            lot_id,
            expected_custodian_id,
            new_custodian_id,
            cause_id,
        }])
    }

    /// Move a lot only when the asserted custodian still possesses it.
    pub fn relocate(
        &mut self,
        lot_id: LotId,
        expected_location_id: LocationId,
        authorized_custodian_id: ActorId,
        new_location_id: LocationId,
        cause_id: CausalId,
    ) -> Result<(), EconomicError> {
        self.transact(vec![StockEvent::Relocate {
            lot_id,
            expected_location_id,
            authorized_custodian_id,
            new_location_id,
            cause_id,
        }])
    }

    pub fn deplete(
        &mut self,
        lot_id: LotId,
        quantity: u64,
        cause: StockDepletionCause,
    ) -> Result<(), EconomicError> {
        self.transact(vec![StockEvent::Deplete {
            lot_id,
            quantity,
            cause,
        }])
    }

    /// Apply a multi-event transaction atomically.
    ///
    /// This is the primitive later market/contract code can use to make several
    /// stock transitions all-or-nothing. Failed validation leaves both materialized
    /// state and append-only history unchanged.
    pub fn transact(&mut self, events: Vec<StockEvent>) -> Result<(), EconomicError> {
        if events.is_empty() {
            return Err(EconomicError::EmptyTransaction);
        }
        let final_len = self
            .entries
            .len()
            .checked_add(events.len())
            .ok_or(EconomicError::ArithmeticOverflow)?;
        if final_len > MAX_STOCK_EVENTS {
            return Err(EconomicError::ModelTooLarge);
        }

        let mut candidate_lots = self.lots.clone();
        for event in &events {
            apply_event(&mut candidate_lots, event)?;
        }

        let mut next_sequence = u64::try_from(self.entries.len())
            .map_err(|_| EconomicError::ArithmeticOverflow)?
            .checked_add(1)
            .ok_or(EconomicError::ArithmeticOverflow)?;
        let mut candidate_entries = self.entries.clone();
        for event in events {
            candidate_entries.push(StockLedgerEntry {
                sequence: next_sequence,
                event,
            });
            next_sequence = next_sequence
                .checked_add(1)
                .ok_or(EconomicError::ArithmeticOverflow)?;
        }

        // Prove that the candidate history independently reconstructs the same state
        // before committing either projection.
        if replay_entries(&candidate_entries)? != candidate_lots {
            return Err(EconomicError::LedgerHistoryMismatch);
        }

        self.lots = candidate_lots;
        self.entries = candidate_entries;
        Ok(())
    }
}

fn replay_entries(entries: &[StockLedgerEntry]) -> Result<BTreeMap<LotId, StockLot>, EconomicError> {
    if entries.len() > MAX_STOCK_EVENTS {
        return Err(EconomicError::ModelTooLarge);
    }
    let mut lots = BTreeMap::new();
    for (index, entry) in entries.iter().enumerate() {
        let expected = u64::try_from(index)
            .map_err(|_| EconomicError::ArithmeticOverflow)?
            .checked_add(1)
            .ok_or(EconomicError::ArithmeticOverflow)?;
        if entry.sequence != expected {
            return Err(EconomicError::InvalidSequence {
                expected,
                actual: entry.sequence,
            });
        }
        apply_event(&mut lots, &entry.event)?;
    }
    Ok(lots)
}

fn apply_event(
    lots: &mut BTreeMap<LotId, StockLot>,
    event: &StockEvent,
) -> Result<(), EconomicError> {
    match event {
        StockEvent::Establish { lot, origin } => {
            origin.validate()?;
            if lot.quantity == 0 {
                return Err(EconomicError::ZeroQuantity);
            }
            if lots.contains_key(&lot.lot_id) {
                return Err(EconomicError::DuplicateLot {
                    lot_id: lot.lot_id.clone(),
                });
            }
            if lots.len() >= MAX_STOCK_LOTS {
                return Err(EconomicError::ModelTooLarge);
            }
            lots.insert(lot.lot_id.clone(), lot.clone());
        }
        StockEvent::Split {
            parent_lot_id,
            child_lot_id,
            child_quantity,
            ..
        } => {
            if parent_lot_id == child_lot_id || lots.contains_key(child_lot_id) {
                return Err(EconomicError::DuplicateLot {
                    lot_id: child_lot_id.clone(),
                });
            }
            if lots.len() >= MAX_STOCK_LOTS {
                return Err(EconomicError::ModelTooLarge);
            }
            let parent = lots
                .get(parent_lot_id)
                .cloned()
                .ok_or_else(|| EconomicError::UnknownLot {
                    lot_id: parent_lot_id.clone(),
                })?;
            if *child_quantity == 0 || *child_quantity >= parent.quantity {
                return Err(EconomicError::InvalidSplitQuantity {
                    parent_lot_id: parent_lot_id.clone(),
                    parent_quantity: parent.quantity,
                    child_quantity: *child_quantity,
                });
            }
            let parent_remaining = parent
                .quantity
                .checked_sub(*child_quantity)
                .ok_or(EconomicError::ArithmeticOverflow)?;
            lots.get_mut(parent_lot_id)
                .expect("parent existence checked above")
                .quantity = parent_remaining;
            lots.insert(
                child_lot_id.clone(),
                StockLot {
                    lot_id: child_lot_id.clone(),
                    commodity_spec_id: parent.commodity_spec_id,
                    quantity: *child_quantity,
                    owner_id: parent.owner_id,
                    custodian_id: parent.custodian_id,
                    location_id: parent.location_id,
                },
            );
        }
        StockEvent::TransferOwnership {
            lot_id,
            expected_owner_id,
            new_owner_id,
            ..
        } => {
            let lot = lots
                .get_mut(lot_id)
                .ok_or_else(|| EconomicError::UnknownLot {
                    lot_id: lot_id.clone(),
                })?;
            if &lot.owner_id != expected_owner_id {
                return Err(EconomicError::StaleOwner {
                    lot_id: lot_id.clone(),
                    expected: expected_owner_id.clone(),
                    actual: lot.owner_id.clone(),
                });
            }
            if expected_owner_id == new_owner_id {
                return Err(EconomicError::NoOpMutation);
            }
            lot.owner_id = new_owner_id.clone();
        }
        StockEvent::TransferCustody {
            lot_id,
            expected_custodian_id,
            new_custodian_id,
            ..
        } => {
            let lot = lots
                .get_mut(lot_id)
                .ok_or_else(|| EconomicError::UnknownLot {
                    lot_id: lot_id.clone(),
                })?;
            if &lot.custodian_id != expected_custodian_id {
                return Err(EconomicError::StaleCustodian {
                    lot_id: lot_id.clone(),
                    expected: expected_custodian_id.clone(),
                    actual: lot.custodian_id.clone(),
                });
            }
            if expected_custodian_id == new_custodian_id {
                return Err(EconomicError::NoOpMutation);
            }
            lot.custodian_id = new_custodian_id.clone();
        }
        StockEvent::Relocate {
            lot_id,
            expected_location_id,
            authorized_custodian_id,
            new_location_id,
            ..
        } => {
            let lot = lots
                .get_mut(lot_id)
                .ok_or_else(|| EconomicError::UnknownLot {
                    lot_id: lot_id.clone(),
                })?;
            if &lot.location_id != expected_location_id {
                return Err(EconomicError::StaleLocation {
                    lot_id: lot_id.clone(),
                    expected: expected_location_id.clone(),
                    actual: lot.location_id.clone(),
                });
            }
            if &lot.custodian_id != authorized_custodian_id {
                return Err(EconomicError::StaleCustodian {
                    lot_id: lot_id.clone(),
                    expected: authorized_custodian_id.clone(),
                    actual: lot.custodian_id.clone(),
                });
            }
            if expected_location_id == new_location_id {
                return Err(EconomicError::NoOpMutation);
            }
            lot.location_id = new_location_id.clone();
        }
        StockEvent::Deplete {
            lot_id,
            quantity,
            cause,
        } => {
            cause.validate()?;
            if *quantity == 0 {
                return Err(EconomicError::ZeroQuantity);
            }
            let available = lots
                .get(lot_id)
                .ok_or_else(|| EconomicError::UnknownLot {
                    lot_id: lot_id.clone(),
                })?
                .quantity;
            if *quantity > available {
                return Err(EconomicError::InsufficientQuantity {
                    lot_id: lot_id.clone(),
                    available,
                    requested: *quantity,
                });
            }
            if *quantity == available {
                lots.remove(lot_id);
            } else {
                lots.get_mut(lot_id)
                    .expect("lot existence checked above")
                    .quantity = available
                    .checked_sub(*quantity)
                    .ok_or(EconomicError::ArithmeticOverflow)?;
            }
        }
    }
    Ok(())
}

fn validate_id(kind: &'static str, value: &str) -> Result<(), EconomicError> {
    if value.is_empty() {
        return Err(EconomicError::EmptyId { kind });
    }
    if value.len() > MAX_ID_LEN {
        return Err(EconomicError::IdTooLong {
            kind,
            max_len: MAX_ID_LEN,
        });
    }
    if value.trim() != value {
        return Err(EconomicError::IdHasSurroundingWhitespace { kind });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomicError {
    EmptyId {
        kind: &'static str,
    },
    IdTooLong {
        kind: &'static str,
        max_len: usize,
    },
    IdHasSurroundingWhitespace {
        kind: &'static str,
    },
    ZeroQuantity,
    DuplicateLot {
        lot_id: LotId,
    },
    UnknownLot {
        lot_id: LotId,
    },
    InvalidSplitQuantity {
        parent_lot_id: LotId,
        parent_quantity: u64,
        child_quantity: u64,
    },
    StaleOwner {
        lot_id: LotId,
        expected: ActorId,
        actual: ActorId,
    },
    StaleCustodian {
        lot_id: LotId,
        expected: ActorId,
        actual: ActorId,
    },
    StaleLocation {
        lot_id: LotId,
        expected: LocationId,
        actual: LocationId,
    },
    InsufficientQuantity {
        lot_id: LotId,
        available: u64,
        requested: u64,
    },
    EmptyTransaction,
    NoOpMutation,
    InvalidSequence {
        expected: u64,
        actual: u64,
    },
    LedgerHistoryMismatch,
    ArithmeticOverflow,
    ModelTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(value: &str) -> ActorId {
        ActorId::new(value).unwrap()
    }

    fn lot_id(value: &str) -> LotId {
        LotId::new(value).unwrap()
    }

    fn spec(value: &str) -> CommoditySpecId {
        CommoditySpecId::new(value).unwrap()
    }

    fn location(value: &str) -> LocationId {
        LocationId::new(value).unwrap()
    }

    fn cause(value: &str) -> CausalId {
        CausalId::new(value).unwrap()
    }

    fn lot(id: &str, quantity: u64) -> StockLot {
        StockLot::new(
            lot_id(id),
            spec("structural-steel"),
            quantity,
            actor("mill-owner"),
            actor("warehouse"),
            location("yard-a"),
        )
        .unwrap()
    }

    fn initial() -> StockOrigin {
        StockOrigin::QualifiedInitial {
            evidence_id: cause("qualification-001"),
        }
    }

    #[test]
    fn identifiers_fail_closed() {
        assert_eq!(ActorId::new(""), Err(EconomicError::EmptyId { kind: "actor" }));
        assert_eq!(
            ActorId::new(" owner "),
            Err(EconomicError::IdHasSurroundingWhitespace { kind: "actor" })
        );
    }

    #[test]
    fn ownership_custody_and_location_are_independent_authorities() {
        let mut ledger = StockLedger::new();
        ledger.establish(lot("lot-a", 100), initial()).unwrap();
        ledger
            .transfer_ownership(
                lot_id("lot-a"),
                actor("mill-owner"),
                actor("buyer"),
                cause("sale-001"),
            )
            .unwrap();

        let current = ledger.lot(&lot_id("lot-a")).unwrap();
        assert_eq!(current.owner_id(), &actor("buyer"));
        assert_eq!(current.custodian_id(), &actor("warehouse"));
        assert_eq!(current.location_id(), &location("yard-a"));
    }

    #[test]
    fn stale_authority_fails_without_mutating_history() {
        let mut ledger = StockLedger::new();
        ledger.establish(lot("lot-a", 100), initial()).unwrap();
        let before = ledger.clone();

        let error = ledger
            .transfer_ownership(
                lot_id("lot-a"),
                actor("not-the-owner"),
                actor("buyer"),
                cause("sale-001"),
            )
            .unwrap_err();
        assert!(matches!(error, EconomicError::StaleOwner { .. }));
        assert_eq!(ledger, before);
    }

    #[test]
    fn split_conserves_quantity_and_inherits_authority() {
        let mut ledger = StockLedger::new();
        ledger.establish(lot("parent", 100), initial()).unwrap();
        ledger
            .split(lot_id("parent"), lot_id("child"), 35, cause("split-001"))
            .unwrap();

        assert_eq!(ledger.quantity_by_spec(&spec("structural-steel")).unwrap(), 100);
        let parent = ledger.lot(&lot_id("parent")).unwrap();
        let child = ledger.lot(&lot_id("child")).unwrap();
        assert_eq!(parent.quantity(), 65);
        assert_eq!(child.quantity(), 35);
        assert_eq!(parent.owner_id(), child.owner_id());
        assert_eq!(parent.custodian_id(), child.custodian_id());
        assert_eq!(parent.location_id(), child.location_id());
    }

    #[test]
    fn full_depletion_closes_lot_but_preserves_audit_history() {
        let mut ledger = StockLedger::new();
        ledger.establish(lot("lot-a", 40), initial()).unwrap();
        ledger
            .deplete(
                lot_id("lot-a"),
                40,
                StockDepletionCause::TransformationInput {
                    process_id: cause("forge-run-001"),
                },
            )
            .unwrap();

        assert!(ledger.lot(&lot_id("lot-a")).is_none());
        assert_eq!(ledger.entries().len(), 2);
        ledger.validate().unwrap();
    }

    #[test]
    fn overdepletion_is_atomic() {
        let mut ledger = StockLedger::new();
        ledger.establish(lot("lot-a", 40), initial()).unwrap();
        let before = ledger.clone();
        let error = ledger
            .deplete(
                lot_id("lot-a"),
                41,
                StockDepletionCause::Loss {
                    incident_id: cause("incident-001"),
                },
            )
            .unwrap_err();
        assert!(matches!(error, EconomicError::InsufficientQuantity { .. }));
        assert_eq!(ledger, before);
    }

    #[test]
    fn multi_event_transaction_is_all_or_nothing() {
        let mut ledger = StockLedger::new();
        ledger.establish(lot("lot-a", 100), initial()).unwrap();
        let before = ledger.clone();

        let error = ledger
            .transact(vec![
                StockEvent::Split {
                    parent_lot_id: lot_id("lot-a"),
                    child_lot_id: lot_id("lot-b"),
                    child_quantity: 25,
                    cause_id: cause("split-001"),
                },
                StockEvent::TransferOwnership {
                    lot_id: lot_id("lot-b"),
                    expected_owner_id: actor("wrong-owner"),
                    new_owner_id: actor("buyer"),
                    cause_id: cause("sale-001"),
                },
            ])
            .unwrap_err();

        assert!(matches!(error, EconomicError::StaleOwner { .. }));
        assert_eq!(ledger, before);
        assert!(ledger.lot(&lot_id("lot-b")).is_none());
    }

    #[test]
    fn relocation_requires_current_custodian_and_location() {
        let mut ledger = StockLedger::new();
        ledger.establish(lot("lot-a", 100), initial()).unwrap();
        ledger
            .relocate(
                lot_id("lot-a"),
                location("yard-a"),
                actor("warehouse"),
                location("rail-terminal"),
                cause("freight-001"),
            )
            .unwrap();
        assert_eq!(
            ledger.lot(&lot_id("lot-a")).unwrap().location_id(),
            &location("rail-terminal")
        );
    }

    #[test]
    fn persisted_history_replays_to_identical_state() {
        let mut ledger = StockLedger::new();
        ledger.establish(lot("lot-a", 100), initial()).unwrap();
        ledger
            .split(lot_id("lot-a"), lot_id("lot-b"), 20, cause("split-001"))
            .unwrap();
        ledger
            .transfer_custody(
                lot_id("lot-b"),
                actor("warehouse"),
                actor("carrier"),
                cause("freight-001"),
            )
            .unwrap();

        let replayed = StockLedger::from_entries(ledger.entries().to_vec()).unwrap();
        assert_eq!(replayed, ledger);
    }

    #[test]
    fn corrupted_materialized_state_is_detected() {
        let mut ledger = StockLedger::new();
        ledger.establish(lot("lot-a", 100), initial()).unwrap();
        ledger.lots.get_mut(&lot_id("lot-a")).unwrap().quantity = 99;
        assert_eq!(ledger.validate(), Err(EconomicError::LedgerHistoryMismatch));
    }

    #[test]
    fn sequence_is_canonical_and_gap_free() {
        let entries = vec![StockLedgerEntry {
            sequence: 2,
            event: StockEvent::Establish {
                lot: lot("lot-a", 100),
                origin: initial(),
            },
        }];
        assert_eq!(
            StockLedger::from_entries(entries),
            Err(EconomicError::InvalidSequence {
                expected: 1,
                actual: 2
            })
        );
    }
}
