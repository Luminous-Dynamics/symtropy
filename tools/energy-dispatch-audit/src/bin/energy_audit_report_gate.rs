//! Enforce the frozen end-to-end benchmark acceptance gate on an audit receipt.

use std::{env, error::Error, fs, io};

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_audit_report_gate <independent-audit.json> <expected-feasible-complete-schedules>",
        ).into());
    }
    let expected_count = args[2].parse::<u64>().map_err(|error| io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid expected feasible-schedule count: {error}"),
    ))?;
    let bytes = fs::read(&args[1])?;
    let report: serde_json::Value = serde_json::from_slice(&bytes)?;

    let status = report.get("status").and_then(serde_json::Value::as_str);
    let frontier = report.get("reference_frontier_status").and_then(serde_json::Value::as_str);
    let count = report.get("reference_feasible_complete_schedules").and_then(serde_json::Value::as_u64);
    let claim = report.get("objective_claim_status").and_then(serde_json::Value::as_str);
    let checks = report.get("checks").and_then(serde_json::Value::as_array);

    let accepted = status == Some("pass")
        && frontier == Some("pass")
        && count == Some(expected_count)
        && claim == Some("verified_for_bounded_model_case")
        && checks.is_some_and(|checks| checks.iter().all(|item| {
            item.get("status").and_then(serde_json::Value::as_str) == Some("pass")
        }));

    println!(
        "audit_status={:?} reference_frontier={:?} feasible_complete_schedules={:?} expected={expected_count} objective_claim={:?}",
        status, frontier, count, claim
    );

    if !accepted {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frozen energy benchmark gate rejected the independent audit report",
        ).into());
    }

    println!("PASS: independent receipt, schedule, and bounded objective-frontier gates satisfied.");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy benchmark gate failed: {error}");
        std::process::exit(2);
    }
}
