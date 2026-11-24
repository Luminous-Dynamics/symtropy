// SPDX-License-Identifier: AGPL-3.0-or-later
//! Analytic aggregate planar friction-wrench ellipsoid.
//!
//! The admissible wrench is [f_u, f_v, m_n] with semi-axes
//! a = mu * normal_load * [e_u, e_v, e_n].
//!
//! This crate implements support, membership and a local maximum-dissipation
//! map. It does not implement normal-contact complementarity or time stepping.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SCHEMA_VERSION: &str = "planar-patch-limit-surface-oracle-v0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PatchErrorKind {
    InvalidParameter,
    NonFiniteInput,
    NonFiniteEvaluation,
    InvalidTolerance,
    UnderflowedCapacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatchError {
    pub kind: PatchErrorKind,
    pub reason: String,
}

impl PatchError {
    fn new(kind: PatchErrorKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            reason: reason.into(),
        }
    }
}

impl Display for PatchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl Error for PatchError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanarPatchModel {
    /// Dimensionless, finite and non-negative.
    pub friction_coefficient: f64,
    /// Compressive normal load in newtons, finite and non-negative.
    pub normal_load: f64,
    /// Positive [tangent-u force, tangent-v force, torsional length] scales.
    pub ellipsoid_scales: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wrench3 {
    pub tangent_u_force: f64,
    pub tangent_v_force: f64,
    pub normal_torsional_moment: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WrenchDirection3 {
    /// Dual direction for [force-u, force-v, torsional-moment].
    pub components: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatchSupportReport {
    pub support_value: f64,
    pub maximizing_wrench: Wrench3,
    pub semi_axes: [f64; 3],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MembershipStatus {
    Interior,
    Boundary,
    Outside,
    ZeroCapacityZeroWrench,
    ZeroCapacityNonzeroWrench,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MembershipReport {
    pub status: MembershipStatus,
    /// None means the zero-capacity case has no finite normalized radius.
    pub normalized_ellipsoid_radius: Option<f64>,
    pub normalized_excess: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaxDissipationTolerance {
    /// Treat ||diag(semi_axes) * slip_twist|| below this as zero slip (watts).
    pub slip_power_tolerance: f64,
    /// Maximum permitted small positive floating-point power residual (watts).
    pub positive_power_tolerance: f64,
}

impl Default for MaxDissipationTolerance {
    fn default() -> Self {
        Self {
            slip_power_tolerance: 1.0e-12,
            positive_power_tolerance: 1.0e-10,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlipTwist3 {
    pub tangent_u_velocity: f64,
    pub tangent_v_velocity: f64,
    pub normal_angular_velocity: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaxDissipationResult {
    Sliding {
        friction_wrench: Wrench3,
        power: f64,
        dissipation_rate: f64,
        normalized_boundary_radius: f64,
        slip_power_norm: f64,
    },
    StickingSet {
        semi_axes: [f64; 3],
        maximum_support_scale: f64,
    },
    ZeroFrictionCapacity {
        friction_wrench: Wrench3,
        power: f64,
    },
}

fn finite3(values: [f64; 3]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn stable_norm(values: [f64; 3]) -> f64 {
    values[0].hypot(values[1]).hypot(values[2])
}

fn wrench_array(wrench: Wrench3) -> [f64; 3] {
    [
        wrench.tangent_u_force,
        wrench.tangent_v_force,
        wrench.normal_torsional_moment,
    ]
}

fn array_wrench(values: [f64; 3]) -> Wrench3 {
    Wrench3 {
        tangent_u_force: values[0],
        tangent_v_force: values[1],
        normal_torsional_moment: values[2],
    }
}

impl PlanarPatchModel {
    pub fn validate(self) -> Result<[f64; 3], PatchError> {
        if !self.friction_coefficient.is_finite()
            || self.friction_coefficient < 0.0
            || !self.normal_load.is_finite()
            || self.normal_load < 0.0
            || !finite3(self.ellipsoid_scales)
            || self.ellipsoid_scales.iter().any(|scale| *scale <= 0.0)
        {
            return Err(PatchError::new(
                PatchErrorKind::InvalidParameter,
                "friction and normal load must be finite/non-negative; ellipsoid scales must be finite/positive",
            ));
        }

        let common_scale = self.friction_coefficient * self.normal_load;
        if !common_scale.is_finite() {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "friction-load product overflowed",
            ));
        }
        if self.friction_coefficient > 0.0 && self.normal_load > 0.0 && common_scale == 0.0 {
            return Err(PatchError::new(
                PatchErrorKind::UnderflowedCapacity,
                "positive friction and normal load underflowed to zero capacity",
            ));
        }

        let axes = self.ellipsoid_scales.map(|scale| common_scale * scale);
        if !finite3(axes) {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "patch friction ellipsoid semi-axes overflowed",
            ));
        }
        if common_scale > 0.0 && axes.iter().any(|axis| *axis == 0.0) {
            return Err(PatchError::new(
                PatchErrorKind::UnderflowedCapacity,
                "positive patch semi-axis underflowed to zero",
            ));
        }
        Ok(axes)
    }

    /// Analytic support h(d) = ||diag(a) d|| and a maximizing boundary witness.
    pub fn support(self, direction: WrenchDirection3) -> Result<PatchSupportReport, PatchError> {
        if !finite3(direction.components) {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteInput,
                "support direction must be finite",
            ));
        }
        let axes = self.validate()?;
        let scaled = std::array::from_fn(|i| axes[i] * direction.components[i]);
        if !finite3(scaled) {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "scaled support direction is non-finite",
            ));
        }
        let support_value = stable_norm(scaled);
        if !support_value.is_finite() {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "support value is non-finite",
            ));
        }
        let witness = if support_value == 0.0 {
            [0.0; 3]
        } else {
            std::array::from_fn(|i| axes[i] * scaled[i] / support_value)
        };
        if !finite3(witness) {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "support witness is non-finite",
            ));
        }
        Ok(PatchSupportReport {
            support_value,
            maximizing_wrench: array_wrench(witness),
            semi_axes: axes,
        })
    }

    /// Classify membership in the declared ellipsoid.
    pub fn membership(
        self,
        wrench: Wrench3,
        tolerance: f64,
    ) -> Result<MembershipReport, PatchError> {
        let values = wrench_array(wrench);
        if !finite3(values) {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteInput,
                "wrench components must be finite",
            ));
        }
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(PatchError::new(
                PatchErrorKind::InvalidTolerance,
                "membership tolerance must be finite and non-negative",
            ));
        }
        let axes = self.validate()?;
        if axes.iter().all(|axis| *axis == 0.0) {
            let zero = values.iter().all(|value| *value == 0.0);
            return Ok(MembershipReport {
                status: if zero {
                    MembershipStatus::ZeroCapacityZeroWrench
                } else {
                    MembershipStatus::ZeroCapacityNonzeroWrench
                },
                normalized_ellipsoid_radius: if zero { Some(0.0) } else { None },
                normalized_excess: if zero { Some(0.0) } else { None },
            });
        }

        let mut normalized = [0.0; 3];
        for ((output, value), axis) in normalized.iter_mut().zip(values.iter()).zip(axes.iter()) {
            if *axis == 0.0 {
                if *value != 0.0 {
                    return Ok(MembershipReport {
                        status: MembershipStatus::Outside,
                        normalized_ellipsoid_radius: None,
                        normalized_excess: None,
                    });
                }
                *output = 0.0;
            } else {
                *output = *value / axis;
            }
        }
        let radius = stable_norm(normalized);
        if !radius.is_finite() {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "normalized membership radius is non-finite",
            ));
        }
        let excess = (radius - 1.0).max(0.0);
        let status = if (radius - 1.0).abs() <= tolerance {
            MembershipStatus::Boundary
        } else if radius < 1.0 {
            MembershipStatus::Interior
        } else {
            MembershipStatus::Outside
        };
        Ok(MembershipReport {
            status,
            normalized_ellipsoid_radius: Some(radius),
            normalized_excess: Some(excess),
        })
    }

    /// Maximum-dissipation wrench for a fixed normal load and nonzero slip twist.
    ///
    /// w_i = -(a_i^2 v_i) / ||diag(a) v||.
    /// Zero slip is set-valued and returns StickingSet.
    pub fn maximum_dissipation(
        self,
        slip: SlipTwist3,
        tolerance: MaxDissipationTolerance,
    ) -> Result<MaxDissipationResult, PatchError> {
        let velocity = [
            slip.tangent_u_velocity,
            slip.tangent_v_velocity,
            slip.normal_angular_velocity,
        ];
        if !finite3(velocity) {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteInput,
                "slip twist must be finite",
            ));
        }
        if !tolerance.slip_power_tolerance.is_finite()
            || tolerance.slip_power_tolerance < 0.0
            || !tolerance.positive_power_tolerance.is_finite()
            || tolerance.positive_power_tolerance < 0.0
        {
            return Err(PatchError::new(
                PatchErrorKind::InvalidTolerance,
                "maximum-dissipation tolerances must be finite and non-negative",
            ));
        }

        let axes = self.validate()?;
        if axes.iter().all(|axis| *axis == 0.0) {
            return Ok(MaxDissipationResult::ZeroFrictionCapacity {
                friction_wrench: array_wrench([0.0; 3]),
                power: 0.0,
            });
        }

        let scaled_slip = std::array::from_fn(|i| axes[i] * velocity[i]);
        if !finite3(scaled_slip) {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "scaled slip twist is non-finite",
            ));
        }
        let slip_power_norm = stable_norm(scaled_slip);
        if !slip_power_norm.is_finite() {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "slip-power norm is non-finite",
            ));
        }
        if slip_power_norm <= tolerance.slip_power_tolerance {
            return Ok(MaxDissipationResult::StickingSet {
                semi_axes: axes,
                maximum_support_scale: 1.0,
            });
        }

        let friction = std::array::from_fn(|i| -(axes[i] * scaled_slip[i]) / slip_power_norm);
        if !finite3(friction) {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "maximum-dissipation wrench is non-finite",
            ));
        }
        let power =
            velocity[0] * friction[0] + velocity[1] * friction[1] + velocity[2] * friction[2];
        if !power.is_finite() || power > tolerance.positive_power_tolerance {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "maximum-dissipation power exceeds the declared positive tolerance",
            ));
        }

        let mut boundary_coordinates = [0.0; 3];
        for ((output, value), axis) in boundary_coordinates
            .iter_mut()
            .zip(friction.iter())
            .zip(axes.iter())
        {
            if *axis == 0.0 {
                if *value != 0.0 {
                    return Err(PatchError::new(
                        PatchErrorKind::NonFiniteEvaluation,
                        "nonzero friction wrench lies on a zero semi-axis",
                    ));
                }
                *output = 0.0;
            } else {
                *output = *value / axis;
            }
        }
        let normalized_boundary_radius = stable_norm(boundary_coordinates);
        if !normalized_boundary_radius.is_finite() {
            return Err(PatchError::new(
                PatchErrorKind::NonFiniteEvaluation,
                "maximum-dissipation boundary radius is non-finite",
            ));
        }

        Ok(MaxDissipationResult::Sliding {
            friction_wrench: array_wrench(friction),
            power,
            dissipation_rate: (-power).max(0.0),
            normalized_boundary_radius,
            slip_power_norm,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> PlanarPatchModel {
        PlanarPatchModel {
            friction_coefficient: 0.5,
            normal_load: 2.0,
            ellipsoid_scales: [1.0, 1.0, 0.25],
        }
    }

    fn wrench(values: [f64; 3]) -> Wrench3 {
        array_wrench(values)
    }

    fn close(left: f64, right: f64) {
        assert!((left - right).abs() < 1.0e-12, "{left} != {right}");
    }

    #[test]
    fn tangential_and_torsional_support_have_known_answers() {
        let tangent = model()
            .support(WrenchDirection3 {
                components: [1.0, 0.0, 0.0],
            })
            .expect("tangent support");
        close(tangent.support_value, 1.0);
        assert_eq!(wrench_array(tangent.maximizing_wrench), [1.0, 0.0, 0.0]);

        let torsion = model()
            .support(WrenchDirection3 {
                components: [0.0, 0.0, 2.0],
            })
            .expect("torsional support");
        close(torsion.support_value, 0.5);
        assert_eq!(wrench_array(torsion.maximizing_wrench), [0.0, 0.0, 0.25]);
    }

    #[test]
    fn anisotropic_support_uses_each_axis() {
        let anisotropic = PlanarPatchModel {
            friction_coefficient: 0.5,
            normal_load: 2.0,
            ellipsoid_scales: [2.0, 1.0, 0.5],
        };
        let report = anisotropic
            .support(WrenchDirection3 {
                components: [1.0, 1.0, 1.0],
            })
            .expect("anisotropic support");
        let expected = 5.25_f64.sqrt();
        close(report.support_value, expected);
        let witness = wrench_array(report.maximizing_wrench);
        close(witness[0], 4.0 / expected);
        close(witness[1], 1.0 / expected);
        close(witness[2], 0.25 / expected);
    }

    #[test]
    fn support_witness_is_on_boundary() {
        let report = model()
            .support(WrenchDirection3 {
                components: [2.0, -3.0, 4.0],
            })
            .expect("support");
        let membership = model
            .membership(report.maximizing_wrench, 1.0e-12)
            .expect("membership");
        assert_eq!(membership.status, MembershipStatus::Boundary);
        close(membership.normalized_ellipsoid_radius.expect("radius"), 1.0);
    }

    #[test]
    fn membership_distinguishes_interior_boundary_and_outside() {
        let patch = model();
        assert_eq!(
            patch
                .membership(wrench([0.0; 3]), 1.0e-12)
                .expect("interior")
                .status,
            MembershipStatus::Interior
        );
        assert_eq!(
            patch
                .membership(wrench([1.0, 0.0, 0.0]), 1.0e-12)
                .expect("boundary")
                .status,
            MembershipStatus::Boundary
        );
        assert_eq!(
            patch
                .membership(wrench([1.01, 0.0, 0.0]), 1.0e-12)
                .expect("outside")
                .status,
            MembershipStatus::Outside
        );
    }

    #[test]
    fn maximum_dissipation_selects_opposing_boundary_wrench() {
        let result = model()
            .maximum_dissipation(
                SlipTwist3 {
                    tangent_u_velocity: 3.0,
                    tangent_v_velocity: 4.0,
                    normal_angular_velocity: 0.0,
                },
                MaxDissipationTolerance::default(),
            )
            .expect("sliding");
        let MaxDissipationResult::Sliding {
            friction_wrench,
            power,
            dissipation_rate,
            normalized_boundary_radius,
            ..
        } = result
        else {
            panic!("nonzero slip must select sliding");
        };
        close(friction_wrench.tangent_u_force, -0.6);
        close(friction_wrench.tangent_v_force, -0.8);
        close(power, -5.0);
        close(dissipation_rate, 5.0);
        close(normalized_boundary_radius, 1.0);
    }

    #[test]
    fn torsional_slip_uses_declared_moment_axis() {
        let result = model()
            .maximum_dissipation(
                SlipTwist3 {
                    tangent_u_velocity: 0.0,
                    tangent_v_velocity: 0.0,
                    normal_angular_velocity: 2.0,
                },
                MaxDissipationTolerance::default(),
            )
            .expect("torsional sliding");
        let MaxDissipationResult::Sliding {
            friction_wrench,
            power,
            ..
        } = result
        else {
            panic!("torsional slip must slide");
        };
        close(friction_wrench.normal_torsional_moment, -0.25);
        close(power, -0.5);
    }

    #[test]
    fn anisotropic_maximum_dissipation_uses_all_axes() {
        let anisotropic = PlanarPatchModel {
            friction_coefficient: 0.5,
            normal_load: 4.0,
            ellipsoid_scales: [1.0, 0.5, 0.125],
        };
        let result = anisotropic
            .maximum_dissipation(
                SlipTwist3 {
                    tangent_u_velocity: 1.0,
                    tangent_v_velocity: 2.0,
                    normal_angular_velocity: 4.0,
                },
                MaxDissipationTolerance::default(),
            )
            .expect("anisotropic sliding");
        let MaxDissipationResult::Sliding {
            friction_wrench,
            power,
            normalized_boundary_radius,
            ..
        } = result
        else {
            panic!("anisotropic nonzero slip must slide");
        };
        close(friction_wrench.tangent_u_force, -4.0 / 3.0);
        close(friction_wrench.tangent_v_force, -2.0 / 3.0);
        close(friction_wrench.normal_torsional_moment, -0.25 / 3.0);
        close(power, -3.0);
        close(normalized_boundary_radius, 1.0);
    }

    #[test]
    fn zero_slip_returns_set_valued_sticking() {
        let result = model()
            .maximum_dissipation(
                SlipTwist3 {
                    tangent_u_velocity: 0.0,
                    tangent_v_velocity: 0.0,
                    normal_angular_velocity: 0.0,
                },
                MaxDissipationTolerance::default(),
            )
            .expect("sticking");
        assert!(matches!(result, MaxDissipationResult::StickingSet { .. }));
    }

    #[test]
    fn zero_capacity_has_only_zero_friction_wrench() {
        let zero = PlanarPatchModel {
            friction_coefficient: 0.0,
            ..model()
        };
        let result = zero
            .maximum_dissipation(
                SlipTwist3 {
                    tangent_u_velocity: 1.0,
                    tangent_v_velocity: 2.0,
                    normal_angular_velocity: 3.0,
                },
                MaxDissipationTolerance::default(),
            )
            .expect("zero capacity");
        assert!(matches!(
            result,
            MaxDissipationResult::ZeroFrictionCapacity { .. }
        ));
        assert_eq!(
            zero.membership(wrench([0.0; 3]), 0.0).expect("zero").status,
            MembershipStatus::ZeroCapacityZeroWrench
        );
        assert_eq!(
            zero.membership(wrench([0.1, 0.0, 0.0]), 0.0)
                .expect("nonzero")
                .status,
            MembershipStatus::ZeroCapacityNonzeroWrench
        );
    }

    #[test]
    fn load_scaling_scales_support_linearly() {
        let base = model();
        let scaled = PlanarPatchModel {
            normal_load: 6.0,
            ..base
        };
        let direction = WrenchDirection3 {
            components: [1.0, 2.0, -3.0],
        };
        let first = base.support(direction).expect("base");
        let second = scaled.support(direction).expect("scaled");
        close(second.support_value, 3.0 * first.support_value);
        for (left, right) in wrench_array(second.maximizing_wrench)
            .iter()
            .zip(wrench_array(first.maximizing_wrench).iter())
        {
            close(*left, 3.0 * *right);
        }
    }

    #[test]
    fn zero_normal_load_has_zero_capacity() {
        let patch = PlanarPatchModel {
            normal_load: 0.0,
            ..model()
        };
        let report = patch
            .support(WrenchDirection3 {
                components: [1.0, 2.0, 3.0],
            })
            .expect("zero normal load");
        assert_eq!(report.support_value, 0.0);
        assert_eq!(wrench_array(report.maximizing_wrench), [0.0; 3]);
    }

    #[test]
    fn invalid_model_parameters_are_rejected() {
        for invalid in [
            PlanarPatchModel {
                friction_coefficient: f64::NAN,
                ..model()
            },
            PlanarPatchModel {
                friction_coefficient: -0.1,
                ..model()
            },
            PlanarPatchModel {
                normal_load: f64::INFINITY,
                ..model()
            },
            PlanarPatchModel {
                normal_load: -1.0,
                ..model()
            },
            PlanarPatchModel {
                ellipsoid_scales: [1.0, 0.0, 0.2],
                ..model()
            },
            PlanarPatchModel {
                ellipsoid_scales: [1.0, 1.0, f64::NAN],
                ..model()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn positive_capacity_underflow_is_rejected() {
        let tiny = PlanarPatchModel {
            friction_coefficient: 1.0e-308,
            normal_load: 1.0e-308,
            ellipsoid_scales: [1.0; 3],
        };
        assert_eq!(
            tiny.validate().expect_err("underflow").kind,
            PatchErrorKind::UnderflowedCapacity
        );
    }

    #[test]
    fn non_finite_wrench_and_direction_are_rejected() {
        let patch = model();
        assert!(patch.membership(wrench([f64::NAN, 0.0, 0.0]), 0.0).is_err());
        assert!(
            patch
                .support(WrenchDirection3 {
                    components: [f64::INFINITY, 0.0, 0.0]
                })
                .is_err()
        );
    }

    #[test]
    fn membership_tolerance_must_be_nonnegative() {
        assert!(model().membership(wrench([0.0; 3]), -1.0).is_err());
    }

    #[test]
    fn slip_twist_and_tolerances_must_be_valid() {
        assert!(
            model()
                .maximum_dissipation(
                    SlipTwist3 {
                        tangent_u_velocity: f64::NAN,
                        tangent_v_velocity: 0.0,
                        normal_angular_velocity: 0.0,
                    },
                    MaxDissipationTolerance::default(),
                )
                .is_err()
        );
        assert!(
            model()
                .maximum_dissipation(
                    SlipTwist3 {
                        tangent_u_velocity: 1.0,
                        tangent_v_velocity: 0.0,
                        normal_angular_velocity: 0.0,
                    },
                    MaxDissipationTolerance {
                        slip_power_tolerance: -1.0,
                        ..MaxDissipationTolerance::default()
                    },
                )
                .is_err()
        );
    }

    #[test]
    fn positive_power_tolerance_must_be_valid() {
        for tolerance in [f64::NAN, -1.0] {
            assert!(
                model()
                    .maximum_dissipation(
                        SlipTwist3 {
                            tangent_u_velocity: 1.0,
                            tangent_v_velocity: 0.0,
                            normal_angular_velocity: 0.0,
                        },
                        MaxDissipationTolerance {
                            slip_power_tolerance: 1.0e-12,
                            positive_power_tolerance: tolerance,
                        },
                    )
                    .is_err()
            );
        }
    }

    #[test]
    fn support_is_positive_homogeneous() {
        let patch = model();
        let base = patch
            .support(WrenchDirection3 {
                components: [1.0, -2.0, 0.5],
            })
            .expect("base")
            .support_value;
        let scaled = patch
            .support(WrenchDirection3 {
                components: [7.0, -14.0, 3.5],
            })
            .expect("scaled")
            .support_value;
        close(scaled, 7.0 * base);
    }
}
