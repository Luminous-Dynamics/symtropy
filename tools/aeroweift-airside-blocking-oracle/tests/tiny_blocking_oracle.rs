use aeroweift_airside_blocking_oracle::{
    audit_capacities, audit_causality, audit_reservation_events, blocking_ticks,
    holding_energy_delta, occupancy_ticks, AuditError, Event, EventKind, Reservation,
    ResourceCapacity,
};

const GOLDEN_FIXTURE: &str = include_str!(
    "../../../fixtures/aeroweift-airside-blocking/tiny-blocking-cascade-v0.json"
);

const CAPACITIES: [ResourceCapacity; 5] = [
    ResourceCapacity {
        resource_id: "tlof-1",
        capacity: 1,
    },
    ResourceCapacity {
        resource_id: "stand-1",
        capacity: 1,
    },
    ResourceCapacity {
        resource_id: "taxi-link-1",
        capacity: 1,
    },
    ResourceCapacity {
        resource_id: "charger-1",
        capacity: 1,
    },
    ResourceCapacity {
        resource_id: "terminal-hold-1",
        capacity: 1,
    },
];

const RESERVATIONS: [Reservation; 10] = [
    Reservation {
        aircraft_id: "aircraft-b",
        resource_id: "stand-1",
        start_tick: 0,
        end_tick: 10,
    },
    Reservation {
        aircraft_id: "aircraft-b",
        resource_id: "charger-1",
        start_tick: 0,
        end_tick: 8,
    },
    Reservation {
        aircraft_id: "aircraft-a",
        resource_id: "tlof-1",
        start_tick: 2,
        end_tick: 10,
    },
    Reservation {
        aircraft_id: "aircraft-c",
        resource_id: "terminal-hold-1",
        start_tick: 6,
        end_tick: 10,
    },
    Reservation {
        aircraft_id: "aircraft-a",
        resource_id: "taxi-link-1",
        start_tick: 10,
        end_tick: 11,
    },
    Reservation {
        aircraft_id: "aircraft-a",
        resource_id: "stand-1",
        start_tick: 11,
        end_tick: 19,
    },
    Reservation {
        aircraft_id: "aircraft-c",
        resource_id: "tlof-1",
        start_tick: 10,
        end_tick: 19,
    },
    Reservation {
        aircraft_id: "aircraft-c",
        resource_id: "taxi-link-1",
        start_tick: 19,
        end_tick: 20,
    },
    Reservation {
        aircraft_id: "aircraft-c",
        resource_id: "stand-1",
        start_tick: 20,
        end_tick: 28,
    },
    Reservation {
        aircraft_id: "aircraft-c",
        resource_id: "charger-1",
        start_tick: 20,
        end_tick: 28,
    },
];

const EVENTS: [Event; 23] = [
    Event {
        event_id: "event-b-stand-acquire",
        aircraft_id: "aircraft-b",
        resource_id: "stand-1",
        kind: EventKind::Acquire,
        tick: 0,
        predecessor: None,
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-b-charge-acquire",
        aircraft_id: "aircraft-b",
        resource_id: "charger-1",
        kind: EventKind::Acquire,
        tick: 0,
        predecessor: None,
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-b-charge-release",
        aircraft_id: "aircraft-b",
        resource_id: "charger-1",
        kind: EventKind::Release,
        tick: 8,
        predecessor: Some("event-b-charge-acquire"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-a-tlof-acquire",
        aircraft_id: "aircraft-a",
        resource_id: "tlof-1",
        kind: EventKind::Acquire,
        tick: 2,
        predecessor: None,
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-a-service-complete",
        aircraft_id: "aircraft-a",
        resource_id: "tlof-1",
        kind: EventKind::ServiceComplete,
        tick: 5,
        predecessor: Some("event-a-tlof-acquire"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-a-downstream-blocked",
        aircraft_id: "aircraft-a",
        resource_id: "stand-1",
        kind: EventKind::DownstreamAcquireBlocked,
        tick: 5,
        predecessor: Some("event-a-service-complete"),
        blocker_aircraft_id: Some("aircraft-b"),
    },
    Event {
        event_id: "event-b-stand-release",
        aircraft_id: "aircraft-b",
        resource_id: "stand-1",
        kind: EventKind::Release,
        tick: 10,
        predecessor: Some("event-b-stand-acquire"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-a-taxi-acquire",
        aircraft_id: "aircraft-a",
        resource_id: "taxi-link-1",
        kind: EventKind::Acquire,
        tick: 10,
        predecessor: None,
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-a-tlof-release",
        aircraft_id: "aircraft-a",
        resource_id: "tlof-1",
        kind: EventKind::Release,
        tick: 10,
        predecessor: Some("event-a-taxi-acquire"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-a-taxi-release",
        aircraft_id: "aircraft-a",
        resource_id: "taxi-link-1",
        kind: EventKind::Release,
        tick: 11,
        predecessor: Some("event-a-taxi-acquire"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-a-stand-acquire",
        aircraft_id: "aircraft-a",
        resource_id: "stand-1",
        kind: EventKind::Acquire,
        tick: 11,
        predecessor: Some("event-a-taxi-release"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-hold-acquire",
        aircraft_id: "aircraft-c",
        resource_id: "terminal-hold-1",
        kind: EventKind::Acquire,
        tick: 6,
        predecessor: None,
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-hold-release",
        aircraft_id: "aircraft-c",
        resource_id: "terminal-hold-1",
        kind: EventKind::Release,
        tick: 10,
        predecessor: Some("event-c-hold-acquire"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-tlof-acquire",
        aircraft_id: "aircraft-c",
        resource_id: "tlof-1",
        kind: EventKind::Acquire,
        tick: 10,
        predecessor: Some("event-a-tlof-release"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-service-complete",
        aircraft_id: "aircraft-c",
        resource_id: "tlof-1",
        kind: EventKind::ServiceComplete,
        tick: 13,
        predecessor: Some("event-c-tlof-acquire"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-downstream-blocked",
        aircraft_id: "aircraft-c",
        resource_id: "stand-1",
        kind: EventKind::DownstreamAcquireBlocked,
        tick: 13,
        predecessor: Some("event-c-service-complete"),
        blocker_aircraft_id: Some("aircraft-a"),
    },
    Event {
        event_id: "event-a-stand-release",
        aircraft_id: "aircraft-a",
        resource_id: "stand-1",
        kind: EventKind::Release,
        tick: 19,
        predecessor: Some("event-a-stand-acquire"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-taxi-acquire",
        aircraft_id: "aircraft-c",
        resource_id: "taxi-link-1",
        kind: EventKind::Acquire,
        tick: 19,
        predecessor: None,
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-tlof-release",
        aircraft_id: "aircraft-c",
        resource_id: "tlof-1",
        kind: EventKind::Release,
        tick: 19,
        predecessor: Some("event-c-taxi-acquire"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-taxi-release",
        aircraft_id: "aircraft-c",
        resource_id: "taxi-link-1",
        kind: EventKind::Release,
        tick: 20,
        predecessor: Some("event-c-taxi-acquire"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-stand-acquire",
        aircraft_id: "aircraft-c",
        resource_id: "stand-1",
        kind: EventKind::Acquire,
        tick: 20,
        predecessor: Some("event-c-taxi-release"),
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-charge-acquire",
        aircraft_id: "aircraft-c",
        resource_id: "charger-1",
        kind: EventKind::Acquire,
        tick: 20,
        predecessor: None,
        blocker_aircraft_id: None,
    },
    Event {
        event_id: "event-c-charge-release",
        aircraft_id: "aircraft-c",
        resource_id: "charger-1",
        kind: EventKind::Release,
        tick: 28,
        predecessor: Some("event-c-charge-acquire"),
        blocker_aircraft_id: None,
    },
];


fn all_events() -> Vec<Event> {
    let mut events = EVENTS.to_vec();
    events.push(Event {
        event_id: "event-c-stand-release",
        aircraft_id: "aircraft-c",
        resource_id: "stand-1",
        kind: EventKind::Release,
        tick: 28,
        predecessor: Some("event-c-stand-acquire"),
        blocker_aircraft_id: None,
    });
    events
}

#[test]
fn frozen_json_fixture_identity_and_expected_metrics_are_bound() {
    assert!(GOLDEN_FIXTURE.contains("AW-AB-TINY-001"));
    assert!(GOLDEN_FIXTURE.contains("\"total_tlof_blocking_ticks\": 11"));
    assert!(GOLDEN_FIXTURE.contains("\"next_mission_eligible_tick\": 28"));
    assert!(GOLDEN_FIXTURE.contains("\"must_not_be_admitted_as_finite_buffer_plan\": true"));
}

#[test]
fn finite_resource_claims_fit_and_peaks_are_exact() {
    let peaks = audit_capacities(&RESERVATIONS, &CAPACITIES)
        .expect("the golden finite-resource trace must fit");
    assert_eq!(
        peaks,
        vec![
            aeroweift_airside_blocking_oracle::ResourcePeak {
                resource_id: "charger-1",
                maximum_occupancy: 1,
            },
            aeroweift_airside_blocking_oracle::ResourcePeak {
                resource_id: "stand-1",
                maximum_occupancy: 1,
            },
            aeroweift_airside_blocking_oracle::ResourcePeak {
                resource_id: "taxi-link-1",
                maximum_occupancy: 1,
            },
            aeroweift_airside_blocking_oracle::ResourcePeak {
                resource_id: "terminal-hold-1",
                maximum_occupancy: 1,
            },
            aeroweift_airside_blocking_oracle::ResourcePeak {
                resource_id: "tlof-1",
                maximum_occupancy: 1,
            },
        ]
    );
}

#[test]
fn blocking_holding_energy_and_arrival_delay_match_golden_values() {
    audit_causality(&EVENTS).expect("all causal predecessors must be present and ordered");

    assert_eq!(
        blocking_ticks(&EVENTS, "event-a-service-complete", "event-a-tlof-release"),
        Some(5)
    );
    assert_eq!(
        blocking_ticks(&EVENTS, "event-c-service-complete", "event-c-tlof-release"),
        Some(6)
    );
    assert_eq!(occupancy_ticks(&RESERVATIONS, "aircraft-c", "terminal-hold-1"), Some(4));
    assert_eq!(holding_energy_delta(4, 1), Some(-4));
    assert_eq!(10_u16.checked_sub(6), Some(4));
    assert_eq!(occupancy_ticks(&RESERVATIONS, "aircraft-c", "stand-1"), Some(8));
}

#[test]
fn every_reservation_has_matching_resource_acquire_and_release_events() {
    // The terminal stand release completes the one-to-one event manifest.
    let events = all_events();
    audit_causality(&events).expect("terminal release must have a valid predecessor");
    audit_reservation_events(&RESERVATIONS, &events)
        .expect("each reservation must have exactly one matching acquisition and release");
}

#[test]
fn capacity_one_overlap_is_rejected() {
    let mut reservations = RESERVATIONS.to_vec();
    reservations.push(Reservation {
        aircraft_id: "aircraft-d",
        resource_id: "tlof-1",
        start_tick: 9,
        end_tick: 11,
    });
    assert_eq!(
        audit_capacities(&reservations, &CAPACITIES),
        Err(AuditError::CapacityExceeded)
    );
}

#[test]
fn half_open_adjacent_claims_are_allowed() {
    let adjacent = [
        Reservation {
            aircraft_id: "aircraft-a",
            resource_id: "tlof-1",
            start_tick: 0,
            end_tick: 1,
        },
        Reservation {
            aircraft_id: "aircraft-b",
            resource_id: "tlof-1",
            start_tick: 1,
            end_tick: 2,
        },
    ];
    assert!(audit_capacities(
        &adjacent,
        &[ResourceCapacity {
            resource_id: "tlof-1",
            capacity: 1,
        }]
    )
    .is_ok());
}

#[test]
fn unknown_resources_invalid_intervals_and_duplicate_capacities_fail_closed() {
    let unknown = [Reservation {
        aircraft_id: "aircraft-a",
        resource_id: "undeclared",
        start_tick: 0,
        end_tick: 1,
    }];
    assert_eq!(
        audit_capacities(&unknown, &CAPACITIES),
        Err(AuditError::UnknownResource)
    );

    let invalid = [Reservation {
        aircraft_id: "aircraft-a",
        resource_id: "tlof-1",
        start_tick: 2,
        end_tick: 2,
    }];
    assert_eq!(
        audit_capacities(&invalid, &CAPACITIES),
        Err(AuditError::InvalidInterval)
    );

    assert_eq!(
        audit_capacities(
            &[],
            &[
                ResourceCapacity {
                    resource_id: "tlof-1",
                    capacity: 1,
                },
                ResourceCapacity {
                    resource_id: "tlof-1",
                    capacity: 2,
                },
            ]
        ),
        Err(AuditError::DuplicateResourceCapacity)
    );
}


#[test]
fn duplicate_resource_reservations_are_rejected_even_with_extra_capacity() {
    let duplicate = [
        Reservation {
            aircraft_id: "aircraft-x",
            resource_id: "tlof-1",
            start_tick: 0,
            end_tick: 1,
        },
        Reservation {
            aircraft_id: "aircraft-x",
            resource_id: "tlof-1",
            start_tick: 0,
            end_tick: 1,
        },
    ];
    let events = [
        Event {
            event_id: "acquire-x",
            aircraft_id: "aircraft-x",
            resource_id: "tlof-1",
            kind: EventKind::Acquire,
            tick: 0,
            predecessor: None,
            blocker_aircraft_id: None,
        },
        Event {
            event_id: "release-x",
            aircraft_id: "aircraft-x",
            resource_id: "tlof-1",
            kind: EventKind::Release,
            tick: 1,
            predecessor: Some("acquire-x"),
            blocker_aircraft_id: None,
        },
    ];
    assert!(audit_capacities(
        &duplicate,
        &[ResourceCapacity {
            resource_id: "tlof-1",
            capacity: 2,
        }]
    ).is_ok());
    assert_eq!(
        audit_reservation_events(&duplicate, &events),
        Err(AuditError::DuplicateReservation)
    );
}

#[test]
fn event_and_reservation_input_order_does_not_change_semantic_audit() {
    let events = all_events();
    let baseline = (
        audit_capacities(&RESERVATIONS, &CAPACITIES),
        audit_reservation_events(&RESERVATIONS, &events),
        audit_causality(&events),
    );

    let mut reservations_permuted = RESERVATIONS.to_vec();
    reservations_permuted.reverse();
    let mut events_permuted = events;
    events_permuted.reverse();

    let permuted = (
        audit_capacities(&reservations_permuted, &CAPACITIES),
        audit_reservation_events(&reservations_permuted, &events_permuted),
        audit_causality(&events_permuted),
    );
    assert_eq!(baseline, permuted);
}

#[test]
fn missing_blocker_or_future_causal_predecessor_is_rejected() {
    let mut events = EVENTS.to_vec();
    let blocked = events
        .iter_mut()
        .find(|event| event.event_id == "event-a-downstream-blocked")
        .expect("fixture event must exist");
    blocked.blocker_aircraft_id = None;
    assert_eq!(
        audit_reservation_events(&RESERVATIONS, &events),
        Err(AuditError::MissingBlockingWitness)
    );

    let invalid_cause = [
        Event {
            event_id: "cause",
            aircraft_id: "aircraft-a",
            resource_id: "tlof-1",
            kind: EventKind::ServiceComplete,
            tick: 10,
            predecessor: None,
            blocker_aircraft_id: None,
        },
        Event {
            event_id: "effect",
            aircraft_id: "aircraft-a",
            resource_id: "tlof-1",
            kind: EventKind::Release,
            tick: 5,
            predecessor: Some("cause"),
            blocker_aircraft_id: None,
        },
    ];
    assert_eq!(
        audit_causality(&invalid_cause),
        Err(AuditError::CausalityViolation)
    );

    let same_tick_cycle = [
        Event {
            event_id: "cycle-a",
            aircraft_id: "aircraft-a",
            resource_id: "tlof-1",
            kind: EventKind::ServiceComplete,
            tick: 5,
            predecessor: Some("cycle-b"),
            blocker_aircraft_id: None,
        },
        Event {
            event_id: "cycle-b",
            aircraft_id: "aircraft-a",
            resource_id: "tlof-1",
            kind: EventKind::Release,
            tick: 5,
            predecessor: Some("cycle-a"),
            blocker_aircraft_id: None,
        },
    ];
    assert_eq!(
        audit_causality(&same_tick_cycle),
        Err(AuditError::CausalityViolation)
    );
}
