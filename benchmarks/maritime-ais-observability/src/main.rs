use maritime_ais_observability::{
    AisFix, DwellProfile, GroundTruthEvent, ScoredEventKind, ServiceState, Zone, evaluate_call,
    evaluate_call_with_initial_state, predictions_from_report, score_events,
};

fn fix(tick: i64, id: &str, zone: Zone, speed: u32) -> AisFix {
    AisFix {
        call_reference_id: "SYNTHETIC_CALL_001".to_string(),
        mmsi: "563123456".to_string(),
        message_id: id.to_string(),
        source_id: "SYNTHETIC_AIS_SOURCE".to_string(),
        receiver_id: "SYNTHETIC_SHORE_RX".to_string(),
        geofence_digest: "sha256:synthetic-geofence-v1".to_string(),
        spatial_reference_id: "EPSG:4326".to_string(),
        tick,
        received_tick: 1_000 + tick,
        zone,
        speed_milli_knots: speed,
        position_valid: true,
        nav_status_raw: None,
    }
}

fn main() {
    let profile = DwellProfile::default();
    let baseline_fixes = [
        fix(0, "A0", Zone::Approach, 5_000),
        fix(1, "A1", Zone::Berth, 100),
        fix(2, "A2", Zone::Berth, 100),
        fix(3, "A3", Zone::Berth, 100),
        fix(4, "A4", Zone::Berth, 100),
    ];

    let qualified = evaluate_call_with_initial_state(
        "563123456",
        ServiceState::Outside,
        Some("SYNTHETIC_BASELINE_OUTSIDE_BEFORE_TICK_0"),
        &baseline_fixes,
        &profile,
    )
    .unwrap_or_else(|error| {
        eprintln!("baseline-qualified example failed: {error}");
        std::process::exit(2);
    });

    println!("Maritime edge-AIS observability verifier v0.1");
    println!("Synthetic ticks only; no physical or civil-time interpretation.");
    println!(
        "call_ref={} receiver={} source={} geofence={} spatial_ref={} profile={}",
        qualified.call_reference_id,
        qualified.receiver_id,
        qualified.source_id,
        qualified.geofence_digest,
        qualified.spatial_reference_id,
        qualified.profile_version
    );
    println!(
        "state={} accepted_fixes={} duplicate={} invalid={} nav_conflicts={} \
         kinematic_conflicts={} gaps={}",
        qualified.final_state.label(),
        qualified.accepted_fix_count,
        qualified.duplicate_fix_count,
        qualified.invalid_fix_count,
        qualified.nav_status_conflict_count,
        qualified.kinematic_conflict_count,
        qualified.observability_gap_count
    );
    for event in &qualified.events {
        println!(
            "  {} interval=[{},{}] confirmed_source={} confirmed_receiver={} fixes={}",
            event.kind.label(),
            event.lower_tick,
            event.upper_tick,
            event.confirmation_tick,
            event.confirmation_received_tick,
            event.supporting_fix_count
        );
    }

    let truth = [GroundTruthEvent {
        call_reference_id: "SYNTHETIC_CALL_001".to_string(),
        kind: ScoredEventKind::BerthEntry,
        event_tick: 2,
    }];
    let predictions = predictions_from_report(&qualified);
    let score = score_events(&truth, &predictions, 0).unwrap_or_else(|error| {
        eprintln!("event scorecard failed: {error}");
        std::process::exit(2);
    });
    println!(
        "Synthetic event score: TP={} FP={} FN={} precision={:?} recall={:?} \
         mean_interval_error={:?} mean_interval_width={:?}",
        score.overall.true_positive,
        score.overall.false_positive,
        score.overall.false_negative,
        score.overall.precision,
        score.overall.recall,
        score.overall.mean_interval_distance_ticks,
        score.overall.mean_matched_interval_width_ticks
    );

    let late_start = [
        fix(20, "B0", Zone::Berth, 100),
        fix(21, "B1", Zone::Berth, 100),
        fix(22, "B2", Zone::Berth, 100),
        fix(23, "B3", Zone::Berth, 100),
    ];
    let unknown_start = evaluate_call("563123456", &late_start, &profile).unwrap_or_else(|error| {
        eprintln!("indeterminate-start example failed: {error}");
        std::process::exit(2);
    });
    println!(
        "No baseline: initial={} final={} berth-arrival-events={} reacquisition-events={}",
        unknown_start.initial_state.label(),
        unknown_start.final_state.label(),
        unknown_start
            .events
            .iter()
            .filter(|event| event.kind.label() == "AIS_INFERRED_BERTH_ZONE_ENTRY")
            .count(),
        unknown_start
            .events
            .iter()
            .filter(|event| event.kind.label() == "AIS_ZONE_REACQUIRED_AFTER_UNCERTAINTY")
            .count()
    );

    println!("Dwell sensitivity on identical synthetic fix sequence:");
    for dwell in 1..=4 {
        let mut candidate = profile.clone();
        candidate.berth_dwell_ticks = dwell;
        let report = evaluate_call_with_initial_state(
            "563123456",
            ServiceState::Outside,
            Some("SYNTHETIC_BASELINE_OUTSIDE_BEFORE_TICK_0"),
            &baseline_fixes,
            &candidate,
        )
        .unwrap_or_else(|error| {
            eprintln!("dwell sensitivity failed: {error}");
            std::process::exit(2);
        });
        let confirmation = report
            .events
            .iter()
            .find(|event| event.kind.label() == "AIS_INFERRED_BERTH_ZONE_ENTRY")
            .map_or_else(
                || "NOT_DETECTED".to_string(),
                |event| event.confirmation_tick.to_string(),
            );
        println!("  berth_dwell_ticks={dwell} confirmation_tick={confirmation}");
    }

    println!();
    println!("All examples are synthetic and use upstream-supplied zone labels.");
}
