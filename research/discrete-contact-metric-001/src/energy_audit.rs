// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Independent replay of the discrete energy/work ledger.
//!
//! This module reconstructs accounting from the frozen input and accepted
//! next-state/impulse data. It does not trust the published ledger, reported
//! contact slack, or reported next position. This is a separate f64 audit path,
//! not an interval-certified proof or a validation of physical model fit.

use super::{DiscreteEnergyLedger, DiscreteStepInput, DiscreteStepResult};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnergyAuditProfile {
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
}

impl Default for EnergyAuditProfile {
    fn default() -> Self {
        Self {
            absolute_tolerance: 1.0e-12,
            relative_tolerance: 1.0e-10,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EnergyAuditError {
    InvalidProfile,
    ShapeMismatch,
    NonFiniteDerivedValue,
    KinematicPositionMismatch { dof: usize },
    ContactSlackMismatch { contact: usize },
    LedgerFieldMismatch(&'static str),
}

impl fmt::Display for EnergyAuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for EnergyAuditError {}

/// Recomputed values. A successful receipt means every published field and
/// derived position/slack component matched this independent replay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnergyAuditReceipt {
    pub kinetic_energy_before: f64,
    pub kinetic_energy_after: f64,
    pub potential_energy_before: f64,
    pub potential_energy_after: f64,
    pub external_force_work: f64,
    pub damping_dissipation: f64,
    pub mass_increment_dissipation: f64,
    pub stiffness_increment_dissipation: f64,
    pub contact_impulse_work: f64,
    pub contact_target_work: f64,
    pub contact_complementarity_work: f64,
    pub energy_balance_residual: f64,
    pub relative_energy_balance_residual: f64,
    pub contact_work_decomposition_residual: f64,
    pub relative_contact_work_decomposition_residual: f64,
    pub maximum_absolute_field_difference: f64,
    pub maximum_scaled_field_difference: f64,
    pub checked_ledger_fields: usize,
    pub checked_position_components: usize,
    pub checked_contact_slack_components: usize,
}

fn validate_profile(profile: EnergyAuditProfile) -> Result<(), EnergyAuditError> {
    if !profile.absolute_tolerance.is_finite()
        || profile.absolute_tolerance < 0.0
        || !profile.relative_tolerance.is_finite()
        || profile.relative_tolerance < 0.0
    {
        return Err(EnergyAuditError::InvalidProfile);
    }
    Ok(())
}

fn check_finite(values: &[f64]) -> Result<(), EnergyAuditError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(EnergyAuditError::NonFiniteDerivedValue);
    }
    Ok(())
}

fn dot(left: &[f64], right: &[f64]) -> Result<f64, EnergyAuditError> {
    if left.len() != right.len() {
        return Err(EnergyAuditError::ShapeMismatch);
    }
    let value = left.iter().zip(right).map(|(a, b)| a * b).sum::<f64>();
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EnergyAuditError::NonFiniteDerivedValue)
    }
}

/// Direct quadratic form evaluation, independent of the solver helper.
fn quadratic_form(
    matrix: &super::Matrix,
    vector: &[f64],
) -> Result<f64, EnergyAuditError> {
    if matrix.rows != vector.len() || matrix.cols != vector.len() {
        return Err(EnergyAuditError::ShapeMismatch);
    }
    let mut value = 0.0_f64;
    for row in 0..vector.len() {
        for column in 0..vector.len() {
            value += vector[row] * matrix[(row, column)] * vector[column];
        }
    }
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EnergyAuditError::NonFiniteDerivedValue)
    }
}

fn close(
    expected: f64,
    observed: f64,
    profile: EnergyAuditProfile,
) -> Result<(f64, f64), EnergyAuditError> {
    if !expected.is_finite() || !observed.is_finite() {
        return Err(EnergyAuditError::NonFiniteDerivedValue);
    }
    let difference = (expected - observed).abs();
    let scale = expected.abs().max(observed.abs());
    let tolerance = profile.absolute_tolerance + profile.relative_tolerance * scale;
    if difference > tolerance {
        return Err(EnergyAuditError::LedgerFieldMismatch("numeric_field"));
    }
    Ok((
        difference,
        difference / scale.max(f64::MIN_POSITIVE),
    ))
}

fn compare_ledger(
    expected: &DiscreteEnergyLedger,
    observed: &DiscreteEnergyLedger,
    profile: EnergyAuditProfile,
) -> Result<(f64, f64, usize), EnergyAuditError> {
    let fields = [
        ("kinetic_energy_before", expected.kinetic_energy_before, observed.kinetic_energy_before),
        ("kinetic_energy_after", expected.kinetic_energy_after, observed.kinetic_energy_after),
        ("potential_energy_before", expected.potential_energy_before, observed.potential_energy_before),
        ("potential_energy_after", expected.potential_energy_after, observed.potential_energy_after),
        ("external_force_work", expected.external_force_work, observed.external_force_work),
        ("damping_dissipation", expected.damping_dissipation, observed.damping_dissipation),
        ("mass_increment_dissipation", expected.mass_increment_dissipation, observed.mass_increment_dissipation),
        ("stiffness_increment_dissipation", expected.stiffness_increment_dissipation, observed.stiffness_increment_dissipation),
        ("contact_impulse_work", expected.contact_impulse_work, observed.contact_impulse_work),
        ("contact_target_work", expected.contact_target_work, observed.contact_target_work),
        ("contact_complementarity_work", expected.contact_complementarity_work, observed.contact_complementarity_work),
        ("energy_balance_residual", expected.energy_balance_residual, observed.energy_balance_residual),
        ("relative_energy_balance_residual", expected.relative_energy_balance_residual, observed.relative_energy_balance_residual),
        ("contact_work_decomposition_residual", expected.contact_work_decomposition_residual, observed.contact_work_decomposition_residual),
        ("relative_contact_work_decomposition_residual", expected.relative_contact_work_decomposition_residual, observed.relative_contact_work_decomposition_residual),
    ];

    let mut maximum_absolute = 0.0_f64;
    let mut maximum_scaled = 0.0_f64;
    for (name, expected_value, observed_value) in fields {
        if !expected_value.is_finite() || !observed_value.is_finite() {
            return Err(EnergyAuditError::NonFiniteDerivedValue);
        }
        let difference = (expected_value - observed_value).abs();
        let scale = expected_value.abs().max(observed_value.abs());
        let tolerance = profile.absolute_tolerance + profile.relative_tolerance * scale;
        if difference > tolerance {
            return Err(EnergyAuditError::LedgerFieldMismatch(name));
        }
        maximum_absolute = maximum_absolute.max(difference);
        maximum_scaled = maximum_scaled.max(difference / scale.max(f64::MIN_POSITIVE));
    }
    Ok((maximum_absolute, maximum_scaled, fields.len()))
}

/// Independently reconstruct and compare every energy/work field.
///
/// Position is independently derived as q + h v_next. Contact slack is
/// independently derived as J v_next - (-gap/h). Neither the published
/// position nor published slack is trusted when forming the ledger.
pub fn audit_discrete_energy_ledger(
    input: &DiscreteStepInput,
    result: &DiscreteStepResult,
    profile: EnergyAuditProfile,
) -> Result<EnergyAuditReceipt, EnergyAuditError> {
    validate_profile(profile)?;
    let n = input.mass.rows;
    let m = input.contact_jacobian.rows;
    if n == 0
        || input.mass.cols != n
        || input.damping.rows != n
        || input.damping.cols != n
        || input.stiffness.rows != n
        || input.stiffness.cols != n
        || input.position_error.len() != n
        || input.velocity.len() != n
        || input.external_force.len() != n
        || result.next_velocity.len() != n
        || result.next_position_error.len() != n
        || input.contact_jacobian.cols != n
        || input.contact_gap.len() != m
        || result.contact_impulse.len() != m
        || result.contact_slack.len() != m
    {
        return Err(EnergyAuditError::ShapeMismatch);
    }

    let h = input.dt;
    if !h.is_finite() || h <= 0.0 {
        return Err(EnergyAuditError::ShapeMismatch);
    }
    check_finite(&input.position_error)?;
    check_finite(&input.velocity)?;
    check_finite(&input.external_force)?;
    check_finite(&input.contact_gap)?;
    check_finite(&result.next_velocity)?;
    check_finite(&result.next_position_error)?;
    check_finite(&result.contact_impulse)?;
    check_finite(&result.contact_slack)?;

    let mut next_position = vec![0.0; n];
    let mut delta_velocity = vec![0.0; n];
    let mut delta_position = vec![0.0; n];
    for dof in 0..n {
        next_position[dof] = input.position_error[dof] + h * result.next_velocity[dof];
        delta_velocity[dof] = result.next_velocity[dof] - input.velocity[dof];
        delta_position[dof] = next_position[dof] - input.position_error[dof];
        if close(next_position[dof], result.next_position_error[dof], profile).is_err() {
            return Err(EnergyAuditError::KinematicPositionMismatch { dof });
        }
    }

    let mut normal_velocity = vec![0.0; m];
    let mut target_velocity = vec![0.0; m];
    let mut slack = vec![0.0; m];
    for contact in 0..m {
        for dof in 0..n {
            normal_velocity[contact] +=
                input.contact_jacobian[(contact, dof)] * result.next_velocity[dof];
        }
        target_velocity[contact] = -input.contact_gap[contact] / h;
        slack[contact] = normal_velocity[contact] - target_velocity[contact];
        if close(slack[contact], result.contact_slack[contact], profile).is_err() {
            return Err(EnergyAuditError::ContactSlackMismatch { contact });
        }
    }
    check_finite(&normal_velocity)?;
    check_finite(&target_velocity)?;
    check_finite(&slack)?;

    let kinetic_energy_before = 0.5 * quadratic_form(&input.mass, &input.velocity)?;
    let kinetic_energy_after = 0.5 * quadratic_form(&input.mass, &result.next_velocity)?;
    let potential_energy_before = 0.5 * quadratic_form(&input.stiffness, &input.position_error)?;
    let potential_energy_after = 0.5 * quadratic_form(&input.stiffness, &next_position)?;
    let external_force_work = h * dot(&input.external_force, &result.next_velocity)?;
    let damping_dissipation = h * quadratic_form(&input.damping, &result.next_velocity)?;
    let mass_increment_dissipation = 0.5 * quadratic_form(&input.mass, &delta_velocity)?;
    let stiffness_increment_dissipation = 0.5 * quadratic_form(&input.stiffness, &delta_position)?;
    let contact_impulse_work = dot(&result.contact_impulse, &normal_velocity)?;
    let contact_target_work = dot(&result.contact_impulse, &target_velocity)?;
    let contact_complementarity_work = dot(&result.contact_impulse, &slack)?;

    let energy_balance_residual = (kinetic_energy_after - kinetic_energy_before)
        + (potential_energy_after - potential_energy_before)
        + damping_dissipation
        + mass_increment_dissipation
        + stiffness_increment_dissipation
        - external_force_work
        - contact_impulse_work;
    let energy_scale = (kinetic_energy_after - kinetic_energy_before).abs()
        + (potential_energy_after - potential_energy_before).abs()
        + damping_dissipation.abs()
        + mass_increment_dissipation.abs()
        + stiffness_increment_dissipation.abs()
        + external_force_work.abs()
        + contact_impulse_work.abs();
    let relative_energy_balance_residual = energy_balance_residual.abs()
        / energy_scale.max(f64::MIN_POSITIVE);

    let contact_work_decomposition_residual =
        contact_impulse_work - contact_target_work - contact_complementarity_work;
    let contact_work_scale =
        contact_impulse_work.abs() + contact_target_work.abs() + contact_complementarity_work.abs();
    let relative_contact_work_decomposition_residual = contact_work_decomposition_residual.abs()
        / contact_work_scale.max(f64::MIN_POSITIVE);

    let expected = DiscreteEnergyLedger {
        kinetic_energy_before,
        kinetic_energy_after,
        potential_energy_before,
        potential_energy_after,
        external_force_work,
        damping_dissipation,
        mass_increment_dissipation,
        stiffness_increment_dissipation,
        contact_impulse_work,
        contact_target_work,
        contact_complementarity_work,
        energy_balance_residual,
        relative_energy_balance_residual,
        contact_work_decomposition_residual,
        relative_contact_work_decomposition_residual,
    };
    let (
        maximum_absolute_field_difference,
        maximum_scaled_field_difference,
        checked_ledger_fields,
    ) = compare_ledger(&expected, &result.energy_ledger, profile)?;

    Ok(EnergyAuditReceipt {
        kinetic_energy_before,
        kinetic_energy_after,
        potential_energy_before,
        potential_energy_after,
        external_force_work,
        damping_dissipation,
        mass_increment_dissipation,
        stiffness_increment_dissipation,
        contact_impulse_work,
        contact_target_work,
        contact_complementarity_work,
        energy_balance_residual,
        relative_energy_balance_residual,
        contact_work_decomposition_residual,
        relative_contact_work_decomposition_residual,
        maximum_absolute_field_difference,
        maximum_scaled_field_difference,
        checked_ledger_fields,
        checked_position_components: n,
        checked_contact_slack_components: m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Matrix, solve_discrete_step};

    fn profile() -> EnergyAuditProfile {
        EnergyAuditProfile {
            absolute_tolerance: 2.0e-13,
            relative_tolerance: 2.0e-12,
        }
    }

    fn fixture_with_contact() -> DiscreteStepInput {
        DiscreteStepInput {
            mass: Matrix::new(2, 2, vec![2.0, 0.2, 0.2, 1.5]).unwrap(),
            damping: Matrix::new(2, 2, vec![0.4, 0.1, 0.1, 0.3]).unwrap(),
            stiffness: Matrix::new(2, 2, vec![4.0, -1.0, -1.0, 2.0]).unwrap(),
            position_error: vec![0.1, -0.2],
            velocity: vec![-1.0, -0.2],
            external_force: vec![0.5, -0.25],
            contact_jacobian: Matrix::new(1, 2, vec![1.0, 0.0]).unwrap(),
            contact_gap: vec![0.005],
            dt: 0.1,
        }
    }

    #[test]
    fn independent_energy_work_replay_accepts_solver_ledger() {
        let input = fixture_with_contact();
        let result = solve_discrete_step(&input).unwrap();
        let receipt = audit_discrete_energy_ledger(&input, &result, profile()).unwrap();
        assert_eq!(receipt.checked_ledger_fields, 15);
        assert_eq!(receipt.checked_position_components, 2);
        assert_eq!(receipt.checked_contact_slack_components, 1);
        assert!(receipt.maximum_absolute_field_difference < 1.0e-14);
        assert!(receipt.relative_energy_balance_residual < 1.0e-12);
        assert!(receipt.relative_contact_work_decomposition_residual < 1.0e-12);
    }

    #[test]
    fn independent_replay_rejects_a_tampered_energy_field() {
        let input = fixture_with_contact();
        let mut result = solve_discrete_step(&input).unwrap();
        result.energy_ledger.external_force_work += 0.01;
        assert_eq!(
            audit_discrete_energy_ledger(&input, &result, profile()),
            Err(EnergyAuditError::LedgerFieldMismatch("external_force_work"))
        );
    }

    #[test]
    fn independent_replay_rejects_tampered_contact_slack() {
        let input = fixture_with_contact();
        let mut result = solve_discrete_step(&input).unwrap();
        result.contact_slack[0] += 0.001;
        assert_eq!(
            audit_discrete_energy_ledger(&input, &result, profile()),
            Err(EnergyAuditError::ContactSlackMismatch { contact: 0 })
        );
    }

    #[test]
    fn independent_replay_rejects_tampered_next_position() {
        let input = fixture_with_contact();
        let mut result = solve_discrete_step(&input).unwrap();
        result.next_position_error[1] += 0.001;
        assert_eq!(
            audit_discrete_energy_ledger(&input, &result, profile()),
            Err(EnergyAuditError::KinematicPositionMismatch { dof: 1 })
        );
    }

    #[test]
    fn independent_replay_rejects_nonfinite_intermediate_values() {
        let input = fixture_with_contact();
        let mut result = solve_discrete_step(&input).unwrap();
        result.next_velocity[0] = f64::INFINITY;
        assert_eq!(
            audit_discrete_energy_ledger(&input, &result, profile()),
            Err(EnergyAuditError::NonFiniteDerivedValue)
        );
    }
}
