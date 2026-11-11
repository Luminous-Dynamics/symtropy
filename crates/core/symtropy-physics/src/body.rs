// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::{SMatrix, SVector};
use crate::mass_properties::RotationalInertiaOperator;
use serde::{Deserialize, Serialize};
use symtropy_math::{Bivector, Point, Shape, Transform};

/// Unique identifier for a body handle.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BodyHandle(pub usize);

/// Unique identifier for networked bodies.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NetId(pub u64);

/// Type of rigid body.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyType {
    Static,
    Kinematic,
    Dynamic,
}

/// Rotational inertia model used by one rigid body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationalInertiaModel {
    /// Static and kinematic bodies do not respond to applied impulses.
    StaticOrKinematic,
    /// 3D body-local principal moments rotated into the world frame.
    PrincipalAxesWorldSpace3D,
    /// Higher-dimensional compatibility path using a scalar mean inverse inertia.
    MeanInertiaApproximation,
    /// Invalid 3D principal moments or reciprocal values; angular response fails closed.
    InvalidPrincipalMoments,
    /// A validated full bivector inertia operator in body coordinates.
    ValidatedSpatialBivectorOperator,
}

/// N-dimensional rigid body.
pub struct RigidBody<const D: usize> {
    pub handle: BodyHandle,
    pub net_id: Option<NetId>,
    pub body_type: BodyType,
    pub transform: Transform<D>,
    pub linear_velocity: SVector<f64, D>,
    pub angular_velocity: Bivector<D>,
    pub mass: f64,
    pub inv_mass: f64,
    /// Principal moments in the body's local frame. In 3D the principal axes
    /// are aligned with the body's local X/Y/Z basis and are rotated into
    /// world space for angular dynamics. For D != 3 this remains a legacy
    /// per-axis compatibility vector used by the explicitly approximate path.
    pub inertia: SVector<f64, D>,
    /// Reciprocals of inertia; invalid source moments produce zero here and
    /// are exposed as InvalidPrincipalMoments by the 3D model census.
    pub inv_inertia: SVector<f64, D>,
    /// Optional full inertia operator over all D(D-1)/2 bivector coordinates.
    /// When present it supersedes the legacy per-axis inertia approximation.
    pub rotational_inertia_operator: Option<RotationalInertiaOperator<D>>,
    /// Set when a mass-distribution change invalidates the attached operator.
    pub rotational_inertia_operator_invalidated: bool,
    /// Mass value (f64 bits) when the full inertia operator was attached.
    /// Detects direct public mass mutation that bypasses the replacement API.
    pub rotational_inertia_operator_mass_bits: Option<u64>,
    /// True when compound mass properties use the legacy approximation.
    pub rotational_inertia_approximate: bool,
    pub collider: Box<dyn Shape<D>>,
    pub friction: f64,
    pub restitution: f64,
    pub linear_damping: f64,
    pub angular_damping: f64,
    pub force_accumulator: SVector<f64, D>,
    pub torque_accumulator: Bivector<D>,
    pub sleeping: bool,
    pub sleep_timer: f32,
    pub sleep_counter: u32,
    pub is_sensor: bool,
    pub collision_group: u32,
    pub collision_mask: u32,
}

impl<const D: usize> RigidBody<D> {
    pub fn new(
        handle: BodyHandle,
        body_type: BodyType,
        transform: Transform<D>,
        collider: Box<dyn Shape<D>>,
        mass: f64,
        inertia: SVector<f64, D>,
    ) -> Self {
        let inv_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
        let inv_inertia = inertia.map(|i| {
            if i.is_finite() && i > 0.0 {
                1.0 / i
            } else {
                0.0
            }
        });

        Self {
            handle,
            net_id: None,
            body_type,
            transform,
            linear_velocity: SVector::zeros(),
            angular_velocity: Bivector::zero(),
            mass,
            inv_mass,
            inertia,
            inv_inertia,
            rotational_inertia_operator: None,
            rotational_inertia_operator_invalidated: false,
            rotational_inertia_operator_mass_bits: None,
            rotational_inertia_approximate: false,
            collider,
            friction: 0.5,
            restitution: 0.2,
            linear_damping: 0.0,
            angular_damping: 0.0,
            force_accumulator: SVector::zeros(),
            torque_accumulator: Bivector::zero(),
            sleeping: false,
            sleep_timer: 0.0,
            sleep_counter: 0,
            is_sensor: false,
            collision_group: 0x0001,
            collision_mask: 0xFFFF,
        }
    }

    pub fn static_body(
        handle: BodyHandle,
        position: Point<D>,
        collider: Box<dyn Shape<D>>,
    ) -> Self {
        Self::new(
            handle,
            BodyType::Static,
            Transform::from_translation(position),
            collider,
            0.0,
            SVector::zeros(),
        )
    }

    pub fn dynamic_sphere(handle: BodyHandle, position: Point<D>, radius: f64, mass: f64) -> Self {
        use symtropy_math::Sphere;
        let inertia = 0.4 * mass * radius * radius;
        Self::new(
            handle,
            BodyType::Dynamic,
            Transform::from_translation(position),
            Box::new(Sphere::new(Point::origin(), radius)),
            mass,
            SVector::from_element(inertia),
        )
    }

    pub fn is_dynamic(&self) -> bool {
        matches!(self.body_type, BodyType::Dynamic)
    }

    pub fn is_static(&self) -> bool {
        matches!(self.body_type, BodyType::Static)
    }

    pub fn apply_force(&mut self, force: SVector<f64, D>) {
        if self.is_dynamic() {
            self.force_accumulator += force;
            self.wake();
        }
    }

    pub fn apply_torque(&mut self, torque: Bivector<D>) {
        if self.is_dynamic() {
            // Need to implement AddAssign for Bivector or do manually
            let mut current = self.torque_accumulator;
            for i in 0..D {
                for j in (i + 1)..D {
                    current.set(i, j, current.get(i, j) + torque.get(i, j));
                }
            }
            self.torque_accumulator = current;
            self.wake();
        }
    }

    pub fn clear_accumulators(&mut self) {
        self.force_accumulator = SVector::zeros();
        self.torque_accumulator = Bivector::zero();
    }

    pub fn wake(&mut self) {
        self.sleeping = false;
        self.sleep_timer = 0.0;
        self.sleep_counter = 0;
    }

    pub fn try_sleep(&mut self, threshold: f64, ticks: u32) {
        if !self.is_dynamic() {
            return;
        }

        let v2 = self.linear_velocity.norm_squared();
        let w2 = self.angular_velocity.norm_squared();

        if v2 < threshold * threshold && w2 < threshold * threshold {
            self.sleep_counter += 1;
            if self.sleep_counter >= ticks {
                self.sleeping = true;
                self.linear_velocity = SVector::zeros();
                self.angular_velocity = Bivector::zero();
            }
        } else {
            self.sleep_counter = 0;
        }
    }

    pub fn position(&self) -> SVector<f64, D> {
        self.transform.translation.0
    }

    /// Report the rotational inertia model actually used by this body.
    /// This is a model-path descriptor, not a physical-validity verdict.
    pub fn rotational_inertia_model(&self) -> RotationalInertiaModel {
        if !self.is_dynamic() {
            return RotationalInertiaModel::StaticOrKinematic;
        }
        if self.rotational_inertia_operator_invalidated {
            return RotationalInertiaModel::InvalidPrincipalMoments;
        }
        if self.rotational_inertia_operator.is_some() {
            return if self.rotational_inertia_operator_mass_bits == Some(self.mass.to_bits()) {
                RotationalInertiaModel::ValidatedSpatialBivectorOperator
            } else {
                RotationalInertiaModel::InvalidPrincipalMoments
            };
        }
        if self.rotational_inertia_approximate {
            return RotationalInertiaModel::MeanInertiaApproximation;
        }
        if D != 3 {
            return RotationalInertiaModel::MeanInertiaApproximation;
        }
        if self.principal_moments_3d_are_valid() {
            RotationalInertiaModel::PrincipalAxesWorldSpace3D
        } else {
            RotationalInertiaModel::InvalidPrincipalMoments
        }
    }

    /// Attach a validated full bivector inertia operator. The operator is
    /// expressed in the body frame and transported through the current rotor
    /// whenever a spatial operation is required.
    pub fn with_rotational_inertia_operator(
        mut self,
        operator: RotationalInertiaOperator<D>,
    ) -> Self {
        self.rotational_inertia_operator = Some(operator);
        self.rotational_inertia_operator_invalidated = false;
        self.rotational_inertia_operator_mass_bits = Some(self.mass.to_bits());
        self.rotational_inertia_approximate = false;
        self
    }

    /// Replace the attached operator after recomputing mass properties.
    pub fn set_rotational_inertia_operator(
        &mut self,
        operator: RotationalInertiaOperator<D>,
    ) {
        self.rotational_inertia_operator = Some(operator);
        self.rotational_inertia_operator_invalidated = false;
        self.rotational_inertia_operator_mass_bits = Some(self.mass.to_bits());
        self.rotational_inertia_approximate = false;
        self.wake();
    }

    fn spatial_inertia_view(
        &self,
    ) -> Option<crate::mass_properties::SpatialRotationalInertiaView<'_, D>> {
        if self.rotational_inertia_operator_mass_bits != Some(self.mass.to_bits())
            || self.rotational_inertia_operator_invalidated
        {
            return None;
        }
        self.rotational_inertia_operator
            .as_ref()?
            .in_spatial_frame(&self.transform.rotation)
            .ok()
    }

    /// Validate 3D body-local principal moments and their stored reciprocals.
    pub fn principal_moments_3d_are_valid(&self) -> bool {
        if D != 3 {
            return false;
        }
        let positive_and_reciprocal = (0..3).all(|axis| {
            let moment = self.inertia[axis];
            let inverse = self.inv_inertia[axis];
            moment.is_finite()
                && moment > 0.0
                && inverse.is_finite()
                && inverse > 0.0
                && (moment * inverse - 1.0).abs() <= 1.0e-8
        });
        if !positive_and_reciprocal {
            return false;
        }

        // A physically realizable 3D principal-inertia triplet must satisfy
        // the rigid-body triangle inequalities. Reject impossible synthetic
        // combinations rather than describing them as exact mass properties.
        let tolerance = 1.0e-10 * self.inertia.iter().copied().fold(1.0_f64, f64::max);
        self.inertia[0] + self.inertia[1] + tolerance >= self.inertia[2]
            && self.inertia[0] + self.inertia[2] + tolerance >= self.inertia[1]
            && self.inertia[1] + self.inertia[2] + tolerance >= self.inertia[0]
    }

    /// Apply the inverse rotational inertia operator to a world-space angular
    /// impulse, returning the world-space angular-velocity change.
    ///
    /// For 3D, the operator is R * diag(1/I_body) * R^T, where R maps the
    /// body-local principal axes into world space. For D != 3, the legacy
    /// scalar mean inverse-inertia approximation is retained and reported by
    /// rotational_inertia_model().
    pub fn apply_inverse_angular_inertia(
        &self,
        angular_impulse: &Bivector<D>,
    ) -> Bivector<D> {
        if !self.is_dynamic() || self.rotational_inertia_operator_invalidated {
            return Bivector::zero();
        }
        if self.rotational_inertia_operator.is_some() {
            return self
                .spatial_inertia_view()
                .and_then(|view| view.solve_bivector(angular_impulse).ok())
                .unwrap_or_else(Bivector::zero);
        }

        if D == 3 {
            if !self.principal_moments_3d_are_valid() {
                return Bivector::zero();
            }
            // Rotor's positive e01 generator maps +X toward +Y; the associated
            // right-handed axial vector is [e12, -e02, e01].
            let impulse_world = SVector::<f64, 3>::new(
                angular_impulse.get(1, 2),
                -angular_impulse.get(0, 2),
                angular_impulse.get(0, 1),
            );
            let rotation = SMatrix::<f64, 3, 3>::from_fn(|row, col| {
                self.transform.rotation.to_matrix()[(row, col)]
            });
            let impulse_local = rotation.transpose() * impulse_world;
            let inverse_principal = SVector::<f64, 3>::new(
                self.inv_inertia[0],
                self.inv_inertia[1],
                self.inv_inertia[2],
            );
            let delta_local = inverse_principal.component_mul(&impulse_local);
            let delta_world = rotation * delta_local;
            let mut result = Bivector::<D>::zero();
            result.set(0, 1, delta_world[2]);
            result.set(0, 2, -delta_world[1]);
            result.set(1, 2, delta_world[0]);
            result
        } else {
            let inverse_mean = self.inv_inertia.sum() / D as f64;
            if inverse_mean.is_finite() && inverse_mean > 0.0 {
                angular_impulse.scale(inverse_mean)
            } else {
                Bivector::zero()
            }
        }
    }

    /// Compute world-space angular acceleration from applied torque using
    /// Euler's rigid-body equation.
    ///
    /// In 3D this includes the gyroscopic term:
    /// alpha = I_world^-1 * (tau_world - omega_world x L_world).
    /// Omitting the gyroscopic term would produce incorrect torque-free
    /// precession for asymmetric bodies. Higher-dimensional compatibility
    /// paths retain the legacy scalar approximation and do not claim this
    /// general rotational coupling.
    pub fn angular_acceleration_from_torque(
        &self,
        torque_world_bivector: &Bivector<D>,
    ) -> Bivector<D> {
        if !self.is_dynamic() || self.rotational_inertia_operator_invalidated {
            return Bivector::zero();
        }
        if self.rotational_inertia_operator.is_some() {
            return self
                .spatial_inertia_view()
                .and_then(|view| {
                    view.angular_acceleration(&self.angular_velocity, torque_world_bivector)
                        .ok()
                })
                .unwrap_or_else(Bivector::zero);
        }
        if D != 3 {
            return self.apply_inverse_angular_inertia(torque_world_bivector);
        }
        if !self.principal_moments_3d_are_valid() {
            return Bivector::zero();
        }

        let rotation = SMatrix::<f64, 3, 3>::from_fn(|row, col| {
            self.transform.rotation.to_matrix()[(row, col)]
        });
        let omega_world = SVector::<f64, 3>::new(
            self.angular_velocity.get(1, 2),
            -self.angular_velocity.get(0, 2),
            self.angular_velocity.get(0, 1),
        );
        let torque_world = SVector::<f64, 3>::new(
            torque_world_bivector.get(1, 2),
            -torque_world_bivector.get(0, 2),
            torque_world_bivector.get(0, 1),
        );

        let omega_local = rotation.transpose() * omega_world;
        let principal_moments = SVector::<f64, 3>::new(
            self.inertia[0],
            self.inertia[1],
            self.inertia[2],
        );
        let inverse_principal = SVector::<f64, 3>::new(
            self.inv_inertia[0],
            self.inv_inertia[1],
            self.inv_inertia[2],
        );
        let momentum_local = principal_moments.component_mul(&omega_local);
        let momentum_world = rotation * momentum_local;

        let gyroscopic = SVector::<f64, 3>::new(
            omega_world[1] * momentum_world[2] - omega_world[2] * momentum_world[1],
            omega_world[2] * momentum_world[0] - omega_world[0] * momentum_world[2],
            omega_world[0] * momentum_world[1] - omega_world[1] * momentum_world[0],
        );
        let net_torque_world = torque_world - gyroscopic;
        let net_torque_local = rotation.transpose() * net_torque_world;
        let alpha_local = inverse_principal.component_mul(&net_torque_local);
        let alpha_world = rotation * alpha_local;

        let mut result = Bivector::<D>::zero();
        result.set(0, 1, alpha_world[2]);
        result.set(0, 2, -alpha_world[1]);
        result.set(1, 2, alpha_world[0]);
        result
    }

    /// Rotational kinetic energy using the same inertia operator as integration.
    /// Invalid 3D principal moments fail closed to zero rotational energy.
    pub fn rotational_kinetic_energy(&self) -> f64 {
        if !self.is_dynamic() || self.rotational_inertia_operator_invalidated {
            return 0.0;
        }
        if self.rotational_inertia_operator.is_some() {
            return self
                .spatial_inertia_view()
                .and_then(|view| {
                    view.kinetic_energy_joules(&self.angular_velocity).ok()
                })
                .unwrap_or(0.0);
        }
        if D == 3 {
            if !self.principal_moments_3d_are_valid() {
                return 0.0;
            }
            let omega_world = SVector::<f64, 3>::new(
                self.angular_velocity.get(1, 2),
                -self.angular_velocity.get(0, 2),
                self.angular_velocity.get(0, 1),
            );
            let rotation = SMatrix::<f64, 3, 3>::from_fn(|row, col| {
                self.transform.rotation.to_matrix()[(row, col)]
            });
            let omega_local = rotation.transpose() * omega_world;
            0.5 * (0..3)
                .map(|axis| self.inertia[axis] * omega_local[axis] * omega_local[axis])
                .sum::<f64>()
        } else {
            let inertia_mean = self.inertia.sum() / D as f64;
            0.5 * inertia_mean * self.angular_velocity.norm_squared()
        }
    }

    pub fn kinetic_energy(&self) -> f64 {
        if !self.is_dynamic() {
            return 0.0;
        }
        let linear = 0.5 * self.mass * self.linear_velocity.norm_squared();
        linear + self.rotational_kinetic_energy()
    }

    pub fn merge_toolhead(
        &mut self,
        local_transform: Transform<D>,
        tool_collider: Box<dyn Shape<D>>,
        tool_mass: f64,
    ) {
        use symtropy_math::CompoundShape;

        let mut compound =
            if let Some(existing) = self.collider.as_any().downcast_ref::<CompoundShape<D>>() {
                let mut new_c = CompoundShape::<D>::new();
                for (tf, s) in existing.children() {
                    new_c.add_child(tf.clone(), s.clone_box());
                }
                new_c
            } else {
                let mut new_c = CompoundShape::<D>::new();
                new_c.add_child(Transform::identity(), self.collider.as_ref().clone_box());
                new_c
            };

        compound.add_child(local_transform.clone(), tool_collider);
        self.collider = Box::new(compound);

        let total_mass = self.mass + tool_mass;
        let com_offset = (local_transform.translation.0 * tool_mass) / total_mass;

        let i_original = self.inertia.sum() / D as f64;
        let i_tool = 0.4 * tool_mass * 0.1 * 0.1;

        let d_original = com_offset.norm();
        let d_tool = (local_transform.translation.0 - com_offset).norm();
        let i_total = (i_original + self.mass * d_original * d_original)
            + (i_tool + tool_mass * d_tool * d_tool);

        self.mass = total_mass;
        self.inv_mass = 1.0 / total_mass;
        self.inertia = SVector::from_element(i_total);
        self.inv_inertia = SVector::from_element(1.0 / i_total);

        self.transform.translation.0 += com_offset;

        // The compatibility merge uses a simplified scalar toolhead inertia,
        // not a composite central-second-moment derivation. Report it as an
        // approximation even when the resulting three stored moments happen
        // to be equal.
        self.rotational_inertia_approximate = true;

        // Toolhead mass changes invalidate any previously certified full
        // inertia operator. Do not silently keep using stale mass properties.
        if self.rotational_inertia_operator.is_some() {
            self.rotational_inertia_operator = None;
            self.rotational_inertia_operator_invalidated = true;
            self.rotational_inertia_operator_mass_bits = None;
        }
    }

    pub fn world_support(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        let local_dir = self.transform.rotation.reverse().rotate_vector(direction);
        let local_support = self.collider.support(&local_dir);
        self.transform.transform_point(&Point(local_support)).0
    }
}


#[cfg(test)]
mod rotational_inertia_tests {
    use super::*;
    use symtropy_math::{Bivector, Point, Rotor};

    fn anisotropic_body() -> RigidBody<3> {
        let mut body = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::origin(),
            0.1,
            1.0,
        );
        body.inertia = SVector::from([2.0, 3.0, 4.0]);
        body.inv_inertia = SVector::from([0.5, 1.0 / 3.0, 0.25]);
        body.angular_damping = 0.0;
        body
    }

    fn axial_vector(bivector: &Bivector<3>) -> SVector<f64, 3> {
        SVector::new(
            bivector.get(1, 2),
            -bivector.get(0, 2),
            bivector.get(0, 1),
        )
    }

    fn from_axial_vector(vector: &SVector<f64, 3>) -> Bivector<3> {
        let mut result = Bivector::zero();
        result.set(0, 1, vector[2]);
        result.set(0, 2, -vector[1]);
        result.set(1, 2, vector[0]);
        result
    }

    fn world_angular_momentum(body: &RigidBody<3>) -> SVector<f64, 3> {
        let rotation = body.transform.rotation.to_matrix();
        let omega_world = axial_vector(&body.angular_velocity);
        let omega_local = rotation.transpose() * omega_world;
        let principal_moments = SVector::<f64, 3>::new(
            body.inertia[0],
            body.inertia[1],
            body.inertia[2],
        );
        rotation * principal_moments.component_mul(&omega_local)
    }

    fn assert_bivector_close(actual: &Bivector<3>, expected: &Bivector<3>, tol: f64) {
        for i in 0..3 {
            for j in (i + 1)..3 {
                assert!(
                    (actual.get(i, j) - expected.get(i, j)).abs() <= tol,
                    "component ({i},{j}): actual={}, expected={}",
                    actual.get(i, j),
                    expected.get(i, j)
                );
            }
        }
    }

    #[test]
    fn validated_full_operator_overrides_diagonal_fallback_and_handles_products_of_inertia() {
        let operator = RotationalInertiaOperator::<3>::try_from_row_major(vec![
            4.0, 0.5, 0.0,
            0.5, 3.0, 0.25,
            0.0, 0.25, 2.0,
        ]).unwrap();
        let body = anisotropic_body().with_rotational_inertia_operator(operator);
        assert_eq!(
            body.rotational_inertia_model(),
            RotationalInertiaModel::ValidatedSpatialBivectorOperator
        );

        let mut impulse = Bivector::zero();
        impulse.set(0, 1, 1.0);
        impulse.set(0, 2, 2.0);
        impulse.set(1, 2, -0.5);
        let delta = body.apply_inverse_angular_inertia(&impulse);
        let recovered = body
            .rotational_inertia_operator
            .as_ref()
            .unwrap()
            .apply_bivector(&delta)
            .unwrap();
        assert_bivector_close(&recovered, &impulse, 1.0e-11);
    }

    #[test]
    fn operator_inertia_is_transported_with_body_orientation() {
        let operator = RotationalInertiaOperator::<3>::try_from_row_major(vec![
            4.0, 0.5, 0.0,
            0.5, 3.0, 0.25,
            0.0, 0.25, 2.0,
        ]).unwrap();
        let mut body = anisotropic_body().with_rotational_inertia_operator(operator);
        let baseline_delta = body.apply_inverse_angular_inertia(&Bivector::<3>::unit_plane(0, 1));
        body.transform.rotation = Rotor::<3>::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 2),
            0.63,
        );
        let rotated_delta =
            body.apply_inverse_angular_inertia(&Bivector::<3>::unit_plane(0, 1));
        assert!(
            (rotated_delta.add(&baseline_delta.scale(-1.0))).norm() > 1.0e-4,
            "changing body pose should rotate the anisotropic operator in spatial coordinates"
        );
    }

    #[test]
    fn direct_mass_mutation_invalidates_attached_operator_until_replaced() {
        let operator = RotationalInertiaOperator::<3>::isotropic(1.0).unwrap();
        let mut body = anisotropic_body().with_rotational_inertia_operator(operator);
        body.mass += 0.25;

        assert_eq!(
            body.rotational_inertia_model(),
            RotationalInertiaModel::InvalidPrincipalMoments
        );
        assert_eq!(
            body.apply_inverse_angular_inertia(&Bivector::<3>::unit_plane(0, 1)),
            Bivector::zero()
        );

        body.set_rotational_inertia_operator(
            RotationalInertiaOperator::<3>::isotropic(1.25).unwrap(),
        );
        assert_eq!(
            body.rotational_inertia_model(),
            RotationalInertiaModel::ValidatedSpatialBivectorOperator
        );
    }

    #[test]
    fn toolhead_merge_invalidates_attached_full_inertia_operator() {
        let operator = RotationalInertiaOperator::<3>::isotropic(1.0).unwrap();
        let mut body = anisotropic_body().with_rotational_inertia_operator(operator);
        body.merge_toolhead(
            Transform::identity(),
            Box::new(symtropy_math::Sphere::unit()),
            0.25,
        );
        assert_eq!(
            body.rotational_inertia_model(),
            RotationalInertiaModel::InvalidPrincipalMoments
        );
        assert_eq!(
            body.apply_inverse_angular_inertia(&Bivector::<3>::unit_plane(0, 1)),
            Bivector::zero()
        );
    }

    #[test]
    fn isotropic_3d_principal_inertia_matches_scalar_path() {
        let mut body = anisotropic_body();
        body.inertia = SVector::from([2.0, 2.0, 2.0]);
        body.inv_inertia = SVector::from([0.5, 0.5, 0.5]);

        let mut impulse = Bivector::zero();
        impulse.set(0, 1, 2.0);
        impulse.set(0, 2, -4.0);
        impulse.set(1, 2, 6.0);

        assert_eq!(
            body.rotational_inertia_model(),
            RotationalInertiaModel::PrincipalAxesWorldSpace3D
        );
        assert_bivector_close(
            &body.apply_inverse_angular_inertia(&impulse),
            &impulse.scale(0.5),
            1.0e-12,
        );
    }

    #[test]
    fn impossible_principal_moment_triplet_is_rejected() {
        let mut body = anisotropic_body();
        body.inertia = SVector::from([1.0, 1.0, 3.0]);
        body.inv_inertia = SVector::from([1.0, 1.0, 1.0 / 3.0]);
        assert_eq!(
            body.rotational_inertia_model(),
            RotationalInertiaModel::InvalidPrincipalMoments
        );
    }

    #[test]
    fn asymmetric_principal_axis_impulses_use_their_own_moments() {
        let body = anisotropic_body();

        let mut about_x = Bivector::zero();
        about_x.set(1, 2, 1.0);
        let mut about_y = Bivector::zero();
        about_y.set(0, 2, 1.0);
        let mut about_z = Bivector::zero();
        about_z.set(0, 1, 1.0);

        assert!((body.apply_inverse_angular_inertia(&about_x).get(1, 2) - 0.5).abs() < 1e-12);
        assert!((body.apply_inverse_angular_inertia(&about_y).get(0, 2) - (1.0 / 3.0)).abs() < 1e-12);
        assert!((body.apply_inverse_angular_inertia(&about_z).get(0, 1) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn rotated_asymmetric_body_uses_world_space_inertia() {
        let mut body = anisotropic_body();
        body.transform.rotation = Rotor::<3>::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 1),
            std::f64::consts::FRAC_PI_2,
        );

        let mut world_x_impulse = Bivector::zero();
        world_x_impulse.set(1, 2, 1.0);
        let delta = body.apply_inverse_angular_inertia(&world_x_impulse);

        // After a +90° Z rotation, world X aligns with local -Y,
        // whose principal moment is 3.
        assert!((delta.get(1, 2) - (1.0 / 3.0)).abs() < 1e-12);
        assert!(delta.get(0, 2).abs() < 1e-12);
        assert!(delta.get(0, 1).abs() < 1e-12);
    }

    #[test]
    fn rotated_asymmetric_body_exposes_nonprincipal_axis_coupling() {
        let mut body = anisotropic_body();
        body.transform.rotation = Rotor::<3>::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 1),
            std::f64::consts::FRAC_PI_4,
        );

        let mut world_x_impulse = Bivector::zero();
        world_x_impulse.set(1, 2, 1.0);
        let delta = body.apply_inverse_angular_inertia(&world_x_impulse);

        // R diag(1/2, 1/3, 1/4) R^T * e_x = (5/12, 1/12, 0).
        assert!((delta.get(1, 2) - (5.0 / 12.0)).abs() < 1e-12);
        assert!((delta.get(0, 2) + (1.0 / 12.0)).abs() < 1e-12);
        assert!(delta.get(0, 1).abs() < 1e-12);
    }

    #[test]
    fn inertia_operator_is_frame_covariant() {
        let mut body = anisotropic_body();
        let body_rotation = Rotor::<3>::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 1),
            0.37,
        );
        body.transform.rotation = body_rotation;

        let mut impulse = Bivector::zero();
        impulse.set(1, 2, 0.7);
        impulse.set(0, 2, -0.4);
        impulse.set(0, 1, 0.2);
        let delta = body.apply_inverse_angular_inertia(&impulse);

        let frame_change = Rotor::<3>::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 2),
            -0.61,
        );
        let mut rotated_body = anisotropic_body();
        rotated_body.transform.rotation = frame_change.compose(&body_rotation);
        let rotated_impulse =
            from_axial_vector(&frame_change.rotate_vector(&axial_vector(&impulse)));
        let rotated_delta = rotated_body.apply_inverse_angular_inertia(&rotated_impulse);
        let expected_delta =
            from_axial_vector(&frame_change.rotate_vector(&axial_vector(&delta)));

        assert_bivector_close(&rotated_delta, &expected_delta, 1.0e-11);
    }

    #[test]
    fn angular_impulse_energy_identity_uses_same_inertia_operator() {
        let mut body = anisotropic_body();
        body.transform.rotation = Rotor::<3>::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 2),
            0.43,
        );
        body.angular_velocity.set(1, 2, 0.3);
        body.angular_velocity.set(0, 2, -0.2);
        body.angular_velocity.set(0, 1, 0.1);

        let omega_before = body.angular_velocity;
        let energy_before = body.rotational_kinetic_energy();
        let mut impulse = Bivector::zero();
        impulse.set(1, 2, 0.8);
        impulse.set(0, 2, -0.5);
        impulse.set(0, 1, 0.25);
        let delta = body.apply_inverse_angular_inertia(&impulse);
        body.angular_velocity = body.angular_velocity.add(&delta);

        let lhs = body.rotational_kinetic_energy() - energy_before;
        let mut impulse_dot_omega = 0.0;
        let mut impulse_dot_delta = 0.0;
        for i in 0..3 {
            for j in (i + 1)..3 {
                impulse_dot_omega += impulse.get(i, j) * omega_before.get(i, j);
                impulse_dot_delta += impulse.get(i, j) * delta.get(i, j);
            }
        }
        let rhs = impulse_dot_omega + 0.5 * impulse_dot_delta;
        assert!((lhs - rhs).abs() < 1e-11, "ΔK={lhs}, impulse identity={rhs}");
    }

    #[test]
    fn invalid_3d_principal_moments_are_reported_and_fail_closed() {
        let mut body = anisotropic_body();
        body.inertia[1] = f64::NAN;
        assert_eq!(
            body.rotational_inertia_model(),
            RotationalInertiaModel::InvalidPrincipalMoments
        );
        let impulse = Bivector::<3>::unit_plane(0, 1);
        assert_eq!(
            body.apply_inverse_angular_inertia(&impulse),
            Bivector::zero()
        );
        assert_eq!(body.rotational_kinetic_energy(), 0.0);
    }

    #[test]
    fn torque_free_asymmetric_top_conserves_world_angular_momentum_with_timestep_refinement() {
        fn run(dt: f64, steps: usize) -> f64 {
            let mut body = anisotropic_body();
            body.angular_velocity.set(1, 2, 0.8);
            body.angular_velocity.set(0, 2, -0.6);
            body.angular_velocity.set(0, 1, 0.35);
            let initial_momentum = world_angular_momentum(&body);

            for _ in 0..steps {
                crate::integrator::integrate(
                    &mut body,
                    &SVector::<f64, 3>::zeros(),
                    dt,
                );
            }

            (world_angular_momentum(&body) - initial_momentum).norm()
                / initial_momentum.norm()
        }

        let coarse_error = run(0.001, 1_000);
        let fine_error = run(0.0005, 2_000);
        assert!(
            fine_error < coarse_error,
            "world angular momentum error should decrease under timestep refinement: coarse={coarse_error}, fine={fine_error}"
        );
        assert!(
            fine_error < 5.0e-3,
            "torque-free angular momentum drift={fine_error}"
        );
    }

    #[test]
    fn asymmetric_torque_free_body_has_gyroscopic_precession() {
        let body = anisotropic_body();
        let mut spinning = body;
        // Axial angular velocity (1, 1, 0) in the local principal frame.
        spinning.angular_velocity.set(1, 2, 1.0);
        spinning.angular_velocity.set(0, 2, -1.0);

        let acceleration =
            spinning.angular_acceleration_from_torque(&Bivector::<3>::zero());

        // omega x (I omega) = (1,1,0) x (1,2,0) = (0,0,1).
        // Euler's equation therefore yields alpha_z = -1 / I_z = -1/4.
        assert!(acceleration.get(1, 2).abs() < 1e-12);
        assert!(acceleration.get(0, 2).abs() < 1e-12);
        assert!((acceleration.get(0, 1) + 0.25).abs() < 1e-12);
    }

    #[test]
    fn torque_free_asymmetric_integration_keeps_rotational_energy_bounded() {
        let mut body = anisotropic_body();
        body.angular_damping = 0.0;
        body.linear_damping = 0.0;
        body.angular_velocity.set(1, 2, 0.8);
        body.angular_velocity.set(0, 2, -0.6);
        body.angular_velocity.set(0, 1, 0.35);

        let initial = body.rotational_kinetic_energy();
        for _ in 0..2_000 {
            crate::integrator::integrate(
                &mut body,
                &SVector::<f64, 3>::zeros(),
                0.0005,
            );
        }
        let final_energy = body.rotational_kinetic_energy();
        let relative_drift = ((final_energy - initial) / initial).abs();

        assert!(relative_drift < 0.01, "relative rotational energy drift={relative_drift}");
        assert!(body.transform.rotation.is_proper_rotation(1e-9));
    }

    #[test]
    fn higher_dimensions_keep_explicit_mean_inertia_compatibility_path() {
        let mut body = RigidBody::<4>::dynamic_sphere(
            BodyHandle(2),
            Point::origin(),
            0.1,
            1.0,
        );
        body.inertia = SVector::from([1.0, 2.0, 3.0, 4.0]);
        body.inv_inertia = SVector::from([1.0, 0.5, 1.0 / 3.0, 0.25]);

        assert_eq!(
            body.rotational_inertia_model(),
            RotationalInertiaModel::MeanInertiaApproximation
        );
        let impulse = Bivector::<4>::unit_plane(0, 1);
        let inverse_mean = body.inv_inertia.sum() / 4.0;
        assert!((body.apply_inverse_angular_inertia(&impulse).get(0, 1) - inverse_mean).abs() < 1e-12);
    }
}
