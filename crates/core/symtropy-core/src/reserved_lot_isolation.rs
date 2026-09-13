// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! ECON-05B isolates one partial reservation onto a shipment-sized child lot.
//!
//! The operation bridges whole-lot physical relocation with partial economic
//! reservations without silently moving unreserved stock. Stock and reservation
//! candidates are built and fully proved before either caller ledger is replaced.

use crate::economic::{
    ActorId, CausalId, CommoditySpecId, EconomicError, LocationId, LotId, StockEvent, StockLedger,
};
use crate::stock_reservation::{
    ReservationError, StockReservation, StockReservationEvent, StockReservationId,
    StockReservationLedger,
};

/// Historical evidence for a reservation-aware physical lot split.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedLotIsolationReceipt {
    stock_event_count_before: u64,
    stock_event_count_after: u64,
    reservation_event_count_before: u64,
    reservation_event_count_after: u64,
    split_sequence: u64,
    release_sequence: u64,
    successor_reserve_sequence: u64,
    isolation_cause_id: CausalId,
    source: StockReservation,
    successor: StockReservation,
    parent_quantity_before: u64,
    parent_quantity_after: u64,
    conserved_reserved_quantity: u64,
    conserved_available_quantity: u64,
}

impl ReservedLotIsolationReceipt {
    pub const fn stock_event_count_before(&self) -> u64 {
        self.stock_event_count_before
    }

    pub const fn stock_event_count_after(&self) -> u64 {
        self.stock_event_count_after
    }

    pub const fn reservation_event_count_before(&self) -> u64 {
        self.reservation_event_count_before
    }

    pub const fn reservation_event_count_after(&self) -> u64 {
        self.reservation_event_count_after
    }

    pub const fn split_sequence(&self) -> u64 {
        self.split_sequence
    }

    pub const fn release_sequence(&self) -> u64 {
        self.release_sequence
    }

    pub const fn successor_reserve_sequence(&self) -> u64 {
        self.successor_reserve_sequence
    }

    pub fn isolation_cause_id(&self) -> &CausalId {
        &self.isolation_cause_id
    }

    pub fn source(&self) -> &StockReservation {
        &self.source
    }

    pub fn successor(&self) -> &StockReservation {
        &self.successor
    }

    pub const fn parent_quantity_before(&self) -> u64 {
        self.parent_quantity_before
    }

    pub const fn parent_quantity_after(&self) -> u64 {
        self.parent_quantity_after
    }

    pub const fn conserved_reserved_quantity(&self) -> u64 {
        self.conserved_reserved_quantity
    }

    pub const fn conserved_available_quantity(&self) -> u64 {
        self.conserved_available_quantity
    }

    /// Reconstruct and prove the exact historical split + reservation retarget.
    /// Later stock and reservation history may be appended.
    pub fn validate(
        &self,
        stock_history: &StockLedger,
        reservation_history: &StockReservationLedger,
    ) -> Result<(), ReservedLotIsolationError> {
        stock_history.validate()?;
        reservation_history.validate()?;
        validate_reservation_continuity(&self.source, &self.successor)?;

        let stock_before_count = count_index(self.stock_event_count_before)?;
        let stock_after_count = count_index(self.stock_event_count_after)?;
        let reservation_before_count = count_index(self.reservation_event_count_before)?;
        let reservation_after_count = count_index(self.reservation_event_count_after)?;

        if stock_before_count > stock_history.entries().len()
            || stock_after_count > stock_history.entries().len()
        {
            return Err(ReservedLotIsolationError::StockHistoryTooShort);
        }
        if reservation_before_count > reservation_history.entries().len()
            || reservation_after_count > reservation_history.entries().len()
        {
            return Err(ReservedLotIsolationError::ReservationHistoryTooShort);
        }
        if self.stock_event_count_after
            != self
                .stock_event_count_before
                .checked_add(1)
                .ok_or(ReservedLotIsolationError::ArithmeticOverflow)?
            || self.split_sequence != self.stock_event_count_after
        {
            return Err(ReservedLotIsolationError::InvalidIsolationBoundary);
        }
        if self.release_sequence
            != self
                .reservation_event_count_before
                .checked_add(1)
                .ok_or(ReservedLotIsolationError::ArithmeticOverflow)?
            || self.successor_reserve_sequence
                != self
                    .release_sequence
                    .checked_add(1)
                    .ok_or(ReservedLotIsolationError::ArithmeticOverflow)?
            || self.reservation_event_count_after != self.successor_reserve_sequence
        {
            return Err(ReservedLotIsolationError::InvalidIsolationBoundary);
        }

        let stock_before = StockLedger::from_entries(
            stock_history.entries()[..stock_before_count].to_vec(),
        )?;
        let stock_after = StockLedger::from_entries(
            stock_history.entries()[..stock_after_count].to_vec(),
        )?;
        let reservations_before = StockReservationLedger::from_entries(
            &stock_before,
            reservation_history.entries()[..reservation_before_count].to_vec(),
        )?;
        let reservations_after = StockReservationLedger::from_entries(
            &stock_after,
            reservation_history.entries()[..reservation_after_count].to_vec(),
        )?;

        let source_before = reservations_before
            .active_reservation(self.source.reservation_id())
            .ok_or(ReservedLotIsolationError::SourceEvidenceMismatch)?;
        if source_before != &self.source {
            return Err(ReservedLotIsolationError::SourceEvidenceMismatch);
        }
        if reservations_before
            .active_reservation(self.successor.reservation_id())
            .is_some()
        {
            return Err(ReservedLotIsolationError::SuccessorAlreadyActiveBeforeIsolation);
        }
        if reservations_after
            .active_reservation(self.source.reservation_id())
            .is_some()
        {
            return Err(ReservedLotIsolationError::SourceStillActiveAfterIsolation);
        }
        if reservations_after.active_reservation(self.successor.reservation_id())
            != Some(&self.successor)
        {
            return Err(ReservedLotIsolationError::SuccessorEvidenceMismatch);
        }

        let split_index = sequence_index(self.split_sequence)?;
        match &stock_history.entries()[split_index].event {
            StockEvent::Split {
                parent_lot_id,
                child_lot_id,
                child_quantity,
                cause_id,
            } if parent_lot_id == self.source.lot_id()
                && child_lot_id == self.successor.lot_id()
                && *child_quantity == self.source.quantity()
                && cause_id == &self.isolation_cause_id => {}
            _ => return Err(ReservedLotIsolationError::SplitEvidenceMismatch),
        }

        let release_index = sequence_index(self.release_sequence)?;
        match &reservation_history.entries()[release_index].event {
            StockReservationEvent::Release {
                reservation_id,
                expected_holder_id,
                cause_id,
            } if reservation_id == self.source.reservation_id()
                && expected_holder_id == self.source.holder_id()
                && cause_id == &self.isolation_cause_id => {}
            _ => return Err(ReservedLotIsolationError::ReleaseEvidenceMismatch),
        }
        let reserve_index = sequence_index(self.successor_reserve_sequence)?;
        match &reservation_history.entries()[reserve_index].event {
            StockReservationEvent::Reserve { reservation } if reservation == &self.successor => {}
            _ => return Err(ReservedLotIsolationError::SuccessorEvidenceMismatch),
        }

        let parent_before = stock_before
            .lot(self.source.lot_id())
            .ok_or_else(|| ReservedLotIsolationError::UnknownLot {
                lot_id: self.source.lot_id().clone(),
            })?;
        let parent_after = stock_after
            .lot(self.source.lot_id())
            .ok_or_else(|| ReservedLotIsolationError::UnknownLot {
                lot_id: self.source.lot_id().clone(),
            })?;
        let child_after = stock_after
            .lot(self.successor.lot_id())
            .ok_or_else(|| ReservedLotIsolationError::UnknownLot {
                lot_id: self.successor.lot_id().clone(),
            })?;

        if parent_before.quantity() != self.parent_quantity_before
            || parent_after.quantity() != self.parent_quantity_after
        {
            return Err(ReservedLotIsolationError::PhysicalQuantityMismatch);
        }
        let expected_parent_after = self
            .parent_quantity_before
            .checked_sub(self.source.quantity())
            .ok_or(ReservedLotIsolationError::ArithmeticOverflow)?;
        if self.parent_quantity_after != expected_parent_after
            || child_after.quantity() != self.source.quantity()
        {
            return Err(ReservedLotIsolationError::PhysicalQuantityMismatch);
        }
        validate_inherited_stock_identity(parent_before, parent_after, child_after)?;

        if reservations_before.active_reservations().len()
            != reservations_after.active_reservations().len()
        {
            return Err(ReservedLotIsolationError::UnrelatedReservationChanged);
        }
        for reservation in reservations_before.active_reservations() {
            if reservation.reservation_id() == self.source.reservation_id() {
                continue;
            }
            if reservations_after.active_reservation(reservation.reservation_id())
                != Some(reservation)
            {
                return Err(ReservedLotIsolationError::UnrelatedReservationChanged);
            }
        }

        let reserved_before = reservations_before
            .reserved_quantity(&stock_before, self.source.lot_id())?;
        let available_before = reservations_before
            .available_quantity(&stock_before, self.source.lot_id())?;
        let reserved_after = checked_add(
            reservations_after.reserved_quantity(&stock_after, self.source.lot_id())?,
            reservations_after.reserved_quantity(&stock_after, self.successor.lot_id())?,
        )?;
        let available_after = checked_add(
            reservations_after.available_quantity(&stock_after, self.source.lot_id())?,
            reservations_after.available_quantity(&stock_after, self.successor.lot_id())?,
        )?;
        if reserved_before != reserved_after
            || available_before != available_after
            || reserved_before != self.conserved_reserved_quantity
            || available_before != self.conserved_available_quantity
        {
            return Err(ReservedLotIsolationError::ReservationConservationMismatch);
        }

        Ok(())
    }
}

/// Atomically prepare a partial reservation for whole-lot logistics by splitting
/// out exactly its quantity and retargeting the reservation to the child lot.
///
/// Both ledgers are mutated only after independent candidate replay and receipt
/// validation succeed. The reservation's holder, purpose, authorization, owner,
/// commodity identity, and quantity are preserved exactly; only lot identity and
/// reservation identity change.
#[allow(clippy::too_many_arguments)]
pub fn isolate_reserved_lot(
    stock: &mut StockLedger,
    reservations: &mut StockReservationLedger,
    source_reservation_id: StockReservationId,
    expected_holder_id: ActorId,
    child_lot_id: LotId,
    successor_reservation_id: StockReservationId,
    isolation_cause_id: CausalId,
) -> Result<ReservedLotIsolationReceipt, ReservedLotIsolationError> {
    stock.validate()?;
    reservations.validate_against_stock(stock)?;

    let source = reservations
        .active_reservation(&source_reservation_id)
        .cloned()
        .ok_or_else(|| ReservedLotIsolationError::UnknownSourceReservation {
            reservation_id: source_reservation_id.clone(),
        })?;
    if source.holder_id() != &expected_holder_id {
        return Err(ReservedLotIsolationError::SourceHolderMismatch {
            expected: expected_holder_id,
            actual: source.holder_id().clone(),
        });
    }
    if source.reservation_id() == &successor_reservation_id {
        return Err(ReservedLotIsolationError::ReservationIdentityReused);
    }

    let parent_before = stock
        .lot(source.lot_id())
        .cloned()
        .ok_or_else(|| ReservedLotIsolationError::UnknownLot {
            lot_id: source.lot_id().clone(),
        })?;
    if source.quantity() >= parent_before.quantity() {
        return Err(ReservedLotIsolationError::ReservationAlreadyWholeLot {
            lot_quantity: parent_before.quantity(),
            reservation_quantity: source.quantity(),
        });
    }

    let reserved_before = reservations.reserved_quantity(stock, source.lot_id())?;
    let available_before = reservations.available_quantity(stock, source.lot_id())?;
    let stock_event_count_before = u64::try_from(stock.entries().len())
        .map_err(|_| ReservedLotIsolationError::ArithmeticOverflow)?;
    let reservation_event_count_before = u64::try_from(reservations.entries().len())
        .map_err(|_| ReservedLotIsolationError::ArithmeticOverflow)?;

    let mut candidate_stock = stock.clone();
    candidate_stock.split(
        source.lot_id().clone(),
        child_lot_id.clone(),
        source.quantity(),
        isolation_cause_id.clone(),
    )?;
    let child = candidate_stock
        .lot(&child_lot_id)
        .ok_or_else(|| ReservedLotIsolationError::UnknownLot {
            lot_id: child_lot_id.clone(),
        })?;
    if child.quantity() != source.quantity()
        || child.commodity_spec_id() != source.commodity_spec_id()
        || child.owner_id() != source.authorized_owner_id()
        || child.custodian_id() != parent_before.custodian_id()
        || child.location_id() != parent_before.location_id()
    {
        return Err(ReservedLotIsolationError::InheritedStockIdentityMismatch);
    }

    let successor = StockReservation::new(
        &candidate_stock,
        successor_reservation_id,
        child_lot_id,
        source.quantity(),
        source.authorized_owner_id().clone(),
        source.holder_id().clone(),
        source.purpose_id().clone(),
        source.authorization_id().clone(),
    )?;
    validate_reservation_continuity(&source, &successor)?;

    let mut candidate_reservations = reservations.clone();
    candidate_reservations.transact(
        &candidate_stock,
        vec![
            StockReservationEvent::Release {
                reservation_id: source.reservation_id().clone(),
                expected_holder_id: source.holder_id().clone(),
                cause_id: isolation_cause_id.clone(),
            },
            StockReservationEvent::Reserve {
                reservation: successor.clone(),
            },
        ],
    )?;

    let parent_after = candidate_stock
        .lot(source.lot_id())
        .ok_or_else(|| ReservedLotIsolationError::UnknownLot {
            lot_id: source.lot_id().clone(),
        })?;
    let reserved_after = checked_add(
        candidate_reservations.reserved_quantity(&candidate_stock, source.lot_id())?,
        candidate_reservations.reserved_quantity(&candidate_stock, successor.lot_id())?,
    )?;
    let available_after = checked_add(
        candidate_reservations.available_quantity(&candidate_stock, source.lot_id())?,
        candidate_reservations.available_quantity(&candidate_stock, successor.lot_id())?,
    )?;
    if reserved_before != reserved_after || available_before != available_after {
        return Err(ReservedLotIsolationError::ReservationConservationMismatch);
    }

    let stock_event_count_after = u64::try_from(candidate_stock.entries().len())
        .map_err(|_| ReservedLotIsolationError::ArithmeticOverflow)?;
    let reservation_event_count_after = u64::try_from(candidate_reservations.entries().len())
        .map_err(|_| ReservedLotIsolationError::ArithmeticOverflow)?;
    let split_sequence = stock_event_count_after;
    let release_sequence = reservation_event_count_before
        .checked_add(1)
        .ok_or(ReservedLotIsolationError::ArithmeticOverflow)?;
    let successor_reserve_sequence = release_sequence
        .checked_add(1)
        .ok_or(ReservedLotIsolationError::ArithmeticOverflow)?;

    let receipt = ReservedLotIsolationReceipt {
        stock_event_count_before,
        stock_event_count_after,
        reservation_event_count_before,
        reservation_event_count_after,
        split_sequence,
        release_sequence,
        successor_reserve_sequence,
        isolation_cause_id,
        source,
        successor,
        parent_quantity_before: parent_before.quantity(),
        parent_quantity_after: parent_after.quantity(),
        conserved_reserved_quantity: reserved_before,
        conserved_available_quantity: available_before,
    };
    receipt.validate(&candidate_stock, &candidate_reservations)?;

    // No fallible operation occurs after the first replacement. Exclusive mutable
    // borrows prevent callers from observing the coordinator between assignments.
    *stock = candidate_stock;
    *reservations = candidate_reservations;
    Ok(receipt)
}

fn validate_reservation_continuity(
    source: &StockReservation,
    successor: &StockReservation,
) -> Result<(), ReservedLotIsolationError> {
    if source.reservation_id() == successor.reservation_id()
        || source.lot_id() == successor.lot_id()
        || source.commodity_spec_id() != successor.commodity_spec_id()
        || source.quantity() != successor.quantity()
        || source.authorized_owner_id() != successor.authorized_owner_id()
        || source.holder_id() != successor.holder_id()
        || source.purpose_id() != successor.purpose_id()
        || source.authorization_id() != successor.authorization_id()
    {
        return Err(ReservedLotIsolationError::ReservationContinuityMismatch);
    }
    Ok(())
}

fn validate_inherited_stock_identity(
    parent_before: &crate::economic::StockLot,
    parent_after: &crate::economic::StockLot,
    child_after: &crate::economic::StockLot,
) -> Result<(), ReservedLotIsolationError> {
    if parent_before.commodity_spec_id() != parent_after.commodity_spec_id()
        || parent_before.commodity_spec_id() != child_after.commodity_spec_id()
        || parent_before.owner_id() != parent_after.owner_id()
        || parent_before.owner_id() != child_after.owner_id()
        || parent_before.custodian_id() != parent_after.custodian_id()
        || parent_before.custodian_id() != child_after.custodian_id()
        || parent_before.location_id() != parent_after.location_id()
        || parent_before.location_id() != child_after.location_id()
    {
        return Err(ReservedLotIsolationError::InheritedStockIdentityMismatch);
    }
    Ok(())
}

fn checked_add(left: u64, right: u64) -> Result<u64, ReservedLotIsolationError> {
    left.checked_add(right)
        .ok_or(ReservedLotIsolationError::ArithmeticOverflow)
}

fn count_index(count: u64) -> Result<usize, ReservedLotIsolationError> {
    usize::try_from(count).map_err(|_| ReservedLotIsolationError::ArithmeticOverflow)
}

fn sequence_index(sequence: u64) -> Result<usize, ReservedLotIsolationError> {
    if sequence == 0 {
        return Err(ReservedLotIsolationError::InvalidSequence);
    }
    usize::try_from(sequence - 1).map_err(|_| ReservedLotIsolationError::ArithmeticOverflow)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservedLotIsolationError {
    Economic(EconomicError),
    Reservation(ReservationError),
    ArithmeticOverflow,
    InvalidSequence,
    InvalidIsolationBoundary,
    StockHistoryTooShort,
    ReservationHistoryTooShort,
    UnknownSourceReservation {
        reservation_id: StockReservationId,
    },
    SourceHolderMismatch {
        expected: ActorId,
        actual: ActorId,
    },
    ReservationIdentityReused,
    UnknownLot {
        lot_id: LotId,
    },
    ReservationAlreadyWholeLot {
        lot_quantity: u64,
        reservation_quantity: u64,
    },
    ReservationContinuityMismatch,
    SourceEvidenceMismatch,
    SuccessorAlreadyActiveBeforeIsolation,
    SourceStillActiveAfterIsolation,
    SuccessorEvidenceMismatch,
    SplitEvidenceMismatch,
    ReleaseEvidenceMismatch,
    InheritedStockIdentityMismatch,
    PhysicalQuantityMismatch,
    UnrelatedReservationChanged,
    ReservationConservationMismatch,
}

impl From<EconomicError> for ReservedLotIsolationError {
    fn from(value: EconomicError) -> Self {
        Self::Economic(value)
    }
}

impl From<ReservationError> for ReservedLotIsolationError {
    fn from(value: ReservationError) -> Self {
        Self::Reservation(value)
    }
}
