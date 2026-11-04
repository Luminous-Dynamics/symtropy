// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! CIVDYN-SWF-001 exact allocation and rhetoric-robustness sweep.

use civdyn_welfare_allocation::{
    AllocationPolicy, AllocationProblem, Recipient, allocate, pareto_frontier,
};

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

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), 16, "every CSV record has 16 columns");
    println!("{}", fields.join(","));
}

fn emit_summary(
    record_type: &str,
    label: &str,
    result: &civdyn_welfare_allocation::AllocationResult,
    frontier_size: usize,
) {
    emit(vec![
        record_type.into(),
        label.into(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        result.total_allocated.to_string(),
        fmt(result.total_verified_return),
        fmt(result.total_asserted_return),
        fmt(result.minimum_coverage),
        fmt(result.gini_coverage),
        fmt(result.coverage_variance),
        result.essential_floor_violations.to_string(),
        frontier_size.to_string(),
        String::new(),
    ]);
}

fn emit_allocations(
    label: &str,
    problem: &AllocationProblem,
    result: &civdyn_welfare_allocation::AllocationResult,
) {
    for (recipient, amount) in problem.recipients.iter().zip(&result.allocations) {
        let index = problem
            .recipients
            .iter()
            .position(|candidate| candidate.id == recipient.id)
            .expect("recipient exists");
        emit(vec![
            "allocation".into(),
            label.into(),
            recipient.id.clone(),
            amount.to_string(),
            fmt(result.coverage[index]),
            fmt(recipient.verified_return_per_unit),
            fmt(recipient.asserted_return_per_unit),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ]);
    }
}

fn main() {
    println!(
        "record_type,policy,recipient_id,allocation,coverage,verified_return,asserted_return,total_allocated,total_verified_return,total_asserted_return,min_coverage,gini_coverage,coverage_variance,floor_violations,frontier_size,decision_invariant"
    );

    let base = problem();
    base.validate()
        .expect("synthetic allocation fixture must validate");
    let frontier = pareto_frontier(&base).expect("exact frontier must enumerate");
    let frontier_size = frontier.len();

    for policy in AllocationPolicy::ALL {
        let result = allocate(&base, policy).expect("allocation policy must return a result");
        emit_summary("summary", policy.name(), &result, frontier_size);
        emit_allocations(policy.name(), &base, &result);
    }

    for (index, point) in frontier.iter().enumerate() {
        emit_summary("pareto", &format!("frontier_{index}"), point, frontier_size);
    }

    let mut manipulated = problem();
    manipulated.recipients[0].asserted_return_per_unit = 50.0;
    manipulated.recipients[1].asserted_return_per_unit = 0.1;
    let verified_base = allocate(&base, AllocationPolicy::VerifiedUtilitarian).unwrap();
    let verified_after_rhetoric =
        allocate(&manipulated, AllocationPolicy::VerifiedUtilitarian).unwrap();
    let susceptible = allocate(
        &manipulated,
        AllocationPolicy::PersuasionSusceptibleUtilitarian,
    )
    .unwrap();

    let invariant = verified_base.allocations == verified_after_rhetoric.allocations;
    emit(vec![
        "rhetoric_control".into(),
        "verified_invariance".into(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        fmt(verified_after_rhetoric.total_verified_return),
        String::new(),
        fmt(verified_after_rhetoric.minimum_coverage),
        fmt(verified_after_rhetoric.gini_coverage),
        fmt(verified_after_rhetoric.coverage_variance),
        verified_after_rhetoric
            .essential_floor_violations
            .to_string(),
        frontier_size.to_string(),
        invariant.to_string(),
    ]);
    emit_summary(
        "rhetoric_control",
        "persuasion_susceptible",
        &susceptible,
        frontier_size,
    );
    emit_allocations("persuasion_susceptible", &manipulated, &susceptible);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_allocation_is_invariant_to_rhetorical_claim_changes() {
        let base = problem();
        let mut manipulated = problem();
        manipulated.recipients[0].asserted_return_per_unit = 50.0;
        manipulated.recipients[1].asserted_return_per_unit = 0.1;
        let before = allocate(&base, AllocationPolicy::VerifiedUtilitarian).unwrap();
        let after = allocate(&manipulated, AllocationPolicy::VerifiedUtilitarian).unwrap();
        assert_eq!(before.allocations, after.allocations);
    }

    #[test]
    fn persuasion_susceptible_control_moves_to_the_claimed_high_return() {
        let mut manipulated = problem();
        manipulated.recipients[0].asserted_return_per_unit = 50.0;
        manipulated.recipients[1].asserted_return_per_unit = 0.1;
        let verified = allocate(&manipulated, AllocationPolicy::VerifiedUtilitarian).unwrap();
        let susceptible = allocate(
            &manipulated,
            AllocationPolicy::PersuasionSusceptibleUtilitarian,
        )
        .unwrap();
        assert_ne!(verified.allocations, susceptible.allocations);
    }

    #[test]
    fn hard_floor_control_is_disclosed_as_a_negative_control() {
        let result = allocate(&problem(), AllocationPolicy::FloorBlindUtilitarian).unwrap();
        assert!(result.essential_floor_violations > 0);
    }

    #[test]
    fn frontier_contains_multiple_nondominated_points() {
        let result = pareto_frontier(&problem()).unwrap();
        assert!(result.len() >= 2);
    }
}
