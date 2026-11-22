//! Anytime risk spending over an unbounded sequence of discrete integrity checks.
//!
//! The telescoping allocation spends alpha_k = B / (k * (k + 1)) for check
//! index k >= 1. Since sum_k 1/(k(k+1)) = 1, the sum of all ideal shares is
//! B. Integer flooring at the declared 1e-15 probability quantum makes the
//! implemented finite-precision allocations no larger than that budget.
//!
//! This is a multiple-check budget allocator, not a confidence sequence or
//! e-process. Each per-check test must already have a valid false-alarm bound
//! at its allocated alpha level. It covers discrete checks only.

use std::fmt;

const MAX_PROBABILITY_QUANTA: u64 = 1_000_000_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnytimeRiskShare {
    pub check_index: u64,
    pub mission_budget_quanta: u64,
    pub allocated_quanta: u64,
    pub denominator: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnytimeRiskSpendError {
    InvalidMissionBudget,
    ZeroCheckIndex,
    AllocationBelowQuantumResolution,
}

impl fmt::Display for AnytimeRiskSpendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMissionBudget => write!(f, "mission risk budget must be an integer probability in (0,1) at 1e-15 resolution"),
            Self::ZeroCheckIndex => write!(f, "check index is one-based and must be greater than zero"),
            Self::AllocationBelowQuantumResolution => write!(f, "per-check allocation is below one 1e-15 probability quantum"),
        }
    }
}
impl std::error::Error for AnytimeRiskSpendError {}

/// Allocate one risk share from a telescoping, horizon-free schedule.
///
/// For check index k >= 1:
///
/// alpha_k = floor(B / (k * (k + 1)))
///
/// The exact real-valued series sums to B. Flooring each share to integer
/// probability quanta makes the sum of implemented shares no greater than
/// B. The schedule is valid without assuming independence between checks,
/// provided each per-check test has a valid marginal false-alarm bound at
/// its assigned alpha level and the union bound is the desired composition.
pub fn allocate_telescoping_anytime_risk(
    mission_budget_quanta: u64,
    check_index: u64,
) -> Result<AnytimeRiskShare, AnytimeRiskSpendError> {
    if mission_budget_quanta == 0 || mission_budget_quanta >= MAX_PROBABILITY_QUANTA {
        return Err(AnytimeRiskSpendError::InvalidMissionBudget);
    }
    if check_index == 0 {
        return Err(AnytimeRiskSpendError::ZeroCheckIndex);
    }
    let index = u128::from(check_index);
    let denominator = index * (index + 1);
    let allocated = u128::from(mission_budget_quanta) / denominator;
    if allocated == 0 {
        return Err(AnytimeRiskSpendError::AllocationBelowQuantumResolution);
    }
    Ok(AnytimeRiskShare {
        check_index,
        mission_budget_quanta,
        allocated_quanta: allocated as u64,
        denominator,
    })
}

/// Sum a finite prefix for audit/demo purposes. Returns an exact integer
/// quanta total and refuses any prefix whose later check allocation falls
/// below the configured probability resolution.
pub fn sum_telescoping_anytime_prefix(
    mission_budget_quanta: u64,
    check_count: u64,
) -> Result<u64, AnytimeRiskSpendError> {
    if check_count == 0 {
        return Err(AnytimeRiskSpendError::ZeroCheckIndex);
    }
    let mut total = 0_u64;
    for index in 1..=check_count {
        let share = allocate_telescoping_anytime_risk(mission_budget_quanta, index)?;
        total = total.saturating_add(share.allocated_quanta);
        if total > mission_budget_quanta {
            return Err(AnytimeRiskSpendError::InvalidMissionBudget);
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_anytime_shares_follow_telescoping_schedule() {
        let budget = 1_000_000_000_000_u64;
        let first = allocate_telescoping_anytime_risk(budget, 1).unwrap();
        let second = allocate_telescoping_anytime_risk(budget, 2).unwrap();
        let third = allocate_telescoping_anytime_risk(budget, 3).unwrap();
        assert_eq!(first.allocated_quanta, 500_000_000_000);
        assert_eq!(second.allocated_quanta, 166_666_666_666);
        assert_eq!(third.allocated_quanta, 83_333_333_333);
        assert!(first.allocated_quanta > second.allocated_quanta);
        assert!(second.allocated_quanta > third.allocated_quanta);
    }

    #[test]
    fn finite_prefix_never_spends_more_than_the_total_budget() {
        let budget = 1_000_000_000_000_u64;
        for count in [1, 2, 3, 10, 100, 1_000] {
            let spent = sum_telescoping_anytime_prefix(budget, count).unwrap();
            assert!(spent <= budget);
        }
    }

    #[test]
    fn prefix_spending_is_monotonic_and_converges_toward_budget() {
        let budget = 1_000_000_000_000_u64;
        let first = sum_telescoping_anytime_prefix(budget, 10).unwrap();
        let second = sum_telescoping_anytime_prefix(budget, 100).unwrap();
        let third = sum_telescoping_anytime_prefix(budget, 1_000).unwrap();
        assert!(first < second);
        assert!(second < third);
        assert!(third <= budget);
        assert!(budget - third < budget / 1_000);
    }

    #[test]
    fn allocations_are_monotonic_and_use_exact_integer_arithmetic() {
        let budget = 123_456_789_u64;
        let mut previous = u64::MAX;
        for index in 1..=500 {
            let share = allocate_telescoping_anytime_risk(budget, index).unwrap();
            assert!(share.allocated_quanta <= previous);
            assert_eq!(share.denominator, u128::from(index) * (u128::from(index) + 1));
            previous = share.allocated_quanta;
        }
    }

    #[test]
    fn invalid_budget_zero_index_and_underflow_fail_closed() {
        assert_eq!(
            allocate_telescoping_anytime_risk(0, 1),
            Err(AnytimeRiskSpendError::InvalidMissionBudget)
        );
        assert_eq!(
            allocate_telescoping_anytime_risk(MAX_PROBABILITY_QUANTA, 1),
            Err(AnytimeRiskSpendError::InvalidMissionBudget)
        );
        assert_eq!(
            allocate_telescoping_anytime_risk(1_000, 0),
            Err(AnytimeRiskSpendError::ZeroCheckIndex)
        );
        assert_eq!(
            allocate_telescoping_anytime_risk(1_000, u64::MAX),
            Err(AnytimeRiskSpendError::AllocationBelowQuantumResolution)
        );
    }

    #[test]
    fn invalid_prefix_inputs_fail_closed() {
        assert_eq!(
            sum_telescoping_anytime_prefix(1_000, 0),
            Err(AnytimeRiskSpendError::ZeroCheckIndex)
        );
        assert_eq!(
            sum_telescoping_anytime_prefix(10, 10_000),
            Err(AnytimeRiskSpendError::AllocationBelowQuantumResolution)
        );
    }
}
