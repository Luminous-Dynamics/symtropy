use maritime_queue_ledger::{
    CargoInventoryLedger, CargoMovementKind, CargoMovementObservation, CargoUnit,
    EventClass, EventHeader, LedgerError, RightsStatus, SourceEvidence, TickWindow,
    VesselEventKind, VesselEventObservation, VesselQueueLedger, audit_cargo_inventory,
    audit_vessel_queue,
};

fn source(id: &str, rights_status: RightsStatus) -> SourceEvidence {
    SourceEvidence {
        evidence_id: id.to_owned(),
        source_id: format!("source-{id}"),
        source_revision: "revision-1".to_owned(),
        raw_sha256: "a".repeat(64),
        rights_reference: format!("rights-review-{id}"),
        rights_status,
        claim_ceiling: "synthetic ledger fixture".to_owned(),
    }
}

fn header(
    event_id: &str,
    entity_id: &str,
    class: EventClass,
    event_tick: u64,
    observed_tick: u64,
    supersedes_event_id: Option<&str>,
) -> EventHeader {
    EventHeader {
        event_id: event_id.to_owned(),
        entity_id: entity_id.to_owned(),
        event_class: class,
        event_tick,
        observed_tick,
        supersedes_event_id: supersedes_event_id.map(str::to_owned),
        source: source(&format!("evidence-{event_id}"), RightsStatus::ReviewedAllowed),
    }
}

fn vessel(
    event_id: &str,
    call_id: &str,
    kind: VesselEventKind,
    class: EventClass,
    event_tick: u64,
    observed_tick: u64,
    supersedes: Option<&str>,
) -> VesselEventObservation {
    VesselEventObservation {
        header: header(
            event_id,
            call_id,
            class,
            event_tick,
            observed_tick,
            supersedes,
        ),
        terminal_id: "dgt".to_owned(),
        kind,
    }
}

fn vessel_ledger(
    opening_queue: u64,
    observations: Vec<VesselEventObservation>,
) -> VesselQueueLedger {
    VesselQueueLedger {
        ledger_id: "dgt-vessel-queue".to_owned(),
        terminal_id: "dgt".to_owned(),
        time_basis_id: "unix-minute-v1".to_owned(),
        window: TickWindow {
            start_tick: 0,
            end_tick: 10,
        },
        opening_queue,
        observations,
    }
}

fn cargo(
    event_id: &str,
    movement_id: &str,
    kind: CargoMovementKind,
    quantity: u64,
    unit: CargoUnit,
    event_tick: u64,
    observed_tick: u64,
    supersedes: Option<&str>,
) -> CargoMovementObservation {
    CargoMovementObservation {
        header: header(
            event_id,
            movement_id,
            EventClass::Actual,
            event_tick,
            observed_tick,
            supersedes,
        ),
        terminal_id: "dgt".to_owned(),
        movement_id: movement_id.to_owned(),
        kind,
        quantity,
        unit,
    }
}

fn cargo_ledger(
    opening_inventory: u64,
    unit: CargoUnit,
    observations: Vec<CargoMovementObservation>,
) -> CargoInventoryLedger {
    CargoInventoryLedger {
        ledger_id: "dgt-import-stock".to_owned(),
        terminal_id: "dgt".to_owned(),
        inventory_scope_id: "import-laden-v1".to_owned(),
        time_basis_id: "unix-minute-v1".to_owned(),
        window: TickWindow {
            start_tick: 0,
            end_tick: 10,
        },
        unit,
        opening_inventory,
        observations,
    }
}

#[test]
fn vessel_queue_conserves_stock_and_calculates_peak_and_area() {
    let ledger = vessel_ledger(
        2,
        vec![
            vessel(
                "arrival-a",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                2,
                2,
                None,
            ),
            vessel(
                "berth-a",
                "call-a",
                VesselEventKind::BerthEntry,
                EventClass::Actual,
                5,
                5,
                None,
            ),
        ],
    );
    let result = audit_vessel_queue(&ledger, 10).expect("valid queue");
    assert_eq!(result.opening_queue, 2);
    assert_eq!(result.closing_queue, 2);
    assert_eq!(result.arrivals, 1);
    assert_eq!(result.exits, 1);
    assert_eq!(result.peak_queue, 3);
    assert_eq!(result.queue_area_vessel_ticks, 23);
    assert_eq!(result.actual_observations_applied, 2);
}

#[test]
fn planned_requested_and_estimated_events_never_change_actual_queue_stock() {
    let ledger = vessel_ledger(
        4,
        vec![
            vessel(
                "estimate-a",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Estimated,
                12,
                1,
                None,
            ),
            vessel(
                "planned-b",
                "call-b",
                VesselEventKind::EligibleArrival,
                EventClass::Planned,
                13,
                1,
                None,
            ),
            vessel(
                "request-c",
                "call-c",
                VesselEventKind::EligibleArrival,
                EventClass::Requested,
                14,
                1,
                None,
            ),
        ],
    );
    let result = audit_vessel_queue(&ledger, 5).expect("non-actual events excluded");
    assert_eq!(result.closing_queue, 4);
    assert_eq!(result.arrivals, 0);
    assert_eq!(result.actual_observations_applied, 0);
    assert_eq!(result.non_actual_observations_excluded, 3);
}

#[test]
fn late_actual_observation_is_absent_before_ingestion_and_present_after() {
    let ledger = vessel_ledger(
        0,
        vec![vessel(
            "late-arrival",
            "call-late",
            VesselEventKind::EligibleArrival,
            EventClass::Actual,
            2,
            8,
            None,
        )],
    );
    let before = audit_vessel_queue(&ledger, 5).expect("historical information cut-off");
    let after = audit_vessel_queue(&ledger, 9).expect("later information cut-off");
    assert_eq!(before.closing_queue, 0);
    assert_eq!(before.actual_observations_applied, 0);
    assert_eq!(after.closing_queue, 1);
    assert_eq!(after.actual_observations_applied, 1);
}

#[test]
fn explicit_correction_changes_only_snapshots_after_the_correction_is_observed() {
    let ledger = vessel_ledger(
        0,
        vec![
            vessel(
                "arrival-v1",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                2,
                2,
                None,
            ),
            vessel(
                "arrival-v2",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                4,
                6,
                Some("arrival-v1"),
            ),
        ],
    );
    let before = audit_vessel_queue(&ledger, 5).expect("original record still known");
    let after = audit_vessel_queue(&ledger, 7).expect("correction now known");
    assert_eq!(before.queue_area_vessel_ticks, 8);
    assert_eq!(after.queue_area_vessel_ticks, 6);
    assert_eq!(before.actual_observations_applied, 1);
    assert_eq!(after.actual_observations_applied, 1);
}

#[test]
fn conflicting_duplicate_semantic_events_are_not_resolved_by_latest_arrival() {
    let ledger = vessel_ledger(
        0,
        vec![
            vessel(
                "arrival-a",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                2,
                2,
                None,
            ),
            vessel(
                "arrival-b",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                3,
                4,
                None,
            ),
        ],
    );
    assert_eq!(
        audit_vessel_queue(&ledger, 5),
        Err(LedgerError::ConflictingObservation {
            entity_id: "call-a".to_owned(),
            semantic_kind: "eligible-arrival".to_owned(),
        })
    );
}

#[test]
fn queue_cannot_become_negative_and_multiple_exits_for_one_call_reject() {
    let underflow = vessel_ledger(
        0,
        vec![vessel(
            "berth-a",
            "call-a",
            VesselEventKind::BerthEntry,
            EventClass::Actual,
            1,
            1,
            None,
        )],
    );
    assert_eq!(
        audit_vessel_queue(&underflow, 5),
        Err(LedgerError::NegativeVesselQueue(1))
    );

    let duplicate_exit = vessel_ledger(
        1,
        vec![
            vessel(
                "berth-a",
                "call-a",
                VesselEventKind::BerthEntry,
                EventClass::Actual,
                2,
                2,
                None,
            ),
            vessel(
                "diversion-a",
                "call-a",
                VesselEventKind::Diversion,
                EventClass::Actual,
                2,
                2,
                None,
            ),
        ],
    );
    assert_eq!(
        audit_vessel_queue(&duplicate_exit, 5),
        Err(LedgerError::DuplicateVesselExit("call-a".to_owned()))
    );
}

#[test]
fn cargo_inventory_conserves_each_declared_unit_without_implicit_conversion() {
    let ledger = cargo_ledger(
        100,
        CargoUnit::Teu,
        vec![
            cargo(
                "discharge-a",
                "movement-a",
                CargoMovementKind::VesselDischarge,
                20,
                CargoUnit::Teu,
                2,
                2,
                None,
            ),
            cargo(
                "gate-out-a",
                "movement-b",
                CargoMovementKind::GateOut,
                15,
                CargoUnit::Teu,
                5,
                5,
                None,
            ),
        ],
    );
    let result = audit_cargo_inventory(&ledger, 10).expect("valid TEU ledger");
    assert_eq!(result.opening_inventory, 100);
    assert_eq!(result.closing_inventory, 105);
    assert_eq!(result.inbound_quantity, 20);
    assert_eq!(result.outbound_quantity, 15);
    assert_eq!(result.peak_inventory, 120);
    assert_eq!(result.inventory_area_unit_ticks, 1_085);
    assert_eq!(result.unit, CargoUnit::Teu);
}

#[test]
fn cargo_unit_mismatch_and_inventory_underflow_fail_closed() {
    let mismatched = cargo_ledger(
        10,
        CargoUnit::Teu,
        vec![cargo(
            "gate-out",
            "movement-a",
            CargoMovementKind::GateOut,
            1,
            CargoUnit::PhysicalContainers,
            1,
            1,
            None,
        )],
    );
    assert_eq!(
        audit_cargo_inventory(&mismatched, 5),
        Err(LedgerError::CargoUnitMismatch("gate-out".to_owned()))
    );

    let underflow = cargo_ledger(
        0,
        CargoUnit::Tonnes,
        vec![cargo(
            "rail-out",
            "movement-b",
            CargoMovementKind::RailOut,
            1,
            CargoUnit::Tonnes,
            1,
            1,
            None,
        )],
    );
    assert_eq!(
        audit_cargo_inventory(&underflow, 5),
        Err(LedgerError::NegativeCargoInventory(1))
    );
}

#[test]
fn unknown_or_restricted_rights_block_a_ledger_snapshot() {
    let mut event = vessel(
        "arrival-a",
        "call-a",
        VesselEventKind::EligibleArrival,
        EventClass::Actual,
        1,
        1,
        None,
    );
    event.header.source.rights_status = RightsStatus::Unknown;
    let ledger = vessel_ledger(0, vec![event]);
    assert_eq!(
        audit_vessel_queue(&ledger, 5),
        Err(LedgerError::RightsUnknown(
            "evidence-arrival-a".to_owned()
        ))
    );
}

#[test]
fn corrections_must_bind_to_the_same_call_and_exact_semantic_kind() {
    let ledger = vessel_ledger(
        0,
        vec![
            vessel(
                "arrival-a",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                2,
                2,
                None,
            ),
            vessel(
                "berth-correction",
                "call-a",
                VesselEventKind::BerthEntry,
                EventClass::Actual,
                4,
                5,
                Some("arrival-a"),
            ),
        ],
    );
    assert_eq!(
        audit_vessel_queue(&ledger, 6),
        Err(LedgerError::SupersessionKindMismatch(
            "berth-correction".to_owned()
        ))
    );
}
