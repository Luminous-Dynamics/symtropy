// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evidence-completeness gate joining EV carrier engineering and module
//! manufacturing reports. It never grants production, road or flight release.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const CLAIM_CEILING: &str = "Evidence-completeness review only; not product certification, homologation, airworthiness, factory release, regulatory approval, dispatch authority or production authorization.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    Measured,
    IndependentTest,
    ExternalAuthority,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateState {
    Qualified,
    Unknown,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    EvidenceCompleteForHumanReview,
    Blocked,
    Indeterminate,
}

#[derive(Debug, Deserialize)]
pub struct CarrierEngineeringReport {
    pub schema_version: String,
    pub programme_id: String,
    pub base_catalog_id: String,
    results: Vec<CarrierEngineeringResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct CarrierEngineeringResult {
    config_id: String,
    base_design_id: String,
    architecture: String,
    base_evidence_class: String,
    incremental_evidence_class: String,
    status: String,
    missing_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModuleFactoryReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub base_factory_benchmark_id: String,
    pub scenario_count: usize,
    results: Vec<ModuleFactoryResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModuleFactoryResult {
    site_id: String,
    template_id: String,
    config_id: String,
    architecture: String,
    evidence_class: String,
    status: String,
    base_factory_production_release_ready: bool,
    module_production_release_ready: bool,
    missing_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationCatalog {
    pub schema_version: String,
    pub benchmark_id: String,
    pub claim_ceiling: String,
    pub cases: Vec<QualificationCase>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationCase {
    pub carrier_config_id: String,
    pub base_design_id: String,
    pub architecture: String,
    pub configuration_digest_sha256: String,
    pub module_process_config_id: Option<String>,
    pub module_process_required: bool,
    pub gates: Vec<QualificationGate>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationGate {
    pub requirement_id: String,
    pub criticality: String,
    pub required_evidence_class: EvidenceClass,
    pub state: GateState,
    pub evidence_class: EvidenceClass,
    pub evidence_reference: String,
    pub evidence_digest_sha256: Option<String>,
    pub subject_configuration_digest_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SourceReportDigests {
    pub carrier_engineering_report_sha256: String,
    pub module_factory_report_sha256: String,
    pub qualification_catalog_sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GateResult {
    pub requirement_id: String,
    pub criticality: String,
    pub required_evidence_class: EvidenceClass,
    pub state: GateState,
    pub evidence_class: EvidenceClass,
    pub qualifying: bool,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QualificationResult {
    pub carrier_config_id: String,
    pub base_design_id: String,
    pub architecture: String,
    pub carrier_engineering_status: String,
    pub carrier_engineering_evidence_class: String,
    pub module_process_config_id: Option<String>,
    pub module_process_scenario_count: usize,
    pub module_process_status: String,
    pub base_factory_production_release_ready: bool,
    pub module_production_release_ready: bool,
    pub disposition: Disposition,
    pub qualified_gate_count: usize,
    pub total_gate_count: usize,
    pub missing_or_nonqualifying_gate_count: usize,
    pub findings: Vec<String>,
    pub gates: Vec<GateResult>,
    pub release_authority_granted: bool,
    pub production_release_ready: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QualificationReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub source_report_digests: SourceReportDigests,
    pub carrier_engineering_programme_id: String,
    pub module_factory_benchmark_id: String,
    pub base_factory_benchmark_id: String,
    pub case_count: usize,
    pub results: Vec<QualificationResult>,
    pub release_authority_granted: bool,
    pub production_release_ready: bool,
    pub claim_ceiling: &'static str,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn evidence_meets_requirement(required: EvidenceClass, actual: EvidenceClass) -> bool {
    match required {
        EvidenceClass::Measured => {
            matches!(actual, EvidenceClass::Measured | EvidenceClass::IndependentTest)
        }
        EvidenceClass::IndependentTest => actual == EvidenceClass::IndependentTest,
        EvidenceClass::ExternalAuthority => actual == EvidenceClass::ExternalAuthority,
        EvidenceClass::Synthetic | EvidenceClass::Unknown => false,
    }
}

fn validate_inputs(
    carrier: &CarrierEngineeringReport,
    factory: &ModuleFactoryReport,
    catalog: &QualificationCatalog,
    digests: &SourceReportDigests,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if carrier.schema_version != "0.1.0"
        || carrier.programme_id != "EV-CARRIER-REF-01"
        || carrier.base_catalog_id != "EV-PLATFORM-001"
    {
        errors.push("unsupported_carrier_engineering_report".into());
    }
    if factory.schema_version != "0.1.0"
        || factory.benchmark_id != "EV-CARRIER-MODULE-MFG-001"
        || factory.base_factory_benchmark_id != "EV-FACTORY-DESIGN-001"
    {
        errors.push("unsupported_module_factory_report".into());
    }
    if catalog.schema_version != "0.1.0"
        || catalog.benchmark_id != "EV-CARRIER-QUALIFICATION-001"
        || catalog.claim_ceiling.trim().is_empty()
    {
        errors.push("unsupported_qualification_catalog".into());
    }
    for (name, digest) in [
        ("carrier_engineering_report_sha256", &digests.carrier_engineering_report_sha256),
        ("module_factory_report_sha256", &digests.module_factory_report_sha256),
        ("qualification_catalog_sha256", &digests.qualification_catalog_sha256),
    ] {
        if !is_sha256(digest) {
            errors.push(format!("{name}: expected 64-character SHA-256 hex digest"));
        }
    }
    if catalog.cases.is_empty() || catalog.cases.len() > 32 {
        errors.push("qualification cases: expected 1..=32".into());
    }
    if carrier.results.is_empty() {
        errors.push("carrier engineering report has no results".into());
    }
    if factory.scenario_count != factory.results.len() || factory.results.is_empty() {
        errors.push("module factory report scenario_count does not match its result rows".into());
    }

    let mut carrier_ids = BTreeSet::new();
    for row in &carrier.results {
        if row.config_id.trim().is_empty() || !carrier_ids.insert(row.config_id.as_str()) {
            errors.push("carrier engineering report has empty or duplicate config_id".into());
        }
    }
    let mut factory_keys = BTreeSet::new();
    for row in &factory.results {
        if row.config_id.trim().is_empty()
            || row.site_id.trim().is_empty()
            || row.template_id.trim().is_empty()
            || !factory_keys.insert((row.config_id.as_str(), row.site_id.as_str(), row.template_id.as_str()))
        {
            errors.push("module factory report has invalid or duplicate scenario identity".into());
        }
    }

    let mut case_ids = BTreeSet::new();
    for (index, case) in catalog.cases.iter().enumerate() {
        if case.carrier_config_id.trim().is_empty()
            || !case_ids.insert(case.carrier_config_id.as_str())
        {
            errors.push(format!("cases[{index}].carrier_config_id: nonempty unique identity required"));
        }
        if !carrier_ids.contains(case.carrier_config_id.as_str()) {
            errors.push(format!("cases[{index}]: carrier config does not exist in engineering report"));
        }
        if case.base_design_id.trim().is_empty()
            || case.architecture.trim().is_empty()
            || case.notes.is_empty()
            || case.notes.iter().any(|note| note.trim().is_empty())
        {
            errors.push(format!("cases[{index}]: base design, architecture and notes are required"));
        }
        if !is_sha256(&case.configuration_digest_sha256) {
            errors.push(format!("cases[{index}].configuration_digest_sha256: invalid SHA-256"));
        }
        if case.module_process_required && case.module_process_config_id.is_none() {
            // A deliberately absent route is a qualification gap, not malformed input.
        }
        if !case.module_process_required && case.module_process_config_id.is_some() {
            errors.push(format!("cases[{index}]: module process config supplied while module_process_required is false"));
        }
        if case.gates.is_empty() || case.gates.len() > 64 {
            errors.push(format!("cases[{index}].gates: expected 1..=64"));
        }
        let mut requirements = BTreeSet::new();
        for (gate_index, gate) in case.gates.iter().enumerate() {
            if gate.requirement_id.trim().is_empty()
                || !requirements.insert(gate.requirement_id.as_str())
            {
                errors.push(format!("cases[{index}].gates[{gate_index}]: requirement IDs must be nonempty and unique"));
            }
            if !matches!(gate.criticality.as_str(), "safety_critical" | "regulatory" | "service" | "quality") {
                errors.push(format!("cases[{index}].gates[{gate_index}].criticality: unsupported criticality"));
            }
            if matches!(gate.required_evidence_class, EvidenceClass::Synthetic | EvidenceClass::Unknown) {
                errors.push(format!("cases[{index}].gates[{gate_index}]: required evidence class cannot be synthetic or unknown"));
            }
            match gate.state {
                GateState::Unknown => {
                    if gate.evidence_digest_sha256.is_some()
                        || gate.subject_configuration_digest_sha256.is_some()
                    {
                        errors.push(format!("cases[{index}].gates[{gate_index}]: unknown gate must not carry a claimed evidence digest or subject binding"));
                    }
                }
                GateState::Qualified | GateState::Failed => {
                    if gate.evidence_reference.trim().is_empty()
                        || gate.evidence_digest_sha256.as_deref().is_none_or(|value| !is_sha256(value))
                    {
                        errors.push(format!("cases[{index}].gates[{gate_index}]: qualified/failed evidence requires a reference and SHA-256 digest"));
                    }
                    if gate.subject_configuration_digest_sha256.as_deref()
                        != Some(case.configuration_digest_sha256.as_str())
                    {
                        errors.push(format!("cases[{index}].gates[{gate_index}]: evidence subject does not match case configuration digest"));
                    }
                }
            }
        }
    }
    if case_ids.len() != carrier_ids.len()
        || carrier_ids.iter().any(|config_id| !case_ids.contains(config_id))
    {
        errors.push("qualification catalog must cover every carrier engineering configuration exactly once".into());
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn evaluate_gate(gate: &QualificationGate) -> GateResult {
    let mut findings = Vec::new();
    let qualifying = match gate.state {
        GateState::Failed => {
            findings.push("required_gate_failed".into());
            false
        }
        GateState::Unknown => {
            findings.push("required_evidence_unknown".into());
            false
        }
        GateState::Qualified => {
            let class_ok = evidence_meets_requirement(
                gate.required_evidence_class,
                gate.evidence_class,
            );
            if !class_ok {
                findings.push("evidence_class_below_required_class".into());
            }
            class_ok
        }
    };
    GateResult {
        requirement_id: gate.requirement_id.clone(),
        criticality: gate.criticality.clone(),
        required_evidence_class: gate.required_evidence_class,
        state: gate.state,
        evidence_class: gate.evidence_class,
        qualifying,
        findings,
    }
}

fn evaluate_case(
    case: &QualificationCase,
    carrier_row: &CarrierEngineeringResult,
    factory_report: &ModuleFactoryReport,
) -> QualificationResult {
    let mut findings = Vec::new();
    let mut blocked = false;
    let mut indeterminate = false;

    if carrier_row.base_design_id != case.base_design_id
        || carrier_row.architecture != case.architecture
    {
        blocked = true;
        findings.push("carrier_configuration_scope_mismatch".into());
    }
    if carrier_row.status == "blocked" {
        blocked = true;
        findings.push("carrier_engineering_screen_blocked".into());
    } else if carrier_row.status != "passes_evidence_backed_screen" {
        indeterminate = true;
        findings.push("carrier_engineering_screen_not_evidence_backed".into());
    }
    if !carrier_row.missing_fields.is_empty() {
        indeterminate = true;
        findings.push("carrier_engineering_inputs_incomplete".into());
    }
    if carrier_row.base_evidence_class == "synthetic"
        || carrier_row.incremental_evidence_class == "synthetic"
        || carrier_row.base_evidence_class == "unknown"
        || carrier_row.incremental_evidence_class == "unknown"
    {
        indeterminate = true;
        findings.push("carrier_engineering_source_is_synthetic_or_unknown".into());
    }

    let mut matching_factory_rows = Vec::new();
    let mut module_process_status = "not_required".to_string();
    let mut module_scenario_count = 0;
    let mut base_factory_release_ready = !factory_report.results.is_empty()
        && factory_report.results.iter().all(|row| row.base_factory_production_release_ready);
    let mut module_release_ready = false;

    if case.module_process_required {
        match case.module_process_config_id.as_deref() {
            None => {
                base_factory_release_ready = false;
                module_release_ready = false;
                indeterminate = true;
                module_process_status = "route_not_modelled".into();
                findings.push("required_module_process_route_not_modelled".into());
            }
            Some(process_config_id) => {
                matching_factory_rows = factory_report.results.iter()
                    .filter(|row| row.config_id == process_config_id)
                    .collect();
                module_scenario_count = matching_factory_rows.len();
                if matching_factory_rows.is_empty() {
                    indeterminate = true;
                    module_process_status = "route_missing_from_factory_report".into();
                    findings.push("module_process_config_missing_from_factory_report".into());
                } else {
                    let mut any_blocked = false;
                    let mut all_evidence_backed = true;
                    for row in &matching_factory_rows {
                        if row.architecture != case.architecture {
                            blocked = true;
                            findings.push("module_process_architecture_mismatch".into());
                        }
                        if row.status == "blocked" {
                            any_blocked = true;
                        }
                        if row.status != "passes_evidence_backed_screen"
                            || row.evidence_class == "synthetic"
                            || row.evidence_class == "unknown"
                            || !row.missing_fields.is_empty()
                        {
                            all_evidence_backed = false;
                        }
                        base_factory_release_ready &= row.base_factory_production_release_ready;
                        module_release_ready &= row.module_production_release_ready;
                    }
                    if any_blocked {
                        blocked = true;
                        module_process_status = "blocked".into();
                        findings.push("module_process_capacity_blocked".into());
                    } else if all_evidence_backed {
                        module_process_status = "evidence_backed_capacity_screen".into();
                    } else {
                        indeterminate = true;
                        module_process_status = "synthetic_or_incomplete_capacity_screen".into();
                        findings.push("module_process_capacity_not_evidence_backed".into());
                    }
                }
            }
        }
        if !base_factory_release_ready {
            indeterminate = true;
            findings.push("base_factory_not_production_release_ready".into());
        }
        if !module_release_ready {
            indeterminate = true;
            findings.push("module_factory_release_not_granted".into());
        }
    } else {
        module_release_ready = false;
    }
    if !base_factory_release_ready {
        indeterminate = true;
        findings.push("base_factory_not_production_release_ready".into());
    }

    let gates: Vec<GateResult> = case.gates.iter().map(evaluate_gate).collect();
    for gate in &gates {
        if gate.state == GateState::Failed {
            blocked = true;
            findings.push(format!("qualification_gate_failed:{}", gate.requirement_id));
        } else if !gate.qualifying {
            indeterminate = true;
            findings.push(format!("qualification_gate_not_qualified:{}", gate.requirement_id));
        }
    }

    findings.sort();
    findings.dedup();
    let qualified_gate_count = gates.iter().filter(|gate| gate.qualifying).count();
    let missing_count = gates.len().saturating_sub(qualified_gate_count);
    let disposition = if blocked {
        Disposition::Blocked
    } else if indeterminate || missing_count > 0 {
        Disposition::Indeterminate
    } else {
        Disposition::EvidenceCompleteForHumanReview
    };

    QualificationResult {
        carrier_config_id: case.carrier_config_id.clone(),
        base_design_id: case.base_design_id.clone(),
        architecture: case.architecture.clone(),
        carrier_engineering_status: carrier_row.status.clone(),
        carrier_engineering_evidence_class: format!(
            "base:{};incremental:{}",
            carrier_row.base_evidence_class, carrier_row.incremental_evidence_class
        ),
        module_process_config_id: case.module_process_config_id.clone(),
        module_process_scenario_count: module_scenario_count,
        module_process_status,
        base_factory_production_release_ready: base_factory_release_ready,
        module_production_release_ready: module_release_ready,
        disposition,
        qualified_gate_count,
        total_gate_count: gates.len(),
        missing_or_nonqualifying_gate_count: missing_count,
        findings,
        gates,
        release_authority_granted: false,
        production_release_ready: false,
        claim_ceiling: CLAIM_CEILING,
    }
}

/// Join the existing carrier engineering report, module manufacturing report
/// and a configuration-bound qualification matrix. No report may authorize release.
pub fn evaluate_qualification_chain(
    carrier: &CarrierEngineeringReport,
    factory: &ModuleFactoryReport,
    catalog: &QualificationCatalog,
    digests: SourceReportDigests,
) -> Result<QualificationReport, Vec<String>> {
    validate_inputs(carrier, factory, catalog, &digests)?;
    let mut cases = catalog.cases.iter().collect::<Vec<_>>();
    cases.sort_by(|left, right| left.carrier_config_id.cmp(&right.carrier_config_id));
    let mut results = Vec::new();
    for case in cases {
        let carrier_row = carrier.results.iter()
            .find(|row| row.config_id == case.carrier_config_id)
            .ok_or_else(|| vec!["validated carrier result disappeared".into()])?;
        results.push(evaluate_case(case, carrier_row, factory));
    }
    Ok(QualificationReport {
        schema_version: catalog.schema_version.clone(),
        benchmark_id: catalog.benchmark_id.clone(),
        source_report_digests: digests,
        carrier_engineering_programme_id: carrier.programme_id.clone(),
        module_factory_benchmark_id: factory.benchmark_id.clone(),
        base_factory_benchmark_id: factory.base_factory_benchmark_id.clone(),
        case_count: results.len(),
        results,
        release_authority_granted: false,
        production_release_ready: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

pub fn evaluate_qualification_chain_from_bytes(
    carrier_bytes: &[u8],
    factory_bytes: &[u8],
    catalog_bytes: &[u8],
) -> Result<QualificationReport, Vec<String>> {
    let carrier: CarrierEngineeringReport = serde_json::from_slice(carrier_bytes)
        .map_err(|error| vec![format!("invalid carrier engineering report: {error}")])?;
    let factory: ModuleFactoryReport = serde_json::from_slice(factory_bytes)
        .map_err(|error| vec![format!("invalid module factory report: {error}")])?;
    let catalog: QualificationCatalog = serde_json::from_slice(catalog_bytes)
        .map_err(|error| vec![format!("invalid qualification catalog: {error}")])?;
    let digests = SourceReportDigests {
        carrier_engineering_report_sha256: digest_bytes(carrier_bytes),
        module_factory_report_sha256: digest_bytes(factory_bytes),
        qualification_catalog_sha256: digest_bytes(catalog_bytes),
    };
    evaluate_qualification_chain(&carrier, &factory, &catalog, digests)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(letter: char) -> String {
        std::iter::repeat_n(letter, 64).collect()
    }

    fn carrier_report(status: &str, evidence: &str, missing: Vec<String>) -> CarrierEngineeringReport {
        CarrierEngineeringReport {
            schema_version: "0.1.0".into(),
            programme_id: "EV-CARRIER-REF-01".into(),
            base_catalog_id: "EV-PLATFORM-001".into(),
            results: vec![CarrierEngineeringResult {
                config_id: "pod-carrier".into(),
                base_design_id: "urban-distribution-truck".into(),
                architecture: "detachable_drone_pod".into(),
                base_evidence_class: "measured".into(),
                incremental_evidence_class: evidence.into(),
                status: status.into(),
                missing_fields: missing,
            }],
        }
    }

    fn factory_report(status: &str, evidence: &str, release_ready: bool) -> ModuleFactoryReport {
        ModuleFactoryReport {
            schema_version: "0.1.0".into(),
            benchmark_id: "EV-CARRIER-MODULE-MFG-001".into(),
            base_factory_benchmark_id: "EV-FACTORY-DESIGN-001".into(),
            scenario_count: 1,
            results: vec![ModuleFactoryResult {
                site_id: "test-site".into(),
                template_id: "test-template".into(),
                config_id: "pod-process".into(),
                architecture: "detachable_drone_pod".into(),
                evidence_class: evidence.into(),
                status: status.into(),
                base_factory_production_release_ready: release_ready,
                module_production_release_ready: release_ready,
                missing_fields: Vec::new(),
            }],
        }
    }

    fn gate(
        requirement_id: &str,
        required: EvidenceClass,
        state: GateState,
        actual: EvidenceClass,
        subject_digest: Option<String>,
    ) -> QualificationGate {
        let qualified = state != GateState::Unknown;
        QualificationGate {
            requirement_id: requirement_id.into(),
            criticality: "safety_critical".into(),
            required_evidence_class: required,
            state,
            evidence_class: actual,
            evidence_reference: if qualified { "synthetic://evidence/receipt".into() } else { "unknown://not-collected".into() },
            evidence_digest_sha256: if qualified { Some(sha('b')) } else { None },
            subject_configuration_digest_sha256: if qualified { subject_digest } else { None },
        }
    }

    fn case(gates: Vec<QualificationGate>) -> QualificationCase {
        QualificationCase {
            carrier_config_id: "pod-carrier".into(),
            base_design_id: "urban-distribution-truck".into(),
            architecture: "detachable_drone_pod".into(),
            configuration_digest_sha256: sha('a'),
            module_process_config_id: Some("pod-process".into()),
            module_process_required: true,
            gates,
            notes: vec!["synthetic qualification fixture".into()],
        }
    }

    fn catalog(case: QualificationCase) -> QualificationCatalog {
        QualificationCatalog {
            schema_version: "0.1.0".into(),
            benchmark_id: "EV-CARRIER-QUALIFICATION-001".into(),
            claim_ceiling: "test".into(),
            cases: vec![case],
        }
    }

    fn digests() -> SourceReportDigests {
        SourceReportDigests {
            carrier_engineering_report_sha256: sha('c'),
            module_factory_report_sha256: sha('d'),
            qualification_catalog_sha256: sha('e'),
        }
    }

    #[test]
    fn unknown_required_evidence_remains_indeterminate() {
        let matrix = catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Unknown,
            EvidenceClass::Unknown,
            None,
        )]));
        let report = evaluate_qualification_chain(
            &carrier_report("passes_evidence_backed_screen", "measured", vec![]),
            &factory_report("passes_evidence_backed_screen", "measured", true),
            &matrix,
            digests(),
        ).expect("valid inputs");
        assert_eq!(report.results[0].disposition, Disposition::Indeterminate);
        assert!(!report.production_release_ready);
        assert!(!report.release_authority_granted);
    }

    #[test]
    fn independent_test_evidence_can_complete_a_gate_for_human_review_only() {
        let matrix = catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Qualified,
            EvidenceClass::IndependentTest,
            Some(sha('a')),
        )]));
        let report = evaluate_qualification_chain(
            &carrier_report("passes_evidence_backed_screen", "measured", vec![]),
            &factory_report("passes_evidence_backed_screen", "measured", true),
            &matrix,
            digests(),
        ).expect("valid inputs");
        assert_eq!(report.results[0].disposition, Disposition::EvidenceCompleteForHumanReview);
        assert!(!report.production_release_ready);
        assert!(!report.release_authority_granted);
    }

    #[test]
    fn synthetic_module_process_values_never_complete_manufacturing_qualification() {
        let matrix = catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Qualified,
            EvidenceClass::IndependentTest,
            Some(sha('a')),
        )]));
        let report = evaluate_qualification_chain(
            &carrier_report("passes_evidence_backed_screen", "measured", vec![]),
            &factory_report("passes_synthetic_screen", "synthetic", false),
            &matrix,
            digests(),
        ).expect("valid inputs");
        assert_eq!(report.results[0].disposition, Disposition::Indeterminate);
        assert_eq!(report.results[0].module_process_status, "synthetic_or_incomplete_capacity_screen");
    }

    #[test]
    fn failed_required_gate_blocks_configuration() {
        let matrix = catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Failed,
            EvidenceClass::IndependentTest,
            Some(sha('a')),
        )]));
        let report = evaluate_qualification_chain(
            &carrier_report("passes_evidence_backed_screen", "measured", vec![]),
            &factory_report("passes_evidence_backed_screen", "measured", true),
            &matrix,
            digests(),
        ).expect("valid inputs");
        assert_eq!(report.results[0].disposition, Disposition::Blocked);
    }

    #[test]
    fn configuration_scope_mismatch_blocks_evidence_promotion() {
        let matrix = catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Qualified,
            EvidenceClass::IndependentTest,
            Some(sha('f')),
        )]));
        assert!(evaluate_qualification_chain(
            &carrier_report("passes_evidence_backed_screen", "measured", vec![]),
            &factory_report("passes_evidence_backed_screen", "measured", true),
            &matrix,
            digests(),
        ).is_err());
    }

    #[test]
    fn measured_evidence_does_not_satisfy_independent_test_requirement() {
        let matrix = catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Qualified,
            EvidenceClass::Measured,
            Some(sha('a')),
        )]));
        let report = evaluate_qualification_chain(
            &carrier_report("passes_evidence_backed_screen", "measured", vec![]),
            &factory_report("passes_evidence_backed_screen", "measured", true),
            &matrix,
            digests(),
        ).expect("valid inputs");
        assert_eq!(report.results[0].disposition, Disposition::Indeterminate);
    }

    #[test]
    fn missing_module_route_is_indeterminate_not_a_zero_cost_pass() {
        let mut test = case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Unknown,
            EvidenceClass::Unknown,
            None,
        )]);
        test.module_process_config_id = None;
        let report = evaluate_qualification_chain(
            &carrier_report("passes_synthetic_screen", "synthetic", vec![]),
            &factory_report("passes_synthetic_screen", "synthetic", false),
            &catalog(test),
            digests(),
        ).expect("deliberate missing route is valid input");
        assert_eq!(report.results[0].disposition, Disposition::Indeterminate);
        assert_eq!(report.results[0].module_process_status, "route_not_modelled");
    }

    #[test]
    fn failed_carrier_engineering_screen_blocks() {
        let matrix = catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Qualified,
            EvidenceClass::IndependentTest,
            Some(sha('a')),
        )]));
        let report = evaluate_qualification_chain(
            &carrier_report("blocked", "measured", vec![]),
            &factory_report("passes_evidence_backed_screen", "measured", true),
            &matrix,
            digests(),
        ).expect("valid inputs");
        assert_eq!(report.results[0].disposition, Disposition::Blocked);
    }

    #[test]
    fn incomplete_carrier_inputs_prevent_review_completion() {
        let matrix = catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Qualified,
            EvidenceClass::IndependentTest,
            Some(sha('a')),
        )]));
        let report = evaluate_qualification_chain(
            &carrier_report("passes_evidence_backed_screen", "measured", vec!["incremental_mass".into()]),
            &factory_report("passes_evidence_backed_screen", "measured", true),
            &matrix,
            digests(),
        ).expect("valid inputs");
        assert_eq!(report.results[0].disposition, Disposition::Indeterminate);
    }

    #[test]
    fn source_digest_function_is_stable() {
        assert_eq!(digest_bytes(b"fixture"), digest_bytes(b"fixture"));
        assert_ne!(digest_bytes(b"fixture"), digest_bytes(b"changed"));
    }

    #[test]
    fn unknown_gate_cannot_smuggle_an_unverified_digest() {
        let matrix = catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Unknown,
            EvidenceClass::Unknown,
            None,
        )]));
        let mut broken = matrix;
        broken.cases[0].gates[0].evidence_digest_sha256 = Some(sha('b'));
        assert!(validate_inputs(
            &carrier_report("passes_evidence_backed_screen", "measured", vec![]),
            &factory_report("passes_evidence_backed_screen", "measured", true),
            &broken,
            &digests(),
        ).is_err());
    }

    #[test]
    fn duplicate_qualification_case_is_rejected() {
        let mut matrix = catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Unknown,
            EvidenceClass::Unknown,
            None,
        )]));
        matrix.cases.push(matrix.cases[0].clone());
        assert!(validate_inputs(
            &carrier_report("passes_evidence_backed_screen", "measured", vec![]),
            &factory_report("passes_evidence_backed_screen", "measured", true),
            &matrix,
            &digests(),
        ).is_err());
    }

    #[test]
    fn source_report_bytes_are_bound_to_report_digests() {
        let carrier = serde_json::to_vec(&carrier_report("passes_synthetic_screen", "synthetic", vec![]))
            .expect("carrier serialize");
        let factory = serde_json::to_vec(&factory_report("passes_synthetic_screen", "synthetic", false))
            .expect("factory serialize");
        let catalog = serde_json::to_vec(&catalog(case(vec![gate(
            "hv-isolation",
            EvidenceClass::IndependentTest,
            GateState::Unknown,
            EvidenceClass::Unknown,
            None,
        )]))).expect("catalog serialize");
        let report = evaluate_qualification_chain_from_bytes(&carrier, &factory, &catalog)
            .expect("valid bytes");
        assert_eq!(
            report.source_report_digests.carrier_engineering_report_sha256,
            digest_bytes(&carrier)
        );
    }
}
