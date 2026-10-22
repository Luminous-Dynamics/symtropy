use physical_transformation_reference::planar_truss::{
    solve, PlanarTrussInput, PlanarTrussResult, TrussBar, TrussNode,
};

const AREA_M2: f64 = 1.0e-4;
const YOUNGS_MODULUS_PA: f64 = 200.0e9;
const MAX_DISPLACEMENT_M: f64 = 3.0e-4;
const DISPLACEMENT_TOLERANCE_M: f64 = 2.0e-12;

#[derive(Clone, Copy)]
struct LoadCase {
    name: &'static str,
    c_fx_n: f64,
    c_fy_n: f64,
    d_fx_n: f64,
    d_fy_n: f64,
    expected_ad: [f64; 4],
    expected_bc: [f64; 4],
}

fn node(
    id: &str,
    x_m: f64,
    y_m: f64,
    supported: bool,
    force_x_n: f64,
    force_y_n: f64,
) -> TrussNode {
    TrussNode {
        id: id.to_owned(),
        x_m,
        y_m,
        restrained_x: supported,
        restrained_y: supported,
        force_x_n,
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

fn square_truss(diagonal: Option<(&str, &str)>, loads: LoadCase) -> PlanarTrussInput {
    let nodes = vec![
        node("A", 0.0, 0.0, true, 0.0, 0.0),
        node("B", 1.0, 0.0, true, 0.0, 0.0),
        node("C", 0.0, 1.0, false, loads.c_fx_n, loads.c_fy_n),
        node("D", 1.0, 1.0, false, loads.d_fx_n, loads.d_fy_n),
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

fn close_to(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= DISPLACEMENT_TOLERANCE_M,
        "{label}: expected {expected:.12e}, got {actual:.12e}"
    );
}

fn check_solution(
    result: &PlanarTrussResult,
    loads: LoadCase,
    expected: [f64; 4],
    topology: &str,
) {
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

    for (label, actual, expected) in [
        ("Cx", c.displacement_x_m, expected[0]),
        ("Cy", c.displacement_y_m, expected[1]),
        ("Dx", d.displacement_x_m, expected[2]),
        ("Dy", d.displacement_y_m, expected[3]),
    ] {
        close_to(actual, expected, &format!("{topology}/{} {label}", loads.name));
        assert!(
            actual.abs() <= MAX_DISPLACEMENT_M,
            "{topology}/{} {label} exceeded the synthetic serviceability envelope: {actual}",
            loads.name
        );
    }

    assert!(
        result.equilibrium.normalized_force_residual < 1.0e-12,
        "{topology}/{} force residual {}",
        loads.name,
        result.equilibrium.normalized_force_residual
    );
    assert!(
        result.equilibrium.normalized_moment_residual < 1.0e-12,
        "{topology}/{} moment residual {}",
        loads.name,
        result.equilibrium.normalized_moment_residual
    );
    assert!(
        result.equilibrium.normalized_free_dof_residual < 1.0e-12,
        "{topology}/{} free-DOF residual {}",
        loads.name,
        result.equilibrium.normalized_free_dof_residual
    );
}

/// Independent closed-form/linear-system reference values, in the order
/// [C_x, C_y, D_x, D_y]. The envelope includes load reversal, magnitude change,
/// asymmetric gravity-like loading and off-axis lateral load. The 300 μm limit
/// is deliberately a synthetic benchmark threshold, not a building-code value.
const LOAD_CASES: [LoadCase; 5] = [
    LoadCase {
        name: "symmetric_down_2kN",
        c_fx_n: 0.0, c_fy_n: -2_000.0, d_fx_n: 0.0, d_fy_n: -2_000.0,
        expected_ad: [100e-6, -100e-6, 100e-6, -100e-6],
        expected_bc: [-100e-6, -100e-6, -100e-6, -100e-6],
    },
    LoadCase {
        name: "asymmetric_down",
        c_fx_n: 0.0, c_fy_n: -1_000.0, d_fx_n: 0.0, d_fy_n: -500.0,
        expected_ad: [25e-6, -50e-6, 25e-6, -25e-6],
        expected_bc: [-50e-6, -50e-6, -50e-6, -25e-6],
    },
    LoadCase {
        name: "lateral_right_at_C",
        c_fx_n: 1_000.0, c_fy_n: 0.0, d_fx_n: 0.0, d_fy_n: 0.0,
        expected_ad: [241.4213562373096e-6, 0.0, 191.42135623730958e-6, -50e-6],
        expected_bc: [191.4213562373096e-6, 50e-6, 191.42135623730964e-6, 0.0],
    },
    LoadCase {
        name: "lateral_left_at_C",
        c_fx_n: -1_000.0, c_fy_n: 0.0, d_fx_n: 0.0, d_fy_n: 0.0,
        expected_ad: [-241.4213562373096e-6, 0.0, -191.42135623730958e-6, 50e-6],
        expected_bc: [-191.4213562373096e-6, -50e-6, -191.42135623730964e-6, 0.0],
    },
    LoadCase {
        name: "symmetric_uplift_1kN",
        c_fx_n: 0.0, c_fy_n: 1_000.0, d_fx_n: 0.0, d_fy_n: 1_000.0,
        expected_ad: [-50e-6, 50e-6, -50e-6, 50e-6],
        expected_bc: [50e-6, 50e-6, 50e-6, 50e-6],
    },
];

/// A single diagonal can satisfy different load paths in the same declared
/// serviceability envelope. This test prevents one favorable load case from
/// silently standing in for all operating conditions. It also requires the
/// visually same square without a diagonal to fail closed as a mechanism.
#[test]
fn alternative_diagonals_meet_independent_load_envelope_oracles() {
    for loads in LOAD_CASES {
        let ad = solve(&square_truss(Some(("A", "D")), loads))
            .unwrap_or_else(|error| panic!("A-D diagonal failed {}: {error}", loads.name));
        let bc = solve(&square_truss(Some(("B", "C")), loads))
            .unwrap_or_else(|error| panic!("B-C diagonal failed {}: {error}", loads.name));
        check_solution(&ad, loads, loads.expected_ad, "A-D");
        check_solution(&bc, loads, loads.expected_bc, "B-C");

        let error = solve(&square_truss(None, loads))
            .expect_err("unbraced square must not be reported as a solved structure");
        assert!(
            error.contains("singular or numerically unresolved truss stiffness"),
            "expected mechanism rejection for {} but got {error}",
            loads.name
        );
    }
}
