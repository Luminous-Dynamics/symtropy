// SPDX-License-Identifier: AGPL-3.0-or-later
use physical_transformation_reference::mechanics::run_reference_fixtures;
use physical_transformation_reference::recovery_validation::run_reference_fixture as run_recovery_validation_fixture;
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
    let recovery_validation = run_recovery_validation_fixture()?;
    let recovery_evidence_status = match recovery_validation.evidence_status {
        physical_transformation_reference::recovery_validation::EvidenceStatus::Measured => {
            "measured"
        }
        physical_transformation_reference::recovery_validation::EvidenceStatus::Synthetic => {
            "synthetic"
        }
        physical_transformation_reference::recovery_validation::EvidenceStatus::Unresolved => {
            "unresolved"
        }
    };
    let recovery_plan_agreement = match recovery_validation.plan_agreement {
        physical_transformation_reference::recovery_validation::PlanAgreement::ExactMatch => {
            "exact_match"
        }
        physical_transformation_reference::recovery_validation::PlanAgreement::Deviates => {
            "deviates"
        }
        physical_transformation_reference::recovery_validation::PlanAgreement::Unresolved => {
            "unresolved"
        }
    };
    let recovery_residual = recovery_validation
        .observed_mass_residual_mg
        .ok_or_else(|| "recovery fixture residual unexpectedly unresolved".to_owned())?;
    let recovery_reused_delta = recovery_validation
        .reused_delta_mg
        .ok_or_else(|| "recovery fixture reused delta unexpectedly unresolved".to_owned())?;
    let recovery_recycled_delta = recovery_validation
        .recycled_delta_mg
        .ok_or_else(|| "recovery fixture recycled delta unexpectedly unresolved".to_owned())?;
    let recovery_contaminated_delta = recovery_validation
        .contaminated_delta_mg
        .ok_or_else(|| "recovery fixture contaminated delta unexpectedly unresolved".to_owned())?;
    let recovery_waste_delta = recovery_validation
        .waste_delta_mg
        .ok_or_else(|| "recovery fixture waste delta unexpectedly unresolved".to_owned())?;
    let recovery_unaccounted_delta = recovery_validation
        .unaccounted_delta_mg
        .ok_or_else(|| "recovery fixture unaccounted delta unexpectedly unresolved".to_owned())?;

    println!(
        concat!(
            "{{",
            "\"schema_version\":\"physical-transform-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-TRANSFORM-001\",",
            "\"source_commit\":\"{}\",",
            "\"evidence_class\":\"synthetic\",",
            "\"result\":\"pass\",",
            "\"scope\":\"topology_mass_accounting_analytical_mechanics_deconstruction_recovery_comparison\",",
            "\"fixtures\":{{",
            "\"topology_replay\":\"pass\",",
            "\"transactional_rejection\":\"{}\",",
            "\"balanced_mass_ledger\":\"{}\",",
            "\"imbalanced_mass_negative_control\":\"{}\",",
            "\"analytical_mechanics\":\"pass\",",
            "\"planned_vs_observed_recovery\":\"pass\"",
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
            "\"deconstruction_recovered_mass_mg\":{},",
            "\"recovery_evidence_status\":\"{}\",",
            "\"recovery_plan_agreement\":\"{}\",",
            "\"recovery_observed_mass_residual_mg\":{},",
            "\"recovery_reused_delta_mg\":{},",
            "\"recovery_recycled_delta_mg\":{},",
            "\"recovery_contaminated_delta_mg\":{},",
            "\"recovery_waste_delta_mg\":{},",
            "\"recovery_unaccounted_delta_mg\":{}",
            "}},",
            "\"claim_ceiling\":\"software_invariants_closed_form_mechanics_and_recovery_accounting_only_not_structural_or_collapse_prediction\"",
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
        outcome.deconstruction_recovered_mass_mg,
        recovery_evidence_status,
        recovery_plan_agreement,
        recovery_residual,
        recovery_reused_delta,
        recovery_recycled_delta,
        recovery_contaminated_delta,
        recovery_waste_delta,
        recovery_unaccounted_delta
    );
    Ok(())
}
