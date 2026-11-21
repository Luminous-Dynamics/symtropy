use std::{env, path::Path, process};

use civil_aero_benchmark::external_solver::read_external_solver_receipt;

fn main() {
    if let Err(error) = run() {
        eprintln!("external_solver_replay=reject error={error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 1 {
        return Err("usage: external_solver_replay_audit <run-directory>".to_owned());
    }

    let receipt = read_external_solver_receipt(Path::new(&arguments[0]))?;
    println!(
        "external_solver_replay=pass run_id={} solver_id={} process_status={} process_success={} receipt_sha256={} evidence_level={}",
        receipt.run_id,
        receipt.solver_id,
        receipt.status,
        receipt.is_process_success(),
        receipt.receipt_sha256,
        receipt.evidence_level
    );
    Ok(())
}
