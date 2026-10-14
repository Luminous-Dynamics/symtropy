// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable oracle for replay-safe fractional-to-exact ecological settlement.
//!
//! The oracle starts after an upstream model has chosen a deterministic rational
//! desired flux. It freezes residual ownership/error-diffusion semantics without
//! selecting how arbitrary floating rates are calibrated into that rational form.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QuantizationSchemeVersion(u16);

const EXACT_RESIDUAL_V1: QuantizationSchemeVersion = QuantizationSchemeVersion(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RationalRate {
    numerator: u64,
    denominator: u64,
    scheme: QuantizationSchemeVersion,
}

impl RationalRate {
    fn v1(numerator: u64, denominator: u64) -> Self {
        Self {
            numerator,
            denominator,
            scheme: EXACT_RESIDUAL_V1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QuantizationState {
    denominator: u64,
    residual_numerator: u64,
    scheme: QuantizationSchemeVersion,
}

impl QuantizationState {
    fn v1(denominator: u64, residual_numerator: u64) -> Result<Self, QuantizationError> {
        if denominator == 0 {
            return Err(QuantizationError::ZeroDenominator);
        }
        if residual_numerator >= denominator {
            return Err(QuantizationError::NonCanonicalResidual {
                residual: residual_numerator,
                denominator,
            });
        }
        Ok(Self {
            denominator,
            residual_numerator,
            scheme: EXACT_RESIDUAL_V1,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QuantizationPlan {
    settled_units: u64,
    next_state: QuantizationState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuantizationError {
    ZeroDenominator,
    NonCanonicalResidual { residual: u64, denominator: u64 },
    SchemeMismatch {
        state: QuantizationSchemeVersion,
        rate: QuantizationSchemeVersion,
    },
    DenominatorMismatch { state: u64, rate: u64 },
    SettlementOverflow,
}

fn plan_quantization(
    state: QuantizationState,
    rate: RationalRate,
) -> Result<QuantizationPlan, QuantizationError> {
    if rate.denominator == 0 {
        return Err(QuantizationError::ZeroDenominator);
    }
    if state.residual_numerator >= state.denominator {
        return Err(QuantizationError::NonCanonicalResidual {
            residual: state.residual_numerator,
            denominator: state.denominator,
        });
    }
    if state.scheme != rate.scheme {
        return Err(QuantizationError::SchemeMismatch {
            state: state.scheme,
            rate: rate.scheme,
        });
    }
    if state.denominator != rate.denominator {
        return Err(QuantizationError::DenominatorMismatch {
            state: state.denominator,
            rate: rate.denominator,
        });
    }

    let accumulated = u128::from(state.residual_numerator) + u128::from(rate.numerator);
    let denominator = u128::from(rate.denominator);
    let settled_units = u64::try_from(accumulated / denominator)
        .map_err(|_| QuantizationError::SettlementOverflow)?;
    let residual_numerator = u64::try_from(accumulated % denominator)
        .expect("remainder is strictly smaller than a u64 denominator");

    Ok(QuantizationPlan {
        settled_units,
        next_state: QuantizationState {
            denominator: state.denominator,
            residual_numerator,
            scheme: state.scheme,
        },
    })
}

fn run_ticks(
    mut state: QuantizationState,
    rate: RationalRate,
    ticks: usize,
) -> Result<(u128, QuantizationState, Vec<u64>), QuantizationError> {
    let mut total = 0u128;
    let mut sequence = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        let plan = plan_quantization(state, rate)?;
        total += u128::from(plan.settled_units);
        sequence.push(plan.settled_units);
        state = plan.next_state;
    }
    Ok((total, state, sequence))
}

#[test]
fn one_third_settles_exactly_over_complete_cycles() {
    let state = QuantizationState::v1(3, 0).unwrap();
    let rate = RationalRate::v1(1, 3);
    let (settled, final_state, sequence) = run_ticks(state, rate, 3_000).unwrap();

    assert_eq!(settled, 1_000);
    assert_eq!(final_state.residual_numerator, 0);
    assert_eq!(&sequence[..6], &[0, 0, 1, 0, 0, 1]);
}

#[test]
fn every_prefix_stays_strictly_within_one_exact_unit_of_rational_integral() {
    let mut state = QuantizationState::v1(17, 0).unwrap();
    let rate = RationalRate::v1(7, 17);
    let mut settled = 0u128;

    for tick in 1u128..=20_000 {
        let plan = plan_quantization(state, rate).unwrap();
        settled += u128::from(plan.settled_units);
        state = plan.next_state;

        let settled_scaled = settled * u128::from(rate.denominator);
        let modeled_scaled = tick * u128::from(rate.numerator);
        assert!(settled_scaled.abs_diff(modeled_scaled) < u128::from(rate.denominator));
    }
}

#[test]
fn save_reload_with_residual_matches_continuous_execution() {
    let initial = QuantizationState::v1(13, 0).unwrap();
    let rate = RationalRate::v1(5, 13);

    let continuous = run_ticks(initial, rate, 10_000).unwrap();

    let first = run_ticks(initial, rate, 4_321).unwrap();
    let restored_state = first.1;
    let second = run_ticks(restored_state, rate, 10_000 - 4_321).unwrap();

    assert_eq!(first.0 + second.0, continuous.0);
    assert_eq!(second.1, continuous.1);
    assert_eq!(
        [first.2, second.2].concat(),
        continuous.2,
        "save/reload must preserve the exact settlement sequence"
    );
}

#[test]
fn resetting_residual_on_reload_changes_future_authority() {
    let initial = QuantizationState::v1(3, 0).unwrap();
    let rate = RationalRate::v1(1, 3);

    let first_tick = plan_quantization(initial, rate).unwrap();
    assert_eq!(first_tick.settled_units, 0);
    assert_eq!(first_tick.next_state.residual_numerator, 1);

    let preserved = run_ticks(first_tick.next_state, rate, 2).unwrap();
    let incorrectly_reset = run_ticks(QuantizationState::v1(3, 0).unwrap(), rate, 2).unwrap();

    assert_eq!(preserved.0, 1);
    assert_eq!(incorrectly_reset.0, 0);
    assert_ne!(preserved.2, incorrectly_reset.2);
}

#[test]
fn numerator_larger_than_denominator_settles_multiple_units_without_drift() {
    let state = QuantizationState::v1(3, 0).unwrap();
    let rate = RationalRate::v1(7, 3);
    let (settled, final_state, sequence) = run_ticks(state, rate, 3).unwrap();

    assert_eq!(sequence, vec![2, 2, 3]);
    assert_eq!(settled, 7);
    assert_eq!(final_state.residual_numerator, 0);
}

#[test]
fn invalid_denominator_and_residual_fail_closed() {
    assert_eq!(
        QuantizationState::v1(0, 0),
        Err(QuantizationError::ZeroDenominator)
    );
    assert_eq!(
        QuantizationState::v1(7, 7),
        Err(QuantizationError::NonCanonicalResidual {
            residual: 7,
            denominator: 7,
        })
    );

    let state = QuantizationState::v1(7, 0).unwrap();
    assert_eq!(
        plan_quantization(state, RationalRate::v1(1, 0)),
        Err(QuantizationError::ZeroDenominator)
    );
}

#[test]
fn scheme_and_scale_cannot_be_silently_reinterpreted() {
    let state = QuantizationState::v1(1_000, 311).unwrap();

    let different_scheme = RationalRate {
        numerator: 100,
        denominator: 1_000,
        scheme: QuantizationSchemeVersion(2),
    };
    assert_eq!(
        plan_quantization(state, different_scheme),
        Err(QuantizationError::SchemeMismatch {
            state: EXACT_RESIDUAL_V1,
            rate: QuantizationSchemeVersion(2),
        })
    );

    assert_eq!(
        plan_quantization(state, RationalRate::v1(100, 1_000_000)),
        Err(QuantizationError::DenominatorMismatch {
            state: 1_000,
            rate: 1_000_000,
        })
    );
}

#[test]
fn valid_u64_boundary_uses_widened_accumulation() {
    let state = QuantizationState::v1(u64::MAX, u64::MAX - 1).unwrap();
    let rate = RationalRate::v1(u64::MAX, u64::MAX);
    let plan = plan_quantization(state, rate).unwrap();

    assert_eq!(plan.settled_units, 1);
    assert_eq!(plan.next_state.residual_numerator, u64::MAX - 1);

    let state = QuantizationState::v1(2, 1).unwrap();
    let rate = RationalRate::v1(u64::MAX, 2);
    let plan = plan_quantization(state, rate).unwrap();
    assert_eq!(plan.settled_units, 1u64 << 63);
    assert_eq!(plan.next_state.residual_numerator, 0);
}

#[test]
fn discarded_plan_does_not_advance_committed_residual() {
    let committed = QuantizationState::v1(5, 2).unwrap();
    let rate = RationalRate::v1(4, 5);

    let planned = plan_quantization(committed, rate).unwrap();
    assert_ne!(planned.next_state, committed);

    // Simulate an outer transaction failure: the plan is discarded and the
    // committed process state remains byte-for-byte unchanged.
    let after_abort = committed;
    assert_eq!(after_abort, committed);

    let retry = plan_quantization(after_abort, rate).unwrap();
    assert_eq!(retry, planned);
}

#[test]
fn independent_process_accumulators_do_not_share_fractional_history() {
    let rate = RationalRate::v1(1, 3);
    let fresh = QuantizationState::v1(3, 0).unwrap();

    let process_a_tick1 = plan_quantization(fresh, rate).unwrap();
    let process_a_tick2 = plan_quantization(process_a_tick1.next_state, rate).unwrap();
    let process_b_tick1 = plan_quantization(fresh, rate).unwrap();

    assert_eq!(process_a_tick2.next_state.residual_numerator, 2);
    assert_eq!(process_b_tick1.next_state.residual_numerator, 1);
    assert_eq!(process_b_tick1.settled_units, 0);
}

#[test]
fn residual_is_not_counted_as_settled_material() {
    let state = QuantizationState::v1(10, 9).unwrap();
    let rate = RationalRate::v1(0, 10);
    let plan = plan_quantization(state, rate).unwrap();

    assert_eq!(plan.settled_units, 0);
    assert_eq!(plan.next_state.residual_numerator, 9);
}
