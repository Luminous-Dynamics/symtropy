use nav_observation_toy_fim::observability_3d::{
    exhaustive_select_3d, DesignCriterion3D, RangeAction3D,
};

fn action(id: &str, direction: [f64; 3]) -> RangeAction3D {
    RangeAction3D::new(id, direction, 1.0, 1)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let planar = vec![
        action("east", [1.0, 0.0, 0.0]),
        action("north", [0.0, 1.0, 0.0]),
        action("diagonal", [std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2, 0.0]),
    ];
    let full_3d = vec![
        action("east", [1.0, 0.0, 0.0]),
        action("north", [0.0, 1.0, 0.0]),
        action("up", [0.0, 0.0, 1.0]),
        action("diagonal", [std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2, 0.0]),
    ];

    let planar_result = exhaustive_select_3d(&planar, 3, DesignCriterion3D::EOptimal)?;
    let full_result = exhaustive_select_3d(&full_3d, 3, DesignCriterion3D::EOptimal)?;

    println!("FULL_3D_OBSERVABILITY");
    println!(
        "  planar: rank={}, eigenvalues={:?}, condition={:?}",
        planar_result.rank,
        planar_result.eigenvalues,
        planar_result.condition_number
    );
    println!(
        "  3D: rank={}, eigenvalues={:?}, condition={:?}, selected={:?}",
        full_result.rank,
        full_result.eigenvalues,
        full_result.condition_number,
        full_result.action_ids
    );
    Ok(())
}
