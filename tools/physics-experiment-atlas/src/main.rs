use std::process::ExitCode;

use physics_experiment_atlas::{json_report, run_suite};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 2 {
        eprintln!("usage: physics-experiment-atlas [--json|--help]");
        return ExitCode::from(2);
    }

    match args.get(1).map(String::as_str) {
        Some("--help") => {
            println!("physics-experiment-atlas");
            println!(
                "Runs deterministic analytic/synthetic checks for ten landmark physics experiment families."
            );
            println!("  --json  emit a stable JSON qualification receipt");
            println!("  --help  show this help");
            println!("No row is historical raw data or evidence of laboratory reproduction.");
            ExitCode::SUCCESS
        }
        Some("--json") => {
            let checks = run_suite();
            println!("{}", json_report(&checks));
            if checks.iter().all(|entry| entry.passed) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Some(unknown) => {
            eprintln!("unknown argument: {unknown}");
            eprintln!("usage: physics-experiment-atlas [--json|--help]");
            ExitCode::from(2)
        }
        None => {
            let checks = run_suite();
            println!("PHYSICS EXPERIMENT ATLAS v0.1");
            println!("Evidence class: analytic references and synthetic-model invariants only");
            println!("---------------------------------------------------------------");
            for entry in &checks {
                let verdict = if entry.passed { "PASS" } else { "FAIL" };
                println!(
                    "{verdict:4} {:28} {:42} value={:.9e} reference={:.9e} tol={:.1e}",
                    entry.experiment_id,
                    entry.metric,
                    entry.value,
                    entry.reference,
                    entry.absolute_tolerance
                );
            }
            let passed = checks.iter().filter(|entry| entry.passed).count();
            println!("---------------------------------------------------------------");
            println!("{passed}/{} checks satisfied", checks.len());
            println!("Claim ceiling: no historical raw-data reproduction or apparatus validation.");
            if passed == checks.len() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}
