use port_nexus_event_trigger_monitor::{
    evaluate_trigger, EvidenceQuality, HardBound, SignalDefinition, SignalDomain,
    SignalMeasurement, TriggerPolicy, TriggerSnapshot, TriggerState,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let definitions = vec![
        SignalDefinition {
            signal_id: "berth_arrival_delay_min".to_owned(),
            domain: SignalDomain::Logistics,
            unit: "min".to_owned(),
            baseline_value: 0.0,
            soft_absolute_threshold: 10.0,
            soft_relative_threshold: 0.0,
            max_age_ms: 60_000,
            critical: true,
            hard_bound: None,
        },
        SignalDefinition {
            signal_id: "pcc_headroom_kw".to_owned(),
            domain: SignalDomain::Grid,
            unit: "kW".to_owned(),
            baseline_value: 400.0,
            soft_absolute_threshold: 80.0,
            soft_relative_threshold: 0.15,
            max_age_ms: 30_000,
            critical: true,
            hard_bound: Some(HardBound::Lower { limit: 100.0 }),
        },
        SignalDefinition {
            signal_id: "compute_deadline_slack_min".to_owned(),
            domain: SignalDomain::Compute,
            unit: "min".to_owned(),
            baseline_value: 120.0,
            soft_absolute_threshold: 30.0,
            soft_relative_threshold: 0.0,
            max_age_ms: 60_000,
            critical: true,
            hard_bound: Some(HardBound::Lower { limit: 15.0 }),
        },
    ];
    let snapshot = TriggerSnapshot {
        snapshot_id: "synthetic-port-15m-004".to_owned(),
        site_id: "synthetic-port-nexus".to_owned(),
        baseline_plan_id: "day-ahead-plan-v1".to_owned(),
        sequence: 4,
        decision_time_ms: 1_000_000,
        plan_generated_at_ms: 900_000,
        measurements: vec![
            SignalMeasurement {
                signal_id: "berth_arrival_delay_min".to_owned(),
                observed_value: 18.0,
                observed_at_ms: 999_000,
                first_available_at_ms: 999_200,
                evidence_ref: "synthetic:berth-004".to_owned(),
                quality: EvidenceQuality::Synthetic,
            },
            SignalMeasurement {
                signal_id: "pcc_headroom_kw".to_owned(),
                observed_value: 75.0,
                observed_at_ms: 999_500,
                first_available_at_ms: 999_700,
                evidence_ref: "synthetic:pcc-004".to_owned(),
                quality: EvidenceQuality::Synthetic,
            },
            SignalMeasurement {
                signal_id: "compute_deadline_slack_min".to_owned(),
                observed_value: 45.0,
                observed_at_ms: 999_000,
                first_available_at_ms: 999_250,
                evidence_ref: "synthetic:compute-004".to_owned(),
                quality: EvidenceQuality::Synthetic,
            },
        ],
    };
    let policy = TriggerPolicy {
        normal_cooldown_ms: 900_000,
        max_plan_age_ms: 3_600_000,
        hysteresis_reset_ratio: 0.5,
    };

    let result = evaluate_trigger(&snapshot, &definitions, &policy, &TriggerState::default())?;
    println!("Port Nexus event-trigger monitor v0.1.0");
    println!("Decision: {:?}", result.decision);
    println!("Reasons: {:?}", result.reason_codes);
    println!("Evidence refresh required: {}", result.evidence_refresh_required);
    println!("No schedule, dispatch, reservation or physical state was changed.");
    Ok(())
}
