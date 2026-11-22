//! Risk-budget composition for a declared finite set of mission checks.
//!
//! The union bound gives P(any check fails) <= sum_i P(check_i fails),
//! regardless of dependence between checks. This module only allocates
//! numeric budgets; it does not calibrate any estimator or prove that
//! continuous-time behaviour between declared checks is bounded.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct MissionRiskAllocation {
    pub mission_risk_budget: f64,
    pub per_check_risk_budgets: Vec<f64>,
    pub allocated_risk: f64,
    pub unallocated_risk: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissionRiskBudgetError {
    InvalidMissionRiskBudget,
    EmptyCheckSet,
    InvalidAllocationWeight,
    AllocationUnderflow,
    AllocationExceedsBudget,
}

impl fmt::Display for MissionRiskBudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMissionRiskBudget => write!(f, "mission risk budget must be finite and in (0,1)"),
            Self::EmptyCheckSet => write!(f, "at least one predeclared mission check is required"),
            Self::InvalidAllocationWeight => write!(f, "risk allocation weights must be finite and positive"),
            Self::AllocationUnderflow => write!(f, "a positive check allocation underflowed to zero"),
            Self::AllocationExceedsBudget => write!(f, "per-check risk allocations exceed the mission budget"),
        }
    }
}
impl std::error::Error for MissionRiskBudgetError {}

fn validate_mission_budget(budget: f64) -> Result<(), MissionRiskBudgetError> {
    if !budget.is_finite() || budget <= 0.0 || budget >= 1.0 {
        return Err(MissionRiskBudgetError::InvalidMissionRiskBudget);
    }
    Ok(())
}

/// Allocate a mission risk budget equally across a fixed set of checks.
///
/// For each check i, the caller must separately ensure P(check_i fails) is
/// bounded by the returned budget. Then the probability of any failure over
/// the finite declared check set is bounded by the sum, without requiring
/// independent checks. Checks are not inferred from sample frequency.
pub fn allocate_equal_mission_risk(
    mission_risk_budget: f64,
    check_count: usize,
) -> Result<MissionRiskAllocation, MissionRiskBudgetError> {
    if check_count == 0 {
        return Err(MissionRiskBudgetError::EmptyCheckSet);
    }
    allocate_weighted_mission_risk(mission_risk_budget, &vec![1.0; check_count])
}

/// Allocate risk proportionally to caller-supplied relative risk shares.
///
/// A larger weight receives a larger permitted risk share. Weights express
/// risk-budget allocation, not hazard severity, observation confidence or
/// empirical failure frequency. At least one check is required.
pub fn allocate_weighted_mission_risk(
    mission_risk_budget: f64,
    allocation_weights: &[f64],
) -> Result<MissionRiskAllocation, MissionRiskBudgetError> {
    validate_mission_budget(mission_risk_budget)?;
    if allocation_weights.is_empty() {
        return Err(MissionRiskBudgetError::EmptyCheckSet);
    }
    if allocation_weights
        .iter()
        .any(|weight| !weight.is_finite() || *weight <= 0.0)
    {
        return Err(MissionRiskBudgetError::InvalidAllocationWeight);
    }

    // Normalize by the maximum first to avoid overflow when summing weights.
    let max_weight = allocation_weights
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);
    let scaled: Vec<f64> = allocation_weights
        .iter()
        .map(|weight| weight / max_weight)
        .collect();
    if scaled.iter().any(|weight| *weight <= 0.0) {
        return Err(MissionRiskBudgetError::AllocationUnderflow);
    }
    let weight_sum: f64 = scaled.iter().sum();
    if !weight_sum.is_finite() || weight_sum <= 0.0 {
        return Err(MissionRiskBudgetError::InvalidAllocationWeight);
    }

    // Assign rounding residue to the largest-weight check. This avoids a
    // computed allocation sum just above the original mission budget.
    let anchor = scaled
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(index, _)| index)
        .ok_or(MissionRiskBudgetError::EmptyCheckSet)?;
    let mut per_check = vec![0.0; scaled.len()];
    let mut allocated_other = 0.0_f64;
    for (index, weight) in scaled.iter().enumerate() {
        if index == anchor {
            continue;
        }
        let share = mission_risk_budget * (*weight / weight_sum);
        if !share.is_finite() || share <= 0.0 {
            return Err(MissionRiskBudgetError::AllocationUnderflow);
        }
        per_check[index] = share;
        allocated_other += share;
    }

    let anchor_share = mission_risk_budget - allocated_other;
    if !anchor_share.is_finite() || anchor_share <= 0.0 {
        return Err(MissionRiskBudgetError::AllocationUnderflow);
    }
    per_check[anchor] = anchor_share;

    let mut allocated_risk: f64 = per_check.iter().sum();
    if allocated_risk > mission_risk_budget {
        per_check[anchor] -= allocated_risk - mission_risk_budget;
        allocated_risk = per_check.iter().sum();
    }
    if per_check.iter().any(|share| !share.is_finite() || *share <= 0.0)
        || !allocated_risk.is_finite()
        || allocated_risk > mission_risk_budget
    {
        return Err(MissionRiskBudgetError::AllocationExceedsBudget);
    }

    Ok(MissionRiskAllocation {
        mission_risk_budget,
        per_check_risk_budgets: per_check,
        allocated_risk,
        unallocated_risk: mission_risk_budget - allocated_risk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_allocation_composes_to_mission_risk_under_union_bound() {
        let allocation = allocate_equal_mission_risk(1.0e-3, 785).unwrap();
        assert_eq!(allocation.per_check_risk_budgets.len(), 785);
        assert!(allocation.per_check_risk_budgets.iter().all(|risk| *risk > 0.0));
        assert!(allocation.allocated_risk <= allocation.mission_risk_budget);
        assert!(allocation.unallocated_risk >= 0.0);
        let expected = 1.0e-3 / 785.0;
        assert!((allocation.per_check_risk_budgets[0] - expected).abs() < 1.0e-15);
    }

    #[test]
    fn weighted_allocation_preserves_relative_shares() {
        let allocation = allocate_weighted_mission_risk(1.0e-3, &[1.0, 2.0, 1.0]).unwrap();
        let budgets = &allocation.per_check_risk_budgets;
        assert!((budgets[0] - 2.5e-4).abs() < 1.0e-15);
        assert!((budgets[1] - 5.0e-4).abs() < 1.0e-15);
        assert!((budgets[2] - 2.5e-4).abs() < 1.0e-15);
        assert!(allocation.allocated_risk <= allocation.mission_risk_budget);
    }

    #[test]
    fn one_check_receives_the_entire_mission_budget() {
        let allocation = allocate_equal_mission_risk(1.0e-3, 1).unwrap();
        assert_eq!(allocation.per_check_risk_budgets, vec![1.0e-3]);
        assert_eq!(allocation.allocated_risk, 1.0e-3);
    }

    #[test]
    fn scaling_large_weights_does_not_overflow() {
        let allocation = allocate_weighted_mission_risk(1.0e-3, &[f64::MAX, f64::MAX]).unwrap();
        assert!((allocation.per_check_risk_budgets[0] - 5.0e-4).abs() < 1.0e-15);
        assert!((allocation.per_check_risk_budgets[1] - 5.0e-4).abs() < 1.0e-15);
    }

    #[test]
    fn empty_invalid_and_underflowing_allocations_fail_closed() {
        assert_eq!(
            allocate_equal_mission_risk(1.0e-3, 0),
            Err(MissionRiskBudgetError::EmptyCheckSet)
        );
        assert_eq!(
            allocate_equal_mission_risk(0.0, 1),
            Err(MissionRiskBudgetError::InvalidMissionRiskBudget)
        );
        assert_eq!(
            allocate_weighted_mission_risk(1.0e-3, &[1.0, 0.0]),
            Err(MissionRiskBudgetError::InvalidAllocationWeight)
        );
        assert_eq!(
            allocate_weighted_mission_risk(1.0e-3, &[f64::MAX, f64::MIN_POSITIVE]),
            Err(MissionRiskBudgetError::AllocationUnderflow)
        );
    }
}
