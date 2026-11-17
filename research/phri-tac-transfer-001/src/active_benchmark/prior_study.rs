//! Development/holdout study for tactile field priors across contact morphologies.
//!
//! Development seeds compare a frozen candidate penalty set. The selected
//! nonnegative L1 penalty is then evaluated once on disjoint final seeds.
//! Evaluator-only morphology labels and field truth never enter the estimator.
//!
//! This is a synthetic model-selection study. The selected penalty is a
//! development-tuned prior, not a sensor calibration or physical measurement.

#![forbid(unsafe_code)]

use super::field_reconstruction::{
    FieldReconstructionConfig, FieldReconstructionProblem, FieldReconstructionReport, SpatialEdge,
    reconstruct_contact_field,
};

const TAXEL_COUNT: usize = 113;
const MODULE_COUNT: usize = 7;
const MODULE_SIZES: [usize; MODULE_COUNT] = [16, 16, 16, 16, 16, 16, 17];
const MIN_REPLICATES: usize = 50;
const GRAPH_L2_PENALTY: f64 = 0.02;
const DEVELOPMENT_L1_PENALTIES: [f64; 3] = [0.001, 0.003, 0.01];
const COUPLING_DIAGONAL: f64 = 0.76;
const COUPLING_NEIGHBOR: f64 = 0.12;
const RESPONSE_NOISE_SCALE: f64 = 0.0015;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactMorphology {
    SparsePatches,
    SmoothDistributed,
    SharpPiecewise,
    MultipleContacts,
    DenseUniform,
    MixedSparseSmooth,
}

impl ContactMorphology {
    pub const ALL: [Self; 6] = [
        Self::SparsePatches,
        Self::SmoothDistributed,
        Self::SharpPiecewise,
        Self::MultipleContacts,
        Self::DenseUniform,
        Self::MixedSparseSmooth,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SparsePatches => "sparse_patches",
            Self::SmoothDistributed => "smooth_distributed",
            Self::SharpPiecewise => "sharp_piecewise",
            Self::MultipleContacts => "multiple_contacts",
            Self::DenseUniform => "dense_uniform",
            Self::MixedSparseSmooth => "mixed_sparse_smooth",
        }
    }

    fn salt(self) -> u64 {
        match self {
            Self::SparsePatches => 0xA076_1D64_78BD_642F,
            Self::SmoothDistributed => 0xE703_7ED1_A0B4_28DB,
            Self::SharpPiecewise => 0x8EBC_6AF0_9C88_C6E3,
            Self::MultipleContacts => 0x5899_65CC_7537_4CC3,
            Self::DenseUniform => 0x1D8E_4E27_C47D_124F,
            Self::MixedSparseSmooth => 0xEB44_ACCA_B455_D165,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ReplicateScore {
    field_rmse: f64,
    support_ratio: f64,
    native_data_rank: usize,
    native_data_full_column_rank: bool,
    projected_gradient_residual: f64,
    converged: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PriorEstimatorSummary {
    pub estimator_id: String,
    pub mean_field_rmse: f64,
    pub p95_field_rmse: f64,
    pub maximum_field_rmse: f64,
    /// Negative means lower RMSE than the paired independent-taxel baseline.
    pub paired_rmse_delta_mean_vs_independent: f64,
    pub paired_rmse_delta_p95_vs_independent: f64,
    pub mean_support_ratio: f64,
    pub minimum_support_ratio: f64,
    pub mean_native_data_rank: f64,
    pub full_rank_replicates: usize,
    pub mean_projected_gradient_residual: f64,
    pub maximum_projected_gradient_residual: f64,
    pub converged_replicates: usize,
    pub replicate_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MorphologyPriorReport {
    pub morphology: &'static str,
    pub seed_first: u64,
    pub replicate_count: usize,
    pub estimators: Vec<PriorEstimatorSummary>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct L1PenaltyCandidateSummary {
    pub penalty: f64,
    /// Equal-weight mean of per-morphology development RMSE means.
    pub development_mean_field_rmse: f64,
    pub development_mean_delta_vs_independent: f64,
    /// Worst relative RMSE regression against the independent baseline across
    /// all development morphologies.
    pub maximum_family_relative_regression: f64,
    /// Promotion requires mean non-inferiority and a <=2% worst-family guard.
    pub eligible_for_promotion: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PriorStudyReport {
    pub development_seed_first: u64,
    pub final_seed_first: u64,
    pub replicate_count_per_morphology: usize,
    pub development: Vec<MorphologyPriorReport>,
    pub development_l1_candidates: Vec<L1PenaltyCandidateSummary>,
    pub selected_l1_penalty: Option<f64>,
    pub l1_selection_disposition: &'static str,
    pub final_test: Vec<MorphologyPriorReport>,
    pub claim_class: &'static str,
}

const MAXIMUM_ALLOWED_FAMILY_REGRESSION: f64 = 0.02;

fn select_l1_penalty(candidates: &[L1PenaltyCandidateSummary]) -> Option<f64> {
    candidates
        .iter()
        .filter(|candidate| candidate.eligible_for_promotion)
        .min_by(|left, right| {
            left.development_mean_field_rmse
                .total_cmp(&right.development_mean_field_rmse)
                .then_with(|| left.penalty.total_cmp(&right.penalty))
        })
        .map(|candidate| candidate.penalty)
}

#[derive(Clone, Debug, PartialEq)]
struct SyntheticFieldCase {
    mechanics_problem: FieldReconstructionProblem,
    independent_native_response: Vec<f64>,
    admitted_native_channels: Vec<bool>,
    evaluator_truth: Vec<f64>,
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn random_unit(state: &mut u64) -> f64 {
    let value = next_random(state) >> 11;
    value as f64 / ((1_u64 << 53) as f64)
}

fn module_ranges() -> Vec<std::ops::Range<usize>> {
    let mut start = 0;
    MODULE_SIZES
        .iter()
        .map(|size| {
            let range = start..start + size;
            start += size;
            range
        })
        .collect()
}

fn coupling_kernel() -> Vec<f64> {
    let mut kernel = vec![0.0; TAXEL_COUNT * TAXEL_COUNT];
    for range in module_ranges() {
        for index in range.clone() {
            kernel[index * TAXEL_COUNT + index] = COUPLING_DIAGONAL;
            if index > range.start {
                kernel[index * TAXEL_COUNT + index - 1] = COUPLING_NEIGHBOR;
            }
            if index + 1 < range.end {
                kernel[index * TAXEL_COUNT + index + 1] = COUPLING_NEIGHBOR;
            }
        }
    }
    kernel
}

fn physical_edges() -> Vec<SpatialEdge> {
    module_ranges()
        .into_iter()
        .flat_map(|range| {
            (range.start..range.end.saturating_sub(1)).map(|left_taxel| SpatialEdge {
                left_taxel,
                right_taxel: left_taxel + 1,
            })
        })
        .collect()
}

fn patch(
    truth: &mut [f64],
    range: &std::ops::Range<usize>,
    state: &mut u64,
    amplitude_min: f64,
    amplitude_max: f64,
    width_min: usize,
    width_max: usize,
) {
    let available = range.end - range.start;
    let width = width_min + next_random(state) as usize % (width_max - width_min + 1);
    let margin = width.min(available / 2);
    let center =
        range.start + margin + next_random(state) as usize % (available - 2 * margin).max(1);
    let amplitude = amplitude_min + random_unit(state) * (amplitude_max - amplitude_min);
    let lower = center.saturating_sub(width).max(range.start);
    let upper = (center + width + 1).min(range.end);
    for value in truth.iter_mut().take(upper).skip(lower) {
        *value = (*value).max(amplitude);
    }
}

fn make_truth(seed: u64, morphology: ContactMorphology) -> Vec<f64> {
    let mut state = seed ^ morphology.salt();
    if state == 0 {
        state = 1;
    }
    let mut truth = vec![0.0; TAXEL_COUNT];

    for range in module_ranges() {
        match morphology {
            ContactMorphology::SparsePatches => {
                if random_unit(&mut state) >= 0.30 {
                    patch(&mut truth, &range, &mut state, 0.25, 1.20, 2, 6);
                }
            }
            ContactMorphology::SmoothDistributed => {
                let phase = random_unit(&mut state) * std::f64::consts::TAU;
                let amplitude = 0.25 + random_unit(&mut state) * 0.50;
                for index in range.clone() {
                    let position =
                        (index - range.start) as f64 / (range.end - range.start).max(1) as f64;
                    truth[index] = amplitude
                        * (0.55 + 0.45 * (std::f64::consts::TAU * position + phase).sin().abs());
                }
            }
            ContactMorphology::SharpPiecewise => {
                let split_a = range.start + (range.end - range.start) / 3;
                let split_b = range.start + (range.end - range.start) * 2 / 3;
                let first = 0.10 + random_unit(&mut state) * 0.80;
                let second = 0.10 + random_unit(&mut state) * 0.80;
                let third = 0.10 + random_unit(&mut state) * 0.80;
                for index in range.clone() {
                    truth[index] = if index < split_a {
                        first
                    } else if index < split_b {
                        second
                    } else {
                        third
                    };
                }
            }
            ContactMorphology::MultipleContacts => {
                for _ in 0..3 {
                    patch(&mut truth, &range, &mut state, 0.30, 1.10, 1, 2);
                }
            }
            ContactMorphology::DenseUniform => {
                let amplitude = 0.30 + random_unit(&mut state) * 0.50;
                for index in range.clone() {
                    truth[index] = amplitude + (random_unit(&mut state) - 0.5) * 0.04;
                }
            }
            ContactMorphology::MixedSparseSmooth => {
                let base = 0.08 + random_unit(&mut state) * 0.18;
                for index in range.clone() {
                    let position =
                        (index - range.start) as f64 / (range.end - range.start).max(1) as f64;
                    truth[index] = base + 0.08 * (std::f64::consts::TAU * position).sin().abs();
                }
                if random_unit(&mut state) >= 0.25 {
                    patch(&mut truth, &range, &mut state, 0.25, 0.90, 1, 3);
                }
            }
        }
    }

    if truth.iter().all(|value| *value == 0.0) {
        truth[20] = 0.5;
    }
    truth
}

fn make_case(seed: u64, morphology: ContactMorphology) -> SyntheticFieldCase {
    let kernel = coupling_kernel();
    let evaluator_truth = make_truth(seed, morphology);
    let mut state = seed ^ morphology.salt().rotate_left(17) ^ 0x9E37_79B9_7F4A_7C15;
    if state == 0 {
        state = 1;
    }

    let mut native_response = vec![0.0; TAXEL_COUNT];
    for row in 0..TAXEL_COUNT {
        let prediction = (0..TAXEL_COUNT)
            .map(|column| kernel[row * TAXEL_COUNT + column] * evaluator_truth[column])
            .sum::<f64>();
        native_response[row] =
            prediction + (random_unit(&mut state) - 0.5) * 2.0 * RESPONSE_NOISE_SCALE;
    }

    let mut admitted_native_channels = vec![true; TAXEL_COUNT];
    for (module_index, range) in module_ranges().into_iter().enumerate() {
        let dropped = range.start + next_random(&mut state) as usize % (range.end - range.start);
        admitted_native_channels[dropped] = false;
        if module_index % 2 == 1 {
            let second = range.start + next_random(&mut state) as usize % (range.end - range.start);
            admitted_native_channels[second] = false;
        }
    }

    let mut opaque_digest_state = seed ^ morphology.salt().rotate_left(23);
    if opaque_digest_state == 0 {
        opaque_digest_state = 1;
    }
    let opaque_digest_value = next_random(&mut opaque_digest_state);

    SyntheticFieldCase {
        mechanics_problem: FieldReconstructionProblem {
            native_response: native_response.clone(),
            coupling_matrix: kernel,
            admitted_native_channels: admitted_native_channels.clone(),
            spatial_edges: physical_edges(),
            smoothness_penalty: 0.0,
            sparsity_penalty: 0.0,
            // Opaque fixture identity: the selector must not receive the
            // evaluator-only morphology family label.
            forward_model_digest: format!("fixture-model-{opaque_digest_value:016x}"),
        },
        independent_native_response: native_response,
        admitted_native_channels,
        evaluator_truth,
    }
}

fn score_baseline(case: &SyntheticFieldCase) -> Result<ReplicateScore, String> {
    let admitted_count = case
        .admitted_native_channels
        .iter()
        .filter(|admitted| **admitted)
        .count();
    if admitted_count == 0 {
        return Err("independent taxel baseline has no admitted channels".to_owned());
    }
    let squared_error = case
        .admitted_native_channels
        .iter()
        .zip(&case.independent_native_response)
        .zip(&case.evaluator_truth)
        .filter_map(|((admitted, estimate), truth)| {
            admitted.then_some((estimate - truth) * (estimate - truth))
        })
        .sum::<f64>();
    Ok(ReplicateScore {
        field_rmse: (squared_error / admitted_count as f64).sqrt(),
        support_ratio: admitted_count as f64 / TAXEL_COUNT as f64,
        native_data_rank: admitted_count,
        native_data_full_column_rank: admitted_count == TAXEL_COUNT,
        projected_gradient_residual: 0.0,
        converged: true,
    })
}

fn score_model_report(
    report: &FieldReconstructionReport,
    truth: &[f64],
) -> Result<ReplicateScore, String> {
    if report.estimates.len() != truth.len() || truth.is_empty() {
        return Err("model field score requires one estimate per truth cell".to_owned());
    }
    let mut squared_error = 0.0;
    let mut supported = 0;
    for (estimate, expected) in report.estimates.iter().zip(truth) {
        if let Some(value) = estimate.latent_contact_intensity {
            let error = value - expected;
            squared_error += error * error;
            supported += 1;
        }
    }
    if supported == 0 {
        return Err("model estimator returned no supported field cells".to_owned());
    }
    Ok(ReplicateScore {
        field_rmse: (squared_error / supported as f64).sqrt(),
        support_ratio: supported as f64 / truth.len() as f64,
        native_data_rank: report.native_data_rank,
        native_data_full_column_rank: report.native_data_full_column_rank,
        projected_gradient_residual: report.projected_gradient_residual,
        converged: report.converged,
    })
}

fn summarize(
    estimator_id: String,
    scores: &[ReplicateScore],
    baseline: &[ReplicateScore],
) -> Result<PriorEstimatorSummary, String> {
    if scores.len() < MIN_REPLICATES || scores.len() != baseline.len() {
        return Err("prior study requires at least fifty aligned paired seeds".to_owned());
    }
    let mut rmse: Vec<_> = scores.iter().map(|score| score.field_rmse).collect();
    rmse.sort_by(f64::total_cmp);
    let p95_index = ((rmse.len() - 1) as f64 * 0.95).ceil() as usize;

    let mut paired_delta: Vec<_> = scores
        .iter()
        .zip(baseline)
        .map(|(score, reference)| score.field_rmse - reference.field_rmse)
        .collect();
    paired_delta.sort_by(f64::total_cmp);
    let delta_p95 = ((paired_delta.len() - 1) as f64 * 0.95).ceil() as usize;

    let count = scores.len() as f64;
    Ok(PriorEstimatorSummary {
        estimator_id,
        mean_field_rmse: rmse.iter().sum::<f64>() / count,
        p95_field_rmse: rmse[p95_index],
        maximum_field_rmse: *rmse.last().expect("nonempty paired scores"),
        paired_rmse_delta_mean_vs_independent: paired_delta.iter().sum::<f64>() / count,
        paired_rmse_delta_p95_vs_independent: paired_delta[delta_p95],
        mean_support_ratio: scores.iter().map(|score| score.support_ratio).sum::<f64>() / count,
        minimum_support_ratio: scores
            .iter()
            .map(|score| score.support_ratio)
            .fold(f64::INFINITY, f64::min),
        mean_native_data_rank: scores
            .iter()
            .map(|score| score.native_data_rank as f64)
            .sum::<f64>()
            / count,
        full_rank_replicates: scores
            .iter()
            .filter(|score| score.native_data_full_column_rank)
            .count(),
        mean_projected_gradient_residual: scores
            .iter()
            .map(|score| score.projected_gradient_residual)
            .sum::<f64>()
            / count,
        maximum_projected_gradient_residual: scores
            .iter()
            .map(|score| score.projected_gradient_residual)
            .fold(0.0_f64, f64::max),
        converged_replicates: scores.iter().filter(|score| score.converged).count(),
        replicate_count: scores.len(),
    })
}

fn run_morphology_development(
    morphology: ContactMorphology,
    first_seed: u64,
    replicate_count: usize,
    config: &FieldReconstructionConfig,
) -> Result<MorphologyPriorReport, String> {
    let mut baseline_scores = Vec::with_capacity(replicate_count);
    let mut mechanics_scores = Vec::with_capacity(replicate_count);
    let mut graph_scores = Vec::with_capacity(replicate_count);
    let mut l1_scores: [Vec<ReplicateScore>; 3] =
        std::array::from_fn(|_| Vec::with_capacity(replicate_count));

    for offset in 0..replicate_count {
        let seed = first_seed + offset as u64;
        let case = make_case(seed, morphology);
        baseline_scores.push(score_baseline(&case)?);

        let mechanics = reconstruct_contact_field(&case.mechanics_problem, config)?;
        mechanics_scores.push(score_model_report(&mechanics, &case.evaluator_truth)?);

        let mut graph_problem = case.mechanics_problem.clone();
        graph_problem.smoothness_penalty = GRAPH_L2_PENALTY;
        graph_problem.forward_model_digest = format!(
            "{}-graph-l2-{GRAPH_L2_PENALTY:.3}",
            graph_problem.forward_model_digest
        );
        let graph = reconstruct_contact_field(&graph_problem, config)?;
        graph_scores.push(score_model_report(&graph, &case.evaluator_truth)?);

        for (index, penalty) in DEVELOPMENT_L1_PENALTIES.iter().enumerate() {
            let mut sparse_problem = case.mechanics_problem.clone();
            sparse_problem.sparsity_penalty = *penalty;
            sparse_problem.forward_model_digest =
                format!("{}-l1-{penalty:.3}", sparse_problem.forward_model_digest);
            let sparse = reconstruct_contact_field(&sparse_problem, config)?;
            l1_scores[index].push(score_model_report(&sparse, &case.evaluator_truth)?);
        }
    }

    let mut estimators = vec![
        summarize(
            "independent_taxel".to_owned(),
            &baseline_scores,
            &baseline_scores,
        )?,
        summarize(
            "mechanics_no_prior".to_owned(),
            &mechanics_scores,
            &baseline_scores,
        )?,
        summarize(
            "mechanics_graph_l2_0_02".to_owned(),
            &graph_scores,
            &baseline_scores,
        )?,
    ];
    for (index, penalty) in DEVELOPMENT_L1_PENALTIES.iter().enumerate() {
        estimators.push(summarize(
            format!("mechanics_nonnegative_l1_{penalty:.3}"),
            &l1_scores[index],
            &baseline_scores,
        )?);
    }

    Ok(MorphologyPriorReport {
        morphology: morphology.as_str(),
        seed_first: first_seed,
        replicate_count,
        estimators,
    })
}

fn run_morphology_final(
    morphology: ContactMorphology,
    first_seed: u64,
    replicate_count: usize,
    frozen_l1_penalty: Option<f64>,
    config: &FieldReconstructionConfig,
) -> Result<MorphologyPriorReport, String> {
    let mut baseline_scores = Vec::with_capacity(replicate_count);
    let mut mechanics_scores = Vec::with_capacity(replicate_count);
    let mut graph_scores = Vec::with_capacity(replicate_count);
    let mut frozen_l1_scores = Vec::with_capacity(replicate_count);

    for offset in 0..replicate_count {
        let seed = first_seed + offset as u64;
        let case = make_case(seed, morphology);
        baseline_scores.push(score_baseline(&case)?);

        let mechanics = reconstruct_contact_field(&case.mechanics_problem, config)?;
        mechanics_scores.push(score_model_report(&mechanics, &case.evaluator_truth)?);

        let mut graph_problem = case.mechanics_problem.clone();
        graph_problem.smoothness_penalty = GRAPH_L2_PENALTY;
        graph_problem.forward_model_digest = format!(
            "{}-graph-l2-{GRAPH_L2_PENALTY:.3}",
            graph_problem.forward_model_digest
        );
        let graph = reconstruct_contact_field(&graph_problem, config)?;
        graph_scores.push(score_model_report(&graph, &case.evaluator_truth)?);

        if let Some(penalty) = frozen_l1_penalty {
            let mut sparse_problem = case.mechanics_problem.clone();
            sparse_problem.sparsity_penalty = penalty;
            sparse_problem.forward_model_digest = format!(
                "{}-frozen-l1-{penalty:.3}",
                sparse_problem.forward_model_digest
            );
            let sparse = reconstruct_contact_field(&sparse_problem, config)?;
            frozen_l1_scores.push(score_model_report(&sparse, &case.evaluator_truth)?);
        }
    }

    let mut estimators = vec![
        summarize(
            "independent_taxel".to_owned(),
            &baseline_scores,
            &baseline_scores,
        )?,
        summarize(
            "mechanics_no_prior".to_owned(),
            &mechanics_scores,
            &baseline_scores,
        )?,
        summarize(
            "mechanics_graph_l2_0_02".to_owned(),
            &graph_scores,
            &baseline_scores,
        )?,
    ];
    if let Some(penalty) = frozen_l1_penalty {
        estimators.push(summarize(
            format!("mechanics_frozen_l1_{penalty:.3}"),
            &frozen_l1_scores,
            &baseline_scores,
        )?);
    }

    Ok(MorphologyPriorReport {
        morphology: morphology.as_str(),
        seed_first: first_seed,
        replicate_count,
        estimators,
    })
}

/// Run a development penalty comparison and a fresh held-out evaluation.
/// The chosen L1 penalty minimizes the equal-weight mean per-morphology
/// development RMSE. Final test seeds are validated as disjoint before any
/// final evaluation is performed.
pub fn run_prior_study(
    development_seed_first: u64,
    final_seed_first: u64,
    replicate_count: usize,
) -> Result<PriorStudyReport, String> {
    if development_seed_first == 0 || final_seed_first == 0 {
        return Err("development and final seed namespaces must be nonzero".to_owned());
    }
    if replicate_count < MIN_REPLICATES {
        return Err("prior study requires at least fifty seeds per morphology".to_owned());
    }
    let final_offset =
        u64::try_from(replicate_count - 1).map_err(|_| "replicate count exceeds u64".to_owned())?;
    let development_last = development_seed_first
        .checked_add(final_offset)
        .ok_or_else(|| "development seed range overflows u64".to_owned())?;
    let final_last = final_seed_first
        .checked_add(final_offset)
        .ok_or_else(|| "final seed range overflows u64".to_owned())?;
    if !(development_last < final_seed_first || final_last < development_seed_first) {
        return Err("development and final seed namespaces overlap".to_owned());
    }

    let config = FieldReconstructionConfig {
        maximum_iterations: 10_000,
        convergence_tolerance: 1.0e-6,
        sensitivity_epsilon: 1.0e-12,
    };

    let mut development = Vec::with_capacity(ContactMorphology::ALL.len());
    for morphology in ContactMorphology::ALL {
        development.push(run_morphology_development(
            morphology,
            development_seed_first,
            replicate_count,
            &config,
        )?);
    }

    let mut candidate_summaries = Vec::with_capacity(DEVELOPMENT_L1_PENALTIES.len());
    for penalty in DEVELOPMENT_L1_PENALTIES {
        let per_morphology: Vec<_> = development
            .iter()
            .map(|morphology| {
                morphology
                    .estimators
                    .iter()
                    .find(|summary| {
                        summary.estimator_id == format!("mechanics_nonnegative_l1_{penalty:.3}")
                    })
                    .map(|summary| summary.mean_field_rmse)
                    .ok_or_else(|| "development campaign is missing an L1 candidate".to_owned())
            })
            .collect::<Result<_, _>>()?;
        let baseline_per_morphology: Vec<_> = development
            .iter()
            .map(|morphology| {
                morphology
                    .estimators
                    .iter()
                    .find(|summary| summary.estimator_id == "independent_taxel")
                    .map(|summary| summary.mean_field_rmse)
                    .ok_or_else(|| {
                        "development campaign is missing the independent baseline".to_owned()
                    })
            })
            .collect::<Result<_, _>>()?;

        let count = development.len() as f64;
        let development_mean_delta_vs_independent = per_morphology
            .iter()
            .zip(&baseline_per_morphology)
            .map(|(candidate, baseline)| candidate - baseline)
            .sum::<f64>()
            / count;
        let maximum_family_relative_regression = per_morphology
            .iter()
            .zip(&baseline_per_morphology)
            .map(|(candidate, baseline)| (candidate - baseline) / baseline)
            .fold(f64::NEG_INFINITY, f64::max);
        candidate_summaries.push(L1PenaltyCandidateSummary {
            penalty,
            development_mean_field_rmse: per_morphology.iter().sum::<f64>() / count,
            development_mean_delta_vs_independent,
            maximum_family_relative_regression,
            eligible_for_promotion: development_mean_delta_vs_independent <= 0.0
                && maximum_family_relative_regression <= MAXIMUM_ALLOWED_FAMILY_REGRESSION,
        });
    }

    let selected_l1_penalty = select_l1_penalty(&candidate_summaries);
    let l1_selection_disposition = if selected_l1_penalty.is_some() {
        "accepted_development_noninferiority_and_worst_family_guard"
    } else {
        "none_meets_development_noninferiority_and_worst_family_guard"
    };

    let mut final_test = Vec::with_capacity(ContactMorphology::ALL.len());
    for morphology in ContactMorphology::ALL {
        final_test.push(run_morphology_final(
            morphology,
            final_seed_first,
            replicate_count,
            selected_l1_penalty,
            &config,
        )?);
    }

    Ok(PriorStudyReport {
        development_seed_first,
        final_seed_first,
        replicate_count_per_morphology: replicate_count,
        development,
        development_l1_candidates: candidate_summaries,
        selected_l1_penalty,
        l1_selection_disposition,
        final_test,
        claim_class: "development_selected_prior_on_disjoint_synthetic_holdout",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_morphology_generators_are_deterministic_and_nonnegative() {
        for morphology in ContactMorphology::ALL {
            let first = make_case(20261117, morphology);
            let second = make_case(20261117, morphology);
            assert_eq!(first, second);
            assert_eq!(first.evaluator_truth.len(), TAXEL_COUNT);
            assert!(
                first
                    .evaluator_truth
                    .iter()
                    .all(|value| value.is_finite() && *value >= 0.0)
            );
            assert_eq!(first.mechanics_problem.native_response.len(), TAXEL_COUNT);
            assert_eq!(
                first.mechanics_problem.coupling_matrix.len(),
                TAXEL_COUNT * TAXEL_COUNT
            );
        }
    }

    #[test]
    fn morphology_names_are_unique_and_module_geometry_is_exact() {
        let names: Vec<_> = ContactMorphology::ALL
            .iter()
            .map(|family| family.as_str())
            .collect();
        let unique: std::collections::BTreeSet<_> = names.iter().copied().collect();
        assert_eq!(unique.len(), ContactMorphology::ALL.len());
        assert_eq!(MODULE_SIZES.iter().sum::<usize>(), TAXEL_COUNT);
        assert_eq!(module_ranges().len(), MODULE_COUNT);
    }

    #[test]
    fn l1_prior_is_not_promoted_when_all_development_candidates_regress() {
        let candidates = vec![
            L1PenaltyCandidateSummary {
                penalty: 0.001,
                development_mean_field_rmse: 0.12,
                development_mean_delta_vs_independent: 0.08,
                maximum_family_relative_regression: 0.30,
                eligible_for_promotion: false,
            },
            L1PenaltyCandidateSummary {
                penalty: 0.01,
                development_mean_field_rmse: 0.11,
                development_mean_delta_vs_independent: 0.07,
                maximum_family_relative_regression: 0.20,
                eligible_for_promotion: false,
            },
        ];
        assert_eq!(select_l1_penalty(&candidates), None);
    }

    #[test]
    fn l1_prior_requires_mean_noninferiority_and_worst_family_guard() {
        let candidates = vec![
            L1PenaltyCandidateSummary {
                penalty: 0.001,
                development_mean_field_rmse: 0.10,
                development_mean_delta_vs_independent: -0.001,
                maximum_family_relative_regression: 0.021,
                eligible_for_promotion: false,
            },
            L1PenaltyCandidateSummary {
                penalty: 0.003,
                development_mean_field_rmse: 0.11,
                development_mean_delta_vs_independent: 0.001,
                maximum_family_relative_regression: 0.01,
                eligible_for_promotion: false,
            },
            L1PenaltyCandidateSummary {
                penalty: 0.01,
                development_mean_field_rmse: 0.12,
                development_mean_delta_vs_independent: -0.002,
                maximum_family_relative_regression: 0.02,
                eligible_for_promotion: true,
            },
        ];
        assert_eq!(select_l1_penalty(&candidates), Some(0.01));
    }

    #[test]
    fn development_and_final_seed_ranges_must_be_disjoint() {
        assert!(run_prior_study(20261117, 20261150, 50).is_err());
        assert!(run_prior_study(0, 20261217, 50).is_err());
        assert!(run_prior_study(20261117, 20261217, 49).is_err());
    }
}
