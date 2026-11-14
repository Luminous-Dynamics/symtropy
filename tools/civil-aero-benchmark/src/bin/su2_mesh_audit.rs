use std::env;
use std::fs;
use std::process::ExitCode;

use civil_aero_benchmark::su2_mesh::inspect_su2_mesh;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("su2_mesh_audit=reject error={error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let path = arguments
        .next()
        .ok_or_else(|| "usage: su2_mesh_audit <mesh.su2>".to_owned())?;
    if arguments.next().is_some() {
        return Err("usage: su2_mesh_audit <mesh.su2>".to_owned());
    }
    let bytes = fs::read(&path).map_err(|error| format!("cannot read SU2 mesh {path}: {error}"))?;
    let summary = inspect_su2_mesh(&bytes)?;
    let element_types = summary
        .volume_element_type_counts
        .iter()
        .map(|(kind, count)| format!("{kind}:{count}"))
        .collect::<Vec<_>>()
        .join(";");
    eprintln!(
        "su2_mesh_audit=pass dimension={} points={} volume_elements={} markers={} boundary_elements={} mesh_sha256={} evidence_level={}",
        summary.dimension,
        summary.point_count,
        summary.volume_element_count,
        summary.marker_count,
        summary.boundary_element_count,
        summary.mesh_sha256,
        summary.evidence_level
    );
    eprintln!("su2_mesh_element_types={element_types}");
    print!("{}", summary.to_csv());
    Ok(())
}
