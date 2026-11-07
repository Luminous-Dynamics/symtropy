use maritime_queue_ledger::{
    CargoInventoryLedger, CargoMovementKind, CargoMovementObservation, CargoUnit,
    EventClass, EventHeader, LedgerError, OpeningCargoInventory, OpeningVesselQueue,
    RightsStatus, SourceEvidence, TickWindow,
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
        opening: OpeningVesselQueue {
            terminal_id: "dgt".to_owned(),
            queue_count: opening_queue,
            measured_tick: 0,
            observed_tick: 0,
            source: source("opening-vessel-stock", RightsStatus::ReviewedAllowed),
        },
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
        opening: OpeningCargoInventory {
            terminal_id: "dgt".to_owned(),
            inventory_scope_id: "import-laden-v1".to_owned(),
            unit,
            quantity: opening_inventory,
            measured_tick: 0,
            observed_tick: 0,
            source: source("opening-cargo-stock", RightsStatus::ReviewedAllowed),
        },
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
fn a_vessel_call_cannot_exit_before_its_observed_in_window_arrival() {
    let ledger = vessel_ledger(
        2,
        vec![
            vessel(
                "arrival-a",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                5,
                5,
                None,
            ),
            vessel(
                "berth-a",
                "call-a",
                VesselEventKind::BerthEntry,
                EventClass::Actual,
                2,
                2,
                None,
            ),
        ],
    );
    assert_eq!(
        audit_vessel_queue(&ledger, 6),
        Err(LedgerError::VesselExitBeforeArrival(
            "call-a".to_owned()
        ))
    );
}

#[test]
fn cyclic_correction_chain_is_rejected() {
    let ledger = vessel_ledger(
        0,
        vec![
            vessel(
                "event-one",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                2,
                3,
                Some("event-two"),
            ),
            vessel(
                "event-two",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                2,
                3,
                Some("event-one"),
            ),
        ],
    );
    assert_eq!(
        audit_vessel_queue(&ledger, 4),
        Err(LedgerError::SupersessionCycle("event-one".to_owned()))
    );
}

#[test]
fn corrections_not_yet_observed_do_not_rewrite_historical_snapshots() {
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
                5,
                Some("arrival-v1"),
            ),
            vessel(
                "arrival-v3",
                "call-a",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                6,
                7,
                Some("arrival-v1"),
            ),
        ],
    );
    let before = audit_vessel_queue(&ledger, 4).expect("future corrections are invisible");
    assert_eq!(before.closing_queue, 1);
    assert_eq!(before.queue_area_vessel_ticks, 8);

    assert_eq!(
        audit_vessel_queue(&ledger, 8),
        Err(LedgerError::MultipleCorrections(
            "arrival-v1".to_owned()
        ))
    );
}

#[test]
fn future_unobserved_bad_record_cannot_invalidate_an_earlier_snapshot() {
    let mut future = vessel(
        "future-event",
        "call-future",
        VesselEventKind::EligibleArrival,
        EventClass::Actual,
        4,
        9,
        None,
    );
    future.terminal_id = "different-terminal".to_owned();
    let ledger = vessel_ledger(
        0,
        vec![
            vessel(
                "known-arrival",
                "call-known",
                VesselEventKind::EligibleArrival,
                EventClass::Actual,
                2,
                2,
                None,
            ),
            future,
        ],
    );

    let historical = audit_vessel_queue(&ledger, 5).expect("future record not yet visible");
    assert_eq!(historical.closing_queue, 1);
    assert_eq!(historical.actual_observations_applied, 1);
    assert_eq!(
        audit_vessel_queue(&ledger, 10),
        Err(LedgerError::TerminalMismatch("future-event".to_owned()))
    );
}

#[test]
fn opening_vessel_queue_requires_matching_scope_and_window_start_measurement() {
    let mut wrong_terminal = vessel_ledger(2, Vec::new());
    wrong_terminal.opening.terminal_id = "pier-1".to_owned();
    assert_eq!(
        audit_vessel_queue(&wrong_terminal, 5),
        Err(LedgerError::OpeningStockScopeMismatch(
            "dgt-vessel-queue".to_owned()
        ))
    );

    let mut wrong_time = vessel_ledger(2, Vec::new());
    wrong_time.opening.measured_tick = 1;
    assert_eq!(
        audit_vessel_queue(&wrong_time, 5),
        Err(LedgerError::OpeningStockTimeMismatch(
            "dgt-vessel-queue".to_owned()
        ))
    );
}

#[test]
fn opening_cargo_stock_requires_matching_inventory_scope_and_unit() {
    let mut wrong_scope = cargo_ledger(10, CargoUnit::Teu, Vec::new());
    wrong_scope.opening.inventory_scope_id = "export-empty-v1".to_owned();
    assert_eq!(
        audit_cargo_inventory(&wrong_scope, 5),
        Err(LedgerError::OpeningStockScopeMismatch(
            "import-laden-v1".to_owned()
        ))
    );

    let mut wrong_unit = cargo_ledger(10, CargoUnit::Teu, Vec::new());
    wrong_unit.opening.unit = CargoUnit::PhysicalContainers;
    assert_eq!(
        audit_cargo_inventory(&wrong_unit, 5),
        Err(LedgerError::OpeningStockScopeMismatch(
            "import-laden-v1".to_owned()
        ))
    );
}

#[test]
fn opening_stock_source_must_be_rights_reviewed_and_known_at_the_cutoff() {
    let mut unknown_rights = vessel_ledger(1, Vec::new());
    unknown_rights.opening.source.rights_status = RightsStatus::Unknown;
    assert_eq!(
        audit_vessel_queue(&unknown_rights, 5),
        Err(LedgerError::RightsUnknown(
            "opening-vessel-stock".to_owned()
        ))
    );

    let mut late_observation = cargo_ledger(10, CargoUnit::Teu, Vec::new());
    late_observation.opening.observed_tick = 6;
    assert_eq!(
        audit_cargo_inventory(&late_observation, 5),
        Err(LedgerError::OpeningStockNotObservedAtCutoff(
            "import-laden-v1".to_owned()
        ))
    );
}

#[test]
fn one_cargo_movement_id_cannot_be_two_actual_movement_kinds() {
    let ledger = cargo_ledger(
        0,
        CargoUnit::Teu,
        vec![
            cargo(
                "movement-discharge",
                "same-movement",
                CargoMovementKind::VesselDischarge,
                1,
                CargoUnit::Teu,
                2,
                2,
                None,
            ),
            cargo(
                "movement-gate-out",
                "same-movement",
                CargoMovementKind::GateOut,
                1,
                CargoUnit::Teu,
                3,
                3,
                None,
            ),
        ],
    );
    assert_eq!(
        audit_cargo_inventory(&ledger, 5),
        Err(LedgerError::ConflictingCargoMovementKind(
            "same-movement".to_owned()
        ))
    );
}

#[test]
fn actual_event_at_the_exclusive_window_end_belongs_to_the_next_window() {
    let ledger = vessel_ledger(
        0,
        vec![vessel(
            "arrival-end",
            "call-end",
            VesselEventKind::EligibleArrival,
            EventClass::Actual,
            10,
            10,
            None,
        )],
    );
    assert_eq!(
        audit_vessel_queue(&ledger, 10),
        Err(LedgerError::ActualEventOutsideWindow(
            "arrival-end".to_owned()
        ))
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
