// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! External/public-surface adversarial corpus for ECON-04D.

use symtropy_core::prelude::*;

fn actor(value: &str) -> ActorId {
    ActorId::new(value).unwrap()
}

fn cause(value: &str) -> CausalId {
    CausalId::new(value).unwrap()
}

fn lot(value: &str) -> LotId {
    LotId::new(value).unwrap()
}

fn reservation(value: &str) -> StockReservationId {
    StockReservationId::new(value).unwrap()
}

fn stock() -> StockLedger {
    let mut stock = StockLedger::new();
    stock
        .establish(
            StockLot::new(
                lot("steel-a"),
                CommoditySpecId::new("steel").unwrap(),
                100,
                actor("owner"),
                actor("warehouse"),
                LocationId::new("yard").unwrap(),
            )
            .unwrap(),
            StockOrigin::QualifiedInitial {
                evidence_id: cause("qualified-steel"),
            },
        )
        .unwrap();
    stock
}

fn reservations(stock: &StockLedger) -> StockReservationLedger {
    let mut reservations = StockReservationLedger::new();
    reservations
        .reserve(
            stock,
            reservation("purchase"),
            lot("steel-a"),
            20,
            actor("owner"),
            actor("buyer"),
            cause("purchase-purpose"),
            cause("purchase-authorization"),
        )
        .unwrap();
    reservations
        .reserve(
            stock,
            reservation("other-claim"),
            lot("steel-a"),
            10,
            actor("owner"),
            actor("factory"),
            cause("factory-purpose"),
            cause("factory-authorization"),
        )
        .unwrap();
    reservations
}

fn perform_handoff(
    stock: &StockLedger,
    reservations: &mut StockReservationLedger,
) -> EncumbranceHandoffReceipt {
    handoff_full_reservation(
        stock,
        reservations,
        reservation("purchase"),
        actor("buyer"),
        reservation("transport"),
        actor("carrier"),
        cause("transport-purpose"),
        cause("transport-authorization"),
        cause("purchase-to-transport"),
    )
    .unwrap()
}

#[test]
fn full_handoff_is_atomic_and_conserves_unavailable_quantity() {
    let stock = stock();
    let mut reservations = reservations(&stock);
    let unrelated_before = reservations
        .active_reservation(&reservation("other-claim"))
        .unwrap()
        .clone();
    assert_eq!(reservations.reserved_quantity(&stock, &lot("steel-a")).unwrap(), 30);
    assert_eq!(reservations.available_quantity(&stock, &lot("steel-a")).unwrap(), 70);

    let receipt = perform_handoff(&stock, &mut reservations);

    assert!(reservations
        .active_reservation(&reservation("purchase"))
        .is_none());
    let transport = reservations
        .active_reservation(&reservation("transport"))
        .unwrap();
    assert_eq!(transport.quantity(), 20);
    assert_eq!(transport.holder_id(), &actor("carrier"));
    assert_eq!(transport.purpose_id(), &cause("transport-purpose"));
    assert_eq!(
        reservations.active_reservation(&reservation("other-claim")),
        Some(&unrelated_before)
    );
    assert_eq!(reservations.reserved_quantity(&stock, &lot("steel-a")).unwrap(), 30);
    assert_eq!(reservations.available_quantity(&stock, &lot("steel-a")).unwrap(), 70);
    assert_eq!(receipt.conserved_reserved_quantity(), 30);
    assert_eq!(receipt.conserved_available_quantity(), 70);
    assert_eq!(
        receipt.successor_reserve_sequence(),
        receipt.release_sequence() + 1
    );
    receipt.validate(&stock, &reservations).unwrap();
}

#[test]
fn wrong_source_holder_fails_without_mutation() {
    let stock = stock();
    let mut reservations = reservations(&stock);
    let before = reservations.clone();

    assert_eq!(
        handoff_full_reservation(
            &stock,
            &mut reservations,
            reservation("purchase"),
            actor("not-the-buyer"),
            reservation("transport"),
            actor("carrier"),
            cause("transport-purpose"),
            cause("transport-authorization"),
            cause("bad-holder-handoff"),
        ),
        Err(EncumbranceHandoffError::SourceHolderMismatch {
            expected: actor("not-the-buyer"),
            actual: actor("buyer"),
        })
    );
    assert_eq!(reservations, before);
}

#[test]
fn source_identity_cannot_be_reused_as_successor() {
    let stock = stock();
    let mut reservations = reservations(&stock);
    let before = reservations.clone();

    assert_eq!(
        handoff_full_reservation(
            &stock,
            &mut reservations,
            reservation("purchase"),
            actor("buyer"),
            reservation("purchase"),
            actor("carrier"),
            cause("transport-purpose"),
            cause("transport-authorization"),
            cause("identity-reuse"),
        ),
        Err(EncumbranceHandoffError::ReservationIdentityReused)
    );
    assert_eq!(reservations, before);
}

#[test]
fn historically_seen_successor_id_fails_atomically() {
    let stock = stock();
    let mut reservations = reservations(&stock);
    reservations
        .reserve(
            &stock,
            reservation("used-id"),
            lot("steel-a"),
            5,
            actor("owner"),
            actor("temporary"),
            cause("temporary-purpose"),
            cause("temporary-authorization"),
        )
        .unwrap();
    reservations
        .release(
            &stock,
            reservation("used-id"),
            actor("temporary"),
            cause("temporary-release"),
        )
        .unwrap();
    let before = reservations.clone();

    let error = handoff_full_reservation(
        &stock,
        &mut reservations,
        reservation("purchase"),
        actor("buyer"),
        reservation("used-id"),
        actor("carrier"),
        cause("transport-purpose"),
        cause("transport-authorization"),
        cause("seen-id-handoff"),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        EncumbranceHandoffError::Reservation(ReservationError::DuplicateReservation { .. })
    ));
    assert_eq!(reservations, before);
}

#[test]
fn semantic_noop_handoff_is_rejected_without_mutation() {
    let stock = stock();
    let mut reservations = reservations(&stock);
    let source = reservations
        .active_reservation(&reservation("purchase"))
        .unwrap()
        .clone();
    let before = reservations.clone();

    assert_eq!(
        handoff_full_reservation(
            &stock,
            &mut reservations,
            reservation("purchase"),
            source.holder_id().clone(),
            reservation("renamed-only"),
            source.holder_id().clone(),
            source.purpose_id().clone(),
            source.authorization_id().clone(),
            cause("meaningless-rekey"),
        ),
        Err(EncumbranceHandoffError::NoSemanticHandoff)
    );
    assert_eq!(reservations, before);
}

#[test]
fn receipt_survives_later_reservation_history_appends() {
    let stock = stock();
    let mut reservations = reservations(&stock);
    let receipt = perform_handoff(&stock, &mut reservations);

    reservations
        .reserve(
            &stock,
            reservation("later-claim"),
            lot("steel-a"),
            5,
            actor("owner"),
            actor("later-holder"),
            cause("later-purpose"),
            cause("later-authorization"),
        )
        .unwrap();

    receipt.validate(&stock, &reservations).unwrap();
}

#[test]
fn receipt_replays_bound_stock_prefix_after_later_stock_mutation() {
    let mut stock = stock();
    let mut reservations = reservations(&stock);
    let receipt = perform_handoff(&stock, &mut reservations);

    // This intentionally makes the *current* reservation overlay impossible: only
    // 20 stock units remain while 30 are reserved. The historical receipt must
    // still prove the earlier valid handoff from the retained stock-history prefix.
    stock
        .deplete(
            lot("steel-a"),
            80,
            StockDepletionCause::Loss {
                incident_id: cause("later-catastrophic-loss"),
            },
        )
        .unwrap();
    assert!(reservations.validate_against_stock(&stock).is_err());

    receipt.validate(&stock, &reservations).unwrap();
}
