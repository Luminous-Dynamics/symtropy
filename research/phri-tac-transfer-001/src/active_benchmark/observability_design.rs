//! Observability-first selection of spatial stimuli for local coupling ID.
//!
//! Selection uses only declared topology, previously chosen stimuli and the
//! candidate stimulus pool. It never sees response labels, hidden kernel
//! coefficients or held-out truth. The primary objective is incremental row
//! rank across local output-taxel regression designs; full-row identifiability
//! is reported separately from a residual/conditioning preference.
//!
//! This is a design-stage primitive. Rank is exact only relative to the
//! declared local support and numerical tolerance. It is not a statement of
//! physical calibration quality or uncertainty.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialStimulusCandidate {
    pub candidate_id: String,
    pub pattern: Vec<f64>,
    /// Relative experiment cost supplied by the apparatus owner. It is not
    /// inferred from active-site count or assumed to be a reference-contact.
    pub cost_units: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObservabilityDesignProblem {
    pub taxel_count: usize,
    pub module_ids: Vec<u32>,
    pub support_radius: usize,
    pub admitted_output_channels: Vec<bool>,
    /// Previously executed patterns, row-major pattern × taxel.
    pub initial_patterns: Vec<f64>,
    pub candidates: Vec<SpatialStimulusCandidate>,
    pub maximum_additional_patterns: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObservabilityDesignConfig {
    pub rank_tolerance: f64,
    pub minimum_normalized_residual_gain: f64,
}

impl Default for ObservabilityDesignConfig {
    fn default() -> Self {
        Self {
            rank_tolerance: 1.0e-10,
            minimum_normalized_residual_gain: 1.0e-8,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct OutputRowRank {
    pub output_taxel_index: usize,
    pub source_taxel_indices: Vec<usize>,
    pub rank_before: usize,
    pub rank_after: usize,
    pub required_rank: usize,
    pub identifiable_before: bool,
    pub identifiable_after: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ObservabilitySelectionStep {
    pub ordinal: usize,
    pub candidate_id: String,
    pub rank_gain: usize,
    pub newly_identifiable_output_rows: usize,
    pub normalized_residual_gain_per_cost: f64,
    pub cost_units: f64,
    pub output_rows: Vec<OutputRowRank>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ObservabilityDesignReport {
    pub taxel_count: usize,
    pub module_count: usize,
    pub support_radius: usize,
    pub initial_pattern_count: usize,
    pub selected_pattern_count: usize,
    pub total_selected_cost_units: f64,
    pub initial_identifiable_output_rows: usize,
    pub final_identifiable_output_rows: usize,
    pub total_required_coefficients: usize,
    pub initial_identifiable_coefficients: usize,
    pub final_identifiable_coefficients: usize,
    pub remaining_unidentifiable_coefficients: usize,
    pub steps: Vec<ObservabilitySelectionStep>,
    pub claim_class: &'static str,
}

fn validate_problem(
    problem: &ObservabilityDesignProblem,
    config: &ObservabilityDesignConfig,
) -> Result<(), String> {
    let n = problem.taxel_count;
    if n == 0 || problem.module_ids.len() != n || problem.admitted_output_channels.len() != n {
        return Err("observability design requires matching nonempty taxel metadata".to_owned());
    }
    if problem.support_radius >= n {
        return Err("observability design support radius must be smaller than taxel count".to_owned());
    }
    if !config.rank_tolerance.is_finite()
        || config.rank_tolerance <= 0.0
        || !config.minimum_normalized_residual_gain.is_finite()
        || config.minimum_normalized_residual_gain < 0.0
    {
        return Err("observability design tolerances must be finite and valid".to_owned());
    }
    if !problem.admitted_output_channels.iter().any(|admitted| *admitted) {
        return Err("observability design requires at least one admitted output row".to_owned());
    }
    if !problem.initial_patterns.len().is_multiple_of(n)
        || problem.initial_patterns.iter().any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("initial stimuli must be finite nonnegative pattern × taxel data".to_owned());
    }

    let mut identifiers = BTreeSet::new();
    for candidate in &problem.candidates {
        if candidate.candidate_id.trim().is_empty() || !identifiers.insert(&candidate.candidate_id) {
            return Err("candidate IDs must be nonempty and unique".to_owned());
        }
        if candidate.pattern.len() != n
            || candidate.pattern.iter().any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err("candidate patterns must be finite nonnegative vectors of taxel_count length".to_owned());
        }
        if !candidate.cost_units.is_finite() || candidate.cost_units <= 0.0 {
            return Err("candidate cost_units must be finite and positive".to_owned());
        }
    }
    Ok(())
}

fn local_sources(
    output_taxel: usize,
    module_ids: &[u32],
    support_radius: usize,
) -> Vec<usize> {
    (0..module_ids.len())
        .filter(|source| {
            module_ids[*source] == module_ids[output_taxel]
                && source.abs_diff(output_taxel) <= support_radius
        })
        .collect()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn append_orthonormal(
    basis: &mut Vec<Vec<f64>>,
    vector: &[f64],
    rank_tolerance: f64,
) -> f64 {
    let original_norm = dot(vector, vector).sqrt();
    if original_norm <= f64::EPSILON {
        return 0.0;
    }
    let mut residual = vector.to_vec();
    for _ in 0..2 {
        for direction in basis.iter() {
            let projection = dot(direction, &residual);
            for (value, basis_value) in residual.iter_mut().zip(direction) {
                *value -= projection * basis_value;
            }
        }
    }
    let residual_norm = dot(&residual, &residual).sqrt();
    if residual_norm > rank_tolerance * original_norm {
        for value in &mut residual {
            *value /= residual_norm;
        }
        basis.push(residual);
        residual_norm
    } else {
        0.0
    }
}

fn row_basis(
    patterns: &[Vec<f64>],
    source_taxels: &[usize],
    rank_tolerance: f64,
) -> Vec<Vec<f64>> {
    let mut basis = Vec::with_capacity(source_taxels.len());
    for pattern in patterns {
        let row: Vec<_> = source_taxels.iter().map(|source| pattern[*source]).collect();
        append_orthonormal(&mut basis, &row, rank_tolerance);
    }
    basis
}

fn evaluate_candidate(
    problem: &ObservabilityDesignProblem,
    config: &ObservabilityDesignConfig,
    selected_patterns: &[Vec<f64>],
    candidate: &SpatialStimulusCandidate,
) -> (usize, usize, f64, Vec<OutputRowRank>) {
    let mut rank_gain = 0;
    let mut newly_identifiable_output_rows = 0;
    let mut normalized_gain_sum = 0.0;
    let mut row_results = Vec::new();

    for output in 0..problem.taxel_count {
        if !problem.admitted_output_channels[output] {
            continue;
        }
        let sources = local_sources(output, &problem.module_ids, problem.support_radius);
        let basis = row_basis(selected_patterns, &sources, config.rank_tolerance);
        let rank_before = basis.len();
        let features: Vec<_> = sources
            .iter()
            .map(|source| candidate.pattern[*source])
            .collect();
        let candidate_norm = dot(&features, &features).sqrt();
        let mut probe_basis = basis.clone();
        let residual_norm = append_orthonormal(
            &mut probe_basis,
            &features,
            config.rank_tolerance,
        );
        let normalized_gain = if candidate_norm > f64::EPSILON {
            residual_norm / candidate_norm
        } else {
            0.0
        };
        let gained = usize::from(
            normalized_gain > config.minimum_normalized_residual_gain
                && probe_basis.len() > rank_before,
        );
        let rank_after = rank_before + gained;
        rank_gain += gained;
        if rank_before < sources.len() && rank_after == sources.len() {
            newly_identifiable_output_rows += 1;
        }
        normalized_gain_sum += normalized_gain;
        row_results.push(OutputRowRank {
            output_taxel_index: output,
            source_taxel_indices: sources.clone(),
            rank_before,
            rank_after,
            required_rank: sources.len(),
            identifiable_before: rank_before == sources.len(),
            identifiable_after: rank_after == sources.len(),
        });
    }

    (
        rank_gain,
        newly_identifiable_output_rows,
        normalized_gain_sum / candidate.cost_units,
        row_results,
    )
}

fn current_identifiability(
    problem: &ObservabilityDesignProblem,
    patterns: &[Vec<f64>],
    tolerance: f64,
) -> (usize, usize, usize) {
    let mut identifiable_rows = 0;
    let mut total_required = 0;
    let mut identifiable_coefficients = 0;
    for output in 0..problem.taxel_count {
        if !problem.admitted_output_channels[output] {
            continue;
        }
        let sources = local_sources(output, &problem.module_ids, problem.support_radius);
        let rank = row_basis(patterns, &sources, tolerance).len();
        total_required += sources.len();
        identifiable_coefficients += rank;
        if rank == sources.len() {
            identifiable_rows += 1;
        }
    }
    (identifiable_rows, total_required, identifiable_coefficients)
}

/// Greedily select the candidate with the largest native-data rank gain.
/// Ties prefer completing output rows, then larger normalized residual gain
/// per declared apparatus cost, then lexical candidate ID for determinism.
pub fn select_observability_first_patterns(
    problem: &ObservabilityDesignProblem,
    config: &ObservabilityDesignConfig,
) -> Result<ObservabilityDesignReport, String> {
    validate_problem(problem, config)?;

    let mut selected_patterns: Vec<Vec<f64>> = problem
        .initial_patterns
        .chunks_exact(problem.taxel_count)
        .map(<[f64]>::to_vec)
        .collect();
    let initial_pattern_count = selected_patterns.len();
    let (initial_identifiable_output_rows, total_required_coefficients, initial_identifiable_coefficients) =
        current_identifiability(problem, &selected_patterns, config.rank_tolerance);

    let mut remaining: Vec<usize> = (0..problem.candidates.len()).collect();
    let mut selected_cost = 0.0;
    let mut steps = Vec::new();

    while steps.len() < problem.maximum_additional_patterns && !remaining.is_empty() {
        let mut best: Option<(usize, usize, f64, String, usize, Vec<OutputRowRank>)> = None;
        for candidate_index in &remaining {
            let candidate = &problem.candidates[*candidate_index];
            let (rank_gain, full_rows, residual_gain_per_cost, row_results) = evaluate_candidate(
                problem,
                config,
                &selected_patterns,
                candidate,
            );
            if rank_gain == 0 {
                continue;
            }
            let replace_best = match &best {
                None => true,
                Some((best_rank, best_full_rows, best_residual, best_id, _, _)) => {
                    rank_gain > *best_rank
                        || (rank_gain == *best_rank && full_rows > *best_full_rows)
                        || (rank_gain == *best_rank
                            && full_rows == *best_full_rows
                            && residual_gain_per_cost.total_cmp(best_residual).is_gt())
                        || (rank_gain == *best_rank
                            && full_rows == *best_full_rows
                            && residual_gain_per_cost.total_cmp(best_residual).is_eq()
                            && candidate.candidate_id < *best_id)
                }
            };
            if replace_best {
                best = Some((
                    rank_gain,
                    full_rows,
                    residual_gain_per_cost,
                    candidate.candidate_id.clone(),
                    *candidate_index,
                    row_results,
                ));
            }
        }

        let Some((rank_gain, full_rows, residual_gain_per_cost, candidate_id, index, rows)) =
            best
        else {
            break;
        };
        let candidate = &problem.candidates[index];
        steps.push(ObservabilitySelectionStep {
            ordinal: steps.len() + 1,
            candidate_id: candidate_id.clone(),
            rank_gain,
            newly_identifiable_output_rows: full_rows,
            normalized_residual_gain_per_cost: residual_gain_per_cost,
            cost_units: candidate.cost_units,
            output_rows: rows,
        });
        selected_cost += candidate.cost_units;
        selected_patterns.push(candidate.pattern.clone());
        remaining.retain(|candidate_index| *candidate_index != index);
    }

    let (final_identifiable_output_rows, _, final_identifiable_coefficients) =
        current_identifiability(problem, &selected_patterns, config.rank_tolerance);

    Ok(ObservabilityDesignReport {
        taxel_count: problem.taxel_count,
        module_count: problem.module_ids.iter().copied().collect::<BTreeSet<_>>().len(),
        support_radius: problem.support_radius,
        initial_pattern_count,
        selected_pattern_count: steps.len(),
        total_selected_cost_units: selected_cost,
        initial_identifiable_output_rows,
        final_identifiable_output_rows,
        total_required_coefficients,
        initial_identifiable_coefficients,
        final_identifiable_coefficients,
        remaining_unidentifiable_coefficients: total_required_coefficients
            .saturating_sub(final_identifiable_coefficients),
        steps,
        claim_class: "synthetic_observability_first_design_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, pattern: &[f64], cost_units: f64) -> SpatialStimulusCandidate {
        SpatialStimulusCandidate {
            candidate_id: id.to_owned(),
            pattern: pattern.to_vec(),
            cost_units,
        }
    }

    #[test]
    fn rank_gain_selection_completes_local_rows_and_is_deterministic() {
        let problem = ObservabilityDesignProblem {
            taxel_count: 3,
            module_ids: vec![0, 0, 0],
            support_radius: 1,
            admitted_output_channels: vec![true; 3],
            initial_patterns: Vec::new(),
            candidates: vec![
                candidate("common-mode", &[1.0, 1.0, 1.0], 1.0),
                candidate("left-only", &[1.0, 0.0, 0.0], 1.0),
                candidate("middle-only", &[0.0, 1.0, 0.0], 1.0),
                candidate("right-only", &[0.0, 0.0, 1.0], 1.0),
                candidate("common-mode-repeat", &[1.0, 1.0, 1.0], 1.0),
            ],
            maximum_additional_patterns: 4,
        };
        let first = select_observability_first_patterns(
            &problem,
            &ObservabilityDesignConfig::default(),
        )
        .expect("rank-gain selection");
        let second = select_observability_first_patterns(
            &problem,
            &ObservabilityDesignConfig::default(),
        )
        .expect("deterministic repeat");
        assert_eq!(first, second);
        assert_eq!(first.initial_identifiable_output_rows, 0);
        assert_eq!(first.final_identifiable_output_rows, 3);
        assert_eq!(first.final_identifiable_coefficients, first.total_required_coefficients);
        assert!(first.steps.iter().all(|step| step.rank_gain > 0));
        assert!(!first.steps.iter().any(|step| step.candidate_id == "common-mode-repeat"));
    }

    #[test]
    fn repeated_common_mode_patterns_do_not_fake_native_rank() {
        let problem = ObservabilityDesignProblem {
            taxel_count: 3,
            module_ids: vec![0, 0, 0],
            support_radius: 1,
            admitted_output_channels: vec![true; 3],
            initial_patterns: vec![1.0, 1.0, 1.0],
            candidates: vec![
                candidate("common-2", &[2.0, 2.0, 2.0], 1.0),
                candidate("common-half", &[0.5, 0.5, 0.5], 1.0),
            ],
            maximum_additional_patterns: 2,
        };
        let report = select_observability_first_patterns(
            &problem,
            &ObservabilityDesignConfig::default(),
        )
        .expect("common-mode design");
        assert_eq!(report.selected_pattern_count, 0);
        assert_eq!(report.initial_identifiable_output_rows, 0);
        assert_eq!(report.final_identifiable_output_rows, 0);
    }

    #[test]
    fn declared_cost_only_breaks_ties_after_identifiability_gain() {
        let problem = ObservabilityDesignProblem {
            taxel_count: 2,
            module_ids: vec![0, 0],
            support_radius: 1,
            admitted_output_channels: vec![true, true],
            initial_patterns: vec![1.0, 1.0],
            candidates: vec![
                candidate("expensive-independent", &[1.0, 0.0], 10.0),
                candidate("cheap-independent", &[0.0, 1.0], 1.0),
            ],
            maximum_additional_patterns: 1,
        };
        let report = select_observability_first_patterns(
            &problem,
            &ObservabilityDesignConfig::default(),
        )
        .expect("cost tie-break");
        assert_eq!(report.steps[0].candidate_id, "cheap-independent");
        assert_eq!(report.steps[0].rank_gain, 2);
    }

    #[test]
    fn invalid_candidate_dimensions_costs_and_nonfinite_values_fail_closed() {
        let base = ObservabilityDesignProblem {
            taxel_count: 2,
            module_ids: vec![0, 0],
            support_radius: 1,
            admitted_output_channels: vec![true, true],
            initial_patterns: Vec::new(),
            candidates: vec![candidate("valid", &[1.0, 0.0], 1.0)],
            maximum_additional_patterns: 1,
        };
        let mut invalid = base.clone();
        invalid.candidates[0].pattern.pop();
        assert!(select_observability_first_patterns(&invalid, &ObservabilityDesignConfig::default()).is_err());

        let mut invalid = base.clone();
        invalid.candidates[0].cost_units = 0.0;
        assert!(select_observability_first_patterns(&invalid, &ObservabilityDesignConfig::default()).is_err());

        let mut invalid = base;
        invalid.candidates[0].pattern[0] = f64::NAN;
        assert!(select_observability_first_patterns(&invalid, &ObservabilityDesignConfig::default()).is_err());
    }
}
