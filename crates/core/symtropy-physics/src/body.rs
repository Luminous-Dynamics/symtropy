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

        // com_offset is body-local; transform it into world coordinates
        // before moving the body's reference point.
        self.transform.translation.0 += self.transform.rotation.rotate_vector(&com_offset);
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
    use symtropy_math::{Rotor, Sphere};

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
            Box::new(Sphere::new(Point::origin(), 0.4)),
            2.0,
            SVector::from_element(0.4),
        )
    }

    fn original_child_world_support(
        body: &RigidBody<3>,
        direction: &SVector<f64, 3>,
    ) -> SVector<f64, 3> {
        let compound = body
            .collider
            .as_any()
            .downcast_ref::<symtropy_math::CompoundShape<3>>()
            .expect("merge_toolhead always stores a compound collider");
        let (child_transform, shape) = compound
            .children()
            .first()
            .expect("original collider remains the first compound child");
        let world_from_child = body.transform.compose(child_transform);
        let child_direction = world_from_child
            .rotation
            .reverse()
            .rotate_vector(direction);
        let local_support = shape.support(&child_direction);
        world_from_child.transform_point(&Point(local_support)).0
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
        let old_rotation = body.transform.rotation;

        body.merge_toolhead(
            tool_transform,
            Box::new(Sphere::new(Point::origin(), 0.2)),
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
            Box::new(Sphere::new(Point::origin(), 0.15)),
            0.7,
        );
        assert_original_geometry_preserved(&initial, &body);

        let after_first = directions
            .iter()
            .map(|direction| body.world_support(direction))
            .collect::<Vec<_>>();
        body.merge_toolhead(
            Transform::from_translation(Point::new([-0.4, 1.1, 0.6])),
            Box::new(Sphere::new(Point::origin(), 0.1)),
            0.3,
        );
        assert_original_geometry_preserved(&after_first, &body);
        assert_support_preserved(&initial, &body);
    }
}