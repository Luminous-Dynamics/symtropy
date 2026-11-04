// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Explicit energy-balance contracts for declared open-system boundaries.
//!
//! This module is deliberately separate from the legacy per-tick
//! `ThermodynamicLedger`. It evaluates the first-law balance only when the
//! caller supplies every required term for one declared system boundary.
//!
//! The sign convention is:
//! - work and heat entering the boundary are positive inputs;
//! - work and heat leaving the boundary are positive outputs;
//! - stored-energy change is final minus initial.
//!
//! Internal transfers between subsystems wholly inside the declared boundary
//! cancel in the total-system balance. They must not be counted again as
//! external boundary flows. If an energy carrier crosses the boundary, it
//! must be represented by the appropriate work or heat flow.
//!
//! This is an accounting validator, not a physical-model validator. It cannot
//! establish that the caller chose the correct boundary, measured every term,
//! or used a valid constitutive model.

/// Energy terms required to evaluate a complete first-law balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBalanceTerm {
    InitialStoredEnergy,
    FinalStoredEnergy,
    WorkIntoSystem,
    WorkOutOfSystem,
    HeatIntoSystem,
    HeatOutOfSystem,
}

impl EnergyBalanceTerm {
    /// Stable machine-readable field name for reports and receipts.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InitialStoredEnergy => "initial_stored_energy_j",
            Self::FinalStoredEnergy => "final_stored_energy_j",
            Self::WorkIntoSystem => "work_into_system_j",
            Self::WorkOutOfSystem => "work_out_of_system_j",
            Self::HeatIntoSystem => "heat_into_system_j",
            Self::HeatOutOfSystem => "heat_out_of_system_j",
        }
    }
}

/// Tolerance used to classify an energy-balance residual.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyBalanceTolerance {
    /// Absolute tolerance in joules.
    pub absolute_j: f64,
    /// Relative tolerance against the larger of the stored-energy change and
    /// gross boundary-flow magnitude.
    pub relative: f64,
}

impl EnergyBalanceTolerance {
    /// Construct a tolerance, rejecting negative or non-finite values.
    pub fn new(absolute_j: f64, relative: f64) -> Result<Self, EnergyBalanceError> {
        if !absolute_j.is_finite() || absolute_j < 0.0 {
            return Err(EnergyBalanceError::InvalidTolerance {
                field: "absolute_j",
            });
        }
        if !relative.is_finite() || relative < 0.0 {
            return Err(EnergyBalanceError::InvalidTolerance { field: "relative" });
        }
        Ok(Self {
            absolute_j,
            relative,
        })
    }

}

/// Complete or partial first-law data for one declared system boundary.
///
/// Every optional term is semantically meaningful: `None` means that the
/// quantity was not supplied or is unknown; `Some(0.0)` means it was
/// explicitly accounted for and measured/modelled as zero. Missing terms
/// therefore produce `Incomplete`, never an implicit zero.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenSystemEnergyBalance {
    /// Non-empty identifier for the exact system boundary being evaluated.
    pub boundary_id: String,
    /// Total stored energy inside the boundary at the start, in joules.
    pub initial_stored_energy_j: Option<f64>,
    /// Total stored energy inside the boundary at the end, in joules.
    pub final_stored_energy_j: Option<f64>,
    /// Work crossing into the boundary during the interval, in joules.
    pub work_into_system_j: Option<f64>,
    /// Work crossing out of the boundary during the interval, in joules.
    pub work_out_of_system_j: Option<f64>,
    /// Heat crossing into the boundary during the interval, in joules.
    pub heat_into_system_j: Option<f64>,
    /// Heat crossing out of the boundary during the interval, in joules.
    pub heat_out_of_system_j: Option<f64>,
}

/// Classification of a balance record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnergyBalanceStatus {
    /// One or more required terms were not supplied. No residual is reported.
    Incomplete {
        missing_terms: Vec<EnergyBalanceTerm>,
    },
    /// Every term was supplied and the residual met the declared tolerance.
    Balanced,
    /// Every term was supplied but the residual exceeded the declared tolerance.
    Imbalanced,
}

/// Result of evaluating the first-law balance for one declared boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyBalanceReport {
    pub boundary_id: String,
    pub status: EnergyBalanceStatus,
    /// Final stored energy minus initial stored energy, in joules.
    pub delta_stored_energy_j: Option<f64>,
    /// Net boundary energy input: work-in + heat-in - work-out - heat-out.
    pub net_boundary_energy_j: Option<f64>,
    /// Residual: ΔE_stored - net_boundary_energy.
    pub residual_j: Option<f64>,
    /// Absolute acceptance threshold used for classification, in joules.
    pub allowed_residual_j: Option<f64>,
    /// Scale-normalized absolute residual. Zero when every energy scale is zero.
    pub normalized_residual: Option<f64>,
}

/// Invalid input supplied to the open-system balance evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnergyBalanceError {
    EmptyBoundaryId,
    InvalidEnergyValue {
        field: EnergyBalanceTerm,
    },
    InvalidTolerance {
        field: &'static str,
    },
}

impl OpenSystemEnergyBalance {
    /// Evaluate a complete or partial first-law balance.
    ///
    /// Complete balances use:
    ///
    /// `ΔE_stored = W_into - W_out + Q_into - Q_out`
    ///
    /// and residual:
    ///
    /// `r = ΔE_stored - (W_into - W_out + Q_into - Q_out)`.
    ///
    /// Stored-energy values may be signed because potential-energy reference
    /// conventions vary. Boundary-flow magnitudes and tolerances must be
    /// finite and non-negative. Missing values are not interpreted as zero.
    pub fn evaluate(
        &self,
        tolerance: EnergyBalanceTolerance,
    ) -> Result<EnergyBalanceReport, EnergyBalanceError> {
        if self.boundary_id.trim().is_empty() {
            return Err(EnergyBalanceError::EmptyBoundaryId);
        }
        validate_tolerance(tolerance)?;

        let required = [
            (EnergyBalanceTerm::InitialStoredEnergy, self.initial_stored_energy_j),
            (EnergyBalanceTerm::FinalStoredEnergy, self.final_stored_energy_j),
            (EnergyBalanceTerm::WorkIntoSystem, self.work_into_system_j),
            (EnergyBalanceTerm::WorkOutOfSystem, self.work_out_of_system_j),
            (EnergyBalanceTerm::HeatIntoSystem, self.heat_into_system_j),
            (EnergyBalanceTerm::HeatOutOfSystem, self.heat_out_of_system_j),
        ];

        // Validate every supplied value even when other terms are missing.
        // An incomplete record must not hide malformed evidence.
        for (term, value) in required {
            if let Some(value) = value {
                if !value.is_finite() {
                    return Err(EnergyBalanceError::InvalidEnergyValue { field: term });
                }
                if term != EnergyBalanceTerm::InitialStoredEnergy
                    && term != EnergyBalanceTerm::FinalStoredEnergy
                    && value < 0.0
                {
                    return Err(EnergyBalanceError::InvalidEnergyValue { field: term });
                }
            }
        }

        let missing_terms: Vec<_> = required
            .iter()
            .filter_map(|(term, value)| value.is_none().then_some(*term))
            .collect();

        if !missing_terms.is_empty() {
            return Ok(EnergyBalanceReport {
                boundary_id: self.boundary_id.clone(),
                status: EnergyBalanceStatus::Incomplete { missing_terms },
                delta_stored_energy_j: None,
                net_boundary_energy_j: None,
                residual_j: None,
                allowed_residual_j: None,
                normalized_residual: None,
            });
        }

        let initial = self.initial_stored_energy_j.expect("validated above");
        let final_energy = self.final_stored_energy_j.expect("validated above");
        let work_in = self.work_into_system_j.expect("validated above");
        let work_out = self.work_out_of_system_j.expect("validated above");
        let heat_in = self.heat_into_system_j.expect("validated above");
        let heat_out = self.heat_out_of_system_j.expect("validated above");

        let delta_stored = final_energy - initial;
        let gross_boundary_flow = work_in + work_out + heat_in + heat_out;
        let net_boundary = work_in + heat_in - work_out - heat_out;
        let residual = delta_stored - net_boundary;
        let scale = delta_stored.abs().max(gross_boundary_flow);
        let allowed = tolerance.absolute_j + tolerance.relative * scale;

        if !delta_stored.is_finite()
            || !gross_boundary_flow.is_finite()
            || !net_boundary.is_finite()
            || !residual.is_finite()
            || !allowed.is_finite()
        {
            return Err(EnergyBalanceError::InvalidEnergyValue {
                field: EnergyBalanceTerm::FinalStoredEnergy,
            });
        }

        let normalized_residual = if scale == 0.0 {
            0.0
        } else {
            residual.abs() / scale
        };

        Ok(EnergyBalanceReport {
            boundary_id: self.boundary_id.clone(),
            status: if residual.abs() <= allowed {
                EnergyBalanceStatus::Balanced
            } else {
                EnergyBalanceStatus::Imbalanced
            },
            delta_stored_energy_j: Some(delta_stored),
            net_boundary_energy_j: Some(net_boundary),
            residual_j: Some(residual),
            allowed_residual_j: Some(allowed),
            normalized_residual: Some(normalized_residual),
        })
    }
}

fn validate_tolerance(
    tolerance: EnergyBalanceTolerance,
) -> Result<(), EnergyBalanceError> {
    if !tolerance.absolute_j.is_finite() || tolerance.absolute_j < 0.0 {
        return Err(EnergyBalanceError::InvalidTolerance {
            field: "absolute_j",
        });
    }
    if !tolerance.relative.is_finite() || tolerance.relative < 0.0 {
        return Err(EnergyBalanceError::InvalidTolerance { field: "relative" });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_balance(
        initial: f64,
        final_energy: f64,
        work_in: f64,
        work_out: f64,
        heat_in: f64,
        heat_out: f64,
    ) -> OpenSystemEnergyBalance {
        OpenSystemEnergyBalance {
            boundary_id: "test-system".to_string(),
            initial_stored_energy_j: Some(initial),
            final_stored_energy_j: Some(final_energy),
            work_into_system_j: Some(work_in),
            work_out_of_system_j: Some(work_out),
            heat_into_system_j: Some(heat_in),
            heat_out_of_system_j: Some(heat_out),
        }
    }

    fn zero_tolerance() -> EnergyBalanceTolerance {
        EnergyBalanceTolerance {
            absolute_j: 0.0,
            relative: 0.0,
        }
    }

    #[test]
    fn closed_system_with_no_flows_is_balanced() {
        let report = complete_balance(12.0, 12.0, 0.0, 0.0, 0.0, 0.0)
            .evaluate(zero_tolerance())
            .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
        assert_eq!(report.residual_j, Some(0.0));
    }

    #[test]
    fn external_work_and_heat_close_the_balance() {
        let report = complete_balance(10.0, 23.0, 8.0, 1.0, 7.0, 1.0)
            .evaluate(zero_tolerance())
            .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
        assert_eq!(report.delta_stored_energy_j, Some(13.0));
        assert_eq!(report.net_boundary_energy_j, Some(13.0));
    }

    #[test]
    fn zero_inputs_and_positive_stored_energy_gain_fail_closed() {
        let report = complete_balance(0.0, 10.0, 0.0, 0.0, 0.0, 0.0)
            .evaluate(zero_tolerance())
            .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Imbalanced);
        assert_eq!(report.residual_j, Some(10.0));
    }

    #[test]
    fn stored_energy_loss_accounts_for_heat_outflow() {
        let report = complete_balance(10.0, 0.0, 0.0, 0.0, 0.0, 10.0)
            .evaluate(zero_tolerance())
            .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
        assert_eq!(report.residual_j, Some(0.0));
    }

    #[test]
    fn positive_outflow_without_stored_energy_or_input_is_imbalanced() {
        let report = complete_balance(0.0, 0.0, 0.0, 0.0, 0.0, 10.0)
            .evaluate(zero_tolerance())
            .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Imbalanced);
        assert_eq!(report.residual_j, Some(10.0));
    }

    #[test]
    fn invalid_supplied_values_are_not_hidden_by_missing_terms() {
        let mut record = complete_balance(0.0, 10.0, f64::NAN, 0.0, 0.0, 0.0);
        record.heat_out_of_system_j = None;
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::InvalidEnergyValue {
                field: EnergyBalanceTerm::WorkIntoSystem,
            })
        );
    }

    #[test]
    fn missing_terms_are_incomplete_not_implicit_zero() {
        let mut record = complete_balance(0.0, 10.0, 0.0, 0.0, 0.0, 0.0);
        record.heat_out_of_system_j = None;

        let report = record.evaluate(zero_tolerance()).unwrap();
        assert_eq!(
            report.status,
            EnergyBalanceStatus::Incomplete {
                missing_terms: vec![EnergyBalanceTerm::HeatOutOfSystem],
            }
        );
        assert_eq!(report.residual_j, None);
    }

    #[test]
    fn negative_boundary_flow_is_rejected() {
        let report = complete_balance(0.0, 0.0, -1.0, 0.0, 0.0, 0.0)
            .evaluate(zero_tolerance());
        assert_eq!(
            report,
            Err(EnergyBalanceError::InvalidEnergyValue {
                field: EnergyBalanceTerm::WorkIntoSystem,
            })
        );
    }

    #[test]
    fn non_finite_values_are_rejected() {
        let report = complete_balance(0.0, f64::NAN, 0.0, 0.0, 0.0, 0.0)
            .evaluate(zero_tolerance());
        assert_eq!(
            report,
            Err(EnergyBalanceError::InvalidEnergyValue {
                field: EnergyBalanceTerm::FinalStoredEnergy,
            })
        );
    }

    #[test]
    fn invalid_boundary_and_tolerances_are_rejected() {
        let mut record = complete_balance(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        record.boundary_id = "  ".to_string();
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::EmptyBoundaryId)
        );

        assert_eq!(
            EnergyBalanceTolerance::new(-1.0, 0.0),
            Err(EnergyBalanceError::InvalidTolerance {
                field: "absolute_j",
            })
        );
        assert_eq!(
            EnergyBalanceTolerance::new(0.0, f64::INFINITY),
            Err(EnergyBalanceError::InvalidTolerance { field: "relative" })
        );
    }

    #[test]
    fn relative_tolerance_scales_with_energy_magnitude() {
        let tolerance = EnergyBalanceTolerance::new(0.0, 1e-3).unwrap();
        let small = complete_balance(0.0, 100.0, 100.05, 0.0, 0.0, 0.0)
            .evaluate(tolerance)
            .unwrap();
        let large = complete_balance(0.0, 100_000.0, 100_050.0, 0.0, 0.0, 0.0)
            .evaluate(tolerance)
            .unwrap();

        assert_eq!(small.status, EnergyBalanceStatus::Balanced);
        assert_eq!(large.status, EnergyBalanceStatus::Balanced);
    }

    #[test]
    fn overflowed_balance_is_rejected() {
        let report = complete_balance(
            -f64::MAX,
            f64::MAX,
            f64::MAX,
            0.0,
            0.0,
            0.0,
        )
        .evaluate(zero_tolerance());
        assert!(matches!(
            report,
            Err(EnergyBalanceError::InvalidEnergyValue { .. })
        ));
    }
}
