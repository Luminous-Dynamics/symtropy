//! Analytic source-spectrum controls for black-hole ringdown research.
//!
//! This crate evaluates the exact Fourier transform of a localized
//! Gaussian-carrier source and composes it with a caller-supplied modal gain.
//! It does not compute black-hole QNMs or physical excitation factors.

use core::fmt;

use num_complex::Complex64;

/// A real localized Gaussian carrier.
///
/// The Fourier convention is f_hat(k) = integral f(x) exp(-i k x) dx.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaussianCarrier {
    amplitude: f64,
    center: f64,
    width: f64,
    carrier_wavenumber: f64,
}

/// Typed errors for malformed profiles and non-finite calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSpectrumError {
    NonFiniteParameter,
    NonPositiveWidth,
    NonFiniteWavenumber,
    NonFiniteModeGain,
    NonFiniteResult,
}

impl fmt::Display for SourceSpectrumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter => write!(f, "source parameters must be finite"),
            Self::NonPositiveWidth => write!(f, "Gaussian width must be strictly positive"),
            Self::NonFiniteWavenumber => write!(f, "wavenumber must be finite"),
            Self::NonFiniteModeGain => write!(f, "intrinsic mode gain must be finite"),
            Self::NonFiniteResult => write!(
                f,
                "source-spectrum calculation overflowed or became non-finite"
            ),
        }
    }
}

impl std::error::Error for SourceSpectrumError {}

fn finite_complex(value: Complex64) -> bool {
    value.re.is_finite() && value.im.is_finite()
}

/// The source transform and the caller-supplied intrinsic mode gain.
///
/// The coefficient is their product. Keeping both factors visible prevents a
/// source-only change from being mislabeled as a change in the background
/// spectrum or source-independent excitation factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModeSourceOverlap {
    /// Declared characteristic (possibly complex) mode wavenumber.
    pub characteristic_wavenumber: Complex64,
    /// Exact Fourier transform of the source profile at that wavenumber.
    pub source_transform: Complex64,
    /// Caller-supplied, source-independent modal gain.
    pub intrinsic_gain: Complex64,
    /// Product of source transform and intrinsic gain.
    pub coefficient: Complex64,
}

impl GaussianCarrier {
    /// Construct a Gaussian-carrier source.
    ///
    /// width is sigma in the spatial Gaussian; carrier_wavenumber may
    /// have either sign because the real profile is cosine-modulated.
    pub fn new(
        amplitude: f64,
        center: f64,
        width: f64,
        carrier_wavenumber: f64,
    ) -> Result<Self, SourceSpectrumError> {
        if !amplitude.is_finite()
            || !center.is_finite()
            || !width.is_finite()
            || !carrier_wavenumber.is_finite()
        {
            return Err(SourceSpectrumError::NonFiniteParameter);
        }
        if width <= 0.0 {
            return Err(SourceSpectrumError::NonPositiveWidth);
        }

        Ok(Self {
            amplitude,
            center,
            width,
            carrier_wavenumber,
        })
    }

    pub const fn amplitude(self) -> f64 {
        self.amplitude
    }

    pub const fn center(self) -> f64 {
        self.center
    }

    pub const fn width(self) -> f64 {
        self.width
    }

    pub const fn carrier_wavenumber(self) -> f64 {
        self.carrier_wavenumber
    }

    /// Evaluate the real spatial source profile.
    pub fn sample(self, x: f64) -> Result<f64, SourceSpectrumError> {
        if !x.is_finite() {
            return Err(SourceSpectrumError::NonFiniteParameter);
        }
        let displacement = x - self.center;
        let envelope = (-0.5 * (displacement / self.width).powi(2)).exp();
        let value = self.amplitude * envelope * (self.carrier_wavenumber * displacement).cos();
        if value.is_finite() {
            Ok(value)
        } else {
            Err(SourceSpectrumError::NonFiniteResult)
        }
    }

    /// Evaluate the exact Fourier transform for real or complex wavenumber.
    ///
    /// This is the analytic continuation of the real-k Fourier transform.
    /// It is a source-profile transform, not automatically the physical
    /// QNM overlap: radial weights, field conventions and boundary
    /// normalizations belong to the caller's physical model.
    pub fn fourier_transform(
        self,
        wavenumber: Complex64,
    ) -> Result<Complex64, SourceSpectrumError> {
        if !finite_complex(wavenumber) {
            return Err(SourceSpectrumError::NonFiniteWavenumber);
        }

        let sigma_squared = self.width * self.width;
        let carrier = Complex64::new(self.carrier_wavenumber, 0.0);
        let minus = wavenumber - carrier;
        let plus = wavenumber + carrier;
        let minus_exponent = minus * minus * (-0.5 * sigma_squared);
        let plus_exponent = plus * plus * (-0.5 * sigma_squared);
        let translation_phase = Complex64::new(0.0, -self.center) * wavenumber;

        let prefactor = self.amplitude * (2.0 * core::f64::consts::PI).sqrt() * self.width * 0.5;
        let result = Complex64::new(prefactor, 0.0)
            * translation_phase.exp()
            * (minus_exponent.exp() + plus_exponent.exp());

        if finite_complex(result) {
            Ok(result)
        } else {
            Err(SourceSpectrumError::NonFiniteResult)
        }
    }

    /// Exact integral of the squared real source profile.
    ///
    /// This is a source-profile L2 norm squared under dx. It is not
    /// asserted to be a conserved physical perturbation energy.
    pub fn l2_norm_squared(self) -> Result<f64, SourceSpectrumError> {
        let sigma = self.width;
        let carrier = self.carrier_wavenumber;
        let value = self.amplitude.powi(2)
            * core::f64::consts::PI.sqrt()
            * sigma
            * 0.5
            * (1.0 + (-sigma * sigma * carrier * carrier).exp());
        if value.is_finite() {
            Ok(value)
        } else {
            Err(SourceSpectrumError::NonFiniteResult)
        }
    }
}

/// Compose the source spectrum with a separately supplied intrinsic mode gain.
///
/// The mode location and intrinsic gain are inputs, not calculated here.
pub fn evaluate_mode_overlap(
    source: GaussianCarrier,
    characteristic_wavenumber: Complex64,
    intrinsic_gain: Complex64,
) -> Result<ModeSourceOverlap, SourceSpectrumError> {
    if !finite_complex(intrinsic_gain) {
        return Err(SourceSpectrumError::NonFiniteModeGain);
    }

    let source_transform = source.fourier_transform(characteristic_wavenumber)?;
    let coefficient = source_transform * intrinsic_gain;
    if !finite_complex(coefficient) {
        return Err(SourceSpectrumError::NonFiniteResult);
    }

    Ok(ModeSourceOverlap {
        characteristic_wavenumber,
        source_transform,
        intrinsic_gain,
        coefficient,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(center: f64, width: f64, carrier: f64) -> GaussianCarrier {
        GaussianCarrier::new(1.3, center, width, carrier).unwrap()
    }

    fn simpson_fourier(source: GaussianCarrier, k: f64) -> Complex64 {
        // Independent real-space composite Simpson quadrature over +/- 12 sigma.
        let intervals = 40_000_usize;
        let lower = source.center() - 12.0 * source.width();
        let upper = source.center() + 12.0 * source.width();
        let step = (upper - lower) / intervals as f64;
        let mut sum = Complex64::new(0.0, 0.0);

        for index in 0..=intervals {
            let x = lower + index as f64 * step;
            let value = source.sample(x).unwrap();
            let phase = Complex64::from_polar(1.0, -k * x);
            let weight = if index == 0 || index == intervals {
                1.0
            } else if index % 2 == 0 {
                2.0
            } else {
                4.0
            };
            sum += phase * (weight * value);
        }

        sum * (step / 3.0)
    }

    #[test]
    fn analytic_transform_matches_independent_real_space_quadrature() {
        let profile = source(0.37, 0.8, 2.2);
        let k = 1.35;
        let analytic = profile.fourier_transform(Complex64::new(k, 0.0)).unwrap();
        let numeric = simpson_fourier(profile, k);
        assert!((analytic - numeric).norm() < 2.0e-11);
    }

    #[test]
    fn translation_changes_fourier_phase_but_not_real_k_magnitude() {
        let centered = source(0.0, 0.75, 1.8);
        let translated = source(1.25, 0.75, 1.8);
        let k = 2.1;
        let a = centered.fourier_transform(Complex64::new(k, 0.0)).unwrap();
        let b = translated
            .fourier_transform(Complex64::new(k, 0.0))
            .unwrap();
        let expected_phase = Complex64::from_polar(1.0, -k * 1.25);
        assert!((b - a * expected_phase).norm() < 1.0e-12);
        assert!((a.norm() - b.norm()).abs() < 1.0e-12);
    }

    #[test]
    fn real_profile_has_conjugate_symmetric_fourier_transform() {
        let profile = source(-0.4, 0.9, 2.4);
        let positive = profile.fourier_transform(Complex64::new(1.7, 0.0)).unwrap();
        let negative = profile
            .fourier_transform(Complex64::new(-1.7, 0.0))
            .unwrap();
        assert!((negative - positive.conj()).norm() < 1.0e-12);
    }

    #[test]
    fn complex_wavenumber_uses_the_analytic_entire_continuation() {
        let profile = source(0.2, 0.6, 1.4);
        let k = Complex64::new(1.9, -0.12);
        let reflected = profile.fourier_transform(-k.conj()).unwrap();
        let original = profile.fourier_transform(k).unwrap();
        assert!((reflected - original.conj()).norm() < 1.0e-11);
    }

    #[test]
    fn carrier_sweep_selects_the_mode_inside_the_source_band() {
        let mode_k = 2.25;
        let width = 2.0;
        let mut maximum_carrier = f64::NAN;
        let mut maximum_overlap = -1.0_f64;

        for step in 0..=200 {
            let carrier = step as f64 * 0.025;
            let profile = source(0.0, width, carrier);
            let overlap = profile
                .fourier_transform(Complex64::new(mode_k, 0.0))
                .unwrap()
                .norm();
            if overlap > maximum_overlap {
                maximum_overlap = overlap;
                maximum_carrier = carrier;
            }
        }

        assert!((maximum_carrier - mode_k).abs() <= 0.025);
    }

    #[test]
    fn source_only_intervention_changes_modal_coefficients_not_mode_locations() {
        let modes = [Complex64::new(1.5, -0.08), Complex64::new(3.5, -0.12)];
        let gains = [Complex64::new(0.7, 0.2), Complex64::new(1.1, -0.1)];
        let source_a = source(0.0, 2.0, 1.5);
        let source_b = source(0.0, 2.0, 3.5);

        let a: Vec<_> = modes
            .iter()
            .zip(gains.iter())
            .map(|(mode, gain)| evaluate_mode_overlap(source_a, *mode, *gain).unwrap())
            .collect();
        let b: Vec<_> = modes
            .iter()
            .zip(gains.iter())
            .map(|(mode, gain)| evaluate_mode_overlap(source_b, *mode, *gain).unwrap())
            .collect();

        for index in 0..modes.len() {
            assert_eq!(
                a[index].characteristic_wavenumber,
                b[index].characteristic_wavenumber
            );
            assert_eq!(a[index].intrinsic_gain, b[index].intrinsic_gain);
            assert!((a[index].coefficient - b[index].coefficient).norm() > 1.0e-6);
        }

        assert!(a[0].coefficient.norm() > a[1].coefficient.norm());
        assert!(b[1].coefficient.norm() > b[0].coefficient.norm());
    }

    #[test]
    fn fixed_peak_and_fixed_l2_interventions_are_not_conflated() {
        let narrow = GaussianCarrier::new(1.0, 0.0, 0.5, 1.8).unwrap();
        let broad = GaussianCarrier::new(1.0, 0.0, 2.0, 1.8).unwrap();
        let narrow_l2 = narrow.l2_norm_squared().unwrap();
        let broad_l2 = broad.l2_norm_squared().unwrap();
        assert!((narrow.amplitude() - broad.amplitude()).abs() < f64::EPSILON);
        assert!((broad_l2 / narrow_l2) > 2.0);

        let normalized_narrow =
            GaussianCarrier::new(1.0 / narrow_l2.sqrt(), 0.0, 0.5, 1.8).unwrap();
        let normalized_broad = GaussianCarrier::new(1.0 / broad_l2.sqrt(), 0.0, 2.0, 1.8).unwrap();
        assert!((normalized_narrow.l2_norm_squared().unwrap() - 1.0).abs() < 1.0e-12);
        assert!((normalized_broad.l2_norm_squared().unwrap() - 1.0).abs() < 1.0e-12);
        assert!((normalized_narrow.amplitude() - normalized_broad.amplitude()).abs() > 0.1);
    }

    #[test]
    fn source_transform_and_intrinsic_mode_gain_remain_separate() {
        let profile = source(0.1, 0.7, 1.9);
        let k = Complex64::new(2.0, -0.05);
        let gain = Complex64::new(0.8, 0.3);
        let report = evaluate_mode_overlap(profile, k, gain).unwrap();
        assert_eq!(report.characteristic_wavenumber, k);
        assert_eq!(report.intrinsic_gain, gain);
        assert!((report.coefficient - report.source_transform * gain).norm() < 1.0e-14);
    }

    #[test]
    fn invalid_profiles_and_non_finite_inputs_fail_closed() {
        assert_eq!(
            GaussianCarrier::new(1.0, 0.0, 0.0, 1.0),
            Err(SourceSpectrumError::NonPositiveWidth)
        );
        assert_eq!(
            GaussianCarrier::new(1.0, 0.0, f64::NAN, 1.0),
            Err(SourceSpectrumError::NonFiniteParameter)
        );
        let profile = source(0.0, 1.0, 1.0);
        assert_eq!(
            profile.fourier_transform(Complex64::new(f64::INFINITY, 0.0)),
            Err(SourceSpectrumError::NonFiniteWavenumber)
        );
        assert_eq!(
            evaluate_mode_overlap(
                profile,
                Complex64::new(1.0, 0.0),
                Complex64::new(f64::NAN, 0.0),
            ),
            Err(SourceSpectrumError::NonFiniteModeGain)
        );
    }
}
