// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline, aggregate-only comparison of a predeclared alert threshold grid.
//! This module emits sensitivity tables, never a selected threshold or action.

use super::{
    sha256_hex, valid_commit_sha, validate_dataset_json, AnalysisUnit, EvidenceClass,
    ForecastDataset, ForecastOutcome, ForecastRecord, SplitName, belongs_to_split,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

const POLICY_SCHEMA: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-policy-evaluation-v0.1.schema.json");

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PolicyKind {
    Confirmatory,
    Exploratory,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    registered_at_month: i64,
    threshold_grid_registered_at_month: i64,
    registration_evidence_reference: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Governance {
    responsible_organization_reference: String,
    approval_reference: String,
    authorization_scope: String,
    human_review_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Action {
    action_id: String,
    action_description: String,
    resource_unit: String,
    resource_units_per_alert: u64,
    latency_hours: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Capacity {
    max_alerts_per_target_month: u64,
    max_resource_units_per_target_month: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CostSchedule {
    cost_unit: String,
    utility_if_true_alert_units: i64,
    cost_false_alert_units: i64,
    cost_missed_event_units: i64,
    cost_per_resource_unit_units: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PolicyPrivacy {
    aggregate_only: bool,
    minimum_subgroup_outcomes: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PolicyManifest {
    schema_version: String,
    policy_id: String,
    evidence_class: EvidenceClass,
    dataset_id: String,
    forecast_archive_sha256: String,
    outcome_release_id: String,
    outcome_release_sha256: String,
    analysis_unit: AnalysisUnit,
    lead_months: i64,
    target_definition: String,
    policy_kind: PolicyKind,
    registration: Registration,
    governance: Governance,
    action: Action,
    thresholds: Vec<f64>,
    capacity: Capacity,
    cost_schedule: CostSchedule,
    privacy: PolicyPrivacy,
    #[serde(default)]
    source_references: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfusionCounts {
    true_positive: usize,
    false_positive: usize,
    false_negative: usize,
    true_negative: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThresholdPolicyResult {
    threshold: f64,
    status: String,
    test_forecast_count: usize,
    scored_count: usize,
    unknown_outcome_count: usize,
    unknown_outcome_alert_count: usize,
    event_count: usize,
    no_event_count: usize,
    event_rate: Option<f64>,
    alert_count_including_unknown_outcomes: usize,
    alert_rate_on_scored_records: Option<f64>,
    coverage_on_test_records: Option<f64>,
    confusion: ConfusionCounts,
    resource_units_required: u64,
    maximum_alerts_in_one_target_month: u64,
    maximum_resource_units_in_one_target_month: u64,
    capacity_feasible: bool,
    utility_units: Option<i64>,
    utility_delta_vs_act_on_none_units: Option<i64>,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyEvaluationReceipt {
    receipt_version: String,
    evaluator: String,
    result: String,
    source_commit: String,
    source_commit_sha_valid: bool,
    rustc_version: String,
    cargo_version: String,
    dependency_lock_sha256: String,
    dataset_sha256: String,
    policy_sha256: String,
    dataset_id: String,
    evidence_class: EvidenceClass,
    forecast_archive_sha256: String,
    outcome_release_id: String,
    outcome_release_sha256: String,
    analysis_unit: AnalysisUnit,
    lead_months: i64,
    target_definition: String,
    policy_id: String,
    policy_kind: PolicyKind,
    policy_registration_month: i64,
    threshold_grid_registration_month: i64,
    predeclaration_timing_conforms_to_manifest: bool,
    registration_timestamp_independently_authenticated: bool,
    governance_responsible_organization_reference: String,
    governance_approval_reference: String,
    authorization_scope: String,
    human_review_required: bool,
    action_id: String,
    action_description: String,
    resource_unit: String,
    action_latency_hours: u64,
    thresholds_evaluated: Vec<f64>,
    thresholds_selected_by_evaluator: bool,
    cost_schedule: CostSchedule,
    capacity: Capacity,
    privacy: PolicyPrivacy,
    act_on_none_utility_units: Option<i64>,
    act_on_none_scored_count: usize,
    policy_results: Vec<ThresholdPolicyResult>,
    live_decision_authority: bool,
    causal_effect_estimated: bool,
    execution_claim: String,
    claim_ceiling: String,
}

fn parse_policy(input: &[u8]) -> Result<PolicyManifest, Vec<String>> {
    let schema: Value = serde_json::from_slice(POLICY_SCHEMA)
        .map_err(|error| vec![format!("checked-in policy schema JSON is invalid: {error}")])?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!("Draft 2020-12 policy schema meta-validation failed: {error}")]
    })?;
    let instance: Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("policy manifest JSON is invalid: {error}")])?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| vec![format!("cannot build policy validator: {error}")])?;
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    serde_json::from_value(instance)
        .map_err(|error| vec![format!("cannot decode policy manifest: {error}")])
}

fn checked_utility(
    confusion: &ConfusionCounts,
    resource_units: u64,
    costs: &CostSchedule,
) -> Option<i64> {
    let true_alert_value = i64::try_from(confusion.true_positive)
        .ok()?
        .checked_mul(costs.utility_if_true_alert_units)?;
    let false_alert_cost = i64::try_from(confusion.false_positive)
        .ok()?
        .checked_mul(costs.cost_false_alert_units)?;
    let missed_event_cost = i64::try_from(confusion.false_negative)
        .ok()?
        .checked_mul(costs.cost_missed_event_units)?;
    let resource_cost = i64::try_from(resource_units)
        .ok()?
        .checked_mul(costs.cost_per_resource_unit_units)?;
    true_alert_value
        .checked_sub(false_alert_cost)?
        .checked_sub(missed_event_cost)?
        .checked_sub(resource_cost)
}

fn checked_resource_units(alerts: usize, per_alert: u64) -> Option<u64> {
    u64::try_from(alerts).ok()?.checked_mul(per_alert)
}

fn scores_for_threshold(
    records: &[&ForecastRecord],
    threshold: f64,
    policy: &PolicyManifest,
    act_on_none_utility_units: Option<i64>,
) -> ThresholdPolicyResult {
    let scored: Vec<&ForecastRecord> = records
        .iter()
        .copied()
        .filter(|record| record.outcome != ForecastOutcome::Unknown)
        .collect();
    let unknown: Vec<&ForecastRecord> = records
        .iter()
        .copied()
        .filter(|record| record.outcome == ForecastOutcome::Unknown)
        .collect();
    let events = scored
        .iter()
        .filter(|record| record.outcome == ForecastOutcome::Event)
        .count();
    let no_events = scored.len() - events;

    let mut confusion = ConfusionCounts {
        true_positive: 0,
        false_positive: 0,
        false_negative: 0,
        true_negative: 0,
    };
    let mut alert_count = 0_usize;
    let mut unknown_alert_count = 0_usize;
    let mut monthly_demand: BTreeMap<i64, (u64, u64)> = BTreeMap::new();

    for record in records {
        let alerted = record.probability >= threshold;
        if alerted {
            alert_count += 1;
            if record.outcome == ForecastOutcome::Unknown {
                unknown_alert_count += 1;
            }
            let (alerts, resource_units) = monthly_demand
                .entry(record.target_month)
                .or_insert((0, 0));
            *alerts = alerts.saturating_add(1);
            *resource_units = resource_units
                .saturating_add(policy.action.resource_units_per_alert);
        }
        if record.outcome == ForecastOutcome::Unknown {
            continue;
        }
        match (alerted, record.outcome) {
            (true, ForecastOutcome::Event) => confusion.true_positive += 1,
            (true, ForecastOutcome::NoEvent) => confusion.false_positive += 1,
            (false, ForecastOutcome::Event) => confusion.false_negative += 1,
            (false, ForecastOutcome::NoEvent) => confusion.true_negative += 1,
            (_, ForecastOutcome::Unknown) => {}
        }
    }

    let resource_units_required = checked_resource_units(
        alert_count,
        policy.action.resource_units_per_alert,
    )
    .unwrap_or(u64::MAX);
    let maximum_alerts = monthly_demand
        .values()
        .map(|(alerts, _)| *alerts)
        .max()
        .unwrap_or(0);
    let maximum_monthly_resources = monthly_demand
        .values()
        .map(|(_, resources)| *resources)
        .max()
        .unwrap_or(0);
    let capacity_feasible = maximum_alerts <= policy.capacity.max_alerts_per_target_month
        && maximum_monthly_resources <= policy.capacity.max_resource_units_per_target_month;

    let reason = if scored.is_empty() {
        Some("no mature test outcomes are available for policy evaluation".to_owned())
    } else if !capacity_feasible {
        Some("declared response capacity is exceeded in at least one target month".to_owned())
    } else {
        None
    };
    let status = if scored.is_empty() {
        "not_estimable"
    } else if !capacity_feasible {
        "review_required"
    } else {
        "estimated"
    };
    let utility_units = if reason.is_none() {
        checked_utility(&confusion, resource_units_required, &policy.cost_schedule)
    } else {
        None
    };
    let utility_delta = match (utility_units, act_on_none_utility_units) {
        (Some(value), Some(baseline)) => value.checked_sub(baseline),
        _ => None,
    };

    ThresholdPolicyResult {
        threshold,
        status: if utility_units.is_none() && reason.is_none() {
            "review_required".to_owned()
        } else {
            status.to_owned()
        },
        test_forecast_count: records.len(),
        scored_count: scored.len(),
        unknown_outcome_count: unknown.len(),
        unknown_outcome_alert_count: unknown_alert_count,
        event_count: events,
        no_event_count: no_events,
        event_rate: if scored.is_empty() {
            None
        } else {
            Some(events as f64 / scored.len() as f64)
        },
        alert_count_including_unknown_outcomes: alert_count,
        alert_rate_on_scored_records: if scored.is_empty() {
            None
        } else {
            Some(
                (confusion.true_positive + confusion.false_positive) as f64
                    / scored.len() as f64,
            )
        },
        coverage_on_test_records: if records.is_empty() {
            None
        } else {
            Some(scored.len() as f64 / records.len() as f64)
        },
        confusion,
        resource_units_required,
        maximum_alerts_in_one_target_month: maximum_alerts,
        maximum_resource_units_in_one_target_month: maximum_monthly_resources,
        capacity_feasible,
        utility_units,
        utility_delta_vs_act_on_none_units: utility_delta,
        reason: reason.or_else(|| {
            utility_units.is_none().then(|| {
                "utility arithmetic overflowed the declared integer unit range".to_owned()
            })
        }),
    }
}

/// Evaluate all registered thresholds exactly as supplied. The first input is
/// a normalized forecast dataset, the second its separate policy manifest.
pub fn evaluate_predeclared_policy(
    dataset_bytes: &[u8],
    policy_bytes: &[u8],
    source_commit: &str,
    rustc_version: &str,
    cargo_version: &str,
    lock_bytes: &[u8],
) -> Result<PolicyEvaluationReceipt, Vec<String>> {
    if source_commit != "UNPINNED_LOCAL_RUN" && !valid_commit_sha(source_commit) {
        return Err(vec![
            "source commit must be a lowercase 40-character Git SHA or UNPINNED_LOCAL_RUN"
                .to_owned(),
        ]);
    }
    let dataset = validate_dataset_json(dataset_bytes)?;
    let policy = parse_policy(policy_bytes)?;

    if policy.schema_version != "0.1" {
        return Err(vec!["policy schema_version must be 0.1".to_owned()]);
    }
    if policy.evidence_class != dataset.evidence_class {
        return Err(vec![
            "policy evidence_class must match the normalized dataset".to_owned(),
        ]);
    }
    if policy.dataset_id != dataset.dataset_id
        || policy.forecast_archive_sha256 != dataset.source.forecast_archive_sha256
        || policy.outcome_release_id != dataset.source.outcome_release_id
        || policy.outcome_release_sha256 != dataset.source.outcome_release_sha256
        || policy.analysis_unit != dataset.source.analysis_unit
        || policy.lead_months != dataset.lead_months
        || policy.target_definition != dataset.source.target_definition
    {
        return Err(vec![
            "policy target, archive, outcome vintage, unit, horizon or dataset ID does not match the normalized dataset".to_owned(),
        ]);
    }
    if !policy.privacy.aggregate_only
        || policy.privacy.minimum_subgroup_outcomes < 2
        || !policy.governance.human_review_required
    {
        return Err(vec![
            "policy requires aggregate-only privacy and human review".to_owned(),
        ]);
    }
    if policy.action.resource_units_per_alert == 0
        || policy.capacity.max_alerts_per_target_month == 0
        || policy.capacity.max_resource_units_per_target_month == 0
    {
        return Err(vec![
            "policy resource use and per-month capacity must be positive".to_owned(),
        ]);
    }
    if policy.thresholds.is_empty()
        || policy.thresholds.iter().any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        || policy.thresholds.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(vec![
            "thresholds must be finite, unique, strictly increasing values in [0, 1]".to_owned(),
        ]);
    }
    if policy.registration.threshold_grid_registered_at_month
        > policy.registration.registered_at_month
    {
        return Err(vec![
            "threshold grid cannot be registered after its parent policy registration".to_owned(),
        ]);
    }
    let test_start = dataset.temporal_split.calibration_end_exclusive_month;
    let registration_precedes_test = policy.registration.registered_at_month < test_start
        && policy.registration.threshold_grid_registered_at_month < test_start;
    if policy.policy_kind == PolicyKind::Confirmatory && !registration_precedes_test {
        return Err(vec![
            "confirmatory policy and complete threshold grid must be registered strictly before the held-out test period".to_owned(),
        ]);
    }
    if policy.evidence_class == EvidenceClass::AuthorizedHistorical
        && [
            policy.registration.registration_evidence_reference.as_str(),
            policy.governance.responsible_organization_reference.as_str(),
            policy.governance.approval_reference.as_str(),
            policy.governance.authorization_scope.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty() || value.starts_with("synthetic:"))
    {
        return Err(vec![
            "authorized historical policy evaluation requires non-synthetic governance and registration references".to_owned(),
        ]);
    }

    let test_records: Vec<&ForecastRecord> = dataset
        .records
        .iter()
        .filter(|record| belongs_to_split(record, &dataset, SplitName::Test))
        .collect();
    let scored_count = test_records
        .iter()
        .filter(|record| record.outcome != ForecastOutcome::Unknown)
        .count();
    let event_count = test_records
        .iter()
        .filter(|record| record.outcome == ForecastOutcome::Event)
        .count();
    let act_on_none_utility_units = (scored_count > 0)
        .then(|| {
            i64::try_from(event_count)
                .ok()?
                .checked_mul(policy.cost_schedule.cost_missed_event_units)?
                .checked_neg()
        })
        .flatten();

    let policy_results: Vec<ThresholdPolicyResult> = policy
        .thresholds
        .iter()
        .map(|threshold| {
            scores_for_threshold(
                &test_records,
                *threshold,
                &policy,
                act_on_none_utility_units,
            )
        })
        .collect();

    let result = if scored_count == 0 {
        "not_estimable"
    } else if policy.policy_kind == PolicyKind::Exploratory {
        "exploratory"
    } else {
        "pass"
    };
    let source_commit_sha_valid = valid_commit_sha(source_commit);
    Ok(PolicyEvaluationReceipt {
        receipt_version: "0.1".to_owned(),
        evaluator: "PIRB predeclared aggregate response-policy evaluation".to_owned(),
        result: result.to_owned(),
        source_commit: source_commit.to_owned(),
        source_commit_sha_valid,
        rustc_version: rustc_version.to_owned(),
        cargo_version: cargo_version.to_owned(),
        dependency_lock_sha256: sha256_hex(lock_bytes),
        dataset_sha256: sha256_hex(dataset_bytes),
        policy_sha256: sha256_hex(policy_bytes),
        dataset_id: dataset.dataset_id,
        evidence_class: dataset.evidence_class,
        forecast_archive_sha256: dataset.source.forecast_archive_sha256,
        outcome_release_id: dataset.source.outcome_release_id,
        outcome_release_sha256: dataset.source.outcome_release_sha256,
        analysis_unit: dataset.source.analysis_unit,
        lead_months: dataset.lead_months,
        target_definition: dataset.source.target_definition,
        policy_id: policy.policy_id,
        policy_kind: policy.policy_kind,
        policy_registration_month: policy.registration.registered_at_month,
        threshold_grid_registration_month: policy.registration.threshold_grid_registered_at_month,
        predeclaration_timing_conforms_to_manifest: registration_precedes_test,
        registration_timestamp_independently_authenticated: false,
        governance_responsible_organization_reference: policy.governance.responsible_organization_reference,
        governance_approval_reference: policy.governance.approval_reference,
        authorization_scope: policy.governance.authorization_scope,
        human_review_required: policy.governance.human_review_required,
        action_id: policy.action.action_id,
        action_description: policy.action.action_description,
        resource_unit: policy.action.resource_unit,
        action_latency_hours: policy.action.latency_hours,
        thresholds_evaluated: policy.thresholds,
        thresholds_selected_by_evaluator: false,
        cost_schedule: policy.cost_schedule,
        capacity: policy.capacity,
        privacy: policy.privacy,
        act_on_none_utility_units,
        act_on_none_scored_count: scored_count,
        policy_results,
        live_decision_authority: false,
        causal_effect_estimated: false,
        execution_claim: "aggregate_hypothetical_response_policy_sensitivity_only".to_owned(),
        claim_ceiling: "A passing receipt reports aggregate retrospective policy arithmetic under the declared cost and capacity assumptions. It does not authenticate policy authority or registration time, establish that a service was delivered, show that any response caused less harm, select an operating threshold, or authorize live warnings, targeting, dispatch, aid denial or other action.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATASET: &[u8] =
        include_bytes!("../../../docs/fixtures/pirb-probability-forecast-v0.1.json");
    const POLICY: &[u8] =
        include_bytes!("../../../docs/fixtures/pirb-policy-evaluation-synthetic-v0.1.json");
    const LOCK: &[u8] = b"synthetic-policy-lock";

    fn evaluate() -> PolicyEvaluationReceipt {
        evaluate_predeclared_policy(
            DATASET,
            POLICY,
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        )
        .expect("synthetic policy should evaluate")
    }

    #[test]
    fn emits_every_predeclared_threshold_and_never_selects_one() {
        let receipt = evaluate();
        assert_eq!(receipt.result, "pass");
        assert_eq!(receipt.policy_results.len(), 3);
        assert_eq!(receipt.thresholds_evaluated, vec![0.2, 0.4, 0.6]);
        assert!(!receipt.thresholds_selected_by_evaluator);
        assert!(!receipt.live_decision_authority);
        assert!(!receipt.causal_effect_estimated);
        assert!(!receipt.registration_timestamp_independently_authenticated);
    }

    #[test]
    fn exact_confusion_and_utility_arithmetic_matches_hand_calculated_oracle() {
        let receipt = evaluate();
        let at_six = receipt
            .policy_results
            .iter()
            .find(|row| (row.threshold - 0.6).abs() < f64::EPSILON)
            .unwrap();
        assert_eq!(at_six.confusion.true_positive, 1);
        assert_eq!(at_six.confusion.false_positive, 1);
        assert_eq!(at_six.confusion.false_negative, 1);
        assert_eq!(at_six.confusion.true_negative, 2);
        assert_eq!(at_six.unknown_outcome_count, 1);
        assert_eq!(at_six.unknown_outcome_alert_count, 0);
        assert_eq!(at_six.utility_units, Some(-80));
        assert_eq!(at_six.utility_delta_vs_act_on_none_units, Some(220));
    }

    #[test]
    fn low_threshold_counts_unknown_outcome_alerts_in_resource_demand() {
        let receipt = evaluate();
        let low = &receipt.policy_results[0];
        assert_eq!(low.unknown_outcome_alert_count, 1);
        assert_eq!(low.alert_count_including_unknown_outcomes, 6);
        assert_eq!(low.resource_units_required, 6);
        assert!(low.capacity_feasible);
    }

    #[test]
    fn confirmatory_policy_registered_after_test_start_is_rejected() {
        let mut value: Value = serde_json::from_slice(POLICY).unwrap();
        value["registration"]["registered_at_month"] = Value::from(120);
        value["registration"]["threshold_grid_registered_at_month"] = Value::from(120);
        let bytes = serde_json::to_vec(&value).unwrap();
        let result = evaluate_predeclared_policy(
            DATASET,
            &bytes,
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        );
        assert!(result
            .unwrap_err()
            .iter()
            .any(|error| error.contains("registered strictly before")));
    }

    #[test]
    fn wrong_outcome_vintage_or_forecast_archive_is_rejected() {
        let mut value: Value = serde_json::from_slice(POLICY).unwrap();
        value["forecast_archive_sha256"] = Value::String("c".repeat(64));
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(evaluate_predeclared_policy(
            DATASET,
            &bytes,
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        ).is_err());
    }

    #[test]
    fn malformed_cost_schedule_is_rejected_by_policy_schema() {
        let mut value: Value = serde_json::from_slice(POLICY).unwrap();
        value["cost_schedule"].as_object_mut().unwrap().remove("cost_false_alert_units");
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(evaluate_predeclared_policy(
            DATASET,
            &bytes,
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        ).is_err());
    }
}
