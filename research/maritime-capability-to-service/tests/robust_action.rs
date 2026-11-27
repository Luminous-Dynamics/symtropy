use maritime_capability_to_service::conformal::{ConformalInterval, CoverageClaim};
use maritime_capability_to_service::robust_action::{
    JointPredictionIntervals, JointRiskBudget, ResourceCapacities, assess_joint_robust_action,
};
use maritime_capability_to_service::conformal::DecisionGate;

fn interval(point: u64, radius: u64, alpha: u32) -> ConformalInterval {
    ConformalInterval {
        point_prediction: point,
        lower: point.saturating_sub(radius),
        upper: point + radius,
        radius,
        alpha_ppm: alpha,
        sample_count: 100,
        quantile_rank_1based: (u128::from(101u64) * u128::from(1_000_000 - alpha))
            .div_ceil(1_000_000) as u64,
        coverage_claim: CoverageClaim::MarginalSplitConformalUnderDeclaredExchangeability,
        nominal_coverage_claim_ppm: Some(1_000_000 - alpha),
        model_fit_cutoff: 10,
        calibration_window_start: 11,
        calibration_window_end: 40,
        model_sha256: "a".repeat(64),
        feature_schema_sha256: "b".repeat(64),
        calibration_data_sha256: "c".repeat(64),
        decision_cutoff: 50,
    }
}

fn budget(alpha: u32, target: u32) -> JointRiskBudget {
    JointRiskBudget {
        target_joint_alpha_ppm: target,
        arrival_alpha_ppm: alpha,
        handling_alpha_ppm: alpha,
        crane_minutes_alpha_ppm: alpha,
        shore_power_alpha_ppm: alpha,
        yard_slots_alpha_ppm: alpha,
    }
}

fn intervals(alpha: u32) -> JointPredictionIntervals {
    JointPredictionIntervals {
        arrival: Some(interval(100, 5, alpha)),
        handling_duration: Some(interval(10, 2, alpha)),
        crane_minutes: Some(interval(30, 5, alpha)),
        shore_power_kw: Some(interval(100, 10, alpha)),
        yard_slots: Some(interval(20, 2, alpha)),
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
fn public_joint_action_gate_accepts_five_two_percent_intervals_under_ten_percent_budget() {
    let result = assess_joint_robust_action(
        budget(20_000, 100_000),
        &intervals(20_000),
        capacities(),
    )
    .expect("robust action");
    assert_eq!(result.gate, DecisionGate::Feasible);
    assert_eq!(result.allocated_joint_alpha_ppm, 100_000);
    assert_eq!(result.worst_case_finish, Some(117));
}

#[test]
fn public_joint_action_gate_refuses_five_ninety_percent_marginals_as_a_joint_ninety_percent_claim() {
    let result = assess_joint_robust_action(
        budget(100_000, 100_000),
        &intervals(100_000),
        capacities(),
    )
    .expect("over-budget assessment");
    assert_eq!(result.gate, DecisionGate::Indeterminate);
}
