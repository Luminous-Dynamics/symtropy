//! Independent analytical oracle for PHYS-CCX-MMS-P1-001.
//!
//! This crate evaluates the manufactured continuum field. It does not run
//! CalculiX or qualify any finite-element result.

use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strain2 {
    pub xx: f64,
    pub yy: f64,
    pub xy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stress3 {
    pub xx: f64,
    pub yy: f64,
    pub zz: f64,
    pub xy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneStrainMaterial {
    youngs_modulus: f64,
    poisson_ratio: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OracleError {
    NonFiniteMaterial,
    NonPositiveYoungsModulus,
    InvalidPoissonRatio,
    NonFiniteAmplitude,
}

impl PlaneStrainMaterial {
    pub fn youngs_modulus(self) -> f64 {
        self.youngs_modulus
    }

    pub fn poisson_ratio(self) -> f64 {
        self.poisson_ratio
    }

    pub fn new(youngs_modulus: f64, poisson_ratio: f64) -> Result<Self, OracleError> {
        if !youngs_modulus.is_finite() || !poisson_ratio.is_finite() {
            return Err(OracleError::NonFiniteMaterial);
        }
        if youngs_modulus <= 0.0 {
            return Err(OracleError::NonPositiveYoungsModulus);
        }
        if !(-1.0 < poisson_ratio && poisson_ratio < 0.5) {
            return Err(OracleError::InvalidPoissonRatio);
        }
        Ok(Self {
            youngs_modulus,
            poisson_ratio,
        })
    }

    /// Return the Lamé parameters (lambda, mu) for isotropic plane strain.
    pub fn lame_parameters(self) -> (f64, f64) {
        let e = self.youngs_modulus;
        let nu = self.poisson_ratio;
        let mu = e / (2.0 * (1.0 + nu));
        let lambda = e * nu / ((1.0 + nu) * (1.0 - 2.0 * nu));
        (lambda, mu)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManufacturedProblem {
    amplitude_x: f64,
    amplitude_y: f64,
    material: PlaneStrainMaterial,
    lambda: f64,
    mu: f64,
}

impl ManufacturedProblem {
    pub fn new(
        material: PlaneStrainMaterial,
        amplitude_x: f64,
        amplitude_y: f64,
    ) -> Result<Self, OracleError> {
        if !amplitude_x.is_finite() || !amplitude_y.is_finite() {
            return Err(OracleError::NonFiniteAmplitude);
        }
        let (lambda, mu) = material.lame_parameters();
        Ok(Self {
            amplitude_x,
            amplitude_y,
            material,
            lambda,
            mu,
        })
    }

    pub fn reference() -> Self {
        let material = PlaneStrainMaterial::new(1.0, 0.3)
            .expect("reference material parameters are admissible");
        Self::new(material, 1.0, 0.7).expect("reference amplitudes are finite")
    }

    pub fn material(self) -> PlaneStrainMaterial {
        self.material
    }

    pub fn amplitudes(self) -> (f64, f64) {
        (self.amplitude_x, self.amplitude_y)
    }

    /// Exact manufactured displacement on the unit square.
    pub fn displacement(self, x: f64, y: f64) -> Vec2 {
        let k = PI;
        Vec2 {
            x: self.amplitude_x * (k * x).sin() * (k * y).sin(),
            y: self.amplitude_y * (2.0 * k * x).sin() * (k * y).sin(),
        }
    }

    /// Exact infinitesimal strain tensor in the x-y plane.
    pub fn strain(self, x: f64, y: f64) -> Strain2 {
        let k = PI;
        let xx = self.amplitude_x * k * (k * x).cos() * (k * y).sin();
        let yy = self.amplitude_y * k * (2.0 * k * x).sin() * (k * y).cos();
        let xy = 0.5
            * (self.amplitude_x * k * (k * x).sin() * (k * y).cos()
                + 2.0 * self.amplitude_y * k * (2.0 * k * x).cos() * (k * y).sin());
        Strain2 { xx, yy, xy }
    }

    /// Exact three-dimensional stress for plane strain, where epsilon_zz is zero.
    pub fn stress(self, x: f64, y: f64) -> Stress3 {
        let strain = self.strain(x, y);
        let trace = strain.xx + strain.yy;
        Stress3 {
            xx: self.lambda * trace + 2.0 * self.mu * strain.xx,
            yy: self.lambda * trace + 2.0 * self.mu * strain.yy,
            zz: self.lambda * trace,
            xy: 2.0 * self.mu * strain.xy,
        }
    }

    /// Evaluate the force using the isotropic Navier operator.
    ///
    /// This is intentionally separate from the explicit symbolic expression below
    /// so tests can detect algebraic coefficient errors in that expression.
    pub fn body_force_from_operator(self, x: f64, y: f64) -> Vec2 {
        let k = PI;
        let u = self.displacement(x, y);
        let lap_x = -2.0 * k * k * u.x;
        let lap_y = -5.0 * k * k * u.y;
        let div_x = -self.amplitude_x * k * k * (k * x).sin() * (k * y).sin()
            + 2.0 * self.amplitude_y * k * k * (2.0 * k * x).cos() * (k * y).cos();
        let div_y = self.amplitude_x * k * k * (k * x).cos() * (k * y).cos()
            - self.amplitude_y * k * k * (2.0 * k * x).sin() * (k * y).sin();
        Vec2 {
            x: -self.mu * lap_x - (self.lambda + self.mu) * div_x,
            y: -self.mu * lap_y - (self.lambda + self.mu) * div_y,
        }
    }

    /// Explicit symbolic force expression from the frozen benchmark contract.
    pub fn body_force_closed_form(self, x: f64, y: f64) -> Vec2 {
        let k = PI;
        Vec2 {
            x: (self.lambda + 3.0 * self.mu)
                * self.amplitude_x
                * k
                * k
                * (k * x).sin()
                * (k * y).sin()
                - 2.0
                    * (self.lambda + self.mu)
                    * self.amplitude_y
                    * k
                    * k
                    * (k * y).cos()
                    * (2.0 * k * x).cos(),
            y: (self.lambda + 6.0 * self.mu)
                * self.amplitude_y
                * k
                * k
                * (2.0 * k * x).sin()
                * (k * y).sin()
                - (self.lambda + self.mu)
                    * self.amplitude_x
                    * k
                    * k
                    * (k * x).cos()
                    * (k * y).cos(),
        }
    }

    /// Analytical integral of the body force over the unit square.
    pub fn integrated_body_force(self) -> Vec2 {
        Vec2 {
            x: 4.0 * self.amplitude_x * (self.lambda + 3.0 * self.mu),
            y: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn reference_lame_parameters_are_plane_strain_values() {
        let material = PlaneStrainMaterial::new(1.0, 0.3).unwrap();
        let (lambda, mu) = material.lame_parameters();
        close(lambda, 0.5769230769230769, 1.0e-15);
        close(mu, 0.3846153846153846, 1.0e-15);
    }

    #[test]
    fn field_is_zero_on_all_four_boundaries() {
        let problem = ManufacturedProblem::reference();
        for index in 0..=16 {
            let t = (index as f64) / 16.0;
            for (x, y) in [(0.0, t), (1.0, t), (t, 0.0), (t, 1.0)] {
                let u = problem.displacement(x, y);
                close(u.x, 0.0, 2.0e-15);
                close(u.y, 0.0, 2.0e-15);
            }
        }
    }

    #[test]
    fn operator_and_expanded_force_agree_at_frozen_points() {
        let problem = ManufacturedProblem::reference();
        let points = [
            (0.071, 0.193),
            (0.2, 0.4),
            (0.333, 0.617),
            (0.5, 0.5),
            (0.823, 0.911),
        ];
        for (x, y) in points {
            let operator = problem.body_force_from_operator(x, y);
            let expanded = problem.body_force_closed_form(x, y);
            close(operator.x, expanded.x, 1.0e-13);
            close(operator.y, expanded.y, 1.0e-13);
        }
    }

    #[test]
    fn analytical_global_resultant_matches_composite_simpson_integration() {
        let problem = ManufacturedProblem::reference();
        let exact = problem.integrated_body_force();
        let n = 400_usize;
        let h = 1.0 / n as f64;
        let mut sums = Vec2 { x: 0.0, y: 0.0 };

        for i in 0..=n {
            let x = i as f64 * h;
            let wx = if i == 0 || i == n {
                1.0
            } else if i % 2 == 0 {
                2.0
            } else {
                4.0
            };
            for j in 0..=n {
                let y = j as f64 * h;
                let wy = if j == 0 || j == n {
                    1.0
                } else if j % 2 == 0 {
                    2.0
                } else {
                    4.0
                };
                let f = problem.body_force_from_operator(x, y);
                sums.x += wx * wy * f.x;
                sums.y += wx * wy * f.y;
            }
        }
        let integral = Vec2 {
            x: sums.x * h * h / 9.0,
            y: sums.y * h * h / 9.0,
        };
        close(integral.x, exact.x, 1.0e-8);
        close(integral.y, exact.y, 1.0e-8);
    }

    #[test]
    fn strain_and_stress_are_finite_and_constitutively_consistent() {
        let problem = ManufacturedProblem::reference();
        let (_, mu) = problem.material().lame_parameters();
        for index in 0..=20 {
            let t = (index as f64) / 20.0;
            let strain = problem.strain(t, 1.0 - t);
            let stress = problem.stress(t, 1.0 - t);
            assert!(strain.xx.is_finite());
            assert!(strain.yy.is_finite());
            assert!(strain.xy.is_finite());
            assert!(stress.xx.is_finite());
            assert!(stress.yy.is_finite());
            assert!(stress.zz.is_finite());
            assert!(stress.xy.is_finite());
            close(stress.xy, 2.0 * mu * strain.xy, 1.0e-14);
        }
    }

    #[test]
    fn invalid_materials_and_nonfinite_amplitudes_are_rejected() {
        assert_eq!(
            PlaneStrainMaterial::new(0.0, 0.3).unwrap_err(),
            OracleError::NonPositiveYoungsModulus
        );
        assert_eq!(
            PlaneStrainMaterial::new(1.0, 0.5).unwrap_err(),
            OracleError::InvalidPoissonRatio
        );
        assert_eq!(
            PlaneStrainMaterial::new(f64::NAN, 0.3).unwrap_err(),
            OracleError::NonFiniteMaterial
        );
        let material = PlaneStrainMaterial::new(1.0, 0.3).unwrap();
        assert_eq!(
            ManufacturedProblem::new(material, f64::INFINITY, 0.7).unwrap_err(),
            OracleError::NonFiniteAmplitude
        );
    }
}
