use physical_transformation_reference::planar_truss::{
    solve, PlanarTrussInput, TrussBar, TrussNode,
};

const BASE_AREA_M2: f64 = 1.0e-4;
const BASE_YOUNGS_MODULUS_PA: f64 = 200.0e9;
const BASE_LOAD_N: f64 = 2_000.0;
const MAX_DISPLACEMENT_M: f64 = 120.0e-6;
const DISPLACEMENT_TOLERANCE_M: f64 = 2.0e-12;
const VARIATION_FACTORS: [f64; 3] = [0.95, 1.0, 1.05];

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

fn bar(
    id: &str,
    node_i: &str,
    node_j: &str,
    area_m2: f64,
    youngs_modulus_pa: f64,
) -> TrussBar {
    TrussBar {
        id: id.to_owned(),
        node_i: node_i.to_owned(),
        node_j: node_j.to_owned(),
        area_m2,
        youngs_modulus_pa,
    }
}

fn square_truss(
    diagonal: Option<(&str, &str)>,
    area_scale: f64,
    modulus_scale: f64,
    load_scale: f64,
) -> PlanarTrussInput {
    let nodes = vec![
        node("A", 0.0, 0.0, true, 0.0),
        node("B", 1.0, 0.0, true, 0.0),
        node("C", 0.0, 1.0, false, -BASE_LOAD_N * load_scale),
        node("D", 1.0, 1.0, false, -BASE_LOAD_N * load_scale),
    ];
    let area_m2 = BASE_AREA_M2 * area_scale;
    let youngs_modulus_pa = BASE_YOUNGS_MODULUS_PA * modulus_scale;
    let mut bars = vec![
        bar("AB", "A", "B", area_m2, youngs_modulus_pa),
        bar("AC", "A", "C", area_m2, youngs_modulus_pa),
        bar("BD", "B", "D", area_m2, youngs_modulus_pa),
        bar("CD", "C", "D", area_m2, youngs_modulus_pa),
    ];
    if let Some((node_i, node_j)) = diagonal {
        bars.push(bar(
            &format!("{node_i}{node_j}"),
            node_i,
            node_j,
            area_m2,
            youngs_modulus_pa,
        ));
    }
    PlanarTrussInput { nodes, bars }
}

fn check_displacement(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= DISPLACEMENT_TOLERANCE_M,
        "{label}: expected {expected:.12e} m, got {actual:.12e} m"
    );
    assert!(
        actual.abs() <= MAX_DISPLACEMENT_M,
        "{label}: {actual} m exceeds the synthetic 120 μm limit"
    );
}

fn check_equilibrium(input: &PlanarTrussInput, topology: &str) {
    let result = solve(input).expect("a braced truss in the declared envelope must solve");
    assert!(
        result.equilibrium.normalized_force_residual < 1.0e-12,
        "{topology} force residual {}",
        result.equilibrium.normalized_force_residual
    );
    assert!(
        result.equilibrium.normalized_moment_residual < 1.0e-12,
        "{topology} moment residual {}",
        result.equilibrium.normalized_moment_residual
    );
    assert!(
        result.equilibrium.normalized_free_dof_residual < 1.0e-12,
        "{topology} free-DOF residual {}",
        result.equilibrium.normalized_free_dof_residual
    );
}

/// Exhaust a fixed 3×3×3 grid of ±5% global section area, modulus and load
/// variations (27 cases per topology). With a 1 m vertical bar, the independent
/// axial oracle is delta = P*L/(E*A). Equal downward loads on the top chord leave
/// the diagonal unstretched in this small-displacement idealization, so the
/// expected nodal coordinates are known exactly. This is a deterministic
/// sensitivity grid, not a probability model or a measured uncertainty distribution.
#[test]
fn analytical_response_and_mechanism_detection_survive_parameter_variation_grid() {
    let mut case_count = 0;

    for area_scale in VARIATION_FACTORS {
        for modulus_scale in VARIATION_FACTORS {
            for load_scale in VARIATION_FACTORS {
                case_count += 1;
                let displacement_m = BASE_LOAD_N * load_scale
                    / (BASE_YOUNGS_MODULUS_PA * modulus_scale * BASE_AREA_M2 * area_scale);
                let case = format!(
                    "area={area_scale:.2}, modulus={modulus_scale:.2}, load={load_scale:.2}"
                );

                for (topology, diagonal, sign) in [
                    ("A-D", Some(("A", "D")), 1.0_f64),
                    ("B-C", Some(("B", "C")), -1.0_f64),
                ] {
                    let input = square_truss(
                        diagonal,
                        area_scale,
                        modulus_scale,
                        load_scale,
                    );
                    let result = solve(&input)
                        .unwrap_or_else(|error| panic!("{topology} {case}: {error}"));
                    let c = result
                        .node_results
                        .iter()
                        .find(|node| node.node_id == "C")
                        .expect("C result exists");
                    let d = result
                        .node_results
                        .iter()
                        .find(|node| node.node_id == "D")
                        .expect("D result exists");

                    check_displacement(c.displacement_x_m, sign * displacement_m, &format!("{topology} {case} Cx"));
                    check_displacement(c.displacement_y_m, -displacement_m, &format!("{topology} {case} Cy"));
                    check_displacement(d.displacement_x_m, sign * displacement_m, &format!("{topology} {case} Dx"));
                    check_displacement(d.displacement_y_m, -displacement_m, &format!("{topology} {case} Dy"));
                    assert!(
                        result.equilibrium.normalized_force_residual < 1.0e-12,
                        "{topology} {case} force residual {}",
                        result.equilibrium.normalized_force_residual
                    );
                    assert!(
                        result.equilibrium.normalized_moment_residual < 1.0e-12,
                        "{topology} {case} moment residual {}",
                        result.equilibrium.normalized_moment_residual
                    );
                    assert!(
                        result.equilibrium.normalized_free_dof_residual < 1.0e-12,
                        "{topology} {case} free-DOF residual {}",
                        result.equilibrium.normalized_free_dof_residual
                    );
                }

                let unbraced = square_truss(None, area_scale, modulus_scale, load_scale);
                let error = solve(&unbraced)
                    .expect_err("unbraced square must remain a mechanism at every parameter point");
                assert!(
                    error.contains("singular or numerically unresolved truss stiffness"),
                    "{case}: expected mechanism rejection, got {error}"
                );
            }
        }
    }

    assert_eq!(case_count, 27);
}
