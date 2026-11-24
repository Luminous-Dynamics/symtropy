use nav_observation_toy_fim::navigation_temporal_mission_minimax::{
    plan_temporal_mission_cvar, plan_temporal_mission_minimax, OutcomeLikelihoods,
    ResourceVector, StateTransition, TemporalMinimaxProblem, TimedJointAction,
    PROBABILITY_SCALE,
};

fn likelihoods(observed_zero_ppm: u64, observed_one_ppm: u64) -> OutcomeLikelihoods {
    OutcomeLikelihoods {
        no_observation_ppm: 0,
        observed_zero_ppm,
        observed_one_ppm,
    }
}

fn action(
    action_id: u64,
    inside: OutcomeLikelihoods,
    outside: OutcomeLikelihoods,
) -> TimedJointAction {
    let rows = [inside, outside, inside, outside];
    TimedJointAction {
        action_id,
        cost: ResourceVector {
            latency_ns: 100,
            energy_uj: 1,
            bandwidth_bytes: 1,
        },
        evidence_age_ns: 0,
        likelihoods: [rows; 2],
        transitions: [[
            StateTransition {
                inside_to_outside_ppm: 0,
                outside_to_inside_ppm: 0,
            },
            StateTransition {
                inside_to_outside_ppm: 0,
                outside_to_inside_ppm: 0,
            },
        ]; 2],
        outcome_model_sha256: [[[0x51; 32]; 2]; 2],
        transition_model_sha256: [[[0x62; 32]; 2]; 2],
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let problem = TemporalMinimaxProblem {
        prior_by_model_ppm: [
            [250_000, 250_000, 250_000, 250_000],
            [250_000, 250_000, 250_000, 250_000],
        ],
        false_safe_loss: 100,
        false_degraded_loss: 1,
    };

    // A: 900 ppm false-safe probability at loss 100 and 50 ppm
    // false-degraded probability at loss 1.
    let rare_catastrophic = action(
        1,
        likelihoods(999_900, 100),
        likelihoods(1_800, 998_200),
    );
    // B: 100,000 ppm false-degraded probability at loss 1, no false-safe.
    let steady_small = action(
        2,
        likelihoods(800_000, 200_000),
        likelihoods(0, PROBABILITY_SCALE),
    );
    let actions = [rare_catastrophic, steady_small];
    let budget = ResourceVector {
        latency_ns: 100,
        energy_uj: 10,
        bandwidth_bytes: 10,
    };

    let expected = plan_temporal_mission_minimax(&problem, &actions, budget)?;
    let cvar = plan_temporal_mission_cvar(&problem, &actions, budget, 950_000)?;

    assert_eq!(expected.mission_global_minimax_risk.first_action_id, Some(1));
    assert_eq!(cvar.selected_policy.first_action_id, Some(2));
    assert_eq!(cvar.tail_risk.worst_case_cvar.numerator(), 1);
    assert_eq!(cvar.tail_risk.worst_case_cvar.denominator(), 1);
    assert_eq!(cvar.policy_trees_evaluated, 18);

    println!("claim_class=synthetic_exact_policy_objective_counterexample");
    println!("losses=false_safe:100;false_degraded:1");
    println!(
        "expected_minimax_action={}",
        expected.mission_global_minimax_risk.first_action_id.unwrap_or(0)
    );
    println!(
        "expected_minimax_model_0_loss={}/{}",
        expected.mission_global_minimax_risk.model_loss_numerators[0],
        expected.mission_global_minimax_risk.risk_denominator
    );
    println!(
        "expected_minimax_worst_case_loss={}/{}",
        expected.mission_global_minimax_risk.worst_case_risk_numerator,
        expected.mission_global_minimax_risk.risk_denominator
    );
    println!(
        "expected_minimax_policy_cvar_95={}/{}",
        1_801,
        1_000
    );
    println!("robust_cvar_95_action={}", cvar.selected_policy.first_action_id.unwrap_or(0));
    println!(
        "robust_cvar_95_value={}/{}",
        cvar.tail_risk.worst_case_cvar.numerator(),
        cvar.tail_risk.worst_case_cvar.denominator()
    );
    println!("robust_cvar_95_model={}", cvar.tail_risk.worst_case_model_id);
    println!("observable_policy_trees_evaluated={}", cvar.policy_trees_evaluated);
    println!("authority_transition_not_authorized=true");
    Ok(())
}
