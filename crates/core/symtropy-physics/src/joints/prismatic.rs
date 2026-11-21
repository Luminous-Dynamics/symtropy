// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Prismatic joint (slider): constrains motion to a single axis.
//!
//! Removes D-1 translational DOF and all rotational DOF, leaving 1 translational DOF.
//! Uses: suspension springs, elevator platforms, sliding doors, linear actuators.

#[cfg(test)]
use nalgebra::SVector;

use crate::body::{BodyHandle, RigidBody, RotationalInertiaModel};
use crate::constraint::Constraint;
use symtropy_math::Bivector;

const MAX_PRISMATIC_ROTATIONAL_DOF: usize = 36;

/// Why a passive prismatic angular lock declined to change body state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrismaticAngularLockRejection {
    InvalidStiffness,
    NonFiniteState,
    UnsupportedInertiaModel,
    NonFiniteResponse,
    NonSymmetricResponse,
    SingularDelassus,
    IllConditionedDelassus,
    SolveResidualTooLarge,
    UnrepresentableResult,
}

/// Observable result of the passive prismatic angular-lock solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PrismaticAngularLockOutcome {
    NoOp,
    Applied { predicted_residual_inf: f64 },
    Rejected(PrismaticAngularLockRejection),
}

fn has_admitted_angular_response<const D: usize>(body: &RigidBody<D>) -> bool {
    if !body.is_dynamic() {
        return true;
    }
    match body.rotational_inertia_model() {
        RotationalInertiaModel::ValidatedSpatialBivectorOperator => true,
        RotationalInertiaModel::PrincipalAxesWorldSpace3D => D == 3,
        _ => false,
    }
}

/// Assemble and solve the angular Delassus system without mutating either body.
/// The matrix and factorization live on the stack (at most 36 rotational DOF).
fn try_prismatic_angular_lock_impulse<const D: usize>(
    body_a: &RigidBody<D>,
    body_b: &RigidBody<D>,
    stiffness: f64,
) -> Result<Option<(Bivector<D>, f64)>, PrismaticAngularLockRejection> {
    if !stiffness.is_finite() || !(0.0..=1.0).contains(&stiffness) {
        return Err(PrismaticAngularLockRejection::InvalidStiffness);
    }
    if stiffness == 0.0 {
        return Ok(None);
    }
    if !body_a.angular_velocity.is_finite() || !body_b.angular_velocity.is_finite() {
        return Err(PrismaticAngularLockRejection::NonFiniteState);
    }
    if !has_admitted_angular_response(body_a) || !has_admitted_angular_response(body_b) {
        return Err(PrismaticAngularLockRejection::UnsupportedInertiaModel);
    }

    let mut relative = Bivector::<D>::zero();
    let mut relative_scale = 0.0_f64;
    for i in 0..D {
        for j in (i + 1)..D {
            let value = body_b.angular_velocity.get(i, j) - body_a.angular_velocity.get(i, j);
            if !value.is_finite() {
                return Err(PrismaticAngularLockRejection::NonFiniteState);
            }
            relative.set(i, j, value);
            relative_scale = relative_scale.max(value.abs());
        }
    }
    if relative_scale == 0.0 {
        return Ok(None);
    }

    let dof = D.saturating_mul(D.saturating_sub(1)) / 2;
    if dof == 0 || dof > MAX_PRISMATIC_ROTATIONAL_DOF {
        return Err(PrismaticAngularLockRejection::UnsupportedInertiaModel);
    }

    let mut planes = [(0usize, 1usize); MAX_PRISMATIC_ROTATIONAL_DOF];
    let mut plane_count = 0;
    for i in 0..D {
        for j in (i + 1)..D {
            planes[plane_count] = (i, j);
            plane_count += 1;
        }
    }
    if plane_count != dof {
        return Err(PrismaticAngularLockRejection::UnsupportedInertiaModel);
    }

    // W = I_a^-1 + I_b^-1 in the canonical lexicographic bivector basis.
    let mut delassus = [[0.0_f64; MAX_PRISMATIC_ROTATIONAL_DOF]; MAX_PRISMATIC_ROTATIONAL_DOF];
    for column in 0..dof {
        let (basis_i, basis_j) = planes[column];
        let mut basis = Bivector::<D>::zero();
        basis.set(basis_i, basis_j, 1.0);
        let response_a = body_a.apply_inverse_angular_inertia(&basis);
        let response_b = body_b.apply_inverse_angular_inertia(&basis);
        if !response_a.is_finite() || !response_b.is_finite() {
            return Err(PrismaticAngularLockRejection::NonFiniteResponse);
        }
        // Body response helpers fail closed to zero when a frame/operator
        // evaluation fails. Do not accidentally reinterpret that silent
        // sentinel as a valid dynamic endpoint with zero mobility.
        if (body_a.is_dynamic() && response_a.get(basis_i, basis_j) <= 0.0)
            || (body_b.is_dynamic() && response_b.get(basis_i, basis_j) <= 0.0)
        {
            return Err(PrismaticAngularLockRejection::UnsupportedInertiaModel);
        }
        for row in 0..dof {
            let (row_i, row_j) = planes[row];
            let value = response_a.get(row_i, row_j) + response_b.get(row_i, row_j);
            if !value.is_finite() {
                return Err(PrismaticAngularLockRejection::NonFiniteResponse);
            }
            delassus[row][column] = value;
        }
    }

    let mut matrix_scale = 0.0_f64;
    for row in 0..dof {
        for column in 0..dof {
            matrix_scale = matrix_scale.max(delassus[row][column].abs());
        }
    }
    if !matrix_scale.is_finite() || matrix_scale <= 0.0 {
        return Err(PrismaticAngularLockRejection::SingularDelassus);
    }
    let symmetry_tolerance = 1.0e-10 * matrix_scale;
    for row in 0..dof {
        for column in 0..row {
            if (delassus[row][column] - delassus[column][row]).abs()
                > symmetry_tolerance
            {
                return Err(PrismaticAngularLockRejection::NonSymmetricResponse);
            }
        }
    }

    // Scale-aware Cholesky. Tiny pivots are rejected rather than silently
    // regularized, because regularization would change the declared lock.
    let mut lower = [[0.0_f64; MAX_PRISMATIC_ROTATIONAL_DOF]; MAX_PRISMATIC_ROTATIONAL_DOF];
    for row in 0..dof {
        for column in 0..=row {
            let mut pivot = delassus[row][column];
            for inner in 0..column {
                pivot -= lower[row][inner] * lower[column][inner];
            }
            if !pivot.is_finite() {
                return Err(PrismaticAngularLockRejection::NonFiniteResponse);
            }
            if row == column {
                if pivot <= 0.0 {
                    return Err(PrismaticAngularLockRejection::SingularDelassus);
                }
                if pivot <= 1.0e-12 * matrix_scale {
                    return Err(PrismaticAngularLockRejection::IllConditionedDelassus);
                }
                lower[row][column] = pivot.sqrt();
            } else {
                let diagonal = lower[column][column];
                if !diagonal.is_finite() || diagonal <= 0.0 {
                    return Err(PrismaticAngularLockRejection::SingularDelassus);
                }
                lower[row][column] = pivot / diagonal;
            }
        }
    }

    // Solve W * lambda = stiffness * (omega_b - omega_a).
    let mut rhs = [0.0_f64; MAX_PRISMATIC_ROTATIONAL_DOF];
    let mut forward = [0.0_f64; MAX_PRISMATIC_ROTATIONAL_DOF];
    let mut solution = [0.0_f64; MAX_PRISMATIC_ROTATIONAL_DOF];
    let mut rhs_inf = 0.0_f64;
    for index in 0..dof {
        let (i, j) = planes[index];
        rhs[index] = stiffness * relative.get(i, j);
        if !rhs[index].is_finite() {
            return Err(PrismaticAngularLockRejection::NonFiniteResponse);
        }
        rhs_inf = rhs_inf.max(rhs[index].abs());
    }
    for row in 0..dof {
        let mut value = rhs[row];
        for column in 0..row {
            value -= lower[row][column] * forward[column];
        }
        forward[row] = value / lower[row][row];
        if !forward[row].is_finite() {
            return Err(PrismaticAngularLockRejection::NonFiniteResponse);
        }
    }
    for row in (0..dof).rev() {
        let mut value = forward[row];
        for column in (row + 1)..dof {
            value -= lower[column][row] * solution[column];
        }
        solution[row] = value / lower[row][row];
        if !solution[row].is_finite() {
            return Err(PrismaticAngularLockRejection::NonFiniteResponse);
        }
    }

    // Verify the unregularized linear solve before any body mutation.
    let mut matrix_inf = 0.0_f64;
    let mut solution_inf = 0.0_f64;
    let mut solve_residual_inf = 0.0_f64;
    for row in 0..dof {
        let mut row_sum = 0.0_f64;
        let mut reconstructed = 0.0_f64;
        for column in 0..dof {
            row_sum += delassus[row][column].abs();
            reconstructed += delassus[row][column] * solution[column];
        }
        if !row_sum.is_finite() || !reconstructed.is_finite() {
            return Err(PrismaticAngularLockRejection::NonFiniteResponse);
        }
        matrix_inf = matrix_inf.max(row_sum);
        solution_inf = solution_inf.max(solution[row].abs());
        solve_residual_inf = solve_residual_inf.max((reconstructed - rhs[row]).abs());
    }
    let solve_scale = matrix_inf * solution_inf + rhs_inf;
    if !solve_scale.is_finite() || solve_scale <= 0.0 {
        return Err(PrismaticAngularLockRejection::NonFiniteResponse);
    }
    if !solve_residual_inf.is_finite() || solve_residual_inf > 1.0e-10 * solve_scale {
        return Err(PrismaticAngularLockRejection::SolveResidualTooLarge);
    }

    let mut impulse = Bivector::<D>::zero();
    for index in 0..dof {
        let (i, j) = planes[index];
        impulse.set(i, j, solution[index]);
    }
    let impulse_b = impulse.scale(-1.0);

    // Predict both endpoint responses with the same body operators used by
    // apply_angular_impulse. Reject before mutation if any result is invalid.
    let delta_a = body_a.apply_inverse_angular_inertia(&impulse);
    let delta_b = body_b.apply_inverse_angular_inertia(&impulse_b);
    if !delta_a.is_finite() || !delta_b.is_finite() {
        return Err(PrismaticAngularLockRejection::NonFiniteResponse);
    }
    let next_a = body_a.angular_velocity.add(&delta_a);
    let next_b = body_b.angular_velocity.add(&delta_b);
    if !next_a.is_finite() || !next_b.is_finite() {
        return Err(PrismaticAngularLockRejection::UnrepresentableResult);
    }

    let mut predicted_residual_inf = 0.0_f64;
    for (i, j) in planes.iter().take(dof) {
        let actual = next_b.get(*i, *j) - next_a.get(*i, *j);
        let target = (1.0 - stiffness) * relative.get(*i, *j);
        if !actual.is_finite() || !target.is_finite() {
            return Err(PrismaticAngularLockRejection::UnrepresentableResult);
        }
        predicted_residual_inf = predicted_residual_inf.max((actual - target).abs());
    }
    if predicted_residual_inf > 1.0e-9 * relative_scale.max(f64::MIN_POSITIVE) {
        return Err(PrismaticAngularLockRejection::SolveResidualTooLarge);
    }

    Ok(Some((impulse, predicted_residual_inf)))
}

/// Prismatic joint: body B slides along one axis relative to body A.
pub struct PrismaticJoint<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    /// The axis along which sliding is permitted (0=X, 1=Y, 2=Z, ...).
    pub axis: usize,
    /// Optional displacement limits (min, max) relative to the initial offset.
    pub limits: Option<(f64, f64)>,
    /// Constraint stiffness [0, 1]. 1.0 = rigid.
    pub stiffness: f64,
    /// Optional motor drive for actuated prismatic joints.
    pub motor: Option<super::MotorDrive>,
}

impl<const D: usize> PrismaticJoint<D> {
    /// Create a prismatic joint along the given axis.
    pub fn new(body_a: BodyHandle, body_b: BodyHandle, axis: usize) -> Self {
        debug_assert!(axis < D, "axis {axis} out of range for D={D}");
        Self {
            body_a,
            body_b,
            axis,
            limits: None,
            stiffness: 1.0,
            motor: None,
        }
    }

    /// Add displacement limits.
    pub fn with_limits(mut self, min: f64, max: f64) -> Self {
        self.limits = Some((min, max));
        self
    }

    /// Add a motor drive.
    pub fn with_motor(mut self, motor: super::MotorDrive) -> Self {
        self.motor = Some(motor);
        self
    }

    /// Solve the passive rotational lock in the combined kinetic metric.
    ///
    /// A common world-space angular impulse solves
    /// (I_a^-1 + I_b^-1) * lambda = stiffness * (omega_b - omega_a).
    /// Static/kinematic endpoints contribute zero response. Unsupported,
    /// invalid, singular or ill-conditioned models are rejected without
    /// changing either body.
    pub fn solve_angular_velocity_lock(
        &self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
    ) -> PrismaticAngularLockOutcome {
        match try_prismatic_angular_lock_impulse(body_a, body_b, self.stiffness) {
            Ok(None) => PrismaticAngularLockOutcome::NoOp,
            Err(reason) => PrismaticAngularLockOutcome::Rejected(reason),
            Ok(Some((impulse, predicted_residual_inf))) => {
                crate::integrator::apply_angular_impulse(body_a, &impulse);
                crate::integrator::apply_angular_impulse(body_b, &impulse.scale(-1.0));
                PrismaticAngularLockOutcome::Applied {
                    predicted_residual_inf,
                }
            }
        }
    }
}

impl<const D: usize> Constraint<D> for PrismaticJoint<D> {
    fn bodies(&self) -> (BodyHandle, BodyHandle) {
        (self.body_a, self.body_b)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn solve(&self, body_a: &mut RigidBody<D>, body_b: &mut RigidBody<D>, _dt: f64) {
        let delta = body_b.transform.translation.0 - body_a.transform.translation.0;

        // Correct all axes EXCEPT the slide axis
        let total_inv_mass = body_a.inv_mass + body_b.inv_mass;
        if total_inv_mass < 1e-15 {
            return;
        }

        let ratio_a = body_a.inv_mass / total_inv_mass;
        let ratio_b = body_b.inv_mass / total_inv_mass;

        for i in 0..D {
            if i == self.axis {
                // Apply limits on the slide axis
                if let Some((min, max)) = self.limits {
                    let displacement = delta[i];
                    if displacement < min {
                        let correction = (min - displacement) * self.stiffness;
                        if body_a.is_dynamic() {
                            body_a.transform.translation.0[i] -= correction * ratio_a;
                        }
                        if body_b.is_dynamic() {
                            body_b.transform.translation.0[i] += correction * ratio_b;
                        }
                    } else if displacement > max {
                        let correction = (displacement - max) * self.stiffness;
                        if body_a.is_dynamic() {
                            body_a.transform.translation.0[i] += correction * ratio_a;
                        }
                        if body_b.is_dynamic() {
                            body_b.transform.translation.0[i] -= correction * ratio_b;
                        }
                    }
                }
                continue;
            }

            // Constrain non-slide axes to zero displacement
            let error = delta[i];
            if error.abs() < 1e-15 {
                continue;
            }
            let correction = error * self.stiffness;
            if body_a.is_dynamic() {
                body_a.transform.translation.0[i] += correction * ratio_a;
            }
            if body_b.is_dynamic() {
                body_b.transform.translation.0[i] -= correction * ratio_b;
            }
        }
    }

    fn solve_velocity(
        &self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        dt: f64,
        mut callback: Option<&mut dyn crate::world::PhysicsCallback<D>>,
    ) {
        let total_inv = body_a.inv_mass + body_b.inv_mass;
        if total_inv < 1e-15 {
            return;
        }

        // Damp velocity on non-slide axes
        for i in 0..D {
            if i == self.axis {
                continue;
            }
            let rel_vel = body_b.linear_velocity[i] - body_a.linear_velocity[i];
            if rel_vel.abs() < 1e-15 {
                continue;
            }
            let impulse = -rel_vel * self.stiffness / total_inv;
            if body_a.is_dynamic() {
                body_a.linear_velocity[i] -= impulse * body_a.inv_mass;
            }
            if body_b.is_dynamic() {
                body_b.linear_velocity[i] += impulse * body_b.inv_mass;
            }
        }

        // Motor drive on slide axis
        if let Some(ref motor) = self.motor {
            let displacement = body_b.transform.translation.0[self.axis]
                - body_a.transform.translation.0[self.axis];
            let rel_vel = body_b.linear_velocity[self.axis] - body_a.linear_velocity[self.axis];

            // PD controller: impulse = (kp * (target - pos) + kd * (target_vel - vel)) * dt
            let impulse = motor.calculate_impulse(displacement, rel_vel, dt);

            // METABOLIC HEARTBEAT: Calculate Work and Energy Recovery
            // Work = Impulse * Velocity
            let work_joules = impulse * rel_vel;
            if let Some(ref mut cb) = callback {
                cb.record_work(self.body_b, work_joules);
            }

            if body_a.is_dynamic() {
                body_a.linear_velocity[self.axis] -= impulse * body_a.inv_mass;
            }
            if body_b.is_dynamic() {
                body_b.linear_velocity[self.axis] += impulse * body_b.inv_mass;
            }
        }

        // Passive rotational lock uses the combined inertia metric rather than
        // editing angular-velocity coordinates independently.
        let _lock_outcome = self.solve_angular_velocity_lock(body_a, body_b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;

    #[test]
    fn slides_along_axis() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([2.0, 3.0, 0.0]), 0.5, 1.0);

        // Prismatic along X — Y should be corrected to 0, X preserved
        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0);

        for _ in 0..20 {
            joint.solve(&mut a, &mut b, 0.016);
        }

        // Y should be near 0 (constrained)
        assert!(
            b.transform.translation.coord(1).abs() < 0.1,
            "Y should be constrained to 0, got {}",
            b.transform.translation.coord(1)
        );
        // X should be preserved (free axis)
        assert!(
            b.transform.translation.coord(0).abs() > 0.5,
            "X should be free, got {}",
            b.transform.translation.coord(0)
        );
    }

    #[test]
    fn limits_respected() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([10.0, 0.0, 0.0]), 0.5, 1.0);

        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0).with_limits(-2.0, 5.0);

        for _ in 0..30 {
            joint.solve(&mut a, &mut b, 0.016);
        }

        let x = b.transform.translation.coord(0);
        assert!(x <= 5.5, "X should be clamped to max 5.0, got {x}");
    }

    #[test]
    fn perpendicular_velocity_damped() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b = RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        b.linear_velocity = SVector::from([0.0, 10.0, 5.0]); // perpendicular velocity

        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0);
        joint.solve_velocity(&mut a, &mut b, 0.016, None);

        assert!(
            b.linear_velocity[1].abs() < 10.0,
            "Y velocity should be damped, got {}",
            b.linear_velocity[1]
        );
    }

    fn static_body_3() -> RigidBody<3> {
        RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        )
    }

    fn dynamic_with_full_inertia(handle: usize, matrix: Vec<f64>) -> RigidBody<3> {
        let operator = crate::mass_properties::RotationalInertiaOperator::<3>::try_from_row_major(
            matrix,
        )
        .expect("test inertia operator must be SPD");
        RigidBody::<3>::dynamic_sphere(BodyHandle(handle), Point::origin(), 0.2, 1.0)
            .with_rotational_inertia_operator(operator)
    }

    #[test]
    fn static_base_absorbs_the_entire_prismatic_angular_lock() {
        let mut a = static_body_3();
        let mut b = RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.2, 1.0);
        b.angular_velocity.set(0, 1, 0.7);
        b.angular_velocity.set(0, 2, -0.4);
        b.angular_velocity.set(1, 2, 0.2);

        let outcome = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0)
            .solve_angular_velocity_lock(&mut a, &mut b);
        assert!(matches!(outcome, PrismaticAngularLockOutcome::Applied { .. }));
        assert!(
            b.angular_velocity.norm() < 1.0e-9,
            "a static base must not halve the dynamic body's angular correction"
        );
    }

    #[test]
    fn unequal_off_diagonal_inertias_obey_stiffness_residual_and_conserve_momentum() {
        let matrix_a = vec![4.0, 0.8, 0.0, 0.8, 3.0, 0.35, 0.0, 0.35, 2.0];
        let matrix_b = vec![2.5, -0.3, 0.1, -0.3, 5.0, 0.6, 0.1, 0.6, 3.5];
        let operator_a = crate::mass_properties::RotationalInertiaOperator::<3>::try_from_row_major(
            matrix_a,
        )
        .expect("test inertia A must be SPD");
        let operator_b = crate::mass_properties::RotationalInertiaOperator::<3>::try_from_row_major(
            matrix_b,
        )
        .expect("test inertia B must be SPD");
        let mut a = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.2, 1.0)
            .with_rotational_inertia_operator(operator_a.clone());
        let mut b = RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.2, 1.0)
            .with_rotational_inertia_operator(operator_b.clone());
        a.angular_velocity.set(0, 1, -0.1);
        a.angular_velocity.set(0, 2, 0.2);
        a.angular_velocity.set(1, 2, -0.3);
        b.angular_velocity.set(0, 1, 0.6);
        b.angular_velocity.set(0, 2, -0.4);
        b.angular_velocity.set(1, 2, 0.5);
        let before_a = a.angular_velocity;
        let before_b = b.angular_velocity;
        let mut before = Bivector::<3>::zero();
        for i in 0..3 {
            for j in (i + 1)..3 {
                before.set(
                    i,
                    j,
                    b.angular_velocity.get(i, j) - a.angular_velocity.get(i, j),
                );
            }
        }

        let mut joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0);
        joint.stiffness = 0.25;
        let outcome = joint.solve_angular_velocity_lock(&mut a, &mut b);
        assert!(matches!(outcome, PrismaticAngularLockOutcome::Applied { .. }));
        for i in 0..3 {
            for j in (i + 1)..3 {
                let after = b.angular_velocity.get(i, j) - a.angular_velocity.get(i, j);
                assert!(
                    (after - 0.75 * before.get(i, j)).abs() < 1.0e-9,
                    "plane ({i},{j}) did not retain the declared 75% residual"
                );
            }
        }

        let delta_a = a.angular_velocity.add(&before_a.scale(-1.0));
        let delta_b = b.angular_velocity.add(&before_b.scale(-1.0));
        let momentum_delta_a = operator_a
            .apply_bivector(&delta_a)
            .expect("finite angular momentum change for A");
        let momentum_delta_b = operator_b
            .apply_bivector(&delta_b)
            .expect("finite angular momentum change for B");
        for i in 0..3 {
            for j in (i + 1)..3 {
                assert!(
                    (momentum_delta_a.get(i, j) + momentum_delta_b.get(i, j)).abs() < 1.0e-9,
                    "equal-and-opposite angular impulse must conserve total angular momentum"
                );
            }
        }
    }

    #[test]
    fn ill_conditioned_delassus_rejects_without_mutation() {
        let matrix = vec![
            1.0e13, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        let mut a = dynamic_with_full_inertia(0, matrix.clone());
        let mut b = dynamic_with_full_inertia(1, matrix);
        a.angular_velocity.set(0, 1, 0.4);
        b.angular_velocity.set(1, 2, -0.7);
        let before_a = a.angular_velocity;
        let before_b = b.angular_velocity;

        let outcome = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0)
            .solve_angular_velocity_lock(&mut a, &mut b);
        assert_eq!(
            outcome,
            PrismaticAngularLockOutcome::Rejected(
                PrismaticAngularLockRejection::IllConditionedDelassus
            )
        );
        assert_eq!(a.angular_velocity, before_a);
        assert_eq!(b.angular_velocity, before_b);
    }

    fn dynamic_with_full_inertia_4d(handle: usize) -> RigidBody<4> {
        let mut matrix = vec![0.0; 36];
        for index in 0..6 {
            matrix[index * 6 + index] = 2.0 + index as f64;
        }
        matrix[3] = 0.2;
        matrix[18] = 0.2;
        let operator = crate::mass_properties::RotationalInertiaOperator::<4>::try_from_row_major(
            matrix,
        )
        .expect("test 4D inertia operator must be SPD");
        RigidBody::<4>::dynamic_sphere(BodyHandle(handle), Point::origin(), 0.2, 1.0)
            .with_rotational_inertia_operator(operator)
    }

    #[test]
    fn full_four_dimensional_operator_solves_all_six_planes() {
        let mut a = dynamic_with_full_inertia_4d(0);
        let mut b = dynamic_with_full_inertia_4d(1);
        a.angular_velocity.set(0, 1, -0.1);
        a.angular_velocity.set(0, 3, 0.2);
        a.angular_velocity.set(1, 2, -0.3);
        b.angular_velocity.set(0, 1, 0.6);
        b.angular_velocity.set(0, 3, -0.4);
        b.angular_velocity.set(1, 2, 0.5);
        b.angular_velocity.set(1, 3, 0.3);
        let before = b.angular_velocity.add(&a.angular_velocity.scale(-1.0));

        let mut joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0);
        joint.stiffness = 0.5;
        let outcome = joint.solve_angular_velocity_lock(&mut a, &mut b);
        assert!(matches!(outcome, PrismaticAngularLockOutcome::Applied { .. }));
        for i in 0..4 {
            for j in (i + 1)..4 {
                let after = b.angular_velocity.get(i, j) - a.angular_velocity.get(i, j);
                assert!(
                    (after - 0.5 * before.get(i, j)).abs() < 1.0e-9,
                    "4D plane ({i},{j}) did not retain the declared 50% residual"
                );
            }
        }
    }

    #[test]
    fn higher_dimensional_mean_inertia_is_not_silently_admitted() {
        let mut a = RigidBody::<4>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.2, 1.0);
        let mut b = RigidBody::<4>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.2, 1.0);
        b.angular_velocity.set(0, 1, 0.5);
        let before_a = a.angular_velocity;
        let before_b = b.angular_velocity;

        let outcome = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0)
            .solve_angular_velocity_lock(&mut a, &mut b);
        assert_eq!(
            outcome,
            PrismaticAngularLockOutcome::Rejected(
                PrismaticAngularLockRejection::UnsupportedInertiaModel
            )
        );
        assert_eq!(a.angular_velocity, before_a);
        assert_eq!(b.angular_velocity, before_b);
    }

    #[test]
    fn prismatic_4d() {
        let mut a = RigidBody::<4>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        let mut b = RigidBody::<4>::dynamic_sphere(
            BodyHandle(1),
            Point::new([0.0, 0.0, 0.0, 5.0]),
            0.5,
            1.0,
        );

        // Slide along W axis
        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 3);

        for _ in 0..20 {
            joint.solve(&mut a, &mut b, 0.016);
        }

        // W should be free, others constrained near 0
        assert!(
            b.transform.translation.coord(0).abs() < 0.1,
            "X should be constrained"
        );
    }
}
