//! Reproducible correlated Gaussian Latin-hypercube input designs.
//!
//! This module generates input samples only. It does not evaluate a CFD model, estimate output
//! uncertainty, or convert a sample design into validation evidence. Marginal normal scores are
//! stratified before a Cholesky correlation transform; finite-sample marginal and Pearson
//! correlation diagnostics are reported rather than claiming exact realization of the target R.

use std::collections::HashSet;

const MAX_SAMPLE_COUNT: usize = 100_000;
const MAX_DIMENSIONS: usize = 128;
const CHOLESKY_PIVOT_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq)]
pub struct NormalInputMarginal {
    pub input_id: String,
    pub input_unit: String,
    pub source_reference: String,
    pub mean: f64,
    /// Standard deviation of the declared normal marginal.
    pub standard_uncertainty: f64,
    /// Hard engineering domain bounds. Samples outside these bounds are rejected, not clipped.
    pub minimum_allowed: f64,
    pub maximum_allowed: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LatinHypercubeStudy {
    pub study_id: String,
    pub seed: u64,
    pub sample_count: usize,
    pub marginals: Vec<NormalInputMarginal>,
    /// Target latent Gaussian-score correlation matrix used for rank reordering.
    /// The achieved physical-input Pearson correlation is measured and reported separately.
    pub correlation_matrix: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LatinHypercubeSample {
    pub sample_id: usize,
    pub input_index: usize,
    /// Uniform LHS probability in (0,1), stratified before correlation.
    pub stratified_probability: f64,
    /// Independent standard-normal score obtained from the stratified probability.
    pub independent_normal_score: f64,
    /// Cholesky-correlated standard-normal score.
    pub correlated_normal_score: f64,
    /// Physical input value: mean + standard_uncertainty * correlated score.
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LatinHypercubeDesign {
    pub study_id: String,
    pub seed: u64,
    pub sample_count: usize,
    pub marginals: Vec<NormalInputMarginal>,
    pub target_correlation_matrix: Vec<Vec<f64>>,
    /// Empirical correlation of the Cholesky-correlated latent Gaussian scores.
    pub empirical_correlation_matrix: Vec<Vec<f64>>,
    /// Empirical Pearson correlation of the final physical input values.
    pub empirical_output_correlation_matrix: Vec<Vec<f64>>,
    pub samples: Vec<LatinHypercubeSample>,
    pub evidence_level: &'static str,
}

pub fn generate_correlated_lhs(study: &LatinHypercubeStudy) -> Result<LatinHypercubeDesign, String> {
    if study.study_id.trim().is_empty()
        || study.study_id.contains(',')
        || study.study_id.contains('"')
        || study.study_id.contains('\n')
        || study.study_id.contains('\r')
    {
        return Err("study_id must be non-empty and CSV-safe".to_owned());
    }
    if study.sample_count < 4 || study.sample_count > MAX_SAMPLE_COUNT {
        return Err(format!(
            "sample_count must be between 4 and {MAX_SAMPLE_COUNT}"
        ));
    }
    let dimensions = study.marginals.len();
    if dimensions == 0 || dimensions > MAX_DIMENSIONS {
        return Err(format!("marginal count must be between 1 and {MAX_DIMENSIONS}"));
    }
    validate_correlation_matrix(&study.correlation_matrix, dimensions)?;
    let lower = cholesky_lower(&study.correlation_matrix)?;

    let mut ids = HashSet::new();
    for (index, marginal) in study.marginals.iter().enumerate() {
        if marginal.input_id.trim().is_empty()
            || marginal.input_id.contains(',')
            || marginal.input_id.contains('"')
            || marginal.input_id.contains('|')
            || marginal.input_id.contains('\n')
            || marginal.input_unit.trim().is_empty()
            || marginal.input_unit.contains(',')
            || marginal.input_unit.contains('"')
            || marginal.source_reference.trim().is_empty()
            || marginal.source_reference.contains(',')
            || marginal.source_reference.contains('"')
            || marginal.source_reference.contains('|')
            || marginal.source_reference.contains('\n')
            || !ids.insert(marginal.input_id.clone())
        {
            return Err(format!(
                "marginal {index}: ID must be unique and all metadata must be non-empty and CSV-safe"
            ));
        }
        for (name, value) in [
            ("mean", marginal.mean),
            ("standard_uncertainty", marginal.standard_uncertainty),
            ("minimum_allowed", marginal.minimum_allowed),
            ("maximum_allowed", marginal.maximum_allowed),
        ] {
            if !value.is_finite() {
                return Err(format!("input {}: {name} must be finite", marginal.input_id));
            }
        }
        if marginal.standard_uncertainty <= 0.0
            || marginal.minimum_allowed >= marginal.maximum_allowed
            || marginal.mean < marginal.minimum_allowed
            || marginal.mean > marginal.maximum_allowed
        {
            return Err(format!(
                "input {}: standard uncertainty must be positive and mean must lie inside ordered physical bounds",
                marginal.input_id
            ));
        }
    }

    let mut rng = SplitMix64::new(study.seed);
    let mut probabilities = vec![vec![0.0; dimensions]; study.sample_count];
    let mut independent_scores = vec![vec![0.0; dimensions]; study.sample_count];

    for dimension in 0..dimensions {
        let mut permutation = (0..study.sample_count).collect::<Vec<_>>();
        for index in (1..study.sample_count).rev() {
            let selected = (rng.next_u64() % ((index + 1) as u64)) as usize;
            permutation.swap(index, selected);
        }
        for sample_id in 0..study.sample_count {
            let probability =
                (permutation[sample_id] as f64 + rng.next_open_unit()) / study.sample_count as f64;
            probabilities[sample_id][dimension] = probability;
            independent_scores[sample_id][dimension] = inverse_standard_normal_cdf(probability)?;
        }
    }

    // Draw an independent normal score matrix used only to create the target correlated
    // ordering. It is separate from the stratified marginal values.
    let mut latent_independent_scores = vec![vec![0.0; dimensions]; study.sample_count];
    for sample in 0..study.sample_count {
        for dimension in 0..dimensions {
            latent_independent_scores[sample][dimension] =
                inverse_standard_normal_cdf(rng.next_open_unit())?;
        }
    }

    let mut correlated_scores = vec![vec![0.0; dimensions]; study.sample_count];
    for sample in 0..study.sample_count {
        for dimension in 0..dimensions {
            let value = (0..=dimension)
                .map(|column| lower[dimension][column] * latent_independent_scores[sample][column])
                .sum::<f64>();
            if !value.is_finite() {
                return Err(format!(
                    "correlated normal score overflowed at sample {sample}, input {dimension}"
                ));
            }
            correlated_scores[sample][dimension] = value;
        }
    }

    // Iman-Conover rank reordering: within each marginal, map the sorted LHS values to
    // sample IDs ordered by their correlated normal scores. This preserves every marginal
    // probability stratum exactly while inducing the requested rank dependence.
    let mut assigned_probabilities = vec![vec![0.0; dimensions]; study.sample_count];
    let mut assigned_independent_scores = vec![vec![0.0; dimensions]; study.sample_count];
    for dimension in 0..dimensions {
        let mut lhs_order = (0..study.sample_count).collect::<Vec<_>>();
        lhs_order.sort_by(|left, right| {
            probabilities[*left][dimension].total_cmp(&probabilities[*right][dimension])
        });
        let mut correlated_order = (0..study.sample_count).collect::<Vec<_>>();
        correlated_order.sort_by(|left, right| {
            correlated_scores[*left][dimension].total_cmp(&correlated_scores[*right][dimension])
        });
        for rank in 0..study.sample_count {
            let destination = correlated_order[rank];
            let source = lhs_order[rank];
            assigned_probabilities[destination][dimension] = probabilities[source][dimension];
            assigned_independent_scores[destination][dimension] =
                independent_scores[source][dimension];
        }
    }

    let mut samples = Vec::with_capacity(study.sample_count * dimensions);
    let mut physical_values = vec![vec![0.0; dimensions]; study.sample_count];
    for sample_id in 0..study.sample_count {
        for dimension in 0..dimensions {
            let marginal = &study.marginals[dimension];
            let physical_value = marginal.mean
                + marginal.standard_uncertainty * assigned_independent_scores[sample_id][dimension];
            if !physical_value.is_finite() {
                return Err(format!(
                    "physical input overflowed at sample {sample_id}, input {}",
                    marginal.input_id
                ));
            }
            if physical_value < marginal.minimum_allowed
                || physical_value > marginal.maximum_allowed
            {
                return Err(format!(
                    "sample {sample_id} input {}={} falls outside declared bounds [{}, {}]; no clipping or silent truncation is applied",
                    marginal.input_id,
                    physical_value,
                    marginal.minimum_allowed,
                    marginal.maximum_allowed
                ));
            }
            physical_values[sample_id][dimension] = physical_value;
            samples.push(LatinHypercubeSample {
                sample_id,
                input_index: dimension,
                stratified_probability: assigned_probabilities[sample_id][dimension],
                independent_normal_score: assigned_independent_scores[sample_id][dimension],
                correlated_normal_score: correlated_scores[sample_id][dimension],
                value: physical_value,
            });
        }
    }

    let empirical_correlation_matrix = empirical_correlation(&correlated_scores)?;
    let empirical_output_correlation_matrix = empirical_correlation(&physical_values)?;
    Ok(LatinHypercubeDesign {
        study_id: study.study_id.clone(),
        seed: study.seed,
        sample_count: study.sample_count,
        marginals: study.marginals.clone(),
        target_correlation_matrix: study.correlation_matrix.clone(),
        empirical_correlation_matrix,
        empirical_output_correlation_matrix,
        samples,
        evidence_level: "seeded_correlated_lhs_input_design_only_no_solver_evaluations_no_output_uq",
    })
}

fn validate_correlation_matrix(matrix: &[Vec<f64>], dimensions: usize) -> Result<(), String> {
    if matrix.len() != dimensions || matrix.iter().any(|row| row.len() != dimensions) {
        return Err("correlation matrix dimensions must match marginal count".to_owned());
    }
    for row in 0..dimensions {
        for column in 0..dimensions {
            let value = matrix[row][column];
            if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                return Err("correlation matrix entries must be finite and in [-1,1]".to_owned());
            }
            if (value - matrix[column][row]).abs() > 1.0e-10 {
                return Err("correlation matrix must be symmetric".to_owned());
            }
        }
        if (matrix[row][row] - 1.0).abs() > 1.0e-10 {
            return Err("correlation matrix diagonal entries must equal one".to_owned());
        }
    }
    Ok(())
}

fn cholesky_lower(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let dimensions = matrix.len();
    let mut lower = vec![vec![0.0; dimensions]; dimensions];
    for row in 0..dimensions {
        for column in 0..=row {
            let mut sum = matrix[row][column];
            for index in 0..column {
                sum -= lower[row][index] * lower[column][index];
            }
            if row == column {
                if !sum.is_finite() || sum <= CHOLESKY_PIVOT_TOLERANCE {
                    return Err(format!(
                        "correlation matrix must be positive definite for Cholesky sampling; pivot {row} is {sum}"
                    ));
                }
                lower[row][column] = sum.sqrt();
            } else {
                lower[row][column] = sum / lower[column][column];
            }
        }
    }
    Ok(lower)
}

fn empirical_correlation(samples: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    if samples.len() < 2 || samples[0].is_empty() {
        return Err("at least two samples and one dimension are required".to_owned());
    }
    let dimensions = samples[0].len();
    if samples.iter().any(|row| row.len() != dimensions) {
        return Err("sample matrix is ragged".to_owned());
    }
    let mut means = vec![0.0; dimensions];
    for row in samples {
        for dimension in 0..dimensions {
            means[dimension] += row[dimension];
        }
    }
    for mean in &mut means {
        *mean /= samples.len() as f64;
    }

    let mut covariance = vec![vec![0.0; dimensions]; dimensions];
    for row in samples {
        for i in 0..dimensions {
            for j in 0..dimensions {
                covariance[i][j] += (row[i] - means[i]) * (row[j] - means[j]);
            }
        }
    }
    let denominator = (samples.len() - 1) as f64;
    let standard_deviations = (0..dimensions)
        .map(|index| (covariance[index][index] / denominator).sqrt())
        .collect::<Vec<_>>();
    if standard_deviations.iter().any(|value| !value.is_finite() || *value <= 0.0) {
        return Err("empirical correlation is undefined because a sample dimension has zero variance".to_owned());
    }
    let mut correlation = vec![vec![0.0; dimensions]; dimensions];
    for i in 0..dimensions {
        for j in 0..dimensions {
            correlation[i][j] =
                covariance[i][j] / (denominator * standard_deviations[i] * standard_deviations[j]);
            if !correlation[i][j].is_finite() {
                return Err("empirical correlation contains a non-finite value".to_owned());
            }
        }
    }
    Ok(correlation)
}

fn inverse_standard_normal_cdf(probability: f64) -> Result<f64, String> {
    if !probability.is_finite() || probability <= 0.0 || probability >= 1.0 {
        return Err("normal quantile probability must be finite and strictly between zero and one".to_owned());
    }

    // Acklam's rational approximation; sufficient for seeded engineering design generation.
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_690e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    const LOW: f64 = 0.024_25;
    const HIGH: f64 = 1.0 - LOW;

    let value = if probability < LOW {
        let q = (-2.0 * probability.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if probability <= HIGH {
        let q = probability - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - probability).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err("normal quantile approximation produced a non-finite value".to_owned())
    }
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn correlation_row(matrix: &[Vec<f64>], index: usize) -> String {
    matrix[index]
        .iter()
        .map(|value| format!("{value:.12}"))
        .collect::<Vec<_>>()
        .join("|")
}

pub const LHS_CSV_HEADER: &str = "study_id,seed,sample_count,sample_id,input_id,input_unit,source_reference,mean,standard_uncertainty,minimum_allowed,maximum_allowed,stratified_probability,independent_normal_score,correlated_normal_score,sampled_value,target_correlation_row,empirical_latent_correlation_row,empirical_output_correlation_row,evidence_level";

pub fn report_to_csv(design: &LatinHypercubeDesign) -> String {
    let mut output = format!("{LHS_CSV_HEADER}\n");
    for sample in &design.samples {
        let marginal = &design.marginals[sample.input_index];
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{},{},{},{}\n",
            csv_escape(&design.study_id),
            design.seed,
            design.sample_count,
            sample.sample_id,
            csv_escape(&marginal.input_id),
            csv_escape(&marginal.input_unit),
            csv_escape(&marginal.source_reference),
            marginal.mean,
            marginal.standard_uncertainty,
            marginal.minimum_allowed,
            marginal.maximum_allowed,
            sample.stratified_probability,
            sample.independent_normal_score,
            sample.correlated_normal_score,
            sample.value,
            correlation_row(&design.target_correlation_matrix, sample.input_index),
            correlation_row(&design.empirical_correlation_matrix, sample.input_index),
            correlation_row(&design.empirical_output_correlation_matrix, sample.input_index),
            design.evidence_level
        ));
    }
    output
}

#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn next_open_unit(&mut self) -> f64 {
        (((self.next_u64() >> 11) as f64) + 0.5) / 9_007_199_254_740_992.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn study(sample_count: usize) -> LatinHypercubeStudy {
        LatinHypercubeStudy {
            study_id: "synthetic-oat15a-input-design-v0.1".to_owned(),
            seed: 20_261_031,
            sample_count,
            marginals: vec![
                NormalInputMarginal {
                    input_id: "Mach".to_owned(),
                    input_unit: "1".to_owned(),
                    source_reference: "synthetic-flow-condition-budget".to_owned(),
                    mean: 0.73,
                    standard_uncertainty: 0.002,
                    minimum_allowed: 0.70,
                    maximum_allowed: 0.76,
                },
                NormalInputMarginal {
                    input_id: "Re_scale".to_owned(),
                    input_unit: "1".to_owned(),
                    source_reference: "synthetic-flow-condition-budget".to_owned(),
                    mean: 1.0,
                    standard_uncertainty: 0.02,
                    minimum_allowed: 0.80,
                    maximum_allowed: 1.20,
                },
                NormalInputMarginal {
                    input_id: "T_static".to_owned(),
                    input_unit: "K".to_owned(),
                    source_reference: "synthetic-flow-condition-budget".to_owned(),
                    mean: 271.0,
                    standard_uncertainty: 1.0,
                    minimum_allowed: 261.0,
                    maximum_allowed: 281.0,
                },
            ],
            correlation_matrix: vec![
                vec![1.0, 0.3, -0.1],
                vec![0.3, 1.0, 0.4],
                vec![-0.1, 0.4, 1.0],
            ],
        }
    }

    #[test]
    fn design_is_seed_deterministic_and_preserves_each_independent_lhs_stratum() {
        let first = generate_correlated_lhs(&study(128)).expect("valid normal LHS design");
        let second = generate_correlated_lhs(&study(128)).expect("same seed design");
        assert_eq!(first, second);
        assert_eq!(first.samples.len(), 128 * 3);
        for dimension in 0..first.marginals.len() {
            let mut strata = first
                .samples
                .iter()
                .filter(|sample| sample.input_index == dimension)
                .map(|sample| (sample.stratified_probability * 128.0).floor() as usize)
                .collect::<Vec<_>>();
            strata.sort_unstable();
            assert_eq!(strata, (0..128).collect::<Vec<_>>());
        }
    }

    #[test]
    fn correlation_transform_and_physical_bounds_are_reported_without_clipping() {
        let design = generate_correlated_lhs(&study(512)).expect("valid correlated design");
        for sample in &design.samples {
            let marginal = &design.marginals[sample.input_index];
            assert!(sample.value >= marginal.minimum_allowed);
            assert!(sample.value <= marginal.maximum_allowed);
        }
        assert!((design.empirical_correlation_matrix[0][1] - 0.3).abs() < 0.15);
        assert!((design.empirical_correlation_matrix[1][2] - 0.4).abs() < 0.15);
        assert!((design.empirical_correlation_matrix[0][2] + 0.1).abs() < 0.15);
        assert!((design.empirical_output_correlation_matrix[0][1] - 0.3).abs() < 0.15);
        assert!((design.empirical_output_correlation_matrix[1][2] - 0.4).abs() < 0.15);
        assert!((design.empirical_output_correlation_matrix[0][2] + 0.1).abs() < 0.15);
    }

    #[test]
    fn singular_or_invalid_correlation_matrix_is_rejected_not_repaired() {
        let mut invalid = study(64);
        invalid.correlation_matrix = vec![
            vec![1.0, 1.0, 1.0],
            vec![1.0, 1.0, -1.0],
            vec![1.0, -1.0, 1.0],
        ];
        let error = generate_correlated_lhs(&invalid).expect_err("singular matrix must fail");
        assert!(error.contains("positive definite"));
    }

    #[test]
    fn out_of_domain_samples_fail_closed_without_clipping() {
        let mut invalid = study(128);
        invalid.marginals[0].minimum_allowed = 0.729;
        invalid.marginals[0].maximum_allowed = 0.731;
        let error = generate_correlated_lhs(&invalid).expect_err("too narrow domain must fail");
        assert!(error.contains("falls outside declared bounds"));
    }

    #[test]
    fn duplicate_input_ids_and_ragged_or_bad_matrices_are_rejected() {
        let mut invalid = study(64);
        invalid.marginals[1].input_id = invalid.marginals[0].input_id.clone();
        assert!(generate_correlated_lhs(&invalid)
            .expect_err("duplicate id must fail")
            .contains("unique"));

        let mut invalid = study(64);
        invalid.correlation_matrix[0][1] = 0.8;
        assert!(generate_correlated_lhs(&invalid)
            .expect_err("asymmetric matrix must fail")
            .contains("symmetric"));

        let mut invalid = study(64);
        invalid.correlation_matrix[0][0] = 0.8;
        assert!(generate_correlated_lhs(&invalid)
            .expect_err("bad diagonal must fail")
            .contains("diagonal"));
    }

    #[test]
    fn csv_retains_target_and_empirical_correlations_and_evidence_ceiling() {
        let design = generate_correlated_lhs(&study(32)).expect("valid synthetic design");
        let output = report_to_csv(&design);
        assert_eq!(output, report_to_csv(&design));
        assert_eq!(output.lines().count(), 1 + 32 * 3);
        assert!(output.contains("seeded_correlated_lhs_input_design_only_no_solver_evaluations_no_output_uq"));
        assert!(output.contains("1.000000000000|0.300000000000|-0.100000000000"));
        assert!(output.contains("empirical_output_correlation_row"));
    }
}
