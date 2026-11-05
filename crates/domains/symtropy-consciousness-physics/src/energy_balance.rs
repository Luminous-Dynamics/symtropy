// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Explicit first-law accounting for a declared open-system boundary.
//!
//! This module is deliberately separate from the legacy telemetry counters in
//! `thermodynamics::ThermodynamicLedger`. It evaluates whether a declared set
//! of stored-energy changes and boundary flows closes arithmetically; it does
//! not infer missing flows or prove that the caller's boundary declaration is
//! physically complete.
//!
//! The sign convention is:
//!
//! ```text
//! ΔE_stored = W_in - W_out + Q_in - Q_out
//!           + E_matter,in - E_matter,out
//! residual  = ΔE_stored - net_boundary_energy
//! ```
//!
//! Work, heat, and matter-carried energy are non-negative magnitudes recorded
//! in the direction indicated by their field names. Matter-carried energy
//! includes the energy transported with mass across the boundary (for example
//! enthalpy, kinetic, and potential contributions under the chosen model).
//! Do not count the same transfer in more than one channel. Dissipation that
//! remains inside the selected system boundary is an internal conversion, not
//! an additional boundary outflow; heat leaving the boundary is recorded as
//! `heat_out_j`. Transfers between subsystems inside the declared boundary
//! cancel in the aggregate balance. A transfer crossing the boundary must be
//! recorded as an appropriate boundary flow.
//!
//! A `Balanced` result means only that the supplied terms close within the
//! caller-selected tolerance and the caller marked the boundary complete. It
//! is not independent proof that all physical stores, ports, or external
//! exchanges have been identified.

/// Unit-bearing energy value in joules.
///
/// This newtype prevents raw scalars and other unit wrappers (for example an
/// impulse magnitude) from being passed directly to energy-balance fields.
/// Finiteness and flow-direction validation occurs when a balance is evaluated.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Joules(f64);

impl Joules {
    /// Construct an energy value. The evaluator validates finiteness.
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    /// Return the represented value in joules.
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Identity of a scalar field in an energy-balance input or report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBalanceField {
    InitialStoredEnergy,
    FinalStoredEnergy,
    WorkIn,
    WorkOut,
    HeatIn,
    HeatOut,
    MatterEnergyIn,
    MatterEnergyOut,
    AbsoluteTolerance,
    RelativeTolerance,
    DeltaStoredEnergy,
    BoundaryEnergyIn,
    BoundaryEnergyOut,
    NetBoundaryEnergy,
    BoundaryEnergyScale,
    Residual,
    AllowedResidual,
    NormalizedResidual,
}

/// Errors produced when the declared energy-balance arithmetic is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBalanceError {
    /// A scalar input was NaN or infinite.
    NonFiniteInput { field: EnergyBalanceField },
    /// A boundary-flow magnitude or tolerance was negative.
    NegativeMagnitude { field: EnergyBalanceField },
    /// A finite input combination overflowed during balance arithmetic.
    ArithmeticOverflow { field: EnergyBalanceField },
}

/// Coverage assertion supplied by the caller for the selected system boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryCoverage {
    /// Caller asserts that every energy store and boundary exchange is included.
    Complete,
    /// One or more stores or boundary exchanges are omitted or unresolved.
    Incomplete,
}

/// Non-negative energy magnitudes crossing a declared system boundary.
///
/// Internal transfers between subsystems inside the declared boundary must not
/// be counted here. Matter-carried energy must not duplicate work or heat.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BoundaryEnergyFlows {
    /// Mechanical, electrical, or other work entering the system, in joules.
    pub work_in_j: Joules,
    /// Work leaving the system, in joules.
    pub work_out_j: Joules,
    /// Heat entering the system, in joules.
    pub heat_in_j: Joules,
    /// Heat leaving the system, in joules.
    pub heat_out_j: Joules,
    /// Energy carried into the system by matter, in joules.
    pub matter_energy_in_j: Joules,
    /// Energy carried out of the system by matter, in joules.
    pub matter_energy_out_j: Joules,
}

/// Inputs for a single first-law balance over one declared boundary and interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyBalanceInput {
    /// Total stored energy at the start of the interval, in joules.
    ///
    /// This may be signed because potential-energy references are conventional;
    /// it must be finite.
    pub initial_stored_energy_j: Joules,
    /// Total stored energy at the end of the interval, in joules.
    pub final_stored_energy_j: Joules,
    /// All energy flows crossing the selected system boundary.
    pub boundary_flows: BoundaryEnergyFlows,
    /// Explicit caller assertion about whether the boundary accounting is complete.
    pub boundary_coverage: BoundaryCoverage,
}

/// Caller-selected absolute and relative residual tolerances.
///
/// The acceptable residual is
/// `absolute_j + relative * boundary_energy_scale_j`. Tolerances must be
/// finite and non-negative; no universal tolerance is imposed by this module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyBalanceTolerances {
    /// Absolute residual tolerance, in joules.
    pub absolute_j: Joules,
    /// Dimensionless relative tolerance.
    pub relative: f64,
}

impl EnergyBalanceTolerances {
    /// Construct tolerances. Validation occurs when the input is evaluated.
    pub const fn new(absolute_j: Joules, relative: f64) -> Self {
        Self {
            absolute_j,
            relative,
        }
    }
}

/// Arithmetic classification of a supplied energy balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBalanceStatus {
    /// Complete declared boundary and residual within the selected tolerance.
    Balanced,
    /// Complete declared boundary but residual outside the selected tolerance.
    Imbalanced,
    /// Boundary coverage is incomplete; a balance PASS is not justified.
    Inconclusive,
}

/// First-law residual and supporting terms for one evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyBalanceReport {
    /// Change in stored energy, final minus initial, in joules.
    pub delta_stored_energy_j: Joules,
    /// Net energy crossing the boundary inward, in joules.
    pub net_boundary_energy_j: Joules,
    /// Scale used for normalization: max(|ΔE_stored|, total absolute boundary flow).
    pub boundary_energy_scale_j: Joules,
    /// Signed residual: ΔE_stored - net boundary energy, in joules.
    pub residual_j: Joules,
    /// Absolute residual divided by the normalization scale (dimensionless).
    /// Equals zero when both the residual and scale are zero.
    pub normalized_residual: f64,
    /// Absolute tolerance plus relative tolerance times the normalization scale.
    pub allowed_residual_j: Joules,
    /// Result classification; incomplete boundary coverage is always inconclusive.
    pub status: EnergyBalanceStatus,
}

impl EnergyBalanceInput {
    /// Evaluate the first-law balance with explicit caller-selected tolerances.
    ///
    /// Negative boundary-flow magnitudes, invalid tolerances, non-finite values,
    /// and arithmetic overflow return typed errors. An incomplete boundary
    /// returns a report with `Inconclusive` status even when the entered terms
    /// happen to balance exactly.
    pub fn evaluate(
        &self,
        tolerances: EnergyBalanceTolerances,
    ) -> Result<EnergyBalanceReport, EnergyBalanceError> {
        validate_finite(
            self.initial_stored_energy_j.get(),
            EnergyBalanceField::InitialStoredEnergy,
        )?;
        validate_finite(
            self.final_stored_energy_j.get(),
            EnergyBalanceField::FinalStoredEnergy,
        )?;
        validate_tolerance(
            tolerances.absolute_j.get(),
            EnergyBalanceField::AbsoluteTolerance,
        )?;
        validate_tolerance(tolerances.relative, EnergyBalanceField::RelativeTolerance)?;

        let flows = self.boundary_flows;
        validate_flow(flows.work_in_j.get(), EnergyBalanceField::WorkIn)?;
        validate_flow(flows.work_out_j.get(), EnergyBalanceField::WorkOut)?;
        validate_flow(flows.heat_in_j.get(), EnergyBalanceField::HeatIn)?;
        validate_flow(flows.heat_out_j.get(), EnergyBalanceField::HeatOut)?;
        validate_flow(
            flows.matter_energy_in_j.get(),
            EnergyBalanceField::MatterEnergyIn,
        )?;
        validate_flow(
            flows.matter_energy_out_j.get(),
            EnergyBalanceField::MatterEnergyOut,
        )?;

        let delta_stored_energy_j = checked_sub(
            self.final_stored_energy_j.get(),
            self.initial_stored_energy_j.get(),
            EnergyBalanceField::DeltaStoredEnergy,
        )?;

        let work_heat_in = checked_add(
            flows.work_in_j.get(),
            flows.heat_in_j.get(),
            EnergyBalanceField::BoundaryEnergyIn,
        )?;
        let energy_in = checked_add(
            work_heat_in,
            flows.matter_energy_in_j.get(),
            EnergyBalanceField::BoundaryEnergyIn,
        )?;
        let work_heat_out = checked_add(
            flows.work_out_j.get(),
            flows.heat_out_j.get(),
            EnergyBalanceField::BoundaryEnergyOut,
        )?;
        let energy_out = checked_add(
            work_heat_out,
            flows.matter_energy_out_j.get(),
            EnergyBalanceField::BoundaryEnergyOut,
        )?;

        let net_boundary_energy_j =
            checked_sub(energy_in, energy_out, EnergyBalanceField::NetBoundaryEnergy)?;
        let total_boundary_flow = checked_add(
            energy_in,
            energy_out,
            EnergyBalanceField::BoundaryEnergyScale,
        )?;
        let boundary_energy_scale_j = delta_stored_energy_j.abs().max(total_boundary_flow);

        let residual_j = checked_sub(
            delta_stored_energy_j,
            net_boundary_energy_j,
            EnergyBalanceField::Residual,
        )?;
        let relative_component = checked_mul(
            tolerances.relative,
            boundary_energy_scale_j,
            EnergyBalanceField::AllowedResidual,
        )?;
        let allowed_residual_j = checked_add(
            tolerances.absolute_j.get(),
            relative_component,
            EnergyBalanceField::AllowedResidual,
        )?;
        let normalized_residual = if boundary_energy_scale_j == 0.0 {
            0.0
        } else {
            residual_j.abs() / boundary_energy_scale_j
        };
        if !normalized_residual.is_finite() {
            return Err(EnergyBalanceError::ArithmeticOverflow {
                field: EnergyBalanceField::NormalizedResidual,
            });
        }

        let status = match self.boundary_coverage {
            BoundaryCoverage::Incomplete => EnergyBalanceStatus::Inconclusive,
            BoundaryCoverage::Complete if residual_j.abs() <= allowed_residual_j => {
                EnergyBalanceStatus::Balanced
            }
            BoundaryCoverage::Complete => EnergyBalanceStatus::Imbalanced,
        };

        Ok(EnergyBalanceReport {
            delta_stored_energy_j: Joules::new(delta_stored_energy_j),
            net_boundary_energy_j: Joules::new(net_boundary_energy_j),
            boundary_energy_scale_j: Joules::new(boundary_energy_scale_j),
            residual_j: Joules::new(residual_j),
            normalized_residual,
            allowed_residual_j: Joules::new(allowed_residual_j),
            status,
        })
    }
}

fn validate_finite(value: f64, field: EnergyBalanceField) -> Result<(), EnergyBalanceError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(EnergyBalanceError::NonFiniteInput { field })
    }
}

fn validate_flow(value: f64, field: EnergyBalanceField) -> Result<(), EnergyBalanceError> {
    validate_finite(value, field)?;
    if value < 0.0 {
        return Err(EnergyBalanceError::NegativeMagnitude { field });
    }
    Ok(())
}

fn validate_tolerance(value: f64, field: EnergyBalanceField) -> Result<(), EnergyBalanceError> {
    validate_finite(value, field)?;
    if value < 0.0 {
        return Err(EnergyBalanceError::NegativeMagnitude { field });
    }
    Ok(())
}

fn checked_add(
    left: f64,
    right: f64,
    field: EnergyBalanceField,
) -> Result<f64, EnergyBalanceError> {
    let value = left + right;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EnergyBalanceError::ArithmeticOverflow { field })
    }
}

fn checked_sub(
    left: f64,
    right: f64,
    field: EnergyBalanceField,
) -> Result<f64, EnergyBalanceError> {
    let value = left - right;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EnergyBalanceError::ArithmeticOverflow { field })
    }
}

fn checked_mul(
    left: f64,
    right: f64,
    field: EnergyBalanceField,
) -> Result<f64, EnergyBalanceError> {
    let value = left * right;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EnergyBalanceError::ArithmeticOverflow { field })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerances() -> EnergyBalanceTolerances {
        EnergyBalanceTolerances::new(Joules::new(1e-12), 1e-9)
    }

    fn complete_input(
        initial_stored_energy_j: f64,
        final_stored_energy_j: f64,
        boundary_flows: BoundaryEnergyFlows,
    ) -> EnergyBalanceInput {
        EnergyBalanceInput {
            initial_stored_energy_j: Joules::new(initial_stored_energy_j),
            final_stored_energy_j: Joules::new(final_stored_energy_j),
            boundary_flows,
            boundary_coverage: BoundaryCoverage::Complete,
        }
    }

    #[test]
    fn isolated_closed_system_balances() {
        let input = complete_input(25.0, 25.0, BoundaryEnergyFlows::default());
        let report = input.evaluate(tolerances()).unwrap();

        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
        assert_eq!(report.delta_stored_energy_j, Joules::new(0.0));
        assert_eq!(report.net_boundary_energy_j, Joules::new(0.0));
        assert_eq!(report.normalized_residual, 0.0);
    }

    #[test]
    fn work_and_heat_inputs_match_stored_energy_increase() {
        let input = complete_input(
            10.0,
            25.0,
            BoundaryEnergyFlows {
                work_in_j: Joules::new(9.0),
                heat_in_j: Joules::new(6.0),
                ..BoundaryEnergyFlows::default()
            },
        );
        let report = input.evaluate(tolerances()).unwrap();

        assert_eq!(report.delta_stored_energy_j, Joules::new(15.0));
        assert_eq!(report.net_boundary_energy_j, Joules::new(15.0));
        assert_eq!(report.residual_j, Joules::new(0.0));
        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
    }

    #[test]
    fn boundary_outflows_match_stored_energy_decrease() {
        let input = complete_input(
            25.0,
            16.0,
            BoundaryEnergyFlows {
                work_out_j: Joules::new(4.0),
                heat_out_j: Joules::new(3.0),
                matter_energy_out_j: Joules::new(2.0),
                ..BoundaryEnergyFlows::default()
            },
        );
        let report = input.evaluate(tolerances()).unwrap();

        assert_eq!(report.delta_stored_energy_j, Joules::new(-9.0));
        assert_eq!(report.net_boundary_energy_j, Joules::new(-9.0));
        assert_eq!(report.residual_j, Joules::new(0.0));
        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
    }

    #[test]
    fn zero_input_with_positive_output_is_not_a_pass() {
        let input = complete_input(
            12.0,
            12.0,
            BoundaryEnergyFlows {
                heat_out_j: Joules::new(8.0),
                ..BoundaryEnergyFlows::default()
            },
        );
        let report = input.evaluate(tolerances()).unwrap();

        assert_eq!(report.status, EnergyBalanceStatus::Imbalanced);
        assert_eq!(report.residual_j, Joules::new(8.0));
        assert_eq!(report.normalized_residual, 1.0);
    }

    #[test]
    fn incomplete_boundary_is_inconclusive_even_when_terms_balance() {
        let input = EnergyBalanceInput {
            initial_stored_energy_j: Joules::new(4.0),
            final_stored_energy_j: Joules::new(9.0),
            boundary_flows: BoundaryEnergyFlows {
                work_in_j: Joules::new(5.0),
                ..BoundaryEnergyFlows::default()
            },
            boundary_coverage: BoundaryCoverage::Incomplete,
        };
        let report = input.evaluate(tolerances()).unwrap();

        assert_eq!(report.residual_j, Joules::new(0.0));
        assert_eq!(report.status, EnergyBalanceStatus::Inconclusive);
    }

    #[test]
    fn matter_energy_crossing_boundary_is_accounted_once() {
        let input = complete_input(
            20.0,
            31.0,
            BoundaryEnergyFlows {
                matter_energy_in_j: Joules::new(12.0),
                work_out_j: Joules::new(1.0),
                ..BoundaryEnergyFlows::default()
            },
        );
        let report = input.evaluate(tolerances()).unwrap();

        assert_eq!(report.net_boundary_energy_j, Joules::new(11.0));
        assert_eq!(report.residual_j, Joules::new(0.0));
        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
    }

    #[test]
    fn negative_or_non_finite_flows_are_rejected() {
        let negative = complete_input(
            0.0,
            0.0,
            BoundaryEnergyFlows {
                heat_in_j: Joules::new(-1.0),
                ..BoundaryEnergyFlows::default()
            },
        );
        assert_eq!(
            negative.evaluate(tolerances()),
            Err(EnergyBalanceError::NegativeMagnitude {
                field: EnergyBalanceField::HeatIn,
            })
        );

        let non_finite = complete_input(
            0.0,
            0.0,
            BoundaryEnergyFlows {
                work_out_j: Joules::new(f64::NAN),
                ..BoundaryEnergyFlows::default()
            },
        );
        assert_eq!(
            non_finite.evaluate(tolerances()),
            Err(EnergyBalanceError::NonFiniteInput {
                field: EnergyBalanceField::WorkOut,
            })
        );
    }

    #[test]
    fn invalid_tolerances_and_stored_energy_are_rejected() {
        let input = complete_input(0.0, 0.0, BoundaryEnergyFlows::default());
        assert_eq!(
            input.evaluate(EnergyBalanceTolerances::new(Joules::new(-1.0), 0.0)),
            Err(EnergyBalanceError::NegativeMagnitude {
                field: EnergyBalanceField::AbsoluteTolerance,
            })
        );
        assert_eq!(
            input.evaluate(EnergyBalanceTolerances::new(
                Joules::new(0.0),
                f64::INFINITY
            )),
            Err(EnergyBalanceError::NonFiniteInput {
                field: EnergyBalanceField::RelativeTolerance,
            })
        );

        let non_finite_state = complete_input(f64::NAN, 0.0, BoundaryEnergyFlows::default());
        assert_eq!(
            non_finite_state.evaluate(tolerances()),
            Err(EnergyBalanceError::NonFiniteInput {
                field: EnergyBalanceField::InitialStoredEnergy,
            })
        );
    }

    #[test]
    fn arithmetic_overflow_is_rejected() {
        let input = complete_input(
            0.0,
            0.0,
            BoundaryEnergyFlows {
                work_in_j: Joules::new(f64::MAX),
                heat_in_j: Joules::new(f64::MAX),
                ..BoundaryEnergyFlows::default()
            },
        );
        assert_eq!(
            input.evaluate(tolerances()),
            Err(EnergyBalanceError::ArithmeticOverflow {
                field: EnergyBalanceField::BoundaryEnergyIn,
            })
        );
    }

    #[test]
    fn tolerance_is_applied_to_scale_aware_residual() {
        let input = complete_input(
            0.0,
            1000.5,
            BoundaryEnergyFlows {
                work_in_j: Joules::new(1000.4995),
                ..BoundaryEnergyFlows::default()
            },
        );
        let strict = input
            .evaluate(EnergyBalanceTolerances::new(Joules::new(0.0), 1e-7))
            .unwrap();
        let loose = input
            .evaluate(EnergyBalanceTolerances::new(Joules::new(0.0), 1e-6))
            .unwrap();

        assert_eq!(strict.status, EnergyBalanceStatus::Imbalanced);
        assert_eq!(loose.status, EnergyBalanceStatus::Balanced);
        assert!((strict.normalized_residual - 0.0005 / 1000.5).abs() < 1e-15);
    }
}
