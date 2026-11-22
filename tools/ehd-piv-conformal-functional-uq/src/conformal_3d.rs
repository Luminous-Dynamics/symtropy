//! Three-component whole-field conformal calibration for stereoscopic / 3D PIV.
//!
//! This uses the same complete-field cluster maximum-score construction as
//! conformal.rs, but scores full 3x3 predictive covariance. It is intentionally
//! separate from the planar 2D scorer so a calibration radius cannot silently
//! cross measurement dimensions.
//!
//! Exchangeability assumptions and fixed-field-size requirements are identical
//! to the planar scorer: complete calibration fields must be exchangeable with
//! a future field under the same frozen protocol.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use crate::FlowRegion;
use crate::functional::Covariance3;

const MINIMUM_DETERMINANT_RATIO: f64 = f64::EPSILON;

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationVector3 {
    pub sample_id: String,
    pub region: FlowRegion,
    /// Reference minus predicted three-component velocity, m/s.
    pub residual_m_s: [f64; 3],
    /// Full 3x3 predictive covariance, (m/s)^2.
    pub covariance_m2_s2: Covariance3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationCluster3 {
    pub cluster_id: String,
    pub protocol_fingerprint: String,
    pub vectors: Vec<CalibrationVector3>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterConformal3Input {
    pub campaign_id: String,
    pub target_protocol_fingerprint: String,
    pub exchangeability_attestation_id: String,
    pub alpha: f64,
    pub expected_vectors_per_cluster: usize,
    pub calibration_clusters: Vec<CalibrationCluster3>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conformal3Disposition {
    FiniteSampleClusterGuaranteeAvailable,
    InsufficientCalibrationClusters,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterScore3 {
    pub cluster_id: String,
    pub maximum_squared_mahalanobis: f64,
    pub vector_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterConformal3Report {
    pub campaign_id: String,
    pub protocol_fingerprint: String,
    pub exchangeability_attestation_id: String,
    pub covariance_dimension: usize,
    pub alpha: f64,
    pub nominal_cluster_coverage: f64,
    pub calibration_cluster_count: usize,
    pub expected_vectors_per_cluster: usize,
    pub quantile_rank: usize,
    pub calibrated_squared_radius: Option<f64>,
    pub finite_sample_coverage_lower_bound: Option<f64>,
    pub cluster_scores: Vec<ClusterScore3>,
    pub disposition: Conformal3Disposition,
    pub claim_scope: &'static str,
    pub limitations: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conformal3Error {
    EmptyIdentity { field: &'static str },
    InvalidMiscoverage,
    InvalidExpectedVectorCount,
    EmptyCalibrationSet,
    DuplicateClusterId { cluster_id: String },
    DuplicateSampleId { sample_id: String },
    ProtocolFingerprintMismatch { cluster_id: String },
    FieldVectorCountMismatch { cluster_id: String, expected: usize, actual: usize },
    MissingPhysicalRegion { cluster_id: String, region: FlowRegion },
    NonFiniteResidual { sample_id: String },
    InvalidCovariance { sample_id: String },
    NonFiniteScore { sample_id: String },
}

impl fmt::Display for Conformal3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity { field } => write!(f, "3D conformal {field} must be non-empty"),
            Self::InvalidMiscoverage => write!(f, "3D conformal alpha must be finite and in (0,1)"),
            Self::InvalidExpectedVectorCount => write!(f, "3D conformal expected vector count must be positive"),
            Self::EmptyCalibrationSet => write!(f, "3D conformal calibration requires complete field clusters"),
            Self::DuplicateClusterId { cluster_id } => write!(f, "duplicate 3D calibration cluster: {cluster_id}"),
            Self::DuplicateSampleId { sample_id } => write!(f, "duplicate 3D calibration sample: {sample_id}"),
            Self::ProtocolFingerprintMismatch { cluster_id } => write!(f, "3D cluster {cluster_id} does not match the target protocol"),
            Self::FieldVectorCountMismatch { cluster_id, expected, actual } => write!(f, "3D cluster {cluster_id} has {actual} vectors; expected {expected}"),
            Self::MissingPhysicalRegion { cluster_id, region } => write!(f, "3D cluster {cluster_id} is missing region {region:?}"),
            Self::NonFiniteResidual { sample_id } => write!(f, "3D residual is non-finite for sample {sample_id}"),
            Self::InvalidCovariance { sample_id } => write!(f, "3D covariance is not positive definite for sample {sample_id}"),
            Self::NonFiniteScore { sample_id } => write!(f, "3D Mahalanobis score is non-finite for sample {sample_id}"),
        }
    }
}

impl Error for Conformal3Error {}

fn squared_mahalanobis_3(
    residual: [f64; 3],
    covariance: Covariance3,
    sample_id: &str,
) -> Result<f64, Conformal3Error> {
    if !residual.iter().all(|value| value.is_finite()) {
        return Err(Conformal3Error::NonFiniteResidual {
            sample_id: sample_id.to_owned(),
        });
    }

    let a = covariance.xx;
    let b = covariance.xy;
    let c = covariance.xz;
    let d = covariance.yy;
    let e = covariance.yz;
    let f = covariance.zz;
    let entries = [a, b, c, d, e, f];
    if !entries.iter().all(|value| value.is_finite()) {
        return Err(Conformal3Error::InvalidCovariance {
            sample_id: sample_id.to_owned(),
        });
    }

    // Sylvester criterion for a symmetric 3x3 positive-definite matrix.
    let leading_minor_2 = a.mul_add(d, -b * b);
    let determinant = a * (d * f - e * e)
        - b * (b * f - c * e)
        + c * (b * e - c * d);
    let scale = entries.iter().map(|value| value.abs()).fold(0.0_f64, f64::max);
    if a <= 0.0
        || leading_minor_2 <= 0.0
        || !determinant.is_finite()
        || determinant <= MINIMUM_DETERMINANT_RATIO * scale.powi(3)
    {
        return Err(Conformal3Error::InvalidCovariance {
            sample_id: sample_id.to_owned(),
        });
    }

    // Cofactor form of the inverse of [[a,b,c],[b,d,e],[c,e,f]].
    let inverse_00 = (d * f - e * e) / determinant;
    let inverse_01 = (c * e - b * f) / determinant;
    let inverse_02 = (b * e - c * d) / determinant;
    let inverse_11 = (a * f - c * c) / determinant;
    let inverse_12 = (b * c - a * e) / determinant;
    let inverse_22 = (a * d - b * b) / determinant;

    let x = residual[0];
    let y = residual[1];
    let z = residual[2];
    let score = x * (inverse_00 * x + inverse_01 * y + inverse_02 * z)
        + y * (inverse_01 * x + inverse_11 * y + inverse_12 * z)
        + z * (inverse_02 * x + inverse_12 * y + inverse_22 * z);
    if !score.is_finite() || score < 0.0 {
        return Err(Conformal3Error::NonFiniteScore {
            sample_id: sample_id.to_owned(),
        });
    }
    Ok(score)
}

/// Calibrate whole-field simultaneous coverage using three-component
/// Mahalanobis scores and the exact split-conformal cluster rank.
pub fn calibrate_cluster_conformal_3d(
    input: &ClusterConformal3Input,
) -> Result<ClusterConformal3Report, Conformal3Error> {
    for (field, value) in [
        ("campaign_id", input.campaign_id.as_str()),
        ("target_protocol_fingerprint", input.target_protocol_fingerprint.as_str()),
        ("exchangeability_attestation_id", input.exchangeability_attestation_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(Conformal3Error::EmptyIdentity { field });
        }
    }
    if !input.alpha.is_finite() || input.alpha <= 0.0 || input.alpha >= 1.0 {
        return Err(Conformal3Error::InvalidMiscoverage);
    }
    if input.expected_vectors_per_cluster == 0 {
        return Err(Conformal3Error::InvalidExpectedVectorCount);
    }
    if input.calibration_clusters.is_empty() {
        return Err(Conformal3Error::EmptyCalibrationSet);
    }

    let mut cluster_ids = BTreeSet::new();
    let mut sample_ids = BTreeSet::new();
    let mut cluster_scores = Vec::with_capacity(input.calibration_clusters.len());

    for cluster in &input.calibration_clusters {
        if cluster.cluster_id.trim().is_empty() {
            return Err(Conformal3Error::EmptyIdentity { field: "cluster_id" });
        }
        if !cluster_ids.insert(cluster.cluster_id.clone()) {
            return Err(Conformal3Error::DuplicateClusterId {
                cluster_id: cluster.cluster_id.clone(),
            });
        }
        if cluster.protocol_fingerprint != input.target_protocol_fingerprint {
            return Err(Conformal3Error::ProtocolFingerprintMismatch {
                cluster_id: cluster.cluster_id.clone(),
            });
        }
        if cluster.vectors.len() != input.expected_vectors_per_cluster {
            return Err(Conformal3Error::FieldVectorCountMismatch {
                cluster_id: cluster.cluster_id.clone(),
                expected: input.expected_vectors_per_cluster,
                actual: cluster.vectors.len(),
            });
        }

        let mut present_regions = BTreeSet::new();
        let mut maximum_score = 0.0_f64;
        for vector in &cluster.vectors {
            if vector.sample_id.trim().is_empty() {
                return Err(Conformal3Error::EmptyIdentity { field: "sample_id" });
            }
            if !sample_ids.insert(vector.sample_id.clone()) {
                return Err(Conformal3Error::DuplicateSampleId {
                    sample_id: vector.sample_id.clone(),
                });
            }
            present_regions.insert(vector.region);
            maximum_score = maximum_score.max(squared_mahalanobis_3(
                vector.residual_m_s,
                vector.covariance_m2_s2,
                &vector.sample_id,
            )?);
        }
        for region in FlowRegion::ALL {
            if !present_regions.contains(&region) {
                return Err(Conformal3Error::MissingPhysicalRegion {
                    cluster_id: cluster.cluster_id.clone(),
                    region,
                });
            }
        }

        cluster_scores.push(ClusterScore3 {
            cluster_id: cluster.cluster_id.clone(),
            maximum_squared_mahalanobis: maximum_score,
            vector_count: cluster.vectors.len(),
        });
    }

    cluster_scores.sort_by(|left, right| {
        left.maximum_squared_mahalanobis
            .total_cmp(&right.maximum_squared_mahalanobis)
    });
    let count = cluster_scores.len();
    let rank = (((count + 1) as f64) * (1.0 - input.alpha)).ceil() as usize;
    let calibrated_squared_radius = if rank <= count {
        Some(cluster_scores[rank - 1].maximum_squared_mahalanobis)
    } else {
        None
    };
    let finite_sample_coverage_lower_bound = if rank <= count {
        Some(rank as f64 / (count + 1) as f64)
    } else {
        None
    };
    let disposition = if calibrated_squared_radius.is_some() {
        Conformal3Disposition::FiniteSampleClusterGuaranteeAvailable
    } else {
        Conformal3Disposition::InsufficientCalibrationClusters
    };

    Ok(ClusterConformal3Report {
        campaign_id: input.campaign_id.clone(),
        protocol_fingerprint: input.target_protocol_fingerprint.clone(),
        exchangeability_attestation_id: input.exchangeability_attestation_id.clone(),
        covariance_dimension: 3,
        alpha: input.alpha,
        nominal_cluster_coverage: 1.0 - input.alpha,
        calibration_cluster_count: count,
        expected_vectors_per_cluster: input.expected_vectors_per_cluster,
        quantile_rank: rank,
        calibrated_squared_radius,
        finite_sample_coverage_lower_bound,
        cluster_scores,
        disposition,
        claim_scope: "Under exchangeability of complete calibration field clusters and a future field cluster under the same frozen protocol, the conformal radius gives finite-sample marginal coverage of the entire 3D target field across all included vectors and the four declared regions.",
        limitations: "The exchangeability attestation is external and cannot be validated by this code. The guarantee is marginal over calibration and target fields, not conditional on the observed calibration set. Distribution shift, dependent clusters, changed field size or changed grid/mask invalidate the stated guarantee.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector(id: &str, region: FlowRegion, residual: [f64; 3]) -> CalibrationVector3 {
        CalibrationVector3 {
            sample_id: id.to_owned(),
            region,
            residual_m_s: residual,
            covariance_m2_s2: Covariance3 {
                xx: 1.0, xy: 0.0, xz: 0.0, yy: 1.0, yz: 0.0, zz: 1.0,
            },
        }
    }

    fn cluster(index: usize, score: f64) -> CalibrationCluster3 {
        let radius = score.sqrt();
        CalibrationCluster3 {
            cluster_id: format!("stereo-session-{index}"),
            protocol_fingerprint: "stereo-protocol-v1".to_owned(),
            vectors: vec![
                vector(&format!("s{index}-bulk"), FlowRegion::Bulk, [radius, 0.0, 0.0]),
                vector(&format!("s{index}-wall"), FlowRegion::NearWall, [radius * 0.8, 0.0, 0.0]),
                vector(&format!("s{index}-shear"), FlowRegion::ShearLayer, [radius * 0.6, 0.0, 0.0]),
                vector(&format!("s{index}-wake"), FlowRegion::Wake, [radius * 0.4, 0.0, 0.0]),
            ],
        }
    }

    fn input(count: usize) -> ClusterConformal3Input {
        ClusterConformal3Input {
            campaign_id: "EHD-STEREO-MANUFACTURED".to_owned(),
            target_protocol_fingerprint: "stereo-protocol-v1".to_owned(),
            exchangeability_attestation_id: "stereo-design-review-v1".to_owned(),
            alpha: 0.05,
            expected_vectors_per_cluster: 4,
            calibration_clusters: (0..count).map(|index| cluster(index, index as f64 + 1.0)).collect(),
        }
    }

    #[test]
    fn three_dimensional_cluster_calibration_has_finite_95_percent_radius_at_nineteen_fields() {
        let report = calibrate_cluster_conformal_3d(&input(19)).unwrap();
        assert_eq!(report.covariance_dimension, 3);
        assert_eq!(report.quantile_rank, 19);
        assert_eq!(report.calibrated_squared_radius, Some(19.0));
        assert!((report.finite_sample_coverage_lower_bound.unwrap() - 0.95).abs() < 1.0e-12);
    }

    #[test]
    fn three_dimensional_correlated_covariance_changes_mahalanobis_score() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].vectors[0].covariance_m2_s2 = Covariance3 {
            xx: 1.0, xy: 0.8, xz: 0.0, yy: 1.0, yz: 0.0, zz: 1.0,
        };
        campaign.calibration_clusters[0].vectors[0].residual_m_s = [1.0, -1.0, 0.0];
        let report = calibrate_cluster_conformal_3d(&campaign).unwrap();
        let row = report.cluster_scores.iter()
            .find(|row| row.cluster_id == "stereo-session-0").unwrap();
        assert!(row.maximum_squared_mahalanobis > 1.0);
    }

    #[test]
    fn three_dimensional_non_positive_definite_covariance_is_rejected() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].vectors[0].covariance_m2_s2 = Covariance3 {
            xx: 1.0, xy: 0.9, xz: 0.9, yy: 1.0, yz: -0.9, zz: 1.0,
        };
        assert!(matches!(
            calibrate_cluster_conformal_3d(&campaign),
            Err(Conformal3Error::InvalidCovariance { .. })
        ));
    }

    #[test]
    fn three_dimensional_field_size_mismatch_is_rejected() {
        let mut campaign = input(19);
        campaign.calibration_clusters[0].vectors.pop();
        assert!(matches!(
            calibrate_cluster_conformal_3d(&campaign),
            Err(Conformal3Error::FieldVectorCountMismatch { .. })
        ));
    }
}
