// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! CIRC-MFG-001 synthetic contract auditor.
//!
//! This tool evaluates evidence semantics in a frozen synthetic fixture set.
//! "Conformant" means only that the fixture satisfies this contract; it is not
//! a manufacturing release, safety approval, building-code determination, or
//! physical qualification.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    benchmark_id: String,
    evidence_class: EvidenceClass,
    cases: Vec<Case>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
enum EvidenceClass {
    Synthetic,
    Modeled,
    Measured,
    IndependentReview,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Case {
    case_id: String,
    expected_decision: Decision,
    required_evidence_ids: Vec<String>,
    evidence: Vec<EvidenceRecord>,
    mandatory_gates: Vec<MandatoryGate>,
    required_external_refs: Vec<ExternalRefRequirement>,
    external_refs: Vec<ExternalReference>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EvidenceRecord {
    evidence_id: String,
    status: EvidenceStatus,
    payload: Option<String>,
    sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum EvidenceStatus {
    Verified,
    Missing,
    Stale,
    Contradictory,
    WrongUnit,
    ScopeInvalid,
    DigestMismatch,
    Unresolved,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct MandatoryGate {
    name: String,
    status: GateStatus,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum GateStatus {
    Pass,
    Fail,
    Unresolved,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExternalRefRequirement {
    authority: ExternalAuthority,
    reference_id: String,
    role: ExternalRefRole,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExternalReference {
    authority: ExternalAuthority,
    reference_id: String,
    role: ExternalRefRole,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum ExternalAuthority {
    Mycelix,
    Symthaea,
    Symtropy,
    SolAtlas,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
enum ExternalRefRole {
    ProcessPlanReference,
    ProcessExecutionReceipt,
    QualificationReceipt,
    MaterialFlowAccount,
    ChainOfCustodyProfile,
    SiteResourceObservation,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
enum Decision {
    Conformant,
    Blocked,
    Unresolved,
}

#[derive(Debug, Clone, Serialize)]
struct CaseResult {
    case_id: String,
    computed_decision: Decision,
    expected_decision: Decision,
    oracle_match: bool,
    issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Summary {
    case_count: usize,
    oracle_matches: usize,
    suite_passed: bool,
}

#[derive(Debug, Clone, Serialize)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    raw_input_sha256: String,
    canonical_input_sha256: String,
    result_sha256: String,
    case_results: Vec<CaseResult>,
    summary: Summary,
}

#[derive(Serialize)]
struct ResultDigestPayload<'a> {
    schema_version: &'a str,
    benchmark_id: &'a str,
    canonical_input_sha256: &'a str,
    case_results: &'a [CaseResult],
    summary: &'a Summary,
}

#[derive(Clone, Copy)]
enum Severity {
    Blocked,
    Unresolved,
}

fn add_issue(
    issues: &mut Vec<String>,
    blocked: &mut bool,
    unresolved: &mut bool,
    severity: Severity,
    message: String,
) {
    match severity {
        Severity::Blocked => *blocked = true,
        Severity::Unresolved => *unresolved = true,
    }
    issues.push(message);
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn normalize_manifest(manifest: &mut Manifest) {
    manifest.cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    for case in &mut manifest.cases {
        case.required_evidence_ids.sort();
        case.evidence.sort_by(|a, b| {
            a.evidence_id
                .cmp(&b.evidence_id)
                .then_with(|| a.status.cmp(&b.status))
                .then_with(|| a.payload.cmp(&b.payload))
                .then_with(|| a.sha256.cmp(&b.sha256))
        });
        case.mandatory_gates
            .sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.status.cmp(&b.status)));
        case.required_external_refs.sort_by(|a, b| {
            a.authority
                .cmp(&b.authority)
                .then_with(|| a.reference_id.cmp(&b.reference_id))
                .then_with(|| a.role.cmp(&b.role))
        });
        case.external_refs.sort_by(|a, b| {
            a.authority
                .cmp(&b.authority)
                .then_with(|| a.reference_id.cmp(&b.reference_id))
                .then_with(|| a.role.cmp(&b.role))
        });
    }
}

fn validate_manifest(manifest: &Manifest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if manifest.schema_version != "0.1.0" {
        errors.push(format!(
            "unsupported_schema_version:{}",
            manifest.schema_version
        ));
    }
    if manifest.benchmark_id.trim().is_empty() {
        errors.push("empty_benchmark_id".to_string());
    }
    if manifest.evidence_class != EvidenceClass::Synthetic {
        errors.push(
            "v0.1_accepts_synthetic_contract_fixtures_only_not_physical_qualification".to_string(),
        );
    }
    if manifest.cases.is_empty() {
        errors.push("empty_case_set".to_string());
    }
    let mut ids = BTreeSet::new();
    for case in &manifest.cases {
        if case.case_id.trim().is_empty() {
            errors.push("empty_case_id".to_string());
        } else if !ids.insert(case.case_id.clone()) {
            errors.push(format!("duplicate_case_id:{}", case.case_id));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort();
        Err(errors)
    }
}

fn evaluate_case(case: &Case) -> CaseResult {
    let mut issues = Vec::new();
    let mut blocked = false;
    let mut unresolved = false;

    if case.required_evidence_ids.is_empty() {
        add_issue(
            &mut issues,
            &mut blocked,
            &mut unresolved,
            Severity::Unresolved,
            "no_required_evidence_declared".to_string(),
        );
    }

    let mut required = BTreeSet::new();
    for id in &case.required_evidence_ids {
        if id.trim().is_empty() {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                "empty_required_evidence_id".to_string(),
            );
        }
        if !required.insert(id.clone()) {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!("duplicate_required_evidence_id:{id}"),
            );
        }
    }

    let mut evidence_counts = std::collections::BTreeMap::<String, usize>::new();
    for record in &case.evidence {
        *evidence_counts
            .entry(record.evidence_id.clone())
            .or_insert(0) += 1;
        if record.evidence_id.trim().is_empty() {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                "empty_evidence_id".to_string(),
            );
        }
        if !required.contains(&record.evidence_id) {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!("unreferenced_evidence:{}", record.evidence_id),
            );
        }
    }

    for required_id in &required {
        let matches: Vec<&EvidenceRecord> = case
            .evidence
            .iter()
            .filter(|record| record.evidence_id == *required_id)
            .collect();
        match matches.as_slice() {
            [] => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Unresolved,
                format!("missing_required_evidence:{required_id}"),
            ),
            [record] => match record.status {
                EvidenceStatus::Verified => {
                    match (record.payload.as_deref(), record.sha256.as_deref()) {
                        (Some(payload), Some(expected_hash)) => {
                            let actual_hash = sha256_hex(payload.as_bytes());
                            if actual_hash != expected_hash.to_ascii_lowercase() {
                                add_issue(
                                    &mut issues,
                                    &mut blocked,
                                    &mut unresolved,
                                    Severity::Blocked,
                                    format!("payload_digest_mismatch:{required_id}"),
                                );
                            }
                        }
                        _ => add_issue(
                            &mut issues,
                            &mut blocked,
                            &mut unresolved,
                            Severity::Blocked,
                            format!("verified_evidence_missing_payload_or_digest:{required_id}"),
                        ),
                    }
                }
                EvidenceStatus::Missing => add_issue(
                    &mut issues,
                    &mut blocked,
                    &mut unresolved,
                    Severity::Unresolved,
                    format!("evidence_missing:{required_id}"),
                ),
                EvidenceStatus::Stale => add_issue(
                    &mut issues,
                    &mut blocked,
                    &mut unresolved,
                    Severity::Unresolved,
                    format!("evidence_stale:{required_id}"),
                ),
                EvidenceStatus::Unresolved => add_issue(
                    &mut issues,
                    &mut blocked,
                    &mut unresolved,
                    Severity::Unresolved,
                    format!("evidence_unresolved:{required_id}"),
                ),
                EvidenceStatus::Contradictory => add_issue(
                    &mut issues,
                    &mut blocked,
                    &mut unresolved,
                    Severity::Blocked,
                    format!("evidence_contradictory:{required_id}"),
                ),
                EvidenceStatus::WrongUnit => add_issue(
                    &mut issues,
                    &mut blocked,
                    &mut unresolved,
                    Severity::Blocked,
                    format!("evidence_wrong_unit:{required_id}"),
                ),
                EvidenceStatus::ScopeInvalid => add_issue(
                    &mut issues,
                    &mut blocked,
                    &mut unresolved,
                    Severity::Blocked,
                    format!("evidence_scope_invalid:{required_id}"),
                ),
                EvidenceStatus::DigestMismatch => add_issue(
                    &mut issues,
                    &mut blocked,
                    &mut unresolved,
                    Severity::Blocked,
                    format!("evidence_declared_digest_mismatch:{required_id}"),
                ),
            },
            _ => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!("duplicate_evidence_record:{required_id}"),
            ),
        }
    }

    for (id, count) in evidence_counts {
        if count > 1 && !required.contains(&id) {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!("duplicate_unreferenced_evidence_record:{id}"),
            );
        }
    }

    if case.mandatory_gates.is_empty() {
        add_issue(
            &mut issues,
            &mut blocked,
            &mut unresolved,
            Severity::Unresolved,
            "no_mandatory_gates_declared".to_string(),
        );
    }
    let mut gate_names = BTreeSet::new();
    for gate in &case.mandatory_gates {
        if gate.name.trim().is_empty() || !gate_names.insert(gate.name.clone()) {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!("empty_or_duplicate_mandatory_gate:{}", gate.name),
            );
        }
        match gate.status {
            GateStatus::Pass => {}
            GateStatus::Fail => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!("mandatory_gate_failed:{}", gate.name),
            ),
            GateStatus::Unresolved => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Unresolved,
                format!("mandatory_gate_unresolved:{}", gate.name),
            ),
        }
    }

    if case.required_external_refs.is_empty() {
        add_issue(
            &mut issues,
            &mut blocked,
            &mut unresolved,
            Severity::Unresolved,
            "no_required_external_refs_declared".to_string(),
        );
    }

    let mut required_ref_keys = BTreeSet::new();
    for requirement in &case.required_external_refs {
        let key = (requirement.authority, requirement.reference_id.clone());
        if requirement.reference_id.trim().is_empty() || !required_ref_keys.insert(key) {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!(
                    "empty_or_duplicate_external_ref_requirement:{:?}:{}",
                    requirement.authority, requirement.reference_id
                ),
            );
        }
    }

    let mut external_ref_counts =
        std::collections::BTreeMap::<(ExternalAuthority, String), usize>::new();
    for reference in &case.external_refs {
        let key = (reference.authority, reference.reference_id.clone());
        *external_ref_counts.entry(key.clone()).or_insert(0) += 1;
        if !required_ref_keys.contains(&key) {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!(
                    "unreferenced_external_ref:{:?}:{}",
                    reference.authority, reference.reference_id
                ),
            );
        }
    }

    for requirement in &case.required_external_refs {
        let matches: Vec<&ExternalReference> = case
            .external_refs
            .iter()
            .filter(|reference| {
                reference.authority == requirement.authority
                    && reference.reference_id == requirement.reference_id
            })
            .collect();
        match matches.as_slice() {
            [] => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Unresolved,
                format!(
                    "missing_external_ref:{:?}:{}",
                    requirement.authority, requirement.reference_id
                ),
            ),
            [reference] if reference.role != requirement.role => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!(
                    "external_ref_role_mismatch:{:?}:{}:expected={:?}:actual={:?}",
                    requirement.authority,
                    requirement.reference_id,
                    requirement.role,
                    reference.role
                ),
            ),
            [_] => {}
            _ => add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!(
                    "duplicate_external_ref:{:?}:{}",
                    requirement.authority, requirement.reference_id
                ),
            ),
        }
    }

    for ((authority, reference_id), count) in external_ref_counts {
        if count > 1 && !required_ref_keys.contains(&(authority, reference_id.clone())) {
            add_issue(
                &mut issues,
                &mut blocked,
                &mut unresolved,
                Severity::Blocked,
                format!("duplicate_unreferenced_external_ref:{authority:?}:{reference_id}"),
            );
        }
    }

    issues.sort();
    issues.dedup();

    let computed_decision = if blocked {
        Decision::Blocked
    } else if unresolved {
        Decision::Unresolved
    } else {
        Decision::Conformant
    };
    CaseResult {
        case_id: case.case_id.clone(),
        computed_decision,
        expected_decision: case.expected_decision,
        oracle_match: computed_decision == case.expected_decision,
        issues,
    }
}

fn build_report(manifest: &Manifest, raw_input: &[u8]) -> Report {
    let mut canonical_manifest = manifest.clone();
    normalize_manifest(&mut canonical_manifest);
    let canonical_bytes = serde_json::to_vec(&canonical_manifest)
        .expect("serializing a parsed manifest should not fail");
    let canonical_input_sha256 = sha256_hex(&canonical_bytes);

    let case_results: Vec<CaseResult> =
        canonical_manifest.cases.iter().map(evaluate_case).collect();
    let oracle_matches = case_results.iter().filter(|r| r.oracle_match).count();
    let summary = Summary {
        case_count: case_results.len(),
        oracle_matches,
        suite_passed: oracle_matches == case_results.len(),
    };
    let digest_payload = ResultDigestPayload {
        schema_version: &canonical_manifest.schema_version,
        benchmark_id: &canonical_manifest.benchmark_id,
        canonical_input_sha256: &canonical_input_sha256,
        case_results: &case_results,
        summary: &summary,
    };
    let digest_bytes =
        serde_json::to_vec(&digest_payload).expect("serializing a report payload should not fail");

    Report {
        schema_version: canonical_manifest.schema_version,
        benchmark_id: canonical_manifest.benchmark_id,
        raw_input_sha256: sha256_hex(raw_input),
        canonical_input_sha256,
        result_sha256: sha256_hex(&digest_bytes),
        case_results,
        summary,
    }
}

fn usage() -> &'static str {
    "Usage: circ-mfg-audit [manifest.json] [--output report.json]\n\
     Evaluates Synthetic CIRC-MFG-001 v0.1 fixtures only. Conformant is not physical qualification."
}

fn main() {
    let mut args = env::args().skip(1);
    let mut input_path: Option<PathBuf> = None;
    let mut output_path: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{}", usage());
                return;
            }
            "--output" | "-o" => {
                let Some(path) = args.next() else {
                    eprintln!("--output requires a path");
                    process::exit(2);
                };
                output_path = Some(PathBuf::from(path));
            }
            _ if arg.starts_with('-') => {
                eprintln!("Unknown option: {arg}\n{}", usage());
                process::exit(2);
            }
            _ => {
                if input_path.replace(PathBuf::from(arg)).is_some() {
                    eprintln!("Only one input manifest is supported.\n{}", usage());
                    process::exit(2);
                }
            }
        }
    }

    let input_path = input_path.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("contract-v0.1.json")
    });
    let raw_input = fs::read(&input_path).unwrap_or_else(|error| {
        eprintln!("Cannot read {}: {error}", input_path.display());
        process::exit(2);
    });
    let manifest: Manifest = serde_json::from_slice(&raw_input).unwrap_or_else(|error| {
        eprintln!("Invalid manifest {}: {error}", input_path.display());
        process::exit(2);
    });

    if let Err(errors) = validate_manifest(&manifest) {
        eprintln!("Manifest contract rejected:");
        for error in errors {
            eprintln!("  - {error}");
        }
        process::exit(2);
    }

    let report = build_report(&manifest, &raw_input);
    let json = serde_json::to_vec_pretty(&report).expect("serializing a report should not fail");
    if let Some(path) = output_path
        && let Err(error) = fs::write(&path, &json)
    {
        eprintln!("Cannot write {}: {error}", path.display());
        process::exit(2);
    }
    println!("{}", String::from_utf8_lossy(&json));

    if !report.summary.suite_passed {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Manifest, Vec<u8>) {
        let raw = include_bytes!("../fixtures/contract-v0.1.json").to_vec();
        let manifest = serde_json::from_slice(&raw).expect("fixture should parse");
        (manifest, raw)
    }

    #[test]
    fn seeded_contract_oracles_match_independent_pinned_dispositions() {
        // These expectations are pinned in test code, not derived from the
        // manifest. Editing both a fixture and its expected_decision must not
        // make the suite agree with itself silently.
        const EXPECTED: &[(&str, Decision)] = &[
            (
                "01-complete-evidence-and-external-bindings",
                Decision::Conformant,
            ),
            ("02-verified-payload-digest-mismatch", Decision::Blocked),
            ("03-required-material-lot-missing", Decision::Unresolved),
            ("04-mandatory-quality-gate-fails", Decision::Blocked),
            (
                "05-required-material-flow-account-missing",
                Decision::Unresolved,
            ),
            ("06-required-evidence-stale", Decision::Unresolved),
            ("07-critical-evidence-wrong-unit", Decision::Blocked),
            ("08-critical-evidence-contradictory", Decision::Blocked),
            ("09-critical-evidence-scope-invalid", Decision::Blocked),
            ("10-duplicate-evidence-record", Decision::Blocked),
            ("11-unreferenced-evidence-record", Decision::Blocked),
            ("12-custody-reference-role-mismatch", Decision::Blocked),
            ("13-duplicate-custody-reference", Decision::Blocked),
            (
                "14-external-reference-authority-mismatch",
                Decision::Blocked,
            ),
            ("15-site-resource-observation-missing", Decision::Unresolved),
        ];

        let (manifest, raw) = fixture();
        validate_manifest(&manifest).expect("manifest contract should be valid");
        let report = build_report(&manifest, &raw);
        assert_eq!(report.summary.case_count, EXPECTED.len());
        assert_eq!(report.summary.oracle_matches, EXPECTED.len());
        assert!(report.summary.suite_passed);
        assert_eq!(report.case_results.len(), EXPECTED.len());

        for result in &report.case_results {
            let pinned = EXPECTED
                .iter()
                .find(|(case_id, _)| *case_id == result.case_id.as_str())
                .map(|(_, decision)| *decision)
                .expect("fixture contains only independently pinned case IDs");
            assert_eq!(
                result.computed_decision, pinned,
                "computed disposition changed for {}",
                result.case_id
            );
            assert_eq!(
                result.expected_decision, pinned,
                "fixture oracle changed for {}",
                result.case_id
            );
        }
    }

    #[test]
    fn case_row_order_does_not_change_semantic_result_digest() {
        let (manifest, raw) = fixture();
        let first = build_report(&manifest, &raw);
        let mut permuted = manifest.clone();
        permuted.cases.reverse();
        for case in &mut permuted.cases {
            case.required_evidence_ids.reverse();
            case.evidence.reverse();
            case.mandatory_gates.reverse();
            case.required_external_refs.reverse();
            case.external_refs.reverse();
        }
        let permuted_raw = serde_json::to_vec(&permuted).expect("serialize permutation");
        let second = build_report(&permuted, &permuted_raw);
        assert_eq!(first.canonical_input_sha256, second.canonical_input_sha256);
        assert_eq!(first.result_sha256, second.result_sha256);
        assert_ne!(first.raw_input_sha256, second.raw_input_sha256);
    }

    #[test]
    fn missing_required_evidence_remains_unresolved() {
        let (manifest, _) = fixture();
        let mut case = manifest.cases[0].clone();
        case.required_evidence_ids
            .push("missing-critical-record".to_string());
        assert_eq!(evaluate_case(&case).computed_decision, Decision::Unresolved);
    }

    #[test]
    fn blank_required_evidence_id_blocks() {
        let (manifest, _) = fixture();
        let mut case = manifest.cases[0].clone();
        case.required_evidence_ids.push("   ".to_string());
        assert_eq!(evaluate_case(&case).computed_decision, Decision::Blocked);
    }

    #[test]
    fn blank_evidence_record_id_blocks() {
        let (manifest, _) = fixture();
        let mut case = manifest.cases[0].clone();
        case.required_evidence_ids.push(" ".to_string());
        let mut record = case.evidence[0].clone();
        record.evidence_id = " ".to_string();
        record.payload = Some("x".to_string());
        record.sha256 =
            Some("2d711642b726b04401627ca9fbac32f5da7e5c8530fb1903cc4db02258717921".to_string());
        case.evidence.push(record);
        assert_eq!(evaluate_case(&case).computed_decision, Decision::Blocked);
    }

    #[test]
    fn verified_payload_digest_mismatch_blocks() {
        let (manifest, _) = fixture();
        let mut case = manifest.cases[0].clone();
        case.evidence[0].sha256 = Some("0".repeat(64));
        assert_eq!(evaluate_case(&case).computed_decision, Decision::Blocked);
    }

    #[test]
    fn mandatory_gate_failure_blocks_even_with_complete_evidence() {
        let (manifest, _) = fixture();
        let mut case = manifest.cases[0].clone();
        case.mandatory_gates[0].status = GateStatus::Fail;
        assert_eq!(evaluate_case(&case).computed_decision, Decision::Blocked);
    }

    #[test]
    fn missing_required_external_reference_is_unresolved() {
        let (manifest, _) = fixture();
        let mut case = manifest.cases[0].clone();
        let required_id = case.required_external_refs[0].reference_id.clone();
        case.external_refs
            .retain(|reference| reference.reference_id != required_id);
        assert_eq!(evaluate_case(&case).computed_decision, Decision::Unresolved);
    }

    #[test]
    fn external_reference_role_mismatch_blocks() {
        let (manifest, _) = fixture();
        let mut case = manifest.cases[0].clone();
        let required = case.required_external_refs[0].clone();
        let actual = case
            .external_refs
            .iter_mut()
            .find(|reference| reference.reference_id == required.reference_id)
            .expect("fixture contains required external reference");
        actual.role = match required.role {
            ExternalRefRole::ProcessExecutionReceipt => ExternalRefRole::QualificationReceipt,
            _ => ExternalRefRole::ProcessExecutionReceipt,
        };
        assert_eq!(evaluate_case(&case).computed_decision, Decision::Blocked);
    }

    #[test]
    fn duplicate_external_reference_blocks() {
        let (manifest, _) = fixture();
        let mut case = manifest.cases[0].clone();
        let duplicate = case.external_refs[0].clone();
        case.external_refs.push(duplicate);
        assert_eq!(evaluate_case(&case).computed_decision, Decision::Blocked);
    }

    #[test]
    fn non_synthetic_evidence_class_is_rejected_by_v01() {
        let (mut manifest, _) = fixture();
        manifest.evidence_class = EvidenceClass::Measured;
        assert!(validate_manifest(&manifest).is_err());
    }
}
