use std::process;

use civil_aero_benchmark::grid_convergence::{
    GciStudy, GridObservation, evaluate_grid_convergence, report_to_csv,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic GCI demo rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let spacings = [4.0_f64, 2.0, 1.0, 0.5, 0.25, 0.125];
    let study = GciStudy {
        quantity: "CD".to_owned(),
        source_reference: "synthetic-known-answer-quadratic-v0.1".to_owned(),
        spacing_definition: "synthetic h values supplied explicitly; not derived from node counts"
            .to_owned(),
        safety_factor: 1.25,
        observations: spacings
            .iter()
            .enumerate()
            .map(|(index, h)| GridObservation {
                grid_id: format!("L{}", index + 1),
                characteristic_spacing: *h,
                value: 2.0 + 0.8 * h.powi(2),
            })
            .collect(),
    };
    let report = evaluate_grid_convergence(&study)?;
    eprintln!(
        "synthetic_gci_known_answer_only grids=6 triplets={} expected_order=2.0 expected_limit=2.0",
        report.triplets.len()
    );
    print!("{}", report_to_csv(&report));
    Ok(())
}
