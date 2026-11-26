// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Descriptive covariance-realism diagnostics for held-out residual histories.
//!
//! For each residual r and declared zero-mean covariance Sigma, compute the
//! normalized innovation squared r^T Sigma^-1 r using the stored Cholesky
//! factor. Under a correctly specified d-dimensional Gaussian residual model,
//! this statistic follows chi-square(d). The auditor reports descriptive
//! ellipsoid coverage at preregistered levels, NIS quantiles, whitened
//! component moments and dependence-cluster sizes.
//!
//! Observations are never silently centered, transformed or pooled across
//! density spaces or strata. Dependence clusters are reported explicitly.
//! This v0.1 emits no IID confidence intervals or inferential calibration
//! p-values: clustered residuals are not assumed independent.

use std::collections::{BTreeMap, BTreeSet};

use crate::Gaussian;

/// Maximum number of residual observations in one audit.
pub const MAX_COVARIANCE_REALISM_OBSERVATIONS: usize = 1_000_000;

/// Maximum residual dimension supported by the reference audit.
pub const MAX_COVARIANCE_REALISM_DIMENSION: usize = 128;

/// Nominal ellipsoid coverage levels, in permille.
pub const NOMINAL_ELLIPSOID_COVERAGE_PERMILLE: [u16; 5] = [500, 900, 950, 990, 999];

/// One prediction/truth residual with frozen provenance identities.
#[derive(Debug, Clone, PartialEq)]
pub struct CovarianceResidualObservation {
    /// Stable, nonzero identity for this prediction/truth pair.
    pub observation_id: [u8; 32],
    /// Stable identity for a dependence cluster (shared OD inputs, forecasts, etc.).
    pub correlation_cluster_id: [u8; 32],
    /// Stable identity for the preregistered diagnostic stratum.
    pub stratum_id: [u8; 32],
    /// Density-space identity: frame, epoch, units and coordinate semantics.
    pub density_space_id: [u8; 32],
    /// Prediction-minus-truth residual in the declared density space.
    pub residual: Vec<f64>,
    /// Zero-mean Gaussian carrying the declared covariance.
    ///
    /// Its mean must be exactly zero; the auditor does not silently center
    /// residuals or change the declared reference point.
    pub covariance: Gaussian,
}

/// Empirical ellipsoid coverage at one nominal Gaussian confidence level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipsoidCoveragePoint {
    /// Nominal coverage probability, in permille.
    pub nominal_coverage_permille: u16,
    /// Chi-square threshold for the declared residual dimension.
    pub chi_square_threshold: f64,
    /// Number of held-out residuals inside the nominal ellipsoid.
    pub inside_count: usize,
    /// Descriptive empirical coverage fraction.
    pub empirical_coverage: f64,
}

/// Distribution-free descriptive quantiles of the observed NIS values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NisQuantileSummary {
    /// Nearest-rank median.
    pub median: f64,
    /// Nearest-rank 95th percentile.
    pub percentile_95: f64,
    /// Nearest-rank 99th percentile.
    pub percentile_99: f64,
    /// Maximum observed normalized innovation squared.
    pub maximum: f64,
}

/// Mean and sample variance of one sequentially whitened residual component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhitenedComponentSummary {
    /// Zero-based component index in the declared Cholesky ordering.
    pub component_index: usize,
    /// Descriptive sample mean.
    pub mean: f64,
    /// Descriptive sample variance (zero for one observation).
    pub sample_variance: f64,
}

/// Covariance-realism summary for one density-space / stratum / dimension group.
#[derive(Debug, Clone, PartialEq)]
pub struct CovarianceRealismGroupSummary {
    /// Density-space identity; never pooled with another frame/epoch/unit space.
    pub density_space_id: [u8; 32],
    /// Preregistered orbit-regime / OD-age / object-class stratum identity.
    pub stratum_id: [u8; 32],
    /// Residual vector dimension and chi-square degrees of freedom.
    pub dimension: usize,
    /// Number of accepted prediction/truth residual pairs.
    pub observation_count: usize,
    /// Number of distinct dependence clusters represented.
    pub correlation_cluster_count: usize,
    /// Largest number of observations in one dependence cluster.
    pub largest_cluster_observation_count: usize,
    /// Largest cluster size divided by the total observation count.
    pub largest_cluster_fraction: f64,
    /// Descriptive NIS distribution.
    pub nis_quantiles: NisQuantileSummary,
    /// Descriptive ellipsoid coverage at frozen nominal levels.
    pub ellipsoid_coverage: Vec<EllipsoidCoveragePoint>,
    /// Sequentially whitened residual-component diagnostics.
    pub whitened_components: Vec<WhitenedComponentSummary>,
}

/// Complete descriptive covariance-realism report.
#[derive(Debug, Clone, PartialEq)]
pub struct CovarianceRealismReport {
    /// Number of input observations accepted.
    pub observation_count: usize,
    /// Number of separately reported density-space / stratum / dimension groups.
    pub group_count: usize,
    /// NIS quantiles use the deterministic nearest-rank definition.
    pub quantile_method: &'static str,
    /// Reports are descriptive; no IID confidence interval is computed.
    pub inferential_status: &'static str,
    /// Groups are emitted in canonical density-space / stratum / dimension order.
    pub groups: Vec<CovarianceRealismGroupSummary>,
}

/// Input-validation and numerical errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CovarianceRealismError {
    /// The audit cannot be run without residual observations.
    EmptyObservationSet,
    /// Observation count exceeds the bounded reference-audit limit.
    TooManyObservations,
    /// Residual dimension is zero or exceeds the configured limit.
    InvalidDimension,
    /// One or more required identity fields are zero.
    InvalidIdentity,
    /// A prediction/truth observation identity is duplicated.
    DuplicateObservationIdentity,
    /// Residual vector length differs from the covariance dimension.
    DimensionMismatch,
    /// Residual vector contains a non-finite component.
    NonFiniteResidual,
    /// Covariance Gaussian has a nonzero mean; the audit never silently recenters.
    NonZeroCovarianceReferenceMean,
    /// Cholesky whitening or a derived statistic failed numerically.
    NumericalFailure,
}

impl core::fmt::Display for CovarianceRealismError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::EmptyObservationSet => "covariance realism requires at least one residual observation",
            Self::TooManyObservations => "residual observation count exceeds the configured limit",
            Self::InvalidDimension => "residual dimension is zero or exceeds the configured limit",
            Self::InvalidIdentity => "observation, cluster, stratum or density-space identity is zero",
            Self::DuplicateObservationIdentity => "prediction/truth observation identity is duplicated",
            Self::DimensionMismatch => "residual and covariance dimensions differ",
            Self::NonFiniteResidual => "residual contains a non-finite value",
            Self::NonZeroCovarianceReferenceMean => "covariance reference mean must be exactly zero",
            Self::NumericalFailure => "NIS, chi-square reference or summary calculation failed numerically",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CovarianceRealismError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    density_space_id: [u8; 32],
    stratum_id: [u8; 32],
    dimension: usize,
}

#[derive(Debug, Clone)]
struct GroupAccumulator {
    nis_values: Vec<f64>,
    cluster_counts: BTreeMap<[u8; 32], usize>,
    whitened_means: Vec<f64>,
    whitened_m2: Vec<f64>,
}

impl GroupAccumulator {
    fn new(dimension: usize) -> Self {
        Self {
            nis_values: Vec::new(),
            cluster_counts: BTreeMap::new(),
            whitened_means: vec![0.0; dimension],
            whitened_m2: vec![0.0; dimension],
        }
    }

    fn add(
        &mut self,
        correlation_cluster_id: [u8; 32],
        nis: f64,
        whitened: &[f64],
    ) -> Result<(), CovarianceRealismError> {
        if !nis.is_finite() || nis < 0.0 || whitened.iter().any(|value| !value.is_finite()) {
            return Err(CovarianceRealismError::NumericalFailure);
        }
        let next_count = self
            .nis_values
            .len()
            .checked_add(1)
            .ok_or(CovarianceRealismError::NumericalFailure)?;
        let count = next_count as f64;

        for index in 0..whitened.len() {
            let delta = whitened[index] - self.whitened_means[index];
            let next_mean = self.whitened_means[index] + delta / count;
            let delta_after = whitened[index] - next_mean;
            let next_m2 = self.whitened_m2[index] + delta * delta_after;
            if !next_mean.is_finite() || !next_m2.is_finite() || next_m2 < 0.0 {
                return Err(CovarianceRealismError::NumericalFailure);
            }
            self.whitened_means[index] = next_mean;
            self.whitened_m2[index] = next_m2;
        }

        self.nis_values.push(nis);
        let cluster_count = self.cluster_counts.entry(correlation_cluster_id).or_insert(0);
        *cluster_count = cluster_count
            .checked_add(1)
            .ok_or(CovarianceRealismError::NumericalFailure)?;
        Ok(())
    }

    fn summarize(
        mut self,
        key: GroupKey,
    ) -> Result<CovarianceRealismGroupSummary, CovarianceRealismError> {
        if self.nis_values.is_empty() {
            return Err(CovarianceRealismError::NumericalFailure);
        }
        self.nis_values.sort_by(f64::total_cmp);
        let count = self.nis_values.len();
        let nearest_rank = |permille: u16| -> f64 {
            let rank = ((f64::from(permille) / 1000.0) * count as f64)
                .ceil()
                .max(1.0) as usize;
            self.nis_values[rank.min(count) - 1]
        };
        let largest_cluster_observation_count =
            self.cluster_counts.values().copied().max().unwrap_or(0);
        let cluster_count = self.cluster_counts.len();
        let coverage = NOMINAL_ELLIPSOID_COVERAGE_PERMILLE
            .iter()
            .map(|nominal| {
                let threshold = chi_square_quantile(
                    f64::from(*nominal) / 1000.0,
                    key.dimension,
                )?;
                let inside_count = self
                    .nis_values
                    .iter()
                    .filter(|value| **value <= threshold)
                    .count();
                Ok(EllipsoidCoveragePoint {
                    nominal_coverage_permille: *nominal,
                    chi_square_threshold: threshold,
                    inside_count,
                    empirical_coverage: inside_count as f64 / count as f64,
                })
            })
            .collect::<Result<Vec<_>, CovarianceRealismError>>()?;

        let whitened_components = (0..key.dimension)
            .map(|index| {
                let sample_variance = if count <= 1 {
                    0.0
                } else {
                    self.whitened_m2[index] / (count - 1) as f64
                };
                WhitenedComponentSummary {
                    component_index: index,
                    mean: self.whitened_means[index],
                    sample_variance,
                }
            })
            .collect();

        let report = CovarianceRealismGroupSummary {
            density_space_id: key.density_space_id,
            stratum_id: key.stratum_id,
            dimension: key.dimension,
            observation_count: count,
            correlation_cluster_count: cluster_count,
            largest_cluster_observation_count,
            largest_cluster_fraction: largest_cluster_observation_count as f64 / count as f64,
            nis_quantiles: NisQuantileSummary {
                median: nearest_rank(500),
                percentile_95: nearest_rank(950),
                percentile_99: nearest_rank(990),
                maximum: self.nis_values[count - 1],
            },
            ellipsoid_coverage: coverage,
            whitened_components,
        };
        Ok(report)
    }
}

/// Audit held-out residuals against their declared covariance matrices.
///
/// Each Gaussian covariance must have an exactly zero reference mean and a
/// dimension matching the residual. The Gaussian constructor already enforces
/// finite, symmetric and positive-definite covariance input. This function
/// performs no frame, epoch, unit or mean correction.
///
/// Results are descriptive. Correlation cluster counts and sizes are emitted
/// so downstream analysis cannot silently pretend that all rows are iid.
pub fn audit_covariance_realism(
    observations: &[CovarianceResidualObservation],
) -> Result<CovarianceRealismReport, CovarianceRealismError> {
    if observations.is_empty() {
        return Err(CovarianceRealismError::EmptyObservationSet);
    }
    if observations.len() > MAX_COVARIANCE_REALISM_OBSERVATIONS {
        return Err(CovarianceRealismError::TooManyObservations);
    }

    let mut observation_ids = BTreeSet::new();
    let mut groups: BTreeMap<GroupKey, GroupAccumulator> = BTreeMap::new();

    for observation in observations {
        if observation.observation_id.iter().all(|byte| *byte == 0)
            || observation.correlation_cluster_id.iter().all(|byte| *byte == 0)
            || observation.stratum_id.iter().all(|byte| *byte == 0)
            || observation.density_space_id.iter().all(|byte| *byte == 0)
        {
            return Err(CovarianceRealismError::InvalidIdentity);
        }
        if !observation_ids.insert(observation.observation_id) {
            return Err(CovarianceRealismError::DuplicateObservationIdentity);
        }
        let dimension = observation.covariance.dimension();
        if dimension == 0 || dimension > MAX_COVARIANCE_REALISM_DIMENSION {
            return Err(CovarianceRealismError::InvalidDimension);
        }
        if observation.residual.len() != dimension {
            return Err(CovarianceRealismError::DimensionMismatch);
        }
        if observation.residual.iter().any(|value| !value.is_finite()) {
            return Err(CovarianceRealismError::NonFiniteResidual);
        }
        if observation
            .covariance
            .mean()
            .iter()
            .any(|value| *value != 0.0)
        {
            return Err(CovarianceRealismError::NonZeroCovarianceReferenceMean);
        }

        let whitened = whiten_residual(
            observation.covariance.lower_cholesky_values(),
            &observation.residual,
            dimension,
        )?;
        let nis = whitened.iter().map(|value| value * value).sum::<f64>();
        if !nis.is_finite() || whitened.iter().any(|value| !value.is_finite()) {
            return Err(CovarianceRealismError::NumericalFailure);
        }

        let key = GroupKey {
            density_space_id: observation.density_space_id,
            stratum_id: observation.stratum_id,
            dimension,
        };
        groups
            .entry(key)
            .or_insert_with(|| GroupAccumulator::new(dimension))
            .add(observation.correlation_cluster_id, nis, &whitened)?;
    }

    let summaries = groups
        .into_iter()
        .map(|(key, accumulator)| accumulator.summarize(key))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CovarianceRealismReport {
        observation_count: observations.len(),
        group_count: summaries.len(),
        quantile_method: "nearest-rank",
        inferential_status: "descriptive-only; dependence clusters are reported; no iid confidence intervals",
        groups: summaries,
    })
}

/// Solve L z = residual using the lower row-major Cholesky factor.
fn whiten_residual(
    lower_cholesky: &[f64],
    residual: &[f64],
    dimension: usize,
) -> Result<Vec<f64>, CovarianceRealismError> {
    if lower_cholesky.len() != dimension * dimension || residual.len() != dimension {
        return Err(CovarianceRealismError::DimensionMismatch);
    }
    let mut whitened = vec![0.0; dimension];
    for row in 0..dimension {
        let mut value = residual[row];
        for column in 0..row {
            value -= lower_cholesky[row * dimension + column] * whitened[column];
        }
        let diagonal = lower_cholesky[row * dimension + row];
        if !diagonal.is_finite() || diagonal <= 0.0 {
            return Err(CovarianceRealismError::NumericalFailure);
        }
        whitened[row] = value / diagonal;
        if !whitened[row].is_finite() {
            return Err(CovarianceRealismError::NumericalFailure);
        }
    }
    Ok(whitened)
}

/// Invert the chi-square CDF for a positive integer degree of freedom.
///
/// Bisection is deterministic and uses the regularized incomplete-gamma
/// function. Its tolerance is appropriate for benchmark reference thresholds,
/// not a formally verified interval-arithmetic guarantee.
fn chi_square_quantile(
    probability: f64,
    degrees_of_freedom: usize,
) -> Result<f64, CovarianceRealismError> {
    if !probability.is_finite()
        || probability <= 0.0
        || probability >= 1.0
        || degrees_of_freedom == 0
    {
        return Err(CovarianceRealismError::NumericalFailure);
    }
    let shape = degrees_of_freedom as f64 * 0.5;
    let mut lower = 0.0;
    let mut upper = (degrees_of_freedom as f64 + 10.0 * (2.0 * degrees_of_freedom as f64).sqrt())
        .max(1.0);
    let mut upper_cdf = regularized_gamma_p(shape, upper * 0.5)?;
    for _ in 0..128 {
        if upper_cdf >= probability {
            break;
        }
        upper *= 2.0;
        if !upper.is_finite() {
            return Err(CovarianceRealismError::NumericalFailure);
        }
        upper_cdf = regularized_gamma_p(shape, upper * 0.5)?;
    }
    if upper_cdf < probability {
        return Err(CovarianceRealismError::NumericalFailure);
    }

    for _ in 0..100 {
        let middle = lower + (upper - lower) * 0.5;
        let cdf = regularized_gamma_p(shape, middle * 0.5)?;
        if cdf < probability {
            lower = middle;
        } else {
            upper = middle;
        }
        if (upper - lower) <= 8.0 * f64::EPSILON * upper.max(1.0) {
            break;
        }
    }
    let result = lower + (upper - lower) * 0.5;
    if result.is_finite() && result > 0.0 {
        Ok(result)
    } else {
        Err(CovarianceRealismError::NumericalFailure)
    }
}

/// Regularized lower incomplete gamma P(a,x) for a>0, x>=0.
fn regularized_gamma_p(a: f64, x: f64) -> Result<f64, CovarianceRealismError> {
    if !a.is_finite() || a <= 0.0 || !x.is_finite() || x < 0.0 {
        return Err(CovarianceRealismError::NumericalFailure);
    }
    if x == 0.0 {
        return Ok(0.0);
    }
    let log_factor = -x + a * x.ln() - log_gamma(a);
    if !log_factor.is_finite() {
        return Err(CovarianceRealismError::NumericalFailure);
    }
    let factor = log_factor.exp();
    if !factor.is_finite() {
        return Err(CovarianceRealismError::NumericalFailure);
    }

    if x < a + 1.0 {
        let mut ap = a;
        let mut term = 1.0 / a;
        let mut sum = term;
        for _ in 0..100_000 {
            ap += 1.0;
            term *= x / ap;
            sum += term;
            if !term.is_finite() || !sum.is_finite() {
                return Err(CovarianceRealismError::NumericalFailure);
            }
            if term.abs() <= sum.abs() * 2.0e-15 {
                break;
            }
        }
        let value = sum * factor;
        if value.is_finite() {
            Ok(value.clamp(0.0, 1.0))
        } else {
            Err(CovarianceRealismError::NumericalFailure)
        }
    } else {
        let tiny = 1.0e-300;
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / tiny;
        let mut d = 1.0 / b;
        let mut h = d;
        for index in 1..=100_000 {
            let i = index as f64;
            let an = -i * (i - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < tiny {
                d = tiny;
            }
            c = b + an / c;
            if c.abs() < tiny {
                c = tiny;
            }
            d = 1.0 / d;
            let delta = d * c;
            h *= delta;
            if !h.is_finite() {
                return Err(CovarianceRealismError::NumericalFailure);
            }
            if (delta - 1.0).abs() <= 2.0e-15 {
                break;
            }
        }
        let q = factor * h;
        let p = 1.0 - q;
        if p.is_finite() {
            Ok(p.clamp(0.0, 1.0))
        } else {
            Err(CovarianceRealismError::NumericalFailure)
        }
    }
}

/// Lanczos log-gamma approximation for positive inputs.
fn log_gamma(value: f64) -> f64 {
    const COEFFICIENTS: [f64; 8] = [
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if value < 0.5 {
        return core::f64::consts::PI.ln()
            - (core::f64::consts::PI * value).sin().ln()
            - log_gamma(1.0 - value);
    }
    let z = value - 1.0;
    let mut sum = 0.999_999_999_999_809_9;
    for (index, coefficient) in COEFFICIENTS.iter().enumerate() {
        sum += coefficient / (z + index as f64 + 1.0);
    }
    let t = z + 7.5;
    0.5 * core::f64::consts::TAU.ln() + (z + 0.5) * t.ln() - t + sum.ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covariance_1d(variance: f64) -> Gaussian {
        Gaussian::new(vec![0.0], vec![variance]).unwrap()
    }

    fn observation(
        id: u8,
        cluster: u8,
        stratum: u8,
        density_space: u8,
        residual: Vec<f64>,
        covariance: Gaussian,
    ) -> CovarianceResidualObservation {
        CovarianceResidualObservation {
            observation_id: [id; 32],
            correlation_cluster_id: [cluster; 32],
            stratum_id: [stratum; 32],
            density_space_id: [density_space; 32],
            residual,
            covariance,
        }
    }

    #[test]
    fn chi_square_one_degree_quantiles_match_known_answers() {
        let expected = [
            (500, 0.454_936_423_119_572_7),
            (900, 2.705_543_454_095_404),
            (950, 3.841_458_820_694_124),
            (990, 6.634_896_601_021_214_5),
            (999, 10.827_566_170_662_733),
        ];
        for (level, reference) in expected {
            let actual = chi_square_quantile(level as f64 / 1000.0, 1).unwrap();
            assert!((actual - reference).abs() < 2.0e-10);
        }
    }

    #[test]
    fn chi_square_two_degree_quantiles_match_closed_form() {
        let expected = [
            (500, 1.386_294_361_119_890_6),
            (900, 4.605_170_185_988_092),
            (950, 5.991_464_547_107_979),
            (990, 9.210_340_371_976_18),
            (999, 13.815_510_557_964_274),
        ];
        for (level, reference) in expected {
            let actual = chi_square_quantile(level as f64 / 1000.0, 2).unwrap();
            assert!((actual - reference).abs() < 2.0e-10);
        }
    }

    #[test]
    fn chi_square_six_degree_quantiles_match_reference_values() {
        let expected = [
            (500, 5.348_120_627_447_118),
            (900, 10.644_640_675_668_422),
            (950, 12.591_587_243_743_977),
            (990, 16.811_893_829_770_927),
            (999, 22.457_744_484_825_323),
        ];
        for (level, reference) in expected {
            let actual = chi_square_quantile(level as f64 / 1000.0, 6).unwrap();
            assert!((actual - reference).abs() < 2.0e-9);
        }
    }

    #[test]
    fn two_dimensional_cholesky_whitening_matches_manual_nis() {
        let covariance = Gaussian::new(vec![0.0, 0.0], vec![4.0, 2.0, 2.0, 5.0]).unwrap();
        let report = audit_covariance_realism(&[
            observation(1, 10, 20, 30, vec![2.0, 3.0], covariance),
        ])
        .unwrap();
        let group = &report.groups[0];

        assert_eq!(group.dimension, 2);
        assert_eq!(group.observation_count, 1);
        assert_eq!(group.nis_quantiles.median, 2.0);
        assert_eq!(group.nis_quantiles.maximum, 2.0);
        assert_eq!(group.whitened_components[0].mean, 1.0);
        assert_eq!(group.whitened_components[1].mean, 1.0);
        assert_eq!(group.correlation_cluster_count, 1);
        assert_eq!(group.largest_cluster_observation_count, 1);
    }

    #[test]
    fn density_spaces_and_strata_are_never_silently_pooled() {
        let observations = [
            observation(1, 10, 20, 30, vec![0.0], covariance_1d(1.0)),
            observation(2, 11, 20, 30, vec![1.0], covariance_1d(1.0)),
            observation(3, 12, 21, 30, vec![2.0], covariance_1d(1.0)),
            observation(4, 13, 20, 31, vec![3.0], covariance_1d(1.0)),
        ];
        let report = audit_covariance_realism(&observations).unwrap();

        assert_eq!(report.observation_count, 4);
        assert_eq!(report.group_count, 3);
        assert_eq!(
            report
                .groups
                .iter()
                .map(|group| (group.density_space_id[0], group.stratum_id[0]))
                .collect::<Vec<_>>(),
            vec![(30, 20), (30, 21), (31, 20)]
        );
    }

    #[test]
    fn dependence_cluster_sizes_are_reported_without_iid_intervals() {
        let observations = [
            observation(1, 10, 20, 30, vec![0.0], covariance_1d(1.0)),
            observation(2, 10, 20, 30, vec![0.1], covariance_1d(1.0)),
            observation(3, 10, 20, 30, vec![0.2], covariance_1d(1.0)),
            observation(4, 11, 20, 30, vec![0.3], covariance_1d(1.0)),
        ];
        let report = audit_covariance_realism(&observations).unwrap();
        let group = &report.groups[0];

        assert_eq!(group.correlation_cluster_count, 2);
        assert_eq!(group.largest_cluster_observation_count, 3);
        assert_eq!(group.largest_cluster_fraction, 0.75);
        assert!(report.inferential_status.contains("no iid confidence intervals"));
    }

    #[test]
    fn whitened_component_means_and_variances_are_descriptive() {
        let covariance = Gaussian::new(vec![0.0, 0.0], vec![4.0, 2.0, 2.0, 5.0]).unwrap();
        let report = audit_covariance_realism(&[
            observation(1, 10, 20, 30, vec![2.0, 3.0], covariance.clone()),
            observation(2, 11, 20, 30, vec![0.0, 1.0], covariance),
        ])
        .unwrap();
        let group = &report.groups[0];

        assert!((group.whitened_components[0].mean - 0.5).abs() < 1.0e-15);
        assert!((group.whitened_components[0].sample_variance - 0.5).abs() < 1.0e-15);
        assert!((group.whitened_components[1].mean - 0.75).abs() < 1.0e-15);
        assert!((group.whitened_components[1].sample_variance - 0.125).abs() < 1.0e-15);
    }

    #[test]
    fn duplicate_and_zero_identities_are_rejected() {
        let covariance = covariance_1d(1.0);
        let duplicate = [
            observation(1, 10, 20, 30, vec![0.0], covariance.clone()),
            observation(1, 11, 20, 30, vec![1.0], covariance.clone()),
        ];
        assert_eq!(
            audit_covariance_realism(&duplicate),
            Err(CovarianceRealismError::DuplicateObservationIdentity)
        );

        let zero_cluster = [observation(1, 0, 20, 30, vec![0.0], covariance)];
        assert_eq!(
            audit_covariance_realism(&zero_cluster),
            Err(CovarianceRealismError::InvalidIdentity)
        );
    }

    #[test]
    fn nonzero_covariance_mean_is_not_silently_subtracted() {
        let covariance = Gaussian::new(vec![0.1], vec![1.0]).unwrap();
        let observations = [observation(1, 10, 20, 30, vec![0.0], covariance)];
        assert_eq!(
            audit_covariance_realism(&observations),
            Err(CovarianceRealismError::NonZeroCovarianceReferenceMean)
        );
    }

    #[test]
    fn sample_order_does_not_change_any_group_metric() {
        let observations = [
            observation(1, 10, 20, 30, vec![0.0], covariance_1d(1.0)),
            observation(2, 10, 20, 30, vec![1.0], covariance_1d(1.0)),
            observation(3, 11, 20, 30, vec![2.0], covariance_1d(1.0)),
            observation(4, 12, 20, 31, vec![3.0], covariance_1d(1.0)),
        ];
        let mut reversed = observations.clone();
        reversed.reverse();
        assert_eq!(
            audit_covariance_realism(&observations),
            audit_covariance_realism(&reversed)
        );
    }

    #[test]
    fn all_five_nominal_coverage_levels_are_reported() {
        let report = audit_covariance_realism(&[
            observation(1, 10, 20, 30, vec![0.0], covariance_1d(1.0)),
        ])
        .unwrap();
        assert_eq!(
            report.groups[0]
                .ellipsoid_coverage
                .iter()
                .map(|point| point.nominal_coverage_permille)
                .collect::<Vec<_>>(),
            NOMINAL_ELLIPSOID_COVERAGE_PERMILLE
        );
    }
}
