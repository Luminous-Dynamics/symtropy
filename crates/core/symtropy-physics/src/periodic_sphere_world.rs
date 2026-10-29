// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Transactional event-driven periodic sphere reference world.
//!
//! This is an independent D=2/3 validation harness for periodic collision
//! semantics. It preserves continuous, unwrapped coordinates and resolves
//! translational sphere impulses at the earliest bounded periodic CCD event.
//! It is deliberately not a replacement for PhysicsWorld: it has no angular
//! response, friction, arbitrary convex shapes, joints, or warm-start cache.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use nalgebra::SVector;
use symtropy_math::{LatticeOffset, PeriodicBox};

use crate::periodic_collision::{
    PeriodicCollisionError, PeriodicSphereCcdHit, sphere_sphere_ccd_periodic_all,
};
use crate::periodic_sphere_response::{
    PeriodicImpulseError, PeriodicImpulseReport, resolve_periodic_sphere_impact,
};

const TIME_EPSILON: f64 = 1.0e-12;

/// A single spherical body in an unwrapped coordinate chart.
#[derive(Clone, Debug, PartialEq)]
pub struct PeriodicSphereBody<const D: usize> {
    pub id: u64,
    /// Continuous/unwrapped position. It is not rewritten at a periodic seam.
    pub position: SVector<f64, D>,
    pub velocity: SVector<f64, D>,
    pub radius: f64,
    pub mass: f64,
    /// Newton restitution coefficient in [0, 1].
    pub restitution: f64,
}

impl<const D: usize> PeriodicSphereBody<D> {
    pub fn new(
        id: u64,
        position: SVector<f64, D>,
        velocity: SVector<f64, D>,
        radius: f64,
        mass: f64,
        restitution: f64,
    ) -> Result<Self, PeriodicSphereWorldError> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(PeriodicSphereWorldError::InvalidBody {
                id,
                field: "radius",
            });
        }
        if !mass.is_finite() || mass <= 0.0 {
            return Err(PeriodicSphereWorldError::InvalidBody {
                id,
                field: "mass",
            });
        }
        if !restitution.is_finite() || !(0.0..=1.0).contains(&restitution) {
            return Err(PeriodicSphereWorldError::InvalidBody {
                id,
                field: "restitution",
            });
        }
        for axis in 0..D {
            if !position[axis].is_finite() {
                return Err(PeriodicSphereWorldError::InvalidBody {
                    id,
                    field: "position",
                });
            }
            if !velocity[axis].is_finite() {
                return Err(PeriodicSphereWorldError::InvalidBody {
                    id,
                    field: "velocity",
                });
            }
        }
        Ok(Self {
            id,
            position,
            velocity,
            radius,
            mass,
            restitution,
        })
    }
}

/// Typed failure. A failed step leaves the original world state unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicSphereWorldError {
    UnsupportedDimension { dimension: usize },
    InvalidConfiguration { field: &'static str },
    DuplicateBodyId { id: u64 },
    BodyRadiusExceedsPeriodicEnvelope { id: u64, axis: usize },
    InvalidBody { id: u64, field: &'static str },
    InvalidTimeStep,
    NonFiniteArithmetic { field: &'static str },
    EventBudgetExceeded { limit: usize },
    Collision(PeriodicCollisionError),
    Impulse(PeriodicImpulseError),
}

impl fmt::Display for PeriodicSphereWorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDimension { dimension } => {
                write!(f, "periodic sphere world supports D=2 or D=3, got D={dimension}")
            }
            Self::InvalidConfiguration { field } => {
                write!(f, "invalid periodic sphere world configuration: {field}")
            }
            Self::DuplicateBodyId { id } => write!(f, "duplicate sphere body id {id}"),
            Self::BodyRadiusExceedsPeriodicEnvelope { id, axis } => write!(
                f,
                "sphere body {id} radius must be less than half the period on periodic axis {axis}"
            ),
            Self::InvalidBody { id, field } => {
                write!(f, "invalid sphere body {id} field {field}")
            }
            Self::InvalidTimeStep => write!(f, "step dt must be finite and positive"),
            Self::NonFiniteArithmetic { field } => {
                write!(f, "non-finite periodic sphere arithmetic in {field}")
            }
            Self::EventBudgetExceeded { limit } => {
                write!(f, "periodic sphere event budget exhausted (limit {limit})")
            }
            Self::Collision(error) => write!(f, "periodic sphere collision error: {error}"),
            Self::Impulse(error) => write!(f, "periodic sphere impulse error: {error}"),
        }
    }
}

impl Error for PeriodicSphereWorldError {}

impl From<PeriodicCollisionError> for PeriodicSphereWorldError {
    fn from(value: PeriodicCollisionError) -> Self {
        Self::Collision(value)
    }
}

impl From<PeriodicImpulseError> for PeriodicSphereWorldError {
    fn from(value: PeriodicImpulseError) -> Self {
        Self::Impulse(value)
    }
}

/// One resolved sphere impact. event_time_fraction is relative to the original
/// requested step, not to the current substep.
#[derive(Clone, Debug)]
pub struct PeriodicSphereWorldEvent<const D: usize> {
    pub body_a: u64,
    pub body_b: u64,
    pub image_offset: LatticeOffset<D>,
    pub event_time_fraction: f64,
    pub impulse: PeriodicImpulseReport<D>,
}

/// Conservation and event evidence for one successful full step.
#[derive(Clone, Debug)]
pub struct PeriodicSphereWorldReport<const D: usize> {
    pub dt: f64,
    pub events: Vec<PeriodicSphereWorldEvent<D>>,
    pub momentum_before: SVector<f64, D>,
    pub momentum_after: SVector<f64, D>,
    pub kinetic_energy_before: f64,
    pub kinetic_energy_after: f64,
    pub event_budget: usize,
    pub max_image_candidates: usize,
}

/// Event-driven, transactional reference world for periodic spheres.
///
/// Bodies are kept in ascending stable-ID order. The domain uses explicit
/// periodic axes; coordinates remain unwrapped so crossing a seam does not
/// introduce a discontinuous integration jump. A step is computed on a cloned
/// state and committed only after the full requested interval succeeds.
#[derive(Clone, Debug)]
pub struct PeriodicSphereWorld<const D: usize> {
    domain: PeriodicBox<D>,
    bodies: Vec<PeriodicSphereBody<D>>,
    max_image_candidates: usize,
    max_events_per_step: usize,
}

impl<const D: usize> PeriodicSphereWorld<D> {
    pub fn new(
        domain: PeriodicBox<D>,
        max_image_candidates: usize,
        max_events_per_step: usize,
    ) -> Result<Self, PeriodicSphereWorldError> {
        if D != 2 && D != 3 {
            return Err(PeriodicSphereWorldError::UnsupportedDimension { dimension: D });
        }
        if max_image_candidates == 0 {
            return Err(PeriodicSphereWorldError::InvalidConfiguration {
                field: "max_image_candidates",
            });
        }
        if max_events_per_step == 0 {
            return Err(PeriodicSphereWorldError::InvalidConfiguration {
                field: "max_events_per_step",
            });
        }
        Ok(Self {
            domain,
            bodies: Vec::new(),
            max_image_candidates,
            max_events_per_step,
        })
    }

    pub fn domain(&self) -> &PeriodicBox<D> {
        &self.domain
    }

    pub fn bodies(&self) -> &[PeriodicSphereBody<D>] {
        &self.bodies
    }

    pub fn add_body(
        &mut self,
        body: PeriodicSphereBody<D>,
    ) -> Result<(), PeriodicSphereWorldError> {
        // Fields are public for inspection/interchange, so validate even when
        // callers did not construct the body through PeriodicSphereBody::new.
        let _validated = PeriodicSphereBody::new(
            body.id,
            body.position,
            body.velocity,
            body.radius,
            body.mass,
            body.restitution,
        )?;
        if self.bodies.iter().any(|existing| existing.id == body.id) {
            return Err(PeriodicSphereWorldError::DuplicateBodyId { id: body.id });
        }
        // Keep each sphere smaller than half every periodic length so a
        // body cannot overlap its own periodic images, and two bodies cannot
        // overlap two different images simultaneously. A long sweep may still
        // hit successive images of the same pair, which the event loop handles
        // by enumerating all per-image TOIs.
        for axis in 0..D {
            if self.domain.is_periodic(axis) == Some(true)
                && body.radius >= 0.5 * self.domain.periods()[axis]
            {
                return Err(PeriodicSphereWorldError::BodyRadiusExceedsPeriodicEnvelope {
                    id: body.id,
                    axis,
                });
            }
        }
        let insertion = self
            .bodies
            .binary_search_by_key(&body.id, |existing| existing.id)
            .unwrap_or_else(|index| index);
        self.bodies.insert(insertion, body);
        Ok(())
    }

    /// Advance all bodies through dt using event-driven periodic sphere CCD.
    ///
    /// If candidate enumeration, numerical validation, or the event budget
    /// fails, this method returns an error without committing any body state.
    pub fn step(
        &mut self,
        dt: f64,
    ) -> Result<PeriodicSphereWorldReport<D>, PeriodicSphereWorldError> {
        if !dt.is_finite() || dt <= 0.0 {
            return Err(PeriodicSphereWorldError::InvalidTimeStep);
        }

        validate_bodies(&self.bodies)?;
        let momentum_before = total_momentum(&self.bodies)?;
        let kinetic_energy_before = total_kinetic_energy(&self.bodies)?;

        let mut working = self.bodies.clone();
        let mut elapsed = 0.0_f64;
        let mut events = Vec::new();
        let mut suppressed_zero_time_contacts = BTreeSet::<(u64, u64, [i64; D])>::new();

        loop {
            let remaining = dt - elapsed;
            if remaining <= 0.0 {
                break;
            }

            let mut earliest: Option<(
                f64,
                u64,
                u64,
                LatticeOffset<D>,
                usize,
                usize,
                PeriodicSphereCcdHit<D>,
            )> = None;

            for index_a in 0..working.len() {
                for index_b in (index_a + 1)..working.len() {
                    let body_a = &working[index_a];
                    let body_b = &working[index_b];
                    let pair_key = (body_a.id, body_b.id);

                    let hits = sphere_sphere_ccd_periodic_all(
                        &self.domain,
                        &body_a.position,
                        &body_a.velocity,
                        body_a.radius,
                        &body_b.position,
                        &body_b.velocity,
                        body_b.radius,
                        remaining,
                        self.max_image_candidates,
                    )?;

                    for hit in hits {
                        let contact_key = (pair_key.0, pair_key.1, hit.image_offset.periods);
                        if hit.toi_seconds <= time_epsilon(dt)
                            && suppressed_zero_time_contacts.contains(&contact_key)
                        {
                            continue;
                        }

                        let candidate = (
                            hit.toi_seconds,
                            body_a.id,
                            body_b.id,
                            hit.image_offset,
                            index_a,
                            index_b,
                            hit,
                        );
                        if earliest
                            .as_ref()
                            .is_none_or(|current| candidate_precedes(&candidate, current))
                        {
                            earliest = Some(candidate);
                        }
                    }
                }
            }

            let Some((toi, id_a, id_b, image_offset, index_a, index_b, hit)) = earliest else {
                advance_all(&mut working, remaining)?;
                elapsed = dt;
                break;
            };

            if events.len() >= self.max_events_per_step {
                return Err(PeriodicSphereWorldError::EventBudgetExceeded {
                    limit: self.max_events_per_step,
                });
            }

            if toi > remaining + time_epsilon(dt) {
                return Err(PeriodicSphereWorldError::NonFiniteArithmetic {
                    field: "TOI exceeded remaining step",
                });
            }
            let advance = toi.clamp(0.0, remaining);
            if advance > time_epsilon(dt) {
                advance_all(&mut working, advance)?;
                elapsed += advance;
                suppressed_zero_time_contacts.clear();
            }

            let body_a = &working[index_a];
            let body_b = &working[index_b];
            let restitution = body_a.restitution.min(body_b.restitution);
            let impulse = resolve_periodic_sphere_impact(
                &hit,
                &body_a.velocity,
                body_a.mass,
                &body_b.velocity,
                body_b.mass,
                restitution,
            )?;

            let affected_a = id_a;
            let affected_b = id_b;
            let applied = impulse.impulse_applied;
            working[index_a].velocity = impulse.velocity_a_after;
            working[index_b].velocity = impulse.velocity_b_after;

            if applied {
                suppressed_zero_time_contacts.retain(|(a, b, _)| {
                    *a != affected_a && *a != affected_b && *b != affected_a && *b != affected_b
                });
            }
            // The pair has just been resolved at a touching configuration.
            // Suppress its immediate zero-time rediscovery even when the
            // original TOI was positive. Any later positive-time advance clears
            // this suppression; an impulse to either body also invalidates
            // other suppressions involving that body.
            suppressed_zero_time_contacts.insert((id_a, id_b, image_offset.periods));

            let fraction = (elapsed / dt).clamp(0.0, 1.0);
            events.push(PeriodicSphereWorldEvent {
                body_a: id_a,
                body_b: id_b,
                image_offset,
                event_time_fraction: fraction,
                impulse,
            });

            // Suppress only this exact image contact. Other lattice images of
            // the same body pair remain eligible for later TOIs during a long
            // step. Any positive-time advancement clears these immediate
            // contact suppressions; an impulse to either body also invalidates
            // prior suppressions involving that body. The event budget remains
            // the final fail-closed guard against contact cycles.
        }

        let momentum_after = total_momentum(&working)?;
        let kinetic_energy_after = total_kinetic_energy(&working)?;

        // Check that all final positions/velocities are finite before commit.
        validate_bodies(&working)?;
        self.bodies = working;

        Ok(PeriodicSphereWorldReport {
            dt,
            events,
            momentum_before,
            momentum_after,
            kinetic_energy_before,
            kinetic_energy_after,
            event_budget: self.max_events_per_step,
            max_image_candidates: self.max_image_candidates,
        })
    }
}

fn time_epsilon(dt: f64) -> f64 {
    TIME_EPSILON * dt
}

fn candidate_precedes<const D: usize>(
    candidate: &(
        f64,
        u64,
        u64,
        LatticeOffset<D>,
        usize,
        usize,
        PeriodicSphereCcdHit<D>,
    ),
    current: &(
        f64,
        u64,
        u64,
        LatticeOffset<D>,
        usize,
        usize,
        PeriodicSphereCcdHit<D>,
    ),
) -> bool {
    if candidate.0 < current.0 {
        return true;
    }
    if candidate.0 > current.0 {
        return false;
    }
    (candidate.1, candidate.2, candidate.3.periods)
        < (current.1, current.2, current.3.periods)
}

fn advance_all<const D: usize>(
    bodies: &mut [PeriodicSphereBody<D>],
    dt: f64,
) -> Result<(), PeriodicSphereWorldError> {
    if !dt.is_finite() || dt < 0.0 {
        return Err(PeriodicSphereWorldError::InvalidTimeStep);
    }
    for body in bodies {
        let position = body.position + body.velocity * dt;
        if !position.iter().all(|value| value.is_finite()) {
            return Err(PeriodicSphereWorldError::NonFiniteArithmetic {
                field: "linear advancement",
            });
        }
        body.position = position;
    }
    Ok(())
}

fn validate_bodies<const D: usize>(
    bodies: &[PeriodicSphereBody<D>],
) -> Result<(), PeriodicSphereWorldError> {
    for body in bodies {
        let _validated = PeriodicSphereBody::new(
            body.id,
            body.position,
            body.velocity,
            body.radius,
            body.mass,
            body.restitution,
        )?;
    }
    Ok(())
}

fn total_momentum<const D: usize>(
    bodies: &[PeriodicSphereBody<D>],
) -> Result<SVector<f64, D>, PeriodicSphereWorldError> {
    let mut momentum = SVector::<f64, D>::zeros();
    for body in bodies {
        momentum += body.velocity * body.mass;
    }
    if !momentum.iter().all(|value| value.is_finite()) {
        return Err(PeriodicSphereWorldError::NonFiniteArithmetic {
            field: "total momentum",
        });
    }
    Ok(momentum)
}

fn total_kinetic_energy<const D: usize>(
    bodies: &[PeriodicSphereBody<D>],
) -> Result<f64, PeriodicSphereWorldError> {
    let mut energy = 0.0;
    for body in bodies {
        energy += 0.5 * body.mass * body.velocity.norm_squared();
    }
    if !energy.is_finite() {
        return Err(PeriodicSphereWorldError::NonFiniteArithmetic {
            field: "kinetic energy",
        });
    }
    Ok(energy)
}

#[cfg(test)]
mod tests {
    use super::{
        PeriodicSphereBody, PeriodicSphereWorld, PeriodicSphereWorldError,
    };
    use nalgebra::SVector;
    use symtropy_math::PeriodicBox;

    fn domain() -> PeriodicBox<2> {
        PeriodicBox::new(
            SVector::zeros(),
            SVector::from([10.0, 0.0]),
            [true, false],
        )
        .unwrap()
    }

    fn body(
        id: u64,
        position: [f64; 2],
        velocity: [f64; 2],
        mass: f64,
        restitution: f64,
    ) -> PeriodicSphereBody<2> {
        PeriodicSphereBody::new(
            id,
            SVector::from(position),
            SVector::from(velocity),
            0.5,
            mass,
            restitution,
        )
        .unwrap()
    }

    fn world(max_events: usize) -> PeriodicSphereWorld<2> {
        PeriodicSphereWorld::new(domain(), 16, max_events).unwrap()
    }

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "expected {a:.15e} ~= {b:.15e}");
    }

    #[test]
    fn seam_impact_resolves_and_keeps_unwrapped_positions_continuous() {
        let mut world = world(8);
        world.add_body(body(1, [0.5, 0.0], [0.0, 0.0], 1.0, 1.0)).unwrap();
        world.add_body(body(2, [9.0, 0.0], [2.0, 0.0], 1.0, 1.0)).unwrap();

        let report = world.step(1.0).unwrap();
        assert_eq!(report.events.len(), 1);
        assert_eq!(report.events[0].image_offset.periods, [-1, 0]);
        close(report.events[0].event_time_fraction, 0.25, 1e-12);
        close(world.bodies()[0].velocity[0], 2.0, 1e-12);
        close(world.bodies()[1].velocity[0], 0.0, 1e-12);
        close(world.bodies()[0].position[0], 2.0, 1e-12);
        close(world.bodies()[1].position[0], 9.5, 1e-12);
        close(report.momentum_after[0], report.momentum_before[0], 1e-12);
        close(report.kinetic_energy_after, report.kinetic_energy_before, 1e-12);
    }

    #[test]
    fn global_lattice_translation_and_insertion_order_preserve_dynamics() {
        let mut baseline = world(16);
        baseline.add_body(body(2, [9.0, 0.0], [2.0, 0.0], 1.0, 1.0)).unwrap();
        baseline.add_body(body(1, [0.5, 0.0], [0.0, 0.0], 1.0, 1.0)).unwrap();

        let mut translated = world(16);
        // Deliberately use a different insertion order and shift every point
        // by three complete cells. Physical trajectories must be equivalent.
        translated.add_body(body(1, [30.5, 0.0], [0.0, 0.0], 1.0, 1.0)).unwrap();
        translated.add_body(body(2, [39.0, 0.0], [2.0, 0.0], 1.0, 1.0)).unwrap();

        let baseline_report = baseline.step(1.0).unwrap();
        let translated_report = translated.step(1.0).unwrap();

        assert_eq!(baseline_report.events.len(), translated_report.events.len());
        assert_eq!(baseline_report.events.len(), 1);
        for (left, right) in baseline_report.events.iter().zip(&translated_report.events) {
            assert_eq!((left.body_a, left.body_b), (right.body_a, right.body_b));
            assert_eq!(left.image_offset, right.image_offset);
            close(left.event_time_fraction, right.event_time_fraction, 1e-12);
        }
        for (left, right) in baseline.bodies().iter().zip(translated.bodies()) {
            assert_eq!(left.id, right.id);
            assert_eq!(left.velocity, right.velocity);
            close(right.position[0] - left.position[0], 30.0, 1e-12);
            close(right.position[1] - left.position[1], 0.0, 1e-12);
        }
        close(
            baseline_report.kinetic_energy_before,
            translated_report.kinetic_energy_before,
            1e-12,
        );
        close(
            baseline_report.kinetic_energy_after,
            translated_report.kinetic_energy_after,
            1e-12,
        );
        for axis in 0..2 {
            close(
                baseline_report.momentum_before[axis],
                translated_report.momentum_before[axis],
                1e-12,
            );
            close(
                baseline_report.momentum_after[axis],
                translated_report.momentum_after[axis],
                1e-12,
            );
        }
    }

    #[test]
    fn inelastic_seam_impact_dissipates_energy_without_losing_momentum() {
        let mut world = world(8);
        world.add_body(body(1, [0.5, 0.0], [0.0, 0.0], 1.0, 0.0)).unwrap();
        world.add_body(body(2, [9.0, 0.0], [2.0, 0.0], 1.0, 0.0)).unwrap();

        let report = world.step(1.0).unwrap();
        assert_eq!(report.events.len(), 1);
        close(report.momentum_after[0], report.momentum_before[0], 1e-12);
        assert!(report.kinetic_energy_after < report.kinetic_energy_before);
        close(world.bodies()[0].velocity[0], 1.0, 1e-12);
        close(world.bodies()[1].velocity[0], 1.0, 1e-12);
    }

    #[test]
    fn non_periodic_axis_does_not_wrap() {
        let mut world = world(8);
        world.add_body(body(1, [0.5, 0.0], [0.0, 0.0], 1.0, 1.0)).unwrap();
        world.add_body(body(2, [9.0, 2.0], [2.0, 0.0], 1.0, 1.0)).unwrap();

        let report = world.step(1.0).unwrap();
        assert!(report.events.is_empty());
    }

    #[test]
    fn long_step_rechecks_later_periodic_images_of_same_pair() {
        let mut world = world(8);
        world.add_body(body(1, [5.0, 0.0], [0.0, 0.0], 1.0, 1.0)).unwrap();
        world.add_body(body(2, [0.0, 0.0], [30.0, 0.0], 1.0, 1.0)).unwrap();

        let report = world.step(1.0).unwrap();
        assert!(report.events.len() >= 3, "events={}", report.events.len());
        assert!(
            report
                .events
                .windows(2)
                .all(|pair| pair[0].event_time_fraction <= pair[1].event_time_fraction)
        );
        close(report.momentum_after[0], report.momentum_before[0], 1e-10);
        close(report.kinetic_energy_after, report.kinetic_energy_before, 1e-9);
        // Physical positions remain unwrapped even after several torus images
        // have participated in the event sequence.
        assert!(world.bodies().iter().any(|body| body.position[0] > 10.0));
    }

    #[test]
    fn event_budget_exhaustion_is_transactional() {
        let mut world = world(1);
        world.add_body(body(1, [0.5, 0.0], [0.0, 0.0], 1.0, 0.0)).unwrap();
        world.add_body(body(2, [9.0, 0.0], [2.0, 0.0], 1.0, 0.0)).unwrap();
        world.add_body(body(3, [2.0, 0.0], [0.0, 0.0], 1.0, 0.0)).unwrap();

        let before = world.bodies().to_vec();
        let result = world.step(1.0);
        assert!(matches!(
            result,
            Err(PeriodicSphereWorldError::EventBudgetExceeded { limit: 1 })
        ));
        // The first event was processed on the clone; the second event exceeds
        // the cap. The authoritative world remains exactly unchanged.
        assert_eq!(world.bodies(), before.as_slice());
    }

    #[test]
    fn duplicate_ids_are_rejected_and_insertion_order_is_canonicalized() {
        let mut world = world(8);
        world.add_body(body(9, [9.0, 0.0], [0.0, 0.0], 1.0, 1.0)).unwrap();
        world.add_body(body(3, [3.0, 0.0], [0.0, 0.0], 1.0, 1.0)).unwrap();
        assert_eq!(world.bodies()[0].id, 3);
        assert_eq!(
            world.add_body(body(3, [4.0, 0.0], [0.0, 0.0], 1.0, 1.0)),
            Err(PeriodicSphereWorldError::DuplicateBodyId { id: 3 })
        );
    }

    #[test]
    fn invalid_dt_fails_without_mutation() {
        let mut world = world(8);
        world.add_body(body(1, [0.5, 0.0], [1.0, 0.0], 1.0, 1.0)).unwrap();
        let before = world.bodies().to_vec();
        assert!(matches!(
            world.step(f64::NAN),
            Err(PeriodicSphereWorldError::InvalidTimeStep)
        ));
        assert_eq!(world.bodies(), before.as_slice());
    }
}
