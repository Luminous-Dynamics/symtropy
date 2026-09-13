// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure scheduler policy for converting one typed physics consequence into either
//! an admissible thermodynamic close or a fail-stop poison decision.
//!
//! The key authority rule is structural: a solver authority fault never becomes
//! `ThermodynamicConsequenceStatus::Rejected`. Rejection is reserved for a
//! consequence that provably did not begin, such as an invalid timestep.

use super::engine_physics::{PhysicsConsequenceOutcome, PhysicsConsequenceRejection};
use super::thermodynamic_transaction::ThermodynamicConsequenceStatus;

#[derive(Debug, PartialEq, Eq)]
pub enum ThermodynamicConsequenceDecision<E> {
    Close {
        status: ThermodynamicConsequenceStatus,
        export_2d_on_commit: bool,
    },
    Poison(E),
}

/// Classify one dynamic-2D physics consequence for thermodynamic scheduling.
///
/// `AuthorityFault(error)` preserves `error` by value and has no close status.
/// Therefore a caller cannot accidentally pass a solver authority failure to the
/// ordinary open/close path without explicitly violating this decision type.
pub fn dynamic_2d_consequence_decision<E>(
    outcome: PhysicsConsequenceOutcome<E>,
) -> ThermodynamicConsequenceDecision<E> {
    match outcome {
        PhysicsConsequenceOutcome::Executed => ThermodynamicConsequenceDecision::Close {
            status: ThermodynamicConsequenceStatus::Executed,
            export_2d_on_commit: true,
        },
        PhysicsConsequenceOutcome::Rejected(PhysicsConsequenceRejection::InvalidTimestep) => {
            ThermodynamicConsequenceDecision::Close {
                status: ThermodynamicConsequenceStatus::Rejected,
                export_2d_on_commit: false,
            }
        }
        PhysicsConsequenceOutcome::AuthorityFault(error) => {
            ThermodynamicConsequenceDecision::Poison(error)
        }
    }
}

/// The 3D compatibility mode intentionally has no consequential 2D physics
/// authority, so it closes as `IntentionallyAbsent` and never exports 2D state.
pub fn kinematic_3d_consequence_decision<E>() -> ThermodynamicConsequenceDecision<E> {
    ThermodynamicConsequenceDecision::Close {
        status: ThermodynamicConsequenceStatus::IntentionallyAbsent,
        export_2d_on_commit: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct ExactAuthorityFault {
        body_a: usize,
        body_b: usize,
        solver_iteration: u32,
        contact_sequence: u32,
        point_sequence: u32,
    }

    #[test]
    fn executed_dynamic_consequence_closes_as_executed_and_exports() {
        assert_eq!(
            dynamic_2d_consequence_decision::<ExactAuthorityFault>(
                PhysicsConsequenceOutcome::Executed
            ),
            ThermodynamicConsequenceDecision::Close {
                status: ThermodynamicConsequenceStatus::Executed,
                export_2d_on_commit: true,
            }
        );
    }

    #[test]
    fn invalid_timestep_closes_as_rejected_without_export() {
        assert_eq!(
            dynamic_2d_consequence_decision::<ExactAuthorityFault>(
                PhysicsConsequenceOutcome::Rejected(
                    PhysicsConsequenceRejection::InvalidTimestep
                )
            ),
            ThermodynamicConsequenceDecision::Close {
                status: ThermodynamicConsequenceStatus::Rejected,
                export_2d_on_commit: false,
            }
        );
    }

    #[test]
    fn authority_fault_is_preserved_exactly_and_has_no_close_status() {
        let fault = ExactAuthorityFault {
            body_a: 7,
            body_b: 11,
            solver_iteration: 3,
            contact_sequence: 19,
            point_sequence: 2,
        };

        assert_eq!(
            dynamic_2d_consequence_decision(PhysicsConsequenceOutcome::AuthorityFault(fault)),
            ThermodynamicConsequenceDecision::Poison(ExactAuthorityFault {
                body_a: 7,
                body_b: 11,
                solver_iteration: 3,
                contact_sequence: 19,
                point_sequence: 2,
            })
        );
    }

    #[test]
    fn kinematic_3d_is_intentionally_absent_without_export() {
        assert_eq!(
            kinematic_3d_consequence_decision::<ExactAuthorityFault>(),
            ThermodynamicConsequenceDecision::Close {
                status: ThermodynamicConsequenceStatus::IntentionallyAbsent,
                export_2d_on_commit: false,
            }
        );
    }
}
