//! Independent per-tick reference for the optimized event-bucket ledger.
//!
//! Expected queue state is calculated by a direct integer-tick simulation.
//! It does not call the production event resolver, grouping, or area helpers.

use maritime_queue_ledger::{
    EventClass, EventHeader, OpeningVesselQueue, RightsStatus, SourceEvidence, TickWindow,
    VesselEventKind, VesselEventObservation, VesselQueueLedger, audit_vessel_queue,
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
    let mut queue = ledger.opening_queue;
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
