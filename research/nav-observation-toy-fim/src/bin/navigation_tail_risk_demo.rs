use nav_observation_toy_fim::navigation_tail_risk::{
    cvar, expected_loss, worst_case_cvar, LossMass, ModelLossDistribution,
};

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

    println!("claim_class=synthetic_exact_discrete_tail_fixture");
    println!("loss_unit=0.01");
    println!("policy_a_expected_loss={}/{}", a_mean.numerator(), a_mean.denominator());
    println!("policy_a_cvar_95={}/{}", a_tail.numerator(), a_tail.denominator());
    println!("policy_b_expected_loss={}/{}", b_mean.numerator(), b_mean.denominator());
    println!("policy_b_cvar_95={}/{}", b_tail.numerator(), b_tail.denominator());
    println!("expected_loss_prefers=policy_a");
    println!("cvar_95_prefers=policy_b");
    println!("persistent_model_worst_case_id={}", robust.worst_case_model_id);
    println!(
        "persistent_model_worst_case_cvar_95={}/{}",
        robust.worst_case_cvar.numerator(),
        robust.worst_case_cvar.denominator()
    );
    println!("authority_transition_not_authorized=true");
    Ok(())
}
