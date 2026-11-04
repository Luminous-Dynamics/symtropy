// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Exact small-instance resource allocation reference.
//!
//! The benchmark separates verified returns, unverified persuasive claims,
//! distributional outcomes and hard essential-service floors. It is a
//! synthetic decision benchmark, not a normative definition of social welfare.

use core::cmp::Ordering;
use core::fmt;
use std::collections::BTreeSet;

pub const MAX_RECIPIENTS: usize = 8;
pub const MAX_ENUMERATED_ALLOCATIONS: usize = 200_000;

#[derive(Debug, Clone, PartialEq)]
pub struct Recipient {
    pub id: String,
    pub essential_floor: u32,
    pub useful_capacity: u32,
    /// Verified marginal return for each allocated unit.
    pub verified_return_per_unit: f64,
    /// Unverified persuasive assertion. Robust policies must ignore this field.
    pub asserted_return_per_unit: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AllocationProblem {
    pub budget: u32,
    pub recipients: Vec<Recipient>,
}

impl AllocationProblem {
    pub fn validate(&self) -> Result<(), AllocationError> {
        if self.recipients.len() < 2 || self.recipients.len() > MAX_RECIPIENTS {
            return Err(AllocationError::InvalidRecipientCount(self.recipients.len()));
        }
        let mut ids = BTreeSet::new();
        let mut minimum_total = 0_u32;
        let mut capacity_total = 0_u32;
        for recipient in &self.recipients {
            if recipient.id.trim().is_empty() {
                return Err(AllocationError::EmptyRecipientId);
            }
            if !ids.insert(recipient.id.clone()) {
                return Err(AllocationError::DuplicateRecipientId(recipient.id.clone()));
            }
            if recipient.useful_capacity == 0 {
                return Err(AllocationError::ZeroUsefulCapacity {
                    recipient: recipient.id.clone(),
                });
            }
            if recipient.essential_floor > recipient.useful_capacity {
                return Err(AllocationError::FloorExceedsCapacity {
                    recipient: recipient.id.clone(),
                });
            }
            for (field, value) in [
                ("verified_return_per_unit", recipient.verified_return_per_unit),
                ("asserted_return_per_unit", recipient.asserted_return_per_unit),
            ] {
                if !value.is_finite() || value < 0.0 {
                    return Err(AllocationError::InvalidReturn {
                        recipient: recipient.id.clone(),
                        field,
                        value,
                    });
                }
            }
            minimum_total = minimum_total
                .checked_add(recipient.essential_floor)
                .ok_or(AllocationError::ArithmeticOverflow)?;
            capacity_total = capacity_total
                .checked_add(recipient.useful_capacity)
                .ok_or(AllocationError::ArithmeticOverflow)?;
        }

        if minimum_total > self.budget {
            return Err(AllocationError::InfeasibleEssentialFloors {
                minimum_total,
                budget: self.budget,
            });
        }
        if capacity_total < self.budget {
            return Err(AllocationError::BudgetExceedsUsefulCapacity {
                budget: self.budget,
                capacity_total,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationPolicy {
    VerifiedUtilitarian,
    MaxMinCoverage,
    LeximinCoverage,
    EqualCoverage,
    /// Deliberately vulnerable control: uses unverified asserted returns.
    PersuasionSusceptibleUtilitarian,
    /// Deliberately unsafe negative control: ignores essential floors.
    FloorBlindUtilitarian,
}

impl AllocationPolicy {
    pub const ALL: [Self; 6] = [
        Self::VerifiedUtilitarian,
        Self::MaxMinCoverage,
        Self::LeximinCoverage,
        Self::EqualCoverage,
        Self::PersuasionSusceptibleUtilitarian,
        Self::FloorBlindUtilitarian,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::VerifiedUtilitarian => "verified_utilitarian",
            Self::MaxMinCoverage => "max_min_coverage",
            Self::LeximinCoverage => "leximin_coverage",
            Self::EqualCoverage => "equal_coverage",
            Self::PersuasionSusceptibleUtilitarian => "persuasion_susceptible",
            Self::FloorBlindUtilitarian => "floor_blind_utilitarian",
        }
    }

    fn respects_floors(self) -> bool {
        self != Self::FloorBlindUtilitarian
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AllocationResult {
    /// Allocation amounts in the same order as problem.recipients.
    pub allocations: Vec<u32>,
    pub coverage: Vec<f64>,
    pub total_allocated: u32,
    pub total_verified_return: f64,
    pub total_asserted_return: f64,
    pub minimum_coverage: f64,
    pub gini_coverage: f64,
    pub coverage_variance: f64,
    pub essential_floor_violations: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AllocationError {
    InvalidRecipientCount(usize),
    EmptyRecipientId,
    DuplicateRecipientId(String),
    FloorExceedsCapacity { recipient: String },
    ZeroUsefulCapacity { recipient: String },
    InvalidReturn {
        recipient: String,
        field: &'static str,
        value: f64,
    },
    InfeasibleEssentialFloors { minimum_total: u32, budget: u32 },
    BudgetExceedsUsefulCapacity { budget: u32, capacity_total: u32 },
    ArithmeticOverflow,
    SearchBudgetExceeded { limit: usize },
    InvalidAllocationLength { expected: usize, actual: usize },
    AllocationExceedsCapacity { recipient: String },
    AllocationDoesNotExhaustBudget { budget: u32, allocated: u32 },
}

impl fmt::Display for AllocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRecipientCount(count) => write!(
                f,
                "recipient count must be between 2 and {MAX_RECIPIENTS}, got {count}"
            ),
            Self::EmptyRecipientId => write!(f, "recipient ID must not be empty"),
            Self::DuplicateRecipientId(id) => write!(f, "duplicate recipient ID: {id}"),
            Self::FloorExceedsCapacity { recipient } => {
                write!(f, "essential floor exceeds useful capacity for {recipient}")
            }
            Self::ZeroUsefulCapacity { recipient } => {
                write!(f, "useful capacity must be positive for {recipient}")
            }
            Self::InvalidReturn { recipient, field, value } => {
                write!(f, "{recipient}.{field} must be finite and non-negative, got {value}")
            }
            Self::InfeasibleEssentialFloors { minimum_total, budget } => write!(
                f,
                "essential floors require {minimum_total} units, exceeding budget {budget}"
            ),
            Self::BudgetExceedsUsefulCapacity { budget, capacity_total } => write!(
                f,
                "budget {budget} exceeds total useful capacity {capacity_total}"
            ),
            Self::ArithmeticOverflow => write!(f, "allocation arithmetic overflow"),
            Self::SearchBudgetExceeded { limit } => {
                write!(f, "exact allocation enumeration exceeded {limit} candidates")
            }
            Self::InvalidAllocationLength { expected, actual } => {
                write!(f, "expected {expected} allocations, got {actual}")
            }
            Self::AllocationExceedsCapacity { recipient } => {
                write!(f, "allocation exceeds useful capacity for {recipient}")
            }
            Self::AllocationDoesNotExhaustBudget { budget, allocated } => {
                write!(f, "allocation uses {allocated} of budget {budget}")
            }
        }
    }
}

impl std::error::Error for AllocationError {}

pub fn evaluate_allocation(
    problem: &AllocationProblem,
    allocations: &[u32],
) -> Result<AllocationResult, AllocationError> {
    problem.validate()?;
    if allocations.len() != problem.recipients.len() {
        return Err(AllocationError::InvalidAllocationLength {
            expected: problem.recipients.len(),
            actual: allocations.len(),
        });
    }

    let mut total_allocated = 0_u32;
    let mut total_verified_return = 0.0;
    let mut total_asserted_return = 0.0;
    let mut coverage = Vec::with_capacity(allocations.len());
    let mut essential_floor_violations = 0_usize;

    for (recipient, amount) in problem.recipients.iter().zip(allocations) {
        if *amount > recipient.useful_capacity {
            return Err(AllocationError::AllocationExceedsCapacity {
                recipient: recipient.id.clone(),
            });
        }
        total_allocated = total_allocated
            .checked_add(*amount)
            .ok_or(AllocationError::ArithmeticOverflow)?;
        total_verified_return += f64::from(*amount) * recipient.verified_return_per_unit;
        total_asserted_return += f64::from(*amount) * recipient.asserted_return_per_unit;
        coverage.push(*amount as f64 / recipient.useful_capacity as f64);
        if *amount < recipient.essential_floor {
            essential_floor_violations += 1;
        }
    }

    if total_allocated != problem.budget {
        return Err(AllocationError::AllocationDoesNotExhaustBudget {
            budget: problem.budget,
            allocated: total_allocated,
        });
    }

    let minimum_coverage = coverage.iter().copied().fold(f64::INFINITY, f64::min);
    let mean = coverage.iter().sum::<f64>() / coverage.len() as f64;
    let coverage_variance = coverage
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / coverage.len() as f64;
    let gini_coverage = gini(&coverage);

    Ok(AllocationResult {
        allocations: allocations.to_vec(),
        coverage,
        total_allocated,
        total_verified_return,
        total_asserted_return,
        minimum_coverage,
        gini_coverage,
        coverage_variance,
        essential_floor_violations,
    })
}

pub fn allocate(
    problem: &AllocationProblem,
    policy: AllocationPolicy,
) -> Result<AllocationResult, AllocationError> {
    problem.validate()?;
    let candidates = enumerate_allocations(problem, policy.respects_floors())?;
    let mut best: Option<AllocationResult> = None;
    for candidate in candidates {
        let replace = match &best {
            None => true,
            Some(current) => compare_candidates(problem, policy, &candidate, current)
                == Ordering::Greater,
        };
        if replace {
            best = Some(candidate);
        }
    }
    Ok(best.expect("validated allocation problem has at least one feasible allocation"))
}

pub fn pareto_frontier(
    problem: &AllocationProblem,
) -> Result<Vec<AllocationResult>, AllocationError> {
    problem.validate()?;
    let mut candidates = enumerate_allocations(problem, true)?;
    candidates.sort_by(|left, right| {
        right
            .total_verified_return
            .total_cmp(&left.total_verified_return)
            .then_with(|| right.minimum_coverage.total_cmp(&left.minimum_coverage))
            .then_with(|| canonical_key(problem, &left.allocations).cmp(&canonical_key(problem, &right.allocations)))
    });

    let mut frontier = Vec::new();
    let mut best_minimum_coverage = f64::NEG_INFINITY;
    for candidate in candidates {
        if candidate.minimum_coverage > best_minimum_coverage + 1e-12 {
            best_minimum_coverage = candidate.minimum_coverage;
            frontier.push(candidate);
        }
    }
    Ok(frontier)
}

fn enumerate_allocations(
    problem: &AllocationProblem,
    respect_floors: bool,
) -> Result<Vec<AllocationResult>, AllocationError> {
    let mut results = Vec::new();
    let mut current = vec![0_u32; problem.recipients.len()];
    enumerate_recursive(
        problem,
        respect_floors,
        0,
        problem.budget,
        &mut current,
        &mut results,
    )?;
    Ok(results)
}

fn enumerate_recursive(
    problem: &AllocationProblem,
    respect_floors: bool,
    index: usize,
    remaining: u32,
    current: &mut [u32],
    results: &mut Vec<AllocationResult>,
) -> Result<(), AllocationError> {
    if results.len() > MAX_ENUMERATED_ALLOCATIONS {
        return Err(AllocationError::SearchBudgetExceeded {
            limit: MAX_ENUMERATED_ALLOCATIONS,
        });
    }
    if index + 1 == problem.recipients.len() {
        let recipient = &problem.recipients[index];
        let floor = if respect_floors { recipient.essential_floor } else { 0 };
        if remaining < floor || remaining > recipient.useful_capacity {
            return Ok(());
        }
        current[index] = remaining;
        results.push(evaluate_allocation(problem, current)?);
        return Ok(());
    }

    let recipient = &problem.recipients[index];
    let floor = if respect_floors { recipient.essential_floor } else { 0 };
    let remaining_capacity_after = problem.recipients[index + 1..]
        .iter()
        .map(|next| next.useful_capacity)
        .sum::<u32>();
    let max_for_current = recipient.useful_capacity.min(remaining.saturating_sub(
        problem.recipients[index + 1..]
            .iter()
            .map(|next| if respect_floors { next.essential_floor } else { 0 })
            .sum::<u32>(),
    ));

    if remaining_capacity_after == 0 && remaining > max_for_current {
        return Ok(());
    }
    if max_for_current < floor {
        return Ok(());
    }

    for amount in floor..=max_for_current {
        current[index] = amount;
        enumerate_recursive(
            problem,
            respect_floors,
            index + 1,
            remaining - amount,
            current,
            results,
        )?;
    }
    Ok(())
}

fn compare_candidates(
    problem: &AllocationProblem,
    policy: AllocationPolicy,
    left: &AllocationResult,
    right: &AllocationResult,
) -> Ordering {
    let primary = match policy {
        AllocationPolicy::VerifiedUtilitarian | AllocationPolicy::FloorBlindUtilitarian => {
            left.total_verified_return.total_cmp(&right.total_verified_return)
        }
        AllocationPolicy::MaxMinCoverage => {
            left.minimum_coverage.total_cmp(&right.minimum_coverage)
        }
        AllocationPolicy::LeximinCoverage => {
            compare_leximin(&left.coverage, &right.coverage)
        }
        AllocationPolicy::EqualCoverage => {
            right.coverage_variance.total_cmp(&left.coverage_variance)
        }
        AllocationPolicy::PersuasionSusceptibleUtilitarian => {
            left.total_asserted_return.total_cmp(&right.total_asserted_return)
        }
    };
    if primary != Ordering::Equal {
        return primary;
    }

    let verified = left.total_verified_return.total_cmp(&right.total_verified_return);
    if verified != Ordering::Equal {
        return verified;
    }
    let min_coverage = left.minimum_coverage.total_cmp(&right.minimum_coverage);
    if min_coverage != Ordering::Equal {
        return min_coverage;
    }
    let gini = right.gini_coverage.total_cmp(&left.gini_coverage);
    if gini != Ordering::Equal {
        return gini;
    }

    // Canonical ID-ordered tie-break: the result does not depend on input order.
    canonical_key(problem, &right.allocations).cmp(&canonical_key(problem, &left.allocations))
}

fn compare_leximin(left: &[f64], right: &[f64]) -> Ordering {
    let mut left_sorted = left.to_vec();
    let mut right_sorted = right.to_vec();
    left_sorted.sort_by(f64::total_cmp);
    right_sorted.sort_by(f64::total_cmp);
    for (left_value, right_value) in left_sorted.iter().zip(&right_sorted) {
        let comparison = left_value.total_cmp(right_value);
        if comparison != Ordering::Equal {
            return comparison;
        }
    }
    Ordering::Equal
}

fn canonical_key(problem: &AllocationProblem, allocations: &[u32]) -> Vec<(String, u32)> {
    let mut pairs = problem
        .recipients
        .iter()
        .zip(allocations)
        .map(|(recipient, amount)| (recipient.id.clone(), *amount))
        .collect::<Vec<_>>();
    pairs.sort_by(|left, right| left.0.cmp(&right.0));
    pairs
}

fn gini(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let total = sorted.iter().sum::<f64>();
    if total <= f64::EPSILON {
        return 0.0;
    }
    let weighted = sorted
        .iter()
        .enumerate()
        .map(|(index, value)| (index + 1) as f64 * value)
        .sum::<f64>();
    (2.0 * weighted / (sorted.len() as f64 * total)
        - (sorted.len() as f64 + 1.0) / sorted.len() as f64)
        .clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem() -> AllocationProblem {
        AllocationProblem {
            budget: 16,
            recipients: vec![
                Recipient {
                    id: "A".into(),
                    essential_floor: 2,
                    useful_capacity: 8,
                    verified_return_per_unit: 1.0,
                    asserted_return_per_unit: 1.0,
                },
                Recipient {
                    id: "B".into(),
                    essential_floor: 2,
                    useful_capacity: 8,
                    verified_return_per_unit: 5.0,
                    asserted_return_per_unit: 5.0,
                },
                Recipient {
                    id: "C".into(),
                    essential_floor: 2,
                    useful_capacity: 8,
                    verified_return_per_unit: 4.0,
                    asserted_return_per_unit: 4.0,
                },
                Recipient {
                    id: "D".into(),
                    essential_floor: 2,
                    useful_capacity: 8,
                    verified_return_per_unit: 2.0,
                    asserted_return_per_unit: 2.0,
                },
            ],
        }
    }

    fn allocations_by_id(problem: &AllocationProblem, result: &AllocationResult) -> Vec<(String, u32)> {
        canonical_key(problem, &result.allocations)
    }

    #[test]
    fn verified_utilitarian_honours_every_essential_floor() {
        let problem = problem();
        let result = allocate(&problem, AllocationPolicy::VerifiedUtilitarian).unwrap();
        assert_eq!(result.essential_floor_violations, 0);
        assert_eq!(result.total_allocated, problem.budget);
        for (recipient, amount) in problem.recipients.iter().zip(&result.allocations) {
            assert!(*amount >= recipient.essential_floor);
        }
    }

    #[test]
    fn max_min_and_leximin_protect_lower_coverage() {
        let problem = problem();
        let utilitarian = allocate(&problem, AllocationPolicy::VerifiedUtilitarian).unwrap();
        let max_min = allocate(&problem, AllocationPolicy::MaxMinCoverage).unwrap();
        let leximin = allocate(&problem, AllocationPolicy::LeximinCoverage).unwrap();

        assert!(max_min.minimum_coverage >= utilitarian.minimum_coverage);
        assert!(leximin.minimum_coverage >= utilitarian.minimum_coverage);
        assert!((max_min.minimum_coverage - 0.5).abs() < 1e-12);
        assert!((leximin.minimum_coverage - 0.5).abs() < 1e-12);
    }

    #[test]
    fn rhetoric_only_change_cannot_move_verified_facts_allocation() {
        let original = problem();
        let mut persuaded = problem();
        persuaded.recipients[0].asserted_return_per_unit = 50.0;
        persuaded.recipients[1].asserted_return_per_unit = 0.1;

        let baseline = allocate(&original, AllocationPolicy::VerifiedUtilitarian).unwrap();
        let verified = allocate(&persuaded, AllocationPolicy::VerifiedUtilitarian).unwrap();
        let vulnerable = allocate(
            &persuaded,
            AllocationPolicy::PersuasionSusceptibleUtilitarian,
        )
        .unwrap();

        assert_eq!(baseline.allocations, verified.allocations);
        assert_ne!(verified.allocations, vulnerable.allocations);
        assert_eq!(verified.essential_floor_violations, 0);
    }

    #[test]
    fn floor_blind_negative_control_can_breach_the_essential_floor() {
        let mut problem = problem();
        problem.recipients[0].verified_return_per_unit = 0.0;
        problem.recipients[1].verified_return_per_unit = 10.0;
        let constrained = allocate(&problem, AllocationPolicy::VerifiedUtilitarian).unwrap();
        let unconstrained = allocate(&problem, AllocationPolicy::FloorBlindUtilitarian).unwrap();

        assert_eq!(constrained.essential_floor_violations, 0);
        assert!(unconstrained.essential_floor_violations > 0);
    }

    #[test]
    fn pareto_frontier_contains_productivity_equity_tradeoff() {
        let frontier = pareto_frontier(&problem()).unwrap();
        assert!(frontier.len() >= 2);
        assert!(frontier.iter().any(|point| point.gini_coverage < 1e-12));
        assert!(frontier.iter().any(|point| point.total_verified_return > 0.0));
    }

    #[test]
    fn recipient_order_does_not_change_verified_result() {
        let original = problem();
        let mut reversed = problem();
        reversed.recipients.reverse();

        let left = allocate(&original, AllocationPolicy::VerifiedUtilitarian).unwrap();
        let right = allocate(&reversed, AllocationPolicy::VerifiedUtilitarian).unwrap();
        assert_eq!(
            allocations_by_id(&original, &left),
            allocations_by_id(&reversed, &right)
        );
    }

    #[test]
    fn infeasible_essential_floors_fail_closed() {
        let mut problem = problem();
        problem.budget = 8;
        for recipient in &mut problem.recipients {
            recipient.essential_floor = 3;
        }
        assert!(matches!(
            problem.validate(),
            Err(AllocationError::InfeasibleEssentialFloors { .. })
        ));
    }

    #[test]
    fn invalid_and_duplicate_recipients_fail_closed() {
        let mut problem = problem();
        problem.recipients[1].id = problem.recipients[0].id.clone();
        assert!(matches!(
            problem.validate(),
            Err(AllocationError::DuplicateRecipientId(_))
        ));

        let mut invalid = problem();
        invalid.recipients[0].verified_return_per_unit = f64::NAN;
        assert!(matches!(
            invalid.validate(),
            Err(AllocationError::InvalidReturn { .. })
        ));
    }

    #[test]
    fn zero_useful_capacity_is_rejected_before_coverage_division() {
        let mut problem = problem();
        problem.recipients[0].useful_capacity = 0;
        problem.recipients[0].essential_floor = 0;
        assert_eq!(
            problem.validate(),
            Err(AllocationError::ZeroUsefulCapacity {
                recipient: "A".into(),
            })
        );
    }

    #[test]
    fn gini_of_equal_coverage_is_zero() {
        assert_eq!(gini(&[0.5, 0.5, 0.5, 0.5]), 0.0);
    }
}
