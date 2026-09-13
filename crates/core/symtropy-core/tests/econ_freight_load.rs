// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! External/public-surface adversarial corpus for ECON-05A.

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

fn stock(quantity: u64, custodian: &str) -> StockLedger {
    let mut stock = StockLedger::new();
    stock
        .establish(
            StockLot::new(
                lot("steel-a"),
                CommoditySpecId::new("steel").unwrap(),
                quantity,
                actor("owner"),
                actor(custodian),
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

fn purchase_reservation(stock: &StockLedger, quantity: u64) -> StockReservationLedger {
    let mut reservations = StockReservationLedger::new();
    reservations
        .reserve(
            stock,
            reservation("purchase"),
            lot("steel-a"),
            quantity,
            actor("owner"),
            actor("buyer"),
            cause("purchase-purpose"),
            cause("purchase-authorization"),
        )
        .unwrap();
    reservations
}

fn transport_handoff(
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

fn carrier(custodian: &str) -> CarrierCargoBinding {
    CarrierCargoBinding::new(
        asset("truck-7"),
        location("truck-7:cargo-hold"),
        actor(custodian),
        cause("fleet-authority:truck-7"),
    )
}

#[test]
fn isolated_full_lot_load_conserves_stock_and_encumbrance() {
    let mut stock = stock(20, "warehouse");
    let mut reservations = purchase_reservation(&stock, 20);
    let handoff = transport_handoff(&stock, &mut reservations);

    let receipt = load_full_transport_reservation(
        &mut stock,
        &reservations,
        &handoff,
        actor("warehouse"),
        location("yard"),
        carrier("carrier"),
        cause("load-steel"),
    )
    .unwrap();

    let loaded = stock.lot(&lot("steel-a")).unwrap();
    assert_eq!(loaded.quantity(), 20);
    assert_eq!(loaded.commodity_spec_id(), &CommoditySpecId::new("steel").unwrap());
    assert_eq!(loaded.owner_id(), &actor("owner"));
    assert_eq!(loaded.custodian_id(), &actor("carrier"));
    assert_eq!(loaded.location_id(), &location("truck-7:cargo-hold"));
    assert_eq!(reservations.reserved_quantity(&stock, &lot("steel-a")).unwrap(), 20);
    assert_eq!(reservations.available_quantity(&stock, &lot("steel-a")).unwrap(), 0);
    assert_eq!(receipt.quantity(), 20);
    assert_eq!(receipt.custody_sequence(), Some(receipt.stock_event_count_before() + 1));
    assert_eq!(receipt.relocation_sequence(), receipt.stock_event_count_before() + 2);
    assert_eq!(receipt.stock_event_count_after(), receipt.relocation_sequence());
    receipt.validate(&stock, &reservations).unwrap();
}

#[test]
fn partial_reservation_over_larger_lot_fails_without_mutating_stock() {
    let mut stock = stock(100, "warehouse");
    let mut reservations = purchase_reservation(&stock, 20);
    let handoff = transport_handoff(&stock, &mut reservations);
    let before = stock.clone();

    assert_eq!(
        load_full_transport_reservation(
            &mut stock,
            &reservations,
            &handoff,
            actor("warehouse"),
            location("yard"),
            carrier("carrier"),
            cause("invalid-partial-load"),
        ),
        Err(FreightLoadError::LotNotTransportIsolated {
            lot_quantity: 100,
            reservation_quantity: 20,
        })
    );
    assert_eq!(stock, before);
}

#[test]
fn carrier_must_be_the_transport_reservation_holder() {
    let mut stock = stock(20, "warehouse");
    let mut reservations = purchase_reservation(&stock, 20);
    let handoff = transport_handoff(&stock, &mut reservations);
    let before = stock.clone();

    assert_eq!(
        load_full_transport_reservation(
            &mut stock,
            &reservations,
            &handoff,
            actor("warehouse"),
            location("yard"),
            carrier("different-carrier"),
            cause("unauthorized-load"),
        ),
        Err(FreightLoadError::CarrierHolderMismatch {
            reservation_holder: actor("carrier"),
            carrier_custodian: actor("different-carrier"),
        })
    );
    assert_eq!(stock, before);
}

#[test]
fn stale_source_location_fails_without_mutating_stock() {
    let mut stock = stock(20, "warehouse");
    let mut reservations = purchase_reservation(&stock, 20);
    let handoff = transport_handoff(&stock, &mut reservations);
    let before = stock.clone();

    assert_eq!(
        load_full_transport_reservation(
            &mut stock,
            &reservations,
            &handoff,
            actor("warehouse"),
            location("wrong-yard"),
            carrier("carrier"),
            cause("stale-location-load"),
        ),
        Err(FreightLoadError::SourceLocationMismatch {
            expected: location("wrong-yard"),
            actual: location("yard"),
        })
    );
    assert_eq!(stock, before);
}

#[test]
fn stale_source_custodian_fails_without_mutating_stock() {
    let mut stock = stock(20, "warehouse");
    let mut reservations = purchase_reservation(&stock, 20);
    let handoff = transport_handoff(&stock, &mut reservations);
    let before = stock.clone();

    assert_eq!(
        load_full_transport_reservation(
            &mut stock,
            &reservations,
            &handoff,
            actor("wrong-warehouse"),
            location("yard"),
            carrier("carrier"),
            cause("stale-custody-load"),
        ),
        Err(FreightLoadError::SourceCustodianMismatch {
            expected: actor("wrong-warehouse"),
            actual: actor("warehouse"),
        })
    );
    assert_eq!(stock, before);
}

#[test]
fn same_custodian_load_emits_only_relocation() {
    let mut stock = stock(20, "carrier");
    let mut reservations = purchase_reservation(&stock, 20);
    let handoff = transport_handoff(&stock, &mut reservations);
    let before_events = stock.entries().len();

    let receipt = load_full_transport_reservation(
        &mut stock,
        &reservations,
        &handoff,
        actor("carrier"),
        location("yard"),
        carrier("carrier"),
        cause("self-custodied-load"),
    )
    .unwrap();

    assert_eq!(receipt.custody_sequence(), None);
    assert_eq!(stock.entries().len(), before_events + 1);
    assert_eq!(receipt.relocation_sequence(), receipt.stock_event_count_before() + 1);
    assert_eq!(stock.lot(&lot("steel-a")).unwrap().custodian_id(), &actor("carrier"));
    assert_eq!(
        stock.lot(&lot("steel-a")).unwrap().location_id(),
        &location("truck-7:cargo-hold")
    );
    receipt.validate(&stock, &reservations).unwrap();
}

#[test]
fn stale_handoff_cannot_load_after_transport_reservation_is_released() {
    let mut stock = stock(20, "warehouse");
    let mut reservations = purchase_reservation(&stock, 20);
    let handoff = transport_handoff(&stock, &mut reservations);
    reservations
        .release(
            &stock,
            reservation("transport"),
            actor("carrier"),
            cause("transport-cancelled"),
        )
        .unwrap();
    let before = stock.clone();

    assert_eq!(
        load_full_transport_reservation(
            &mut stock,
            &reservations,
            &handoff,
            actor("warehouse"),
            location("yard"),
            carrier("carrier"),
            cause("stale-handoff-load"),
        ),
        Err(FreightLoadError::TransportReservationNotActive)
    );
    assert_eq!(stock, before);
}

#[test]
fn historical_receipt_survives_later_release_and_stock_loss() {
    let mut stock = stock(20, "warehouse");
    let mut reservations = purchase_reservation(&stock, 20);
    let handoff = transport_handoff(&stock, &mut reservations);
    let receipt = load_full_transport_reservation(
        &mut stock,
        &reservations,
        &handoff,
        actor("warehouse"),
        location("yard"),
        carrier("carrier"),
        cause("load-before-loss"),
    )
    .unwrap();

    reservations
        .release(
            &stock,
            reservation("transport"),
            actor("carrier"),
            cause("delivery-release"),
        )
        .unwrap();
    stock
        .deplete(
            lot("steel-a"),
            20,
            StockDepletionCause::Loss {
                incident_id: cause("later-total-loss"),
            },
        )
        .unwrap();

    assert!(stock.lot(&lot("steel-a")).is_none());
    receipt.validate(&stock, &reservations).unwrap();
}
