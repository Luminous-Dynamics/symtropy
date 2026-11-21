// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Articulated chain builder for serial kinematic chains.
//!
//! Constructs a series of rigid bodies connected by hinge joints with
//! optional motor drives. Used by robotics platforms (manipulator arm,
//! quadruped legs, exoskeleton limbs).

use nalgebra::SVector;
use std::error::Error;
use std::fmt::{Display, Formatter};
use symtropy_math::Point;

use crate::body::{BodyHandle, RigidBody};
use crate::joints::{HingeJoint, MotorDrive};
use crate::world::PhysicsWorld;

/// Specification for a single link in an articulated chain.
#[derive(Clone, Debug)]
pub struct LinkSpec {
    pub mass: f64,
    pub length: f64,
    pub radius: f64,
    pub plane_a: usize,
    pub plane_b: usize,
    pub angle_limits: Option<(f64, f64)>,
    pub motor_max_force: Option<f64>,
    pub motor_damping: Option<f64>,
}

impl Default for LinkSpec {
    fn default() -> Self {
        Self {
            mass: 1.0,
            length: 0.3,
            radius: 0.03,
            plane_a: 0,
            plane_b: 2,
            angle_limits: Some((-2.9, 2.9)),
            motor_max_force: Some(50.0),
            motor_damping: None,
        }
    }
}

/// Error returned when a chain specification cannot be placed geometrically.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainBuildError {
    InvalidLinkLength { index: usize },
}

impl Display for ChainBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLinkLength { index } => write!(
                formatter,
                "link {index} length must be finite and strictly positive"
            ),
        }
    }
}

impl Error for ChainBuildError {}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LinkPlacement {
    center_step: f64,
    previous_body_anchor: f64,
    current_body_anchor: f64,
}

fn compute_link_placements(lengths: &[f64]) -> Result<Vec<LinkPlacement>, ChainBuildError> {
    let mut placements = Vec::with_capacity(lengths.len());
    let mut previous_length = None;

    for (index, &length) in lengths.iter().enumerate() {
        if !length.is_finite() || length <= 0.0 {
            return Err(ChainBuildError::InvalidLinkLength { index });
        }

        let (center_step, previous_body_anchor) = match previous_length {
            Some(previous) => ((previous + length) * 0.5, -previous * 0.5),
            None => (length, -length * 0.5),
        };

        placements.push(LinkPlacement {
            center_step,
            previous_body_anchor,
            current_body_anchor: length * 0.5,
        });
        previous_length = Some(length);
    }

    Ok(placements)
}

/// An articulated chain in a physics world.
#[derive(Debug)]
pub struct ArticulatedChain {
    pub base: BodyHandle,
    pub links: Vec<BodyHandle>,
    pub num_joints: usize,
}

impl ArticulatedChain {
    /// Read approximate joint angles from relative body positions.
    pub fn read_joint_states(&self, world: &PhysicsWorld<3>) -> Vec<(f64, f64)> {
        let mut states = Vec::with_capacity(self.num_joints);
        let mut prev = self.base;
        for &link in &self.links {
            let (angle, vel) = match (world.body(prev), world.body(link)) {
                (Some(a), Some(b)) => {
                    let d = b.transform.translation.0 - a.transform.translation.0;
                    (
                        d[2].atan2(d[0]),
                        b.angular_velocity.get(0, 2) - a.angular_velocity.get(0, 2),
                    )
                }
                _ => (0.0, 0.0),
            };
            states.push((angle, vel));
            prev = link;
        }
        states
    }

    /// Get the tip (last link) body handle.
    pub fn tip(&self) -> BodyHandle {
        *self.links.last().unwrap_or(&self.base)
    }
}

/// Builder for articulated chains.
pub struct ChainBuilder {
    base_pos: Point<3>,
    links: Vec<LinkSpec>,
}

impl ChainBuilder {
    pub fn new() -> Self {
        Self {
            base_pos: Point::origin(),
            links: Vec::new(),
        }
    }

    pub fn base_position(mut self, pos: Point<3>) -> Self {
        self.base_pos = pos;
        self
    }

    pub fn add_link(mut self, spec: LinkSpec) -> Self {
        self.links.push(spec);
        self
    }

    /// Build the chain in the physics world.
    ///
    /// Panics when a link length is non-finite or non-positive. Use
    /// `try_build` when specifications are not trusted.
    pub fn build(self, world: &mut PhysicsWorld<3>) -> ArticulatedChain {
        self.try_build(world)
            .expect("ChainBuilder::build requires finite, positive link lengths")
    }

    /// Validate and build the chain without partially mutating the world on an
    /// invalid link length.
    pub fn try_build(
        self,
        world: &mut PhysicsWorld<3>,
    ) -> Result<ArticulatedChain, ChainBuildError> {
        let lengths: Vec<f64> = self.links.iter().map(|spec| spec.length).collect();
        let placements = compute_link_placements(&lengths)?;

        // Static base (small sphere)
        let base = world.add_body(RigidBody::static_body(
            BodyHandle(0), // overwritten by add_body
            self.base_pos,
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.05)),
        ));

        let mut links = Vec::with_capacity(self.links.len());
        let mut prev = base;
        let mut pos = self.base_pos.0;

        for (spec, placement) in self.links.iter().zip(&placements) {
            pos[2] -= placement.center_step;
            let link_pos = Point::new([pos[0], pos[1], pos[2]]);
            let handle = world.add_body(RigidBody::dynamic_sphere(
                BodyHandle(0),
                link_pos,
                spec.radius.max(0.01),
                spec.mass,
            ));

            let anchor_a: SVector<f64, 3> =
                SVector::from([0.0, 0.0, placement.previous_body_anchor]);
            let anchor_b: SVector<f64, 3> =
                SVector::from([0.0, 0.0, placement.current_body_anchor]);

            let mut hinge = HingeJoint::with_anchors(
                prev,
                handle,
                anchor_a,
                anchor_b,
                spec.plane_a,
                spec.plane_b,
            );
            if let Some((min, max)) = spec.angle_limits {
                hinge = hinge.with_limits(min, max);
            }
            if let Some(max_force) = spec.motor_max_force {
                let mut motor = MotorDrive::new(0.0, max_force);
                if let Some(d) = spec.motor_damping {
                    motor.kd = d;
                }
                hinge = hinge.with_motor(motor);
            }

            world.add_constraint(Box::new(hinge));
            links.push(handle);
            prev = handle;
        }

        Ok(ArticulatedChain {
            base,
            links,
            num_joints: self.links.len(),
        })
    }
}

impl Default for ChainBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with_gravity() -> PhysicsWorld<3> {
        PhysicsWorld::new(SVector::from([0.0, 0.0, -9.81]))
    }

    #[test]
    fn unequal_link_lengths_produce_coincident_adjacent_pivots() {
        let lengths = [0.5, 0.4, 0.3];
        let placements = compute_link_placements(&lengths).expect("valid geometry");
        assert_eq!(placements.len(), 3);

        let mut centers = Vec::with_capacity(placements.len());
        let mut z = 3.0;
        for placement in &placements {
            z -= placement.center_step;
            centers.push(z);
        }

        // First link retains the original base-to-link offset. Subsequent
        // centers are half the sum of adjacent pivot-to-pivot lengths apart.
        assert!((centers[0] - 2.5).abs() < 1.0e-12);
        assert!((centers[1] - 2.05).abs() < 1.0e-12);
        assert!((centers[2] - 1.70).abs() < 1.0e-12);
        assert!((placements[1].center_step - 0.45).abs() < 1.0e-12);
        assert!((placements[2].center_step - 0.35).abs() < 1.0e-12);

        // Check each pivot in world Z: base to first link, then link-to-link.
        let base_joint = 3.0 + placements[0].previous_body_anchor;
        let first_link_joint = centers[0] + placements[0].current_body_anchor;
        assert!((base_joint - first_link_joint).abs() < 1.0e-12);

        for index in 1..placements.len() {
            let previous_distal = centers[index - 1] + placements[index].previous_body_anchor;
            let current_proximal = centers[index] + placements[index].current_body_anchor;
            assert!(
                (previous_distal - current_proximal).abs() < 1.0e-12,
                "pivot {index}: previous={previous_distal}, current={current_proximal}"
            );
        }
    }

    #[test]
    fn equal_link_lengths_preserve_original_center_spacing_and_anchors() {
        let placements = compute_link_placements(&[0.3, 0.3]).expect("valid geometry");
        for placement in placements {
            assert!((placement.center_step - 0.3).abs() < 1.0e-12);
            assert!((placement.previous_body_anchor + 0.15).abs() < 1.0e-12);
            assert!((placement.current_body_anchor - 0.15).abs() < 1.0e-12);
        }
    }

    #[test]
    fn invalid_link_lengths_are_rejected() {
        for invalid_length in [0.0, -0.1, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            assert_eq!(
                compute_link_placements(&[invalid_length]),
                Err(ChainBuildError::InvalidLinkLength { index: 0 })
            );
        }
    }

    #[test]
    fn invalid_link_length_fails_before_world_mutation() {
        let mut world = world_with_gravity();
        let result = ChainBuilder::new()
            .add_link(LinkSpec {
                length: 0.5,
                ..Default::default()
            })
            .add_link(LinkSpec {
                length: f64::NAN,
                ..Default::default()
            })
            .try_build(&mut world);

        assert!(matches!(
            result,
            Err(ChainBuildError::InvalidLinkLength { index: 1 })
        ));
        assert!(world.bodies.is_empty());
        assert!(world.constraints.is_empty());
    }

    #[test]
    fn test_single_link_pendulum() {
        let mut world = world_with_gravity();
        let chain = ChainBuilder::new()
            .base_position(Point::new([0.0, 0.0, 2.0]))
            .add_link(LinkSpec {
                mass: 1.0,
                length: 0.5,
                ..Default::default()
            })
            .build(&mut world);

        assert_eq!(chain.num_joints, 1);
        // After gravity steps, the tip should have moved
        for _ in 0..500 {
            world.step(0.002);
        }
        let tip = world.body(chain.tip()).unwrap();
        let pos = &tip.transform.translation.0;
        assert!(
            pos[0].is_finite() && pos[2].is_finite(),
            "Tip position should be finite: {pos:?}"
        );
        // The constraint keeps it connected to base; verify it hasn't exploded
        let dist_from_base = ((pos[0]).powi(2) + (pos[1]).powi(2) + (pos[2] - 2.0).powi(2)).sqrt();
        assert!(
            dist_from_base < 2.0,
            "Tip should stay near base: dist={dist_from_base}"
        );
    }

    #[test]
    fn test_three_link_chain_finite() {
        let mut world = world_with_gravity();
        let chain = ChainBuilder::new()
            .base_position(Point::new([0.0, 0.0, 3.0]))
            .add_link(LinkSpec {
                mass: 2.0,
                length: 0.5,
                ..Default::default()
            })
            .add_link(LinkSpec {
                mass: 1.5,
                length: 0.4,
                ..Default::default()
            })
            .add_link(LinkSpec {
                mass: 1.0,
                length: 0.3,
                ..Default::default()
            })
            .build(&mut world);

        assert_eq!(chain.num_joints, 3);
        for _ in 0..200 {
            world.step(0.001);
        }
        for &h in &chain.links {
            let p = &world.body(h).unwrap().transform.translation.0;
            assert!(p[0].is_finite() && p[1].is_finite() && p[2].is_finite());
        }
    }

    #[test]
    fn test_joint_readback() {
        let mut world = world_with_gravity();
        let chain = ChainBuilder::new()
            .base_position(Point::new([0.0, 0.0, 2.0]))
            .add_link(LinkSpec::default())
            .add_link(LinkSpec::default())
            .build(&mut world);

        let states = chain.read_joint_states(&world);
        assert_eq!(states.len(), 2);
        for (a, v) in &states {
            assert!(a.is_finite());
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_tip_is_last_link() {
        let mut world = PhysicsWorld::new(SVector::from([0.0, 0.0, 0.0]));
        let chain = ChainBuilder::new()
            .add_link(LinkSpec::default())
            .add_link(LinkSpec::default())
            .build(&mut world);
        assert_eq!(chain.tip(), chain.links[1]);
    }
}
