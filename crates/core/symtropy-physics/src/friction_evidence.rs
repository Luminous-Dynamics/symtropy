// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Signed mechanical evidence for one friction impulse.
//!
//! This module is intentionally narrower than physical heat authority. It applies
//! the same linear + lever-arm angular impulse sequence used by the world contact
//! solver, measures the modeled pair kinetic-energy change immediately around
//! that impulse, and classifies whether the result is even eligible for physical
//! promotion.
//!
//! A measured loss is **not automatically heat**. Centered dynamic/dynamic pairs
//! are the only regime currently eligible for the existing audited #9 conversion.
//! Off-center contacts remain diagnostic until the production angular solver is
//! qualified. Dynamic/static or dynamic/kinematic contacts remain diagnostic
//! until the omitted boundary reservoir is represented explicitly.

use nalgebra::SVector;
use serde::{Deserialize, Serialize};
use symtropy_math::Bivector;

use crate::body::{BodyHandle, BodyType, RigidBody};
use crate::integrator;

const CENTERED_OFFSET_EPSILON_SQUARED: f64 = 1.0e-24;
const ENERGY_EPSILON_J: f64 = 1.0e-15;

/// Deterministic identity of one friction impulse within a fixed physics tick.
///
/// The hot world integration will ultimately source these coordinates from the
/// #824 fixed-tick transaction boundary. Floating-point observation content is
/// deliberately not used as identity: two numerically identical impulses in
/// different ticks are distinct events.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct FrictionTransactionId {
    pub fixed_tick: u64,
    pub solver_iteration: u32,
    pub contact_sequence: u32,
    pub point_sequence: u32,
}

impl FrictionTransactionId {
    pub const fn new(
        fixed_tick: u64,
        solver_iteration: u32,
        contact_sequence: u32,
        point_sequence: u32,
    ) -> Self {
        Self {
            fixed_tick,
            solver_iteration,
            contact_sequence,
            point_sequence,
        }
    }
}

/// Why a friction observation may or may not be eligible for physical promotion.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionEvidenceRegime {
    /// Both bodies are dynamic and the impulse is centered on both bodies.
    /// This is the bounded regime covered by the existing audited friction-to-heat
    /// reference primitive.
    CenteredClosedDynamicPair,
    /// Both bodies are dynamic, but at least one lever arm is non-zero. The
    /// current production angular impulse path still uses scalar-mean inertia.
    OffCenterUnqualified,
    /// At least one participant is static or kinematic, so work can cross an
    /// unmodeled mechanical boundary.
    ExternalBoundaryUnqualified,
}

/// Signed interpretation of the pair kinetic-energy delta.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FrictionMechanicalDelta {
    DissipationCandidate { joules: f64 },
    SolverInjection { joules: f64 },
    Neutral,
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct MechanicalSnapshot<const D: usize> {
    linear_velocity: SVector<f64, D>,
    angular_velocity: Bivector<D>,
}

impl<const D: usize> MechanicalSnapshot<D> {
    fn capture(body: &RigidBody<D>) -> Self {
        Self {
            linear_velocity: body.linear_velocity,
            angular_velocity: body.angular_velocity,
        }
    }

    fn restore(self, body: &mut RigidBody<D>) {
        body.linear_velocity = self.linear_velocity;
        body.angular_velocity = self.angular_velocity;
    }
}

/// Immediate pre/post evidence around one applied friction impulse.
///
/// The pre-impulse velocity snapshots are private provenance. Public callers may
/// inspect the historical scalar energy diagnostics, but cannot author a new
/// observation that masquerades as one produced by the application primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct FrictionMechanicalObservation<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub contact_point: SVector<f64, D>,
    /// Equal-and-opposite impulse applied to body B. Body A receives `-J`.
    pub impulse_on_b: SVector<f64, D>,
    pub kinetic_before_a_joules: f64,
    pub kinetic_before_b_joules: f64,
    pub kinetic_after_a_joules: f64,
    pub kinetic_after_b_joules: f64,
    /// `K_after_pair - K_before_pair`. Positive means the solver injected
    /// modeled mechanical energy; negative means modeled mechanical loss.
    pub pair_delta_joules: f64,
    pub delta: FrictionMechanicalDelta,
    pub regime: FrictionEvidenceRegime,
    pre_state_a: MechanicalSnapshot<D>,
    pre_state_b: MechanicalSnapshot<D>,
}

impl<const D: usize> FrictionMechanicalObservation<D> {
    /// Candidate Joules that may be considered for the existing centered
    /// physical-promotion theorem. This method does not mutate heat or a ledger.
    pub fn centered_promotable_loss_candidate_joules(&self) -> Option<f64> {
        if self.regime != FrictionEvidenceRegime::CenteredClosedDynamicPair {
            return None;
        }
        match self.delta {
            FrictionMechanicalDelta::DissipationCandidate { joules } => Some(joules),
            FrictionMechanicalDelta::SolverInjection { .. } | FrictionMechanicalDelta::Neutral => {
                None
            }
        }
    }

    pub(crate) fn pre_linear_velocity_a(&self) -> SVector<f64, D> {
        self.pre_state_a.linear_velocity
    }

    pub(crate) fn pre_linear_velocity_b(&self) -> SVector<f64, D> {
        self.pre_state_b.linear_velocity
    }

    pub(crate) fn pre_angular_velocity_a(&self) -> Bivector<D> {
        self.pre_state_a.angular_velocity
    }

    pub(crate) fn pre_angular_velocity_b(&self) -> Bivector<D> {
        self.pre_state_b.angular_velocity
    }
}

/// Exact mechanical authority state that must remain unchanged between application
/// and terminalization of one bound friction transaction.
///
/// This deliberately binds only state that changes the interpretation or
/// admissibility of the mechanical transition. Damping, friction coefficients,
/// masks, sensors and other unrelated future-step policy are not part of this
/// immediate applied-transition freshness theorem.
#[derive(Copy, Clone, Debug, PartialEq)]
struct BoundMechanicalAuthorityState<const D: usize> {
    body_type: BodyType,
    position: SVector<f64, D>,
    linear_velocity: SVector<f64, D>,
    angular_velocity: Bivector<D>,
    mass: f64,
    inv_mass: f64,
    inertia: SVector<f64, D>,
    inv_inertia: SVector<f64, D>,
}

impl<const D: usize> BoundMechanicalAuthorityState<D> {
    fn capture(body: &RigidBody<D>) -> Self {
        Self {
            body_type: body.body_type,
            position: body.position(),
            linear_velocity: body.linear_velocity,
            angular_velocity: body.angular_velocity,
            mass: body.mass,
            inv_mass: body.inv_mass,
            inertia: body.inertia,
            inv_inertia: body.inv_inertia,
        }
    }

    fn matches(self, body: &RigidBody<D>) -> bool {
        self == Self::capture(body)
    }
}

/// Non-cloneable binding between one applied mechanical transition, its exact
/// post-state, and its deterministic solver transaction identity.
///
/// External callers cannot manufacture this token from an unbound observation;
/// it is created only by [`apply_friction_impulse_measured_bound`], which applies
/// the mechanical impulse as part of producing the evidence. The private
/// mechanical-authority snapshots prevent later velocity, body-type, position,
/// mass or inertia changes from silently reinterpreting the same transaction.
#[derive(Debug, PartialEq)]
pub struct BoundFrictionMechanicalObservation<const D: usize> {
    transaction_id: FrictionTransactionId,
    observation: FrictionMechanicalObservation<D>,
    post_state_a: BoundMechanicalAuthorityState<D>,
    post_state_b: BoundMechanicalAuthorityState<D>,
}

impl<const D: usize> BoundFrictionMechanicalObservation<D> {
    pub const fn transaction_id(&self) -> FrictionTransactionId {
        self.transaction_id
    }

    pub fn observation(&self) -> &FrictionMechanicalObservation<D> {
        &self.observation
    }

    /// Exact freshness check used by the in-crate physical promotion layer.
    ///
    /// Equality is intentional: this is an exactly-once solver transaction
    /// boundary, not a fuzzy physical-state comparison. Any later mechanical
    /// authority change makes the token stale, even if scalar kinetic energy or
    /// velocity happens to remain equal.
    pub(crate) fn matches_post_state(
        &self,
        body_a: &RigidBody<D>,
        body_b: &RigidBody<D>,
    ) -> bool {
        self.observation.body_a == body_a.handle
            && self.observation.body_b == body_b.handle
            && self.post_state_a.matches(body_a)
            && self.post_state_b.matches(body_b)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionEvidenceError {
    SameBody,
    NonFiniteImpulse,
    NonFiniteContactPoint,
    NonFiniteMechanicalState,
    NonFiniteKineticEnergy,
    UnrepresentablePairDelta,
}

fn mechanical_state_is_finite<const D: usize>(body: &RigidBody<D>) -> bool {
    body.position().iter().all(|value| value.is_finite())
        && body.linear_velocity.iter().all(|value| value.is_finite())
        && body.angular_velocity.is_finite()
        && body.mass.is_finite()
        && body.inv_mass.is_finite()
        && body.inertia.iter().all(|value| value.is_finite())
        && body.inv_inertia.iter().all(|value| value.is_finite())
}

/// Apply the exact equal-and-opposite linear + lever-arm angular transition used
/// by the current world friction solver.
///
/// This is crate-private on purpose: it is a shared mechanics kernel for the core
/// solver/evidence implementation, not a public authority surface. The caller is
/// responsible for validating the request and for any rollback/evidence protocol
/// around the transition. Positions are not mutated here.
pub(crate) fn apply_friction_impulse_mechanics<const D: usize>(
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    contact_point: &SVector<f64, D>,
    impulse_on_b: &SVector<f64, D>,
) {
    let r_a = *contact_point - body_a.position();
    let r_b = *contact_point - body_b.position();

    integrator::apply_impulse(body_a, &(-*impulse_on_b));
    integrator::apply_impulse(body_b, impulse_on_b);

    let torque_a = Bivector::from_wedge(&(-*impulse_on_b), &r_a);
    let torque_b = Bivector::from_wedge(impulse_on_b, &r_b);
    integrator::apply_angular_impulse(body_a, &torque_a);
    integrator::apply_angular_impulse(body_b, &torque_b);
}

/// Classify whether the contact geometry/body types are eligible for the
/// currently bounded physical friction theorem.
pub fn classify_friction_evidence_regime<const D: usize>(
    body_a: &RigidBody<D>,
    body_b: &RigidBody<D>,
    contact_point: &SVector<f64, D>,
) -> Result<FrictionEvidenceRegime, FrictionEvidenceError> {
    if body_a.handle == body_b.handle {
        return Err(FrictionEvidenceError::SameBody);
    }
    if !contact_point.iter().all(|value| value.is_finite()) {
        return Err(FrictionEvidenceError::NonFiniteContactPoint);
    }

    if !body_a.is_dynamic() || !body_b.is_dynamic() {
        return Ok(FrictionEvidenceRegime::ExternalBoundaryUnqualified);
    }

    let r_a = *contact_point - body_a.position();
    let r_b = *contact_point - body_b.position();
    if r_a.norm_squared() <= CENTERED_OFFSET_EPSILON_SQUARED
        && r_b.norm_squared() <= CENTERED_OFFSET_EPSILON_SQUARED
    {
        Ok(FrictionEvidenceRegime::CenteredClosedDynamicPair)
    } else {
        Ok(FrictionEvidenceRegime::OffCenterUnqualified)
    }
}

fn apply_friction_impulse_measured_inner<const D: usize>(
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    contact_point: &SVector<f64, D>,
    impulse_on_b: &SVector<f64, D>,
) -> Result<FrictionMechanicalObservation<D>, FrictionEvidenceError> {
    if body_a.handle == body_b.handle {
        return Err(FrictionEvidenceError::SameBody);
    }
    if !impulse_on_b.iter().all(|value| value.is_finite()) {
        return Err(FrictionEvidenceError::NonFiniteImpulse);
    }
    if !contact_point.iter().all(|value| value.is_finite()) {
        return Err(FrictionEvidenceError::NonFiniteContactPoint);
    }
    if !mechanical_state_is_finite(body_a) || !mechanical_state_is_finite(body_b) {
        return Err(FrictionEvidenceError::NonFiniteMechanicalState);
    }

    let regime = classify_friction_evidence_regime(body_a, body_b, contact_point)?;
    let pre_a = body_a.kinetic_energy();
    let pre_b = body_b.kinetic_energy();
    if !pre_a.is_finite() || !pre_b.is_finite() || pre_a < 0.0 || pre_b < 0.0 {
        return Err(FrictionEvidenceError::NonFiniteKineticEnergy);
    }

    let snapshot_a = MechanicalSnapshot::capture(body_a);
    let snapshot_b = MechanicalSnapshot::capture(body_b);

    apply_friction_impulse_mechanics(body_a, body_b, contact_point, impulse_on_b);

    if !mechanical_state_is_finite(body_a) || !mechanical_state_is_finite(body_b) {
        snapshot_a.restore(body_a);
        snapshot_b.restore(body_b);
        return Err(FrictionEvidenceError::NonFiniteMechanicalState);
    }

    let post_a = body_a.kinetic_energy();
    let post_b = body_b.kinetic_energy();
    if !post_a.is_finite() || !post_b.is_finite() || post_a < 0.0 || post_b < 0.0 {
        snapshot_a.restore(body_a);
        snapshot_b.restore(body_b);
        return Err(FrictionEvidenceError::NonFiniteKineticEnergy);
    }

    let before_pair = pre_a + pre_b;
    let after_pair = post_a + post_b;
    let pair_delta = after_pair - before_pair;
    if !before_pair.is_finite() || !after_pair.is_finite() || !pair_delta.is_finite() {
        snapshot_a.restore(body_a);
        snapshot_b.restore(body_b);
        return Err(FrictionEvidenceError::UnrepresentablePairDelta);
    }

    let delta = if pair_delta < -ENERGY_EPSILON_J {
        FrictionMechanicalDelta::DissipationCandidate {
            joules: -pair_delta,
        }
    } else if pair_delta > ENERGY_EPSILON_J {
        FrictionMechanicalDelta::SolverInjection { joules: pair_delta }
    } else {
        FrictionMechanicalDelta::Neutral
    };

    Ok(FrictionMechanicalObservation {
        body_a: body_a.handle,
        body_b: body_b.handle,
        contact_point: *contact_point,
        impulse_on_b: *impulse_on_b,
        kinetic_before_a_joules: pre_a,
        kinetic_before_b_joules: pre_b,
        kinetic_after_a_joules: post_a,
        kinetic_after_b_joules: post_b,
        pair_delta_joules: pair_delta,
        delta,
        regime,
        pre_state_a: snapshot_a,
        pre_state_b: snapshot_b,
    })
}

/// Apply one friction impulse exactly as the current world solver does and
/// produce signed, immediate mechanical evidence without authoritative solver
/// transaction identity.
///
/// This path is suitable for diagnostics and analytical reference work. Because
/// the result is unbound, the physical promotion layer must not accept it as an
/// exactly-once heat transaction.
pub fn apply_friction_impulse_measured<const D: usize>(
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    contact_point: &SVector<f64, D>,
    impulse_on_b: &SVector<f64, D>,
) -> Result<FrictionMechanicalObservation<D>, FrictionEvidenceError> {
    apply_friction_impulse_measured_inner(body_a, body_b, contact_point, impulse_on_b)
}

/// Apply one friction impulse and bind the resulting mechanical observation to
/// a deterministic solver transaction identity and exact post-mechanical state.
///
/// This is the only constructor for [`BoundFrictionMechanicalObservation`].
/// The returned token is intentionally non-cloneable and is the input expected
/// by authoritative physical promotion.
pub fn apply_friction_impulse_measured_bound<const D: usize>(
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    contact_point: &SVector<f64, D>,
    impulse_on_b: &SVector<f64, D>,
    transaction_id: FrictionTransactionId,
) -> Result<BoundFrictionMechanicalObservation<D>, FrictionEvidenceError> {
    let observation =
        apply_friction_impulse_measured_inner(body_a, body_b, contact_point, impulse_on_b)?;
    Ok(BoundFrictionMechanicalObservation {
        transaction_id,
        observation,
        post_state_a: BoundMechanicalAuthorityState::capture(body_a),
        post_state_b: BoundMechanicalAuthorityState::capture(body_b),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use symtropy_math::Point;

    fn body(handle: usize, position: [f64; 3], velocity_x: f64) -> RigidBody<3> {
        let mut body = RigidBody::dynamic_sphere(
            BodyHandle(handle),
            Point::new(position),
            0.5,
            1.0,
        );
        body.linear_velocity[0] = velocity_x;
        body
    }

    #[test]
    fn shared_mechanical_kernel_matches_measured_path_state() {
        let contact = SVector::from([0.0, 0.5, 0.0]);
        let impulse = SVector::from([0.0, -0.1, 0.0]);

        let mut direct_a = body(1, [-1.0, 0.0, 0.0], 1.0);
        let mut direct_b = body(2, [1.0, 0.0, 0.0], 0.0);
        let mut measured_a = body(1, [-1.0, 0.0, 0.0], 1.0);
        let mut measured_b = body(2, [1.0, 0.0, 0.0], 0.0);

        apply_friction_impulse_mechanics(
            &mut direct_a,
            &mut direct_b,
            &contact,
            &impulse,
        );
        apply_friction_impulse_measured(
            &mut measured_a,
            &mut measured_b,
            &contact,
            &impulse,
        )
        .unwrap();

        assert_eq!(direct_a.linear_velocity, measured_a.linear_velocity);
        assert_eq!(direct_b.linear_velocity, measured_b.linear_velocity);
        assert_eq!(direct_a.angular_velocity, measured_a.angular_velocity);
        assert_eq!(direct_b.angular_velocity, measured_b.angular_velocity);
    }

    #[test]
    fn centered_loss_is_signed_and_eligible_only_as_candidate() {
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let observation = apply_friction_impulse_measured(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
        )
        .unwrap();

        assert_eq!(
            observation.regime,
            FrictionEvidenceRegime::CenteredClosedDynamicPair
        );
        assert_eq!(
            observation.delta,
            FrictionMechanicalDelta::DissipationCandidate { joules: 0.25 }
        );
        assert_eq!(
            observation.centered_promotable_loss_candidate_joules(),
            Some(0.25)
        );
    }

    #[test]
    fn bound_observation_carries_identity_and_exact_post_state() {
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let id = FrictionTransactionId::new(9, 2, 4, 1);
        let bound = apply_friction_impulse_measured_bound(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            id,
        )
        .unwrap();

        assert_eq!(bound.transaction_id(), id);
        assert_eq!(
            bound.observation().delta,
            FrictionMechanicalDelta::DissipationCandidate { joules: 0.25 }
        );
        assert!(bound.matches_post_state(&a, &b));

        // Change direction while preserving body A's scalar kinetic energy.
        // A KE-only freshness check could miss this; exact state binding cannot.
        let speed = a.linear_velocity.norm();
        a.linear_velocity = SVector::from([0.0, speed, 0.0]);
        assert!(!bound.matches_post_state(&a, &b));
    }

    #[test]
    fn observation_retains_private_pre_mechanical_velocity_provenance() {
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        a.angular_velocity.set(0, 1, 3.0);
        b.angular_velocity.set(0, 1, -2.0);
        let expected_a_linear = a.linear_velocity;
        let expected_b_linear = b.linear_velocity;
        let expected_a_angular = a.angular_velocity;
        let expected_b_angular = b.angular_velocity;

        let observation = apply_friction_impulse_measured(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
        )
        .unwrap();

        assert_eq!(observation.pre_linear_velocity_a(), expected_a_linear);
        assert_eq!(observation.pre_linear_velocity_b(), expected_b_linear);
        assert_eq!(observation.pre_angular_velocity_a(), expected_a_angular);
        assert_eq!(observation.pre_angular_velocity_b(), expected_b_angular);
    }

    #[test]
    fn bound_observation_rejects_mechanical_parameter_reinterpretation() {
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let bound = apply_friction_impulse_measured_bound(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionTransactionId::new(9, 2, 4, 2),
        )
        .unwrap();
        assert!(bound.matches_post_state(&a, &b));

        // Preserve reciprocal coherence and all velocities while changing the
        // quadratic that would interpret this exact transition.
        a.inertia[0] *= 2.0;
        a.inv_inertia[0] *= 0.5;
        assert!(!bound.matches_post_state(&a, &b));
    }

    #[test]
    fn bound_observation_rejects_body_type_or_position_drift() {
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let bound = apply_friction_impulse_measured_bound(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionTransactionId::new(9, 2, 4, 3),
        )
        .unwrap();
        assert!(bound.matches_post_state(&a, &b));

        a.body_type = BodyType::Kinematic;
        assert!(!bound.matches_post_state(&a, &b));
        a.body_type = BodyType::Dynamic;
        assert!(bound.matches_post_state(&a, &b));

        a.transform.translation.0[0] += 1.0e-9;
        assert!(!bound.matches_post_state(&a, &b));
    }

    #[test]
    fn energy_injecting_impulse_is_not_absed_into_dissipation() {
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let observation = apply_friction_impulse_measured(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([2.0, 0.0, 0.0]),
        )
        .unwrap();

        assert_eq!(
            observation.delta,
            FrictionMechanicalDelta::SolverInjection { joules: 2.0 }
        );
        assert_eq!(
            observation.centered_promotable_loss_candidate_joules(),
            None
        );
    }

    #[test]
    fn off_center_loss_remains_unqualified_even_when_measured() {
        let mut a = body(1, [-1.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [1.0, 0.0, 0.0], 0.0);
        let contact = SVector::from([0.0, 0.5, 0.0]);
        let observation = apply_friction_impulse_measured(
            &mut a,
            &mut b,
            &contact,
            &SVector::from([0.0, -0.1, 0.0]),
        )
        .unwrap();

        assert_eq!(observation.regime, FrictionEvidenceRegime::OffCenterUnqualified);
        assert_eq!(
            observation.centered_promotable_loss_candidate_joules(),
            None
        );
    }

    #[test]
    fn dynamic_static_pair_is_an_explicit_boundary_regime() {
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = RigidBody::<3>::static_body(
            BodyHandle(2),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.5)),
        );
        let observation = apply_friction_impulse_measured(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.25, 0.0, 0.0]),
        )
        .unwrap();

        assert_eq!(
            observation.regime,
            FrictionEvidenceRegime::ExternalBoundaryUnqualified
        );
        assert_eq!(
            observation.centered_promotable_loss_candidate_joules(),
            None
        );
    }

    #[test]
    fn invalid_impulse_fails_before_mutation() {
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity);
        let before_b = (b.linear_velocity, b.angular_velocity);

        let error = apply_friction_impulse_measured(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([f64::NAN, 0.0, 0.0]),
        )
        .unwrap_err();

        assert_eq!(error, FrictionEvidenceError::NonFiniteImpulse);
        assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
    }

    #[test]
    fn unrepresentable_post_energy_rolls_back_mechanics() {
        let mut a = body(1, [0.0, 0.0, 0.0], 0.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity);
        let before_b = (b.linear_velocity, b.angular_velocity);

        let error = apply_friction_impulse_measured(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([f64::MAX, 0.0, 0.0]),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            FrictionEvidenceError::NonFiniteKineticEnergy
                | FrictionEvidenceError::NonFiniteMechanicalState
                | FrictionEvidenceError::UnrepresentablePairDelta
        ));
        assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
    }
}
