use nav_observation_toy_fim::{exhaustive_select, greedy_select, RangeAction};

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
    println!("  condition_number={:?}", exact.condition_number);

    println!("GREEDY_BASELINE");
    println!("  selected={:?}", greedy.action_ids);
    println!("  cost_units={}", greedy.total_cost_units);
    println!("  rank={}", greedy.rank);
    println!("  determinant={:.12}", greedy.fisher.determinant());
    println!("  condition_number={:?}", greedy.condition_number);

    Ok(())
}
