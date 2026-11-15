// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emit a deterministic exact-versus-SPT certificate as TSV.

use factory_exact_schedule::{reference_instance, solve_exact, OracleStatus, Schedule};

fn emit_schedule(policy: &str, instance_id: &str, schedule: &Schedule, makespan: u64) {
    for operation in &schedule.operations {
        println!(
            "OPERATION\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t-",
            policy,
            instance_id,
            operation.job_id,
            operation.operation_id,
            operation.machine_id,
            operation.sequence_index,
            operation.setup_start,
            operation.setup_ticks,
            operation.processing_start,
            operation.processing_end,
            makespan
        );
    }
    for order in &schedule.machine_orders {
        println!(
            "MACHINE_ORDER\t{}\t{}\t-\t-\t{}\t-\t-\t-\t-\t-\t{}\t{}",
            policy,
            instance_id,
            order.machine_id,
            makespan,
            order.operation_ids.join(">")
        );
    }
}

fn main() {
    let instance = reference_instance();
    let result = solve_exact(&instance);
    println!(
        "record_type\tpolicy\tinstance_id\tjob_id\toperation_id\tmachine_id\tsequence_index\tsetup_start\tsetup_ticks\tprocessing_start\tprocessing_end\tmakespan\tdetails"
    );
    let status = match result.status {
        OracleStatus::Exact => "EXACT",
        OracleStatus::InvalidInput => "INVALID_INPUT",
        OracleStatus::NoFeasibleSchedule => "NO_FEASIBLE_SCHEDULE",
    };
    let exact = result.optimal_schedule.as_ref().map(|schedule| schedule.makespan.to_string()).unwrap_or_else(|| "-".to_owned());
    let spt = result.spt_schedule.as_ref().map(|schedule| schedule.makespan.to_string()).unwrap_or_else(|| "-".to_owned());
    let gap = result.gap_ticks.map(|ticks| ticks.to_string()).unwrap_or_else(|| "-".to_owned());

    println!(
        "SUMMARY\tEXACT_ORACLE\t{}\t-\t-\t-\t-\t-\t-\t-\t-\t{}\tstatus={};proven={};gap_ticks={};orders={};cyclic={};feasible={}",
        instance.instance_id,
        exact,
        status,
        result.optimality_proven,
        gap,
        result.stats.machine_order_combinations,
        result.stats.cyclic_combinations,
        result.stats.feasible_combinations
    );
    println!(
        "SUMMARY\tSPT_BASELINE\t{}\t-\t-\t-\t-\t-\t-\t-\t-\t{}\tgap_ticks={}",
        instance.instance_id,
        spt,
        gap
    );

    if let Some(schedule) = result.optimal_schedule.as_ref() {
        emit_schedule("EXACT_ORACLE", instance.instance_id, schedule, schedule.makespan);
    }
    if let Some(schedule) = result.spt_schedule.as_ref() {
        emit_schedule("SPT_BASELINE", instance.instance_id, schedule, schedule.makespan);
    }
}
