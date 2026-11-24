use nav_observation_toy_fim::navigation_temporal_mission_minimax::{
    plan_temporal_mission_minimax, OutcomeLikelihoods, ResourceVector, StateTransition,
    TemporalMinimaxError, TemporalMinimaxPlans, TemporalMinimaxProblem, TimedJointAction,
    MAX_TERMINAL_LOSS, PROBABILITY_SCALE,
};

fn likelihoods(no: u64, zero: u64, one: u64) -> OutcomeLikelihoods {
    OutcomeLikelihoods {
        no_observation_ppm: no,
        observed_zero_ppm: zero,
        observed_one_ppm: one,
    }
}

fn identity() -> StateTransition {
    StateTransition {
        inside_to_outside_ppm: 0,
        outside_to_inside_ppm: 0,
    }
}

fn action(id: u64, age_ns: u64, inside: OutcomeLikelihoods, outside: OutcomeLikelihoods) -> TimedJointAction {
    let likelihood_rows = [inside, outside, inside, outside];
    TimedJointAction {
        action_id: id,
        cost: ResourceVector {
            latency_ns: 100,
            energy_uj: 1,
            bandwidth_bytes: 1,
        },
        evidence_age_ns: age_ns,
        likelihoods: [likelihood_rows; 2],
        transitions: [[identity(); 2]; 2],
        outcome_model_sha256: [[[0x31; 32]; 2]; 2],
        transition_model_sha256: [[[0x42; 32]; 2]; 2],
    }
}

fn build_plans() -> Result<TemporalMinimaxPlans, TemporalMinimaxError> {
    let problem = TemporalMinimaxProblem {
        prior_by_model_ppm: [
            [250_000, 250_000, 250_000, 250_000],
            [250_000, 250_000, 250_000, 250_000],
        ],
        false_safe_loss: 1,
        false_degraded_loss: 1,
    };

    // Action 1 is perfect at capture but the first persistent model allows
    // half of the inside state to leave the safe envelope before evidence use.
    let perfect_inside = likelihoods(0, PROBABILITY_SCALE, 0);
    let perfect_outside = likelihoods(0, 0, PROBABILITY_SCALE);
    let mut stale_perfect = action(1, 100, perfect_inside, perfect_outside);
    stale_perfect.transitions[0] = [
        StateTransition {
            inside_to_outside_ppm: 500_000,
            outside_to_inside_ppm: 0,
        },
        StateTransition {
            inside_to_outside_ppm: 500_000,
            outside_to_inside_ppm: 0,
        },
    ];

    // Action 2 has symmetric ten-percent observation error and zero evidence age.
    let fresh_inside = likelihoods(0, 900_000, 100_000);
    let fresh_outside = likelihoods(0, 100_000, 900_000);
    let fresh_noisy = action(2, 0, fresh_inside, fresh_outside);

    plan_temporal_mission_minimax(
        &problem,
        &[stale_perfect, fresh_noisy],
        ResourceVector {
            latency_ns: 100,
            energy_uj: 10,
            bandwidth_bytes: 10,
        },
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plans = build_plans()?;
    let robust = &plans.mission_global_minimax_risk;
    assert_eq!(robust.first_action_id, Some(2), "fresh noisier evidence must win");
    assert_eq!(robust.risk_denominator, 1_000_000_000_000_000_000_000_000_000_000_u128);

    println!("claim_class=synthetic_exact_fixture_only");
    println!("action_1=perfect_at_capture; evidence_age_ns=100; model_0_inside_to_outside_ppm=500000");
    println!("action_2=10_percent_observation_error; evidence_age_ns=0");
    println!("selected_mission_global_minimax_action={}", robust.first_action_id.unwrap_or(0));
    println!("model_0_risk={}/{}", robust.model_loss_numerators[0], robust.risk_denominator);
    println!("model_1_risk={}/{}", robust.model_loss_numerators[1], robust.risk_denominator);
    println!("worst_case_risk={}/{}", robust.worst_case_risk_numerator, robust.risk_denominator);
    println!("model_aware_oracle_risks={:?}", plans.model_aware_oracle_risk_numerators);
    println!("minimax_regret_numerator={}", plans.mission_global_minimax_regret.maximum_model_regret_numerator);
    println!("policy_trees_evaluated={}", plans.policy_trees_evaluated);
    println!("terminal_loss_cap={MAX_TERMINAL_LOSS}");
    println!("authority_transition_not_authorized={}", plans.authority_transition_not_authorized_by_plan);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_demo_selects_fresh_sensor_at_use_epoch() {
        let plans = build_plans().unwrap();
        assert_eq!(plans.mission_global_minimax_risk.first_action_id, Some(2));
        assert_eq!(
            plans.mission_global_minimax_risk.risk_denominator,
            1_000_000_000_000_000_000_000_000_000_000_u128
        );
    }

    #[test]
    fn executable_demo_reports_exact_model_risks_oracles_and_enumeration() {
        let plans = build_plans().unwrap();
        let selected = &plans.mission_global_minimax_risk;
        let expected = 100_000_000_000_000_000_000_000_000_000_u128;
        assert_eq!(selected.model_loss_numerators, [expected, expected]);
        assert_eq!(selected.worst_case_risk_numerator, expected);
        assert_eq!(plans.model_aware_oracle_risk_numerators, [expected, 0]);
        assert_eq!(
            plans.mission_global_minimax_regret.maximum_model_regret_numerator,
            expected
        );
        assert_eq!(plans.policy_trees_evaluated, 18);
    }
}
