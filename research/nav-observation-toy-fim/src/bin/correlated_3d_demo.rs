use nav_observation_toy_fim::correlated_3d::{
    correlated_fisher_3d, exhaustive_select_correlated_3d, CorrelatedRangeAction3D,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let independent = vec![
        CorrelatedRangeAction3D::independent("east-a", [1.0, 0.0, 0.0], 1.0, 1),
        CorrelatedRangeAction3D::independent("east-b", [1.0, 0.0, 0.0], 1.0, 1),
    ];
    let shared = vec![
        CorrelatedRangeAction3D::with_common_mode("east-a", [1.0, 0.0, 0.0], 1.0, "clock-front-end", 1.0, 1),
        CorrelatedRangeAction3D::with_common_mode("east-b", [1.0, 0.0, 0.0], 1.0, "clock-front-end", 1.0, 1),
    ];
    let independent_fim = correlated_fisher_3d(&independent)?;
    let shared_fim = correlated_fisher_3d(&shared)?;

    let candidates = vec![
        CorrelatedRangeAction3D::with_common_mode("east-a", [1.0, 0.0, 0.0], 1.0, "clock-front-end", 0.8, 1),
        CorrelatedRangeAction3D::with_common_mode("east-b", [1.0, 0.0, 0.0], 1.0, "clock-front-end", 0.8, 1),
        CorrelatedRangeAction3D::independent("north", [0.0, 1.0, 0.0], 1.0, 1),
        CorrelatedRangeAction3D::independent("up", [0.0, 0.0, 1.0], 1.0, 1),
    ];
    let selection = exhaustive_select_correlated_3d(&candidates, 3)?;

    println!("CORRELATED_3D_OBSERVABILITY");
    println!(
        "  duplicate_east_independent_Jxx={:.9}, shared_Jxx={:.9}",
        independent_fim.matrix[0][0],
        shared_fim.matrix[0][0]
    );
    println!(
        "  selected={:?}, rank={}, eigenvalues={:?}",
        selection.action_ids,
        selection.rank,
        selection.eigenvalues
    );
    Ok(())
}
