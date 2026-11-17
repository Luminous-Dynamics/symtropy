// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emit the deterministic 4-architecture × 2-route × 7-scenario fixture matrix.

use factory_matched_routes::run_reference_benchmark;

fn main() {
    let rows = run_reference_benchmark();
    println!(
        "architecture_id\tproduct_id\tscenario_id\tdisposition\tfeasible_assignments\tpareto_frontier\tblocked_operations\tinput_failures"
    );

    for row in rows {
        let frontier = row
            .evaluation
            .pareto_frontier
            .iter()
            .map(|candidate| {
                format!(
                    "{}:{}:{}",
                    candidate.elapsed_ticks, candidate.cost_units, candidate.energy_units
                )
            })
            .collect::<Vec<_>>()
            .join(";");

        let blocked = row
            .evaluation
            .blocked_operations
            .iter()
            .map(|operation| operation.operation_id)
            .collect::<Vec<_>>()
            .join(",");

        let input_failures = row
            .evaluation
            .input_failures
            .iter()
            .map(|failure| format!("{failure:?}"))
            .collect::<Vec<_>>()
            .join(";");

        println!(
            "{}\t{}\t{}\t{:?}\t{}\t{}\t{}\t{}",
            row.architecture_id,
            row.product_id,
            row.scenario_id,
            row.evaluation.disposition,
            row.evaluation.feasible_assignment_count,
            if frontier.is_empty() { "-" } else { &frontier },
            if blocked.is_empty() { "-" } else { &blocked },
            if input_failures.is_empty() {
                "-"
            } else {
                &input_failures
            },
        );
    }
}
