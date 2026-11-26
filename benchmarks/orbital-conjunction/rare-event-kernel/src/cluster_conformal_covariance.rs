// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Cluster-level split-conformal calibration of covariance-shaped ellipsoids.
//!
//! The declared covariance determines ellipsoid shape. A calibration block
//! contributes one nonconformity score: the maximum NIS across every residual
//! observation in that fixed-length block. The split-conformal order statistic
//! is then used as the NIS radius for a future exchangeable block.
//!
//! This preserves arbitrary within-block dependence. Finite-sample marginal
//! block coverage is conditional on exchangeability of calibration and future
//! block-level scores. The code cannot establish that assumption or guarantee
//! coverage under drift between calibration and evaluation regimes.

use std::collections::{BTreeMap, BTreeSet};

use crate::covariance_realism::{
    CovarianceRealismError, CovarianceResidualObservation, chi_square_quantile,
    whiten_residual,
};

/// Maximum number of held-out calibration residual observations.
pub const MAX_CLUSTER_CONFORMAL_CALIBRATION_OBSERVATIONS: usize = 1_000_000;

/// Nonconformity score for one complete calibration dependence block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClusterMaximumNis {
    /// Stable identity of the calibration dependence block.
    pub correlation_cluster_id: [u8; 32],
    /// Maximum NIS over the fixed observations in this block.
    pub maximum_nis: f64,
    /// Number of residual observations aggregated into this block score.
    pub observation_count: usize,
}

/// Frozen split and block contract for one covariance ellipsoid calibrator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClusterConformalCalibrationConfig {
    /// Nonzero identity of the held-out calibration partition.
    pub calibration_partition_id: [u8; 32],
    /// Nonzero identity of the future evaluation partition; must differ from calibration.
    pub evaluation_partition_id: [u8; 32],
    /// Nonzero identity for fixed horizon, cadence, regime and block-construction semantics.
    pub block_contract_id: [u8; 32],
    /// Exact number of residual observations required in each dependence block.
    pub observations_per_cluster: usize,
    /// Desired block-level marginal miscoverage probability.
    pub alpha: f64,
}

/// One covariance residual explicitly assigned to a calibration split and block contract.
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterConformalResidualObservation {
    /// Partition identity; must equal the frozen calibration partition ID.
    pub partition_id: [u8; 32],
    /// Block-contract identity; must equal the frozen block-contract ID.
    pub block_contract_id: [u8; 32],
    /// Prediction/truth residual and covariance provenance.
    pub residual: CovarianceResidualObservation,
}

/// Result of split-conformal calibration of one covariance ellipsoid.
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterConformalEllipsoidCalibration {
    /// Requested block-level miscoverage probability.
    pub alpha: f64,
    /// Number of complete exchangeability units / dependence blocks.
    pub cluster_count: usize,
    /// Number of residual observations represented by the calibration blocks.
    pub observation_count: usize,
    /// Frozen number of residual observations per block.
    pub observations_per_cluster: usize,
    /// Split-conformal order-statistic rank, one-based.
    pub order_statistic_rank: usize,
    /// Present only when the calibration sample supports a finite order statistic.
    ///
    /// None means the required rank is greater than the number of calibration
    /// blocks; the correct conformal set is unbounded at this alpha.
    pub calibrated_max_nis_threshold: Option<f64>,
    /// Gaussian chi-square NIS threshold at the same nominal coverage.
    pub gaussian_reference_nis_threshold: f64,
    /// Calibrated NIS threshold divided by the Gaussian reference threshold.
    pub quadratic_inflation_factor: Option<f64>,
    /// Square root of the quadratic inflation factor.
    pub radius_inflation_factor: Option<f64>,
    /// Common density-space identity.
    pub density_space_id: [u8; 32],
    /// Common stratum identity.
    pub stratum_id: [u8; 32],
    /// Residual dimension / chi-square degrees of freedom.
    pub dimension: usize,
    /// Calibration split identity.
    pub calibration_partition_id: [u8; 32],
    /// Declared disjoint evaluation split identity.
    pub evaluation_partition_id: [u8; 32],
    /// Frozen block-construction contract identity.
    pub block_contract_id: [u8; 32],
    /// Cluster maximum NIS values in canonical cluster-ID order.
    pub cluster_scores_by_identity: Vec<ClusterMaximumNis>,
    /// Explicit finite-sample guarantee assumption.
    pub guarantee_assumption: &'static str,
}

/// Validation or numerical errors for cluster-level conformal calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterConformalCalibrationError {
    /// Calibration observation set is empty.
    EmptyCalibrationSet,
    /// Calibration residual count exceeds the configured bound.
    TooManyCalibrationObservations,
    /// Calibration/evaluation, block-contract or other required identity is zero.
    InvalidIdentity,
    /// Calibration and evaluation partition IDs must be distinct.
    CalibrationEvaluationPartitionCollision,
    /// Fixed observations-per-cluster count must be positive.
    InvalidBlockSize,
    /// A residual component is not finite.
    NonFiniteResidual,
    /// Observation is labelled with a different calibration partition.
    CalibrationPartitionMismatch,
    /// Alpha is not finite or not strictly between zero and one.
    InvalidAlpha,
    /// Observation is labelled with a different block contract.
    BlockContractMismatch,
    /// Observations mix density spaces, strata or residual dimensions.
    MixedCalibrationGroup,
    /// Observation identity is duplicated.
    DuplicateObservationIdentity,
    /// Cluster contains more observations than the frozen block contract.
    ClusterExceedsBlockSize,
    /// Cluster contains fewer observations than the frozen block contract.
    IncompleteCalibrationCluster,
    /// Covariance reference mean is nonzero; no implicit residual centering is allowed.
    NonZeroCovarianceReferenceMean,
    /// Residual/covariance dimensions differ.
    DimensionMismatch,
    /// Cholesky whitening or a derived NIS score failed numerically.
    NumericalFailure,
    /// Existing covariance-realism primitive rejected an input.
    CovarianceRealismFailure,
}

impl core::fmt::Display for ClusterConformalCalibrationError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::EmptyCalibrationSet => "cluster-conformal calibration set is empty",
            Self::TooManyCalibrationObservations => "cluster-conformal calibration exceeds its observation limit",
            Self::InvalidIdentity => "calibration, evaluation or block-contract identity is zero",
            Self::CalibrationEvaluationPartitionCollision => "calibration and evaluation partitions must differ",
            Self::InvalidBlockSize => "observations-per-cluster must be strictly positive",
            Self::CalibrationPartitionMismatch => "residual does not belong to the frozen calibration partition",
            Self::BlockContractMismatch => "residual does not belong to the frozen block contract",
            Self::InvalidAlpha => "conformal miscoverage alpha must lie strictly between zero and one",
            Self::MixedCalibrationGroup => "calibration mixes density spaces, strata or residual dimensions",
            Self::DuplicateObservationIdentity => "calibration residual observation identity is duplicated",
            Self::ClusterExceedsBlockSize => "dependence cluster exceeds its frozen observation count",
            Self::IncompleteCalibrationCluster => "dependence cluster is incomplete under the frozen block contract",
            Self::NonZeroCovarianceReferenceMean => "covariance reference mean must be exactly zero",
            Self::DimensionMismatch => "residual and covariance dimensions differ",
            Self::NonFiniteResidual => "residual contains a non-finite component",
            Self::NumericalFailure => "cluster NIS or conformal quantile calculation failed numerically",
            Self::CovarianceRealismFailure => "shared covariance-realism primitive rejected an input",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ClusterConformalCalibrationError {}

impl From<CovarianceRealismError> for ClusterConformalCalibrationError {
    fn from(_: CovarianceRealismError) -> Self {
        Self::CovarianceRealismFailure
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CalibrationGroup {
    density_space_id: [u8; 32],
    stratum_id: [u8; 32],
    dimension: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ClusterAccumulator {
    count: usize,
    maximum_nis: f64,
}

/// Calibrate one ellipsoid radius using maximum NIS per fixed dependence block.
///
/// Calibration and evaluation partition IDs are bound into the output, but
/// their semantic disjointness is the caller's responsibility. A finite
/// split-conformal guarantee requires exchangeability of complete block
/// scores, not independence of individual residual rows.
pub fn calibrate_cluster_conformal_ellipsoid(
    config: ClusterConformalCalibrationConfig,
    observations: &[ClusterConformalResidualObservation],
) -> Result<ClusterConformalEllipsoidCalibration, ClusterConformalCalibrationError> {
    if config.calibration_partition_id.iter().all(|byte| *byte == 0)
        || config.evaluation_partition_id.iter().all(|byte| *byte == 0)
        || config.block_contract_id.iter().all(|byte| *byte == 0)
    {
        return Err(ClusterConformalCalibrationError::InvalidIdentity);
    }
    if config.calibration_partition_id == config.evaluation_partition_id {
        return Err(ClusterConformalCalibrationError::CalibrationEvaluationPartitionCollision);
    }
    if config.observations_per_cluster == 0 {
        return Err(ClusterConformalCalibrationError::InvalidBlockSize);
    }
    if !config.alpha.is_finite() || config.alpha <= 0.0 || config.alpha >= 1.0 {
        return Err(ClusterConformalCalibrationError::InvalidAlpha);
    }
    if observations.is_empty() {
        return Err(ClusterConformalCalibrationError::EmptyCalibrationSet);
    }
    if observations.len() > MAX_CLUSTER_CONFORMAL_CALIBRATION_OBSERVATIONS {
        return Err(ClusterConformalCalibrationError::TooManyCalibrationObservations);
    }

    let mut observation_ids = BTreeSet::new();
    let mut clusters: BTreeMap<[u8; 32], ClusterAccumulator> = BTreeMap::new();
    let mut calibration_group: Option<CalibrationGroup> = None;

    for item in observations {
        if item.partition_id.iter().all(|byte| *byte == 0)
            || item.block_contract_id.iter().all(|byte| *byte == 0)
            || item.residual.observation_id.iter().all(|byte| *byte == 0)
            || item.residual.correlation_cluster_id.iter().all(|byte| *byte == 0)
            || item.residual.stratum_id.iter().all(|byte| *byte == 0)
            || item.residual.density_space_id.iter().all(|byte| *byte == 0)
        {
            return Err(ClusterConformalCalibrationError::InvalidIdentity);
        }
        if item.partition_id != config.calibration_partition_id {
            return Err(ClusterConformalCalibrationError::CalibrationPartitionMismatch);
        }
        if item.block_contract_id != config.block_contract_id {
            return Err(ClusterConformalCalibrationError::BlockContractMismatch);
        }
        if !observation_ids.insert(item.residual.observation_id) {
            return Err(ClusterConformalCalibrationError::DuplicateObservationIdentity);
        }

        let dimension = item.residual.covariance.dimension();
        if dimension == 0 || dimension > crate::MAX_COVARIANCE_REALISM_DIMENSION {
            return Err(ClusterConformalCalibrationError::DimensionMismatch);
        }
        if item.residual.residual.len() != dimension {
            return Err(ClusterConformalCalibrationError::DimensionMismatch);
        }
        if item.residual.residual.iter().any(|value| !value.is_finite()) {
            return Err(ClusterConformalCalibrationError::NonFiniteResidual);
        }
        if item
            .residual
            .covariance
            .mean()
            .iter()
            .any(|value| *value != 0.0)
        {
            return Err(ClusterConformalCalibrationError::NonZeroCovarianceReferenceMean);
        }

        let group = CalibrationGroup {
            density_space_id: item.residual.density_space_id,
            stratum_id: item.residual.stratum_id,
            dimension,
        };
        if let Some(expected) = calibration_group {
            if expected != group {
                return Err(ClusterConformalCalibrationError::MixedCalibrationGroup);
            }
        } else {
            calibration_group = Some(group);
        }

        let whitened = whiten_residual(
            item.residual.covariance.lower_cholesky_values(),
            &item.residual.residual,
            dimension,
        )?;
        let nis = whitened.iter().map(|value| value * value).sum::<f64>();
        if !nis.is_finite() || nis < 0.0 {
            return Err(ClusterConformalCalibrationError::NumericalFailure);
        }

        let accumulator = clusters
            .entry(item.residual.correlation_cluster_id)
            .or_insert(ClusterAccumulator {
                count: 0,
                maximum_nis: 0.0,
            });
        accumulator.count = accumulator
            .count
            .checked_add(1)
            .ok_or(ClusterConformalCalibrationError::NumericalFailure)?;
        if accumulator.count > config.observations_per_cluster {
            return Err(ClusterConformalCalibrationError::ClusterExceedsBlockSize);
        }
        accumulator.maximum_nis = accumulator.maximum_nis.max(nis);
    }

    if clusters
        .values()
        .any(|cluster| cluster.count != config.observations_per_cluster)
    {
        return Err(ClusterConformalCalibrationError::IncompleteCalibrationCluster);
    }
    let group = calibration_group.ok_or(ClusterConformalCalibrationError::EmptyCalibrationSet)?;
    let mut cluster_scores_by_identity = clusters
        .iter()
        .map(|(cluster_id, accumulator)| ClusterMaximumNis {
            correlation_cluster_id: *cluster_id,
            maximum_nis: accumulator.maximum_nis,
            observation_count: accumulator.count,
        })
        .collect::<Vec<_>>();

    let mut sorted_scores = cluster_scores_by_identity
        .iter()
        .map(|score| score.maximum_nis)
        .collect::<Vec<_>>();
    sorted_scores.sort_by(f64::total_cmp);
    let cluster_count = sorted_scores.len();
    let observation_count = observations.len();
    let raw_rank = ((cluster_count + 1) as f64 * (1.0 - config.alpha)).ceil();
    if !raw_rank.is_finite() || raw_rank < 1.0 {
        return Err(ClusterConformalCalibrationError::NumericalFailure);
    }
    let order_statistic_rank = raw_rank as usize;
    let calibrated_max_nis_threshold = if order_statistic_rank <= cluster_count {
        Some(sorted_scores[order_statistic_rank - 1])
    } else {
        None
    };
    let gaussian_reference_nis_threshold =
        chi_square_quantile(1.0 - config.alpha, group.dimension)?;
    let quadratic_inflation_factor = calibrated_max_nis_threshold
        .map(|threshold| threshold / gaussian_reference_nis_threshold);
    let radius_inflation_factor = quadratic_inflation_factor.map(f64::sqrt);

    // Keep canonical identity order and scores for independent replay.
    cluster_scores_by_identity.sort_unstable_by_key(|score| score.correlation_cluster_id);

    Ok(ClusterConformalEllipsoidCalibration {
        alpha: config.alpha,
        cluster_count,
        observation_count,
        observations_per_cluster: config.observations_per_cluster,
        order_statistic_rank,
        calibrated_max_nis_threshold,
        gaussian_reference_nis_threshold,
        quadratic_inflation_factor,
        radius_inflation_factor,
        density_space_id: group.density_space_id,
        stratum_id: group.stratum_id,
        dimension: group.dimension,
        calibration_partition_id: config.calibration_partition_id,
        evaluation_partition_id: config.evaluation_partition_id,
        block_contract_id: config.block_contract_id,
        cluster_scores_by_identity,
        guarantee_assumption: "marginal block-level coverage conditional on exchangeability of complete calibration and future block maximum-NIS scores under the frozen block contract",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gaussian;

    const CALIBRATION_PARTITION: [u8; 32] = [40; 32];
    const EVALUATION_PARTITION: [u8; 32] = [41; 32];
    const BLOCK_CONTRACT: [u8; 32] = [42; 32];
    const DENSITY_SPACE: [u8; 32] = [43; 32];
    const STRATUM: [u8; 32] = [44; 32];

    fn covariance_1d() -> Gaussian {
        Gaussian::new(vec![0.0], vec![1.0]).unwrap()
    }

    fn row(observation: u8, cluster: u8, value: f64) -> ClusterConformalResidualObservation {
        ClusterConformalResidualObservation {
            partition_id: CALIBRATION_PARTITION,
            block_contract_id: BLOCK_CONTRACT,
            residual: CovarianceResidualObservation {
                observation_id: [observation; 32],
                correlation_cluster_id: [cluster; 32],
                stratum_id: STRATUM,
                density_space_id: DENSITY_SPACE,
                residual: vec![value],
                covariance: covariance_1d(),
            },
        }
    }

    fn config(alpha: f64, observations_per_cluster: usize) -> ClusterConformalCalibrationConfig {
        ClusterConformalCalibrationConfig {
            calibration_partition_id: CALIBRATION_PARTITION,
            evaluation_partition_id: EVALUATION_PARTITION,
            block_contract_id: BLOCK_CONTRACT,
            observations_per_cluster,
            alpha,
        }
    }

    #[test]
    fn cluster_maxima_produce_the_exact_split_conformal_order_statistic() {
        let mut observations = Vec::new();
        for cluster in 1_u8..=9 {
            observations.push(row(cluster * 2 - 1, cluster, f64::from(cluster).sqrt()));
            observations.push(row(cluster * 2, cluster, 0.25));
        }
        let report =
            calibrate_cluster_conformal_ellipsoid(config(0.1, 2), &observations).unwrap();

        assert_eq!(report.cluster_count, 9);
        assert_eq!(report.observation_count, 18);
        assert_eq!(report.order_statistic_rank, 9);
        assert_eq!(report.calibrated_max_nis_threshold, Some(9.0));
        assert!(report.radius_inflation_factor.unwrap() > 1.0);
        assert_eq!(report.cluster_scores_by_identity[0].maximum_nis, 1.0);
        assert_eq!(report.cluster_scores_by_identity[8].maximum_nis, 9.0);
    }

    #[test]
    fn too_few_clusters_return_no_finite_threshold_instead_of_sample_maximum() {
        let observations = (1_u8..=9)
            .map(|cluster| row(cluster, cluster, f64::from(cluster)))
            .collect::<Vec<_>>();
        let report =
            calibrate_cluster_conformal_ellipsoid(config(0.05, 1), &observations).unwrap();

        assert_eq!(report.order_statistic_rank, 10);
        assert_eq!(report.calibrated_max_nis_threshold, None);
        assert_eq!(report.quadratic_inflation_factor, None);
        assert_eq!(report.radius_inflation_factor, None);
    }

    #[test]
    fn split_contract_and_block_size_must_match() {
        let mut mixed_partition = vec![row(1, 1, 0.0)];
        mixed_partition[0].partition_id = EVALUATION_PARTITION;
        assert_eq!(
            calibrate_cluster_conformal_ellipsoid(config(0.1, 1), &mixed_partition),
            Err(ClusterConformalCalibrationError::CalibrationPartitionMismatch)
        );

        let mut mixed_contract = vec![row(1, 1, 0.0)];
        mixed_contract[0].block_contract_id = [99; 32];
        assert_eq!(
            calibrate_cluster_conformal_ellipsoid(config(0.1, 1), &mixed_contract),
            Err(ClusterConformalCalibrationError::BlockContractMismatch)
        );

        assert_eq!(
            calibrate_cluster_conformal_ellipsoid(config(0.1, 2), &[row(1, 1, 0.0)]),
            Err(ClusterConformalCalibrationError::IncompleteCalibrationCluster)
        );
    }

    #[test]
    fn calibration_and_evaluation_partitions_must_be_distinct() {
        let observations = [row(1, 1, 0.0)];
        let mut invalid = config(0.1, 1);
        invalid.evaluation_partition_id = CALIBRATION_PARTITION;
        assert_eq!(
            calibrate_cluster_conformal_ellipsoid(invalid, &observations),
            Err(ClusterConformalCalibrationError::CalibrationEvaluationPartitionCollision)
        );
    }

    #[test]
    fn duplicate_residual_identity_is_rejected() {
        let observations = [row(1, 1, 0.0), row(1, 1, 1.0)];
        assert_eq!(
            calibrate_cluster_conformal_ellipsoid(config(0.1, 2), &observations),
            Err(ClusterConformalCalibrationError::DuplicateObservationIdentity)
        );
    }

    #[test]
    fn all_rows_in_one_cluster_use_the_maximum_not_an_iid_row_quantile() {
        let observations = [row(1, 1, 1.0), row(2, 1, 5.0), row(3, 1, 2.0)];
        let report =
            calibrate_cluster_conformal_ellipsoid(config(0.1, 3), &observations).unwrap();

        assert_eq!(report.cluster_count, 1);
        assert_eq!(report.calibrated_max_nis_threshold, None);
        assert_eq!(report.cluster_scores_by_identity[0].maximum_nis, 25.0);
    }

    #[test]
    fn input_order_does_not_change_calibration_or_cluster_score_receipt() {
        let observations = [
            row(1, 1, 1.0),
            row(2, 1, 2.0),
            row(3, 2, 3.0),
            row(4, 2, 1.0),
            row(5, 3, 0.5),
            row(6, 3, 0.25),
        ];
        let mut reversed = observations.clone();
        reversed.reverse();
        assert_eq!(
            calibrate_cluster_conformal_ellipsoid(config(0.1, 2), &observations),
            calibrate_cluster_conformal_ellipsoid(config(0.1, 2), &reversed)
        );
    }
}
