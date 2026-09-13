// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Stable, compact operator-facing summary of a typed friction-step authority fault.
//!
//! The scheduler should retain enough exact identity to diagnose a poisoned fixed
//! tick without storing an arbitrarily nested runtime error object or parsing logs.

use symtropy_physics::{
    BodyHandle, FrictionApplicationError, FrictionApplicationRollbackError,
    FrictionAuthorityFailure, FrictionDiagnosticFinalizeError, FrictionPromotionError,
    FrictionSolverCoordinateComponent, FrictionSolverCoordinates, FrictionStepError,
};

use super::thermodynamic_runtime::{RuntimeFrictionError, RuntimeFrictionGateError};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuntimeFrictionTerminalizationSummary {
    Gate(RuntimeFrictionGateError),
    Application(FrictionApplicationError),
    Diagnostic(FrictionDiagnosticFinalizeError),
    Promotion(FrictionPromotionError),
    RollbackInvariant,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuntimeFrictionFaultSummary {
    Gate(RuntimeFrictionGateError),
    Application(FrictionApplicationError),
    Diagnostic(FrictionDiagnosticFinalizeError),
    Promotion(FrictionPromotionError),
    RollbackInvariant {
        terminalization: RuntimeFrictionTerminalizationSummary,
        rollback: FrictionApplicationRollbackError,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThermodynamicPhysicsAuthorityFault {
    /// Native solver traversal exceeded the replay-stable coordinate grammar.
    /// The friction authority was never invoked and no friction mechanics were
    /// applied for this request.
    CoordinateOverflow {
        component: FrictionSolverCoordinateComponent,
        value: usize,
    },
    /// The authority was entered for one exact body pair and exact losslessly
    /// converted solver coordinate.
    FrictionAuthority {
        body_a: BodyHandle,
        body_b: BodyHandle,
        coordinates: FrictionSolverCoordinates,
        reason: RuntimeFrictionFaultSummary,
    },
}

fn terminalization_summary(
    error: &RuntimeFrictionError,
) -> RuntimeFrictionTerminalizationSummary {
    match error {
        RuntimeFrictionError::Gate(error) => RuntimeFrictionTerminalizationSummary::Gate(*error),
        RuntimeFrictionError::Application(error) => {
            RuntimeFrictionTerminalizationSummary::Application(*error)
        }
        RuntimeFrictionError::Diagnostic(error) => {
            RuntimeFrictionTerminalizationSummary::Diagnostic(*error)
        }
        RuntimeFrictionError::Promotion(error) => {
            RuntimeFrictionTerminalizationSummary::Promotion(*error)
        }
        RuntimeFrictionError::Rollback { .. } => {
            RuntimeFrictionTerminalizationSummary::RollbackInvariant
        }
    }
}

fn runtime_fault_summary(error: &RuntimeFrictionError) -> RuntimeFrictionFaultSummary {
    match error {
        RuntimeFrictionError::Gate(error) => RuntimeFrictionFaultSummary::Gate(*error),
        RuntimeFrictionError::Application(error) => {
            RuntimeFrictionFaultSummary::Application(*error)
        }
        RuntimeFrictionError::Diagnostic(error) => {
            RuntimeFrictionFaultSummary::Diagnostic(*error)
        }
        RuntimeFrictionError::Promotion(error) => {
            RuntimeFrictionFaultSummary::Promotion(*error)
        }
        RuntimeFrictionError::Rollback {
            terminalization,
            rollback,
        } => RuntimeFrictionFaultSummary::RollbackInvariant {
            terminalization: terminalization_summary(terminalization),
            rollback: *rollback,
        },
    }
}

/// Reduce one typed solver friction failure to the stable context the scheduler
/// can retain after entering its fail-stop `Poisoned` state.
pub fn summarize_physics_authority_fault(
    error: &FrictionStepError<RuntimeFrictionError>,
) -> ThermodynamicPhysicsAuthorityFault {
    match error {
        FrictionStepError::Coordinate(error) => {
            ThermodynamicPhysicsAuthorityFault::CoordinateOverflow {
                component: error.component,
                value: error.value,
            }
        }
        FrictionStepError::Authority(FrictionAuthorityFailure {
            body_a,
            body_b,
            coordinates,
            error,
        }) => ThermodynamicPhysicsAuthorityFault::FrictionAuthority {
            body_a: *body_a,
            body_b: *body_b,
            coordinates: *coordinates,
            reason: runtime_fault_summary(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_physics::{
        FrictionApplicationRollbackError, FrictionPromotionError,
        FrictionSolverCoordinateError,
    };

    #[test]
    fn coordinate_overflow_preserves_exact_component_and_native_value() {
        let native = (u32::MAX as usize).saturating_add(17);
        let error = FrictionStepError::<RuntimeFrictionError>::Coordinate(
            FrictionSolverCoordinateError {
                component: FrictionSolverCoordinateComponent::PointSequence,
                value: native,
            },
        );

        assert_eq!(
            summarize_physics_authority_fault(&error),
            ThermodynamicPhysicsAuthorityFault::CoordinateOverflow {
                component: FrictionSolverCoordinateComponent::PointSequence,
                value: native,
            }
        );
    }

    #[test]
    fn promotion_failure_preserves_exact_body_pair_coordinates_and_reason() {
        let coordinates = FrictionSolverCoordinates::new(4, 9, 2);
        let error = FrictionStepError::Authority(FrictionAuthorityFailure {
            body_a: BodyHandle(7),
            body_b: BodyHandle(11),
            coordinates,
            error: RuntimeFrictionError::Promotion(FrictionPromotionError::MissingThermalState),
        });

        assert_eq!(
            summarize_physics_authority_fault(&error),
            ThermodynamicPhysicsAuthorityFault::FrictionAuthority {
                body_a: BodyHandle(7),
                body_b: BodyHandle(11),
                coordinates,
                reason: RuntimeFrictionFaultSummary::Promotion(
                    FrictionPromotionError::MissingThermalState
                ),
            }
        );
    }

    #[test]
    fn fatal_rollback_summary_retains_terminalization_class_and_rollback_reason() {
        let coordinates = FrictionSolverCoordinates::new(1, 2, 3);
        let error = FrictionStepError::Authority(FrictionAuthorityFailure {
            body_a: BodyHandle(3),
            body_b: BodyHandle(5),
            coordinates,
            error: RuntimeFrictionError::Rollback {
                terminalization: Box::new(RuntimeFrictionError::Promotion(
                    FrictionPromotionError::InvalidHeatPartition,
                )),
                rollback: FrictionApplicationRollbackError::ObservationStateMismatch,
            },
        });

        assert_eq!(
            summarize_physics_authority_fault(&error),
            ThermodynamicPhysicsAuthorityFault::FrictionAuthority {
                body_a: BodyHandle(3),
                body_b: BodyHandle(5),
                coordinates,
                reason: RuntimeFrictionFaultSummary::RollbackInvariant {
                    terminalization: RuntimeFrictionTerminalizationSummary::Promotion(
                        FrictionPromotionError::InvalidHeatPartition,
                    ),
                    rollback: FrictionApplicationRollbackError::ObservationStateMismatch,
                },
            }
        );
    }
}
