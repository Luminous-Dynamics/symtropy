// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! External/public-surface adversarial corpus for ECON-04B.

use symtropy_core::prelude::*;

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
    let mut stock = StockLedger::new();
    stock
        .establish(
            StockLot::new(
                lot_id("steel-parent"),
                CommoditySpecId::new("steel").unwrap(),
                quantity,
                actor("owner"),
                actor("warehouse"),
                LocationId::new("yard").unwrap(),
            )
            .unwrap(),
            StockOrigin::QualifiedInitial {
                evidence_id: cause("qualified-stock"),
            },
        )
        .unwrap();
    stock
}

#[test]
fn public_batch_reservation_is_atomic() {
    let stock = stock(100);
    let r1 = StockReservation::new(
        &stock,
        reservation_id("r1"),
        lot_id("steel-parent"),
        40,
        actor("owner"),
        actor("buyer-a"),
        cause("order-a"),
        cause("owner-auth-a"),
    )
    .unwrap();
    let r2 = StockReservation::new(
        &stock,
        reservation_id("r2"),
        lot_id("steel-parent"),
        60,
        actor("owner"),
        actor("buyer-b"),
        cause("order-b"),
        cause("owner-auth-b"),
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
        reservations.available_quantity(&stock, &lot_id("steel-parent")),
        Ok(0)
    );
    assert_eq!(stock.lot(&lot_id("steel-parent")).unwrap().quantity(), 100);
}

#[test]
fn wrong_owner_cannot_prepare_reservation() {
    let stock = stock(100);
    let error = StockReservation::new(
        &stock,
        reservation_id("r1"),
        lot_id("steel-parent"),
        10,
        actor("not-owner"),
        actor("buyer"),
        cause("order"),
        cause("fake-auth"),
    )
    .unwrap_err();

    assert!(matches!(error, ReservationError::OwnerMismatch { .. }));
}

#[test]
fn wrong_holder_cannot_release_claim() {
    let stock = stock(100);
    let mut reservations = StockReservationLedger::new();
    reservations
        .reserve(
            &stock,
            reservation_id("r1"),
            lot_id("steel-parent"),
            20,
            actor("owner"),
            actor("buyer"),
            cause("order"),
            cause("owner-auth"),
        )
        .unwrap();
    let before = reservations.clone();

    assert!(matches!(
        reservations.release(
            &stock,
            reservation_id("r1"),
            actor("other-holder"),
            cause("unauthorized-release"),
        ),
        Err(ReservationError::HolderMismatch { .. })
    ));
    assert_eq!(reservations, before);
}

#[test]
fn harmless_split_does_not_migrate_parent_reservation() {
    let mut stock = stock(100);
    let mut reservations = StockReservationLedger::new();
    reservations
        .reserve(
            &stock,
            reservation_id("r1"),
            lot_id("steel-parent"),
            60,
            actor("owner"),
            actor("buyer"),
            cause("order"),
            cause("owner-auth"),
        )
        .unwrap();

    stock
        .split(
            lot_id("steel-parent"),
            lot_id("steel-child"),
            30,
            cause("warehouse-split"),
        )
        .unwrap();

    reservations.validate_against_stock(&stock).unwrap();
    assert_eq!(
        reservations.reserved_quantity(&stock, &lot_id("steel-parent")),
        Ok(60)
    );
    assert_eq!(
        reservations.reserved_quantity(&stock, &lot_id("steel-child")),
        Ok(0)
    );
    assert_eq!(
        reservations.available_quantity(&stock, &lot_id("steel-parent")),
        Ok(10)
    );
    assert_eq!(
        reservations.available_quantity(&stock, &lot_id("steel-child")),
        Ok(30)
    );
}

#[test]
fn split_that_strands_reserved_quantity_fails_closed() {
    let mut stock = stock(100);
    let mut reservations = StockReservationLedger::new();
    reservations
        .reserve(
            &stock,
            reservation_id("r1"),
            lot_id("steel-parent"),
            60,
            actor("owner"),
            actor("buyer"),
            cause("order"),
            cause("owner-auth"),
        )
        .unwrap();

    stock
        .split(
            lot_id("steel-parent"),
            lot_id("steel-child"),
            50,
            cause("warehouse-split"),
        )
        .unwrap();

    assert_eq!(
        reservations.validate_against_stock(&stock),
        Err(ReservationError::OverReserved {
            lot_id: lot_id("steel-parent"),
            reserved: 60,
            stock_quantity: 50,
        })
    );
}

#[test]
fn full_release_repairs_claim_after_parent_lot_vanishes() {
    let mut stock = stock(20);
    let mut reservations = StockReservationLedger::new();
    reservations
        .reserve(
            &stock,
            reservation_id("r1"),
            lot_id("steel-parent"),
            20,
            actor("owner"),
            actor("buyer"),
            cause("order"),
            cause("owner-auth"),
        )
        .unwrap();

    stock
        .deplete(
            lot_id("steel-parent"),
            20,
            StockDepletionCause::Loss {
                incident_id: cause("warehouse-fire"),
            },
        )
        .unwrap();
    assert!(matches!(
        reservations.validate_against_stock(&stock),
        Err(ReservationError::UnknownLot { .. })
    ));

    reservations
        .release(
            &stock,
            reservation_id("r1"),
            actor("buyer"),
            cause("claim-cancelled-after-loss"),
        )
        .unwrap();
    reservations.validate_against_stock(&stock).unwrap();
    assert!(reservations.active_reservations().next().is_none());
}
