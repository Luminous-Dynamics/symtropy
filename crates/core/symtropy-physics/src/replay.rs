// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Determinism helpers: record/replay command streams and bitwise snapshots.
//!
//! This module is intentionally minimal: it provides
//! - a small `WorldCommand` vocabulary for driving a `PhysicsWorld`
//! - a `ReplayTape` of per-tick frames (dt + commands)
//! - `WorldSnapshot`/`BodySnapshot` that capture simulation state as raw `f64` bits
//!
//! The goal is to make it easy to build a replay harness that asserts
//! **bitwise-equal state per tick** across record/replay passes.

use nalgebra::SVector;
use symtropy_math::Bivector;

use crate::body::{BodyHandle, BodyType, RigidBody};
use crate::integrator;
use crate::world::PhysicsWorld;

/// Commands that mutate a physics world at a tick boundary.
#[derive(Clone, Debug)]
pub enum WorldCommand<const D: usize> {
    ApplyForce {
        body: BodyHandle,
        force: Box<SVector<f64, D>>,
    },
    ApplyImpulse {
        body: BodyHandle,
        impulse: Box<SVector<f64, D>>,
    },
    SetLinearVelocity {
        body: BodyHandle,
        velocity: Box<SVector<f64, D>>,
    },
    SetAngularVelocity {
        body: BodyHandle,
        velocity: Box<Bivector<D>>,
    },
    Wake {
        body: BodyHandle,
    },
}

/// A single replay frame: `dt` + ordered list of commands to apply before stepping.
#[derive(Clone, Debug)]
pub struct ReplayFrame<const D: usize> {
    pub dt: f64,
    pub commands: Vec<WorldCommand<D>>,
}

/// A full replay tape: ordered frames.
#[derive(Clone, Debug, Default)]
pub struct ReplayTape<const D: usize> {
    pub frames: Vec<ReplayFrame<D>>,
}

impl<const D: usize> ReplayTape<D> {
    pub fn push_frame(&mut self, dt: f64, commands: Vec<WorldCommand<D>>) {
        self.frames.push(ReplayFrame { dt, commands });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyCommandError {
    MissingBody(BodyHandle),
}

/// Apply a list of commands to a world.
pub fn apply_commands<const D: usize>(
    world: &mut PhysicsWorld<D>,
    commands: &[WorldCommand<D>],
) -> Result<(), ApplyCommandError> {
    for cmd in commands {
        match cmd {
            WorldCommand::ApplyForce { body, force } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.apply_force(**force);
            }
            WorldCommand::ApplyImpulse { body, impulse } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                integrator::apply_impulse(b, &**impulse);
            }
            WorldCommand::SetLinearVelocity { body, velocity } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.linear_velocity = **velocity;
            }
            WorldCommand::SetAngularVelocity { body, velocity } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.angular_velocity = **velocity;
            }
            WorldCommand::Wake { body } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.wake();
            }
        }
    }
    Ok(())
}

/// Bitwise snapshot of a rigid body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodySnapshot<const D: usize> {
    pub handle: BodyHandle,
    pub body_type: BodyType,
    pub translation: [u64; D],
    pub rotation: [[u64; D]; D],
    pub linear_velocity: [u64; D],
    pub angular_velocity: [[u64; D]; D],
    pub sleeping: bool,
    pub sleep_counter: u32,
}

impl<const D: usize> BodySnapshot<D> {
    pub fn from_body(body: &RigidBody<D>) -> Self {
        let translation = std::array::from_fn(|i| body.transform.translation.0[i].to_bits());

        let rot = body.transform.rotation.to_matrix();
        let rotation = std::array::from_fn(|r| std::array::from_fn(|c| rot[(r, c)].to_bits()));

        let linear_velocity = std::array::from_fn(|i| body.linear_velocity[i].to_bits());

        let ang = body.angular_velocity.to_matrix();
        let angular_velocity =
            std::array::from_fn(|r| std::array::from_fn(|c| ang[(r, c)].to_bits()));

        Self {
            handle: body.handle,
            body_type: body.body_type,
            translation,
            rotation,
            linear_velocity,
            angular_velocity,
            sleeping: body.sleeping,
            sleep_counter: body.sleep_counter,
        }
    }
}

/// Bitwise snapshot of a collision event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionEventSnapshot<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub impulse: u64,
    pub normal: [u64; D],
    pub depth: u64,
}

impl<const D: usize> CollisionEventSnapshot<D> {
    pub fn from_event(event: &crate::contact::CollisionEvent<D>) -> Self {
        Self {
            body_a: event.body_a,
            body_b: event.body_b,
            impulse: event.impulse.to_bits(),
            normal: std::array::from_fn(|i| event.normal[i].to_bits()),
            depth: event.depth.to_bits(),
        }
    }
}

/// Bitwise snapshot of a physics world (bodies + last-step collision events).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldSnapshot<const D: usize> {
    pub bodies: Vec<BodySnapshot<D>>,
    pub collision_events: Vec<CollisionEventSnapshot<D>>,
}

impl<const D: usize> WorldSnapshot<D> {
    pub fn capture(world: &PhysicsWorld<D>) -> Self {
        let mut bodies: Vec<_> = world.bodies.iter().map(BodySnapshot::from_body).collect();
        bodies.sort_by_key(|b| b.handle);

        let mut collision_events: Vec<_> = world
            .collision_events
            .iter()
            .map(CollisionEventSnapshot::from_event)
            .collect();
        collision_events.sort_by_key(|e| (e.body_a, e.body_b, e.impulse, e.depth));

        Self {
            bodies,
            collision_events,
        }
    }
}

/// One completed independent simulation, bound to its stable rollout ID.
///
/// The ID is derived from the frozen input corpus, never from worker index or
/// completion order. Results from parallel execution must be passed through
/// [`order_batch_results`] before comparison or receipt generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRolloutSnapshot<const D: usize> {
    /// Stable ID of this rollout within the frozen campaign.
    pub rollout_id: u64,
    /// Canonical bitwise world snapshot after the declared horizon.
    pub snapshot: WorldSnapshot<D>,
}

/// Structural errors in an independent-rollout result set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchResultError {
    /// A campaign with zero expected rollouts cannot produce a meaningful result set.
    EmptyCampaign,
    /// A result ID cannot be represented as a host index.
    RolloutIdOutOfRange { rollout_id: u64 },
    /// A result ID is outside the declared campaign.
    UnexpectedRolloutId {
        rollout_id: u64,
        expected_rollouts: usize,
    },
    /// More than one result was supplied for the same rollout ID.
    DuplicateRolloutId { rollout_id: u64 },
    /// No result was supplied for an expected rollout ID.
    MissingRolloutId { rollout_id: u64 },
}

/// Validate and order a batch of independent-rollout snapshots by stable ID.
///
/// Completion order is deliberately ignored. Missing, duplicate and unexpected
/// IDs fail closed; a caller must not silently shorten the denominator or let
/// a duplicate replace a missing rollout.
pub fn order_batch_results<const D: usize>(
    expected_rollouts: usize,
    results: Vec<BatchRolloutSnapshot<D>>,
) -> Result<Vec<BatchRolloutSnapshot<D>>, BatchResultError> {
    if expected_rollouts == 0 {
        return Err(BatchResultError::EmptyCampaign);
    }

    let mut slots: Vec<Option<BatchRolloutSnapshot<D>>> = std::iter::repeat_with(|| None)
        .take(expected_rollouts)
        .collect();

    for result in results {
        let index = usize::try_from(result.rollout_id).map_err(|_| {
            BatchResultError::RolloutIdOutOfRange {
                rollout_id: result.rollout_id,
            }
        })?;
        if index >= expected_rollouts {
            return Err(BatchResultError::UnexpectedRolloutId {
                rollout_id: result.rollout_id,
                expected_rollouts,
            });
        }
        if slots[index].is_some() {
            return Err(BatchResultError::DuplicateRolloutId {
                rollout_id: result.rollout_id,
            });
        }
        slots[index] = Some(result);
    }

    slots
        .into_iter()
        .enumerate()
        .map(|(index, result)| {
            result.ok_or(BatchResultError::MissingRolloutId {
                rollout_id: index as u64,
            })
        })
        .collect()
}

#[cfg(test)]
mod batch_replay_tests {
    use super::{
        BatchResultError, BatchRolloutSnapshot, ReplayTape, WorldCommand, WorldSnapshot,
        apply_commands, order_batch_results,
    };
    use crate::world::PhysicsWorld;
    use nalgebra::SVector;
    use rayon::ThreadPoolBuilder;
    use rayon::prelude::*;
    use std::sync::Mutex;
    use symtropy_math::Point;

    const ROLLOUTS: usize = 16;
    const TICKS: usize = 90;

    fn run_rollout(rollout_id: u64) -> BatchRolloutSnapshot<3> {
        let offset = (rollout_id % 7) as f64 * 0.015;
        let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
        let mut handles = Vec::new();

        for body_index in 0..5 {
            let x = (body_index % 3) as f64 * 0.82 + offset;
            let y = 2.0 + (body_index / 3) as f64 * 0.86;
            let mass = 1.0 + (body_index % 3) as f64 * 0.25;
            let handle = world.add_sphere(Point::new([x, y, 0.0]), 0.5, mass);
            if let Some(body) = world.body_mut(handle) {
                body.linear_damping = 0.0;
                body.angular_damping = 0.0;
                let vx = ((rollout_id * 13 + body_index as u64 * 7) % 11) as f64 * 0.01;
                body.linear_velocity = SVector::from([vx, 0.0, 0.0]);
            }
            handles.push(handle);
        }

        let mut tape = ReplayTape::<3>::default();
        for tick in 0..TICKS {
            let commands = if tick % 30 == 0 {
                let body_index = (tick / 30) % handles.len();
                vec![WorldCommand::ApplyForce {
                    body: handles[body_index],
                    force: Box::new(SVector::from([0.05 + offset, 0.0, 0.0])),
                }]
            } else {
                Vec::new()
            };
            tape.push_frame(1.0 / 120.0, commands);
        }

        for frame in &tape.frames {
            apply_commands(&mut world, &frame.commands).unwrap();
            world.step(frame.dt);
        }

        let invariants = world.invariant_snapshot();
        assert!(
            invariants.is_numerically_healthy(1e-9),
            "rollout {rollout_id} produced a numerically unhealthy state"
        );
        assert_eq!(invariants.non_finite_body_count, 0);

        BatchRolloutSnapshot {
            rollout_id,
            snapshot: WorldSnapshot::capture(&world),
        }
    }

    fn run_parallel(rollouts: usize, workers: usize) -> Vec<BatchRolloutSnapshot<3>> {
        assert!(workers > 0);
        let completed = Mutex::new(Vec::with_capacity(rollouts));

        std::thread::scope(|scope| {
            for worker in 0..workers {
                let completed = &completed;
                scope.spawn(move || {
                    for rollout_id in (worker..rollouts).step_by(workers) {
                        let result = run_rollout(rollout_id as u64);
                        completed.lock().unwrap().push(result);
                    }
                });
            }
        });

        order_batch_results(rollouts, completed.into_inner().unwrap()).unwrap()
    }

    #[test]
    fn independent_rollouts_are_bitwise_equal_across_worker_counts() {
        let sequential: Vec<_> = (0..ROLLOUTS)
            .map(|rollout_id| run_rollout(rollout_id as u64))
            .collect();

        for workers in [1, 2, 4, 8] {
            let parallel = run_parallel(ROLLOUTS, workers);
            assert_eq!(
                parallel, sequential,
                "rollout outputs changed at worker count {workers}"
            );
        }
    }

    #[test]
    fn empty_campaign_is_not_treated_as_a_valid_result() {
        assert_eq!(
            order_batch_results::<3>(0, Vec::new()),
            Err(BatchResultError::EmptyCampaign)
        );
    }

    fn run_with_rayon(rollouts: usize, workers: usize) -> Vec<BatchRolloutSnapshot<3>> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .unwrap();
        let completed: Vec<_> = pool.install(|| {
            (0..rollouts)
                .into_par_iter()
                .map(|rollout_id| run_rollout(rollout_id as u64))
                .collect()
        });
        order_batch_results(rollouts, completed).unwrap()
    }

    #[test]
    fn independent_rollouts_are_bitwise_equal_across_persistent_rayon_pools() {
        let sequential: Vec<_> = (0..ROLLOUTS)
            .map(|rollout_id| run_rollout(rollout_id as u64))
            .collect();

        for workers in [1, 2, 4, 8] {
            let parallel = run_with_rayon(ROLLOUTS, workers);
            assert_eq!(
                parallel, sequential,
                "persistent Rayon outputs changed at worker count {workers}"
            );
        }
    }

    #[test]
    fn batch_ordering_is_independent_of_completion_order() {
        let later = run_rollout(1);
        let earlier = run_rollout(0);
        let ordered = order_batch_results(2, vec![later, earlier]).unwrap();

        assert_eq!(ordered[0].rollout_id, 0);
        assert_eq!(ordered[1].rollout_id, 1);
        assert_eq!(ordered[0].snapshot, run_rollout(0).snapshot);
        assert_eq!(ordered[1].snapshot, run_rollout(1).snapshot);
    }

    #[test]
    fn batch_corpus_rejects_duplicate_missing_and_unexpected_ids() {
        let first = run_rollout(0);
        let second = run_rollout(1);
        let duplicate = second.clone();
        assert_eq!(
            order_batch_results(2, vec![first.clone(), second, duplicate]),
            Err(BatchResultError::DuplicateRolloutId { rollout_id: 1 })
        );

        assert_eq!(
            order_batch_results(2, vec![first.clone()]),
            Err(BatchResultError::MissingRolloutId { rollout_id: 1 })
        );

        let unexpected = BatchRolloutSnapshot {
            rollout_id: 2,
            snapshot: WorldSnapshot {
                bodies: Vec::new(),
                collision_events: Vec::new(),
            },
        };
        assert_eq!(
            order_batch_results(2, vec![first, unexpected]),
            Err(BatchResultError::UnexpectedRolloutId {
                rollout_id: 2,
                expected_rollouts: 2
            })
        );
    }
}
