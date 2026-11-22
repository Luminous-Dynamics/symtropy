//! Session-clustered calibration for two-component PIV velocity covariance.
//!
//! This crate evaluates supplied reference/estimate pairs and their 2x2
//! predictive covariance matrices. It does not process particle images,
//! estimate displacement covariance, or create physical ground truth.
//!
//! The key distinction from vector-count-only coverage is that repeated,
//! overlapping vectors from one acquisition session are not treated as
//! independent calibration evidence. Coverage is summarized both by pooled
//! vector count and by equal-weight independent-dependence cluster means.
//!
//! The 95% cluster interval is an approximate Student-t interval over
//! independent cluster-level coverage fractions. It is available only for
//! at least 20 clusters and uses t=2.093, the df=19 two-sided 95% critical
//! value; this is conservative for larger cluster counts, conditional on
//! the declared clusters being independent. It is not a substitute for
//! experimental design or metrological traceability.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

const MARGINAL_Z_95: f64 = 1.959_963_984_540_054;
const JOINT_CHI_SQUARED_2_95: f64 = 5.991_464_547_107_979;
const CLUSTER_T_95_DF_19: f64 = 2.093;
const MIN_CLUSTERS_FOR_INTERVAL: usize = 20;
const REQUIRED_REGION_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlowRegion {
    Bulk,
    NearWall,
    ShearLayer,
    Wake,
}

impl FlowRegion {
    pub const ALL: [Self; REQUIRED_REGION_COUNT] = [
        Self::Bulk,
        Self::NearWall,
        Self::ShearLayer,
        Self::Wake,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Covariance2 {
    /// Variance of streamwise velocity, in (m/s)^2.
    pub xx: f64,
    /// Covariance of streamwise and wall-normal velocity, in (m/s)^2.
    pub xy: f64,
    /// Variance of wall-normal velocity, in (m/s)^2.
    pub yy: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VectorObservation {
    /// Globally unique vector/sample identity within the evaluation cohort.
    pub sample_id: String,
    /// Highest known common-cause group: e.g. acquisition session/day,
    /// shared camera/laser calibration batch, tracer lot, or raw-image cohort.
    /// A new file name or algorithm run does not itself establish independence.
    pub independence_cluster_id: String,
    pub region: FlowRegion,
    /// Independent reference velocity, e.g. synthetic ground truth.
    pub reference_velocity_m_s: [f64; 2],
    /// Estimated two-component velocity.
    pub estimated_velocity_m_s: [f64; 2],
    /// Predictive covariance emitted by the method under evaluation.
    pub predicted_covariance_m2_s2: Covariance2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalibrationGate {
    pub nominal_coverage: f64,
    pub minimum_acceptable_coverage: f64,
    pub maximum_acceptable_coverage: f64,
    pub minimum_samples_per_region: usize,
    pub minimum_independent_clusters_per_region: usize,
}

impl Default for CalibrationGate {
    fn default() -> Self {
        Self {
            nominal_coverage: 0.95,
            minimum_acceptable_coverage: 0.90,
            maximum_acceptable_coverage: 0.98,
            minimum_samples_per_region: 200,
            minimum_independent_clusters_per_region: 20,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationDisposition {
    MeetsDeclaredCalibrationGates,
    CoverageOutsideTargetBand,
    NominalOutsideClusterConfidenceInterval,
    InsufficientIndependentEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoverageSummary {
    pub sample_count: usize,
    pub covered_count: usize,
    /// Vector-count-weighted coverage. May be dominated by a large session.
    pub pooled_coverage: f64,
    /// Mean of per-cluster coverage fractions; each cluster has equal weight.
    pub cluster_equal_coverage: f64,
    /// Approximate 95% Student-t interval over independent cluster fractions.
    /// Omitted when fewer than 20 independent clusters are present.
    pub cluster_95_ci_lower: Option<f64>,
    pub cluster_95_ci_upper: Option<f64>,
    pub cluster_95_ci_contains_nominal: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegionCalibrationReport {
    pub region: FlowRegion,
    pub sample_count: usize,
    pub independent_cluster_count: usize,
    pub streamwise_marginal_95: CoverageSummary,
    pub wall_normal_marginal_95: CoverageSummary,
    pub joint_2d_ellipse_95: CoverageSummary,
    /// Ideal expectation is 2 for a correctly calibrated 2-D Gaussian.
    pub mean_squared_mahalanobis_distance: f64,
    /// Average bivariate Gaussian negative log likelihood, in nats/sample.
    /// Lower is better only on the exact same observations and reference.
    pub mean_gaussian_nll_nats: f64,
    /// Mean area of the nominal 95% covariance ellipse, in (m/s)^2.
    /// Compare only alongside coverage and proper score.
    pub mean_95_ellipse_area_m2_s2: f64,
    pub mean_streamwise_interval_width_m_s: f64,
    pub mean_wall_normal_interval_width_m_s: f64,
    pub disposition: CalibrationDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationReport {
    pub gate: CalibrationGate,
    pub regions: Vec<RegionCalibrationReport>,
    pub all_regions_meet_calibration: bool,
    pub limitations: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalibrationError {
    EmptyObservations,
    InvalidGate,
    EmptySampleId,
    EmptyIndependenceClusterId,
    DuplicateSampleId { sample_id: String },
    NonFiniteVelocity { sample_id: String },
    InvalidCovariance { sample_id: String },
    MissingRequiredRegion { region: FlowRegion },
    NonFiniteDerivedValue { sample_id: String },
}

impl fmt::Display for CalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyObservations => write!(f, "PIV covariance calibration requires observations"),
            Self::InvalidGate => write!(f, "PIV covariance calibration gate is invalid"),
            Self::EmptySampleId => write!(f, "PIV calibration sample identity must not be empty"),
            Self::EmptyIndependenceClusterId => {
                write!(f, "PIV calibration independence-cluster identity must not be empty")
            }
            Self::DuplicateSampleId { sample_id } => {
                write!(f, "duplicate PIV calibration sample identity: {sample_id}")
            }
            Self::NonFiniteVelocity { sample_id } => {
                write!(f, "non-finite reference or estimated velocity for sample {sample_id}")
            }
            Self::InvalidCovariance { sample_id } => {
                write!(f, "predictive covariance must be finite and positive definite for sample {sample_id}")
            }
            Self::MissingRequiredRegion { region } => {
                write!(f, "required PIV flow region is missing: {region:?}")
            }
            Self::NonFiniteDerivedValue { sample_id } => {
                write!(f, "non-finite covariance calibration result for sample {sample_id}")
            }
        }
    }
}

impl Error for CalibrationError {}

#[derive(Default)]
struct ClusterAccumulator {
    sample_count: usize,
    streamwise_covered: usize,
    wall_normal_covered: usize,
    joint_covered: usize,
    gaussian_nll_sum: f64,
    mahalanobis_squared_sum: f64,
    ellipse_area_sum: f64,
    streamwise_width_sum: f64,
    wall_normal_width_sum: f64,
}

#[derive(Default)]
struct RegionAccumulator {
    clusters: BTreeMap<String, ClusterAccumulator>,
}

fn validate_gate(gate: CalibrationGate) -> Result<(), CalibrationError> {
    let probabilities = [
        gate.nominal_coverage,
        gate.minimum_acceptable_coverage,
        gate.maximum_acceptable_coverage,
    ];
    if !probabilities.iter().all(|value| value.is_finite() && *value > 0.0 && *value < 1.0)
        || gate.minimum_acceptable_coverage >= gate.nominal_coverage
        || gate.nominal_coverage >= gate.maximum_acceptable_coverage
        || gate.minimum_samples_per_region == 0
        || gate.minimum_independent_clusters_per_region == 0
    {
        return Err(CalibrationError::InvalidGate);
    }
    Ok(())
}

fn validate_observation(
    observation: &VectorObservation,
) -> Result<(f64, f64, f64, f64, f64), CalibrationError> {
    if observation.sample_id.trim().is_empty() {
        return Err(CalibrationError::EmptySampleId);
    }
    if observation.independence_cluster_id.trim().is_empty() {
        return Err(CalibrationError::EmptyIndependenceClusterId);
    }
    if !observation
        .reference_velocity_m_s
        .iter()
        .chain(observation.estimated_velocity_m_s.iter())
        .all(|value| value.is_finite())
    {
        return Err(CalibrationError::NonFiniteVelocity {
            sample_id: observation.sample_id.clone(),
        });
    }

    let covariance = observation.predicted_covariance_m2_s2;
    if !covariance.xx.is_finite()
        || !covariance.xy.is_finite()
        || !covariance.yy.is_finite()
        || covariance.xx <= 0.0
        || covariance.yy <= 0.0
    {
        return Err(CalibrationError::InvalidCovariance {
            sample_id: observation.sample_id.clone(),
        });
    }

    let determinant = covariance.xx.mul_add(covariance.yy, -covariance.xy * covariance.xy);
    // Reject numerically singular matrices instead of generating enormous,
    // unstable Mahalanobis distances from an effectively non-invertible fit.
    if !determinant.is_finite()
        || determinant <= f64::EPSILON * covariance.xx * covariance.yy
    {
        return Err(CalibrationError::InvalidCovariance {
            sample_id: observation.sample_id.clone(),
        });
    }

    let error_x =
        observation.reference_velocity_m_s[0] - observation.estimated_velocity_m_s[0];
    let error_y =
        observation.reference_velocity_m_s[1] - observation.estimated_velocity_m_s[1];

    let mahalanobis_squared = (covariance.yy * error_x * error_x
        - 2.0 * covariance.xy * error_x * error_y
        + covariance.xx * error_y * error_y)
        / determinant;
    let gaussian_nll = std::f64::consts::TAU.ln()
        + 0.5 * determinant.ln()
        + 0.5 * mahalanobis_squared;
    let ellipse_area = std::f64::consts::PI
        * JOINT_CHI_SQUARED_2_95
        * determinant.sqrt();
    let streamwise_width = 2.0 * MARGINAL_Z_95 * covariance.xx.sqrt();
    let wall_normal_width = 2.0 * MARGINAL_Z_95 * covariance.yy.sqrt();

    if !mahalanobis_squared.is_finite()
        || mahalanobis_squared < 0.0
        || !gaussian_nll.is_finite()
        || !ellipse_area.is_finite()
        || !streamwise_width.is_finite()
        || !wall_normal_width.is_finite()
    {
        return Err(CalibrationError::NonFiniteDerivedValue {
            sample_id: observation.sample_id.clone(),
        });
    }

    Ok((
        error_x,
        error_y,
        mahalanobis_squared,
        gaussian_nll,
        ellipse_area,
    ))
}

fn cluster_confidence_interval(
    cluster_coverages: &[f64],
) -> Option<(f64, f64, f64)> {
    if cluster_coverages.len() < MIN_CLUSTERS_FOR_INTERVAL {
        return None;
    }
    let count = cluster_coverages.len() as f64;
    let mean = cluster_coverages.iter().sum::<f64>() / count;
    let sum_squared_deviation = cluster_coverages
        .iter()
        .map(|value| {
            let deviation = *value - mean;
            deviation * deviation
        })
        .sum::<f64>();
    let sample_variance = sum_squared_deviation / (count - 1.0);
    let standard_error = (sample_variance / count).sqrt();
    let margin = CLUSTER_T_95_DF_19 * standard_error;
    Some(((mean - margin).max(0.0), (mean + margin).min(1.0), mean))
}

fn summarize_coverage(
    sample_count: usize,
    covered_count: usize,
    cluster_counts: impl Iterator<Item = (usize, usize)>,
    nominal_coverage: f64,
) -> CoverageSummary {
    let pooled_coverage = covered_count as f64 / sample_count as f64;
    let cluster_coverages: Vec<f64> = cluster_counts
        .map(|(count, covered)| covered as f64 / count as f64)
        .collect();
    let fallback_mean = cluster_coverages.iter().sum::<f64>()
        / cluster_coverages.len().max(1) as f64;
    let interval = cluster_confidence_interval(&cluster_coverages);
    CoverageSummary {
        sample_count,
        covered_count,
        pooled_coverage,
        cluster_equal_coverage: interval.map(|row| row.2).unwrap_or(fallback_mean),
        cluster_95_ci_lower: interval.map(|row| row.0),
        cluster_95_ci_upper: interval.map(|row| row.1),
        cluster_95_ci_contains_nominal: interval.map(|row| {
            row.0 <= nominal_coverage && nominal_coverage <= row.1
        }),
    }
}

fn coverage_in_target(summary: CoverageSummary, gate: CalibrationGate) -> bool {
    summary.cluster_equal_coverage >= gate.minimum_acceptable_coverage
        && summary.cluster_equal_coverage <= gate.maximum_acceptable_coverage
        && summary.cluster_95_ci_contains_nominal == Some(true)
}

fn summarize_region(
    region: FlowRegion,
    accumulator: RegionAccumulator,
    gate: CalibrationGate,
) -> RegionCalibrationReport {
    let mut sample_count = 0usize;
    let mut streamwise_covered = 0usize;
    let mut wall_normal_covered = 0usize;
    let mut joint_covered = 0usize;
    let mut nll_sum = 0.0;
    let mut mahalanobis_sum = 0.0;
    let mut ellipse_area_sum = 0.0;
    let mut streamwise_width_sum = 0.0;
    let mut wall_normal_width_sum = 0.0;

    for cluster in accumulator.clusters.values() {
        sample_count += cluster.sample_count;
        streamwise_covered += cluster.streamwise_covered;
        wall_normal_covered += cluster.wall_normal_covered;
        joint_covered += cluster.joint_covered;
        nll_sum += cluster.gaussian_nll_sum;
        mahalanobis_sum += cluster.mahalanobis_squared_sum;
        ellipse_area_sum += cluster.ellipse_area_sum;
        streamwise_width_sum += cluster.streamwise_width_sum;
        wall_normal_width_sum += cluster.wall_normal_width_sum;
    }

    let streamwise_marginal_95 = summarize_coverage(
        sample_count,
        streamwise_covered,
        accumulator
            .clusters
            .values()
            .map(|cluster| (cluster.sample_count, cluster.streamwise_covered)),
        gate.nominal_coverage,
    );
    let wall_normal_marginal_95 = summarize_coverage(
        sample_count,
        wall_normal_covered,
        accumulator
            .clusters
            .values()
            .map(|cluster| (cluster.sample_count, cluster.wall_normal_covered)),
        gate.nominal_coverage,
    );
    let joint_2d_ellipse_95 = summarize_coverage(
        sample_count,
        joint_covered,
        accumulator
            .clusters
            .values()
            .map(|cluster| (cluster.sample_count, cluster.joint_covered)),
        gate.nominal_coverage,
    );

    let independent_cluster_count = accumulator.clusters.len();
    let enough_evidence = sample_count >= gate.minimum_samples_per_region
        && independent_cluster_count >= gate.minimum_independent_clusters_per_region
        && joint_2d_ellipse_95.cluster_95_ci_contains_nominal.is_some();

    let disposition = if !enough_evidence {
        CalibrationDisposition::InsufficientIndependentEvidence
    } else if !coverage_in_target(streamwise_marginal_95, gate)
        || !coverage_in_target(wall_normal_marginal_95, gate)
        || !coverage_in_target(joint_2d_ellipse_95, gate)
    {
        let any_nominal_outside_ci = [
            streamwise_marginal_95,
            wall_normal_marginal_95,
            joint_2d_ellipse_95,
        ]
        .iter()
        .any(|summary| summary.cluster_95_ci_contains_nominal == Some(false));
        if any_nominal_outside_ci {
            CalibrationDisposition::NominalOutsideClusterConfidenceInterval
        } else {
            CalibrationDisposition::CoverageOutsideTargetBand
        }
    } else {
        CalibrationDisposition::MeetsDeclaredCalibrationGates
    };

    RegionCalibrationReport {
        region,
        sample_count,
        independent_cluster_count,
        streamwise_marginal_95,
        wall_normal_marginal_95,
        joint_2d_ellipse_95,
        mean_squared_mahalanobis_distance: mahalanobis_sum / sample_count as f64,
        mean_gaussian_nll_nats: nll_sum / sample_count as f64,
        mean_95_ellipse_area_m2_s2: ellipse_area_sum / sample_count as f64,
        mean_streamwise_interval_width_m_s: streamwise_width_sum / sample_count as f64,
        mean_wall_normal_interval_width_m_s: wall_normal_width_sum / sample_count as f64,
        disposition,
    }
}

/// Evaluate 2-D predictive covariance calibration by physical region and
/// independent acquisition/dependence cluster.
///
/// For each sample, this reports both 95% marginal intervals and the 95%
/// joint ellipse defined by eᵀΣ⁻¹e ≤ χ²(2, 0.95). It also reports the proper
/// bivariate Gaussian negative log likelihood and ellipse area so a method
/// cannot improve coverage merely by emitting arbitrarily wide uncertainty.
///
/// The regional cluster-level confidence interval is approximate. The caller
/// must assign cluster identities at the highest defensible common-cause
/// level; different file names or processing algorithms do not prove
/// independent acquisition evidence.
pub fn evaluate_covariance_calibration(
    observations: &[VectorObservation],
    gate: CalibrationGate,
) -> Result<CalibrationReport, CalibrationError> {
    validate_gate(gate)?;
    if observations.is_empty() {
        return Err(CalibrationError::EmptyObservations);
    }

    let mut sample_ids = BTreeSet::new();
    let mut regions: BTreeMap<FlowRegion, RegionAccumulator> = BTreeMap::new();

    for observation in observations {
        if !sample_ids.insert(observation.sample_id.clone()) {
            return Err(CalibrationError::DuplicateSampleId {
                sample_id: observation.sample_id.clone(),
            });
        }
        let (error_x, error_y, mahalanobis_squared, gaussian_nll, ellipse_area) =
            validate_observation(observation)?;
        let covariance = observation.predicted_covariance_m2_s2;
        let streamwise_covered = error_x.abs() <= MARGINAL_Z_95 * covariance.xx.sqrt();
        let wall_normal_covered = error_y.abs() <= MARGINAL_Z_95 * covariance.yy.sqrt();
        let joint_covered = mahalanobis_squared <= JOINT_CHI_SQUARED_2_95;
        let cluster = regions
            .entry(observation.region)
            .or_default()
            .clusters
            .entry(observation.independence_cluster_id.clone())
            .or_default();

        cluster.sample_count += 1;
        cluster.streamwise_covered += if streamwise_covered { 1 } else { 0 };
        cluster.wall_normal_covered += if wall_normal_covered { 1 } else { 0 };
        cluster.joint_covered += if joint_covered { 1 } else { 0 };
        cluster.gaussian_nll_sum += gaussian_nll;
        cluster.mahalanobis_squared_sum += mahalanobis_squared;
        cluster.ellipse_area_sum += ellipse_area;
        cluster.streamwise_width_sum += 2.0 * MARGINAL_Z_95 * covariance.xx.sqrt();
        cluster.wall_normal_width_sum += 2.0 * MARGINAL_Z_95 * covariance.yy.sqrt();
    }

    let mut reports = Vec::with_capacity(REQUIRED_REGION_COUNT);
    for region in FlowRegion::ALL {
        let accumulator = regions
            .remove(&region)
            .ok_or(CalibrationError::MissingRequiredRegion { region })?;
        reports.push(summarize_region(region, accumulator, gate));
    }

    let all_regions_meet_calibration = reports.iter().all(|report| {
        report.disposition == CalibrationDisposition::MeetsDeclaredCalibrationGates
    });

    Ok(CalibrationReport {
        gate,
        regions: reports,
        all_regions_meet_calibration,
        limitations: "Reference-data covariance calibration only; assumes supplied reference velocities are valid; cluster t intervals are approximate and rely on correctly declared independent common-cause groups; does not process images, establish diagnostic passivity, or validate EHD thrust.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        sample_id: impl Into<String>,
        cluster_id: impl Into<String>,
        region: FlowRegion,
        error: [f64; 2],
    ) -> VectorObservation {
        VectorObservation {
            sample_id: sample_id.into(),
            independence_cluster_id: cluster_id.into(),
            region,
            reference_velocity_m_s: [0.0, 0.0],
            estimated_velocity_m_s: [-error[0], -error[1]],
            predicted_covariance_m2_s2: Covariance2 {
                xx: 1.0,
                xy: 0.0,
                yy: 1.0,
            },
        }
    }

    fn calibrated_fixture() -> Vec<VectorObservation> {
        let mut observations = Vec::new();
        for region in FlowRegion::ALL {
            for cluster in 0..20 {
                for sample in 0..100 {
                    let error = if sample < 95 { [0.0, 0.0] } else { [3.0, 3.0] };
                    observations.push(observation(
                        format!("{region:?}-{cluster}-{sample}"),
                        format!("session-{cluster}"),
                        region,
                        error,
                    ));
                }
            }
        }
        observations
    }

    #[test]
    fn calibrated_two_dimensional_covariance_passes_region_and_cluster_gates() {
        let report = evaluate_covariance_calibration(
            &calibrated_fixture(),
            CalibrationGate::default(),
        )
        .unwrap();
        assert!(report.all_regions_meet_calibration);
        assert!(report.regions.iter().all(|region| {
            region.sample_count == 2_000
                && region.independent_cluster_count == 20
                && (region.joint_2d_ellipse_95.cluster_equal_coverage - 0.95).abs() < 1.0e-12
                && region.disposition == CalibrationDisposition::MeetsDeclaredCalibrationGates
        }));
    }

    #[test]
    fn marginal_intervals_can_pass_when_joint_covariance_ellipse_fails() {
        let mut observations = Vec::new();
        for region in FlowRegion::ALL {
            observations.push(observation(
                format!("diagonal-{region:?}"),
                "one-session",
                region,
                [1.9, 1.9],
            ));
        }
        let report = evaluate_covariance_calibration(
            &observations,
            CalibrationGate {
                minimum_samples_per_region: 1,
                minimum_independent_clusters_per_region: 1,
                ..CalibrationGate::default()
            },
        )
        .unwrap();
        for region in &report.regions {
            assert_eq!(region.streamwise_marginal_95.pooled_coverage, 1.0);
            assert_eq!(region.wall_normal_marginal_95.pooled_coverage, 1.0);
            assert_eq!(region.joint_2d_ellipse_95.pooled_coverage, 0.0);
        }
    }

    #[test]
    fn off_diagonal_covariance_changes_the_joint_ellipse_test() {
        let mut observations = Vec::new();
        for region in FlowRegion::ALL {
            let mut row = observation(
                format!("correlated-{region:?}"),
                "session-correlated",
                region,
                [1.0, -1.0],
            );
            row.predicted_covariance_m2_s2 = Covariance2 {
                xx: 1.0,
                xy: 0.9,
                yy: 1.0,
            };
            observations.push(row);
        }
        let report = evaluate_covariance_calibration(
            &observations,
            CalibrationGate {
                minimum_samples_per_region: 1,
                minimum_independent_clusters_per_region: 1,
                ..CalibrationGate::default()
            },
        )
        .unwrap();
        assert!(report.regions.iter().all(|region| {
            region.joint_2d_ellipse_95.pooled_coverage == 0.0
        }));
    }

    #[test]
    fn pooled_vector_coverage_cannot_hide_a_bad_small_session() {
        let mut observations = Vec::new();
        for region in FlowRegion::ALL {
            for sample in 0..1_000 {
                observations.push(observation(
                    format!("large-{region:?}-{sample}"),
                    "large-session",
                    region,
                    [0.0, 0.0],
                ));
            }
            observations.push(observation(
                format!("small-bad-{region:?}"),
                "small-bad-session",
                region,
                [3.0, 0.0],
            ));
        }
        let report = evaluate_covariance_calibration(
            &observations,
            CalibrationGate {
                minimum_samples_per_region: 1,
                minimum_independent_clusters_per_region: 1,
                ..CalibrationGate::default()
            },
        )
        .unwrap();
        for region in &report.regions {
            assert!(region.joint_2d_ellipse_95.pooled_coverage > 0.99);
            assert!((region.joint_2d_ellipse_95.cluster_equal_coverage - 0.5).abs() < 1.0e-12);
        }
    }

    #[test]
    fn many_vectors_from_one_session_do_not_create_independent_evidence() {
        let mut observations = Vec::new();
        for region in FlowRegion::ALL {
            for sample in 0..2_000 {
                observations.push(observation(
                    format!("same-session-{region:?}-{sample}"),
                    "single-session",
                    region,
                    [0.0, 0.0],
                ));
            }
        }
        let report = evaluate_covariance_calibration(
            &observations,
            CalibrationGate::default(),
        )
        .unwrap();
        assert!(report.regions.iter().all(|region| {
            region.sample_count >= 200
                && region.independent_cluster_count == 1
                && region.disposition == CalibrationDisposition::InsufficientIndependentEvidence
        }));
    }

    #[test]
    fn singular_or_non_positive_covariance_is_rejected() {
        let mut observations = Vec::new();
        for region in FlowRegion::ALL {
            let mut row = observation(
                format!("singular-{region:?}"),
                "session",
                region,
                [0.0, 0.0],
            );
            row.predicted_covariance_m2_s2 = Covariance2 {
                xx: 1.0,
                xy: 1.0,
                yy: 1.0,
            };
            observations.push(row);
        }
        assert!(matches!(
            evaluate_covariance_calibration(&observations, CalibrationGate::default()),
            Err(CalibrationError::InvalidCovariance { .. })
        ));
    }

    #[test]
    fn duplicate_sample_identity_is_rejected() {
        let observations = FlowRegion::ALL
            .into_iter()
            .map(|region| observation("same-id", "session", region, [0.0, 0.0]))
            .collect::<Vec<_>>();
        assert!(matches!(
            evaluate_covariance_calibration(&observations, CalibrationGate::default()),
            Err(CalibrationError::DuplicateSampleId { .. })
        ));
    }

    #[test]
    fn missing_required_region_is_rejected() {
        let observations = FlowRegion::ALL[..3]
            .iter()
            .map(|region| observation(format!("only-{region:?}"), "session", *region, [0.0, 0.0]))
            .collect::<Vec<_>>();
        assert!(matches!(
            evaluate_covariance_calibration(&observations, CalibrationGate::default()),
            Err(CalibrationError::MissingRequiredRegion {
                region: FlowRegion::Wake
            })
        ));
    }

    #[test]
    fn invalid_gate_and_non_finite_velocity_are_rejected() {
        let invalid_gate = CalibrationGate {
            nominal_coverage: 0.90,
            minimum_acceptable_coverage: 0.95,
            maximum_acceptable_coverage: 0.98,
            ..CalibrationGate::default()
        };
        assert_eq!(
            evaluate_covariance_calibration(&calibrated_fixture(), invalid_gate).unwrap_err(),
            CalibrationError::InvalidGate
        );

        let mut observations = FlowRegion::ALL
            .into_iter()
            .map(|region| observation(format!("finite-{region:?}"), "session", region, [0.0, 0.0]))
            .collect::<Vec<_>>();
        observations[0].reference_velocity_m_s[0] = f64::NAN;
        assert!(matches!(
            evaluate_covariance_calibration(&observations, CalibrationGate::default()),
            Err(CalibrationError::NonFiniteVelocity { .. })
        ));
    }

    #[test]
    fn gaussian_nll_and_ellipse_area_penalize_arbitrary_uncertainty_inflation() {
        let mut narrow_observations = Vec::new();
        let mut broad_observations = Vec::new();
        for region in FlowRegion::ALL {
            narrow_observations.push(observation(
                format!("narrow-{region:?}"),
                "session",
                region,
                [0.0, 0.0],
            ));
            let mut broad = observation(
                format!("broad-{region:?}"),
                "session",
                region,
                [0.0, 0.0],
            );
            broad.predicted_covariance_m2_s2 = Covariance2 {
                xx: 4.0,
                xy: 0.0,
                yy: 4.0,
            };
            broad_observations.push(broad);
        }
        let gate = CalibrationGate {
            minimum_samples_per_region: 1,
            minimum_independent_clusters_per_region: 1,
            ..CalibrationGate::default()
        };
        let narrow = evaluate_covariance_calibration(&narrow_observations, gate).unwrap();
        let broad = evaluate_covariance_calibration(&broad_observations, gate).unwrap();
        for (narrow_region, broad_region) in narrow.regions.iter().zip(broad.regions.iter()) {
            assert!(
                broad_region.mean_95_ellipse_area_m2_s2
                    > narrow_region.mean_95_ellipse_area_m2_s2
            );
            assert!(
                broad_region.mean_gaussian_nll_nats
                    > narrow_region.mean_gaussian_nll_nats
            );
        }
    }

    #[test]
    fn empty_observations_are_rejected() {
        assert_eq!(
            evaluate_covariance_calibration(&[], CalibrationGate::default()).unwrap_err(),
            CalibrationError::EmptyObservations
        );
    }
}
