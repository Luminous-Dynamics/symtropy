//! Independent per-tick reference for the optimized event-bucket ledger.
//!
//! Expected queue state is calculated by a direct integer-tick simulation.
//! It does not call the production event resolver, grouping, or area helpers.

use maritime_queue_ledger::{
    CargoInventoryLedger, CargoMovementKind, CargoMovementObservation, CargoUnit, EventClass,
    EventHeader, OpeningCargoInventory, OpeningVesselQueue, RightsStatus, SourceEvidence,
    TickWindow, VesselEventKind, VesselEventObservation, VesselQueueLedger,
    audit_cargo_inventory, audit_vessel_queue,
};

fn source(id: &str) -> SourceEvidence {
    SourceEvidence {
        evidence_id: format!("evidence-{id}"),
        source_id: format!("source-{id}"),
        source_revision: "r1".to_owned(),
        raw_sha256: "b".repeat(64),
        rights_reference: format!("rights-{id}"),
        rights_status: RightsStatus::ReviewedAllowed,
        claim_ceiling: "independent synthetic oracle".to_owned(),
    }
}

fn observation(
    id: &str,
    call: &str,
    kind: VesselEventKind,
    tick: u64,
) -> VesselEventObservation {
    VesselEventObservation {
        header: EventHeader {
            event_id: id.to_owned(),
            entity_id: call.to_owned(),
            event_class: EventClass::Actual,
            event_tick: tick,
            observed_tick: tick,
            supersedes_event_id: None,
            source: source(id),
        },
        terminal_id: "terminal".to_owned(),
        kind,
    }
}

fn workload(
    opening: u64,
    arrival_a: u64,
    wait_a: u64,
    arrival_b: u64,
    wait_b: u64,
) -> VesselQueueLedger {
    VesselQueueLedger {
        ledger_id: "oracle-queue".to_owned(),
        terminal_id: "terminal".to_owned(),
        time_basis_id: "ticks-v1".to_owned(),
        window: TickWindow {
            start_tick: 0,
            end_tick: 12,
        },
        opening: OpeningVesselQueue {
            terminal_id: "terminal".to_owned(),
            queue_count: opening,
            measured_tick: 0,
            observed_tick: 0,
            source: evidence("opening-stock"),
        },
        observations: vec![
            observation(
                "arrival-a",
                "call-a",
                VesselEventKind::EligibleArrival,
                arrival_a,
            ),
            observation(
                "exit-a",
                "call-a",
                VesselEventKind::BerthEntry,
                arrival_a + wait_a,
            ),
            observation(
                "arrival-b",
                "call-b",
                VesselEventKind::EligibleArrival,
                arrival_b,
            ),
            observation(
                "exit-b",
                "call-b",
                VesselEventKind::Diversion,
                arrival_b + wait_b,
            ),
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReferenceReport {
    closing: u64,
    peak: u64,
    arrivals: u128,
    exits: u128,
    area: u128,
}

fn direct_per_tick_reference(ledger: &VesselQueueLedger) -> ReferenceReport {
    let mut queue = ledger.opening.queue_count;
    let mut peak = queue;
    let mut arrivals = 0_u128;
    let mut exits = 0_u128;
    let mut area = 0_u128;

    for tick in ledger.window.start_tick..ledger.window.end_tick {
        for event in &ledger.observations {
            if event.header.event_class != EventClass::Actual
                || event.header.event_tick != tick
            {
                continue;
            }
            if event.kind == VesselEventKind::EligibleArrival {
                queue += 1;
                arrivals += 1;
            }
        }
        for event in &ledger.observations {
            if event.header.event_class == EventClass::Actual
                && event.header.event_tick == tick
                && event.kind != VesselEventKind::EligibleArrival
            {
                queue -= 1;
                exits += 1;
            }
        }
        peak = peak.max(queue);
        area += u128::from(queue);
    }

    ReferenceReport {
        closing: queue,
        peak,
        arrivals,
        exits,
        area,
    }
}

#[test]
fn event_bucket_ledger_matches_direct_tick_simulation_over_768_frozen_cases() {
    let mut comparisons = 0;
    for opening in 0..=2 {
        for arrival_a in 0..=3 {
            for wait_a in 0..=3 {
                for arrival_b in 0..=3 {
                    for wait_b in 0..=3 {
                        let input = workload(
                            opening,
                            arrival_a,
                            wait_a,
                            arrival_b,
                            wait_b,
                        );
                        let expected = direct_per_tick_reference(&input);
                        let actual = audit_vessel_queue(&input, 12)
                            .expect("all generated call event pairs are ordered");

                        assert_eq!(actual.closing_queue, expected.closing);
                        assert_eq!(actual.peak_queue, expected.peak);
                        assert_eq!(actual.arrivals, expected.arrivals);
                        assert_eq!(actual.exits, expected.exits);
                        assert_eq!(actual.queue_area_vessel_ticks, expected.area);
                        comparisons += 1;
                    }
                }
            }
        }
    }
    assert_eq!(comparisons, 768);
}


fn cargo_observation(
    event_id: &str,
    movement_id: &str,
    kind: CargoMovementKind,
    quantity: u64,
    tick: u64,
) -> CargoMovementObservation {
    CargoMovementObservation {
        header: EventHeader {
            event_id: event_id.to_owned(),
            entity_id: movement_id.to_owned(),
            event_class: EventClass::Actual,
            event_tick: tick,
            observed_tick: tick,
            supersedes_event_id: None,
            source: source(event_id),
        },
        terminal_id: "terminal".to_owned(),
        movement_id: movement_id.to_owned(),
        kind,
        quantity,
        unit: CargoUnit::Teu,
    }
}

fn cargo_workload(
    opening: u64,
    inbound_tick: u64,
    inbound_quantity: u64,
    outbound_delay: u64,
    outbound_quantity: u64,
) -> CargoInventoryLedger {
    CargoInventoryLedger {
        ledger_id: "oracle-cargo-stock".to_owned(),
        terminal_id: "terminal".to_owned(),
        inventory_scope_id: "import-laden".to_owned(),
        time_basis_id: "ticks-v1".to_owned(),
        window: TickWindow {
            start_tick: 0,
            end_tick: 12,
        },
        unit: CargoUnit::Teu,
        opening: OpeningCargoInventory {
            terminal_id: "terminal".to_owned(),
            inventory_scope_id: "import-laden".to_owned(),
            unit: CargoUnit::Teu,
            quantity: opening,
            measured_tick: 0,
            observed_tick: 0,
            source: source("opening-cargo-stock"),
        },
        observations: vec![
            cargo_observation(
                "discharge",
                "movement-in",
                CargoMovementKind::VesselDischarge,
                inbound_quantity,
                inbound_tick,
            ),
            cargo_observation(
                "gate-out",
                "movement-out",
                CargoMovementKind::GateOut,
                outbound_quantity,
                inbound_tick + outbound_delay,
            ),
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CargoReferenceReport {
    closing: u64,
    peak: u64,
    inbound: u128,
    outbound: u128,
    area: u128,
}

/// Direct per-tick stock arithmetic, separately coded from the production
/// event-bucket implementation. Inflows and outflows at the same tick are
/// treated as one simultaneous accounting batch, with net stock measured
/// after the batch.
fn direct_cargo_tick_reference(ledger: &CargoInventoryLedger) -> CargoReferenceReport {
    let mut stock = ledger.opening.quantity;
    let mut peak = stock;
    let mut inbound = 0_u128;
    let mut outbound = 0_u128;
    let mut area = 0_u128;

    for tick in ledger.window.start_tick..ledger.window.end_tick {
        for event in &ledger.observations {
            if event.header.event_class == EventClass::Actual
                && event.header.event_tick == tick
                && matches!(
                    event.kind,
                    CargoMovementKind::VesselDischarge
                        | CargoMovementKind::GateIn
                        | CargoMovementKind::RailIn
                        | CargoMovementKind::OtherDeclaredInflow
                )
            {
                stock += event.quantity;
                inbound += u128::from(event.quantity);
            }
        }
        for event in &ledger.observations {
            if event.header.event_class == EventClass::Actual
                && event.header.event_tick == tick
                && matches!(
                    event.kind,
                    CargoMovementKind::VesselLoad
                        | CargoMovementKind::GateOut
                        | CargoMovementKind::RailOut
                        | CargoMovementKind::OtherDeclaredOutflow
                )
            {
                stock -= event.quantity;
                outbound += u128::from(event.quantity);
            }
        }
        peak = peak.max(stock);
        area += u128::from(stock);
    }

    CargoReferenceReport {
        closing: stock,
        peak,
        inbound,
        outbound,
        area,
    }
}

#[test]
fn cargo_inventory_bucket_ledger_matches_direct_tick_reference_over_432_cases() {
    let mut comparisons = 0;
    for opening in 0..=2 {
        for inbound_tick in 0..=3 {
            for inbound_quantity in 1..=3 {
                for outbound_delay in 0..=3 {
                    for outbound_quantity in 1..=(opening + inbound_quantity) {
                        let input = cargo_workload(
                            opening,
                            inbound_tick,
                            inbound_quantity,
                            outbound_delay,
                            outbound_quantity,
                        );
                        let expected = direct_cargo_tick_reference(&input);
                        let actual = audit_cargo_inventory(&input, 12)
                            .expect("generated cargo movements conserve non-negative stock");

                        assert_eq!(actual.closing_inventory, expected.closing);
                        assert_eq!(actual.peak_inventory, expected.peak);
                        assert_eq!(actual.inbound_quantity, expected.inbound);
                        assert_eq!(actual.outbound_quantity, expected.outbound);
                        assert_eq!(actual.inventory_area_unit_ticks, expected.area);
                        comparisons += 1;
                    }
                }
            }
        }
    }
    assert_eq!(comparisons, 432);
}
