use nav_observation_toy_fim::{
    exhaustive_select, exhaustive_select_with_criterion, greedy_select, DesignCriterion, RangeAction,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let degrees = |value: f64| value.to_radians();
    let actions = vec![
        RangeAction::new("a-east", degrees(0.0), 1.0, 1),
        RangeAction::new("b-east-near", degrees(10.0), 1.0, 1),
        RangeAction::new("c-north-costly", degrees(90.0), 1.0, 2),
        RangeAction::new("d-diagonal-45", degrees(45.0), 1.0, 1),
        RangeAction::new("e-antidiagonal-135", degrees(135.0), 1.0, 1),
        RangeAction::new("f-west", degrees(180.0), 1.0, 1),
    ];

    let exact = exhaustive_select(&actions, 2)?;
    let greedy = greedy_select(&actions, 2)?;

    println!("EXHAUSTIVE_ORACLE");
    println!("  selected={:?}", exact.action_ids);
    println!("  cost_units={}", exact.total_cost_units);
    println!("  rank={}", exact.rank);
    println!("  determinant={:.12}", exact.fisher.determinant());
    println!("  minimum_eigenvalue={:.12}", exact.minimum_eigenvalue);
    println!("  A_opt_trace_inverse={:?}", exact.a_optimal_trace_inverse);
    println!("  condition_number={:?}", exact.condition_number);

    println!("GREEDY_BASELINE");
    println!("  selected={:?}", greedy.action_ids);
    println!("  cost_units={}", greedy.total_cost_units);
    println!("  rank={}", greedy.rank);
    println!("  determinant={:.12}", greedy.fisher.determinant());
    println!("  minimum_eigenvalue={:.12}", greedy.minimum_eigenvalue);
    println!("  A_opt_trace_inverse={:?}", greedy.a_optimal_trace_inverse);
    println!("  condition_number={:?}", greedy.condition_number);

    let criterion_actions = vec![
        RangeAction::new("a-east-precise", degrees(0.0), 0.1, 1),
        RangeAction::new("b-north-weak", degrees(90.0), 1.0, 1),
        RangeAction::new("c-diagonal", degrees(45.0), 1.0 / 10.0_f64.sqrt(), 1),
        RangeAction::new("d-antidiagonal", degrees(135.0), 1.0 / 9.0_f64.sqrt(), 1),
    ];

    println!("CRITERION_TRADEOFFS");
    for criterion in [
        DesignCriterion::DOptimal,
        DesignCriterion::AOptimal,
        DesignCriterion::EOptimal,
    ] {
        let selected = exhaustive_select_with_criterion(&criterion_actions, 2, criterion)?;
        println!(
            "  {:?}: selected={:?}, det={:.6}, min_eigen={:.6}, A_opt={:?}, condition={:?}",
            criterion,
            selected.action_ids,
            selected.fisher.determinant(),
            selected.minimum_eigenvalue,
            selected.a_optimal_trace_inverse,
            selected.condition_number
        );
    }

    Ok(())
}
