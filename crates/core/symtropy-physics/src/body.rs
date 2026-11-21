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

/// Invariant violation in the canonical mass values or their reciprocal caches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MassPropertyInvariantError {
    NonFiniteMass,
    NegativeMass,
    DynamicMassNotPositive,
    NonFiniteInverseMass,
    NegativeInverseMass,
    InverseMassMismatch,
    NonFiniteInertia { axis: usize },
    NegativeInertia { axis: usize },
    DynamicInertiaNotPositive { axis: usize },
    NonFiniteInverseInertia { axis: usize },
    NegativeInverseInertia { axis: usize },
    InverseInertiaMismatch { axis: usize },
}

fn reciprocal_matches(actual: f64, canonical: f64) -> bool {
    let expected = if canonical > 0.0 { 1.0 / canonical } else { 0.0 };
    if !expected.is_finite() {
        return false;
    }
    let scale = actual.abs().max(expected.abs());
    scale == 0.0 || (actual - expected).abs() <= 32.0 * f64::EPSILON * scale
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

    /// Validate canonical mass/inertia values against their derived inverse caches.
    ///
    /// `mass` and `inertia` are treated as canonical. This method deliberately
    /// rejects stale inverse caches rather than silently repairing them, so an
    /// inconsistent body cannot enter a checked physics step.
    pub fn validate_mass_properties(&self) -> Result<(), MassPropertyInvariantError> {
        if !self.mass.is_finite() {
            return Err(MassPropertyInvariantError::NonFiniteMass);
        }
        if self.mass < 0.0 {
            return Err(MassPropertyInvariantError::NegativeMass);
        }
        if self.is_dynamic() && self.mass <= 0.0 {
            return Err(MassPropertyInvariantError::DynamicMassNotPositive);
        }
        if !self.inv_mass.is_finite() {
            return Err(MassPropertyInvariantError::NonFiniteInverseMass);
        }
        if self.inv_mass < 0.0 {
            return Err(MassPropertyInvariantError::NegativeInverseMass);
        }
        if !reciprocal_matches(self.inv_mass, self.mass) {
            return Err(MassPropertyInvariantError::InverseMassMismatch);
        }

        for axis in 0..D {
            let inertia = self.inertia[axis];
            let inverse = self.inv_inertia[axis];
            if !inertia.is_finite() {
                return Err(MassPropertyInvariantError::NonFiniteInertia { axis });
            }
            if inertia < 0.0 {
                return Err(MassPropertyInvariantError::NegativeInertia { axis });
            }
            if self.is_dynamic() && inertia <= 0.0 {
                return Err(MassPropertyInvariantError::DynamicInertiaNotPositive { axis });
            }
            if !inverse.is_finite() {
                return Err(MassPropertyInvariantError::NonFiniteInverseInertia { axis });
            }
            if inverse < 0.0 {
                return Err(MassPropertyInvariantError::NegativeInverseInertia { axis });
            }
            if !reciprocal_matches(inverse, inertia) {
                return Err(MassPropertyInvariantError::InverseInertiaMismatch { axis });
            }
        }
        Ok(())
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


#[cfg(test)]
mod mass_property_invariant_tests {
    use super::*;

    fn dynamic_body() -> RigidBody<3> {
        RigidBody::dynamic_sphere(BodyHandle(7), Point::origin(), 0.25, 2.0)
    }

    #[test]
    fn constructor_derives_consistent_inverse_mass_properties() {
        let body = dynamic_body();
        assert_eq!(body.validate_mass_properties(), Ok(()));
    }

    #[test]
    fn static_zero_mass_and_inertia_are_valid() {
        let body = RigidBody::<3>::static_body(
            BodyHandle(1),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.5)),
        );
        assert_eq!(body.validate_mass_properties(), Ok(()));
    }

    #[test]
    fn detects_stale_inverse_mass_cache() {
        let mut body = dynamic_body();
        body.inv_mass *= 2.0;
        assert_eq!(
            body.validate_mass_properties(),
            Err(MassPropertyInvariantError::InverseMassMismatch)
        );
    }

    #[test]
    fn detects_stale_inverse_inertia_cache_with_axis() {
        let mut body = dynamic_body();
        body.inv_inertia[1] *= 2.0;
        assert_eq!(
            body.validate_mass_properties(),
            Err(MassPropertyInvariantError::InverseInertiaMismatch { axis: 1 })
        );
    }

    #[test]
    fn rejects_nonfinite_and_nonpositive_dynamic_mass_properties() {
        let mut nonfinite_mass = dynamic_body();
        nonfinite_mass.mass = f64::NAN;
        assert_eq!(
            nonfinite_mass.validate_mass_properties(),
            Err(MassPropertyInvariantError::NonFiniteMass)
        );

        let mut zero_mass = dynamic_body();
        zero_mass.mass = 0.0;
        zero_mass.inv_mass = 0.0;
        assert_eq!(
            zero_mass.validate_mass_properties(),
            Err(MassPropertyInvariantError::DynamicMassNotPositive)
        );

        let mut nonfinite_inertia = dynamic_body();
        nonfinite_inertia.inertia[2] = f64::INFINITY;
        assert_eq!(
            nonfinite_inertia.validate_mass_properties(),
            Err(MassPropertyInvariantError::NonFiniteInertia { axis: 2 })
        );

        let mut zero_inertia = dynamic_body();
        zero_inertia.inertia[0] = 0.0;
        zero_inertia.inv_inertia[0] = 0.0;
        assert_eq!(
            zero_inertia.validate_mass_properties(),
            Err(MassPropertyInvariantError::DynamicInertiaNotPositive { axis: 0 })
        );
    }

    #[test]
    fn rejects_negative_or_nonfinite_inverse_cache_values() {
        let mut negative_inverse_mass = dynamic_body();
        negative_inverse_mass.inv_mass = -1.0;
        assert_eq!(
            negative_inverse_mass.validate_mass_properties(),
            Err(MassPropertyInvariantError::NegativeInverseMass)
        );

        let mut nonfinite_inverse_inertia = dynamic_body();
        nonfinite_inverse_inertia.inv_inertia[0] = f64::INFINITY;
        assert_eq!(
            nonfinite_inverse_inertia.validate_mass_properties(),
            Err(MassPropertyInvariantError::NonFiniteInverseInertia { axis: 0 })
        );
    }
}
