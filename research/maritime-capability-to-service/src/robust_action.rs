//! Joint-risk-budgeted robust feasibility for a bounded port-resource action.
//!
//! Combines marginal split-conformal intervals using a predeclared union-bound
//! alpha allocation. This is a conservative resource gate, not a full berth,
//! quay-crane, yard or port-call optimizer.

use crate::conformal::{
    ConformalInterval, CoverageClaim, DecisionGate,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JointRiskBudget {
    pub target_joint_alpha_ppm: u32,
    pub arrival_alpha_ppm: u32,
    pub handling_alpha_ppm: u32,
    pub crane_minutes_alpha_ppm: u32,
    pub shore_power_alpha_ppm: u32,
    pub yard_slots_alpha_ppm: u32,
}

impl JointRiskBudget {
    fn allocations(self) -> [u32; 5] {
        [
            self.arrival_alpha_ppm,
            self.handling_alpha_ppm,
            self.crane_minutes_alpha_ppm,
            self.shore_power_alpha_ppm,
            self.yard_slots_alpha_ppm,
        ]
    }

    fn total_allocated_alpha(self) -> Result<u32, String> {
        if self.target_joint_alpha_ppm == 0 || self.target_joint_alpha_ppm >= 1_000_000 {
            return Err("target joint alpha must be between 0 and 1,000,000 ppm".to_owned());
        }
        let mut total = 0u32;
        for allocation in self.allocations() {
            if allocation == 0 || allocation >= 1_000_000 {
                return Err("each resource alpha allocation must be positive and below 1,000,000 ppm".to_owned());
            }
            total = total
                .checked_add(allocation)
                .ok_or_else(|| "joint alpha allocation arithmetic overflow".to_owned())?;
        }
        Ok(total)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointPredictionIntervals {
    pub arrival: Option<ConformalInterval>,
    pub handling_duration: Option<ConformalInterval>,
    pub crane_minutes: Option<ConformalInterval>,
    pub shore_power_kw: Option<ConformalInterval>,
    pub yard_slots: Option<ConformalInterval>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceCapacities {
    pub berth_slot_start: u64,
    pub berth_slot_end: u64,
    pub receiver_deadline: u64,
    pub crane_minutes_available: u64,
    pub shore_power_kw_available: u64,
    pub yard_slots_available: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceConstraintCheck {
    pub constraint: &'static str,
    pub gate: DecisionGate,
    pub required_upper: u64,
    pub available: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RobustActionAssessment {
    pub gate: DecisionGate,
    pub reason: &'static str,
    pub allocated_joint_alpha_ppm: u32,
    pub target_joint_alpha_ppm: u32,
    pub joint_coverage_lower_bound_ppm: Option<u32>,
    pub decision_cutoff: Option<u64>,
    pub worst_case_berth_start: Option<u64>,
    pub worst_case_finish: Option<u64>,
    pub constraints: Vec<ResourceConstraintCheck>,
}

fn indeterminate(
    budget: JointRiskBudget,
    reason: &'static str,
    decision_cutoff: Option<u64>,
) -> RobustActionAssessment {
    RobustActionAssessment {
        gate: DecisionGate::Indeterminate,
        reason,
        allocated_joint_alpha_ppm: budget.allocations().iter().copied().sum(),
        target_joint_alpha_ppm: budget.target_joint_alpha_ppm,
        joint_coverage_lower_bound_ppm: None,
        decision_cutoff,
        worst_case_berth_start: None,
        worst_case_finish: None,
        constraints: Vec::new(),
    }
}

fn interval_is_well_formed(interval: &ConformalInterval, expected_alpha_ppm: u32) -> bool {
    interval.alpha_ppm == expected_alpha_ppm
        && interval.coverage_claim
            == CoverageClaim::MarginalSplitConformalUnderDeclaredExchangeability
        && interval.nominal_coverage_claim_ppm == Some(1_000_000 - expected_alpha_ppm)
        && interval.sample_count > 0
        && interval.quantile_rank_1based > 0
        && interval.quantile_rank_1based <= interval.sample_count
        && interval.lower <= interval.point_prediction
        && interval.point_prediction <= interval.upper
        && interval.point_prediction.saturating_sub(interval.radius) == interval.lower
        && interval.point_prediction.checked_add(interval.radius) == Some(interval.upper)
}

/// Evaluate a single action against simultaneous uncertainty bounds.
///
/// The five marginal miscoverage allocations must sum to no more than the
/// predeclared joint alpha. By the union bound, this gives a conservative lower
/// bound on joint interval coverage without assuming independent resources.
/// Every interval must be calibrated under the same decision cutoff and must
/// retain its predeclared alpha allocation.
pub fn assess_joint_robust_action(
    risk_budget: JointRiskBudget,
    intervals: &JointPredictionIntervals,
    capacities: ResourceCapacities,
) -> Result<RobustActionAssessment, String> {
    let allocated_alpha = risk_budget.total_allocated_alpha()?;
    if allocated_alpha > risk_budget.target_joint_alpha_ppm {
        return Ok(RobustActionAssessment {
            gate: DecisionGate::Indeterminate,
            reason: "sum of marginal alpha allocations exceeds the joint risk budget",
            allocated_joint_alpha_ppm: allocated_alpha,
            target_joint_alpha_ppm: risk_budget.target_joint_alpha_ppm,
            joint_coverage_lower_bound_ppm: None,
            decision_cutoff: None,
            worst_case_berth_start: None,
            worst_case_finish: None,
            constraints: Vec::new(),
        });
    }

    if capacities.berth_slot_start > capacities.berth_slot_end {
        return Err("berth slot start must not exceed berth slot end".to_owned());
    }

    let (
        Some(arrival),
        Some(handling),
        Some(crane),
        Some(power),
        Some(yard),
    ) = (
        intervals.arrival.as_ref(),
        intervals.handling_duration.as_ref(),
        intervals.crane_minutes.as_ref(),
        intervals.shore_power_kw.as_ref(),
        intervals.yard_slots.as_ref(),
    )
    else {
        return Ok(RobustActionAssessment {
            gate: DecisionGate::Indeterminate,
            reason: "one or more resource prediction intervals are unbounded or missing",
            allocated_joint_alpha_ppm: allocated_alpha,
            target_joint_alpha_ppm: risk_budget.target_joint_alpha_ppm,
            joint_coverage_lower_bound_ppm: None,
            decision_cutoff: None,
            worst_case_berth_start: None,
            worst_case_finish: None,
            constraints: Vec::new(),
        });
    };

    let decision_cutoff = arrival.decision_cutoff;
    if [
        handling.decision_cutoff,
        crane.decision_cutoff,
        power.decision_cutoff,
        yard.decision_cutoff,
    ]
    .iter()
    .any(|cutoff| *cutoff != decision_cutoff)
    {
        return Ok(indeterminate(
            risk_budget,
            "resource intervals were calibrated for different decision cutoffs",
            None,
        ));
    }

    let allocations = risk_budget.allocations();
    let used_intervals = [arrival, handling, crane, power, yard];
    if !used_intervals
        .iter()
        .zip(allocations)
        .all(|(interval, alpha)| interval_is_well_formed(interval, alpha))
    {
        return Ok(indeterminate(
            risk_budget,
            "interval applicability, alpha allocation or endpoint contract is invalid",
            Some(decision_cutoff),
        ));
    }

    let worst_case_berth_start = arrival.upper.max(capacities.berth_slot_start);
    let Some(worst_case_finish) = worst_case_berth_start.checked_add(handling.upper) else {
        return Ok(indeterminate(
            risk_budget,
            "worst-case berth completion arithmetic overflow",
            Some(decision_cutoff),
        ));
    };

    let berth_available = capacities.berth_slot_end.min(capacities.receiver_deadline);
    let constraints = vec![
        ResourceConstraintCheck {
            constraint: "berth_window_and_receiver_deadline",
            gate: if worst_case_finish <= berth_available {
                DecisionGate::Feasible
            } else {
                DecisionGate::Infeasible
            },
            required_upper: worst_case_finish,
            available: berth_available,
        },
        ResourceConstraintCheck {
            constraint: "quay_crane_minutes",
            gate: if crane.upper <= capacities.crane_minutes_available {
                DecisionGate::Feasible
            } else {
                DecisionGate::Infeasible
            },
            required_upper: crane.upper,
            available: capacities.crane_minutes_available,
        },
        ResourceConstraintCheck {
            constraint: "shore_power_kw",
            gate: if power.upper <= capacities.shore_power_kw_available {
                DecisionGate::Feasible
            } else {
                DecisionGate::Infeasible
            },
            required_upper: power.upper,
            available: capacities.shore_power_kw_available,
        },
        ResourceConstraintCheck {
            constraint: "yard_slots",
            gate: if yard.upper <= capacities.yard_slots_available {
                DecisionGate::Feasible
            } else {
                DecisionGate::Infeasible
            },
            required_upper: yard.upper,
            available: capacities.yard_slots_available,
        },
    ];

    let gate = if constraints
        .iter()
        .any(|check| check.gate == DecisionGate::Infeasible)
    {
        DecisionGate::Infeasible
    } else {
        DecisionGate::Feasible
    };

    Ok(RobustActionAssessment {
        gate,
        reason: if gate == DecisionGate::Feasible {
            "joint conformal risk budget and all adverse resource bounds pass"
        } else {
            "at least one adverse resource or service-time bound exceeds capacity"
        },
        allocated_joint_alpha_ppm: allocated_alpha,
        target_joint_alpha_ppm: risk_budget.target_joint_alpha_ppm,
        joint_coverage_lower_bound_ppm: Some(1_000_000 - allocated_alpha),
        decision_cutoff: Some(decision_cutoff),
        worst_case_berth_start: Some(worst_case_berth_start),
        worst_case_finish: Some(worst_case_finish),
        constraints,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(each_alpha: u32, target: u32) -> JointRiskBudget {
        JointRiskBudget {
            target_joint_alpha_ppm: target,
            arrival_alpha_ppm: each_alpha,
            handling_alpha_ppm: each_alpha,
            crane_minutes_alpha_ppm: each_alpha,
            shore_power_alpha_ppm: each_alpha,
            yard_slots_alpha_ppm: each_alpha,
        }
    }

    fn interval(
        point: u64,
        radius: u64,
        alpha_ppm: u32,
        cutoff: u64,
    ) -> ConformalInterval {
        ConformalInterval {
            point_prediction: point,
            lower: point.saturating_sub(radius),
            upper: point + radius,
            radius,
            alpha_ppm,
            sample_count: 100,
            quantile_rank_1based: 91,
            coverage_claim: CoverageClaim::MarginalSplitConformalUnderDeclaredExchangeability,
            nominal_coverage_claim_ppm: Some(1_000_000 - alpha_ppm),
            decision_cutoff: cutoff,
        }
    }

    fn intervals(alpha_ppm: u32, cutoff: u64) -> JointPredictionIntervals {
        JointPredictionIntervals {
            arrival: Some(interval(100, 5, alpha_ppm, cutoff)),
            handling_duration: Some(interval(10, 2, alpha_ppm, cutoff)),
            crane_minutes: Some(interval(30, 5, alpha_ppm, cutoff)),
            shore_power_kw: Some(interval(100, 10, alpha_ppm, cutoff)),
            yard_slots: Some(interval(20, 2, alpha_ppm, cutoff)),
        }
    }

    fn capacities() -> ResourceCapacities {
        ResourceCapacities {
            berth_slot_start: 90,
            berth_slot_end: 130,
            receiver_deadline: 140,
            crane_minutes_available: 40,
            shore_power_kw_available: 120,
            yard_slots_available: 22,
        }
    }

    #[test]
    fn five_two_percent_marginals_fit_a_ten_percent_joint_union_bound() {
        let result = assess_joint_robust_action(
            budget(20_000, 100_000),
            &intervals(20_000, 50),
            capacities(),
        )
        .expect("robust action assessment");
        assert_eq!(result.allocated_joint_alpha_ppm, 100_000);
        assert_eq!(result.joint_coverage_lower_bound_ppm, Some(900_000));
        assert_eq!(result.gate, DecisionGate::Feasible);
        assert_eq!(result.worst_case_berth_start, Some(105));
        assert_eq!(result.worst_case_finish, Some(117));
        assert_eq!(result.constraints.len(), 4);
    }

    #[test]
    fn five_ten_percent_marginals_do_not_claim_ninety_percent_joint_coverage() {
        let result = assess_joint_robust_action(
            budget(100_000, 100_000),
            &intervals(100_000, 50),
            capacities(),
        )
        .expect("invalid total risk is a gate result");
        assert_eq!(result.allocated_joint_alpha_ppm, 500_000);
        assert_eq!(result.gate, DecisionGate::Indeterminate);
    }

    #[test]
    fn interval_alpha_must_match_the_predeclared_allocation() {
        let mut values = intervals(20_000, 50);
        values.crane_minutes = Some(interval(30, 5, 10_000, 50));
        let result = assess_joint_robust_action(
            budget(20_000, 100_000),
            &values,
            capacities(),
        )
        .expect("risk mismatch is a gate result");
        assert_eq!(result.gate, DecisionGate::Indeterminate);
    }

    #[test]
    fn missing_or_shifted_intervals_never_pass() {
        let mut missing = intervals(20_000, 50);
        missing.yard_slots = None;
        assert_eq!(
            assess_joint_robust_action(budget(20_000, 100_000), &missing, capacities())
                .expect("missing interval result")
                .gate,
            DecisionGate::Indeterminate
        );

        let mut shifted = intervals(20_000, 50);
        shifted.arrival.as_mut().expect("arrival").coverage_claim =
            CoverageClaim::NoFormalGuaranteeKnownDistributionShift;
        shifted.arrival.as_mut().expect("arrival").nominal_coverage_claim_ppm = None;
        assert_eq!(
            assess_joint_robust_action(budget(20_000, 100_000), &shifted, capacities())
                .expect("shifted interval result")
                .gate,
            DecisionGate::Indeterminate
        );
    }

    #[test]
    fn mismatched_decision_cutoffs_are_rejected_as_indeterminate() {
        let mut values = intervals(20_000, 50);
        values.shore_power_kw.as_mut().expect("shore power").decision_cutoff = 51;
        assert_eq!(
            assess_joint_robust_action(budget(20_000, 100_000), &values, capacities())
                .expect("cutoff mismatch result")
                .gate,
            DecisionGate::Indeterminate
        );
    }

    #[test]
    fn time_and_resource_upper_bounds_are_enforced() {
        let mut caps = capacities();
        caps.crane_minutes_available = 34;
        let result = assess_joint_robust_action(
            budget(20_000, 100_000),
            &intervals(20_000, 50),
            caps,
        )
        .expect("crane constraint result");
        assert_eq!(result.gate, DecisionGate::Infeasible);
        assert!(result.constraints.iter().any(|check| {
            check.constraint == "quay_crane_minutes"
                && check.gate == DecisionGate::Infeasible
        }));

        let mut caps = capacities();
        caps.receiver_deadline = 116;
        let result = assess_joint_robust_action(
            budget(20_000, 100_000),
            &intervals(20_000, 50),
            caps,
        )
        .expect("deadline constraint result");
        assert_eq!(result.gate, DecisionGate::Infeasible);
    }

    #[test]
    fn invalid_risk_budget_or_berth_window_is_rejected() {
        let invalid_budget = budget(0, 100_000);
        assert!(
            assess_joint_robust_action(invalid_budget, &intervals(0, 50), capacities()).is_err()
        );

        let mut caps = capacities();
        caps.berth_slot_start = 131;
        assert!(
            assess_joint_robust_action(
                budget(20_000, 100_000),
                &intervals(20_000, 50),
                caps
            )
            .is_err()
        );
    }
}
