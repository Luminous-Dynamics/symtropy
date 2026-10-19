// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Response-weighted sampling for multispectral and hyperspectral observations.
//!
//! This module is a renderer-independent numerical kernel. It integrates a sampled
//! spectral signal against a caller-supplied relative spectral response function
//! (RSR), producing a band-equivalent value in the input signal's declared units.
//!
//! The signal and response are linearly interpolated between their declared knots.
//! The product of two linear segments is integrated analytically on each interval;
//! this avoids adding avoidable trapezoidal-product error. Source samples must cover
//! the entire response support: the implementation never silently extrapolates.
//!
//! Uncertainty propagation assumes independent input-sample uncertainties. It does
//! not include response-curve, calibration, registration, atmospheric, or model
//! uncertainty. This module neither authenticates source data nor calibrates a
//! sensor; callers must retain the original provenance and calibration evidence.

/// Quantity carried by a spectral signal. No unit conversion is performed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpectralSignalQuantity {
    /// Spectral radiance in W/(sr m^2 um).
    SpectralRadianceWPerSrM2Um,
    /// Dimensionless surface reflectance in [0, 1].
    ReflectanceFraction,
}

impl SpectralSignalQuantity {
    pub const fn unit(self) -> &'static str {
        match self {
            Self::SpectralRadianceWPerSrM2Um => "W/(sr m^2 um)",
            Self::ReflectanceFraction => "1",
        }
    }
}

/// A single sample of a dimensionless relative spectral response curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpectralResponseSample {
    /// Wavelength in nanometres.
    pub wavelength_nm: f64,
    /// Dimensionless relative response, constrained to [0, 1].
    pub relative_response: f64,
}

/// A sensor or processing-band response function.
///
/// The identifier is caller supplied and is not an authenticated sensor identity.
/// Multiplying all response values by a common positive factor does not change the
/// weighted signal value; samples are nevertheless constrained to [0, 1] so that
/// malformed response curves are caught at the import boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct SpectralResponseFunction {
    pub identifier: String,
    pub samples: Vec<SpectralResponseSample>,
}

impl SpectralResponseFunction {
    pub fn validate(&self) -> Result<(), SpectralResponseError> {
        if self.identifier.trim().is_empty() {
            return Err(SpectralResponseError::MissingResponseIdentifier);
        }
        if self.samples.len() < 2 {
            return Err(SpectralResponseError::TooFewResponseSamples);
        }

        let mut has_positive_response = false;
        let mut previous_wavelength = None;
        for sample in &self.samples {
            if !sample.wavelength_nm.is_finite() || sample.wavelength_nm <= 0.0 {
                return Err(SpectralResponseError::InvalidWavelength);
            }
            if previous_wavelength.is_some_and(|previous| sample.wavelength_nm <= previous) {
                return Err(SpectralResponseError::NonIncreasingWavelength);
            }
            if !sample.relative_response.is_finite()
                || !(0.0..=1.0).contains(&sample.relative_response)
            {
                return Err(SpectralResponseError::InvalidRelativeResponse);
            }
            has_positive_response |= sample.relative_response > 0.0;
            previous_wavelength = Some(sample.wavelength_nm);
        }

        if !has_positive_response {
            return Err(SpectralResponseError::ZeroResponse);
        }
        Ok(())
    }

    pub fn wavelength_support_nm(&self) -> Option<[f64; 2]> {
        Some([
            self.samples.first()?.wavelength_nm,
            self.samples.last()?.wavelength_nm,
        ])
    }
}

/// A sampled spectral value and its optional standard uncertainty.
///
/// Wavelength is in nanometres. Value and uncertainty use the units implied by the
/// enclosing SpectralSignalCurve. Uncertainty is an absolute one-standard-deviation
/// uncertainty in those same units, not a relative percentage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpectralSignalSample {
    pub wavelength_nm: f64,
    pub value: f64,
    pub standard_uncertainty: Option<f64>,
}

/// A sampled, homogeneous spectral signal.
///
/// Samples must share the declared quantity and unit. The sample wavelengths are
/// strictly increasing and must cover the full support of the response function.
#[derive(Clone, Debug, PartialEq)]
pub struct SpectralSignalCurve {
    pub quantity: SpectralSignalQuantity,
    pub samples: Vec<SpectralSignalSample>,
}

impl SpectralSignalCurve {
    pub fn validate(&self) -> Result<(), SpectralResponseError> {
        if self.samples.len() < 2 {
            return Err(SpectralResponseError::TooFewSignalSamples);
        }

        let mut previous_wavelength = None;
        for sample in &self.samples {
            if !sample.wavelength_nm.is_finite() || sample.wavelength_nm <= 0.0 {
                return Err(SpectralResponseError::InvalidWavelength);
            }
            if previous_wavelength.is_some_and(|previous| sample.wavelength_nm <= previous) {
                return Err(SpectralResponseError::NonIncreasingWavelength);
            }
            if !sample.value.is_finite() {
                return Err(SpectralResponseError::InvalidSignalValue);
            }
            match self.quantity {
                SpectralSignalQuantity::SpectralRadianceWPerSrM2Um if sample.value < 0.0 => {
                    return Err(SpectralResponseError::InvalidSignalValue);
                }
                SpectralSignalQuantity::ReflectanceFraction
                    if !(0.0..=1.0).contains(&sample.value) =>
                {
                    return Err(SpectralResponseError::InvalidSignalValue);
                }
                _ => {}
            }
            if let Some(uncertainty) = sample.standard_uncertainty
                && (!uncertainty.is_finite() || uncertainty < 0.0)
            {
                return Err(SpectralResponseError::InvalidUncertainty);
            }
            previous_wavelength = Some(sample.wavelength_nm);
        }
        Ok(())
    }
}

/// Result of integrating a spectral signal against a response curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpectralBandObservation {
    pub quantity: SpectralSignalQuantity,
    /// Response-weighted mean in the input signal's declared units.
    pub value: f64,
    /// Propagated standard uncertainty when every source sample has one.
    ///
    /// The calculation treats source-sample uncertainties as independent and omits
    /// response-function and calibration uncertainty. Missing any source uncertainty
    /// yields None rather than a deceptively partial uncertainty.
    pub standard_uncertainty: Option<f64>,
    /// Integral of the dimensionless response with respect to wavelength, in nm.
    pub response_integral_nm: f64,
    /// Response support actually integrated, in nm.
    pub wavelength_support_nm: [f64; 2],
    /// Count of source samples with a nonzero integrated interpolation weight.
    pub contributing_sample_count: usize,
}

/// Errors detected before or during response-weighted spectral integration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpectralResponseError {
    MissingResponseIdentifier,
    TooFewResponseSamples,
    TooFewSignalSamples,
    InvalidWavelength,
    NonIncreasingWavelength,
    InvalidRelativeResponse,
    ZeroResponse,
    InvalidSignalValue,
    InvalidUncertainty,
    InsufficientSpectralCoverage,
    DegenerateIntegrationInterval,
    NonFiniteIntegrationResult,
}

/// Integrates a sampled spectral curve against a wavelength-dependent response.
///
/// This computes
///
///   integral(signal(lambda) * response(lambda) d-lambda)
///   ------------------------------------------------------
///                 integral(response(lambda) d-lambda)
///
/// using piecewise-linear interpolation for both input curves. On each interval
/// formed by the union of their knots, the product is quadratic and its integral
/// is evaluated analytically. No extrapolation, calibration correction, unit
/// conversion, response normalization, or spectral resampling beyond this integral
/// is performed.
///
/// If every source sample carries an independent standard uncertainty, the output
/// uncertainty is propagated using the exact linear weights of this integration.
/// Otherwise output uncertainty is None. Response-curve uncertainty is not included.
pub fn integrate_spectral_response(
    signal: &SpectralSignalCurve,
    response: &SpectralResponseFunction,
) -> Result<SpectralBandObservation, SpectralResponseError> {
    signal.validate()?;
    response.validate()?;

    let signal_min = signal.samples[0].wavelength_nm;
    let signal_max = signal.samples[signal.samples.len() - 1].wavelength_nm;
    let response_min = response.samples[0].wavelength_nm;
    let response_max = response.samples[response.samples.len() - 1].wavelength_nm;
    if signal_min > response_min || signal_max < response_max {
        return Err(SpectralResponseError::InsufficientSpectralCoverage);
    }

    // Every knot from both curves inside the response support is an interval
    // boundary, so each interval lies within one linear segment of both curves.
    let mut grid: Vec<f64> = response
        .samples
        .iter()
        .map(|sample| sample.wavelength_nm)
        .chain(
            signal
                .samples
                .iter()
                .map(|sample| sample.wavelength_nm)
                .filter(|wavelength| *wavelength > response_min && *wavelength < response_max),
        )
        .collect();
    grid.sort_by(f64::total_cmp);
    grid.dedup_by(|a, b| *a == *b);

    if grid.len() < 2 {
        return Err(SpectralResponseError::DegenerateIntegrationInterval);
    }

    // These coefficients make the numerator a linear combination of the original
    // signal samples, which also permits correct independent-error propagation.
    let mut signal_weights = vec![0.0_f64; signal.samples.len()];
    let mut response_integral_nm = 0.0_f64;

    for interval in grid.windows(2) {
        let x0 = interval[0];
        let x1 = interval[1];
        let dx = x1 - x0;
        let midpoint = x0 + dx * 0.5;
        if !dx.is_finite() || dx <= 0.0 || midpoint <= x0 || midpoint >= x1 {
            return Err(SpectralResponseError::DegenerateIntegrationInterval);
        }

        let response_index = segment_index(&response.samples, midpoint, |sample| {
            sample.wavelength_nm
        });
        let response_left = response.samples[response_index];
        let response_right = response.samples[response_index + 1];
        let r0 = interpolate(
            response_left.wavelength_nm,
            response_right.wavelength_nm,
            response_left.relative_response,
            response_right.relative_response,
            x0,
        );
        let r1 = interpolate(
            response_left.wavelength_nm,
            response_right.wavelength_nm,
            response_left.relative_response,
            response_right.relative_response,
            x1,
        );

        let signal_index = segment_index(&signal.samples, midpoint, |sample| {
            sample.wavelength_nm
        });
        let left = signal.samples[signal_index];
        let right = signal.samples[signal_index + 1];
        let signal_span = right.wavelength_nm - left.wavelength_nm;
        let signal_fraction0 = (x0 - left.wavelength_nm) / signal_span;
        let signal_fraction1 = (x1 - left.wavelength_nm) / signal_span;

        response_integral_nm += dx * (r0 + r1) * 0.5;
        // Integrate the response against the signal value at each subinterval
        // endpoint, then distribute those weights back to the original signal
        // knots. Response knots can split an input-signal segment, so the local
        // subinterval endpoints are not necessarily the original sample knots.
        let endpoint_weight0 = dx * (2.0 * r0 + r1) / 6.0;
        let endpoint_weight1 = dx * (r0 + 2.0 * r1) / 6.0;
        signal_weights[signal_index] += endpoint_weight0 * (1.0 - signal_fraction0)
            + endpoint_weight1 * (1.0 - signal_fraction1);
        signal_weights[signal_index + 1] +=
            endpoint_weight0 * signal_fraction0 + endpoint_weight1 * signal_fraction1;
    }

    if !response_integral_nm.is_finite() || response_integral_nm <= 0.0 {
        return Err(SpectralResponseError::ZeroResponse);
    }

    let numerator = signal_weights
        .iter()
        .zip(&signal.samples)
        .map(|(weight, sample)| weight * sample.value)
        .sum::<f64>();
    let value = numerator / response_integral_nm;
    if !numerator.is_finite() || !value.is_finite() {
        return Err(SpectralResponseError::NonFiniteIntegrationResult);
    }

    let standard_uncertainty = if signal
        .samples
        .iter()
        .all(|sample| sample.standard_uncertainty.is_some())
    {
        let variance = signal_weights
            .iter()
            .zip(&signal.samples)
            .map(|(weight, sample)| {
                let normalized_weight = weight / response_integral_nm;
                let uncertainty = sample.standard_uncertainty.unwrap_or(0.0);
                (normalized_weight * uncertainty).powi(2)
            })
            .sum::<f64>();
        if !variance.is_finite() {
            return Err(SpectralResponseError::NonFiniteIntegrationResult);
        }
        Some(variance.sqrt())
    } else {
        None
    };

    if let SpectralSignalQuantity::ReflectanceFraction = signal.quantity
        && !(0.0..=1.0).contains(&value)
    {
        return Err(SpectralResponseError::InvalidSignalValue);
    }

    Ok(SpectralBandObservation {
        quantity: signal.quantity,
        value,
        standard_uncertainty,
        response_integral_nm,
        wavelength_support_nm: [response_min, response_max],
        contributing_sample_count: signal_weights
            .iter()
            .filter(|weight| **weight > 0.0)
            .count(),
    })
}

fn segment_index<T>(
    samples: &[T],
    wavelength_nm: f64,
    wavelength: impl Fn(&T) -> f64,
) -> usize {
    let insertion = samples.partition_point(|sample| wavelength(sample) <= wavelength_nm);
    insertion.saturating_sub(1).min(samples.len() - 2)
}

fn interpolate(
    wavelength0_nm: f64,
    wavelength1_nm: f64,
    value0: f64,
    value1: f64,
    wavelength_nm: f64,
) -> f64 {
    let fraction = (wavelength_nm - wavelength0_nm) / (wavelength1_nm - wavelength0_nm);
    value0 + fraction * (value1 - value0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(points: &[(f64, f64)]) -> SpectralResponseFunction {
        SpectralResponseFunction {
            identifier: "test-channel".to_owned(),
            samples: points
                .iter()
                .map(|(wavelength_nm, relative_response)| SpectralResponseSample {
                    wavelength_nm: *wavelength_nm,
                    relative_response: *relative_response,
                })
                .collect(),
        }
    }

    fn signal(
        quantity: SpectralSignalQuantity,
        points: &[(f64, f64, Option<f64>)],
    ) -> SpectralSignalCurve {
        SpectralSignalCurve {
            quantity,
            samples: points
                .iter()
                .map(
                    |(wavelength_nm, value, standard_uncertainty)| SpectralSignalSample {
                        wavelength_nm: *wavelength_nm,
                        value: *value,
                        standard_uncertainty: *standard_uncertainty,
                    },
                )
                .collect(),
        }
    }

    fn approx_eq(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "expected {left} to be within {tolerance} of {right}"
        );
    }

    #[test]
    fn constant_signal_is_preserved_under_irregular_response_and_sampling() {
        let spectrum = signal(
            SpectralSignalQuantity::SpectralRadianceWPerSrM2Um,
            &[
                (390.0, 42.0, Some(0.1)),
                (450.0, 42.0, Some(0.1)),
                (525.0, 42.0, Some(0.1)),
                (610.0, 42.0, Some(0.1)),
            ],
        );
        let response = response(&[
            (400.0, 0.0),
            (440.0, 0.4),
            (500.0, 1.0),
            (560.0, 0.3),
            (600.0, 0.0),
        ]);

        let observation = integrate_spectral_response(&spectrum, &response).unwrap();
        approx_eq(observation.value, 42.0, 1e-12);
        assert_eq!(observation.quantity, spectrum.quantity);
        assert_eq!(observation.wavelength_support_nm, [400.0, 600.0]);
        assert!(observation.standard_uncertainty.is_some());
    }

    #[test]
    fn response_weighted_integral_matches_an_analytic_ramp_case() {
        // For t=(lambda-400)/200, signal=t and response=t. Their weighted mean
        // is integral(t^2)/integral(t)=((1/3))/(1/2)=2/3.
        let spectrum = signal(
            SpectralSignalQuantity::ReflectanceFraction,
            &[
                (400.0, 0.0, None),
                (500.0, 0.5, None),
                (600.0, 1.0, None),
            ],
        );
        let response = response(&[(400.0, 0.0), (600.0, 1.0)]);

        let observation = integrate_spectral_response(&spectrum, &response).unwrap();
        approx_eq(observation.value, 2.0 / 3.0, 1e-12);
        assert_eq!(observation.standard_uncertainty, None);
        approx_eq(observation.response_integral_nm, 100.0, 1e-12);
    }

    #[test]
    fn response_knots_inside_one_signal_segment_preserve_linear_spectra() {
        // The spectrum is linear from 0 at 390 nm to 1 at 610 nm. The asymmetric
        // triangular response has a weighted wavelength centroid at 483 1/3 nm,
        // so the expected response-weighted value is (483 1/3 - 390) / 220.
        let spectrum = signal(
            SpectralSignalQuantity::ReflectanceFraction,
            &[(390.0, 0.0, None), (610.0, 1.0, None)],
        );
        let response = response(&[(400.0, 0.0), (450.0, 1.0), (600.0, 0.0)]);

        let observation = integrate_spectral_response(&spectrum, &response).unwrap();
        approx_eq(observation.value, 0.42424242424242425, 1e-12);
    }

    #[test]
    fn independent_sample_uncertainty_is_propagated_with_integration_weights() {
        let spectrum = signal(
            SpectralSignalQuantity::SpectralRadianceWPerSrM2Um,
            &[
                (400.0, 1.0, Some(0.1)),
                (500.0, 2.0, Some(0.2)),
                (600.0, 3.0, Some(0.3)),
            ],
        );
        let response = response(&[(400.0, 0.0), (600.0, 1.0)]);

        let observation = integrate_spectral_response(&spectrum, &response).unwrap();
        let expected = ((0.5_f64 / 6.0 * 0.1).powi(2)
            + (0.5_f64 * 0.2).powi(2)
            + (5.0_f64 / 12.0 * 0.3).powi(2))
        .sqrt();
        approx_eq(
            observation.standard_uncertainty.unwrap(),
            expected,
            1e-12,
        );
    }

    #[test]
    fn missing_source_uncertainty_does_not_emit_a_partial_uncertainty() {
        let spectrum = signal(
            SpectralSignalQuantity::SpectralRadianceWPerSrM2Um,
            &[
                (400.0, 1.0, Some(0.1)),
                (500.0, 2.0, None),
                (600.0, 3.0, Some(0.3)),
            ],
        );
        let response = response(&[(400.0, 0.0), (600.0, 1.0)]);
        let observation = integrate_spectral_response(&spectrum, &response).unwrap();
        assert_eq!(observation.standard_uncertainty, None);
    }

    #[test]
    fn response_support_outside_signal_domain_fails_closed() {
        let spectrum = signal(
            SpectralSignalQuantity::ReflectanceFraction,
            &[(410.0, 0.2, None), (500.0, 0.5, None), (590.0, 0.8, None)],
        );
        let response = response(&[(400.0, 0.0), (600.0, 1.0)]);
        assert_eq!(
            integrate_spectral_response(&spectrum, &response),
            Err(SpectralResponseError::InsufficientSpectralCoverage)
        );
    }

    #[test]
    fn invalid_wavelength_response_and_signal_values_are_rejected() {
        let invalid_response = response(&[(400.0, 0.1), (400.0, 0.8)]);
        assert_eq!(
            invalid_response.validate(),
            Err(SpectralResponseError::NonIncreasingWavelength)
        );

        let zero_response = response(&[(400.0, 0.0), (500.0, 0.0)]);
        assert_eq!(
            zero_response.validate(),
            Err(SpectralResponseError::ZeroResponse)
        );

        let invalid_signal = signal(
            SpectralSignalQuantity::ReflectanceFraction,
            &[(400.0, 0.2, None), (500.0, 1.1, None)],
        );
        assert_eq!(
            invalid_signal.validate(),
            Err(SpectralResponseError::InvalidSignalValue)
        );
    }

    #[test]
    fn malformed_uncertainty_and_missing_identity_are_rejected() {
        let invalid_signal = signal(
            SpectralSignalQuantity::SpectralRadianceWPerSrM2Um,
            &[(400.0, 1.0, Some(-0.1)), (500.0, 2.0, Some(0.2))],
        );
        assert_eq!(
            invalid_signal.validate(),
            Err(SpectralResponseError::InvalidUncertainty)
        );

        let mut missing_id = response(&[(400.0, 0.0), (500.0, 1.0)]);
        missing_id.identifier.clear();
        assert_eq!(
            missing_id.validate(),
            Err(SpectralResponseError::MissingResponseIdentifier)
        );
    }

    #[test]
    fn reweighting_is_deterministic_and_does_not_mutate_inputs() {
        let spectrum = signal(
            SpectralSignalQuantity::ReflectanceFraction,
            &[(390.0, 0.2, None), (450.0, 0.4, None), (550.0, 0.6, None)],
        );
        let response = response(&[(400.0, 0.1), (500.0, 1.0), (540.0, 0.2)]);
        let spectrum_before = spectrum.clone();
        let response_before = response.clone();

        let first = integrate_spectral_response(&spectrum, &response).unwrap();
        let second = integrate_spectral_response(&spectrum, &response).unwrap();
        assert_eq!(first, second);
        assert_eq!(spectrum, spectrum_before);
        assert_eq!(response, response_before);
        assert!(first.contributing_sample_count > 0);
    }
}
