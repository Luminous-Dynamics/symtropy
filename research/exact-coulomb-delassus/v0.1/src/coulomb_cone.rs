#![forbid(unsafe_code)]

//! Euclidean Coulomb-cone projection and De Saxce/Feng local-law residuals.
//!
//! This module checks one 3D contact in coordinates (normal, tangent_1,
//! tangent_2). It does not solve an island, find all physical branches, or
//! establish convergence of an outer solver.

use std::error::Error;
use std::fmt;

const DEFAULT_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactImpulse3 {
    pub normal: f64,
    pub tangent: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactVelocity3 {
    pub normal: f64,
    pub tangent: [f64; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionRegion {
    Interior,
    Boundary,
    Apex,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConeProjection {
    pub point: ContactImpulse3,
    pub region: ProjectionRegion,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactLawResiduals {
    /// Normalized maximum of negative normal impulse and friction-disk excess.
    pub primal_cone_violation: f64,
    /// Normalized violation of the dual cone for the augmented velocity.
    pub dual_cone_violation: f64,
    /// Complementarity residual normalized by impulse and velocity scales.
    pub complementarity: f64,
    /// Signed kinetic-energy change for the supplied pre/post contact velocities.
    pub kinetic_energy_delta: f64,
    /// Normalized positive energy injection (zero for non-injecting states).
    pub energy_injection: f64,
    /// Maximum of the four normalized qualification residuals.
    pub max_normalized_residual: f64,
    pub augmented_velocity: ContactVelocity3,
}

impl ContactLawResiduals {
    #[must_use]
    pub fn qualifies(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance > 0.0
            && self.max_normalized_residual <= tolerance
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoulombConeError {
    InvalidFrictionCoefficient,
    NonFiniteInput,
    NonFiniteResult,
    InvalidTolerance,
}

impl fmt::Display for CoulombConeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for CoulombConeError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoulombCone {
    friction_coefficient: f64,
}

impl CoulombCone {
    pub fn new(friction_coefficient: f64) -> Result<Self, CoulombConeError> {
        if !friction_coefficient.is_finite() {
            return Err(CoulombConeError::NonFiniteInput);
        }
        if friction_coefficient < 0.0 {
            return Err(CoulombConeError::InvalidFrictionCoefficient);
        }
        Ok(Self {
            friction_coefficient,
        })
    }

    #[must_use]
    pub fn friction_coefficient(self) -> f64 {
        self.friction_coefficient
    }

    /// Project onto K_mu = {(n,t): n >= 0, ||t|| <= mu*n} in the ordinary
    /// Euclidean metric. The formula is the exact circular-cone projection.
    pub fn project(
        self,
        input: ContactImpulse3,
    ) -> Result<ConeProjection, CoulombConeError> {
        if !finite_impulse(input) {
            return Err(CoulombConeError::NonFiniteInput);
        }

        let mu = self.friction_coefficient;
        let radius = norm2(input.tangent);

        if mu == 0.0 {
            if input.normal <= 0.0 {
                return Ok(ConeProjection {
                    point: zero_impulse(),
                    region: ProjectionRegion::Apex,
                });
            }
            return Ok(ConeProjection {
                point: ContactImpulse3 {
                    normal: input.normal,
                    tangent: [0.0; 2],
                },
                region: ProjectionRegion::Boundary,
            });
        }

        if input.normal >= 0.0 {
            let inside = if mu <= 1.0 {
                radius <= mu * input.normal
            } else {
                radius / mu <= input.normal
            };
            if inside {
                let boundary = if mu <= 1.0 {
                    radius == mu * input.normal
                } else {
                    radius / mu == input.normal
                };
                return Ok(ConeProjection {
                    point: input,
                    region: if boundary {
                        ProjectionRegion::Boundary
                    } else {
                        ProjectionRegion::Interior
                    },
                });
            }
        }

        // The unconstrained radial boundary minimizer is at
        // n* = (n + mu*r)/(1 + mu^2). If its normal coordinate is <= 0,
        // the projection is the cone apex. Use a scaled comparison for mu > 1.
        let projects_to_apex = if mu <= 1.0 {
            input.normal <= -(mu * radius)
        } else {
            input.normal / mu <= -radius
        };
        if projects_to_apex {
            return Ok(ConeProjection {
                point: zero_impulse(),
                region: ProjectionRegion::Apex,
            });
        }

        // Evaluate the same closed form without squaring a large mu.
        let projected_normal = if mu <= 1.0 {
            let denominator = 1.0 + mu * mu;
            input.normal / denominator + radius * (mu / denominator)
        } else {
            let inverse_mu = 1.0 / mu;
            let inverse_squared = inverse_mu * inverse_mu;
            let denominator = 1.0 + inverse_squared;
            input.normal * (inverse_squared / denominator)
                + radius * (inverse_mu / denominator)
        };

        let projected_radius = if mu <= 1.0 {
            mu * projected_normal
        } else {
            projected_normal / (1.0 / mu)
        };
        let direction_scale = if radius == 0.0 {
            0.0
        } else {
            projected_radius / radius
        };
        let point = ContactImpulse3 {
            normal: projected_normal,
            tangent: [
                input.tangent[0] * direction_scale,
                input.tangent[1] * direction_scale,
            ],
        };

        if !finite_impulse(point) {
            return Err(CoulombConeError::NonFiniteResult);
        }
        Ok(ConeProjection {
            point,
            region: ProjectionRegion::Boundary,
        })
    }

    /// Evaluate the exact reduced local-law conditions for a supplied impulse
    /// and pre/post contact velocities. The caller MUST supply
    /// post_velocity = free_velocity + W * impulse for the same symmetric
    /// positive-semidefinite Delassus operator used by the solver. This module
    /// checks the resulting local law; it cannot independently verify that
    /// operator-response identity because W is not an input here.
    ///
    /// The De Saxce/Feng augmented velocity is
    /// u_tilde = (u_n + mu*||u_t||, u_t). Qualification requires:
    /// p in K_mu, u_tilde in K_mu*, p dot u_tilde = 0, and no positive
    /// kinetic-energy change beyond the declared normalized tolerance.
    pub fn evaluate_contact(
        self,
        impulse: ContactImpulse3,
        free_velocity: ContactVelocity3,
        post_velocity: ContactVelocity3,
    ) -> Result<ContactLawResiduals, CoulombConeError> {
        if !finite_impulse(impulse)
            || !finite_velocity(free_velocity)
            || !finite_velocity(post_velocity)
        {
            return Err(CoulombConeError::NonFiniteInput);
        }

        let mu = self.friction_coefficient;
        let tangent_impulse_norm = norm2(impulse.tangent);
        let tangent_velocity_norm = norm2(post_velocity.tangent);
        let augmented_normal = post_velocity.normal + mu * tangent_velocity_norm;
        if !augmented_normal.is_finite() {
            return Err(CoulombConeError::NonFiniteResult);
        }

        let impulse_scale = 1.0_f64
            .max(impulse.normal.abs())
            .max(tangent_impulse_norm)
            .max(mu * impulse.normal.abs());
        let velocity_scale = 1.0_f64
            .max(post_velocity.normal.abs())
            .max(tangent_velocity_norm)
            .max(augmented_normal.abs())
            .max(mu * tangent_velocity_norm);
        if !impulse_scale.is_finite() || !velocity_scale.is_finite() {
            return Err(CoulombConeError::NonFiniteResult);
        }

        let scaled_normal_impulse = impulse.normal / impulse_scale;
        let scaled_tangent_impulse = tangent_impulse_norm / impulse_scale;
        let scaled_disk_radius = mu * scaled_normal_impulse;
        let primal_cone_violation = (-scaled_normal_impulse)
            .max(scaled_tangent_impulse - scaled_disk_radius)
            .max(0.0);

        let scaled_augmented_normal = augmented_normal / velocity_scale;
        let scaled_tangent_velocity = tangent_velocity_norm / velocity_scale;
        let dual_cone_violation =
            (mu * scaled_tangent_velocity - scaled_augmented_normal).max(0.0);

        let complementarity = (scaled_normal_impulse * scaled_augmented_normal
            + (impulse.tangent[0] / impulse_scale)
                * (post_velocity.tangent[0] / velocity_scale)
            + (impulse.tangent[1] / impulse_scale)
                * (post_velocity.tangent[1] / velocity_scale))
            .abs();

        // Delta K = 1/2 p^T(q + u), with u = q + Wp and symmetric W.
        // This equals p^T q + 1/2 p^T Wp, without reconstructing W.
        let half_pre_work = 0.5
            * (impulse.normal * free_velocity.normal
                + impulse.tangent[0] * free_velocity.tangent[0]
                + impulse.tangent[1] * free_velocity.tangent[1]);
        let half_post_work = 0.5
            * (impulse.normal * post_velocity.normal
                + impulse.tangent[0] * post_velocity.tangent[0]
                + impulse.tangent[1] * post_velocity.tangent[1]);
        let kinetic_energy_delta = half_pre_work + half_post_work;
        let energy_scale = 1.0_f64
            .max(half_pre_work.abs())
            .max(half_post_work.abs());
        if !kinetic_energy_delta.is_finite() || !energy_scale.is_finite() {
            return Err(CoulombConeError::NonFiniteResult);
        }
        let energy_injection = (kinetic_energy_delta / energy_scale).max(0.0);

        let augmented_velocity = ContactVelocity3 {
            normal: augmented_normal,
            tangent: post_velocity.tangent,
        };
        let max_normalized_residual = primal_cone_violation
            .max(dual_cone_violation)
            .max(complementarity)
            .max(energy_injection);

        Ok(ContactLawResiduals {
            primal_cone_violation,
            dual_cone_violation,
            complementarity,
            kinetic_energy_delta,
            energy_injection,
            max_normalized_residual,
            augmented_velocity,
        })
    }
}

fn zero_impulse() -> ContactImpulse3 {
    ContactImpulse3 {
        normal: 0.0,
        tangent: [0.0; 2],
    }
}

fn norm2(vector: [f64; 2]) -> f64 {
    vector[0].hypot(vector[1])
}

fn finite_impulse(value: ContactImpulse3) -> bool {
    value.normal.is_finite() && value.tangent.iter().all(|item| item.is_finite())
}

fn finite_velocity(value: ContactVelocity3) -> bool {
    value.normal.is_finite() && value.tangent.iter().all(|item| item.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impulse(normal: f64, tangent: [f64; 2]) -> ContactImpulse3 {
        ContactImpulse3 { normal, tangent }
    }

    fn velocity(normal: f64, tangent: [f64; 2]) -> ContactVelocity3 {
        ContactVelocity3 { normal, tangent }
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
        );
    }

    fn distance(left: ContactImpulse3, right: ContactImpulse3) -> f64 {
        (left.normal - right.normal)
            .hypot(left.tangent[0] - right.tangent[0])
            .hypot(left.tangent[1] - right.tangent[1])
    }

    #[test]
    fn projection_preserves_interior_and_boundary_points() {
        let cone = CoulombCone::new(0.5).unwrap();
        let interior = impulse(2.0, [0.2, 0.3]);
        let projected_interior = cone.project(interior).unwrap();
        assert_eq!(projected_interior.region, ProjectionRegion::Interior);
        assert_eq!(projected_interior.point, interior);

        let boundary = impulse(2.0, [0.6, 0.8]);
        let projected_boundary = cone.project(boundary).unwrap();
        assert_eq!(projected_boundary.region, ProjectionRegion::Boundary);
        assert_eq!(projected_boundary.point, boundary);
    }

    #[test]
    fn projection_of_outside_point_matches_closed_form() {
        let cone = CoulombCone::new(0.5).unwrap();
        let projected = cone.project(impulse(1.0, [2.0, 0.0])).unwrap();
        assert_eq!(projected.region, ProjectionRegion::Boundary);
        close(projected.point.normal, 1.6, 1.0e-14);
        close(projected.point.tangent[0], 0.8, 1.0e-14);
        close(projected.point.tangent[1], 0.0, 1.0e-14);
        close(
            norm2(projected.point.tangent),
            0.5 * projected.point.normal,
            1.0e-14,
        );
    }

    #[test]
    fn projection_to_apex_and_zero_friction_are_explicit() {
        let cone = CoulombCone::new(0.5).unwrap();
        let apex = cone.project(impulse(-2.0, [0.1, 0.0])).unwrap();
        assert_eq!(apex.region, ProjectionRegion::Apex);
        assert_eq!(apex.point, zero_impulse());

        let frictionless = CoulombCone::new(0.0).unwrap();
        let ray = frictionless.project(impulse(2.0, [1.0, -3.0])).unwrap();
        assert_eq!(ray.region, ProjectionRegion::Boundary);
        assert_eq!(ray.point, impulse(2.0, [0.0, 0.0]));
        let zero = frictionless.project(impulse(-1.0, [0.0, 0.0])).unwrap();
        assert_eq!(zero.region, ProjectionRegion::Apex);
        assert_eq!(zero.point, zero_impulse());
    }

    #[test]
    fn projection_is_idempotent_and_nonexpansive_on_frozen_panel() {
        let cone = CoulombCone::new(0.7).unwrap();
        let panel = [
            impulse(-3.0, [0.2, -0.4]),
            impulse(-0.5, [2.0, 1.0]),
            impulse(0.0, [0.0, 0.0]),
            impulse(0.5, [0.1, 0.1]),
            impulse(1.0, [2.0, 0.0]),
            impulse(5.0, [-1.0, 0.8]),
            impulse(2.0, [1.0, 1.0]),
        ];
        let projected = panel
            .iter()
            .map(|point| cone.project(*point).unwrap().point)
            .collect::<Vec<_>>();
        for point in &projected {
            let twice = cone.project(*point).unwrap().point;
            close(distance(*point, twice), 0.0, 1.0e-12);
        }
        for left in 0..panel.len() {
            for right in 0..panel.len() {
                let before = distance(panel[left], panel[right]);
                let after = distance(projected[left], projected[right]);
                assert!(after <= before + 1.0e-10);
            }
        }
    }

    #[test]
    fn projection_is_equivariant_under_tangent_plane_rotation() {
        let cone = CoulombCone::new(0.6).unwrap();
        let point = impulse(-0.2, [1.3, -0.7]);
        let angle: f64 = 0.83;
        let cosine = angle.cos();
        let sine = angle.sin();
        let rotate = |tangent: [f64; 2]| {
            [
                cosine * tangent[0] - sine * tangent[1],
                sine * tangent[0] + cosine * tangent[1],
            ]
        };
        let original_projection = cone.project(point).unwrap().point;
        let rotated_projection = cone
            .project(impulse(point.normal, rotate(point.tangent)))
            .unwrap()
            .point;
        let expected = impulse(original_projection.normal, rotate(original_projection.tangent));
        close(distance(rotated_projection, expected), 0.0, 1.0e-12);
    }

    #[test]
    fn separating_state_satisfies_augmented_dual_cone() {
        let cone = CoulombCone::new(0.5).unwrap();
        let report = cone
            .evaluate_contact(
                impulse(0.0, [0.0, 0.0]),
                velocity(1.0, [3.0, 4.0]),
                velocity(1.0, [3.0, 4.0]),
            )
            .unwrap();
        assert!(report.qualifies(DEFAULT_TOLERANCE));
        assert_eq!(report.augmented_velocity, velocity(3.5, [3.0, 4.0]));
        close(report.kinetic_energy_delta, 0.0, 1.0e-14);
    }

    #[test]
    fn sticking_state_satisfies_primal_dual_and_complementarity() {
        let cone = CoulombCone::new(0.5).unwrap();
        let report = cone
            .evaluate_contact(
                impulse(2.0, [0.3, 0.4]),
                velocity(-2.0, [-0.3, -0.4]),
                velocity(0.0, [0.0, 0.0]),
            )
            .unwrap();
        assert!(report.qualifies(DEFAULT_TOLERANCE));
        assert_eq!(report.primal_cone_violation, 0.0);
        assert_eq!(report.dual_cone_violation, 0.0);
        assert_eq!(report.complementarity, 0.0);
        assert!(report.kinetic_energy_delta < 0.0);
    }

    #[test]
    fn sliding_state_uses_de_saxce_feng_augmented_velocity() {
        let cone = CoulombCone::new(0.5).unwrap();
        let report = cone
            .evaluate_contact(
                impulse(2.0, [-0.6, -0.8]),
                velocity(-2.0, [3.6, 4.8]),
                velocity(0.0, [3.0, 4.0]),
            )
            .unwrap();
        assert!(report.qualifies(DEFAULT_TOLERANCE));
        close(report.augmented_velocity.normal, 2.5, 1.0e-14);
        close(report.complementarity, 0.0, 1.0e-14);
        assert!(report.kinetic_energy_delta < 0.0);
    }

    #[test]
    fn wrong_sliding_direction_and_negative_normal_impulse_are_rejected() {
        let cone = CoulombCone::new(0.5).unwrap();
        let wrong_direction = cone
            .evaluate_contact(
                impulse(2.0, [0.6, 0.8]),
                velocity(-2.0, [2.4, 3.2]),
                velocity(0.0, [3.0, 4.0]),
            )
            .unwrap();
        assert!(!wrong_direction.qualifies(DEFAULT_TOLERANCE));
        assert!(wrong_direction.complementarity > 0.0);

        let negative_normal = cone
            .evaluate_contact(
                impulse(-1.0, [0.0, 0.0]),
                velocity(1.0, [0.0, 0.0]),
                velocity(1.0, [0.0, 0.0]),
            )
            .unwrap();
        assert!(!negative_normal.qualifies(DEFAULT_TOLERANCE));
        assert!(negative_normal.primal_cone_violation > 0.0);
    }

    #[test]
    fn friction_disk_violation_is_not_accepted() {
        let cone = CoulombCone::new(0.25).unwrap();
        let report = cone
            .evaluate_contact(
                impulse(1.0, [1.0, 0.0]),
                velocity(0.0, [0.0, 0.0]),
                velocity(0.0, [0.0, 0.0]),
            )
            .unwrap();
        assert!(!report.qualifies(DEFAULT_TOLERANCE));
        assert!(report.primal_cone_violation > 0.0);
    }

    #[test]
    fn invalid_inputs_and_overflow_fail_closed() {
        assert_eq!(
            CoulombCone::new(-0.1).err(),
            Some(CoulombConeError::InvalidFrictionCoefficient)
        );
        assert_eq!(
            CoulombCone::new(f64::NAN).err(),
            Some(CoulombConeError::NonFiniteInput)
        );

        let cone = CoulombCone::new(0.5).unwrap();
        assert_eq!(
            cone.project(impulse(f64::NAN, [0.0, 0.0])).err(),
            Some(CoulombConeError::NonFiniteInput)
        );
        assert_eq!(
            CoulombCone::new(1.0)
                .unwrap()
                .project(impulse(0.0, [f64::MAX, f64::MAX]))
                .err(),
            Some(CoulombConeError::NonFiniteResult)
        );
    }
}
