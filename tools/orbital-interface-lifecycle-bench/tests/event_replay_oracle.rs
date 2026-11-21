use orbital_interface_lifecycle_bench::event_replay::{Corpus, ReplayStatus};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

fn expected_authority(kind: &str) -> &'static str {
    match kind {
        "qualification_issued" | "qualification_revoked" => "qualification-authority",
        "cycle_completed" => "service-event-source",
        "inspection_recorded" | "environment_exposure" => "metrology-authority",
        "maintenance_performed" | "component_replaced" => "maintenance-authority",
        "configuration_changed"
        | "procedure_changed"
        | "interface_changed"
        | "cycle_counter_reset" => "configuration-authority",
        _ => "unsupported-authority",
    }
}

fn reference(
    case_events: &[orbital_interface_lifecycle_bench::event_replay::EventRecord],
    at_tick: u64,
) -> (ReplayStatus, &'static str, u64, usize) {
    let mut unique: HashMap<String, String> = HashMap::new();
    let mut parsed = Vec::<Value>::new();
    let mut duplicates = 0usize;

    for record in case_events {
        let digest = Sha256::digest(record.raw_event.as_bytes());
        let actual_digest = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if actual_digest != record.claimed_sha256 {
            return (
                ReplayStatus::Disputed,
                "event_byte_digest_mismatch",
                0,
                duplicates,
            );
        }
        let event: Value = match serde_json::from_str(&record.raw_event) {
            Ok(value) => value,
            Err(_) => {
                return (
                    ReplayStatus::Rejected,
                    "event_schema_invalid",
                    0,
                    duplicates,
                );
            }
        };
        if event["event_schema"] != "luminous.orbital-interface-lifecycle.event.v0.1" {
            return (
                ReplayStatus::Rejected,
                "unsupported_event_schema",
                0,
                duplicates,
            );
        }
        let id = event["event_id"].as_str().unwrap_or("");
        if id.is_empty() {
            return (
                ReplayStatus::Indeterminate,
                "event_identity_missing",
                0,
                duplicates,
            );
        }
        if let Some(prior) = unique.get(id) {
            if prior == &record.raw_event {
                duplicates += 1;
                continue;
            }
            return (
                ReplayStatus::Disputed,
                "event_id_reused_with_conflicting_bytes",
                0,
                duplicates,
            );
        }
        unique.insert(id.to_owned(), record.raw_event.clone());
        parsed.push(event);
    }

    if parsed.is_empty() {
        return (
            ReplayStatus::Indeterminate,
            "event_stream_empty",
            0,
            duplicates,
        );
    }

    let sid = parsed[0]["stream_id"].as_str().unwrap_or("");
    let pair = parsed[0]["pair_id"].as_str().unwrap_or("");
    let service = parsed[0]["service_kind"].as_str().unwrap_or("");
    if parsed
        .iter()
        .any(|e| e["stream_id"] != sid || e["pair_id"] != pair || e["service_kind"] != service)
    {
        return (
            ReplayStatus::Rejected,
            "stream_pair_or_service_scope_mismatch",
            0,
            duplicates,
        );
    }

    for e in &parsed {
        let kind = e["event_type"].as_str().unwrap_or("");
        if e["producer_id"] != "luminous-synthetic-lab" {
            return (
                ReplayStatus::Indeterminate,
                "producer_not_in_v0_1_allowlist",
                0,
                duplicates,
            );
        }
        if e["authority_scope"] != expected_authority(kind) {
            return (
                ReplayStatus::Rejected,
                "event_authority_scope_mismatch",
                0,
                duplicates,
            );
        }
        let event_tick = e["event_tick"].as_u64().unwrap_or(0);
        let observation_tick = e["observation_tick"].as_u64().unwrap_or(0);
        let ingestion_tick = e["ingestion_tick"].as_u64().unwrap_or(0);
        if observation_tick < event_tick || ingestion_tick < observation_tick {
            return (
                ReplayStatus::Indeterminate,
                "event_observation_ingestion_time_order_invalid",
                0,
                duplicates,
            );
        }
    }

    let mut ordered = BTreeMap::<u64, Value>::new();
    for e in parsed {
        let seq = e["sequence"].as_u64().unwrap_or(0);
        if ordered.insert(seq, e).is_some() {
            return (ReplayStatus::Disputed, "sequence_fork", 0, duplicates);
        }
    }

    let mut prev_id: Option<String> = None;
    let mut component = String::new();
    let mut config = String::new();
    let mut procedure = String::new();
    let mut interface = String::new();
    let mut pair_id = String::new();
    let mut service_kind = String::new();
    let mut cycles = 0u64;
    let mut limit = 0u64;
    let mut expiry = 0u64;
    let mut inspection_interval = 0u64;
    let mut last_inspection = 0u64;
    let mut status = ReplayStatus::Indeterminate;
    let mut reason = "event_stream_not_replayed";

    for (index, (seq, e)) in ordered.iter().enumerate() {
        if *seq != index as u64 + 1 {
            return (
                ReplayStatus::Indeterminate,
                "event_sequence_gap_or_nonzero_start",
                cycles,
                duplicates,
            );
        }
        let event_id = e["event_id"].as_str().unwrap_or("");
        let kind = e["event_type"].as_str().unwrap_or("");
        if index == 0 {
            if kind != "qualification_issued"
                || !e["previous_event_id"].is_null()
                || e["requalification"] == true
            {
                return (
                    ReplayStatus::Rejected,
                    "stream_must_begin_with_initial_qualification",
                    cycles,
                    duplicates,
                );
            }
            component = e["component_id"].as_str().unwrap_or("").to_owned();
            config = e["configuration_id"].as_str().unwrap_or("").to_owned();
            procedure = e["procedure_revision"].as_str().unwrap_or("").to_owned();
            interface = e["interface_revision"].as_str().unwrap_or("").to_owned();
            pair_id = e["pair_id"].as_str().unwrap_or("").to_owned();
            service_kind = e["service_kind"].as_str().unwrap_or("").to_owned();
            limit = e["qualified_cycle_limit"].as_u64().unwrap_or(0);
            expiry = e["valid_until_tick"].as_u64().unwrap_or(0);
            inspection_interval = e["inspection_interval_ticks"].as_u64().unwrap_or(0);
            last_inspection = e["event_tick"].as_u64().unwrap_or(0);
            if e["qualified_cycle_limit"].is_null()
                || e["valid_until_tick"].is_null()
                || e["inspection_interval_ticks"].is_null()
            {
                return (
                    ReplayStatus::Indeterminate,
                    "initial_qualification_envelope_missing",
                    cycles,
                    duplicates,
                );
            }
            status = ReplayStatus::Qualified;
            reason = "qualification_stream_initialized";
        } else {
            if e["previous_event_id"].as_str() != prev_id.as_deref() {
                return (
                    ReplayStatus::Indeterminate,
                    "causal_predecessor_mismatch",
                    cycles,
                    duplicates,
                );
            }
            let prior = &ordered[&(index as u64)];
            let prior_hi = prior["event_tick"]
                .as_u64()
                .unwrap_or(0)
                .saturating_add(prior["time_uncertainty_ticks"].as_u64().unwrap_or(0));
            let current_lo = e["event_tick"]
                .as_u64()
                .unwrap_or(0)
                .saturating_sub(e["time_uncertainty_ticks"].as_u64().unwrap_or(0));
            if prior_hi >= current_lo {
                return (
                    ReplayStatus::Indeterminate,
                    "event_time_order_not_proven",
                    cycles,
                    duplicates,
                );
            }
            if e["pair_id"].as_str() != Some(&pair_id)
                || e["service_kind"].as_str() != Some(&service_kind)
                || e["component_id"].as_str() != Some(&component)
            {
                return (
                    ReplayStatus::Rejected,
                    "article_pair_component_or_service_identity_mismatch",
                    cycles,
                    duplicates,
                );
            }
            if e["configuration_id"].as_str() != Some(config.as_str())
                || e["procedure_revision"].as_str() != Some(procedure.as_str())
                || e["interface_revision"].as_str() != Some(interface.as_str())
            {
                return (
                    ReplayStatus::Rejected,
                    "event_uses_stale_configuration_or_interface",
                    cycles,
                    duplicates,
                );
            }
            match kind {
                "qualification_issued" => {
                    if status == ReplayStatus::Withdrawn {
                        return (
                            ReplayStatus::Withdrawn,
                            "withdrawn_qualification_is_terminal_in_v0_1",
                            cycles,
                            duplicates,
                        );
                    }
                    if status != ReplayStatus::RequalificationRequired
                        || e["requalification"] != true
                    {
                        return (
                            ReplayStatus::Rejected,
                            "unexpected_or_unscoped_requalification_event",
                            cycles,
                            duplicates,
                        );
                    }
                    limit = e["qualified_cycle_limit"].as_u64().unwrap_or(0);
                    expiry = e["valid_until_tick"].as_u64().unwrap_or(0);
                    inspection_interval = e["inspection_interval_ticks"].as_u64().unwrap_or(0);
                    if e["qualified_cycle_limit"].is_null()
                        || e["valid_until_tick"].is_null()
                        || e["inspection_interval_ticks"].is_null()
                    {
                        return (
                            ReplayStatus::Indeterminate,
                            "requalification_envelope_missing",
                            cycles,
                            duplicates,
                        );
                    }
                    cycles = 0;
                    last_inspection = e["event_tick"].as_u64().unwrap_or(0);
                    status = ReplayStatus::Qualified;
                    reason = "qualification_reissued_after_requalification";
                }
                "cycle_completed" => {
                    if status != ReplayStatus::Qualified {
                        return (
                            ReplayStatus::Rejected,
                            "service_cycle_attempted_while_not_qualified",
                            cycles,
                            duplicates,
                        );
                    }
                    cycles += 1;
                    if cycles > limit {
                        status = ReplayStatus::RequalificationRequired;
                        reason = "qualified_cycle_limit_exceeded";
                    } else {
                        reason = "cycle_replayed_once";
                    }
                }
                "inspection_recorded" => {
                    if e["wear_within_limit"] == false {
                        status = ReplayStatus::Withdrawn;
                        reason = "wear_exceeds_declared_limit";
                    } else if e["calibration_valid"] != true
                        || e["measurement_in_range"] != true
                        || e["inspection_coverage_complete"] != true
                        || e["wear_within_limit"] != true
                        || e["uncertainty_clears_margin"] != true
                        || e["raw_artifact_sha256"]
                            .as_str()
                            .map_or(true, |s| s.len() != 64)
                    {
                        return (
                            ReplayStatus::Indeterminate,
                            "inspection_evidence_incomplete_or_invalid",
                            cycles,
                            duplicates,
                        );
                    } else if status == ReplayStatus::Qualified {
                        last_inspection = e["event_tick"].as_u64().unwrap_or(0);
                        reason = "inspection_evidence_admitted";
                    }
                }
                "maintenance_performed" => {
                    status = ReplayStatus::RequalificationRequired;
                    reason = "maintenance_requires_requalification";
                }
                "component_replaced" => {
                    let new = e["new_component_id"].as_str().unwrap_or("");
                    if new.is_empty() {
                        return (
                            ReplayStatus::Indeterminate,
                            "replacement_component_identity_missing",
                            cycles,
                            duplicates,
                        );
                    }
                    if new == component {
                        return (
                            ReplayStatus::Rejected,
                            "replacement_component_identity_invalid",
                            cycles,
                            duplicates,
                        );
                    }
                    component = new.to_owned();
                    cycles = 0;
                    status = ReplayStatus::RequalificationRequired;
                    reason = "component_replacement_requires_requalification";
                }
                "configuration_changed" => {
                    let new = e["new_configuration_id"].as_str().unwrap_or("");
                    if new.is_empty() {
                        return (
                            ReplayStatus::Indeterminate,
                            "new_configuration_identity_missing",
                            cycles,
                            duplicates,
                        );
                    }
                    config = new.to_owned();
                    status = ReplayStatus::RequalificationRequired;
                    reason = "configuration_change_requires_requalification";
                }
                "procedure_changed" => {
                    let new = e["new_procedure_revision"].as_str().unwrap_or("");
                    if new.is_empty() {
                        return (
                            ReplayStatus::Indeterminate,
                            "new_procedure_revision_missing",
                            cycles,
                            duplicates,
                        );
                    }
                    procedure = new.to_owned();
                    status = ReplayStatus::RequalificationRequired;
                    reason = "procedure_change_requires_requalification";
                }
                "interface_changed" => {
                    let new = e["new_interface_revision"].as_str().unwrap_or("");
                    if new.is_empty() {
                        return (
                            ReplayStatus::Indeterminate,
                            "new_interface_revision_missing",
                            cycles,
                            duplicates,
                        );
                    }
                    interface = new.to_owned();
                    status = ReplayStatus::RequalificationRequired;
                    reason = "interface_change_requires_requalification";
                }
                "qualification_revoked" => {
                    status = ReplayStatus::Withdrawn;
                    reason = "qualification_revoked";
                }
                "cycle_counter_reset" => {
                    return (
                        ReplayStatus::Disputed,
                        "cycle_counter_reset_without_component_lineage",
                        cycles,
                        duplicates,
                    );
                }
                "environment_exposure" => {
                    if e["environment_within_envelope"] == false {
                        status = ReplayStatus::RequalificationRequired;
                        reason = "environment_exceeds_qualified_envelope";
                    } else if e["environment_within_envelope"] == true {
                        reason = "environment_within_declared_envelope";
                    } else {
                        return (
                            ReplayStatus::Indeterminate,
                            "environment_observation_missing",
                            cycles,
                            duplicates,
                        );
                    }
                }
                _ => {
                    return (
                        ReplayStatus::Rejected,
                        "unsupported_event_kind",
                        cycles,
                        duplicates,
                    );
                }
            }
        }
        prev_id = Some(event_id.to_owned());
    }
    if at_tick > expiry && status == ReplayStatus::Qualified {
        status = ReplayStatus::RequalificationRequired;
        reason = "qualification_expired_at_evaluation";
    } else if at_tick > last_inspection.saturating_add(inspection_interval)
        && status == ReplayStatus::Qualified
    {
        status = ReplayStatus::RequalificationRequired;
        reason = "inspection_interval_expired_at_evaluation";
    }
    (status, reason, cycles, duplicates)
}

#[test]
fn independent_value_oracle_matches_all_frozen_event_stream_cases() {
    let raw = include_str!("../fixtures/v0.1/cases.json");
    let corpus: Corpus = serde_json::from_str(raw).expect("event corpus parses");
    assert_eq!(corpus.cases.len(), 55);
    for case in &corpus.cases {
        let (status, reason, cycles, duplicates) = reference(&case.events, case.at_tick);
        assert_eq!(status, case.expected_status, "{}", case.case_id);
        assert_eq!(reason, case.expected_reason, "{}", case.case_id);
        assert_eq!(cycles, case.expected_cycle_count, "{}", case.case_id);
        assert_eq!(
            duplicates, case.expected_duplicate_count,
            "{}",
            case.case_id
        );
    }
}

#[test]
fn production_replay_matches_frozen_expected_outcomes() {
    let raw = include_str!("../fixtures/v0.1/cases.json");
    let corpus: Corpus = serde_json::from_str(raw).expect("event corpus parses");
    let report = orbital_interface_lifecycle_bench::event_replay::evaluate(&corpus)
        .expect("corpus evaluates");
    assert_eq!(report.result, "pass");
    assert_eq!(report.summary.case_count, 55);
    assert_eq!(report.summary.expected_result_mismatches, 0);
    assert_eq!(report.summary.duplicate_events, 2);
    assert!(!report.authority_granted);
}
