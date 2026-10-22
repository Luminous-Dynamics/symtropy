use std::env;

use ccx_mms_p1_mesh::{generate_mesh, relative_load_difference, Mesh};
use ccx_mms_p1_oracle::ManufacturedProblem;

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let resolution = match args.next() {
        Some(value) => value
            .parse::<usize>()
            .map_err(|error| format!("invalid mesh resolution: {error}"))?,
        None => 8,
    };
    if args.next().is_some() {
        return Err("usage: mms-p1-probe [cells-per-side]".to_owned());
    }

    let mesh = generate_mesh(resolution).map_err(|error| format!("mesh generation: {error:?}"))?;
    let problem = ManufacturedProblem::reference();
    let loads_32 = mesh
        .assemble_body_force(problem, 32)
        .map_err(|error| format!("32-point load assembly: {error:?}"))?;
    let loads_48 = mesh
        .assemble_body_force(problem, 48)
        .map_err(|error| format!("48-point load assembly: {error:?}"))?;
    let quadrature_difference = relative_load_difference(&loads_32, &loads_48)
        .map_err(|error| format!("quadrature comparison: {error:?}"))?;
    let resultant = Mesh::resultant(&loads_48)
        .map_err(|error| format!("load resultant: {error:?}"))?;
    let exact = problem.integrated_body_force();
    let x_residual = (resultant.x - exact.x).abs() / exact.x.abs().max(1.0);
    let y_scale = loads_48.iter().map(|load| load.y.abs()).sum::<f64>().max(1.0);
    let y_residual = resultant.y.abs() / y_scale;
    let load_assembly_checks_pass = quadrature_difference <= 1.0e-12
        && x_residual <= 1.0e-10
        && y_residual <= 1.0e-10;

    println!("{{");
    println!("  \"protocol_id\": \"PHYS-CCX-MMS-P1-001\",");
    println!("  \"probe_status\": \"assembly_only_not_fem_qualification\",");
    println!("  \"cells_per_side\": {resolution},");
    println!("  \"nodes\": {},", mesh.nodes.len());
    println!("  \"triangles\": {},", mesh.triangles.len());
    println!("  \"load_vector_entries\": {},", loads_48.len());
    println!("  \"quadrature_orders\": [32, 48],");
    println!("  \"relative_load_vector_difference\": {quadrature_difference:.17e},");
    println!("  \"resultant_x\": {resultant.x:.17e},");
    println!("  \"resultant_y\": {resultant.y:.17e},");
    println!("  \"exact_resultant_x\": {exact.x:.17e},");
    println!("  \"normalized_x_resultant_residual\": {x_residual:.17e},");
    println!("  \"normalized_y_resultant_residual\": {y_residual:.17e},");
    println!("  \"canonical_mesh_bytes\": {},", mesh.canonical_text().len());
    println!("  \"load_assembly_checks_pass\": {load_assembly_checks_pass},");
    println!("  \"solver_qualification\": false");
    println!("}}");
    Ok(())
}
