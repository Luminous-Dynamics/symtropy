// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: &str = "0.1";
const BENCHMARK_NAME: &str = "RRVR-EXECUTABLE-RESULTS";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmId {
    Native,
    EastValidity,
    RoborecoverDeviation,
    RestartRetry,
    JournalOnly,
    JournalInjector,
    EndpointFenced,
    GraphTemporalClosure,
    FullRrvr,
}

impl ArmId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::EastValidity => "east_validity",
            Self::RoborecoverDeviation => "roborecover_deviation",
            Self::RestartRetry => "restart_retry",
            Self::JournalOnly => "journal_only",
            Self::JournalInjector => "journal_injector",
            Self::EndpointFenced => "endpoint_fenced",
            Self::GraphTemporalClosure => "graph_temporal_closure",
            Self::FullRrvr => "full_rrvr",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Applied,
    NoEffect,
    Partial,
    Unknown,
    Disputed,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchKey {
    pub task_id: String,
    pub policy_sha256: String,
    pub scene_sha256: String,
    pub initial_state_sha256: String,
    pub action_prefix_sha256: String,
    pub seed: u64,
    pub simulator_profile_sha256: String,
    pub fault_schedule_sha256: String,
    pub observer_profile_sha256: String,
    pub expected_disposition: Disposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterIdentity {
    pub world_id: String,
    pub robot_asset_sha256: String,
    pub hardware_profile_sha256: String,
    pub firmware_profile_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultEvidence {
    pub requested: bool,
    pub accepted_by_injector: Option<bool>,
    pub execution_receipt: Option<bool>,
    pub independent_delivery_observation: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub oracle_disposition: Disposition,
    pub reported_disposition: Disposition,
    pub task_success: bool,
    pub episode_safe: bool,
    pub recovery_success: Option<bool>,
    pub attempted_effect: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceEvidence {
    pub one_shot_eligible: bool,
    pub physical_effect_count: Option<u32>,
    pub closure_certificate_issued: bool,
    pub closure_invalidated_by_old_effect: Option<bool>,
    pub endpoint_fence_accepted: bool,
    pub old_epoch_effect_after_fence: Option<bool>,
    pub fresh_attempt_decision: bool,
    pub fresh_attempt_admitted: bool,
    pub admission_preconditions_satisfied: bool,
    pub positive_control_opportunity: bool,
    pub evidence_eligible: bool,
    pub evidence_bundle_reconstructable: bool,
    pub evidence_scope_valid: bool,
    pub evidence_independently_checkable: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LatenciesMs {
    pub append_sync: Option<f64>,
    pub owner_reservation: Option<f64>,
    pub reset_replay: Option<f64>,
    pub endpoint_fence: Option<f64>,
    pub temporal_closure: Option<f64>,
    pub independent_observation: Option<f64>,
    pub native_authority_admission: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeRecord {
    pub episode_id: String,
    pub pair_id: String,
    pub arm_id: ArmId,
    pub qualification_tier: String,
    pub native_benchmark_preserved: bool,
    pub matching: MatchKey,
    pub clusters: ClusterIdentity,
    pub fault: FaultEvidence,
    pub outcome: Outcome,
    pub assurance: AssuranceEvidence,
    pub latencies_ms: LatenciesMs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultsBundle {
    pub schema_version: String,
    pub benchmark: String,
    pub campaign_id: String,
    pub claim_ceiling: String,
    pub oracle_revision: String,
    pub episodes: Vec<EpisodeRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RateMetric {
    pub eligible_n: usize,
    pub adjudicated_n: usize,
    pub event_n: usize,
    pub unresolved_n: usize,
    pub rate: Option<f64>,
}

impl RateMetric {
    fn complete(eligible_n: usize, event_n: usize) -> Self {
        Self {
            eligible_n,
            adjudicated_n: eligible_n,
            event_n,
            unresolved_n: 0,
            rate: ratio(event_n, eligible_n),
        }
    }

    fn partially_adjudicated(eligible_n: usize, adjudicated_n: usize, event_n: usize) -> Self {
        Self {
            eligible_n,
            adjudicated_n,
            event_n,
            unresolved_n: eligible_n.saturating_sub(adjudicated_n),
            rate: ratio(event_n, adjudicated_n),
        }
    }
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then(|| numerator as f64 / denominator as f64)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FaultStageCounts {
    pub requested_n: usize,
    pub accepted_yes_n: usize,
    pub accepted_no_n: usize,
    pub accepted_unknown_n: usize,
    pub executed_yes_n: usize,
    pub executed_no_n: usize,
    pub executed_unknown_n: usize,
    pub observed_yes_n: usize,
    pub observed_no_n: usize,
    pub observed_unknown_n: usize,
    pub accepted_without_request_n: usize,
    pub executed_without_acceptance_n: usize,
    pub observed_without_execution_receipt_n: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DurationPercentiles {
    pub sample_n: usize,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub p999_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClusterCounts {
    pub pair_n: usize,
    pub task_n: usize,
    pub policy_n: usize,
    pub scene_n: usize,
    pub seed_n: usize,
    pub world_n: usize,
    pub robot_asset_n: usize,
    pub hardware_profile_n: usize,
    pub firmware_profile_n: usize,
    pub fault_schedule_n: usize,
    pub observer_profile_n: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ArmScorecard {
    pub arm_id: String,
    pub episode_n: usize,
    pub native_task_success: RateMetric,
    pub native_episode_safety: RateMetric,
    pub native_policy_recovery_success: RateMetric,
    pub fault_delivery: FaultStageCounts,
    pub false_no_effect: RateMetric,
    pub false_applied: RateMetric,
    pub false_closed: RateMetric,
    pub duplicate_one_shot_effect: RateMetric,
    pub stale_effect_after_fence: RateMetric,
    pub unsafe_admission: RateMetric,
    pub false_hold: RateMetric,
    pub unknown_disputed: RateMetric,
    pub evidence_yield: RateMetric,
    pub latency_ms: BTreeMap<String, DurationPercentiles>,
    pub clusters: ClusterCounts,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvaluationReport {
    pub schema_version: String,
    pub benchmark: String,
    pub campaign_id: String,
    pub oracle_revision: String,
    pub claim_ceiling: String,
    pub pair_n: usize,
    pub episode_n: usize,
    pub scorecards: Vec<ArmScorecard>,
    pub inference_note: String,
}

pub fn evaluate_json(input: &str) -> Result<EvaluationReport, String> {
    let bundle: ResultsBundle = serde_json::from_str(input)
        .map_err(|error| format!("RESULTS_JSON_INVALID: {error}"))?;
    validate_bundle(&bundle)?;
    Ok(evaluate_bundle(&bundle))
}

fn validate_bundle(bundle: &ResultsBundle) -> Result<(), String> {
    if bundle.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "SCHEMA_VERSION_UNSUPPORTED: expected {SCHEMA_VERSION}, got {}",
            bundle.schema_version
        ));
    }
    if bundle.benchmark != BENCHMARK_NAME {
        return Err(format!(
            "BENCHMARK_NAME_MISMATCH: expected {BENCHMARK_NAME}, got {}",
            bundle.benchmark
        ));
    }
    require_text("campaign_id", &bundle.campaign_id)?;
    require_text("oracle_revision", &bundle.oracle_revision)?;
    if bundle.claim_ceiling.trim().chars().count() < 40 {
        return Err("CLAIM_CEILING_TOO_SHORT".to_owned());
    }
    if bundle.episodes.is_empty() {
        return Err("NO_EPISODES".to_owned());
    }

    let mut episode_ids = BTreeSet::new();
    let mut pairs: BTreeMap<&str, Vec<&EpisodeRecord>> = BTreeMap::new();

    for episode in &bundle.episodes {
        require_text("episode_id", &episode.episode_id)?;
        require_text("pair_id", &episode.pair_id)?;
        require_text("qualification_tier", &episode.qualification_tier)?;
        if !episode_ids.insert(episode.episode_id.as_str()) {
            return Err(format!("DUPLICATE_EPISODE_ID: {}", episode.episode_id));
        }
        if !episode.native_benchmark_preserved {
            return Err(format!(
                "NATIVE_BENCHMARK_NOT_PRESERVED: {}",
                episode.episode_id
            ));
        }

        require_text("task_id", &episode.matching.task_id)?;
        for (name, digest) in [
            ("policy_sha256", &episode.matching.policy_sha256),
            ("scene_sha256", &episode.matching.scene_sha256),
            ("initial_state_sha256", &episode.matching.initial_state_sha256),
            ("action_prefix_sha256", &episode.matching.action_prefix_sha256),
            ("simulator_profile_sha256", &episode.matching.simulator_profile_sha256),
            ("fault_schedule_sha256", &episode.matching.fault_schedule_sha256),
            ("observer_profile_sha256", &episode.matching.observer_profile_sha256),
            ("robot_asset_sha256", &episode.clusters.robot_asset_sha256),
            ("hardware_profile_sha256", &episode.clusters.hardware_profile_sha256),
            ("firmware_profile_sha256", &episode.clusters.firmware_profile_sha256),
        ] {
            require_digest(name, digest)?;
        }
        require_text("world_id", &episode.clusters.world_id)?;

        for (name, value) in [
            ("append_sync", episode.latencies_ms.append_sync),
            ("owner_reservation", episode.latencies_ms.owner_reservation),
            ("reset_replay", episode.latencies_ms.reset_replay),
            ("endpoint_fence", episode.latencies_ms.endpoint_fence),
            ("temporal_closure", episode.latencies_ms.temporal_closure),
            ("independent_observation", episode.latencies_ms.independent_observation),
            ("native_authority_admission", episode.latencies_ms.native_authority_admission),
        ] {
            if let Some(milliseconds) = value
                && (!milliseconds.is_finite() || milliseconds < 0.0)
            {
                return Err(format!(
                    "INVALID_LATENCY: episode={} field={name}",
                    episode.episode_id
                ));
            }
        }

        pairs.entry(episode.pair_id.as_str()).or_default().push(episode);
    }

    for (pair_id, rows) in pairs {
        if rows.len() < 2 {
            return Err(format!("PAIR_HAS_FEWER_THAN_TWO_ARMS: {pair_id}"));
        }

        let reference_key = &rows[0].matching;
        let reference_oracle = rows[0].outcome.oracle_disposition;
        let mut arms = BTreeSet::new();

        for row in &rows {
            if row.matching != *reference_key {
                let field = first_match_difference(reference_key, &row.matching);
                return Err(format!("PAIR_MATCH_KEY_MISMATCH: pair={pair_id} field={field}"));
            }
            if row.outcome.oracle_disposition != reference_oracle {
                return Err(format!("PAIR_ORACLE_DISPOSITION_MISMATCH: pair={pair_id}"));
            }
            if !arms.insert(row.arm_id) {
                return Err(format!(
                    "DUPLICATE_ARM_IN_PAIR: pair={pair_id} arm={}",
                    row.arm_id.as_str()
                ));
            }
        }
        if !arms.contains(&ArmId::Native) {
            return Err(format!("PAIR_NATIVE_ARM_MISSING: {pair_id}"));
        }
        if !arms.contains(&ArmId::FullRrvr) {
            return Err(format!("PAIR_FULL_RRVR_ARM_MISSING: {pair_id}"));
        }
    }

    Ok(())
}

fn require_text(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("EMPTY_FIELD: {name}"))
    } else {
        Ok(())
    }
}

fn require_digest(name: &str, value: &str) -> Result<(), String> {
    let valid = value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(format!("INVALID_SHA256: {name}"))
    }
}

fn first_match_difference(left: &MatchKey, right: &MatchKey) -> &'static str {
    if left.task_id != right.task_id {
        "task_id"
    } else if left.policy_sha256 != right.policy_sha256 {
        "policy_sha256"
    } else if left.scene_sha256 != right.scene_sha256 {
        "scene_sha256"
    } else if left.initial_state_sha256 != right.initial_state_sha256 {
        "initial_state_sha256"
    } else if left.action_prefix_sha256 != right.action_prefix_sha256 {
        "action_prefix_sha256"
    } else if left.seed != right.seed {
        "seed"
    } else if left.simulator_profile_sha256 != right.simulator_profile_sha256 {
        "simulator_profile_sha256"
    } else if left.fault_schedule_sha256 != right.fault_schedule_sha256 {
        "fault_schedule_sha256"
    } else if left.observer_profile_sha256 != right.observer_profile_sha256 {
        "observer_profile_sha256"
    } else {
        "expected_disposition"
    }
}

fn evaluate_bundle(bundle: &ResultsBundle) -> EvaluationReport {
    let mut by_arm: BTreeMap<ArmId, Vec<&EpisodeRecord>> = BTreeMap::new();
    let mut pair_ids = BTreeSet::new();
    for episode in &bundle.episodes {
        by_arm.entry(episode.arm_id).or_default().push(episode);
        pair_ids.insert(episode.pair_id.as_str());
    }

    let scorecards = by_arm
        .into_iter()
        .map(|(arm_id, rows)| score_arm(arm_id, &rows))
        .collect();

    EvaluationReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: BENCHMARK_NAME.to_owned(),
        campaign_id: bundle.campaign_id.clone(),
        oracle_revision: bundle.oracle_revision.clone(),
        claim_ceiling: bundle.claim_ceiling.clone(),
        pair_n: pair_ids.len(),
        episode_n: bundle.episodes.len(),
        scorecards,
        inference_note: "Rates include explicit eligible, adjudicated and unresolved counts. Cluster cardinalities are reported; no confidence interval or independence claim is inferred without a preregistered sampling and cluster design.".to_owned(),
    }
}

fn metric_from_optional(
    eligible_n: usize,
    values: impl Iterator<Item = Option<bool>>,
) -> RateMetric {
    let known: Vec<bool> = values.flatten().collect();
    let adjudicated_n = known.len();
    let event_n = known.iter().filter(|value| **value).count();
    RateMetric::partially_adjudicated(eligible_n, adjudicated_n, event_n)
}

fn score_arm(arm_id: ArmId, rows: &[&EpisodeRecord]) -> ArmScorecard {
    let episode_n = rows.len();

    let task_success_n = rows.iter().filter(|row| row.outcome.task_success).count();
    let episode_safe_n = rows.iter().filter(|row| row.outcome.episode_safe).count();
    let recovery_values = rows.iter().map(|row| row.outcome.recovery_success);

    let false_no_effect_eligible = rows
        .iter()
        .filter(|row| {
            matches!(
                row.outcome.oracle_disposition,
                Disposition::Applied | Disposition::Partial
            )
        })
        .collect::<Vec<_>>();
    let false_no_effect_n = false_no_effect_eligible
        .iter()
        .filter(|row| row.outcome.reported_disposition == Disposition::NoEffect)
        .count();

    let false_applied_eligible = rows
        .iter()
        .filter(|row| {
            matches!(
                row.outcome.oracle_disposition,
                Disposition::NoEffect | Disposition::Partial
            )
        })
        .collect::<Vec<_>>();
    let false_applied_n = false_applied_eligible
        .iter()
        .filter(|row| row.outcome.reported_disposition == Disposition::Applied)
        .count();

    let closure_rows = rows
        .iter()
        .filter(|row| row.assurance.closure_certificate_issued)
        .collect::<Vec<_>>();
    let false_closed = metric_from_optional(
        closure_rows.len(),
        closure_rows.iter().map(|row| row.assurance.closure_invalidated_by_old_effect),
    );

    let one_shot_rows = rows
        .iter()
        .filter(|row| row.assurance.one_shot_eligible)
        .collect::<Vec<_>>();
    let duplicate_one_shot = metric_from_optional(
        one_shot_rows.len(),
        one_shot_rows.iter().map(|row| {
            row.assurance.physical_effect_count.map(|count| count > 1)
        }),
    );

    let fenced_rows = rows
        .iter()
        .filter(|row| row.assurance.endpoint_fence_accepted)
        .collect::<Vec<_>>();
    let stale_effect = metric_from_optional(
        fenced_rows.len(),
        fenced_rows.iter().map(|row| row.assurance.old_epoch_effect_after_fence),
    );

    let admission_rows = rows
        .iter()
        .filter(|row| row.assurance.fresh_attempt_decision)
        .collect::<Vec<_>>();
    let unsafe_admission_n = admission_rows
        .iter()
        .filter(|row| {
            row.assurance.fresh_attempt_admitted
                && !row.assurance.admission_preconditions_satisfied
        })
        .count();

    let positive_hold_rows = rows
        .iter()
        .filter(|row| {
            row.assurance.positive_control_opportunity
                && row.assurance.fresh_attempt_decision
                && row.assurance.admission_preconditions_satisfied
        })
        .collect::<Vec<_>>();
    let false_hold_n = positive_hold_rows
        .iter()
        .filter(|row| !row.assurance.fresh_attempt_admitted)
        .count();

    let attempted_rows = rows
        .iter()
        .filter(|row| row.outcome.attempted_effect)
        .collect::<Vec<_>>();
    let unknown_disputed_n = attempted_rows
        .iter()
        .filter(|row| {
            matches!(
                row.outcome.reported_disposition,
                Disposition::Unknown | Disposition::Disputed
            )
        })
        .count();

    let evidence_rows = rows
        .iter()
        .filter(|row| row.assurance.evidence_eligible)
        .collect::<Vec<_>>();
    let evidence_yield_n = evidence_rows
        .iter()
        .filter(|row| {
            row.assurance.evidence_bundle_reconstructable
                && row.assurance.evidence_scope_valid
                && row.assurance.evidence_independently_checkable
        })
        .count();

    ArmScorecard {
        arm_id: arm_id.as_str().to_owned(),
        episode_n,
        native_task_success: RateMetric::complete(episode_n, task_success_n),
        native_episode_safety: RateMetric::complete(episode_n, episode_safe_n),
        native_policy_recovery_success: metric_from_optional(
            episode_n,
            recovery_values,
        ),
        fault_delivery: fault_stage_counts(rows),
        false_no_effect: RateMetric::complete(
            false_no_effect_eligible.len(),
            false_no_effect_n,
        ),
        false_applied: RateMetric::complete(false_applied_eligible.len(), false_applied_n),
        false_closed,
        duplicate_one_shot_effect: duplicate_one_shot,
        stale_effect_after_fence: stale_effect,
        unsafe_admission: RateMetric::complete(admission_rows.len(), unsafe_admission_n),
        false_hold: RateMetric::complete(positive_hold_rows.len(), false_hold_n),
        unknown_disputed: RateMetric::complete(attempted_rows.len(), unknown_disputed_n),
        evidence_yield: RateMetric::complete(evidence_rows.len(), evidence_yield_n),
        latency_ms: latency_scorecard(rows),
        clusters: cluster_counts(rows),
    }
}

fn fault_stage_counts(rows: &[&EpisodeRecord]) -> FaultStageCounts {
    let count = |predicate: fn(&EpisodeRecord) -> bool| {
        rows.iter().filter(|row| predicate(row)).count()
    };
    FaultStageCounts {
        requested_n: count(|row| row.fault.requested),
        accepted_yes_n: count(|row| row.fault.accepted_by_injector == Some(true)),
        accepted_no_n: count(|row| row.fault.accepted_by_injector == Some(false)),
        accepted_unknown_n: count(|row| row.fault.accepted_by_injector.is_none()),
        executed_yes_n: count(|row| row.fault.execution_receipt == Some(true)),
        executed_no_n: count(|row| row.fault.execution_receipt == Some(false)),
        executed_unknown_n: count(|row| row.fault.execution_receipt.is_none()),
        observed_yes_n: count(|row| row.fault.independent_delivery_observation == Some(true)),
        observed_no_n: count(|row| row.fault.independent_delivery_observation == Some(false)),
        observed_unknown_n: count(|row| row.fault.independent_delivery_observation.is_none()),
        accepted_without_request_n: count(|row| {
            !row.fault.requested && row.fault.accepted_by_injector == Some(true)
        }),
        executed_without_acceptance_n: count(|row| {
            row.fault.execution_receipt == Some(true)
                && row.fault.accepted_by_injector != Some(true)
        }),
        observed_without_execution_receipt_n: count(|row| {
            row.fault.independent_delivery_observation == Some(true)
                && row.fault.execution_receipt != Some(true)
        }),
    }
}

fn cluster_counts(rows: &[&EpisodeRecord]) -> ClusterCounts {
    let unique = |values: Vec<String>| values.into_iter().collect::<BTreeSet<_>>().len();
    ClusterCounts {
        pair_n: unique(rows.iter().map(|row| row.pair_id.clone()).collect()),
        task_n: unique(rows.iter().map(|row| row.matching.task_id.clone()).collect()),
        policy_n: unique(rows.iter().map(|row| row.matching.policy_sha256.clone()).collect()),
        scene_n: unique(rows.iter().map(|row| row.matching.scene_sha256.clone()).collect()),
        seed_n: rows.iter().map(|row| row.matching.seed).collect::<BTreeSet<_>>().len(),
        world_n: unique(rows.iter().map(|row| row.clusters.world_id.clone()).collect()),
        robot_asset_n: unique(
            rows.iter()
                .map(|row| row.clusters.robot_asset_sha256.clone())
                .collect(),
        ),
        hardware_profile_n: unique(
            rows.iter()
                .map(|row| row.clusters.hardware_profile_sha256.clone())
                .collect(),
        ),
        firmware_profile_n: unique(
            rows.iter()
                .map(|row| row.clusters.firmware_profile_sha256.clone())
                .collect(),
        ),
        fault_schedule_n: unique(
            rows.iter()
                .map(|row| row.matching.fault_schedule_sha256.clone())
                .collect(),
        ),
        observer_profile_n: unique(
            rows.iter()
                .map(|row| row.matching.observer_profile_sha256.clone())
                .collect(),
        ),
    }
}

fn nearest_rank(samples: &[f64], percentile: f64) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut ordered = samples.to_vec();
    ordered.sort_by(f64::total_cmp);
    let rank = ((percentile * ordered.len() as f64).ceil() as usize).max(1);
    ordered.get(rank.saturating_sub(1)).copied()
}

fn duration_percentiles(samples: &[f64]) -> DurationPercentiles {
    DurationPercentiles {
        sample_n: samples.len(),
        p50_ms: nearest_rank(samples, 0.50),
        p95_ms: nearest_rank(samples, 0.95),
        p99_ms: nearest_rank(samples, 0.99),
        p999_ms: nearest_rank(samples, 0.999),
    }
}

fn latency_scorecard(rows: &[&EpisodeRecord]) -> BTreeMap<String, DurationPercentiles> {
    let metrics: [(&str, fn(&EpisodeRecord) -> Option<f64>); 7] = [
        ("append_sync", |row| row.latencies_ms.append_sync),
        ("owner_reservation", |row| row.latencies_ms.owner_reservation),
        ("reset_replay", |row| row.latencies_ms.reset_replay),
        ("endpoint_fence", |row| row.latencies_ms.endpoint_fence),
        ("temporal_closure", |row| row.latencies_ms.temporal_closure),
        ("independent_observation", |row| row.latencies_ms.independent_observation),
        ("native_authority_admission", |row| row.latencies_ms.native_authority_admission),
    ];

    metrics
        .into_iter()
        .map(|(name, select)| {
            let samples = rows.iter().filter_map(|row| select(row)).collect::<Vec<_>>();
            (name.to_owned(), duration_percentiles(&samples))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/rrvr_results_v0_1.json");

    #[test]
    fn synthetic_known_answer_fixture_evaluates() {
        let report = evaluate_json(FIXTURE).expect("fixture should evaluate");
        assert_eq!(report.pair_n, 2);
        assert_eq!(report.episode_n, 4);
        assert_eq!(report.scorecards.len(), 2);
        assert_eq!(report.scorecards[0].arm_id, "native");
        assert_eq!(report.scorecards[1].arm_id, "full_rrvr");
    }

    #[test]
    fn pair_key_mutation_is_rejected() {
        let changed = FIXTURE.replacen(
            "\"policy_sha256\": \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"",
            "\"policy_sha256\": \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
            1,
        );
        let error = evaluate_json(&changed).expect_err("mutated pair must reject");
        assert!(error.starts_with("PAIR_MATCH_KEY_MISMATCH"));
        assert!(error.contains("policy_sha256"));
    }

    #[test]
    fn duplicate_episode_id_is_rejected() {
        let changed = FIXTURE.replacen(
            "\"episode_id\": \"EP-002\"",
            "\"episode_id\": \"EP-001\"",
            1,
        );
        let error = evaluate_json(&changed).expect_err("duplicate ID must reject");
        assert!(error.starts_with("DUPLICATE_EPISODE_ID"));
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let changed = FIXTURE.replacen(
            "\"schema_version\": \"0.1\"",
            "\"schema_version\": \"0.2\"",
            1,
        );
        let error = evaluate_json(&changed).expect_err("wrong version must reject");
        assert!(error.starts_with("SCHEMA_VERSION_UNSUPPORTED"));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let changed = FIXTURE.replacen(
            "\"campaign_id\": \"rrvr-scorecard-known-answer-001\"",
            "\"campaign_id\": \"rrvr-scorecard-known-answer-001\", \"unreviewed\": true",
            1,
        );
        let error = evaluate_json(&changed).expect_err("unknown field must reject");
        assert!(error.starts_with("RESULTS_JSON_INVALID"));
    }

    #[test]
    fn no_effect_false_verdicts_have_exact_denominator() {
        let report = evaluate_json(FIXTURE).expect("fixture should evaluate");
        let native = report
            .scorecards
            .iter()
            .find(|scorecard| scorecard.arm_id == "native")
            .expect("native arm");
        assert_eq!(native.false_no_effect.eligible_n, 1);
        assert_eq!(native.false_no_effect.event_n, 1);
        assert_eq!(native.false_no_effect.rate, Some(1.0));
    }

    #[test]
    fn unknown_and_disputed_are_not_relabelled_as_no_effect() {
        let report = evaluate_json(FIXTURE).expect("fixture should evaluate");
        let rrvr = report
            .scorecards
            .iter()
            .find(|scorecard| scorecard.arm_id == "full_rrvr")
            .expect("full RRVR arm");
        assert_eq!(rrvr.unknown_disputed.eligible_n, 2);
        assert_eq!(rrvr.unknown_disputed.event_n, 1);
        assert_eq!(rrvr.false_no_effect.event_n, 0);
    }

    #[test]
    fn missing_measurement_stays_unresolved() {
        let report = evaluate_json(FIXTURE).expect("fixture should evaluate");
        let rrvr = report
            .scorecards
            .iter()
            .find(|scorecard| scorecard.arm_id == "full_rrvr")
            .expect("full RRVR arm");
        assert_eq!(rrvr.duplicate_one_shot_effect.eligible_n, 2);
        assert_eq!(rrvr.duplicate_one_shot_effect.adjudicated_n, 1);
        assert_eq!(rrvr.duplicate_one_shot_effect.unresolved_n, 1);
    }

    #[test]
    fn injector_stage_contradictions_are_reported_not_silently_dropped() {
        let report = evaluate_json(FIXTURE).expect("fixture should evaluate");
        let rrvr = report
            .scorecards
            .iter()
            .find(|scorecard| scorecard.arm_id == "full_rrvr")
            .expect("full RRVR arm");
        assert_eq!(rrvr.fault_delivery.executed_without_acceptance_n, 1);
        assert_eq!(rrvr.fault_delivery.observed_without_execution_receipt_n, 1);
    }

    #[test]
    fn zero_denominator_is_null_not_zero() {
        let report = evaluate_json(FIXTURE).expect("fixture should evaluate");
        let native = report
            .scorecards
            .iter()
            .find(|scorecard| scorecard.arm_id == "native")
            .expect("native arm");
        assert_eq!(native.false_closed.eligible_n, 0);
        assert_eq!(native.false_closed.rate, None);
    }

    #[test]
    fn latency_percentiles_use_nearest_rank_and_keep_sample_count() {
        let result = duration_percentiles(&[30.0, 10.0, 20.0]);
        assert_eq!(result.sample_n, 3);
        assert_eq!(result.p50_ms, Some(20.0));
        assert_eq!(result.p95_ms, Some(30.0));
        assert_eq!(result.p999_ms, Some(30.0));
    }

    #[test]
    fn native_benchmark_cannot_be_silently_replaced() {
        let changed = FIXTURE.replacen(
            "\"native_benchmark_preserved\": true",
            "\"native_benchmark_preserved\": false",
            1,
        );
        let error = evaluate_json(&changed).expect_err("native metric substitution must reject");
        assert!(error.starts_with("NATIVE_BENCHMARK_NOT_PRESERVED"));
    }

    #[test]
    fn a_pair_requires_both_native_and_full_rrvr_arms() {
        let changed = FIXTURE.replacen(
            "\"arm_id\": \"full_rrvr\"",
            "\"arm_id\": \"journal_only\"",
            1,
        );
        let error = evaluate_json(&changed).expect_err("missing full RRVR arm must reject");
        assert!(error.starts_with("PAIR_FULL_RRVR_ARM_MISSING"));
    }

    #[test]
    fn uncertain_oracle_rows_do_not_inflate_false_verdict_denominators() {
        let changed = FIXTURE.replace(
            "\"oracle_disposition\": \"applied\"",
            "\"oracle_disposition\": \"disputed\"",
        );
        let report = evaluate_json(&changed).expect("paired disputed oracle should evaluate");
        let native = report
            .scorecards
            .iter()
            .find(|scorecard| scorecard.arm_id == "native")
            .expect("native arm");
        assert_eq!(native.false_no_effect.eligible_n, 0);
        assert_eq!(native.false_no_effect.rate, None);
    }

}
