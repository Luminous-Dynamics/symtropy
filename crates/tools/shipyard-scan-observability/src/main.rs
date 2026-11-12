use shipyard_scan_observability::{
    analyze_registration, assess_decision_observability, fixtures,
};

fn main() {
    for input in [
        fixtures::three_face_corner(),
        fixtures::plane(),
        fixtures::cylinder(),
    ] {
        match analyze_registration(&input) {
            Ok(report) => println!(
                "{}: {:?}; weak_directions={}; eigenvalues={:?}; claim={}",
                report.case_id,
                report.class,
                report.weak_direction_count,
                report.eigenvalues,
                report.claim_ceiling
            ),
            Err(error) => {
                eprintln!("{}: rejected: {:?}", input.case_id, error);
                std::process::exit(1);
            }
        }
    }
    let plane = fixtures::plane();
    if let Ok(report) = analyze_registration(&plane) {
        for (decision_id, gradient) in [
            ("in-plane-bead-offset", [0.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            ("surface-normal-offset", [0.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
        ] {
            match assess_decision_observability(&report, decision_id, gradient, 0.05) {
                Ok(assessment) => println!(
                    "{}: {:?}; normalized_weak_sensitivity={:.6}",
                    assessment.decision_id,
                    assessment.class,
                    assessment.normalized_weak_sensitivity
                ),
                Err(error) => {
                    eprintln!("{}: rejected: {:?}", decision_id, error);
                    std::process::exit(1);
                }
            }
        }
    }
}
