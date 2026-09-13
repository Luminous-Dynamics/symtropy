// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Atomic handoff of one full stock reservation into one typed successor claim.
//!
//! ECON-04D does not create a second cargo/shipment ledger. Instead, it uses the
//! existing ECON-04B reservation transaction grammar to replace one active claim
//! with exactly one successor claim in a single atomic commit. The underlying
//! quantity therefore never becomes transiently available between phases.

use crate::economic::{ActorId, CausalId, EconomicError, StockLedger};
use crate::stock_reservation::{
    ReservationError, StockReservation, StockReservationEvent, StockReservationId,
    StockReservationLedger,
};

/// Evidence for one full reservation-to-reservation authority handoff.
///
/// The receipt is derivable from retained stock and reservation histories. It is
/// not an independent authority surface and grants no rights by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncumbranceHandoffReceipt {
    stock_event_count: u64,
    release_sequence: u64,
    successor_reserve_sequence: u64,
    handoff_cause_id: CausalId,
    source: StockReservation,
    successor: StockReservation,
    conserved_reserved_quantity: u64,
    conserved_available_quantity: u64,
}

impl EncumbranceHandoffReceipt {
    pub const fn stock_event_count(&self) -> u64 {
        self.stock_event_count
    }

    pub const fn release_sequence(&self) -> u64 {
        self.release_sequence
    }

    pub const fn successor_reserve_sequence(&self) -> u64 {
        self.successor_reserve_sequence
    }

    pub fn handoff_cause_id(&self) -> &CausalId {
        &self.handoff_cause_id
    }

    pub fn source(&self) -> &StockReservation {
        &self.source
    }

    pub fn successor(&self) -> &StockReservation {
        &self.successor
    }

    pub const fn conserved_reserved_quantity(&self) -> u64 {
        self.conserved_reserved_quantity
    }

    pub const fn conserved_available_quantity(&self) -> u64 {
        self.conserved_available_quantity
    }

    /// Re-derive the handoff from retained histories at the exact historical
    /// boundary recorded by this receipt.
    ///
    /// Later stock or reservation events are allowed. Validation truncates both
    /// histories to the original handoff point before replaying the theorem.
    pub fn validate(
        &self,
        stock_history: &StockLedger,
        reservation_history: &StockReservationLedger,
    ) -> Result<(), EncumbranceHandoffError> {
        stock_history.validate()?;
        reservation_history.validate()?;
        validate_successor_continuity(&self.source, &self.successor)?;

        let stock_event_count = usize::try_from(self.stock_event_count)
            .map_err(|_| EncumbranceHandoffError::ArithmeticOverflow)?;
        if stock_event_count > stock_history.entries().len() {
            return Err(EncumbranceHandoffError::StockHistoryTooShort);
        }
        let stock_at_handoff = StockLedger::from_entries(
            stock_history.entries()[..stock_event_count].to_vec(),
        )?;

        let release_index = sequence_index(self.release_sequence)?;
        let reserve_index = sequence_index(self.successor_reserve_sequence)?;
        if self.successor_reserve_sequence
            != self
                .release_sequence
                .checked_add(1)
                .ok_or(EncumbranceHandoffError::ArithmeticOverflow)?
        {
            return Err(EncumbranceHandoffError::NonAdjacentHandoff);
        }
        if reserve_index >= reservation_history.entries().len() {
            return Err(EncumbranceHandoffError::ReservationHistoryTooShort);
        }

        let release_entry = &reservation_history.entries()[release_index];
        match &release_entry.event {
            StockReservationEvent::Release {
                reservation_id,
                expected_holder_id,
                cause_id,
            } if reservation_id == self.source.reservation_id()
                && expected_holder_id == self.source.holder_id()
                && cause_id == &self.handoff_cause_id => {}
            _ => return Err(EncumbranceHandoffError::ReleaseEvidenceMismatch),
        }

        let reserve_entry = &reservation_history.entries()[reserve_index];
        match &reserve_entry.event {
            StockReservationEvent::Reserve { reservation }
                if reservation == &self.successor => {}
            _ => return Err(EncumbranceHandoffError::SuccessorEvidenceMismatch),
        }

        let before = StockReservationLedger::from_entries(
            &stock_at_handoff,
            reservation_history.entries()[..release_index].to_vec(),
        )?;
        let after = StockReservationLedger::from_entries(
            &stock_at_handoff,
            reservation_history.entries()[..=reserve_index].to_vec(),
        )?;

        if before.active_reservation(self.source.reservation_id()) != Some(&self.source) {
            return Err(EncumbranceHandoffError::SourceEvidenceMismatch);
        }
        if before
            .active_reservation(self.successor.reservation_id())
            .is_some()
        {
            return Err(EncumbranceHandoffError::SuccessorAlreadyActiveBeforeHandoff);
        }
        if after
            .active_reservation(self.source.reservation_id())
            .is_some()
        {
            return Err(EncumbranceHandoffError::SourceStillActiveAfterHandoff);
        }
        if after.active_reservation(self.successor.reservation_id()) != Some(&self.successor) {
            return Err(EncumbranceHandoffError::SuccessorEvidenceMismatch);
        }

        let lot_id = self.source.lot_id();
        let reserved_before = before.reserved_quantity(&stock_at_handoff, lot_id)?;
        let reserved_after = after.reserved_quantity(&stock_at_handoff, lot_id)?;
        let available_before = before.available_quantity(&stock_at_handoff, lot_id)?;
        let available_after = after.available_quantity(&stock_at_handoff, lot_id)?;
        if reserved_before != reserved_after
            || available_before != available_after
            || reserved_before != self.conserved_reserved_quantity
            || available_before != self.conserved_available_quantity
        {
            return Err(EncumbranceHandoffError::QuantityConservationMismatch);
        }

        Ok(())
    }
}

/// Atomically replace one complete active reservation with one successor claim.
///
/// The source quantity is not released to general availability between phases:
/// both reservation events are validated and committed together by ECON-04B's
/// transaction primitive. V0.1 deliberately supports only full handoff.
#[allow(clippy::too_many_arguments)]
pub fn handoff_full_reservation(
    stock: &StockLedger,
    reservations: &mut StockReservationLedger,
    source_reservation_id: StockReservationId,
    expected_source_holder_id: ActorId,
    successor_reservation_id: StockReservationId,
    successor_holder_id: ActorId,
    successor_purpose_id: CausalId,
    successor_authorization_id: CausalId,
    handoff_cause_id: CausalId,
) -> Result<EncumbranceHandoffReceipt, EncumbranceHandoffError> {
    stock.validate()?;
    reservations.validate_against_stock(stock)?;

    let source = reservations
        .active_reservation(&source_reservation_id)
        .cloned()
        .ok_or_else(|| EncumbranceHandoffError::UnknownSourceReservation {
            reservation_id: source_reservation_id.clone(),
        })?;
    if source.holder_id() != &expected_source_holder_id {
        return Err(EncumbranceHandoffError::SourceHolderMismatch {
            expected: expected_source_holder_id,
            actual: source.holder_id().clone(),
        });
    }
    if source_reservation_id == successor_reservation_id {
        return Err(EncumbranceHandoffError::ReservationIdentityReused);
    }

    let successor = StockReservation::new(
        stock,
        successor_reservation_id,
        source.lot_id().clone(),
        source.quantity(),
        source.authorized_owner_id().clone(),
        successor_holder_id,
        successor_purpose_id,
        successor_authorization_id,
    )?;
    validate_successor_continuity(&source, &successor)?;

    let reserved_before = reservations.reserved_quantity(stock, source.lot_id())?;
    let available_before = reservations.available_quantity(stock, source.lot_id())?;
    let stock_event_count = u64::try_from(stock.entries().len())
        .map_err(|_| EncumbranceHandoffError::ArithmeticOverflow)?;
    let release_sequence = next_sequence(reservations.entries().len())?;
    let successor_reserve_sequence = release_sequence
        .checked_add(1)
        .ok_or(EncumbranceHandoffError::ArithmeticOverflow)?;

    let mut candidate = reservations.clone();
    candidate.transact(
        stock,
        vec![
            StockReservationEvent::Release {
                reservation_id: source.reservation_id().clone(),
                expected_holder_id: source.holder_id().clone(),
                cause_id: handoff_cause_id.clone(),
            },
            StockReservationEvent::Reserve {
                reservation: successor.clone(),
            },
        ],
    )?;

    let reserved_after = candidate.reserved_quantity(stock, source.lot_id())?;
    let available_after = candidate.available_quantity(stock, source.lot_id())?;
    if reserved_before != reserved_after || available_before != available_after {
        return Err(EncumbranceHandoffError::QuantityConservationMismatch);
    }
    if candidate
        .active_reservation(source.reservation_id())
        .is_some()
    {
        return Err(EncumbranceHandoffError::SourceStillActiveAfterHandoff);
    }
    if candidate.active_reservation(successor.reservation_id()) != Some(&successor) {
        return Err(EncumbranceHandoffError::SuccessorEvidenceMismatch);
    }

    let receipt = EncumbranceHandoffReceipt {
        stock_event_count,
        release_sequence,
        successor_reserve_sequence,
        handoff_cause_id,
        source,
        successor,
        conserved_reserved_quantity: reserved_before,
        conserved_available_quantity: available_before,
    };
    receipt.validate(stock, &candidate)?;

    *reservations = candidate;
    Ok(receipt)
}

fn validate_successor_continuity(
    source: &StockReservation,
    successor: &StockReservation,
) -> Result<(), EncumbranceHandoffError> {
    if source.lot_id() != successor.lot_id()
        || source.commodity_spec_id() != successor.commodity_spec_id()
        || source.quantity() != successor.quantity()
        || source.authorized_owner_id() != successor.authorized_owner_id()
    {
        return Err(EncumbranceHandoffError::IdentityContinuityMismatch);
    }
    if source.holder_id() == successor.holder_id()
        && source.purpose_id() == successor.purpose_id()
        && source.authorization_id() == successor.authorization_id()
    {
        return Err(EncumbranceHandoffError::NoSemanticHandoff);
    }
    Ok(())
}

fn next_sequence(current_len: usize) -> Result<u64, EncumbranceHandoffError> {
    u64::try_from(current_len)
        .map_err(|_| EncumbranceHandoffError::ArithmeticOverflow)?
        .checked_add(1)
        .ok_or(EncumbranceHandoffError::ArithmeticOverflow)
}

fn sequence_index(sequence: u64) -> Result<usize, EncumbranceHandoffError> {
    if sequence == 0 {
        return Err(EncumbranceHandoffError::InvalidSequence);
    }
    usize::try_from(sequence - 1).map_err(|_| EncumbranceHandoffError::ArithmeticOverflow)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncumbranceHandoffError {
    Economic(EconomicError),
    Reservation(ReservationError),
    ArithmeticOverflow,
    InvalidSequence,
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
    IdentityContinuityMismatch,
    NoSemanticHandoff,
    NonAdjacentHandoff,
    ReleaseEvidenceMismatch,
    SourceEvidenceMismatch,
    SuccessorEvidenceMismatch,
    SuccessorAlreadyActiveBeforeHandoff,
    SourceStillActiveAfterHandoff,
    QuantityConservationMismatch,
}

impl From<EconomicError> for EncumbranceHandoffError {
    fn from(value: EconomicError) -> Self {
        Self::Economic(value)
    }
}

impl From<ReservationError> for EncumbranceHandoffError {
    fn from(value: ReservationError) -> Self {
        Self::Reservation(value)
    }
}
