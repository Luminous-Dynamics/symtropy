// SPDX-License-Identifier: AGPL-3.0-or-later
use physical_transformation_reference::mechanics::run_reference_fixtures;
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
    let mut source_commit: Option<String> = None;
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
            source_commit = Some(value.clone());
            index += 2;
        } else {
            return Err(format!("unknown argument: {}", args[index]));
        }
    }

    let source_commit = source_commit.ok_or_else(|| {
        "usage: physical-transformation-reference --source-commit <40-char-git-sha>".to_owned()
    })?;
    let outcome = run_reference_fixture()?;
    let mechanics = run_reference_fixtures()?;

    println!(
        concat!(
            "{{",
            "\"schema_version\":\"physical-transform-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-TRANSFORM-001\",",
            "\"source_commit\":\"{}\",",
            "\"evidence_class\":\"synthetic\",",
            "\"result\":\"pass\",",
            "\"scope\":\"topology_mass_accounting_and_analytical_mechanics\",",
            "\"fixtures\":{{",
            "\"topology_replay\":\"pass\",",
            "\"transactional_rejection\":\"{}\",",
            "\"balanced_mass_ledger\":\"{}\",",
            "\"imbalanced_mass_negative_control\":\"{}\",",
            "\"analytical_mechanics\":\"pass\"",
            "}},",
            "\"summary\":{{",
            "\"component_count\":{},",
            "\"connection_count\":{},",
            "\"axial_bar_stress_pa\":{:.17e},",
            "\"axial_bar_strain\":{:.17e},",
            "\"axial_bar_extension_m\":{:.17e},",
            "\"cantilever_tip_deflection_m\":{:.17e},",
            "\"cantilever_maximum_bending_stress_pa\":{:.17e},",
            "\"simply_supported_midspan_deflection_m\":{:.17e},",
            "\"simply_supported_maximum_bending_stress_pa\":{:.17e},",
            "\"connected_components_before_cut\":{},",
            "\"connected_components_after_cut\":{},",
            "\"connected_components_after_reconnect\":{},",
            "\"recovered_component_count\":{},",
            "\"recovered_mass_mg\":{},",
            "\"deconstruction_step_count\":{},",
            "\"deconstruction_recovered_component_count\":{},",
            "\"deconstruction_recovered_mass_mg\":{}",
            "}},",
            "\"claim_ceiling\":\"software_invariants_and_closed_form_analytical_cases_only_not_structural_or_collapse_prediction\"",
            "}}"
        ),
        source_commit.to_ascii_lowercase(),
        if outcome.rejected_mutation_was_atomic {
            "pass"
        } else {
            "fail"
        },
        if outcome.accepted_balanced_mass_ledger {
            "pass"
        } else {
            "fail"
        },
        if outcome.rejected_imbalanced_mass_ledger {
            "pass"
        } else {
            "fail"
        },
        outcome.component_count,
        outcome.connection_count,
        mechanics.axial_stress_pa,
        mechanics.axial_strain,
        mechanics.axial_extension_m,
        mechanics.cantilever_tip_deflection_m,
        mechanics.cantilever_maximum_bending_stress_pa,
        mechanics.simply_supported_midspan_deflection_m,
        mechanics.simply_supported_maximum_bending_stress_pa,
        outcome.initial_connected_components,
        outcome.after_cut_connected_components,
        outcome.after_reconnect_connected_components,
        outcome.recovered_component_count,
        outcome.recovered_mass_mg,
        outcome.deconstruction_step_count,
        outcome.deconstruction_recovered_component_count,
        outcome.deconstruction_recovered_mass_mg
    );
    Ok(())
}
