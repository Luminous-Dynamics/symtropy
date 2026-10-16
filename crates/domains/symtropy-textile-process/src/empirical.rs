// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed ingestion and fail-closed cross-field validation for empirical textile
//! trials. Structural JSON Schema validation remains a separate validation layer:
//! this module enforces the trial's arithmetic and pairing invariants after parse.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

/// Versioned top-level manifest for a paired policy evaluation.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Trial {
    pub protocol_id: String,
    pub version: String,
    pub trial_id: String,
    pub status: TrialStatus,
    pub frozen_at: String,
    pub design: TrialDesign,
    pub baseline_policy_id: String,
    pub comparison_policy_id: String,
    pub primary_endpoint: String,
    pub target_effect: TargetEffect,
    pub guardrails: Guardrails,
    pub profile_strata: Vec<String>,
    pub observation_window_days: u32,
    pub synthetic_only: bool,
    pub interpretation_boundary: String,
    pub records: Vec<CohortRecord>,
}

/// Protocol lifecycle state. It does not by itself prove a study was preregistered.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrialStatus {
    Proposed,
    Preregistered,
    InProgress,
    Completed,
}

/// The assignment design declared in the preregistration.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrialDesign {
    RandomizedBlocked,
    ClusterCrossover,
    MatchedObservational,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TargetEffect {
    pub direction: String,
    pub relative_reduction_bps: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Guardrails {
    pub max_on_time_regression_bps: u16,
    pub max_quality_acceptance_regression_bps: u16,
    pub max_stockout_increase_units: u64,
}

/// One paired cohort/period record. Counts are physical garment or service units,
/// not environmental impact scores.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CohortRecord {
    pub record_id: String,
    pub pair_id: String,
    pub cluster_id: String,
    pub profile_id: String,
    pub functional_unit_id: String,
    pub assigned_policy_id: String,
    pub actual_policy_id: String,
    pub confirmed_order_units: u64,
    pub frozen_forecast_units: u64,
    pub realized_uncancelled_demand_units: u64,
    pub planned_new_units: u64,
    pub completed_new_units: u64,
    pub accepted_new_units: u64,
    pub rejected_new_units: u64,
    pub fulfilled_from_new_units: u64,
    pub fulfilled_from_existing_units: u64,
    pub unsold_new_units_end_of_window: u64,
    pub other_accepted_new_fates: u64,
    pub eligible_existing_inventory_units: u64,
    pub existing_inventory_issued_units: u64,
    pub existing_inventory_remaining_units: u64,
    pub other_existing_inventory_fates: u64,
    pub on_time_fulfilled_units: u64,
    pub late_fulfilled_units: u64,
    pub stockout_units: u64,
    pub window_days: u32,
    pub resource_measurements: ResourceMeasurements,
    pub evidence: EvidenceRefs,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourceMeasurements {
    pub energy_kwh: Measurement,
    pub water_litres: Measurement,
    pub material_kg: Measurement,
    pub direct_labor_hours: Measurement,
}

/// Missing and not-applicable measurements remain distinct from a measured zero.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Measurement {
    Observed {
        value: f64,
        evidence_refs: Vec<String>,
    },
    Missing {
        reason: String,
    },
    NotApplicable {
        reason: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRefs {
    pub assignment_ref: String,
    pub order_snapshot_ref: String,
    pub production_ledger_ref: String,
    pub quality_record_ref: String,
    pub demand_closeout_ref: String,
    pub inventory_issue_receipt_ref: Option<String>,
}

/// All validation findings are stable, human-readable paths/messages. Validation
/// fails closed: a report is returned only when every hard invariant is satisfied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationErrors {
    pub errors: Vec<String>,
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "empirical textile trial failed validation")?;
        for error in &self.errors {
            write!(f, "\n- {error}")?;
        }
        Ok(())
    }
}

impl Error for ValidationErrors {}

/// Parse JSON to a typed trial and validate cross-field, pairing, and evidence-shape
/// invariants. This is not a replacement for independent JSON Schema validation or
/// authentication of the evidence referenced by the document.
pub fn validate_trial_json(input: &str) -> Result<Trial, ValidationErrors> {
    let trial: Trial = serde_json::from_str(input).map_err(|error| ValidationErrors {
        errors: vec![format!("$: JSON structure/type error: {error}")],
    })?;
    validate_trial(&trial)?;
    Ok(trial)
}

/// Validate a typed trial without treating missing measurements as zero.
pub fn validate_trial(trial: &Trial) -> Result<(), ValidationErrors> {
    let mut errors = Vec::new();

    require_text(&mut errors, "$.protocol_id", &trial.protocol_id);
    require_text(&mut errors, "$.trial_id", &trial.trial_id);
    require_text(&mut errors, "$.baseline_policy_id", &trial.baseline_policy_id);
    require_text(
        &mut errors,
        "$.comparison_policy_id",
        &trial.comparison_policy_id,
    );
    require_text(
        &mut errors,
        "$.interpretation_boundary",
        &trial.interpretation_boundary,
    );

    if !is_semver(&trial.version) {
        errors.push("$.version must be a three-part numeric version such as 0.1.0".into());
    }
    if !looks_like_rfc3339(&trial.frozen_at) {
        errors.push("$.frozen_at must be an RFC 3339 timestamp with timezone".into());
    }
    if trial.baseline_policy_id == trial.comparison_policy_id {
        errors.push("$.baseline_policy_id and $.comparison_policy_id must differ".into());
    }
    if trial.primary_endpoint != "unsold_new_per_fulfilled_service_unit" {
        errors.push(
            "$.primary_endpoint must be unsold_new_per_fulfilled_service_unit".into(),
        );
    }
    if trial.target_effect.direction != "reduce"
        || trial.target_effect.relative_reduction_bps == 0
        || trial.target_effect.relative_reduction_bps > 10_000
    {
        errors.push("$.target_effect must declare reduce and 1..=10000 basis points".into());
    }
    if trial.guardrails.max_on_time_regression_bps > 10_000
        || trial.guardrails.max_quality_acceptance_regression_bps > 10_000
    {
        errors.push("$.guardrails regression margins must be in 0..=10000 basis points".into());
    }
    if trial.observation_window_days == 0 {
        errors.push("$.observation_window_days must be positive".into());
    }
    if trial.records.len() < 2 || trial.records.len() > 10_000 {
        errors.push("$.records must contain between 2 and 10000 records".into());
    }
    if trial.profile_strata.is_empty() {
        errors.push("$.profile_strata must not be empty".into());
    }

    let mut strata = BTreeSet::new();
    for (index, profile) in trial.profile_strata.iter().enumerate() {
        if !is_profile_id(profile) {
            errors.push(format!(
                "$.profile_strata[{index}] must use lowercase profile-id characters"
            ));
        }
        if !strata.insert(profile) {
            errors.push(format!("$.profile_strata contains duplicate '{profile}'"));
        }
    }

    let mut record_ids = BTreeSet::new();
    let mut pairs: BTreeMap<&str, Vec<(usize, &CohortRecord)>> = BTreeMap::new();

    for (index, record) in trial.records.iter().enumerate() {
        let path = format!("$.records[{index}]");
        require_text(&mut errors, &format!("{path}.record_id"), &record.record_id);
        require_text(&mut errors, &format!("{path}.pair_id"), &record.pair_id);
        require_text(&mut errors, &format!("{path}.cluster_id"), &record.cluster_id);
        require_text(&mut errors, &format!("{path}.profile_id"), &record.profile_id);
        require_text(
            &mut errors,
            &format!("{path}.functional_unit_id"),
            &record.functional_unit_id,
        );
        require_text(
            &mut errors,
            &format!("{path}.assigned_policy_id"),
            &record.assigned_policy_id,
        );
        require_text(
            &mut errors,
            &format!("{path}.actual_policy_id"),
            &record.actual_policy_id,
        );

        if !is_profile_id(&record.profile_id) {
            errors.push(format!("{path}.profile_id is not a valid profile identifier"));
        }
        if !strata.contains(&record.profile_id) {
            errors.push(format!(
                "{path}.profile_id is absent from $.profile_strata"
            ));
        }
        if record.window_days == 0 {
            errors.push(format!("{path}.window_days must be positive"));
        }
        if record.window_days != trial.observation_window_days {
            errors.push(format!(
                "{path}.window_days differs from $.observation_window_days"
            ));
        }

        if !record_ids.insert(record.record_id.as_str()) {
            errors.push(format!(
                "{path}.record_id duplicates '{}'",
                record.record_id
            ));
        }

        check_sum(
            &mut errors,
            &format!("{path}.completed_new_units"),
            record.completed_new_units,
            &[record.accepted_new_units, record.rejected_new_units],
        );
        if record.completed_new_units > record.planned_new_units {
            errors.push(format!(
                "{path}.completed_new_units exceeds planned_new_units"
            ));
        }
        check_sum(
            &mut errors,
            &format!("{path}.accepted_new_units fates"),
            record.accepted_new_units,
            &[
                record.fulfilled_from_new_units,
                record.unsold_new_units_end_of_window,
                record.other_accepted_new_fates,
            ],
        );
        check_sum(
            &mut errors,
            &format!("{path}.eligible_existing_inventory_units fates"),
            record.eligible_existing_inventory_units,
            &[
                record.existing_inventory_issued_units,
                record.existing_inventory_remaining_units,
                record.other_existing_inventory_fates,
            ],
        );

        let fulfilled = checked_sum(&[
            record.fulfilled_from_new_units,
            record.fulfilled_from_existing_units,
        ]);
        let timed_fulfilled = checked_sum(&[
            record.on_time_fulfilled_units,
            record.late_fulfilled_units,
        ]);
        match (fulfilled, timed_fulfilled) {
            (Some(actual), Some(expected)) if actual == expected => {}
            (Some(_), Some(_)) => errors.push(format!(
                "{path} fulfilled-unit ledger does not equal on-time plus late units"
            )),
            _ => errors.push(format!("{path} fulfilled-unit arithmetic overflows u64")),
        }

        check_sum(
            &mut errors,
            &format!("{path}.realized_uncancelled_demand_units"),
            record.realized_uncancelled_demand_units,
            &[
                record.on_time_fulfilled_units,
                record.late_fulfilled_units,
                record.stockout_units,
            ],
        );

        if record.existing_inventory_issued_units > 0 {
            match &record.evidence.inventory_issue_receipt_ref {
                Some(receipt) if !receipt.trim().is_empty() => {}
                _ if trial.synthetic_only => errors.push(format!(
                    "{path}.evidence.inventory_issue_receipt_ref is required when stock is issued, including in fixtures"
                )),
                _ => errors.push(format!(
                    "{path}.evidence.inventory_issue_receipt_ref is required when existing inventory is issued"
                )),
            }
        }

        validate_evidence(&mut errors, &path, &record.evidence);
        validate_measurement(
            &mut errors,
            &format!("{path}.resource_measurements.energy_kwh"),
            &record.resource_measurements.energy_kwh,
        );
        validate_measurement(
            &mut errors,
            &format!("{path}.resource_measurements.water_litres"),
            &record.resource_measurements.water_litres,
        );
        validate_measurement(
            &mut errors,
            &format!("{path}.resource_measurements.material_kg"),
            &record.resource_measurements.material_kg,
        );
        validate_measurement(
            &mut errors,
            &format!("{path}.resource_measurements.direct_labor_hours"),
            &record.resource_measurements.direct_labor_hours,
        );

        pairs
            .entry(record.pair_id.as_str())
            .or_default()
            .push((index, record));
    }

    for (pair_id, members) in &pairs {
        if members.len() != 2 {
            errors.push(format!(
                "$.records pair_id '{pair_id}' has {} rows; exactly 2 are required for this paired v0.1 protocol",
                members.len()
            ));
            continue;
        }

        let (left_index, left) = members[0];
        let (right_index, right) = members[1];
        let pair_path = format!("pair_id '{pair_id}'");
        if left.profile_id != right.profile_id {
            errors.push(format!("{pair_path} has mismatched profile_id"));
        }
        if left.functional_unit_id != right.functional_unit_id {
            errors.push(format!("{pair_path} has mismatched functional_unit_id"));
        }
        if left.confirmed_order_units != right.confirmed_order_units {
            errors.push(format!("{pair_path} has mismatched frozen confirmed orders"));
        }
        if left.frozen_forecast_units != right.frozen_forecast_units {
            errors.push(format!("{pair_path} has mismatched frozen forecast"));
        }
        if left.realized_uncancelled_demand_units != right.realized_uncancelled_demand_units {
            errors.push(format!("{pair_path} has mismatched realized demand"));
        }

        let assignments = [left.assigned_policy_id.as_str(), right.assigned_policy_id.as_str()];
        let has_baseline = assignments.contains(&trial.baseline_policy_id.as_str());
        let has_comparison = assignments.contains(&trial.comparison_policy_id.as_str());
        if !has_baseline || !has_comparison {
            errors.push(format!(
                "{pair_path} must contain one baseline assignment and one comparison assignment"
            ));
        }
        if left.assigned_policy_id == right.assigned_policy_id {
            errors.push(format!("{pair_path} assigns both records to the same policy"));
        }

        // A paired comparison must not be formed across unrelated service units or
        // differing preregistered periods. Keep source row indices in error paths.
        if left.window_days != right.window_days {
            errors.push(format!(
                "$.records[{left_index}] and $.records[{right_index}] in {pair_path} have different windows"
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort();
        errors.dedup();
        Err(ValidationErrors { errors })
    }
}

fn require_text(errors: &mut Vec<String>, path: &str, value: &str) {
    if value.trim().is_empty() {
        errors.push(format!("{path} must not be empty"));
    }
}

fn check_sum(errors: &mut Vec<String>, path: &str, expected: u64, values: &[u64]) {
    match checked_sum(values) {
        Some(actual) if actual == expected => {}
        Some(actual) => errors.push(format!("{path} ledger mismatch: expected {expected}, accounted {actual}")),
        None => errors.push(format!("{path} ledger arithmetic overflows u64")),
    }
}

fn checked_sum(values: &[u64]) -> Option<u64> {
    values
        .iter()
        .copied()
        .try_fold(0_u64, u64::checked_add)
}

fn validate_evidence(errors: &mut Vec<String>, path: &str, evidence: &EvidenceRefs) {
    for (name, value) in [
        ("assignment_ref", &evidence.assignment_ref),
        ("order_snapshot_ref", &evidence.order_snapshot_ref),
        ("production_ledger_ref", &evidence.production_ledger_ref),
        ("quality_record_ref", &evidence.quality_record_ref),
        ("demand_closeout_ref", &evidence.demand_closeout_ref),
    ] {
        require_text(errors, &format!("{path}.evidence.{name}"), value);
    }
    if let Some(receipt) = &evidence.inventory_issue_receipt_ref {
        require_text(
            errors,
            &format!("{path}.evidence.inventory_issue_receipt_ref"),
            receipt,
        );
    }
}

fn validate_measurement(errors: &mut Vec<String>, path: &str, measurement: &Measurement) {
    match measurement {
        Measurement::Observed { value, evidence_refs } => {
            if !value.is_finite() || *value < 0.0 {
                errors.push(format!("{path}.value must be finite and non-negative"));
            }
            if evidence_refs.is_empty() {
                errors.push(format!("{path}.evidence_refs must not be empty for observed values"));
            }
            for (index, reference) in evidence_refs.iter().enumerate() {
                require_text(
                    errors,
                    &format!("{path}.evidence_refs[{index}]"),
                    reference,
                );
            }
        }
        Measurement::Missing { reason } | Measurement::NotApplicable { reason } => {
            require_text(errors, &format!("{path}.reason"), reason);
        }
    }
}

fn is_semver(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn is_profile_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_lowercase().then_some(true).unwrap_or(false)
            || (!bytes.is_empty() && bytes[0].is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_' || *byte == b'-'
        })
}

fn looks_like_rfc3339(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20 || bytes.get(10) != Some(&b'T') {
        return false;
    }
    let has_time_zone = value.ends_with('Z')
        || (bytes.len() >= 6
            && matches!(bytes[bytes.len() - 6], b'+' | b'-')
            && bytes[bytes.len() - 3] == b':');
    has_time_zone
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str =
        include_str!("../../../../docs/ops/fixtures/textile_empirical_evaluation_v0_1/trial.example.json");

    fn parsed_example() -> serde_json::Value {
        serde_json::from_str(EXAMPLE).expect("committed synthetic example is valid JSON")
    }

    #[test]
    fn committed_synthetic_example_passes_cross_field_validation() {
        let trial = validate_trial_json(EXAMPLE).expect("fixture should be coherent");
        assert!(trial.synthetic_only);
        assert_eq!(trial.records.len(), 2);
    }

    #[test]
    fn broken_production_ledger_fails_closed() {
        let mut value = parsed_example();
        value["records"][0]["accepted_new_units"] = serde_json::json!(77);
        let input = serde_json::to_string(&value).unwrap();
        let err = validate_trial_json(&input).unwrap_err();
        assert!(err.errors.iter().any(|message| {
            message.contains("completed_new_units ledger mismatch")
                || message.contains("accepted_new_units fates ledger mismatch")
        }));
    }

    #[test]
    fn stock_issue_without_receipt_is_rejected() {
        let mut value = parsed_example();
        value["records"][1]["evidence"]["inventory_issue_receipt_ref"] = serde_json::Value::Null;
        let input = serde_json::to_string(&value).unwrap();
        let err = validate_trial_json(&input).unwrap_err();
        assert!(err.errors.iter().any(|message| {
            message.contains("inventory_issue_receipt_ref is required")
        }));
    }

    #[test]
    fn paired_records_must_share_frozen_order_and_forecast_inputs() {
        let mut value = parsed_example();
        value["records"][1]["frozen_forecast_units"] = serde_json::json!(99);
        let input = serde_json::to_string(&value).unwrap();
        let err = validate_trial_json(&input).unwrap_err();
        assert!(err.errors.iter().any(|message| {
            message.contains("mismatched frozen forecast")
        }));
    }

    #[test]
    fn paired_records_must_include_both_policy_assignments() {
        let mut value = parsed_example();
        value["records"][1]["assigned_policy_id"] = serde_json::json!("made_to_order");
        let input = serde_json::to_string(&value).unwrap();
        let err = validate_trial_json(&input).unwrap_err();
        assert!(err.errors.iter().any(|message| {
            message.contains("assigns both records to the same policy")
        }));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value = parsed_example();
        value["unexpected_override"] = serde_json::json!(true);
        let input = serde_json::to_string(&value).unwrap();
        let err = validate_trial_json(&input).unwrap_err();
        assert!(err.errors[0].contains("unknown field"));
    }

    #[test]
    fn missing_meter_is_not_coerced_to_zero() {
        let mut value = parsed_example();
        value["records"][0]["resource_measurements"]["energy_kwh"] =
            serde_json::json!({"status":"missing","reason":"meter offline"});
        let input = serde_json::to_string(&value).unwrap();
        let trial = validate_trial_json(&input).expect("missingness is representable");
        assert!(matches!(
            trial.records[0].resource_measurements.energy_kwh,
            Measurement::Missing { .. }
        ));
    }

    #[test]
    fn observed_measurement_requires_evidence_and_finite_value() {
        let mut value = parsed_example();
        value["records"][0]["resource_measurements"]["energy_kwh"] =
            serde_json::json!({"status":"observed","value":-1.0,"evidence_refs":[]});
        let input = serde_json::to_string(&value).unwrap();
        let err = validate_trial_json(&input).unwrap_err();
        assert!(err.errors.iter().any(|message| message.contains("finite and non-negative")));
        assert!(err.errors.iter().any(|message| message.contains("evidence_refs must not be empty")));
    }

    #[test]
    fn duplicate_record_ids_are_rejected() {
        let mut value = parsed_example();
        value["records"][1]["record_id"] = value["records"][0]["record_id"].clone();
        let input = serde_json::to_string(&value).unwrap();
        let err = validate_trial_json(&input).unwrap_err();
        assert!(err.errors.iter().any(|message| message.contains("record_id duplicates")));
    }

    #[test]
    fn unclosed_demand_ledger_is_rejected() {
        let mut value = parsed_example();
        value["records"][0]["stockout_units"] = serde_json::json!(19);
        let input = serde_json::to_string(&value).unwrap();
        let err = validate_trial_json(&input).unwrap_err();
        assert!(err.errors.iter().any(|message| {
            message.contains("realized_uncancelled_demand_units ledger mismatch")
        }));
    }

    #[test]
    fn unobserved_measurements_remain_explicit_states() {
        let missing = Measurement::Missing {
            reason: "source unavailable".into(),
        };
        let not_applicable = Measurement::NotApplicable {
            reason: "no water input for this operation".into(),
        };
        assert_ne!(missing, not_applicable);
    }
}
