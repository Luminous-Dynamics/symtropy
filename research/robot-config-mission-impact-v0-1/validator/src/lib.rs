// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural schema and cross-file checks for public synthetic benchmark artifacts.
//! This package does not score robots, infer physical root causes, or qualify equipment.

use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const PACKAGE_VERSION: &str = "robot-config-mission-impact-package-check/0.1";

pub fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root must resolve")
}

pub fn read_json(path: &Path) -> Result<Value, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot_read_file:{}:{error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("invalid_json:{}:{error}", path.display()))
}

pub fn schema_is_valid(schema: &Value) -> bool {
    jsonschema::draft202012::meta::is_valid(schema)
}

pub fn validate_instance(schema: &Value, instance: &Value) -> Vec<String> {
    let validator = match jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(_) => return vec!["schema_could_not_be_compiled".to_owned()],
    };

    let mut errors = validator
        .iter_errors(instance)
        .map(|error| format!("schema_violation:{}", error.instance_path()))
        .collect::<Vec<_>>();
    errors.sort();
    errors.dedup();
    errors
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}

fn array_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Vec<Value>> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_array()
}

fn check_unique_ids(values: &[Value], key: &str, prefix: &str, errors: &mut Vec<String>) {
    let mut ids = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let Some(id) = value.get(key).and_then(Value::as_str) else {
            errors.push(format!("{prefix}_id_missing:{index}"));
            continue;
        };
        if !ids.insert(id.to_owned()) {
            errors.push(format!("{prefix}_id_duplicate:{id}"));
        }
    }
}

fn has_nonempty_ref(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|reference| !reference.is_empty())
}

fn check_trace(
    pair_id: &str,
    side: &str,
    trace: &[Value],
    stage_ids: &[&str],
    errors: &mut Vec<String>,
) -> bool {
    let mut ids = BTreeSet::new();
    let mut previous_event_time = None;
    let mut previous_arrival_time = None;
    let mut event_clock_domain: Option<String> = None;
    let mut arrival_clock_domain: Option<String> = None;
    let mut has_unmapped_clock_relation = false;

    for (index, event) in trace.iter().enumerate() {
        let event_id = event.get("event_id").and_then(Value::as_str);
        let event_time = event.get("event_time_tick").and_then(Value::as_u64);
        let arrival_time = event.get("arrival_time_tick").and_then(Value::as_u64);
        let event_clock = string_at(event, &["event_time_clock_domain_ref"]);
        let arrival_clock = string_at(event, &["arrival_time_clock_domain_ref"]);
        let mapping_status = string_at(event, &["clock_mapping_status"]);
        let mapping_ref = event.get("clock_mapping_ref");
        let stage = event.get("stage_id");

        if let Some(id) = event_id {
            if !ids.insert(id.to_owned()) {
                errors.push(format!("event_id_duplicate:{pair_id}:{side}:{id}"));
            }
        } else {
            errors.push(format!("event_id_missing:{pair_id}:{side}:{index}"));
        }

        if let Some(time) = event_time {
            if previous_event_time.is_some_and(|previous| time < previous) {
                errors.push(format!("event_time_decreased:{pair_id}:{side}:{index}"));
            }
            previous_event_time = Some(time);
        } else {
            errors.push(format!("event_time_missing:{pair_id}:{side}:{index}"));
        }

        if let Some(time) = arrival_time {
            if previous_arrival_time.is_some_and(|previous| time < previous) {
                errors.push(format!("arrival_time_decreased:{pair_id}:{side}:{index}"));
            }
            previous_arrival_time = Some(time);
        } else {
            errors.push(format!("arrival_time_missing:{pair_id}:{side}:{index}"));
        }

        match event_clock {
            Some(domain) if !domain.trim().is_empty() => {
                if event_clock_domain
                    .as_deref()
                    .is_some_and(|previous| previous != domain)
                {
                    errors.push(format!(
                        "event_clock_domain_changed_within_trace:{pair_id}:{side}:{index}"
                    ));
                }
                event_clock_domain = Some(domain.to_owned());
            }
            _ => errors.push(format!("event_clock_domain_missing:{pair_id}:{side}:{index}")),
        }

        match arrival_clock {
            Some(domain) if !domain.trim().is_empty() => {
                if arrival_clock_domain
                    .as_deref()
                    .is_some_and(|previous| previous != domain)
                {
                    errors.push(format!(
                        "arrival_clock_domain_changed_within_trace:{pair_id}:{side}:{index}"
                    ));
                }
                arrival_clock_domain = Some(domain.to_owned());
            }
            _ => errors.push(format!("arrival_clock_domain_missing:{pair_id}:{side}:{index}")),
        }

        match stage {
            Some(Value::Null) => {}
            Some(Value::String(stage_id)) if stage_ids.contains(&stage_id.as_str()) => {}
            Some(Value::String(stage_id)) => errors.push(format!(
                "event_stage_undeclared:{pair_id}:{side}:{index}:{stage_id}"
            )),
            _ => errors.push(format!(
                "event_stage_missing_or_invalid:{pair_id}:{side}:{index}"
            )),
        }

        match mapping_status {
            Some("qualified") if has_nonempty_ref(mapping_ref) => {}
            Some("unqualified") if has_nonempty_ref(mapping_ref) => {
                if event_clock != arrival_clock {
                    has_unmapped_clock_relation = true;
                }
            }
            Some("absent") if mapping_ref.is_some_and(Value::is_null) => {
                if event_clock != arrival_clock {
                    has_unmapped_clock_relation = true;
                }
            }
            _ => errors.push(format!(
                "clock_mapping_status_reference_mismatch:{pair_id}:{side}:{index}"
            )),
        }
    }

    has_unmapped_clock_relation
}

/// Validate the explicit event-type-to-stage and event-source mapping catalog.
/// This prevents the validator from inferring stage semantics from event-name strings.
pub fn event_type_catalog_invariants(
    stage_catalog: &Value,
    event_catalog: &Value,
    corpus: &Value,
) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(stages) = array_at(stage_catalog, &["stages"]) else {
        return vec!["event_catalog_stage_catalog_missing".to_owned()];
    };
    let Some(specs) = array_at(event_catalog, &["events"]) else {
        return vec!["event_catalog_events_missing".to_owned()];
    };
    let Some(cases) = array_at(corpus, &["cases"]) else {
        return vec!["event_catalog_corpus_cases_missing".to_owned()];
    };

    check_unique_ids(specs, "event_type", "event_type", &mut errors);
    let stage_ids = stages
        .iter()
        .filter_map(|stage| stage.get("stage_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let mut observed_types = BTreeSet::new();

    for spec in specs {
        let Some(event_type) = string_at(spec, &["event_type"]) else {
            continue;
        };
        match spec.get("stage_id") {
            Some(Value::Null) => {}
            Some(Value::String(stage_id)) if stage_ids.contains(stage_id.as_str()) => {}
            _ => errors.push(format!("event_catalog_stage_undeclared:{event_type}")),
        }
        let allowed = array_at(spec, &["allowed_source_classes"])
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        if allowed.is_empty() {
            errors.push(format!("event_catalog_allowed_sources_empty:{event_type}"));
        }
    }

    for case in cases {
        let pair_id = string_at(case, &["pair_id"]).unwrap_or("unknown_pair");
        for (side, trace_name) in [("source", "source_trace"), ("target", "target_trace")] {
            let Some(trace) = array_at(case, &[trace_name]) else {
                continue;
            };
            for (index, event) in trace.iter().enumerate() {
                let Some(event_type) = string_at(event, &["event_type"]) else {
                    errors.push(format!("event_type_missing:{pair_id}:{side}:{index}"));
                    continue;
                };
                observed_types.insert(event_type.to_owned());
                let Some(spec) = specs
                    .iter()
                    .find(|spec| string_at(spec, &["event_type"]) == Some(event_type))
                else {
                    errors.push(format!(
                        "event_type_not_catalogued:{pair_id}:{side}:{index}:{event_type}"
                    ));
                    continue;
                };

                if spec.get("stage_id") != event.get("stage_id") {
                    errors.push(format!(
                        "event_type_stage_mapping_mismatch:{pair_id}:{side}:{index}:{event_type}"
                    ));
                }

                let source_class = string_at(event, &["source_class"]);
                let allowed = array_at(spec, &["allowed_source_classes"])
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<BTreeSet<_>>();
                if source_class.is_none_or(|source| !allowed.contains(source)) {
                    errors.push(format!(
                        "event_type_source_class_not_allowed:{pair_id}:{side}:{index}:{event_type}"
                    ));
                }
            }
        }
    }

    let catalog_types = specs
        .iter()
        .filter_map(|spec| string_at(spec, &["event_type"]))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for missing in observed_types.difference(&catalog_types) {
        errors.push(format!("event_type_catalog_missing:{missing}"));
    }
    for unused in catalog_types.difference(&observed_types) {
        errors.push(format!("event_type_catalog_unused:{unused}"));
    }

    errors.sort();
    errors.dedup();
    errors
}

/// Cross-file invariants for the stage catalog, paired development corpus and public oracle.
/// The checks concern package integrity only; the oracle labels remain human-authored fixtures.
pub fn cross_file_invariants(stage_catalog: &Value, corpus: &Value, oracle: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let stages = match array_at(stage_catalog, &["stages"]) {
        Some(stages) => stages,
        None => return vec!["stage_catalog_missing_stages".to_owned()],
    };
    let cases = match array_at(corpus, &["cases"]) {
        Some(cases) => cases,
        None => return vec!["corpus_missing_cases".to_owned()],
    };
    let dispositions = match array_at(oracle, &["dispositions"]) {
        Some(dispositions) => dispositions,
        None => return vec!["oracle_missing_dispositions".to_owned()],
    };

    check_unique_ids(stages, "stage_id", "stage", &mut errors);
    check_unique_ids(cases, "pair_id", "pair", &mut errors);
    check_unique_ids(dispositions, "pair_id", "oracle_pair", &mut errors);

    let stage_ids = stages
        .iter()
        .filter_map(|stage| stage.get("stage_id").and_then(Value::as_str))
        .collect::<Vec<_>>();
    if stage_ids.len() != 10 {
        errors.push("stage_catalog_count_not_ten".to_owned());
    }
    for (index, stage) in stages.iter().enumerate() {
        let expected_id = format!("S{:02}", index + 1);
        if string_at(stage, &["stage_id"]) != Some(expected_id.as_str())
            || stage.get("order").and_then(Value::as_u64) != Some((index + 1) as u64)
        {
            errors.push(format!("stage_order_or_identity_mismatch:{index}"));
        }
    }

    let case_ids = cases
        .iter()
        .filter_map(|case| case.get("pair_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let oracle_ids = dispositions
        .iter()
        .filter_map(|entry| entry.get("pair_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if case_ids != oracle_ids {
        errors.push("corpus_oracle_pair_coverage_mismatch".to_owned());
    }

    let mut has_zero_aggregate_delta_with_stage_sensitivity = false;
    let mut has_unlocalized_unknown = false;

    for case in cases {
        let pair_id = string_at(case, &["pair_id"]).unwrap_or("unknown_pair");
        let source_ref = string_at(case, &["source_configuration", "configuration_ref"]);
        let target_ref = string_at(case, &["target_configuration", "configuration_ref"]);
        if source_ref.is_none() || target_ref.is_none() || source_ref == target_ref {
            errors.push(format!("source_target_configuration_invalid:{pair_id}"));
        }

        let changed = array_at(case, &["declared_delta", "changed_components"]);
        if changed.is_none_or(Vec::is_empty) {
            errors.push(format!("changed_component_list_empty:{pair_id}"));
        }

        for field in [
            "same_task",
            "same_site_snapshot",
            "same_acceptance_profile",
            "same_reference_method",
            "same_crew_policy",
        ] {
            if bool_at(case, &["matched_conditions", field]) != Some(true) {
                errors.push(format!("matched_condition_not_true:{pair_id}:{field}"));
            }
        }

        let mut source_clock_unmapped = false;
        let mut target_clock_unmapped = false;
        for (side, path) in [("source", "source_trace"), ("target", "target_trace")] {
            match array_at(case, &[path]) {
                Some(trace) if !trace.is_empty() => {
                    let unmapped =
                        check_trace(pair_id, side, trace, &stage_ids, &mut errors);
                    if side == "source" {
                        source_clock_unmapped = unmapped;
                    } else {
                        target_clock_unmapped = unmapped;
                    }
                }
                _ => errors.push(format!(
                    "trace_missing_or_empty:{pair_id}:{side}"
                )),
            }
        }
        if source_clock_unmapped {
            errors.push(format!(
                "source_trace_clock_mapping_not_qualified:{pair_id}"
            ));
        }

        let Some(disposition) = dispositions
            .iter()
            .find(|entry| entry.get("pair_id").and_then(Value::as_str) == Some(pair_id))
        else {
            continue;
        };

        if target_clock_unmapped {
            let progression = array_at(disposition, &["failure_progression"]);
            let has_temporal_conflict = progression.is_some_and(|items| {
                items.iter().any(|item| {
                    matches!(
                        item.as_str(),
                        Some("event_time_arrival_time_conflict" | "temporal_reconciliation_unknown")
                    )
                })
            });
            let stage_status = array_at(disposition, &["target_stage_status"]);
            let acceptance_or_reconciliation_unknown = stage_status.is_some_and(|rows| {
                rows.iter().any(|row| {
                    let stage_id = row.get(0).and_then(Value::as_str);
                    let status = row.get(1).and_then(Value::as_str);
                    matches!(stage_id, Some("S08" | "S10")) && status == Some("unknown")
                })
            });
            if !has_temporal_conflict || !acceptance_or_reconciliation_unknown {
                errors.push(format!(
                    "target_unmapped_clock_not_reflected_as_temporal_unknown:{pair_id}"
                ));
            }
        }

        let stage_status = array_at(disposition, &["target_stage_status"]);
        if stage_status.is_none_or(|rows| rows.len() != stage_ids.len()) {
            errors.push(format!("oracle_stage_denominator_mismatch:{pair_id}"));
        } else if let Some(rows) = stage_status {
            for (index, row) in rows.iter().enumerate() {
                let row_stage = row.get(0).and_then(Value::as_str);
                if row_stage != Some(stage_ids[index]) {
                    errors.push(format!("oracle_stage_order_mismatch:{pair_id}:{index}"));
                }
            }
        }

        let target_trace = array_at(case, &["target_trace"]);
        if let (Some(rows), Some(events)) = (stage_status, target_trace) {
            let stage_dispositions = rows
                .iter()
                .filter_map(|row| Some((row.get(0)?.as_str()?, row.get(1)?.as_str()?)))
                .collect::<std::collections::BTreeMap<_, _>>();
            for event in events {
                if let Some(stage_id) = event.get("stage_id").and_then(Value::as_str) {
                    if matches!(
                        stage_dispositions.get(stage_id),
                        Some(&"not_reached") | Some(&"not_run")
                    ) {
                        let event_id = string_at(event, &["event_id"]).unwrap_or("unknown_event");
                        errors.push(format!(
                            "event_emitted_by_unreached_stage:{pair_id}:{stage_id}:{event_id}"
                        ));
                    }
                }
            }
        }

        let divergence = disposition.get("first_observed_divergence_stage");
        let attribution = string_at(disposition, &["attribution_status"]);
        if divergence.is_some_and(Value::is_null) {
            if attribution != Some("unlocalized_unknown") {
                errors.push(format!("null_divergence_not_marked_unknown:{pair_id}"));
            }
            has_unlocalized_unknown = true;
        } else if let Some(stage_id) = divergence.and_then(Value::as_str) {
            if !stage_ids.contains(&stage_id) {
                errors.push(format!(
                    "oracle_divergence_stage_undeclared:{pair_id}:{stage_id}"
                ));
            }
            if attribution != Some("stage_localized_not_root_cause") {
                errors.push(format!("localized_divergence_marked_unknown:{pair_id}"));
            }
        } else {
            errors.push(format!("oracle_divergence_stage_missing:{pair_id}"));
        }

        if bool_at(disposition, &["aggregate_mission_success_changed"]) == Some(false)
            && bool_at(disposition, &["paired_outcome_sensitive"]) == Some(true)
        {
            has_zero_aggregate_delta_with_stage_sensitivity = true;
        }

        if array_at(disposition, &["failure_progression"]).is_none_or(Vec::is_empty) {
            errors.push(format!("failure_progression_missing:{pair_id}"));
        }
    }

    if !has_zero_aggregate_delta_with_stage_sensitivity {
        errors.push("zero_aggregate_delta_stage_sensitivity_control_missing".to_owned());
    }
    if !has_unlocalized_unknown {
        errors.push("unlocalized_unknown_control_missing".to_owned());
    }

    errors.sort();
    errors.dedup();
    errors
}

/// Check that the separate A/B workflow and accepted-output metrics reconcile with
/// the public mission oracle. No composite productivity or safety score is created.
pub fn outcome_metric_invariants(metrics: &Value, oracle: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(rows) = array_at(metrics, &["rows"]) else {
        return vec!["outcome_metrics_rows_missing".to_owned()];
    };
    let Some(dispositions) = array_at(oracle, &["dispositions"]) else {
        return vec!["outcome_metrics_oracle_missing".to_owned()];
    };

    check_unique_ids(rows, "pair_id", "outcome_pair", &mut errors);
    let metric_ids = rows
        .iter()
        .filter_map(|row| row.get("pair_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let oracle_ids = dispositions
        .iter()
        .filter_map(|row| row.get("pair_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if metric_ids != oracle_ids {
        errors.push("outcome_metrics_oracle_pair_coverage_mismatch".to_owned());
    }

    for row in rows {
        let pair_id = string_at(row, &["pair_id"]).unwrap_or("unknown_pair");
        let Some(disposition) = dispositions
            .iter()
            .find(|item| string_at(item, &["pair_id"]) == Some(pair_id))
        else {
            continue;
        };

        for side in ["source", "target"] {
            let Some(outcome) = row.get(side) else {
                errors.push(format!("outcome_metrics_side_missing:{pair_id}:{side}"));
                continue;
            };
            let status = string_at(outcome, &["mission_outcome"]);
            let counts = [
                "accepted_units",
                "rejected_units",
                "held_units",
                "aborted_units",
                "unknown_units",
                "not_started_units",
            ];
            let mut outcome_total = 0_u64;
            for field in counts {
                match outcome.get(field).and_then(Value::as_u64) {
                    Some(value) => outcome_total = outcome_total.saturating_add(value),
                    None => errors.push(format!(
                        "outcome_count_missing_or_invalid:{pair_id}:{side}:{field}"
                    )),
                }
            }
            if outcome_total != 1 {
                errors.push(format!("functional_unit_denominator_not_one:{pair_id}:{side}"));
            }

            let attempted = outcome
                .get("functional_units_attempted")
                .and_then(Value::as_u64);
            if attempted.is_none_or(|count| count > 1) {
                errors.push(format!("functional_units_attempted_invalid:{pair_id}:{side}"));
            }
            let status_allows_zero_attempts =
                matches!(status, Some("not_started" | "unknown"));
            if (attempted == Some(0) && !status_allows_zero_attempts)
                || (status == Some("not_started") && attempted != Some(0))
            {
                errors.push(format!("not_started_attempt_denominator_mismatch:{pair_id}:{side}"));
            }

            let expected = if side == "source" {
                string_at(disposition, &["source_mission_status"])
            } else {
                string_at(disposition, &["target_mission_status"])
            };
            let normalized = match expected {
                Some("accepted" | "accepted_after_reinspection") => Some("accepted"),
                Some("rejected") => Some("rejected"),
                Some("held") => Some("held"),
                Some("aborted_safe_hold") => Some("aborted_safe_hold"),
                Some("unresolved" | "unknown_blocked") => Some("unknown"),
                Some("not_run_target_hold") => Some("not_started"),
                _ => None,
            };
            if normalized != status {
                errors.push(format!("outcome_status_oracle_mismatch:{pair_id}:{side}"));
            }

            let interventions = outcome
                .get("operator_interventions")
                .and_then(Value::as_u64);
            let intervention_seconds = outcome
                .get("intervention_seconds")
                .and_then(Value::as_u64);
            let crew_seconds = outcome
                .get("total_workflow_crew_seconds")
                .and_then(Value::as_u64);
            if interventions.is_none() || intervention_seconds.is_none() || crew_seconds.is_none()
            {
                errors.push(format!("intervention_or_crew_metric_missing:{pair_id}:{side}"));
            } else if intervention_seconds > crew_seconds {
                errors.push(format!(
                    "intervention_time_exceeds_workflow_crew_time:{pair_id}:{side}"
                ));
            }
            if outcome
                .get("reinspection_count")
                .and_then(Value::as_u64)
                .is_none()
            {
                errors.push(format!("reinspection_count_invalid:{pair_id}:{side}"));
            }

            let quality = outcome.get("quality");
            let signed_error = quality.and_then(|value| value.get("signed_error_mm"));
            let uncertainty = quality.and_then(|value| value.get("expanded_uncertainty_mm"));
            let measurement_ref = quality.and_then(|value| value.get("independent_measurement_ref"));
            let has_measurement = signed_error.is_some_and(Value::is_number)
                && uncertainty.is_some_and(Value::is_number)
                && has_nonempty_ref(measurement_ref);
            let no_measurement = signed_error.is_some_and(Value::is_null)
                && uncertainty.is_some_and(Value::is_null)
                && measurement_ref.is_some_and(Value::is_null);
            if matches!(status, Some("accepted" | "rejected")) && !has_measurement {
                errors.push(format!("accepted_or_rejected_output_lacks_referee_measurement:{pair_id}:{side}"));
            }
            if !matches!(status, Some("accepted" | "rejected")) && !has_measurement && !no_measurement {
                errors.push(format!("partial_quality_measurement_fields:{pair_id}:{side}"));
            }
            if has_measurement {
                let deviation = signed_error.and_then(Value::as_f64).unwrap_or(f64::INFINITY);
                let expanded = uncertainty.and_then(Value::as_f64).unwrap_or(f64::INFINITY);
                let tolerance = quality
                    .and_then(|value| value.get("acceptance_tolerance_mm"))
                    .and_then(Value::as_f64)
                    .unwrap_or(f64::NEG_INFINITY);
                if !deviation.is_finite()
                    || !expanded.is_finite()
                    || !tolerance.is_finite()
                    || expanded < 0.0
                    || tolerance <= 0.0
                {
                    errors.push(format!("quality_measurement_numeric_invalid:{pair_id}:{side}"));
                } else if status == Some("accepted") && deviation.abs() + expanded > tolerance {
                    errors.push(format!("accepted_output_outside_uncertainty_guarded_tolerance:{pair_id}:{side}"));
                } else if status == Some("rejected") && deviation.abs() + expanded <= tolerance {
                    errors.push(format!("rejected_output_inside_uncertainty_guarded_tolerance:{pair_id}:{side}"));
                }
            }
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

pub fn validate_package(root: &Path) -> Result<(), Vec<String>> {
    let base = root.join("research/robot-config-mission-impact-v0-1");
    let files = [
        (
            "stage-catalog-v0.1.json",
            "schemas/stage-catalog-v0.1.schema.json",
        ),
        (
            "mission-pair-corpus-v0.1.json",
            "schemas/mission-pair-corpus-v0.1.schema.json",
        ),
        (
            "development-oracle-v0.1.json",
            "schemas/development-oracle-v0.1.schema.json",
        ),
        (
            "event-type-catalog-v0.1.json",
            "schemas/event-type-catalog-v0.1.schema.json",
        ),
        (
            "mission-outcome-metrics-v0.1.json",
            "schemas/mission-outcome-metrics-v0.1.schema.json",
        ),
    ];
    let mut values = Vec::new();
    let mut errors = Vec::new();

    for (instance_name, schema_name) in files {
        let instance = read_json(&base.join(instance_name)).map_err(|error| vec![error])?;
        let schema = read_json(&base.join(schema_name)).map_err(|error| vec![error])?;
        if !schema_is_valid(&schema) {
            errors.push(format!("schema_meta_invalid:{schema_name}"));
        }
        errors.extend(
            validate_instance(&schema, &instance)
                .into_iter()
                .map(|error| format!("{instance_name}:{error}")),
        );
        values.push(instance);
    }

    if errors.is_empty() {
        errors.extend(cross_file_invariants(&values[0], &values[1], &values[2]));
        errors.extend(event_type_catalog_invariants(&values[0], &values[3], &values[1]));
        errors.extend(outcome_metric_invariants(&values[4], &values[2]));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort();
        errors.dedup();
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schemas_validate_against_draft_2020_12_meta_schema() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        for name in [
            "schemas/stage-catalog-v0.1.schema.json",
            "schemas/mission-pair-corpus-v0.1.schema.json",
            "schemas/development-oracle-v0.1.schema.json",
            "schemas/event-type-catalog-v0.1.schema.json",
            "schemas/mission-outcome-metrics-v0.1.schema.json",
        ] {
            let schema = read_json(&base.join(name)).expect("schema JSON parses");
            assert!(
                schema_is_valid(&schema),
                "{name} must satisfy Draft 2020-12"
            );
        }
    }

    #[test]
    fn all_development_artifacts_validate_and_reconcile() {
        validate_package(&repository_root()).expect("mission-impact package must validate");
    }

    #[test]
    fn event_catalog_matches_every_event_stage_and_source_class() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let catalog = read_json(&base.join("event-type-catalog-v0.1.json")).unwrap();
        let corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        assert!(event_type_catalog_invariants(&stage, &catalog, &corpus).is_empty());
    }

    #[test]
    fn event_catalog_rejects_stage_mapping_drift() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let mut catalog = read_json(&base.join("event-type-catalog-v0.1.json")).unwrap();
        let corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        let item = catalog["events"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["event_type"] == "tool_identity_read")
            .unwrap();
        item["stage_id"] = Value::String("S05".to_owned());
        assert!(event_type_catalog_invariants(&stage, &catalog, &corpus)
            .iter()
            .any(|error| error.starts_with("event_type_stage_mapping_mismatch:")));
    }

    #[test]
    fn event_catalog_rejects_unapproved_source_class() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let mut catalog = read_json(&base.join("event-type-catalog-v0.1.json")).unwrap();
        let corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        let item = catalog["events"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["event_type"] == "tool_identity_read")
            .unwrap();
        item["allowed_source_classes"] = Value::Array(vec![Value::String(
            "independent_referee".to_owned(),
        )]);
        assert!(event_type_catalog_invariants(&stage, &catalog, &corpus)
            .iter()
            .any(|error| error.starts_with("event_type_source_class_not_allowed:")));
    }

    #[test]
    fn outcome_metrics_reconcile_with_mission_oracle() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let metrics = read_json(&base.join("mission-outcome-metrics-v0.1.json")).unwrap();
        let oracle = read_json(&base.join("development-oracle-v0.1.json")).unwrap();
        assert!(outcome_metric_invariants(&metrics, &oracle).is_empty());
    }

    #[test]
    fn accepted_output_cannot_exceed_uncertainty_guarded_tolerance() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let mut metrics = read_json(&base.join("mission-outcome-metrics-v0.1.json")).unwrap();
        let oracle = read_json(&base.join("development-oracle-v0.1.json")).unwrap();
        metrics["rows"][5]["target"]["quality"]["signed_error_mm"] = Value::from(1.9);
        assert!(outcome_metric_invariants(&metrics, &oracle)
            .iter()
            .any(|error| error.starts_with("accepted_output_outside_uncertainty_guarded_tolerance:")));
    }

    #[test]
    fn accepted_output_requires_independent_measurement_reference() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let mut metrics = read_json(&base.join("mission-outcome-metrics-v0.1.json")).unwrap();
        let oracle = read_json(&base.join("development-oracle-v0.1.json")).unwrap();
        metrics["rows"][0]["source"]["quality"]["independent_measurement_ref"] = Value::Null;
        assert!(outcome_metric_invariants(&metrics, &oracle)
            .iter()
            .any(|error| error.starts_with("accepted_or_rejected_output_lacks_referee_measurement:")));
    }

    #[test]
    fn reordered_oracle_entries_do_not_break_pair_coverage() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        let mut oracle = read_json(&base.join("development-oracle-v0.1.json")).unwrap();
        oracle["dispositions"].as_array_mut().unwrap().reverse();
        assert!(cross_file_invariants(&stage, &corpus, &oracle).is_empty());
    }

    #[test]
    fn duplicated_pair_identifier_is_rejected() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let mut corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        let duplicate = corpus["cases"][0].clone();
        corpus["cases"].as_array_mut().unwrap().push(duplicate);
        assert!(
            cross_file_invariants(
                &stage,
                &corpus,
                &read_json(&base.join("development-oracle-v0.1.json")).unwrap()
            )
            .iter()
            .any(|error| error.starts_with("pair_id_duplicate:"))
        );
    }

    #[test]
    fn event_cannot_be_emitted_by_a_stage_marked_not_reached() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        let mut oracle = read_json(&base.join("development-oracle-v0.1.json")).unwrap();
        let entry = oracle["dispositions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["pair_id"] == "MIP-002")
            .unwrap();
        let stage_row = entry["target_stage_status"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row[0] == "S06")
            .unwrap();
        stage_row[1] = Value::String("not_reached".to_owned());
        assert!(
            cross_file_invariants(&stage, &corpus, &oracle)
                .iter()
                .any(|error| error.starts_with("event_emitted_by_unreached_stage:MIP-002:S06:"))
        );
    }

    #[test]
    fn unknown_impact_cannot_be_forced_into_a_stage_guess() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        let mut oracle = read_json(&base.join("development-oracle-v0.1.json")).unwrap();
        let unknown = oracle["dispositions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["pair_id"] == "MIP-008")
            .unwrap();
        unknown["first_observed_divergence_stage"] = Value::String("S02".to_owned());
        unknown["attribution_status"] = Value::String("unlocalized_unknown".to_owned());
        let errors = cross_file_invariants(&stage, &corpus, &oracle);
        assert!(
            !errors.is_empty(),
            "unknown dependency scope must not receive invented stage attribution"
        );
        assert!(
            errors
                .iter()
                .any(|error| error == "localized_divergence_marked_unknown:MIP-008")
        );
    }

    #[test]
    fn non_monotonic_arrival_time_is_rejected() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let mut corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        corpus["cases"][0]["target_trace"][1]["arrival_time_tick"] = Value::from(0);
        let oracle = read_json(&base.join("development-oracle-v0.1.json")).unwrap();
        assert!(
            cross_file_invariants(&stage, &corpus, &oracle)
                .iter()
                .any(|error| error.starts_with("arrival_time_decreased:"))
        );
    }

    #[test]
    fn event_stage_must_exist_in_the_stage_catalog() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let mut corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        corpus["cases"][0]["target_trace"][0]["stage_id"] = Value::String("S99".to_owned());
        let oracle = read_json(&base.join("development-oracle-v0.1.json")).unwrap();
        assert!(
            cross_file_invariants(&stage, &corpus, &oracle)
                .iter()
                .any(|error| error.starts_with("event_stage_undeclared:"))
        );
    }

    #[test]
    fn missing_target_clock_mapping_requires_temporal_unknown_disposition() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        let mut oracle = read_json(&base.join("development-oracle-v0.1.json")).unwrap();
        let entry = oracle["dispositions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["pair_id"] == "MIP-005")
            .unwrap();
        entry["failure_progression"] =
            Value::Array(vec![Value::String("generic_failure".to_owned())]);
        assert!(cross_file_invariants(&stage, &corpus, &oracle).iter().any(
            |error| error == "target_unmapped_clock_not_reflected_as_temporal_unknown:MIP-005"
        ));
    }

    #[test]
    fn non_monotonic_event_time_is_rejected() {
        let root = repository_root();
        let base = root.join("research/robot-config-mission-impact-v0-1");
        let stage = read_json(&base.join("stage-catalog-v0.1.json")).unwrap();
        let mut corpus = read_json(&base.join("mission-pair-corpus-v0.1.json")).unwrap();
        corpus["cases"][0]["target_trace"][1]["event_time_tick"] = Value::from(0);
        let oracle = read_json(&base.join("development-oracle-v0.1.json")).unwrap();
        assert!(
            cross_file_invariants(&stage, &corpus, &oracle)
                .iter()
                .any(|error| error.starts_with("event_time_decreased:"))
        );
    }
}
