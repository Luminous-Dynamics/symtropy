// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! External/public-surface adversarial corpus for ECON-05B.

use symtropy_core::prelude::*;

fn actor(value: &str) -> ActorId {
    ActorId::new(value).unwrap()
}

fn asset(value: &str) -> AssetId {
    AssetId::new(value).unwrap()
}

fn cause(value: &str) -> CausalId {
    CausalId::new(value).unwrap()
}

fn location(value: &str) -> LocationId {
    LocationId::new(value).unwrap()
}

fn lot(value: &str) -> LotId {
    LotId::new(value).unwrap()
}

fn reservation(value: &str) -> StockReservationId {
    StockReservationId::new(value).unwrap()
}

fn stock(quantity: u64) -> StockLedger {
    let mut stock = StockLedger::new();
    stock
        .establish(
            StockLot::new(
                lot("steel-bulk"),
                CommoditySpecId::new("steel").unwrap(),
                quantity,
                actor("owner"),
                actor("warehouse"),
                location("yard"),
            )
            .unwrap(),
            StockOrigin::QualifiedInitial {
                evidence_id: cause("qualified-steel"),
            },
        )
        .unwrap();
    stock
}

fn reservations(stock: &StockLedger, purchase_quantity: u64) -> StockReservationLedger {
    let mut reservations = StockReservationLedger::new();
    reservations
        .reserve(
            stock,
            reservation("purchase"),
            lot("steel-bulk"),
            purchase_quantity,
            actor("owner"),
            actor("buyer"),
            cause("purchase-purpose"),
            cause("purchase-authorization"),
        )
        .unwrap();
    reservations
}

fn add_unrelated(stock: &StockLedger, reservations: &mut StockReservationLedger) {
    reservations
        .reserve(
            stock,
            reservation("factory-claim"),
            lot("steel-bulk"),
            10,
            actor("owner"),
            actor("factory"),
            cause("factory-purpose"),
            cause("factory-authorization"),
        )
        .unwrap();
}

fn isolate(
    stock: &mut StockLedger,
    reservations: &mut StockReservationLedger,
) -> ReservedLotIsolationReceipt {
    isolate_reserved_lot(
        stock,
        reservations,
        reservation("purchase"),
        actor("buyer"),
        lot("steel-shipment"),
        reservation("purchase-isolated"),
        cause("isolate-purchase"),
    )
    .unwrap()
}

#[test]
fn partial_reservation_isolated_with_physical_and_economic_conservation() {
    let mut stock = stock(100);
    let mut reservations = reservations(&stock, 20);
    add_unrelated(&stock, &mut reservations);
    let unrelated_before = reservations
        .active_reservation(&reservation("factory-claim"))
        .unwrap()
        .clone();

    assert_eq!(reservations.reserved_quantity(&stock, &lot("steel-bulk")).unwrap(), 30);
    assert_eq!(reservations.available_quantity(&stock, &lot("steel-bulk")).unwrap(), 70);

    let receipt = isolate(&mut stock, &mut reservations);

    let parent = stock.lot(&lot("steel-bulk")).unwrap();
    let child = stock.lot(&lot("steel-shipment")).unwrap();
    assert_eq!(parent.quantity(), 80);
    assert_eq!(child.quantity(), 20);
    assert_eq!(child.commodity_spec_id(), parent.commodity_spec_id());
    assert_eq!(child.owner_id(), parent.owner_id());
    assert_eq!(child.custodian_id(), parent.custodian_id());
    assert_eq!(child.location_id(), parent.location_id());

    assert!(reservations
        .active_reservation(&reservation("purchase"))
        .is_none());
    let successor = reservations
        .active_reservation(&reservation("purchase-isolated"))
        .unwrap();
    assert_eq!(successor.lot_id(), &lot("steel-shipment"));
    assert_eq!(successor.quantity(), 20);
    assert_eq!(successor.holder_id(), &actor("buyer"));
    assert_eq!(successor.purpose_id(), &cause("purchase-purpose"));
    assert_eq!(
        successor.authorization_id(),
        &cause("purchase-authorization")
    );
    assert_eq!(
        reservations.active_reservation(&reservation("factory-claim")),
        Some(&unrelated_before)
    );

    let reserved_after = reservations
        .reserved_quantity(&stock, &lot("steel-bulk"))
        .unwrap()
        + reservations
            .reserved_quantity(&stock, &lot("steel-shipment"))
            .unwrap();
    let available_after = reservations
        .available_quantity(&stock, &lot("steel-bulk"))
        .unwrap()
        + reservations
            .available_quantity(&stock, &lot("steel-shipment"))
            .unwrap();
    assert_eq!(reserved_after, 30);
    assert_eq!(available_after, 70);
    assert_eq!(receipt.conserved_reserved_quantity(), 30);
    assert_eq!(receipt.conserved_available_quantity(), 70);
    assert_eq!(receipt.stock_event_count_after(), receipt.stock_event_count_before() + 1);
    assert_eq!(
        receipt.reservation_event_count_after(),
        receipt.reservation_event_count_before() + 2
    );
    receipt.validate(&stock, &reservations).unwrap();
}

#[test]
fn wrong_holder_fails_without_mutating_either_ledger() {
    let mut stock = stock(100);
    let mut reservations = reservations(&stock, 20);
    let stock_before = stock.clone();
    let reservations_before = reservations.clone();

    assert_eq!(
        isolate_reserved_lot(
            &mut stock,
            &mut reservations,
            reservation("purchase"),
            actor("not-buyer"),
            lot("steel-shipment"),
            reservation("purchase-isolated"),
            cause("bad-holder-isolation"),
        ),
        Err(ReservedLotIsolationError::SourceHolderMismatch {
            expected: actor("not-buyer"),
            actual: actor("buyer"),
        })
    );
    assert_eq!(stock, stock_before);
    assert_eq!(reservations, reservations_before);
}

#[test]
fn duplicate_child_lot_fails_without_mutating_either_ledger() {
    let mut stock = stock(100);
    stock
        .establish(
            StockLot::new(
                lot("steel-shipment"),
                CommoditySpecId::new("steel").unwrap(),
                1,
                actor("owner"),
                actor("warehouse"),
                location("yard"),
            )
            .unwrap(),
            StockOrigin::QualifiedInitial {
                evidence_id: cause("existing-child"),
            },
        )
        .unwrap();
    let mut reservations = reservations(&stock, 20);
    let stock_before = stock.clone();
    let reservations_before = reservations.clone();

    let error = isolate_reserved_lot(
        &mut stock,
        &mut reservations,
        reservation("purchase"),
        actor("buyer"),
        lot("steel-shipment"),
        reservation("purchase-isolated"),
        cause("duplicate-child-isolation"),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ReservedLotIsolationError::Economic(EconomicError::DuplicateLot { .. })
    ));
    assert_eq!(stock, stock_before);
    assert_eq!(reservations, reservations_before);
}

#[test]
fn historically_seen_successor_reservation_id_fails_without_commit() {
    let mut stock = stock(100);
    let mut reservations = reservations(&stock, 20);
    reservations
        .reserve(
            &stock,
            reservation("used-successor"),
            lot("steel-bulk"),
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
            reservation("used-successor"),
            actor("temporary"),
            cause("temporary-release"),
        )
        .unwrap();
    let stock_before = stock.clone();
    let reservations_before = reservations.clone();

    let error = isolate_reserved_lot(
        &mut stock,
        &mut reservations,
        reservation("purchase"),
        actor("buyer"),
        lot("steel-shipment"),
        reservation("used-successor"),
        cause("used-id-isolation"),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ReservedLotIsolationError::Reservation(ReservationError::DuplicateReservation { .. })
    ));
    assert_eq!(stock, stock_before);
    assert_eq!(reservations, reservations_before);
}

#[test]
fn already_whole_lot_reservation_is_rejected_as_unnecessary() {
    let mut stock = stock(20);
    let mut reservations = reservations(&stock, 20);
    let stock_before = stock.clone();
    let reservations_before = reservations.clone();

    assert_eq!(
        isolate_reserved_lot(
            &mut stock,
            &mut reservations,
            reservation("purchase"),
            actor("buyer"),
            lot("steel-shipment"),
            reservation("purchase-isolated"),
            cause("unnecessary-isolation"),
        ),
        Err(ReservedLotIsolationError::ReservationAlreadyWholeLot {
            lot_quantity: 20,
            reservation_quantity: 20,
        })
    );
    assert_eq!(stock, stock_before);
    assert_eq!(reservations, reservations_before);
}

#[test]
fn isolated_partial_reservation_flows_through_handoff_and_freight_load() {
    let mut stock = stock(100);
    let mut reservations = reservations(&stock, 20);
    let isolation = isolate(&mut stock, &mut reservations);

    let handoff = handoff_full_reservation(
        &stock,
        &mut reservations,
        reservation("purchase-isolated"),
        actor("buyer"),
        reservation("transport"),
        actor("carrier"),
        cause("transport-purpose"),
        cause("transport-authorization"),
        cause("purchase-to-transport"),
    )
    .unwrap();

    let load = load_full_transport_reservation(
        &mut stock,
        &reservations,
        &handoff,
        actor("warehouse"),
        location("yard"),
        CarrierCargoBinding::new(
            asset("truck-7"),
            location("truck-7:cargo-hold"),
            actor("carrier"),
            cause("fleet-authority:truck-7"),
        ),
        cause("load-steel-shipment"),
    )
    .unwrap();

    let loaded = stock.lot(&lot("steel-shipment")).unwrap();
    assert_eq!(loaded.quantity(), 20);
    assert_eq!(loaded.custodian_id(), &actor("carrier"));
    assert_eq!(loaded.location_id(), &location("truck-7:cargo-hold"));
    assert_eq!(reservations.available_quantity(&stock, &lot("steel-shipment")).unwrap(), 0);

    isolation.validate(&stock, &reservations).unwrap();
    handoff.validate(&stock, &reservations).unwrap();
    load.validate(&stock, &reservations).unwrap();
}

#[test]
fn historical_isolation_receipt_survives_later_handoff_load_release_and_loss() {
    let mut stock = stock(100);
    let mut reservations = reservations(&stock, 20);
    let isolation = isolate(&mut stock, &mut reservations);

    let handoff = handoff_full_reservation(
        &stock,
        &mut reservations,
        reservation("purchase-isolated"),
        actor("buyer"),
        reservation("transport"),
        actor("carrier"),
        cause("transport-purpose"),
        cause("transport-authorization"),
        cause("later-handoff"),
    )
    .unwrap();
    load_full_transport_reservation(
        &mut stock,
        &reservations,
        &handoff,
        actor("warehouse"),
        location("yard"),
        CarrierCargoBinding::new(
            asset("truck-9"),
            location("truck-9:cargo-hold"),
            actor("carrier"),
            cause("fleet-authority:truck-9"),
        ),
        cause("later-load"),
    )
    .unwrap();
    reservations
        .release(
            &stock,
            reservation("transport"),
            actor("carrier"),
            cause("later-delivery-release"),
        )
        .unwrap();
    stock
        .deplete(
            lot("steel-shipment"),
            20,
            StockDepletionCause::Loss {
                incident_id: cause("later-total-loss"),
            },
        )
        .unwrap();

    isolation.validate(&stock, &reservations).unwrap();
}
