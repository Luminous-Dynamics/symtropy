// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Decision-impact audit for forecast-driven service proposals.
//!
//! The evaluator uses a single frozen policy on both the forecast and the
//! realized-information replay. Hard-gate failures are exclusions, never
//! finite penalties. This is a retrospective benchmark, not an online
//! decision engine or an authority to admit work.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::battery_service_network::{EvidenceClass, EvidenceEnvelope};

const MAX_RECORDS: usize = 2_000;
const MAX_VALUE: u64 = 1_000_000_000_000;
const CLAIM_CEILING: &str = "Bounded synthetic/recorded decision replay only. Realized-information replay is retrospective and unavailable to the live policy. No field qualification, dispatch, flight, grid, market, ledger or physical authority.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdDirection {
    /// Admit only when the entire forecast interval is at or above threshold.
    MinimumAtLeastThreshold,
    /// Admit only when the entire forecast interval is at or below threshold.
    MaximumAtMostThreshold,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HardGateDisposition {
    Passed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionImpactRecord {
    pub sample_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub site_evidence: EvidenceEnvelope,
    pub asset_id: String,
    pub asset_configuration_digest: String,
    pub stratum_id: String,
    pub forecast_model_id: String,
    pub forecast_model_version: String,
    pub policy_id: String,
    pub policy_version: String,
    pub decision_time_unix_ms: u64,
    pub target_start_unix_ms: u64,
    pub target_end_unix_ms: u64,
    pub direction: ThresholdDirection,
    pub threshold_value: u64,
    pub predicted_lower: u64,
    pub predicted_point: u64,
    pub predicted_upper: u64,
    pub observed_value: u64,
    pub hard_gate_disposition: HardGateDisposition,
    pub candidate_value_proxy_units: u64,
    pub forecast_evidence: EvidenceEnvelope,
    pub hard_gate_evidence: EvidenceEnvelope,
    pub outcome_evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceDecisionImpactInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub audit_cutoff_unix_ms: u64,
    pub records: Vec<DecisionImpactRecord>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WilsonInterval {
    pub lower_permille: u16,
    pub upper_permille: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DecisionRiskCoveragePoint {
    /// Minimum signed conservative-bound margin required for a proposal.
    /// A positive margin is conservative headroom; a negative margin relaxes it.
    pub minimum_margin: i64,
    pub accepted_proposals: usize,
    pub false_safe_proposals: usize,
    pub coverage_permille: u16,
    pub false_safe_rate_permille: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DecisionImpactGroup {
    pub site_id: String,
    pub asset_id: String,
    pub asset_configuration_digest: String,
    pub stratum_id: String,
    pub forecast_model_id: String,
    pub forecast_model_version: String,
    pub policy_id: String,
    pub policy_version: String,
    pub direction: ThresholdDirection,
    pub threshold_value: u64,
    pub sample_count: usize,
    pub candidate_admissions: usize,
    pub operational_admissions_after_hard_gate: usize,
    pub hard_gate_blocked_proposals: usize,
    pub unsupported_proposals: usize,
    pub hard_gate_bypass_attempts: usize,
    pub false_safe_proposals: usize,
    pub false_safe_admissions: usize,
    pub false_safe_admission_rate_wilson_95: Option<WilsonInterval>,
    pub false_abstentions: usize,
    pub realized_feasible_count: usize,
    pub false_abstention_rate_wilson_95: Option<WilsonInterval>,
    pub decision_flips: usize,
    pub replay_defined_count: usize,
    pub potentially_admissible_value_proxy_units: u64,
    pub risk_coverage: Vec<DecisionRiskCoveragePoint>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ServiceDecisionImpactReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub audit_cutoff_unix_ms: u64,
    /// Wilson intervals are IID reference calculations; clustered temporal and
    /// common-cause inference must be reported separately.
    pub wilson_reference_assumes_iid_samples: bool,
    pub record_count: usize,
    pub candidate_admission_count: usize,
    pub operational_admission_count: usize,
    pub hard_gate_blocked_proposal_count: usize,
    pub unsupported_proposal_count: usize,
    pub hard_gate_bypass_attempt_count: usize,
    pub false_safe_proposal_count: usize,
    pub false_safe_admission_count: usize,
    pub false_abstention_count: usize,
    pub decision_flip_count: usize,
    pub replay_defined_count: usize,
    pub potentially_admissible_value_proxy_units: u64,
    pub risk_coverage_group_count: usize,
    pub all_evidence_source_grade: bool,
    pub any_synthetic_evidence: bool,
    pub input_digest: String,
    pub groups: Vec<DecisionImpactGroup>,
    pub field_qualification_granted: bool,
    pub model_update_performed: bool,
    pub operational_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    site_id: String,
    asset_id: String,
    asset_configuration_digest: String,
    stratum_id: String,
    forecast_model_id: String,
    forecast_model_version: String,
    policy_id: String,
    policy_version: String,
    direction: ThresholdDirection,
    threshold_value: u64,
}

#[derive(Debug, Clone, Copy)]
struct EvaluatedDecision {
    margin: i64,
    candidate_admit: bool,
    realized_metric_admit: bool,
    hard_gate: HardGateDisposition,
    candidate_value_proxy_units: u64,
}

fn evidence_well_formed(e: &EvidenceEnvelope) -> bool {
    !e.asset_id.trim().is_empty()
        && !e.configuration_digest.trim().is_empty()
        && !e.producer_id.trim().is_empty()
        && !e.method_id.trim().is_empty()
        && !e.evidence_reference.trim().is_empty()
        && e.evidence_class != EvidenceClass::Unknown
}

fn forecast_evidence_valid(
    e: &EvidenceEnvelope,
    asset_id: &str,
    config: &str,
    decision_time: u64,
    target_end: u64,
) -> bool {
    if !evidence_well_formed(e) || e.asset_id != asset_id || e.configuration_digest != config {
        return false;
    }
    matches!(
        (e.observed_at_unix_ms, e.valid_from_unix_ms, e.valid_until_unix_ms),
        (Some(observed), Some(from), Some(until))
            if from <= observed
                && observed <= decision_time
                && from <= decision_time
                && target_end <= until
    )
}

fn outcome_evidence_valid(
    e: &EvidenceEnvelope,
    asset_id: &str,
    config: &str,
    target_end: u64,
    audit_cutoff: u64,
) -> bool {
    if !evidence_well_formed(e) || e.asset_id != asset_id || e.configuration_digest != config {
        return false;
    }
    matches!(
        (e.observed_at_unix_ms, e.valid_from_unix_ms, e.valid_until_unix_ms),
        (Some(observed), Some(from), Some(until))
            if from <= target_end
                && observed >= target_end
                && observed <= audit_cutoff
                && observed <= until
    )
}

fn signed_margin(direction: ThresholdDirection, lower: u64, upper: u64, threshold: u64) -> i64 {
    let margin = match direction {
        ThresholdDirection::MinimumAtLeastThreshold => i128::from(lower) - i128::from(threshold),
        ThresholdDirection::MaximumAtMostThreshold => i128::from(threshold) - i128::from(upper),
    };
    // Values are capped at 1e12, so this conversion is bounded.
    i64::try_from(margin).expect("validated decision values fit in i64")
}

fn realized_metric_admit(direction: ThresholdDirection, observed: u64, threshold: u64) -> bool {
    match direction {
        ThresholdDirection::MinimumAtLeastThreshold => observed >= threshold,
        ThresholdDirection::MaximumAtMostThreshold => observed <= threshold,
    }
}

fn proposal_from_margin(margin: i64) -> bool {
    margin >= 0
}

fn permille(numerator: usize, denominator: usize) -> u16 {
    if denominator == 0 {
        0
    } else {
        ((numerator as u128 * 1000) / denominator as u128) as u16
    }
}

fn wilson_interval(successes: usize, trials: usize) -> Option<WilsonInterval> {
    if trials == 0 || successes > trials {
        return None;
    }
    let z = 1.959_963_984_540_054_f64;
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = p + z2 / (2.0 * n);
    let radius = z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt());
    let lower = ((center - radius) / denominator).clamp(0.0, 1.0) * 1000.0;
    let upper = ((center + radius) / denominator).clamp(0.0, 1.0) * 1000.0;
    Some(WilsonInterval {
        lower_permille: lower.floor() as u16,
        upper_permille: upper.ceil() as u16,
    })
}

fn group_key(r: &DecisionImpactRecord) -> GroupKey {
    GroupKey {
        site_id: r.site_id.clone(),
        asset_id: r.asset_id.clone(),
        asset_configuration_digest: r.asset_configuration_digest.clone(),
        stratum_id: r.stratum_id.clone(),
        forecast_model_id: r.forecast_model_id.clone(),
        forecast_model_version: r.forecast_model_version.clone(),
        policy_id: r.policy_id.clone(),
        policy_version: r.policy_version.clone(),
        direction: r.direction,
        threshold_value: r.threshold_value,
    }
}

fn validate(input: &ServiceDecisionImpactInput) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0" {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if input.benchmark_id != "SERVICE-DECISION-IMPACT-001" {
        errors.push("benchmark_id must be SERVICE-DECISION-IMPACT-001".to_owned());
    }
    if input.claim_ceiling.trim().is_empty() {
        errors.push("claim_ceiling must be non-empty".to_owned());
    }
    if input.records.is_empty() || input.records.len() > MAX_RECORDS {
        errors.push(format!("records must contain 1..={MAX_RECORDS} items"));
    }

    let mut sample_ids = BTreeSet::new();
    let mut evidence_refs = BTreeSet::new();
    for (i, r) in input.records.iter().enumerate() {
        let prefix = format!("records[{i}]");
        for (name, value) in [
            ("sample_id", r.sample_id.as_str()),
            ("site_id", r.site_id.as_str()),
            (
                "site_configuration_digest",
                r.site_configuration_digest.as_str(),
            ),
            ("asset_id", r.asset_id.as_str()),
            (
                "asset_configuration_digest",
                r.asset_configuration_digest.as_str(),
            ),
            ("stratum_id", r.stratum_id.as_str()),
            ("forecast_model_id", r.forecast_model_id.as_str()),
            ("forecast_model_version", r.forecast_model_version.as_str()),
            ("policy_id", r.policy_id.as_str()),
            ("policy_version", r.policy_version.as_str()),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("{prefix}.{name} must be non-empty"));
            }
        }
        if !sample_ids.insert(r.sample_id.as_str()) {
            errors.push(format!("{prefix}.sample_id is duplicated"));
        }
        if r.decision_time_unix_ms == 0
            || r.decision_time_unix_ms > r.target_start_unix_ms
            || r.target_start_unix_ms >= r.target_end_unix_ms
            || r.target_end_unix_ms > input.audit_cutoff_unix_ms
        {
            errors.push(format!(
                "{prefix} has invalid decision/target/audit chronology"
            ));
        }
        if [
            r.threshold_value,
            r.predicted_lower,
            r.predicted_point,
            r.predicted_upper,
            r.observed_value,
            r.candidate_value_proxy_units,
        ]
        .into_iter()
        .any(|v| v > MAX_VALUE)
        {
            errors.push(format!("{prefix} contains a value above {MAX_VALUE}"));
        }
        if !(r.predicted_lower <= r.predicted_point && r.predicted_point <= r.predicted_upper) {
            errors.push(format!(
                "{prefix} forecast interval/point ordering is invalid"
            ));
        }

        let site_ok = forecast_evidence_valid(
            &r.site_evidence,
            &r.site_id,
            &r.site_configuration_digest,
            r.decision_time_unix_ms,
            r.target_end_unix_ms,
        );
        let forecast_ok = forecast_evidence_valid(
            &r.forecast_evidence,
            &r.asset_id,
            &r.asset_configuration_digest,
            r.decision_time_unix_ms,
            r.target_end_unix_ms,
        );
        let gate_ok = forecast_evidence_valid(
            &r.hard_gate_evidence,
            &r.asset_id,
            &r.asset_configuration_digest,
            r.decision_time_unix_ms,
            r.target_end_unix_ms,
        );
        let outcome_ok = outcome_evidence_valid(
            &r.outcome_evidence,
            &r.asset_id,
            &r.asset_configuration_digest,
            r.target_end_unix_ms,
            input.audit_cutoff_unix_ms,
        );
        if !site_ok {
            errors.push(format!(
                "{prefix}.site_evidence is stale, mismatched or invalid"
            ));
        }
        if !forecast_ok {
            errors.push(format!(
                "{prefix}.forecast_evidence is stale, mismatched or invalid"
            ));
        }
        if !gate_ok {
            errors.push(format!(
                "{prefix}.hard_gate_evidence is stale, mismatched or invalid"
            ));
        }
        if !outcome_ok {
            errors.push(format!(
                "{prefix}.outcome_evidence is premature, mismatched or invalid"
            ));
        }
        for e in [
            &r.site_evidence,
            &r.forecast_evidence,
            &r.hard_gate_evidence,
            &r.outcome_evidence,
        ] {
            if evidence_well_formed(e) && !evidence_refs.insert(e.evidence_reference.as_str()) {
                errors.push(format!("{prefix} reuses an evidence reference"));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn evaluate_service_decision_impact(
    input: &ServiceDecisionImpactInput,
) -> Result<ServiceDecisionImpactReport, Vec<String>> {
    validate(input)?;

    let mut canonical = input.clone();
    canonical
        .records
        .sort_by(|a, b| a.sample_id.cmp(&b.sample_id));
    let canonical_bytes = serde_json::to_vec(&canonical)
        .map_err(|e| vec![format!("cannot canonicalize decision-impact input: {e}")])?;
    let input_digest = format!("sha256:{:x}", Sha256::digest(canonical_bytes));

    let mut evaluated: BTreeMap<GroupKey, Vec<EvaluatedDecision>> = BTreeMap::new();
    let mut all_evidence_source_grade = true;
    let mut any_synthetic_evidence = false;

    for r in &input.records {
        let evidence = [
            &r.site_evidence,
            &r.forecast_evidence,
            &r.hard_gate_evidence,
            &r.outcome_evidence,
        ];
        for e in evidence {
            if e.evidence_class == EvidenceClass::Synthetic {
                any_synthetic_evidence = true;
            }
            if !matches!(
                e.evidence_class,
                EvidenceClass::Measured | EvidenceClass::IndependentTest
            ) {
                all_evidence_source_grade = false;
            }
        }

        let margin = signed_margin(
            r.direction,
            r.predicted_lower,
            r.predicted_upper,
            r.threshold_value,
        );
        evaluated
            .entry(group_key(r))
            .or_default()
            .push(EvaluatedDecision {
                margin,
                candidate_admit: proposal_from_margin(margin),
                realized_metric_admit: realized_metric_admit(
                    r.direction,
                    r.observed_value,
                    r.threshold_value,
                ),
                hard_gate: r.hard_gate_disposition,
                candidate_value_proxy_units: r.candidate_value_proxy_units,
            });
    }

    let mut groups = Vec::new();
    let mut totals = DecisionImpactGroup {
        site_id: String::new(),
        asset_id: String::new(),
        asset_configuration_digest: String::new(),
        stratum_id: String::new(),
        forecast_model_id: String::new(),
        forecast_model_version: String::new(),
        policy_id: String::new(),
        policy_version: String::new(),
        direction: ThresholdDirection::MinimumAtLeastThreshold,
        threshold_value: 0,
        sample_count: 0,
        candidate_admissions: 0,
        operational_admissions_after_hard_gate: 0,
        hard_gate_blocked_proposals: 0,
        unsupported_proposals: 0,
        hard_gate_bypass_attempts: 0,
        false_safe_proposals: 0,
        false_safe_admissions: 0,
        false_safe_admission_rate_wilson_95: None,
        false_abstentions: 0,
        realized_feasible_count: 0,
        false_abstention_rate_wilson_95: None,
        decision_flips: 0,
        replay_defined_count: 0,
        potentially_admissible_value_proxy_units: 0,
        risk_coverage: Vec::new(),
    };

    for (key, decisions) in evaluated {
        let mut group = DecisionImpactGroup {
            site_id: key.site_id,
            asset_id: key.asset_id,
            asset_configuration_digest: key.asset_configuration_digest,
            stratum_id: key.stratum_id,
            forecast_model_id: key.forecast_model_id,
            forecast_model_version: key.forecast_model_version,
            policy_id: key.policy_id,
            policy_version: key.policy_version,
            direction: key.direction,
            threshold_value: key.threshold_value,
            sample_count: decisions.len(),
            candidate_admissions: 0,
            operational_admissions_after_hard_gate: 0,
            hard_gate_blocked_proposals: 0,
            unsupported_proposals: 0,
            hard_gate_bypass_attempts: 0,
            false_safe_proposals: 0,
            false_safe_admissions: 0,
            false_safe_admission_rate_wilson_95: None,
            false_abstentions: 0,
            realized_feasible_count: 0,
            false_abstention_rate_wilson_95: None,
            decision_flips: 0,
            replay_defined_count: 0,
            potentially_admissible_value_proxy_units: 0,
            risk_coverage: Vec::new(),
        };

        for d in &decisions {
            if d.candidate_admit {
                group.candidate_admissions += 1;
                if d.hard_gate != HardGateDisposition::Passed {
                    group.hard_gate_bypass_attempts += 1;
                }
                if d.hard_gate == HardGateDisposition::Failed {
                    group.hard_gate_blocked_proposals += 1;
                }
                if d.hard_gate == HardGateDisposition::Unknown {
                    group.unsupported_proposals += 1;
                }
                if !d.realized_metric_admit {
                    group.false_safe_proposals += 1;
                }
            }
            if d.hard_gate == HardGateDisposition::Passed {
                group.replay_defined_count += 1;
                if d.realized_metric_admit {
                    group.realized_feasible_count += 1;
                }
                if d.candidate_admit {
                    group.operational_admissions_after_hard_gate += 1;
                    if !d.realized_metric_admit {
                        group.false_safe_admissions += 1;
                    }
                } else if d.realized_metric_admit {
                    group.false_abstentions += 1;
                    group.potentially_admissible_value_proxy_units = group
                        .potentially_admissible_value_proxy_units
                        .saturating_add(d.candidate_value_proxy_units);
                }
                if d.candidate_admit != d.realized_metric_admit {
                    group.decision_flips += 1;
                }
            }
        }

        group.false_safe_admission_rate_wilson_95 = wilson_interval(
            group.false_safe_admissions,
            group.operational_admissions_after_hard_gate,
        );
        group.false_abstention_rate_wilson_95 =
            wilson_interval(group.false_abstentions, group.realized_feasible_count);

        // A within-group threshold sweep gives a reproducible risk/coverage
        // frontier. Include explicit zero-admission and full-admission endpoints.
        // It is not a statistical guarantee and is never pooled across groups.
        let mut margins: Vec<i64> = decisions
            .iter()
            .filter(|d| d.hard_gate == HardGateDisposition::Passed)
            .map(|d| d.margin)
            .collect();
        margins.sort_unstable();
        margins.dedup();
        margins.reverse();
        if !margins.is_empty() {
            let zero_admission_margin = margins[0].saturating_add(1);
            let full_admission_margin = margins[margins.len() - 1].saturating_sub(1);
            margins.insert(0, zero_admission_margin);
            margins.push(full_admission_margin);
        }
        let replay_denominator = group.replay_defined_count;
        for minimum_margin in margins {
            let accepted: Vec<_> = decisions
                .iter()
                .filter(|d| {
                    d.hard_gate == HardGateDisposition::Passed && d.margin >= minimum_margin
                })
                .collect();
            let false_safe = accepted.iter().filter(|d| !d.realized_metric_admit).count();
            group.risk_coverage.push(DecisionRiskCoveragePoint {
                minimum_margin,
                accepted_proposals: accepted.len(),
                false_safe_proposals: false_safe,
                coverage_permille: permille(accepted.len(), replay_denominator),
                false_safe_rate_permille: permille(false_safe, accepted.len()),
            });
        }

        totals.sample_count += group.sample_count;
        totals.candidate_admissions += group.candidate_admissions;
        totals.operational_admissions_after_hard_gate +=
            group.operational_admissions_after_hard_gate;
        totals.hard_gate_blocked_proposals += group.hard_gate_blocked_proposals;
        totals.unsupported_proposals += group.unsupported_proposals;
        totals.hard_gate_bypass_attempts += group.hard_gate_bypass_attempts;
        totals.false_safe_proposals += group.false_safe_proposals;
        totals.false_safe_admissions += group.false_safe_admissions;
        totals.false_abstentions += group.false_abstentions;
        totals.realized_feasible_count += group.realized_feasible_count;
        totals.decision_flips += group.decision_flips;
        totals.replay_defined_count += group.replay_defined_count;
        totals.potentially_admissible_value_proxy_units = totals
            .potentially_admissible_value_proxy_units
            .saturating_add(group.potentially_admissible_value_proxy_units);
        groups.push(group);
    }

    Ok(ServiceDecisionImpactReport {
        schema_version: "0.1.0".to_owned(),
        benchmark_id: "SERVICE-DECISION-IMPACT-001".to_owned(),
        audit_cutoff_unix_ms: input.audit_cutoff_unix_ms,
        wilson_reference_assumes_iid_samples: true,
        record_count: totals.sample_count,
        candidate_admission_count: totals.candidate_admissions,
        operational_admission_count: totals.operational_admissions_after_hard_gate,
        hard_gate_blocked_proposal_count: totals.hard_gate_blocked_proposals,
        unsupported_proposal_count: totals.unsupported_proposals,
        hard_gate_bypass_attempt_count: totals.hard_gate_bypass_attempts,
        false_safe_proposal_count: totals.false_safe_proposals,
        false_safe_admission_count: totals.false_safe_admissions,
        false_abstention_count: totals.false_abstentions,
        decision_flip_count: totals.decision_flips,
        replay_defined_count: totals.replay_defined_count,
        potentially_admissible_value_proxy_units: totals.potentially_admissible_value_proxy_units,
        risk_coverage_group_count: groups.len(),
        all_evidence_source_grade,
        any_synthetic_evidence,
        input_digest,
        groups,
        field_qualification_granted: false,
        model_update_performed: false,
        operational_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(
        asset: &str,
        reference: &str,
        observed: u64,
        from: u64,
        until: u64,
    ) -> EvidenceEnvelope {
        EvidenceEnvelope {
            asset_id: asset.to_owned(),
            configuration_digest: "cfg-1".to_owned(),
            producer_id: "fixture-producer".to_owned(),
            method_id: "fixture-method".to_owned(),
            evidence_reference: reference.to_owned(),
            evidence_class: EvidenceClass::Synthetic,
            observed_at_unix_ms: Some(observed),
            valid_from_unix_ms: Some(from),
            valid_until_unix_ms: Some(until),
        }
    }

    fn record(
        id: &str,
        lower: u64,
        upper: u64,
        observed: u64,
        gate: HardGateDisposition,
    ) -> DecisionImpactRecord {
        DecisionImpactRecord {
            sample_id: id.to_owned(),
            site_id: "site-1".to_owned(),
            site_configuration_digest: "site-cfg-1".to_owned(),
            site_evidence: {
                let mut e = evidence("site-1", &format!("{id}-site"), 900, 800, 2_500);
                e.configuration_digest = "site-cfg-1".to_owned();
                e
            },
            asset_id: "ESS-01".to_owned(),
            asset_configuration_digest: "cfg-1".to_owned(),
            stratum_id: "warm".to_owned(),
            forecast_model_id: "model-1".to_owned(),
            forecast_model_version: "1".to_owned(),
            policy_id: "reserve-policy".to_owned(),
            policy_version: "1".to_owned(),
            decision_time_unix_ms: 1_000,
            target_start_unix_ms: 1_100,
            target_end_unix_ms: 2_000,
            direction: ThresholdDirection::MinimumAtLeastThreshold,
            threshold_value: 1_000,
            predicted_lower: lower,
            predicted_point: (lower + upper) / 2,
            predicted_upper: upper,
            observed_value: observed,
            hard_gate_disposition: gate,
            candidate_value_proxy_units: 10,
            forecast_evidence: evidence("ESS-01", &format!("{id}-forecast"), 900, 800, 2_500),
            hard_gate_evidence: evidence("ESS-01", &format!("{id}-gate"), 950, 800, 2_500),
            outcome_evidence: evidence("ESS-01", &format!("{id}-outcome"), 2_100, 2_000, 2_500),
        }
    }

    fn input(records: Vec<DecisionImpactRecord>) -> ServiceDecisionImpactInput {
        ServiceDecisionImpactInput {
            schema_version: "0.1.0".to_owned(),
            benchmark_id: "SERVICE-DECISION-IMPACT-001".to_owned(),
            audit_cutoff_unix_ms: 3_000,
            records,
            claim_ceiling: "synthetic test".to_owned(),
        }
    }

    #[test]
    fn detects_false_safe_admission_and_false_abstention_separately() {
        let input = input(vec![
            record("unsafe", 1_050, 1_100, 900, HardGateDisposition::Passed),
            record("abstain", 900, 1_100, 1_080, HardGateDisposition::Passed),
        ]);
        let report = evaluate_service_decision_impact(&input).unwrap();
        assert_eq!(report.false_safe_admission_count, 1);
        assert_eq!(report.false_abstention_count, 1);
        assert_eq!(report.decision_flip_count, 2);
        assert_eq!(report.potentially_admissible_value_proxy_units, 10);
    }

    #[test]
    fn failed_hard_gate_blocks_proposal_and_records_bypass_attempt() {
        let input = input(vec![record(
            "blocked",
            1_050,
            1_100,
            1_080,
            HardGateDisposition::Failed,
        )]);
        let report = evaluate_service_decision_impact(&input).unwrap();
        assert_eq!(report.hard_gate_blocked_proposal_count, 1);
        assert_eq!(report.hard_gate_bypass_attempt_count, 1);
        assert_eq!(report.operational_admission_count, 0);
        assert_eq!(report.false_safe_admission_count, 0);
    }

    #[test]
    fn unknown_hard_gate_is_unsupported_not_safe() {
        let input = input(vec![record(
            "unknown",
            1_050,
            1_100,
            1_080,
            HardGateDisposition::Unknown,
        )]);
        let report = evaluate_service_decision_impact(&input).unwrap();
        assert_eq!(report.unsupported_proposal_count, 1);
        assert_eq!(report.operational_admission_count, 0);
    }

    #[test]
    fn upper_bound_policy_uses_upper_forecast_and_actual_value() {
        let mut r = record("queue", 200, 250, 350, HardGateDisposition::Passed);
        r.asset_id = "CHG-01".to_owned();
        r.forecast_evidence.asset_id = "CHG-01".to_owned();
        r.hard_gate_evidence.asset_id = "CHG-01".to_owned();
        r.outcome_evidence.asset_id = "CHG-01".to_owned();
        r.direction = ThresholdDirection::MaximumAtMostThreshold;
        r.threshold_value = 300;
        let report = evaluate_service_decision_impact(&input(vec![r])).unwrap();
        assert_eq!(report.false_safe_admission_count, 1);
    }

    #[test]
    fn digest_is_invariant_to_record_order() {
        let a = record("a", 1_050, 1_100, 1_080, HardGateDisposition::Passed);
        let b = record("b", 900, 1_100, 1_080, HardGateDisposition::Passed);
        let forward = evaluate_service_decision_impact(&input(vec![a.clone(), b.clone()])).unwrap();
        let reverse = evaluate_service_decision_impact(&input(vec![b, a])).unwrap();
        assert_eq!(forward.input_digest, reverse.input_digest);
        assert_eq!(forward.groups, reverse.groups);
    }

    #[test]
    fn duplicate_evidence_reference_is_rejected() {
        let mut a = record("a", 1_050, 1_100, 1_080, HardGateDisposition::Passed);
        let mut b = record("b", 1_050, 1_100, 1_080, HardGateDisposition::Passed);
        b.forecast_evidence.evidence_reference = a.forecast_evidence.evidence_reference.clone();
        a.sample_id = "a".to_owned();
        b.sample_id = "b".to_owned();
        assert!(evaluate_service_decision_impact(&input(vec![a, b])).is_err());
    }

    #[test]
    fn synthetic_evidence_cannot_claim_field_qualification() {
        let report = evaluate_service_decision_impact(&input(vec![record(
            "synthetic",
            1_050,
            1_100,
            1_080,
            HardGateDisposition::Passed,
        )]))
        .unwrap();
        assert!(report.any_synthetic_evidence);
        assert!(!report.all_evidence_source_grade);
        assert!(!report.field_qualification_granted);
        assert!(!report.operational_authority_granted);
    }

    #[test]
    fn rejects_forecast_evidence_from_after_decision_time() {
        let mut r = record("late", 1_050, 1_100, 1_080, HardGateDisposition::Passed);
        r.forecast_evidence.observed_at_unix_ms = Some(1_001);
        assert!(evaluate_service_decision_impact(&input(vec![r])).is_err());
    }

    #[test]
    fn risk_coverage_frontier_is_group_local_and_monotone_in_coverage() {
        let input = input(vec![
            record("a", 1_050, 1_100, 900, HardGateDisposition::Passed),
            record("b", 1_020, 1_100, 1_080, HardGateDisposition::Passed),
            record("c", 900, 1_100, 1_080, HardGateDisposition::Passed),
        ]);
        let report = evaluate_service_decision_impact(&input).unwrap();
        let points = &report.groups[0].risk_coverage;
        assert!(points.len() >= 2);
        assert!(
            points
                .windows(2)
                .all(|w| w[0].coverage_permille <= w[1].coverage_permille)
        );
    }

    #[test]
    fn zero_service_value_does_not_relax_a_hard_gate() {
        let mut r = record("zero", 1_050, 1_100, 1_080, HardGateDisposition::Failed);
        r.candidate_value_proxy_units = 0;
        let report = evaluate_service_decision_impact(&input(vec![r])).unwrap();
        assert_eq!(report.operational_admission_count, 0);
        assert_eq!(report.hard_gate_bypass_attempt_count, 1);
    }
}
