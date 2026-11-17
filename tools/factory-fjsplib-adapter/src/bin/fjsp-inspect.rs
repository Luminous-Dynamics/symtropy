// SPDX-License-Identifier: AGPL-3.0-or-later
//! Inspect an FJSPLib text instance or emit one of the documented reference fixtures.

use factory_fjsplib_adapter::{FlexibleInstance, parse_fajsp, parse_fjsp};
use std::env;
use std::fs;

const FJSP_FATTAHI1: &str = include_str!("../../fixtures/fattahi1.fjsp");

const FAJSP_FATTAHI1: &str = include_str!("../../fixtures/fattahi1.fajsp");

fn emit(instance: &FlexibleInstance) {
    println!(
        "record_type\tinstance_id\tformat\tjob_count\tmachine_count\toperation_id\tjob_id\tposition\tmachine_id\tprocessing_ticks\tprecedence_before\tprecedence_after\tprecedence_job\tfingerprint_fnv1a64"
    );
    println!(
        "SUMMARY\t{}\t{}\t{}\t{}\t-\t-\t-\t-\t-\t-\t-\t-\t{}",
        instance.instance_id,
        instance.source_format.as_str(),
        instance
            .declared_job_count
            .map_or_else(|| "-".to_owned(), |value| value.to_string()),
        instance.machine_count,
        instance.canonical_fingerprint_fnv1a64
    );
    for operation in &instance.operations {
        println!(
            "OPERATION\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t-\t-\t-\t-\t{}",
            instance.instance_id,
            instance.source_format.as_str(),
            instance
                .declared_job_count
                .map_or_else(|| "-".to_owned(), |value| value.to_string()),
            instance.machine_count,
            operation.id,
            operation
                .job_id
                .map_or_else(|| "-".to_owned(), |value| value.to_string()),
            operation
                .position_in_job
                .map_or_else(|| "-".to_owned(), |value| value.to_string()),
            operation.alternatives.len(),
            instance.canonical_fingerprint_fnv1a64
        );
        for alternative in &operation.alternatives {
            println!(
                "ALTERNATIVE\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t-\t-\t-\t{}",
                instance.instance_id,
                instance.source_format.as_str(),
                instance
                    .declared_job_count
                    .map_or_else(|| "-".to_owned(), |value| value.to_string()),
                instance.machine_count,
                operation.id,
                operation
                    .job_id
                    .map_or_else(|| "-".to_owned(), |value| value.to_string()),
                operation
                    .position_in_job
                    .map_or_else(|| "-".to_owned(), |value| value.to_string()),
                alternative.machine_id,
                alternative.processing_ticks,
                instance.canonical_fingerprint_fnv1a64
            );
        }
    }
    for edge in &instance.precedences {
        println!(
            "PRECEDENCE\t{}\t{}\t{}\t{}\t-\t-\t-\t-\t-\t{}\t{}\t{}\t{}",
            instance.instance_id,
            instance.source_format.as_str(),
            instance
                .declared_job_count
                .map_or_else(|| "-".to_owned(), |value| value.to_string()),
            instance.machine_count,
            edge.before,
            edge.after,
            edge.job_id
                .map_or_else(|| "-".to_owned(), |value| value.to_string()),
            instance.canonical_fingerprint_fnv1a64
        );
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
                "usage: fjsp-inspect --reference-fjsp | --reference-fajsp | <fjsp|fajsp> <instance-id> <path>"
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
