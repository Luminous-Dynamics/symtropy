use ccx_mms_p1_mesh::{generate_mesh, ErrorNorms, Mesh};
use ccx_mms_p1_oracle::{ManufacturedProblem, Vec2};

const RESOLUTIONS: [usize; 4] = [8, 16, 32, 64];
const L2_ORDER_RANGE: (f64, f64) = (1.5, 2.5);
const ENERGY_ORDER_RANGE: (f64, f64) = (0.75, 1.25);

#[derive(Clone, Copy, Debug)]
struct QuadratureCheck {
    cells_per_side: usize,
    load_vector_difference: f64,
    x_resultant_residual: f64,
    y_resultant_residual: f64,
    integrated_absolute_force_x: f64,
    integrated_absolute_force_y: f64,
    l2_error_difference: f64,
    energy_error_difference: f64,
    stress_error_difference: f64,
    load_pass: bool,
    error_norm_pass: bool,
}

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

    let positive_finite_errors = levels.iter().all(|(_, _, error)| {
        error.displacement_l2_relative.is_finite()
            && error.displacement_l2_relative > 0.0
            && error.energy_error_relative.is_finite()
            && error.energy_error_relative > 0.0
            && error.stress_l2_relative.is_finite()
            && error.stress_l2_relative > 0.0
    });

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

    let mut quadrature_checks: Vec<QuadratureCheck> = Vec::with_capacity(levels.len());
    for (n, mesh, _) in &levels {
        let assembly_32 = mesh
            .assemble_body_force_with_diagnostics(problem, 32)
            .map_err(|error| format!("32-point load assembly at n={n}: {error:?}"))?;
        let assembly_48 = mesh
            .assemble_body_force_with_diagnostics(problem, 48)
            .map_err(|error| format!("48-point load assembly at n={n}: {error:?}"))?;
        let load_difference = ccx_mms_p1_mesh::relative_load_difference(
            &assembly_32.nodal_loads,
            &assembly_48.nodal_loads,
        )
        .map_err(|error| format!("load quadrature comparison at n={n}: {error:?}"))?;
        let load_resultant = Mesh::resultant(&assembly_48.nodal_loads)
            .map_err(|error| format!("load resultant at n={n}: {error:?}"))?;
        let exact_resultant = problem.integrated_body_force();
        let x_resultant_residual = (load_resultant.x - exact_resultant.x).abs()
            / exact_resultant.x.abs().max(1.0);
        let y_load_scale = assembly_48.integrated_absolute_force.y.max(1.0);
        let y_resultant_residual = load_resultant.y.abs() / y_load_scale;
        let load_quadrature_pass = load_difference <= 1.0e-12
            && x_resultant_residual <= 1.0e-10
            && y_resultant_residual <= 1.0e-10;

        let exact_nodal: Vec<Vec2> = mesh
            .nodes
            .iter()
            .map(|node| problem.displacement(node.x, node.y))
            .collect();
        let errors_32 = mesh
            .compute_error_norms(problem, &exact_nodal, 32)
            .map_err(|error| format!("32-point error integration at n={n}: {error:?}"))?;
        let errors_48 = mesh
            .compute_error_norms(problem, &exact_nodal, 48)
            .map_err(|error| format!("48-point error integration at n={n}: {error:?}"))?;
        let l2_difference = relative_difference(
            errors_32.displacement_l2_relative,
            errors_48.displacement_l2_relative,
        );
        let energy_difference = relative_difference(
            errors_32.energy_error_relative,
            errors_48.energy_error_relative,
        );
        let stress_difference = relative_difference(
            errors_32.stress_l2_relative,
            errors_48.stress_l2_relative,
        );
        let error_quadrature_pass = l2_difference <= 1.0e-10
            && energy_difference <= 1.0e-10
            && stress_difference <= 1.0e-10;

        quadrature_checks.push(QuadratureCheck {
            cells_per_side: *n,
            load_vector_difference: load_difference,
            x_resultant_residual,
            y_resultant_residual,
            integrated_absolute_force_x: assembly_48.integrated_absolute_force.x,
            integrated_absolute_force_y: assembly_48.integrated_absolute_force.y,
            l2_error_difference: l2_difference,
            energy_error_difference: energy_difference,
            stress_error_difference: stress_difference,
            load_pass: load_quadrature_pass,
            error_norm_pass: error_quadrature_pass,
        });
    }
    let load_quadrature_pass = quadrature_checks.iter().all(|checks| checks.load_pass);
    let error_quadrature_pass = quadrature_checks.iter().all(|checks| checks.error_norm_pass);
    let quadrature_pass = load_quadrature_pass && error_quadrature_pass;
    let calibration_pass =
        positive_finite_errors && strict_decrease && final_orders_pass && quadrature_pass;

    println!("{{");
    println!("  \"protocol_id\": \"PHYS-CCX-MMS-P1-001\",");
    println!("  \"protocol_version\": \"0.3.0\",");
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
    println!("  \"quadrature_cross_checks\": [");
    for (index, checks) in quadrature_checks.iter().enumerate() {
        println!(
            concat!(
                "    {{\"cells_per_side\": {}, ",
                "\"relative_load_vector_difference_32_vs_48\": {:.17e}, ",
                "\"normalized_x_resultant_residual\": {:.17e}, ",
                "\"normalized_y_resultant_residual\": {:.17e}, ",
                "\"integrated_absolute_force_x\": {:.17e}, ",
                "\"integrated_absolute_force_y\": {:.17e}, ",
                "\"relative_displacement_l2_difference_32_vs_48\": {:.17e}, ",
                "\"relative_energy_error_difference_32_vs_48\": {:.17e}, ",
                "\"relative_stress_l2_difference_32_vs_48\": {:.17e}, ",
                "\"load_quadrature_pass\": {}, ",
                "\"error_quadrature_pass\": {}}}{}"
            ),
            checks.cells_per_side,
            checks.load_vector_difference,
            checks.x_resultant_residual,
            checks.y_resultant_residual,
            checks.integrated_absolute_force_x,
            checks.integrated_absolute_force_y,
            checks.l2_error_difference,
            checks.energy_error_difference,
            checks.stress_error_difference,
            checks.load_pass,
            checks.error_norm_pass,
            if index + 1 < quadrature_checks.len() { "," } else { "" }
        );
    }
    println!("  ],");
    println!("  \"gates\": {{");
    println!("    \"positive_finite_error_norms\": {},", positive_finite_errors);
    println!("    \"strict_l2_and_energy_decrease\": {},", strict_decrease);
    println!("    \"last_two_order_intervals_pass\": {},", final_orders_pass);
    println!("    \"load_quadrature_independence_pass\": {},", load_quadrature_pass);
    println!("    \"error_norm_quadrature_independence_pass\": {}", error_quadrature_pass);
    println!("  }},");
    println!("  \"postprocessor_calibration_pass\": {calibration_pass}");
    println!("}}");

    Ok(())
}
