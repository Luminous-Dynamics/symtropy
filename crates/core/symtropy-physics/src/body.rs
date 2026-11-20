// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::{SMatrix, SVector};
use crate::mass_properties::{MassProperties, RotationalInertiaOperator};
use serde::{Deserialize, Serialize};
use symtropy_math::{Bivector, CompoundShape, Point, Shape, Transform};

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

/// Errors returned by the checked legacy toolhead merge path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolheadMergeError {
    /// Only a dynamic body can accept additional rigid mass.
    BodyNotDynamic,
    /// The dimensionality must support at least one rotational plane.
    InvalidDimension,
    /// The base body's mass is not finite and positive.
    InvalidBodyMass,
    /// Toolhead mass is not finite and positive.
    InvalidToolMass,
    /// Stored compatibility inertia is not finite and positive.
    InvalidBodyInertia,
    /// A body or toolhead transform is not finite or not a proper rotation.
    InvalidTransform,
    /// Existing dynamic state, force, or torque is non-finite.
    InvalidBodyState,
    /// A merged mass, inertia, reference point, velocity, or wrench is unrepresentable.
    UnrepresentableResult,
}

/// Errors returned when binding validated mass properties to a rigid body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MassPropertiesBodyError {
    /// Rotational dynamics require at least one bivector plane.
    InvalidDimension,
    /// The supplied geometry-frame transform is not finite or proper.
    InvalidGeometryTransform,
    /// Moving the origin to the supplied COM produced an unrepresentable pose.
    UnrepresentableCenterOfMass,
    /// The supplied full inertia operator has no finite positive mean-plane view.
    InvalidMeanPlaneCompatibilityValue,
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
    /// Legacy per-axis inertia compatibility values.
    ///
    /// When no full operator is attached, the 3D values are principal moments
    /// in the body-local frame; for D != 3 they feed an explicitly approximate
    /// mean-inertia path. When a validated full bivector operator is attached,
    /// that operator is authoritative. In particular, the COM-bound constructor
    /// stores the operator's mean plane diagonal here as a compatibility view;
    /// these values are not claimed to be principal moments of a non-diagonal
    /// operator.
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
    /// Mass value against which the legacy diagonal inertia was derived.
    pub(crate) mass_properties_mass_bits: Option<u64>,
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
            mass_properties_mass_bits: Some(mass.to_bits()),
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

    /// Construct a dynamic body from validated algebraic mass properties.
    ///
    /// The supplied transform maps the collider's **geometry frame** into world
    /// space. The body's live origin is moved to the validated body-frame COM,
    /// and the collider is re-expressed about that COM before the body is
    /// returned. The full bivector inertia operator is attached as the
    /// authoritative rotational model.
    ///
    /// This constructor validates the mass, COM, operator, and frame
    /// transformation. It does not establish that the caller's collider,
    /// material/density assignment, or geometry revision actually produced
    /// those mass properties; that external source binding remains a separate
    /// engineering-evidence obligation.
    pub fn try_new_with_mass_properties(
        handle: BodyHandle,
        geometry_transform: Transform<D>,
        collider: Box<dyn Shape<D>>,
        mass_properties: MassProperties<D>,
    ) -> Result<Self, MassPropertiesBodyError> {
        if D < 2 {
            return Err(MassPropertiesBodyError::InvalidDimension);
        }
        if !geometry_transform.translation.0.iter().all(|value| value.is_finite())
            || !geometry_transform.rotation.is_proper_rotation(1.0e-8)
        {
            return Err(MassPropertiesBodyError::InvalidGeometryTransform);
        }

        let center_of_mass_local = *mass_properties.center_of_mass_m();
        if !center_of_mass_local.iter().all(|value| value.is_finite()) {
            return Err(MassPropertiesBodyError::UnrepresentableCenterOfMass);
        }

        let world_com_offset = geometry_transform
            .rotation
            .rotate_vector(&center_of_mass_local);
        let center_of_mass_world = geometry_transform.translation.0 + world_com_offset;
        if !world_com_offset.iter().all(|value| value.is_finite())
            || !center_of_mass_world.iter().all(|value| value.is_finite())
        {
            return Err(MassPropertiesBodyError::UnrepresentableCenterOfMass);
        }

        // Preserve the complete plane-space operator. The vector field is
        // populated only as a legacy compatibility view; it is not used while
        // the full operator remains attached.
        let operator = mass_properties.rotational_inertia();
        let dof = operator.dof();
        if dof == 0 {
            return Err(MassPropertiesBodyError::InvalidDimension);
        }
        let matrix = operator.as_row_major();
        if matrix.len() != dof * dof {
            return Err(MassPropertiesBodyError::InvalidMeanPlaneCompatibilityValue);
        }
        let mean_plane_moment = (0..dof)
            .map(|index| matrix[index * dof + index])
            .sum::<f64>()
            / dof as f64;
        if !mean_plane_moment.is_finite() || mean_plane_moment <= 0.0 {
            return Err(MassPropertiesBodyError::InvalidMeanPlaneCompatibilityValue);
        }

        // Rebase the collider's child transforms from the geometry origin to
        // the COM origin. This also preserves existing compound child rotations.
        let mut centered_collider = CompoundShape::<D>::new();
        if let Some(existing) = collider.as_any().downcast_ref::<CompoundShape<D>>() {
            for (child_transform, shape) in existing.children() {
                let mut shifted = child_transform.clone();
                shifted.translation.0 -= center_of_mass_local;
                centered_collider.add_child(shifted, shape.clone_box());
            }
        } else {
            let geometry_from_centered = Transform::from_translation(Point(-center_of_mass_local));
            centered_collider.add_child(geometry_from_centered, collider);
        }

        let body_transform = Transform {
            translation: Point(center_of_mass_world),
            rotation: geometry_transform.rotation,
        };
        let mass = mass_properties.mass_kg();
        let compatibility_inertia = SVector::from_element(mean_plane_moment);
        let mut body = Self::new(
            handle,
            BodyType::Dynamic,
            body_transform,
            Box::new(centered_collider),
            mass,
            compatibility_inertia,
        );
        body.set_rotational_inertia_operator(operator.clone());
        Ok(body)
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
        if self.mass_properties_mass_bits != Some(self.mass.to_bits())
            || self.rotational_inertia_operator_invalidated
        {
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
        self.mass_properties_mass_bits = Some(self.mass.to_bits());
        self.rotational_inertia_approximate = false;
        self
    }

    /// Replace legacy diagonal mass properties as one provenance-bearing update.
    ///
    /// This is the supported way to change mass and retain the diagonal
    /// compatibility path. D != 3 remains an explicitly approximate model.
    /// The caller owns the geometry/material/centre-of-mass evidence used to
    /// derive the supplied inertia vector.
    pub fn replace_principal_mass_properties(
        &mut self,
        mass: f64,
        inertia: SVector<f64, D>,
    ) -> bool {
        if !mass.is_finite() || mass <= 0.0 {
            return false;
        }
        self.mass = mass;
        self.inv_mass = 1.0 / mass;
        self.inertia = inertia;
        self.inv_inertia = inertia.map(|moment| {
            if moment.is_finite() && moment > 0.0 {
                1.0 / moment
            } else {
                0.0
            }
        });
        self.mass_properties_mass_bits = Some(mass.to_bits());
        self.rotational_inertia_operator = None;
        self.rotational_inertia_operator_invalidated = false;
        self.rotational_inertia_operator_mass_bits = None;
        self.rotational_inertia_approximate = false;
        self.wake();
        true
    }

    /// Replace the attached operator after recomputing mass properties.
    pub fn set_rotational_inertia_operator(
        &mut self,
        operator: RotationalInertiaOperator<D>,
    ) {
        self.rotational_inertia_operator = Some(operator);
        self.rotational_inertia_operator_invalidated = false;
        self.rotational_inertia_operator_mass_bits = Some(self.mass.to_bits());
        self.mass_properties_mass_bits = Some(self.mass.to_bits());
        self.rotational_inertia_approximate = false;
        self.wake();
    }

    fn spatial_inertia_view(
        &self,
    ) -> Option<crate::mass_properties::SpatialRotationalInertiaView<'_, D>> {
        if self.rotational_inertia_operator_mass_bits != Some(self.mass.to_bits())
            || self.mass_properties_mass_bits != Some(self.mass.to_bits())
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
        if !self.is_dynamic()
            || self.rotational_inertia_operator_invalidated
            || self.mass_properties_mass_bits != Some(self.mass.to_bits())
        {
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
        if !self.is_dynamic()
            || self.rotational_inertia_operator_invalidated
            || self.mass_properties_mass_bits != Some(self.mass.to_bits())
        {
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
        if !self.is_dynamic()
            || self.rotational_inertia_operator_invalidated
            || self.mass_properties_mass_bits != Some(self.mass.to_bits())
        {
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

    /// Fallibly merge a rigid toolhead using the legacy isotropic mass approximation.
    ///
    /// The toolhead is assumed to share the base body's rigid motion at the
    /// instant of attachment. Its mass distribution is approximated by the
    /// historical 0.1 m solid-sphere model; arbitrary collider geometry is
    /// **not** inferred as physical mass properties. The resulting inertia is
    /// explicitly marked approximate and any previously attached full
    /// rotational-inertia operator is invalidated.
    ///
    /// The body origin moves to the combined COM. Existing compound children
    /// and the new tool transform are re-expressed relative to that COM, while
    /// world-space geometry, point velocity and the accumulated wrench about
    /// any fixed world point are preserved.
    pub fn try_merge_toolhead(
        &mut self,
        local_transform: Transform<D>,
        tool_collider: Box<dyn Shape<D>>,
        tool_mass: f64,
    ) -> Result<(), ToolheadMergeError> {
        if D < 2 {
            return Err(ToolheadMergeError::InvalidDimension);
        }
        if !self.is_dynamic() {
            return Err(ToolheadMergeError::BodyNotDynamic);
        }
        if !self.mass.is_finite() || self.mass <= 0.0 {
            return Err(ToolheadMergeError::InvalidBodyMass);
        }
        if !tool_mass.is_finite() || tool_mass <= 0.0 {
            return Err(ToolheadMergeError::InvalidToolMass);
        }
        if self.inertia.iter().any(|value| !value.is_finite() || *value <= 0.0) {
            return Err(ToolheadMergeError::InvalidBodyInertia);
        }
        if !local_transform.translation.0.iter().all(|value| value.is_finite())
            || !local_transform.rotation.is_proper_rotation(1.0e-8)
            || !self.transform.translation.0.iter().all(|value| value.is_finite())
            || !self.transform.rotation.is_proper_rotation(1.0e-8)
        {
            return Err(ToolheadMergeError::InvalidTransform);
        }
        if !self.linear_velocity.iter().all(|value| value.is_finite())
            || !self.angular_velocity.is_finite()
            || !self.force_accumulator.iter().all(|value| value.is_finite())
            || !self.torque_accumulator.is_finite()
        {
            return Err(ToolheadMergeError::InvalidBodyState);
        }

        let total_mass = self.mass + tool_mass;
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return Err(ToolheadMergeError::UnrepresentableResult);
        }
        let tool_mass_fraction = tool_mass / total_mass;
        if !tool_mass_fraction.is_finite() || tool_mass_fraction <= 0.0 {
            return Err(ToolheadMergeError::UnrepresentableResult);
        }
        let com_offset = local_transform.translation.0 * tool_mass_fraction;
        if !com_offset.iter().all(|value| value.is_finite()) {
            return Err(ToolheadMergeError::UnrepresentableResult);
        }

        // The legacy convenience path uses a nominal 0.1 m solid-sphere
        // approximation for the toolhead. This is intentionally not inferred
        // from the supplied collider's geometry.
        let original_mean_inertia = self.inertia.sum() / D as f64;
        let tool_approximate_inertia = 0.4 * tool_mass * 0.1 * 0.1;
        let original_offset_squared = com_offset.norm_squared();
        let tool_offset = local_transform.translation.0 - com_offset;
        let tool_offset_squared = tool_offset.norm_squared();
        // Averaging the full plane inertia m(d_i^2 + d_j^2) over all
        // D(D-1)/2 planes gives (2m/D)|d|^2, not m|d|^2.
        let mean_parallel_axis_factor = 2.0 / D as f64;
        let original_parallel_axis_mean =
            mean_parallel_axis_factor * self.mass * original_offset_squared;
        let tool_parallel_axis_mean =
            mean_parallel_axis_factor * tool_mass * tool_offset_squared;
        let merged_mean_inertia = original_mean_inertia
            + original_parallel_axis_mean
            + tool_approximate_inertia
            + tool_parallel_axis_mean;
        let inverse_mass = 1.0 / total_mass;
        let inverse_mean_inertia = 1.0 / merged_mean_inertia;
        if !original_mean_inertia.is_finite()
            || !tool_approximate_inertia.is_finite()
            || !original_offset_squared.is_finite()
            || !tool_offset_squared.is_finite()
            || !original_parallel_axis_mean.is_finite()
            || !tool_parallel_axis_mean.is_finite()
            || !merged_mean_inertia.is_finite()
            || merged_mean_inertia <= 0.0
            || !inverse_mass.is_finite()
            || inverse_mass <= 0.0
            || !inverse_mean_inertia.is_finite()
            || inverse_mean_inertia <= 0.0
        {
            return Err(ToolheadMergeError::UnrepresentableResult);
        }

        // Build the entire replacement collider before mutating the body.
        // Child transforms are expressed relative to the new COM:
        // r_child,new = r_child,old - c_offset.
        let mut compound = CompoundShape::<D>::new();
        if let Some(existing) = self.collider.as_any().downcast_ref::<CompoundShape<D>>() {
            for (child_transform, shape) in existing.children() {
                let mut shifted = child_transform.clone();
                shifted.translation.0 -= com_offset;
                compound.add_child(shifted, shape.clone_box());
            }
        } else {
            let mut base_transform = Transform::identity();
            base_transform.translation.0 -= com_offset;
            compound.add_child(base_transform, self.collider.as_ref().clone_box());
        }

        let mut shifted_tool_transform = local_transform;
        shifted_tool_transform.translation.0 -= com_offset;
        compound.add_child(shifted_tool_transform, tool_collider);

        // The COM offset is body-local. Convert it to a world-space reference
        // point shift before updating world translation, velocity, or wrench.
        // Rotor::from_bivector(B)=exp(-B), so the point-velocity increment is
        // -B*r for the stored angular-velocity convention.
        let world_com_offset = self.transform.rotation.rotate_vector(&com_offset);
        let new_translation = self.transform.translation.0 + world_com_offset;
        let new_linear_velocity =
            self.linear_velocity - self.angular_velocity.apply_to_vector(&world_com_offset);
        let torque_shift = Bivector::from_wedge(&world_com_offset, &self.force_accumulator);
        let new_torque_accumulator =
            self.torque_accumulator.add(&torque_shift.scale(-1.0));
        if !world_com_offset.iter().all(|value| value.is_finite())
            || !new_translation.iter().all(|value| value.is_finite())
            || !new_linear_velocity.iter().all(|value| value.is_finite())
            || !new_torque_accumulator.is_finite()
        {
            return Err(ToolheadMergeError::UnrepresentableResult);
        }

        // All validation is complete. Commit the new compound and mass state.
        let source_mass_properties_current =
            self.mass_properties_mass_bits == Some(self.mass.to_bits());
        let had_full_operator = self.rotational_inertia_operator.is_some();
        self.collider = Box::new(compound);
        self.mass = total_mass;
        self.inv_mass = inverse_mass;
        self.inertia = SVector::from_element(merged_mean_inertia);
        self.inv_inertia = SVector::from_element(inverse_mean_inertia);
        self.transform.translation.0 = new_translation;
        self.linear_velocity = new_linear_velocity;
        self.torque_accumulator = new_torque_accumulator;

        self.rotational_inertia_approximate = true;
        self.mass_properties_mass_bits = if source_mass_properties_current {
            Some(total_mass.to_bits())
        } else {
            None
        };

        // Toolhead mass changes invalidate any previously attached full
        // operator; the scalar compatibility approximation must not masquerade
        // as the old source-qualified tensor.
        if had_full_operator {
            self.rotational_inertia_operator = None;
            self.rotational_inertia_operator_invalidated = true;
            self.rotational_inertia_operator_mass_bits = None;
        }
        self.wake();

        Ok(())
    }

    /// Compatibility wrapper for the legacy approximate merge.
    ///
    /// Invalid inputs are a no-op. Call try_merge_toolhead to observe an error.
    pub fn merge_toolhead(
        &mut self,
        local_transform: Transform<D>,
        tool_collider: Box<dyn Shape<D>>,
        tool_mass: f64,
    ) {
        let _ = self.try_merge_toolhead(local_transform, tool_collider, tool_mass);
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
    fn full_bivector_operator_matches_principal_3d_reference_after_rotation() {
        // A diagonal central second-moment matrix produces principal moments
        // Ix=2, Iy=3, Iz=4. In canonical bivector order (01,02,12), the
        // resulting operator is diag(Iz,Iy,Ix) because of the axial mapping
        // [B12,-B02,B01].
        let operator = RotationalInertiaOperator::<3>::from_body_second_moment_matrix(vec![
            2.5, 0.0, 0.0,
            0.0, 1.5, 0.0,
            0.0, 0.0, 0.5,
        ]).unwrap();

        let mut principal = anisotropic_body();
        principal.transform.rotation = Rotor::<3>::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 2),
            0.47,
        );
        principal.angular_velocity.set(0, 1, 0.3);
        principal.angular_velocity.set(0, 2, -0.7);
        principal.angular_velocity.set(1, 2, 0.2);

        let mut full = anisotropic_body()
            .with_rotational_inertia_operator(operator);
        full.transform.rotation = principal.transform.rotation;
        full.angular_velocity = principal.angular_velocity;

        let mut impulse = Bivector::<3>::zero();
        impulse.set(0, 1, 0.8);
        impulse.set(0, 2, -0.4);
        impulse.set(1, 2, 0.6);
        assert_bivector_close(
            &full.apply_inverse_angular_inertia(&impulse),
            &principal.apply_inverse_angular_inertia(&impulse),
            1.0e-11,
        );

        let mut torque = Bivector::<3>::zero();
        torque.set(0, 1, -0.2);
        torque.set(0, 2, 0.5);
        torque.set(1, 2, 0.1);
        assert_bivector_close(
            &full.angular_acceleration_from_torque(&torque),
            &principal.angular_acceleration_from_torque(&torque),
            1.0e-10,
        );
        assert!(
            (full.rotational_kinetic_energy() - principal.rotational_kinetic_energy()).abs()
                < 1.0e-11
        );
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
    fn direct_mass_mutation_invalidates_diagonal_inertia_until_replaced() {
        let mut body = anisotropic_body();
        let original_mass = body.mass;
        body.mass += 0.25;

        assert_eq!(
            body.rotational_inertia_model(),
            RotationalInertiaModel::InvalidPrincipalMoments
        );
        assert_eq!(
            body.apply_inverse_angular_inertia(&Bivector::<3>::unit_plane(0, 1)),
            Bivector::zero()
        );
        assert_eq!(body.rotational_kinetic_energy(), 0.0);

        assert!(body.replace_principal_mass_properties(
            original_mass + 0.25,
            SVector::from([2.0, 3.0, 4.0]),
        ));
        assert_eq!(
            body.rotational_inertia_model(),
            RotationalInertiaModel::PrincipalAxesWorldSpace3D
        );
        assert!(!body.replace_principal_mass_properties(f64::NAN, SVector::from([1.0, 1.0, 1.0])));
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

#[cfg(test)]
mod compound_com_tests {
    use super::*;
    use symtropy_math::{HyperBox, Rotor};

    fn rotated_body() -> RigidBody<3> {
        let mut generator = Bivector::<3>::zero();
        generator.set(0, 1, 0.37);
        generator.set(0, 2, -0.21);
        generator.set(1, 2, 0.48);
        RigidBody::new(
            BodyHandle(10),
            BodyType::Dynamic,
            Transform {
                translation: Point::new([3.0, -2.0, 5.0]),
                rotation: Rotor::from_bivector(&generator),
            },
            Box::new(HyperBox::<3>::new([0.4, 0.2, 0.3])),
            2.0,
            SVector::from_element(0.4),
        )
    }

    fn child_world_support(
        body: &RigidBody<3>,
        child_index: usize,
        direction: &SVector<f64, 3>,
    ) -> SVector<f64, 3> {
        let compound = body
            .collider
            .as_any()
            .downcast_ref::<symtropy_math::CompoundShape<3>>()
            .expect("merge_toolhead always stores a compound collider");
        let (child_transform, shape) = compound
            .children()
            .get(child_index)
            .expect("requested compound child exists");
        let world_from_child = body.transform.compose(child_transform);
        let child_direction = world_from_child.rotation.reverse().rotate_vector(direction);
        let local_support = shape.support(&child_direction);
        world_from_child.transform_point(&Point(local_support)).0
    }

    fn original_child_world_support(
        body: &RigidBody<3>,
        direction: &SVector<f64, 3>,
    ) -> SVector<f64, 3> {
        child_world_support(body, 0, direction)
    }

    fn fibonacci_directions() -> Vec<SVector<f64, 3>> {
        let count = 64;
        let golden_angle = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
        (0..count)
            .map(|index| {
                let index = index as f64;
                let z = 1.0 - 2.0 * (index + 0.5) / count as f64;
                let radius = (1.0 - z * z).sqrt();
                let angle = golden_angle * index;
                SVector::from([radius * angle.cos(), z, radius * angle.sin()])
            })
            .collect()
    }

    fn transformed_shape_world_support(
        shape: &dyn Shape<3>,
        transform: &Transform<3>,
        direction: &SVector<f64, 3>,
    ) -> SVector<f64, 3> {
        let local_direction = transform.rotation.reverse().rotate_vector(direction);
        let local_support = shape.support(&local_direction);
        transform.transform_point(&Point(local_support)).0
    }

    fn assert_child_geometry_preserved(
        before: &[SVector<f64, 3>],
        body: &RigidBody<3>,
        child_index: usize,
    ) {
        let directions = [
            SVector::from([1.0, 0.0, 0.0]),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::from([0.0, 0.0, 1.0]),
            SVector::from([1.0, 2.0, -3.0]),
            SVector::from([-2.0, 0.5, 1.0]),
        ];
        for (index, direction) in directions.iter().enumerate() {
            let after = child_world_support(body, child_index, direction);
            assert!(
                (after - before[index]).norm() < 2.0e-10,
                "compound child {child_index} moved in direction {direction:?}: before={:?}, after={after:?}",
                before[index]
            );
        }
    }

    fn assert_original_geometry_preserved(before: &[SVector<f64, 3>], body: &RigidBody<3>) {
        let directions = [
            SVector::from([1.0, 0.0, 0.0]),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::from([0.0, 0.0, 1.0]),
            SVector::from([1.0, 2.0, -3.0]),
            SVector::from([-2.0, 0.5, 1.0]),
        ];
        for (index, direction) in directions.iter().enumerate() {
            let after = original_child_world_support(body, direction);
            assert!(
                (after - before[index]).norm() < 2.0e-10,
                "original child geometry moved in direction {direction:?}: before={:?}, after={after:?}",
                before[index]
            );
        }
    }

    #[test]
    fn mass_properties_constructor_recenters_body_and_preserves_world_geometry() {
        let mut generator = Bivector::<3>::zero();
        generator.set(0, 1, 0.37);
        generator.set(0, 2, -0.21);
        generator.set(1, 2, 0.48);
        let geometry_transform = Transform {
            translation: Point::new([3.0, -2.0, 5.0]),
            rotation: Rotor::from_bivector(&generator),
        };
        let center_of_mass_local = SVector::new(0.17, -0.08, 0.23);
        let collider: Box<dyn Shape<3>> = Box::new(HyperBox::<3>::new([0.4, 0.2, 0.3]));

        let directions = [
            SVector::from([1.0, 0.0, 0.0]),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::from([0.0, 0.0, 1.0]),
            SVector::from([1.0, 2.0, -3.0]),
            SVector::from([-2.0, 0.5, 1.0]),
        ];
        let expected_support = directions
            .iter()
            .map(|direction| {
                transformed_shape_world_support(collider.as_ref(), &geometry_transform, direction)
            })
            .collect::<Vec<_>>();
        let expected_com_world = geometry_transform.translation.0
            + geometry_transform.rotation.rotate_vector(&center_of_mass_local);

        let operator = RotationalInertiaOperator::<3>::from_body_second_moment_matrix(vec![
            0.003, 0.0002, -0.0001,
            0.0002, 0.002, 0.0003,
            -0.0001, 0.0003, 0.0015,
        ])
        .unwrap();
        let mass_properties =
            MassProperties::try_new(2.5, center_of_mass_local, operator.clone()).unwrap();
        let mut body = RigidBody::<3>::try_new_with_mass_properties(
            BodyHandle(90),
            geometry_transform,
            collider,
            mass_properties,
        )
        .expect("validated COM-bound dynamic body");

        assert!((body.transform.translation.0 - expected_com_world).norm() < 1.0e-12);
        assert_eq!(body.mass, 2.5);
        assert!(body.rotational_inertia_operator.is_some());
        assert!(!body.rotational_inertia_operator_invalidated);
        assert!(!body.rotational_inertia_approximate);
        assert_eq!(body.mass_properties_mass_bits, Some(body.mass.to_bits()));
        let attached = body.rotational_inertia_operator.as_ref().unwrap();
        assert_eq!(attached.as_row_major(), operator.as_row_major());

        for (index, direction) in directions.iter().enumerate() {
            let actual = body.world_support(direction);
            assert!(
                (actual - expected_support[index]).norm() < 2.0e-10,
                "geometry changed after COM attachment for direction {index}: actual={actual:?}, expected={:?}",
                expected_support[index]
            );
        }

        let mut angular = Bivector::<3>::zero();
        angular.set(0, 1, 0.31);
        angular.set(0, 2, -0.47);
        angular.set(1, 2, 0.83);
        body.angular_velocity = angular;
        let expected_energy = operator.kinetic_energy_bivector_joules(&angular).unwrap();
        assert!((body.rotational_kinetic_energy() - expected_energy).abs() < 1.0e-12);
    }

    #[test]
    fn mass_properties_constructor_rejects_improper_geometry_frame() {
        let mut invalid_transform =
            Transform::from_translation(Point::new([1.0, 2.0, 3.0]));
        invalid_transform.rotation =
            Rotor::from_matrix(nalgebra::SMatrix::<f64, 3, 3>::identity() * 2.0);
        let operator = RotationalInertiaOperator::<3>::isotropic(1.0).unwrap();
        let properties = MassProperties::try_new(
            1.0,
            SVector::<f64, 3>::new(0.1, 0.0, 0.0),
            operator,
        )
        .unwrap();

        let result = RigidBody::<3>::try_new_with_mass_properties(
            BodyHandle(91),
            invalid_transform,
            Box::new(HyperBox::<3>::new([0.2, 0.2, 0.2])),
            properties,
        );
        assert_eq!(
            result.err(),
            Some(MassPropertiesBodyError::InvalidGeometryTransform)
        );
    }

    #[test]
    fn merging_toolhead_invalidates_previously_attached_full_inertia_operator() {
        let operator = RotationalInertiaOperator::<3>::from_body_second_moment_matrix(vec![
            0.003, 0.0002, -0.0001,
            0.0002, 0.002, 0.0003,
            -0.0001, 0.0003, 0.0015,
        ])
        .unwrap();
        let mut body = rotated_body().with_rotational_inertia_operator(operator);
        assert!(body.rotational_inertia_operator.is_some());
        assert!(!body.rotational_inertia_operator_invalidated);

        body.try_merge_toolhead(
            Transform::from_translation(Point::new([0.6, -0.2, 0.4])),
            Box::new(HyperBox::<3>::new([0.12, 0.08, 0.06])),
            0.4,
        )
        .expect("finite positive approximate toolhead merge");

        assert!(body.rotational_inertia_operator.is_none());
        assert!(body.rotational_inertia_operator_invalidated);
        assert!(body.rotational_inertia_operator_mass_bits.is_none());
        assert!(body.rotational_inertia_approximate);
        assert_eq!(body.mass_properties_mass_bits, Some(body.mass.to_bits()));
    }

    #[test]
    fn approximate_merge_uses_dimension_aware_mean_parallel_axis_term() {
        let mut body = rotated_body();
        let base_mass = body.mass;
        let tool_mass = 1.0;
        let offset = SVector::from([1.2, -0.7, 0.4]);
        let total_mass = base_mass + tool_mass;
        let base_com_shift = offset * (tool_mass / total_mass);
        let tool_com_shift = offset - base_com_shift;
        let expected_mean_inertia = 0.4
            + (0.4 * tool_mass * 0.1 * 0.1)
            + (2.0 / 3.0) * base_mass * base_com_shift.norm_squared()
            + (2.0 / 3.0) * tool_mass * tool_com_shift.norm_squared();

        body.try_merge_toolhead(
            Transform::from_translation(Point(offset)),
            Box::new(HyperBox::<3>::new([0.2, 0.1, 0.15])),
            tool_mass,
        )
        .expect("valid approximate merge");

        assert!(
            (body.inertia[0] - expected_mean_inertia).abs() < 1.0e-12,
            "mean plane inertia did not use 2m|d|^2/D: actual={}, expected={expected_mean_inertia}",
            body.inertia[0]
        );
        assert_eq!(body.inertia, SVector::from_element(expected_mean_inertia));
    }

    #[test]
    fn invalid_toolhead_mass_is_rejected_without_mutation() {
        let mut body = rotated_body();
        let mass_before = body.mass;
        let translation_before = body.transform.translation.0;
        let velocity_before = body.linear_velocity;
        let force_before = body.force_accumulator;
        let inertia_before = body.inertia;
        let rotation_before = *body.transform.rotation.to_matrix();

        let result = body.try_merge_toolhead(
            Transform::from_translation(Point::new([0.5, 0.2, -0.1])),
            Box::new(HyperBox::<3>::new([0.1, 0.1, 0.1])),
            f64::NAN,
        );
        assert_eq!(result, Err(ToolheadMergeError::InvalidToolMass));
        assert_eq!(body.mass, mass_before);
        assert_eq!(body.transform.translation.0, translation_before);
        assert_eq!(body.linear_velocity, velocity_before);
        assert_eq!(body.force_accumulator, force_before);
        assert_eq!(body.inertia, inertia_before);
        assert_eq!(*body.transform.rotation.to_matrix(), rotation_before);
        assert!(body.collider.as_any().downcast_ref::<symtropy_math::CompoundShape<3>>().is_none());
    }

    #[test]
    fn improper_toolhead_rotation_is_rejected_without_mutation() {
        let mut body = rotated_body();
        let mass_before = body.mass;
        let translation_before = body.transform.translation.0;
        let mut invalid = Transform::from_translation(Point::new([0.5, 0.2, -0.1]));
        invalid.rotation = Rotor::from_matrix(nalgebra::SMatrix::<f64, 3, 3>::identity() * 2.0);

        let result = body.try_merge_toolhead(
            invalid,
            Box::new(HyperBox::<3>::new([0.1, 0.1, 0.1])),
            0.5,
        );
        assert_eq!(result, Err(ToolheadMergeError::InvalidTransform));
        assert_eq!(body.mass, mass_before);
        assert_eq!(body.transform.translation.0, translation_before);
        assert!(body.collider.as_any().downcast_ref::<symtropy_math::CompoundShape<3>>().is_none());
    }

    #[test]
    fn merge_toolhead_preserves_world_geometry_for_rotated_body() {
        let mut body = rotated_body();
        let directions = [
            SVector::from([1.0, 0.0, 0.0]),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::from([0.0, 0.0, 1.0]),
            SVector::from([1.0, 2.0, -3.0]),
            SVector::from([-2.0, 0.5, 1.0]),
        ];
        let before = directions
            .iter()
            .map(|direction| body.world_support(direction))
            .collect::<Vec<_>>();

        let mut tool_generator = Bivector::<3>::zero();
        tool_generator.set(0, 2, 0.31);
        let tool_transform = Transform {
            translation: Point::new([1.2, -0.7, 0.4]),
            rotation: Rotor::from_bivector(&tool_generator),
        };
        let expected_local_com = tool_transform.translation.0 / 3.0;
        let old_translation = body.transform.translation.0;
        let old_rotation = body.transform.rotation.clone();

        body.merge_toolhead(
            tool_transform,
            Box::new(HyperBox::<3>::new([0.2, 0.1, 0.15])),
            1.0,
        );

        let expected_world_translation =
            old_translation + old_rotation.rotate_vector(&expected_local_com);
        assert!(
            (body.transform.translation.0 - expected_world_translation).norm() < 1.0e-12,
            "COM translation was not rotated from body frame into world frame"
        );
        assert_original_geometry_preserved(&before, &body);
    }

    #[test]
    fn merge_toolhead_preserves_rotated_box_children_over_direction_sweep() {
        let mut body = rotated_body();
        let directions = fibonacci_directions();
        let original_before = directions
            .iter()
            .map(|direction| body.world_support(direction))
            .collect::<Vec<_>>();

        let mut tool_generator = Bivector::<3>::zero();
        tool_generator.set(0, 1, -0.29);
        tool_generator.set(0, 2, 0.44);
        tool_generator.set(1, 2, 0.18);
        let tool_transform = Transform {
            translation: Point::new([0.8, -0.6, 0.5]),
            rotation: Rotor::from_bivector(&tool_generator),
        };
        let world_tool_transform_before = body.transform.compose(&tool_transform);
        let tool_shape: Box<dyn Shape<3>> = Box::new(HyperBox::<3>::new([0.23, 0.11, 0.07]));
        let tool_before = directions
            .iter()
            .map(|direction| {
                transformed_shape_world_support(
                    tool_shape.as_ref(),
                    &world_tool_transform_before,
                    direction,
                )
            })
            .collect::<Vec<_>>();

        body.merge_toolhead(tool_transform, tool_shape, 0.8);

        for (index, direction) in directions.iter().enumerate() {
            let original_after = child_world_support(&body, 0, direction);
            let tool_after = child_world_support(&body, 1, direction);
            assert!(
                (original_after - original_before[index]).norm() < 3.0e-10,
                "original rotated box support changed at direction {index}"
            );
            assert!(
                (tool_after - tool_before[index]).norm() < 3.0e-10,
                "rotated toolhead support changed at direction {index}"
            );
        }
    }

    #[test]
    fn merge_toolhead_preserves_velocity_at_new_com_reference() {
        let mut body = rotated_body();
        body.linear_velocity = SVector::from([0.6, -0.3, 0.8]);
        let mut omega = Bivector::<3>::zero();
        omega.set(0, 1, 0.73);
        omega.set(0, 2, -0.41);
        omega.set(1, 2, 0.29);
        body.angular_velocity = omega;

        let tool_transform = Transform::from_translation(Point::new([1.2, -0.7, 0.4]));
        let local_com_offset = tool_transform.translation.0 / 3.0;
        let old_translation = body.transform.translation.0;
        let old_rotation = body.transform.rotation.clone();
        let old_linear_velocity = body.linear_velocity;
        let dt = 1.0e-7;

        // Finite-difference the world trajectory of the point that becomes
        // the combined COM, using the exact Rotor update used by integrate().
        let point_before = old_translation + old_rotation.rotate_vector(&local_com_offset);
        let delta_rotation = Rotor::from_bivector(&omega.scale(dt));
        let rotation_after = delta_rotation.compose(&old_rotation);
        let point_after = old_translation
            + old_linear_velocity * dt
            + rotation_after.rotate_vector(&local_com_offset);
        let finite_difference_velocity = (point_after - point_before) / dt;

        body.merge_toolhead(
            tool_transform,
            Box::new(HyperBox::<3>::new([0.2, 0.1, 0.15])),
            1.0,
        );

        assert!(
            (body.linear_velocity - finite_difference_velocity).norm() < 2.0e-6,
            "new COM velocity disagrees with Rotor point-velocity finite difference: actual={:?}, expected={finite_difference_velocity:?}",
            body.linear_velocity
        );
    }

    #[test]
    fn merge_toolhead_preserves_accumulated_wrench_about_fixed_world_point() {
        let mut body = rotated_body();
        body.force_accumulator = SVector::from([0.8, -1.1, 0.6]);
        let mut torque = Bivector::<3>::zero();
        torque.set(0, 1, 0.7);
        torque.set(0, 2, -0.4);
        torque.set(1, 2, 0.2);
        body.torque_accumulator = torque;

        let fixed_world_point = SVector::from([-1.0, 0.5, 2.0]);
        let old_origin = body.transform.translation.0;
        let old_force = body.force_accumulator;
        let torque_about_fixed_before = body.torque_accumulator.add(&Bivector::from_wedge(
            &(old_origin - fixed_world_point),
            &old_force,
        ));

        body.merge_toolhead(
            Transform::from_translation(Point::new([1.2, -0.7, 0.4])),
            Box::new(HyperBox::<3>::new([0.2, 0.1, 0.15])),
            1.0,
        );

        let new_origin = body.transform.translation.0;
        let torque_about_fixed_after = body.torque_accumulator.add(&Bivector::from_wedge(
            &(new_origin - fixed_world_point),
            &body.force_accumulator,
        ));
        for i in 0..3 {
            for j in (i + 1)..3 {
                assert!(
                    (torque_about_fixed_after.get(i, j) - torque_about_fixed_before.get(i, j))
                        .abs()
                        < 1.0e-12,
                    "wrench moment about fixed world point changed in plane ({i},{j})"
                );
            }
        }
        assert_eq!(body.force_accumulator, old_force);
    }

    #[test]
    fn repeated_toolhead_merges_preserve_all_prior_world_geometry() {
        let mut body = rotated_body();
        let directions = [
            SVector::from([1.0, 0.0, 0.0]),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::from([0.0, 0.0, 1.0]),
            SVector::from([1.0, 2.0, -3.0]),
            SVector::from([-2.0, 0.5, 1.0]),
        ];
        let initial = directions
            .iter()
            .map(|direction| body.world_support(direction))
            .collect::<Vec<_>>();

        body.merge_toolhead(
            Transform::from_translation(Point::new([1.0, 0.3, -0.2])),
            Box::new(HyperBox::<3>::new([0.15, 0.1, 0.08])),
            0.7,
        );
        assert_original_geometry_preserved(&initial, &body);

        let after_first_original = directions
            .iter()
            .map(|direction| original_child_world_support(&body, direction))
            .collect::<Vec<_>>();
        let after_first_tool = directions
            .iter()
            .map(|direction| child_world_support(&body, 1, direction))
            .collect::<Vec<_>>();
        body.merge_toolhead(
            Transform::from_translation(Point::new([-0.4, 1.1, 0.6])),
            Box::new(HyperBox::<3>::new([0.1, 0.07, 0.04])),
            0.3,
        );
        assert_child_geometry_preserved(&after_first_original, &body, 0);
        assert_child_geometry_preserved(&after_first_tool, &body, 1);
        assert_original_geometry_preserved(&initial, &body);
    }
}
