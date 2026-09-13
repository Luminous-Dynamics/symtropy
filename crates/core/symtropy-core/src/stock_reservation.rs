// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Append-only stock reservation/encumbrance authority.
//!
//! A reservation does not create stock, transfer ownership, transfer custody,
//! relocate a lot, settle money, or prove a contract. It only removes a quantity
//! from the presently available portion of one already-live economic lot.
//!
//! The ledger is intentionally layered over [`crate::economic::StockLedger`]
//! instead of modifying ECON-00/01. This lets later market, manufacturing, and
//! logistics code prevent double-promising stock while preserving the original
//! stock ledger as the sole quantity/ownership/custody/location authority.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::economic::{ActorId, CausalId, CommoditySpecId, LotId, StockLedger};

const MAX_ID_LEN: usize = 256;
const MAX_RESERVATIONS: usize = 131_072;
const MAX_RESERVATION_EVENTS: usize = 524_288;

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

/// One active claim on part or all of a live stock lot.
///
/// `authorized_owner_id` is bound to the current owner at reservation creation.
/// While the reservation remains active, validation requires that owner to remain
/// unchanged. Ownership transfer therefore cannot silently drag an encumbrance to
/// a new principal.
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
    /// Construct a reservation event payload from one validated live stock lot.
    ///
    /// This proves lot/spec/owner identity at construction time. Aggregate
    /// availability remains the responsibility of `StockReservationLedger`, so
    /// several independently prepared reservations can still be admitted or
    /// rejected atomically by one `transact()` call.
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

/// Complete mutation grammar for the reservation authority.
///
/// There is deliberately no quantity-increase or reassignment event. Increasing
/// an encumbrance requires a new reservation ID; changing beneficiary or purpose
/// requires releasing the old claim and authoring a new one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StockReservationEvent {
    Reserve {
        reservation: StockReservation,
    },
    /// Release the whole remaining active reservation.
    Release {
        reservation_id: StockReservationId,
        expected_holder_id: ActorId,
        cause_id: CausalId,
    },
    /// Release only part of an active reservation. `quantity` must be strictly
    /// less than the remaining reservation; whole release uses `Release`.
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

/// Replayable reservation ledger with a materialized active-claim projection.
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

    /// Reconstruct the materialized reservation state from append-only history,
    /// then prove that every remaining active claim is valid against current stock.
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

    /// Independently replay history and require exact equality with both
    /// materialized active state and historical reservation-ID census.
    pub fn validate(&self) -> Result<(), ReservationError> {
        if self.active.len() > MAX_RESERVATIONS
            || self.seen_ids.len() > MAX_RESERVATIONS
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

    /// Prove that all active reservations remain possible in the supplied stock
    /// snapshot and that no lot is over-encumbered.
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

    /// Sum active reservations against one lot after proving that the complete
    /// overlay is valid against the supplied stock state.
    pub fn reserved_quantity(
        &self,
        stock: &StockLedger,
        lot_id: &LotId,
    ) -> Result<u64, ReservationError> {
        self.validate_against_stock(stock)?;
        self.reserved_quantity_unchecked(lot_id)
    }

    /// Authoritative stock available to new claims at this instant.
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
        let reserved = self.reserved_quantity_unchecked(lot_id)?;
        lot.quantity()
            .checked_sub(reserved)
            .ok_or(ReservationError::ArithmeticOverflow)
    }

    /// Reserve stock only under the current owner's explicit identity and causal
    /// authorization reference.
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

    /// Atomically apply multiple reservation transitions. A transaction may repair
    /// stock-relative invalidity (for example by fully releasing a reservation
    /// whose owner changed), so only structural ledger validity is required before
    /// applying events. The resulting candidate MUST validate against current stock
    /// before either materialized state or history is committed.
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
            if seen_ids.len() >= MAX_RESERVATIONS {
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
                crate::economic::StockLot::new(
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
    fn reservations_reduce_available_without_mutating_stock() {
        let stock = stock(100);
        let original_entries = stock.entries().to_vec();
        let mut reservations = StockReservationLedger::new();

        reserve(&mut reservations, &stock, "r1", 30, "buyer-a").unwrap();
        reserve(&mut reservations, &stock, "r2", 20, "buyer-b").unwrap();

        assert_eq!(
            reservations.reserved_quantity(&stock, &lot_id("steel-a")),
            Ok(50)
        );
        assert_eq!(
            reservations.available_quantity(&stock, &lot_id("steel-a")),
            Ok(50)
        );
        assert_eq!(stock.lot(&lot_id("steel-a")).unwrap().quantity(), 100);
        assert_eq!(stock.entries(), original_entries.as_slice());
    }

    #[test]
    fn over_reservation_is_atomic_and_fails_closed() {
        let stock = stock(100);
        let mut reservations = StockReservationLedger::new();
        reserve(&mut reservations, &stock, "r1", 70, "buyer-a").unwrap();
        let before = reservations.clone();

        assert!(matches!(
            reserve(&mut reservations, &stock, "r2", 31, "buyer-b"),
            Err(ReservationError::OverReserved { .. })
        ));
        assert_eq!(reservations, before);
    }

    #[test]
    fn duplicate_reservation_id_cannot_be_reused_after_release() {
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

        assert!(matches!(
            reserve(&mut reservations, &stock, "r1", 10, "buyer"),
            Err(ReservationError::DuplicateReservation { .. })
        ));
    }

    #[test]
    fn partial_release_only_decreases_claim() {
        let stock = stock(100);
        let mut reservations = StockReservationLedger::new();
        reserve(&mut reservations, &stock, "r1", 70, "buyer").unwrap();

        reservations
            .release_partial(
                &stock,
                reservation_id("r1"),
                actor("buyer"),
                30,
                cause("scope-reduced"),
            )
            .unwrap();
        assert_eq!(
            reservations
                .active_reservation(&reservation_id("r1"))
                .unwrap()
                .quantity(),
            40
        );
        assert_eq!(
            reservations.available_quantity(&stock, &lot_id("steel-a")),
            Ok(60)
        );
    }

    #[test]
    fn whole_quantity_must_use_explicit_release() {
        let stock = stock(100);
        let mut reservations = StockReservationLedger::new();
        reserve(&mut reservations, &stock, "r1", 20, "buyer").unwrap();

        assert!(matches!(
            reservations.release_partial(
                &stock,
                reservation_id("r1"),
                actor("buyer"),
                20,
                cause("bad-partial"),
            ),
            Err(ReservationError::InvalidPartialRelease { .. })
        ));
    }

    #[test]
    fn owner_change_makes_active_reservation_invalid() {
        let mut stock = stock(100);
        let mut reservations = StockReservationLedger::new();
        reserve(&mut reservations, &stock, "r1", 20, "buyer").unwrap();

        stock
            .transfer_ownership(
                lot_id("steel-a"),
                actor("owner"),
                actor("new-owner"),
                cause("sale-outside-reservation-layer"),
            )
            .unwrap();

        assert!(matches!(
            reservations.validate_against_stock(&stock),
            Err(ReservationError::AuthorizedOwnerChanged { .. })
        ));
    }

    #[test]
    fn full_release_can_repair_owner_changed_reservation() {
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

        reservations
            .release(
                &stock,
                reservation_id("r1"),
                actor("buyer"),
                cause("repair-stale-claim"),
            )
            .unwrap();
        assert!(reservations.active_reservations().next().is_none());
        reservations.validate_against_stock(&stock).unwrap();
    }

    #[test]
    fn partial_release_cannot_leave_owner_changed_claim_active() {
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
        let before = reservations.clone();

        assert!(matches!(
            reservations.release_partial(
                &stock,
                reservation_id("r1"),
                actor("buyer"),
                5,
                cause("insufficient-repair"),
            ),
            Err(ReservationError::AuthorizedOwnerChanged { .. })
        ));
        assert_eq!(reservations, before);
    }

    #[test]
    fn underlying_depletion_cannot_silently_overrun_reservation() {
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

    #[test]
    fn checked_constructor_supports_atomic_multi_reserve() {
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
            40,
            actor("owner"),
            actor("buyer-b"),
            cause("r2-purpose"),
            cause("r2-auth"),
        )
        .unwrap();
        let mut reservations = StockReservationLedger::new();
        reservations
            .transact(
                &stock,
                vec![
                    StockReservationEvent::Reserve { reservation: r1 },
                    StockReservationEvent::Reserve { reservation: r2 },
                ],
            )
            .unwrap();
        assert_eq!(
            reservations.available_quantity(&stock, &lot_id("steel-a")),
            Ok(0)
        );
    }

    #[test]
    fn atomic_multi_reserve_detects_combined_overbooking() {
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
    fn released_history_replays_exactly() {
        let stock = stock(100);
        let mut reservations = StockReservationLedger::new();
        reserve(&mut reservations, &stock, "r1", 30, "buyer").unwrap();
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
        assert!(replayed.active_reservations().next().is_none());
        assert!(replayed.has_seen(&reservation_id("r1")));
    }
}
