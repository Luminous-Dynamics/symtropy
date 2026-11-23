// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Probability-provenance and tail-risk evaluator for nonanticipative mobility
//! service policies.
//!
//! This module re-runs the upstream scenario-tree oracle. It never accepts
//! user-authored leaf service outcomes or probabilities. It computes exact
//! finite-scenario expectations, fractional-mass CVaR, and a total-variation
//! ambiguity sensitivity bound over the upstream leaf distribution.
//!
//! A total-variation ball is not a Wasserstein or moment ambiguity set. The
//! evaluator does not calibrate probabilities; that evidence remains owned by
//! #6460. Stress-only scenarios are not probability-weighted here.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::nonanticipative_service_oracle::{
    NonanticipativeOracleStatus, NonanticipativeServiceOracleInput,
    evaluate_nonanticipative_service_oracle,
};
use crate::sustainable_mobility_service_capacity::SustainableServiceCapacityInput;
use crate::sustainable_mobility_service_quality::LinkedServiceQualityInput;

const MAX_LEAVES: usize = 16;
const PROBABILITY_SCALE_PPM: u128 = 1_000_000;
const MAX_PROBABILITY_DENOMINATOR: u128 = 1_000_000_000_000_000_000_000_000;
const CLAIM_CEILING: &str = "Exact finite-scenario probability, CVaR and total-variation sensitivity arithmetic over leaf outcomes re-derived from the supplied nonanticipative service oracle. No probability calibration, DRO guarantee, field reliability, flight, dispatch or market authority.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbabilityBasis {
    Synthetic,
    HistoricalFrequencyEstimate,
    ForecastModel,
    ExpertElicitation,
    StressOnly,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProbabilityManifest {
    pub schema_version: String,
    pub benchmark_id: String,
    pub probability_set_id: String,
    pub probability_basis: ProbabilityBasis,
    pub probability_source_reference: String,
    pub scenario_tree_revision: String,
    pub scenario_tree_digest: String,
    pub scenario_tree_reference: String,
    pub scenario_tree_benchmark_id: String,
    pub expected_leaf_node_ids: Vec<String>,
    pub source_dataset_digest: Option<String>,
    pub calibration_report_digest: Option<String>,
    pub geography: String,
    pub service_population: String,
    pub service_class: String,
    pub joint_event_definition: String,
    pub data_version: String,
    pub training_cutoff_seconds: Option<u64>,
    pub evaluation_window_start_seconds: Option<u64>,
    pub evaluation_window_end_seconds: Option<u64>,
    pub forecast_issue_time_seconds: Option<u64>,
    pub sample_count: Option<u64>,
    pub effective_sample_size_milli: Option<u64>,
    pub expert_panel_count: Option<u16>,
    pub elicitation_protocol_reference: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityServiceRiskInput {
    pub schema_version: String,
    pub benchmark_id: String,
    /// Worst-tail probability mass; 100_000 means the worst 10% of probability.
    pub tail_mass_ppm: u32,
    /// Maximum total-variation probability mass shifted to adverse leaves.
    pub total_variation_radius_ppm: u32,
    pub probability_manifest: ServiceProbabilityManifest,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExactRiskRatio {
    /// The metric is numerator / denominator. No rounded value is used as the
    /// audit result.
    pub numerator: u128,
    pub denominator: u128,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ServiceLeafRiskResult {
    pub leaf_node_id: String,
    pub probability_numerator: u128,
    pub probability_denominator: u128,
    pub on_time_services: usize,
    pub completed_by_horizon: usize,
    pub late_services: usize,
    pub in_progress_services: usize,
    pub rejected_requests: usize,
    pub unserved_requests: usize,
    pub lateness_seconds: u64,
    pub completed_latency_seconds: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MobilityServiceRiskReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub probability_set_id: String,
    pub probability_basis: ProbabilityBasis,
    pub probability_source_reference: String,
    pub probability_provenance_complete: bool,
    pub calibration_quality_recomputed: bool,
    pub synthetic_only: bool,
    pub source_tree_digest: String,
    pub source_service_digest: String,
    pub source_capacity_digest: String,
    pub upstream_information_model: &'static str,
    pub upstream_policy_is_deployable: bool,
    pub leaf_count: usize,
    pub probability_denominator: u128,
    pub tail_mass_ppm: u32,
    pub total_variation_radius_ppm: u32,
    pub nominal_expected_on_time_services: ExactRiskRatio,
    pub nominal_expected_completed_services: ExactRiskRatio,
    pub nominal_expected_lateness_seconds: ExactRiskRatio,
    pub lower_tail_cvar_on_time_services: ExactRiskRatio,
    pub lower_tail_cvar_completed_services: ExactRiskRatio,
    pub upper_tail_cvar_lateness_seconds: ExactRiskRatio,
    pub upper_tail_cvar_unserved_plus_rejected: ExactRiskRatio,
    pub tv_robust_min_expected_on_time_services: ExactRiskRatio,
    pub tv_robust_min_expected_completed_services: ExactRiskRatio,
    pub tv_robust_max_expected_lateness_seconds: ExactRiskRatio,
    pub worst_case_on_time_services: usize,
    pub worst_case_completed_services: usize,
    pub worst_case_lateness_seconds: u64,
    pub leaf_results: Vec<ServiceLeafRiskResult>,
    pub stress_only_scenarios_included: bool,
    pub probability_calibration_claim_granted: bool,
    pub ledger_or_physical_mutation_performed: bool,
    pub flight_authority_granted: bool,
    pub market_or_dispatch_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone)]
struct RiskLeaf {
    id: String,
    weight: u128,
    on_time: u64,
    completed: u64,
    lateness: u64,
    unserved_plus_rejected: u64,
}

fn add_error(errors: &mut Vec<String>, message: impl Into<String>) {
    errors.push(message.into());
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(71);
    output.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn source_digest<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("risk_source_serialization_failed:{error}"))?;
    Ok(sha256_hex(&bytes))
}

fn canonical_tree_digest(tree: &NonanticipativeServiceOracleInput) -> Result<String, String> {
    let mut canonical = tree.clone();
    canonical.nodes.sort_by(|a, b| a.node_id.cmp(&b.node_id));
    for node in &mut canonical.nodes {
        node.request_reveals
            .sort_by(|a, b| a.request_id.cmp(&b.request_id));
        node.aerial_reveals
            .sort_by(|a, b| a.opportunity_id.cmp(&b.opportunity_id));
        node.fallback_reveals
            .sort_by(|a, b| a.opportunity_id.cmp(&b.opportunity_id));
    }
    source_digest(&canonical)
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_probability_manifest(
    manifest: &ServiceProbabilityManifest,
    tree: &NonanticipativeServiceOracleInput,
    leaf_ids: &BTreeSet<&str>,
    source_tree_digest: &str,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if manifest.schema_version != "0.1.0"
        || manifest.benchmark_id != "MOBILITY-SERVICE-PROBABILITY-SET-001"
        || manifest.probability_set_id.trim().is_empty()
        || manifest.probability_source_reference.trim().is_empty()
        || manifest.scenario_tree_revision.len() != 40
        || !manifest
            .scenario_tree_revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || !valid_sha256(&manifest.scenario_tree_digest)
        || manifest.scenario_tree_reference.trim().is_empty()
        || manifest.scenario_tree_benchmark_id != tree.benchmark_id
        || manifest.geography.trim().is_empty()
        || manifest.service_population.trim().is_empty()
        || manifest.service_class.trim().is_empty()
        || manifest.joint_event_definition.trim().is_empty()
        || manifest.data_version.trim().is_empty()
        || manifest.expected_leaf_node_ids.is_empty()
        || manifest.expected_leaf_node_ids.len() > MAX_LEAVES
    {
        add_error(
            &mut errors,
            "probability_manifest_identity_or_bounds_invalid",
        );
    }

    let mut expected = BTreeSet::new();
    for id in &manifest.expected_leaf_node_ids {
        if id.trim().is_empty() || !expected.insert(id.as_str()) {
            add_error(
                &mut errors,
                "probability_manifest_leaf_identity_duplicate_or_empty",
            );
        }
    }
    if &expected != leaf_ids {
        add_error(&mut errors, "probability_manifest_leaf_coverage_mismatch");
    }
    if manifest.scenario_tree_digest != source_tree_digest {
        add_error(&mut errors, "probability_manifest_scenario_tree_digest_mismatch");
    }

    match manifest.probability_basis {
        ProbabilityBasis::Synthetic => {
            if manifest.sample_count.unwrap_or(0) != 0
                || manifest.effective_sample_size_milli.unwrap_or(0) != 0
                || manifest.source_dataset_digest.is_some()
                || manifest.calibration_report_digest.is_some()
            {
                add_error(
                    &mut errors,
                    "synthetic_probability_must_not_claim_empirical_calibration",
                );
            }
        }
        ProbabilityBasis::HistoricalFrequencyEstimate => {
            if manifest
                .source_dataset_digest
                .as_deref()
                .is_none_or(|v| !valid_sha256(v))
                || manifest
                    .calibration_report_digest
                    .as_deref()
                    .is_none_or(|v| !valid_sha256(v))
                || manifest.sample_count.unwrap_or(0) == 0
                || manifest.effective_sample_size_milli.unwrap_or(0) == 0
                || manifest.effective_sample_size_milli.unwrap_or(0)
                    > manifest.sample_count.unwrap_or(0).saturating_mul(1_000)
                || manifest
                    .evaluation_window_start_seconds
                    .zip(manifest.evaluation_window_end_seconds)
                    .is_none_or(|(start, end)| start >= end)
                || manifest
                    .training_cutoff_seconds
                    .zip(manifest.evaluation_window_start_seconds)
                    .is_none_or(|(cutoff, start)| cutoff >= start)
            {
                add_error(
                    &mut errors,
                    "historical_probability_provenance_or_holdout_invalid",
                );
            }
        }
        ProbabilityBasis::ForecastModel => {
            if manifest
                .source_dataset_digest
                .as_deref()
                .is_none_or(|v| !valid_sha256(v))
                || manifest
                    .calibration_report_digest
                    .as_deref()
                    .is_none_or(|v| !valid_sha256(v))
                || manifest.training_cutoff_seconds.is_none()
                || manifest.forecast_issue_time_seconds.is_none()
                || manifest
                    .training_cutoff_seconds
                    .zip(manifest.forecast_issue_time_seconds)
                    .is_none_or(|(cutoff, issue)| cutoff >= issue)
                || manifest
                    .forecast_issue_time_seconds
                    .zip(manifest.evaluation_window_start_seconds)
                    .is_none_or(|(issue, start)| issue > start)
                || manifest
                    .evaluation_window_start_seconds
                    .zip(manifest.evaluation_window_end_seconds)
                    .is_none_or(|(start, end)| start >= end)
                || manifest.sample_count.unwrap_or(0) == 0
                || manifest.effective_sample_size_milli.unwrap_or(0) == 0
            {
                add_error(
                    &mut errors,
                    "forecast_probability_provenance_or_no_leakage_invalid",
                );
            }
        }
        ProbabilityBasis::ExpertElicitation => {
            if manifest.source_reference_is_missing()
                || manifest.expert_panel_count.unwrap_or(0) == 0
                || manifest
                    .elicitation_protocol_reference
                    .as_deref()
                    .is_none_or(str::is_empty)
            {
                add_error(
                    &mut errors,
                    "expert_probability_elicitation_provenance_invalid",
                );
            }
        }
        ProbabilityBasis::StressOnly => {
            add_error(
                &mut errors,
                "stress_only_scenarios_cannot_enter_probabilistic_risk_metrics",
            );
        }
        ProbabilityBasis::Unknown => {
            add_error(
                &mut errors,
                "unknown_probability_basis_cannot_enter_probabilistic_risk_metrics",
            );
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        Err(errors)
    } else {
        Ok(())
    }
}

impl ServiceProbabilityManifest {
    fn source_reference_is_missing(&self) -> bool {
        self.probability_source_reference.trim().is_empty()
    }
}

fn exact_weighted_expectation(
    leaves: &[RiskLeaf],
    denominator: u128,
    metric: fn(&RiskLeaf) -> u64,
) -> Result<ExactRiskRatio, String> {
    let numerator = leaves.iter().try_fold(0_u128, |sum, leaf| {
        u128::from(metric(leaf))
            .checked_mul(leaf.weight)
            .and_then(|value| sum.checked_add(value))
    });
    Ok(ExactRiskRatio {
        numerator: numerator.ok_or_else(|| "risk_expectation_overflow".to_owned())?,
        denominator,
    })
}

fn weighted_tail_average(
    leaves: &[RiskLeaf],
    denominator: u128,
    tail_mass: u128,
    metric: fn(&RiskLeaf) -> u64,
    lower_tail: bool,
) -> Result<ExactRiskRatio, String> {
    if tail_mass == 0 || tail_mass > denominator {
        return Err("risk_tail_mass_out_of_range".to_owned());
    }
    let mut ordered = leaves.to_vec();
    ordered.sort_by(|a, b| {
        let order = metric(a).cmp(&metric(b));
        let order = if lower_tail { order } else { order.reverse() };
        order.then_with(|| a.id.cmp(&b.id))
    });
    let mut remaining = tail_mass;
    let mut numerator = 0_u128;
    for leaf in &ordered {
        if remaining == 0 {
            break;
        }
        let consumed = remaining.min(leaf.weight);
        let contribution = u128::from(metric(leaf))
            .checked_mul(consumed)
            .ok_or_else(|| "risk_tail_numerator_overflow".to_owned())?;
        numerator = numerator
            .checked_add(contribution)
            .ok_or_else(|| "risk_tail_numerator_overflow".to_owned())?;
        remaining -= consumed;
    }
    if remaining != 0 {
        return Err("risk_tail_probability_mass_incomplete".to_owned());
    }
    Ok(ExactRiskRatio {
        numerator,
        denominator: tail_mass,
    })
}

fn worst_case_tv_expectation(
    leaves: &[RiskLeaf],
    denominator: u128,
    radius_mass: u128,
    metric: fn(&RiskLeaf) -> u64,
    minimize: bool,
) -> Result<ExactRiskRatio, String> {
    if radius_mass > denominator {
        return Err("risk_tv_radius_out_of_range".to_owned());
    }
    let mut mass: Vec<u128> = leaves.iter().map(|leaf| leaf.weight).collect();
    let mut receiver_order: Vec<usize> = (0..leaves.len()).collect();
    let mut donor_order: Vec<usize> = (0..leaves.len()).collect();

    receiver_order.sort_by(|a, b| {
        let order = metric(&leaves[*a]).cmp(&metric(&leaves[*b]));
        let order = if minimize { order } else { order.reverse() };
        order.then_with(|| leaves[*a].id.cmp(&leaves[*b].id))
    });
    donor_order.sort_by(|a, b| {
        let order = metric(&leaves[*a]).cmp(&metric(&leaves[*b]));
        let order = if minimize { order.reverse() } else { order };
        order.then_with(|| leaves[*a].id.cmp(&leaves[*b].id))
    });

    let mut remaining = radius_mass;
    let mut receiver_cursor = 0_usize;
    let mut donor_cursor = 0_usize;
    while remaining > 0 {
        while receiver_cursor < receiver_order.len()
            && mass[receiver_order[receiver_cursor]] == denominator
        {
            receiver_cursor += 1;
        }
        while donor_cursor < donor_order.len() && mass[donor_order[donor_cursor]] == 0 {
            donor_cursor += 1;
        }
        if receiver_cursor >= receiver_order.len() || donor_cursor >= donor_order.len() {
            break;
        }
        let receiver = receiver_order[receiver_cursor];
        let donor = donor_order[donor_cursor];
        if receiver == donor || metric(&leaves[receiver]) == metric(&leaves[donor]) {
            break;
        }
        let room = denominator - mass[receiver];
        let transferable = remaining.min(room).min(mass[donor]);
        if transferable == 0 {
            break;
        }
        mass[receiver] = mass[receiver]
            .checked_add(transferable)
            .ok_or_else(|| "risk_tv_mass_overflow".to_owned())?;
        mass[donor] -= transferable;
        remaining -= transferable;
    }

    let numerator = leaves
        .iter()
        .enumerate()
        .try_fold(0_u128, |sum, (index, leaf)| {
            u128::from(metric(leaf))
                .checked_mul(mass[index])
                .and_then(|value| sum.checked_add(value))
        });
    Ok(ExactRiskRatio {
        numerator: numerator.ok_or_else(|| "risk_tv_numerator_overflow".to_owned())?,
        denominator,
    })
}

fn on_time(leaf: &RiskLeaf) -> u64 {
    leaf.on_time
}

fn completed(leaf: &RiskLeaf) -> u64 {
    leaf.completed
}

fn lateness(leaf: &RiskLeaf) -> u64 {
    leaf.lateness
}

fn unserved_plus_rejected(leaf: &RiskLeaf) -> u64 {
    leaf.unserved_plus_rejected
}

pub fn evaluate_mobility_service_risk_envelope(
    tree: &NonanticipativeServiceOracleInput,
    service: &LinkedServiceQualityInput,
    capacity: &SustainableServiceCapacityInput,
    input: &MobilityServiceRiskInput,
) -> Result<MobilityServiceRiskReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "MOBILITY-SERVICE-RISK-ENVELOPE-001"
        || input.claim_ceiling.trim().is_empty()
        || input.tail_mass_ppm == 0
        || input.tail_mass_ppm > 1_000_000
        || input.total_variation_radius_ppm > 1_000_000
    {
        add_error(
            &mut errors,
            "mobility_service_risk_identity_or_bounds_invalid",
        );
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let oracle =
        evaluate_nonanticipative_service_oracle(tree, service, capacity).map_err(|items| {
            items
                .into_iter()
                .map(|item| format!("upstream_oracle_rejected:{item}"))
                .collect::<Vec<_>>()
        })?;
    if oracle.status != NonanticipativeOracleStatus::Evaluated
        || oracle.leaf_results.is_empty()
        || oracle.leaf_results.len() > MAX_LEAVES
    {
        return Err(vec!["upstream_oracle_not_fully_evaluated".to_owned()]);
    }

    let source_tree_digest = canonical_tree_digest(tree).map_err(|error| vec![error])?;
    let source_service_digest = source_digest(service).map_err(|error| vec![error])?;
    let source_capacity_digest = source_digest(capacity).map_err(|error| vec![error])?;

    let leaf_ids: BTreeSet<&str> = oracle
        .leaf_results
        .iter()
        .map(|leaf| leaf.leaf_node_id.as_str())
        .collect();
    if leaf_ids.len() != oracle.leaf_results.len() {
        return Err(vec!["upstream_oracle_leaf_id_duplicate".to_owned()]);
    }

    validate_probability_manifest(
        &input.probability_manifest,
        tree,
        &leaf_ids,
        &source_tree_digest,
    )?;

    let probability_denominator = oracle.leaf_results[0].probability_denominator;
    if probability_denominator == 0
        || probability_denominator > MAX_PROBABILITY_DENOMINATOR
        || oracle.leaf_results.iter().any(|leaf| {
            leaf.probability_denominator != probability_denominator
                || leaf.probability_numerator == 0
        })
    {
        return Err(vec![
            "upstream_probability_denominator_or_leaf_weight_invalid".to_owned(),
        ]);
    }
    let probability_sum = oracle
        .leaf_results
        .iter()
        .try_fold(0_u128, |sum, leaf| {
            sum.checked_add(leaf.probability_numerator)
        })
        .ok_or_else(|| vec!["upstream_probability_sum_overflow".to_owned()])?;
    if probability_sum != probability_denominator {
        return Err(vec![
            "upstream_leaf_probability_mass_does_not_sum_to_one".to_owned(),
        ]);
    }

    let tail_mass = probability_denominator
        .checked_mul(u128::from(input.tail_mass_ppm))
        .ok_or_else(|| vec!["risk_tail_mass_overflow".to_owned()])?
        / PROBABILITY_SCALE_PPM;
    let radius_mass = probability_denominator
        .checked_mul(u128::from(input.total_variation_radius_ppm))
        .ok_or_else(|| vec!["risk_tv_mass_overflow".to_owned()])?
        / PROBABILITY_SCALE_PPM;
    if tail_mass == 0 {
        return Err(vec!["risk_tail_mass_rounds_to_zero".to_owned()]);
    }

    let mut leaves: Vec<RiskLeaf> = oracle
        .leaf_results
        .iter()
        .map(|leaf| RiskLeaf {
            id: leaf.leaf_node_id.clone(),
            weight: leaf.probability_numerator,
            on_time: u64::try_from(leaf.on_time_services).unwrap_or(u64::MAX),
            completed: u64::try_from(leaf.completed_by_horizon).unwrap_or(u64::MAX),
            lateness: leaf.lateness_seconds,
            unserved_plus_rejected: u64::try_from(
                leaf.unserved_requests
                    .saturating_add(leaf.rejected_requests),
            )
            .unwrap_or(u64::MAX),
        })
        .collect();
    leaves.sort_by(|a, b| a.id.cmp(&b.id));

    let nominal_expected_on_time_services =
        exact_weighted_expectation(&leaves, probability_denominator, on_time)
            .map_err(|error| vec![error])?;
    let nominal_expected_completed_services =
        exact_weighted_expectation(&leaves, probability_denominator, completed)
            .map_err(|error| vec![error])?;
    let nominal_expected_lateness_seconds =
        exact_weighted_expectation(&leaves, probability_denominator, lateness)
            .map_err(|error| vec![error])?;
    let lower_tail_cvar_on_time_services =
        weighted_tail_average(&leaves, probability_denominator, tail_mass, on_time, true)
            .map_err(|error| vec![error])?;
    let lower_tail_cvar_completed_services =
        weighted_tail_average(&leaves, probability_denominator, tail_mass, completed, true)
            .map_err(|error| vec![error])?;
    let upper_tail_cvar_lateness_seconds =
        weighted_tail_average(&leaves, probability_denominator, tail_mass, lateness, false)
            .map_err(|error| vec![error])?;
    let upper_tail_cvar_unserved_plus_rejected = weighted_tail_average(
        &leaves,
        probability_denominator,
        tail_mass,
        unserved_plus_rejected,
        false,
    )
    .map_err(|error| vec![error])?;
    let tv_robust_min_expected_on_time_services =
        worst_case_tv_expectation(&leaves, probability_denominator, radius_mass, on_time, true)
            .map_err(|error| vec![error])?;
    let tv_robust_min_expected_completed_services = worst_case_tv_expectation(
        &leaves,
        probability_denominator,
        radius_mass,
        completed,
        true,
    )
    .map_err(|error| vec![error])?;
    let tv_robust_max_expected_lateness_seconds = worst_case_tv_expectation(
        &leaves,
        probability_denominator,
        radius_mass,
        lateness,
        false,
    )
    .map_err(|error| vec![error])?;

    let mut leaf_results: Vec<ServiceLeafRiskResult> = oracle
        .leaf_results
        .iter()
        .map(|leaf| ServiceLeafRiskResult {
            leaf_node_id: leaf.leaf_node_id.clone(),
            probability_numerator: leaf.probability_numerator,
            probability_denominator: leaf.probability_denominator,
            on_time_services: leaf.on_time_services,
            completed_by_horizon: leaf.completed_by_horizon,
            late_services: leaf.late_services,
            in_progress_services: leaf.in_progress_services,
            rejected_requests: leaf.rejected_requests,
            unserved_requests: leaf.unserved_requests,
            lateness_seconds: leaf.lateness_seconds,
            completed_latency_seconds: leaf.completed_latency_seconds,
        })
        .collect();
    leaf_results.sort_by(|a, b| a.leaf_node_id.cmp(&b.leaf_node_id));

    let all_synthetic = input.probability_manifest.probability_basis == ProbabilityBasis::Synthetic;
    let mut claim_ceiling = CLAIM_CEILING;
    if all_synthetic {
        claim_ceiling = "Synthetic scenario probabilities: exact conditional arithmetic only; no empirically calibrated reliability or field risk claim.";
    }

    Ok(MobilityServiceRiskReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        probability_set_id: input.probability_manifest.probability_set_id.clone(),
        probability_basis: input.probability_manifest.probability_basis,
        probability_source_reference: input
            .probability_manifest
            .probability_source_reference
            .clone(),
        probability_provenance_complete: true,
        calibration_quality_recomputed: false,
        synthetic_only: all_synthetic,
        source_tree_digest,
        source_service_digest,
        source_capacity_digest,
        upstream_information_model: oracle.information_model,
        upstream_policy_is_deployable: oracle.deployable_online_policy,
        leaf_count: leaves.len(),
        probability_denominator,
        tail_mass_ppm: input.tail_mass_ppm,
        total_variation_radius_ppm: input.total_variation_radius_ppm,
        nominal_expected_on_time_services,
        nominal_expected_completed_services,
        nominal_expected_lateness_seconds,
        lower_tail_cvar_on_time_services,
        lower_tail_cvar_completed_services,
        upper_tail_cvar_lateness_seconds,
        upper_tail_cvar_unserved_plus_rejected,
        tv_robust_min_expected_on_time_services,
        tv_robust_min_expected_completed_services,
        tv_robust_max_expected_lateness_seconds,
        worst_case_on_time_services: oracle.worst_case_on_time_services,
        worst_case_completed_services: oracle.worst_case_completed_by_horizon,
        worst_case_lateness_seconds: oracle.worst_case_lateness_seconds,
        leaf_results,
        stress_only_scenarios_included: false,
        probability_calibration_claim_granted: false,
        ledger_or_physical_mutation_performed: false,
        flight_authority_granted: false,
        market_or_dispatch_authority_granted: false,
        claim_ceiling,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_fixture() -> NonanticipativeServiceOracleInput {
        serde_json::from_str(include_str!(
            "../fixtures/nonanticipative_service_oracle_v0_1.json"
        ))
        .expect("tree fixture")
    }

    fn service_fixture() -> LinkedServiceQualityInput {
        serde_json::from_str(include_str!(
            "../fixtures/sustainable_mobility_service_quality_linked_v0_1.json"
        ))
        .expect("linked service fixture")
    }

    fn capacity_fixture() -> SustainableServiceCapacityInput {
        serde_json::from_str(include_str!(
            "../fixtures/sustainable_mobility_service_capacity_v0_1.json"
        ))
        .expect("capacity fixture")
    }

    fn risk_fixture() -> MobilityServiceRiskInput {
        serde_json::from_str(include_str!(
            "../fixtures/mobility_service_risk_envelope_v0_1.json"
        ))
        .expect("risk manifest fixture")
    }

    fn leaf(
        id: &str,
        weight: u128,
        on_time: u64,
        completed: u64,
        lateness: u64,
        unserved: u64,
    ) -> RiskLeaf {
        RiskLeaf {
            id: id.to_owned(),
            weight,
            on_time,
            completed,
            lateness,
            unserved_plus_rejected: unserved,
        }
    }

    #[test]
    fn exact_risk_run_recomputes_leaf_outcomes_and_keeps_calibration_claim_false() {
        let report = evaluate_mobility_service_risk_envelope(
            &tree_fixture(),
            &service_fixture(),
            &capacity_fixture(),
            &risk_fixture(),
        )
        .expect("risk fixture");
        assert_eq!(report.leaf_count, 2);
        assert_eq!(report.probability_denominator, MAX_PROBABILITY_DENOMINATOR);
        assert!(report.synthetic_only);
        assert!(!report.calibration_quality_recomputed);
        assert!(!report.probability_calibration_claim_granted);
        assert!(!report.stress_only_scenarios_included);
        assert!(!report.ledger_or_physical_mutation_performed);
        assert!(!report.flight_authority_granted);
        assert!(!report.market_or_dispatch_authority_granted);
        assert_eq!(report.source_tree_digest.len(), 71);
        assert_eq!(report.source_service_digest.len(), 71);
        assert_eq!(report.source_capacity_digest.len(), 71);
    }

    #[test]
    fn fractional_lower_tail_consumes_exact_boundary_probability_mass() {
        let leaves = vec![
            leaf("low", 20, 0, 1, 50, 4),
            leaf("mid", 30, 3, 3, 10, 1),
            leaf("high", 50, 5, 5, 0, 0),
        ];
        let result = weighted_tail_average(&leaves, 100, 40, on_time, true).expect("CVaR");
        assert_eq!(
            result,
            ExactRiskRatio {
                numerator: 60,
                denominator: 40
            }
        );
    }

    #[test]
    fn fractional_upper_tail_consumes_exact_boundary_probability_mass() {
        let leaves = vec![
            leaf("low", 50, 5, 5, 0, 0),
            leaf("mid", 30, 3, 3, 10, 1),
            leaf("high", 20, 0, 1, 100, 4),
        ];
        let result = weighted_tail_average(&leaves, 100, 40, lateness, false).expect("CVaR");
        assert_eq!(
            result,
            ExactRiskRatio {
                numerator: 2_200,
                denominator: 40
            }
        );
    }

    #[test]
    fn rare_high_lateness_changes_tail_risk_more_than_nominal_expectation() {
        let leaves = vec![
            leaf("normal", 990, 5, 5, 0, 0),
            leaf("rare", 10, 0, 1, 10_000, 4),
        ];
        let expected = exact_weighted_expectation(&leaves, 1_000, lateness).expect("expectation");
        let tail = weighted_tail_average(&leaves, 1_000, 50, lateness, false).expect("tail");
        assert_eq!(
            expected,
            ExactRiskRatio {
                numerator: 100_000,
                denominator: 1_000
            }
        );
        assert_eq!(
            tail,
            ExactRiskRatio {
                numerator: 100_000,
                denominator: 50
            }
        );
    }

    #[test]
    fn tv_radius_zero_is_nominal_and_full_radius_reaches_the_worst_leaf() {
        let leaves = vec![
            leaf("best", 40, 4, 4, 0, 0),
            leaf("middle", 30, 2, 3, 10, 1),
            leaf("worst", 30, 0, 1, 100, 3),
        ];
        let nominal = worst_case_tv_expectation(&leaves, 100, 0, on_time, true).expect("nominal");
        let full = worst_case_tv_expectation(&leaves, 100, 100, on_time, true).expect("full");
        let high_late =
            worst_case_tv_expectation(&leaves, 100, 100, lateness, false).expect("late");
        assert_eq!(
            nominal,
            ExactRiskRatio {
                numerator: 220,
                denominator: 100
            }
        );
        assert_eq!(
            full,
            ExactRiskRatio {
                numerator: 0,
                denominator: 100
            }
        );
        assert_eq!(
            high_late,
            ExactRiskRatio {
                numerator: 10_000,
                denominator: 100
            }
        );
    }

    fn validate_probability_mass(weights: &[u128], denominator: u128) -> Result<(), String> {
        let total = weights
            .iter()
            .try_fold(0_u128, |sum, weight| sum.checked_add(*weight))
            .ok_or_else(|| "probability_mass_overflow".to_owned())?;
        if denominator == 0 || total != denominator || weights.iter().any(|weight| *weight == 0) {
            return Err("probability_mass_does_not_sum_to_one".to_owned());
        }
        Ok(())
    }

    #[test]
    fn probability_mass_that_does_not_sum_to_one_is_rejected() {
        assert!(validate_probability_mass(&[50, 49], 100).is_err());
        assert!(validate_probability_mass(&[50, 50], 100).is_ok());
        assert!(validate_probability_mass(&[0, 100], 100).is_err());
    }

    #[test]
    fn probability_manifest_must_bind_exact_canonical_tree_digest() {
        let mut risk = risk_fixture();
        risk.probability_manifest.scenario_tree_digest =
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
        let errors = evaluate_mobility_service_risk_envelope(
            &tree_fixture(),
            &service_fixture(),
            &capacity_fixture(),
            &risk,
        )
        .expect_err("a probability set must be bound to the exact tree contents");
        assert!(errors
            .iter()
            .any(|error| error == "probability_manifest_scenario_tree_digest_mismatch"));
    }

    #[test]
    fn stress_only_and_unknown_probability_bases_are_rejected() {
        let mut risk = risk_fixture();
        risk.probability_manifest.probability_basis = ProbabilityBasis::StressOnly;
        let errors = evaluate_mobility_service_risk_envelope(
            &tree_fixture(),
            &service_fixture(),
            &capacity_fixture(),
            &risk,
        )
        .expect_err("stress-only weights may not enter CVaR");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("stress_only_scenarios"))
        );
        risk.probability_manifest.probability_basis = ProbabilityBasis::Unknown;
        let errors = evaluate_mobility_service_risk_envelope(
            &tree_fixture(),
            &service_fixture(),
            &capacity_fixture(),
            &risk,
        )
        .expect_err("unknown probability source rejected");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("unknown_probability_basis"))
        );
    }

    #[test]
    fn historical_frequency_requires_holdout_after_training_cutoff() {
        let mut risk = risk_fixture();
        risk.probability_manifest.probability_basis = ProbabilityBasis::HistoricalFrequencyEstimate;
        risk.probability_manifest.source_dataset_digest = Some(
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
        );
        risk.probability_manifest.calibration_report_digest = Some(
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
        );
        risk.probability_manifest.sample_count = Some(100);
        risk.probability_manifest.effective_sample_size_milli = Some(80_000);
        risk.probability_manifest.training_cutoff_seconds = Some(500);
        risk.probability_manifest.evaluation_window_start_seconds = Some(400);
        risk.probability_manifest.evaluation_window_end_seconds = Some(900);
        let errors = evaluate_mobility_service_risk_envelope(
            &tree_fixture(),
            &service_fixture(),
            &capacity_fixture(),
            &risk,
        )
        .expect_err("holdout overlaps training data");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("historical_probability_provenance"))
        );
    }

    #[test]
    fn forecast_manifest_requires_complete_held_out_evaluation_window() {
        let mut risk = risk_fixture();
        let manifest = &mut risk.probability_manifest;
        manifest.probability_basis = ProbabilityBasis::ForecastModel;
        manifest.source_dataset_digest =
            Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned());
        manifest.calibration_report_digest =
            Some("sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned());
        manifest.training_cutoff_seconds = Some(100);
        manifest.forecast_issue_time_seconds = Some(200);
        manifest.evaluation_window_start_seconds = Some(200);
        manifest.evaluation_window_end_seconds = None;
        manifest.sample_count = Some(50);
        manifest.effective_sample_size_milli = Some(40_000);
        let errors = evaluate_mobility_service_risk_envelope(
            &tree_fixture(),
            &service_fixture(),
            &capacity_fixture(),
            &risk,
        )
        .expect_err("forecast manifest without held-out window end is incomplete");
        assert!(errors.iter().any(|error| error.contains("forecast_probability_provenance")));
    }

    #[test]
    fn expert_elicitation_requires_panel_and_protocol() {
        let mut risk = risk_fixture();
        risk.probability_manifest.probability_basis = ProbabilityBasis::ExpertElicitation;
        risk.probability_manifest.expert_panel_count = Some(0);
        risk.probability_manifest.elicitation_protocol_reference = None;
        let errors = evaluate_mobility_service_risk_envelope(
            &tree_fixture(),
            &service_fixture(),
            &capacity_fixture(),
            &risk,
        )
        .expect_err("expert probabilities require panel and protocol provenance");
        assert!(errors.iter().any(|error| error.contains("expert_probability_elicitation_provenance")));
    }

    #[test]
    fn duplicate_or_missing_leaf_manifest_entries_are_rejected() {
        let mut risk = risk_fixture();
        risk.probability_manifest.expected_leaf_node_ids[1] =
            risk.probability_manifest.expected_leaf_node_ids[0].clone();
        let errors = evaluate_mobility_service_risk_envelope(
            &tree_fixture(),
            &service_fixture(),
            &capacity_fixture(),
            &risk,
        )
        .expect_err("leaf evidence identity must be exact and unique");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("leaf_identity_duplicate"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("leaf_coverage_mismatch"))
        );
    }

    #[test]
    fn scenario_order_permutation_preserves_exact_risk_report() {
        let tree = tree_fixture();
        let service = service_fixture();
        let capacity = capacity_fixture();
        let risk = risk_fixture();
        let a = evaluate_mobility_service_risk_envelope(&tree, &service, &capacity, &risk)
            .expect("first");
        let mut permuted_tree = tree_fixture();
        permuted_tree.nodes.reverse();
        let b = evaluate_mobility_service_risk_envelope(&permuted_tree, &service, &capacity, &risk)
            .expect("permuted tree");
        assert_eq!(a.leaf_results, b.leaf_results);
        assert_eq!(a.source_tree_digest, b.source_tree_digest);
        assert_eq!(
            a.lower_tail_cvar_on_time_services,
            b.lower_tail_cvar_on_time_services
        );
        assert_eq!(
            a.tv_robust_min_expected_on_time_services,
            b.tv_robust_min_expected_on_time_services
        );
    }

    #[test]
    fn invalid_tail_fraction_and_tv_radius_are_rejected() {
        let mut risk = risk_fixture();
        risk.tail_mass_ppm = 0;
        assert!(
            evaluate_mobility_service_risk_envelope(
                &tree_fixture(),
                &service_fixture(),
                &capacity_fixture(),
                &risk
            )
            .is_err()
        );
        risk = risk_fixture();
        risk.total_variation_radius_ppm = 1_000_001;
        assert!(
            evaluate_mobility_service_risk_envelope(
                &tree_fixture(),
                &service_fixture(),
                &capacity_fixture(),
                &risk
            )
            .is_err()
        );
    }
}
