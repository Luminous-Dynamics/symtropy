// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Generate shared forest-fire event tapes and run the Rust reference.
//!
//! CSV files are deterministic apart from elapsed_ns. The tape bundle is a
//! portable, versioned input artifact for cross-framework adapters.

use civdyn_engine_compare::{RunResult, default_scenarios, encode_tape_bundle, run_reference};
use std::fs;
use std::path::PathBuf;

fn write_trajectory_csv(path: &PathBuf, results: &[RunResult]) {
    let mut csv = String::from(
        "workload_id,width,height,seed,tick,empty,tree,burning,ash,total,trajectory_fnv64
",
    );
    for result in results {
        for row in &result.trajectory {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{:016x}
",
                result.spec.workload_id,
                result.spec.width,
                result.spec.height,
                result.spec.seed,
                row.tick,
                row.empty,
                row.tree,
                row.burning,
                row.ash,
                row.total,
                result.trajectory_fingerprint_fnv64
            ));
        }
    }
    fs::write(path, csv).expect("write trajectory CSV");
}

fn write_summary_csv(path: &PathBuf, results: &[RunResult]) {
    let mut csv = String::from(
        "workload_id,width,height,seed,ticks,initial_cells,tape_words,elapsed_ns,initial_fnv64,tape_fnv64,trajectory_fnv64
",
    );
    for result in results {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{:016x},{:016x},{:016x}
",
            result.spec.workload_id,
            result.spec.width,
            result.spec.height,
            result.spec.seed,
            result.spec.ticks,
            result.spec.width * result.spec.height,
            result.spec.ticks * result.spec.width * result.spec.height,
            result.elapsed.as_nanos(),
            result.initial_fingerprint_fnv64,
            result.tape_fingerprint_fnv64,
            result.trajectory_fingerprint_fnv64
        ));
    }
    fs::write(path, csv).expect("write summary CSV");
}

fn main() {
    let output_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/civdyn-engine-evidence"));
    fs::create_dir_all(&output_dir).expect("create output directory");

    let scenarios = default_scenarios().expect("default scenarios must validate");
    let tape = encode_tape_bundle(&scenarios).expect("tape bundle must encode");
    fs::write(output_dir.join("forest-fire-event-tapes.bin"), tape).expect("write tape bundle");

    let results = scenarios
        .iter()
        .map(|scenario| run_reference(scenario).expect("reference workload must run"))
        .collect::<Vec<_>>();
    write_summary_csv(&output_dir.join("forest-fire-summary.csv"), &results);
    write_trajectory_csv(&output_dir.join("forest-fire-trajectories.csv"), &results);

    println!(
        "CIVDYN-ENGINE-FOREST-FIRE-TRACE-001: {} exact reference runs; {} shared tape records; outputs: {}",
        results.len(),
        scenarios.len(),
        output_dir.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_portfolio_has_twenty_seed_pairs() {
        let scenarios = default_scenarios().unwrap();
        assert_eq!(scenarios.len(), 40);
        assert_eq!(
            scenarios
                .iter()
                .filter(|scenario| scenario.spec.width == 32)
                .count(),
            20
        );
        assert_eq!(
            scenarios
                .iter()
                .filter(|scenario| scenario.spec.width == 64)
                .count(),
            20
        );
    }

    #[test]
    fn same_seed_uses_same_event_tape_across_grid_sizes_only_for_shared_prefix() {
        let small = civdyn_engine_compare::generate_scenario(32, 32, 5, 1).unwrap();
        let large = civdyn_engine_compare::generate_scenario(64, 64, 5, 1).unwrap();
        assert_eq!(&small.event_tape[..32 * 32], &large.event_tape[..32 * 32]);
    }
}
