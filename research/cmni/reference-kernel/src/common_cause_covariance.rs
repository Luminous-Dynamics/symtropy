//! Joint observation covariance assembly for shared experimental error sources.
//!
//! Each experiment contributes an independent observation covariance plus
//! Jacobian sensitivities to named shared uncertainty sources (for example a
//! reference evaluation, detector calibration, or evaluated cross-section
//! library). Repeated source IDs create the mathematically implied
//! cross-experiment covariance; labels alone are never treated as evidence of
//! independence or dependence.
//!
//! For experiment i and shared source s:
//!
//!   C_ij += J_(i,s) C_s J_(j,s)^T
//!
//! A source ID must have one consistent covariance definition throughout the
//! portfolio. Different source IDs are accumulated as independent latent
//! blocks. If two calibration components are correlated, they must be bundled
//! into one joint source ID with the full covariance matrix and matching
//! Jacobians. The inter-source qualification records whether this block
//! independence assumption is established or still unknown.

use crate::ModelError;
use std::collections::{BTreeMap, BTreeSet};

/// One experiment's sensitivity to a shared uncertain input.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedUncertaintyContribution {
    pub source_id: String,
    /// Covariance of this source in its native parameterization.
    pub source_covariance: Vec<Vec<f64>>,
    /// Derivative of this experiment's observation vector with respect to source.
    /// Shape: observation_count x source_dimension.
    pub observation_jacobian: Vec<Vec<f64>>,
}

/// Evidence qualification for covariance relationships between distinct source IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterSourceDependenceQualification {
    /// Correlated latent components are bundled into joint source covariance blocks.
    JointBlocksModelled { receipt_id: String },
    /// Distinct source blocks have a documented independence attestation.
    DistinctBlocksIndependenceAttested { receipt_id: String },
    /// Correlations between different source IDs may be unmodeled.
    Unknown,
}

impl InterSourceDependenceQualification {
    /// Record that known cross-component dependence is represented by joint
    /// covariance blocks identified by this evidence receipt.
    pub fn joint_blocks_modelled(receipt_id: impl Into<String>) -> Self {
        Self::JointBlocksModelled {
            receipt_id: receipt_id.into(),
        }
    }

    /// Record an explicit evidence receipt attesting that distinct source
    /// covariance blocks may be treated as independent.
    pub fn distinct_blocks_independence_attested(receipt_id: impl Into<String>) -> Self {
        Self::DistinctBlocksIndependenceAttested {
            receipt_id: receipt_id.into(),
        }
    }

    pub fn is_qualified(&self) -> bool {
        match self {
            Self::JointBlocksModelled { receipt_id }
            | Self::DistinctBlocksIndependenceAttested { receipt_id } => {
                !receipt_id.trim().is_empty()
            }
            Self::Unknown => false,
        }
    }

    pub fn receipt_id(&self) -> Option<&str> {
        match self {
            Self::JointBlocksModelled { receipt_id }
            | Self::DistinctBlocksIndependenceAttested { receipt_id } => Some(receipt_id),
            Self::Unknown => None,
        }
    }

    fn validate_receipt(&self) -> Result<(), ModelError> {
        match self {
            Self::JointBlocksModelled { receipt_id }
            | Self::DistinctBlocksIndependenceAttested { receipt_id }
                if receipt_id.trim().is_empty() =>
            {
                Err(ModelError::InvalidParameter(
                    "inter_source_dependence_receipt_must_be_nonempty",
                ))
            }
            _ => Ok(()),
        }
    }
}


/// Independent observation noise and shared-cause sensitivities for one experiment.
#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentNoiseModel {
    pub experiment_id: String,
    pub independent_observation_covariance: Vec<Vec<f64>>,
    pub shared_sources: Vec<SharedUncertaintyContribution>,
    /// False means unrecorded common-cause dependencies may remain.
    /// It never means those omitted dependencies have zero covariance.
    pub lineage_coverage_complete: bool,
    /// Portfolio-scope qualification for dependence between different source IDs.
    /// The receipt must cover the complete set of IDs in the assembled portfolio.
    /// Distinct IDs are otherwise mathematically added as independent blocks.
    pub inter_source_dependence: InterSourceDependenceQualification,
}

/// Joint covariance assembled in the order of the supplied experiment list.
#[derive(Debug, Clone, PartialEq)]
pub struct JointExperimentCovariance {
    pub covariance: Vec<Vec<f64>>,
    /// Half-open observation ranges, one per experiment, in input order.
    pub experiment_ranges: Vec<(usize, usize)>,
    /// False if any input experiment reports incomplete common-cause provenance.
    pub lineage_coverage_complete: bool,
    /// False if any experiment has unknown inter-source dependence.
    pub inter_source_dependence_complete: bool,
    /// Sorted non-empty receipts that support inter-source qualification.
    pub inter_source_dependence_receipt_ids: Vec<String>,
    /// Sorted IDs of shared sources represented in the joint covariance.
    pub shared_source_ids: Vec<String>,
}

/// Assemble the joint covariance for a portfolio of experiments.
///
/// The diagonal observation blocks contain each experiment's independent
/// covariance plus its shared-source marginal contributions. Off-diagonal
/// blocks contain cross-covariance induced by sources used by both experiments.
pub fn assemble_joint_experiment_covariance(
    experiments: &[ExperimentNoiseModel],
) -> Result<JointExperimentCovariance, ModelError> {
    if experiments.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }

    let mut experiment_ids = BTreeSet::new();
    let mut source_definitions: BTreeMap<String, Vec<Vec<f64>>> = BTreeMap::new();
    let mut observation_counts = Vec::with_capacity(experiments.len());
    let mut total_observations = 0usize;

    for experiment in experiments {
        experiment.inter_source_dependence.validate_receipt()?;
        if experiment.experiment_id.trim().is_empty()
            || !experiment_ids.insert(experiment.experiment_id.as_str())
        {
            return Err(ModelError::InvalidParameter(
                "experiment_ids_must_be_nonempty_and_unique",
            ));
        }
        let observation_count = experiment.independent_observation_covariance.len();
        if observation_count == 0
            || experiment
                .independent_observation_covariance
                .iter()
                .any(|row| row.len() != observation_count)
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        validate_covariance(
            &experiment.independent_observation_covariance,
            "independent_observation_covariance",
        )?;
        observation_counts.push(observation_count);
        total_observations = total_observations.checked_add(observation_count).ok_or(
            ModelError::InvalidParameter("joint_observation_dimension_overflow"),
        )?;

        let mut local_source_ids = BTreeSet::new();
        for contribution in &experiment.shared_sources {
            if contribution.source_id.trim().is_empty()
                || !local_source_ids.insert(contribution.source_id.as_str())
            {
                return Err(ModelError::InvalidParameter(
                    "shared_source_ids_must_be_nonempty_and_unique_per_experiment",
                ));
            }
            validate_covariance(&contribution.source_covariance, "shared_source_covariance")?;
            let source_dimension = contribution.source_covariance.len();
            if contribution.observation_jacobian.len() != observation_count
                || contribution
                    .observation_jacobian
                    .iter()
                    .any(|row| row.len() != source_dimension)
            {
                return Err(ModelError::MismatchedDataLengths);
            }
            if contribution
                .observation_jacobian
                .iter()
                .flatten()
                .any(|value| !value.is_finite())
            {
                return Err(ModelError::NonFiniteValue(
                    "shared_source_observation_jacobian",
                ));
            }

            if let Some(existing) = source_definitions.get(&contribution.source_id) {
                if !matrices_approximately_equal(existing, &contribution.source_covariance) {
                    return Err(ModelError::InvalidCovariance(
                        "shared_source_covariance_definition_mismatch",
                    ));
                }
            } else {
                source_definitions.insert(
                    contribution.source_id.clone(),
                    contribution.source_covariance.clone(),
                );
            }
        }
    }

    let mut experiment_ranges = Vec::with_capacity(experiments.len());
    let mut offset = 0usize;
    for count in &observation_counts {
        let end = offset + count;
        experiment_ranges.push((offset, end));
        offset = end;
    }

    let mut joint = vec![vec![0.0; total_observations]; total_observations];
    for (experiment_index, experiment) in experiments.iter().enumerate() {
        let (start, _) = experiment_ranges[experiment_index];
        for (row_index, row) in experiment
            .independent_observation_covariance
            .iter()
            .enumerate()
        {
            for (column_index, value) in row.iter().enumerate() {
                joint[start + row_index][start + column_index] += value;
            }
        }
    }

    // Iterate only over the upper block triangle. Mirror the off-diagonal
    // covariance block explicitly to preserve exact numerical symmetry.
    for (left_index, left_experiment) in experiments.iter().enumerate() {
        let (left_start, left_end) = experiment_ranges[left_index];
        for (right_index, right_experiment) in experiments.iter().enumerate().skip(left_index) {
            let (right_start, right_end) = experiment_ranges[right_index];

            for left_source in &left_experiment.shared_sources {
                let Some(right_source) = right_experiment
                    .shared_sources
                    .iter()
                    .find(|source| source.source_id == left_source.source_id)
                else {
                    continue;
                };

                let left_times_source = matrix_multiply(
                    &left_source.observation_jacobian,
                    &left_source.source_covariance,
                )?;
                let cross = matrix_multiply(
                    &left_times_source,
                    &transpose(&right_source.observation_jacobian),
                )?;
                if cross.len() != left_end - left_start
                    || cross.iter().any(|row| row.len() != right_end - right_start)
                {
                    return Err(ModelError::MismatchedDataLengths);
                }
                for (row_index, row) in cross.iter().enumerate() {
                    for (column_index, value) in row.iter().enumerate() {
                        joint[left_start + row_index][right_start + column_index] += value;
                        if left_index != right_index {
                            joint[right_start + column_index][left_start + row_index] += value;
                        }
                    }
                }
            }
        }
    }

    if joint.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidCovariance(
            "joint_experiment_covariance_nonfinite",
        ));
    }
    validate_covariance(&joint, "joint_experiment_covariance")?;

    Ok(JointExperimentCovariance {
        covariance: joint,
        experiment_ranges,
        lineage_coverage_complete: experiments
            .iter()
            .all(|experiment| experiment.lineage_coverage_complete),
        inter_source_dependence_complete: {
            let receipts: BTreeSet<&str> = experiments
                .iter()
                .filter_map(|experiment| experiment.inter_source_dependence.receipt_id())
                .collect();
            receipts.len() == 1
                && experiments
                    .iter()
                    .all(|experiment| experiment.inter_source_dependence.is_qualified())
        },
        inter_source_dependence_receipt_ids: experiments
            .iter()
            .filter_map(|experiment| experiment.inter_source_dependence.receipt_id())
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        shared_source_ids: source_definitions.into_keys().collect(),
    })
}

fn matrices_approximately_equal(left: &[Vec<f64>], right: &[Vec<f64>]) -> bool {
    if left.len() != right.len()
        || left.iter().any(|row| row.len() != right.len())
        || right.iter().any(|row| row.len() != left.len())
    {
        return false;
    }
    let scale = left
        .iter()
        .flatten()
        .chain(right.iter().flatten())
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let tolerance = 1.0e-10 * scale.max(f64::MIN_POSITIVE);
    left.iter().zip(right.iter()).all(|(left_row, right_row)| {
        left_row
            .iter()
            .zip(right_row.iter())
            .all(|(left, right)| (*left - *right).abs() <= tolerance)
    })
}

fn validate_covariance(matrix: &[Vec<f64>], name: &'static str) -> Result<(), ModelError> {
    let dimension = matrix.len();
    if dimension == 0 || matrix.iter().any(|row| row.len() != dimension) {
        return Err(ModelError::MismatchedDataLengths);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue(name));
    }
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let tolerance = 1.0e-10 * scale.max(f64::MIN_POSITIVE);
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate().take(row_index) {
            if (*value - matrix[column_index][row_index]).abs() > tolerance {
                return Err(ModelError::InvalidCovariance(name));
            }
        }
    }

    // Semidefinite Cholesky validation: zero pivots are allowed only when the
    // associated residual off-diagonal terms are also numerically zero.
    let mut lower = vec![vec![0.0; dimension]; dimension];
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, entry) in row.iter().enumerate().take(row_index + 1) {
            let correction: f64 = lower[row_index][..column_index]
                .iter()
                .zip(lower[column_index][..column_index].iter())
                .map(|(left, right)| *left * *right)
                .sum();
            let value = *entry - correction;
            if row_index == column_index {
                if value < -tolerance {
                    return Err(ModelError::InvalidCovariance(name));
                }
                lower[row_index][column_index] = if value > 0.0 { value.sqrt() } else { 0.0 };
            } else if lower[column_index][column_index] > 0.0 {
                lower[row_index][column_index] = value / lower[column_index][column_index];
            } else if value.abs() > tolerance {
                return Err(ModelError::InvalidCovariance(name));
            }
        }
    }
    Ok(())
}

fn matrix_multiply(left: &[Vec<f64>], right: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    if left.is_empty() || right.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let inner = left[0].len();
    let columns = right[0].len();
    if inner == 0
        || columns == 0
        || left.iter().any(|row| row.len() != inner)
        || right.len() != inner
        || right.iter().any(|row| row.len() != columns)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut output = vec![vec![0.0; columns]; left.len()];
    for (row_index, output_row) in output.iter_mut().enumerate() {
        for (column_index, output_value) in output_row.iter_mut().enumerate() {
            *output_value = left[row_index]
                .iter()
                .zip(right.iter().map(|row| row[column_index]))
                .map(|(left, right)| *left * right)
                .sum();
        }
    }
    Ok(output)
}

fn transpose(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = matrix.len();
    let columns = matrix.first().map_or(0, Vec::len);
    let mut output = vec![vec![0.0; rows]; columns];
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            output[column_index][row_index] = *value;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_experiments_remain_block_diagonal_without_shared_sources() {
        let result = assemble_joint_experiment_covariance(&[
            ExperimentNoiseModel {
                experiment_id: "exp-a".to_owned(),
                independent_observation_covariance: vec![vec![2.0]],
                shared_sources: Vec::new(),
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "fixture:joint-source-model-v1",
                    ),
            },
            ExperimentNoiseModel {
                experiment_id: "exp-b".to_owned(),
                independent_observation_covariance: vec![vec![3.0]],
                shared_sources: Vec::new(),
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "fixture:joint-source-model-v1",
                    ),
            },
        ])
        .expect("independent portfolio");
        assert_eq!(result.covariance, vec![vec![2.0, 0.0], vec![0.0, 3.0]]);
        assert!(result.shared_source_ids.is_empty());
        assert_eq!(result.experiment_ranges, vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn shared_calibration_creates_cross_experiment_covariance() {
        let shared = |sensitivity: f64| SharedUncertaintyContribution {
            source_id: "detector-calibration-v1".to_owned(),
            source_covariance: vec![vec![4.0]],
            observation_jacobian: vec![vec![sensitivity]],
        };
        let result = assemble_joint_experiment_covariance(&[
            ExperimentNoiseModel {
                experiment_id: "exp-a".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: vec![shared(2.0)],
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "fixture:joint-source-model-v1",
                    ),
            },
            ExperimentNoiseModel {
                experiment_id: "exp-b".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: vec![shared(3.0)],
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "fixture:joint-source-model-v1",
                    ),
            },
        ])
        .expect("shared calibration portfolio");
        assert_eq!(result.covariance, vec![vec![17.0, 24.0], vec![24.0, 37.0]]);
        assert_eq!(result.shared_source_ids, vec!["detector-calibration-v1"]);
    }

    #[test]
    fn multiple_shared_sources_add_independent_cross_covariance_terms() {
        let contribution =
            |source_id: &str, variance: f64, sensitivity: f64| SharedUncertaintyContribution {
                source_id: source_id.to_owned(),
                source_covariance: vec![vec![variance]],
                observation_jacobian: vec![vec![sensitivity]],
            };
        let result = assemble_joint_experiment_covariance(&[
            ExperimentNoiseModel {
                experiment_id: "exp-a".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: vec![
                    contribution("reference", 2.0, 1.0),
                    contribution("response", 3.0, 2.0),
                ],
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::distinct_blocks_independence_attested(
                        "fixture:independent-source-blocks-v1",
                    ),
            },
            ExperimentNoiseModel {
                experiment_id: "exp-b".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: vec![
                    contribution("reference", 2.0, 2.0),
                    contribution("response", 3.0, 1.0),
                ],
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::distinct_blocks_independence_attested(
                        "fixture:independent-source-blocks-v1",
                    ),
            },
        ])
        .expect("multiple shared sources");
        assert_eq!(result.covariance, vec![vec![15.0, 10.0], vec![10.0, 12.0]]);
    }

    #[test]
    fn repeated_source_id_requires_identical_covariance_definition() {
        let result = assemble_joint_experiment_covariance(&[
            ExperimentNoiseModel {
                experiment_id: "exp-a".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: vec![SharedUncertaintyContribution {
                    source_id: "common".to_owned(),
                    source_covariance: vec![vec![2.0]],
                    observation_jacobian: vec![vec![1.0]],
                }],
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "fixture:joint-source-model-v1",
                    ),
            },
            ExperimentNoiseModel {
                experiment_id: "exp-b".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: vec![SharedUncertaintyContribution {
                    source_id: "common".to_owned(),
                    source_covariance: vec![vec![3.0]],
                    observation_jacobian: vec![vec![1.0]],
                }],
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "fixture:joint-source-model-v1",
                    ),
            },
        ]);
        assert_eq!(
            result,
            Err(ModelError::InvalidCovariance(
                "shared_source_covariance_definition_mismatch",
            ))
        );
    }

    #[test]
    fn incomplete_lineage_coverage_is_not_reported_as_independence() {
        let result = assemble_joint_experiment_covariance(&[
            ExperimentNoiseModel {
                experiment_id: "exp-a".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: Vec::new(),
                lineage_coverage_complete: false,
                inter_source_dependence: InterSourceDependenceQualification::Unknown,
            },
            ExperimentNoiseModel {
                experiment_id: "exp-b".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: Vec::new(),
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "fixture:joint-source-model-v1",
                    ),
            },
        ])
        .expect("partial-lineage portfolio");
        assert_eq!(result.covariance, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
        assert!(!result.lineage_coverage_complete);
    }

    #[test]
    fn joint_source_block_preserves_correlations_between_calibration_components() {
        let shared = |experiment_id: &str, jacobian: Vec<f64>| ExperimentNoiseModel {
            experiment_id: experiment_id.to_owned(),
            independent_observation_covariance: vec![vec![1.0]],
            shared_sources: vec![SharedUncertaintyContribution {
                source_id: "joint-calibration-v1".to_owned(),
                source_covariance: vec![vec![4.0, 1.5], vec![1.5, 9.0]],
                observation_jacobian: vec![jacobian],
            }],
            lineage_coverage_complete: true,
            inter_source_dependence:
                InterSourceDependenceQualification::joint_blocks_modelled(
                    "fixture:joint-source-model-v1",
                ),
        };
        let result = assemble_joint_experiment_covariance(&[
            shared("experiment-a", vec![1.0, 0.5]),
            shared("experiment-b", vec![0.25, 1.0]),
        ])
        .expect("joint correlated calibration block");
        let expected = vec![vec![8.75, 7.1875], vec![7.1875, 11.0]];
        for (actual_row, expected_row) in result.covariance.iter().zip(expected.iter()) {
            for (actual, expected) in actual_row.iter().zip(expected_row.iter()) {
                assert!((actual - expected).abs() < 1.0e-12);
            }
        }
        assert!(result.inter_source_dependence_complete);
        assert_eq!(
            result.inter_source_dependence_receipt_ids,
            vec!["fixture:joint-source-model-v1"]
        );
    }

    #[test]
    fn distinct_source_ids_do_not_imply_attested_independence() {
        let result = assemble_joint_experiment_covariance(&[
            ExperimentNoiseModel {
                experiment_id: "experiment-a".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: vec![
                    SharedUncertaintyContribution {
                        source_id: "detector-calibration".to_owned(),
                        source_covariance: vec![vec![2.0]],
                        observation_jacobian: vec![vec![1.0]],
                    },
                    SharedUncertaintyContribution {
                        source_id: "reference-evaluation".to_owned(),
                        source_covariance: vec![vec![3.0]],
                        observation_jacobian: vec![vec![1.0]],
                    },
                ],
                lineage_coverage_complete: true,
                inter_source_dependence: InterSourceDependenceQualification::Unknown,
            },
        ])
        .expect("conditional covariance with unknown inter-source dependence");
        assert!(result.lineage_coverage_complete);
        assert!(!result.inter_source_dependence_complete);
        assert!(result.inter_source_dependence_receipt_ids.is_empty());
        // The known marginal contributions remain numerically represented, but
        // the output is not an independence-qualified evidence model.
        assert_eq!(result.covariance, vec![vec![6.0]]);
    }

    #[test]
    fn inter_source_receipt_must_cover_the_whole_joint_portfolio() {
        let contribution = |source_id: &str| SharedUncertaintyContribution {
            source_id: source_id.to_owned(),
            source_covariance: vec![vec![2.0]],
            observation_jacobian: vec![vec![1.0]],
        };
        let result = assemble_joint_experiment_covariance(&[
            ExperimentNoiseModel {
                experiment_id: "exp-detector".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: vec![contribution("detector-calibration")],
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "receipt:detector-only",
                    ),
            },
            ExperimentNoiseModel {
                experiment_id: "exp-reference".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: vec![contribution("reference-evaluation")],
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "receipt:reference-only",
                    ),
            },
        ])
        .expect("known marginal source covariance");
        assert!(!result.inter_source_dependence_complete);
        assert_eq!(
            result.inter_source_dependence_receipt_ids,
            vec!["receipt:detector-only", "receipt:reference-only"]
        );
        assert_eq!(result.covariance, vec![vec![3.0, 0.0], vec![0.0, 3.0]]);
    }

    #[test]
    fn empty_inter_source_receipt_fails_closed() {
        let result = assemble_joint_experiment_covariance(&[ExperimentNoiseModel {
            experiment_id: "exp-empty-receipt".to_owned(),
            independent_observation_covariance: vec![vec![1.0]],
            shared_sources: Vec::new(),
            lineage_coverage_complete: true,
            inter_source_dependence:
                InterSourceDependenceQualification::joint_blocks_modelled("  "),
        }]);
        assert_eq!(
            result,
            Err(ModelError::InvalidParameter(
                "inter_source_dependence_receipt_must_be_nonempty"
            ))
        );
    }

    #[test]
    fn duplicate_experiment_or_source_ids_are_rejected() {
        let result = assemble_joint_experiment_covariance(&[
            ExperimentNoiseModel {
                experiment_id: "same".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: Vec::new(),
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "fixture:joint-source-model-v1",
                    ),
            },
            ExperimentNoiseModel {
                experiment_id: "same".to_owned(),
                independent_observation_covariance: vec![vec![1.0]],
                shared_sources: Vec::new(),
                lineage_coverage_complete: true,
                inter_source_dependence:
                    InterSourceDependenceQualification::joint_blocks_modelled(
                        "fixture:joint-source-model-v1",
                    ),
            },
        ]);
        assert_eq!(
            result,
            Err(ModelError::InvalidParameter(
                "experiment_ids_must_be_nonempty_and_unique",
            ))
        );
    }
    #[test]
    fn common_cause_batch_design_oracle_reverses_naive_portfolio_ranking() {
        let shared_calibration = || SharedUncertaintyContribution {
            source_id: "calibration:A-B".to_owned(),
            source_covariance: vec![vec![9.0]],
            observation_jacobian: vec![vec![1.0]],
        };
        let experiment_a = ExperimentNoiseModel {
            experiment_id: "A".to_owned(),
            independent_observation_covariance: vec![vec![0.1]],
            shared_sources: vec![shared_calibration()],
            lineage_coverage_complete: true,
            inter_source_dependence:
                InterSourceDependenceQualification::joint_blocks_modelled(
                    "fixture:joint-source-model-v1",
                ),
        };
        let experiment_b = ExperimentNoiseModel {
            experiment_id: "B".to_owned(),
            independent_observation_covariance: vec![vec![0.1]],
            shared_sources: vec![shared_calibration()],
            lineage_coverage_complete: true,
            inter_source_dependence:
                InterSourceDependenceQualification::joint_blocks_modelled(
                    "fixture:joint-source-model-v1",
                ),
        };
        let experiment_c = ExperimentNoiseModel {
            experiment_id: "C".to_owned(),
            independent_observation_covariance: vec![vec![20.0]],
            shared_sources: Vec::new(),
            lineage_coverage_complete: true,
            inter_source_dependence:
                InterSourceDependenceQualification::joint_blocks_modelled(
                    "fixture:joint-source-model-v1",
                ),
        };

        let shared_portfolio =
            assemble_joint_experiment_covariance(&[experiment_a.clone(), experiment_b.clone()])
                .expect("shared A+B portfolio");
        let complementary_portfolio =
            assemble_joint_experiment_covariance(&[experiment_a, experiment_c])
                .expect("complementary A+C portfolio");

        let close = |left: f64, right: f64| (left - right).abs() < 1.0e-12;
        assert!(close(shared_portfolio.covariance[0][0], 9.1));
        assert!(close(shared_portfolio.covariance[0][1], 9.0));
        assert!(close(shared_portfolio.covariance[1][0], 9.0));
        assert!(close(shared_portfolio.covariance[1][1], 9.1));
        assert!(shared_portfolio.lineage_coverage_complete);

        assert!(close(complementary_portfolio.covariance[0][0], 9.1));
        assert!(close(complementary_portfolio.covariance[0][1], 0.0));
        assert!(close(complementary_portfolio.covariance[1][0], 0.0));
        assert!(close(complementary_portfolio.covariance[1][1], 20.0));
        assert!(complementary_portfolio.lineage_coverage_complete);

        // Exact linear-Gaussian posterior oracle with prior covariance I_2.
        let shared_a_utility = 20.0 / 201.0;
        let shared_d_utility = (201.0_f64 / 181.0).ln();
        let complementary_a_utility = 311.0 / 2121.0;
        let complementary_d_utility = (2121.0_f64 / 1820.0).ln();
        assert!(complementary_a_utility > shared_a_utility);
        assert!(complementary_d_utility > shared_d_utility);

        // Negative control: retain each marginal variance but drop known covariance.
        let naive_independence_portfolio = assemble_joint_experiment_covariance(&[
            ExperimentNoiseModel {
                experiment_id: "A-naive".to_owned(),
                independent_observation_covariance: vec![vec![9.1]],
                shared_sources: Vec::new(),
                lineage_coverage_complete: false,
                inter_source_dependence: InterSourceDependenceQualification::Unknown,
            },
            ExperimentNoiseModel {
                experiment_id: "B-naive".to_owned(),
                independent_observation_covariance: vec![vec![9.1]],
                shared_sources: Vec::new(),
                lineage_coverage_complete: false,
                inter_source_dependence: InterSourceDependenceQualification::Unknown,
            },
        ])
        .expect("naive independence negative control");
        assert!(close(naive_independence_portfolio.covariance[0][1], 0.0));
        assert!(!naive_independence_portfolio.lineage_coverage_complete);

        let naive_a_utility = 20.0 / 111.0;
        let naive_d_utility = (111.0_f64 / 91.0).ln();
        assert!(naive_a_utility > complementary_a_utility);
        assert!(naive_d_utility > complementary_d_utility);
    }
}
