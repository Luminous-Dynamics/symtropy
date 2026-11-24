// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Prequential temporal interval-risk monitor for mobility forecasts.
//!
//! At each forecast issue time, only outcomes whose evidence was available
//! strictly earlier may update the risk history. The anytime upper bound
//! concerns the average predictable historical interval-miscoverage rate
//! under bounded martingale differences; it is not a bound on the next
//! conditional error probability or arbitrary future distribution shift.
//! CUSUM is a deterministic shift alarm, not a probability guarantee.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::battery_service_network::{EvidenceClass, EvidenceEnvelope};

const MAX_RECORDS: usize = 5_000;
const MAX_VALUE: u64 = 1_000_000_000_000;
const MIN_COVERAGE_PERMILLE: u16 = 500;
const MAX_COVERAGE_PERMILLE: u16 = 990;
const MIN_ALPHA_PERMILLE: u16 = 1;
const MAX_ALPHA_PERMILLE: u16 = 200;
const CLAIM_CEILING: &str = "Prequential, asset/configuration-specific monitor of historical interval-miscoverage under bounded martingale differences. The anytime upper bound concerns average predictable risk over the observed sequence, not next-step conditional risk or arbitrary future shift. CUSUM is a deterministic alarm only. No model update, battery/flight/grid safety qualification, field qualification, dispatch or operational authority.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum OnlineForecastMetric {
    MissionDurationSeconds,
    PackTerminalEnergyWh,
    ChargeRestorationSeconds,
    ChargerInputEnergyWh,
    QueueWaitSeconds,
    SitePccPeakImportW,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum OnlineMeasurementBoundary {
    MissionClock,
    PackTerminal,
    ChargerServiceClock,
    ChargerInput,
    SitePccMeter,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrequentialRiskStatus {
    InsufficientHistory,
    HistoricalRiskWithinBudget,
    HistoricalRiskAboveBudget,
    DriftAlarmLatched,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OnlineRiskObservation {
    pub sample_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub asset_id: String,
    pub asset_configuration_digest: String,
    pub stratum_id: String,
    pub forecast_model_id: String,
    pub forecast_model_version: String,
    pub metric: OnlineForecastMetric,
    pub measurement_boundary: OnlineMeasurementBoundary,
    pub nominal_coverage_permille: u16,
    pub forecast_issued_unix_ms: u64,
    pub target_start_unix_ms: u64,
    pub target_end_unix_ms: u64,
    pub outcome_available_unix_ms: Option<u64>,
    pub predicted_lower: u64,
    pub predicted_upper: u64,
    pub observed_value: Option<u64>,
    pub forecast_evidence: EvidenceEnvelope,
    pub outcome_evidence: Option<EvidenceEnvelope>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OnlineServiceRiskInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub audit_cutoff_unix_ms: u64,
    /// Alpha in permille. A value of 50 is a 5% time-uniform error budget.
    pub alpha_permille: u16,
    /// Maximum historical mean interval-miscoverage budget.
    pub maximum_miscoverage_permille: u16,
    /// Minimum number of prior, already-observed outcomes before any status
    /// other than insufficient history can be emitted.
    pub minimum_prior_observations: u16,
    /// Slack above the maximum miscoverage budget before CUSUM accumulates.
    pub cusum_slack_permille: u16,
    /// CUSUM alarm threshold in permille-observation units.
    pub cusum_alarm_threshold: u64,
    pub observations: Vec<OnlineRiskObservation>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PrequentialAssessment {
    pub sample_id: String,
    pub forecast_issued_unix_ms: u64,
    pub prior_observation_count: usize,
    pub prior_interval_miss_count: usize,
    pub prior_miscoverage_upper_bound_permille: Option<u16>,
    pub prior_cusum_permille_observations: u64,
    pub drift_alarm_latched_before_forecast: bool,
    pub status: PrequentialRiskStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OnlineRiskGroupReport {
    pub site_id: String,
    pub site_configuration_digest: String,
    pub asset_id: String,
    pub asset_configuration_digest: String,
    pub stratum_id: String,
    pub forecast_model_id: String,
    pub forecast_model_version: String,
    pub metric: OnlineForecastMetric,
    pub measurement_boundary: OnlineMeasurementBoundary,
    pub nominal_coverage_permille: u16,
    pub forecast_count: usize,
    pub observed_outcome_count_at_audit_cutoff: usize,
    pub pending_outcome_count_at_audit_cutoff: usize,
    pub interval_miss_count_at_audit_cutoff: usize,
    pub empirical_miscoverage_permille_at_audit_cutoff: u16,
    pub anytime_miscoverage_upper_bound_permille_at_audit_cutoff: Option<u16>,
    pub cusum_permille_observations_at_audit_cutoff: u64,
    pub drift_alarm_latched: bool,
    pub historical_risk_upper_bound_exceeds_budget: bool,
    pub assessments: Vec<PrequentialAssessment>,
    pub final_status: PrequentialRiskStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OnlineServiceRiskReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub audit_cutoff_unix_ms: u64,
    pub alpha_permille: u16,
    pub maximum_miscoverage_permille: u16,
    pub forecast_count: usize,
    pub observed_outcome_count: usize,
    pub pending_outcome_count: usize,
    pub group_count: usize,
    pub groups_with_shift_alarm: usize,
    pub groups_with_risk_above_budget: usize,
    pub groups_with_insufficient_history: usize,
    pub all_evidence_source_grade: bool,
    pub any_synthetic_evidence: bool,
    pub input_digest: String,
    pub confidence_bound_kind: String,
    pub confidence_bound_assumption: String,
    pub cusum_is_probability_guarantee: bool,
    pub model_update_performed: bool,
    pub field_qualification_granted: bool,
    pub operational_authority_granted: bool,
    pub groups: Vec<OnlineRiskGroupReport>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    site_id: String,
    site_configuration_digest: String,
    asset_id: String,
    asset_configuration_digest: String,
    stratum_id: String,
    forecast_model_id: String,
    forecast_model_version: String,
    metric: OnlineForecastMetric,
    measurement_boundary: OnlineMeasurementBoundary,
    nominal_coverage_permille: u16,
}

#[derive(Debug, Clone, Copy)]
struct RiskState {
    count: usize,
    misses: usize,
    cusum: u64,
    alarm_latched: bool,
}

impl RiskState {
    fn new() -> Self {
        Self {
            count: 0,
            misses: 0,
            cusum: 0,
            alarm_latched: false,
        }
    }

    fn observe(
        &mut self,
        interval_miss: bool,
        maximum_miscoverage_permille: u16,
        cusum_slack_permille: u16,
        cusum_alarm_threshold: u64,
    ) -> Result<(), String> {
        self.count = self
            .count
            .checked_add(1)
            .ok_or_else(|| "observation_count_overflow".to_owned())?;
        if interval_miss {
            self.misses = self
                .misses
                .checked_add(1)
                .ok_or_else(|| "interval_miss_count_overflow".to_owned())?;
        }

        // CUSUM uses integer permille-observation units. It is deliberately
        // a separate deterministic alarm, not part of the confidence bound.
        let expected_plus_slack =
            u64::from(maximum_miscoverage_permille) + u64::from(cusum_slack_permille);
        let observed = if interval_miss { 1_000_u64 } else { 0 };
        if observed >= expected_plus_slack {
            self.cusum = self
                .cusum
                .checked_add(observed - expected_plus_slack)
                .ok_or_else(|| "cusum_overflow".to_owned())?;
        } else {
            self.cusum = self.cusum.saturating_sub(expected_plus_slack - observed);
        }
        if self.cusum >= cusum_alarm_threshold {
            self.alarm_latched = true;
        }
        Ok(())
    }
}

fn expected_boundary(metric: OnlineForecastMetric) -> OnlineMeasurementBoundary {
    match metric {
        OnlineForecastMetric::MissionDurationSeconds => OnlineMeasurementBoundary::MissionClock,
        OnlineForecastMetric::PackTerminalEnergyWh => OnlineMeasurementBoundary::PackTerminal,
        OnlineForecastMetric::ChargeRestorationSeconds
        | OnlineForecastMetric::QueueWaitSeconds => OnlineMeasurementBoundary::ChargerServiceClock,
        OnlineForecastMetric::ChargerInputEnergyWh => OnlineMeasurementBoundary::ChargerInput,
        OnlineForecastMetric::SitePccPeakImportW => OnlineMeasurementBoundary::SitePccMeter,
    }
}

fn evidence_well_formed(evidence: &EvidenceEnvelope) -> bool {
    !evidence.asset_id.trim().is_empty()
        && evidence.asset_id.len() <= 128
        && !evidence.configuration_digest.trim().is_empty()
        && evidence.configuration_digest.len() <= 256
        && !evidence.producer_id.trim().is_empty()
        && evidence.producer_id.len() <= 128
        && !evidence.method_id.trim().is_empty()
        && evidence.method_id.len() <= 128
        && !evidence.evidence_reference.trim().is_empty()
        && evidence.evidence_reference.len() <= 256
        && evidence.evidence_class != EvidenceClass::Unknown
}

fn forecast_evidence_valid(record: &OnlineRiskObservation) -> bool {
    let evidence = &record.forecast_evidence;
    if !evidence_well_formed(evidence)
        || evidence.asset_id != record.asset_id
        || evidence.configuration_digest != record.asset_configuration_digest
    {
        return false;
    }
    matches!(
        (
            evidence.observed_at_unix_ms,
            evidence.valid_from_unix_ms,
            evidence.valid_until_unix_ms
        ),
        (Some(observed), Some(valid_from), Some(valid_until))
            if valid_from <= observed
                && observed <= record.forecast_issued_unix_ms
                && valid_from <= record.forecast_issued_unix_ms
                && record.target_end_unix_ms <= valid_until
    )
}

fn outcome_evidence_valid(record: &OnlineRiskObservation, audit_cutoff: u64) -> bool {
    let (Some(available), Some(_observed_value), Some(evidence)) = (
        record.outcome_available_unix_ms,
        record.observed_value,
        record.outcome_evidence.as_ref(),
    ) else {
        return false;
    };
    if !evidence_well_formed(evidence)
        || evidence.asset_id != record.asset_id
        || evidence.configuration_digest != record.asset_configuration_digest
    {
        return false;
    }
    matches!(
        (
            evidence.observed_at_unix_ms,
            evidence.valid_from_unix_ms,
            evidence.valid_until_unix_ms
        ),
        (Some(observed), Some(valid_from), Some(valid_until))
            if observed == available
                && observed >= record.target_end_unix_ms
                && observed <= audit_cutoff
                && valid_from <= record.target_end_unix_ms
                && observed <= valid_until
    )
}

fn group_key(record: &OnlineRiskObservation) -> GroupKey {
    GroupKey {
        site_id: record.site_id.clone(),
        site_configuration_digest: record.site_configuration_digest.clone(),
        asset_id: record.asset_id.clone(),
        asset_configuration_digest: record.asset_configuration_digest.clone(),
        stratum_id: record.stratum_id.clone(),
        forecast_model_id: record.forecast_model_id.clone(),
        forecast_model_version: record.forecast_model_version.clone(),
        metric: record.metric,
        measurement_boundary: record.measurement_boundary,
        nominal_coverage_permille: record.nominal_coverage_permille,
    }
}

fn is_interval_miss(record: &OnlineRiskObservation) -> bool {
    let observed = record
        .observed_value
        .expect("only complete outcomes enter the risk queue");
    observed < record.predicted_lower || observed > record.predicted_upper
}

fn miscoverage_upper_bound_permille(
    misses: usize,
    count: usize,
    alpha_permille: u16,
) -> Option<u16> {
    if count == 0 || misses > count {
        return None;
    }
    let n = count as f64;
    // alpha_n = alpha * 6 / (pi^2 n^2). Since sum(1/n^2)=pi^2/6,
    // the union-bound expenditure across all n is at most alpha.
    let alpha = f64::from(alpha_permille) / 1_000.0;
    let delta_n = alpha * 6.0 / (std::f64::consts::PI.powi(2) * n * n);
    let empirical = misses as f64 / n;
    let radius = (-delta_n.ln() / (2.0 * n)).sqrt();
    let upper = ((empirical + radius).min(1.0) * 1_000.0).ceil();
    Some(upper.clamp(0.0, 1_000.0) as u16)
}

fn status(
    state: RiskState,
    input: &OnlineServiceRiskInput,
) -> PrequentialRiskStatus {
    if state.count < usize::from(input.minimum_prior_observations) {
        PrequentialRiskStatus::InsufficientHistory
    } else if state.alarm_latched {
        PrequentialRiskStatus::DriftAlarmLatched
    } else if miscoverage_upper_bound_permille(
        state.misses,
        state.count,
        input.alpha_permille,
    )
    .is_some_and(|upper| upper > input.maximum_miscoverage_permille)
    {
        PrequentialRiskStatus::HistoricalRiskAboveBudget
    } else {
        PrequentialRiskStatus::HistoricalRiskWithinBudget
    }
}

fn validate(input: &OnlineServiceRiskInput) -> Result<(bool, bool), Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "ONLINE-SERVICE-RISK-CERTIFICATE-001"
        || input.audit_cutoff_unix_ms == 0
        || !(MIN_ALPHA_PERMILLE..=MAX_ALPHA_PERMILLE).contains(&input.alpha_permille)
        || input.maximum_miscoverage_permille == 0
        || input.maximum_miscoverage_permille > 500
        || input.minimum_prior_observations < 2
        || input.minimum_prior_observations > 1_000
        || input.cusum_slack_permille > 250
        || input.cusum_alarm_threshold == 0
        || input.cusum_alarm_threshold > 10_000_000
        || input.observations.is_empty()
        || input.observations.len() > MAX_RECORDS
        || input.claim_ceiling.trim().is_empty()
    {
        errors.push("online_risk_identity_or_policy_bounds_invalid".to_owned());
    }

    let mut sample_ids = BTreeSet::new();
    let mut evidence_refs = BTreeSet::new();
    let mut all_source_grade = true;
    let mut any_synthetic = false;
    for record in &input.observations {
        if record.sample_id.trim().is_empty()
            || record.sample_id.len() > 128
            || record.site_id.trim().is_empty()
            || record.site_id.len() > 128
            || record.site_configuration_digest.trim().is_empty()
            || record.site_configuration_digest.len() > 256
            || record.asset_id.trim().is_empty()
            || record.asset_id.len() > 128
            || record.asset_configuration_digest.trim().is_empty()
            || record.asset_configuration_digest.len() > 256
            || record.stratum_id.trim().is_empty()
            || record.stratum_id.len() > 96
            || record.forecast_model_id.trim().is_empty()
            || record.forecast_model_id.len() > 96
            || record.forecast_model_version.trim().is_empty()
            || record.forecast_model_version.len() > 64
            || record.forecast_issued_unix_ms == 0
            || record.forecast_issued_unix_ms >= record.target_start_unix_ms
            || record.target_start_unix_ms >= record.target_end_unix_ms
            || record.measurement_boundary != expected_boundary(record.metric)
            || !(MIN_COVERAGE_PERMILLE..=MAX_COVERAGE_PERMILLE)
                .contains(&record.nominal_coverage_permille)
            || record.predicted_lower > record.predicted_upper
            || record.predicted_upper > MAX_VALUE
            || record.observed_value.is_some_and(|value| value > MAX_VALUE)
        {
            errors.push(format!("online_record_identity_or_time_invalid:{}", record.sample_id));
        }
        if !sample_ids.insert(record.sample_id.as_str()) {
            errors.push(format!("online_sample_id_duplicate:{}", record.sample_id));
        }
        if !forecast_evidence_valid(record) {
            errors.push(format!("online_forecast_evidence_invalid:{}", record.sample_id));
        }
        let outcome_presence = (
            record.outcome_available_unix_ms.is_some(),
            record.observed_value.is_some(),
            record.outcome_evidence.is_some(),
        );
        match outcome_presence {
            (false, false, false) => {}
            (true, true, true) => {
                let available = record.outcome_available_unix_ms.expect("presence checked");
                if available < record.target_end_unix_ms
                    || available <= record.forecast_issued_unix_ms
                    || available > input.audit_cutoff_unix_ms
                    || !outcome_evidence_valid(record, input.audit_cutoff_unix_ms)
                {
                    errors.push(format!("online_outcome_evidence_invalid:{}", record.sample_id));
                }
            }
            _ => errors.push(format!("online_outcome_fields_partial:{}", record.sample_id)),
        }

        let mut record_evidence = vec![&record.forecast_evidence];
        if let Some(outcome_evidence) = record.outcome_evidence.as_ref() {
            record_evidence.push(outcome_evidence);
        }
        for evidence in record_evidence {
            if evidence.evidence_reference.trim().is_empty()
                || !evidence_refs.insert(evidence.evidence_reference.as_str())
            {
                errors.push(format!(
                    "online_evidence_reference_empty_or_reused:{}",
                    evidence.evidence_reference
                ));
            }
            all_source_grade &= matches!(
                evidence.evidence_class,
                EvidenceClass::Measured | EvidenceClass::IndependentTest
            );
            any_synthetic |= evidence.evidence_class == EvidenceClass::Synthetic;
        }
    }
    if errors.is_empty() {
        Ok((all_source_grade, any_synthetic))
    } else {
        errors.sort();
        errors.dedup();
        Err(errors)
    }
}

fn canonical_input_digest(input: &OnlineServiceRiskInput) -> Result<String, String> {
    let mut canonical = input.clone();
    canonical
        .observations
        .sort_by(|a, b| a.sample_id.cmp(&b.sample_id));
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("online_risk_serialization_failed:{error}"))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

pub fn evaluate_online_service_risk(
    input: &OnlineServiceRiskInput,
) -> Result<OnlineServiceRiskReport, Vec<String>> {
    let (all_evidence_source_grade, any_synthetic_evidence) = validate(input)?;
    let input_digest =
        canonical_input_digest(input).map_err(|error| vec![error])?;

    let mut grouped: BTreeMap<GroupKey, Vec<&OnlineRiskObservation>> = BTreeMap::new();
    for observation in &input.observations {
        grouped
            .entry(group_key(observation))
            .or_default()
            .push(observation);
    }

    let mut groups = Vec::with_capacity(grouped.len());
    for (key, mut records) in grouped {
        records.sort_by(|a, b| {
            (a.forecast_issued_unix_ms, a.sample_id.as_str())
                .cmp(&(b.forecast_issued_unix_ms, b.sample_id.as_str()))
        });
        if records.windows(2).any(|window| {
            window[0].forecast_issued_unix_ms == window[1].forecast_issued_unix_ms
        }) {
            return Err(vec![format!(
                "online_forecast_issue_timestamp_ambiguous:{}:{}",
                key.site_id, key.asset_id
            )]);
        }

        let mut state = RiskState::new();
        let mut pending: BTreeMap<(u64, String), bool> = BTreeMap::new();
        let mut pending_without_outcome = 0_usize;
        let mut assessments = Vec::with_capacity(records.len());

        for record in &records {
            let ready: Vec<_> = pending
                .range(..(record.forecast_issued_unix_ms, String::new()))
                .map(|(key, miss)| (key.clone(), *miss))
                .collect();
            for (key, miss) in ready {
                pending.remove(&key);
                state
                    .observe(
                        miss,
                        input.maximum_miscoverage_permille,
                        input.cusum_slack_permille,
                        input.cusum_alarm_threshold,
                    )
                    .map_err(|error| vec![error])?;
            }

            let prior_upper =
                miscoverage_upper_bound_permille(state.misses, state.count, input.alpha_permille);
            assessments.push(PrequentialAssessment {
                sample_id: record.sample_id.clone(),
                forecast_issued_unix_ms: record.forecast_issued_unix_ms,
                prior_observation_count: state.count,
                prior_interval_miss_count: state.misses,
                prior_miscoverage_upper_bound_permille: prior_upper,
                prior_cusum_permille_observations: state.cusum,
                drift_alarm_latched_before_forecast: state.alarm_latched,
                status: status(state, input),
            });
            match record.outcome_available_unix_ms {
                Some(available) => {
                    pending.insert(
                        (available, record.sample_id.clone()),
                        is_interval_miss(record),
                    );
                }
                None => pending_without_outcome += 1,
            }
        }

        for miss in pending.values().copied() {
            state
                .observe(
                    miss,
                    input.maximum_miscoverage_permille,
                    input.cusum_slack_permille,
                    input.cusum_alarm_threshold,
                )
                .map_err(|error| vec![error])?;
        }

        let final_upper =
            miscoverage_upper_bound_permille(state.misses, state.count, input.alpha_permille);
        let empirical = if state.count == 0 {
            0
        } else {
            ((state.misses as u128 * 1_000) / state.count as u128) as u16
        };
        let final_status = status(state, input);
        groups.push(OnlineRiskGroupReport {
            site_id: key.site_id,
            site_configuration_digest: key.site_configuration_digest,
            asset_id: key.asset_id,
            asset_configuration_digest: key.asset_configuration_digest,
            stratum_id: key.stratum_id,
            forecast_model_id: key.forecast_model_id,
            forecast_model_version: key.forecast_model_version,
            metric: key.metric,
            measurement_boundary: key.measurement_boundary,
            nominal_coverage_permille: key.nominal_coverage_permille,
            forecast_count: records.len(),
            observed_outcome_count_at_audit_cutoff: state.count,
            pending_outcome_count_at_audit_cutoff: pending_without_outcome,
            interval_miss_count_at_audit_cutoff: state.misses,
            empirical_miscoverage_permille_at_audit_cutoff: empirical,
            anytime_miscoverage_upper_bound_permille_at_audit_cutoff: final_upper,
            cusum_permille_observations_at_audit_cutoff: state.cusum,
            drift_alarm_latched: state.alarm_latched,
            historical_risk_upper_bound_exceeds_budget: final_upper
                .is_some_and(|upper| upper > input.maximum_miscoverage_permille),
            assessments,
            final_status,
        });
    }

    let groups_with_shift_alarm = groups.iter().filter(|g| g.drift_alarm_latched).count();
    let groups_with_risk_above_budget = groups
        .iter()
        .filter(|g| g.historical_risk_upper_bound_exceeds_budget)
        .count();
    let groups_with_insufficient_history = groups
        .iter()
        .filter(|g| g.final_status == PrequentialRiskStatus::InsufficientHistory)
        .count();

    Ok(OnlineServiceRiskReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        audit_cutoff_unix_ms: input.audit_cutoff_unix_ms,
        alpha_permille: input.alpha_permille,
        maximum_miscoverage_permille: input.maximum_miscoverage_permille,
        forecast_count: input.observations.len(),
        observed_outcome_count: groups.iter().map(|group| group.observed_outcome_count_at_audit_cutoff).sum(),
        pending_outcome_count: groups.iter().map(|group| group.pending_outcome_count_at_audit_cutoff).sum(),
        group_count: groups.len(),
        groups_with_shift_alarm,
        groups_with_risk_above_budget,
        groups_with_insufficient_history,
        all_evidence_source_grade,
        any_synthetic_evidence,
        input_digest,
        confidence_bound_kind: "one_sided_time_uniform_hoeffding_azuma_martingale_upper_bound".to_owned(),
        confidence_bound_assumption: "Each interval-miss indicator is bounded in [0,1]; conditional expectations are taken with respect to the pre-issuance filtration; the martingale-difference sequence obeys the bounded-increment Hoeffding-Azuma assumptions. Union-bound allocation alpha_n=alpha*6/(pi^2*n^2) gives simultaneous coverage over observed prefix lengths. Bound targets average predictable historical miscoverage, not next-step conditional risk or arbitrary future shift.".to_owned(),
        cusum_is_probability_guarantee: false,
        model_update_performed: false,
        field_qualification_granted: false,
        operational_authority_granted: false,
        groups,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(asset: &str, cfg: &str, reference: &str, observed: u64, from: u64, until: u64) -> EvidenceEnvelope {
        EvidenceEnvelope {
            asset_id: asset.to_owned(),
            configuration_digest: cfg.to_owned(),
            producer_id: "synthetic-online-fixture".to_owned(),
            method_id: "prequential-reference-v1".to_owned(),
            evidence_reference: reference.to_owned(),
            evidence_class: EvidenceClass::Synthetic,
            observed_at_unix_ms: Some(observed),
            valid_from_unix_ms: Some(from),
            valid_until_unix_ms: Some(until),
        }
    }

    fn observation(
        id: &str,
        issued: u64,
        target_start: u64,
        target_end: u64,
        available: u64,
        miss: bool,
        config: &str,
    ) -> OnlineRiskObservation {
        let lower = 100;
        let upper = 200;
        let observed = if miss { 250 } else { 150 };
        OnlineRiskObservation {
            sample_id: id.to_owned(),
            site_id: "depot-01".to_owned(),
            site_configuration_digest: "site-config-1".to_owned(),
            asset_id: "ESS-01".to_owned(),
            asset_configuration_digest: config.to_owned(),
            stratum_id: "warm-lfp".to_owned(),
            forecast_model_id: "energy-model".to_owned(),
            forecast_model_version: "v1".to_owned(),
            metric: OnlineForecastMetric::PackTerminalEnergyWh,
            measurement_boundary: OnlineMeasurementBoundary::PackTerminal,
            nominal_coverage_permille: 900,
            forecast_issued_unix_ms: issued,
            target_start_unix_ms: target_start,
            target_end_unix_ms: target_end,
            outcome_available_unix_ms: Some(available),
            predicted_lower: lower,
            predicted_upper: upper,
            observed_value: Some(observed),
            forecast_evidence: evidence("ESS-01", config, &format!("{id}-forecast"), issued - 10, issued - 20, target_end + 2_000),
            outcome_evidence: Some(evidence("ESS-01", config, &format!("{id}-outcome"), available, target_end, available + 100)),
        }
    }

    fn input(observations: Vec<OnlineRiskObservation>) -> OnlineServiceRiskInput {
        OnlineServiceRiskInput {
            schema_version: "0.1.0".to_owned(),
            benchmark_id: "ONLINE-SERVICE-RISK-CERTIFICATE-001".to_owned(),
            audit_cutoff_unix_ms: 10_000,
            alpha_permille: 50,
            maximum_miscoverage_permille: 100,
            minimum_prior_observations: 2,
            cusum_slack_permille: 50,
            cusum_alarm_threshold: 1_500,
            observations,
            claim_ceiling: "synthetic prequential monitor only".to_owned(),
        }
    }

    #[test]
    fn outcomes_are_not_used_before_they_become_available() {
        let report = evaluate_online_service_risk(&input(vec![
            observation("a", 1_000, 1_100, 1_500, 1_800, true, "cfg-1"),
            observation("b", 1_200, 1_300, 1_600, 1_900, true, "cfg-1"),
            observation("c", 1_850, 1_900, 2_000, 2_300, false, "cfg-1"),
        ]))
        .unwrap();
        let assessments = &report.groups[0].assessments;
        assert_eq!(assessments[0].prior_observation_count, 0);
        assert_eq!(assessments[1].prior_observation_count, 0);
        assert_eq!(assessments[2].prior_observation_count, 1);
        assert_eq!(report.groups[0].observed_outcome_count_at_audit_cutoff, 3);
        assert_eq!(report.groups[0].interval_miss_count_at_audit_cutoff, 2);
    }

    #[test]
    fn later_issued_forecast_outcome_can_arrive_before_older_pending_outcome() {
        let report = evaluate_online_service_risk(&input(vec![
            observation("a", 1_000, 1_100, 1_500, 5_000, false, "cfg-1"),
            observation("b", 2_000, 2_100, 2_300, 2_500, false, "cfg-1"),
            observation("c", 3_000, 3_100, 3_300, 3_500, false, "cfg-1"),
        ]))
        .unwrap();
        let assessments = &report.groups[0].assessments;
        assert_eq!(assessments[1].prior_observation_count, 0);
        assert_eq!(assessments[2].prior_observation_count, 1);
    }

    #[test]
    fn forecast_without_outcome_remains_visible_as_pending() {
        let mut pending = observation("pending", 1_000, 1_100, 1_500, 1_600, false, "cfg-1");
        pending.outcome_available_unix_ms = None;
        pending.observed_value = None;
        pending.outcome_evidence = None;
        let report = evaluate_online_service_risk(&input(vec![pending])).unwrap();
        assert_eq!(report.forecast_count, 1);
        assert_eq!(report.observed_outcome_count, 0);
        assert_eq!(report.pending_outcome_count, 1);
        assert_eq!(report.groups[0].forecast_count, 1);
        assert_eq!(report.groups[0].observed_outcome_count_at_audit_cutoff, 0);
        assert_eq!(report.groups[0].pending_outcome_count_at_audit_cutoff, 1);
    }

    #[test]
    fn repeated_misses_latch_the_separate_cusum_drift_alarm() {
        let report = evaluate_online_service_risk(&input(vec![
            observation("a", 1_000, 1_100, 1_500, 1_600, true, "cfg-1"),
            observation("b", 1_700, 1_800, 2_000, 2_100, true, "cfg-1"),
            observation("c", 2_200, 2_300, 2_500, 2_600, false, "cfg-1"),
        ]))
        .unwrap();
        assert!(report.groups[0].drift_alarm_latched);
        assert!(report.groups[0].assessments[2].drift_alarm_latched_before_forecast);
        assert_eq!(
            report.groups[0].assessments[2].status,
            PrequentialRiskStatus::DriftAlarmLatched
        );
    }

    #[test]
    fn new_configuration_starts_a_separate_history() {
        let report = evaluate_online_service_risk(&input(vec![
            observation("a", 1_000, 1_100, 1_500, 1_600, false, "cfg-1"),
            observation("b", 1_700, 1_800, 2_000, 2_100, false, "cfg-2"),
        ]))
        .unwrap();
        assert_eq!(report.group_count, 2);
        assert!(report.groups.iter().all(|g| g.observed_outcome_count_at_audit_cutoff == 1));
        assert!(report.groups.iter().all(|g| g.final_status == PrequentialRiskStatus::InsufficientHistory));
    }

    #[test]
    fn record_order_does_not_change_digest_or_assessments() {
        let a = observation("a", 1_000, 1_100, 1_500, 1_800, true, "cfg-1");
        let b = observation("b", 1_700, 1_800, 2_000, 2_100, false, "cfg-1");
        let first = evaluate_online_service_risk(&input(vec![a.clone(), b.clone()])).unwrap();
        let second = evaluate_online_service_risk(&input(vec![b, a])).unwrap();
        assert_eq!(first.input_digest, second.input_digest);
        assert_eq!(first.groups, second.groups);
    }

    #[test]
    fn rejects_outcome_available_before_target_end_or_at_forecast_time() {
        let mut bad = observation("bad", 1_000, 1_100, 1_500, 1_600, false, "cfg-1");
        bad.outcome_available_unix_ms = Some(1_000);
        bad.outcome_evidence.as_mut().unwrap().observed_at_unix_ms = Some(1_000);
        assert!(evaluate_online_service_risk(&input(vec![bad])).is_err());
    }

    #[test]
    fn rejects_forecast_evidence_observed_after_forecast_issuance() {
        let mut bad = observation("bad", 1_000, 1_100, 1_500, 1_600, false, "cfg-1");
        bad.forecast_evidence.observed_at_unix_ms = Some(1_001);
        assert!(evaluate_online_service_risk(&input(vec![bad])).is_err());
    }

    #[test]
    fn duplicate_evidence_reference_is_rejected() {
        let mut a = observation("a", 1_000, 1_100, 1_500, 1_600, false, "cfg-1");
        let mut b = observation("b", 1_700, 1_800, 2_000, 2_100, false, "cfg-1");
        b.outcome_evidence.as_mut().unwrap().evidence_reference = a.forecast_evidence.evidence_reference.clone();
        assert!(evaluate_online_service_risk(&input(vec![a, b])).is_err());
    }

    #[test]
    fn interval_endpoints_are_covered() {
        let mut a = observation("a", 1_000, 1_100, 1_500, 1_600, false, "cfg-1");
        a.observed_value = Some(a.predicted_lower);
        let mut b = observation("b", 1_700, 1_800, 2_000, 2_100, false, "cfg-1");
        b.observed_value = Some(b.predicted_upper);
        let report = evaluate_online_service_risk(&input(vec![a, b])).unwrap();
        assert_eq!(report.groups[0].interval_miss_count_at_audit_cutoff, 0);
    }

    #[test]
    fn confidence_bound_is_never_below_empirical_miscoverage() {
        for count in [2, 5, 20, 100, 1_000] {
            let upper = miscoverage_upper_bound_permille(1, count, 50).unwrap();
            let empirical = ((1_u128 * 1_000) / count as u128) as u16;
            assert!(upper >= empirical);
        }
    }

    #[test]
    fn drift_alarm_and_historical_risk_bound_are_reported_independently() {
        let report = evaluate_online_service_risk(&input(vec![
            observation("a", 1_000, 1_100, 1_500, 1_600, true, "cfg-1"),
            observation("b", 1_700, 1_800, 2_000, 2_100, true, "cfg-1"),
        ]))
        .unwrap();
        let group = &report.groups[0];
        assert!(group.drift_alarm_latched);
        assert!(group.historical_risk_upper_bound_exceeds_budget);
        assert_eq!(report.groups_with_shift_alarm, 1);
        assert_eq!(report.groups_with_risk_above_budget, 1);
    }

    #[test]
    fn source_grade_flags_are_never_granted_by_synthetic_fixture() {
        let report = evaluate_online_service_risk(&input(vec![
            observation("a", 1_000, 1_100, 1_500, 1_600, false, "cfg-1"),
        ]))
        .unwrap();
        assert!(report.any_synthetic_evidence);
        assert!(!report.all_evidence_source_grade);
        assert!(!report.model_update_performed);
        assert!(!report.field_qualification_granted);
        assert!(!report.operational_authority_granted);
        assert!(!report.cusum_is_probability_guarantee);
    }

    #[test]
    fn equal_issue_timestamps_in_one_group_are_rejected() {
        let a = observation("a", 1_000, 1_100, 1_500, 1_600, false, "cfg-1");
        let b = observation("b", 1_000, 1_200, 1_700, 1_800, false, "cfg-1");
        assert!(evaluate_online_service_risk(&input(vec![a, b])).is_err());
    }

    #[test]
    fn input_rejects_measurement_boundary_mismatch() {
        let mut bad = observation("bad", 1_000, 1_100, 1_500, 1_600, false, "cfg-1");
        bad.measurement_boundary = OnlineMeasurementBoundary::ChargerInput;
        assert!(evaluate_online_service_risk(&input(vec![bad])).is_err());
    }
}
