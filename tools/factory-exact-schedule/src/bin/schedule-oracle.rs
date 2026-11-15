// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emit exact-oracle versus SPT results for the frozen reference instance.

use factory_exact_schedule::{reference_instance, solve_exact, OracleStatus};

fn main() {
    let instance = reference_instance();
    let result = solve_exact(&instance);
    println!("instance_id\tstatus\toptimality_proven\tmakespan_exact\tmakespan_spt\tgap_ticks\torders_examined\tcyclic\tfeasible");
    let status = match result.status {
        OracleStatus::Exact => "EXACT",
        OracleStatus::InvalidInput => "INVALID_INPUT",
        OracleStatus::NoFeasibleSchedule => "NO_FEASIBLE_SCHEDULE",
    };
    let exact = result.optimal_schedule.as_ref().map(|schedule| schedule.makespan.to_string()).unwrap_or_else(|| "-".to_owned());
    let spt = result.spt_schedule.as_ref().map(|schedule| schedule.makespan.to_string()).unwrap_or_else(|| "-".to_owned());
    let gap = result.gap_ticks.map(|ticks| ticks.to_string()).unwrap_or_else(|| "-".to_owned());
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        instance.instance_id,
        status,
        result.optimality_proven,
        exact,
        spt,
        gap,
        result.stats.machine_order_combinations,
        result.stats.cyclic_combinations,
        result.stats.feasible_combinations
    );
}
