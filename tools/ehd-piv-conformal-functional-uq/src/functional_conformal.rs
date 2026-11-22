//! Whole-session conformal calibration for integrated field functionals.
//!
//! This calibrates the four-component output vector produced by the functional
//! reducer: mass flow and three advective momentum-flux components. Residuals
//! must be measured against independent reference functional evidence (e.g.
//! traceable flow and force/flux references). Calibrating the underlying PIV
//! vectors does not calibrate these integrated outputs automatically.
//!
//! Each independent acquisition/session cluster is scored by the maximum
//! 4D Mahalanobis residual over its complete predeclared set of functional
//! observations. The same split-conformal finite-sample order statistic then
//! provides whole-cluster coverage under exchangeability of complete sessions.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

const FUNCTIONAL_DIMENSION: usize = 4;

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionalObservation {
    pub observation_id: String,
    /// Independent reference evidence for all four functional components.
    pub reference_evidence_id: String,
    /// Reference minus predicted values in [kg/s, N, N, N].
    pub residual: [f64; FUNCTIONAL_DIMENSION],
    /// Full predictive covariance in the same mixed physical units.
    pub covariance: [[f64; FUNCTIONAL_DIMENSION]; FUNCTIONAL_DIMENSION],
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionalCalibrationCluster {
    /// Highest defensible independent laboratory session/common-cause unit.
    pub cluster_id: String,
    pub protocol_fingerprint: String,
    pub functional_contract_id: String,
    pub observations: Vec<FunctionalObservation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionalConformalInput {
    pub campaign_id: String,
    pub target_protocol_fingerprint: String,
    /// Frozen definition of the integrated functionals, units, surface and sign.
    pub functional_contract_id: String,
    pub exchangeability_attestation_id: String,
    pub alpha: f64,
    /// Exact number of functional reference observations in every session.
    pub expected_observations_per_cluster: usize,
    pub calibration_clusters: Vec<FunctionalCalibrationCluster>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionalConformalDisposition {
    FiniteSampleSessionGuaranteeAvailable,
    InsufficientCalibrationSessions,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionalClusterScore {
    pub cluster_id: String,
    pub maximum_squared_mahalanobis: f64,
    pub observation_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionalConformalReport {
    pub campaign_id: String,
    pub protocol_fingerprint: String,
    pub functional_contract_id: String,
    pub exchangeability_attestation_id: String,
    pub covariance_dimension: usize,
    pub alpha: f64,
    pub nominal_session_coverage: f64,
    pub calibration_session_count: usize,
    pub expected_observations_per_cluster: usize,
    pub quantile_rank: usize,
    pub calibrated_squared_radius: Option<f64>,
    pub finite_sample_coverage_lower_bound: Option<f64>,
    pub cluster_scores: Vec<FunctionalClusterScore>,
    pub disposition: FunctionalConformalDisposition,
    pub claim_scope: &'static str,
    pub limitations: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunctionalConformalError {
    EmptyIdentity { field: &'static str },
    InvalidMiscoverage,
    InvalidExpectedObservationCount,
    EmptyCalibrationSet,
    DuplicateClusterId { cluster_id: String },
    DuplicateObservationId { observation_id: String },
    DuplicateReferenceEvidenceId { evidence_id: String },
    ProtocolFingerprintMismatch { cluster_id: String },
    FunctionalContractMismatch { cluster_id: String },
    ObservationCountMismatch { cluster_id: String, expected: usize, actual: usize },
    NonFiniteResidual { observation_id: String },
    InvalidCovariance { observation_id: String },
    NonFiniteScore { observation_id: String },
}

impl fmt::Display for FunctionalConformalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity { field } => write!(f, "functional conformal {field} must be non-empty"),
            Self::InvalidMiscoverage => write!(f, "functional conformal alpha must be finite and in (0,1)"),
            Self::InvalidExpectedObservationCount => write!(f, "expected functional observation count must be positive"),
            Self::EmptyCalibrationSet => write!(f, "functional conformal calibration requires complete session clusters"),
            Self::DuplicateClusterId { cluster_id } => write!(f, "duplicate functional calibration cluster: {cluster_id}"),
            Self::DuplicateObservationId { observation_id } => write!(f, "duplicate functional observation: {observation_id}"),
            Self::DuplicateReferenceEvidenceId { evidence_id } => write!(f, "duplicate functional reference evidence: {evidence_id}"),
            Self::ProtocolFingerprintMismatch { cluster_id } => write!(f, "functional cluster {cluster_id} protocol fingerprint differs"),
            Self::FunctionalContractMismatch { cluster_id } => write!(f, "functional cluster {cluster_id} observable contract differs"),
            Self::ObservationCountMismatch { cluster_id, expected, actual } => write!(f, "functional cluster {cluster_id} has {actual} observations; expected {expected}"),
            Self::NonFiniteResidual { observation_id } => write!(f, "functional residual is non-finite for {observation_id}"),
            Self::InvalidCovariance { observation_id } => write!(f, "functional covariance is not symmetric positive definite for {observation_id}"),
            Self::NonFiniteScore { observation_id } => write!(f, "functional Mahalanobis score is non-finite for {observation_id}"),
        }
    }
}

impl Error for FunctionalConformalError {}

fn squared_mahalanobis_4(
    residual: [f64; FUNCTIONAL_DIMENSION],
    covariance: [[f64; FUNCTIONAL_DIMENSION]; FUNCTIONAL_DIMENSION],
    observation_id: &str,
) -> Result<f64, FunctionalConformalError> {
    if !residual.iter().all(|value| value.is_finite()) {
        return Err(FunctionalConformalError::NonFiniteResidual {
            observation_id: observation_id.to_owned(),
        });
    }

    if !covariance.iter().flatten().all(|value| value.is_finite()) {
        return Err(FunctionalConformalError::InvalidCovariance {
            observation_id: observation_id.to_owned(),
        });
    }

    // Functional components have heterogeneous units (kg/s versus N).
    // Standardize to a dimensionless correlation matrix before Cholesky;
    // otherwise pivot tolerances can be dominated by whichever unit is largest.
    let mut standard_deviations = [0.0_f64; FUNCTIONAL_DIMENSION];
    for index in 0..FUNCTIONAL_DIMENSION {
        let variance = covariance[index][index];
        if !variance.is_finite() || variance <= 0.0 {
            return Err(FunctionalConformalError::InvalidCovariance {
                observation_id: observation_id.to_owned(),
            });
        }
        standard_deviations[index] = variance.sqrt();
    }

    let mut correlation = [[0.0_f64; FUNCTIONAL_DIMENSION]; FUNCTIONAL_DIMENSION];
    for row in 0..FUNCTIONAL_DIMENSION {
        for column in 0..FUNCTIONAL_DIMENSION {
            let left = covariance[row][column];
            let right = covariance[column][row];
            let covariance_scale = standard_deviations[row] * standard_deviations[column];
            let symmetry_tolerance = 1.0e-10 * covariance_scale;
            if (left - right).abs() > symmetry_tolerance {
                return Err(FunctionalConformalError::InvalidCovariance {
                    observation_id: observation_id.to_owned(),
                });
            }
            correlation[row][column] =
                0.5 * (left + right) / covariance_scale;
        }
    }

    // Cholesky factorization validates positive definiteness and lets us
    // compute e^T C^-1 e as ||L^-1 D^-1 e||^2, where D contains standard
    // deviations. This scaling is important for mixed-unit functional vectors.
    let mut lower = [[0.0_f64; FUNCTIONAL_DIMENSION]; FUNCTIONAL_DIMENSION];
    for row in 0..FUNCTIONAL_DIMENSION {
        for column in 0..=row {
            let mut value = correlation[row][column];
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if row == column {
                if !value.is_finite() || value <= f64::EPSILON {
                    return Err(FunctionalConformalError::InvalidCovariance {
                        observation_id: observation_id.to_owned(),
                    });
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }

    let mut whitened = [0.0_f64; FUNCTIONAL_DIMENSION];
    for row in 0..FUNCTIONAL_DIMENSION {
        let mut value = residual[row] / standard_deviations[row];
        for column in 0..row {
            value -= lower[row][column] * whitened[column];
        }
        whitened[row] = value / lower[row][row];
    }
    let score = whitened.iter().map(|value| value * value).sum::<f64>();
    if !score.is_finite() || score < 0.0 {
        return Err(FunctionalConformalError::NonFiniteScore {
            observation_id: observation_id.to_owned(),
        });
    }
    Ok(score)
}

/// Calibrate whole-session simultaneous coverage for the integrated
/// [mass-flow, momentum-x, momentum-y, momentum-z] functional vector.
pub fn calibrate_functional_conformal(
    input: &FunctionalConformalInput,
) -> Result<FunctionalConformalReport, FunctionalConformalError> {
    for (field, value) in [
        ("campaign_id", input.campaign_id.as_str()),
        ("target_protocol_fingerprint", input.target_protocol_fingerprint.as_str()),
        ("functional_contract_id", input.functional_contract_id.as_str()),
        ("exchangeability_attestation_id", input.exchangeability_attestation_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(FunctionalConformalError::EmptyIdentity { field });
        }
    }
    if !input.alpha.is_finite() || input.alpha <= 0.0 || input.alpha >= 1.0 {
        return Err(FunctionalConformalError::InvalidMiscoverage);
    }
    if input.expected_observations_per_cluster == 0 {
        return Err(FunctionalConformalError::InvalidExpectedObservationCount);
    }
    if input.calibration_clusters.is_empty() {
        return Err(FunctionalConformalError::EmptyCalibrationSet);
    }

    let mut cluster_ids = BTreeSet::new();
    let mut observation_ids = BTreeSet::new();
    let mut reference_evidence_ids = BTreeSet::new();
    let mut cluster_scores = Vec::with_capacity(input.calibration_clusters.len());

    for cluster in &input.calibration_clusters {
        if cluster.cluster_id.trim().is_empty() {
            return Err(FunctionalConformalError::EmptyIdentity { field: "cluster_id" });
        }
        if !cluster_ids.insert(cluster.cluster_id.clone()) {
            return Err(FunctionalConformalError::DuplicateClusterId {
                cluster_id: cluster.cluster_id.clone(),
            });
        }
        if cluster.protocol_fingerprint != input.target_protocol_fingerprint {
            return Err(FunctionalConformalError::ProtocolFingerprintMismatch {
                cluster_id: cluster.cluster_id.clone(),
            });
        }
        if cluster.functional_contract_id != input.functional_contract_id {
            return Err(FunctionalConformalError::FunctionalContractMismatch {
                cluster_id: cluster.cluster_id.clone(),
            });
        }
        if cluster.observations.len() != input.expected_observations_per_cluster {
            return Err(FunctionalConformalError::ObservationCountMismatch {
                cluster_id: cluster.cluster_id.clone(),
                expected: input.expected_observations_per_cluster,
                actual: cluster.observations.len(),
            });
        }

        let mut maximum_score = 0.0_f64;
        for observation in &cluster.observations {
            if observation.observation_id.trim().is_empty()
                || observation.reference_evidence_id.trim().is_empty()
            {
                return Err(FunctionalConformalError::EmptyIdentity {
                    field: "functional_observation_or_reference_evidence_id",
                });
            }
            if !observation_ids.insert(observation.observation_id.clone()) {
                return Err(FunctionalConformalError::DuplicateObservationId {
                    observation_id: observation.observation_id.clone(),
                });
            }
            if !reference_evidence_ids.insert(observation.reference_evidence_id.clone()) {
                return Err(FunctionalConformalError::DuplicateReferenceEvidenceId {
                    evidence_id: observation.reference_evidence_id.clone(),
                });
            }
            let score = squared_mahalanobis_4(
                observation.residual,
                observation.covariance,
                &observation.observation_id,
            )?;
            maximum_score = maximum_score.max(score);
        }
        cluster_scores.push(FunctionalClusterScore {
            cluster_id: cluster.cluster_id.clone(),
            maximum_squared_mahalanobis: maximum_score,
            observation_count: cluster.observations.len(),
        });
    }

    cluster_scores.sort_by(|left, right| {
        left.maximum_squared_mahalanobis.total_cmp(&right.maximum_squared_mahalanobis)
    });
    let n = cluster_scores.len();
    let rank = (((n + 1) as f64) * (1.0 - input.alpha)).ceil() as usize;
    let calibrated_squared_radius = if rank <= n {
        Some(cluster_scores[rank - 1].maximum_squared_mahalanobis)
    } else {
        None
    };
    let finite_sample_coverage_lower_bound = if rank <= n {
        Some(rank as f64 / (n + 1) as f64)
    } else {
        None
    };
    let disposition = if calibrated_squared_radius.is_some() {
        FunctionalConformalDisposition::FiniteSampleSessionGuaranteeAvailable
    } else {
        FunctionalConformalDisposition::InsufficientCalibrationSessions
    };

    Ok(FunctionalConformalReport {
        campaign_id: input.campaign_id.clone(),
        protocol_fingerprint: input.target_protocol_fingerprint.clone(),
        functional_contract_id: input.functional_contract_id.clone(),
        exchangeability_attestation_id: input.exchangeability_attestation_id.clone(),
        covariance_dimension: FUNCTIONAL_DIMENSION,
        alpha: input.alpha,
        nominal_session_coverage: 1.0 - input.alpha,
        calibration_session_count: n,
        expected_observations_per_cluster: input.expected_observations_per_cluster,
        quantile_rank: rank,
        calibrated_squared_radius,
        finite_sample_coverage_lower_bound,
        cluster_scores,
        disposition,
        claim_scope: "Under exchangeability of complete calibration and future session clusters under the same functional contract, the conformal radius gives finite-sample marginal coverage of the complete 4D integrated mass/advective-momentum functional vector for each future session.",
        limitations: "Reference functional residuals must be independent of the PIV processing under evaluation and traceably measured. The exchangeability assumption is external. This does not validate missing control-volume terms, pressure/viscous/body forces, momentum storage, or thrust. Coverage is not conditional on the fixed calibration sample.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_covariance() -> [[f64; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    fn observation(index: usize, score: f64) -> FunctionalObservation {
        FunctionalObservation {
            observation_id: format!("functional-{index}"),
            reference_evidence_id: format!("reference-{index}"),
            residual: [score.sqrt(), 0.0, 0.0, 0.0],
            covariance: identity_covariance(),
        }
    }

    fn cluster(index: usize, score: f64) -> FunctionalCalibrationCluster {
        FunctionalCalibrationCluster {
            cluster_id: format!("lab-session-{index}"),
            protocol_fingerprint: "cv-functional-protocol-v1".to_owned(),
            functional_contract_id: "mass-momentum-all-surface-v1".to_owned(),
            observations: vec![observation(index, score)],
        }
    }

    fn input(count: usize) -> FunctionalConformalInput {
        FunctionalConformalInput {
            campaign_id: "EHD-FUNCTIONAL-CALIBRATION".to_owned(),
            target_protocol_fingerprint: "cv-functional-protocol-v1".to_owned(),
            functional_contract_id: "mass-momentum-all-surface-v1".to_owned(),
            exchangeability_attestation_id: "independent-reference-design-v1".to_owned(),
            alpha: 0.05,
            expected_observations_per_cluster: 1,
            calibration_clusters: (0..count)
                .map(|index| cluster(index, index as f64 + 1.0))
                .collect(),
        }
    }

    #[test]
    fn nineteen_independent_sessions_produce_finite_95_percent_functional_radius() {
        let report = calibrate_functional_conformal(&input(19)).unwrap();
        assert_eq!(report.covariance_dimension, 4);
        assert_eq!(report.quantile_rank, 19);
        assert_eq!(report.calibrated_squared_radius, Some(19.0));
        assert!((report.finite_sample_coverage_lower_bound.unwrap() - 0.95).abs() < 1.0e-12);
    }

    #[test]
    fn eighteen_sessions_produce_unbounded_functional_set() {
        let report = calibrate_functional_conformal(&input(18)).unwrap();
        assert_eq!(report.quantile_rank, 19);
        assert_eq!(report.calibrated_squared_radius, None);
        assert_eq!(report.disposition, FunctionalConformalDisposition::InsufficientCalibrationSessions);
    }

    #[test]
    fn cholesky_mahalanobis_handles_full_cross_functional_covariance() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].observations[0].covariance = [
            [1.0, 0.8, 0.0, 0.0],
            [0.8, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        campaign.calibration_clusters[0].observations[0].residual = [1.0, -1.0, 0.0, 0.0];
        let report = calibrate_functional_conformal(&campaign).unwrap();
        let row = report.cluster_scores.iter()
            .find(|row| row.cluster_id == "lab-session-0").unwrap();
        assert!((row.maximum_squared_mahalanobis - 10.0).abs() < 1.0e-12);
    }

    #[test]
    fn non_symmetric_or_singular_functional_covariance_is_rejected() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].observations[0].covariance[0][1] = 0.2;
        assert!(matches!(
            calibrate_functional_conformal(&campaign),
            Err(FunctionalConformalError::InvalidCovariance { .. })
        ));

        let mut campaign = input(19);
        campaign.calibration_clusters[0].observations[0].covariance[3][3] = 0.0;
        assert!(matches!(
            calibrate_functional_conformal(&campaign),
            Err(FunctionalConformalError::InvalidCovariance { .. })
        ));
    }

    #[test]
    fn functional_contract_and_protocol_must_match_target() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].protocol_fingerprint = "changed-surface".to_owned();
        assert!(matches!(
            calibrate_functional_conformal(&campaign),
            Err(FunctionalConformalError::ProtocolFingerprintMismatch { .. })
        ));
    }

    #[test]
    fn functional_contract_must_match_target() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].functional_contract_id =
            "different-surface-and-sign-v2".to_owned();
        assert!(matches!(
            calibrate_functional_conformal(&campaign),
            Err(FunctionalConformalError::FunctionalContractMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_reference_evidence_is_rejected() {
        let mut campaign = input(19);
        campaign.expected_observations_per_cluster = 2;
        let mut duplicate_reference = campaign.calibration_clusters[0].observations[0].clone();
        duplicate_reference.observation_id = "new-functional-observation".to_owned();
        campaign.calibration_clusters[0].observations.push(duplicate_reference);
        for (index, cluster) in campaign.calibration_clusters.iter_mut().enumerate().skip(1) {
            cluster.observations.push(observation(index + 100, index as f64 + 1.0));
        }
        assert!(matches!(
            calibrate_functional_conformal(&campaign),
            Err(FunctionalConformalError::DuplicateObservationId { .. })
                | Err(FunctionalConformalError::DuplicateReferenceEvidenceId { .. })
        ));
    }
}
