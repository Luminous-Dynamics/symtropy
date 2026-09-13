// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Launcher adapter binding the core solver friction-authority contract to the
//! fixed-tick thermodynamic transaction runtime.
//!
//! For the production 2D path, checked solver-consistent energy becomes mandatory
//! only for the centered dynamic/dynamic regime that can authorize physical heat.
//! Off-center and external-boundary friction remains mechanics-first diagnostic
//! evidence, so the measurement layer does not become a second physics controller.

use nalgebra::SVector;
use symtropy_physics::{
    FrictionEvidenceRegime, FrictionImpulseAuthority, FrictionPairEnergy2dError,
    FrictionPairEnergyDelta2d, FrictionSolverCoordinates, HeatPartition, RigidBody,
    capture_friction_pair_energy_2d_checked, classify_friction_evidence_regime,
    classify_friction_pair_energy_change_2d_checked,
};

use super::thermodynamic_runtime::{
    RuntimeFrictionError, TerminalFrictionOutcome, ThermodynamicTransactionRuntime,
};

const ENERGY_REL_TOLERANCE: f64 = 1.0e-12;

#[inline]
fn close_enough(a: f64, b: f64, scale: f64) -> bool {
    a.is_finite()
        && b.is_finite()
        && (a - b).abs() <= ENERGY_REL_TOLERANCE * scale.abs().max(1.0)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TerminalFrictionEvidenceMismatch {
    CenteredTransactionDidNotRemainCentered,
    PromotedWithoutCheckedDissipation,
    PromotedDissipationMismatch,
    PromotedBodyAChangeMismatch,
    PromotedBodyBChangeMismatch,
    DiagnosticSolverInjectionMismatch,
    DiagnosticNeutralMismatch,
}

#[derive(Debug)]
pub enum ThermodynamicFrictionAuthorityError {
    CheckedPre(FrictionPairEnergy2dError),
    Runtime(RuntimeFrictionError),
    CheckedPost(FrictionPairEnergy2dError),
    CheckedClassification(FrictionPairEnergy2dError),
    TerminalEvidenceMismatch(TerminalFrictionEvidenceMismatch),
}

impl From<RuntimeFrictionError> for ThermodynamicFrictionAuthorityError {
    fn from(value: RuntimeFrictionError) -> Self {
        Self::Runtime(value)
    }
}

/// Borrowed production friction authority for one physics consequence.
///
/// The heat partition is explicit at construction rather than hidden inside the
/// core physics solver. The runtime remains the owner of fixed-tick identity,
/// friction lifecycle, thermal promotion, and the canonical physical ledger.
pub struct ThermodynamicFrictionAuthority<'a> {
    runtime: &'a mut ThermodynamicTransactionRuntime,
    partition: HeatPartition,
}

impl<'a> ThermodynamicFrictionAuthority<'a> {
    pub fn new(
        runtime: &'a mut ThermodynamicTransactionRuntime,
        partition: HeatPartition,
    ) -> Self {
        Self { runtime, partition }
    }

    /// Current v0.1 production policy: split certified centered friction heat
    /// equally between the two dynamic participants.
    pub fn equal(runtime: &'a mut ThermodynamicTransactionRuntime) -> Self {
        Self::new(runtime, HeatPartition::equal())
    }
}

impl FrictionImpulseAuthority<2> for ThermodynamicFrictionAuthority<'_> {
    type Error = ThermodynamicFrictionAuthorityError;

    fn execute_friction_impulse(
        &mut self,
        body_a: &mut RigidBody<2>,
        body_b: &mut RigidBody<2>,
        contact_point: &SVector<f64, 2>,
        impulse_on_b: &SVector<f64, 2>,
        coordinates: FrictionSolverCoordinates,
    ) -> Result<(), Self::Error> {
        let regime = classify_friction_evidence_regime(body_a, body_b, contact_point)
            .ok();

        // Checked solver-consistent energy is mandatory only for the regime that
        // can actually promote physical heat. Diagnostic-only geometry must not
        // gain veto authority over otherwise valid mechanics.
        let checked_before = if regime == Some(FrictionEvidenceRegime::CenteredClosedDynamicPair) {
            Some(
                capture_friction_pair_energy_2d_checked(body_a, body_b)
                    .map_err(ThermodynamicFrictionAuthorityError::CheckedPre)?,
            )
        } else {
            None
        };

        let terminal = self.runtime.execute_terminal_friction_impulse_at(
            body_a,
            body_b,
            contact_point,
            impulse_on_b,
            coordinates,
            self.partition,
        )?;

        let Some(before) = checked_before else {
            return Ok(());
        };

        // A centered transaction does not move positions/body type. If the live
        // geometry no longer classifies centered here, the terminal receipt cannot
        // be admitted as coherent checked physical evidence.
        if classify_friction_evidence_regime(body_a, body_b, contact_point)
            != Ok(FrictionEvidenceRegime::CenteredClosedDynamicPair)
        {
            return Err(ThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                TerminalFrictionEvidenceMismatch::CenteredTransactionDidNotRemainCentered,
            ));
        }

        let after = capture_friction_pair_energy_2d_checked(body_a, body_b)
            .map_err(ThermodynamicFrictionAuthorityError::CheckedPost)?;
        let checked = classify_friction_pair_energy_change_2d_checked(before, after)
            .map_err(ThermodynamicFrictionAuthorityError::CheckedClassification)?;

        match terminal.outcome {
            TerminalFrictionOutcome::Promoted(receipt) => {
                let checked_dissipated = match checked.delta {
                    FrictionPairEnergyDelta2d::DissipationCandidate { joules } => joules,
                    FrictionPairEnergyDelta2d::SolverInjection { .. }
                    | FrictionPairEnergyDelta2d::Neutral => {
                        return Err(
                            ThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                                TerminalFrictionEvidenceMismatch::PromotedWithoutCheckedDissipation,
                            ),
                        );
                    }
                };

                let checked_change_a =
                    checked.after.kinetic_a_joules - checked.before.kinetic_a_joules;
                let checked_change_b =
                    checked.after.kinetic_b_joules - checked.before.kinetic_b_joules;
                if !close_enough(
                    receipt.heat.dissipated_joules,
                    checked_dissipated,
                    checked_dissipated,
                ) {
                    return Err(ThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                        TerminalFrictionEvidenceMismatch::PromotedDissipationMismatch,
                    ));
                }
                if !close_enough(
                    receipt.heat.kinetic_change_a_joules,
                    checked_change_a,
                    checked_dissipated,
                ) {
                    return Err(ThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                        TerminalFrictionEvidenceMismatch::PromotedBodyAChangeMismatch,
                    ));
                }
                if !close_enough(
                    receipt.heat.kinetic_change_b_joules,
                    checked_change_b,
                    checked_dissipated,
                ) {
                    return Err(ThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                        TerminalFrictionEvidenceMismatch::PromotedBodyBChangeMismatch,
                    ));
                }
            }
            TerminalFrictionOutcome::Diagnostic(reason) => match reason {
                symtropy_physics::FrictionDiagnosticReason::SolverInjection => {
                    if !matches!(checked.delta, FrictionPairEnergyDelta2d::SolverInjection { .. }) {
                        return Err(
                            ThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                                TerminalFrictionEvidenceMismatch::DiagnosticSolverInjectionMismatch,
                            ),
                        );
                    }
                }
                symtropy_physics::FrictionDiagnosticReason::Neutral => {
                    if checked.delta != FrictionPairEnergyDelta2d::Neutral {
                        return Err(
                            ThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                                TerminalFrictionEvidenceMismatch::DiagnosticNeutralMismatch,
                            ),
                        );
                    }
                }
                // A transaction pre-classified as centered cannot coherently end
                // in either geometry-only diagnostic class; the check above would
                // already have failed if the live geometry changed.
                symtropy_physics::FrictionDiagnosticReason::OffCenterUnqualified
                | symtropy_physics::FrictionDiagnosticReason::ExternalBoundaryUnqualified => {
                    return Err(ThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                        TerminalFrictionEvidenceMismatch::CenteredTransactionDidNotRemainCentered,
                    ));
                }
            },
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::{Point, Sphere, Transform};
    use symtropy_physics::{
        BodyHandle, BodyType, FrictionTransactionId, FrictionTransactionPhase, ThermalBody,
        ThermalMaterial, ThermalState,
    };

    fn thermal_body(handle: usize, velocity_x: f64) -> RigidBody<2> {
        let mut body = RigidBody::<2>::dynamic_sphere(
            BodyHandle(handle),
            Point::origin(),
            0.5,
            1.0,
        );
        body.linear_velocity[0] = velocity_x;
        body.set_thermal(
            ThermalBody::new(
                ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
                ThermalState::new(300.0).unwrap(),
                1.0,
            )
            .unwrap(),
        );
        body
    }

    fn anisotropic_thermal_body(handle: usize, velocity_x: f64) -> RigidBody<2> {
        let mut body = RigidBody::new(
            BodyHandle(handle),
            BodyType::Dynamic,
            Transform::from_translation(Point::origin()),
            Box::new(Sphere::<2>::unit()),
            2.0,
            SVector::from([2.0, 8.0]),
        );
        body.linear_velocity[0] = velocity_x;
        body.set_thermal(
            ThermalBody::new(
                ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
                ThermalState::new(300.0).unwrap(),
                1.0,
            )
            .unwrap(),
        );
        body
    }

    #[test]
    fn centered_authority_routes_into_runtime_promotion_with_checked_agreement() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = thermal_body(1, 1.0);
        let mut b = thermal_body(2, 0.0);

        {
            let mut authority = ThermodynamicFrictionAuthority::equal(&mut runtime);
            authority
                .execute_friction_impulse(
                    &mut a,
                    &mut b,
                    &SVector::zeros(),
                    &SVector::from([0.5, 0.0]),
                    FrictionSolverCoordinates::new(2, 3, 4),
                )
                .unwrap();
        }

        let id = FrictionTransactionId::new(0, 2, 3, 4);
        assert_eq!(
            runtime.friction_journal().phase(id),
            Some(FrictionTransactionPhase::Promoted)
        );
        assert_eq!(runtime.physical_energy_ledger().len(), 3);
        assert_eq!(runtime.pending_friction_reservation_count(), 0);
    }

    #[test]
    fn centered_inconsistent_mass_fails_before_mechanical_mutation_or_ledger_write() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = thermal_body(1, 1.0);
        let mut b = thermal_body(2, 0.0);
        a.inv_mass *= 0.5;
        let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
        let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);

        let error = {
            let mut authority = ThermodynamicFrictionAuthority::equal(&mut runtime);
            authority
                .execute_friction_impulse(
                    &mut a,
                    &mut b,
                    &SVector::zeros(),
                    &SVector::from([0.25, 0.0]),
                    FrictionSolverCoordinates::new(0, 0, 0),
                )
                .unwrap_err()
        };

        assert!(matches!(
            error,
            ThermodynamicFrictionAuthorityError::CheckedPre(
                FrictionPairEnergy2dError::BodyA(_)
            )
        ));
        assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
        assert!(runtime.friction_journal().is_empty());
        assert!(runtime.physical_energy_ledger().is_empty());
    }

    #[test]
    fn centered_anisotropic_state_can_promote_when_solver_consistent_delta_agrees() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = anisotropic_thermal_body(1, 1.0);
        let mut b = anisotropic_thermal_body(2, 0.0);
        a.angular_velocity.set(0, 1, 2.0);
        b.angular_velocity.set(0, 1, -1.5);

        let before_a_legacy = a.kinetic_energy();
        let before_a_checked = a.kinetic_energy_2d_solver_checked().unwrap();
        assert!((before_a_legacy - before_a_checked).abs() > 1.0);

        {
            let mut authority = ThermodynamicFrictionAuthority::equal(&mut runtime);
            authority
                .execute_friction_impulse(
                    &mut a,
                    &mut b,
                    &SVector::zeros(),
                    &SVector::from([0.5, 0.0]),
                    FrictionSolverCoordinates::new(1, 2, 3),
                )
                .unwrap();
        }

        assert_eq!(
            runtime
                .friction_journal()
                .phase(FrictionTransactionId::new(0, 1, 2, 3)),
            Some(FrictionTransactionPhase::Promoted)
        );
    }

    #[test]
    fn off_center_diagnostic_path_does_not_gain_checked_energy_veto_authority() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = anisotropic_thermal_body(1, 0.0);
        let mut b = anisotropic_thermal_body(2, 0.0);
        a.transform.translation = Point::new([-1.0, 0.0]);
        b.transform.translation = Point::new([1.0, 0.0]);
        // Corrupt reciprocal inertia deliberately. The contact is off-center, so
        // this makes checked energy unavailable but must not suppress mechanics.
        a.inv_inertia[1] *= 0.5;
        let before_a = a.linear_velocity;
        let before_b = b.linear_velocity;

        {
            let mut authority = ThermodynamicFrictionAuthority::equal(&mut runtime);
            authority
                .execute_friction_impulse(
                    &mut a,
                    &mut b,
                    &SVector::from([0.0, 0.5]),
                    &SVector::from([0.0, -0.1]),
                    FrictionSolverCoordinates::new(0, 1, 0),
                )
                .unwrap();
        }

        assert_ne!(a.linear_velocity, before_a);
        assert_ne!(b.linear_velocity, before_b);
        assert_eq!(
            runtime
                .friction_journal()
                .phase(FrictionTransactionId::new(0, 0, 1, 0)),
            Some(FrictionTransactionPhase::DiagnosticOnly(
                symtropy_physics::FrictionDiagnosticReason::OffCenterUnqualified
            ))
        );
        assert!(runtime.physical_energy_ledger().is_empty());
    }
}
