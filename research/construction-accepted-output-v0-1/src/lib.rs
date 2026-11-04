use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn u64_at(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_u64()
}

fn inspection_outcome(attempt: &Value) -> Option<&str> {
    string_at(attempt, &["inspection", "outcome"])
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn add_error(errors: &mut Vec<String>, code: impl Into<String>) {
    errors.push(code.into());
}

/// Validates JSON Schema and semantic cross-field constraints.
///
/// These checks establish structural and ledger consistency only. They do not
/// authenticate evidence bytes, inspector authority, calibration, physical
/// measurement truth, independence, or safety qualification.
pub fn validate_ledger(schema: &Value, ledger: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let validator = match jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(_) => return vec!["accepted_output_schema_could_not_be_compiled".to_owned()],
    };

    for error in validator.iter_errors(ledger) {
        errors.push(format!("schema_violation:{}", error.instance_path()));
    }

    let package_start = u64_at(ledger, &["work_package", "started_at_unix_ms"]);
    let package_end = u64_at(ledger, &["work_package", "ended_at_unix_ms"]);
    if !matches!((package_start, package_end), (Some(start), Some(end)) if start < end) {
        add_error(&mut errors, "work_package_interval_invalid");
    }

    let units = ledger.get("work_units").and_then(Value::as_array);
    let mut unit_ids = BTreeSet::new();
    let mut attempt_ids = BTreeSet::new();
    let mut defect_ids = BTreeSet::new();

    if let Some(units) = units {
        for unit in units {
            let unit_id = unit
                .get("unit_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if !unit_ids.insert(unit_id.to_owned()) {
                add_error(&mut errors, format!("duplicate_work_unit_id:{unit_id}"));
            }

            let attempts = unit
                .get("attempts")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let mut latest_outcome = None;
            let mut previous_attempt_end = None;

            for (index, attempt) in attempts.iter().enumerate() {
                let attempt_id = attempt
                    .get("attempt_id")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                if !attempt_ids.insert(attempt_id.to_owned()) {
                    add_error(&mut errors, format!("duplicate_attempt_id:{attempt_id}"));
                }

                let expected_number = u64::try_from(index + 1).unwrap_or(u64::MAX);
                let attempt_number = attempt.get("attempt_number").and_then(Value::as_u64);
                if attempt_number != Some(expected_number) {
                    add_error(
                        &mut errors,
                        format!("attempt_sequence_not_contiguous:{unit_id}:{attempt_id}"),
                    );
                }

                let start = attempt.get("started_at_unix_ms").and_then(Value::as_u64);
                let end = attempt.get("ended_at_unix_ms").and_then(Value::as_u64);
                if !matches!((start, end), (Some(start), Some(end)) if start < end) {
                    add_error(
                        &mut errors,
                        format!("attempt_interval_invalid:{attempt_id}"),
                    );
                }
                if let (Some(start), Some(end)) = (start, end) {
                    if previous_attempt_end.is_some_and(|previous_end| start < previous_end) {
                        add_error(
                            &mut errors,
                            format!("attempt_intervals_not_sequential:{unit_id}:{attempt_id}"),
                        );
                    }
                    previous_attempt_end = Some(end);
                }
                if let (Some(start), Some(end), Some(package_start), Some(package_end)) =
                    (start, end, package_start, package_end)
                    && (start < package_start || end > package_end)
                {
                    add_error(
                        &mut errors,
                        format!("attempt_outside_work_package:{attempt_id}"),
                    );
                }

                if let Some(digest) = attempt
                    .get("output_artifact_digest")
                    .and_then(Value::as_str)
                    && !is_digest(digest)
                {
                    add_error(
                        &mut errors,
                        format!("output_artifact_digest_not_canonical:{attempt_id}"),
                    );
                }

                let inspection = attempt.get("inspection");
                let outcome = inspection
                    .and_then(|value| value.get("outcome"))
                    .and_then(Value::as_str);
                latest_outcome = outcome;

                let authority = inspection
                    .and_then(|value| value.get("acceptance_authority_ref"))
                    .and_then(Value::as_str);
                let method = inspection
                    .and_then(|value| value.get("method_ref"))
                    .and_then(Value::as_str);
                let evidence = inspection
                    .and_then(|value| value.get("evidence_refs"))
                    .and_then(Value::as_array);

                match outcome {
                    Some("accepted" | "rejected") => {
                        if authority.is_none_or(str::is_empty) {
                            add_error(
                                &mut errors,
                                format!("inspection_authority_missing:{attempt_id}"),
                            );
                        }
                        if method.is_none_or(str::is_empty) {
                            add_error(
                                &mut errors,
                                format!("inspection_method_missing:{attempt_id}"),
                            );
                        }
                        if evidence.is_none_or(Vec::is_empty) {
                            add_error(
                                &mut errors,
                                format!("inspection_evidence_missing:{attempt_id}"),
                            );
                        }
                    }
                    Some("not_inspected") => {
                        if authority.is_some()
                            || method.is_some()
                            || evidence.is_some_and(|items| !items.is_empty())
                        {
                            add_error(
                                &mut errors,
                                format!(
                                    "uninspected_attempt_carries_acceptance_evidence:{attempt_id}"
                                ),
                            );
                        }
                    }
                    _ => add_error(
                        &mut errors,
                        format!("inspection_outcome_missing:{attempt_id}"),
                    ),
                }

                if let Some(defects) = inspection
                    .and_then(|value| value.get("defects"))
                    .and_then(Value::as_array)
                {
                    for defect in defects {
                        let defect_id = defect
                            .get("defect_id")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown");
                        if !defect_ids.insert(defect_id.to_owned()) {
                            add_error(&mut errors, format!("duplicate_defect_id:{defect_id}"));
                        }
                    }
                }
            }

            let terminal = unit
                .get("terminal_disposition")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            match terminal {
                "accepted" => {
                    if latest_outcome != Some("accepted") {
                        add_error(
                            &mut errors,
                            format!(
                                "accepted_disposition_without_latest_accepted_inspection:{unit_id}"
                            ),
                        );
                    }
                    let has_open_critical = attempts.iter().any(|attempt| {
                        attempt
                            .get("inspection")
                            .and_then(|value| value.get("defects"))
                            .and_then(Value::as_array)
                            .is_some_and(|defects| {
                                defects.iter().any(|defect| {
                                    defect.get("severity").and_then(Value::as_str)
                                        == Some("critical")
                                        && defect.get("disposition").and_then(Value::as_str)
                                            == Some("open")
                                })
                            })
                    });
                    if has_open_critical {
                        add_error(
                            &mut errors,
                            format!("accepted_unit_has_open_critical_defect:{unit_id}"),
                        );
                    }
                }
                "rejected" if latest_outcome != Some("rejected") => {
                    add_error(
                        &mut errors,
                        format!(
                            "rejected_disposition_without_latest_rejected_inspection:{unit_id}"
                        ),
                    );
                }
                "unresolved" if !attempts.is_empty() && latest_outcome != Some("not_inspected") => {
                    add_error(
                        &mut errors,
                        format!(
                            "unresolved_disposition_conflicts_with_latest_inspection:{unit_id}"
                        ),
                    );
                }
                _ => {}
            }
        }
    }

    let entries = ledger.get("time_entries").and_then(Value::as_array);
    let mut entry_ids = BTreeSet::new();
    let mut intervals: BTreeMap<String, Vec<(u64, u64, String)>> = BTreeMap::new();
    if let Some(entries) = entries {
        for entry in entries {
            let entry_id = entry
                .get("entry_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if !entry_ids.insert(entry_id.to_owned()) {
                add_error(&mut errors, format!("duplicate_time_entry_id:{entry_id}"));
            }
            let resource_id = entry
                .get("resource_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let start = entry.get("start_at_unix_ms").and_then(Value::as_u64);
            let end = entry.get("end_at_unix_ms").and_then(Value::as_u64);
            let package_clock = string_at(ledger, &["work_package", "clock_domain_id"]);
            let entry_clock = entry.get("clock_domain_id").and_then(Value::as_str);
            if package_clock.is_some() && entry_clock != package_clock {
                add_error(
                    &mut errors,
                    format!("time_entry_clock_domain_mismatch:{entry_id}"),
                );
            }
            if !matches!((start, end), (Some(start), Some(end)) if start < end) {
                add_error(
                    &mut errors,
                    format!("time_entry_interval_invalid:{entry_id}"),
                );
                continue;
            }
            if let (Some(start), Some(end), Some(package_start), Some(package_end)) =
                (start, end, package_start, package_end)
                && (start < package_start || end > package_end)
            {
                add_error(
                    &mut errors,
                    format!("time_entry_outside_accounting_window:{entry_id}"),
                );
            }
            intervals.entry(resource_id.to_owned()).or_default().push((
                start.unwrap_or(0),
                end.unwrap_or(0),
                entry_id.to_owned(),
            ));
        }
    }

    for (resource_id, resource_intervals) in &mut intervals {
        resource_intervals.sort_by_key(|interval| (interval.0, interval.1));
        for pair in resource_intervals.windows(2) {
            if pair[1].0 < pair[0].1 {
                add_error(
                    &mut errors,
                    format!(
                        "resource_time_overlap:{resource_id}:{}:{}",
                        pair[0].2, pair[1].2
                    ),
                );
            }
        }
    }

    if string_at(ledger, &["work_package", "completion_state"]) == Some("completed")
        && units.is_some_and(|units| {
            units.iter().any(|unit| {
                matches!(
                    unit.get("terminal_disposition").and_then(Value::as_str),
                    Some("active" | "unresolved")
                )
            })
        })
    {
        add_error(
            &mut errors,
            "completed_work_package_contains_unresolved_units",
        );
    }

    if let Some(interruptions) = ledger.get("interruptions").and_then(Value::as_array) {
        let mut interruption_ids = BTreeSet::new();
        for interruption in interruptions {
            let id = interruption
                .get("interruption_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if !interruption_ids.insert(id.to_owned()) {
                add_error(&mut errors, format!("duplicate_interruption_id:{id}"));
            }
            let start = interruption.get("start_at_unix_ms").and_then(Value::as_u64);
            let end = interruption.get("end_at_unix_ms").and_then(Value::as_u64);
            if !matches!((start, end), (Some(start), Some(end)) if start < end) {
                add_error(&mut errors, format!("interruption_interval_invalid:{id}"));
            }
            if let (Some(start), Some(end), Some(package_start), Some(package_end)) =
                (start, end, package_start, package_end)
                && (start < package_start || end > package_end)
            {
                add_error(
                    &mut errors,
                    format!("interruption_outside_work_package:{id}"),
                );
            }
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn metric(
    metric_id: &str,
    numerator: f64,
    denominator: f64,
    unit: &str,
    provisional: bool,
) -> Value {
    if denominator > 0.0 {
        json!({
            "metric_id": metric_id,
            "numerator": numerator,
            "denominator": denominator,
            "unit": unit,
            "status": if provisional { "provisional" } else { "estimable" },
            "value": numerator / denominator
        })
    } else {
        json!({
            "metric_id": metric_id,
            "numerator": numerator,
            "denominator": denominator,
            "unit": unit,
            "status": "not_estimable",
            "value": Value::Null
        })
    }
}

fn seconds_for(entries: &[Value], predicate: impl Fn(&Value) -> bool) -> f64 {
    entries
        .iter()
        .filter(|entry| predicate(entry))
        .filter_map(|entry| {
            let start = entry.get("start_at_unix_ms")?.as_u64()?;
            let end = entry.get("end_at_unix_ms")?.as_u64()?;
            (end > start).then_some((end - start) as f64 / 1000.0)
        })
        .sum()
}

/// Derives an unweighted, per-functional-unit report from a validated ledger.
/// The source digest is SHA-256 over serde_json's deterministic Value encoding.
/// This hash proves byte identity of the serialized input, not evidence truth.
pub fn derive_report(ledger: &Value) -> Value {
    let units = ledger
        .get("work_units")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let entries = ledger
        .get("time_entries")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let planned = units.len() as u64;
    let attempted_units = units
        .iter()
        .filter(|unit| {
            unit.get("attempts")
                .and_then(Value::as_array)
                .is_some_and(|v| !v.is_empty())
        })
        .count() as u64;
    let first_pass_inspected = units
        .iter()
        .filter(|unit| {
            unit.get("attempts")
                .and_then(Value::as_array)
                .and_then(|attempts| attempts.first())
                .and_then(inspection_outcome)
                .is_some_and(|outcome| matches!(outcome, "accepted" | "rejected"))
        })
        .count() as u64;
    let first_pass_accepted = units
        .iter()
        .filter(|unit| {
            unit.get("attempts")
                .and_then(Value::as_array)
                .and_then(|attempts| attempts.first())
                .and_then(inspection_outcome)
                == Some("accepted")
        })
        .count() as u64;
    let finally_accepted = units
        .iter()
        .filter(|unit| unit.get("terminal_disposition").and_then(Value::as_str) == Some("accepted"))
        .count() as u64;
    let finally_rejected = units
        .iter()
        .filter(|unit| unit.get("terminal_disposition").and_then(Value::as_str) == Some("rejected"))
        .count() as u64;
    let not_attempted = units
        .iter()
        .filter(|unit| {
            unit.get("attempts")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
        })
        .count() as u64;
    let uninspected = units
        .iter()
        .filter(|unit| {
            unit.get("attempts")
                .and_then(Value::as_array)
                .is_some_and(|attempts| {
                    !attempts.is_empty()
                        && attempts
                            .last()
                            .and_then(inspection_outcome)
                            .is_none_or(|outcome| outcome == "not_inspected")
                })
        })
        .count() as u64;
    let cancelled_units = units
        .iter()
        .filter(|unit| {
            unit.get("terminal_disposition").and_then(Value::as_str) == Some("cancelled")
        })
        .count() as u64;
    let superseded_units = units
        .iter()
        .filter(|unit| {
            unit.get("terminal_disposition").and_then(Value::as_str) == Some("superseded")
        })
        .count() as u64;
    let unresolved_units = units
        .iter()
        .filter(|unit| {
            unit.get("terminal_disposition").and_then(Value::as_str) == Some("unresolved")
        })
        .count() as u64;
    let active_units = units
        .iter()
        .filter(|unit| unit.get("terminal_disposition").and_then(Value::as_str) == Some("active"))
        .count() as u64;
    let rework_attempts = units
        .iter()
        .map(|unit| {
            unit.get("attempts")
                .and_then(Value::as_array)
                .map(|attempts| attempts.len().saturating_sub(1) as u64)
                .unwrap_or(0)
        })
        .sum::<u64>();

    let critical_planned_units = units
        .iter()
        .filter(|unit| unit.get("criticality").and_then(Value::as_str) == Some("critical"))
        .count() as u64;
    let critical_inspected_units = units
        .iter()
        .filter(|unit| {
            unit.get("criticality").and_then(Value::as_str) == Some("critical")
                && unit
                    .get("attempts")
                    .and_then(Value::as_array)
                    .is_some_and(|attempts| {
                        attempts.iter().any(|attempt| {
                            inspection_outcome(attempt)
                                .is_some_and(|outcome| matches!(outcome, "accepted" | "rejected"))
                        })
                    })
        })
        .count() as u64;
    let critical_first_pass_inspected_units = units
        .iter()
        .filter(|unit| {
            unit.get("criticality").and_then(Value::as_str) == Some("critical")
                && unit
                    .get("attempts")
                    .and_then(Value::as_array)
                    .and_then(|attempts| attempts.first())
                    .and_then(inspection_outcome)
                    .is_some_and(|outcome| matches!(outcome, "accepted" | "rejected"))
        })
        .count() as u64;

    let mut first_pass_critical_defects = 0_u64;
    let mut first_pass_major_defects = 0_u64;
    let mut first_pass_minor_defects = 0_u64;
    let mut first_pass_unknown_severity_defects = 0_u64;
    for unit in units {
        if let Some(first_attempt) = unit
            .get("attempts")
            .and_then(Value::as_array)
            .and_then(|attempts| attempts.first())
            && let Some(defects) = first_attempt
                .get("inspection")
                .and_then(|inspection| inspection.get("defects"))
                .and_then(Value::as_array)
        {
            for defect in defects {
                match defect.get("severity").and_then(Value::as_str) {
                    Some("critical") => first_pass_critical_defects += 1,
                    Some("major") => first_pass_major_defects += 1,
                    Some("minor") => first_pass_minor_defects += 1,
                    _ => first_pass_unknown_severity_defects += 1,
                }
            }
        }
    }

    let robot_seconds = seconds_for(entries, |entry| {
        entry.get("resource_type").and_then(Value::as_str) == Some("robot")
    });
    let total_human_seconds = seconds_for(entries, |entry| {
        entry.get("resource_type").and_then(Value::as_str) == Some("human")
    });
    let direct_categories = [
        "survey_control",
        "robot_setup_calibration",
        "operator_supervision",
        "spotting",
        "execution",
        "charging_changeover",
        "waiting",
        "qa_inspection",
        "rework_reprint",
        "interruption_recovery",
        "site_coordination_requalification",
    ];
    let direct_human_seconds = seconds_for(entries, |entry| {
        entry.get("resource_type").and_then(Value::as_str) == Some("human")
            && entry
                .get("activity_category")
                .and_then(Value::as_str)
                .is_some_and(|category| direct_categories.contains(&category))
    });
    let rework_human_seconds = seconds_for(entries, |entry| {
        entry.get("resource_type").and_then(Value::as_str) == Some("human")
            && entry.get("activity_category").and_then(Value::as_str) == Some("rework_reprint")
    });

    let elapsed_seconds = match (
        u64_at(ledger, &["work_package", "started_at_unix_ms"]),
        u64_at(ledger, &["work_package", "ended_at_unix_ms"]),
    ) {
        (Some(start), Some(end)) if end > start => (end - start) as f64 / 1000.0,
        _ => 0.0,
    };

    let source_bytes = serde_json::to_vec(ledger).unwrap_or_default();
    let source_digest = format!("{:x}", Sha256::digest(source_bytes));
    let profile_id = string_at(ledger, &["profile_id"]).unwrap_or("unknown");
    let unit_class = string_at(ledger, &["functional_unit", "unit_class"]).unwrap_or("unknown");
    let accepted_unit = format!("{unit_class}_per_hour");
    let provisional = string_at(ledger, &["work_package", "completion_state"]) != Some("completed");

    let metrics = vec![
        metric(
            "first_pass_yield",
            first_pass_accepted as f64,
            first_pass_inspected as f64,
            "ratio",
            provisional,
        ),
        metric(
            "critical_feature_inspection_coverage",
            critical_inspected_units as f64,
            critical_planned_units as f64,
            "ratio",
            provisional,
        ),
        metric(
            "critical_first_pass_inspection_coverage",
            critical_first_pass_inspected_units as f64,
            critical_planned_units as f64,
            "ratio",
            provisional,
        ),
        metric(
            "first_pass_critical_defects_per_first_pass_inspected_unit",
            first_pass_critical_defects as f64,
            first_pass_inspected as f64,
            "defects_per_inspected_unit",
            provisional,
        ),
        metric(
            "first_pass_major_defects_per_first_pass_inspected_unit",
            first_pass_major_defects as f64,
            first_pass_inspected as f64,
            "defects_per_inspected_unit",
            provisional,
        ),
        metric(
            "first_pass_minor_defects_per_first_pass_inspected_unit",
            first_pass_minor_defects as f64,
            first_pass_inspected as f64,
            "defects_per_inspected_unit",
            provisional,
        ),
        metric(
            "final_acceptance_fraction",
            finally_accepted as f64,
            attempted_units as f64,
            "ratio",
            provisional,
        ),
        metric(
            "accepted_units_per_robot_hour",
            finally_accepted as f64,
            robot_seconds / 3600.0,
            &accepted_unit,
            provisional,
        ),
        metric(
            "accepted_units_per_direct_crew_hour",
            finally_accepted as f64,
            direct_human_seconds / 3600.0,
            &accepted_unit,
            provisional,
        ),
        metric(
            "accepted_units_per_total_workflow_crew_hour",
            finally_accepted as f64,
            total_human_seconds / 3600.0,
            &accepted_unit,
            provisional,
        ),
        metric(
            "elapsed_seconds_per_finally_accepted_unit",
            elapsed_seconds,
            finally_accepted as f64,
            "seconds_per_accepted_unit",
            provisional,
        ),
        metric(
            "rework_human_hours_per_finally_accepted_unit",
            rework_human_seconds / 3600.0,
            finally_accepted as f64,
            "human_hours_per_accepted_unit",
            provisional,
        ),
    ];

    json!({
        "schema_version": "construction-accepted-output-report-v0.1",
        "source_record_digest": source_digest,
        "profile_id": profile_id,
        "unit_class": unit_class,
        "derived_counts": {
            "planned_units": planned,
            "attempted_units": attempted_units,
            "first_pass_inspected_units": first_pass_inspected,
            "first_pass_accepted_units": first_pass_accepted,
            "finally_accepted_units": finally_accepted,
            "finally_rejected_units": finally_rejected,
            "not_attempted_units": not_attempted,
            "uninspected_units": uninspected,
            "cancelled_units": cancelled_units,
            "superseded_units": superseded_units,
            "unresolved_units": unresolved_units,
            "active_units": active_units,
            "rework_attempts": rework_attempts,
            "critical_planned_units": critical_planned_units,
            "critical_inspected_units": critical_inspected_units,
            "critical_first_pass_inspected_units": critical_first_pass_inspected_units,
            "first_pass_critical_defects": first_pass_critical_defects,
            "first_pass_major_defects": first_pass_major_defects,
            "first_pass_minor_defects": first_pass_minor_defects,
            "first_pass_unknown_severity_defects": first_pass_unknown_severity_defects
        },
        "metrics": metrics,
        "limitations": [
            "The report is derived from a synthetic or supplied ledger; it does not authenticate raw evidence or inspector authority.",
            "Rates are valid only for the declared functional unit, task scope, acceptance profile and accounting window.",
            "Safety qualification is external and non-compensable.",
            "A zero denominator is reported as not_estimable, never as zero productivity or perfect yield."
        ]
    })
}

/// Checks that a ledger uses a unit class explicitly declared by its task profile.
/// This is profile binding, not authentication of the profile publisher or digest.
pub fn validate_profile_binding(profile: &Value, ledger: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let profile_id = profile.get("profile_id").and_then(Value::as_str);
    let ledger_profile_id = string_at(ledger, &["profile_id"]);
    let acceptance_profile_id = string_at(ledger, &["functional_unit", "acceptance_profile_id"]);
    let unit_class = string_at(ledger, &["functional_unit", "unit_class"]);
    let allowed = profile
        .get("allowed_unit_classes")
        .and_then(Value::as_array)
        .is_some_and(|classes| {
            classes
                .iter()
                .any(|item| item.get("unit_class").and_then(Value::as_str) == unit_class)
        });

    if profile_id.is_none()
        || ledger_profile_id != profile_id
        || acceptance_profile_id != profile_id
    {
        errors.push("ledger_profile_identity_mismatch".to_owned());
    }
    if unit_class.is_none() || !allowed {
        errors.push("functional_unit_class_not_allowed_by_profile".to_owned());
    }
    errors
}

/// Checks a report against the machine-readable report schema.
pub fn validate_report(schema: &Value, report: &Value) -> Vec<String> {
    let validator = match jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(_) => return vec!["accepted_output_report_schema_could_not_be_compiled".to_owned()],
    };
    let mut errors = validator
        .iter_errors(report)
        .map(|error| format!("schema_violation:{}", error.instance_path()))
        .collect::<Vec<_>>();
    errors.sort();
    errors.dedup();
    errors
}

pub fn schema_is_valid(schema: &Value) -> bool {
    jsonschema::draft202012::meta::is_valid(schema)
}

pub fn read_json(path: &std::path::Path) -> Result<Value, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot_read_file:{}:{error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("invalid_json:{}:{error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn package_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn load(relative: &str) -> Value {
        read_json(&package_path(relative)).expect("fixture/schema JSON must parse")
    }

    fn ledger_schema() -> Value {
        load("schema/accepted-output-ledger-v0.1.schema.json")
    }

    fn report_schema() -> Value {
        load("schema/accepted-output-report-v0.1.schema.json")
    }

    #[test]
    fn schemas_validate_against_draft_2020_12() {
        assert!(schema_is_valid(&ledger_schema()));
        assert!(schema_is_valid(&report_schema()));
    }

    #[test]
    fn synthetic_valid_fixture_passes_schema_and_semantic_checks() {
        let errors = validate_ledger(
            &ledger_schema(),
            &load("fixtures/layout-run.synthetic.json"),
        );
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn uninspected_units_cannot_be_labelled_accepted() {
        let errors = validate_ledger(
            &ledger_schema(),
            &load("fixtures/invalid/uninspected-output-counted-as-accepted.synthetic.json"),
        );
        assert!(errors.iter().any(|error| {
            error == "accepted_disposition_without_latest_accepted_inspection:p3"
        }));
    }

    #[test]
    fn not_attempted_units_are_not_reported_as_uninspected_outputs() {
        let mut ledger = load("fixtures/layout-run.synthetic.json");
        ledger["work_units"][2]["attempts"] = json!([]);
        ledger["work_units"][2]["terminal_disposition"] = Value::String("superseded".to_owned());
        let report = derive_report(&ledger);
        assert_eq!(report["derived_counts"]["not_attempted_units"], 1);
        assert_eq!(report["derived_counts"]["uninspected_units"], 0);
        assert_eq!(report["derived_counts"]["superseded_units"], 1);
    }

    #[test]
    fn derived_first_pass_yield_excludes_uninspected_units() {
        let report = derive_report(&load("fixtures/layout-run.synthetic.json"));
        let counts = &report["derived_counts"];
        assert_eq!(counts["first_pass_inspected_units"], 2);
        assert_eq!(counts["first_pass_accepted_units"], 1);
        assert_eq!(counts["finally_accepted_units"], 2);
        assert_eq!(counts["uninspected_units"], 1);
    }

    #[test]
    fn completed_work_package_cannot_contain_unresolved_units() {
        let mut ledger = load("fixtures/layout-run.synthetic.json");
        ledger["work_package"]["completion_state"] = Value::String("completed".to_owned());
        let errors = validate_ledger(&ledger_schema(), &ledger);
        assert!(
            errors
                .iter()
                .any(|error| { error == "completed_work_package_contains_unresolved_units" })
        );
    }

    #[test]
    fn critical_inspection_coverage_is_reported_separately() {
        let report = derive_report(&load("fixtures/layout-run.synthetic.json"));
        assert_eq!(report["derived_counts"]["critical_planned_units"], 3);
        assert_eq!(
            report["derived_counts"]["critical_first_pass_inspected_units"],
            2
        );
        let metric = report["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["metric_id"] == "critical_feature_inspection_coverage")
            .unwrap();
        assert_eq!(metric["numerator"].as_f64(), Some(2.0));
        assert_eq!(metric["denominator"].as_f64(), Some(3.0));
    }

    #[test]
    fn critical_inspection_coverage_counts_later_reinspection_separately() {
        let mut ledger = load("fixtures/layout-run.synthetic.json");
        let mut retry = ledger["work_units"][2]["attempts"][0].clone();
        retry["attempt_id"] = Value::String("p3-a2".to_owned());
        retry["attempt_number"] = Value::from(2);
        retry["started_at_unix_ms"] = Value::from(1_600_000_u64);
        retry["ended_at_unix_ms"] = Value::from(1_650_000_u64);
        retry["output_artifact_digest"] = Value::String(
            "5555555555555555555555555555555555555555555555555555555555555555".to_owned(),
        );
        retry["inspection"]["outcome"] = Value::String("accepted".to_owned());
        retry["inspection"]["acceptance_authority_ref"] =
            Value::String("synthetic-authority".to_owned());
        retry["inspection"]["method_ref"] = Value::String("synthetic-survey-method".to_owned());
        retry["inspection"]["evidence_refs"] = json!(["synthetic-inspection-p3-retry"]);
        ledger["work_units"][2]["attempts"]
            .as_array_mut()
            .unwrap()
            .push(retry);
        ledger["work_units"][2]["terminal_disposition"] = Value::String("accepted".to_owned());

        let report = derive_report(&ledger);
        assert_eq!(report["derived_counts"]["critical_inspected_units"], 3);
        assert_eq!(
            report["derived_counts"]["critical_first_pass_inspected_units"],
            2
        );

        let metrics = report["metrics"].as_array().unwrap();
        let all_inspection = metrics
            .iter()
            .find(|metric| metric["metric_id"] == "critical_feature_inspection_coverage")
            .unwrap();
        let first_pass = metrics
            .iter()
            .find(|metric| metric["metric_id"] == "critical_first_pass_inspection_coverage")
            .unwrap();
        assert_eq!(all_inspection["numerator"].as_f64(), Some(3.0));
        assert_eq!(all_inspection["denominator"].as_f64(), Some(3.0));
        assert_eq!(first_pass["numerator"].as_f64(), Some(2.0));
        assert_eq!(first_pass["denominator"].as_f64(), Some(3.0));
    }

    #[test]
    fn derived_report_matches_report_schema() {
        let report = derive_report(&load("fixtures/layout-run.synthetic.json"));
        let errors = validate_report(&report_schema(), &report);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn same_resource_time_intervals_must_not_overlap() {
        let mut ledger = load("fixtures/layout-run.synthetic.json");
        let overlapping = ledger["time_entries"][1].clone();
        ledger["time_entries"]
            .as_array_mut()
            .unwrap()
            .push(overlapping);
        let errors = validate_ledger(&ledger_schema(), &ledger);
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("duplicate_time_entry_id:"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("resource_time_overlap:"))
        );
    }

    #[test]
    fn profile_binding_rejects_unlisted_unit_class() {
        let profile = load("profile/layout-accepted-output-v0.1.json");
        let mut ledger = load("fixtures/layout-run.synthetic.json");
        ledger["functional_unit"]["unit_class"] = Value::String("unlisted_unit".to_owned());
        assert!(
            validate_profile_binding(&profile, &ledger)
                .iter()
                .any(|error| error == "functional_unit_class_not_allowed_by_profile")
        );
    }

    #[test]
    fn incomplete_work_package_metrics_are_provisional() {
        let report = derive_report(&load("fixtures/layout-run.synthetic.json"));
        let metrics = report["metrics"].as_array().unwrap();
        assert!(
            metrics
                .iter()
                .all(|metric| metric["status"] == "provisional")
        );
    }

    #[test]
    fn attempt_intervals_must_not_overlap_or_reverse() {
        let mut ledger = load("fixtures/layout-run.synthetic.json");
        ledger["work_units"][1]["attempts"][1]["started_at_unix_ms"] = Value::from(1190000_u64);
        let errors = validate_ledger(&ledger_schema(), &ledger);
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("attempt_intervals_not_sequential:"))
        );
    }

    #[test]
    fn time_entries_must_use_work_package_clock_domain() {
        let mut ledger = load("fixtures/layout-run.synthetic.json");
        ledger["time_entries"][0]["clock_domain_id"] = Value::String("other-clock".to_owned());
        let errors = validate_ledger(&ledger_schema(), &ledger);
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("time_entry_clock_domain_mismatch:"))
        );
    }

    #[test]
    fn zero_denominators_are_not_estimable_not_zero() {
        let mut ledger = load("fixtures/layout-run.synthetic.json");
        ledger["time_entries"] = json!([]);
        let report = derive_report(&ledger);
        let rate = report["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|metric| metric["metric_id"] == "accepted_units_per_robot_hour")
            .unwrap();
        assert_eq!(rate["status"], "not_estimable");
        assert!(rate["value"].is_null());
    }
}
