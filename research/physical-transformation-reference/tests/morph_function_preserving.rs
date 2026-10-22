use physical_transformation_reference::planar_truss::{
    solve, PlanarTrussInput, PlanarTrussResult, TrussBar, TrussNode,
};

const AREA_M2: f64 = 1.0e-4;
const YOUNGS_MODULUS_PA: f64 = 200.0e9;
const LOAD_N: f64 = 1_000.0;
const ANALYTIC_DISPLACEMENT_M: f64 = 5.0e-5;
const SERVICEABILITY_LIMIT_M: f64 = 1.0e-4;

fn node(
    id: &str,
    x_m: f64,
    y_m: f64,
    supported: bool,
    force_y_n: f64,
) -> TrussNode {
    TrussNode {
        id: id.to_owned(),
        x_m,
        y_m,
        restrained_x: supported,
        restrained_y: supported,
        force_x_n: 0.0,
        force_y_n,
    }
}

fn bar(id: &str, node_i: &str, node_j: &str) -> TrussBar {
    TrussBar {
        id: id.to_owned(),
        node_i: node_i.to_owned(),
        node_j: node_j.to_owned(),
        area_m2: AREA_M2,
        youngs_modulus_pa: YOUNGS_MODULUS_PA,
    }
}

/// Same four node locations and loads in each case; only the diagonal topology changes.
/// A square with one diagonal has a constrained in-plane load path; without a diagonal,
/// the top chord can translate laterally as a mechanism.
fn square_truss(diagonal: Option<(&str, &str)>) -> PlanarTrussInput {
    let nodes = vec![
        node("A", 0.0, 0.0, true, 0.0),
        node("B", 1.0, 0.0, true, 0.0),
        node("C", 0.0, 1.0, false, -LOAD_N),
        node("D", 1.0, 1.0, false, -LOAD_N),
    ];
    let mut bars = vec![
        bar("AB", "A", "B"),
        bar("AC", "A", "C"),
        bar("BD", "B", "D"),
        bar("CD", "C", "D"),
    ];
    if let Some((node_i, node_j)) = diagonal {
        bars.push(bar(&format!("{node_i}{node_j}"), node_i, node_j));
    }
    PlanarTrussInput { nodes, bars }
}

fn close_to(actual: f64, expected: f64, tolerance: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected:.12e}, got {actual:.12e}"
    );
}

fn displacement(result: &PlanarTrussResult, node_id: &str) -> (f64, f64) {
    let node = result
        .node_results
        .iter()
        .find(|node| node.node_id == node_id)
        .expect("fixture node must have a solved result");
    (node.displacement_x_m, node.displacement_y_m)
}

fn assert_functionally_acceptable(result: &PlanarTrussResult) {
    for id in ["C", "D"] {
        let (ux, uy) = displacement(result, id);
        assert!(ux.abs() <= SERVICEABILITY_LIMIT_M, "{id} ux={ux}");
        assert!(uy.abs() <= SERVICEABILITY_LIMIT_M, "{id} uy={uy}");
        close_to(
            uy,
            -ANALYTIC_DISPLACEMENT_M,
            1.0e-12,
            &format!("{id} analytical vertical displacement"),
        );
    }
    assert!(
        result.equilibrium.normalized_force_residual < 1.0e-12,
        "force equilibrium residual: {}",
        result.equilibrium.normalized_force_residual
    );
    assert!(
        result.equilibrium.normalized_moment_residual < 1.0e-12,
        "moment equilibrium residual: {}",
        result.equilibrium.normalized_moment_residual
    );
    assert!(
        result.equilibrium.normalized_free_dof_residual < 1.0e-12,
        "free-DOF residual: {}",
        result.equilibrium.normalized_free_dof_residual
    );
}

/// With E*A = 20 MN and a 1 m vertical bar, k = E*A/L = 20 MN/m.
/// The two equal 1 kN vertical loads therefore give -50 um at C_y and D_y.
/// Both diagonal orientations preserve the same vertical result and remain below
/// the declared 100 um serviceability threshold. Removing the diagonal leaves a
/// lateral mechanism, even though every node and every visible coordinate is unchanged.
#[test]
fn geometry_preserving_diagonal_exchange_is_functionally_valid_but_deletion_is_a_mechanism() {
    let diagonal_ad = square_truss(Some(("A", "D")));
    let diagonal_bc = square_truss(Some(("B", "C")));
    let no_diagonal = square_truss(None);

    assert_eq!(diagonal_ad.nodes, diagonal_bc.nodes);
    assert_eq!(diagonal_ad.nodes, no_diagonal.nodes);
    assert_ne!(diagonal_ad.bars, diagonal_bc.bars);
    assert_eq!(diagonal_ad.bars.len(), diagonal_bc.bars.len());
    assert_eq!(diagonal_ad.bars.len(), no_diagonal.bars.len() + 1);

    let ad_result = solve(&diagonal_ad).expect("reference diagonal should stabilize the truss");
    let bc_result = solve(&diagonal_bc).expect("alternate diagonal should stabilize the truss");
    assert_functionally_acceptable(&ad_result);
    assert_functionally_acceptable(&bc_result);

    let (ad_cx, _) = displacement(&ad_result, "C");
    let (bc_cx, _) = displacement(&bc_result, "C");
    close_to(ad_cx, ANALYTIC_DISPLACEMENT_M, 1.0e-12, "A-D diagonal Cx");
    close_to(bc_cx, -ANALYTIC_DISPLACEMENT_M, 1.0e-12, "B-C diagonal Cx");

    let error = solve(&no_diagonal).expect_err(
        "the square without a diagonal has a lateral mechanism and must not receive a displacement result",
    );
    assert!(
        error.contains("singular or numerically unresolved truss stiffness"),
        "expected explicit mechanism rejection, got: {error}"
    );
}
