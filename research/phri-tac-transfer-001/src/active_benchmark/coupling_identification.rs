//! Identify a local cross-taxel mechanical forward operator from spatial probes.
//!
//! Training and held-out spatial excitation patterns are kept separate. Each
//! output-taxel row is estimated only over an explicit same-module support
//! profile. Rank-deficient rows return no coefficient estimates. Effects
//! outside the declared support remain visible as held-out prediction error.
//!
//! This is synthetic research code. It assumes the applied spatial stimulus
//! field is known to the identification routine; it does not qualify the
//! physical fixture, force reference, or applied contact locations.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub struct CouplingIdentificationProblem {
    pub taxel_count: usize,
    pub module_ids: Vec<u32>,
    pub support_radius: usize,
    /// Row-major pattern by source-taxel stimulus matrix.
    pub training_stimuli: Vec<f64>,
    /// Row-major pattern by output-taxel response matrix.
    pub training_responses: Vec<f64>,
    pub heldout_stimuli: Vec<f64>,
    pub heldout_responses: Vec<f64>,
    pub admitted_output_channels: Vec<bool>,
    pub forward_profile_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CouplingIdentificationConfig {
    pub rank_tolerance: f64,
}

impl Default for CouplingIdentificationConfig {
    fn default() -> Self {
        Self {
            rank_tolerance: 1.0e-10,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CouplingRowDisposition {
    IdentifiedFromTrainingPatterns,
    UnderidentifiedByTrainingPatterns,
    OutputChannelExcluded,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CouplingRowEstimate {
    pub output_taxel_index: usize,
    pub source_taxel_indices: Vec<usize>,
    /// None for every coefficient when the training design is rank deficient.
    pub coefficient_estimates: Vec<Option<f64>>,
    pub design_rank: usize,
    pub required_rank: usize,
    pub disposition: CouplingRowDisposition,
    pub training_rmse: Option<f64>,
    pub heldout_rmse: Option<f64>,
    /// Largest/smallest QR diagonal ratio; a conditioning proxy only.
    pub qr_diagonal_ratio_proxy: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CouplingIdentificationReport {
    pub taxel_count: usize,
    pub module_count: usize,
    pub support_radius: usize,
    pub training_pattern_count: usize,
    pub heldout_pattern_count: usize,
    pub admitted_output_channel_count: usize,
    pub identified_output_row_count: usize,
    pub underidentified_output_row_count: usize,
    pub excluded_output_row_count: usize,
    pub identified_coefficient_count: usize,
    pub required_coefficient_count: usize,
    pub mean_training_rmse: Option<f64>,
    pub mean_heldout_rmse: Option<f64>,
    pub maximum_heldout_rmse: Option<f64>,
    pub maximum_qr_diagonal_ratio_proxy: Option<f64>,
    pub rows: Vec<CouplingRowEstimate>,
    pub forward_profile_digest: String,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
struct LeastSquaresFit {
    coefficients: Option<Vec<f64>>,
    rank: usize,
    condition_proxy: Option<f64>,
}

fn validate_problem(
    problem: &CouplingIdentificationProblem,
    config: &CouplingIdentificationConfig,
) -> Result<(usize, usize, usize, usize), String> {
    let n = problem.taxel_count;
    if n == 0 || problem.module_ids.len() != n || problem.admitted_output_channels.len() != n {
        return Err("coupling problem requires matching nonempty taxel metadata".to_owned());
    }
    if problem.forward_profile_digest.trim().is_empty() {
        return Err("forward profile digest must not be empty".to_owned());
    }
    if problem.support_radius >= n {
        return Err("support radius must be smaller than taxel count".to_owned());
    }
    if !config.rank_tolerance.is_finite() || config.rank_tolerance <= 0.0 {
        return Err("QR rank tolerance must be finite and positive".to_owned());
    }
    if !problem
        .admitted_output_channels
        .iter()
        .any(|admitted| *admitted)
    {
        return Err("at least one output channel must be admitted".to_owned());
    }
    if problem.training_stimuli.is_empty()
        || !problem.training_stimuli.len().is_multiple_of(n)
        || problem.training_responses.is_empty()
        || !problem.training_responses.len().is_multiple_of(n)
        || problem.heldout_stimuli.is_empty()
        || !problem.heldout_stimuli.len().is_multiple_of(n)
        || problem.heldout_responses.is_empty()
        || !problem.heldout_responses.len().is_multiple_of(n)
    {
        return Err("stimuli and responses must be nonempty pattern by taxel arrays".to_owned());
    }
    if problem
        .training_stimuli
        .iter()
        .chain(&problem.heldout_stimuli)
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("spatial stimulus values must be finite and nonnegative".to_owned());
    }
    if problem
        .training_responses
        .iter()
        .chain(&problem.heldout_responses)
        .any(|value| !value.is_finite())
    {
        return Err("coupling responses must be finite".to_owned());
    }

    let training_patterns = problem.training_stimuli.len() / n;
    let training_response_patterns = problem.training_responses.len() / n;
    let heldout_patterns = problem.heldout_stimuli.len() / n;
    let heldout_response_patterns = problem.heldout_responses.len() / n;
    if training_patterns != training_response_patterns {
        return Err("training stimulus and response pattern counts differ".to_owned());
    }
    if heldout_patterns != heldout_response_patterns {
        return Err("held-out stimulus and response pattern counts differ".to_owned());
    }

    let module_count = problem
        .module_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .len();
    Ok((n, training_patterns, heldout_patterns, module_count))
}

fn local_sources(output_taxel: usize, module_ids: &[u32], support_radius: usize) -> Vec<usize> {
    (0..module_ids.len())
        .filter(|source_taxel| {
            module_ids[*source_taxel] == module_ids[output_taxel]
                && source_taxel.abs_diff(output_taxel) <= support_radius
        })
        .collect()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left_value, right_value)| left_value * right_value)
        .sum()
}

fn fit_row_least_squares(
    problem: &CouplingIdentificationProblem,
    output_taxel: usize,
    source_taxels: &[usize],
    training_pattern_count: usize,
    config: &CouplingIdentificationConfig,
) -> Result<LeastSquaresFit, String> {
    let required_rank = source_taxels.len();
    let mut residual_columns: Vec<Vec<f64>> = source_taxels
        .iter()
        .map(|source_taxel| {
            (0..training_pattern_count)
                .map(|pattern| {
                    problem.training_stimuli[pattern * problem.taxel_count + source_taxel]
                })
                .collect()
        })
        .collect();
    let mut permutation: Vec<usize> = (0..required_rank).collect();
    let mut q_columns: Vec<Vec<f64>> = Vec::with_capacity(required_rank);
    let mut upper = vec![vec![0.0; required_rank]; required_rank];
    let maximum_column_norm = residual_columns
        .iter()
        .map(|column| dot(column, column).sqrt())
        .fold(0.0_f64, f64::max);

    if maximum_column_norm <= f64::EPSILON {
        return Ok(LeastSquaresFit {
            coefficients: None,
            rank: 0,
            condition_proxy: None,
        });
    }

    // Column-pivoted, two-pass modified Gram-Schmidt. Pivoting reports the
    // numerical rank even when an early taxel has no excitation while later
    // source columns still contain independent information.
    for pivot_step in 0..required_rank {
        let pivot = (pivot_step..required_rank)
            .max_by(|left, right| {
                dot(
                    &residual_columns[*left],
                    &residual_columns[*left],
                )
                .total_cmp(&dot(
                    &residual_columns[*right],
                    &residual_columns[*right],
                ))
            })
            .ok_or_else(|| "QR pivot selection produced no candidate".to_owned())?;

        let pivot_norm = dot(
            &residual_columns[pivot],
            &residual_columns[pivot],
        )
        .sqrt();
        if pivot_norm <= config.rank_tolerance * maximum_column_norm {
            return Ok(LeastSquaresFit {
                coefficients: None,
                rank: pivot_step,
                condition_proxy: None,
            });
        }

        if pivot != pivot_step {
            residual_columns.swap(pivot_step, pivot);
            permutation.swap(pivot_step, pivot);
            for previous in 0..pivot_step {
                upper[previous].swap(pivot_step, pivot);
            }
        }

        let diagonal = dot(
            &residual_columns[pivot_step],
            &residual_columns[pivot_step],
        )
        .sqrt();
        upper[pivot_step][pivot_step] = diagonal;
        let basis: Vec<_> = residual_columns[pivot_step]
            .iter()
            .map(|value| value / diagonal)
            .collect();
        q_columns.push(basis.clone());

        for column in (pivot_step + 1)..required_rank {
            for pass in 0..2 {
                let projection = dot(&basis, &residual_columns[column]);
                if pass == 0 {
                    upper[pivot_step][column] = projection;
                } else {
                    upper[pivot_step][column] += projection;
                }
                for (value, component) in residual_columns[column].iter_mut().zip(&basis) {
                    *value -= projection * component;
                }
            }
        }
    }

    let response: Vec<_> = (0..training_pattern_count)
        .map(|pattern| {
            problem.training_responses[pattern * problem.taxel_count + output_taxel]
        })
        .collect();
    let projected: Vec<_> = q_columns
        .iter()
        .map(|basis| dot(basis, &response))
        .collect();

    let mut pivoted_coefficients = vec![0.0; required_rank];
    for row in (0..required_rank).rev() {
        let upper_sum = ((row + 1)..required_rank)
            .map(|column| upper[row][column] * pivoted_coefficients[column])
            .sum::<f64>();
        pivoted_coefficients[row] =
            (projected[row] - upper_sum) / upper[row][row];
    }

    let mut coefficients = vec![0.0; required_rank];
    for (pivoted_index, original_index) in permutation.iter().copied().enumerate() {
        coefficients[original_index] = pivoted_coefficients[pivoted_index];
    }

    let largest_diagonal = (0..required_rank)
        .map(|index| upper[index][index])
        .fold(0.0_f64, f64::max);
    let smallest_diagonal = (0..required_rank)
        .map(|index| upper[index][index])
        .fold(f64::INFINITY, f64::min);
    let condition_proxy = largest_diagonal / smallest_diagonal;

    if coefficients.iter().any(|value| !value.is_finite()) || !condition_proxy.is_finite() {
        return Err("least-squares solve produced non-finite coefficients".to_owned());
    }

    Ok(LeastSquaresFit {
        coefficients: Some(coefficients),
        rank: required_rank,
        condition_proxy: Some(condition_proxy),
    })
}

fn prediction_rmse(
    stimuli: &[f64],
    responses: &[f64],
    pattern_count: usize,
    taxel_count: usize,
    output_taxel: usize,
    source_taxels: &[usize],
    coefficients: &[f64],
) -> f64 {
    let squared_error = (0..pattern_count)
        .map(|pattern| {
            let predicted = source_taxels
                .iter()
                .zip(coefficients)
                .map(|(source, coefficient)| stimuli[pattern * taxel_count + source] * coefficient)
                .sum::<f64>();
            let residual = predicted - responses[pattern * taxel_count + output_taxel];
            residual * residual
        })
        .sum::<f64>();
    (squared_error / pattern_count as f64).sqrt()
}

/// Estimate local coupling coefficients for every admitted output-taxel row.
/// A rank-deficient spatial design returns no coefficient estimates.
pub fn identify_coupling_operator(
    problem: &CouplingIdentificationProblem,
    config: &CouplingIdentificationConfig,
) -> Result<CouplingIdentificationReport, String> {
    let (n, training_patterns, heldout_patterns, module_count) = validate_problem(problem, config)?;
    let mut rows = Vec::with_capacity(n);
    let mut identified_output_row_count = 0;
    let mut underidentified_output_row_count = 0;
    let mut excluded_output_row_count = 0;
    let mut identified_coefficient_count = 0;
    let mut required_coefficient_count = 0;
    let mut training_rmse_values = Vec::new();
    let mut heldout_rmse_values = Vec::new();
    let mut condition_proxies = Vec::new();

    for output_taxel in 0..n {
        if !problem.admitted_output_channels[output_taxel] {
            excluded_output_row_count += 1;
            rows.push(CouplingRowEstimate {
                output_taxel_index: output_taxel,
                source_taxel_indices: Vec::new(),
                coefficient_estimates: Vec::new(),
                design_rank: 0,
                required_rank: 0,
                disposition: CouplingRowDisposition::OutputChannelExcluded,
                training_rmse: None,
                heldout_rmse: None,
                qr_diagonal_ratio_proxy: None,
            });
            continue;
        }

        let sources = local_sources(output_taxel, &problem.module_ids, problem.support_radius);
        let required_rank = sources.len();
        let fit =
            fit_row_least_squares(problem, output_taxel, &sources, training_patterns, config)?;
        required_coefficient_count += required_rank;

        match fit.coefficients {
            Some(coefficients) => {
                identified_output_row_count += 1;
                identified_coefficient_count += coefficients.len();
                let training_rmse = prediction_rmse(
                    &problem.training_stimuli,
                    &problem.training_responses,
                    training_patterns,
                    n,
                    output_taxel,
                    &sources,
                    &coefficients,
                );
                let heldout_rmse = prediction_rmse(
                    &problem.heldout_stimuli,
                    &problem.heldout_responses,
                    heldout_patterns,
                    n,
                    output_taxel,
                    &sources,
                    &coefficients,
                );
                training_rmse_values.push(training_rmse);
                heldout_rmse_values.push(heldout_rmse);
                if let Some(condition) = fit.condition_proxy {
                    condition_proxies.push(condition);
                }
                rows.push(CouplingRowEstimate {
                    output_taxel_index: output_taxel,
                    source_taxel_indices: sources,
                    coefficient_estimates: coefficients.into_iter().map(Some).collect(),
                    design_rank: fit.rank,
                    required_rank,
                    disposition: CouplingRowDisposition::IdentifiedFromTrainingPatterns,
                    training_rmse: Some(training_rmse),
                    heldout_rmse: Some(heldout_rmse),
                    qr_diagonal_ratio_proxy: fit.condition_proxy,
                });
            }
            None => {
                underidentified_output_row_count += 1;
                rows.push(CouplingRowEstimate {
                    output_taxel_index: output_taxel,
                    coefficient_estimates: vec![None; required_rank],
                    design_rank: fit.rank,
                    required_rank,
                    disposition: CouplingRowDisposition::UnderidentifiedByTrainingPatterns,
                    source_taxel_indices: sources,
                    training_rmse: None,
                    heldout_rmse: None,
                    qr_diagonal_ratio_proxy: None,
                });
            }
        }
    }

    let mean = |values: &[f64]| -> Option<f64> {
        (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
    };
    Ok(CouplingIdentificationReport {
        taxel_count: n,
        module_count,
        support_radius: problem.support_radius,
        training_pattern_count: training_patterns,
        heldout_pattern_count: heldout_patterns,
        admitted_output_channel_count: problem
            .admitted_output_channels
            .iter()
            .filter(|admitted| **admitted)
            .count(),
        identified_output_row_count,
        underidentified_output_row_count,
        excluded_output_row_count,
        identified_coefficient_count,
        required_coefficient_count,
        mean_training_rmse: mean(&training_rmse_values),
        mean_heldout_rmse: mean(&heldout_rmse_values),
        maximum_heldout_rmse: heldout_rmse_values.iter().copied().reduce(f64::max),
        maximum_qr_diagonal_ratio_proxy: condition_proxies.iter().copied().reduce(f64::max),
        rows,
        forward_profile_digest: problem.forward_profile_digest.clone(),
        claim_class: "synthetic_local_coupling_operator_identification_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_unit(state: &mut u64) -> f64 {
        let mut value = *state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        *state = value;
        (value >> 11) as f64 / ((1_u64 << 53) as f64)
    }

    fn spatial_patterns(taxel_count: usize, pattern_count: usize, seed: u64) -> Vec<f64> {
        let mut state = seed.max(1);
        (0..pattern_count * taxel_count)
            .map(|_| 0.1 + random_unit(&mut state))
            .collect()
    }

    fn kernel(taxel_count: usize, module_ids: &[u32], diagonal: f64, neighbor: f64) -> Vec<f64> {
        let mut result = vec![0.0; taxel_count * taxel_count];
        for output in 0..taxel_count {
            result[output * taxel_count + output] = diagonal;
            for source in 0..taxel_count {
                if source != output
                    && module_ids[source] == module_ids[output]
                    && source.abs_diff(output) == 1
                {
                    result[output * taxel_count + source] = neighbor;
                }
            }
        }
        result
    }

    fn apply_kernel(
        stimuli: &[f64],
        kernel: &[f64],
        pattern_count: usize,
        taxel_count: usize,
    ) -> Vec<f64> {
        (0..pattern_count)
            .flat_map(|pattern| {
                (0..taxel_count).map(move |output| {
                    (0..taxel_count)
                        .map(|source| {
                            kernel[output * taxel_count + source]
                                * stimuli[pattern * taxel_count + source]
                        })
                        .sum()
                })
            })
            .collect()
    }

    #[test]
    fn randomized_nonnegative_spatial_patterns_identify_a_known_local_kernel() {
        let taxel_count = 5;
        let module_ids = vec![0, 0, 0, 1, 1];
        let true_kernel = kernel(taxel_count, &module_ids, 0.7, 0.12);
        let training = spatial_patterns(taxel_count, 12, 41);
        let heldout = spatial_patterns(taxel_count, 5, 99);
        let problem = CouplingIdentificationProblem {
            taxel_count,
            module_ids,
            support_radius: 1,
            training_responses: apply_kernel(&training, &true_kernel, 12, taxel_count),
            heldout_responses: apply_kernel(&heldout, &true_kernel, 5, taxel_count),
            training_stimuli: training,
            heldout_stimuli: heldout,
            admitted_output_channels: vec![true; taxel_count],
            forward_profile_digest: "sha256:local-kernel-profile".to_owned(),
        };
        let report = identify_coupling_operator(&problem, &CouplingIdentificationConfig::default())
            .expect("known local kernel");
        assert_eq!(report.identified_output_row_count, taxel_count);
        assert_eq!(report.underidentified_output_row_count, 0);
        assert!(report.maximum_heldout_rmse.unwrap() < 1.0e-9);
        for row in &report.rows {
            assert_eq!(
                row.disposition,
                CouplingRowDisposition::IdentifiedFromTrainingPatterns
            );
            assert!(row.coefficient_estimates.iter().all(Option::is_some));
            for (source, coefficient) in row
                .source_taxel_indices
                .iter()
                .zip(&row.coefficient_estimates)
            {
                let expected = if *source == row.output_taxel_index {
                    0.7
                } else {
                    0.12
                };
                assert!((coefficient.unwrap() - expected).abs() < 1.0e-9);
            }
        }
    }

    #[test]
    fn rank_deficient_spatial_excitation_never_emits_partial_coefficients() {
        let taxel_count = 3;
        let module_ids = vec![0, 0, 0];
        let stimuli = vec![1.0; 2 * taxel_count];
        let true_kernel = kernel(taxel_count, &module_ids, 0.7, 0.1);
        let responses = apply_kernel(&stimuli, &true_kernel, 2, taxel_count);
        let problem = CouplingIdentificationProblem {
            taxel_count,
            module_ids,
            support_radius: 1,
            training_stimuli: stimuli.clone(),
            training_responses: responses.clone(),
            heldout_stimuli: stimuli,
            heldout_responses: responses,
            admitted_output_channels: vec![true; taxel_count],
            forward_profile_digest: "sha256:rank-deficient-patterns".to_owned(),
        };
        let report = identify_coupling_operator(&problem, &CouplingIdentificationConfig::default())
            .expect("rank-deficient design");
        assert_eq!(report.underidentified_output_row_count, 3);
        assert!(report.rows.iter().all(|row| {
            row.disposition == CouplingRowDisposition::UnderidentifiedByTrainingPatterns
                && row.coefficient_estimates.iter().all(Option::is_none)
        }));
    }

    #[test]
    fn pivoted_qr_reports_rank_after_an_unexcited_early_source_column() {
        let problem = CouplingIdentificationProblem {
            taxel_count: 3,
            module_ids: vec![0, 0, 0],
            support_radius: 1,
            training_stimuli: vec![
                0.0, 1.0, 0.0,
                0.0, 0.0, 1.0,
                0.0, 1.0, 1.0,
                0.0, 2.0, 0.0,
            ],
            training_responses: vec![0.0; 12],
            heldout_stimuli: vec![
                0.0, 1.0, 0.0,
                0.0, 0.0, 1.0,
            ],
            heldout_responses: vec![0.0; 6],
            admitted_output_channels: vec![true; 3],
            forward_profile_digest: "sha256:pivoted-rank-known-answer".to_owned(),
        };
        let report = identify_coupling_operator(
            &problem,
            &CouplingIdentificationConfig::default(),
        )
        .expect("rank revealing design");
        let middle = &report.rows[1];
        assert_eq!(middle.required_rank, 3);
        assert_eq!(middle.design_rank, 2);
        assert_eq!(
            middle.disposition,
            CouplingRowDisposition::UnderidentifiedByTrainingPatterns
        );
        assert!(middle.coefficient_estimates.iter().all(Option::is_none));
    }

    #[test]
    fn cross_module_coupling_is_exposed_as_heldout_model_mismatch() {
        let taxel_count = 4;
        let module_ids = vec![0, 0, 1, 1];
        let mut true_kernel = kernel(taxel_count, &module_ids, 0.7, 0.1);
        // Inject coupling across the declared module seam.
        true_kernel[taxel_count + 2] = 0.08;
        let training = spatial_patterns(taxel_count, 16, 7);
        let heldout = spatial_patterns(taxel_count, 6, 8);
        let problem = CouplingIdentificationProblem {
            taxel_count,
            module_ids,
            support_radius: 1,
            training_responses: apply_kernel(&training, &true_kernel, 16, taxel_count),
            heldout_responses: apply_kernel(&heldout, &true_kernel, 6, taxel_count),
            training_stimuli: training,
            heldout_stimuli: heldout,
            admitted_output_channels: vec![true; taxel_count],
            forward_profile_digest: "sha256:cross-seam-negative-control".to_owned(),
        };
        let report = identify_coupling_operator(&problem, &CouplingIdentificationConfig::default())
            .expect("cross seam model mismatch");
        assert!(report.maximum_heldout_rmse.unwrap() > 0.01);
        assert!(!report.rows[1].source_taxel_indices.contains(&2));
    }

    #[test]
    fn excluded_output_channels_are_not_fitted() {
        let taxel_count = 3;
        let module_ids = vec![0, 0, 0];
        let true_kernel = kernel(taxel_count, &module_ids, 0.7, 0.1);
        let training = spatial_patterns(taxel_count, 8, 100);
        let heldout = spatial_patterns(taxel_count, 4, 200);
        let problem = CouplingIdentificationProblem {
            taxel_count,
            module_ids,
            support_radius: 1,
            training_responses: apply_kernel(&training, &true_kernel, 8, taxel_count),
            heldout_responses: apply_kernel(&heldout, &true_kernel, 4, taxel_count),
            training_stimuli: training,
            heldout_stimuli: heldout,
            admitted_output_channels: vec![true, false, true],
            forward_profile_digest: "sha256:excluded-output".to_owned(),
        };
        let report = identify_coupling_operator(&problem, &CouplingIdentificationConfig::default())
            .expect("excluded output");
        assert_eq!(report.excluded_output_row_count, 1);
        assert_eq!(
            report.rows[1].disposition,
            CouplingRowDisposition::OutputChannelExcluded
        );
        assert!(report.rows[1].coefficient_estimates.is_empty());
    }

    #[test]
    fn malformed_dimensions_negative_stimuli_and_nonfinite_values_fail_closed() {
        let taxel_count = 2;
        let module_ids = vec![0, 0];
        let true_kernel = kernel(taxel_count, &module_ids, 0.7, 0.1);
        let training = spatial_patterns(taxel_count, 4, 4);
        let heldout = spatial_patterns(taxel_count, 3, 5);
        let base = CouplingIdentificationProblem {
            taxel_count,
            module_ids,
            support_radius: 1,
            training_responses: apply_kernel(&training, &true_kernel, 4, taxel_count),
            heldout_responses: apply_kernel(&heldout, &true_kernel, 3, taxel_count),
            training_stimuli: training,
            heldout_stimuli: heldout,
            admitted_output_channels: vec![true; taxel_count],
            forward_profile_digest: "sha256:validation".to_owned(),
        };
        let mut malformed = base.clone();
        malformed.heldout_stimuli.pop();
        assert!(
            identify_coupling_operator(&malformed, &CouplingIdentificationConfig::default())
                .is_err()
        );

        let mut negative = base.clone();
        negative.training_stimuli[0] = -0.1;
        assert!(
            identify_coupling_operator(&negative, &CouplingIdentificationConfig::default())
                .is_err()
        );

        let mut nonfinite = base;
        nonfinite.heldout_responses[0] = f64::NAN;
        assert!(
            identify_coupling_operator(&nonfinite, &CouplingIdentificationConfig::default())
                .is_err()
        );
    }
}
