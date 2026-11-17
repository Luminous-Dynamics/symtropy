//! Paired synthetic 113-taxel / seven-module field-reconstruction campaign.
//!
//! The campaign compares:
//! - independent taxel inversion (identity forward model);
//! - mechanics-kernel inversion (known within-module coupling);
//! - mechanics-kernel inversion with graph regularization over within-module
//!   physical adjacency edges.
//!
//! Each replicate uses the same generated latent field, coupled native
//! responses, noise, and dropped-channel mask for every estimator. Metrics
//! report field error and support coverage together. An estimator cannot
//! improve its score merely by returning fewer cells.
//!
//! All outputs are arbitrary-unit synthetic results. The coupling kernel is
//! supplied by the generator, not identified from physical hardware.

#![forbid(unsafe_code)]

use super::field_reconstruction::{
    FieldReconstructionConfig, FieldReconstructionProblem, FieldReconstructionReport,
    FieldCellSupport, SpatialEdge, reconstruct_contact_field,
};

const TAXEL_COUNT: usize = 113;
const MODULE_COUNT: usize = 7;
const MODULE_SIZES: [usize; MODULE_COUNT] = [16, 16, 16, 16, 16, 16, 17];
const REQUIRED_REPLICATES: usize = 50;
const COUPLING_DIAGONAL: f64 = 0.76;
const COUPLING_NEIGHBOR: f64 = 0.12;
const RESPONSE_NOISE_SCALE: f64 = 0.0015;
const GRAPH_SMOOTHNESS_PENALTY: f64 = 0.02;

#[derive(Clone, Debug, PartialEq)]
pub struct FieldEstimatorSummary {
    pub estimator_id: &'static str,
    pub mean_field_rmse: f64,
    pub p95_field_rmse: f64,
    pub maximum_field_rmse: f64,
    /// Paired field-RMSE delta vs independent per-taxel inversion (negative is better).
    pub paired_rmse_delta_mean_vs_independent: f64,
    pub paired_rmse_delta_p95_vs_independent: f64,
    /// Fraction of field cells with some admitted native-channel sensitivity.
    /// This is not a unique-identifiability or measurement-coverage claim.
    pub mean_sensitivity_support_ratio: f64,
    pub minimum_sensitivity_support_ratio: f64,
    pub mean_unsupported_cells: f64,
    /// Mean numerical rank of the native data matrix before any graph prior.
    pub mean_native_data_rank: f64,
    pub mean_projected_gradient_residual: f64,
    pub maximum_projected_gradient_residual: f64,
    /// Replicates whose native data matrix identifies every latent field cell.
    pub full_rank_replicates: usize,
    pub converged_replicates: usize,
    pub replicate_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldReconstructionCampaignReport {
    pub taxel_count: usize,
    pub module_count: usize,
    pub module_sizes: Vec<usize>,
    pub first_seed: u64,
    pub replicate_count: usize,
    pub results: Vec<FieldEstimatorSummary>,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
struct SyntheticFieldCase {
    problem: FieldReconstructionProblem,
    evaluator_truth: Vec<f64>,
    independent_taxel_problem: FieldReconstructionProblem,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ReplicateScore {
    field_rmse: f64,
    coverage_ratio: f64,
    unsupported_cells: usize,
    native_data_rank: usize,
    native_data_full_column_rank: bool,
    projected_gradient_residual: f64,
    converged: bool,
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

fn make_coupling_kernel() -> Vec<f64> {
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

fn make_spatial_edges() -> Vec<SpatialEdge> {
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

fn make_truth(seed: u64) -> Vec<f64> {
    let mut state = seed ^ 0xA076_1D64_78BD_642F;
    if state == 0 {
        state = 1;
    }
    let mut truth = vec![0.0; TAXEL_COUNT];

    // Generate sparse contact patches with abrupt boundaries. Varying the
    // number, width and amplitude exposes the smoothing-vs-resolution tradeoff.
    for range in module_ranges() {
        if random_unit(&mut state) < 0.30 {
            continue;
        }
        let width = 2 + (next_random(&mut state) as usize % 5);
        let available = range.end - range.start;
        let margin = width.min(available / 2);
        let center = range.start + margin
            + (next_random(&mut state) as usize % (available - 2 * margin).max(1));
        let amplitude = 0.25 + random_unit(&mut state) * 0.95;
        let lower = center.saturating_sub(width).max(range.start);
        let upper = (center + width + 1).min(range.end);
        for (index, value) in truth.iter_mut().enumerate().take(upper).skip(lower) {
            *value = amplitude;
            // Add a small within-patch slope without erasing the sharp edge.
            if index < center {
                *value *= 0.85 + 0.15 * (index - lower) as f64 / width.max(1) as f64;
            }
        }
    }

    // Ensure every generated field has at least one nonzero region, including
    // the final 17-taxel module, which differs in size from the other modules.
    if truth.iter().all(|value| *value == 0.0) {
        truth[MODULE_SIZES[0] + 4] = 0.75;
    }
    truth
}

fn make_case(seed: u64) -> SyntheticFieldCase {
    let kernel = make_coupling_kernel();
    let evaluator_truth = make_truth(seed);
    let mut state = seed ^ 0xE703_7ED1_A0B4_28DB;
    if state == 0 {
        state = 1;
    }

    let mut native_response = vec![0.0; TAXEL_COUNT];
    for row in 0..TAXEL_COUNT {
        let predicted = (0..TAXEL_COUNT)
            .map(|column| kernel[row * TAXEL_COUNT + column] * evaluator_truth[column])
            .sum::<f64>();
        let noise = (random_unit(&mut state) - 0.5) * 2.0 * RESPONSE_NOISE_SCALE;
        native_response[row] = predicted + noise;
    }

    // One channel outage in every module. A missing channel remains missing;
    // neighboring response is allowed to support a model-derived estimate
    // only when the declared forward kernel has sensitivity to that cell.
    let mut admitted_native_channels = vec![true; TAXEL_COUNT];
    for (module_index, range) in module_ranges().into_iter().enumerate() {
        let dropped = range.start
            + (next_random(&mut state) as usize % (range.end - range.start));
        admitted_native_channels[dropped] = false;
        // Deterministic seed-dependent second outage in alternating modules.
        if module_index % 2 == 1 && range.len() > 3 {
            let second = range.start
                + (next_random(&mut state) as usize % (range.end - range.start));
            admitted_native_channels[second] = false;
        }
    }

    let forward_model_digest =
        format!("sha256:synthetic-113-taxel-kernel-seed-{seed:016x}");
    let problem = FieldReconstructionProblem {
        native_response: native_response.clone(),
        coupling_matrix: kernel,
        admitted_native_channels: admitted_native_channels.clone(),
        spatial_edges: make_spatial_edges(),
        smoothness_penalty: 0.0,
        forward_model_digest: forward_model_digest.clone(),
    };

    let mut identity_kernel = vec![0.0; TAXEL_COUNT * TAXEL_COUNT];
    for index in 0..TAXEL_COUNT {
        identity_kernel[index * TAXEL_COUNT + index] = 1.0;
    }
    let independent_taxel_problem = FieldReconstructionProblem {
        native_response,
        coupling_matrix: identity_kernel,
        admitted_native_channels,
        spatial_edges: Vec::new(),
        smoothness_penalty: 0.0,
        forward_model_digest: format!("sha256:independent-taxel-identity-seed-{seed:016x}"),
    };

    SyntheticFieldCase {
        problem,
        evaluator_truth,
        independent_taxel_problem,
    }
}

fn score_report(
    report: &FieldReconstructionReport,
    evaluator_truth: &[f64],
) -> Result<ReplicateScore, String> {
    if report.estimates.len() != evaluator_truth.len() || evaluator_truth.is_empty() {
        return Err("field scorecard requires one estimate per held-out truth cell".to_owned());
    }

    let mut squared_error = 0.0;
    let mut supported_count = 0;
    for (estimate, truth) in report.estimates.iter().zip(evaluator_truth) {
        if let Some(value) = estimate.latent_contact_intensity {
            let error = value - truth;
            squared_error += error * error;
            supported_count += 1;
        }
    }
    if supported_count == 0 {
        return Err("field estimator returned no observable support".to_owned());
    }

    Ok(ReplicateScore {
        field_rmse: (squared_error / supported_count as f64).sqrt(),
        coverage_ratio: supported_count as f64 / evaluator_truth.len() as f64,
        unsupported_cells: report.unsupported_cell_count,
        native_data_rank: report.native_data_rank,
        native_data_full_column_rank: report.native_data_full_column_rank,
        projected_gradient_residual: report.projected_gradient_residual,
        converged: report.converged,
    })
}

fn score_independent_taxels(case: &SyntheticFieldCase) -> Result<ReplicateScore, String> {
    let problem = &case.independent_taxel_problem;
    if problem.native_response.len() != case.evaluator_truth.len()
        || problem.admitted_native_channels.len() != case.evaluator_truth.len()
    {
        return Err("independent-taxel baseline requires matching native response, mask and truth".to_owned());
    }

    let mut squared_error = 0.0;
    let mut admitted_count = 0;
    for index in 0..case.evaluator_truth.len() {
        if problem.admitted_native_channels[index] {
            let error = problem.native_response[index] - case.evaluator_truth[index];
            squared_error += error * error;
            admitted_count += 1;
        }
    }
    if admitted_count == 0 {
        return Err("independent-taxel baseline has no admitted native channels".to_owned());
    }

    // The baseline is the closed-form identity forward model: each admitted
    // channel estimates only its own field cell. Its numerical data rank is
    // exactly the number of admitted rows; unobserved cells remain Unknown.
    Ok(ReplicateScore {
        field_rmse: (squared_error / admitted_count as f64).sqrt(),
        coverage_ratio: admitted_count as f64 / case.evaluator_truth.len() as f64,
        unsupported_cells: case.evaluator_truth.len() - admitted_count,
        native_data_rank: admitted_count,
        native_data_full_column_rank: admitted_count == case.evaluator_truth.len(),
        projected_gradient_residual: 0.0,
        converged: true,
    })
}

fn summarize_scores(
    estimator_id: &'static str,
    scores: &[ReplicateScore],
    independent_baseline: &[ReplicateScore],
) -> Result<FieldEstimatorSummary, String> {
    if scores.len() < REQUIRED_REPLICATES || scores.len() != independent_baseline.len() {
        return Err("field campaign summary requires at least fifty aligned paired replicates".to_owned());
    }

    let mut errors: Vec<_> = scores.iter().map(|score| score.field_rmse).collect();
    errors.sort_by(f64::total_cmp);
    let mean_field_rmse = errors.iter().sum::<f64>() / errors.len() as f64;
    let p95_index = ((errors.len() - 1) as f64 * 0.95).ceil() as usize;
    let maximum_field_rmse = errors.last().copied().unwrap_or(f64::NAN);
    let mut paired_deltas: Vec<_> = scores
        .iter()
        .zip(independent_baseline)
        .map(|(score, baseline)| score.field_rmse - baseline.field_rmse)
        .collect();
    paired_deltas.sort_by(f64::total_cmp);
    let paired_rmse_delta_mean_vs_independent =
        paired_deltas.iter().sum::<f64>() / paired_deltas.len() as f64;
    let paired_rmse_delta_p95_vs_independent = paired_deltas
        [((paired_deltas.len() - 1) as f64 * 0.95).ceil() as usize];

    let mean_sensitivity_support_ratio = scores
        .iter()
        .map(|score| score.coverage_ratio)
        .sum::<f64>()
        / scores.len() as f64;
    let minimum_sensitivity_support_ratio = scores
        .iter()
        .map(|score| score.coverage_ratio)
        .fold(f64::INFINITY, f64::min);
    let mean_unsupported_cells = scores
        .iter()
        .map(|score| score.unsupported_cells as f64)
        .sum::<f64>()
        / scores.len() as f64;
    let mean_native_data_rank = scores
        .iter()
        .map(|score| score.native_data_rank as f64)
        .sum::<f64>()
        / scores.len() as f64;
    let mean_projected_gradient_residual = scores
        .iter()
        .map(|score| score.projected_gradient_residual)
        .sum::<f64>()
        / scores.len() as f64;
    let maximum_projected_gradient_residual = scores
        .iter()
        .map(|score| score.projected_gradient_residual)
        .fold(0.0_f64, f64::max);
    let full_rank_replicates = scores
        .iter()
        .filter(|score| score.native_data_full_column_rank)
        .count();
    let converged_replicates = scores.iter().filter(|score| score.converged).count();

    Ok(FieldEstimatorSummary {
        estimator_id,
        mean_field_rmse,
        p95_field_rmse: errors[p95_index],
        maximum_field_rmse,
        paired_rmse_delta_mean_vs_independent,
        paired_rmse_delta_p95_vs_independent,
        mean_sensitivity_support_ratio,
        minimum_sensitivity_support_ratio,
        mean_unsupported_cells,
        mean_native_data_rank,
        mean_projected_gradient_residual,
        maximum_projected_gradient_residual,
        full_rank_replicates,
        converged_replicates,
        replicate_count: scores.len(),
    })
}

/// Run a paired 113-taxel field-reconstruction campaign. All estimator arms
/// consume identical observations, row masks and evaluator truth for each
/// seed. The evaluator-only truth is not passed to the inverse solver.
pub fn run_field_reconstruction_campaign(
    first_seed: u64,
    replicate_count: usize,
) -> Result<FieldReconstructionCampaignReport, String> {
    if first_seed == 0 {
        return Err("field campaign first seed must be nonzero".to_owned());
    }
    if replicate_count < REQUIRED_REPLICATES {
        return Err("field campaign requires at least fifty paired replicates".to_owned());
    }
    let last_offset = u64::try_from(replicate_count - 1)
        .map_err(|_| "field campaign replicate count exceeds u64".to_owned())?;
    first_seed
        .checked_add(last_offset)
        .ok_or_else(|| "field campaign seed range overflows u64".to_owned())?;

    let config = FieldReconstructionConfig {
        maximum_iterations: 10_000,
        convergence_tolerance: 1.0e-6,
        sensitivity_epsilon: 1.0e-12,
    };

    let mut independent_scores = Vec::with_capacity(replicate_count);
    let mut mechanics_scores = Vec::with_capacity(replicate_count);
    let mut graph_scores = Vec::with_capacity(replicate_count);

    for replicate in 0..replicate_count {
        let seed = first_seed + replicate as u64;
        let case = make_case(seed);

        independent_scores.push(score_independent_taxels(&case)?);

        let mechanics_report = reconstruct_contact_field(&case.problem, &config)?;
        mechanics_scores.push(score_report(&mechanics_report, &case.evaluator_truth)?);

        let mut graph_problem = case.problem.clone();
        graph_problem.smoothness_penalty = GRAPH_SMOOTHNESS_PENALTY;
        graph_problem.forward_model_digest =
            format!("{}-graph-lambda-{GRAPH_SMOOTHNESS_PENALTY:.3}", graph_problem.forward_model_digest);
        let graph_report = reconstruct_contact_field(&graph_problem, &config)?;
        graph_scores.push(score_report(&graph_report, &case.evaluator_truth)?);
    }

    Ok(FieldReconstructionCampaignReport {
        taxel_count: TAXEL_COUNT,
        module_count: MODULE_COUNT,
        module_sizes: MODULE_SIZES.to_vec(),
        first_seed,
        replicate_count,
        results: vec![
            summarize_scores(
                "independent_taxel_inversion",
                &independent_scores,
                &independent_scores,
            )?,
            summarize_scores(
                "mechanics_kernel_no_smoothing",
                &mechanics_scores,
                &independent_scores,
            )?,
            summarize_scores(
                "mechanics_kernel_graph_regularized",
                &graph_scores,
                &independent_scores,
            )?,
        ],
        claim_class: "paired_synthetic_113_taxel_field_campaign_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_partition_is_exactly_113_taxels_across_seven_modules() {
        assert_eq!(MODULE_SIZES.iter().sum::<usize>(), TAXEL_COUNT);
        assert_eq!(module_ranges().len(), MODULE_COUNT);
        assert_eq!(module_ranges().last().expect("last module").end, TAXEL_COUNT);
        assert_eq!(make_spatial_edges().len(), TAXEL_COUNT - MODULE_COUNT);
    }

    #[test]
    fn campaign_requires_fifty_replicates_and_a_nonzero_seed() {
        assert!(run_field_reconstruction_campaign(1, 49).is_err());
        assert!(run_field_reconstruction_campaign(0, 50).is_err());
        assert!(run_field_reconstruction_campaign(u64::MAX, 50).is_err());
    }

    #[test]
    fn generated_field_has_module_local_coupling_and_no_cross_seam_edges() {
        let kernel = make_coupling_kernel();
        let ranges = module_ranges();
        for (left, right) in ranges.windows(2).map(|pair| (pair[0].end - 1, pair[1].start)) {
            assert_eq!(kernel[left * TAXEL_COUNT + right], 0.0);
            assert_eq!(kernel[right * TAXEL_COUNT + left], 0.0);
        }
        for edge in make_spatial_edges() {
            let same_module = ranges.iter().any(|range| {
                edge.left_taxel >= range.start
                    && edge.left_taxel < range.end
                    && edge.right_taxel >= range.start
                    && edge.right_taxel < range.end
            });
            assert!(same_module);
        }
    }

    #[test]
    fn fifty_seed_mechanics_campaign_is_deterministic_and_reports_coverage() {
        // Generator determinism is tested directly; do not execute a full
        // 50-seed optimization twice inside the ordinary unit-test suite.
        let generated_first = make_case(20261117);
        let generated_second = make_case(20261117);
        assert_eq!(generated_first, generated_second);
        let first = run_field_reconstruction_campaign(20261117, 50).expect("campaign");
        assert_eq!(first.taxel_count, 113);
        assert_eq!(first.module_count, 7);
        assert_eq!(first.replicate_count, 50);
        assert_eq!(first.results.len(), 3);
        for result in &first.results {
            assert_eq!(result.replicate_count, 50);
            assert_eq!(result.converged_replicates, 50);
            assert!(result.mean_field_rmse.is_finite());
            assert!(result.mean_sensitivity_support_ratio > 0.0);
            assert!(result.mean_sensitivity_support_ratio <= 1.0);
            assert!(result.mean_native_data_rank <= 113.0);
            assert!(result.mean_projected_gradient_residual.is_finite());
            assert!(result.maximum_projected_gradient_residual.is_finite());
        }
        let independent = &first.results[0];
        let mechanics = &first.results[1];
        assert!(mechanics.paired_rmse_delta_mean_vs_independent.is_finite());
        assert!(mechanics.paired_rmse_delta_p95_vs_independent.is_finite());
        // Do not assume the supplied mechanical inverse beats independent
        // inversion on a sparse, sharp-boundary field. The paired scorecard
        // exists to quantify that trade-off without changing evaluator truth.
        assert!(
            mechanics.mean_sensitivity_support_ratio
                > independent.mean_sensitivity_support_ratio,
            "mechanics coupling should provide sensitivity support to some dropped cells"
        );
        assert_eq!(mechanics.full_rank_replicates, 0);
        assert_eq!(independent.full_rank_replicates, 0);
        assert!(
            mechanics.mean_native_data_rank < 113.0,
            "dropped native channels must remain visible as rank deficiency"
        );
    }
}
