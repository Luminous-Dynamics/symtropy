use ccx_mms_p1_mesh::{generate_mesh, ErrorNorms, Mesh};
use ccx_mms_p1_oracle::{ManufacturedProblem, Vec2};

const RESOLUTIONS: [usize; 4] = [8, 16, 32, 64];
const L2_ORDER_RANGE: (f64, f64) = (1.5, 2.5);
const ENERGY_ORDER_RANGE: (f64, f64) = (0.75, 1.25);

fn relative_difference(left: f64, right: f64) -> f64 {
    (left - right).abs() / right.abs().max(f64::MIN_POSITIVE)
}

fn json_number(value: Option<f64>) -> String {
    match value {
        Some(number) => format!("{number:.17e}"),
        None => "null".to_owned(),
    }
}

fn observed_order(previous: f64, current: f64) -> f64 {
    (previous / current).ln() / 2.0_f64.ln()
}

fn main() -> Result<(), String> {
    let problem = ManufacturedProblem::reference();
    let mut levels: Vec<(usize, Mesh, ErrorNorms)> = Vec::with_capacity(RESOLUTIONS.len());

    for n in RESOLUTIONS {
        let mesh = generate_mesh(n).map_err(|error| format!("mesh {n}: {error:?}"))?;
        let nodal: Vec<Vec2> = mesh
            .nodes
            .iter()
            .map(|node| problem.displacement(node.x, node.y))
            .collect();
        let errors = mesh
            .compute_error_norms(problem, &nodal, 12)
            .map_err(|error| format!("postprocessor at n={n}: {error:?}"))?;
        levels.push((n, mesh, errors));
    }

    let mut strict_decrease = true;
    for pair in levels.windows(2) {
        strict_decrease &= pair[1].2.displacement_l2_relative < pair[0].2.displacement_l2_relative;
        strict_decrease &= pair[1].2.energy_error_relative < pair[0].2.energy_error_relative;
    }

    let mut final_orders_pass = true;
    let mut l2_orders = Vec::with_capacity(levels.len());
    let mut energy_orders = Vec::with_capacity(levels.len());
    l2_orders.push(None);
    energy_orders.push(None);
    for index in 1..levels.len() {
        let l2 = observed_order(
            levels[index - 1].2.displacement_l2_relative,
            levels[index].2.displacement_l2_relative,
        );
        let energy = observed_order(
            levels[index - 1].2.energy_error_relative,
            levels[index].2.energy_error_relative,
        );
        l2_orders.push(Some(l2));
        energy_orders.push(Some(energy));
        if index >= 2 {
            final_orders_pass &= (L2_ORDER_RANGE.0..=L2_ORDER_RANGE.1).contains(&l2);
            final_orders_pass &= (ENERGY_ORDER_RANGE.0..=ENERGY_ORDER_RANGE.1).contains(&energy);
        }
    }

    let mesh8 = &levels[0].1;
    let nodal8: Vec<Vec2> = mesh8
        .nodes
        .iter()
        .map(|node| problem.displacement(node.x, node.y))
        .collect();
    let error32 = mesh8
        .compute_error_norms(problem, &nodal8, 32)
        .map_err(|error| format!("32-point error integration: {error:?}"))?;
    let error48 = mesh8
        .compute_error_norms(problem, &nodal8, 48)
        .map_err(|error| format!("48-point error integration: {error:?}"))?;
    let l2_quadrature_difference = relative_difference(
        error32.displacement_l2_relative,
        error48.displacement_l2_relative,
    );
    let energy_quadrature_difference = relative_difference(
        error32.energy_error_relative,
        error48.energy_error_relative,
    );
    let quadrature_pass =
        l2_quadrature_difference <= 1.0e-10 && energy_quadrature_difference <= 1.0e-10;
    let calibration_pass = strict_decrease && final_orders_pass && quadrature_pass;

    println!("{{");
    println!("  \"protocol_id\": \"PHYS-CCX-MMS-P1-001\",");
    println!("  \"protocol_version\": \"0.2.0\",");
    println!("  \"probe_status\": \"exact_nodal_interpolation_postprocessor_calibration\",");
    println!("  \"solver_execution\": false,");
    println!("  \"solver_qualification\": false,");
    println!("  \"mesh_levels\": [");
    for (index, (n, mesh, errors)) in levels.iter().enumerate() {
        let l2_order = json_number(l2_orders[index]);
        let energy_order = json_number(energy_orders[index]);
        let h_max = 2.0_f64.sqrt() / *n as f64;
        println!(
            concat!(
                "    {{\"cells_per_side\": {}, \"nodes\": {}, \"triangles\": {}, ",
                "\"total_dofs\": {}, \"h_max\": {:.17e}, ",
                "\"displacement_l2_absolute\": {:.17e}, ",
                "\"displacement_l2_relative\": {:.17e}, ",
                "\"energy_error_absolute\": {:.17e}, ",
                "\"energy_error_relative\": {:.17e}, ",
                "\"stress_l2_relative\": {:.17e}, ",
                "\"l2_order_from_previous\": {}, ",
                "\"energy_order_from_previous\": {}}}{}"
            ),
            n,
            mesh.nodes.len(),
            mesh.triangles.len(),
            2 * mesh.nodes.len(),
            h_max,
            errors.displacement_l2_absolute,
            errors.displacement_l2_relative,
            errors.energy_error_absolute,
            errors.energy_error_relative,
            errors.stress_l2_relative,
            l2_order,
            energy_order,
            if index + 1 < levels.len() { "," } else { "" }
        );
    }
    println!("  ],");
    println!("  \"quadrature_cross_check_n8\": {{");
    println!("    \"orders\": [32, 48],");
    println!(
        "    \"relative_displacement_l2_difference\": {:.17e},",
        l2_quadrature_difference
    );
    println!(
        "    \"relative_energy_error_difference\": {:.17e},",
        energy_quadrature_difference
    );
    println!("    \"pass\": {}", quadrature_pass);
    println!("  }},");
    println!("  \"gates\": {{");
    println!("    \"strict_l2_and_energy_decrease\": {},", strict_decrease);
    println!("    \"last_two_order_intervals_pass\": {},", final_orders_pass);
    println!("    \"quadrature_independence_pass\": {}", quadrature_pass);
    println!("  }},");
    println!("  \"postprocessor_calibration_pass\": {calibration_pass}");
    println!("}}");

    Ok(())
}
