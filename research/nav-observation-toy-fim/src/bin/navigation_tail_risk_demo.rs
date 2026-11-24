use nav_observation_toy_fim::navigation_tail_risk::{
    cvar, expected_loss, mission_path_loss_distributions, worst_case_cvar, ExactRisk,
    LossMass, ModelLossDistribution,
};
use nav_observation_toy_fim::navigation_temporal_mission_minimax::{
    plan_temporal_mission_minimax, OutcomeLikelihoods, ResourceVector, StateTransition,
    TemporalMinimaxProblem, TimedJointAction, PROBABILITY_SCALE,
};

fn nav_action(
    action_id: u64,
    evidence_age_ns: u64,
    latency_ns: u64,
    inside_error_ppm: u64,
    outside_error_ppm: u64,
) -> TimedJointAction {
    let inside = OutcomeLikelihoods {
        no_observation_ppm: 0,
        observed_zero_ppm: PROBABILITY_SCALE - inside_error_ppm,
        observed_one_ppm: inside_error_ppm,
    };
    let outside = OutcomeLikelihoods {
        no_observation_ppm: 0,
        observed_zero_ppm: outside_error_ppm,
        observed_one_ppm: PROBABILITY_SCALE - outside_error_ppm,
    };
    TimedJointAction {
        action_id,
        cost: ResourceVector {
            latency_ns,
            energy_uj: 1,
            bandwidth_bytes: 1,
        },
        evidence_age_ns,
        likelihoods: [[inside, outside, inside, outside]; 2],
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
        outcome_model_sha256: [[[0x31; 32]; 2]; 2],
        transition_model_sha256: [[[0x42; 32]; 2]; 2],
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Loss units are hundredths. These are exact synthetic distributions.
    // Policy A: 1% chance of loss 1.00, otherwise zero.
    let policy_a = [
        LossMass { loss: 100, mass: 1 },
        LossMass { loss: 0, mass: 99 },
    ];
    // Policy B: constant loss 0.02.
    let policy_b = [LossMass { loss: 2, mass: 100 }];

    let a_mean = expected_loss(&policy_a)?;
    let a_tail = cvar(&policy_a, 950_000)?;
    let b_mean = expected_loss(&policy_b)?;
    let b_tail = cvar(&policy_b, 950_000)?;

    assert_eq!((a_mean.numerator(), a_mean.denominator()), (1, 1));
    assert_eq!((a_tail.numerator(), a_tail.denominator()), (20, 1));
    assert_eq!((b_mean.numerator(), b_mean.denominator()), (2, 1));
    assert_eq!((b_tail.numerator(), b_tail.denominator()), (2, 1));

    let benign = [LossMass { loss: 2, mass: 100 }];
    let robust = worst_case_cvar(
        &[
            ModelLossDistribution { model_id: 1, outcomes: &benign },
            ModelLossDistribution { model_id: 2, outcomes: &policy_a },
        ],
        950_000,
    )?;

    // End-to-end connection: reconstruct a fixed minimax policy's complete
    // loss distribution for each persistent mission model, then evaluate tail
    // risk over those actual finite policy-tree paths.
    let problem = TemporalMinimaxProblem {
        prior_by_model_ppm: [
            [250_000, 250_000, 250_000, 250_000],
            [250_000, 250_000, 250_000, 250_000],
        ],
        false_safe_loss: 1,
        false_degraded_loss: 1,
    };
    let actions = [
        nav_action(1, 100, 100, 50_000, 50_000),
        nav_action(2, 0, 100, 100_000, 100_000),
    ];
    let plans = plan_temporal_mission_minimax(
        &problem,
        &actions,
        ResourceVector {
            latency_ns: 200,
            energy_uj: 10,
            bandwidth_bytes: 10,
        },
    )?;
    let selected = &plans.mission_global_minimax_risk;
    let path_distributions = mission_path_loss_distributions(&problem, &actions, selected)?;

    for (model, distribution) in path_distributions.iter().enumerate() {
        let reconstructed_mean = expected_loss(distribution)?;
        let oracle_mean = ExactRisk::from_ratio(
            selected.model_loss_numerators[model],
            selected.risk_denominator,
        )?;
        assert_eq!(reconstructed_mean, oracle_mean);
    }

    let mission_tail = worst_case_cvar(
        &[
            ModelLossDistribution {
                model_id: 0,
                outcomes: &path_distributions[0],
            },
            ModelLossDistribution {
                model_id: 1,
                outcomes: &path_distributions[1],
            },
        ],
        950_000,
    )?;

    println!("claim_class=synthetic_exact_discrete_tail_fixture");
    println!("loss_unit=0.01");
    println!("policy_a_expected_loss={}/{}", a_mean.numerator(), a_mean.denominator());
    println!("policy_a_cvar_95={}/{}", a_tail.numerator(), a_tail.denominator());
    println!("policy_b_expected_loss={}/{}", b_mean.numerator(), b_mean.denominator());
    println!("policy_b_cvar_95={}/{}", b_tail.numerator(), b_tail.denominator());
    println!("expected_loss_prefers=policy_a");
    println!("cvar_95_prefers=policy_b");
    println!("synthetic_persistent_model_worst_case_id={}", robust.worst_case_model_id);
    println!(
        "synthetic_persistent_model_worst_case_cvar_95={}/{}",
        robust.worst_case_cvar.numerator(),
        robust.worst_case_cvar.denominator()
    );
    println!("temporal_policy_selected_first_action={:?}", selected.first_action_id);
    println!(
        "temporal_policy_model_0_expected_loss={}/{}",
        expected_loss(&path_distributions[0])?.numerator(),
        expected_loss(&path_distributions[0])?.denominator()
    );
    println!(
        "temporal_policy_model_1_expected_loss={}/{}",
        expected_loss(&path_distributions[1])?.numerator(),
        expected_loss(&path_distributions[1])?.denominator()
    );
    println!("temporal_policy_worst_case_cvar_95={}/{}", 
        mission_tail.worst_case_cvar.numerator(),
        mission_tail.worst_case_cvar.denominator()
    );
    println!("authority_transition_not_authorized=true");
    Ok(())
}
