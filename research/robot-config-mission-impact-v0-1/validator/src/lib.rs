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
                    let unmapped = check_trace(pair_id, side, trace, &stage_ids, &mut errors);
                    if side == "source" {
                        source_clock_unmapped = unmapped;
                    } else {
                        target_clock_unmapped = unmapped;
                    }
                }
                _ => errors.push(format!("trace_missing_or_empty:{pair_id}:{side}")),
            }
        }
        if source_clock_unmapped {
            errors.push(format!("source_trace_clock_mapping_not_qualified:{pair_id}"));
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
        entry["failure_progression"] = Value::Array(vec![Value::String(
            "generic_failure".to_owned(),
        )]);
        assert!(
            cross_file_invariants(&stage, &corpus, &oracle)
                .iter()
                .any(|error| error == "target_unmapped_clock_not_reflected_as_temporal_unknown:MIP-005")
        );
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
