// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Paired mobility-first energy-policy benchmark evaluator.
//!
//! This crate consumes completed event/replay outcome summaries. It does not
//! implement policies, optimize schedules, simulate batteries, verify PCC
//! metrology, or solve network power flow. Every comparator must use the same
//! exogenous scenario, replication, random seed, evidence class, compute
//! environment and compute budget.
//!
//! Hard assurance outcomes are reported separately from performance metrics.
//! No scalar reward can compensate for a hard-gate violation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: &str = "0.1.0";
const BENCHMARK_ID: &str = "MOBILITY-ENERGY-POLICY-TOURNAMENT-001";
const MAX_POLICIES: usize = 32;
const MAX_RUNS: usize = 100_000;
const MAX_RUNS_PER_POLICY: usize = 25_000;
const MAX_CLUSTERS: usize = 10_000;
const MIN_BOOTSTRAP_RESAMPLES: u32 = 1_000;
const MAX_BOOTSTRAP_RESAMPLES: u32 = 50_000;
const MAX_BOOTSTRAP_CLUSTERS: u32 = 10_000;
const MAX_BOOTSTRAP_WORK_ITEMS: u64 = 100_000_000;
const BOOTSTRAP_METRIC_COUNT: u64 = 6;
const CLAIM_CEILING: &str = "Matched outcome evaluation over the supplied finite scenario/replay corpus only. This report does not implement or endorse an optimizer, prove physical performance, authenticate source artifacts, establish causal effects, grant market eligibility, authorize dispatch, or establish AC feasibility/ELCC. Confidence intervals describe deterministic cluster-bootstrap resampling of the supplied independent-cluster labels; they are not distribution-free field guarantees.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    PaperReproduction,
    MeasuredReplay,
    PhysicalTrial,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyFamily {
    MobilityOnlyFixedCap,
    PvStationaryEss,
    TouMilp,
    RollingHorizonMpc,
    RobustOptimization,
    ClusterBidTracking,
    DistributionallyRobustRl,
    ShieldedLearnedPolicy,
    Other,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum InformationModel {
    NonanticipativeOnline,
    ClairvoyantOfflineUpperBound,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Completed,
    TimedOut,
    Abstained,
    Failed,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum HardGateCategory {
    MissionReserve,
    ReserveRecovery,
    PccLimit,
    NetworkDeliverability,
    Authorization,
    DoubleCommitment,
    EvidenceIntegrity,
    CyberIntegrity,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyIdentity {
    pub policy_id: String,
    pub policy_version: String,
    pub family: PolicyFamily,
    pub information_model: InformationModel,
    pub configuration_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HardGateViolation {
    pub category: HardGateCategory,
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyRun {
    pub run_id: String,
    pub policy_id: String,
    pub scenario_id: String,
    pub scenario_family: String,
    /// Independent resampling unit, e.g. site-day or independently generated incident block.
    pub cluster_id: String,
    pub replication_id: String,
    pub random_seed: u64,
    pub scenario_digest: String,
    /// Digest of exogenous inputs only; policy configuration is pinned separately.
    pub scenario_input_digest: String,
    pub policy_configuration_digest: String,
    pub compute_environment_digest: String,
    pub compute_budget_ms: u64,
    pub evidence_class: EvidenceClass,
    pub information_trace_digest: String,
    pub information_audit_digest: String,
    pub future_information_access_count: u32,
    pub nonanticipativity_violation_count: u32,
    pub status: RunStatus,
    /// Feasibility label from the independently declared oracle for this proposed action.
    pub oracle_action_feasible: bool,
    /// Whether the deterministic assurance shield admitted the proposed grid action.
    pub shield_admitted: bool,
    pub requested_grid_service_wh: u64,
    pub hard_gate_violations: Vec<HardGateViolation>,
    pub committed_missions: u32,
    pub completed_missions: u32,
    pub on_time_missions: u32,
    pub unserved_missions: u32,
    pub in_progress_at_horizon: u32,
    pub queue_wait_p95_ms: u64,
    pub mission_reserve_shortfall_wh: u64,
    pub recovery_shortfall_wh: u64,
    /// Signed: negative means increased PCC import, not zero delivery.
    pub instructed_grid_response_wh: i64,
    /// Signed: negative means increased metered import, not zero delivery.
    pub metered_grid_response_wh: i64,
    pub maximum_dc_network_deliverable_wh: u64,
    pub site_import_energy_wh: u64,
    pub renewable_curtailed_energy_wh: u64,
    pub battery_throughput_wh: u64,
    pub operating_cost_micro_units: u64,
    pub degradation_cost_micro_units: u64,
    pub emissions_grams_co2e: u64,
    pub decision_latency_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BootstrapConfig {
    pub seed: u64,
    pub resamples: u32,
    pub confidence_basis_points: u16,
    pub minimum_independent_clusters: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MobilityPolicyTournamentInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub evidence_class: EvidenceClass,
    pub scenario_manifest_digest: String,
    pub feasibility_oracle_digest: String,
    pub compute_environment_digest: String,
    pub compute_budget_ms: u64,
    pub reference_policy_id: String,
    pub bootstrap: BootstrapConfig,
    pub policies: Vec<PolicyIdentity>,
    pub runs: Vec<PolicyRun>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GateViolationCount {
    pub category: HardGateCategory,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StratumOutcomeSummary {
    pub stratum_id: String,
    pub run_count: u64,
    pub committed_missions: u64,
    pub on_time_service_rate_basis_points: u64,
    pub mean_queue_wait_p95_ms: f64,
    pub total_metered_grid_response_wh_signed: i64,
    pub max_mission_reserve_shortfall_wh: u64,
    pub max_recovery_shortfall_wh: u64,
    pub empirical_cvar95_mission_reserve_shortfall_wh: f64,
    pub empirical_cvar95_recovery_shortfall_wh: f64,
    pub future_information_access_count: u64,
    pub nonanticipativity_violation_count: u64,
    pub information_boundary_clean: bool,
    pub hard_gate_violation_count: u64,
    pub false_safe_admissions: u64,
    pub false_abstentions: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PolicySummary {
    pub policy_id: String,
    pub policy_version: String,
    pub family: PolicyFamily,
    pub information_model: InformationModel,
    pub run_count: u64,
    pub completed_run_count: u64,
    pub timed_out_run_count: u64,
    pub abstained_run_count: u64,
    pub failed_run_count: u64,
    pub independent_cluster_count: u64,
    pub committed_missions: u64,
    pub completed_missions: u64,
    pub on_time_missions: u64,
    pub unserved_missions: u64,
    pub in_progress_at_horizon: u64,
    pub on_time_service_rate_basis_points: u64,
    pub completion_rate_basis_points: u64,
    pub mean_queue_wait_p95_ms: f64,
    pub p95_queue_wait_p95_ms: u64,
    pub future_information_access_count: u64,
    pub nonanticipativity_violation_count: u64,
    pub information_boundary_clean: bool,
    pub p95_mission_reserve_shortfall_wh: u64,
    pub p95_recovery_shortfall_wh: u64,
    pub empirical_cvar95_mission_reserve_shortfall_wh: f64,
    pub empirical_cvar95_recovery_shortfall_wh: f64,
    pub scenario_family_outcomes: Vec<StratumOutcomeSummary>,
    pub independent_cluster_outcomes: Vec<StratumOutcomeSummary>,
    pub total_requested_grid_service_wh: u64,
    pub total_instructed_grid_response_wh_signed: i64,
    pub total_metered_grid_response_wh_signed: i64,
    pub total_maximum_dc_network_deliverable_wh: u64,
    pub total_site_import_energy_wh: u64,
    pub total_renewable_curtailed_energy_wh: u64,
    pub total_battery_throughput_wh: u64,
    pub operating_cost_micro_units: u64,
    pub degradation_cost_micro_units: u64,
    pub emissions_grams_co2e: u64,
    pub operating_cost_per_committed_mission_micro_units: Option<f64>,
    pub operating_cost_per_on_time_mission_micro_units: Option<f64>,
    pub total_mission_reserve_shortfall_wh: u64,
    pub total_recovery_shortfall_wh: u64,
    pub mean_decision_latency_ms: f64,
    pub p95_decision_latency_ms: u64,
    pub false_safe_admissions: u64,
    pub false_abstentions: u64,
    pub shield_rejections: u64,
    pub network_cap_conflicts: u64,
    pub hard_gate_violations: Vec<GateViolationCount>,
    pub hard_gate_clean: bool,
    pub false_safe_free: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BootstrapDelta {
    /// Equal-weight mean of per-cluster paired deltas.
    pub mean_delta: Option<f64>,
    pub lower_percentile: Option<f64>,
    pub upper_percentile: Option<f64>,
    pub independent_cluster_count: u64,
    pub bootstrap_resamples: u32,
    pub confidence_basis_points: u16,
    pub interval_available: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PolicyComparison {
    pub policy_id: String,
    pub reference_policy_id: String,
    pub paired_run_count: u64,
    pub independent_cluster_count: u64,
    pub on_time_service_rate_basis_points: BootstrapDelta,
    pub mean_queue_wait_p95_ms: BootstrapDelta,
    pub signed_metered_grid_response_wh_per_run: BootstrapDelta,
    pub recovery_shortfall_wh_per_run: BootstrapDelta,
    pub operating_cost_micro_units_per_committed_mission: BootstrapDelta,
    pub decision_latency_ms_per_run: BootstrapDelta,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MobilityPolicyTournamentReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub evidence_class: EvidenceClass,
    pub information_model: InformationModel,
    pub scenario_manifest_digest: String,
    pub feasibility_oracle_digest: String,
    pub compute_environment_digest: String,
    pub compute_budget_ms: u64,
    pub reference_policy_id: String,
    pub input_digest: String,
    pub matched_pair_count: u64,
    pub independent_cluster_count: u64,
    pub policies: Vec<PolicySummary>,
    pub comparisons: Vec<PolicyComparison>,
    /// True only when every policy has zero reported hard violations and zero false-safe admission.
    pub all_policies_hard_gate_clean: bool,
    pub all_policies_false_safe_free: bool,
    pub no_performance_score_offsets_hard_gate_failures: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TournamentError {
    InvalidInput(String),
    MissingPolicy(String),
    DuplicatePolicy(String),
    DuplicateRun(String),
    MissingRun(String),
    PolicyMismatch(String),
    PairMismatch(String),
    EvidenceMismatch(String),
    ComputeMismatch(String),
    InvalidMetric(String),
    Arithmetic(String),
}

impl core::fmt::Display for TournamentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput(message)
            | Self::MissingPolicy(message)
            | Self::DuplicatePolicy(message)
            | Self::DuplicateRun(message)
            | Self::MissingRun(message)
            | Self::PolicyMismatch(message)
            | Self::PairMismatch(message)
            | Self::EvidenceMismatch(message)
            | Self::ComputeMismatch(message)
            | Self::InvalidMetric(message)
            | Self::Arithmetic(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for TournamentError {}

fn nonempty(field: &str, value: &str) -> Result<(), TournamentError> {
    if value.trim().is_empty() {
        Err(TournamentError::InvalidInput(format!(
            "{field} must not be empty"
        )))
    } else {
        Ok(())
    }
}

fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn check_digest(field: &str, value: &str) -> Result<(), TournamentError> {
    if valid_digest(value) {
        Ok(())
    } else {
        Err(TournamentError::InvalidInput(format!(
            "{field} must be sha256:<64 hexadecimal characters>"
        )))
    }
}

fn checked_add(total: &mut u64, value: u64, field: &str) -> Result<(), TournamentError> {
    *total = total.checked_add(value).ok_or_else(|| {
        TournamentError::Arithmetic(format!("{field} accumulation overflow"))
    })?;
    Ok(())
}

fn checked_add_signed(total: &mut i64, value: i64, field: &str) -> Result<(), TournamentError> {
    *total = total.checked_add(value).ok_or_else(|| {
        TournamentError::Arithmetic(format!("{field} signed accumulation overflow"))
    })?;
    Ok(())
}

fn percentile_u64(values: &[u64], basis_points: u16) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank_numerator = (basis_points as usize).saturating_mul(sorted.len());
    let rank = rank_numerator.div_ceil(10_000).max(1).min(sorted.len());
    Some(sorted[rank - 1])
}

fn validate_input(input: &MobilityPolicyTournamentInput) -> Result<(), TournamentError> {
    if input.schema_version != SCHEMA_VERSION || input.benchmark_id != BENCHMARK_ID {
        return Err(TournamentError::InvalidInput(
            "schema_version or benchmark_id mismatch".to_owned(),
        ));
    }
    for (field, value) in [
        ("reference_policy_id", input.reference_policy_id.as_str()),
        ("compute_environment_digest", input.compute_environment_digest.as_str()),
    ] {
        nonempty(field, value)?;
    }
    check_digest("scenario manifest digest", &input.scenario_manifest_digest)?;
    check_digest("feasibility oracle digest", &input.feasibility_oracle_digest)?;
    check_digest("compute environment digest", &input.compute_environment_digest)?;

    if input.compute_budget_ms == 0
        || input.policies.len() < 2
        || input.policies.len() > MAX_POLICIES
        || input.runs.is_empty()
        || input.runs.len() > MAX_RUNS
    {
        return Err(TournamentError::InvalidInput(
            "compute budget or policy/run count is outside the bounded contract".to_owned(),
        ));
    }
    if input.bootstrap.seed == 0
        || !(MIN_BOOTSTRAP_RESAMPLES..=MAX_BOOTSTRAP_RESAMPLES)
            .contains(&input.bootstrap.resamples)
        || !(5_000..=9_900).contains(&input.bootstrap.confidence_basis_points)
        || !(2..=MAX_BOOTSTRAP_CLUSTERS)
            .contains(&input.bootstrap.minimum_independent_clusters)
    {
        return Err(TournamentError::InvalidInput(
            "bootstrap seed, resample count, confidence or minimum cluster count is invalid"
                .to_owned(),
        ));
    }

    let mut policies = BTreeMap::new();
    let mut information_models = BTreeSet::new();
    for policy in &input.policies {
        nonempty("policy_id", &policy.policy_id)?;
        nonempty("policy_version", &policy.policy_version)?;
        check_digest("policy configuration digest", &policy.configuration_digest)?;
        if policies.insert(policy.policy_id.as_str(), policy).is_some() {
            return Err(TournamentError::DuplicatePolicy(policy.policy_id.clone()));
        }
        information_models.insert(policy.information_model);
    }
    if !policies.contains_key(input.reference_policy_id.as_str()) {
        return Err(TournamentError::MissingPolicy(format!(
            "reference policy {} is not declared",
            input.reference_policy_id
        )));
    }
    if information_models.len() != 1 {
        return Err(TournamentError::PolicyMismatch(
            "online nonanticipative policies and clairvoyant offline upper bounds must use separate tournament tracks"
                .to_owned(),
        ));
    }

    let policy_families: BTreeMap<_, _> = policies
        .iter()
        .map(|(id, policy)| (*id, policy.family))
        .collect();
    let policy_configs: BTreeMap<_, _> = policies
        .iter()
        .map(|(id, policy)| (*id, policy.configuration_digest.as_str()))
        .collect();

    let mut run_ids = BTreeSet::new();
    let mut pair_keys: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    let mut pair_metadata: BTreeMap<(String, String), &PolicyRun> = BTreeMap::new();
    let mut run_count_by_policy: BTreeMap<&str, usize> = BTreeMap::new();

    for run in &input.runs {
        nonempty("run_id", &run.run_id)?;
        nonempty("policy_id", &run.policy_id)?;
        nonempty("scenario_id", &run.scenario_id)?;
        nonempty("scenario_family", &run.scenario_family)?;
        nonempty("cluster_id", &run.cluster_id)?;
        nonempty("replication_id", &run.replication_id)?;
        if !run_ids.insert(run.run_id.as_str()) {
            return Err(TournamentError::DuplicateRun(run.run_id.clone()));
        }
        if !policy_families.contains_key(run.policy_id.as_str()) {
            return Err(TournamentError::MissingPolicy(format!(
                "run {} references undeclared policy {}",
                run.run_id, run.policy_id
            )));
        }
        if run.evidence_class != input.evidence_class {
            return Err(TournamentError::EvidenceMismatch(format!(
                "run {} evidence class differs from the tournament manifest",
                run.run_id
            )));
        }
        check_digest("scenario digest", &run.scenario_digest)?;
        check_digest("scenario input digest", &run.scenario_input_digest)?;
        check_digest("information trace digest", &run.information_trace_digest)?;
        check_digest("information audit digest", &run.information_audit_digest)?;
        check_digest("policy configuration digest", &run.policy_configuration_digest)?;
        check_digest("run compute environment digest", &run.compute_environment_digest)?;
        if run.policy_configuration_digest
            != *policy_configs.get(run.policy_id.as_str()).unwrap()
        {
            return Err(TournamentError::PolicyMismatch(format!(
                "run {} does not match its declared policy configuration",
                run.run_id
            )));
        }
        if run.compute_environment_digest != input.compute_environment_digest
            || run.compute_budget_ms != input.compute_budget_ms
        {
            return Err(TournamentError::ComputeMismatch(format!(
                "run {} uses a different compute environment or budget",
                run.run_id
            )));
        }
        if run.committed_missions == 0
            || run.completed_missions > run.committed_missions
            || run.on_time_missions > run.completed_missions
        {
            return Err(TournamentError::InvalidMetric(format!(
                "run {} has inconsistent mission denominators",
                run.run_id
            )));
        }
        let accounted = run
            .completed_missions
            .checked_add(run.unserved_missions)
            .and_then(|value| value.checked_add(run.in_progress_at_horizon))
            .ok_or_else(|| TournamentError::Arithmetic("mission count overflow".to_owned()))?;
        if accounted != run.committed_missions {
            return Err(TournamentError::InvalidMetric(format!(
                "run {} must account for every committed mission as completed, unserved or in-progress",
                run.run_id
            )));
        }
        let mut categories = BTreeSet::new();
        for violation in &run.hard_gate_violations {
            if violation.count == 0 || !categories.insert(violation.category) {
                return Err(TournamentError::InvalidMetric(format!(
                    "run {} contains a zero-count or duplicate hard-gate category",
                    run.run_id
                )));
            }
        }
        *run_count_by_policy.entry(run.policy_id.as_str()).or_default() += 1;
        let pair_key = (run.scenario_id.clone(), run.replication_id.clone());
        let keys = pair_keys.entry(run.policy_id.clone()).or_default();
        if !keys.insert(pair_key.clone()) {
            return Err(TournamentError::DuplicateRun(format!(
                "duplicate policy/scenario/replication pair for run {}",
                run.run_id
            )));
        }
        if let Some(existing) = pair_metadata.get(&pair_key) {
            if existing.cluster_id != run.cluster_id
                || existing.scenario_family != run.scenario_family
                || existing.random_seed != run.random_seed
                || existing.scenario_digest != run.scenario_digest
                || existing.scenario_input_digest != run.scenario_input_digest
                || existing.compute_environment_digest != run.compute_environment_digest
                || existing.compute_budget_ms != run.compute_budget_ms
                || existing.evidence_class != run.evidence_class
            {
                return Err(TournamentError::PairMismatch(format!(
                    "policy outcomes do not share exogenous inputs and compute conditions for scenario {} replication {}",
                    run.scenario_id, run.replication_id
                )));
            }
        } else {
            pair_metadata.insert(pair_key, run);
        }
    }

    let expected_pairs = pair_keys
        .values()
        .next()
        .cloned()
        .ok_or_else(|| TournamentError::InvalidInput("no policy runs".to_owned()))?;
    for (policy_id, keys) in &pair_keys {
        if keys != &expected_pairs {
            let missing_count = expected_pairs.difference(keys).count();
            let extra_count = keys.difference(&expected_pairs).count();
            return Err(TournamentError::MissingRun(format!(
                "policy {policy_id} has a non-matching pair set (missing {missing_count}, extra {extra_count})"
            )));
        }
        if run_count_by_policy.get(policy_id.as_str()).copied().unwrap_or(0)
            > MAX_RUNS_PER_POLICY
        {
            return Err(TournamentError::InvalidInput(format!(
                "policy {policy_id} exceeds the per-policy run bound"
            )));
        }
    }
    if pair_keys.len() != input.policies.len() {
        return Err(TournamentError::MissingRun(
            "every declared policy must have at least one run".to_owned(),
        ));
    }
    if pair_metadata.len() > MAX_RUNS_PER_POLICY {
        return Err(TournamentError::InvalidInput(
            "paired scenario count exceeds the bounded contract".to_owned(),
        ));
    }
    let clusters: BTreeSet<_> = pair_metadata.values().map(|run| run.cluster_id.as_str()).collect();
    if clusters.len() > MAX_CLUSTERS {
        return Err(TournamentError::InvalidInput(
            "independent cluster count exceeds the bounded contract".to_owned(),
        ));
    }
    let bootstrap_work = (clusters.len() as u64)
        .checked_mul(input.bootstrap.resamples as u64)
        .and_then(|value| value.checked_mul((input.policies.len() - 1) as u64))
        .and_then(|value| value.checked_mul(BOOTSTRAP_METRIC_COUNT))
        .ok_or_else(|| {
            TournamentError::Arithmetic("bootstrap work estimate overflow".to_owned())
        })?;
    if bootstrap_work > MAX_BOOTSTRAP_WORK_ITEMS {
        return Err(TournamentError::InvalidInput(format!(
            "bootstrap work estimate {bootstrap_work} exceeds budget {MAX_BOOTSTRAP_WORK_ITEMS}; reduce resamples, policies or independent clusters"
        )));
    }
    Ok(())
}

fn canonical_input_digest(input: &MobilityPolicyTournamentInput) -> Result<String, TournamentError> {
    let mut canonical = input.clone();
    canonical.policies.sort_by(|a, b| a.policy_id.cmp(&b.policy_id));
    canonical.runs.sort_by(|a, b| {
        (&a.policy_id, &a.scenario_id, &a.replication_id)
            .cmp(&(&b.policy_id, &b.scenario_id, &b.replication_id))
    });
    for run in &mut canonical.runs {
        run.hard_gate_violations
            .sort_by_key(|violation| violation.category);
    }
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| TournamentError::Arithmetic(format!("canonical serialization failed: {error}")))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn percentile_f64(values: &[f64], basis_points: u16) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((basis_points as usize) * sorted.len()).div_ceil(10_000);
    Some(sorted[rank.max(1).min(sorted.len()) - 1])
}

#[derive(Clone)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed } }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn index(&mut self, upper: usize) -> usize {
        (self.next() as usize) % upper
    }
}

fn metric_seed(seed: u64, policy_id: &str, label: &str) -> u64 {
    let bytes = format!("{seed}:{policy_id}:{label}");
    let digest = Sha256::digest(bytes.as_bytes());
    u64::from_le_bytes(digest[0..8].try_into().unwrap()).max(1)
}

fn bootstrap_delta(
    per_cluster_deltas: &BTreeMap<String, f64>,
    config: &BootstrapConfig,
    policy_id: &str,
    label: &str,
) -> BootstrapDelta {
    let cluster_count = per_cluster_deltas.len() as u64;
    let mean_delta = if cluster_count == 0 {
        None
    } else {
        Some(per_cluster_deltas.values().sum::<f64>() / cluster_count as f64)
    };
    if cluster_count < config.minimum_independent_clusters as u64 {
        return BootstrapDelta {
            mean_delta,
            lower_percentile: None,
            upper_percentile: None,
            independent_cluster_count: cluster_count,
            bootstrap_resamples: config.resamples,
            confidence_basis_points: config.confidence_basis_points,
            interval_available: false,
        };
    }

    let values: Vec<_> = per_cluster_deltas.values().copied().collect();
    let tail = ((10_000 - config.confidence_basis_points) / 2) as u16;
    let upper = 10_000_u16.saturating_sub(tail);
    let mut rng = XorShift64::new(metric_seed(config.seed, policy_id, label));
    let mut samples = Vec::with_capacity(config.resamples as usize);
    for _ in 0..config.resamples {
        let mut sum = 0.0;
        for _ in 0..values.len() {
            sum += values[rng.index(values.len())];
        }
        samples.push(sum / values.len() as f64);
    }
    BootstrapDelta {
        mean_delta,
        lower_percentile: percentile_f64(&samples, tail),
        upper_percentile: percentile_f64(&samples, upper),
        independent_cluster_count: cluster_count,
        bootstrap_resamples: config.resamples,
        confidence_basis_points: config.confidence_basis_points,
        interval_available: true,
    }
}

fn cluster_means(
    values: &BTreeMap<String, Vec<f64>>,
) -> BTreeMap<String, f64> {
    values
        .iter()
        .filter_map(|(cluster, observations)| {
            if observations.is_empty() {
                None
            } else {
                Some((
                    cluster.clone(),
                    observations.iter().sum::<f64>() / observations.len() as f64,
                ))
            }
        })
        .collect()
}

fn sum_violations(
    run: &PolicyRun,
    information_model: InformationModel,
) -> BTreeMap<HardGateCategory, u64> {
    let mut result = BTreeMap::new();
    for violation in &run.hard_gate_violations {
        result.insert(violation.category, violation.count as u64);
    }
    if run.metered_grid_response_wh > 0
        && run.metered_grid_response_wh as u64 > run.maximum_dc_network_deliverable_wh
    {
        *result.entry(HardGateCategory::NetworkDeliverability).or_default() += 1;
    }
    if run.mission_reserve_shortfall_wh > 0 {
        result
            .entry(HardGateCategory::MissionReserve)
            .and_modify(|count| *count = (*count).max(1))
            .or_insert(1);
    }
    if run.recovery_shortfall_wh > 0 {
        result
            .entry(HardGateCategory::ReserveRecovery)
            .and_modify(|count| *count = (*count).max(1))
            .or_insert(1);
    }
    if run.nonanticipativity_violation_count > 0
        || (information_model == InformationModel::NonanticipativeOnline
            && run.future_information_access_count > 0)
    {
        result
            .entry(HardGateCategory::EvidenceIntegrity)
            .and_modify(|count| *count = (*count).max(1))
            .or_insert(1);
    }
    result
}

fn empirical_upper_cvar95(values: &[u64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    let tail_count = values.len().div_ceil(20).max(1);
    sorted.iter().take(tail_count).map(|value| *value as f64).sum::<f64>()
        / tail_count as f64
}

fn summarize_strata(
    runs: &[&PolicyRun],
    key: impl Fn(&PolicyRun) -> String,
    information_model: InformationModel,
) -> Result<Vec<StratumOutcomeSummary>, TournamentError> {
    let mut grouped: BTreeMap<String, Vec<&PolicyRun>> = BTreeMap::new();
    for run in runs {
        grouped.entry(key(run)).or_default().push(run);
    }
    let mut summaries = Vec::with_capacity(grouped.len());
    for (stratum_id, group) in grouped {
        let mut committed = 0_u64;
        let mut on_time = 0_u64;
        let mut metered = 0_i64;
        let mut hard_violation_count = 0_u64;
        let mut false_safe = 0_u64;
        let mut false_abstentions = 0_u64;
        let mut future_information_access_count = 0_u64;
        let mut nonanticipativity_violation_count = 0_u64;
        let mut waits = Vec::with_capacity(group.len());
        let mut reserve_shortfalls = Vec::with_capacity(group.len());
        let mut recovery_shortfalls = Vec::with_capacity(group.len());
        for run in &group {
            checked_add(&mut committed, run.committed_missions as u64, "stratum committed missions")?;
            checked_add(&mut on_time, run.on_time_missions as u64, "stratum on-time missions")?;
            checked_add_signed(&mut metered, run.metered_grid_response_wh, "stratum metered response")?;
            for count in sum_violations(run, information_model).values() {
                checked_add(&mut hard_violation_count, *count, "stratum hard violations")?;
            }
            if run.requested_grid_service_wh > 0 && run.shield_admitted && !run.oracle_action_feasible {
                false_safe += 1;
            }
            if run.requested_grid_service_wh > 0 && !run.shield_admitted && run.oracle_action_feasible {
                false_abstentions += 1;
            }
            checked_add(
                &mut future_information_access_count,
                run.future_information_access_count as u64,
                "stratum future information access count",
            )?;
            checked_add(
                &mut nonanticipativity_violation_count,
                run.nonanticipativity_violation_count as u64,
                "stratum nonanticipativity violation count",
            )?;
            waits.push(run.queue_wait_p95_ms);
            reserve_shortfalls.push(run.mission_reserve_shortfall_wh);
            recovery_shortfalls.push(run.recovery_shortfall_wh);
        }
        summaries.push(StratumOutcomeSummary {
            stratum_id,
            run_count: group.len() as u64,
            committed_missions: committed,
            on_time_service_rate_basis_points: on_time * 10_000 / committed,
            mean_queue_wait_p95_ms: waits.iter().map(|value| *value as f64).sum::<f64>() / waits.len() as f64,
            total_metered_grid_response_wh_signed: metered,
            max_mission_reserve_shortfall_wh: reserve_shortfalls.iter().copied().max().unwrap_or(0),
            max_recovery_shortfall_wh: recovery_shortfalls.iter().copied().max().unwrap_or(0),
            empirical_cvar95_mission_reserve_shortfall_wh: empirical_upper_cvar95(&reserve_shortfalls),
            empirical_cvar95_recovery_shortfall_wh: empirical_upper_cvar95(&recovery_shortfalls),
            future_information_access_count,
            nonanticipativity_violation_count,
            information_boundary_clean: nonanticipativity_violation_count == 0
                && (information_model != InformationModel::NonanticipativeOnline
                    || future_information_access_count == 0),
            hard_gate_violation_count,
            false_safe_admissions: false_safe,
            false_abstentions,
        });
    }
    Ok(summaries)
}

fn summarize_policy(
    identity: &PolicyIdentity,
    runs: &[&PolicyRun],
) -> Result<PolicySummary, TournamentError> {
    let mut totals = PolicySummary {
        policy_id: identity.policy_id.clone(),
        policy_version: identity.policy_version.clone(),
        family: identity.family,
        information_model: identity.information_model,
        run_count: runs.len() as u64,
        completed_run_count: 0,
        timed_out_run_count: 0,
        abstained_run_count: 0,
        failed_run_count: 0,
        independent_cluster_count: 0,
        committed_missions: 0,
        completed_missions: 0,
        on_time_missions: 0,
        unserved_missions: 0,
        in_progress_at_horizon: 0,
        on_time_service_rate_basis_points: 0,
        completion_rate_basis_points: 0,
        mean_queue_wait_p95_ms: 0.0,
        p95_queue_wait_p95_ms: 0,
        future_information_access_count: 0,
        nonanticipativity_violation_count: 0,
        information_boundary_clean: true,
        p95_mission_reserve_shortfall_wh: 0,
        p95_recovery_shortfall_wh: 0,
        empirical_cvar95_mission_reserve_shortfall_wh: 0.0,
        empirical_cvar95_recovery_shortfall_wh: 0.0,
        scenario_family_outcomes: Vec::new(),
        independent_cluster_outcomes: Vec::new(),
        total_requested_grid_service_wh: 0,
        total_instructed_grid_response_wh_signed: 0,
        total_metered_grid_response_wh_signed: 0,
        total_maximum_dc_network_deliverable_wh: 0,
        total_site_import_energy_wh: 0,
        total_renewable_curtailed_energy_wh: 0,
        total_battery_throughput_wh: 0,
        operating_cost_micro_units: 0,
        degradation_cost_micro_units: 0,
        emissions_grams_co2e: 0,
        operating_cost_per_committed_mission_micro_units: None,
        operating_cost_per_on_time_mission_micro_units: None,
        total_mission_reserve_shortfall_wh: 0,
        total_recovery_shortfall_wh: 0,
        mean_decision_latency_ms: 0.0,
        p95_decision_latency_ms: 0,
        false_safe_admissions: 0,
        false_abstentions: 0,
        shield_rejections: 0,
        network_cap_conflicts: 0,
        hard_gate_violations: Vec::new(),
        hard_gate_clean: true,
        false_safe_free: true,
    };
    let mut queue_wait = Vec::with_capacity(runs.len());
    let mut decision_latency = Vec::with_capacity(runs.len());
    let mut mission_reserve_shortfall = Vec::with_capacity(runs.len());
    let mut recovery_shortfall = Vec::with_capacity(runs.len());
    let mut clusters = BTreeSet::new();
    let mut violations: BTreeMap<HardGateCategory, u64> = BTreeMap::new();
    for run in runs {
        clusters.insert(run.cluster_id.as_str());
        match run.status {
            RunStatus::Completed => totals.completed_run_count += 1,
            RunStatus::TimedOut => totals.timed_out_run_count += 1,
            RunStatus::Abstained => totals.abstained_run_count += 1,
            RunStatus::Failed => totals.failed_run_count += 1,
        }
        checked_add(&mut totals.committed_missions, run.committed_missions as u64, "committed missions")?;
        checked_add(&mut totals.completed_missions, run.completed_missions as u64, "completed missions")?;
        checked_add(&mut totals.on_time_missions, run.on_time_missions as u64, "on-time missions")?;
        checked_add(&mut totals.unserved_missions, run.unserved_missions as u64, "unserved missions")?;
        checked_add(&mut totals.in_progress_at_horizon, run.in_progress_at_horizon as u64, "in-progress missions")?;
        checked_add(&mut totals.total_requested_grid_service_wh, run.requested_grid_service_wh, "requested grid service")?;
        checked_add_signed(&mut totals.total_instructed_grid_response_wh_signed, run.instructed_grid_response_wh, "instructed grid response")?;
        checked_add_signed(&mut totals.total_metered_grid_response_wh_signed, run.metered_grid_response_wh, "metered grid response")?;
        checked_add(&mut totals.total_maximum_dc_network_deliverable_wh, run.maximum_dc_network_deliverable_wh, "network deliverability")?;
        checked_add(&mut totals.total_site_import_energy_wh, run.site_import_energy_wh, "site import energy")?;
        checked_add(&mut totals.total_renewable_curtailed_energy_wh, run.renewable_curtailed_energy_wh, "renewable curtailment")?;
        checked_add(&mut totals.total_battery_throughput_wh, run.battery_throughput_wh, "battery throughput")?;
        checked_add(&mut totals.operating_cost_micro_units, run.operating_cost_micro_units, "operating cost")?;
        checked_add(&mut totals.degradation_cost_micro_units, run.degradation_cost_micro_units, "degradation cost")?;
        checked_add(&mut totals.emissions_grams_co2e, run.emissions_grams_co2e, "emissions")?;
        checked_add(&mut totals.total_mission_reserve_shortfall_wh, run.mission_reserve_shortfall_wh, "mission reserve shortfall")?;
        checked_add(&mut totals.total_recovery_shortfall_wh, run.recovery_shortfall_wh, "recovery shortfall")?;
        checked_add(
            &mut totals.future_information_access_count,
            run.future_information_access_count as u64,
            "future information access count",
        )?;
        checked_add(
            &mut totals.nonanticipativity_violation_count,
            run.nonanticipativity_violation_count as u64,
            "nonanticipativity violation count",
        )?;
        if run.requested_grid_service_wh > 0 && run.shield_admitted && !run.oracle_action_feasible {
            totals.false_safe_admissions += 1;
        }
        if run.requested_grid_service_wh > 0 && !run.shield_admitted && run.oracle_action_feasible {
            totals.false_abstentions += 1;
        }
        if run.requested_grid_service_wh > 0 && !run.shield_admitted {
            totals.shield_rejections += 1;
        }
        if run.metered_grid_response_wh > 0
            && run.metered_grid_response_wh as u64 > run.maximum_dc_network_deliverable_wh
        {
            totals.network_cap_conflicts += 1;
        }
        for (category, count) in sum_violations(run, identity.information_model) {
            checked_add(
                violations.entry(category).or_default(),
                count,
                "hard-gate violation count",
            )?;
        }
        queue_wait.push(run.queue_wait_p95_ms);
        decision_latency.push(run.decision_latency_ms);
        mission_reserve_shortfall.push(run.mission_reserve_shortfall_wh);
        recovery_shortfall.push(run.recovery_shortfall_wh);
    }

    totals.independent_cluster_count = clusters.len() as u64;
    totals.on_time_service_rate_basis_points =
        (totals.on_time_missions * 10_000) / totals.committed_missions;
    totals.completion_rate_basis_points =
        (totals.completed_missions * 10_000) / totals.committed_missions;
    totals.mean_queue_wait_p95_ms =
        queue_wait.iter().map(|value| *value as f64).sum::<f64>() / queue_wait.len() as f64;
    totals.p95_queue_wait_p95_ms = percentile_u64(&queue_wait, 9_500).unwrap_or(0);
    totals.information_boundary_clean =
        totals.nonanticipativity_violation_count == 0
            && (identity.information_model != InformationModel::NonanticipativeOnline
                || totals.future_information_access_count == 0);
    totals.p95_mission_reserve_shortfall_wh =
        percentile_u64(&mission_reserve_shortfall, 9_500).unwrap_or(0);
    totals.p95_recovery_shortfall_wh =
        percentile_u64(&recovery_shortfall, 9_500).unwrap_or(0);
    totals.empirical_cvar95_mission_reserve_shortfall_wh =
        empirical_upper_cvar95(&mission_reserve_shortfall);
    totals.empirical_cvar95_recovery_shortfall_wh =
        empirical_upper_cvar95(&recovery_shortfall);
    totals.scenario_family_outcomes = summarize_strata(
        runs,
        |run| run.scenario_family.clone(),
        identity.information_model,
    )?;
    totals.independent_cluster_outcomes = summarize_strata(
        runs,
        |run| run.cluster_id.clone(),
        identity.information_model,
    )?;
    totals.mean_decision_latency_ms =
        decision_latency.iter().map(|value| *value as f64).sum::<f64>() / decision_latency.len() as f64;
    totals.p95_decision_latency_ms = percentile_u64(&decision_latency, 9_500).unwrap_or(0);
    totals.operating_cost_per_committed_mission_micro_units =
        Some(totals.operating_cost_micro_units as f64 / totals.committed_missions as f64);
    if totals.on_time_missions > 0 {
        totals.operating_cost_per_on_time_mission_micro_units =
            Some(totals.operating_cost_micro_units as f64 / totals.on_time_missions as f64);
    }
    totals.hard_gate_violations = violations
        .iter()
        .map(|(category, count)| GateViolationCount { category: *category, count: *count })
        .collect();
    totals.hard_gate_clean = violations.values().all(|count| *count == 0);
    totals.false_safe_free = totals.false_safe_admissions == 0;
    Ok(totals)
}

fn paired_comparison(
    candidate: &PolicyIdentity,
    reference: &PolicyIdentity,
    runs_by_policy: &BTreeMap<&str, BTreeMap<(String, String), &PolicyRun>>,
    bootstrap: &BootstrapConfig,
) -> Result<PolicyComparison, TournamentError> {
    let candidate_runs = runs_by_policy.get(candidate.policy_id.as_str()).unwrap();
    let reference_runs = runs_by_policy.get(reference.policy_id.as_str()).unwrap();

    let mut cluster_service = BTreeMap::<String, Vec<f64>>::new();
    let mut cluster_queue = BTreeMap::<String, Vec<f64>>::new();
    let mut cluster_grid = BTreeMap::<String, Vec<f64>>::new();
    let mut cluster_recovery = BTreeMap::<String, Vec<f64>>::new();
    let mut cluster_cost = BTreeMap::<String, Vec<f64>>::new();
    let mut cluster_latency = BTreeMap::<String, Vec<f64>>::new();

    for (key, candidate_run) in candidate_runs {
        let reference_run = reference_runs.get(key).ok_or_else(|| {
            TournamentError::MissingRun(format!(
                "reference policy is missing paired scenario {} replication {}",
                key.0, key.1
            ))
        })?;
        let candidate_rate =
            candidate_run.on_time_missions as f64 / candidate_run.committed_missions as f64;
        let reference_rate =
            reference_run.on_time_missions as f64 / reference_run.committed_missions as f64;
        cluster_service
            .entry(candidate_run.cluster_id.clone())
            .or_default()
            .push((candidate_rate - reference_rate) * 10_000.0);
        cluster_queue
            .entry(candidate_run.cluster_id.clone())
            .or_default()
            .push(candidate_run.queue_wait_p95_ms as f64 - reference_run.queue_wait_p95_ms as f64);
        cluster_grid
            .entry(candidate_run.cluster_id.clone())
            .or_default()
            .push(candidate_run.metered_grid_response_wh as f64 - reference_run.metered_grid_response_wh as f64);
        cluster_recovery
            .entry(candidate_run.cluster_id.clone())
            .or_default()
            .push(candidate_run.recovery_shortfall_wh as f64 - reference_run.recovery_shortfall_wh as f64);
        let candidate_cost =
            candidate_run.operating_cost_micro_units as f64 / candidate_run.committed_missions as f64;
        let reference_cost =
            reference_run.operating_cost_micro_units as f64 / reference_run.committed_missions as f64;
        cluster_cost
            .entry(candidate_run.cluster_id.clone())
            .or_default()
            .push(candidate_cost - reference_cost);
        cluster_latency
            .entry(candidate_run.cluster_id.clone())
            .or_default()
            .push(candidate_run.decision_latency_ms as f64 - reference_run.decision_latency_ms as f64);
    }

    let service = cluster_means(&cluster_service);
    let queue = cluster_means(&cluster_queue);
    let grid = cluster_means(&cluster_grid);
    let recovery = cluster_means(&cluster_recovery);
    let cost = cluster_means(&cluster_cost);
    let latency = cluster_means(&cluster_latency);
    let clusters = service.len() as u64;

    Ok(PolicyComparison {
        policy_id: candidate.policy_id.clone(),
        reference_policy_id: reference.policy_id.clone(),
        paired_run_count: candidate_runs.len() as u64,
        independent_cluster_count: clusters,
        on_time_service_rate_basis_points: bootstrap_delta(&service, bootstrap, &candidate.policy_id, "on_time_service_rate_bps"),
        mean_queue_wait_p95_ms: bootstrap_delta(&queue, bootstrap, &candidate.policy_id, "queue_wait_p95_ms"),
        signed_metered_grid_response_wh_per_run: bootstrap_delta(&grid, bootstrap, &candidate.policy_id, "metered_grid_response_wh"),
        recovery_shortfall_wh_per_run: bootstrap_delta(&recovery, bootstrap, &candidate.policy_id, "recovery_shortfall_wh"),
        operating_cost_micro_units_per_committed_mission: bootstrap_delta(&cost, bootstrap, &candidate.policy_id, "cost_per_committed_mission"),
        decision_latency_ms_per_run: bootstrap_delta(&latency, bootstrap, &candidate.policy_id, "decision_latency_ms"),
    })
}

pub fn evaluate_mobility_policy_tournament(
    input: &MobilityPolicyTournamentInput,
) -> Result<MobilityPolicyTournamentReport, TournamentError> {
    validate_input(input)?;
    let input_digest = canonical_input_digest(input)?;
    let information_model = input.policies[0].information_model;

    let mut runs_by_policy: BTreeMap<&str, BTreeMap<(String, String), &PolicyRun>> =
        BTreeMap::new();
    for run in &input.runs {
        runs_by_policy
            .entry(run.policy_id.as_str())
            .or_default()
            .insert((run.scenario_id.clone(), run.replication_id.clone()), run);
    }

    let mut policies = Vec::with_capacity(input.policies.len());
    for identity in &input.policies {
        let runs: Vec<_> = runs_by_policy
            .get(identity.policy_id.as_str())
            .unwrap()
            .values()
            .copied()
            .collect();
        policies.push(summarize_policy(identity, &runs)?);
    }
    policies.sort_by(|a, b| a.policy_id.cmp(&b.policy_id));

    let reference = input
        .policies
        .iter()
        .find(|policy| policy.policy_id == input.reference_policy_id)
        .unwrap();
    let mut comparisons = Vec::new();
    for candidate in &input.policies {
        if candidate.policy_id != input.reference_policy_id {
            comparisons.push(paired_comparison(
                candidate,
                reference,
                &runs_by_policy,
                &input.bootstrap,
            )?);
        }
    }
    comparisons.sort_by(|a, b| a.policy_id.cmp(&b.policy_id));

    let all_policies_hard_gate_clean = policies.iter().all(|policy| policy.hard_gate_clean);
    let all_policies_false_safe_free = policies.iter().all(|policy| policy.false_safe_free);
    let first = runs_by_policy.values().next().unwrap();
    Ok(MobilityPolicyTournamentReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        evidence_class: input.evidence_class,
        information_model,
        scenario_manifest_digest: input.scenario_manifest_digest.clone(),
        feasibility_oracle_digest: input.feasibility_oracle_digest.clone(),
        compute_environment_digest: input.compute_environment_digest.clone(),
        compute_budget_ms: input.compute_budget_ms,
        reference_policy_id: input.reference_policy_id.clone(),
        input_digest,
        matched_pair_count: first.len() as u64,
        independent_cluster_count: first.values().map(|run| run.cluster_id.as_str()).collect::<BTreeSet<_>>().len() as u64,
        policies,
        comparisons,
        all_policies_hard_gate_clean,
        all_policies_false_safe_free,
        no_performance_score_offsets_hard_gate_failures: true,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn policy(policy_id: &str, family: PolicyFamily, marker: char) -> PolicyIdentity {
        PolicyIdentity {
            policy_id: policy_id.to_owned(),
            policy_version: "v0.1".to_owned(),
            family,
            information_model: InformationModel::NonanticipativeOnline,
            configuration_digest: digest(marker),
        }
    }

    fn run(
        policy_id: &str,
        policy_config: &str,
        scenario_id: &str,
        cluster_id: &str,
        seed: u64,
        on_time: u32,
        metered_wh: i64,
        cost: u64,
    ) -> PolicyRun {
        let scenario_index = scenario_id.chars().last().unwrap().to_digit(10).unwrap();
        let reference_feasible = true;
        PolicyRun {
            run_id: format!("{policy_id}-{scenario_id}"),
            policy_id: policy_id.to_owned(),
            scenario_id: scenario_id.to_owned(),
            scenario_family: format!("family-{}", scenario_index),
            cluster_id: cluster_id.to_owned(),
            replication_id: format!("rep-{scenario_id}"),
            random_seed: seed,
            scenario_digest: digest('1'),
            scenario_input_digest: digest('2'),
            policy_configuration_digest: policy_config.to_owned(),
            compute_environment_digest: digest('3'),
            compute_budget_ms: 10_000,
            evidence_class: EvidenceClass::Synthetic,
            status: RunStatus::Completed,
            oracle_action_feasible: reference_feasible,
            shield_admitted: true,
            requested_grid_service_wh: 100,
            hard_gate_violations: Vec::new(),
            committed_missions: 10,
            completed_missions: 9,
            on_time_missions: on_time,
            unserved_missions: 1,
            in_progress_at_horizon: 0,
            queue_wait_p95_ms: 20_000 + scenario_index as u64 * 1_000,
            mission_reserve_shortfall_wh: 0,
            recovery_shortfall_wh: 0,
            instructed_grid_response_wh: metered_wh,
            metered_grid_response_wh: metered_wh,
            maximum_dc_network_deliverable_wh: 200,
            site_import_energy_wh: 10_000,
            renewable_curtailed_energy_wh: 100,
            battery_throughput_wh: 5_000,
            operating_cost_micro_units: cost,
            degradation_cost_micro_units: cost / 10,
            emissions_grams_co2e: 500,
            decision_latency_ms: 5,
        }
    }

    fn fixture() -> MobilityPolicyTournamentInput {
        let fixed = policy("fixed-cap", PolicyFamily::MobilityOnlyFixedCap, 'a');
        let mpc = policy("mpc", PolicyFamily::RollingHorizonMpc, 'b');
        let shielded = policy("shielded-drl", PolicyFamily::ShieldedLearnedPolicy, 'c');
        let mut runs = Vec::new();
        for i in 1..=4 {
            let scenario = format!("scenario-{i}");
            let cluster = format!("site-day-{i}");
            let seed = 100 + i;
            runs.push(run("fixed-cap", &fixed.configuration_digest, &scenario, &cluster, seed, 8, 100, 100_000));
            runs.push(run("mpc", &mpc.configuration_digest, &scenario, &cluster, seed, 9, 140, 110_000));
            runs.push(run("shielded-drl", &shielded.configuration_digest, &scenario, &cluster, seed, 9, 160, 105_000));
        }
        MobilityPolicyTournamentInput {
            schema_version: SCHEMA_VERSION.to_owned(),
            benchmark_id: BENCHMARK_ID.to_owned(),
            evidence_class: EvidenceClass::Synthetic,
            scenario_manifest_digest: digest('d'),
            feasibility_oracle_digest: digest('e'),
            compute_environment_digest: digest('3'),
            compute_budget_ms: 10_000,
            reference_policy_id: "fixed-cap".to_owned(),
            bootstrap: BootstrapConfig {
                seed: 20261125,
                resamples: 1_000,
                confidence_basis_points: 9_500,
                minimum_independent_clusters: 4,
            },
            policies: vec![fixed, mpc, shielded],
            runs,
        }
    }

    #[test]
    fn matched_policy_matrix_reports_paired_metrics_and_bootstrap_intervals() {
        let report = evaluate_mobility_policy_tournament(&fixture()).unwrap();
        assert_eq!(report.matched_pair_count, 4);
        assert_eq!(report.independent_cluster_count, 4);
        assert!(report.all_policies_hard_gate_clean);
        assert!(report.all_policies_false_safe_free);
        assert!(report.no_performance_score_offsets_hard_gate_failures);
        let mpc = report.comparisons.iter().find(|c| c.policy_id == "mpc").unwrap();
        assert_eq!(mpc.paired_run_count, 4);
        assert_eq!(mpc.on_time_service_rate_basis_points.mean_delta, Some(1_000.0));
        assert!(mpc.on_time_service_rate_basis_points.interval_available);
        assert_eq!(
            mpc.on_time_service_rate_basis_points.lower_percentile,
            Some(1_000.0)
        );
        assert_eq!(
            mpc.on_time_service_rate_basis_points.upper_percentile,
            Some(1_000.0)
        );
        assert_eq!(mpc.signed_metered_grid_response_wh_per_run.mean_delta, Some(40.0));
        assert_eq!(
            mpc.signed_metered_grid_response_wh_per_run.lower_percentile,
            Some(40.0)
        );
        assert_eq!(
            mpc.signed_metered_grid_response_wh_per_run.upper_percentile,
            Some(40.0)
        );
        let summary = report.policies.iter().find(|p| p.policy_id == "mpc").unwrap();
        assert_eq!(summary.on_time_service_rate_basis_points, 9_000);
        assert_eq!(summary.timed_out_run_count, 0);
        assert_eq!(summary.operating_cost_micro_units, 440_000);
    }

    #[test]
    fn timeout_and_abstention_remain_in_the_policy_denominator() {
        let mut input = fixture();
        let run = input.runs.iter_mut().find(|r| r.policy_id == "mpc" && r.scenario_id == "scenario-4").unwrap();
        run.status = RunStatus::TimedOut;
        run.shield_admitted = false;
        let report = evaluate_mobility_policy_tournament(&input).unwrap();
        let mpc = report.policies.iter().find(|p| p.policy_id == "mpc").unwrap();
        assert_eq!(mpc.run_count, 4);
        assert_eq!(mpc.timed_out_run_count, 1);
        assert_eq!(mpc.on_time_missions, 36);
    }

    #[test]
    fn false_safe_admission_is_visible_and_not_compensated_by_cost_or_service() {
        let mut input = fixture();
        let run = input.runs.iter_mut().find(|r| r.policy_id == "shielded-drl" && r.scenario_id == "scenario-4").unwrap();
        run.oracle_action_feasible = false;
        run.shield_admitted = true;
        let report = evaluate_mobility_policy_tournament(&input).unwrap();
        let shielded = report.policies.iter().find(|p| p.policy_id == "shielded-drl").unwrap();
        assert_eq!(shielded.false_safe_admissions, 1);
        assert!(!shielded.false_safe_free);
        assert!(!report.all_policies_false_safe_free);
    }

    #[test]
    fn false_abstention_is_separately_counted() {
        let mut input = fixture();
        let run = input.runs.iter_mut().find(|r| r.policy_id == "shielded-drl" && r.scenario_id == "scenario-1").unwrap();
        run.shield_admitted = false;
        let report = evaluate_mobility_policy_tournament(&input).unwrap();
        let shielded = report.policies.iter().find(|p| p.policy_id == "shielded-drl").unwrap();
        assert_eq!(shielded.false_abstentions, 1);
        assert_eq!(shielded.shield_rejections, 1);
    }

    #[test]
    fn missing_or_duplicate_pair_fails_instead_of_dropping_runs() {
        let mut missing = fixture();
        missing.runs.retain(|r| !(r.policy_id == "mpc" && r.scenario_id == "scenario-4"));
        assert!(matches!(
            evaluate_mobility_policy_tournament(&missing),
            Err(TournamentError::MissingRun(_))
        ));

        let mut duplicate = fixture();
        duplicate.runs.push(duplicate.runs[0].clone());
        assert!(matches!(
            evaluate_mobility_policy_tournament(&duplicate),
            Err(TournamentError::DuplicateRun(_))
        ));
    }

    #[test]
    fn scenario_family_and_exogenous_input_digest_must_match_across_policies() {
        let mut family = fixture();
        family
            .runs
            .iter_mut()
            .find(|run| run.policy_id == "mpc" && run.scenario_id == "scenario-2")
            .unwrap()
            .scenario_family = "other-family".to_owned();
        assert!(matches!(
            evaluate_mobility_policy_tournament(&family),
            Err(TournamentError::PairMismatch(_))
        ));

        let mut input_digest = fixture();
        input_digest
            .runs
            .iter_mut()
            .find(|run| run.policy_id == "shielded-drl" && run.scenario_id == "scenario-3")
            .unwrap()
            .scenario_input_digest = digest('9');
        assert!(matches!(
            evaluate_mobility_policy_tournament(&input_digest),
            Err(TournamentError::PairMismatch(_))
        ));
    }

    #[test]
    fn evidence_class_must_match_declared_tournament_track() {
        let mut input = fixture();
        input
            .runs
            .iter_mut()
            .find(|run| run.policy_id == "mpc")
            .unwrap()
            .evidence_class = EvidenceClass::MeasuredReplay;
        assert!(matches!(
            evaluate_mobility_policy_tournament(&input),
            Err(TournamentError::EvidenceMismatch(_))
        ));
    }

    #[test]
    fn compute_environment_budget_and_seed_must_match_across_policies() {
        let mut environment = fixture();
        environment.runs.iter_mut().find(|r| r.policy_id == "mpc").unwrap().compute_budget_ms += 1;
        assert!(matches!(
            evaluate_mobility_policy_tournament(&environment),
            Err(TournamentError::ComputeMismatch(_))
        ));

        let mut seed = fixture();
        seed.runs.iter_mut().find(|r| r.policy_id == "mpc").unwrap().random_seed += 1;
        assert!(matches!(
            evaluate_mobility_policy_tournament(&seed),
            Err(TournamentError::PairMismatch(_))
        ));
    }

    #[test]
    fn clairvoyant_upper_bound_cannot_share_online_comparison_track() {
        let mut input = fixture();
        input.policies[1].information_model = InformationModel::ClairvoyantOfflineUpperBound;
        assert!(matches!(
            evaluate_mobility_policy_tournament(&input),
            Err(TournamentError::PolicyMismatch(_))
        ));
    }

    #[test]
    fn signed_negative_metered_response_is_preserved() {
        let mut input = fixture();
        input.runs.iter_mut().find(|r| r.policy_id == "mpc" && r.scenario_id == "scenario-4").unwrap().metered_grid_response_wh = -25;
        let report = evaluate_mobility_policy_tournament(&input).unwrap();
        let mpc = report.policies.iter().find(|p| p.policy_id == "mpc").unwrap();
        assert_eq!(mpc.total_metered_grid_response_wh_signed, 395);
    }

    #[test]
    fn measured_response_above_dc_cap_counts_as_hard_network_violation() {
        let mut input = fixture();
        let run = input.runs.iter_mut().find(|r| r.policy_id == "mpc" && r.scenario_id == "scenario-1").unwrap();
        run.metered_grid_response_wh = 250;
        run.maximum_dc_network_deliverable_wh = 200;
        let report = evaluate_mobility_policy_tournament(&input).unwrap();
        let mpc = report.policies.iter().find(|p| p.policy_id == "mpc").unwrap();
        assert_eq!(mpc.network_cap_conflicts, 1);
        assert!(!mpc.hard_gate_clean);
    }

    #[test]
    fn hard_gate_counts_are_not_hidden_by_aggregate_policy_performance() {
        let mut input = fixture();
        let run = input.runs.iter_mut().find(|r| r.policy_id == "mpc" && r.scenario_id == "scenario-2").unwrap();
        run.hard_gate_violations.push(HardGateViolation {
            category: HardGateCategory::MissionReserve,
            count: 1,
        });
        let report = evaluate_mobility_policy_tournament(&input).unwrap();
        let mpc = report.policies.iter().find(|p| p.policy_id == "mpc").unwrap();
        assert!(!mpc.hard_gate_clean);
        assert_eq!(mpc.hard_gate_violations[0].count, 1);
    }

    #[test]
    fn order_permutations_preserve_canonical_digest_and_report() {
        let first = evaluate_mobility_policy_tournament(&fixture()).unwrap();
        let mut permuted = fixture();
        permuted.policies.reverse();
        permuted.runs.reverse();
        for run in &mut permuted.runs {
            run.hard_gate_violations.reverse();
        }
        let second = evaluate_mobility_policy_tournament(&permuted).unwrap();
        assert_eq!(first.input_digest, second.input_digest);
        assert_eq!(first.policies, second.policies);
        assert_eq!(first.comparisons, second.comparisons);
    }

    #[test]
    fn tail_risk_and_worst_strata_remain_visible_beside_means() {
        let mut input = fixture();
        let run = input.runs.iter_mut().find(|r| r.policy_id == "mpc" && r.scenario_id == "scenario-4").unwrap();
        run.mission_reserve_shortfall_wh = 900;
        run.recovery_shortfall_wh = 1_200;
        let report = evaluate_mobility_policy_tournament(&input).unwrap();
        let mpc = report.policies.iter().find(|p| p.policy_id == "mpc").unwrap();
        assert_eq!(mpc.p95_mission_reserve_shortfall_wh, 900);
        assert_eq!(mpc.empirical_cvar95_mission_reserve_shortfall_wh, 900.0);
        assert_eq!(mpc.empirical_cvar95_recovery_shortfall_wh, 1_200.0);
        assert_eq!(mpc.scenario_family_outcomes.len(), 4);
        assert_eq!(mpc.independent_cluster_outcomes.len(), 4);
        assert_eq!(
            mpc.scenario_family_outcomes
                .iter()
                .find(|s| s.stratum_id == "family-4")
                .unwrap()
                .max_recovery_shortfall_wh,
            1_200
        );
    }

    #[test]
    fn insufficient_independent_clusters_does_not_emit_bootstrap_interval() {
        let mut input = fixture();
        input.bootstrap.minimum_independent_clusters = 5;
        let report = evaluate_mobility_policy_tournament(&input).unwrap();
        let mpc = report.comparisons.iter().find(|c| c.policy_id == "mpc").unwrap();
        assert!(!mpc.on_time_service_rate_basis_points.interval_available);
        assert_eq!(mpc.on_time_service_rate_basis_points.mean_delta, Some(1_000.0));
    }

    #[test]
    fn bootstrap_work_budget_rejects_excessive_resample_cluster_policy_product() {
        let mut input = fixture();
        input.bootstrap.resamples = 50_000;
        input.policies = (0..32)
            .map(|index| {
                let marker = char::from_digit((index % 16) as u32, 16).unwrap();
                policy(
                    &format!("policy-{index}"),
                    PolicyFamily::Other,
                    marker,
                )
            })
            .collect();
        input.reference_policy_id = "policy-0".to_owned();
        input.runs.clear();
        for policy_identity in &input.policies {
            for index in 0..100 {
                let scenario_id = format!("scenario-{index}");
                let cluster_id = format!("cluster-{index}");
                let seed = index as u64 + 1;
                let mut run = run(
                    &policy_identity.policy_id,
                    &policy_identity.configuration_digest,
                    &scenario_id,
                    &cluster_id,
                    seed,
                    8,
                    100,
                    100_000,
                );
                run.scenario_family = format!("family-{index}");
                input.runs.push(run);
            }
        }
        assert!(matches!(
            evaluate_mobility_policy_tournament(&input),
            Err(TournamentError::InvalidInput(message)) if message.contains("bootstrap work estimate")
        ));
    }

    #[test]
    fn mission_denominators_must_account_for_in_progress_and_unserved_work() {
        let mut input = fixture();
        input.runs[0].completed_missions = 8;
        assert!(matches!(
            evaluate_mobility_policy_tournament(&input),
            Err(TournamentError::InvalidMetric(_))
        ));
    }

    #[test]
    fn zero_on_time_denominator_makes_cost_per_success_unavailable() {
        let mut input = fixture();
        for run in input.runs.iter_mut().filter(|r| r.policy_id == "mpc") {
            run.on_time_missions = 0;
        }
        let report = evaluate_mobility_policy_tournament(&input).unwrap();
        let mpc = report.policies.iter().find(|p| p.policy_id == "mpc").unwrap();
        assert_eq!(mpc.on_time_service_rate_basis_points, 0);
        assert!(mpc.operating_cost_per_on_time_mission_micro_units.is_none());
    }
}
