// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact metrics for fairness across a sequence of allocation ticks.
//!
//! This crate evaluates allocation traces; it is not an allocator, source/cap
//! feasibility checker, or proof of optimality. Per-tick demand must be explicit.

use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSeries {
    pub id: u64,
    pub demand: Vec<u64>,
    pub eligible: Vec<bool>,
    pub allocated: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllocationTrace {
    pub case_id: String,
    pub agents: Vec<AgentSeries>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fraction {
    numerator: u128,
    denominator: u128,
}

impl Fraction {
    pub fn new(numerator: u128, denominator: u128) -> Self {
        assert!(denominator > 0, "fraction denominator must be positive");
        let divisor = gcd(numerator, denominator);
        Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        }
    }

    pub fn numerator(&self) -> u128 {
        self.numerator
    }

    pub fn denominator(&self) -> u128 {
        self.denominator
    }

    pub fn compare(&self, other: &Self) -> Ordering {
        fraction_cmp(
            self.numerator,
            self.denominator,
            other.numerator,
            other.denominator,
        )
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Compare nonnegative rational numbers without multiplication, so ordering
/// cannot overflow even for large u128 numerators and denominators.
fn fraction_cmp(
    mut left_num: u128,
    mut left_den: u128,
    mut right_num: u128,
    mut right_den: u128,
) -> Ordering {
    let mut reversed = false;
    loop {
        let left_whole = left_num / left_den;
        let right_whole = right_num / right_den;
        if left_whole != right_whole {
            let order = left_whole.cmp(&right_whole);
            return if reversed { order.reverse() } else { order };
        }

        let left_remainder = left_num % left_den;
        let right_remainder = right_num % right_den;
        match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if reversed { Ordering::Greater } else { Ordering::Less };
            }
            (false, true) => {
                return if reversed { Ordering::Less } else { Ordering::Greater };
            }
            (false, false) => {
                left_num = left_den;
                left_den = left_remainder;
                right_num = right_den;
                right_den = right_remainder;
                reversed = !reversed;
            }
        }
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Violation {
    pub code: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentTemporalMetrics {
    pub agent_id: u64,
    pub total_demand: u128,
    pub total_allocated: u128,
    pub total_unmet_demand: u128,
    pub total_eligible_demand: u128,
    pub total_eligible_allocated: u128,
    pub total_eligible_unmet_demand: u128,
    pub horizon_service_ratio: Option<Fraction>,
    pub eligible_demand_ticks: u64,
    pub starved_ticks: u64,
    pub maximum_consecutive_starvation_ticks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalReport {
    /// True means the trace obeyed this evaluator's input contract only.
    pub valid_trace: bool,
    pub case_id: String,
    pub tick_count: usize,
    pub violations: Vec<Violation>,
    pub per_agent: Vec<AgentTemporalMetrics>,
    /// Sorted low-to-high cumulative service/demand fractions; zero-demand
    /// agents are excluded instead of being assigned an undefined 0/0 ratio.
    pub sorted_horizon_service_profile: Vec<Fraction>,
    /// Lowest allocation/demand ratio among eligible, positive-demand agent-tick
    /// pairs. This is a worst-case diagnostic, not a universal fairness score.
    pub worst_eligible_slot_service_ratio: Option<Fraction>,
    pub minimum_horizon_service_ratio: Option<Fraction>,
    pub maximum_horizon_service_ratio: Option<Fraction>,
}

fn violation(violations: &mut Vec<Violation>, code: &str, detail: impl Into<String>) {
    violations.push(Violation {
        code: code.to_owned(),
        detail: detail.into(),
    });
}

fn checked_total(values: &[u64], id: u64, label: &str, violations: &mut Vec<Violation>) -> u128 {
    let mut total = 0_u128;
    for value in values {
        match total.checked_add(u128::from(*value)) {
            Some(sum) => total = sum,
            None => {
                violation(
                    violations,
                    "total_overflow",
                    format!("agent {id} {label} total exceeds u128"),
                );
                return u128::MAX;
            }
        }
    }
    total
}

/// Evaluate a complete set of per-agent time series.
///
/// An eligible demand tick is starved precisely when demand is positive and
/// allocation is zero. Ineligible or zero-demand ticks break starvation runs.
/// Allocation above demand and allocation while ineligible are invalid inputs.
pub fn evaluate(trace: &AllocationTrace) -> TemporalReport {
    let tick_count = trace.agents.first().map_or(0, |agent| agent.demand.len());
    let mut violations = Vec::new();
    let mut ids = BTreeSet::new();

    for agent in &trace.agents {
        if !ids.insert(agent.id) {
            violation(
                &mut violations,
                "duplicate_agent_id",
                format!("agent ID {} appears more than once", agent.id),
            );
        }
        if agent.demand.len() != agent.eligible.len()
            || agent.demand.len() != agent.allocated.len()
        {
            violation(
                &mut violations,
                "series_length_mismatch",
                format!(
                    "agent {} has demand/eligibility/allocation lengths {}/{}/{}",
                    agent.id,
                    agent.demand.len(),
                    agent.eligible.len(),
                    agent.allocated.len()
                ),
            );
        }
    }

    // ID ambiguity and jagged traces are rejected before indexing any series.
    if violations.iter().any(|item| {
        item.code == "duplicate_agent_id" || item.code == "series_length_mismatch"
    }) {
        violations.sort_by(|left, right| {
            (&left.code, &left.detail).cmp(&(&right.code, &right.detail))
        });
        return TemporalReport {
            valid_trace: false,
            case_id: trace.case_id.clone(),
            tick_count,
            violations,
            per_agent: Vec::new(),
            sorted_horizon_service_profile: Vec::new(),
            worst_eligible_slot_service_ratio: None,
            minimum_horizon_service_ratio: None,
            maximum_horizon_service_ratio: None,
        };
    }

    let mut per_agent = Vec::with_capacity(trace.agents.len());
    let mut profile = Vec::new();
    let mut worst_slot_ratio: Option<Fraction> = None;

    for agent in &trace.agents {
        let total_demand = checked_total(&agent.demand, agent.id, "demand", &mut violations);
        let total_allocated =
            checked_total(&agent.allocated, agent.id, "allocation", &mut violations);
        let total_unmet_demand = agent
            .demand
            .iter()
            .zip(&agent.allocated)
            .map(|(demand, allocation)| u128::from(demand.saturating_sub(*allocation)))
            .sum();
        let eligible_demand_values: Vec<u64> = agent
            .demand
            .iter()
            .zip(&agent.eligible)
            .filter_map(|(demand, eligible)| (*eligible).then_some(*demand))
            .collect();
        let eligible_allocation_values: Vec<u64> = agent
            .allocated
            .iter()
            .zip(&agent.eligible)
            .filter_map(|(allocation, eligible)| (*eligible).then_some(*allocation))
            .collect();
        let total_eligible_demand = checked_total(
            &eligible_demand_values,
            agent.id,
            "eligible demand",
            &mut violations,
        );
        let total_eligible_allocated = checked_total(
            &eligible_allocation_values,
            agent.id,
            "eligible allocation",
            &mut violations,
        );
        let total_eligible_unmet_demand: u128 = eligible_demand_values
            .iter()
            .zip(&eligible_allocation_values)
            .map(|(demand, allocation)| u128::from(demand.saturating_sub(*allocation)))
            .sum();

        let mut eligible_demand_ticks = 0_u64;
        let mut starved_ticks = 0_u64;
        let mut current_streak = 0_u64;
        let mut maximum_streak = 0_u64;

        for tick in 0..tick_count {
            let demand = agent.demand[tick];
            let allocated = agent.allocated[tick];
            let eligible = agent.eligible[tick];

            if allocated > demand {
                violation(
                    &mut violations,
                    "allocation_exceeds_demand",
                    format!(
                        "agent {} tick {} receives {} against demand {}",
                        agent.id, tick, allocated, demand
                    ),
                );
            }
            if !eligible && allocated > 0 {
                violation(
                    &mut violations,
                    "ineligible_allocation",
                    format!(
                        "agent {} receives {} at tick {} while ineligible",
                        agent.id, allocated, tick
                    ),
                );
            }

            if eligible && demand > 0 {
                eligible_demand_ticks += 1;
                if allocated == 0 {
                    starved_ticks += 1;
                    current_streak += 1;
                    maximum_streak = maximum_streak.max(current_streak);
                } else {
                    current_streak = 0;
                }

                let ratio = Fraction::new(u128::from(allocated), u128::from(demand));
                if worst_slot_ratio.as_ref().is_none_or(|previous| ratio.cmp(previous).is_lt()) {
                    worst_slot_ratio = Some(ratio);
                }
            } else {
                current_streak = 0;
            }
        }

        let horizon_ratio = (total_eligible_demand > 0)
            .then(|| Fraction::new(total_eligible_allocated, total_eligible_demand));
        if let Some(ratio) = &horizon_ratio {
            profile.push(ratio.clone());
        }
        per_agent.push(AgentTemporalMetrics {
            agent_id: agent.id,
            total_demand,
            total_allocated,
            total_unmet_demand,
            total_eligible_demand,
            total_eligible_allocated,
            total_eligible_unmet_demand,
            horizon_service_ratio: horizon_ratio,
            eligible_demand_ticks,
            starved_ticks,
            maximum_consecutive_starvation_ticks: maximum_streak,
        });
    }

    profile.sort();
    let minimum_horizon_service_ratio = profile.first().cloned();
    let maximum_horizon_service_ratio = profile.last().cloned();

    per_agent.sort_by_key(|metrics| metrics.agent_id);
    violations.sort_by(|left, right| {
        (&left.code, &left.detail).cmp(&(&right.code, &right.detail))
    });

    TemporalReport {
        valid_trace: violations.is_empty(),
        case_id: trace.case_id.clone(),
        tick_count,
        violations,
        per_agent,
        sorted_horizon_service_profile: profile,
        worst_eligible_slot_service_ratio: worst_slot_ratio,
        minimum_horizon_service_ratio,
        maximum_horizon_service_ratio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(id: u64, demand: &[u64], eligible: &[bool], allocated: &[u64]) -> AgentSeries {
        AgentSeries {
            id,
            demand: demand.to_vec(),
            eligible: eligible.to_vec(),
            allocated: allocated.to_vec(),
        }
    }

    #[test]
    fn alternation_prevents_horizon_starvation() {
        let trace = AllocationTrace {
            case_id: "alternating".to_owned(),
            agents: vec![
                series(1, &[1, 1], &[true, true], &[1, 0]),
                series(2, &[1, 1], &[true, true], &[0, 1]),
            ],
        };
        let report = evaluate(&trace);
        assert!(report.valid_trace, "{:?}", report.violations);
        assert_eq!(
            report.sorted_horizon_service_profile,
            vec![Fraction::new(1, 2), Fraction::new(1, 2)]
        );
        assert_eq!(report.per_agent[0].maximum_consecutive_starvation_ticks, 1);
        assert_eq!(report.per_agent[1].maximum_consecutive_starvation_ticks, 1);
    }

    #[test]
    fn same_per_tick_minimum_can_hide_horizon_monopoly() {
        let trace = AllocationTrace {
            case_id: "monopoly".to_owned(),
            agents: vec![
                series(1, &[1, 1], &[true, true], &[1, 1]),
                series(2, &[1, 1], &[true, true], &[0, 0]),
            ],
        };
        let report = evaluate(&trace);
        assert!(report.valid_trace, "{:?}", report.violations);
        assert_eq!(
            report.sorted_horizon_service_profile,
            vec![Fraction::new(0, 1), Fraction::new(1, 1)]
        );
        assert_eq!(report.per_agent[1].starved_ticks, 2);
        assert_eq!(report.per_agent[1].maximum_consecutive_starvation_ticks, 2);
    }

    #[test]
    fn ineligible_ticks_break_starvation_streaks() {
        let trace = AllocationTrace {
            case_id: "ineligibility-breaks-streak".to_owned(),
            agents: vec![series(7, &[1, 5, 1], &[true, false, true], &[0, 0, 1])],
        };
        let report = evaluate(&trace);
        assert!(report.valid_trace, "{:?}", report.violations);
        assert_eq!(report.per_agent[0].eligible_demand_ticks, 2);
        assert_eq!(report.per_agent[0].starved_ticks, 1);
        assert_eq!(report.per_agent[0].maximum_consecutive_starvation_ticks, 1);
        assert_eq!(report.per_agent[0].total_demand, 7);
        assert_eq!(report.per_agent[0].total_eligible_demand, 2);
        assert_eq!(report.per_agent[0].total_eligible_allocated, 1);
        assert_eq!(report.per_agent[0].total_eligible_unmet_demand, 1);
        assert_eq!(
            report.per_agent[0].horizon_service_ratio,
            Some(Fraction::new(1, 2))
        );
    }

    #[test]
    fn rejects_allocation_over_demand_and_ineligible_credit() {
        let trace = AllocationTrace {
            case_id: "invalid-trace".to_owned(),
            agents: vec![series(1, &[1, 0], &[true, false], &[2, 1])],
        };
        let report = evaluate(&trace);
        assert!(!report.valid_trace);
        assert!(report.violations.iter().any(|item| item.code == "allocation_exceeds_demand"));
        assert!(report.violations.iter().any(|item| item.code == "ineligible_allocation"));
    }

    #[test]
    fn permutations_preserve_canonical_report() {
        let first = AllocationTrace {
            case_id: "same-trace".to_owned(),
            agents: vec![
                series(2, &[1, 1], &[true, true], &[0, 1]),
                series(1, &[1, 1], &[true, true], &[1, 0]),
            ],
        };
        let mut second = first.clone();
        second.agents.reverse();
        assert_eq!(evaluate(&first), evaluate(&second));
    }

    #[test]
    fn zero_demand_agents_are_excluded_from_horizon_profile() {
        let trace = AllocationTrace {
            case_id: "zero-demand".to_owned(),
            agents: vec![
                series(1, &[0, 0], &[true, true], &[0, 0]),
                series(2, &[1, 1], &[true, true], &[1, 0]),
            ],
        };
        let report = evaluate(&trace);
        assert!(report.valid_trace, "{:?}", report.violations);
        assert_eq!(report.sorted_horizon_service_profile.len(), 1);
        assert_eq!(report.sorted_horizon_service_profile[0], Fraction::new(1, 2));
    }

    #[test]
    fn rational_ordering_does_not_depend_on_multiplication() {
        let left = Fraction {
            numerator: u128::MAX - 1,
            denominator: u128::MAX,
        };
        let right = Fraction {
            numerator: u128::MAX - 2,
            denominator: u128::MAX - 1,
        };
        assert_eq!(left.cmp(&right), Ordering::Greater);
        assert_eq!(Fraction::new(2, 3).cmp(&Fraction::new(4, 6)), Ordering::Equal);
    }

    #[test]
    fn empty_trace_is_well_defined() {
        let report = evaluate(&AllocationTrace {
            case_id: "empty".to_owned(),
            agents: Vec::new(),
        });
        assert!(report.valid_trace);
        assert_eq!(report.tick_count, 0);
        assert!(report.sorted_horizon_service_profile.is_empty());
    }
}
