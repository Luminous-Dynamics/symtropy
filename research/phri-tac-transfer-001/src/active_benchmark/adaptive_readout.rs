//! Adaptive partial tactile readout under hard observability and event deadlines.
//!
//! This is a readout-planning oracle, not a sensor driver. It selects taxels
//! using only declared current predictions and acquisition costs. Evaluator-only
//! contact/event truth is held in a separate type and is never passed to the
//! selector. A skipped taxel is always reported as Unknown; this module never
//! manufactures a full field from an unobserved region.
//!
//! Cost values must be measured for the actual bus, multiplexer, driver and
//! sensor. Sample count alone is not a valid latency or energy estimate.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

pub const DEFAULT_TAXEL_COUNT: usize = 1_024;
pub const DEFAULT_READOUT_BUDGET: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct TaxelAcquisitionCandidate {
    pub taxel_index: usize,
    pub grid_x: usize,
    pub grid_y: usize,
    pub predicted_contact_probability: f64,
    pub predicted_field_uncertainty: f64,
    pub predicted_event_hazard: f64,
    pub readout_available: bool,
    pub estimated_readout_latency_ns: u64,
    pub estimated_readout_energy_nj: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveAcquisitionProblem {
    pub grid_width: usize,
    pub grid_height: usize,
    pub candidates: Vec<TaxelAcquisitionCandidate>,
    pub maximum_taxel_reads: usize,
    pub bus_setup_latency_ns: u64,
    pub bus_setup_energy_nj: u64,
    pub maximum_total_latency_ns: u64,
    pub maximum_total_energy_nj: u64,
    pub predicted_contact_center: Option<[f64; 2]>,
    pub seed: u64,
    pub sensorized_body_context_digest: String,
    pub readout_chain_digest: String,
    pub acquisition_profile_digest: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum AcquisitionPolicy {
    FullRaster,
    FixedRasterPrefix,
    UniformRandom,
    ContactLocality,
    RiskAdaptive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum TaxelEvidenceState {
    Observed,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct AcquisitionPlan {
    pub policy: AcquisitionPolicy,
    pub selected_taxel_indices: Vec<usize>,
    pub evidence_state_by_taxel: Vec<TaxelEvidenceState>,
    pub selected_taxel_count: usize,
    pub unknown_taxel_count: usize,
    pub estimated_total_latency_ns: u64,
    pub estimated_total_energy_nj: u64,
    pub maximum_taxel_reads: usize,
    pub sensorized_body_context_digest: String,
    pub readout_chain_digest: String,
    pub acquisition_profile_digest: String,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluatorOnlyContactTruth {
    pub active_contact_by_taxel: Vec<bool>,
    pub urgent_event_by_taxel: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct AcquisitionScorecard {
    pub policy: AcquisitionPolicy,
    pub taxel_count: usize,
    pub selected_taxel_count: usize,
    pub unknown_taxel_count: usize,
    pub observed_taxel_fraction: f64,
    pub active_contact_taxel_count: usize,
    pub observed_active_contact_taxel_count: usize,
    pub active_contact_recall: Option<f64>,
    pub unknown_active_contact_taxel_count: usize,
    pub urgent_event_taxel_count: usize,
    pub observed_urgent_event_taxel_count: usize,
    pub urgent_event_recall: Option<f64>,
    pub estimated_total_latency_ns: u64,
    pub estimated_total_energy_nj: u64,
    pub full_raster_latency_ns: u64,
    pub speedup_vs_full_raster: Option<f64>,
    pub full_raster_energy_nj: u64,
    pub energy_reduction_fraction_vs_full_raster: Option<f64>,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
struct CandidateOrder {
    index: usize,
    event_hazard: f64,
    uncertainty: f64,
    contact_probability: f64,
    distance_squared: f64,
}

fn validate_problem(problem: &AdaptiveAcquisitionProblem) -> Result<usize, String> {
    if problem.grid_width == 0 || problem.grid_height == 0 {
        return Err("acquisition grid dimensions must be nonzero".to_owned());
    }
    let taxel_count = problem
        .grid_width
        .checked_mul(problem.grid_height)
        .ok_or_else(|| "acquisition grid size overflows usize".to_owned())?;
    if problem.candidates.len() != taxel_count {
        return Err("candidate count must equal grid_width × grid_height".to_owned());
    }
    if problem.maximum_taxel_reads == 0 || problem.maximum_taxel_reads > taxel_count {
        return Err("maximum_taxel_reads must be within 1..=taxel_count".to_owned());
    }
    if problem.maximum_total_latency_ns == 0 || problem.maximum_total_energy_nj == 0 {
        return Err("latency and energy budgets must be positive".to_owned());
    }
    if problem.seed == 0 {
        return Err("deterministic acquisition seed must be nonzero".to_owned());
    }
    for (name, value) in [
        ("sensorized_body_context_digest", problem.sensorized_body_context_digest.as_str()),
        ("readout_chain_digest", problem.readout_chain_digest.as_str()),
        ("acquisition_profile_digest", problem.acquisition_profile_digest.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("acquisition context field {name} is missing"));
        }
    }
    if let Some(center) = problem.predicted_contact_center {
        if center.iter().any(|value| !value.is_finite())
            || center[0] < 0.0
            || center[0] > (problem.grid_width - 1) as f64
            || center[1] < 0.0
            || center[1] > (problem.grid_height - 1) as f64
        {
            return Err("predicted contact center must be finite and inside the acquisition grid".to_owned());
        }
    }

    let mut seen = BTreeSet::new();
    for candidate in &problem.candidates {
        if candidate.taxel_index >= taxel_count || !seen.insert(candidate.taxel_index) {
            return Err("candidate taxel indices must be unique and in range".to_owned());
        }
        let expected_x = candidate.taxel_index % problem.grid_width;
        let expected_y = candidate.taxel_index / problem.grid_width;
        if candidate.grid_x != expected_x || candidate.grid_y != expected_y {
            return Err("candidate grid coordinates do not match its canonical taxel index".to_owned());
        }
        for (name, value) in [
            ("predicted_contact_probability", candidate.predicted_contact_probability),
            ("predicted_field_uncertainty", candidate.predicted_field_uncertainty),
            ("predicted_event_hazard", candidate.predicted_event_hazard),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(format!("{name} must be finite and in [0, 1]"));
            }
        }
        if candidate.estimated_readout_latency_ns == 0
            || candidate.estimated_readout_energy_nj == 0
        {
            return Err("each candidate must have positive declared latency and energy cost".to_owned());
        }
    }
    Ok(taxel_count)
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn candidate_order(
    problem: &AdaptiveAcquisitionProblem,
    candidate: &TaxelAcquisitionCandidate,
) -> CandidateOrder {
    let distance_squared = problem
        .predicted_contact_center
        .map(|center| {
            let dx = candidate.grid_x as f64 - center[0];
            let dy = candidate.grid_y as f64 - center[1];
            dx * dx + dy * dy
        })
        .unwrap_or(f64::INFINITY);
    CandidateOrder {
        index: candidate.taxel_index,
        event_hazard: candidate.predicted_event_hazard,
        uncertainty: candidate.predicted_field_uncertainty,
        contact_probability: candidate.predicted_contact_probability,
        distance_squared,
    }
}

fn sort_candidates(
    problem: &AdaptiveAcquisitionProblem,
    policy: AcquisitionPolicy,
) -> Vec<CandidateOrder> {
    let mut ordered: Vec<_> = problem
        .candidates
        .iter()
        .filter(|candidate| candidate.readout_available)
        .map(|candidate| candidate_order(problem, candidate))
        .collect();

    match policy {
        AcquisitionPolicy::FullRaster | AcquisitionPolicy::FixedRasterPrefix => {
            ordered.sort_by_key(|candidate| candidate.index);
        }
        AcquisitionPolicy::UniformRandom => {
            let mut state = problem.seed;
            for index in (1..ordered.len()).rev() {
                let swap_index = (next_random(&mut state) as usize) % (index + 1);
                ordered.swap(index, swap_index);
            }
        }
        AcquisitionPolicy::ContactLocality => {
            ordered.sort_by(|left, right| {
                left.distance_squared
                    .total_cmp(&right.distance_squared)
                    .then_with(|| right.event_hazard.total_cmp(&left.event_hazard))
                    .then_with(|| right.contact_probability.total_cmp(&left.contact_probability))
                    .then_with(|| left.index.cmp(&right.index))
            });
        }
        AcquisitionPolicy::RiskAdaptive => {
            ordered.sort_by(|left, right| {
                right
                    .event_hazard
                    .total_cmp(&left.event_hazard)
                    .then_with(|| right.uncertainty.total_cmp(&left.uncertainty))
                    .then_with(|| right.contact_probability.total_cmp(&left.contact_probability))
                    .then_with(|| left.index.cmp(&right.index))
            });
        }
    }
    ordered
}

/// Build a readout plan from current prediction features only. Evaluator-only
/// contact and event labels are deliberately not accepted by this function.
pub fn plan_adaptive_readout(
    problem: &AdaptiveAcquisitionProblem,
    policy: AcquisitionPolicy,
) -> Result<AcquisitionPlan, String> {
    let taxel_count = validate_problem(problem)?;
    if policy == AcquisitionPolicy::ContactLocality && problem.predicted_contact_center.is_none() {
        return Err("contact-locality acquisition requires a predicted contact center".to_owned());
    }
    let ordered = sort_candidates(problem, policy);
    let mut selected = Vec::new();
    let mut total_latency = problem.bus_setup_latency_ns;
    let mut total_energy = problem.bus_setup_energy_nj;

    let available_count = problem
        .candidates
        .iter()
        .filter(|candidate| candidate.readout_available)
        .count();
    if policy == AcquisitionPolicy::FullRaster && problem.maximum_taxel_reads < available_count {
        return Err("full-raster policy exceeds the maximum taxel-read budget".to_owned());
    }
    let required_count = if policy == AcquisitionPolicy::FullRaster {
        available_count
    } else {
        problem.maximum_taxel_reads
    };

    for candidate_order in ordered {
        if selected.len() >= required_count {
            break;
        }
        let candidate = &problem.candidates[candidate_order.index];
        let next_latency = total_latency
            .checked_add(candidate.estimated_readout_latency_ns)
            .ok_or_else(|| "estimated acquisition latency overflows u64".to_owned())?;
        let next_energy = total_energy
            .checked_add(candidate.estimated_readout_energy_nj)
            .ok_or_else(|| "estimated acquisition energy overflows u64".to_owned())?;

        if next_latency > problem.maximum_total_latency_ns
            || next_energy > problem.maximum_total_energy_nj
        {
            if policy == AcquisitionPolicy::FullRaster {
                return Err("full-raster readout exceeds the declared latency or energy budget".to_owned());
            }
            continue;
        }

        total_latency = next_latency;
        total_energy = next_energy;
        selected.push(candidate_order.index);
    }

    if selected.is_empty() {
        return Err("no taxel can be admitted within the declared acquisition budget".to_owned());
    }
    if policy == AcquisitionPolicy::FullRaster && selected.len() != required_count {
        return Err("full-raster readout could not acquire every available taxel".to_owned());
    }

    let mut evidence_state_by_taxel = vec![TaxelEvidenceState::Unknown; taxel_count];
    for index in &selected {
        evidence_state_by_taxel[*index] = TaxelEvidenceState::Observed;
    }
    let unknown_taxel_count = evidence_state_by_taxel
        .iter()
        .filter(|state| **state == TaxelEvidenceState::Unknown)
        .count();

    Ok(AcquisitionPlan {
        policy,
        selected_taxel_count: selected.len(),
        unknown_taxel_count,
        selected_taxel_indices: selected,
        evidence_state_by_taxel,
        estimated_total_latency_ns: total_latency,
        estimated_total_energy_nj: total_energy,
        maximum_taxel_reads: problem.maximum_taxel_reads,
        sensorized_body_context_digest: problem.sensorized_body_context_digest.clone(),
        readout_chain_digest: problem.readout_chain_digest.clone(),
        acquisition_profile_digest: problem.acquisition_profile_digest.clone(),
        claim_class: "synthetic_adaptive_partial_readout_plan_only",
    })
}

/// Evaluator-only scoring is kept outside the selector. A selected reading
/// counts as covered only when it maps to a true active/event taxel. Skipped
/// taxels remain Unknown, even when model predictions assign them values.
pub fn score_acquisition_plan(
    problem: &AdaptiveAcquisitionProblem,
    plan: &AcquisitionPlan,
    truth: &EvaluatorOnlyContactTruth,
) -> Result<AcquisitionScorecard, String> {
    let taxel_count = validate_problem(problem)?;
    if truth.active_contact_by_taxel.len() != taxel_count
        || truth.urgent_event_by_taxel.len() != taxel_count
    {
        return Err("evaluator-only truth vectors must match the taxel count".to_owned());
    }
    if plan.sensorized_body_context_digest != problem.sensorized_body_context_digest
        || plan.readout_chain_digest != problem.readout_chain_digest
        || plan.acquisition_profile_digest != problem.acquisition_profile_digest
    {
        return Err("acquisition plan context does not match the current sensor/readout profile".to_owned());
    }
    if plan.evidence_state_by_taxel.len() != taxel_count
        || plan.selected_taxel_count != plan.selected_taxel_indices.len()
        || plan.maximum_taxel_reads != problem.maximum_taxel_reads
        || (plan.selected_taxel_count > problem.maximum_taxel_reads
            && plan.policy != AcquisitionPolicy::FullRaster)
    {
        return Err("acquisition plan dimensions or readout budget are invalid".to_owned());
    }
    let mut observed = vec![false; taxel_count];
    let mut expected_latency = problem.bus_setup_latency_ns;
    let mut expected_energy = problem.bus_setup_energy_nj;
    for index in &plan.selected_taxel_indices {
        if *index >= taxel_count || observed[*index] {
            return Err("acquisition plan contains duplicate or out-of-range taxel indices".to_owned());
        }
        let candidate = &problem.candidates[*index];
        if !candidate.readout_available {
            return Err("acquisition plan includes an unavailable taxel channel".to_owned());
        }
        observed[*index] = true;
        expected_latency = expected_latency
            .checked_add(candidate.estimated_readout_latency_ns)
            .ok_or_else(|| "recomputed acquisition latency overflows u64".to_owned())?;
        expected_energy = expected_energy
            .checked_add(candidate.estimated_readout_energy_nj)
            .ok_or_else(|| "recomputed acquisition energy overflows u64".to_owned())?;
    }
    if plan.estimated_total_latency_ns != expected_latency
        || plan.estimated_total_energy_nj != expected_energy
    {
        return Err("acquisition plan latency/energy receipts do not match selected taxel costs".to_owned());
    }
    let available_count = problem
        .candidates
        .iter()
        .filter(|candidate| candidate.readout_available)
        .count();
    if plan.policy == AcquisitionPolicy::FullRaster && plan.selected_taxel_count != available_count {
        return Err("full-raster plan must include every available taxel".to_owned());
    }
    let expected_unknown = taxel_count.saturating_sub(plan.selected_taxel_count);
    if plan.unknown_taxel_count != expected_unknown {
        return Err("acquisition plan unknown-taxel count does not match selected observations".to_owned());
    }
    for (index, is_observed) in observed.iter().enumerate() {
        let expected = if *is_observed {
            TaxelEvidenceState::Observed
        } else {
            TaxelEvidenceState::Unknown
        };
        if plan.evidence_state_by_taxel[index] != expected {
            return Err("plan evidence-state mask does not match its selected indices".to_owned());
        }
    }
    if plan.selected_taxel_count == 0 || plan.estimated_total_latency_ns > problem.maximum_total_latency_ns
        || plan.estimated_total_energy_nj > problem.maximum_total_energy_nj
    {
        return Err("plan exceeds readout budget or has no selected taxels".to_owned());
    }

    let active_contact_taxel_count = truth
        .active_contact_by_taxel
        .iter()
        .filter(|active| **active)
        .count();
    let observed_active_contact_taxel_count = truth
        .active_contact_by_taxel
        .iter()
        .zip(&observed)
        .filter(|(active, seen)| **active && **seen)
        .count();
    let unknown_active_contact_taxel_count =
        active_contact_taxel_count.saturating_sub(observed_active_contact_taxel_count);
    let urgent_event_taxel_count = truth
        .urgent_event_by_taxel
        .iter()
        .filter(|event| **event)
        .count();
    let observed_urgent_event_taxel_count = truth
        .urgent_event_by_taxel
        .iter()
        .zip(&observed)
        .filter(|(event, seen)| **event && **seen)
        .count();

    let full_raster_latency_ns = problem
        .candidates
        .iter()
        .filter(|candidate| candidate.readout_available)
        .try_fold(problem.bus_setup_latency_ns, |total, candidate| {
            total.checked_add(candidate.estimated_readout_latency_ns)
        })
        .ok_or_else(|| "full-raster latency overflows u64".to_owned())?;
    let full_raster_energy_nj = problem
        .candidates
        .iter()
        .filter(|candidate| candidate.readout_available)
        .try_fold(problem.bus_setup_energy_nj, |total, candidate| {
            total.checked_add(candidate.estimated_readout_energy_nj)
        })
        .ok_or_else(|| "full-raster energy overflows u64".to_owned())?;

    Ok(AcquisitionScorecard {
        policy: plan.policy,
        taxel_count,
        selected_taxel_count: plan.selected_taxel_count,
        unknown_taxel_count: plan.unknown_taxel_count,
        observed_taxel_fraction: plan.selected_taxel_count as f64 / taxel_count as f64,
        active_contact_taxel_count,
        observed_active_contact_taxel_count,
        active_contact_recall: (active_contact_taxel_count > 0).then(|| {
            observed_active_contact_taxel_count as f64 / active_contact_taxel_count as f64
        }),
        unknown_active_contact_taxel_count,
        urgent_event_taxel_count,
        observed_urgent_event_taxel_count,
        urgent_event_recall: (urgent_event_taxel_count > 0).then(|| {
            observed_urgent_event_taxel_count as f64 / urgent_event_taxel_count as f64
        }),
        estimated_total_latency_ns: plan.estimated_total_latency_ns,
        estimated_total_energy_nj: plan.estimated_total_energy_nj,
        full_raster_latency_ns,
        speedup_vs_full_raster: (plan.estimated_total_latency_ns > 0).then(|| {
            full_raster_latency_ns as f64 / plan.estimated_total_latency_ns as f64
        }),
        full_raster_energy_nj,
        energy_reduction_fraction_vs_full_raster: (full_raster_energy_nj > 0).then(|| {
            1.0 - plan.estimated_total_energy_nj as f64 / full_raster_energy_nj as f64
        }),
        claim_class: "synthetic_partial_readout_evaluator_only",
    })
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct AdaptiveReadoutExample {
    pub observable_event_scores: Vec<AcquisitionScorecard>,
    pub off_prediction_region_event_scores: Vec<AcquisitionScorecard>,
    pub claim_class: &'static str,
}

fn synthetic_readout_problem(seed: u64, maximum_taxel_reads: usize) -> AdaptiveAcquisitionProblem {
    let grid_width = 32;
    let grid_height = 32;
    let center = [15.0, 15.0];
    let candidates = (0..(grid_width * grid_height))
        .map(|taxel_index| {
            let grid_x = taxel_index % grid_width;
            let grid_y = taxel_index / grid_width;
            let dx = grid_x as f64 - center[0];
            let dy = grid_y as f64 - center[1];
            let distance = (dx * dx + dy * dy).sqrt();
            let contact_probability = (1.0 - distance / 12.0).clamp(0.0, 1.0);
            let uncertainty = (1.0 - contact_probability) * 0.75;
            let event_hazard = if (grid_x == 16 || grid_x == 17)
                && (grid_y == 15 || grid_y == 16)
            {
                0.99
            } else {
                contact_probability * 0.1
            };
            TaxelAcquisitionCandidate {
                taxel_index,
                grid_x,
                grid_y,
                predicted_contact_probability: contact_probability,
                predicted_field_uncertainty: uncertainty,
                predicted_event_hazard: event_hazard,
                readout_available: true,
                estimated_readout_latency_ns: 10_000,
                estimated_readout_energy_nj: 40,
            }
        })
        .collect();

    AdaptiveAcquisitionProblem {
        grid_width,
        grid_height,
        candidates,
        maximum_taxel_reads,
        bus_setup_latency_ns: 25_000,
        bus_setup_energy_nj: 100,
        maximum_total_latency_ns: 20_000_000,
        maximum_total_energy_nj: 100_000,
        predicted_contact_center: Some(center),
        seed: seed.max(1),
        sensorized_body_context_digest: "sha256:synthetic-1024-taxel-body".to_owned(),
        readout_chain_digest: "sha256:synthetic-readout-chain".to_owned(),
        acquisition_profile_digest: "sha256:synthetic-readout-cost-profile".to_owned(),
    }
}

fn synthetic_readout_truth(event_center: [usize; 2]) -> EvaluatorOnlyContactTruth {
    let width = 32;
    let height = 32;
    let mut active = vec![false; width * height];
    let mut urgent = vec![false; width * height];
    for y in 0..height {
        for x in 0..width {
            if x.abs_diff(15) <= 2 && y.abs_diff(15) <= 2 {
                active[y * width + x] = true;
            }
            if x.abs_diff(event_center[0]) <= 1 && y.abs_diff(event_center[1]) <= 1 {
                urgent[y * width + x] = true;
            }
        }
    }
    EvaluatorOnlyContactTruth {
        active_contact_by_taxel: active,
        urgent_event_by_taxel: urgent,
    }
}

/// Run a deterministic 1,024-taxel example comparing full raster, fixed,
/// random, locality and risk-adaptive plans. The second evaluator case places
/// an event outside the predicted contact region to expose the limits of
/// prediction-guided sparse sampling.
pub fn run_synthetic_readout_example(seed: u64) -> Result<AdaptiveReadoutExample, String> {
    if seed == 0 {
        return Err("synthetic readout example seed must be nonzero".to_owned());
    }
    let policies = [
        (AcquisitionPolicy::FullRaster, DEFAULT_TAXEL_COUNT),
        (AcquisitionPolicy::FixedRasterPrefix, DEFAULT_READOUT_BUDGET),
        (AcquisitionPolicy::UniformRandom, DEFAULT_READOUT_BUDGET),
        (AcquisitionPolicy::ContactLocality, DEFAULT_READOUT_BUDGET),
        (AcquisitionPolicy::RiskAdaptive, DEFAULT_READOUT_BUDGET),
        (AcquisitionPolicy::RiskAdaptive, 64),
    ];
    let observable_truth = synthetic_readout_truth([16, 16]);
    let off_prediction_truth = synthetic_readout_truth([30, 30]);
    let mut observable_event_scores = Vec::with_capacity(policies.len());
    let mut off_prediction_region_event_scores = Vec::with_capacity(policies.len());

    for (policy, budget) in policies {
        let problem = synthetic_readout_problem(seed, budget);
        let plan = plan_adaptive_readout(&problem, policy)?;
        observable_event_scores.push(score_acquisition_plan(&problem, &plan, &observable_truth)?);

        let off_prediction_plan = plan_adaptive_readout(&problem, policy)?;
        off_prediction_region_event_scores.push(score_acquisition_plan(
            &problem,
            &off_prediction_plan,
            &off_prediction_truth,
        )?);
    }

    Ok(AdaptiveReadoutExample {
        observable_event_scores,
        off_prediction_region_event_scores,
        claim_class: "synthetic_partial_readout_policy_comparison_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem(budget: usize) -> AdaptiveAcquisitionProblem {
        let width = 32;
        let height = 32;
        let candidates = (0..(width * height))
            .map(|taxel_index| {
                let grid_x = taxel_index % width;
                let grid_y = taxel_index / width;
                let dx = grid_x as f64 - 15.0;
                let dy = grid_y as f64 - 15.0;
                let distance = (dx * dx + dy * dy).sqrt();
                let contact_probability = (1.0 - distance / 12.0).clamp(0.0, 1.0);
                let uncertainty = (1.0 - contact_probability) * 0.75;
                TaxelAcquisitionCandidate {
                    taxel_index,
                    grid_x,
                    grid_y,
                    predicted_contact_probability: contact_probability,
                    predicted_field_uncertainty: uncertainty,
                    predicted_event_hazard: if (grid_x == 16 || grid_x == 17)
                        && (grid_y == 15 || grid_y == 16)
                    {
                        0.99
                    } else {
                        contact_probability * 0.1
                    },
                    readout_available: true,
                    estimated_readout_latency_ns: 10_000,
                    estimated_readout_energy_nj: 40,
                }
            })
            .collect();
        AdaptiveAcquisitionProblem {
            grid_width: width,
            grid_height: height,
            candidates,
            maximum_taxel_reads: budget,
            bus_setup_latency_ns: 25_000,
            bus_setup_energy_nj: 100,
            maximum_total_latency_ns: 20_000_000,
            maximum_total_energy_nj: 100_000,
            predicted_contact_center: Some([15.0, 15.0]),
            seed: 2026,
            sensorized_body_context_digest: "sha256:body-context".to_owned(),
            readout_chain_digest: "sha256:readout-chain".to_owned(),
            acquisition_profile_digest: "sha256:readout-profile".to_owned(),
        }
    }

    fn truth(width: usize, height: usize, center: [usize; 2], event_center: [usize; 2]) -> EvaluatorOnlyContactTruth {
        let mut active = vec![false; width * height];
        let mut urgent = vec![false; width * height];
        for y in 0..height {
            for x in 0..width {
                if x.abs_diff(center[0]) <= 2 && y.abs_diff(center[1]) <= 2 {
                    active[y * width + x] = true;
                }
                if x.abs_diff(event_center[0]) <= 1 && y.abs_diff(event_center[1]) <= 1 {
                    urgent[y * width + x] = true;
                }
            }
        }
        EvaluatorOnlyContactTruth {
            active_contact_by_taxel: active,
            urgent_event_by_taxel: urgent,
        }
    }

    #[test]
    fn sparse_plans_never_promote_unread_taxels_to_observed() {
        let problem = problem(DEFAULT_READOUT_BUDGET);
        let plan = plan_adaptive_readout(&problem, AcquisitionPolicy::RiskAdaptive)
            .expect("risk-adaptive plan");
        assert_eq!(plan.selected_taxel_count, 16);
        assert_eq!(plan.unknown_taxel_count, 1_008);
        assert_eq!(
            plan.evidence_state_by_taxel
                .iter()
                .filter(|state| **state == TaxelEvidenceState::Observed)
                .count(),
            16
        );
        assert!(plan
            .evidence_state_by_taxel
            .iter()
            .filter(|state| **state == TaxelEvidenceState::Unknown)
            .count()
            > 1_000);
    }

    #[test]
    fn risk_adaptive_policy_prioritizes_declared_event_hazard_and_is_deterministic() {
        let input = problem(DEFAULT_READOUT_BUDGET);
        let first = plan_adaptive_readout(&input, AcquisitionPolicy::RiskAdaptive)
            .expect("first risk plan");
        let second = plan_adaptive_readout(&input, AcquisitionPolicy::RiskAdaptive)
            .expect("second risk plan");
        assert_eq!(first, second);
        let highest_hazard_selected = first
            .selected_taxel_indices
            .iter()
            .filter(|index| input.candidates[**index].predicted_event_hazard >= 0.99)
            .count();
        assert_eq!(highest_hazard_selected, 4);
        assert!(first.selected_taxel_indices[..4].iter().all(|index| {
            input.candidates[*index].predicted_event_hazard >= 0.99
        }));
    }

    #[test]
    fn off_prediction_region_event_is_not_falsely_credited_to_the_selector() {
        let input = problem(DEFAULT_READOUT_BUDGET);
        let plan = plan_adaptive_readout(&input, AcquisitionPolicy::RiskAdaptive)
            .expect("risk plan");
        let truth = truth(32, 32, [15, 15], [30, 30]);
        let score = score_acquisition_plan(&input, &plan, &truth).expect("score");
        assert_eq!(score.urgent_event_recall, Some(0.0));
        assert!(score.unknown_active_contact_taxel_count > 0);
    }

    #[test]
    fn locality_selects_spatial_neighbours_while_uniform_is_seeded() {
        let input = problem(DEFAULT_READOUT_BUDGET);
        let locality = plan_adaptive_readout(&input, AcquisitionPolicy::ContactLocality)
            .expect("locality plan");
        assert!(locality.selected_taxel_indices.iter().all(|index| {
            let candidate = &input.candidates[*index];
            (candidate.grid_x as f64 - 15.0).powi(2)
                + (candidate.grid_y as f64 - 15.0).powi(2)
                <= 9.0
        }));

        let random_a = plan_adaptive_readout(&input, AcquisitionPolicy::UniformRandom)
            .expect("random plan a");
        let random_b = plan_adaptive_readout(&input, AcquisitionPolicy::UniformRandom)
            .expect("random plan b");
        assert_eq!(random_a.selected_taxel_indices, random_b.selected_taxel_indices);
        assert_eq!(random_a.selected_taxel_count, DEFAULT_READOUT_BUDGET);
    }

    #[test]
    fn full_raster_is_reference_and_sparse_readout_cost_is_accounted_explicitly() {
        let mut full_input = problem(DEFAULT_TAXEL_COUNT);
        full_input.maximum_total_latency_ns = 20_000_000;
        let full_plan = plan_adaptive_readout(&full_input, AcquisitionPolicy::FullRaster)
            .expect("full raster");
        let full_truth = truth(32, 32, [15, 15], [16, 16]);
        let full_score = score_acquisition_plan(&full_input, &full_plan, &full_truth)
            .expect("full score");
        assert_eq!(full_score.active_contact_recall, Some(1.0));
        assert_eq!(full_score.urgent_event_recall, Some(1.0));

        let sparse_input = problem(DEFAULT_READOUT_BUDGET);
        let sparse_plan =
            plan_adaptive_readout(&sparse_input, AcquisitionPolicy::RiskAdaptive)
                .expect("sparse plan");
        let sparse_score = score_acquisition_plan(&sparse_input, &sparse_plan, &full_truth)
            .expect("sparse score");
        assert!(sparse_score.speedup_vs_full_raster.unwrap() > 1.0);
        assert!(sparse_score.estimated_total_energy_nj < sparse_score.full_raster_energy_nj);
        assert_eq!(sparse_score.unknown_taxel_count, 1_008);
    }

    #[test]
    fn unavailable_channels_and_deadline_or_energy_limits_are_fail_closed() {
        let mut unavailable = problem(DEFAULT_READOUT_BUDGET);
        for candidate in unavailable.candidates.iter_mut().take(DEFAULT_READOUT_BUDGET) {
            candidate.readout_available = false;
        }
        let plan = plan_adaptive_readout(&unavailable, AcquisitionPolicy::FixedRasterPrefix)
            .expect("remaining channels available");
        assert_eq!(plan.selected_taxel_count, DEFAULT_READOUT_BUDGET);
        assert!(!plan.selected_taxel_indices.iter().any(|index| *index < DEFAULT_READOUT_BUDGET));

        let mut no_budget = problem(DEFAULT_READOUT_BUDGET);
        no_budget.maximum_total_latency_ns = no_budget.bus_setup_latency_ns;
        assert!(plan_adaptive_readout(&no_budget, AcquisitionPolicy::RiskAdaptive).is_err());

        let mut full_no_budget = problem(DEFAULT_TAXEL_COUNT);
        full_no_budget.maximum_total_latency_ns = 100_000;
        assert!(plan_adaptive_readout(&full_no_budget, AcquisitionPolicy::FullRaster).is_err());
    }

    #[test]
    fn evaluator_rejects_context_mismatch_and_bad_plan_masks() {
        let input = problem(DEFAULT_READOUT_BUDGET);
        let plan = plan_adaptive_readout(&input, AcquisitionPolicy::RiskAdaptive)
            .expect("risk plan");
        let evaluator_truth = truth(32, 32, [15, 15], [16, 16]);
        let mut changed_context = input.clone();
        changed_context.readout_chain_digest = "sha256:new-readout".to_owned();
        assert!(score_acquisition_plan(&changed_context, &plan, &evaluator_truth).is_err());

        let mut bad_mask = plan;
        bad_mask.evidence_state_by_taxel[0] = TaxelEvidenceState::Observed;
        assert!(score_acquisition_plan(&input, &bad_mask, &evaluator_truth).is_err());
    }

    #[test]
    fn evaluator_rejects_forged_costs_unavailable_taxels_and_incomplete_full_raster() {
        let input = problem(DEFAULT_READOUT_BUDGET);
        let plan = plan_adaptive_readout(&input, AcquisitionPolicy::RiskAdaptive)
            .expect("risk plan");
        let evaluator_truth = truth(32, 32, [15, 15], [16, 16]);

        let mut forged_cost = plan.clone();
        forged_cost.estimated_total_latency_ns -= 1;
        assert!(score_acquisition_plan(&input, &forged_cost, &evaluator_truth).is_err());

        let mut unavailable_input = input.clone();
        unavailable_input.candidates[plan.selected_taxel_indices[0]].readout_available = false;
        assert!(score_acquisition_plan(&unavailable_input, &plan, &evaluator_truth).is_err());

        let mut forged_full_raster = plan;
        forged_full_raster.policy = AcquisitionPolicy::FullRaster;
        assert!(score_acquisition_plan(&input, &forged_full_raster, &evaluator_truth).is_err());
    }

    #[test]
    fn synthetic_example_separates_predicted_from_off_prediction_region_events() {
        let example = run_synthetic_readout_example(20261117).expect("synthetic example");
        assert_eq!(example.observable_event_scores.len(), 6);
        assert_eq!(example.off_prediction_region_event_scores.len(), 6);
        assert_eq!(
            example.observable_event_scores[0].active_contact_recall,
            Some(1.0)
        );
        assert_eq!(
            example.observable_event_scores[0].urgent_event_recall,
            Some(1.0)
        );
        assert!(example.off_prediction_region_event_scores[1..]
            .iter()
            .all(|score| score.urgent_event_recall == Some(0.0)));
        assert_eq!(
            example.off_prediction_region_event_scores[0].urgent_event_recall,
            Some(1.0)
        );
    }

    #[test]
    fn malformed_candidates_and_truth_dimensions_fail_closed() {
        let mut bad_candidate = problem(DEFAULT_READOUT_BUDGET);
        bad_candidate.candidates[0].predicted_event_hazard = 1.1;
        assert!(plan_adaptive_readout(&bad_candidate, AcquisitionPolicy::RiskAdaptive).is_err());

        let input = problem(DEFAULT_READOUT_BUDGET);
        let plan = plan_adaptive_readout(&input, AcquisitionPolicy::RiskAdaptive)
            .expect("risk plan");
        let bad_truth = EvaluatorOnlyContactTruth {
            active_contact_by_taxel: vec![false; 3],
            urgent_event_by_taxel: vec![false; 3],
        };
        assert!(score_acquisition_plan(&input, &plan, &bad_truth).is_err());
    }
}
