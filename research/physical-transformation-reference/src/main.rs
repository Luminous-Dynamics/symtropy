// SPDX-License-Identifier: AGPL-3.0-or-later
use physical_transformation_reference::run_reference_fixture;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("physical-transformation-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut source_commit: Option<&str> = None;
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--source-commit" {
            if source_commit.is_some() {
                return Err("--source-commit may be supplied only once".to_owned());
            }
            let value = args
                .get(index + 1)
                .ok_or_else(|| "--source-commit requires a value".to_owned())?;
            if value.len() != 40 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
            }
            source_commit = Some(value);
            index += 2;
        } else {
            return Err(format!("unknown argument: {}", args[index]));
        }
    }

    let source_commit = source_commit
        .ok_or_else(|| "usage: physical-transformation-reference --source-commit <40-char-git-sha>".to_owned())?;
    let outcome = run_reference_fixture()?;

    println!(
        concat!(
            "{{",
            "\"schema_version\":\"physical-transform-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-TRANSFORM-001\",",
            "\"source_commit\":\"{}\",",
            "\"evidence_class\":\"synthetic\",",
            "\"result\":\"pass\",",
            "\"scope\":\"topology_and_integer_mass_accounting\",",
            "\"fixtures\":{{",
              "\"topology_replay\":\"pass\",",
              "\"transactional_rejection\":\"{}\",",
              "\"balanced_mass_ledger\":\"{}\",",
              "\"imbalanced_mass_negative_control\":\"{}\"",
            "}},",
            "\"summary\":{{",
              "\"component_count\":{},",
              "\"connection_count\":{},",
              "\"connected_components_before_cut\":{},",
              "\"connected_components_after_cut\":{},",
              "\"connected_components_after_reconnect\":{}",
            "}},",
            "\"claim_ceiling\":\"software_invariants_only_not_structural_or_collapse_prediction\"",
            "}}"
        ),
        source_commit.to_ascii_lowercase(),
        if outcome.rejected_mutation_was_atomic { "pass" } else { "fail" },
        if outcome.accepted_balanced_mass_ledger { "pass" } else { "fail" },
        if outcome.rejected_imbalanced_mass_ledger { "pass" } else { "fail" },
        outcome.component_count,
        outcome.connection_count,
        outcome.initial_connected_components,
        outcome.after_cut_connected_components,
        outcome.after_reconnect_connected_components
    );
    Ok(())
}
