//! Evaluate a frozen selective-driving decision dataset.
//!
//! Usage:
//!   selective-driving-competence <input.json> <report.json>
//!
//! Exit code 0 means the declared records passed structural checks and metrics
//! were computed. It is not a safety approval or an authorization to drive.

use selective_driving_competence::EvaluationInput;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

const MAX_INPUT_BYTES: u64 = 256 * 1024 * 1024;

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: selective-driving-competence <input.json> <report.json>".into());
    }
    let input_path = Path::new(&args[0]);
    let report_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err("input must be a regular file no larger than 256 MiB".into());
    }
    let encoded = fs::read_to_string(input_path)?;
    let input = EvaluationInput::from_json(&encoded)
        .map_err(|error| format!("cannot parse evaluation input: {error}"))?;
    let report = input
        .evaluate()
        .map_err(|error| format!("evaluation rejected: {error}"))?;
    if let Some(parent) = report_path.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "selective-driving evaluation: {} decisions, {:.3} actual coverage, AURC={:.6}; report={}",
        report.observed_decision_count,
        report.actual_autonomy_coverage,
        report.risk_coverage_area_failure_rate,
        report_path.display()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("selective-driving-competence: {error}");
        std::process::exit(2);
    }
}
