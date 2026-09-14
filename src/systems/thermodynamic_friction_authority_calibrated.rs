// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Solver-facing calibrated thermodynamic friction authority.
//!
//! Construction requires a pre-consequence [`PhysicalFrictionReadiness`] receipt,
//! so the authority carries the exact admitted M-L-T calibration and canonical
//! dynamic-body census for the consequence it is about to govern. The runtime
//! remains the owner of fixed-tick identity, lifecycle, thermal promotion and the
//! canonical physical ledger.

use nalgebra::SVector;
use symtropy_physics::{
    BodyHandle, FrictionEvidenceRegime, FrictionImpulseAuthority, FrictionPairEnergy2dError,
    FrictionPairEnergyDelta2d, FrictionSolverCoordinates, HeatPartition,
    MechanicalUnitCalibrationError, RigidBody, capture_friction_pair_energy_2d_checked,
    classify_friction_evidence_regime, classify_friction_pair_energy_change_2d_checked,
};

use super::thermodynamic_physical_admission::PhysicalFrictionReadiness;
use super::thermodynamic_runtime::{
    CalibratedRuntimeFrictionError, TerminalFrictionOutcome, ThermodynamicTransactionRuntime,
};

const ENERGY_REL_TOLERANCE: f64 = 1.0e-12;

#[inline]
fn close_enough_physical(a: f64, b: f64, scale: f64) -> bool {
    if !a.is_finite() || !b.is_finite() || !scale.is_finite() || scale < 0.0 {
        return false;
    }
    let tolerance = ENERGY_REL_TOLERANCE * scale;
    tolerance.is_finite() && (a - b).abs() <= tolerance
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum CalibratedFrictionAdmissionMismatch {
    ParticipantDynamicCensusMismatch {
        body: BodyHandle,
        admitted_dynamic: bool,
        current_dynamic: bool,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum CalibratedTerminalFrictionEvidenceMismatch {
    CenteredTransactionDidNotRemainCentered,
    PromotedWithoutCheckedDissipation,
    PromotedDissipationMismatch,
    PromotedBodyAChangeMismatch,
    PromotedBodyBChangeMismatch,
    DiagnosticSolverInjectionMismatch,
    DiagnosticNeutralMismatch,
}

#[derive(Debug)]
pub(crate) enum CalibratedThermodynamicFrictionAuthorityError {
    Admission(CalibratedFrictionAdmissionMismatch),
    CheckedPre(FrictionPairEnergy2dError),
    Runtime(CalibratedRuntimeFrictionError),
    CheckedPost(FrictionPairEnergy2dError),
    CheckedClassification(FrictionPairEnergy2dError),
    Calibration(MechanicalUnitCalibrationError),
    TerminalEvidenceMismatch(CalibratedTerminalFrictionEvidenceMismatch),
}

/// Borrowed production friction authority for one admitted physics consequence.
///
/// The non-cloneable readiness receipt is consumed at construction so ordinary
/// safe callers cannot construct this authority from a naked calibration, bypass
/// the canonical dynamic thermal census, or fan one admission out into multiple
/// independent physical-friction authorities.
pub(crate) struct CalibratedThermodynamicFrictionAuthority<'a> {
    runtime: &'a mut ThermodynamicTransactionRuntime,
    partition: HeatPartition,
    readiness: PhysicalFrictionReadiness,
}

impl<'a> CalibratedThermodynamicFrictionAuthority<'a> {
    pub(crate) fn new(
        runtime: &'a mut ThermodynamicTransactionRuntime,
        partition: HeatPartition,
        readiness: PhysicalFrictionReadiness,
    ) -> Self {
        Self {
            runtime,
            partition,
            readiness,
        }
    }

    pub(crate) fn equal(
        runtime: &'a mut ThermodynamicTransactionRuntime,
        readiness: PhysicalFrictionReadiness,
    ) -> Self {
        Self::new(runtime, HeatPartition::equal(), readiness)
    }

    /// Require the current participant's dynamic/static classification to agree
    /// with the exact pre-consequence dynamic census.
    ///
    /// This is deliberately symmetric. It rejects both a late dynamic insertion
    /// (`false -> true`) and an admitted dynamic body that was silently converted
    /// to static/kinematic (`true -> false`). Static/kinematic participants that
    /// were already outside the dynamic census remain admissible for the existing
    /// external-boundary diagnostic path.
    fn require_admitted_participant_census(
        &self,
        body: &RigidBody<2>,
    ) -> Result<(), CalibratedFrictionAdmissionMismatch> {
        let admitted_dynamic = self
            .readiness
            .dynamic_handles()
            .binary_search(&body.handle)
            .is_ok();
        let current_dynamic = body.is_dynamic();
        if admitted_dynamic != current_dynamic {
            return Err(
                CalibratedFrictionAdmissionMismatch::ParticipantDynamicCensusMismatch {
                    body: body.handle,
                    admitted_dynamic,
                    current_dynamic,
                },
            );
        }
        Ok(())
    }

    fn require_admitted_participants(
        &self,
        body_a: &RigidBody<2>,
        body_b: &RigidBody<2>,
    ) -> Result<(), CalibratedFrictionAdmissionMismatch> {
        self.require_admitted_participant_census(body_a)?;
        self.require_admitted_participant_census(body_b)?;
        Ok(())
    }

    fn fail(
        &mut self,
        error: CalibratedThermodynamicFrictionAuthorityError,
    ) -> Result<(), CalibratedThermodynamicFrictionAuthorityError> {
        self.runtime.poison_authority();
        Err(error)
    }
}

impl FrictionImpulseAuthority<2> for CalibratedThermodynamicFrictionAuthority<'_> {
    type Error = CalibratedThermodynamicFrictionAuthorityError;

    fn execute_friction_impulse(
        &mut self,
        body_a: &mut RigidBody<2>,
        body_b: &mut RigidBody<2>,
        contact_point: &SVector<f64, 2>,
        impulse_on_b: &SVector<f64, 2>,
        coordinates: FrictionSolverCoordinates,
    ) -> Result<(), Self::Error> {
        if let Err(error) = self.require_admitted_participants(body_a, body_b) {
            return self.fail(CalibratedThermodynamicFrictionAuthorityError::Admission(error));
        }

        let calibration = self.readiness.calibration();
        let regime = classify_friction_evidence_regime(body_a, body_b, contact_point).ok();

        // Checked solver-consistent energy is mandatory only for the regime that
        // can authorize physical heat. Diagnostic-only geometry must not gain a
        // checked-energy veto over otherwise valid mechanics.
        let checked_before = if regime == Some(FrictionEvidenceRegime::CenteredClosedDynamicPair) {
            match capture_friction_pair_energy_2d_checked(body_a, body_b) {
                Ok(before) => Some(before),
                Err(error) => {
                    return self.fail(CalibratedThermodynamicFrictionAuthorityError::CheckedPre(
                        error,
                    ));
                }
            }
        } else {
            None
        };

        let terminal = match self.runtime.execute_terminal_friction_impulse_at_calibrated(
            body_a,
            body_b,
            contact_point,
            impulse_on_b,
            coordinates,
            self.partition,
            calibration,
        ) {
            Ok(terminal) => terminal,
            Err(error) => {
                return self.fail(CalibratedThermodynamicFrictionAuthorityError::Runtime(error));
            }
        };

        let Some(before) = checked_before else {
            return Ok(());
        };

        if classify_friction_evidence_regime(body_a, body_b, contact_point)
            != Ok(FrictionEvidenceRegime::CenteredClosedDynamicPair)
        {
            return self.fail(
                CalibratedThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                    CalibratedTerminalFrictionEvidenceMismatch::CenteredTransactionDidNotRemainCentered,
                ),
            );
        }

        let after = match capture_friction_pair_energy_2d_checked(body_a, body_b) {
            Ok(after) => after,
            Err(error) => {
                return self.fail(CalibratedThermodynamicFrictionAuthorityError::CheckedPost(
                    error,
                ));
            }
        };
        let checked = match classify_friction_pair_energy_change_2d_checked(before, after) {
            Ok(checked) => checked,
            Err(error) => {
                return self.fail(
                    CalibratedThermodynamicFrictionAuthorityError::CheckedClassification(error),
                );
            }
        };

        match terminal.outcome {
            TerminalFrictionOutcome::Promoted(receipt) => {
                let checked_dissipated_solver = match checked.delta {
                    FrictionPairEnergyDelta2d::DissipationCandidate { joules } => joules,
                    FrictionPairEnergyDelta2d::SolverInjection { .. }
                    | FrictionPairEnergyDelta2d::Neutral => {
                        return self.fail(
                            CalibratedThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                                CalibratedTerminalFrictionEvidenceMismatch::PromotedWithoutCheckedDissipation,
                            ),
                        );
                    }
                };

                let checked_change_a_solver =
                    checked.after.kinetic_a_joules - checked.before.kinetic_a_joules;
                let checked_change_b_solver =
                    checked.after.kinetic_b_joules - checked.before.kinetic_b_joules;
                let checked_dissipated = match calibration
                    .energy_to_joules(checked_dissipated_solver)
                {
                    Ok(value) => value,
                    Err(error) => {
                        return self.fail(
                            CalibratedThermodynamicFrictionAuthorityError::Calibration(error),
                        );
                    }
                };
                let checked_change_a = match calibration
                    .signed_energy_to_joules(checked_change_a_solver)
                {
                    Ok(value) => value,
                    Err(error) => {
                        return self.fail(
                            CalibratedThermodynamicFrictionAuthorityError::Calibration(error),
                        );
                    }
                };
                let checked_change_b = match calibration
                    .signed_energy_to_joules(checked_change_b_solver)
                {
                    Ok(value) => value,
                    Err(error) => {
                        return self.fail(
                            CalibratedThermodynamicFrictionAuthorityError::Calibration(error),
                        );
                    }
                };

                if !close_enough_physical(
                    receipt.heat.dissipated_joules,
                    checked_dissipated,
                    checked_dissipated,
                ) {
                    return self.fail(
                        CalibratedThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                            CalibratedTerminalFrictionEvidenceMismatch::PromotedDissipationMismatch,
                        ),
                    );
                }
                if !close_enough_physical(
                    receipt.heat.kinetic_change_a_joules,
                    checked_change_a,
                    checked_dissipated,
                ) {
                    return self.fail(
                        CalibratedThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                            CalibratedTerminalFrictionEvidenceMismatch::PromotedBodyAChangeMismatch,
                        ),
                    );
                }
                if !close_enough_physical(
                    receipt.heat.kinetic_change_b_joules,
                    checked_change_b,
                    checked_dissipated,
                ) {
                    return self.fail(
                        CalibratedThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                            CalibratedTerminalFrictionEvidenceMismatch::PromotedBodyBChangeMismatch,
                        ),
                    );
                }
            }
            TerminalFrictionOutcome::Diagnostic(reason) => match reason {
                symtropy_physics::FrictionDiagnosticReason::SolverInjection => {
                    if !matches!(checked.delta, FrictionPairEnergyDelta2d::SolverInjection { .. }) {
                        return self.fail(
                            CalibratedThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                                CalibratedTerminalFrictionEvidenceMismatch::DiagnosticSolverInjectionMismatch,
                            ),
                        );
                    }
                }
                symtropy_physics::FrictionDiagnosticReason::Neutral => {
                    if checked.delta != FrictionPairEnergyDelta2d::Neutral {
                        return self.fail(
                            CalibratedThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                                CalibratedTerminalFrictionEvidenceMismatch::DiagnosticNeutralMismatch,
                            ),
                        );
                    }
                }
                symtropy_physics::FrictionDiagnosticReason::OffCenterUnqualified
                | symtropy_physics::FrictionDiagnosticReason::ExternalBoundaryUnqualified => {
                    return self.fail(
                        CalibratedThermodynamicFrictionAuthorityError::TerminalEvidenceMismatch(
                            CalibratedTerminalFrictionEvidenceMismatch::CenteredTransactionDidNotRemainCentered,
                        ),
                    );
                }
            },
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;
    use symtropy_physics::{
        BodyType, FrictionDiagnosticReason, FrictionTransactionId, FrictionTransactionPhase,
        PhysicsWorld, ThermalBody, ThermalMaterial, ThermalState,
    };

    use super::super::thermodynamic_physical_admission::admit_physical_friction_readiness;

    fn thermal() -> ThermalBody {
        ThermalBody::new(
            ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
            ThermalState::new(300.0).unwrap(),
            1.0,
        )
        .unwrap()
    }

    fn admitted_pair(
        velocity_a: f64,
        velocity_b: f64,
    ) -> (
        RigidBody<2>,
        RigidBody<2>,
        PhysicalFrictionReadiness,
    ) {
        let mut world = PhysicsWorld::<2>::default();
        let a = world.add_sphere(Point::origin(), 0.5, 1.0);
        let b = world.add_sphere(Point::origin(), 0.5, 1.0);
        world.body_mut(a).unwrap().set_thermal(thermal());
        world.body_mut(b).unwrap().set_thermal(thermal());
        let readiness = admit_physical_friction_readiness(
            &world,
            Some(symtropy_physics::MechanicalUnitCalibration::new(
                1.0,
                1.0 / 32.0,
                1.0,
            )
            .unwrap()),
        )
        .unwrap();

        let mut body_a = RigidBody::<2>::dynamic_sphere(a, Point::origin(), 0.5, 1.0);
        let mut body_b = RigidBody::<2>::dynamic_sphere(b, Point::origin(), 0.5, 1.0);
        body_a.linear_velocity[0] = velocity_a;
        body_b.linear_velocity[0] = velocity_b;
        body_a.set_thermal(thermal());
        body_b.set_thermal(thermal());
        (body_a, body_b, readiness)
    }

    #[test]
    fn non_si_authority_promotes_and_checked_evidence_agrees_after_same_calibration() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let (mut a, mut b, readiness) = admitted_pair(1.0, 0.0);

        {
            let mut authority = CalibratedThermodynamicFrictionAuthority::equal(
                &mut runtime,
                readiness,
            );
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
        assert!(!runtime.is_authority_poisoned());
        let transferred: f64 = runtime
            .physical_energy_ledger()
            .entries()
            .iter()
            .filter(|entry| {
                matches!(
                    entry.destination.form,
                    symtropy_physics::EnergyForm::ThermalSensible
                )
            })
            .map(|entry| entry.joules)
            .sum();
        assert_eq!(transferred, 0.25 / 1024.0);
    }

    #[test]
    fn late_dynamic_participant_is_rejected_before_mechanics() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let (mut a, _b, readiness) = admitted_pair(1.0, 0.0);
        let mut late = RigidBody::<2>::dynamic_sphere(
            BodyHandle(99),
            Point::origin(),
            0.5,
            1.0,
        );
        late.set_thermal(thermal());
        let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
        let before_late = (late.linear_velocity, late.angular_velocity, late.thermal);

        let error = {
            let mut authority = CalibratedThermodynamicFrictionAuthority::equal(
                &mut runtime,
                readiness,
            );
            authority
                .execute_friction_impulse(
                    &mut a,
                    &mut late,
                    &SVector::zeros(),
                    &SVector::from([0.5, 0.0]),
                    FrictionSolverCoordinates::new(0, 0, 0),
                )
                .unwrap_err()
        };

        assert!(matches!(
            error,
            CalibratedThermodynamicFrictionAuthorityError::Admission(
                CalibratedFrictionAdmissionMismatch::ParticipantDynamicCensusMismatch {
                    body: BodyHandle(99),
                    admitted_dynamic: false,
                    current_dynamic: true,
                }
            )
        ));
        assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
        assert_eq!(
            (late.linear_velocity, late.angular_velocity, late.thermal),
            before_late
        );
        assert!(runtime.friction_journal().is_empty());
        assert!(runtime.physical_energy_ledger().is_empty());
        assert!(runtime.is_authority_poisoned());
    }

    #[test]
    fn admitted_dynamic_body_type_drift_is_rejected_before_mechanics() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let (mut a, mut b, readiness) = admitted_pair(1.0, 0.0);
        let drifted_handle = a.handle;
        a.body_type = BodyType::Static;
        let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
        let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);

        let error = {
            let mut authority = CalibratedThermodynamicFrictionAuthority::equal(
                &mut runtime,
                readiness,
            );
            authority
                .execute_friction_impulse(
                    &mut a,
                    &mut b,
                    &SVector::zeros(),
                    &SVector::from([0.5, 0.0]),
                    FrictionSolverCoordinates::new(0, 0, 0),
                )
                .unwrap_err()
        };

        assert!(matches!(
            error,
            CalibratedThermodynamicFrictionAuthorityError::Admission(
                CalibratedFrictionAdmissionMismatch::ParticipantDynamicCensusMismatch {
                    body,
                    admitted_dynamic: true,
                    current_dynamic: false,
                } if body == drifted_handle
            )
        ));
        assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
        assert!(runtime.friction_journal().is_empty());
        assert!(runtime.physical_energy_ledger().is_empty());
        assert!(runtime.is_authority_poisoned());
    }

    #[test]
    fn centered_solver_injection_is_diagnostic_not_heat_under_non_si_authority() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let (mut a, mut b, readiness) = admitted_pair(0.0, 0.0);

        {
            let mut authority = CalibratedThermodynamicFrictionAuthority::equal(
                &mut runtime,
                readiness,
            );
            authority
                .execute_friction_impulse(
                    &mut a,
                    &mut b,
                    &SVector::zeros(),
                    &SVector::from([0.5, 0.0]),
                    FrictionSolverCoordinates::new(0, 0, 0),
                )
                .unwrap();
        }

        let id = FrictionTransactionId::new(0, 0, 0, 0);
        assert_eq!(
            runtime.friction_journal().phase(id),
            Some(FrictionTransactionPhase::DiagnosticOnly(
                FrictionDiagnosticReason::SolverInjection
            ))
        );
        assert!(runtime.physical_energy_ledger().is_empty());
        assert!(!runtime.is_authority_poisoned());
    }
}
