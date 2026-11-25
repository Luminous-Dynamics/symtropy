use maritime_evidence_decision_composition::audit_manifest;
use std::process::ExitCode;

fn main() -> ExitCode {
    let manifest = include_str!("../../fixtures/composition-cases.tsv");
    match audit_manifest(manifest) {
        Ok(summary) => {
            println!(
                "cases={} passed={} failed={}",
                summary.total,
                summary.passed,
                summary.total - summary.passed
            );
            for failure in &summary.failures {
                eprintln!("{failure}");
            }
            if summary.failures.is_empty() && summary.total == 30 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("composition corpus audit error: {error}");
            ExitCode::FAILURE
        }
    }
}
