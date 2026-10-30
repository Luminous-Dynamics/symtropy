// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use crate::inertia::BivectorInertia;
use nalgebra::SVector;
use serde::{Deserialize, Serialize};
use symtropy_math::{Bivector, Point, Rotor, Shape, Transform};

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
    /// Legacy per-axis inertia approximation retained for API compatibility.
    /// When `rotational_inertia` is Some, the bivector operator is authoritative.
    pub inv_inertia: SVector<f64, D>,
    /// Optional body-space inertia operator on the full bivector rotation space.
    pub rotational_inertia: Option<BivectorInertia<D>>,
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
            rotational_inertia: None,
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

    /// Construct a dynamic body with a uniformly-filled D-dimensional ball collider.
    ///
    /// For a uniform solid D-ball, the moment of inertia in each independent
    /// rotation plane is 2*m*r^2/(D+2). This reduces to the familiar 2D disk,
    /// 3D sphere, and 4D hypersphere coefficients without applying the 3D
    /// coefficient to every dimension.
    pub fn dynamic_sphere(handle: BodyHandle, position: Point<D>, radius: f64, mass: f64) -> Self {
        use symtropy_math::Sphere;
        let inertia = (2.0 / (D as f64 + 2.0)) * mass * radius * radius;
        let mut body = Self::new(
            handle,
            BodyType::Dynamic,
            Transform::from_translation(position),
            Box::new(Sphere::new(Point::origin(), radius)),
            mass,
            SVector::from_element(inertia),
        );
        if mass.is_finite() && mass > 0.0 && radius.is_finite() && radius > 0.0 {
            body.rotational_inertia = BivectorInertia::uniform_solid_ball(mass, radius).ok();
        }
        body
    }

    /// Set the authoritative body-space rotational inertia operator.
    ///
    /// The supplied mass properties must be centered at this body's center of
    /// mass and expressed in its local body frame. Simulation and contact paths
    /// rotate the operator into world coordinates as required.
    pub fn set_rotational_inertia_operator(&mut self, inertia: BivectorInertia<D>) {
        self.rotational_inertia = Some(inertia);
        self.wake();
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

    /// Accumulate a physical torque in the impulse-first bivector convention.
    ///
    /// The rigid-body integrator converts this to the stored angular-state sign
    /// required by Rotor::from_bivector's exp(-B) update.
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

    pub fn kinetic_energy(&self) -> f64 {
        if !self.is_dynamic() {
            return 0.0;
        }
        let linear = 0.5 * self.mass * self.linear_velocity.norm_squared();
        let angular = if let Some(inertia) = &self.rotational_inertia {
            inertia
                .rotated_by(&self.transform.rotation)
                .kinetic_energy(&self.angular_velocity)
        } else {
            let i_avg = self.inertia.sum() / D as f64;
            0.5 * i_avg * self.angular_velocity.norm_squared()
        };
        linear + angular
    }

    pub fn merge_toolhead(
        &mut self,
        local_transform: Transform<D>,
        tool_collider: Box<dyn Shape<D>>,
        tool_mass: f64,
    ) {
        use symtropy_math::CompoundShape;

        let total_mass = self.mass + tool_mass;
        if !self.mass.is_finite()
            || !tool_mass.is_finite()
            || self.mass < 0.0
            || tool_mass < 0.0
            || !total_mass.is_finite()
            || total_mass <= 0.0
            || !local_transform.translation.0.iter().all(|value| value.is_finite())
        {
            return;
        }

        let tool_local_translation = local_transform.translation.0;
        let com_offset = (tool_local_translation * tool_mass) / total_mass;
        let world_com_offset = self.transform.rotation.rotate_vector(&com_offset);

        // Combine authoritative mass properties around the new center of mass.
        let combined_rotational_inertia = self.rotational_inertia.as_ref().and_then(|body_inertia| {
            let tool_inertia = BivectorInertia::<D>::uniform_solid_ball(tool_mass, 0.1).ok()?;
            let body_offset = -com_offset;
            let tool_offset = tool_local_translation - com_offset;
            BivectorInertia::combine_at_common_center(
                self.mass,
                body_inertia,
                &body_offset,
                tool_mass,
                &tool_inertia,
                &tool_offset,
            )
            .ok()
        });

        // Rebase every collider child to the new local COM. The parent origin
        // will move by R*com_offset in world coordinates, so subtracting the
        // same local offset from each child leaves all world-space geometry
        // exactly where it was before the mass-property merge.
        let mut compound =
            if let Some(existing) = self.collider.as_any().downcast_ref::<CompoundShape<D>>() {
                let mut new_c = CompoundShape::<D>::new();
                for (transform, shape) in existing.children() {
                    let mut shifted = transform.clone();
                    shifted.translation.0 -= com_offset;
                    new_c.add_child(shifted, shape.clone_box());
                }
                new_c
            } else {
                let mut new_c = CompoundShape::<D>::new();
                let mut existing_transform = Transform::identity();
                existing_transform.translation.0 = -com_offset;
                new_c.add_child(existing_transform, self.collider.as_ref().clone_box());
                new_c
            };

        let mut shifted_tool_transform = local_transform;
        shifted_tool_transform.translation.0 -= com_offset;
        compound.add_child(shifted_tool_transform, tool_collider);
        self.collider = Box::new(compound);

        // Keep legacy per-axis fields as a compatibility approximation. The
        // second-moment operator is authoritative whenever both components
        // supplied validated mass properties.
        let i_original = self.inertia.sum() / D as f64;
        let i_tool = (2.0 / (D as f64 + 2.0)) * tool_mass * 0.1 * 0.1;
        let d_original = com_offset.norm();
        let d_tool = (tool_local_translation - com_offset).norm();
        let i_total = (i_original + self.mass * d_original * d_original)
            + (i_tool + tool_mass * d_tool * d_tool);

        self.mass = total_mass;
        self.inv_mass = 1.0 / total_mass;
        self.inertia = SVector::from_element(i_total);
        self.inv_inertia = SVector::from_element(1.0 / i_total);
        self.rotational_inertia = combined_rotational_inertia;
        self.transform.translation.0 += world_com_offset;
    }

    pub fn world_support(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        let local_dir = self.transform.rotation.reverse().rotate_vector(direction);
        let local_support = self.collider.support(&local_dir);
        self.transform.transform_point(&Point(local_support)).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_uniform_ball_inertia<const D: usize>(expected: f64) {
        let body = RigidBody::<D>::dynamic_sphere(BodyHandle(0), Point::origin(), 2.0, 3.0);
        for (axis, value) in body.inertia.iter().enumerate() {
            assert!(
                (*value - expected).abs() < 1e-12,
                "D={D} axis={axis}: inertia={value}, expected={expected}"
            );
        }
    }

    #[test]
    fn merge_toolhead_preserves_world_geometry_under_parent_rotation() {
        let mut body = RigidBody::<3>::dynamic_sphere(
            BodyHandle(0),
            Point::new([3.0, -2.0, 1.0]),
            1.0,
            2.0,
        );
        body.transform.rotation = Rotor::from_plane_angle(
            &Bivector::<3>::unit_plane(0, 1),
            0.73,
        );

        let local_negative_x = SVector::new(-1.0, 0.0, 0.0);
        let world_direction = body.transform.rotation.rotate_vector(&local_negative_x);
        let before = body.world_support(&world_direction);

        let tool_collider = Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1));
        let tool_transform = Transform::from_translation(Point::new([2.0, 0.0, 0.0]));
        body.merge_toolhead(tool_transform, tool_collider, 1.0);

        let after = body.world_support(&world_direction);
        assert!(
            (after - before).norm() < 1e-12,
            "merge changed the original body's world-space support point: before={before:?}, after={after:?}"
        );
    }

    #[test]
    fn merge_toolhead_uses_second_moment_parallel_axis_rule() {
        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 2.0);
        let tool_collider = Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1));
        let tool_transform = Transform::from_translation(Point::new([2.0, 0.0, 0.0]));
        body.merge_toolhead(tool_transform, tool_collider, 1.0);

        let inertia = body.rotational_inertia.as_ref().unwrap();
        let expected_xx = 0.4 + 2.0 * (4.0 / 9.0) + 0.002 + 16.0 / 9.0;
        let expected_yy = 0.4 + 0.002;
        assert!((inertia.second_moment()[(0, 0)] - expected_xx).abs() < 1e-12);
        assert!((inertia.second_moment()[(1, 1)] - expected_yy).abs() < 1e-12);
        assert!((inertia.second_moment()[(2, 2)] - expected_yy).abs() < 1e-12);
        assert!((body.mass - 3.0).abs() < 1e-12);
        assert!((body.position()[0] - 2.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn uniform_ball_inertia_uses_the_spatial_dimension() {
        // I_plane = 2*m*r^2/(D+2), with m=3 and r=2.
        assert_uniform_ball_inertia::<2>(6.0);
        assert_uniform_ball_inertia::<3>(4.8);
        assert_uniform_ball_inertia::<4>(4.0);
    }
}
