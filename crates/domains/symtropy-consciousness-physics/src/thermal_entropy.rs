// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Entropy changes for explicitly declared elementary thermal processes.
//!
//! This module intentionally implements only two well-defined cases:
//! 1. an equilibrium body with constant heat capacity and positive endpoint
//!    temperatures; and
//! 2. a bath maintained at a fixed positive temperature.
//!
//! For a constant heat-capacity body:
//!
//! `ΔS_body = C ln(T_final / T_initial)`
//!
//! For a fixed-temperature bath, with heat positive into the body:
//!
//! `ΔS_bath = -Q_into_body / T_bath`
//!
//! These expressions do not cover phase transitions, temperature-dependent
//! heat capacity, non-equilibrium internal states, or arbitrary entropy
//! production. Energy balance must be checked separately.

/// Invalid input or result in an elementary thermal entropy calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalEntropyError {
    NonFiniteParameter { field: &'static str },
    NonPositiveParameter { field: &'static str },
    NegativeTolerance,
    NonFiniteResult { field: &'static str },
}

/// Classification of the total entropy change for a declared system plus bath.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondLawStatus {
    /// Total entropy change is non-negative within the supplied numerical tolerance.
    Satisfied,
    /// Total entropy change is negative beyond the supplied numerical tolerance.
    Violated,
}

/// Result of combining a body and fixed-temperature bath entropy change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalEntropyBalance {
    /// Entropy change of the constant-heat-capacity body, J/K.
    pub body_entropy_change_j_per_k: f64,
    /// Entropy change of the fixed-temperature bath, J/K.
    pub bath_entropy_change_j_per_k: f64,
    /// Sum of body and bath entropy changes, J/K.
    pub total_entropy_change_j_per_k: f64,
    pub status: SecondLawStatus,
}

/// Entropy change of an equilibrium body with constant heat capacity.
///
/// Assumptions: constant positive heat capacity, positive equilibrium endpoint
/// temperatures, and no phase transition across the interval.
pub fn constant_heat_capacity_entropy_change(
    heat_capacity_j_per_k: f64,
    initial_temperature_k: f64,
    final_temperature_k: f64,
) -> Result<f64, ThermalEntropyError> {
    validate_positive_finite(heat_capacity_j_per_k, "heat_capacity_j_per_k")?;
    validate_positive_finite(initial_temperature_k, "initial_temperature_k")?;
    validate_positive_finite(final_temperature_k, "final_temperature_k")?;

    // log(T1) - log(T0) avoids overflow/underflow in the intermediate T1/T0.
    let delta_s = heat_capacity_j_per_k
        * (final_temperature_k.ln() - initial_temperature_k.ln());
    if !delta_s.is_finite() {
        return Err(ThermalEntropyError::NonFiniteResult {
            field: "body_entropy_change_j_per_k",
        });
    }
    Ok(delta_s)
}

/// Entropy change of a fixed-temperature bath.
///
/// `heat_into_body_j` is positive when heat leaves the bath and enters the
/// body. The bath entropy change therefore has the opposite sign.
pub fn fixed_temperature_bath_entropy_change(
    heat_into_body_j: f64,
    bath_temperature_k: f64,
) -> Result<f64, ThermalEntropyError> {
    if !heat_into_body_j.is_finite() {
        return Err(ThermalEntropyError::NonFiniteParameter {
            field: "heat_into_body_j",
        });
    }
    validate_positive_finite(bath_temperature_k, "bath_temperature_k")?;

    let delta_s = -heat_into_body_j / bath_temperature_k;
    if !delta_s.is_finite() {
        return Err(ThermalEntropyError::NonFiniteResult {
            field: "bath_entropy_change_j_per_k",
        });
    }
    Ok(delta_s)
}

/// Combine body and fixed-temperature bath entropy changes and classify the
/// second-law inequality `ΔS_total >= 0` within an explicit tolerance.
///
/// This checks only the entropy terms supplied by the caller. It does not
/// establish that the selected body/bath model captures all reservoirs or
/// that the corresponding energy balance closes.
pub fn evaluate_constant_capacity_heat_exchange(
    heat_capacity_j_per_k: f64,
    initial_body_temperature_k: f64,
    final_body_temperature_k: f64,
    heat_into_body_j: f64,
    bath_temperature_k: f64,
    absolute_tolerance_j_per_k: f64,
) -> Result<ThermalEntropyBalance, ThermalEntropyError> {
    if !absolute_tolerance_j_per_k.is_finite() {
        return Err(ThermalEntropyError::NonFiniteParameter {
            field: "absolute_tolerance_j_per_k",
        });
    }
    if absolute_tolerance_j_per_k < 0.0 {
        return Err(ThermalEntropyError::NegativeTolerance);
    }

    let body_entropy_change_j_per_k = constant_heat_capacity_entropy_change(
        heat_capacity_j_per_k,
        initial_body_temperature_k,
        final_body_temperature_k,
    )?;
    let bath_entropy_change_j_per_k =
        fixed_temperature_bath_entropy_change(heat_into_body_j, bath_temperature_k)?;
    let total_entropy_change_j_per_k =
        body_entropy_change_j_per_k + bath_entropy_change_j_per_k;
    if !total_entropy_change_j_per_k.is_finite() {
        return Err(ThermalEntropyError::NonFiniteResult {
            field: "total_entropy_change_j_per_k",
        });
    }

    Ok(ThermalEntropyBalance {
        body_entropy_change_j_per_k,
        bath_entropy_change_j_per_k,
        total_entropy_change_j_per_k,
        status: if total_entropy_change_j_per_k >= -absolute_tolerance_j_per_k {
            SecondLawStatus::Satisfied
        } else {
            SecondLawStatus::Violated
        },
    })
}

fn validate_positive_finite(
    value: f64,
    field: &'static str,
) -> Result<(), ThermalEntropyError> {
    if !value.is_finite() {
        return Err(ThermalEntropyError::NonFiniteParameter { field });
    }
    if value <= 0.0 {
        return Err(ThermalEntropyError::NonPositiveParameter { field });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_capacity_body_entropy_increases_when_heated() {
        let delta_s =
            constant_heat_capacity_entropy_change(100.0, 300.0, 310.0).unwrap();
        assert!(delta_s > 0.0);
        assert!((delta_s - 100.0 * (310.0_f64.ln() - 300.0_f64.ln())).abs() < 1e-12);
    }

    #[test]
    fn constant_capacity_body_entropy_decreases_when_cooled() {
        let delta_s =
            constant_heat_capacity_entropy_change(100.0, 310.0, 300.0).unwrap();
        assert!(delta_s < 0.0);
    }

    #[test]
    fn fixed_temperature_bath_has_opposite_heat_sign() {
        assert_eq!(
            fixed_temperature_bath_entropy_change(100.0, 300.0).unwrap(),
            -100.0 / 300.0
        );
        assert_eq!(
            fixed_temperature_bath_entropy_change(-100.0, 300.0).unwrap(),
            100.0 / 300.0
        );
    }

    #[test]
    fn hot_bath_heating_cooler_body_satisfies_second_law() {
        let report =
            evaluate_constant_capacity_heat_exchange(100.0, 300.0, 301.0, 100.0, 400.0, 1e-12)
                .unwrap();
        assert_eq!(report.status, SecondLawStatus::Satisfied);
        assert!(report.total_entropy_change_j_per_k > 0.0);
    }

    #[test]
    fn cold_bath_heating_body_is_a_negative_control() {
        let report =
            evaluate_constant_capacity_heat_exchange(100.0, 300.0, 301.0, 100.0, 200.0, 1e-12)
                .unwrap();
        assert_eq!(report.status, SecondLawStatus::Violated);
        assert!(report.total_entropy_change_j_per_k < 0.0);
    }

    #[test]
    fn zero_heat_is_valid_and_has_zero_bath_entropy_change() {
        assert_eq!(fixed_temperature_bath_entropy_change(0.0, 300.0), Ok(0.0));
    }

    #[test]
    fn invalid_body_state_is_rejected() {
        assert_eq!(
            constant_heat_capacity_entropy_change(0.0, 300.0, 310.0),
            Err(ThermalEntropyError::NonPositiveParameter {
                field: "heat_capacity_j_per_k",
            })
        );
        assert_eq!(
            constant_heat_capacity_entropy_change(100.0, 0.0, 310.0),
            Err(ThermalEntropyError::NonPositiveParameter {
                field: "initial_temperature_k",
            })
        );
        assert_eq!(
            constant_heat_capacity_entropy_change(100.0, 300.0, f64::NAN),
            Err(ThermalEntropyError::NonFiniteParameter {
                field: "final_temperature_k",
            })
        );
    }

    #[test]
    fn invalid_bath_inputs_are_rejected() {
        assert_eq!(
            fixed_temperature_bath_entropy_change(1.0, 0.0),
            Err(ThermalEntropyError::NonPositiveParameter {
                field: "bath_temperature_k",
            })
        );
        assert_eq!(
            fixed_temperature_bath_entropy_change(f64::INFINITY, 300.0),
            Err(ThermalEntropyError::NonFiniteParameter {
                field: "heat_into_body_j",
            })
        );
    }

    #[test]
    fn invalid_tolerances_are_rejected() {
        assert_eq!(
            evaluate_constant_capacity_heat_exchange(100.0, 300.0, 301.0, 100.0, 400.0, -1.0),
            Err(ThermalEntropyError::NegativeTolerance)
        );
        assert_eq!(
            evaluate_constant_capacity_heat_exchange(
                100.0,
                300.0,
                301.0,
                100.0,
                400.0,
                f64::NAN,
            ),
            Err(ThermalEntropyError::NonFiniteParameter {
                field: "absolute_tolerance_j_per_k",
            })
        );
    }
}
