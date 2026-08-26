// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::SVector;
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

/// Failures produced while interpreting live `RigidBody<3>` state through the
/// checked principal-inertia reference model.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RigidBodyEnergyError {
    InvalidMass,
    NonFiniteLinearVelocity,
    UnrepresentableLinearEnergy,
    Angular(crate::angular_dynamics::AngularDynamicsError),
    UnrepresentableTotalEnergy,
}

impl From<crate::angular_dynamics::AngularDynamicsError> for RigidBodyEnergyError {
    fn from(value: crate::angular_dynamics::AngularDynamicsError) -> Self {
        Self::Angular(value)
    }
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
    pub inertia: SVector<f64, D>,
    pub inv_inertia: SVector<f64, D>,
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
        let inv_inertia = inertia.map(|i| if i > 0.0 { 1.0 / i } else { 0.0 });

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

    /// Historical N-D compatibility energy.
    ///
    /// In 3D this still collapses principal inertia to a scalar mean. Use
    /// [`RigidBody::<3>::kinetic_energy_3d_checked`] when exact anisotropic 3D
    /// energy is required for validation or reservoir reconciliation.
    pub fn kinetic_energy(&self) -> f64 {
        if !self.is_dynamic() {
            return 0.0;
        }
        let linear = 0.5 * self.mass * self.linear_velocity.norm_squared();
        let i_avg = self.inertia.sum() / D as f64;
        let angular = 0.5 * i_avg * self.angular_velocity.norm_squared();
        linear + angular
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
    }

    pub fn world_support(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        let local_dir = self.transform.rotation.reverse().rotate_vector(direction);
        let local_support = self.collider.support(&local_dir);
        self.transform.transform_point(&Point(local_support)).0
    }
}

impl RigidBody<3> {
    /// Interpret the body's three stored inertia components as body-space
    /// principal moments under the checked #35 asymmetric-top convention.
    pub fn principal_inertia_3_checked(
        &self,
    ) -> Result<crate::angular_dynamics::PrincipalInertia3, RigidBodyEnergyError> {
        Ok(crate::angular_dynamics::PrincipalInertia3::new([
            self.inertia[0],
            self.inertia[1],
            self.inertia[2],
        ])?)
    }

    /// Exact checked 3D kinetic energy for the body's current represented state.
    ///
    /// This is a validation/reference path, not yet the generic production hot
    /// path. Dynamic bodies use exact linear kinetic energy plus the #35
    /// anisotropic body-space principal-inertia rotational energy. Static and
    /// kinematic bodies return zero, matching the existing compatibility API.
    pub fn kinetic_energy_3d_checked(&self) -> Result<f64, RigidBodyEnergyError> {
        if !self.is_dynamic() {
            return Ok(0.0);
        }
        if !self.mass.is_finite() || self.mass <= 0.0 {
            return Err(RigidBodyEnergyError::InvalidMass);
        }
        if !self.linear_velocity.iter().all(|value| value.is_finite()) {
            return Err(RigidBodyEnergyError::NonFiniteLinearVelocity);
        }

        let speed_squared = self.linear_velocity.norm_squared();
        let linear = 0.5 * self.mass * speed_squared;
        if !speed_squared.is_finite() || !linear.is_finite() || linear < 0.0 {
            return Err(RigidBodyEnergyError::UnrepresentableLinearEnergy);
        }

        let inertia = self.principal_inertia_3_checked()?;
        let angular = crate::angular_dynamics::rotational_kinetic_energy(
            &self.transform.rotation,
            &self.angular_velocity,
            inertia,
        )?;
        let total = linear + angular;
        if !total.is_finite() || total < 0.0 {
            return Err(RigidBodyEnergyError::UnrepresentableTotalEnergy);
        }
        Ok(total)
    }
}
