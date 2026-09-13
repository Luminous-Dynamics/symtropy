// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! ECON-05A physically binds one full transport reservation to carrier custody.
//!
//! This module deliberately does not create a cargo-availability ledger. Physical
//! stock remains authoritative in `StockLedger`; economic encumbrance remains
//! authoritative in `StockReservationLedger`. A load changes only custody and
//! location for an already-existing, fully reserved lot.

use crate::economic::{
    ActorId, AssetId, CausalId, CommoditySpecId, EconomicError, LocationId, LotId, StockEvent,
    StockLedger,
};
use crate::encumbrance_handoff::{EncumbranceHandoffError, EncumbranceHandoffReceipt};
use crate::stock_reservation::{ReservationError, StockReservationLedger};

/// Reference from the economic kernel to an externally qualified carrier cargo
/// space.
///
/// `carrier_asset_id` and `physical_authority_id` are bindings, not proof that the
/// carrier exists, is mobile, has any particular capacity, or can traverse a
/// route. Those facts remain the responsibility of a future physical transport
/// authority. `cargo_location_id` is the carrier-local location into which stock is
/// relocated; future carrier motion can therefore move the carrier without
/// rewriting every contained stock lot on every simulation tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarrierCargoBinding {
    carrier_asset_id: AssetId,
    cargo_location_id: LocationId,
    carrier_custodian_id: ActorId,
    physical_authority_id: CausalId,
}

impl CarrierCargoBinding {
    pub fn new(
        carrier_asset_id: AssetId,
        cargo_location_id: LocationId,
        carrier_custodian_id: ActorId,
        physical_authority_id: CausalId,
    ) -> Self {
        Self {
            carrier_asset_id,
            cargo_location_id,
            carrier_custodian_id,
            physical_authority_id,
        }
    }

    pub fn carrier_asset_id(&self) -> &AssetId {
        &self.carrier_asset_id
    }

    pub fn cargo_location_id(&self) -> &LocationId {
        &self.cargo_location_id
    }

    pub fn carrier_custodian_id(&self) -> &ActorId {
        &self.carrier_custodian_id
    }

    pub fn physical_authority_id(&self) -> &CausalId {
        &self.physical_authority_id
    }
}

/// Derivable evidence that one isolated, fully reserved lot entered a carrier
/// cargo space without changing stock quantity, ownership, commodity identity, or
/// economic encumbrance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreightLoadReceipt {
    stock_event_count_before: u64,
    stock_event_count_after: u64,
    reservation_event_count: u64,
    custody_sequence: Option<u64>,
    relocation_sequence: u64,
    handoff: EncumbranceHandoffReceipt,
    lot_id: LotId,
    commodity_spec_id: CommoditySpecId,
    quantity: u64,
    owner_id: ActorId,
    source_custodian_id: ActorId,
    source_location_id: LocationId,
    carrier: CarrierCargoBinding,
    load_cause_id: CausalId,
}

impl FreightLoadReceipt {
    pub const fn stock_event_count_before(&self) -> u64 {
        self.stock_event_count_before
    }

    pub const fn stock_event_count_after(&self) -> u64 {
        self.stock_event_count_after
    }

    pub const fn reservation_event_count(&self) -> u64 {
        self.reservation_event_count
    }

    pub const fn custody_sequence(&self) -> Option<u64> {
        self.custody_sequence
    }

    pub const fn relocation_sequence(&self) -> u64 {
        self.relocation_sequence
    }

    pub fn handoff(&self) -> &EncumbranceHandoffReceipt {
        &self.handoff
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

    pub fn source_custodian_id(&self) -> &ActorId {
        &self.source_custodian_id
    }

    pub fn source_location_id(&self) -> &LocationId {
        &self.source_location_id
    }

    pub fn carrier(&self) -> &CarrierCargoBinding {
        &self.carrier
    }

    pub fn load_cause_id(&self) -> &CausalId {
        &self.load_cause_id
    }

    /// Reconstruct the exact historical loading boundary from retained histories.
    ///
    /// Later stock and reservation events are allowed. Validation replays only the
    /// prefixes bound by this receipt, so later delivery, loss, reservation release,
    /// or further relocation cannot erase evidence that this load occurred.
    pub fn validate(
        &self,
        stock_history: &StockLedger,
        reservation_history: &StockReservationLedger,
    ) -> Result<(), FreightLoadError> {
        stock_history.validate()?;
        reservation_history.validate()?;
        self.handoff.validate(stock_history, reservation_history)?;

        let before_count = count_index(self.stock_event_count_before)?;
        let after_count = count_index(self.stock_event_count_after)?;
        let reservation_count = count_index(self.reservation_event_count)?;
        if before_count > stock_history.entries().len() || after_count > stock_history.entries().len()
        {
            return Err(FreightLoadError::StockHistoryTooShort);
        }
        if reservation_count > reservation_history.entries().len() {
            return Err(FreightLoadError::ReservationHistoryTooShort);
        }
        if after_count <= before_count {
            return Err(FreightLoadError::InvalidLoadBoundary);
        }

        let stock_before = StockLedger::from_entries(
            stock_history.entries()[..before_count].to_vec(),
        )?;
        let stock_after = StockLedger::from_entries(
            stock_history.entries()[..after_count].to_vec(),
        )?;
        let reservations_at_load = StockReservationLedger::from_entries(
            &stock_before,
            reservation_history.entries()[..reservation_count].to_vec(),
        )?;

        let transport = self.handoff.successor();
        let active = reservations_at_load
            .active_reservation(transport.reservation_id())
            .ok_or(FreightLoadError::TransportReservationNotActive)?;
        if active != transport {
            return Err(FreightLoadError::TransportReservationMismatch);
        }
        if active.lot_id() != &self.lot_id
            || active.commodity_spec_id() != &self.commodity_spec_id
            || active.quantity() != self.quantity
            || active.authorized_owner_id() != &self.owner_id
        {
            return Err(FreightLoadError::TransportReservationMismatch);
        }
        if active.holder_id() != self.carrier.carrier_custodian_id() {
            return Err(FreightLoadError::CarrierHolderMismatch {
                reservation_holder: active.holder_id().clone(),
                carrier_custodian: self.carrier.carrier_custodian_id().clone(),
            });
        }

        let before_lot = stock_before
            .lot(&self.lot_id)
            .ok_or_else(|| FreightLoadError::UnknownLot {
                lot_id: self.lot_id.clone(),
            })?;
        validate_preload_lot(self, before_lot)?;
        validate_isolated_full_reservation(&stock_before, &reservations_at_load, active)?;

        let expected_next = next_sequence(before_count)?;
        match self.custody_sequence {
            Some(custody_sequence) => {
                if custody_sequence != expected_next
                    || self.relocation_sequence
                        != custody_sequence
                            .checked_add(1)
                            .ok_or(FreightLoadError::ArithmeticOverflow)?
                {
                    return Err(FreightLoadError::InvalidLoadBoundary);
                }
                let custody_index = sequence_index(custody_sequence)?;
                match &stock_history.entries()[custody_index].event {
                    StockEvent::TransferCustody {
                        lot_id,
                        expected_custodian_id,
                        new_custodian_id,
                        cause_id,
                    } if lot_id == &self.lot_id
                        && expected_custodian_id == &self.source_custodian_id
                        && new_custodian_id == self.carrier.carrier_custodian_id()
                        && cause_id == &self.load_cause_id => {}
                    _ => return Err(FreightLoadError::CustodyEvidenceMismatch),
                }
            }
            None => {
                if self.source_custodian_id != *self.carrier.carrier_custodian_id()
                    || self.relocation_sequence != expected_next
                {
                    return Err(FreightLoadError::InvalidLoadBoundary);
                }
            }
        }

        if self.stock_event_count_after != self.relocation_sequence {
            return Err(FreightLoadError::InvalidLoadBoundary);
        }
        let relocation_index = sequence_index(self.relocation_sequence)?;
        match &stock_history.entries()[relocation_index].event {
            StockEvent::Relocate {
                lot_id,
                expected_location_id,
                authorized_custodian_id,
                new_location_id,
                cause_id,
            } if lot_id == &self.lot_id
                && expected_location_id == &self.source_location_id
                && authorized_custodian_id == self.carrier.carrier_custodian_id()
                && new_location_id == self.carrier.cargo_location_id()
                && cause_id == &self.load_cause_id => {}
            _ => return Err(FreightLoadError::RelocationEvidenceMismatch),
        }

        let after_lot = stock_after
            .lot(&self.lot_id)
            .ok_or_else(|| FreightLoadError::UnknownLot {
                lot_id: self.lot_id.clone(),
            })?;
        if after_lot.commodity_spec_id() != &self.commodity_spec_id
            || after_lot.quantity() != self.quantity
            || after_lot.owner_id() != &self.owner_id
            || after_lot.custodian_id() != self.carrier.carrier_custodian_id()
            || after_lot.location_id() != self.carrier.cargo_location_id()
        {
            return Err(FreightLoadError::PostLoadStateMismatch);
        }

        reservations_at_load.validate_against_stock(&stock_after)?;
        let reserved_after = reservations_at_load.reserved_quantity(&stock_after, &self.lot_id)?;
        let available_after = reservations_at_load.available_quantity(&stock_after, &self.lot_id)?;
        if reserved_after != self.quantity || available_after != 0 {
            return Err(FreightLoadError::EncumbranceConservationMismatch);
        }
        Ok(())
    }
}

/// Load one complete transport-reserved lot into an externally qualified carrier
/// cargo space.
///
/// ECON-05A v0.1 intentionally requires the transport reservation to cover the
/// entire live lot. This avoids silently relocating unreserved stock or unrelated
/// reservations. Partial/multi-lot loading requires a later reservation-aware lot
/// isolation theorem rather than an implicit split.
#[allow(clippy::too_many_arguments)]
pub fn load_full_transport_reservation(
    stock: &mut StockLedger,
    reservations: &StockReservationLedger,
    handoff: &EncumbranceHandoffReceipt,
    expected_source_custodian_id: ActorId,
    expected_source_location_id: LocationId,
    carrier: CarrierCargoBinding,
    load_cause_id: CausalId,
) -> Result<FreightLoadReceipt, FreightLoadError> {
    stock.validate()?;
    reservations.validate_against_stock(stock)?;
    handoff.validate(stock, reservations)?;

    let transport = handoff.successor();
    let active = reservations
        .active_reservation(transport.reservation_id())
        .ok_or(FreightLoadError::TransportReservationNotActive)?;
    if active != transport {
        return Err(FreightLoadError::TransportReservationMismatch);
    }
    if active.holder_id() != carrier.carrier_custodian_id() {
        return Err(FreightLoadError::CarrierHolderMismatch {
            reservation_holder: active.holder_id().clone(),
            carrier_custodian: carrier.carrier_custodian_id().clone(),
        });
    }

    let lot = stock
        .lot(active.lot_id())
        .cloned()
        .ok_or_else(|| FreightLoadError::UnknownLot {
            lot_id: active.lot_id().clone(),
        })?;
    if lot.custodian_id() != &expected_source_custodian_id {
        return Err(FreightLoadError::SourceCustodianMismatch {
            expected: expected_source_custodian_id,
            actual: lot.custodian_id().clone(),
        });
    }
    if lot.location_id() != &expected_source_location_id {
        return Err(FreightLoadError::SourceLocationMismatch {
            expected: expected_source_location_id,
            actual: lot.location_id().clone(),
        });
    }
    if lot.location_id() == carrier.cargo_location_id() {
        return Err(FreightLoadError::CargoLocationUnchanged);
    }
    validate_isolated_full_reservation(stock, reservations, active)?;

    let stock_event_count_before = u64::try_from(stock.entries().len())
        .map_err(|_| FreightLoadError::ArithmeticOverflow)?;
    let reservation_event_count = u64::try_from(reservations.entries().len())
        .map_err(|_| FreightLoadError::ArithmeticOverflow)?;
    let first_sequence = stock_event_count_before
        .checked_add(1)
        .ok_or(FreightLoadError::ArithmeticOverflow)?;

    let mut events = Vec::with_capacity(2);
    let custody_sequence = if lot.custodian_id() != carrier.carrier_custodian_id() {
        events.push(StockEvent::TransferCustody {
            lot_id: lot.lot_id().clone(),
            expected_custodian_id: lot.custodian_id().clone(),
            new_custodian_id: carrier.carrier_custodian_id().clone(),
            cause_id: load_cause_id.clone(),
        });
        Some(first_sequence)
    } else {
        None
    };
    let relocation_sequence = if custody_sequence.is_some() {
        first_sequence
            .checked_add(1)
            .ok_or(FreightLoadError::ArithmeticOverflow)?
    } else {
        first_sequence
    };
    events.push(StockEvent::Relocate {
        lot_id: lot.lot_id().clone(),
        expected_location_id: lot.location_id().clone(),
        authorized_custodian_id: carrier.carrier_custodian_id().clone(),
        new_location_id: carrier.cargo_location_id().clone(),
        cause_id: load_cause_id.clone(),
    });

    let mut candidate = stock.clone();
    candidate.transact(events)?;
    reservations.validate_against_stock(&candidate)?;

    let after = candidate
        .lot(lot.lot_id())
        .ok_or_else(|| FreightLoadError::UnknownLot {
            lot_id: lot.lot_id().clone(),
        })?;
    if after.quantity() != lot.quantity()
        || after.commodity_spec_id() != lot.commodity_spec_id()
        || after.owner_id() != lot.owner_id()
        || after.custodian_id() != carrier.carrier_custodian_id()
        || after.location_id() != carrier.cargo_location_id()
    {
        return Err(FreightLoadError::PostLoadStateMismatch);
    }
    let reserved_after = reservations.reserved_quantity(&candidate, lot.lot_id())?;
    let available_after = reservations.available_quantity(&candidate, lot.lot_id())?;
    if reserved_after != lot.quantity() || available_after != 0 {
        return Err(FreightLoadError::EncumbranceConservationMismatch);
    }

    let stock_event_count_after = u64::try_from(candidate.entries().len())
        .map_err(|_| FreightLoadError::ArithmeticOverflow)?;
    let receipt = FreightLoadReceipt {
        stock_event_count_before,
        stock_event_count_after,
        reservation_event_count,
        custody_sequence,
        relocation_sequence,
        handoff: handoff.clone(),
        lot_id: lot.lot_id().clone(),
        commodity_spec_id: lot.commodity_spec_id().clone(),
        quantity: lot.quantity(),
        owner_id: lot.owner_id().clone(),
        source_custodian_id: lot.custodian_id().clone(),
        source_location_id: lot.location_id().clone(),
        carrier,
        load_cause_id,
    };
    receipt.validate(&candidate, reservations)?;

    *stock = candidate;
    Ok(receipt)
}

fn validate_preload_lot(
    receipt: &FreightLoadReceipt,
    lot: &crate::economic::StockLot,
) -> Result<(), FreightLoadError> {
    if lot.commodity_spec_id() != &receipt.commodity_spec_id
        || lot.quantity() != receipt.quantity
        || lot.owner_id() != &receipt.owner_id
        || lot.custodian_id() != &receipt.source_custodian_id
        || lot.location_id() != &receipt.source_location_id
    {
        return Err(FreightLoadError::PreLoadStateMismatch);
    }
    if lot.location_id() == receipt.carrier.cargo_location_id() {
        return Err(FreightLoadError::CargoLocationUnchanged);
    }
    Ok(())
}

fn validate_isolated_full_reservation(
    stock: &StockLedger,
    reservations: &StockReservationLedger,
    transport: &crate::stock_reservation::StockReservation,
) -> Result<(), FreightLoadError> {
    let lot = stock
        .lot(transport.lot_id())
        .ok_or_else(|| FreightLoadError::UnknownLot {
            lot_id: transport.lot_id().clone(),
        })?;
    if lot.quantity() != transport.quantity() {
        return Err(FreightLoadError::LotNotTransportIsolated {
            lot_quantity: lot.quantity(),
            reservation_quantity: transport.quantity(),
        });
    }
    let reserved = reservations.reserved_quantity(stock, transport.lot_id())?;
    let available = reservations.available_quantity(stock, transport.lot_id())?;
    if reserved != lot.quantity() || available != 0 {
        return Err(FreightLoadError::EncumbranceConservationMismatch);
    }
    Ok(())
}

fn count_index(count: u64) -> Result<usize, FreightLoadError> {
    usize::try_from(count).map_err(|_| FreightLoadError::ArithmeticOverflow)
}

fn next_sequence(current_len: usize) -> Result<u64, FreightLoadError> {
    u64::try_from(current_len)
        .map_err(|_| FreightLoadError::ArithmeticOverflow)?
        .checked_add(1)
        .ok_or(FreightLoadError::ArithmeticOverflow)
}

fn sequence_index(sequence: u64) -> Result<usize, FreightLoadError> {
    if sequence == 0 {
        return Err(FreightLoadError::InvalidSequence);
    }
    usize::try_from(sequence - 1).map_err(|_| FreightLoadError::ArithmeticOverflow)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreightLoadError {
    Economic(EconomicError),
    Reservation(ReservationError),
    Handoff(EncumbranceHandoffError),
    ArithmeticOverflow,
    InvalidSequence,
    InvalidLoadBoundary,
    StockHistoryTooShort,
    ReservationHistoryTooShort,
    TransportReservationNotActive,
    TransportReservationMismatch,
    CarrierHolderMismatch {
        reservation_holder: ActorId,
        carrier_custodian: ActorId,
    },
    UnknownLot {
        lot_id: LotId,
    },
    SourceCustodianMismatch {
        expected: ActorId,
        actual: ActorId,
    },
    SourceLocationMismatch {
        expected: LocationId,
        actual: LocationId,
    },
    CargoLocationUnchanged,
    LotNotTransportIsolated {
        lot_quantity: u64,
        reservation_quantity: u64,
    },
    PreLoadStateMismatch,
    CustodyEvidenceMismatch,
    RelocationEvidenceMismatch,
    PostLoadStateMismatch,
    EncumbranceConservationMismatch,
}

impl From<EconomicError> for FreightLoadError {
    fn from(value: EconomicError) -> Self {
        Self::Economic(value)
    }
}

impl From<ReservationError> for FreightLoadError {
    fn from(value: ReservationError) -> Self {
        Self::Reservation(value)
    }
}

impl From<EncumbranceHandoffError> for FreightLoadError {
    fn from(value: EncumbranceHandoffError) -> Self {
        Self::Handoff(value)
    }
}
