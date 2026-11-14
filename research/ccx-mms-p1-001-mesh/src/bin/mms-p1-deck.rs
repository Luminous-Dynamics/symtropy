use std::env;
use std::process::ExitCode;

use ccx_mms_p1_mesh::generate_mesh;
use ccx_mms_p1_oracle::ManufacturedProblem;

fn parse_arg(name: &str, value: Option<String>, default: usize) -> Result<usize, String> {
    match value {
        Some(raw) => raw
            .parse::<usize>()
            .map_err(|error| format!("invalid {name}: {error}")),
        None => Ok(default),
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let resolution = parse_arg("cells-per-side", args.next(), 8)?;
    let quadrature = parse_arg("quadrature-order", args.next(), 48)?;
    if args.next().is_some() {
        return Err("usage: mms-p1-deck [cells-per-side] [quadrature-order]".to_owned());
    }

    let mesh = generate_mesh(resolution).map_err(|error| format!("mesh generation: {error:?}"))?;
    let problem = ManufacturedProblem::reference();
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
