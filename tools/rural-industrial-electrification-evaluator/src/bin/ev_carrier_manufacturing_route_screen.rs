// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Structural process-route coverage contract for EV carrier modules.
//! This validates that the required manufacturing and service operations
//! are declared. It is not a factory cost model or production qualification.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process::ExitCode};

const REQUIRED_PROCESS_FAMILIES: [&str; 9] = [
    "structural_module",
    "mounting_retention",
    "electrical_integration",
    "power_charging",
    "thermal_environmental",
    "cargo_handling",
    "software_calibration",
    "end_of_line",
    "service_repair",
];

const REQUIRED_QUALIFICATION_GATES: [&str; 7] = [
    "structural_retention",
    "electrical_hv_safety",
    "battery_thermal_fire",
    "cargo_restraint",
    "software_cybersecurity",
    "end_of_line_test",
    "local_site_and_process_safety",
];

const CLAIM_CEILING: &str = "Structural process-route coverage only. Does not establish station capacity, cycle time, process capability, manufacturing cost, supplier availability, fire engineering, product safety, airworthiness, homologation, production qualification or investment readiness.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: String,
    programme_id: String,
    claim_ceiling: String,
    architectures: Vec<Architecture>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Architecture {
    architecture_id: String,
    base_vehicle_design_id: String,
    architecture_type: String,
    product_boundary_note: String,
    operations: Vec<ManufacturingOperation>,
    qualification_gates: Vec<QualificationGate>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManufacturingOperation {
    process_id: String,
    process_family: String,
    implementation: ImplementationRoute,
    predecessor_process_id: Option<String>,
    input_state: String,
    output_state: String,
    work_content_note: String,
    cycle_time_seconds_per_unit: Option<f64>,
    equipment_or_fixture_id: Option<String>,
    operator_skill_profile: Option<String>,
    inspection_or_acceptance_method: Option<String>,
    evidence_status: EvidenceStatus,
    evidence_reference: String,
    not_applicable_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ImplementationRoute {
    InHouse,
    Purchased,
    Integrated,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceStatus {
    Qualified,
    Measured,
    Manufacturer,
    PublicReference,
    IndependentTest,
    Unknown,
    Stale,
    Conflicted,
    Revoked,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct QualificationGate {
    gate_id: String,
    status: GateStatus,
    evidence_status: EvidenceStatus,
    evidence_reference: String,
    reviewer_or_authority: String,
    note: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GateStatus {
    Passed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RouteStatus {
    StructurallyValid,
    IncompleteProcessCoverage,
    InvalidProcessRoute,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ArchitectureResult {
    architecture_id: String,
    base_vehicle_design_id: String,
    architecture_type: String,
    route_status: RouteStatus,
    structurally_valid: bool,
    required_process_family_count: usize,
    declared_process_count: usize,
    missing_process_families: Vec<String>,
    duplicate_process_families: Vec<String>,
    process_data_incomplete_count: usize,
    process_evidence_not_qualified_count: usize,
    qualification_gate_count: usize,
    passed_qualification_gate_count: usize,
    failed_qualification_gate_count: usize,
    unknown_qualification_gate_count: usize,
    missing_qualification_gates: Vec<String>,
    production_release_ready: bool,
    findings: Vec<String>,
    product_boundary_note: String,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    programme_id: String,
    architecture_count: usize,
    structurally_valid_architecture_count: usize,
    production_release_ready_architecture_count: usize,
    results: Vec<ArchitectureResult>,
    claim_ceiling: &'static str,
}

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!(
            "usage: {} <carrier-manufacturing-routes.json>",
            program.to_string_lossy()
        );
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one carrier manufacturing route input path");
        return ExitCode::from(2);
    }

    let path = path.to_string_lossy();
    let raw = match fs::read_to_string(path.as_ref()) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read manufacturing route input: {error}");
            return ExitCode::from(2);
        }
    };
    let corpus: Corpus = match serde_json::from_str(&raw) {
        Ok(corpus) => corpus,
        Err(error) => {
            eprintln!("invalid manufacturing route input JSON: {error}");
            return ExitCode::from(1);
        }
    };
    if let Err(errors) = validate_corpus(&corpus) {
        eprintln!("manufacturing route input rejected:");
        for error in errors {
            eprintln!("- {error}");
        }
        return ExitCode::from(1);
    }

    let mut results: Vec<ArchitectureResult> =
        corpus.architectures.iter().map(evaluate_architecture).collect();
    results.sort_by(|left, right| left.architecture_id.cmp(&right.architecture_id));
    let structurally_valid_architecture_count =
        results.iter().filter(|result| result.structurally_valid).count();
    let production_release_ready_architecture_count =
        results.iter().filter(|result| result.production_release_ready).count();

    let report = Report {
        schema_version: corpus.schema_version,
        programme_id: corpus.programme_id,
        architecture_count: results.len(),
        structurally_valid_architecture_count,
        production_release_ready_architecture_count,
        results,
        claim_ceiling: CLAIM_CEILING,
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot encode manufacturing route report: {error}");
            ExitCode::from(1)
        }
    }
}

fn validate_corpus(corpus: &Corpus) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if corpus.schema_version != "0.1.0" {
        errors.push("schema_version: expected 0.1.0".into());
    }
    if corpus.programme_id != "EV-CARRIER-MFG-001" {
        errors.push("programme_id: expected EV-CARRIER-MFG-001".into());
    }
    if corpus.claim_ceiling.trim().is_empty() || corpus.architectures.is_empty() {
        errors.push("claim_ceiling and nonempty architectures are required".into());
    }

    let mut architecture_ids = BTreeSet::new();
    for (index, architecture) in corpus.architectures.iter().enumerate() {
        if architecture.architecture_id.trim().is_empty()
            || !architecture_ids.insert(architecture.architecture_id.as_str())
        {
            errors.push(format!(
                "architectures[{index}].architecture_id: must be nonempty and unique"
            ));
        }
        if architecture.base_vehicle_design_id.trim().is_empty()
            || architecture.product_boundary_note.trim().is_empty()
        {
            errors.push(format!(
                "architectures[{index}]: base vehicle and product boundary note are required"
            ));
        }
        if ![
            "integrated_enclosed_bay",
            "detachable_drone_pod",
            "mobile_drone_station_trailer",
            "amr_cargo_handoff",
        ]
        .contains(&architecture.architecture_type.as_str())
        {
            errors.push(format!(
                "architectures[{index}].architecture_type: unsupported value"
            ));
        }
        validate_architecture(index, architecture, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_architecture(
    architecture_index: usize,
    architecture: &Architecture,
    errors: &mut Vec<String>,
) {
    if architecture.operations.is_empty() {
        errors.push(format!(
            "architectures[{architecture_index}].operations: must not be empty"
        ));
    }
    if architecture.qualification_gates.is_empty() {
        errors.push(format!(
            "architectures[{architecture_index}].qualification_gates: must not be empty"
        ));
    }

    let mut process_ids = BTreeSet::new();
    for (operation_index, operation) in architecture.operations.iter().enumerate() {
        let prefix = format!(
            "architectures[{architecture_index}].operations[{operation_index}]"
        );
        if operation.process_id.trim().is_empty()
            || !process_ids.insert(operation.process_id.as_str())
        {
            errors.push(format!("{prefix}.process_id: must be nonempty and unique"));
        }
        if !REQUIRED_PROCESS_FAMILIES.contains(&operation.process_family.as_str()) {
            errors.push(format!("{prefix}.process_family: unsupported value"));
        }
        for (name, value) in [
            ("input_state", operation.input_state.as_str()),
            ("output_state", operation.output_state.as_str()),
            ("work_content_note", operation.work_content_note.as_str()),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("{prefix}.{name}: must be nonempty"));
            }
        }
        if let Some(cycle_time) = operation.cycle_time_seconds_per_unit {
            if !cycle_time.is_finite() || cycle_time <= 0.0 {
                errors.push(format!(
                    "{prefix}.cycle_time_seconds_per_unit: must be finite and positive"
                ));
            }
        }
        if operation.implementation == ImplementationRoute::NotApplicable {
            if operation
                .not_applicable_reason
                .as_ref()
                .is_none_or(|reason| reason.trim().is_empty())
            {
                errors.push(format!(
                    "{prefix}.not_applicable_reason: required for not_applicable route"
                ));
            }
        } else if operation.not_applicable_reason.is_some() {
            errors.push(format!(
                "{prefix}.not_applicable_reason: only valid for not_applicable route"
            ));
        }
        if let Some(predecessor) = &operation.predecessor_process_id
            && !architecture
                .operations
                .iter()
                .any(|candidate| candidate.process_id == *predecessor)
        {
            errors.push(format!("{prefix}.predecessor_process_id: undeclared process"));
        }
    }

    let mut gate_ids = BTreeSet::new();
    for (gate_index, gate) in architecture.qualification_gates.iter().enumerate() {
        let prefix = format!(
            "architectures[{architecture_index}].qualification_gates[{gate_index}]"
        );
        if gate.gate_id.trim().is_empty() || !gate_ids.insert(gate.gate_id.as_str()) {
            errors.push(format!("{prefix}.gate_id: must be nonempty and unique"));
        }
        if !REQUIRED_QUALIFICATION_GATES.contains(&gate.gate_id.as_str()) {
            errors.push(format!("{prefix}.gate_id: unsupported qualification gate"));
        }
        if gate.evidence_reference.trim().is_empty()
            || gate.reviewer_or_authority.trim().is_empty()
            || gate.note.trim().is_empty()
        {
            errors.push(format!(
                "{prefix}: evidence reference, reviewer/authority and note are required"
            ));
        }
    }
    for required_gate in REQUIRED_QUALIFICATION_GATES {
        if !gate_ids.contains(required_gate) {
            errors.push(format!(
                "architectures[{architecture_index}].qualification_gates: missing {required_gate}"
            ));
        }
    }
}

fn evaluate_architecture(architecture: &Architecture) -> ArchitectureResult {
    let mut findings = Vec::new();
    let declared_families: BTreeSet<&str> = architecture
        .operations
        .iter()
        .map(|operation| operation.process_family.as_str())
        .collect();
    let missing_process_families: Vec<String> = REQUIRED_PROCESS_FAMILIES
        .iter()
        .filter(|family| !declared_families.contains(**family))
        .map(|family| (*family).to_string())
        .collect();

    let mut counts_by_family = std::collections::BTreeMap::<&str, usize>::new();
    for operation in &architecture.operations {
        *counts_by_family
            .entry(operation.process_family.as_str())
            .or_default() += 1;
    }
    let duplicate_process_families: Vec<String> = counts_by_family
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(family, _)| (*family).to_string())
        .collect();

    let process_data_incomplete_count = architecture
        .operations
        .iter()
        .filter(|operation| {
            operation.implementation != ImplementationRoute::NotApplicable
                && (operation.cycle_time_seconds_per_unit.is_none()
                    || operation.equipment_or_fixture_id.is_none()
                    || operation.operator_skill_profile.is_none()
                    || operation.inspection_or_acceptance_method.is_none())
        })
        .count();
    let process_evidence_not_qualified_count = architecture
        .operations
        .iter()
        .filter(|operation| {
            operation.implementation != ImplementationRoute::NotApplicable
                && !matches!(
                    operation.evidence_status,
                    EvidenceStatus::Qualified
                        | EvidenceStatus::Measured
                        | EvidenceStatus::Manufacturer
                        | EvidenceStatus::PublicReference
                )
        })
        .count();

    let passed_qualification_gate_count = architecture
        .qualification_gates
        .iter()
        .filter(|gate| {
            gate.status == GateStatus::Passed
                && matches!(
                    gate.evidence_status,
                    EvidenceStatus::Qualified | EvidenceStatus::Measured | EvidenceStatus::IndependentTest
                )
        })
        .count();
    let failed_qualification_gate_count = architecture
        .qualification_gates
        .iter()
        .filter(|gate| gate.status == GateStatus::Failed)
        .count();
    let unknown_qualification_gate_count = architecture
        .qualification_gates
        .iter()
        .filter(|gate| {
            gate.status == GateStatus::Unknown
                || !matches!(
                    gate.evidence_status,
                    EvidenceStatus::Qualified | EvidenceStatus::Measured | EvidenceStatus::IndependentTest
                )
        })
        .count();
    let present_gate_ids: BTreeSet<&str> = architecture
        .qualification_gates
        .iter()
        .map(|gate| gate.gate_id.as_str())
        .collect();
    let missing_qualification_gates: Vec<String> = REQUIRED_QUALIFICATION_GATES
        .iter()
        .filter(|gate| !present_gate_ids.contains(**gate))
        .map(|gate| (*gate).to_string())
        .collect();

    for family in &missing_process_families {
        findings.push(format!("missing_required_process_family:{family}"));
    }
    for family in &duplicate_process_families {
        findings.push(format!("duplicate_process_family:{family}"));
    }
    if process_data_incomplete_count > 0 {
        findings.push("process_times_tooling_skills_or_acceptance_data_incomplete".into());
    }
    if process_evidence_not_qualified_count > 0 {
        findings.push("process_evidence_not_qualified".into());
    }
    if failed_qualification_gate_count > 0 {
        findings.push("one_or_more_qualification_gates_failed".into());
    }
    if unknown_qualification_gate_count > 0 {
        findings.push("one_or_more_qualification_gates_unknown_or_unqualified".into());
    }

    let operation_ids: BTreeSet<&str> = architecture
        .operations
        .iter()
        .map(|operation| operation.process_id.as_str())
        .collect();
    for operation in &architecture.operations {
        if let Some(predecessor_id) = &operation.predecessor_process_id {
            if !operation_ids.contains(predecessor_id.as_str()) {
                findings.push(format!(
                    "undeclared_predecessor_process:{}",
                    operation.process_id
                ));
            } else if let Some(predecessor) = architecture
                .operations
                .iter()
                .find(|candidate| candidate.process_id == *predecessor_id)
                && predecessor.output_state != operation.input_state
            {
                findings.push(format!(
                    "process_state_discontinuity:{}",
                    operation.process_id
                ));
            }
        }
    }

    findings.sort();
    let structurally_valid = missing_process_families.is_empty()
        && duplicate_process_families.is_empty()
        && findings
            .iter()
            .all(|finding| !finding.starts_with("process_state_discontinuity:"));
    let production_release_ready = structurally_valid
        && process_data_incomplete_count == 0
        && process_evidence_not_qualified_count == 0
        && failed_qualification_gate_count == 0
        && unknown_qualification_gate_count == 0;

    ArchitectureResult {
        architecture_id: architecture.architecture_id.clone(),
        base_vehicle_design_id: architecture.base_vehicle_design_id.clone(),
        architecture_type: architecture.architecture_type.clone(),
        route_status: if !missing_process_families.is_empty()
            || !duplicate_process_families.is_empty()
        {
            RouteStatus::IncompleteProcessCoverage
        } else if structurally_valid {
            RouteStatus::StructurallyValid
        } else {
            RouteStatus::InvalidProcessRoute
        },
        structurally_valid,
        required_process_family_count: REQUIRED_PROCESS_FAMILIES.len(),
        declared_process_count: architecture.operations.len(),
        missing_process_families,
        duplicate_process_families,
        process_data_incomplete_count,
        process_evidence_not_qualified_count,
        qualification_gate_count: architecture.qualification_gates.len(),
        passed_qualification_gate_count,
        failed_qualification_gate_count,
        unknown_qualification_gate_count,
        missing_qualification_gates,
        production_release_ready,
        findings,
        product_boundary_note: architecture.product_boundary_note.clone(),
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(
        "../fixtures/ev_carrier_manufacturing_routes_v0_1.json"
    );

    fn fixture() -> Corpus {
        serde_json::from_str(FIXTURE).expect("manufacturing route fixture must deserialize")
    }

    #[test]
    fn fixture_covers_all_required_architecture_routes_without_false_release() {
        let corpus = fixture();
        validate_corpus(&corpus).expect("fixture structure must be valid");
        let results: Vec<ArchitectureResult> =
            corpus.architectures.iter().map(evaluate_architecture).collect();
        assert_eq!(results.len(), 4);
        assert!(results.iter().all(|result| result.structurally_valid));
        assert!(results.iter().all(|result| !result.production_release_ready));
        assert!(results.iter().all(|result| result.process_data_incomplete_count > 0));
        assert!(results.iter().all(|result| result.unknown_qualification_gate_count > 0));
    }

    #[test]
    fn rejects_missing_required_process_family() {
        let mut corpus = fixture();
        corpus.architectures[0]
            .operations
            .retain(|operation| operation.process_family != "thermal_environmental");
        let result = evaluate_architecture(&corpus.architectures[0]);
        assert!(!result.structurally_valid);
        assert!(result.missing_process_families.contains(&"thermal_environmental".into()));
    }

    #[test]
    fn rejects_discontinuous_process_state_chain() {
        let mut corpus = fixture();
        let process_id = corpus.architectures[0].operations[2].process_id.clone();
        let operation = corpus.architectures[0]
            .operations
            .iter_mut()
            .find(|operation| operation.process_id == process_id)
            .expect("operation exists");
        operation.input_state = "unrelated_state".into();
        let result = evaluate_architecture(&corpus.architectures[0]);
        assert!(!result.structurally_valid);
        assert!(result.findings.iter().any(|finding| finding.contains("process_state_discontinuity")));
    }

    #[test]
    fn rejects_not_applicable_operation_without_reason() {
        let mut corpus = fixture();
        let operation = &mut corpus.architectures[0].operations[0];
        operation.implementation = ImplementationRoute::NotApplicable;
        operation.not_applicable_reason = None;
        let errors = validate_corpus(&corpus).expect_err("reason required");
        assert!(errors.iter().any(|error| error.contains("not_applicable_reason")));
    }

    #[test]
    fn rejects_nonfinite_or_nonpositive_cycle_time() {
        let mut corpus = fixture();
        corpus.architectures[0].operations[0].cycle_time_seconds_per_unit = Some(0.0);
        let errors = validate_corpus(&corpus).expect_err("invalid cycle time");
        assert!(errors.iter().any(|error| error.contains("finite and positive")));
    }
}
