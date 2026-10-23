//! Enforce frozen known-answer gates for generated energy-design candidates.

use std::{env, error::Error, fs, io};

use symtropy_energy_dispatch_audit::DispatchObjective;

struct ExpectedBenchmark {
    case_id: String,
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
            case_id: "candidate:synthetic-grid-only".to_owned(),
            feasible_schedules: 1,
            frontier: vec![objective(2, 0, 0, 0, 0, 0)],
        },
        "synthetic-pv-grid" => ExpectedBenchmark {
            case_id: "candidate:synthetic-pv-grid".to_owned(),
            feasible_schedules: 1,
            frontier: vec![objective(2, 0, 0, 0, 2, 0)],
        },
        "synthetic-pv-battery-grid" => ExpectedBenchmark {
            case_id: "candidate:synthetic-pv-battery-grid".to_owned(),
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


fn parse_parametric_id(candidate_id: &str) -> Option<(u16, u16, u16, u16)> {
    let rest = candidate_id.strip_prefix("synthetic-parametric-pv-")?;
    let (pv, rest) = rest.split_once("kw-battery-")?;
    let (capacity, rest) = rest.split_once("kwh-battery-power-")?;
    let (battery_power, grid_power) = rest.split_once("kw-grid-")?;
    let grid_power = grid_power.strip_suffix("kw")?;
    let values = (
        pv.parse::<u16>().ok()?,
        capacity.parse::<u16>().ok()?,
        battery_power.parse::<u16>().ok()?,
        grid_power.parse::<u16>().ok()?,
    );
    let (pv, capacity, battery_power, grid_power) = values;
    if ![1, 2, 3].contains(&pv)
        || ![1, 2, 3].contains(&capacity)
        || ![1, 2].contains(&battery_power)
        || ![1, 2].contains(&grid_power)
    {
        return None;
    }
    Some(values)
}

/// Closed-form known answer for this frozen two-interval family.
///
/// Let C=min(battery_capacity, charge/discharge_power). Every c in 0..=C
/// can be charged in interval 1. For each c, grid import g must satisfy
/// max(0,c-PV)<=g<=min(grid_power,c); each feasible (c,g) allows
/// min(c,battery_power)+1 terminal withdrawal choices in interval 2.
/// With explicitly zero synthetic grid cost/carbon, the nondominated
/// frontier uses maximum feasible charge C and withdrawals C..0.
fn expected_parametric_benchmark(candidate_id: &str) -> Option<ExpectedBenchmark> {
    let (pv, capacity, battery_power, grid_power) = parse_parametric_id(candidate_id)?;
    let max_charge = capacity.min(battery_power);
    let mut feasible_schedules = 0_u64;
    for charge in 0..=max_charge {
        let minimum_grid = charge.saturating_sub(pv);
        let maximum_grid = grid_power.min(charge);
        if maximum_grid >= minimum_grid {
            let grid_choices = u64::from(maximum_grid - minimum_grid) + 1;
            let withdrawal_choices = u64::from(charge.min(battery_power)) + 1;
            feasible_schedules = feasible_schedules
                .checked_add(grid_choices.checked_mul(withdrawal_choices)?)?;
        }
    }

    let curtailed_pv = u64::from(pv.saturating_sub(max_charge));
    let frontier = (0..=max_charge)
        .rev()
        .map(|withdrawal| {
            objective(
                2 - u64::from(withdrawal),
                0,
                0,
                0,
                curtailed_pv,
                max_charge - withdrawal,
            )
        })
        .collect();

    Some(ExpectedBenchmark {
        case_id: format!("candidate:{candidate_id}"),
        feasible_schedules,
        frontier,
    })
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

    let expected = expected_benchmark(&args[2]).or_else(|| expected_parametric_benchmark(&args[2]));
    if expected.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("no frozen benchmark or bounded-search case for candidate ID {}", args[2]),
        )
        .into());
    }
    let expected_case_id = format!("candidate:{}", args[2]);
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

    let frontier_matches_known_answer = match expected.as_ref() {
        Some(expected) => {
            feasible_schedules == Some(expected.feasible_schedules)
                && frontier == expected.frontier
        }
        None => false,
    };
    let expected_count = expected.as_ref().map(|value| value.feasible_schedules);
    let expected_frontier_count = expected.as_ref().map(|value| value.frontier.len());

    let accepted = status == Some("pass")
        && case_id == expected.as_ref().map(|value| value.case_id.as_str())
        && case_id == Some(expected_case_id.as_str())
        && frontier_status == Some("pass")
        && frontier_matches_known_answer
        && objective_claim == Some("verified_for_bounded_model_case")
        && checks.is_some_and(|checks| {
            checks.iter().all(|item| {
                item.get("status").and_then(serde_json::Value::as_str) == Some("pass")
            })
        });

    println!(
        "candidate_id={} status={:?} case_id={:?} reference_frontier={:?} feasible_schedules={:?} expected={:?} frontier_vectors={} expected_frontier_vectors={:?} objective_claim={:?}",
        args[2],
        status,
        case_id,
        frontier_status,
        feasible_schedules,
        expected_count,
        frontier.len(),
        expected_frontier_count,
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_36_parameter_combinations_have_analytical_frontiers() {
        let mut ids = Vec::new();
        for pv in [1, 2, 3] {
            for capacity in [1, 2, 3] {
                for battery_power in [1, 2] {
                    for grid_power in [1, 2] {
                        let id = format!(
                            "synthetic-parametric-pv-{pv}kw-battery-{capacity}kwh-battery-power-{battery_power}kw-grid-{grid_power}kw"
                        );
                        let known = expected_parametric_benchmark(&id)
                            .expect("every axis combination has a known answer");
                        let max_charge = capacity.min(battery_power);
                        assert_eq!(known.frontier.len(), usize::from(max_charge) + 1);
                        assert!(known.feasible_schedules > 0);
                        assert_eq!(known.case_id, format!("candidate:{id}"));
                        ids.push(id);
                    }
                }
            }
        }
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 36);
    }

    #[test]
    fn analytical_baseline_matches_frozen_reference_case() {
        let known = expected_benchmark("synthetic-pv-battery-grid").unwrap();
        assert_eq!(known.feasible_schedules, 14);
        assert_eq!(known.frontier.len(), 3);
        assert_eq!(
            known.frontier,
            vec![
                objective(0, 0, 0, 0, 0, 0),
                objective(1, 0, 0, 0, 0, 1),
                objective(2, 0, 0, 0, 0, 2),
            ]
        );
    }

    #[test]
    fn malformed_or_out_of_range_parametric_ids_are_rejected() {
        for id in [
            "synthetic-parametric-pv-4kw-battery-1kwh-battery-power-1kw-grid-1kw",
            "synthetic-parametric-pv-1kw-battery-1kwh-battery-power-3kw-grid-1kw",
            "synthetic-parametric-pv-1kw-battery-1kwh-battery-power-1kw-grid-3kw",
            "synthetic-parametric-pv-xkw-battery-1kwh-battery-power-1kw-grid-1kw",
        ] {
            assert!(expected_parametric_benchmark(id).is_none(), "{id}");
        }
    }
}
