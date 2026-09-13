// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Append-only stock reservation/encumbrance authority.
//!
//! A reservation does not create stock, transfer ownership, transfer custody,
//! relocate a lot, settle money, or prove a contract. It removes quantity only
//! from the presently available portion of an already-live economic lot.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::economic::{ActorId, CausalId, CommoditySpecId, LotId, StockLedger};

const MAX_ID_LEN: usize = 256;
const MAX_ACTIVE_RESERVATIONS: usize = 131_072;
const MAX_RESERVATION_EVENTS: usize = 524_288;
const MAX_SEEN_RESERVATION_IDS: usize = MAX_RESERVATION_EVENTS;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StockReservationId(String);

impl StockReservationId {
    pub fn new(value: impl Into<String>) -> Result<Self, ReservationError> {
        let value = value.into();
        validate_id("stock-reservation", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StockReservationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// One active claim on part or all of one live stock lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockReservation {
    reservation_id: StockReservationId,
    lot_id: LotId,
    commodity_spec_id: CommoditySpecId,
    quantity: u64,
    authorized_owner_id: ActorId,
    holder_id: ActorId,
    purpose_id: CausalId,
    authorization_id: CausalId,
}

impl StockReservation {
    /// Prepare a reservation payload from one validated live stock lot.
    /// Aggregate availability is checked only when the reservation ledger admits
    /// the complete candidate transaction.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stock: &StockLedger,
        reservation_id: StockReservationId,
        lot_id: LotId,
        quantity: u64,
        expected_owner_id: ActorId,
        holder_id: ActorId,
        purpose_id: CausalId,
        authorization_id: CausalId,
    ) -> Result<Self, ReservationError> {
        if quantity == 0 {
            return Err(ReservationError::ZeroQuantity);
        }
        if stock.validate().is_err() {
            return Err(ReservationError::InvalidStockLedger);
        }
        let lot = stock
            .lot(&lot_id)
            .ok_or_else(|| ReservationError::UnknownLot {
                lot_id: lot_id.clone(),
            })?;
        if lot.owner_id() != &expected_owner_id {
            return Err(ReservationError::OwnerMismatch {
                lot_id,
                expected: expected_owner_id,
                actual: lot.owner_id().clone(),
            });
        }
        Ok(Self {
            reservation_id,
            lot_id: lot.lot_id().clone(),
            commodity_spec_id: lot.commodity_spec_id().clone(),
            quantity,
            authorized_owner_id: expected_owner_id,
            holder_id,
            purpose_id,
            authorization_id,
        })
    }

    pub fn reservation_id(&self) -> &StockReservationId {
        &self.reservation_id
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

    pub fn authorized_owner_id(&self) -> &ActorId {
        &self.authorized_owner_id
    }

    pub fn holder_id(&self) -> &ActorId {
        &self.holder_id
    }

    pub fn purpose_id(&self) -> &CausalId {
        &self.purpose_id
    }

    pub fn authorization_id(&self) -> &CausalId {
        &self.authorization_id
    }
}

/// Complete v0.1 reservation mutation grammar.
///
/// There is no generic quantity increase or identity reassignment. Increasing a
/// claim requires a new reservation ID; changing its meaning requires release and
/// a new reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StockReservationEvent {
    Reserve {
        reservation: StockReservation,
    },
    Release {
        reservation_id: StockReservationId,
        expected_holder_id: ActorId,
        cause_id: CausalId,
    },
    ReleasePartial {
        reservation_id: StockReservationId,
        expected_holder_id: ActorId,
        quantity: u64,
        cause_id: CausalId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockReservationLedgerEntry {
    pub sequence: u64,
    pub event: StockReservationEvent,
}

/// Replayable reservation authority with a materialized active-claim projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockReservationLedger {
    active: BTreeMap<StockReservationId, StockReservation>,
    seen_ids: BTreeSet<StockReservationId>,
    entries: Vec<StockReservationLedgerEntry>,
}

impl Default for StockReservationLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl StockReservationLedger {
    pub const fn new() -> Self {
        Self {
            active: BTreeMap::new(),
            seen_ids: BTreeSet::new(),
            entries: Vec::new(),
        }
    }

    pub fn from_entries(
        stock: &StockLedger,
        entries: Vec<StockReservationLedgerEntry>,
    ) -> Result<Self, ReservationError> {
        if entries.len() > MAX_RESERVATION_EVENTS {
            return Err(ReservationError::ModelTooLarge);
        }
        let (active, seen_ids) = replay_entries(&entries)?;
        let ledger = Self {
            active,
            seen_ids,
            entries,
        };
        ledger.validate_against_stock(stock)?;
        Ok(ledger)
    }

    /// Replay history independently and compare both materialized active state and
    /// the complete retained reservation-ID census.
    pub fn validate(&self) -> Result<(), ReservationError> {
        if self.active.len() > MAX_ACTIVE_RESERVATIONS
            || self.seen_ids.len() > MAX_SEEN_RESERVATION_IDS
            || self.entries.len() > MAX_RESERVATION_EVENTS
        {
            return Err(ReservationError::ModelTooLarge);
        }
        let (active, seen_ids) = replay_entries(&self.entries)?;
        if active != self.active || seen_ids != self.seen_ids {
            return Err(ReservationError::LedgerHistoryMismatch);
        }
        Ok(())
    }

    /// Prove all active claims remain possible in this exact stock snapshot.
    pub fn validate_against_stock(&self, stock: &StockLedger) -> Result<(), ReservationError> {
        self.validate()?;
        if stock.validate().is_err() {
            return Err(ReservationError::InvalidStockLedger);
        }

        let mut reserved_by_lot: BTreeMap<LotId, u64> = BTreeMap::new();
        for reservation in self.active.values() {
            let lot = stock
                .lot(&reservation.lot_id)
                .ok_or_else(|| ReservationError::UnknownLot {
                    lot_id: reservation.lot_id.clone(),
                })?;
            if lot.commodity_spec_id() != &reservation.commodity_spec_id {
                return Err(ReservationError::CommoditySpecMismatch {
                    reservation_id: reservation.reservation_id.clone(),
                    expected: reservation.commodity_spec_id.clone(),
                    actual: lot.commodity_spec_id().clone(),
                });
            }
            if lot.owner_id() != &reservation.authorized_owner_id {
                return Err(ReservationError::AuthorizedOwnerChanged {
                    reservation_id: reservation.reservation_id.clone(),
                    expected: reservation.authorized_owner_id.clone(),
                    actual: lot.owner_id().clone(),
                });
            }

            let total = reserved_by_lot
                .entry(reservation.lot_id.clone())
                .or_default();
            *total = total
                .checked_add(reservation.quantity)
                .ok_or(ReservationError::ArithmeticOverflow)?;
            if *total > lot.quantity() {
                return Err(ReservationError::OverReserved {
                    lot_id: reservation.lot_id.clone(),
                    reserved: *total,
                    stock_quantity: lot.quantity(),
                });
            }
        }
        Ok(())
    }

    pub fn active_reservation(
        &self,
        reservation_id: &StockReservationId,
    ) -> Option<&StockReservation> {
        self.active.get(reservation_id)
    }

    pub fn active_reservations(&self) -> impl ExactSizeIterator<Item = &StockReservation> {
        self.active.values()
    }

    pub fn entries(&self) -> &[StockReservationLedgerEntry] {
        &self.entries
    }

    pub fn has_seen(&self, reservation_id: &StockReservationId) -> bool {
        self.seen_ids.contains(reservation_id)
    }

    pub fn reserved_quantity(
        &self,
        stock: &StockLedger,
        lot_id: &LotId,
    ) -> Result<u64, ReservationError> {
        self.validate_against_stock(stock)?;
        self.reserved_quantity_unchecked(lot_id)
    }

    /// Quantity in a live lot that is not presently claimed by this authority.
    pub fn available_quantity(
        &self,
        stock: &StockLedger,
        lot_id: &LotId,
    ) -> Result<u64, ReservationError> {
        self.validate_against_stock(stock)?;
        let lot = stock
            .lot(lot_id)
            .ok_or_else(|| ReservationError::UnknownLot {
                lot_id: lot_id.clone(),
            })?;
        lot.quantity()
            .checked_sub(self.reserved_quantity_unchecked(lot_id)?)
            .ok_or(ReservationError::ArithmeticOverflow)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reserve(
        &mut self,
        stock: &StockLedger,
        reservation_id: StockReservationId,
        lot_id: LotId,
        quantity: u64,
        expected_owner_id: ActorId,
        holder_id: ActorId,
        purpose_id: CausalId,
        authorization_id: CausalId,
    ) -> Result<(), ReservationError> {
        let reservation = StockReservation::new(
            stock,
            reservation_id,
            lot_id,
            quantity,
            expected_owner_id,
            holder_id,
            purpose_id,
            authorization_id,
        )?;
        self.transact(
            stock,
            vec![StockReservationEvent::Reserve { reservation }],
        )
    }

    pub fn release(
        &mut self,
        stock: &StockLedger,
        reservation_id: StockReservationId,
        expected_holder_id: ActorId,
        cause_id: CausalId,
    ) -> Result<(), ReservationError> {
        self.transact(
            stock,
            vec![StockReservationEvent::Release {
                reservation_id,
                expected_holder_id,
                cause_id,
            }],
        )
    }

    pub fn release_partial(
        &mut self,
        stock: &StockLedger,
        reservation_id: StockReservationId,
        expected_holder_id: ActorId,
        quantity: u64,
        cause_id: CausalId,
    ) -> Result<(), ReservationError> {
        self.transact(
            stock,
            vec![StockReservationEvent::ReleasePartial {
                reservation_id,
                expected_holder_id,
                quantity,
                cause_id,
            }],
        )
    }

    /// Apply several reservation transitions atomically.
    ///
    /// Stock-relative invalidity may be repaired by this transaction (for example,
    /// by fully releasing a stale claim), so only structural ledger validity is
    /// required before event application. The final candidate must validate against
    /// current stock before any state/history commit occurs.
    pub fn transact(
        &mut self,
        stock: &StockLedger,
        events: Vec<StockReservationEvent>,
    ) -> Result<(), ReservationError> {
        if events.is_empty() {
            return Err(ReservationError::EmptyTransaction);
        }
        self.validate()?;
        if stock.validate().is_err() {
            return Err(ReservationError::InvalidStockLedger);
        }

        let final_len = self
            .entries
            .len()
            .checked_add(events.len())
            .ok_or(ReservationError::ArithmeticOverflow)?;
        if final_len > MAX_RESERVATION_EVENTS {
            return Err(ReservationError::ModelTooLarge);
        }

        let mut candidate_active = self.active.clone();
        let mut candidate_seen = self.seen_ids.clone();
        for event in &events {
            apply_event(&mut candidate_active, &mut candidate_seen, event)?;
        }

        let mut next_sequence = u64::try_from(self.entries.len())
            .map_err(|_| ReservationError::ArithmeticOverflow)?
            .checked_add(1)
            .ok_or(ReservationError::ArithmeticOverflow)?;
        let mut candidate_entries = self.entries.clone();
        for event in events {
            candidate_entries.push(StockReservationLedgerEntry {
                sequence: next_sequence,
                event,
            });
            next_sequence = next_sequence
                .checked_add(1)
                .ok_or(ReservationError::ArithmeticOverflow)?;
        }

        let (replayed_active, replayed_seen) = replay_entries(&candidate_entries)?;
        if replayed_active != candidate_active || replayed_seen != candidate_seen {
            return Err(ReservationError::LedgerHistoryMismatch);
        }
        let candidate = Self {
            active: candidate_active,
            seen_ids: candidate_seen,
            entries: candidate_entries,
        };
        candidate.validate_against_stock(stock)?;

        self.active = candidate.active;
        self.seen_ids = candidate.seen_ids;
        self.entries = candidate.entries;
        Ok(())
    }

    fn reserved_quantity_unchecked(&self, lot_id: &LotId) -> Result<u64, ReservationError> {
        self.active
            .values()
            .filter(|reservation| &reservation.lot_id == lot_id)
            .try_fold(0_u64, |total, reservation| {
                total
                    .checked_add(reservation.quantity)
                    .ok_or(ReservationError::ArithmeticOverflow)
            })
    }
}

fn replay_entries(
    entries: &[StockReservationLedgerEntry],
) -> Result<
    (
        BTreeMap<StockReservationId, StockReservation>,
        BTreeSet<StockReservationId>,
    ),
    ReservationError,
> {
    if entries.len() > MAX_RESERVATION_EVENTS {
        return Err(ReservationError::ModelTooLarge);
    }
    let mut active = BTreeMap::new();
    let mut seen_ids = BTreeSet::new();
    for (index, entry) in entries.iter().enumerate() {
        let expected = u64::try_from(index)
            .map_err(|_| ReservationError::ArithmeticOverflow)?
            .checked_add(1)
            .ok_or(ReservationError::ArithmeticOverflow)?;
        if entry.sequence != expected {
            return Err(ReservationError::InvalidSequence {
                expected,
                actual: entry.sequence,
            });
        }
        apply_event(&mut active, &mut seen_ids, &entry.event)?;
    }
    Ok((active, seen_ids))
}

fn apply_event(
    active: &mut BTreeMap<StockReservationId, StockReservation>,
    seen_ids: &mut BTreeSet<StockReservationId>,
    event: &StockReservationEvent,
) -> Result<(), ReservationError> {
    match event {
        StockReservationEvent::Reserve { reservation } => {
            if reservation.quantity == 0 {
                return Err(ReservationError::ZeroQuantity);
            }
            if seen_ids.contains(&reservation.reservation_id) {
                return Err(ReservationError::DuplicateReservation {
                    reservation_id: reservation.reservation_id.clone(),
                });
            }
            if active.len() >= MAX_ACTIVE_RESERVATIONS
                || seen_ids.len() >= MAX_SEEN_RESERVATION_IDS
            {
                return Err(ReservationError::ModelTooLarge);
            }
            seen_ids.insert(reservation.reservation_id.clone());
            active.insert(reservation.reservation_id.clone(), reservation.clone());
        }
        StockReservationEvent::Release {
            reservation_id,
            expected_holder_id,
            ..
        } => {
            let reservation = active.get(reservation_id).ok_or_else(|| {
                ReservationError::UnknownActiveReservation {
                    reservation_id: reservation_id.clone(),
                }
            })?;
            if &reservation.holder_id != expected_holder_id {
                return Err(ReservationError::HolderMismatch {
                    reservation_id: reservation_id.clone(),
                    expected: expected_holder_id.clone(),
                    actual: reservation.holder_id.clone(),
                });
            }
            active.remove(reservation_id);
        }
        StockReservationEvent::ReleasePartial {
            reservation_id,
            expected_holder_id,
            quantity,
            ..
        } => {
            if *quantity == 0 {
                return Err(ReservationError::ZeroQuantity);
            }
            let reservation = active.get_mut(reservation_id).ok_or_else(|| {
                ReservationError::UnknownActiveReservation {
                    reservation_id: reservation_id.clone(),
                }
            })?;
            if &reservation.holder_id != expected_holder_id {
                return Err(ReservationError::HolderMismatch {
                    reservation_id: reservation_id.clone(),
                    expected: expected_holder_id.clone(),
                    actual: reservation.holder_id.clone(),
                });
            }
            if *quantity >= reservation.quantity {
                return Err(ReservationError::InvalidPartialRelease {
                    reservation_id: reservation_id.clone(),
                    active_quantity: reservation.quantity,
                    release_quantity: *quantity,
                });
            }
            reservation.quantity = reservation
                .quantity
                .checked_sub(*quantity)
                .ok_or(ReservationError::ArithmeticOverflow)?;
        }
    }
    Ok(())
}

fn validate_id(kind: &'static str, value: &str) -> Result<(), ReservationError> {
    if value.is_empty() || value.len() > MAX_ID_LEN || value.trim() != value {
        return Err(ReservationError::InvalidId {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationError {
    InvalidId {
        kind: &'static str,
        value: String,
    },
    InvalidStockLedger,
    ZeroQuantity,
    ModelTooLarge,
    ArithmeticOverflow,
    EmptyTransaction,
    InvalidSequence {
        expected: u64,
        actual: u64,
    },
    LedgerHistoryMismatch,
    DuplicateReservation {
        reservation_id: StockReservationId,
    },
    UnknownActiveReservation {
        reservation_id: StockReservationId,
    },
    UnknownLot {
        lot_id: LotId,
    },
    OwnerMismatch {
        lot_id: LotId,
        expected: ActorId,
        actual: ActorId,
    },
    AuthorizedOwnerChanged {
        reservation_id: StockReservationId,
        expected: ActorId,
        actual: ActorId,
    },
    CommoditySpecMismatch {
        reservation_id: StockReservationId,
        expected: CommoditySpecId,
        actual: CommoditySpecId,
    },
    HolderMismatch {
        reservation_id: StockReservationId,
        expected: ActorId,
        actual: ActorId,
    },
    OverReserved {
        lot_id: LotId,
        reserved: u64,
        stock_quantity: u64,
    },
    InvalidPartialRelease {
        reservation_id: StockReservationId,
        active_quantity: u64,
        release_quantity: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economic::{LocationId, StockDepletionCause, StockLot, StockOrigin};

    fn actor(value: &str) -> ActorId {
        ActorId::new(value).unwrap()
    }

    fn cause(value: &str) -> CausalId {
        CausalId::new(value).unwrap()
    }

    fn lot_id(value: &str) -> LotId {
        LotId::new(value).unwrap()
    }

    fn reservation_id(value: &str) -> StockReservationId {
        StockReservationId::new(value).unwrap()
    }

    fn stock(quantity: u64) -> StockLedger {
        let mut ledger = StockLedger::new();
        ledger
            .establish(
                StockLot::new(
                    lot_id("steel-a"),
                    CommoditySpecId::new("steel").unwrap(),
                    quantity,
                    actor("owner"),
                    actor("warehouse"),
                    LocationId::new("yard").unwrap(),
                )
                .unwrap(),
                StockOrigin::QualifiedInitial {
                    evidence_id: cause("stock-evidence"),
                },
            )
            .unwrap();
        ledger
    }

    fn reserve(
        reservations: &mut StockReservationLedger,
        stock: &StockLedger,
        id: &str,
        quantity: u64,
        holder: &str,
    ) -> Result<(), ReservationError> {
        reservations.reserve(
            stock,
            reservation_id(id),
            lot_id("steel-a"),
            quantity,
            actor("owner"),
            actor(holder),
            cause(&format!("{id}-purpose")),
            cause(&format!("{id}-authorization")),
        )
    }

    #[test]
    fn reservation_changes_availability_not_stock() {
        let stock = stock(100);
        let stock_history = stock.entries().to_vec();
        let mut reservations = StockReservationLedger::new();
        reserve(&mut reservations, &stock, "r1", 30, "buyer-a").unwrap();
        reserve(&mut reservations, &stock, "r2", 20, "buyer-b").unwrap();

        assert_eq!(reservations.reserved_quantity(&stock, &lot_id("steel-a")), Ok(50));
        assert_eq!(reservations.available_quantity(&stock, &lot_id("steel-a")), Ok(50));
        assert_eq!(stock.lot(&lot_id("steel-a")).unwrap().quantity(), 100);
        assert_eq!(stock.entries(), stock_history.as_slice());
    }

    #[test]
    fn combined_overbooking_is_atomic() {
        let stock = stock(100);
        let r1 = StockReservation::new(
            &stock,
            reservation_id("r1"),
            lot_id("steel-a"),
            60,
            actor("owner"),
            actor("buyer-a"),
            cause("r1-purpose"),
            cause("r1-auth"),
        )
        .unwrap();
        let r2 = StockReservation::new(
            &stock,
            reservation_id("r2"),
            lot_id("steel-a"),
            50,
            actor("owner"),
            actor("buyer-b"),
            cause("r2-purpose"),
            cause("r2-auth"),
        )
        .unwrap();
        let mut reservations = StockReservationLedger::new();
        let before = reservations.clone();
        assert!(matches!(
            reservations.transact(
                &stock,
                vec![
                    StockReservationEvent::Reserve { reservation: r1 },
                    StockReservationEvent::Reserve { reservation: r2 },
                ],
            ),
            Err(ReservationError::OverReserved { .. })
        ));
        assert_eq!(reservations, before);
    }

    #[test]
    fn released_id_cannot_be_reused_and_history_replays() {
        let stock = stock(100);
        let mut reservations = StockReservationLedger::new();
        reserve(&mut reservations, &stock, "r1", 20, "buyer").unwrap();
        reservations
            .release(
                &stock,
                reservation_id("r1"),
                actor("buyer"),
                cause("cancel"),
            )
            .unwrap();
        let replayed =
            StockReservationLedger::from_entries(&stock, reservations.entries().to_vec()).unwrap();
        assert_eq!(replayed, reservations);
        assert!(replayed.has_seen(&reservation_id("r1")));
        assert!(matches!(
            reserve(&mut reservations, &stock, "r1", 10, "buyer"),
            Err(ReservationError::DuplicateReservation { .. })
        ));
    }

    #[test]
    fn full_release_repairs_owner_change_but_partial_does_not() {
        let mut stock = stock(100);
        let mut reservations = StockReservationLedger::new();
        reserve(&mut reservations, &stock, "r1", 20, "buyer").unwrap();
        stock
            .transfer_ownership(
                lot_id("steel-a"),
                actor("owner"),
                actor("new-owner"),
                cause("external-title-change"),
            )
            .unwrap();
        let stale = reservations.clone();
        assert!(matches!(
            reservations.release_partial(
                &stock,
                reservation_id("r1"),
                actor("buyer"),
                5,
                cause("partial"),
            ),
            Err(ReservationError::AuthorizedOwnerChanged { .. })
        ));
        assert_eq!(reservations, stale);
        reservations
            .release(
                &stock,
                reservation_id("r1"),
                actor("buyer"),
                cause("full-release"),
            )
            .unwrap();
        reservations.validate_against_stock(&stock).unwrap();
    }

    #[test]
    fn external_depletion_cannot_silently_overrun_claim() {
        let mut stock = stock(100);
        let mut reservations = StockReservationLedger::new();
        reserve(&mut reservations, &stock, "r1", 80, "buyer").unwrap();
        stock
            .deplete(
                lot_id("steel-a"),
                30,
                StockDepletionCause::Loss {
                    incident_id: cause("fire"),
                },
            )
            .unwrap();
        assert_eq!(
            reservations.validate_against_stock(&stock),
            Err(ReservationError::OverReserved {
                lot_id: lot_id("steel-a"),
                reserved: 80,
                stock_quantity: 70,
            })
        );
    }
}
