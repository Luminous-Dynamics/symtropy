//! Enforce frozen known-answer gates for generated energy-design candidates.

use std::{env, error::Error, fs, io};

use symtropy_energy_dispatch_audit::DispatchObjective;

struct ExpectedBenchmark {
    case_id: &'static str,
    feasible_schedules: u64,
    frontier: Vec<DispatchObjective>,
}

fn objective(
    essential_unserved: u64,
    flexible_unserved: u64,
    operating_cost: u64,
    carbon: u64,
    curtailed_pv: u64,
    terminal_soc: u16,
) -> DispatchObjective {
    DispatchObjective {
        essential_unserved_quanta: essential_unserved,
        flexible_unserved_quanta: flexible_unserved,
        operating_cost_units: operating_cost,
        grid_carbon_grams: carbon,
        curtailed_pv_quanta: curtailed_pv,
        terminal_soc_quanta: terminal_soc,
    }
}

fn expected_benchmark(candidate_id: &str) -> Option<ExpectedBenchmark> {
    let value = match candidate_id {
        "synthetic-grid-only" => ExpectedBenchmark {
            case_id: "candidate:synthetic-grid-only",
            feasible_schedules: 1,
            frontier: vec![objective(2, 0, 0, 0, 0, 0)],
        },
        "synthetic-pv-grid" => ExpectedBenchmark {
            case_id: "candidate:synthetic-pv-grid",
            feasible_schedules: 1,
            frontier: vec![objective(2, 0, 0, 0, 2, 0)],
        },
        "synthetic-pv-battery-grid" => ExpectedBenchmark {
            case_id: "candidate:synthetic-pv-battery-grid",
            feasible_schedules: 14,
            frontier: vec![
                objective(0, 0, 0, 0, 0, 0),
                objective(1, 0, 0, 0, 0, 1),
                objective(2, 0, 0, 0, 0, 2),
            ],
        },
        _ => return None,
    };
    Some(value)
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_audit_report_gate <independent-audit.json> <candidate-id>",
        )
        .into());
    }

    let expected = expected_benchmark(&args[2]).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("no frozen benchmark case for candidate ID {}", args[2]),
        )
    })?;
    let bytes = fs::read(&args[1])?;
    let report: serde_json::Value = serde_json::from_slice(&bytes)?;

    let status = report.get("status").and_then(serde_json::Value::as_str);
    let case_id = report.get("case_id").and_then(serde_json::Value::as_str);
    let frontier_status = report
        .get("reference_frontier_status")
        .and_then(serde_json::Value::as_str);
    let feasible_schedules = report
        .get("reference_feasible_complete_schedules")
        .and_then(serde_json::Value::as_u64);
    let objective_claim = report
        .get("objective_claim_status")
        .and_then(serde_json::Value::as_str);
    let frontier_value = report
        .get("reference_frontier_objectives")
        .cloned()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "audit report has no independently enumerated frontier objectives",
            )
        })?;
    let frontier: Vec<DispatchObjective> = serde_json::from_value(frontier_value)?;
    let checks = report
        .get("checks")
        .and_then(serde_json::Value::as_array);

    let accepted = status == Some("pass")
        && case_id == Some(expected.case_id)
        && frontier_status == Some("pass")
        && feasible_schedules == Some(expected.feasible_schedules)
        && frontier == expected.frontier
        && objective_claim == Some("verified_for_bounded_model_case")
        && checks.is_some_and(|checks| {
            checks.iter().all(|item| {
                item.get("status").and_then(serde_json::Value::as_str) == Some("pass")
            })
        });

    println!(
        "candidate_id={} status={:?} case_id={:?} reference_frontier={:?} feasible_schedules={:?}/{} frontier_vectors={}/{} objective_claim={:?}",
        args[2],
        status,
        case_id,
        frontier_status,
        feasible_schedules,
        expected.feasible_schedules,
        frontier.len(),
        expected.frontier.len(),
        objective_claim
    );

    if !accepted {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frozen energy benchmark gate rejected the audit report: schedule invariants, independent frontier, or known-answer result mismatched",
        )
        .into());
    }

    println!("PASS: receipt integrity, schedule feasibility, and frozen objective-frontier known answer.");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy benchmark gate failed: {error}");
        std::process::exit(2);
    }
}
