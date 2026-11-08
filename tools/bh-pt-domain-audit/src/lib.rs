//! Pöschl–Teller analytic pole and causal-domain controls.
//!
//! V(x) = V0 sech²(x / b), V0 > 0, b > 0, λ = V0 b², ν² = 1/4 - λ.
//! Time dependence is exp(-i ω Δt), giving
//! ω_n^± = [-i(n + 1/2) ± i ν] / b.
//!
//! This crate pins analytic poles, dimensional scaling, potential profile and
//! causal-region selection only. It does not evaluate special functions,
//! Green functions, residues, contour integrals or time-domain evolution.

use core::fmt;
use num_complex::Complex64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QnmBranch {
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalRegion {
    RegionIII,
    RegionII,
    RegionI,
    DirectLightcone,
    ReflectedLightcone,
    CoincidentLightcones,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoschlTellerModel {
    width: f64,
    peak_strength: f64,
    lambda: f64,
    nu: Complex64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CausalDomain {
    pub delta_t: f64,
    pub direct_delay: f64,
    pub reflected_delay: f64,
    pub region: CausalRegion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelError {
    NonFiniteParameter,
    NonPositiveWidth,
    NonPositivePeakStrength,
    ParameterOverflow,
    NonFiniteCoordinate,
    NonFiniteTime,
    InvalidBoundaryTolerance,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter => write!(f, "model parameters must be finite"),
            Self::NonPositiveWidth => write!(f, "potential width b must be strictly positive"),
            Self::NonPositivePeakStrength => {
                write!(f, "barrier peak strength V0 must be strictly positive")
            }
            Self::ParameterOverflow => write!(f, "dimensionless barrier strength V0 b² overflowed"),
            Self::NonFiniteCoordinate => {
                write!(f, "source and observer coordinates must be finite")
            }
            Self::NonFiniteTime => write!(f, "time separation must be finite"),
            Self::InvalidBoundaryTolerance => {
                write!(f, "boundary tolerance must be finite and non-negative")
            }
        }
    }
}
impl std::error::Error for ModelError {}

impl PoschlTellerModel {
    /// Construct the barrier from dimensional width b and peak V0.
    /// The principal square root is used for nu² = 1/4 - V0 b².
    pub fn new(width: f64, peak_strength: f64) -> Result<Self, ModelError> {
        if !width.is_finite() || !peak_strength.is_finite() {
            return Err(ModelError::NonFiniteParameter);
        }
        if width <= 0.0 {
            return Err(ModelError::NonPositiveWidth);
        }
        if peak_strength <= 0.0 {
            return Err(ModelError::NonPositivePeakStrength);
        }
        let lambda = peak_strength * width * width;
        if !lambda.is_finite() {
            return Err(ModelError::ParameterOverflow);
        }
        let nu = Complex64::new(0.25 - lambda, 0.0).sqrt();
        Ok(Self {
            width,
            peak_strength,
            lambda,
            nu,
        })
    }

    pub const fn width(self) -> f64 {
        self.width
    }
    pub const fn peak_strength(self) -> f64 {
        self.peak_strength
    }
    pub const fn dimensionless_strength(self) -> f64 {
        self.lambda
    }
    pub const fn nu(self) -> Complex64 {
        self.nu
    }

    /// Evaluate V(x) = V0 sech²(x / b) without large-argument cosh overflow.
    pub fn potential(self, x: f64) -> Result<f64, ModelError> {
        if !x.is_finite() {
            return Err(ModelError::NonFiniteCoordinate);
        }
        let y = (x / self.width).abs();
        let sech_squared = if y > 20.0 {
            let e = (-y).exp();
            4.0 * e * e / (1.0 + e * e).powi(2)
        } else {
            1.0 / y.cosh().powi(2)
        };
        let value = self.peak_strength * sech_squared;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(ModelError::ParameterOverflow)
        }
    }

    /// Dimensional QNM frequency:
    /// omega_n^± = [-i(n+1/2) ± sqrt(lambda-1/4)] / b.
    ///
    /// The principal square root fixes the plus branch to positive real
    /// frequency above threshold and to the less-damped branch below it.
    /// A branch coincidence at lambda=1/4 is not by itself an EP diagnosis.
    pub fn qnm_frequency(self, n: u32, branch: QnmBranch) -> Complex64 {
        let base = Complex64::new(0.0, -(f64::from(n) + 0.5));
        let split = Complex64::new(self.lambda - 0.25, 0.0).sqrt();
        let signed_split = match branch {
            QnmBranch::Plus => split,
            QnmBranch::Minus => -split,
        };
        (base + signed_split) / self.width
    }

    /// Dimensional Matsubara pole: omega_n = i n / b.
    /// These poles occur in the split terms and cancel in the full G-tilde.
    pub fn matsubara_frequency(self, n: i32) -> Complex64 {
        Complex64::new(0.0, f64::from(n) / self.width)
    }

    /// Euclidean period for the Pöschl–Teller Rindler limit: beta = 2 pi b.
    pub fn euclidean_period(self) -> f64 {
        2.0 * core::f64::consts::PI * self.width
    }

    /// Classify causal region using direct/reflected null delays.
    /// Coordinates and delta_t are dimensional; c=1. The absolute tolerance
    /// has the same units. Region II may be empty for opposite-side points.
    pub fn classify_causal_domain(
        self,
        delta_t: f64,
        x_source: f64,
        x_observer: f64,
        boundary_tolerance: f64,
    ) -> Result<CausalDomain, ModelError> {
        if !delta_t.is_finite() {
            return Err(ModelError::NonFiniteTime);
        }
        if !x_source.is_finite() || !x_observer.is_finite() {
            return Err(ModelError::NonFiniteCoordinate);
        }
        if !boundary_tolerance.is_finite() || boundary_tolerance < 0.0 {
            return Err(ModelError::InvalidBoundaryTolerance);
        }
        let direct_delay = (x_observer - x_source).abs();
        let reflected_delay = (x_observer + x_source).abs();
        let on_direct = (delta_t - direct_delay).abs() <= boundary_tolerance;
        let on_reflected = (delta_t - reflected_delay).abs() <= boundary_tolerance;
        let region = match (on_direct, on_reflected) {
            (true, true) => CausalRegion::CoincidentLightcones,
            (true, false) => CausalRegion::DirectLightcone,
            (false, true) => CausalRegion::ReflectedLightcone,
            (false, false) if delta_t < direct_delay => CausalRegion::RegionIII,
            (false, false) if delta_t < reflected_delay => CausalRegion::RegionII,
            (false, false) => CausalRegion::RegionI,
        };
        Ok(CausalDomain {
            delta_t,
            direct_delay,
            reflected_delay,
            region,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(width: f64, lambda: f64) -> PoschlTellerModel {
        PoschlTellerModel::new(width, lambda / (width * width)).unwrap()
    }

    #[test]
    fn potential_peak_symmetry_and_dimensionless_scaling() {
        let a = model(1.0, 0.75);
        let b = model(2.0, 0.75);
        assert!((a.potential(0.0).unwrap() - 0.75).abs() < 1e-14);
        assert!((b.potential(0.0).unwrap() - 0.1875).abs() < 1e-14);
        assert!((a.potential(1.0).unwrap() - a.potential(-1.0).unwrap()).abs() < 1e-14);
        assert!((a.dimensionless_strength() - b.dimensionless_strength()).abs() < 1e-14);
    }

    #[test]
    fn qnm_branches_have_opposite_real_parts_above_threshold() {
        let m = model(1.0, 0.75);
        let plus = m.qnm_frequency(0, QnmBranch::Plus);
        let minus = m.qnm_frequency(0, QnmBranch::Minus);
        assert!((plus.re + minus.re).abs() < 1e-14);
        assert!((plus.im + 0.5).abs() < 1e-14);
        assert!((minus.im + 0.5).abs() < 1e-14);
        assert!((plus.re - 0.5_f64.sqrt()).abs() < 1e-14);
        assert!((minus.re + 0.5_f64.sqrt()).abs() < 1e-14);
    }

    #[test]
    fn subthreshold_plus_branch_is_the_less_damped_branch() {
        let m = model(1.0, 0.125);
        let plus = m.qnm_frequency(0, QnmBranch::Plus);
        let minus = m.qnm_frequency(0, QnmBranch::Minus);
        assert!(plus.im < 0.0);
        assert!(minus.im < 0.0);
        assert!(plus.im > minus.im);
    }

    #[test]
    fn qnm_frequency_scales_inversely_with_width() {
        let a = model(1.0, 0.75).qnm_frequency(2, QnmBranch::Plus);
        let b = model(2.0, 0.75).qnm_frequency(2, QnmBranch::Plus);
        assert!((b - a / 2.0).norm() < 1e-14);
    }

    #[test]
    fn subthreshold_modes_are_purely_damped() {
        let m = model(1.0, 0.125);
        for n in 0..8 {
            for branch in [QnmBranch::Plus, QnmBranch::Minus] {
                let omega = m.qnm_frequency(n, branch);
                assert!(omega.re.abs() < 1e-14);
                assert!(omega.im < 0.0);
            }
        }
    }

    #[test]
    fn threshold_branch_coalescence_is_not_an_ep_classification() {
        let m = model(1.0, 0.25);
        let a = m.qnm_frequency(3, QnmBranch::Plus);
        let b = m.qnm_frequency(3, QnmBranch::Minus);
        assert!((a - b).norm() < 1e-14);
        assert!((a - Complex64::new(0.0, -3.5)).norm() < 1e-14);
    }

    #[test]
    fn matsubara_spacing_and_euclidean_period_scale_with_width() {
        let m = model(2.5, 0.75);
        let one = m.matsubara_frequency(1);
        assert!((one.im - 0.4).abs() < 1e-14);
        assert!((m.matsubara_frequency(3) - 3.0 * one).norm() < 1e-14);
        assert!((m.euclidean_period() - 5.0 * core::f64::consts::PI).abs() < 1e-14);
    }

    #[test]
    fn same_side_points_select_all_three_regions() {
        let m = model(1.0, 0.75);
        assert_eq!(
            m.classify_causal_domain(0.5, 3.0, 4.0, 0.0).unwrap().region,
            CausalRegion::RegionIII
        );
        assert_eq!(
            m.classify_causal_domain(2.0, 3.0, 4.0, 0.0).unwrap().region,
            CausalRegion::RegionII
        );
        assert_eq!(
            m.classify_causal_domain(8.0, 3.0, 4.0, 0.0).unwrap().region,
            CausalRegion::RegionI
        );
    }

    #[test]
    fn lightcone_boundaries_are_explicit() {
        let m = model(1.0, 0.75);
        assert_eq!(
            m.classify_causal_domain(1.0, 3.0, 4.0, 0.0).unwrap().region,
            CausalRegion::DirectLightcone
        );
        assert_eq!(
            m.classify_causal_domain(7.0, 3.0, 4.0, 0.0).unwrap().region,
            CausalRegion::ReflectedLightcone
        );
        assert_eq!(
            m.classify_causal_domain(0.0, 0.0, 0.0, 0.0).unwrap().region,
            CausalRegion::CoincidentLightcones
        );
    }

    #[test]
    fn opposite_side_points_may_have_no_region_ii() {
        let m = model(1.0, 0.75);
        let d = m.classify_causal_domain(4.0, -3.0, 4.0, 0.0).unwrap();
        assert_eq!(d.direct_delay, 7.0);
        assert_eq!(d.reflected_delay, 1.0);
        assert_eq!(d.region, CausalRegion::RegionIII);
        assert_eq!(
            m.classify_causal_domain(8.0, -3.0, 4.0, 0.0)
                .unwrap()
                .region,
            CausalRegion::RegionI
        );
    }

    #[test]
    fn boundary_tolerance_is_explicit() {
        let m = model(1.0, 0.75);
        assert_eq!(
            m.classify_causal_domain(1.0001, 3.0, 4.0, 0.001)
                .unwrap()
                .region,
            CausalRegion::DirectLightcone
        );
        assert_eq!(
            m.classify_causal_domain(1.0001, 3.0, 4.0, 0.00001)
                .unwrap()
                .region,
            CausalRegion::RegionII
        );
        assert_eq!(
            m.classify_causal_domain(1.0, 3.0, 4.0, -1.0),
            Err(ModelError::InvalidBoundaryTolerance)
        );
    }

    #[test]
    fn invalid_model_inputs_fail_closed() {
        assert_eq!(
            PoschlTellerModel::new(0.0, 1.0),
            Err(ModelError::NonPositiveWidth)
        );
        assert_eq!(
            PoschlTellerModel::new(1.0, 0.0),
            Err(ModelError::NonPositivePeakStrength)
        );
        assert_eq!(
            PoschlTellerModel::new(f64::NAN, 1.0),
            Err(ModelError::NonFiniteParameter)
        );
    }
}
