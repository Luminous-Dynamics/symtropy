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

    /// Accumulate a world-space force on the body.
    pub fn apply_force(&mut self, force: SVector<f64, D>) {
        if self.is_dynamic() {
            self.force_accumulator += force;
            self.wake();
        }
    }

    /// Accumulate a world-space torque about the body's current reference COM.
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
        let i_avg = self.inertia.sum() / D as f64;
        let angular = 0.5 * i_avg * self.angular_velocity.norm_squared();
        linear + angular
    }

    /// Merge a tool collider expressed relative to the current body frame.
    ///
    /// The body origin is moved to the combined centre of mass. Child collider
    /// transforms are therefore re-expressed about the new origin, and the
    /// world-space translation is rotated by the body's current orientation.
    /// This preserves the compound's world-space geometry during recentering.
    pub fn merge_toolhead(
        &mut self,
        local_transform: Transform<D>,
        tool_collider: Box<dyn Shape<D>>,
        tool_mass: f64,
    ) {
        use symtropy_math::CompoundShape;

        let total_mass = self.mass + tool_mass;
        let com_offset = (local_transform.translation.0 * tool_mass) / total_mass;

        // Re-express every existing child relative to the new COM. Cloning the
        // children into a fresh CompoundShape avoids mutating through the
        // immutable children() view.
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

        let mut shifted_tool_transform = local_transform.clone();
        shifted_tool_transform.translation.0 -= com_offset;
        compound.add_child(shifted_tool_transform, tool_collider);
        self.collider = Box::new(compound);

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

        // The linear velocity and accumulated wrench belong to the body's
        // current world-space reference point. Moving that point by a
        // body-local COM offset changes both.
        // Rotor::from_bivector uses exp(-B), so point velocity is -B*r.
        let world_com_offset = self.transform.rotation.rotate_vector(&com_offset);
        let torque_shift = Bivector::from_wedge(&world_com_offset, &self.force_accumulator);
        self.torque_accumulator = self.torque_accumulator.add(&torque_shift.scale(-1.0));
        self.linear_velocity -= self.angular_velocity.apply_to_vector(&world_com_offset);
        self.transform.translation.0 += world_com_offset;
    }

    pub fn world_support(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        let local_dir = self.transform.rotation.reverse().rotate_vector(direction);
        let local_support = self.collider.support(&local_dir);
        self.transform.transform_point(&Point(local_support)).0
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
        let tool_shape: Box<dyn Shape<3>> =
            Box::new(HyperBox::<3>::new([0.23, 0.11, 0.07]));
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
