use std::env;
use std::process::ExitCode;

use ccx_mms_p1_mesh::generate_mesh;
use ccx_mms_p1_oracle::{ManufacturedProblem, PlaneStrainMaterial};

fn parse_usize(name: &str, raw: Option<String>, default: usize) -> Result<usize, String> {
    match raw {
        Some(value) => value
            .parse::<usize>()
            .map_err(|error| format!("invalid {name}: {error}")),
        None => Ok(default),
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let resolution = parse_usize("cells-per-side", args.next(), 8)?;
    let quadrature = parse_usize("quadrature-order", args.next(), 48)?;
    let mut poisson_ratio = 0.3_f64;
    let mut field = "smooth-sine";

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--poisson-ratio" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--poisson-ratio requires a value".to_owned())?;
                poisson_ratio = raw
                    .parse::<f64>()
                    .map_err(|error| format!("invalid Poisson ratio: {error}"))?;
            }
            "--field" => {
                field = match args
                    .next()
                    .ok_or_else(|| "--field requires a value".to_owned())?
                    .as_str()
                {
                    "smooth-sine" => "smooth-sine",
                    "divergence-free" => "divergence-free",
                    other => return Err(format!("unsupported manufactured field: {other}")),
                };
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    let mesh = generate_mesh(resolution).map_err(|error| format!("mesh generation: {error:?}"))?;
    let material = PlaneStrainMaterial::new(1.0, poisson_ratio)
        .map_err(|error| format!("material parameters: {error:?}"))?;
    let problem = match field {
        "smooth-sine" => ManufacturedProblem::new(material, 1.0, 0.7)
            .map_err(|error| format!("manufactured problem: {error:?}"))?,
        "divergence-free" => ManufacturedProblem::divergence_free(material),
        _ => unreachable!("field was validated while parsing"),
    };
    let deck = mesh
        .to_calculix_input_deck(problem, quadrature)
        .map_err(|error| format!("deck generation: {error:?}"))?;
    print!("{deck}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_parameters_preserve_the_frozen_reference_deck() {
        let mesh = generate_mesh(4).unwrap();
        let baseline = mesh
            .to_calculix_input_deck(ManufacturedProblem::reference(), 48)
            .unwrap();
        let material = PlaneStrainMaterial::new(1.0, 0.3).unwrap();
        let parameterized = mesh
            .to_calculix_input_deck(
                ManufacturedProblem::new(material, 1.0, 0.7).unwrap(),
                48,
            )
            .unwrap();
        assert_eq!(baseline.as_bytes(), parameterized.as_bytes());
    }

    #[test]
    fn incompressible_field_has_a_distinct_deterministic_deck() {
        let mesh = generate_mesh(4).unwrap();
        let material = PlaneStrainMaterial::new(1.0, 0.49999).unwrap();
        let problem = ManufacturedProblem::divergence_free(material);
        let first = mesh.to_calculix_input_deck(problem, 48).unwrap();
        let second = mesh.to_calculix_input_deck(problem, 48).unwrap();
        assert_eq!(first.as_bytes(), second.as_bytes());
        assert!(first.contains("4.99990000000000000e-1"));
    }
}
