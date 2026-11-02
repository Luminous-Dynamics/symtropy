//! Exact-formula repeated-bounce sequence for the one-dimensional reference model.
//!
//! This module separates three mathematical regimes:
//! - e = 0: the first impact settles the point mass at the plane;
//! - 0 < e < 1: infinitely many impacts accumulate in finite time (Zeno);
//! - e = 1: impacts continue forever without finite-time accumulation.
//!
//! It is an analytic point-mass reference, not a chatter regularization policy
//! or a multi-contact impact solver.

use super::{first_impact, finite_or_overflow, ImpactError, ImpactParameters};

/// Mathematical regime of the ideal repeated-bounce sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactSequenceRegime {
    /// A perfectly inelastic first impact leaves the body at rest.
    SettlesAfterFirstImpact,
    /// For 0 < restitution < 1, infinitely many impacts accumulate in finite time.
    FiniteTimeZeno,
    /// For restitution = 1, impacts continue with a nonzero period.
    NonAccumulating,
}

/// Summary of the ideal infinite impact sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactSequenceSummary {
    pub regime: ImpactSequenceRegime,
    pub first_impact_time_s: f64,
    /// Present only when 0 < restitution < 1.
    pub zeno_accumulation_time_s: Option<f64>,
}

/// State immediately after a specified impact (impact numbers are one-based).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BounceImpactState {
    pub impact_number: u64,
    pub impact_time_s: f64,
    pub incoming_speed_m_s: f64,
    pub outgoing_speed_m_s: f64,
    pub pre_impact_kinetic_energy_j: f64,
    pub post_impact_kinetic_energy_j: f64,
    pub impact_dissipation_j: f64,
    pub cumulative_impact_dissipation_j: f64,
    /// Initial mechanical energy minus post-impact kinetic energy and
    /// cumulative physical impact dissipation.
    pub cumulative_energy_balance_residual_j: f64,
}

fn is_zero_restitution(value: f64) -> bool {
    value.to_bits() & 0x7fff_ffff_ffff_ffff == 0
}

fn is_unit_restitution(value: f64) -> bool {
    value.to_bits() == 1.0_f64.to_bits()
}

fn pow_u64(mut base: f64, mut exponent: u64) -> f64 {
    let mut result = 1.0;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result *= base;
        }
        exponent >>= 1;
        if exponent > 0 {
            base *= base;
        }
    }
    result
}

/// Compute e + e^2 + ... + e^n without catastrophic cancellation near e = 1.
fn geometric_sum_from_one(restitution: f64, terms: u64) -> f64 {
    if terms == 0 || is_zero_restitution(restitution) {
        return 0.0;
    }
    if is_unit_restitution(restitution) {
        return terms as f64;
    }

    let exponent = (terms as f64) * restitution.ln();
    restitution * (-exponent.exp_m1()) / (1.0 - restitution)
}

/// Classify the ideal repeated-impact sequence and, for Zeno cases,
/// calculate its finite accumulation time.
pub fn impact_sequence_summary(
    parameters: ImpactParameters,
) -> Result<ImpactSequenceSummary, ImpactError> {
    let first = first_impact(parameters)?;
    let initial_speed = -first.pre_impact_velocity_m_s;

    let restitution = parameters.restitution;
    let (regime, zeno_accumulation_time_s) = if is_zero_restitution(restitution) {
        (ImpactSequenceRegime::SettlesAfterFirstImpact, None)
    } else if is_unit_restitution(restitution) {
        (ImpactSequenceRegime::NonAccumulating, None)
    } else {
        let flight_scale = finite_or_overflow(
            (initial_speed / parameters.gravity_m_s2) * 2.0,
        )?;
        let remaining_flight_sum =
            finite_or_overflow(restitution / (1.0 - restitution))?;
        let tail = finite_or_overflow(flight_scale * remaining_flight_sum)?;
        let zeno_time =
            finite_or_overflow(first.impact_time_s + tail)?;
        if zeno_time <= first.impact_time_s {
            return Err(ImpactError::TimeResolutionExhausted);
        }
        (ImpactSequenceRegime::FiniteTimeZeno, Some(zeno_time))
    };

    Ok(ImpactSequenceSummary {
        regime,
        first_impact_time_s: first.impact_time_s,
        zeno_accumulation_time_s,
    })
}

/// Compute the state immediately after impact number impact_number.
///
/// The index is one-based. A zero index is invalid. With zero restitution,
/// no second distinct impact exists: the body settles at the plane after the
/// first impact. For finite precision, an event whose time cannot be
/// distinguished from the preceding event returns TimeResolutionExhausted.
pub fn bounce_impact_state(
    parameters: ImpactParameters,
    impact_number: u64,
) -> Result<BounceImpactState, ImpactError> {
    if impact_number == 0 {
        return Err(ImpactError::InvalidImpactNumber);
    }

    let first = first_impact(parameters)?;
    let restitution = parameters.restitution;
    if impact_number > 1 && is_zero_restitution(restitution) {
        return Err(ImpactError::NoFurtherImpactAfterRest);
    }

    let initial_speed = -first.pre_impact_velocity_m_s;
    let flight_scale = finite_or_overflow(
        (initial_speed / parameters.gravity_m_s2) * 2.0,
    )?;
    let terms = impact_number - 1;
    let elapsed_flight = finite_or_overflow(
        flight_scale * geometric_sum_from_one(restitution, terms),
    )?;
    let impact_time = finite_or_overflow(first.impact_time_s + elapsed_flight)?;

    if impact_number > 1 {
        let previous_elapsed = finite_or_overflow(
            flight_scale * geometric_sum_from_one(restitution, terms - 1),
        )?;
        let previous_time =
            finite_or_overflow(first.impact_time_s + previous_elapsed)?;
        if impact_time <= previous_time {
            return Err(ImpactError::TimeResolutionExhausted);
        }
    }

    let incoming_speed = finite_or_overflow(
        initial_speed * pow_u64(restitution, impact_number - 1),
    )?;
    let outgoing_speed = finite_or_overflow(incoming_speed * restitution)?;
    let incoming_speed_squared = finite_or_overflow(incoming_speed * incoming_speed)?;
    let outgoing_speed_squared = finite_or_overflow(outgoing_speed * outgoing_speed)?;

    let pre_impact_kinetic_energy_j = finite_or_overflow(
        0.5 * parameters.mass_kg * incoming_speed_squared,
    )?;
    let post_impact_kinetic_energy_j = finite_or_overflow(
        0.5 * parameters.mass_kg * outgoing_speed_squared,
    )?;
    let one_minus_e_squared =
        finite_or_overflow(1.0 - restitution * restitution)?;
    let impact_dissipation_j = finite_or_overflow(
        0.5 * parameters.mass_kg * one_minus_e_squared * incoming_speed_squared,
    )?;

    let cumulative_fraction = if is_zero_restitution(restitution) {
        1.0
    } else if is_unit_restitution(restitution) {
        0.0
    } else {
        let exponent = 2.0 * (impact_number as f64) * restitution.ln();
        -exponent.exp_m1()
    };
    let cumulative_impact_dissipation_j = finite_or_overflow(
        first.pre_impact_kinetic_energy_j * cumulative_fraction,
    )?;
    let cumulative_energy_balance_residual_j = finite_or_overflow(
        first.initial_mechanical_energy_j
            - post_impact_kinetic_energy_j
            - cumulative_impact_dissipation_j,
    )?;

    Ok(BounceImpactState {
        impact_number,
        impact_time_s: impact_time,
        incoming_speed_m_s: incoming_speed,
        outgoing_speed_m_s: outgoing_speed,
        pre_impact_kinetic_energy_j,
        post_impact_kinetic_energy_j,
        impact_dissipation_j,
        cumulative_impact_dissipation_j,
        cumulative_energy_balance_residual_j,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(restitution: f64) -> ImpactParameters {
        ImpactParameters {
            mass_kg: 3.0,
            gravity_m_s2: 2.0,
            initial_height_m: 1.0,
            initial_velocity_m_s: 0.0,
            restitution,
        }
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:?} differs from expected {expected:?} by more than {tolerance:?}"
        );
    }

    #[test]
    fn inelastic_sequence_has_finite_zeno_accumulation_time() {
        let summary = impact_sequence_summary(fixture(0.5)).expect("valid fixture");
        assert_eq!(summary.regime, ImpactSequenceRegime::FiniteTimeZeno);
        close(summary.first_impact_time_s, 1.0, 1.0e-14);
        close(summary.zeno_accumulation_time_s.expect("Zeno time"), 3.0, 1.0e-14);
    }

    #[test]
    fn first_three_bounces_match_geometric_sequence_and_energy_ledger() {
        let first = bounce_impact_state(fixture(0.5), 1).expect("first impact");
        let second = bounce_impact_state(fixture(0.5), 2).expect("second impact");
        let third = bounce_impact_state(fixture(0.5), 3).expect("third impact");

        close(first.impact_time_s, 1.0, 1.0e-14);
        close(first.incoming_speed_m_s, 2.0, 1.0e-14);
        close(first.outgoing_speed_m_s, 1.0, 1.0e-14);
        close(first.impact_dissipation_j, 4.5, 1.0e-14);

        close(second.impact_time_s, 2.0, 1.0e-14);
        close(second.incoming_speed_m_s, 1.0, 1.0e-14);
        close(second.outgoing_speed_m_s, 0.5, 1.0e-14);
        close(second.impact_dissipation_j, 1.125, 1.0e-14);
        close(second.cumulative_impact_dissipation_j, 5.625, 1.0e-14);
        close(second.cumulative_energy_balance_residual_j, 0.0, 1.0e-14);

        close(third.impact_time_s, 2.5, 1.0e-14);
        close(third.incoming_speed_m_s, 0.5, 1.0e-14);
        close(third.outgoing_speed_m_s, 0.25, 1.0e-14);
    }

    #[test]
    fn perfectly_inelastic_first_impact_settles_without_a_second_impact() {
        let summary = impact_sequence_summary(fixture(0.0)).expect("valid fixture");
        assert_eq!(
            summary.regime,
            ImpactSequenceRegime::SettlesAfterFirstImpact
        );
        assert_eq!(summary.zeno_accumulation_time_s, None);
        assert_eq!(
            bounce_impact_state(fixture(0.0), 2),
            Err(ImpactError::NoFurtherImpactAfterRest)
        );
    }

    #[test]
    fn perfectly_elastic_sequence_does_not_accumulate_in_finite_time() {
        let summary = impact_sequence_summary(fixture(1.0)).expect("valid fixture");
        assert_eq!(summary.regime, ImpactSequenceRegime::NonAccumulating);
        assert_eq!(summary.zeno_accumulation_time_s, None);

        let second = bounce_impact_state(fixture(1.0), 2).expect("second impact");
        let third = bounce_impact_state(fixture(1.0), 3).expect("third impact");
        close(second.impact_time_s, 3.0, 1.0e-14);
        close(third.impact_time_s, 5.0, 1.0e-14);
        close(third.incoming_speed_m_s, 2.0, 1.0e-14);
        close(third.cumulative_impact_dissipation_j, 0.0, 0.0);
    }

    #[test]
    fn near_elastic_finite_sum_does_not_collapse_from_subtraction_cancellation() {
        let parameters = fixture(1.0 - f64::EPSILON);
        let state = bounce_impact_state(parameters, 2).expect("second impact");
        let expected = 1.0 + 2.0 * 2.0 / 2.0 * parameters.restitution;
        close(state.impact_time_s, expected, 1.0e-14);
    }

    #[test]
    fn rejects_zero_impact_number() {
        assert_eq!(
            bounce_impact_state(fixture(0.5), 0),
            Err(ImpactError::InvalidImpactNumber)
        );
    }

    #[test]
    fn finite_precision_does_not_report_collapsed_event_times_as_distinct() {
        assert_eq!(
            bounce_impact_state(fixture(0.5), u64::MAX),
            Err(ImpactError::TimeResolutionExhausted)
        );
    }
}
