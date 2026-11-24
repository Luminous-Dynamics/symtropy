use nav_observation_toy_fim::navigation_tail_risk::{
    cvar, iterated_cvar, ConditionalLossBranch, ExactRisk, LossMass,
};
use nav_observation_toy_fim::navigation_temporal_mission_minimax::TerminalDecision;

fn branch(
    branch_id: u64,
    probability_mass: u128,
    continuation: &[(u64, u128)],
) -> ConditionalLossBranch {
    ConditionalLossBranch {
        branch_id,
        continuation_action_id: branch_id,
        terminal_decisions: [TerminalDecision::DeclareSafe; 3],
        probability_mass,
        continuation_losses: continuation
            .iter()
            .map(|(loss, mass)| LossMass {
                loss: *loss,
                mass: *mass,
            })
            .collect(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root_a = [
        LossMass { loss: 300, mass: 80 },
        LossMass { loss: 200, mass: 1 },
        LossMass { loss: 150, mass: 19 },
        LossMass { loss: 0, mass: 1_900 },
    ];
    let root_b = [
        LossMass { loss: 300, mass: 80 },
        LossMass { loss: 180, mass: 20 },
        LossMass { loss: 0, mass: 1_900 },
    ];

    let branches_a = [
        branch(1, 99, &[(300, 4), (0, 95)]),
        branch(2, 1, &[(200, 1), (150, 19)]),
    ];
    let branches_b = [
        branch(1, 99, &[(300, 4), (0, 95)]),
        branch(2, 1, &[(180, 20)]),
    ];

    let static_a = cvar(&root_a, 950_000)?;
    let static_b = cvar(&root_b, 950_000)?;
    let iterated_a = iterated_cvar(&branches_a, 50_000)?;
    let iterated_b = iterated_cvar(&branches_b, 50_000)?;

    assert_eq!(
        static_a,
        ExactRisk::from_ratio(541, 2)?
    );
    assert_eq!(
        static_b,
        ExactRisk::from_ratio(276, 1)?
    );
    assert_eq!(
        iterated_a.branch_risks[1].conditional_cvar,
        ExactRisk::from_ratio(200, 1)?
    );
    assert_eq!(
        iterated_b.branch_risks[1].conditional_cvar,
        ExactRisk::from_ratio(180, 1)?
    );
    assert_eq!(iterated_a.iterated_cvar, iterated_b.iterated_cvar);

    println!("claim_class=synthetic_exact_iterated_cvar_fixture");
    println!("tail_mass_ppm=50000");
    println!(
        "static_root_cvar95_action_a={}/{}",
        static_a.numerator(),
        static_a.denominator()
    );
    println!(
        "static_root_cvar95_action_b={}/{}",
        static_b.numerator(),
        static_b.denominator()
    );
    println!(
        "branch_h_conditional_cvar_action_a={}/{}",
        iterated_a.branch_risks[1].conditional_cvar.numerator(),
        iterated_a.branch_risks[1].conditional_cvar.denominator()
    );
    println!(
        "branch_h_conditional_cvar_action_b={}/{}",
        iterated_b.branch_risks[1].conditional_cvar.numerator(),
        iterated_b.branch_risks[1].conditional_cvar.denominator()
    );
    println!(
        "iterated_root_cvar={}/{}",
        iterated_a.iterated_cvar.numerator(),
        iterated_a.iterated_cvar.denominator()
    );
    println!("authority_transition_not_authorized=true");
    Ok(())
}
