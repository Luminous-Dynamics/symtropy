// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emit an exact tiny FJSP/FAJSP schedule certificate as TSV.

use factory_fjsplib_adapter::exact::{ExactStatus, solve_exact, verify_schedule};
use factory_fjsplib_adapter::{FlexibleInstance, parse_fajsp, parse_fjsp};
use std::env;
use std::fs;

const FJSP_FATTAHI1: &str = include_str!("../../fixtures/fattahi1.fjsp");

const FAJSP_FATTAHI1: &str = include_str!("../../fixtures/fattahi1.fajsp");

fn emit(instance: &FlexibleInstance) {
    let result = solve_exact(instance);
    if let Some(schedule) = result.schedule.as_ref() {
        assert!(
            verify_schedule(instance, schedule),
            "internal exact schedule certificate failed verification"
        );
    }
    println!(
        "record_type\tinstance_id\tformat\tstatus\toptimality_proven\tmakespan\tassignment_combinations\tmachine_order_combinations\tcyclic\tfeasible\toperation_id\tmachine_id\tsequence_index\tstart_ticks\tend_ticks\tdetails"
    );

    let status = match result.status {
        ExactStatus::Exact => "EXACT",
        ExactStatus::Inconclusive => "INCONCLUSIVE",
        ExactStatus::InvalidInput => "INVALID_INPUT",
        ExactStatus::NoFeasibleSchedule => "NO_FEASIBLE_SCHEDULE",
    };
    let makespan = result
        .schedule
        .as_ref()
        .map(|schedule| schedule.makespan.to_string())
        .unwrap_or_else(|| "-".to_owned());

    println!(
        "SUMMARY\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t-\t-\t-\t-\t-\t{:?}",
        instance.instance_id,
        instance.source_format.as_str(),
        status,
        result.optimality_proven,
        makespan,
        result.stats.assignment_combinations,
        result.stats.machine_order_combinations,
        result.stats.cyclic_combinations,
        result.stats.feasible_combinations,
        result.inconclusive_reason
    );

    if let Some(schedule) = result.schedule {
        for operation in &schedule.operations {
            println!(
                "OPERATION\t{}\t{}\t{}\ttrue\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t-",
                instance.instance_id,
                instance.source_format.as_str(),
                status,
                schedule.makespan,
                result.stats.assignment_combinations,
                result.stats.machine_order_combinations,
                result.stats.cyclic_combinations,
                result.stats.feasible_combinations,
                operation.operation_id,
                operation.machine_id,
                operation.sequence_index,
                operation.start_ticks,
                operation.end_ticks
            );
        }
        for order in &schedule.machine_orders {
            println!(
                "MACHINE_ORDER\t{}\t{}\t{}\ttrue\t{}\t{}\t{}\t{}\t{}\t-\t-\t-\t-\t-\t{}",
                instance.instance_id,
                instance.source_format.as_str(),
                status,
                schedule.makespan,
                result.stats.assignment_combinations,
                result.stats.machine_order_combinations,
                result.stats.cyclic_combinations,
                result.stats.feasible_combinations,
                order
                    .operation_ids
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(">")
            );
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let parsed = match args.as_slice() {
        [_, flag] if flag == "--reference-fjsp" => {
            parse_fjsp("fattahi1", Some("fattahi"), FJSP_FATTAHI1)
        }
        [_, flag] if flag == "--reference-fajsp" => {
            parse_fajsp("fattahi1", Some("fattahi"), FAJSP_FATTAHI1)
        }
        [_, format, instance_id, path] => {
            let source =
                fs::read_to_string(path).expect("unable to read the supplied instance file");
            match format.as_str() {
                "fjsp" => parse_fjsp(instance_id, None, &source),
                "fajsp" => parse_fajsp(instance_id, None, &source),
                other => panic!("unsupported format: {other}"),
            }
        }
        _ => {
            eprintln!(
                "usage: fjsp-solve --reference-fjsp | --reference-fajsp | <fjsp|fajsp> <instance-id> <path>"
            );
            std::process::exit(2);
        }
    };

    match parsed {
        Ok(instance) => emit(&instance),
        Err(error) => {
            eprintln!("parse failed: {error}");
            std::process::exit(1);
        }
    }
}
