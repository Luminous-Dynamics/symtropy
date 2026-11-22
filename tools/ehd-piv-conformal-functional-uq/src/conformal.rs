//! Split-conformal calibration at the independent acquisition/field level.
//!
//! A calibration unit is a complete field/acquisition cluster. Its score is
//! the maximum squared Mahalanobis residual across every vector in the field.
//! The split-conformal order statistic therefore calibrates simultaneous
//! coverage of all scored vectors in one future field, not pointwise coverage
//! averaged over interrogation windows.
//!
//! Finite-sample marginal coverage over future field clusters requires the
//! calibration clusters and future cluster to be exchangeable under the same
//! predeclared acquisition/grid/processing protocol. A matching fingerprint is
//! necessary but cannot prove exchangeability. Distribution shift, temporal
//! drift, shared latent causes between nominal clusters, or a changed grid
//! invalidate the guarantee unless handled by a separately justified method.
//!
//! The field-size contract is exact: each calibration field has the declared
//! number of vectors and all four physical regions. A different deployment
//! field size changes the maximum-score distribution and is rejected.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use crate::FlowRegion;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Covariance2 {
    pub xx: f64,
    pub xy: f64,
    pub yy: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationVector {
    pub sample_id: String,
    pub region: FlowRegion,
    /// Reference minus predicted velocity, m/s.
    pub residual_m_s: [f64; 2],
    /// Base predictive covariance, (m/s)^2.
    pub covariance_m2_s2: Covariance2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationCluster {
    /// Must identify an independent acquisition/common-cause unit, not a file
    /// name or a processing batch derived from shared raw images.
    pub cluster_id: String,
    /// Frozen fingerprint covering geometry, grid/mask, processing, tracer,
    /// calibration and acquisition protocol.
    pub protocol_fingerprint: String,
    pub vectors: Vec<CalibrationVector>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterConformalInput {
    pub campaign_id: String,
    pub target_protocol_fingerprint: String,
    /// External record of the exchangeability/design assumption. This code
    /// checks that it exists; it cannot validate the scientific assertion.
    pub exchangeability_attestation_id: String,
    /// Desired cluster-level miscoverage probability, e.g. 0.05.
    pub alpha: f64,
    /// Exact number of vectors in every calibration and target field.
    pub expected_vectors_per_cluster: usize,
    pub calibration_clusters: Vec<CalibrationCluster>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConformalDisposition {
    FiniteSampleClusterGuaranteeAvailable,
    InsufficientCalibrationClusters,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterScore {
    pub cluster_id: String,
    pub maximum_squared_mahalanobis: f64,
    pub vector_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterConformalReport {
    pub campaign_id: String,
    pub protocol_fingerprint: String,
    pub exchangeability_attestation_id: String,
    pub alpha: f64,
    pub nominal_cluster_coverage: f64,
    pub calibration_cluster_count: usize,
    pub expected_vectors_per_cluster: usize,
    /// One-based finite-sample order-statistic rank.
    pub quantile_rank: usize,
    /// None means the exact finite-sample rule requires an unbounded set.
    pub calibrated_squared_radius: Option<f64>,
    /// Rank/(n+1), the distribution-free lower coverage bound when rank<=n.
    pub finite_sample_coverage_lower_bound: Option<f64>,
    /// Maximum squared Mahalanobis score per complete calibration field.
    pub cluster_scores: Vec<ClusterScore>,
    pub disposition: ConformalDisposition,
    pub claim_scope: &'static str,
    pub limitations: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConformalError {
    EmptyIdentity { field: &'static str },
    InvalidMiscoverage,
    InvalidExpectedVectorCount,
    EmptyCalibrationSet,
    DuplicateClusterId { cluster_id: String },
    DuplicateSampleId { sample_id: String },
    ProtocolFingerprintMismatch { cluster_id: String },
    FieldVectorCountMismatch {
        cluster_id: String,
        expected: usize,
        actual: usize,
    },
    MissingPhysicalRegion { cluster_id: String, region: FlowRegion },
    NonFiniteResidual { sample_id: String },
    InvalidCovariance { sample_id: String },
    NonFiniteScore { sample_id: String },
}

impl fmt::Display for ConformalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity { field } => write!(f, "conformal {field} must be non-empty"),
            Self::InvalidMiscoverage => write!(f, "conformal alpha must be finite and in (0,1)"),
            Self::InvalidExpectedVectorCount => write!(f, "expected vector count must be positive"),
            Self::EmptyCalibrationSet => write!(f, "at least one complete calibration cluster is required"),
            Self::DuplicateClusterId { cluster_id } => write!(f, "duplicate calibration cluster: {cluster_id}"),
            Self::DuplicateSampleId { sample_id } => write!(f, "duplicate calibration sample: {sample_id}"),
            Self::ProtocolFingerprintMismatch { cluster_id } => {
                write!(f, "cluster {cluster_id} does not match the target protocol fingerprint")
            }
            Self::FieldVectorCountMismatch { cluster_id, expected, actual } => {
                write!(f, "cluster {cluster_id} has {actual} vectors; expected {expected}")
            }
            Self::MissingPhysicalRegion { cluster_id, region } => {
                write!(f, "cluster {cluster_id} is missing physical region {region:?}")
            }
            Self::NonFiniteResidual { sample_id } => {
                write!(f, "non-finite velocity residual for sample {sample_id}")
            }
            Self::InvalidCovariance { sample_id } => {
                write!(f, "predictive covariance is not positive definite for sample {sample_id}")
            }
            Self::NonFiniteScore { sample_id } => {
                write!(f, "non-finite Mahalanobis score for sample {sample_id}")
            }
        }
    }
}

impl Error for ConformalError {}

fn squared_mahalanobis(
    residual: [f64; 2],
    covariance: Covariance2,
    sample_id: &str,
) -> Result<f64, ConformalError> {
    if !residual.iter().all(|value| value.is_finite()) {
        return Err(ConformalError::NonFiniteResidual {
            sample_id: sample_id.to_owned(),
        });
    }
    if !covariance.xx.is_finite()
        || !covariance.xy.is_finite()
        || !covariance.yy.is_finite()
        || covariance.xx <= 0.0
        || covariance.yy <= 0.0
    {
        return Err(ConformalError::InvalidCovariance {
            sample_id: sample_id.to_owned(),
        });
    }
    let determinant = covariance.xx.mul_add(covariance.yy, -covariance.xy * covariance.xy);
    if !determinant.is_finite() || determinant <= f64::EPSILON * covariance.xx * covariance.yy {
        return Err(ConformalError::InvalidCovariance {
            sample_id: sample_id.to_owned(),
        });
    }
    let x = residual[0];
    let y = residual[1];
    let score = (covariance.yy * x * x - 2.0 * covariance.xy * x * y
        + covariance.xx * y * y) / determinant;
    if !score.is_finite() || score < 0.0 {
        return Err(ConformalError::NonFiniteScore {
            sample_id: sample_id.to_owned(),
        });
    }
    Ok(score)
}

/// Calibrate a simultaneous field-level ellipsoid multiplier.
///
/// With n exchangeable complete calibration fields, the conformal rank is
/// k=ceil((n+1)(1-alpha)). If k>n, the mathematically correct split-conformal
/// prediction set is unbounded; the function reports no finite radius rather
/// than silently substituting the largest observed score.
pub fn calibrate_cluster_conformal(
    input: &ClusterConformalInput,
) -> Result<ClusterConformalReport, ConformalError> {
    for (field, value) in [
        ("campaign_id", input.campaign_id.as_str()),
        ("target_protocol_fingerprint", input.target_protocol_fingerprint.as_str()),
        ("exchangeability_attestation_id", input.exchangeability_attestation_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ConformalError::EmptyIdentity { field });
        }
    }
    if !input.alpha.is_finite() || input.alpha <= 0.0 || input.alpha >= 1.0 {
        return Err(ConformalError::InvalidMiscoverage);
    }
    if input.expected_vectors_per_cluster == 0 {
        return Err(ConformalError::InvalidExpectedVectorCount);
    }
    if input.calibration_clusters.is_empty() {
        return Err(ConformalError::EmptyCalibrationSet);
    }

    let mut cluster_ids = BTreeSet::new();
    let mut sample_ids = BTreeSet::new();
    let mut cluster_scores = Vec::with_capacity(input.calibration_clusters.len());

    for cluster in &input.calibration_clusters {
        if cluster.cluster_id.trim().is_empty() {
            return Err(ConformalError::EmptyIdentity { field: "cluster_id" });
        }
        if !cluster_ids.insert(cluster.cluster_id.clone()) {
            return Err(ConformalError::DuplicateClusterId {
                cluster_id: cluster.cluster_id.clone(),
            });
        }
        if cluster.protocol_fingerprint != input.target_protocol_fingerprint {
            return Err(ConformalError::ProtocolFingerprintMismatch {
                cluster_id: cluster.cluster_id.clone(),
            });
        }
        if cluster.vectors.len() != input.expected_vectors_per_cluster {
            return Err(ConformalError::FieldVectorCountMismatch {
                cluster_id: cluster.cluster_id.clone(),
                expected: input.expected_vectors_per_cluster,
                actual: cluster.vectors.len(),
            });
        }

        let mut present_regions = BTreeSet::new();
        let mut maximum_score = 0.0_f64;
        for vector in &cluster.vectors {
            if vector.sample_id.trim().is_empty() {
                return Err(ConformalError::EmptyIdentity { field: "sample_id" });
            }
            if !sample_ids.insert(vector.sample_id.clone()) {
                return Err(ConformalError::DuplicateSampleId {
                    sample_id: vector.sample_id.clone(),
                });
            }
            present_regions.insert(vector.region);
            let score = squared_mahalanobis(
                vector.residual_m_s,
                vector.covariance_m2_s2,
                &vector.sample_id,
            )?;
            maximum_score = maximum_score.max(score);
        }
        for region in FlowRegion::ALL {
            if !present_regions.contains(&region) {
                return Err(ConformalError::MissingPhysicalRegion {
                    cluster_id: cluster.cluster_id.clone(),
                    region,
                });
            }
        }
        cluster_scores.push(ClusterScore {
            cluster_id: cluster.cluster_id.clone(),
            maximum_squared_mahalanobis: maximum_score,
            vector_count: cluster.vectors.len(),
        });
    }

    cluster_scores.sort_by(|left, right| {
        left.maximum_squared_mahalanobis
            .total_cmp(&right.maximum_squared_mahalanobis)
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
        ConformalDisposition::FiniteSampleClusterGuaranteeAvailable
    } else {
        ConformalDisposition::InsufficientCalibrationClusters
    };

    Ok(ClusterConformalReport {
        campaign_id: input.campaign_id.clone(),
        protocol_fingerprint: input.target_protocol_fingerprint.clone(),
        exchangeability_attestation_id: input.exchangeability_attestation_id.clone(),
        alpha: input.alpha,
        nominal_cluster_coverage: 1.0 - input.alpha,
        calibration_cluster_count: n,
        expected_vectors_per_cluster: input.expected_vectors_per_cluster,
        quantile_rank: rank,
        calibrated_squared_radius,
        finite_sample_coverage_lower_bound,
        cluster_scores,
        disposition,
        claim_scope: "Under exchangeability of complete calibration field clusters and a future field cluster under the same frozen protocol, the conformal radius gives finite-sample marginal coverage of the entire target field (all included vectors and the four declared physical regions).",
        limitations: "The exchangeability attestation is external and cannot be verified by this code. Matching protocol fingerprints and field sizes do not prove exchangeability. The guarantee is marginal over the calibration and future clusters, not conditional on the observed calibration set. It does not survive unaccounted distribution shift, temporal dependence between clusters, changed masks/grids, or a different field size.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector(id: &str, region: FlowRegion, score: f64) -> CalibrationVector {
        CalibrationVector {
            sample_id: id.to_owned(),
            region,
            residual_m_s: [score.sqrt(), 0.0],
            covariance_m2_s2: Covariance2 { xx: 1.0, xy: 0.0, yy: 1.0 },
        }
    }

    fn cluster(index: usize, score: f64) -> CalibrationCluster {
        CalibrationCluster {
            cluster_id: format!("session-{index}"),
            protocol_fingerprint: "protocol-frozen-v1".to_owned(),
            vectors: vec![
                vector(&format!("sample-{index}-bulk"), FlowRegion::Bulk, score),
                vector(&format!("sample-{index}-wall"), FlowRegion::NearWall, score * 0.8),
                vector(&format!("sample-{index}-shear"), FlowRegion::ShearLayer, score * 0.6),
                vector(&format!("sample-{index}-wake"), FlowRegion::Wake, score * 0.4),
            ],
        }
    }

    fn input(cluster_count: usize) -> ClusterConformalInput {
        ClusterConformalInput {
            campaign_id: "EHD-CALIBRATION-MANUFACTURED".to_owned(),
            target_protocol_fingerprint: "protocol-frozen-v1".to_owned(),
            exchangeability_attestation_id: "design-review-receipt-v1".to_owned(),
            alpha: 0.05,
            expected_vectors_per_cluster: 4,
            calibration_clusters: (0..cluster_count)
                .map(|index| cluster(index, index as f64 + 1.0))
                .collect(),
        }
    }

    #[test]
    fn nineteen_exchangeable_fields_produce_finite_95_percent_radius() {
        let report = calibrate_cluster_conformal(&input(19)).unwrap();
        assert_eq!(report.quantile_rank, 19);
        assert_eq!(report.calibrated_squared_radius, Some(19.0));
        assert!((report.finite_sample_coverage_lower_bound.unwrap() - 0.95).abs() < 1.0e-12);
        assert_eq!(report.disposition, ConformalDisposition::FiniteSampleClusterGuaranteeAvailable);
    }

    #[test]
    fn eighteen_fields_require_unbounded_finite_sample_set() {
        let report = calibrate_cluster_conformal(&input(18)).unwrap();
        assert_eq!(report.quantile_rank, 19);
        assert_eq!(report.calibrated_squared_radius, None);
        assert_eq!(report.finite_sample_coverage_lower_bound, None);
        assert_eq!(report.disposition, ConformalDisposition::InsufficientCalibrationClusters);
    }

    #[test]
    fn each_field_score_is_the_maximum_over_every_region_and_vector() {
        let mut campaign = input(19);
        campaign.calibration_clusters[4].vectors[3].residual_m_s = [100.0, 0.0];
        let report = calibrate_cluster_conformal(&campaign).unwrap();
        let row = report.cluster_scores.iter()
            .find(|row| row.cluster_id == "session-4").unwrap();
        assert_eq!(row.maximum_squared_mahalanobis, 10_000.0);
    }

    #[test]
    fn calibration_protocol_must_match_target_protocol() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].protocol_fingerprint = "changed-grid-v2".to_owned();
        assert!(matches!(
            calibrate_cluster_conformal(&campaign),
            Err(ConformalError::ProtocolFingerprintMismatch { .. })
        ));
    }

    #[test]
    fn different_field_size_is_rejected() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].vectors.pop();
        assert!(matches!(
            calibrate_cluster_conformal(&campaign),
            Err(ConformalError::FieldVectorCountMismatch { .. })
        ));
    }

    #[test]
    fn missing_region_is_rejected() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].vectors[3].region = FlowRegion::Bulk;
        assert!(matches!(
            calibrate_cluster_conformal(&campaign),
            Err(ConformalError::MissingPhysicalRegion {
                region: FlowRegion::Wake,
                ..
            })
        ));
    }

    #[test]
    fn duplicate_acquisition_cluster_is_rejected() {
        let mut campaign = input(19);
        campaign.calibration_clusters[1].cluster_id =
            campaign.calibration_clusters[0].cluster_id.clone();
        assert!(matches!(
            calibrate_cluster_conformal(&campaign),
            Err(ConformalError::DuplicateClusterId { .. })
        ));
    }

    #[test]
    fn duplicate_vector_identity_is_rejected() {
        let mut campaign = input(19);
        campaign.calibration_clusters[1].vectors[0].sample_id =
            campaign.calibration_clusters[0].vectors[0].sample_id.clone();
        assert!(matches!(
            calibrate_cluster_conformal(&campaign),
            Err(ConformalError::DuplicateSampleId { .. })
        ));
    }

    #[test]
    fn invalid_covariance_is_rejected() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].vectors[0].covariance_m2_s2 =
            Covariance2 { xx: 1.0, xy: 2.0, yy: 1.0 };
        assert!(matches!(
            calibrate_cluster_conformal(&campaign),
            Err(ConformalError::InvalidCovariance { .. })
        ));
    }

    #[test]
    fn invalid_exchangeability_alpha_and_empty_campaign_are_rejected() {
        let mut campaign = input(19);
        campaign.exchangeability_attestation_id.clear();
        assert!(matches!(
            calibrate_cluster_conformal(&campaign),
            Err(ConformalError::EmptyIdentity { .. })
        ));
        let mut campaign = input(19);
        campaign.alpha = 0.0;
        assert_eq!(calibrate_cluster_conformal(&campaign).unwrap_err(), ConformalError::InvalidMiscoverage);
        let mut campaign = input(19);
        campaign.calibration_clusters.clear();
        assert_eq!(calibrate_cluster_conformal(&campaign).unwrap_err(), ConformalError::EmptyCalibrationSet);
    }
}
