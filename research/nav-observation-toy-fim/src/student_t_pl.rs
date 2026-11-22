//! Heavy-tailed Student-t protection-level baseline used by the 2026
//! adaptive-PID PL paper's Section 5.1 comparator.
//!
//! Formula is transcribed as printed in the paper. The 2D Student-t TIR
//! multiplier K_TIR(dof) is supplied by the caller; this module deliberately
//! does not substitute a univariate normal quantile or guess the CDF convention.

use std::fmt;

const PSD_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Covariance2 {
    pub xx: f64,
    pub xy: f64,
    pub yy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EigenFrame2 {
    pub lambda_max: f64,
    pub lambda_min: f64,
    /// Unit eigenvector associated with lambda_max.
    pub maximum_axis: [f64; 2],
    /// Unit eigenvector associated with lambda_min.
    pub minimum_axis: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StudentTProtectionLevels {
    pub degrees_of_freedom: f64,
    pub k_tir_2d: f64,
    pub maximum_eigenvalue: f64,
    pub overall_pl_m: f64,
    pub longitudinal_pl_m: f64,
    pub lateral_pl_m: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StudentTPlError {
    NonFiniteCovariance,
    CovarianceNotPositiveSemidefinite,
    InvalidDegreesOfFreedom,
    InvalidTirMultiplier,
    InvalidTargetIntegrityRisk,
    NonFiniteOutput,
}

impl fmt::Display for StudentTPlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteCovariance => write!(f, "covariance matrix must be finite"),
            Self::CovarianceNotPositiveSemidefinite => write!(f, "covariance matrix must be positive semidefinite"),
            Self::InvalidDegreesOfFreedom => write!(f, "Student-t degrees of freedom must be finite and greater than 2"),
            Self::InvalidTirMultiplier => write!(f, "2D TIR multiplier must be finite and positive"),
            Self::InvalidTargetIntegrityRisk => write!(f, "target integrity risk must be in (0,1) as a 2D radial tail probability"),
            Self::NonFiniteOutput => write!(f, "Student-t protection-level result is non-finite"),
        }
    }
}
impl std::error::Error for StudentTPlError {}

impl Covariance2 {
    pub fn validate(self) -> Result<(), StudentTPlError> {
        if !self.xx.is_finite() || !self.xy.is_finite() || !self.yy.is_finite() {
            return Err(StudentTPlError::NonFiniteCovariance);
        }
        let scale = self.xx.abs().max(self.yy.abs()).max(1.0);
        if self.xx < -PSD_TOLERANCE * scale || self.yy < -PSD_TOLERANCE * scale {
            return Err(StudentTPlError::CovarianceNotPositiveSemidefinite);
        }
        let determinant = self.xx.mul_add(self.yy, -(self.xy * self.xy));
        if !determinant.is_finite() || determinant < -PSD_TOLERANCE * scale * scale {
            return Err(StudentTPlError::CovarianceNotPositiveSemidefinite);
        }
        Ok(())
    }

    pub fn eigenframe(self) -> Result<EigenFrame2, StudentTPlError> {
        self.validate()?;
        let trace = self.xx + self.yy;
        let discriminant = (self.xx - self.yy).hypot(2.0 * self.xy);
        let lambda_max = (0.5 * (trace + discriminant)).max(0.0);
        let determinant = self.xx.mul_add(self.yy, -(self.xy * self.xy));
        let lambda_min = if lambda_max > 0.0 {
            (determinant / lambda_max).max(0.0)
        } else {
            0.0
        };

        // Stable eigenvector for the larger eigenvalue.
        let (mut vx, mut vy) = if self.xy.abs() > f64::EPSILON * lambda_max.max(1.0) {
            (self.xy, lambda_max - self.xx)
        } else if self.xx >= self.yy {
            (1.0, 0.0)
        } else {
            (0.0, 1.0)
        };
        let norm = vx.hypot(vy);
        if norm <= 0.0 || !norm.is_finite() {
            return Err(StudentTPlError::NonFiniteOutput);
        }
        vx /= norm;
        vy /= norm;
        let frame = EigenFrame2 {
            lambda_max,
            lambda_min,
            maximum_axis: [vx, vy],
            minimum_axis: [-vy, vx],
        };
        Ok(frame)
    }
}

/// Closed-form radial TIR multiplier for a 2D multivariate Student-t whose
/// supplied matrix is its covariance (not its scale matrix).
///
/// For a 2D Student-t with covariance P and nu > 2, the normalized radial
/// tail is P(r^2 > (nu-2) K^2) = (1 + K^2)^(-nu/2). Solving this for a
/// requested tail probability TIR gives K = sqrt(TIR^(-2/nu) - 1).
///
/// This assumes that the TIR is the total 2D radial tail probability. It is
/// not a one-dimensional per-axis Gaussian multiplier.
pub fn student_t_k_tir_2d(
    degrees_of_freedom: f64,
    target_integrity_risk: f64,
) -> Result<f64, StudentTPlError> {
    if !degrees_of_freedom.is_finite() || degrees_of_freedom <= 2.0 {
        return Err(StudentTPlError::InvalidDegreesOfFreedom);
    }
    if !target_integrity_risk.is_finite()
        || target_integrity_risk <= 0.0
        || target_integrity_risk >= 1.0
    {
        return Err(StudentTPlError::InvalidTargetIntegrityRisk);
    }
    let multiplier = (target_integrity_risk.powf(-2.0 / degrees_of_freedom) - 1.0).sqrt();
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return Err(StudentTPlError::NonFiniteOutput);
    }
    Ok(multiplier)
}

/// Convenience function that computes K_TIR from the explicit 2D radial-tail
/// convention and then evaluates the paper's overall/directional PL equations.
pub fn student_t_protection_levels_for_tir(
    covariance: Covariance2,
    degrees_of_freedom: f64,
    target_integrity_risk: f64,
) -> Result<StudentTProtectionLevels, StudentTPlError> {
    let multiplier = student_t_k_tir_2d(degrees_of_freedom, target_integrity_risk)?;
    student_t_protection_levels(covariance, degrees_of_freedom, multiplier)
}

/// Paper's printed directional Student-t PL equation:
///
/// PL_i = K_TIR(dof) * sqrt((dof - 2) *
///       max(lambda_1 * |V_1,i|, lambda_2 * |V_2,i|)).
///
/// The vector-component weighting is kept exactly as printed; do not silently
/// replace it with squared projections or a conventional covariance ellipse.
pub fn student_t_protection_levels(
    covariance: Covariance2,
    degrees_of_freedom: f64,
    k_tir_2d: f64,
) -> Result<StudentTProtectionLevels, StudentTPlError> {
    if !degrees_of_freedom.is_finite() || degrees_of_freedom <= 2.0 {
        return Err(StudentTPlError::InvalidDegreesOfFreedom);
    }
    if !k_tir_2d.is_finite() || k_tir_2d <= 0.0 {
        return Err(StudentTPlError::InvalidTirMultiplier);
    }
    let frame = covariance.eigenframe()?;
    let multiplier = k_tir_2d * (degrees_of_freedom - 2.0).sqrt();

    let directional = |axis: usize| {
        let maximum_component = (frame.lambda_max * frame.maximum_axis[axis].abs())
            .max(frame.lambda_min * frame.minimum_axis[axis].abs());
        multiplier * maximum_component.sqrt()
    };

    let result = StudentTProtectionLevels {
        degrees_of_freedom,
        k_tir_2d,
        maximum_eigenvalue: frame.lambda_max,
        overall_pl_m: multiplier * frame.lambda_max.sqrt(),
        longitudinal_pl_m: directional(0),
        lateral_pl_m: directional(1),
    };
    if !result.overall_pl_m.is_finite()
        || !result.longitudinal_pl_m.is_finite()
        || !result.lateral_pl_m.is_finite()
    {
        return Err(StudentTPlError::NonFiniteOutput);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_covariance_matches_paper_student_t_pl_formula() {
        let result = student_t_protection_levels(
            Covariance2 { xx: 9.0, xy: 0.0, yy: 1.0 },
            4.0,
            2.0,
        ).unwrap();
        assert!((result.maximum_eigenvalue - 9.0).abs() < 1.0e-12);
        assert!((result.overall_pl_m - 2.0 * (2.0_f64 * 9.0).sqrt()).abs() < 1.0e-12);
        assert!((result.longitudinal_pl_m - 2.0 * (2.0_f64 * 9.0).sqrt()).abs() < 1.0e-12);
        assert!((result.lateral_pl_m - 2.0 * (2.0_f64).sqrt()).abs() < 1.0e-12);
    }

    #[test]
    fn isotropic_covariance_has_equal_directional_pls() {
        let result = student_t_protection_levels(
            Covariance2 { xx: 4.0, xy: 0.0, yy: 4.0 },
            5.0,
            1.7,
        ).unwrap();
        assert!((result.longitudinal_pl_m - result.lateral_pl_m).abs() < 1.0e-12);
    }

    #[test]
    fn rotated_anisotropic_covariance_projects_both_axes() {
        // Eigenvalues 9 and 1 rotated by 45 degrees.
        let result = student_t_protection_levels(
            Covariance2 { xx: 5.0, xy: 4.0, yy: 5.0 },
            4.0,
            2.0,
        ).unwrap();
        assert!((result.longitudinal_pl_m - result.lateral_pl_m).abs() < 1.0e-12);
        assert!(result.longitudinal_pl_m < result.overall_pl_m);
    }

    #[test]
    fn paper_reported_degrees_of_freedom_sweep_is_supported() {
        for dof in [4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 100.0] {
            let result = student_t_protection_levels(
                Covariance2 { xx: 9.0, xy: 1.0, yy: 2.0 },
                dof,
                2.0,
            ).unwrap();
            assert!(result.overall_pl_m.is_finite());
            assert!(result.longitudinal_pl_m.is_finite());
            assert!(result.lateral_pl_m.is_finite());
        }
    }

    #[test]
    fn two_dimensional_student_t_tir_multiplier_matches_closed_form_tail() {
        for dof in [4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 100.0] {
            let tir = 1.0e-3;
            let k = student_t_k_tir_2d(dof, tir).unwrap();
            let reconstructed_tail = (1.0 + k * k).powf(-dof / 2.0);
            assert!((reconstructed_tail - tir).abs() < 1.0e-12);
        }
        assert!(student_t_k_tir_2d(4.0, 1.0e-3).unwrap()
            > student_t_k_tir_2d(100.0, 1.0e-3).unwrap());
    }

    #[test]
    fn convenience_tir_api_reproduces_dof_sweep_without_normal_quantile_substitution() {
        let covariance = Covariance2 { xx: 9.0, xy: 1.0, yy: 2.0 };
        let results: Vec<_> = [4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 100.0]
            .into_iter()
            .map(|dof| student_t_protection_levels_for_tir(covariance, dof, 1.0e-3).unwrap())
            .collect();
        assert_eq!(results.len(), 8);
        assert!(results.windows(2).all(|w| w[0].overall_pl_m > w[1].overall_pl_m));
    }

    #[test]
    fn invalid_target_tir_fails_closed() {
        assert_eq!(
            student_t_k_tir_2d(4.0, 0.0),
            Err(StudentTPlError::InvalidTargetIntegrityRisk)
        );
        assert_eq!(
            student_t_k_tir_2d(4.0, 1.0),
            Err(StudentTPlError::InvalidTargetIntegrityRisk)
        );
    }

    #[test]
    fn invalid_covariance_degrees_of_freedom_and_multiplier_fail_closed() {
        assert_eq!(
            student_t_protection_levels(
                Covariance2 { xx: 1.0, xy: 2.0, yy: 1.0 },
                4.0,
                2.0,
            ),
            Err(StudentTPlError::CovarianceNotPositiveSemidefinite)
        );
        assert_eq!(
            student_t_protection_levels(
                Covariance2 { xx: 1.0, xy: 0.0, yy: 1.0 },
                2.0,
                2.0,
            ),
            Err(StudentTPlError::InvalidDegreesOfFreedom)
        );
        assert_eq!(
            student_t_protection_levels(
                Covariance2 { xx: 1.0, xy: 0.0, yy: 1.0 },
                4.0,
                f64::NAN,
            ),
            Err(StudentTPlError::InvalidTirMultiplier)
        );
    }
}
