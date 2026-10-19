// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-historical-replay-v0.1.schema.json");

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatasetClass {
    Synthetic,
    AuthorizedHistorical,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Harm,
    NoHarm,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Verified,
    Stale,
    Conflicting,
    Missing,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    NotAlerted,
    NotDispatched,
    Dispatched,
    CompletedUnverified,
    IndependentlyVerifiedCompleted,
    Disputed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Authorization {
    authority_reference: String,
    protocol_reference: String,
    privacy_review_reference: String,
    data_steward_reference: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyPolicy {
    aggregate_only: bool,
    minimum_subgroup_denominator: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalSplit {
    evaluation_start_at: i64,
    training_end_exclusive: i64,
    calibration_end_exclusive: i64,
    test_end_exclusive: i64,
    evaluation_as_of: i64,
    outcome_horizon_seconds: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceReference {
    evidence_id: String,
    available_at: Option<i64>,
    status: EvidenceStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseRecord {
    status: ResponseStatus,
    completed_at: Option<i64>,
    verified_at: Option<i64>,
    independent_evidence_reference: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRecord {
    case_id: String,
    community_id: String,
    period_id: String,
    decision_at: i64,
    evidence: Vec<EvidenceReference>,
    alert_issued_at: Option<i64>,
    outcome: Outcome,
    harm_occurred_at: Option<i64>,
    outcome_available_at: Option<i64>,
    response: ResponseRecord,
    scope_assessed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayDataset {
    schema_version: String,
    dataset_id: String,
    dataset_class: DatasetClass,
    authorization: Authorization,
    privacy: PrivacyPolicy,
    temporal_split: TemporalSplit,
    records: Vec<ReplayRecord>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SplitMetrics {
    split: String,
    records_total: usize,
    known_outcome_records: usize,
    unknown_outcome_records: usize,
    alerts_with_unknown_outcome: usize,
    evidence_insufficient_records: usize,
    scope_unassessed_records: usize,
    alerts_issued: usize,
    alerts_with_known_outcome: usize,
    true_positives: usize,
    false_alerts: usize,
    missed_harms: usize,
    true_negatives: usize,
    late_alerts: usize,
    timely_warning_precision_percent: Option<f64>,
    recall_percent: Option<f64>,
    false_alerts_per_100_community_periods: Option<f64>,
    outcome_coverage_percent: Option<f64>,
    median_warning_lead_seconds: Option<f64>,
    independently_verified_response_count: usize,
    verified_response_completion_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SplitMetricsBundle {
    training: SplitMetrics,
    calibration: SplitMetrics,
    test: SplitMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommunityMetrics {
    community_id: String,
    known_outcome_records: usize,
    true_positives: usize,
    false_alerts: usize,
    missed_harms: usize,
    true_negatives: usize,
    late_alerts: usize,
    recall_percent: Option<f64>,
    false_alerts_per_100_community_periods: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayReceipt {
    receipt_version: String,
    evaluator: String,
    result: String,
    source_commit: String,
    source_commit_sha_valid: bool,
    input_sha256: String,
    schema_sha256: String,
    dependency_lock_sha256: String,
    rustc_version: String,
    cargo_version: String,
    dataset_id: String,
    dataset_class: DatasetClass,
    temporal_split: TemporalSplit,
    privacy: PrivacyPolicy,
    split_metrics: SplitMetricsBundle,
    test_community_metrics: Vec<CommunityMetrics>,
    test_suppressed_community_count: usize,
    probability_calibration: String,
    causal_effect_estimated: bool,
    live_decision_authority: bool,
    execution_claim: String,
    claim_ceiling: String,
}

/// Validates the checked-in Draft 2020-12 schema, the input instance, and
/// semantic timing/governance constraints before any metrics are calculated.
pub fn validate_dataset_json(input: &[u8]) -> Result<ReplayDataset, Vec<String>> {
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES)
        .map_err(|error| vec![format!("checked-in schema JSON is invalid: {error}")])?;

    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!(
            "Draft 2020-12 schema meta-validation failed: {error}"
        )]
    })?;

    let instance: Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("replay input JSON is invalid: {error}")])?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| vec![format!("cannot build replay schema validator: {error}")])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let dataset: ReplayDataset = serde_json::from_value(instance)
        .map_err(|error| vec![format!("cannot decode replay input: {error}")])?;
    let semantic_findings = semantic_errors(&dataset);
    if semantic_findings.is_empty() {
        Ok(dataset)
    } else {
        Err(semantic_findings)
    }
}

/// Evaluates recorded historical alerts; it does not train a model or authorize
/// an intervention. The caller supplies the exact input bytes and lock bytes so
/// their hashes can be retained in the deterministic receipt.
pub fn evaluate(
    input: &[u8],
    source_commit: &str,
    rustc_version: &str,
    cargo_version: &str,
    lock_bytes: &[u8],
) -> Result<ReplayReceipt, Vec<String>> {
    let source_commit_sha_valid = valid_commit_sha(source_commit);
    if source_commit != "UNPINNED_LOCAL_RUN" && !source_commit_sha_valid {
        return Err(vec![
            "source commit must be a lowercase 40-character Git SHA or UNPINNED_LOCAL_RUN"
                .to_owned(),
        ]);
    }

    let dataset = validate_dataset_json(input)?;
    let split = dataset.temporal_split;
    let training_records = records_in_split(&dataset, SplitName::Training);
    let calibration_records = records_in_split(&dataset, SplitName::Calibration);
    let test_records = records_in_split(&dataset, SplitName::Test);

    let test_groups =
        community_metrics(&test_records, dataset.privacy.minimum_subgroup_denominator);
    let split_metrics = SplitMetricsBundle {
        training: compute_metrics(&training_records, "training"),
        calibration: compute_metrics(&calibration_records, "calibration"),
        test: compute_metrics(&test_records, "test"),
    };

    Ok(ReplayReceipt {
        receipt_version: "0.1".to_owned(),
        evaluator: "PIRB historical replay".to_owned(),
        result: "pass".to_owned(),
        source_commit: source_commit.to_owned(),
        source_commit_sha_valid,
        input_sha256: sha256_hex(input),
        schema_sha256: sha256_hex(SCHEMA_BYTES),
        dependency_lock_sha256: sha256_hex(lock_bytes),
        rustc_version: rustc_version.to_owned(),
        cargo_version: cargo_version.to_owned(),
        dataset_id: dataset.dataset_id,
        dataset_class: dataset.dataset_class,
        temporal_split: split,
        privacy: dataset.privacy,
        split_metrics,
        test_community_metrics: test_groups.0,
        test_suppressed_community_count: test_groups.1,
        probability_calibration:
            "unavailable: input contains binary recorded alerts, not probability forecasts".to_owned(),
        causal_effect_estimated: false,
        live_decision_authority: false,
        execution_claim: "descriptive_retrospective_performance_only".to_owned(),
        claim_ceiling: "A passing receipt proves input/schema conformance and deterministic descriptive replay only. It does not prove consent, authority legitimacy, data truth, intervention safety, causal impact, or reduced violence.".to_owned(),
    })
}

#[derive(Debug, Clone, Copy)]
enum SplitName {
    Training,
    Calibration,
    Test,
}

fn records_in_split(dataset: &ReplayDataset, name: SplitName) -> Vec<&ReplayRecord> {
    dataset
        .records
        .iter()
        .filter(|record| match name {
            SplitName::Training => {
                record.decision_at >= dataset.temporal_split.evaluation_start_at
                    && record.decision_at < dataset.temporal_split.training_end_exclusive
            }
            SplitName::Calibration => {
                record.decision_at >= dataset.temporal_split.training_end_exclusive
                    && record.decision_at < dataset.temporal_split.calibration_end_exclusive
            }
            SplitName::Test => {
                record.decision_at >= dataset.temporal_split.calibration_end_exclusive
                    && record.decision_at < dataset.temporal_split.test_end_exclusive
            }
        })
        .collect()
}

fn semantic_errors(dataset: &ReplayDataset) -> Vec<String> {
    let mut findings = Vec::new();
    if !dataset.privacy.aggregate_only {
        findings.push("privacy.aggregate_only must be true".to_owned());
    }
    if dataset.privacy.minimum_subgroup_denominator == 0 {
        findings.push("privacy.minimum_subgroup_denominator must be positive".to_owned());
    }
    if dataset.schema_version != "0.1" {
        findings.push("schema_version must be 0.1".to_owned());
    }
    for (name, reference) in [
        (
            "authority_reference",
            &dataset.authorization.authority_reference,
        ),
        (
            "protocol_reference",
            &dataset.authorization.protocol_reference,
        ),
        (
            "privacy_review_reference",
            &dataset.authorization.privacy_review_reference,
        ),
        (
            "data_steward_reference",
            &dataset.authorization.data_steward_reference,
        ),
    ] {
        if reference.trim().is_empty() {
            findings.push(format!("authorization.{name} must not be empty"));
        }
        if dataset.dataset_class == DatasetClass::AuthorizedHistorical
            && reference.starts_with("synthetic:")
        {
            findings.push(format!(
                "authorized_historical input cannot use a synthetic {name}"
            ));
        }
    }

    let split = dataset.temporal_split;
    if !(split.evaluation_start_at < split.training_end_exclusive
        && split.training_end_exclusive < split.calibration_end_exclusive
        && split.calibration_end_exclusive < split.test_end_exclusive
        && split.test_end_exclusive <= split.evaluation_as_of
        && split.outcome_horizon_seconds > 0)
    {
        findings.push(
            "temporal split must be strictly ordered and evaluation_as_of must be at or after test_end_exclusive"
                .to_owned(),
        );
    }

    let mut seen_cases = BTreeSet::new();
    let mut seen_community_periods = BTreeSet::new();
    for record in &dataset.records {
        if !seen_cases.insert(record.case_id.clone()) {
            findings.push(format!("{} duplicates a case_id", record.case_id));
        }
        let pair = (record.community_id.clone(), record.period_id.clone());
        if !seen_community_periods.insert(pair) {
            findings.push(format!(
                "{} duplicates a community_id/period_id observation",
                record.case_id
            ));
        }
        if record.decision_at < split.evaluation_start_at
            || record.decision_at >= split.test_end_exclusive
        {
            findings.push(format!(
                "{} decision_at is outside the declared temporal evaluation window",
                record.case_id
            ));
        }

        let mut evidence_ids = BTreeSet::new();
        for evidence in &record.evidence {
            if !evidence_ids.insert(evidence.evidence_id.clone()) {
                findings.push(format!(
                    "{} repeats an evidence_id within one decision",
                    record.case_id
                ));
            }
            match (evidence.status, evidence.available_at) {
                (EvidenceStatus::Missing, None) => {}
                (EvidenceStatus::Missing, Some(_)) => {
                    findings.push(format!(
                        "{} marks missing evidence with an available_at timestamp",
                        record.case_id
                    ));
                }
                (_, None) => findings.push(format!(
                    "{} has non-missing evidence without available_at",
                    record.case_id
                )),
                (_, Some(available_at)) if available_at > record.decision_at => {
                    findings.push(format!(
                        "{} contains look-ahead evidence available after decision_at",
                        record.case_id
                    ));
                }
                _ => {}
            }
        }

        match record.outcome {
            Outcome::Harm => {
                match (record.harm_occurred_at, record.outcome_available_at) {
                    (Some(harm_at), Some(available_at)) => {
                        if harm_at <= record.decision_at {
                            findings.push(format!(
                                "{} harm must occur strictly after the decision time",
                                record.case_id
                            ));
                        }
                        match record.decision_at.checked_add(split.outcome_horizon_seconds) {
                            Some(followup_end) if harm_at > followup_end => {
                                findings.push(format!(
                                    "{} harm lies beyond the declared outcome horizon",
                                    record.case_id
                                ));
                            }
                            None => findings.push(format!(
                                "{} decision time plus outcome horizon overflows the timestamp range",
                                record.case_id
                            )),
                            _ => {}
                        }
                        if available_at <= record.decision_at || available_at < harm_at {
                            findings.push(format!(
                                "{} outcome label must become available after the decision and no earlier than harm",
                                record.case_id
                            ));
                        }
                        if available_at > split.evaluation_as_of {
                            findings.push(format!(
                                "{} outcome label was not available by evaluation_as_of",
                                record.case_id
                            ));
                        }
                    }
                    _ => findings.push(format!(
                        "{} harm outcome requires harm_occurred_at and outcome_available_at",
                        record.case_id
                    )),
                }
            }
            Outcome::NoHarm => {
                if record.harm_occurred_at.is_some() {
                    findings.push(format!(
                        "{} no_harm outcome cannot have harm_occurred_at",
                        record.case_id
                    ));
                }
                match (
                    record.outcome_available_at,
                    record.decision_at.checked_add(split.outcome_horizon_seconds),
                ) {
                    (Some(available_at), Some(followup_end)) if available_at >= followup_end => {
                        if available_at > split.evaluation_as_of {
                            findings.push(format!(
                                "{} outcome label was not available by evaluation_as_of",
                                record.case_id
                            ));
                        }
                    }
                    _ => findings.push(format!(
                        "{} no_harm requires complete follow-up through the declared outcome horizon",
                        record.case_id
                    )),
                }
            }
            Outcome::Unknown => {
                if record.harm_occurred_at.is_some() || record.outcome_available_at.is_some() {
                    findings.push(format!(
                        "{} unknown outcome must not carry outcome timestamps",
                        record.case_id
                    ));
                }
            }
        }

        if let Some(alert_at) = record.alert_issued_at {
            if alert_at < record.decision_at {
                findings.push(format!(
                    "{} alert_issued_at predates decision_at",
                    record.case_id
                ));
            }
            if alert_at > split.evaluation_as_of {
                findings.push(format!(
                    "{} alert_issued_at is after evaluation_as_of",
                    record.case_id
                ));
            }
        }
        validate_response(record, split.evaluation_as_of, &mut findings);
    }
    findings
}

fn validate_response(record: &ReplayRecord, evaluation_as_of: i64, findings: &mut Vec<String>) {
    let response = &record.response;
    let alert_at = record.alert_issued_at;
    match (alert_at, response.status) {
        (None, ResponseStatus::NotAlerted) => {
            if response.completed_at.is_some()
                || response.verified_at.is_some()
                || response.independent_evidence_reference.is_some()
            {
                findings.push(format!(
                    "{} has response evidence but no alert was issued",
                    record.case_id
                ));
            }
        }
        (None, _) => findings.push(format!(
            "{} has a response state despite no alert",
            record.case_id
        )),
        (Some(_), ResponseStatus::NotAlerted) => findings.push(format!(
            "{} marks an issued alert as not_alerted",
            record.case_id
        )),
        (Some(alert_at), ResponseStatus::CompletedUnverified) => {
            match response.completed_at {
                Some(completed_at)
                    if completed_at >= alert_at && completed_at <= evaluation_as_of => {}
                _ => findings.push(format!(
                    "{} completed_unverified response requires a completion time after the alert and by evaluation_as_of",
                    record.case_id
                )),
            }
            if response.verified_at.is_some() || response.independent_evidence_reference.is_some() {
                findings.push(format!(
                    "{} unverified completion cannot carry independent verification fields",
                    record.case_id
                ));
            }
        }
        (Some(alert_at), ResponseStatus::IndependentlyVerifiedCompleted) => {
            let completion_is_valid = response
                .completed_at
                .is_some_and(|time| time >= alert_at && time <= evaluation_as_of);
            let verification_is_valid = response.verified_at.is_some_and(|time| {
                response
                    .completed_at
                    .is_some_and(|completed| time >= completed)
                    && time <= evaluation_as_of
            });
            let has_independent_reference = response
                .independent_evidence_reference
                .as_ref()
                .is_some_and(|reference| !reference.trim().is_empty());
            if !completion_is_valid || !verification_is_valid || !has_independent_reference {
                findings.push(format!(
                    "{} verified completion requires ordered completion/verification times and an independent evidence reference",
                    record.case_id
                ));
            }
        }
        (Some(_), ResponseStatus::NotDispatched)
        | (Some(_), ResponseStatus::Dispatched)
        | (Some(_), ResponseStatus::Disputed) => {
            if response.completed_at.is_some()
                || response.verified_at.is_some()
                || response.independent_evidence_reference.is_some()
            {
                findings.push(format!(
                    "{} response state is inconsistent with its completion/verification fields",
                    record.case_id
                ));
            }
        }
    }
}

fn community_metrics(
    test_records: &[&ReplayRecord],
    minimum_denominator: usize,
) -> (Vec<CommunityMetrics>, usize) {
    let mut groups: BTreeMap<String, Vec<&ReplayRecord>> = BTreeMap::new();
    for record in test_records {
        groups
            .entry(record.community_id.clone())
            .or_default()
            .push(record);
    }

    let mut visible = Vec::new();
    let mut suppressed = 0;
    for (community_id, records) in groups {
        let metrics = compute_metrics(&records, "test");
        if metrics.known_outcome_records < minimum_denominator {
            suppressed += 1;
            continue;
        }
        visible.push(CommunityMetrics {
            community_id,
            known_outcome_records: metrics.known_outcome_records,
            true_positives: metrics.true_positives,
            false_alerts: metrics.false_alerts,
            missed_harms: metrics.missed_harms,
            true_negatives: metrics.true_negatives,
            late_alerts: metrics.late_alerts,
            recall_percent: metrics.recall_percent,
            false_alerts_per_100_community_periods: metrics.false_alerts_per_100_community_periods,
        });
    }
    (visible, suppressed)
}

fn compute_metrics(records: &[&ReplayRecord], split: &str) -> SplitMetrics {
    let mut metrics = SplitMetrics {
        split: split.to_owned(),
        records_total: records.len(),
        ..SplitMetrics::default()
    };
    let mut warning_leads = Vec::new();

    for record in records {
        if !record.scope_assessed {
            metrics.scope_unassessed_records += 1;
        }
        if record.evidence.is_empty()
            || record
                .evidence
                .iter()
                .any(|evidence| evidence.status != EvidenceStatus::Verified)
        {
            metrics.evidence_insufficient_records += 1;
        }
        if record.alert_issued_at.is_some() {
            metrics.alerts_issued += 1;
        }

        match record.outcome {
            Outcome::Unknown => {
                metrics.unknown_outcome_records += 1;
                if record.alert_issued_at.is_some() {
                    metrics.alerts_with_unknown_outcome += 1;
                }
            }
            Outcome::Harm => {
                metrics.known_outcome_records += 1;
                if let Some(alert_at) = record.alert_issued_at {
                    metrics.alerts_with_known_outcome += 1;
                    let harm_at = record
                        .harm_occurred_at
                        .expect("validated harm records contain a harm time");
                    if alert_at < harm_at {
                        metrics.true_positives += 1;
                        warning_leads.push(harm_at - alert_at);
                    } else {
                        metrics.missed_harms += 1;
                        metrics.late_alerts += 1;
                    }
                } else {
                    metrics.missed_harms += 1;
                }
            }
            Outcome::NoHarm => {
                metrics.known_outcome_records += 1;
                if record.alert_issued_at.is_some() {
                    metrics.alerts_with_known_outcome += 1;
                    metrics.false_alerts += 1;
                } else {
                    metrics.true_negatives += 1;
                }
            }
        }

        if record.response.status == ResponseStatus::IndependentlyVerifiedCompleted {
            metrics.independently_verified_response_count += 1;
        }
    }

    metrics.timely_warning_precision_percent =
        percentage(metrics.true_positives, metrics.alerts_with_known_outcome);
    metrics.recall_percent = percentage(
        metrics.true_positives,
        metrics.true_positives + metrics.missed_harms,
    );
    metrics.false_alerts_per_100_community_periods =
        percentage(metrics.false_alerts, metrics.known_outcome_records);
    metrics.outcome_coverage_percent =
        percentage(metrics.known_outcome_records, metrics.records_total);
    metrics.verified_response_completion_percent = percentage(
        metrics.independently_verified_response_count,
        metrics.alerts_issued,
    );
    warning_leads.sort_unstable();
    metrics.median_warning_lead_seconds = match warning_leads.len() {
        0 => None,
        n if n % 2 == 1 => Some(warning_leads[n / 2] as f64),
        n => Some((warning_leads[n / 2 - 1] as f64 + warning_leads[n / 2] as f64) / 2.0),
    };
    metrics
}

fn percentage(numerator: usize, denominator: usize) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(((numerator as f64 / denominator as f64) * 10_000.0).round() / 100.0)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn valid_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] =
        include_bytes!("../../../docs/fixtures/pirb-historical-replay-v0.1.json");
    const LOCK: &[u8] = b"synthetic-test-lock";

    fn parsed_fixture() -> ReplayDataset {
        validate_dataset_json(FIXTURE).expect("checked-in synthetic fixture should validate")
    }

    fn test_receipt() -> ReplayReceipt {
        evaluate(
            FIXTURE,
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        )
        .expect("checked-in synthetic fixture should evaluate")
    }

    #[test]
    fn schema_and_synthetic_fixture_validate() {
        let receipt = test_receipt();
        assert_eq!(receipt.result, "pass");
        assert!(receipt.source_commit_sha_valid);
        assert_eq!(receipt.dataset_class, DatasetClass::Synthetic);
        assert!(receipt.privacy.aggregate_only);
        assert_eq!(receipt.privacy.minimum_subgroup_denominator, 2);
        assert_eq!(receipt.split_metrics.test.records_total, 6);
        assert_eq!(receipt.split_metrics.test.known_outcome_records, 5);
        assert_eq!(receipt.split_metrics.test.unknown_outcome_records, 1);
    }

    #[test]
    fn unknown_outcomes_are_not_treated_as_no_harm() {
        let receipt = test_receipt();
        let metrics = &receipt.split_metrics.test;
        assert_eq!(metrics.known_outcome_records, 5);
        assert_eq!(metrics.unknown_outcome_records, 1);
        assert_eq!(metrics.true_negatives, 1);
        assert_eq!(metrics.false_alerts, 2);
    }

    #[test]
    fn late_alert_is_a_missed_harm_and_not_a_timely_true_positive() {
        let receipt = test_receipt();
        assert_eq!(receipt.split_metrics.calibration.true_positives, 1);
        assert_eq!(receipt.split_metrics.calibration.missed_harms, 1);
        assert_eq!(receipt.split_metrics.calibration.late_alerts, 1);
    }

    #[test]
    fn small_community_subgroups_are_suppressed() {
        let receipt = test_receipt();
        assert_eq!(receipt.test_suppressed_community_count, 2);
        assert_eq!(receipt.test_community_metrics.len(), 2);
        assert_eq!(
            receipt
                .test_community_metrics
                .iter()
                .map(|group| group.known_outcome_records)
                .min(),
            Some(2)
        );
    }

    #[test]
    fn evidence_available_after_decision_is_rejected() {
        let mut dataset = parsed_fixture();
        dataset.records[0].evidence[0].available_at = Some(dataset.records[0].decision_at + 1);
        assert!(
            semantic_errors(&dataset)
                .iter()
                .any(|finding| finding.contains("look-ahead evidence"))
        );
    }

    #[test]
    fn splits_must_be_strictly_time_ordered() {
        let mut dataset = parsed_fixture();
        dataset.temporal_split.calibration_end_exclusive =
            dataset.temporal_split.training_end_exclusive;
        assert!(
            semantic_errors(&dataset)
                .iter()
                .any(|finding| finding.contains("strictly ordered"))
        );
    }

    #[test]
    fn duplicate_community_period_denominator_is_rejected() {
        let mut dataset = parsed_fixture();
        dataset.records[1].community_id = dataset.records[0].community_id.clone();
        dataset.records[1].period_id = dataset.records[0].period_id.clone();
        assert!(
            semantic_errors(&dataset)
                .iter()
                .any(|finding| finding.contains("community_id/period_id"))
        );
    }

    #[test]
    fn verified_response_requires_an_independent_reference() {
        let mut dataset = parsed_fixture();
        dataset.records[7].response.independent_evidence_reference = None;
        assert!(
            semantic_errors(&dataset)
                .iter()
                .any(|finding| finding.contains("independent evidence reference"))
        );
    }

    #[test]
    fn unknown_fields_such_as_person_level_ids_are_rejected_by_schema() {
        let mut input: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON parses");
        input["records"][0]["person_id"] = Value::String("must-not-be-present".to_owned());
        let encoded = serde_json::to_vec(&input).expect("modified fixture serializes");
        assert!(validate_dataset_json(&encoded).is_err());
    }

    #[test]
    fn historical_dataset_cannot_claim_synthetic_authorization_references() {
        let mut dataset = parsed_fixture();
        dataset.dataset_class = DatasetClass::AuthorizedHistorical;
        assert!(
            semantic_errors(&dataset)
                .iter()
                .any(|finding| finding.contains("cannot use a synthetic"))
        );
    }

    #[test]
    fn local_unpinned_run_is_never_marked_commit_sha_valid() {
        let receipt = evaluate(FIXTURE, "UNPINNED_LOCAL_RUN", "unknown", "unknown", LOCK)
            .expect("local run should still be useful");
        assert!(!receipt.source_commit_sha_valid);
    }

    #[test]
    fn malformed_source_commit_is_rejected() {
        assert!(evaluate(FIXTURE, "not-a-sha", "rustc", "cargo", LOCK).is_err());
    }

    #[test]
    fn verified_response_completion_is_not_reported_as_causal_prevention() {
        let receipt = test_receipt();
        assert!(!receipt.causal_effect_estimated);
        assert!(!receipt.live_decision_authority);
        assert_eq!(
            receipt.execution_claim,
            "descriptive_retrospective_performance_only"
        );
    }

    #[test]
    fn negative_outcome_requires_full_outcome_horizon() {
        let mut dataset = parsed_fixture();
        let decision_at = dataset.records[1].decision_at;
        dataset.records[1].outcome_available_at =
            Some(decision_at + dataset.temporal_split.outcome_horizon_seconds - 1);
        assert!(semantic_errors(&dataset).iter().any(|finding| {
            finding.contains("complete follow-up through the declared outcome horizon")
        }));
    }

    #[test]
    fn harm_outside_declared_outcome_horizon_is_rejected() {
        let mut dataset = parsed_fixture();
        let decision_at = dataset.records[0].decision_at;
        dataset.records[0].harm_occurred_at =
            Some(decision_at + dataset.temporal_split.outcome_horizon_seconds + 1);
        assert!(
            semantic_errors(&dataset)
                .iter()
                .any(|finding| finding.contains("beyond the declared outcome horizon"))
        );
    }

    #[test]
    fn alert_before_decision_is_rejected() {
        let mut dataset = parsed_fixture();
        let decision_at = dataset.records[0].decision_at;
        dataset.records[0].alert_issued_at = Some(decision_at - 1);
        assert!(
            semantic_errors(&dataset)
                .iter()
                .any(|finding| finding.contains("predates decision_at"))
        );
    }

    #[test]
    fn bounded_temporal_confusion_table_matches_independent_oracle() {
        let mut dataset = parsed_fixture();
        let scenarios = [
            (Outcome::Harm, false, false),
            (Outcome::Harm, true, false),
            (Outcome::Harm, true, true),
            (Outcome::NoHarm, false, false),
            (Outcome::NoHarm, true, false),
            (Outcome::Unknown, false, false),
            (Outcome::Unknown, true, false),
        ];
        let mut generated = Vec::new();

        for (index, (outcome, alerted, late)) in scenarios.into_iter().enumerate() {
            let mut record = dataset.records[7].clone();
            record.case_id = format!("case-oracle-{index}");
            record.community_id = "community-oracle".to_owned();
            record.period_id = format!("period-oracle-{index}");
            record.decision_at =
                dataset.temporal_split.calibration_end_exclusive + 5 + index as i64 * 10;
            record.evidence[0].available_at = Some(record.decision_at - 1);
            record.outcome = outcome;
            let harm_at = if outcome == Outcome::Harm {
                Some(record.decision_at + 5)
            } else {
                None
            };
            record.harm_occurred_at = harm_at;
            record.outcome_available_at = match outcome {
                Outcome::Harm => Some(record.decision_at + 10),
                Outcome::NoHarm => {
                    Some(record.decision_at + dataset.temporal_split.outcome_horizon_seconds)
                }
                Outcome::Unknown => None,
            };
            record.alert_issued_at = if alerted {
                Some(if late {
                    harm_at.expect("late scenario is a harm case") + 1
                } else {
                    record.decision_at + 1
                })
            } else {
                None
            };
            record.response = if alerted {
                ResponseRecord {
                    status: ResponseStatus::NotDispatched,
                    completed_at: None,
                    verified_at: None,
                    independent_evidence_reference: None,
                }
            } else {
                ResponseRecord {
                    status: ResponseStatus::NotAlerted,
                    completed_at: None,
                    verified_at: None,
                    independent_evidence_reference: None,
                }
            };
            generated.push(record);
        }

        dataset.records = generated.clone();
        assert!(semantic_errors(&dataset).is_empty());
        let references: Vec<&ReplayRecord> = generated.iter().collect();
        let actual = compute_metrics(&references, "test");

        // Independently count the seven enumerated state combinations.
        // Indexes: known, unknown, unknown-alert, alerts, known-alert,
        // timely-TP, false-alert, missed-harm, TN, late-alert.
        let mut oracle = [0_usize; 10];
        for (outcome, alerted, late) in scenarios {
            if outcome == Outcome::Unknown {
                oracle[1] += 1;
                if alerted {
                    oracle[2] += 1;
                }
            } else {
                oracle[0] += 1;
                if alerted {
                    oracle[4] += 1;
                }
            }
            if alerted {
                oracle[3] += 1;
            }
            match outcome {
                Outcome::Harm if alerted && !late => oracle[5] += 1,
                Outcome::Harm => {
                    oracle[7] += 1;
                    if late {
                        oracle[9] += 1;
                    }
                }
                Outcome::NoHarm if alerted => oracle[6] += 1,
                Outcome::NoHarm => oracle[8] += 1,
                Outcome::Unknown => {}
            }
        }

        assert_eq!(actual.known_outcome_records, oracle[0]);
        assert_eq!(actual.unknown_outcome_records, oracle[1]);
        assert_eq!(actual.alerts_with_unknown_outcome, oracle[2]);
        assert_eq!(actual.alerts_issued, oracle[3]);
        assert_eq!(actual.alerts_with_known_outcome, oracle[4]);
        assert_eq!(actual.true_positives, oracle[5]);
        assert_eq!(actual.false_alerts, oracle[6]);
        assert_eq!(actual.missed_harms, oracle[7]);
        assert_eq!(actual.true_negatives, oracle[8]);
        assert_eq!(actual.late_alerts, oracle[9]);
        assert_eq!(actual.timely_warning_precision_percent, Some(33.33));
        assert_eq!(actual.recall_percent, Some(33.33));
        assert_eq!(actual.false_alerts_per_100_community_periods, Some(20.0));
        assert_eq!(actual.outcome_coverage_percent, Some(71.43));
        assert_eq!(actual.median_warning_lead_seconds, Some(4.0));
    }
}
