use nav_observation_toy_fim::DesignCriterion;

use nav_observation_toy_fim::correlated::{
    correlated_fisher, exhaustive_select_correlated, greedy_select_correlated,
    CorrelatedRangeAction,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let independent = vec![
        CorrelatedRangeAction::independent("receiver-a", 0.0, 1.0, 1),
        CorrelatedRangeAction::independent("receiver-b", 0.0, 1.0, 1),
    ];
    let shared = vec![
        CorrelatedRangeAction::with_common_mode(
            "receiver-a", 0.0, 1.0, "shared-clock-antenna", 1.0, 1,
        ),
        CorrelatedRangeAction::with_common_mode(
            "receiver-b", 0.0, 1.0, "shared-clock-antenna", 1.0, 1,
        ),
    ];

    let naive = correlated_fisher(&independent)?;
    let correlation_aware = correlated_fisher(&shared)?;

    println!("SAME_BEARING_DUPLICATE_MEASUREMENTS");
    println!("  naive_independent_information_xx={:.9}", naive.xx);
    println!("  explicit_shared_error_information_xx={:.9}", correlation_aware.xx);
    println!(
        "  naive_information_overstatement_factor={:.6}",
        naive.xx / correlation_aware.xx
    );

    let candidates = vec![
        CorrelatedRangeAction::with_common_mode(
            "a-colocated-1", 0.0, 1.0, "shared-front-end", 1.0, 1,
        ),
        CorrelatedRangeAction::with_common_mode(
            "b-colocated-2", 0.0, 1.0, "shared-front-end", 1.0, 1,
        ),
        CorrelatedRangeAction::independent(
            "c-north", std::f64::consts::FRAC_PI_2, 1.0, 1,
        ),
        CorrelatedRangeAction::independent(
            "d-diagonal", std::f64::consts::FRAC_PI_4, 1.0, 1,
        ),
    ];
    let exact = exhaustive_select_correlated(&candidates, 2, DesignCriterion::DOptimal)?;
    let greedy = greedy_select_correlated(&candidates, 2, DesignCriterion::DOptimal)?;

    println!("CORRELATION_AWARE_SELECTION");
    println!(
        "  exhaustive={:?}, det={:.9}, cost={}",
        exact.action_ids,
        exact.fisher.determinant(),
        exact.total_cost_units
    );
    println!(
        "  greedy={:?}, det={:.9}, cost={}",
        greedy.action_ids,
        greedy.fisher.determinant(),
        greedy.total_cost_units
    );
    Ok(())
}
