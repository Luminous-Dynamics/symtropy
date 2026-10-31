use std::process;

use civil_aero_benchmark::input_sampling::{
    LatinHypercubeStudy, NormalInputMarginal, generate_correlated_lhs, report_to_csv,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic LHS input-design demo rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let study = LatinHypercubeStudy {
        study_id: "synthetic-oat15a-input-design-v0.1".to_owned(),
        seed: 20_261_031,
        sample_count: 128,
        marginals: vec![
            NormalInputMarginal {
                input_id: "Mach".to_owned(),
                input_unit: "1".to_owned(),
                source_reference: "synthetic-flow-condition-budget".to_owned(),
                mean: 0.73,
                standard_uncertainty: 0.002,
                minimum_allowed: 0.70,
                maximum_allowed: 0.76,
            },
            NormalInputMarginal {
                input_id: "Re_scale".to_owned(),
                input_unit: "1".to_owned(),
                source_reference: "synthetic-flow-condition-budget".to_owned(),
                mean: 1.0,
                standard_uncertainty: 0.02,
                minimum_allowed: 0.80,
                maximum_allowed: 1.20,
            },
            NormalInputMarginal {
                input_id: "T_static".to_owned(),
                input_unit: "K".to_owned(),
                source_reference: "synthetic-flow-condition-budget".to_owned(),
                mean: 271.0,
                standard_uncertainty: 1.0,
                minimum_allowed: 261.0,
                maximum_allowed: 281.0,
            },
        ],
        correlation_matrix: vec![
            vec![1.0, 0.3, -0.1],
            vec![0.3, 1.0, 0.4],
            vec![-0.1, 0.4, 1.0],
        ],
    };
    let design = generate_correlated_lhs(&study)?;
    eprintln!(
        "synthetic_lhs_input_design_only samples={} dimensions={} rows={} seed={} no_solver_evaluations=true",
        design.sample_count,
        design.marginals.len(),
        design.samples.len(),
        design.seed
    );
    print!("{}", report_to_csv(&design));
    Ok(())
}
