// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Cluster-level intention-to-treat analysis and randomization inference.
//!
//! The estimator consumes a preregistered cluster-randomized plan and one
//! complete, source-digested primary outcome per randomized cluster. Assignment
//! is the treatment variable. Treatment uptake and control contamination are
//! reported descriptively and never replace assignment in the ITT estimator.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use symtropy_grid_causal_design::{
    AssignmentArm, GridCausalDesignAuditReport, GridCausalDesignInput, audit_causal_design,
};

const SCHEMA_VERSION: &str = "0.1.0";
const BENCHMARK_ID: &str = "GRID-CAUSAL-ITT-ESTIMATOR-001";
const MAX_EXACT_ASSIGNMENTS: u128 = 100_000;
const MIN_MONTE_CARLO_DRAWS: usize = 1_000;
const MAX_MONTE_CARLO_DRAWS: usize = 100_000;
const CLAIM_CEILING: &str = "Cluster-level intention-to-treat estimate for the preregistered randomized clusters and primary outcome, conditional on assignment integrity, complete outcomes, valid interference mapping and outcome measurement. Randomization inference tests the sharp null under the declared stratified assignment. This is not treatment-on-treated, a transported population effect, market settlement, payment authorization, ELCC, dispatch authority or field/product accreditation.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClusterOutcomeObservation {
    pub cluster_id: String,
    pub reported_assignment: AssignmentArm,
    pub primary_outcome_id: String,
    pub primary_outcome_unit: String,
    pub measurement_boundary_id: String,
    pub outcome_window_start_unix_ms: u64,
    pub outcome_window_end_unix_ms: u64,
    pub outcome_observed_at_unix_ms: u64,
    /// Signed outcome. For the pilot fixture, positive values represent
    /// net PCC reduction in kWh; negative values are retained as negative.
    pub outcome_value: f64,
    /// Descriptive treatment uptake / control contamination, in [0, 1].
    /// Never used to redefine the ITT assignment variable.
    pub received_treatment_fraction: f64,
    pub evidence_reference: String,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClusterIttAnalysisInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub design: GridCausalDesignInput,
    pub outcomes: Vec<ClusterOutcomeObservation>,
    /// Monte Carlo draws used only when the exact stratified assignment
    /// space exceeds MAX_EXACT_ASSIGNMENTS.
    pub permutation_budget: usize,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RandomizationInferenceMethod {
    ExactStratifiedPermutation,
    DeterministicSha256MonteCarlo,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StratumIttEffect {
    pub stratum_id: String,
    pub treated_cluster_count: usize,
    pub control_cluster_count: usize,
    pub stratum_weight: f64,
    pub treated_mean_outcome: f64,
    pub control_mean_outcome: f64,
    pub itt_effect_estimate: f64,
    pub variance_contribution: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClusterIttAnalysisReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub input_digest: String,
    pub design_input_digest: String,
    pub randomization_assignment_digest: String,
    pub experiment_id: String,
    pub primary_outcome_id: String,
    pub primary_outcome_unit: String,
    pub measurement_boundary_id: String,
    pub estimand: String,
    pub analysis_weighting: String,
    pub cluster_count: usize,
    pub stratum_count: usize,
    pub treated_cluster_count: usize,
    pub control_cluster_count: usize,
    pub complete_cluster_outcomes: bool,
    pub cluster_level_analysis: bool,
    pub assignment_based_itt: bool,
    pub treated_mean_outcome: f64,
    pub control_mean_outcome: f64,
    pub itt_effect_estimate: f64,
    pub neyman_conservative_variance: f64,
    pub standard_error: f64,
    pub satterthwaite_degrees_of_freedom: Option<f64>,
    pub student_t_critical_value_975: f64,
    pub confidence_interval_95_lower: f64,
    pub confidence_interval_95_upper: f64,
    pub confidence_interval_method: String,
    pub randomization_inference_method: RandomizationInferenceMethod,
    pub randomization_inference_exact: bool,
    pub randomization_assignments_evaluated: usize,
    pub randomization_extreme_assignments: usize,
    pub two_sided_sharp_null_p_value: f64,
    pub treatment_uptake_fraction: f64,
    pub control_contamination_fraction: f64,
    pub missing_cluster_outcome_count: usize,
    pub outcome_evidence_authenticated: bool,
    pub design_pre_registration_authenticated: bool,
    pub interference_mapping_authenticated: bool,
    pub external_randomness_source_authenticated: bool,
    pub treatment_on_treated_identified: bool,
    pub effect_transportability_established: bool,
    pub settlement_authority_granted: bool,
    pub payment_authorized: bool,
    pub dispatch_authority_granted: bool,
    pub strata: Vec<StratumIttEffect>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IttAnalysisError {
    InvalidInput(String),
    MissingOutcome(String),
    DuplicateOutcome(String),
    InvalidDigest(String),
    IdentityMismatch(String),
    TimeBoundaryMismatch(String),
    NonFiniteOutcome(String),
    InvalidTreatmentDose(String),
    InsufficientClusters(String),
    RandomizationSpaceTooLarge(String),
    Arithmetic(String),
}

impl core::fmt::Display for IttAnalysisError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput(message)
            | Self::MissingOutcome(message)
            | Self::DuplicateOutcome(message)
            | Self::InvalidDigest(message)
            | Self::IdentityMismatch(message)
            | Self::TimeBoundaryMismatch(message)
            | Self::NonFiniteOutcome(message)
            | Self::InvalidTreatmentDose(message)
            | Self::InsufficientClusters(message)
            | Self::RandomizationSpaceTooLarge(message)
            | Self::Arithmetic(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for IttAnalysisError {}

fn require_nonempty(field: &str, value: &str) -> Result<(), IttAnalysisError> {
    if value.trim().is_empty() {
        Err(IttAnalysisError::InvalidInput(format!(
            "{field} must not be empty"
        )))
    } else {
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn digest_serializable<T: Serialize>(value: &T, label: &str) -> Result<String, IttAnalysisError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        IttAnalysisError::Arithmetic(format!("{label} canonical serialization failed: {error}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn stratified_effect(
    design: &GridCausalDesignInput,
    outcomes: &BTreeMap<String, f64>,
    treated_cluster_ids: &BTreeSet<String>,
) -> Result<(f64, Vec<StratumIttEffect>), IttAnalysisError> {
    let total_clusters = design.clusters.len() as f64;
    let mut pooled_effect = 0.0_f64;
    let mut stratum_effects = Vec::with_capacity(design.strata.len());

    for stratum in &design.strata {
        let mut treated_values = Vec::new();
        let mut control_values = Vec::new();

        for cluster in design
            .clusters
            .iter()
            .filter(|cluster| cluster.stratum_id == stratum.stratum_id)
        {
            let outcome = outcomes.get(&cluster.cluster_id).ok_or_else(|| {
                IttAnalysisError::MissingOutcome(format!(
                    "missing primary outcome for cluster {}",
                    cluster.cluster_id
                ))
            })?;
            if treated_cluster_ids.contains(&cluster.cluster_id) {
                treated_values.push(*outcome);
            } else {
                control_values.push(*outcome);
            }
        }

        if treated_values.len() < 2 || control_values.len() < 2 {
            return Err(IttAnalysisError::InsufficientClusters(format!(
                "stratum {} requires at least two observed clusters per arm for the Neyman variance estimate",
                stratum.stratum_id
            )));
        }

        let treated_mean = mean(&treated_values)?;
        let control_mean = mean(&control_values)?;
        let weight = (treated_values.len() + control_values.len()) as f64 / total_clusters;
        let effect = treated_mean - control_mean;
        pooled_effect += weight * effect;

        stratum_effects.push(StratumIttEffect {
            stratum_id: stratum.stratum_id.clone(),
            treated_cluster_count: treated_values.len(),
            control_cluster_count: control_values.len(),
            stratum_weight: weight,
            treated_mean_outcome: treated_mean,
            control_mean_outcome: control_mean,
            itt_effect_estimate: effect,
            variance_contribution: 0.0,
        });
    }

    Ok((pooled_effect, stratum_effects))
}

fn mean(values: &[f64]) -> Result<f64, IttAnalysisError> {
    if values.is_empty() {
        return Err(IttAnalysisError::InsufficientClusters(
            "cannot compute a mean from an empty cluster arm".to_owned(),
        ));
    }
    let sum = values.iter().try_fold(0.0_f64, |total, value| {
        let next = total + value;
        if next.is_finite() {
            Ok(next)
        } else {
            Err(IttAnalysisError::Arithmetic(
                "cluster outcome sum overflow".to_owned(),
            ))
        }
    })?;
    Ok(sum / values.len() as f64)
}

fn sample_variance(values: &[f64], average: f64) -> Result<f64, IttAnalysisError> {
    if values.len() < 2 {
        return Err(IttAnalysisError::InsufficientClusters(
            "sample variance requires at least two clusters per arm".to_owned(),
        ));
    }
    let sum_squares = values.iter().try_fold(0.0_f64, |sum, value| {
        let difference = value - average;
        let next = sum + difference * difference;
        if next.is_finite() {
            Ok(next)
        } else {
            Err(IttAnalysisError::Arithmetic(
                "cluster outcome variance accumulation overflow".to_owned(),
            ))
        }
    })?;
    Ok(sum_squares / (values.len() - 1) as f64)
}

fn stratified_neyman_variance(
    design: &GridCausalDesignInput,
    outcomes: &BTreeMap<String, f64>,
) -> Result<(f64, Option<f64>, Vec<StratumIttEffect>), IttAnalysisError> {
    let mut total_variance = 0.0_f64;
    let mut df_terms: Vec<(f64, f64)> = Vec::new();
    let mut summaries = Vec::with_capacity(design.strata.len());
    let total_clusters = design.clusters.len() as f64;

    for stratum in &design.strata {
        let mut treated = Vec::new();
        let mut control = Vec::new();
        for cluster in design
            .clusters
            .iter()
            .filter(|cluster| cluster.stratum_id == stratum.stratum_id)
        {
            let value = *outcomes.get(&cluster.cluster_id).ok_or_else(|| {
                IttAnalysisError::MissingOutcome(format!(
                    "missing cluster outcome for {}",
                    cluster.cluster_id
                ))
            })?;
            match cluster.assigned_arm {
                AssignmentArm::Treatment => treated.push(value),
                AssignmentArm::Control => control.push(value),
            }
        }
        let n_treated = treated.len();
        let n_control = control.len();
        if n_treated < 2 || n_control < 2 {
            return Err(IttAnalysisError::InsufficientClusters(format!(
                "stratum {} needs at least two clusters in each arm for cluster-level variance estimation",
                stratum.stratum_id
            )));
        }

        let treated_mean = mean(&treated)?;
        let control_mean = mean(&control)?;
        let treated_variance = sample_variance(&treated, treated_mean)?;
        let control_variance = sample_variance(&control, control_mean)?;
        let weight = (n_treated + n_control) as f64 / total_clusters;
        let effect = treated_mean - control_mean;

        let treated_component = weight.powi(2) * treated_variance / n_treated as f64;
        let control_component = weight.powi(2) * control_variance / n_control as f64;
        let stratum_variance = treated_component + control_component;
        total_variance += stratum_variance;
        df_terms.push((treated_component, (n_treated - 1) as f64));
        df_terms.push((control_component, (n_control - 1) as f64));

        summaries.push(StratumIttEffect {
            stratum_id: stratum.stratum_id.clone(),
            treated_cluster_count: n_treated,
            control_cluster_count: n_control,
            stratum_weight: weight,
            treated_mean_outcome: treated_mean,
            control_mean_outcome: control_mean,
            itt_effect_estimate: effect,
            variance_contribution: stratum_variance,
        });
    }

    let df_denominator = df_terms
        .iter()
        .filter(|(component, _)| *component > 0.0)
        .map(|(component, df)| component.powi(2) / df)
        .sum::<f64>();
    let degrees_of_freedom = if total_variance > 0.0 && df_denominator > 0.0 {
        Some(total_variance.powi(2) / df_denominator)
    } else {
        None
    };

    Ok((total_variance, degrees_of_freedom, summaries))
}

fn student_t_critical_975(df: Option<f64>) -> f64 {
    // Use floor(df), which is conservative because the two-sided 97.5th
    // percentile decreases monotonically with degrees of freedom.
    const TABLE: [f64; 31] = [
        0.0, 12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262,
        2.228, 2.201, 2.179, 2.160, 2.145, 2.131, 2.120, 2.110, 2.101, 2.093,
        2.086, 2.080, 2.074, 2.069, 2.064, 2.060, 2.056, 2.052, 2.048, 2.045,
        2.042,
    ];
    match df {
        Some(value) if value.is_finite() && value >= 1.0 => {
            let floor_df = value.floor() as usize;
            if floor_df >= 30 {
                2.042
            } else {
                TABLE[floor_df]
            }
        }
        _ => 1.96,
    }
}

fn choose_count(n: usize, k: usize) -> u128 {
    let k = k.min(n.saturating_sub(k));
    let mut result = 1_u128;
    for i in 0..k {
        result = result.saturating_mul((n - i) as u128) / (i + 1) as u128;
    }
    result
}

fn combinations(values: &[String], choose: usize) -> Vec<Vec<String>> {
    fn recurse(
        values: &[String],
        choose: usize,
        start: usize,
        current: &mut Vec<String>,
        result: &mut Vec<Vec<String>>,
    ) {
        if current.len() == choose {
            result.push(current.clone());
            return;
        }
        let still_needed = choose - current.len();
        if values.len().saturating_sub(start) < still_needed {
            return;
        }
        let last_start = values.len() - still_needed;
        for index in start..=last_start {
            current.push(values[index].clone());
            recurse(values, choose, index + 1, current, result);
            current.pop();
        }
    }
    let mut result = Vec::new();
    recurse(values, choose, 0, &mut Vec::new(), &mut result);
    result
}

fn visit_exact_assignments(
    strata: &[Vec<Vec<String>>],
    index: usize,
    treated: &mut BTreeSet<String>,
    design: &GridCausalDesignInput,
    outcomes: &BTreeMap<String, f64>,
    observed_abs_effect: f64,
    tolerance: f64,
    totals: &mut (usize, usize),
) -> Result<(), IttAnalysisError> {
    if index == strata.len() {
        let (effect, _) = stratified_effect(design, outcomes, treated)?;
        totals.1 += 1;
        if effect.abs() + tolerance >= observed_abs_effect {
            totals.0 += 1;
        }
        return Ok(());
    }

    for subset in &strata[index] {
        for cluster_id in subset {
            treated.insert(cluster_id.clone());
        }
        visit_exact_assignments(
            strata,
            index + 1,
            treated,
            design,
            outcomes,
            observed_abs_effect,
            tolerance,
            totals,
        )?;
        for cluster_id in subset {
            treated.remove(cluster_id);
        }
    }
    Ok(())
}

fn stratified_effect(
    design: &GridCausalDesignInput,
    outcomes: &BTreeMap<String, f64>,
    treated_ids: &BTreeSet<String>,
) -> Result<(f64, Vec<(String, f64)>), IttAnalysisError> {
    let mut weighted_effect = 0.0_f64;
    let mut details = Vec::with_capacity(design.strata.len());
    let total = design.clusters.len() as f64;
    for stratum in &design.strata {
        let mut treated = Vec::new();
        let mut control = Vec::new();
        for cluster in design
            .clusters
            .iter()
            .filter(|cluster| cluster.stratum_id == stratum.stratum_id)
        {
            let value = *outcomes.get(&cluster.cluster_id).ok_or_else(|| {
                IttAnalysisError::MissingOutcome(format!(
                    "missing outcome for cluster {}",
                    cluster.cluster_id
                ))
            })?;
            if treated_ids.contains(&cluster.cluster_id) {
                treated.push(value);
            } else {
                control.push(value);
            }
        }
        if treated.is_empty() || control.is_empty() {
            return Err(IttAnalysisError::InsufficientClusters(format!(
                "permutation leaves an empty arm in stratum {}",
                stratum.stratum_id
            )));
        }
        let weight = (treated.len() + control.len()) as f64 / total;
        let difference = mean(&treated)? - mean(&control)?;
        weighted_effect += weight * difference;
        details.push((stratum.stratum_id.clone(), difference));
    }
    Ok((weighted_effect, details))
}

fn deterministic_permutation_assignment(
    design: &GridCausalDesignInput,
    draw: usize,
) -> BTreeSet<String> {
    let mut treated = BTreeSet::new();
    for stratum in &design.strata {
        let mut ranks: Vec<_> = design
            .clusters
            .iter()
            .filter(|cluster| cluster.stratum_id == stratum.stratum_id)
            .map(|cluster| {
                let material = format!(
                    "{}|{}|{}|itt-permutation|{}|{}|{}",
                    design.experiment_id,
                    design.randomization.seed_reveal,
                    design.randomization.public_randomness_value,
                    draw,
                    stratum.stratum_id,
                    cluster.cluster_id
                );
                (
                    format!("{:x}", Sha256::digest(material.as_bytes())),
                    cluster.cluster_id.clone(),
                )
            })
            .collect();
        ranks.sort();
        for (_, cluster_id) in ranks.iter().take(stratum.treated_cluster_count) {
            treated.insert(cluster_id.clone());
        }
    }
    treated
}

fn permutation_p_value(
    design: &GridCausalDesignInput,
    outcomes: &BTreeMap<String, f64>,
    observed_effect: f64,
    permutation_budget: usize,
) -> Result<(RandomizationInferenceMethod, bool, usize, usize, f64), IttAnalysisError> {
    let mut assignment_space = 1_u128;
    for stratum in &design.strata {
        let clusters_in_stratum = design
            .clusters
            .iter()
            .filter(|cluster| cluster.stratum_id == stratum.stratum_id)
            .count();
        assignment_space = assignment_space.saturating_mul(choose_count(
            clusters_in_stratum,
            stratum.treated_cluster_count,
        ));
    }

    let observed_abs = observed_effect.abs();
    let tolerance = 1.0e-12;
    if assignment_space <= MAX_EXACT_ASSIGNMENTS {
        let mut per_stratum_choices = Vec::with_capacity(design.strata.len());
        for stratum in &design.strata {
            let mut cluster_ids: Vec<_> = design
                .clusters
                .iter()
                .filter(|cluster| cluster.stratum_id == stratum.stratum_id)
                .map(|cluster| cluster.cluster_id.clone())
                .collect();
            cluster_ids.sort();
            per_stratum_choices.push(combinations(
                &cluster_ids,
                stratum.treated_cluster_count,
            ));
        }

        let mut totals = (0_usize, 0_usize);
        visit_exact_assignments(
            &per_stratum_choices,
            0,
            &mut BTreeSet::new(),
            design,
            outcomes,
            observed_abs,
            tolerance,
            &mut totals,
        )?;
        if totals.1 == 0 {
            return Err(IttAnalysisError::RandomizationSpaceTooLarge(
                "exact randomization enumeration produced no assignments".to_owned(),
            ));
        }
        return Ok((
            RandomizationInferenceMethod::ExactStratifiedPermutation,
            true,
            totals.1,
            totals.0,
            totals.0 as f64 / totals.1 as f64,
        ));
    }

    let mut extreme_count = 0_usize;
    for draw in 0..permutation_budget {
        let assignment = deterministic_permutation_assignment(design, draw);
        let (effect, _) = stratified_effect(design, outcomes, &assignment)?;
        if effect.abs() + tolerance >= observed_abs {
            extreme_count += 1;
        }
    }
    let p_value = (extreme_count + 1) as f64 / (permutation_budget + 1) as f64;
    Ok((
        RandomizationInferenceMethod::DeterministicSha256MonteCarlo,
        false,
        permutation_budget,
        extreme_count,
        p_value,
    ))
}

fn validate_outcomes(
    input: &ClusterIttAnalysisInput,
    audit: &GridCausalDesignAuditReport,
) -> Result<BTreeMap<String, f64>, IttAnalysisError> {
    let design = &input.design;
    if input.outcomes.len() != design.clusters.len() {
        return Err(IttAnalysisError::MissingOutcome(format!(
            "expected one outcome for each of {} randomized clusters; received {}",
            design.clusters.len(),
            input.outcomes.len()
        )));
    }

    let expected_clusters: BTreeMap<_, _> = design
        .clusters
        .iter()
        .map(|cluster| (cluster.cluster_id.as_str(), cluster))
        .collect();
    let mut seen_clusters = BTreeSet::new();
    let mut seen_evidence_refs = BTreeSet::new();
    let mut seen_digests = BTreeSet::new();
    let mut outcomes = BTreeMap::new();

    for outcome in &input.outcomes {
        require_nonempty("outcome cluster ID", &outcome.cluster_id)?;
        require_nonempty("outcome evidence reference", &outcome.evidence_reference)?;
        if !seen_clusters.insert(outcome.cluster_id.as_str()) {
            return Err(IttAnalysisError::DuplicateOutcome(format!(
                "duplicate outcome for cluster {}",
                outcome.cluster_id
            )));
        }
        if !seen_evidence_refs.insert(outcome.evidence_reference.as_str()) {
            return Err(IttAnalysisError::DuplicateOutcome(format!(
                "reused outcome evidence reference {}",
                outcome.evidence_reference
            )));
        }
        if !valid_sha256(&outcome.evidence_digest) {
            return Err(IttAnalysisError::InvalidDigest(format!(
                "outcome {} evidence digest must be sha256:<64 hexadecimal characters>",
                outcome.cluster_id
            )));
        }
        if !seen_digests.insert(outcome.evidence_digest.as_str()) {
            return Err(IttAnalysisError::DuplicateOutcome(format!(
                "reused outcome evidence digest {}",
                outcome.evidence_digest
            )));
        }
        let cluster = expected_clusters.get(outcome.cluster_id.as_str()).ok_or_else(|| {
            IttAnalysisError::IdentityMismatch(format!(
                "outcome references unknown cluster {}",
                outcome.cluster_id
            ))
        })?;
        if outcome.reported_assignment != cluster.assigned_arm {
            return Err(IttAnalysisError::IdentityMismatch(format!(
                "outcome arm for cluster {} differs from frozen assignment",
                outcome.cluster_id
            )));
        }
        if outcome.primary_outcome_id != design.primary_outcome_id
            || outcome.primary_outcome_unit != design.primary_outcome_unit
            || outcome.measurement_boundary_id != design.measurement_boundary_id
        {
            return Err(IttAnalysisError::IdentityMismatch(format!(
                "outcome definition/unit/measurement boundary mismatch for cluster {}",
                outcome.cluster_id
            )));
        }
        if outcome.outcome_window_start_unix_ms != design.dispatch_window.start_unix_ms
            || outcome.outcome_window_end_unix_ms != design.recovery_window.end_unix_ms
            || outcome.outcome_observed_at_unix_ms < design.recovery_window.end_unix_ms
        {
            return Err(IttAnalysisError::TimeBoundaryMismatch(format!(
                "outcome for cluster {} does not cover and follow the full dispatch+recovery horizon",
                outcome.cluster_id
            )));
        }
        if !outcome.outcome_value.is_finite() {
            return Err(IttAnalysisError::NonFiniteOutcome(format!(
                "outcome for cluster {} must be finite",
                outcome.cluster_id
            )));
        }
        if !outcome.received_treatment_fraction.is_finite()
            || !(0.0..=1.0).contains(&outcome.received_treatment_fraction)
        {
            return Err(IttAnalysisError::InvalidTreatmentDose(format!(
                "treatment receipt fraction for cluster {} must be within [0, 1]",
                outcome.cluster_id
            )));
        }
        outcomes.insert(outcome.cluster_id.clone(), outcome.outcome_value);
    }

    if outcomes.len() != audit.cluster_count {
        return Err(IttAnalysisError::MissingOutcome(
            "one complete outcome per randomized cluster is required; no post-randomization cluster dropping is allowed"
                .to_owned(),
        ));
    }
    Ok(outcomes)
}

/// Computes an assignment-based, stratified cluster-level ITT estimate.
/// All outcomes must cover the full dispatch and recovery horizon.
pub fn estimate_cluster_itt(
    input: &ClusterIttAnalysisInput,
) -> Result<ClusterIttAnalysisReport, IttAnalysisError> {
    if input.schema_version != SCHEMA_VERSION || input.benchmark_id != BENCHMARK_ID {
        return Err(IttAnalysisError::InvalidInput(
            "schema_version or benchmark_id mismatch".to_owned(),
        ));
    }
    require_nonempty("claim ceiling", &input.claim_ceiling)?;
    if !(MIN_MONTE_CARLO_DRAWS..=MAX_MONTE_CARLO_DRAWS).contains(&input.permutation_budget) {
        return Err(IttAnalysisError::InvalidInput(format!(
            "permutation_budget must be between {MIN_MONTE_CARLO_DRAWS} and {MAX_MONTE_CARLO_DRAWS}"
        )));
    }

    let design_audit = audit_causal_design(&input.design)
        .map_err(|error| IttAnalysisError::InvalidInput(format!("design audit failed: {error}")))?;
    if !design_audit.design_qualified {
        return Err(IttAnalysisError::InvalidInput(
            "causal design did not pass the pre-registration audit".to_owned(),
        ));
    }

    let outcomes = validate_outcomes(input, &design_audit)?;
    let treated_ids: BTreeSet<_> = input
        .design
        .clusters
        .iter()
        .filter(|cluster| cluster.assigned_arm == AssignmentArm::Treatment)
        .map(|cluster| cluster.cluster_id.clone())
        .collect();
    let (itt_effect_estimate, _) =
        stratified_effect(&input.design, &outcomes, &treated_ids)?;
    let (variance, degrees_of_freedom, mut strata) =
        stratified_neyman_variance(&input.design, &outcomes)?;
    let standard_error = variance.sqrt();
    let critical = student_t_critical_975(degrees_of_freedom);
    let confidence_interval_lower = itt_effect_estimate - critical * standard_error;
    let confidence_interval_upper = itt_effect_estimate + critical * standard_error;

    let (randomization_method, exact, permutations_evaluated, extreme, p_value) =
        permutation_p_value(
            &input.design,
            &outcomes,
            itt_effect_estimate,
            input.permutation_budget,
        )?;

    let treated_outcomes: Vec<_> = input
        .outcomes
        .iter()
        .filter(|outcome| outcome.reported_assignment == AssignmentArm::Treatment)
        .collect();
    let control_outcomes: Vec<_> = input
        .outcomes
        .iter()
        .filter(|outcome| outcome.reported_assignment == AssignmentArm::Control)
        .collect();
    let treatment_uptake = mean(
        &treated_outcomes
            .iter()
            .map(|outcome| outcome.received_treatment_fraction)
            .collect::<Vec<_>>(),
    )?;
    let control_contamination = mean(
        &control_outcomes
            .iter()
            .map(|outcome| outcome.received_treatment_fraction)
            .collect::<Vec<_>>(),
    )?;

    let pooled_treated_mean: f64 = strata
        .iter()
        .map(|stratum| stratum.stratum_weight * stratum.treated_mean_outcome)
        .sum();
    let pooled_control_mean: f64 = strata
        .iter()
        .map(|stratum| stratum.stratum_weight * stratum.control_mean_outcome)
        .sum();

    strata.sort_by(|left, right| left.stratum_id.cmp(&right.stratum_id));
    let input_digest = digest_serializable(input, "ITT input bundle")?;
    let design_digest = digest_serializable(&input.design, "causal design")?;

    Ok(ClusterIttAnalysisReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        input_digest,
        design_input_digest: design_digest,
        randomization_assignment_digest: design_audit.randomization_assignment_digest,
        experiment_id: input.design.experiment_id.clone(),
        primary_outcome_id: input.design.primary_outcome_id.clone(),
        primary_outcome_unit: input.design.primary_outcome_unit.clone(),
        measurement_boundary_id: input.design.measurement_boundary_id.clone(),
        estimand: "intention_to_treat".to_owned(),
        analysis_weighting: "equal_cluster_within_stratum_weights_by_randomized_cluster_count"
            .to_owned(),
        cluster_count: input.design.clusters.len(),
        stratum_count: input.design.strata.len(),
        treated_cluster_count: treated_outcomes.len(),
        control_cluster_count: control_outcomes.len(),
        complete_cluster_outcomes: true,
        cluster_level_analysis: true,
        assignment_based_itt: true,
        treated_mean_outcome: pooled_treated_mean,
        control_mean_outcome: pooled_control_mean,
        itt_effect_estimate,
        neyman_conservative_variance: variance,
        standard_error,
        satterthwaite_degrees_of_freedom: degrees_of_freedom,
        student_t_critical_value_975: critical,
        confidence_interval_95_lower: confidence_interval_lower,
        confidence_interval_95_upper: confidence_interval_upper,
        confidence_interval_method: "stratified_neyman_variance_with_conservative_floor_df_student_t_critical"
            .to_owned(),
        randomization_inference_method: randomization_method,
        randomization_inference_exact: exact,
        randomization_assignments_evaluated: permutations_evaluated,
        randomization_extreme_assignments: extreme,
        two_sided_sharp_null_p_value: p_value,
        treatment_uptake_fraction: treatment_uptake,
        control_contamination_fraction: control_contamination,
        missing_cluster_outcome_count: 0,
        outcome_evidence_authenticated: false,
        design_pre_registration_authenticated: false,
        interference_mapping_authenticated: false,
        external_randomness_source_authenticated: false,
        treatment_on_treated_identified: false,
        effect_transportability_established: false,
        settlement_authority_granted: false,
        payment_authorized: false,
        dispatch_authority_granted: false,
        strata,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sha(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn design() -> GridCausalDesignInput {
        serde_json::from_str(include_str!(
            "../../symtropy-grid-causal-design/fixtures/grid_causal_design_v0_1.json"
        ))
        .expect("grid causal design fixture must parse")
    }

    fn input() -> ClusterIttAnalysisInput {
        let design = design();
        let values = [
            ("cluster-N1", 10.0, true),
            ("cluster-N2", 12.0, true),
            ("cluster-N3", 1.0, false),
            ("cluster-N4", 3.0, false),
            ("cluster-S1", 11.0, true),
            ("cluster-S2", 2.0, false),
            ("cluster-S3", 13.0, true),
            ("cluster-S4", 4.0, false),
        ];
        let outcomes = values
            .iter()
            .enumerate()
            .map(|(index, (cluster_id, outcome_value, received))| {
                let cluster = design
                    .clusters
                    .iter()
                    .find(|cluster| cluster.cluster_id == *cluster_id)
                    .unwrap();
                ClusterOutcomeObservation {
                    cluster_id: (*cluster_id).to_owned(),
                    reported_assignment: cluster.assigned_arm,
                    primary_outcome_id: design.primary_outcome_id.clone(),
                    primary_outcome_unit: design.primary_outcome_unit.clone(),
                    measurement_boundary_id: design.measurement_boundary_id.clone(),
                    outcome_window_start_unix_ms: design.dispatch_window.start_unix_ms,
                    outcome_window_end_unix_ms: design.recovery_window.end_unix_ms,
                    outcome_observed_at_unix_ms: design.recovery_window.end_unix_ms + 1_000,
                    outcome_value: *outcome_value,
                    received_treatment_fraction: if cluster.assigned_arm == AssignmentArm::Control {
                        if *received { 1.0 } else { 0.0 }
                    } else if *received {
                        1.0
                    } else {
                        0.5
                    },
                    evidence_reference: format!("outcome-evidence-{index}"),
                    evidence_digest: sha(char::from_u32('a' as u32 + (index % 6) as u32).unwrap()),
                }
            })
            .collect();

        ClusterIttAnalysisInput {
            schema_version: SCHEMA_VERSION.to_owned(),
            benchmark_id: BENCHMARK_ID.to_owned(),
            design,
            outcomes,
            permutation_budget: 10_000,
            claim_ceiling: "synthetic cluster-level ITT estimator fixture only".to_owned(),
        }
    }

    #[test]
    fn exact_stratified_itt_matches_known_answer_and_randomization_p_value() {
        let report = estimate_cluster_itt(&input()).unwrap();
        assert_eq!(report.itt_effect_estimate, 9.0);
        assert_eq!(report.standard_error, 1.0);
        assert_eq!(report.randomization_inference_method, RandomizationInferenceMethod::ExactStratifiedPermutation);
        assert!(report.randomization_inference_exact);
        assert_eq!(report.randomization_assignments_evaluated, 36);
        assert_eq!(report.randomization_extreme_assignments, 2);
        assert!((report.two_sided_sharp_null_p_value - (2.0 / 36.0)).abs() < 1.0e-12);
        assert_eq!(report.treatment_uptake_fraction, 1.0);
        assert_eq!(report.control_contamination_fraction, 0.0);
        assert!(report.complete_cluster_outcomes);
        assert!(report.assignment_based_itt);
        assert!(report.cluster_level_analysis);
        assert!(!report.treatment_on_treated_identified);
        assert!(!report.outcome_evidence_authenticated);
        assert!(!report.effect_transportability_established);
        assert!(!report.settlement_authority_granted);
        assert!(!report.dispatch_authority_granted);
    }

    #[test]
    fn uptake_changes_do_not_change_assignment_based_itt() {
        let first = estimate_cluster_itt(&input()).unwrap();
        let mut changed_uptake = input();
        for outcome in &mut changed_uptake.outcomes {
            outcome.received_treatment_fraction = match outcome.reported_assignment {
                AssignmentArm::Treatment => 0.25,
                AssignmentArm::Control => 0.5,
            };
        }
        let second = estimate_cluster_itt(&changed_uptake).unwrap();
        assert_eq!(first.itt_effect_estimate, second.itt_effect_estimate);
        assert_eq!(second.treatment_uptake_fraction, 0.25);
        assert_eq!(second.control_contamination_fraction, 0.5);
        assert!(!second.treatment_on_treated_identified);
    }

    #[test]
    fn missing_and_duplicate_cluster_outcomes_are_rejected() {
        let mut missing = input();
        missing.outcomes.pop();
        assert!(matches!(
            estimate_cluster_itt(&missing),
            Err(IttAnalysisError::MissingOutcome(_))
        ));

        let mut duplicate = input();
        duplicate.outcomes[1].cluster_id = duplicate.outcomes[0].cluster_id.clone();
        assert!(matches!(
            estimate_cluster_itt(&duplicate),
            Err(IttAnalysisError::DuplicateOutcome(_))
        ));
    }

    #[test]
    fn outcome_cannot_be_observed_before_full_recovery_horizon() {
        let mut bad = input();
        bad.outcomes[0].outcome_observed_at_unix_ms = bad.design.recovery_window.end_unix_ms - 1;
        assert!(matches!(
            estimate_cluster_itt(&bad),
            Err(IttAnalysisError::TimeBoundaryMismatch(_))
        ));
    }

    #[test]
    fn reported_arm_outcome_unit_and_measurement_boundary_must_match_frozen_plan() {
        let mut wrong_arm = input();
        wrong_arm.outcomes[0].reported_assignment = AssignmentArm::Control;
        assert!(matches!(
            estimate_cluster_itt(&wrong_arm),
            Err(IttAnalysisError::IdentityMismatch(_))
        ));

        let mut wrong_unit = input();
        wrong_unit.outcomes[0].primary_outcome_unit = "MW".to_owned();
        assert!(matches!(
            estimate_cluster_itt(&wrong_unit),
            Err(IttAnalysisError::IdentityMismatch(_))
        ));

        let mut wrong_boundary = input();
        wrong_boundary.outcomes[0].measurement_boundary_id = "asset-terminal".to_owned();
        assert!(matches!(
            estimate_cluster_itt(&wrong_boundary),
            Err(IttAnalysisError::IdentityMismatch(_))
        ));
    }

    #[test]
    fn negative_event_plus_recovery_outcomes_are_preserved() {
        let mut bad = input();
        bad.outcomes[7].outcome_value = -2.5;
        let report = estimate_cluster_itt(&bad).unwrap();
        assert!(report.strata.iter().flat_map(|s| [s.treated_mean_outcome, s.control_mean_outcome]).any(|value| value < 0.0));
        assert!(report.itt_effect_estimate.is_finite());
    }

    #[test]
    fn evidence_references_and_digests_must_be_unique_and_well_formed() {
        let mut duplicate_reference = input();
        duplicate_reference.outcomes[1].evidence_reference =
            duplicate_reference.outcomes[0].evidence_reference.clone();
        assert!(matches!(
            estimate_cluster_itt(&duplicate_reference),
            Err(IttAnalysisError::DuplicateOutcome(_))
        ));

        let mut bad_digest = input();
        bad_digest.outcomes[0].evidence_digest = "sha256:bad".to_owned();
        assert!(matches!(
            estimate_cluster_itt(&bad_digest),
            Err(IttAnalysisError::InvalidDigest(_))
        ));
    }

    #[test]
    fn permutation_p_value_is_invariant_to_outcome_input_order() {
        let first = estimate_cluster_itt(&input()).unwrap();
        let mut reversed = input();
        reversed.outcomes.reverse();
        let second = estimate_cluster_itt(&reversed).unwrap();
        assert_eq!(first.itt_effect_estimate, second.itt_effect_estimate);
        assert_eq!(first.randomization_assignments_evaluated, second.randomization_assignments_evaluated);
        assert_eq!(first.two_sided_sharp_null_p_value, second.two_sided_sharp_null_p_value);
    }
}
