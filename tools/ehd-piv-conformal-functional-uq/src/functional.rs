//! Spatially correlated uncertainty propagation for PIV field functionals.
//!
//! Each face sample carries a local 3x3 velocity covariance. Local errors are
//! treated as independent across samples. Cross-location dependencies are
//! represented by explicit shared latent modes (camera scale, pulse timing,
//! registration, density calibration, surface/area calibration, etc.). Each
//! mode is one independent unit-normal latent variable whose per-sample
//! sensitivities are one-sigma perturbations. Multiple correlated nuisance
//! variables must be orthogonalized into independent modes using an externally
//! validated factorization; independence is not inferred here.
//!
//! The propagated outputs are mass flux and the three components of advective
//! momentum flux over the supplied oriented surface-area vectors. Pressure,
//! viscous stress, body force, finite density uncertainty not entered as a
//! shared mode, and control-volume storage are outside this kernel.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Covariance3 {
    pub xx: f64,
    pub xy: f64,
    pub xz: f64,
    pub yy: f64,
    pub yz: f64,
    pub zz: f64,
}

impl Covariance3 {
    fn validate_psd(self, sample_index: usize) -> Result<(), FunctionalUqError> {
        let values = [self.xx, self.xy, self.xz, self.yy, self.yz, self.zz];
        if !values.iter().all(|value| value.is_finite()) {
            return Err(FunctionalUqError::NonFiniteCovariance { sample_index });
        }
        let scale = values.iter().map(|value| value.abs()).fold(0.0_f64, f64::max).max(1.0);
        let tolerance_1 = 1.0e-12 * scale;
        let tolerance_2 = 1.0e-12 * scale * scale;
        let tolerance_3 = 1.0e-12 * scale * scale * scale;

        let minor_xy = self.xx * self.yy - self.xy * self.xy;
        let minor_xz = self.xx * self.zz - self.xz * self.xz;
        let minor_yz = self.yy * self.zz - self.yz * self.yz;
        let determinant = self.xx * (self.yy * self.zz - self.yz * self.yz)
            - self.xy * (self.xy * self.zz - self.xz * self.yz)
            + self.xz * (self.xy * self.yz - self.xz * self.yy);

        if self.xx < -tolerance_1
            || self.yy < -tolerance_1
            || self.zz < -tolerance_1
            || minor_xy < -tolerance_2
            || minor_xz < -tolerance_2
            || minor_yz < -tolerance_2
            || determinant < -tolerance_3
            || !determinant.is_finite()
        {
            return Err(FunctionalUqError::CovarianceNotPositiveSemidefinite { sample_index });
        }
        Ok(())
    }

    fn quadratic_bilinear(self, left: [f64; 3], right: [f64; 3]) -> f64 {
        let c_left = [
            self.xx * right[0] + self.xy * right[1] + self.xz * right[2],
            self.xy * right[0] + self.yy * right[1] + self.yz * right[2],
            self.xz * right[0] + self.yz * right[1] + self.zz * right[2],
        ];
        left[0] * c_left[0] + left[1] * c_left[1] + left[2] * c_left[2]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FaceVelocitySample {
    pub velocity_m_s: [f64; 3],
    /// Oriented differential area vector dA = n dA, m^2.
    pub oriented_area_vector_m2: [f64; 3],
    pub density_kg_m3: f64,
    pub local_velocity_covariance_m2_s2: Covariance3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SharedModeSensitivity {
    /// One-sigma velocity perturbation at this sample, m/s.
    pub velocity_m_s: [f64; 3],
    /// One-sigma density perturbation at this sample, kg/m^3.
    pub density_kg_m3: f64,
    /// One-sigma oriented-area-vector perturbation at this sample, m^2.
    pub oriented_area_vector_m2: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct SharedFieldMode {
    /// Unique latent source identity. Each mode is treated as an independent
    /// unit-normal source after factorization.
    pub mode_id: String,
    pub per_sample_sensitivity: Vec<SharedModeSensitivity>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldFunctionalInput {
    pub run_id: String,
    pub control_surface_id: String,
    pub sign_convention_id: String,
    pub covariance_multiplier: f64,
    pub samples: Vec<FaceVelocitySample>,
    pub shared_modes: Vec<SharedFieldMode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldFunctionalReport {
    pub run_id: String,
    pub control_surface_id: String,
    pub sign_convention_id: String,
    /// [mass flow, momentum-x flux, momentum-y flux, momentum-z flux].
    /// Units: [kg/s, N, N, N].
    pub functional_values: [f64; 4],
    /// Full covariance among the four integrated functionals.
    pub functional_covariance: [[f64; 4]; 4],
    pub standard_uncertainties: [f64; 4],
    pub sample_count: usize,
    pub shared_mode_ids: Vec<String>,
    pub covariance_multiplier: f64,
    pub uncertainty_scope: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunctionalUqError {
    EmptyIdentity { field: &'static str },
    EmptySamples,
    InvalidCovarianceMultiplier,
    NonFiniteSample { sample_index: usize, field: &'static str },
    InvalidDensity { sample_index: usize },
    NonFiniteCovariance { sample_index: usize },
    CovarianceNotPositiveSemidefinite { sample_index: usize },
    SharedModeCountMismatch { mode_id: String, expected: usize, actual: usize },
    EmptySharedModeId,
    DuplicateSharedModeId { mode_id: String },
    NonFiniteSharedSensitivity { mode_id: String, sample_index: usize },
    NonFiniteDerivedValue,
    NegativePropagatedVariance { functional_index: usize },
}

impl fmt::Display for FunctionalUqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity { field } => write!(f, "field-functional {field} must be non-empty"),
            Self::EmptySamples => write!(f, "field-functional propagation requires samples"),
            Self::InvalidCovarianceMultiplier => write!(f, "covariance multiplier must be finite and positive"),
            Self::NonFiniteSample { sample_index, field } => write!(f, "sample {sample_index} has non-finite {field}"),
            Self::InvalidDensity { sample_index } => write!(f, "sample {sample_index} density must be finite and positive"),
            Self::NonFiniteCovariance { sample_index } => write!(f, "sample {sample_index} covariance is non-finite"),
            Self::CovarianceNotPositiveSemidefinite { sample_index } => write!(f, "sample {sample_index} covariance is not positive semidefinite"),
            Self::SharedModeCountMismatch { mode_id, expected, actual } => write!(f, "shared mode {mode_id} has {actual} sensitivities; expected {expected}"),
            Self::EmptySharedModeId => write!(f, "shared nuisance mode has an empty identity"),
            Self::DuplicateSharedModeId { mode_id } => write!(f, "shared nuisance mode is duplicated: {mode_id}"),
            Self::NonFiniteSharedSensitivity { mode_id, sample_index } => write!(f, "shared mode {mode_id} has non-finite sensitivity at sample {sample_index}"),
            Self::NonFiniteDerivedValue => write!(f, "field-functional propagation produced a non-finite result"),
            Self::NegativePropagatedVariance { functional_index } => write!(f, "propagated variance is negative for functional {functional_index}"),
        }
    }
}

impl Error for FunctionalUqError {}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

/// Propagate a field covariance model into integrated mass and advective
/// momentum fluxes using first-order Jacobians.
///
/// For each face sample i:
/// mass_i = rho_i * (u_i · dA_i)
/// momentum_ij = rho_i * u_ij * (u_i · dA_i)
///
/// The returned covariance includes:
/// - local 3x3 velocity covariance at each sample;
/// - cross-location covariance induced by the same shared latent mode;
/// - shared density and oriented-area-vector calibration modes;
/// - cross-covariance between mass flow and all momentum components.
///
/// `covariance_multiplier` scales the entire covariance model. It can carry
/// an externally calibrated field-wide covariance factor, e.g. a conformal
/// squared-radius scaling, but that factor must apply to all covariance sources
/// consistently and must not be interpreted as a new independent observation.
pub fn propagate_field_functional_uncertainty(
    input: &FieldFunctionalInput,
) -> Result<FieldFunctionalReport, FunctionalUqError> {
    for (field, value) in [
        ("run_id", input.run_id.as_str()),
        ("control_surface_id", input.control_surface_id.as_str()),
        ("sign_convention_id", input.sign_convention_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(FunctionalUqError::EmptyIdentity { field });
        }
    }
    if input.samples.is_empty() {
        return Err(FunctionalUqError::EmptySamples);
    }
    if !input.covariance_multiplier.is_finite() || input.covariance_multiplier <= 0.0 {
        return Err(FunctionalUqError::InvalidCovarianceMultiplier);
    }

    let mut functional_values = [0.0_f64; 4];
    // Jacobian rows per sample: mass flow, momentum-x, momentum-y, momentum-z.
    let mut jacobians: Vec<[[f64; 3]; 4]> = Vec::with_capacity(input.samples.len());

    for (sample_index, sample) in input.samples.iter().enumerate() {
        for value in sample.velocity_m_s {
            if !value.is_finite() {
                return Err(FunctionalUqError::NonFiniteSample { sample_index, field: "velocity" });
            }
        }
        for value in sample.oriented_area_vector_m2 {
            if !value.is_finite() {
                return Err(FunctionalUqError::NonFiniteSample { sample_index, field: "oriented_area_vector" });
            }
        }
        if !sample.density_kg_m3.is_finite() || sample.density_kg_m3 <= 0.0 {
            return Err(FunctionalUqError::InvalidDensity { sample_index });
        }
        sample.local_velocity_covariance_m2_s2.validate_psd(sample_index)?;

        let velocity = sample.velocity_m_s;
        let area = sample.oriented_area_vector_m2;
        let density = sample.density_kg_m3;
        let normal_velocity_flux = dot(velocity, area);
        functional_values[0] += density * normal_velocity_flux;
        for component in 0..3 {
            functional_values[component + 1] += density * velocity[component] * normal_velocity_flux;
        }

        let mut sample_jacobians = [[0.0_f64; 3]; 4];
        for axis in 0..3 {
            sample_jacobians[0][axis] = density * area[axis];
        }
        for momentum_component in 0..3 {
            for velocity_component in 0..3 {
                let identity_term = if momentum_component == velocity_component {
                    normal_velocity_flux
                } else {
                    0.0
                };
                sample_jacobians[momentum_component + 1][velocity_component] =
                    density * (identity_term + velocity[momentum_component] * area[velocity_component]);
            }
        }
        jacobians.push(sample_jacobians);
    }

    let mut functional_covariance = [[0.0_f64; 4]; 4];

    // Independent local random errors contribute only within a sample.
    for (sample_index, sample) in input.samples.iter().enumerate() {
        for left in 0..4 {
            for right in 0..4 {
                functional_covariance[left][right] += sample
                    .local_velocity_covariance_m2_s2
                    .quadratic_bilinear(jacobians[sample_index][left], jacobians[sample_index][right]);
            }
        }
    }

    let mut mode_ids = BTreeSet::new();
    for mode in &input.shared_modes {
        if mode.mode_id.trim().is_empty() {
            return Err(FunctionalUqError::EmptySharedModeId);
        }
        if !mode_ids.insert(mode.mode_id.clone()) {
            return Err(FunctionalUqError::DuplicateSharedModeId {
                mode_id: mode.mode_id.clone(),
            });
        }
        if mode.per_sample_sensitivity.len() != input.samples.len() {
            return Err(FunctionalUqError::SharedModeCountMismatch {
                mode_id: mode.mode_id.clone(),
                expected: input.samples.len(),
                actual: mode.per_sample_sensitivity.len(),
            });
        }

        let mut functional_sensitivity = [0.0_f64; 4];
        for (sample_index, sensitivity) in mode.per_sample_sensitivity.iter().enumerate() {
            if !sensitivity.velocity_m_s.iter().all(|value| value.is_finite())
                || !sensitivity.area_vector_m2.iter().all(|value| value.is_finite())
                || !sensitivity.density_kg_m3.is_finite()
            {
                return Err(FunctionalUqError::NonFiniteSharedSensitivity {
                    mode_id: mode.mode_id.clone(),
                    sample_index,
                });
            }
            let sample = &input.samples[sample_index];
            let velocity = sample.velocity_m_s;
            let area = sample.oriented_area_vector_m2;
            let density = sample.density_kg_m3;
            let normal_velocity_flux = dot(velocity, area);

            // d mass / d rho = u·dA; d mass / dA = rho*u.
            functional_sensitivity[0] +=
                dot(jacobians[sample_index][0], sensitivity.velocity_m_s)
                    + normal_velocity_flux * sensitivity.density_kg_m3
                    + density * dot(velocity, sensitivity.area_vector_m2);

            for momentum_component in 0..3 {
                let velocity_derivative = dot(
                    jacobians[sample_index][momentum_component + 1],
                    sensitivity.velocity_m_s,
                );
                let density_derivative =
                    velocity[momentum_component] * normal_velocity_flux
                        * sensitivity.density_kg_m3;
                let area_derivative = density
                    * velocity[momentum_component]
                    * dot(velocity, sensitivity.area_vector_m2);
                functional_sensitivity[momentum_component + 1] +=
                    velocity_derivative + density_derivative + area_derivative;
            }
        }

        // Each shared latent factor is an independent unit-normal source.
        for left in 0..4 {
            for right in 0..4 {
                functional_covariance[left][right] +=
                    functional_sensitivity[left] * functional_sensitivity[right];
            }
        }
    }

    for row in &mut functional_covariance {
        for value in row {
            *value *= input.covariance_multiplier;
            if !value.is_finite() {
                return Err(FunctionalUqError::NonFiniteDerivedValue);
            }
        }
    }

    let mut standard_uncertainties = [0.0_f64; 4];
    for functional_index in 0..4 {
        let variance = functional_covariance[functional_index][functional_index];
        if variance < -1.0e-12 {
            return Err(FunctionalUqError::NegativePropagatedVariance { functional_index });
        }
        standard_uncertainties[functional_index] = variance.max(0.0).sqrt();
    }
    if !functional_values.iter().all(|value| value.is_finite()) {
        return Err(FunctionalUqError::NonFiniteDerivedValue);
    }

    Ok(FieldFunctionalReport {
        run_id: input.run_id.clone(),
        control_surface_id: input.control_surface_id.clone(),
        sign_convention_id: input.sign_convention_id.clone(),
        functional_values,
        functional_covariance,
        standard_uncertainties,
        sample_count: input.samples.len(),
        shared_mode_ids: mode_ids.into_iter().collect(),
        covariance_multiplier: input.covariance_multiplier,
        uncertainty_scope: "First-order propagation of supplied 3D velocity covariance at each sample plus explicitly declared shared latent density/area/velocity modes. Local sample errors are assumed independent across samples. Cross-location dependence not represented by a shared mode is omitted. Pressure, viscous/body forces, fluid density model discrepancy, storage, temporal covariance and force-balance uncertainty are outside this reducer.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(velocity: [f64; 3], area: [f64; 3], variance_x: f64) -> FaceVelocitySample {
        FaceVelocitySample {
            velocity_m_s: velocity,
            oriented_area_vector_m2: area,
            density_kg_m3: 1.0,
            local_velocity_covariance_m2_s2: Covariance3 {
                xx: variance_x, xy: 0.0, xz: 0.0, yy: 0.0, yz: 0.0, zz: 0.0,
            },
        }
    }

    fn input(samples: Vec<FaceVelocitySample>) -> FieldFunctionalInput {
        FieldFunctionalInput {
            run_id: "manufactured-run".to_owned(),
            control_surface_id: "surface-all-faces-v1".to_owned(),
            sign_convention_id: "outward-area-vector-v1".to_owned(),
            covariance_multiplier: 1.0,
            samples,
            shared_modes: Vec::new(),
        }
    }

    #[test]
    fn one_face_sample_reproduces_mass_and_momentum_flux_jacobians() {
        let report = propagate_field_functional_uncertainty(&input(vec![
            sample([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25),
        ])).unwrap();
        assert_eq!(report.functional_values, [2.0, 4.0, 0.0, 0.0]);
        assert!((report.functional_covariance[0][0] - 0.25).abs() < 1.0e-12);
        assert!((report.functional_covariance[1][1] - 4.0).abs() < 1.0e-12);
        assert!((report.functional_covariance[0][1] - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn shared_velocity_calibration_mode_creates_cross_sample_covariance() {
        let mut data = input(vec![
            sample([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25),
            sample([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25),
        ]);
        data.shared_modes.push(SharedFieldMode {
            mode_id: "common-camera-scale".to_owned(),
            per_sample_sensitivity: vec![
                SharedModeSensitivity {
                    velocity_m_s: [0.1, 0.0, 0.0],
                    density_kg_m3: 0.0,
                    oriented_area_vector_m2: [0.0; 3],
                },
                SharedModeSensitivity {
                    velocity_m_s: [0.1, 0.0, 0.0],
                    density_kg_m3: 0.0,
                    oriented_area_vector_m2: [0.0; 3],
                },
            ],
        });
        let report = propagate_field_functional_uncertainty(&data).unwrap();
        // Independent local variance: 2 * 0.25. Common mode derivative: 0.1+0.1.
        assert!((report.functional_covariance[0][0] - 0.54).abs() < 1.0e-12);
    }

    #[test]
    fn shared_uncertainty_is_not_reduced_as_if_each_vector_were_independent() {
        let samples = vec![
            sample([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25),
            sample([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25),
        ];
        let local_only = propagate_field_functional_uncertainty(&input(samples.clone())).unwrap();
        let mut with_shared = input(samples);
        with_shared.shared_modes.push(SharedFieldMode {
            mode_id: "calibration-offset".to_owned(),
            per_sample_sensitivity: vec![
                SharedModeSensitivity {
                    velocity_m_s: [0.1, 0.0, 0.0],
                    density_kg_m3: 0.0,
                    oriented_area_vector_m2: [0.0; 3],
                },
                SharedModeSensitivity {
                    velocity_m_s: [0.1, 0.0, 0.0],
                    density_kg_m3: 0.0,
                    oriented_area_vector_m2: [0.0; 3],
                },
            ],
        });
        let shared = propagate_field_functional_uncertainty(&with_shared).unwrap();
        assert!(shared.functional_covariance[0][0] > local_only.functional_covariance[0][0]);
        assert!((local_only.functional_covariance[0][0] - 0.5).abs() < 1.0e-12);
        assert!((shared.functional_covariance[0][0] - 0.54).abs() < 1.0e-12);
    }

    #[test]
    fn common_density_mode_propagates_into_mass_and_momentum_covariance() {
        let mut data = input(vec![sample([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.0)]);
        data.shared_modes.push(SharedFieldMode {
            mode_id: "density-calibration".to_owned(),
            per_sample_sensitivity: vec![SharedModeSensitivity {
                velocity_m_s: [0.0; 3],
                density_kg_m3: 0.1,
                oriented_area_vector_m2: [0.0; 3],
            }],
        });
        let report = propagate_field_functional_uncertainty(&data).unwrap();
        assert!((report.functional_covariance[0][0] - 0.04).abs() < 1.0e-12);
        assert!((report.functional_covariance[0][1] - 0.08).abs() < 1.0e-12);
    }

    #[test]
    fn oriented_area_mode_is_propagated_into_integrated_functionals() {
        let mut data = input(vec![sample([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.0)]);
        data.shared_modes.push(SharedFieldMode {
            mode_id: "surface-registration".to_owned(),
            per_sample_sensitivity: vec![SharedModeSensitivity {
                velocity_m_s: [0.0; 3],
                density_kg_m3: 0.0,
                oriented_area_vector_m2: [0.1, 0.0, 0.0],
            }],
        });
        let report = propagate_field_functional_uncertainty(&data).unwrap();
        assert!((report.functional_covariance[0][0] - 0.04).abs() < 1.0e-12);
        assert!((report.functional_covariance[1][1] - 0.64).abs() < 1.0e-12);
    }

    #[test]
    fn covariance_multiplier_scales_every_covariance_contribution() {
        let mut data = input(vec![sample([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.25)]);
        data.covariance_multiplier = 4.0;
        let report = propagate_field_functional_uncertainty(&data).unwrap();
        assert!((report.functional_covariance[0][0] - 1.0).abs() < 1.0e-12);
        assert!((report.standard_uncertainties[0] - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn non_positive_semidefinite_local_covariance_is_rejected() {
        let mut data = input(vec![sample([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0)]);
        data.samples[0].local_velocity_covariance_m2_s2 = Covariance3 {
            xx: 1.0, xy: 0.9, xz: 0.9, yy: 1.0, yz: -0.9, zz: 1.0,
        };
        assert!(matches!(
            propagate_field_functional_uncertainty(&data),
            Err(FunctionalUqError::CovarianceNotPositiveSemidefinite { .. })
        ));
    }

    #[test]
    fn shared_mode_requires_one_sensitivity_per_field_sample() {
        let mut data = input(vec![
            sample([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.1),
            sample([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.1),
        ]);
        data.shared_modes.push(SharedFieldMode {
            mode_id: "wrong-shape".to_owned(),
            per_sample_sensitivity: vec![SharedModeSensitivity {
                velocity_m_s: [0.1, 0.0, 0.0],
                density_kg_m3: 0.0,
                oriented_area_vector_m2: [0.0; 3],
            }],
        });
        assert!(matches!(
            propagate_field_functional_uncertainty(&data),
            Err(FunctionalUqError::SharedModeCountMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_shared_mode_ids_are_rejected() {
        let mut data = input(vec![sample([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.1)]);
        let mode = SharedFieldMode {
            mode_id: "same-source".to_owned(),
            per_sample_sensitivity: vec![SharedModeSensitivity {
                velocity_m_s: [0.1, 0.0, 0.0],
                density_kg_m3: 0.0,
                oriented_area_vector_m2: [0.0; 3],
            }],
        };
        data.shared_modes = vec![mode.clone(), mode];
        assert!(matches!(
            propagate_field_functional_uncertainty(&data),
            Err(FunctionalUqError::DuplicateSharedModeId { .. })
        ));
    }
}
