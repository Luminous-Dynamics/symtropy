// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Receipt-bound predictive covariance-realism diagnostics.
//!
//! Each observation binds a frozen forecast, a later reference-orbit
//! realization, the covariance semantics used to describe their residual, and
//! a coordinate-frame / model revision. The residual covariance must represent
//! the uncertainty of forecast-minus-reference error, including reference
//! uncertainty and cross-correlation where relevant.
//!
//! The normalized innovation squared (NIS) is computed through a strict
//! Cholesky solve with the full covariance. For a correctly specified
//! zero-mean Gaussian residual model, NIS follows chi-square with d degrees of
//! freedom. The coverage counts below are descriptive diagnostics; this
//! module does not provide a confidence interval for empirical coverage,
//! prove reference-orbit truth, model temporal dependence, or mutate a
//! covariance model.

use std::collections::BTreeSet;

use crate::Gaussian;

/// Predictive ellipsoid coverage level reported by the realism ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CovarianceCoverageLevel {
    /// The 50% predictive ellipsoid.
    P50,
    /// The 90% predictive ellipsoid.
    P90,
    /// The 95% predictive ellipsoid.
    P95,
    /// The 99% predictive ellipsoid.
    P99,
}

impl CovarianceCoverageLevel {
    /// All supported levels in ascending order.
    pub const ALL: [Self; 4] = [Self::P50, Self::P90, Self::P95, Self::P99];

    /// Nominal probability associated with the level.
    pub fn nominal_probability(self) -> f64 {
        match self {
            Self::P50 => 0.50,
            Self::P90 => 0.90,
            Self::P95 => 0.95,
            Self::P99 => 0.99,
        }
    }

    /// Chi-square threshold for the 3D-position or 6D-state residual.
    pub fn chi_square_threshold(self, dimension: usize) -> Option<f64> {
        match (dimension, self) {
            (3, Self::P50) => Some(2.365_973_884_375_337_7),
            (3, Self::P90) => Some(6.251_388_631_170_325),
            (3, Self::P95) => Some(7.814_727_903_251_179),
            (3, Self::P99) => Some(11.344_866_730_144_373),
            (6, Self::P50) => Some(5.348_120_627_447_118),
            (6, Self::P90) => Some(10.644_640_675_668_422),
            (6, Self::P95) => Some(12.591_587_243_743_977),
            (6, Self::P99) => Some(16.811_893_829_770_927),
            _ => None,
        }
    }
}

/// One independently matched forecast-to-reference residual observation.
#[derive(Debug, Clone, PartialEq)]
pub struct CovarianceRealismReceipt {
    /// Stable sequence index within the calibration ledger.
    pub sequence_index: u64,
    /// Stable unique observation identity used for duplicate rejection.
    pub observation_id: [u8; 32],
    /// Digest of the frozen forecast / ephemeris revision.
    pub forecast_digest: [u8; 32],
    /// Digest of the independent reference-orbit solution.
    pub reference_orbit_digest: [u8; 32],
    /// Digest of the raw residual and observation-quality record.
    pub realization_digest: [u8; 32],
    /// Digest of the object whose forecast is being evaluated.
    pub object_digest: [u8; 32],
    /// Digest of the coordinate frame / transformation convention.
    pub coordinate_frame_digest: [u8; 32],
    /// Digest of the orbit-determination / propagation model revision.
    pub model_revision_digest: [u8; 32],
    /// Digest describing how forecast and reference uncertainty are combined.
    ///
    /// The associated matrix must describe the residual covariance, including
    /// reference-orbit uncertainty and cross-correlation if they are material.
    pub residual_covariance_semantics_digest: [u8; 32],
    /// Forecast generation time in TAI nanoseconds since the declared epoch.
    pub forecast_generated_tai_ns: u64,
    /// Time at which this forecast is evaluated, in the same TAI nanosecond epoch.
    pub prediction_epoch_tai_ns: u64,
    /// Declared prediction horizon in nanoseconds; must equal epoch minus generation time.
    pub prediction_horizon_ns: u64,
    /// Forecast-minus-reference residual in the declared frame.
    pub residual: Vec<f64>,
    /// Row-major covariance of the forecast-minus-reference residual.
    pub residual_covariance: Vec<f64>,
}

/// Immutable scope for one covariance-realism calibration cohort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CovarianceRealismScope {
    /// State dimension: 3 for position-only or 6 for position/velocity.
    pub dimension: usize,
    /// Object identity covered by this calibration ledger.
    pub object_digest: [u8; 32],
    /// Coordinate frame used for all residuals.
    pub coordinate_frame_digest: [u8; 32],
    /// Orbit-determination / propagation model revision.
    pub model_revision_digest: [u8; 32],
    /// Residual covariance composition semantics.
    pub residual_covariance_semantics_digest: [u8; 32],
}

/// Empirical predictive-ellipsoid coverage for one nominal level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CovarianceCoverageResult {
    /// Nominal coverage level.
    pub level: CovarianceCoverageLevel,
    /// Chi-square cutoff for this state dimension.
    pub chi_square_threshold: f64,
    /// Number of residuals inside or on the predictive ellipsoid.
    pub covered_count: u64,
    /// Total matched residual count.
    pub observation_count: u64,
    /// Empirical coverage fraction; descriptive only.
    pub empirical_coverage: f64,
    /// Empirical minus nominal coverage.
    pub coverage_error: f64,
}

/// Reproducible descriptive summary of a covariance-realism ledger.
#[derive(Debug, Clone, PartialEq)]
pub struct CovarianceRealismReport {
    /// Scope of the calibration cohort.
    pub scope: CovarianceRealismScope,
    /// Number of accepted forecast/reference residual receipts.
    pub observation_count: u64,
    /// Mean normalized innovation squared.
    pub mean_nis: f64,
    /// Mean NIS divided by state dimension; ideal Gaussian calibration is near 1.
    pub normalized_mean_nis: f64,
    /// Sample variance of NIS, or None when fewer than two observations exist.
    pub sample_variance_nis: Option<f64>,
    /// Mean forecast-minus-reference residual by state component.
    pub mean_residual: Vec<f64>,
    /// Minimum observed NIS.
    pub minimum_nis: f64,
    /// Maximum observed NIS.
    pub maximum_nis: f64,
    /// Mean prediction horizon in seconds.
    pub mean_prediction_horizon_seconds: f64,
    /// Minimum prediction horizon in seconds.
    pub minimum_prediction_horizon_seconds: f64,
    /// Maximum prediction horizon in seconds.
    pub maximum_prediction_horizon_seconds: f64,
    /// Coverage results at 50%, 90%, 95% and 99%.
    pub coverage: [CovarianceCoverageResult; 4],
}

/// Input, receipt, and numerical errors in the calibration ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CovarianceRealismError {
    /// State dimension must be 3 or 6.
    InvalidDimension,
    /// One or more required scope identities are all zero.
    InvalidScopeDigest,
    /// One or more receipt identities are all zero.
    InvalidReceiptDigest,
    /// Receipt object, frame, model or covariance semantics differ from the ledger.
    ScopeMismatch,
    /// Receipt sequence index is not the next expected index.
    NonSequentialReceipt,
    /// Observation identity has already been accepted.
    DuplicateObservation,
    /// Prediction epoch is not strictly later than forecast generation time.
    InvalidPredictionEpoch,
    /// Prediction horizon is zero or differs from epoch minus generation time.
    InvalidPredictionHorizon,
    /// Residual or covariance array shape does not match the declared dimension.
    DimensionMismatch,
    /// Residual contains a non-finite value.
    InvalidResidual,
    /// Residual covariance is asymmetric, non-finite or not positive definite.
    InvalidResidualCovariance,
    /// NIS or streaming diagnostic arithmetic became non-finite.
    NumericalFailure,
    /// Counter or horizon arithmetic overflowed.
    CounterOverflow,
    /// A report was requested before any valid observation was accepted.
    EmptyLedger,
}

impl core::fmt::Display for CovarianceRealismError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidDimension => "covariance realism dimension must be 3 or 6",
            Self::InvalidScopeDigest => "covariance realism scope digest is invalid",
            Self::InvalidReceiptDigest => "a required forecast or realization digest is invalid",
            Self::ScopeMismatch => "forecast/reference receipt differs from the calibration scope",
            Self::NonSequentialReceipt => "covariance realism receipt sequence is not contiguous",
            Self::DuplicateObservation => "covariance realism observation identity is duplicated",
            Self::InvalidPredictionEpoch => "prediction epoch must follow forecast generation time",
            Self::InvalidPredictionHorizon => "prediction horizon does not match the TAI epoch difference",
            Self::DimensionMismatch => "residual or covariance dimension does not match the ledger",
            Self::InvalidResidual => "state residual contains a non-finite value",
            Self::InvalidResidualCovariance => {
                "residual covariance is not finite, symmetric and positive definite"
            }
            Self::NumericalFailure => "covariance realism diagnostic failed numerically",
            Self::CounterOverflow => "covariance realism counter or horizon sum overflowed",
            Self::EmptyLedger => "covariance realism report has no accepted observations",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CovarianceRealismError {}

/// Streaming receipt verifier and predictive-coverage accumulator.
///
/// This accumulator retains exact observation IDs for duplicate rejection,
/// while NIS moments, residual means, horizon moments and coverage counters
/// are maintained in constant-size numeric state.
#[derive(Debug, Clone, PartialEq)]
pub struct CovarianceRealismAccumulator {
    scope: CovarianceRealismScope,
    next_sequence_index: u64,
    observation_ids: BTreeSet<[u8; 32]>,
    previous_prediction_epoch_tai_ns: Option<u64>,
    observation_count: u64,
    mean_nis: f64,
    nis_m2: f64,
    mean_residual: Vec<f64>,
    minimum_nis: f64,
    maximum_nis: f64,
    horizon_sum_ns: u128,
    minimum_horizon_ns: u64,
    maximum_horizon_ns: u64,
    coverage_counts: [u64; 4],
}

impl CovarianceRealismAccumulator {
    /// Create a new ledger for a single object, model revision, frame and residual-covariance semantics.
    pub fn new(scope: CovarianceRealismScope) -> Result<Self, CovarianceRealismError> {
        if scope.dimension != 3 && scope.dimension != 6 {
            return Err(CovarianceRealismError::InvalidDimension);
        }
        if is_zero_digest(&scope.object_digest)
            || is_zero_digest(&scope.coordinate_frame_digest)
            || is_zero_digest(&scope.model_revision_digest)
            || is_zero_digest(&scope.residual_covariance_semantics_digest)
        {
            return Err(CovarianceRealismError::InvalidScopeDigest);
        }

        Ok(Self {
            scope,
            next_sequence_index: 0,
            observation_ids: BTreeSet::new(),
            previous_prediction_epoch_tai_ns: None,
            observation_count: 0,
            mean_nis: 0.0,
            nis_m2: 0.0,
            mean_residual: vec![0.0; scope.dimension],
            minimum_nis: f64::INFINITY,
            maximum_nis: f64::NEG_INFINITY,
            horizon_sum_ns: 0,
            minimum_horizon_ns: u64::MAX,
            maximum_horizon_ns: 0,
            coverage_counts: [0; 4],
        })
    }

    /// Validate and add one forecast/reference residual receipt transactionally.
    ///
    /// The covariance must describe the residual between the predicted and
    /// reference state, not merely the forecast covariance if reference-orbit
    /// uncertainty is material. The exact covariance-combination rule is bound
    /// by the scope semantics digest and remains an external evidence obligation.
    pub fn observe(
        &mut self,
        receipt: CovarianceRealismReceipt,
    ) -> Result<f64, CovarianceRealismError> {
        if is_zero_digest(&receipt.observation_id)
            || is_zero_digest(&receipt.forecast_digest)
            || is_zero_digest(&receipt.reference_orbit_digest)
            || is_zero_digest(&receipt.realization_digest)
        {
            return Err(CovarianceRealismError::InvalidReceiptDigest);
        }
        if receipt.object_digest != self.scope.object_digest
            || receipt.coordinate_frame_digest != self.scope.coordinate_frame_digest
            || receipt.model_revision_digest != self.scope.model_revision_digest
            || receipt.residual_covariance_semantics_digest
                != self.scope.residual_covariance_semantics_digest
        {
            return Err(CovarianceRealismError::ScopeMismatch);
        }
        if receipt.sequence_index != self.next_sequence_index {
            return Err(CovarianceRealismError::NonSequentialReceipt);
        }
        if self.observation_ids.contains(&receipt.observation_id) {
            return Err(CovarianceRealismError::DuplicateObservation);
        }
        if receipt.prediction_epoch_tai_ns <= receipt.forecast_generated_tai_ns {
            return Err(CovarianceRealismError::InvalidPredictionEpoch);
        }
        let expected_horizon = receipt
            .prediction_epoch_tai_ns
            .checked_sub(receipt.forecast_generated_tai_ns)
            .ok_or(CovarianceRealismError::CounterOverflow)?;
        if receipt.prediction_horizon_ns == 0 || receipt.prediction_horizon_ns != expected_horizon {
            return Err(CovarianceRealismError::InvalidPredictionHorizon);
        }
        if self
            .previous_prediction_epoch_tai_ns
            .is_some_and(|previous| receipt.prediction_epoch_tai_ns <= previous)
        {
            return Err(CovarianceRealismError::InvalidPredictionEpoch);
        }
        if receipt.residual.len() != self.scope.dimension
            || receipt.residual_covariance.len()
                != self.scope.dimension * self.scope.dimension
        {
            return Err(CovarianceRealismError::DimensionMismatch);
        }
        if receipt.residual.iter().any(|value| !value.is_finite()) {
            return Err(CovarianceRealismError::InvalidResidual);
        }

        let zero_mean = vec![0.0; self.scope.dimension];
        let gaussian = Gaussian::new(zero_mean, receipt.residual_covariance)
            .map_err(|_| CovarianceRealismError::InvalidResidualCovariance)?;
        let nis = gaussian
            .mahalanobis_distance_squared(&receipt.residual)
            .map_err(|_| CovarianceRealismError::NumericalFailure)?;
        if !nis.is_finite() || nis < 0.0 {
            return Err(CovarianceRealismError::NumericalFailure);
        }

        let next_count = self
            .observation_count
            .checked_add(1)
            .ok_or(CovarianceRealismError::CounterOverflow)?;
        let next_index = self
            .next_sequence_index
            .checked_add(1)
            .ok_or(CovarianceRealismError::CounterOverflow)?;
        let next_horizon_sum = self
            .horizon_sum_ns
            .checked_add(u128::from(receipt.prediction_horizon_ns))
            .ok_or(CovarianceRealismError::CounterOverflow)?;

        let delta = nis - self.mean_nis;
        let next_mean_nis = self.mean_nis + delta / next_count as f64;
        let next_nis_m2 = self.nis_m2 + delta * (nis - next_mean_nis);
        if !next_mean_nis.is_finite() || !next_nis_m2.is_finite() || next_nis_m2 < 0.0 {
            return Err(CovarianceRealismError::NumericalFailure);
        }

        let mut next_mean_residual = self.mean_residual.clone();
        for (mean, residual) in next_mean_residual.iter_mut().zip(&receipt.residual) {
            *mean += (*residual - *mean) / next_count as f64;
            if !mean.is_finite() {
                return Err(CovarianceRealismError::NumericalFailure);
            }
        }

        let mut next_coverage_counts = self.coverage_counts;
        for (index, level) in CovarianceCoverageLevel::ALL.into_iter().enumerate() {
            let threshold = level
                .chi_square_threshold(self.scope.dimension)
                .ok_or(CovarianceRealismError::InvalidDimension)?;
            if nis <= threshold {
                next_coverage_counts[index] = next_coverage_counts[index]
                    .checked_add(1)
                    .ok_or(CovarianceRealismError::CounterOverflow)?;
            }
        }

        self.observation_ids.insert(receipt.observation_id);
        self.next_sequence_index = next_index;
        self.previous_prediction_epoch_tai_ns = Some(receipt.prediction_epoch_tai_ns);
        self.observation_count = next_count;
        self.mean_nis = next_mean_nis;
        self.nis_m2 = next_nis_m2;
        self.mean_residual = next_mean_residual;
        self.minimum_nis = self.minimum_nis.min(nis);
        self.maximum_nis = self.maximum_nis.max(nis);
        self.horizon_sum_ns = next_horizon_sum;
        self.minimum_horizon_ns = self.minimum_horizon_ns.min(receipt.prediction_horizon_ns);
        self.maximum_horizon_ns = self.maximum_horizon_ns.max(receipt.prediction_horizon_ns);
        self.coverage_counts = next_coverage_counts;
        Ok(nis)
    }

    /// Number of accepted forecast/reference residual observations.
    pub fn observation_count(&self) -> u64 {
        self.observation_count
    }

    /// Return the descriptive covariance-realism report.
    pub fn report(&self) -> Result<CovarianceRealismReport, CovarianceRealismError> {
        if self.observation_count == 0 {
            return Err(CovarianceRealismError::EmptyLedger);
        }
        let count = self.observation_count as f64;
        let dimension = self.scope.dimension as f64;
        let coverage = core::array::from_fn(|index| {
            let level = CovarianceCoverageLevel::ALL[index];
            let covered = self.coverage_counts[index];
            let empirical = covered as f64 / count;
            CovarianceCoverageResult {
                level,
                chi_square_threshold: level
                    .chi_square_threshold(self.scope.dimension)
                    .unwrap_or(f64::NAN),
                covered_count: covered,
                observation_count: self.observation_count,
                empirical_coverage: empirical,
                coverage_error: empirical - level.nominal_probability(),
            }
        });
        let mean_horizon_seconds =
            self.horizon_sum_ns as f64 / count / 1_000_000_000.0;

        Ok(CovarianceRealismReport {
            scope: self.scope,
            observation_count: self.observation_count,
            mean_nis: self.mean_nis,
            normalized_mean_nis: self.mean_nis / dimension,
            sample_variance_nis: (self.observation_count > 1)
                .then(|| self.nis_m2 / (count - 1.0)),
            mean_residual: self.mean_residual.clone(),
            minimum_nis: self.minimum_nis,
            maximum_nis: self.maximum_nis,
            mean_prediction_horizon_seconds: mean_horizon_seconds,
            minimum_prediction_horizon_seconds: self.minimum_horizon_ns as f64 / 1_000_000_000.0,
            maximum_prediction_horizon_seconds: self.maximum_horizon_ns as f64 / 1_000_000_000.0,
            coverage,
        })
    }
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBJECT: [u8; 32] = [1; 32];
    const FRAME: [u8; 32] = [2; 32];
    const MODEL: [u8; 32] = [3; 32];
    const COVARIANCE_SEMANTICS: [u8; 32] = [4; 32];

    fn scope(dimension: usize) -> CovarianceRealismScope {
        CovarianceRealismScope {
            dimension,
            object_digest: OBJECT,
            coordinate_frame_digest: FRAME,
            model_revision_digest: MODEL,
            residual_covariance_semantics_digest: COVARIANCE_SEMANTICS,
        }
    }

    fn receipt(
        index: u8,
        residual: Vec<f64>,
        residual_covariance: Vec<f64>,
    ) -> CovarianceRealismReceipt {
        let forecast_generated_tai_ns = 1_000_000_000;
        let prediction_horizon_ns = (u64::from(index) + 1) * 1_000_000_000;
        CovarianceRealismReceipt {
            sequence_index: u64::from(index),
            observation_id: [10 + index; 32],
            forecast_digest: [30 + index; 32],
            reference_orbit_digest: [50 + index; 32],
            realization_digest: [70 + index; 32],
            object_digest: OBJECT,
            coordinate_frame_digest: FRAME,
            model_revision_digest: MODEL,
            residual_covariance_semantics_digest: COVARIANCE_SEMANTICS,
            forecast_generated_tai_ns,
            prediction_epoch_tai_ns: forecast_generated_tai_ns + prediction_horizon_ns,
            prediction_horizon_ns,
            residual,
            residual_covariance,
        }
    }

    #[test]
    fn diagonal_position_covariance_has_known_nis() {
        let covariance = vec![
            4.0, 0.0, 0.0,
            0.0, 9.0, 0.0,
            0.0, 0.0, 16.0,
        ];
        let mut ledger = CovarianceRealismAccumulator::new(scope(3)).unwrap();
        let nis = ledger
            .observe(receipt(0, vec![2.0, 3.0, 4.0], covariance))
            .unwrap();
        assert!((nis - 3.0).abs() < 1.0e-12);
        let report = ledger.report().unwrap();
        assert!((report.normalized_mean_nis - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn correlated_position_covariance_preserves_cross_axis_terms() {
        // L = [[2,0,0],[1,2,0],[0,1,3]], Sigma = L*L^T.
        // Residual L*[1,-2,0.5] = [2,-3,-0.5], so NIS = 1+4+0.25 = 5.25.
        let covariance = vec![
            4.0, 2.0, 0.0,
            2.0, 5.0, 2.0,
            0.0, 2.0, 10.0,
        ];
        let mut ledger = CovarianceRealismAccumulator::new(scope(3)).unwrap();
        let nis = ledger
            .observe(receipt(0, vec![2.0, -3.0, -0.5], covariance))
            .unwrap();
        assert!((nis - 5.25).abs() < 1.0e-12);
    }

    #[test]
    fn six_dimensional_state_covariance_uses_full_cholesky_solve() {
        let covariance = vec![
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 4.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 9.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 16.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 25.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 36.0,
        ];
        let mut ledger = CovarianceRealismAccumulator::new(scope(6)).unwrap();
        let nis = ledger
            .observe(receipt(0, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], covariance))
            .unwrap();
        assert!((nis - 6.0).abs() < 1.0e-12);
    }

    #[test]
    fn predictive_ellipsoid_coverage_counts_use_dimension_specific_quantiles() {
        let identity = vec![
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        let mut ledger = CovarianceRealismAccumulator::new(scope(3)).unwrap();
        ledger.observe(receipt(0, vec![3.0_f64.sqrt(), 0.0, 0.0], identity.clone())).unwrap();
        ledger.observe(receipt(1, vec![8.0_f64.sqrt(), 0.0, 0.0], identity)).unwrap();

        let report = ledger.report().unwrap();
        assert_eq!(report.coverage[0].covered_count, 0);
        assert_eq!(report.coverage[1].covered_count, 1);
        assert_eq!(report.coverage[2].covered_count, 1);
        assert_eq!(report.coverage[3].covered_count, 2);
        assert!((report.coverage[1].empirical_coverage - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn welford_summary_tracks_mean_variance_residual_bias_and_horizon() {
        let identity = vec![
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        let mut ledger = CovarianceRealismAccumulator::new(scope(3)).unwrap();
        ledger.observe(receipt(0, vec![1.0, 0.0, 0.0], identity.clone())).unwrap();
        ledger.observe(receipt(1, vec![3.0, 0.0, 0.0], identity)).unwrap();
        let report = ledger.report().unwrap();

        assert_eq!(report.observation_count, 2);
        assert!((report.mean_nis - 5.0).abs() < 1.0e-12);
        assert!((report.sample_variance_nis.unwrap() - 8.0).abs() < 1.0e-12);
        assert!((report.mean_residual[0] - 2.0).abs() < 1.0e-12);
        assert!((report.mean_prediction_horizon_seconds - 1.5).abs() < 1.0e-12);
        assert!((report.minimum_prediction_horizon_seconds - 1.0).abs() < 1.0e-12);
        assert!((report.maximum_prediction_horizon_seconds - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn receipt_scope_sequence_and_duplicate_identity_are_fail_closed() {
        let identity = vec![
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        let mut ledger = CovarianceRealismAccumulator::new(scope(3)).unwrap();
        let valid = receipt(0, vec![0.0; 3], identity.clone());
        ledger.observe(valid.clone()).unwrap();

        let mut duplicate = valid.clone();
        duplicate.sequence_index = 1;
        assert_eq!(
            ledger.observe(duplicate),
            Err(CovarianceRealismError::DuplicateObservation)
        );

        let mut wrong_model = receipt(1, vec![0.0; 3], identity.clone());
        wrong_model.model_revision_digest = [99; 32];
        assert_eq!(
            ledger.observe(wrong_model),
            Err(CovarianceRealismError::ScopeMismatch)
        );

        let mut wrong_index = receipt(2, vec![0.0; 3], identity);
        assert_eq!(
            ledger.observe(wrong_index.clone()),
            Err(CovarianceRealismError::NonSequentialReceipt)
        );
        wrong_index.sequence_index = 1;
        ledger.observe(wrong_index).unwrap();
        assert_eq!(ledger.observation_count(), 2);
    }

    #[test]
    fn invalid_epoch_horizon_dimension_and_non_spd_covariance_fail_closed() {
        let identity = vec![
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        let mut ledger = CovarianceRealismAccumulator::new(scope(3)).unwrap();

        let mut bad_epoch = receipt(0, vec![0.0; 3], identity.clone());
        bad_epoch.prediction_epoch_tai_ns = bad_epoch.forecast_generated_tai_ns;
        assert_eq!(
            ledger.observe(bad_epoch),
            Err(CovarianceRealismError::InvalidPredictionEpoch)
        );

        let mut bad_horizon = receipt(0, vec![0.0; 3], identity.clone());
        bad_horizon.prediction_horizon_ns += 1;
        assert_eq!(
            ledger.observe(bad_horizon),
            Err(CovarianceRealismError::InvalidPredictionHorizon)
        );

        let bad_dimension = receipt(0, vec![0.0; 2], identity.clone());
        assert_eq!(
            ledger.observe(bad_dimension),
            Err(CovarianceRealismError::DimensionMismatch)
        );

        let non_spd = receipt(
            0,
            vec![0.0; 3],
            vec![
                1.0, 0.0, 0.0,
                0.0, -1.0, 0.0,
                0.0, 0.0, 1.0,
            ],
        );
        assert_eq!(
            ledger.observe(non_spd),
            Err(CovarianceRealismError::InvalidResidualCovariance)
        );
        assert_eq!(ledger.observation_count(), 0);
    }

    #[test]
    fn invalid_dimension_scope_and_empty_report_fail_closed() {
        assert_eq!(
            CovarianceRealismAccumulator::new(scope(4)),
            Err(CovarianceRealismError::InvalidDimension)
        );
        let mut invalid_scope = scope(3);
        invalid_scope.object_digest = [0; 32];
        assert_eq!(
            CovarianceRealismAccumulator::new(invalid_scope),
            Err(CovarianceRealismError::InvalidScopeDigest)
        );
        assert_eq!(
            CovarianceRealismAccumulator::new(scope(3)).unwrap().report(),
            Err(CovarianceRealismError::EmptyLedger)
        );
    }
}
