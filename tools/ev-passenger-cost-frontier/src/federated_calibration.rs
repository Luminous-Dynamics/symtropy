// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded deterministic aggregation of site-local calibration summaries.
//!
//! This module implements merge, compatibility, contribution, cohort and
//! release-ledger preflight semantics only. It does not authenticate member
//! credentials, perform secure aggregation, provide differential privacy, or
//! allow a fleet result to qualify a local site.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_SUMMARIES: usize = 64;
const MAX_BINS: usize = 64;
const MAX_SAMPLES_PER_SITE: u64 = 10_000;
const MAX_BIN_COUNT_PER_SITE: u64 = 10_000;
const MAX_COVERAGE_PERMILLE: u16 = 990;
const MIN_COVERAGE_PERMILLE: u16 = 500;
const MAX_WIDTH_SUM: u64 = 1_000_000_000_000;
const CLAIM_CEILING: &str = "Bounded deterministic summary merge only. Membership authenticity, secure aggregation, differential privacy, site-local calibration validity, field qualification and operational authority are not established.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct FederatedCompatibilityKey {
    pub forecast_model_id: String,
    pub forecast_model_version: String,
    pub forecast_horizon_ms: u64,
    pub metric_kind: String,
    pub measurement_boundary: String,
    pub unit: String,
    pub stratum_id: String,
    pub nominal_coverage_permille: u16,
    pub score_definition_id: String,
    pub bin_edges: Vec<u64>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalCalibrationStatus {
    LocallySupported,
    UndercoverageAlarm,
    InsufficientLocalSupport,
    UnsupportedStratum,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SiteCalibrationSummary {
    pub site_id: String,
    pub epoch_id: String,
    pub compatibility: FederatedCompatibilityKey,
    pub calibration_sample_count: u64,
    pub calibration_score_bin_counts: Vec<u64>,
    pub holdout_sample_count: u64,
    pub holdout_covered_count: u64,
    pub holdout_interval_width_sum: u64,
    pub local_status: LocalCalibrationStatus,
    pub local_audit_cutoff_unix_ms: u64,
    pub local_evidence_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SiteReleaseLedgerSnapshot {
    pub site_id: String,
    pub epoch_id: String,
    pub releases_in_epoch: u16,
    pub maximum_releases_in_epoch: u16,
    pub previously_released_query_digests: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FederatedCalibrationInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub epoch_id: String,
    pub aggregation_time_unix_ms: u64,
    pub maximum_summary_age_ms: u64,
    pub query_scope_digest: String,
    pub compatibility: FederatedCompatibilityKey,
    pub registered_site_ids: Vec<String>,
    pub minimum_participating_sites: usize,
    pub minimum_total_calibration_samples: u64,
    pub maximum_samples_per_site: u64,
    pub maximum_count_per_bin_per_site: u64,
    pub release_ledger_snapshot: Vec<SiteReleaseLedgerSnapshot>,
    pub summaries: Vec<SiteCalibrationSummary>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FederatedAggregationStatus {
    Published,
    SuppressedMinimumCohort,
    SuppressedMinimumSamples,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FederatedCalibrationReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub epoch_id: String,
    pub query_scope_digest: String,
    pub compatibility: FederatedCompatibilityKey,
    pub status: FederatedAggregationStatus,
    pub participating_site_count: Option<usize>,
    pub total_calibration_sample_count: Option<u64>,
    pub total_holdout_sample_count: Option<u64>,
    pub total_holdout_covered_count: Option<u64>,
    pub holdout_coverage_permille: Option<u16>,
    pub total_holdout_interval_width_sum: Option<u64>,
    pub mean_holdout_interval_width: Option<u64>,
    pub merged_calibration_score_bin_counts: Option<Vec<u64>>,
    pub conformal_quantile_rank: Option<u64>,
    pub conformal_quantile_upper_edge: Option<u64>,
    pub local_undercoverage_alarm_site_count: Option<usize>,
    pub local_insufficient_support_site_count: Option<usize>,
    pub local_unsupported_or_unknown_site_count: Option<usize>,
    pub release_budget_admissible: bool,
    pub release_budget_consumed: bool,
    pub input_digest: String,
    pub membership_cryptographically_verified: bool,
    pub secure_aggregation_performed: bool,
    pub differential_privacy_applied: bool,
    pub site_local_qualification_granted: bool,
    pub fleet_field_qualification_granted: bool,
    pub operational_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn quantile_rank(sample_count: u64, nominal_coverage_permille: u16) -> u64 {
    let numerator = u128::from(sample_count + 1) * u128::from(nominal_coverage_permille);
    let rank = numerator.div_ceil(1000).max(1);
    u64::try_from(rank).unwrap_or(u64::MAX).min(sample_count).max(1)
}

fn resolve_quantile_upper_edge(
    bin_counts: &[u64],
    bin_edges: &[u64],
    rank: u64,
) -> Option<u64> {
    let mut cumulative = 0_u64;
    for (index, count) in bin_counts.iter().enumerate() {
        cumulative = cumulative.checked_add(*count)?;
        if cumulative >= rank {
            return bin_edges.get(index + 1).copied();
        }
    }
    None
}

fn validate(input: &FederatedCalibrationInput) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0" {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if input.benchmark_id != "FEDERATED-SERVICE-CALIBRATION-001" {
        errors.push("benchmark_id must be FEDERATED-SERVICE-CALIBRATION-001".to_owned());
    }
    if !non_empty(&input.epoch_id) || !non_empty(&input.query_scope_digest) {
        errors.push("epoch_id and query_scope_digest must be non-empty".to_owned());
    }
    if input.aggregation_time_unix_ms == 0 || input.maximum_summary_age_ms == 0 {
        errors.push("aggregation time and maximum summary age must be positive".to_owned());
    }
    if input.minimum_participating_sites < 2
        || input.minimum_participating_sites > MAX_SUMMARIES
        || input.minimum_total_calibration_samples == 0
        || input.maximum_samples_per_site == 0
        || input.maximum_samples_per_site > MAX_SAMPLES_PER_SITE
        || input.maximum_count_per_bin_per_site == 0
        || input.maximum_count_per_bin_per_site > MAX_BIN_COUNT_PER_SITE
    {
        errors.push("cohort and contribution limits are outside v0.1 bounds".to_owned());
    }
    if input.claim_ceiling.trim().is_empty() {
        errors.push("claim_ceiling must be non-empty".to_owned());
    }
    let key = &input.compatibility;
    for (name, value) in [
        ("forecast_model_id", key.forecast_model_id.as_str()),
        ("forecast_model_version", key.forecast_model_version.as_str()),
        ("metric_kind", key.metric_kind.as_str()),
        ("measurement_boundary", key.measurement_boundary.as_str()),
        ("unit", key.unit.as_str()),
        ("stratum_id", key.stratum_id.as_str()),
        ("score_definition_id", key.score_definition_id.as_str()),
    ] {
        if !non_empty(value) {
            errors.push(format!("compatibility.{name} must be non-empty"));
        }
    }
    if key.forecast_horizon_ms == 0
        || !(MIN_COVERAGE_PERMILLE..=MAX_COVERAGE_PERMILLE)
            .contains(&key.nominal_coverage_permille)
    {
        errors.push("compatibility horizon or coverage is outside v0.1 bounds".to_owned());
    }
    if key.bin_edges.len() < 2 || key.bin_edges.len() > MAX_BINS + 1 {
        errors.push(format!("compatibility.bin_edges must contain 2..={} items", MAX_BINS + 1));
    }
    if key.bin_edges.windows(2).any(|pair| pair[0] >= pair[1]) {
        errors.push("compatibility.bin_edges must be strictly increasing".to_owned());
    }

    let mut registered = BTreeSet::new();
    for site_id in &input.registered_site_ids {
        if !non_empty(site_id) || !registered.insert(site_id.as_str()) {
            errors.push("registered_site_ids contains an empty or duplicate site".to_owned());
        }
    }
    if registered.len() < input.minimum_participating_sites {
        errors.push("registered roster is smaller than the minimum cohort".to_owned());
    }
    if input.summaries.is_empty() || input.summaries.len() > MAX_SUMMARIES {
        errors.push(format!("summaries must contain 1..={MAX_SUMMARIES} items"));
    }

    let mut summaries_by_site = BTreeSet::new();
    let mut evidence_digests = BTreeSet::new();
    let mut ledgers_by_site: BTreeMap<&str, &SiteReleaseLedgerSnapshot> = BTreeMap::new();
    for ledger in &input.release_ledger_snapshot {
        if ledger.epoch_id != input.epoch_id
            || !registered.contains(ledger.site_id.as_str())
            || ledger.maximum_releases_in_epoch == 0
            || ledger.releases_in_epoch >= ledger.maximum_releases_in_epoch
        {
            errors.push(format!("release ledger for {} is invalid or exhausted", ledger.site_id));
        }
        if ledgers_by_site.insert(ledger.site_id.as_str(), ledger).is_some() {
            errors.push(format!("duplicate release ledger for {}", ledger.site_id));
        }
        let mut released = BTreeSet::new();
        for digest in &ledger.previously_released_query_digests {
            if !non_empty(digest) || !released.insert(digest.as_str()) {
                errors.push(format!("release ledger for {} has duplicate/empty query digest", ledger.site_id));
            }
            if digest == &input.query_scope_digest {
                errors.push(format!("query scope was already released for {}", ledger.site_id));
            }
        }
    }

    for summary in &input.summaries {
        if !registered.contains(summary.site_id.as_str()) {
            errors.push(format!("{} is not in the registered site roster", summary.site_id));
        }
        if !summaries_by_site.insert(summary.site_id.as_str()) {
            errors.push(format!("{} contributes more than once in the epoch", summary.site_id));
        }
        if !non_empty(&summary.local_evidence_digest)
            || !evidence_digests.insert(summary.local_evidence_digest.as_str())
        {
            errors.push(format!("{} has empty or reused local evidence digest", summary.site_id));
        }
        if summary.epoch_id != input.epoch_id {
            errors.push(format!("{} summary epoch does not match", summary.site_id));
        }
        if summary.compatibility != input.compatibility {
            errors.push(format!("{} summary is compatibility-inconsistent", summary.site_id));
        }
        if summary.calibration_sample_count == 0
            || summary.calibration_sample_count > input.maximum_samples_per_site
            || summary.calibration_sample_count > MAX_SAMPLES_PER_SITE
        {
            errors.push(format!("{} exceeds or violates the sample contribution bound", summary.site_id));
        }
        if summary.calibration_score_bin_counts.len() + 1 != input.compatibility.bin_edges.len()
            || summary.calibration_score_bin_counts.iter().any(|count| {
                *count > input.maximum_count_per_bin_per_site
                    || *count > MAX_BIN_COUNT_PER_SITE
            })
        {
            errors.push(format!("{} histogram bins violate shape or per-bin contribution bounds", summary.site_id));
        }
        let histogram_total = summary
            .calibration_score_bin_counts
            .iter()
            .try_fold(0_u64, |sum, count| sum.checked_add(*count));
        if histogram_total != Some(summary.calibration_sample_count) {
            errors.push(format!("{} histogram count does not equal calibration sample count", summary.site_id));
        }
        if summary.holdout_covered_count > summary.holdout_sample_count
            || summary.holdout_interval_width_sum > MAX_WIDTH_SUM
            || summary.local_audit_cutoff_unix_ms == 0
            || summary.local_audit_cutoff_unix_ms > input.aggregation_time_unix_ms
            || input.aggregation_time_unix_ms - summary.local_audit_cutoff_unix_ms
                > input.maximum_summary_age_ms
        {
            errors.push(format!("{} holdout metrics or evidence cutoff are invalid/stale", summary.site_id));
        }
        if !ledgers_by_site.contains_key(summary.site_id.as_str()) {
            errors.push(format!("{} has no release ledger snapshot", summary.site_id));
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn canonical_digest(input: &FederatedCalibrationInput) -> Result<String, Vec<String>> {
    let mut canonical = input.clone();
    canonical.summaries.sort_by(|a, b| a.site_id.cmp(&b.site_id));
    canonical.registered_site_ids.sort();
    canonical
        .release_ledger_snapshot
        .sort_by(|a, b| a.site_id.cmp(&b.site_id));
    for ledger in &mut canonical.release_ledger_snapshot {
        ledger.previously_released_query_scope_digests_sort();
    }
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| vec![format!("cannot canonicalize federation input: {error}")])?;
    Ok(format!("sha256:{:x}", Sha256::digest(encoded)))
}

pub fn evaluate_federated_calibration(
    input: &FederatedCalibrationInput,
) -> Result<FederatedCalibrationReport, Vec<String>> {
    validate(input)?;
    let input_digest = canonical_digest(input)?;
    let participant_count = input.summaries.len();
    let calibration_total = input
        .summaries
        .iter()
        .try_fold(0_u64, |sum, item| sum.checked_add(item.calibration_sample_count))
        .ok_or_else(|| vec!["aggregate calibration count overflow".to_owned()])?;

    let status = if participant_count < input.minimum_participating_sites {
        FederatedAggregationStatus::SuppressedMinimumCohort
    } else if calibration_total < input.minimum_total_calibration_samples {
        FederatedAggregationStatus::SuppressedMinimumSamples
    } else {
        FederatedAggregationStatus::Published
    };

    if status != FederatedAggregationStatus::Published {
        return Ok(FederatedCalibrationReport {
            schema_version: "0.1.0".to_owned(),
            benchmark_id: input.benchmark_id.clone(),
            epoch_id: input.epoch_id.clone(),
            query_scope_digest: input.query_scope_digest.clone(),
            compatibility: input.compatibility.clone(),
            status,
            participating_site_count: None,
            total_calibration_sample_count: None,
            total_holdout_sample_count: None,
            total_holdout_covered_count: None,
            holdout_coverage_permille: None,
            total_holdout_interval_width_sum: None,
            mean_holdout_interval_width: None,
            merged_calibration_score_bin_counts: None,
            conformal_quantile_rank: None,
            conformal_quantile_upper_edge: None,
            local_undercoverage_alarm_site_count: None,
            local_insufficient_support_site_count: None,
            local_unsupported_or_unknown_site_count: None,
            release_budget_admissible: true,
            release_budget_consumed: false,
            input_digest,
            membership_cryptographically_verified: false,
            secure_aggregation_performed: false,
            differential_privacy_applied: false,
            site_local_qualification_granted: false,
            fleet_field_qualification_granted: false,
            operational_authority_granted: false,
            claim_ceiling: CLAIM_CEILING,
        });
    }

    let mut merged_bins = vec![0_u64; input.compatibility.bin_edges.len() - 1];
    let mut holdout_total = 0_u64;
    let mut holdout_covered = 0_u64;
    let mut interval_width_total = 0_u64;
    let mut local_undercoverage_alarm_site_count = 0_usize;
    let mut local_insufficient_support_site_count = 0_usize;
    let mut local_unsupported_or_unknown_site_count = 0_usize;

    for summary in &input.summaries {
        for (merged, local) in merged_bins
            .iter_mut()
            .zip(summary.calibration_score_bin_counts.iter())
        {
            *merged = merged
                .checked_add(*local)
                .ok_or_else(|| vec!["aggregate histogram count overflow".to_owned()])?;
        }
        holdout_total = holdout_total
            .checked_add(summary.holdout_sample_count)
            .ok_or_else(|| vec!["aggregate holdout sample count overflow".to_owned()])?;
        holdout_covered = holdout_covered
            .checked_add(summary.holdout_covered_count)
            .ok_or_else(|| vec!["aggregate holdout covered count overflow".to_owned()])?;
        interval_width_total = interval_width_total
            .checked_add(summary.holdout_interval_width_sum)
            .ok_or_else(|| vec!["aggregate interval-width sum overflow".to_owned()])?;
        match summary.local_status {
            LocalCalibrationStatus::LocallySupported => {}
            LocalCalibrationStatus::UndercoverageAlarm => {
                local_undercoverage_alarm_site_count += 1;
            }
            LocalCalibrationStatus::InsufficientLocalSupport => {
                local_insufficient_support_site_count += 1;
            }
            LocalCalibrationStatus::UnsupportedStratum | LocalCalibrationStatus::Unknown => {
                local_unsupported_or_unknown_site_count += 1;
            }
        }
    }

    let quantile_rank = quantile_rank(
        calibration_total,
        input.compatibility.nominal_coverage_permille,
    );
    let quantile_upper_edge = resolve_quantile_upper_edge(
        &merged_bins,
        &input.compatibility.bin_edges,
        quantile_rank,
    );
    if quantile_upper_edge.is_none() {
        return Err(vec!["merged histogram cannot resolve the configured quantile".to_owned()]);
    }

    Ok(FederatedCalibrationReport {
        schema_version: "0.1.0".to_owned(),
        benchmark_id: input.benchmark_id.clone(),
        epoch_id: input.epoch_id.clone(),
        query_scope_digest: input.query_scope_digest.clone(),
        compatibility: input.compatibility.clone(),
        status,
        participating_site_count: Some(participant_count),
        total_calibration_sample_count: Some(calibration_total),
        total_holdout_sample_count: Some(holdout_total),
        total_holdout_covered_count: Some(holdout_covered),
        holdout_coverage_permille: if holdout_total == 0 {
            None
        } else {
            Some(((u128::from(holdout_covered) * 1000) / u128::from(holdout_total)) as u16)
        },
        total_holdout_interval_width_sum: Some(interval_width_total),
        mean_holdout_interval_width: if holdout_total == 0 {
            None
        } else {
            Some(interval_width_total / holdout_total)
        },
        merged_calibration_score_bin_counts: Some(merged_bins),
        conformal_quantile_rank: Some(quantile_rank),
        conformal_quantile_upper_edge: quantile_upper_edge,
        local_undercoverage_alarm_site_count: Some(local_undercoverage_alarm_site_count),
        local_insufficient_support_site_count: Some(local_insufficient_support_site_count),
        local_unsupported_or_unknown_site_count: Some(local_unsupported_or_unknown_site_count),
        release_budget_admissible: true,
        release_budget_consumed: false,
        input_digest,
        membership_cryptographically_verified: false,
        secure_aggregation_performed: false,
        differential_privacy_applied: false,
        site_local_qualification_granted: false,
        fleet_field_qualification_granted: false,
        operational_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compatibility() -> FederatedCompatibilityKey {
        FederatedCompatibilityKey {
            forecast_model_id: "model-1".to_owned(),
            forecast_model_version: "v1".to_owned(),
            forecast_horizon_ms: 900_000,
            metric_kind: "pack_terminal_energy_wh".to_owned(),
            measurement_boundary: "pack_terminal".to_owned(),
            unit: "Wh".to_owned(),
            stratum_id: "lfp_warm_mission".to_owned(),
            nominal_coverage_permille: 900,
            score_definition_id: "absolute_nonconformity_hist_v1".to_owned(),
            bin_edges: vec![0, 10, 20, 40],
        }
    }

    fn summary(site: &str, counts: Vec<u64>, covered: u64, status: LocalCalibrationStatus) -> SiteCalibrationSummary {
        SiteCalibrationSummary {
            site_id: site.to_owned(),
            epoch_id: "epoch-01".to_owned(),
            compatibility: compatibility(),
            calibration_sample_count: counts.iter().sum(),
            calibration_score_bin_counts: counts,
            holdout_sample_count: 10,
            holdout_covered_count: covered,
            holdout_interval_width_sum: 100,
            local_status: status,
            local_audit_cutoff_unix_ms: 1_900,
            local_evidence_digest: format!("sha256:{site}-evidence"),
        }
    }

    fn input(summaries: Vec<SiteCalibrationSummary>) -> FederatedCalibrationInput {
        let site_ids = ["depot-a", "depot-b", "depot-c"];
        FederatedCalibrationInput {
            schema_version: "0.1.0".to_owned(),
            benchmark_id: "FEDERATED-SERVICE-CALIBRATION-001".to_owned(),
            epoch_id: "epoch-01".to_owned(),
            aggregation_time_unix_ms: 2_000,
            maximum_summary_age_ms: 500,
            query_scope_digest: "sha256:query-01".to_owned(),
            compatibility: compatibility(),
            registered_site_ids: site_ids.iter().map(|id| (*id).to_owned()).collect(),
            minimum_participating_sites: 3,
            minimum_total_calibration_samples: 24,
            maximum_samples_per_site: 12,
            maximum_count_per_bin_per_site: 12,
            release_ledger_snapshot: site_ids
                .iter()
                .map(|site_id| SiteReleaseLedgerSnapshot {
                    site_id: (*site_id).to_owned(),
                    epoch_id: "epoch-01".to_owned(),
                    releases_in_epoch: 0,
                    maximum_releases_in_epoch: 2,
                    previously_released_query_digests: Vec::new(),
                })
                .collect(),
            summaries,
            claim_ceiling: "bounded synthetic summary merge only".to_owned(),
        }
    }

    fn fixture_input() -> FederatedCalibrationInput {
        input(vec![
            summary("depot-a", vec![2, 4, 4], 9, LocalCalibrationStatus::LocallySupported),
            summary("depot-b", vec![1, 3, 6], 7, LocalCalibrationStatus::UndercoverageAlarm),
            summary("depot-c", vec![3, 2, 5], 8, LocalCalibrationStatus::InsufficientLocalSupport),
        ])
    }

    #[test]
    fn merges_bounded_histograms_and_resolves_rank_quantile() {
        let report = evaluate_federated_calibration(&fixture_input()).unwrap();
        assert_eq!(report.status, FederatedAggregationStatus::Published);
        assert_eq!(report.participating_site_count, Some(3));
        assert_eq!(report.total_calibration_sample_count, Some(30));
        assert_eq!(report.merged_calibration_score_bin_counts, Some(vec![6, 9, 15]));
        assert_eq!(report.conformal_quantile_rank, Some(28));
        assert_eq!(report.conformal_quantile_upper_edge, Some(40));
        assert_eq!(report.total_holdout_sample_count, Some(30));
        assert_eq!(report.total_holdout_covered_count, Some(24));
        assert_eq!(report.holdout_coverage_permille, Some(800));
    }

    #[test]
    fn deterministic_digest_and_merge_are_input_order_invariant() {
        let first = fixture_input();
        let mut reversed = fixture_input();
        reversed.summaries.reverse();
        reversed.registered_site_ids.reverse();
        reversed.release_ledger_snapshot.reverse();
        let a = evaluate_federated_calibration(&first).unwrap();
        let b = evaluate_federated_calibration(&reversed).unwrap();
        assert_eq!(a.input_digest, b.input_digest);
        assert_eq!(a.merged_calibration_score_bin_counts, b.merged_calibration_score_bin_counts);
    }

    #[test]
    fn duplicate_site_contribution_is_rejected() {
        let mut input = fixture_input();
        input.summaries[1].site_id = input.summaries[0].site_id.clone();
        assert!(evaluate_federated_calibration(&input).is_err());
    }

    #[test]
    fn unregistered_site_is_rejected() {
        let mut input = fixture_input();
        input.summaries[0].site_id = "unknown-depot".to_owned();
        assert!(evaluate_federated_calibration(&input).is_err());
    }

    #[test]
    fn incompatible_measurement_boundary_is_rejected() {
        let mut input = fixture_input();
        input.summaries[1].compatibility.measurement_boundary = "charger_input".to_owned();
        assert!(evaluate_federated_calibration(&input).is_err());
    }

    #[test]
    fn contribution_above_site_cap_is_rejected() {
        let mut input = fixture_input();
        input.summaries[0].calibration_sample_count = 13;
        assert!(evaluate_federated_calibration(&input).is_err());
    }

    #[test]
    fn inconsistent_histogram_mass_is_rejected() {
        let mut input = fixture_input();
        input.summaries[0].calibration_score_bin_counts[0] = 99;
        assert!(evaluate_federated_calibration(&input).is_err());
    }

    #[test]
    fn minimum_cohort_suppression_withholds_aggregate_values() {
        let mut input = fixture_input();
        input.summaries.pop();
        let report = evaluate_federated_calibration(&input).unwrap();
        assert_eq!(report.status, FederatedAggregationStatus::SuppressedMinimumCohort);
        assert_eq!(report.participating_site_count, None);
        assert_eq!(report.merged_calibration_score_bin_counts, None);
        assert_eq!(report.conformal_quantile_upper_edge, None);
    }

    #[test]
    fn pooled_result_never_overrides_local_undercoverage() {
        let report = evaluate_federated_calibration(&fixture_input()).unwrap();
        assert_eq!(report.local_undercoverage_alarm_site_count, Some(1));
        assert_eq!(report.site_local_qualification_granted, false);
        assert_eq!(report.fleet_field_qualification_granted, false);
        assert_eq!(report.operational_authority_granted, false);
    }

    #[test]
    fn repeated_query_scope_is_rejected_by_release_ledger() {
        let mut input = fixture_input();
        input.release_ledger_snapshot[0]
            .previously_released_query_digests
            .push(input.query_scope_digest.clone());
        assert!(evaluate_federated_calibration(&input).is_err());
    }

    #[test]
    fn exhausted_site_release_budget_is_rejected() {
        let mut input = fixture_input();
        input.release_ledger_snapshot[0].releases_in_epoch = 2;
        assert!(evaluate_federated_calibration(&input).is_err());
    }

    #[test]
    fn stale_site_summary_is_rejected() {
        let mut input = fixture_input();
        input.summaries[0].local_audit_cutoff_unix_ms = 1;
        assert!(evaluate_federated_calibration(&input).is_err());
    }

    #[test]
    fn synthetic_arithmetic_does_not_establish_privacy_or_qualification() {
        let report = evaluate_federated_calibration(&fixture_input()).unwrap();
        assert_eq!(report.membership_cryptographically_verified, false);
        assert_eq!(report.secure_aggregation_performed, false);
        assert_eq!(report.differential_privacy_applied, false);
        assert_eq!(report.site_local_qualification_granted, false);
        assert_eq!(report.fleet_field_qualification_granted, false);
        assert_eq!(report.operational_authority_granted, false);
    }
}

trait CanonicalLedgerQueryDigests {
    fn previously_released_query_scope_digests_sort(&mut self);
}

impl CanonicalLedgerQueryDigests for SiteReleaseLedgerSnapshot {
    fn previously_released_query_scope_digests_sort(&mut self) {
        self.previously_released_query_digests.sort();
    }
}
