use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const INPUT_SCHEMA_VERSION: &str = "engineering-claim-closure-v0.1";
pub const REPORT_SCHEMA_VERSION: &str = "engineering-claim-closure-report-v0.1";
pub const EVALUATOR_VERSION: &str = "0.1.0";
pub const MAX_EVIDENCE_RECORDS: usize = 20_000;
pub const MAX_DEPENDENCY_EDGES: usize = 100_000;
pub const MAX_GRAPH_DEPTH: usize = 256;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimClosureInput {
    pub schema_version: String,
    pub claim: Claim,
    pub evidence_records: Vec<EvidenceRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub claim_id: String,
    pub subject_ref: String,
    pub configuration_ref: String,
    pub scope_ref: String,
    pub acquisition_start_unix_ms: u64,
    pub acquisition_end_unix_ms: u64,
    pub knowledge_cut_unix_ms: u64,
    pub evaluated_at_unix_ms: u64,
    pub requirements: Vec<EvidenceRequirement>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRequirement {
    pub evidence_ref: String,
    pub expected_record_sha256: String,
    pub role: String,
    pub uncertainty_budget_ref: String,
    pub uncertainty_budget_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    pub evidence_ref: String,
    pub record_sha256: String,
    pub role: String,
    pub subject_ref: String,
    pub configuration_ref: String,
    pub scope_ref: String,
    pub recorded_at_unix_ms: u64,
    pub effective_from_unix_ms: u64,
    pub effective_until_exclusive_unix_ms: Option<u64>,
    pub producer_authority: ProducerAuthority,
    pub trust_snapshot: TrustSnapshot,
    pub uncertainty_budget: UncertaintyBudget,
    pub dependencies: Vec<DependencyBinding>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProducerAuthority {
    pub identity_ref: String,
    pub status: AttestationStatus,
    pub as_of_unix_ms: Option<u64>,
    pub record_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TrustSnapshot {
    pub policy_ref: String,
    pub status: AttestationStatus,
    pub as_of_unix_ms: Option<u64>,
    pub policy_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UncertaintyBudget {
    pub budget_ref: String,
    pub digest_sha256: String,
    pub resolved: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DependencyBinding {
    pub evidence_ref: String,
    pub expected_record_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttestationStatus {
    Verified,
    Unverified,
    Revoked,
    Disputed,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClosureDisposition {
    ClosedForIndependentReview,
    Blocked,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FindingClass {
    Missing,
    Invalid,
    OutOfScope,
    Unverified,
    Indeterminate,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub code: String,
    pub class: FindingClass,
    pub evidence_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClosureReport {
    pub schema_version: String,
    pub evaluator_version: String,
    pub claim_id: String,
    pub canonical_input_sha256: String,
    pub disposition: ClosureDisposition,
    pub reached_evidence_refs: Vec<String>,
    pub findings: Vec<Finding>,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn add_finding(
    findings: &mut BTreeSet<Finding>,
    code: &str,
    class: FindingClass,
    evidence_ref: Option<&str>,
) {
    findings.insert(Finding {
        code: code.to_owned(),
        class,
        evidence_ref: evidence_ref.map(str::to_owned),
    });
}

fn canonical_input_sha256(input: &ClaimClosureInput) -> Result<String, String> {
    let mut canonical = input.clone();

    canonical.claim.requirements.sort_by(|left, right| {
        (
            left.evidence_ref.as_str(),
            left.role.as_str(),
            left.expected_record_sha256.as_str(),
            left.uncertainty_budget_ref.as_str(),
            left.uncertainty_budget_sha256.as_str(),
        )
            .cmp(&(
                right.evidence_ref.as_str(),
                right.role.as_str(),
                right.expected_record_sha256.as_str(),
                right.uncertainty_budget_ref.as_str(),
                right.uncertainty_budget_sha256.as_str(),
            ))
    });

    for record in &mut canonical.evidence_records {
        record.dependencies.sort_by(|left, right| {
            (
                left.evidence_ref.as_str(),
                left.expected_record_sha256.as_str(),
            )
                .cmp(&(
                    right.evidence_ref.as_str(),
                    right.expected_record_sha256.as_str(),
                ))
        });
    }

    canonical.evidence_records.sort_by(|left, right| {
        (
            left.evidence_ref.as_str(),
            left.record_sha256.as_str(),
        )
            .cmp(&(
                right.evidence_ref.as_str(),
                right.record_sha256.as_str(),
            ))
    });

    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| format!("canonical input serialization failed: {error}"))?;
    Ok(sha256_hex(&encoded))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn validate_claim(claim: &Claim, findings: &mut BTreeSet<Finding>) {
    for (field, value) in [
        ("claim_id", claim.claim_id.as_str()),
        ("subject_ref", claim.subject_ref.as_str()),
        ("configuration_ref", claim.configuration_ref.as_str()),
        ("scope_ref", claim.scope_ref.as_str()),
    ] {
        if !valid_identifier(value) {
            add_finding(
                findings,
                &format!("CLAIM_{field.to_ascii_uppercase()}_INVALID"),
                FindingClass::Invalid,
                None,
            );
        }
    }

    if claim.acquisition_start_unix_ms > claim.acquisition_end_unix_ms {
        add_finding(
            findings,
            "CLAIM_ACQUISITION_INTERVAL_REVERSED",
            FindingClass::Invalid,
            None,
        );
    }
    if claim.knowledge_cut_unix_ms > claim.evaluated_at_unix_ms {
        add_finding(
            findings,
            "KNOWLEDGE_CUT_AFTER_EVALUATION_TIME",
            FindingClass::Invalid,
            None,
        );
    }
    if claim.requirements.is_empty() {
        add_finding(
            findings,
            "CLAIM_HAS_NO_REQUIRED_EVIDENCE",
            FindingClass::Missing,
            None,
        );
    }
}

fn validate_attestation(
    status: AttestationStatus,
    identity_ref: &str,
    as_of_unix_ms: Option<u64>,
    digest: Option<&str>,
    knowledge_cut_unix_ms: u64,
    evidence_ref: &str,
    label: &str,
    findings: &mut BTreeSet<Finding>,
) {
    match status {
        AttestationStatus::Verified => {
            if !valid_identifier(identity_ref) {
                add_finding(
                    findings,
                    &format!("{label}_IDENTITY_MISSING"),
                    FindingClass::Invalid,
                    Some(evidence_ref),
                );
            }
            if !digest.is_some_and(valid_sha256) {
                add_finding(
                    findings,
                    &format!("{label}_DIGEST_INVALID"),
                    FindingClass::Invalid,
                    Some(evidence_ref),
                );
            }
            if as_of_unix_ms.is_none_or(|as_of| as_of > knowledge_cut_unix_ms) {
                add_finding(
                    findings,
                    &format!("{label}_NOT_KNOWN_AT_CLAIM_CUT"),
                    FindingClass::Unverified,
                    Some(evidence_ref),
                );
            }
        }
        AttestationStatus::Unverified | AttestationStatus::Disputed => {
            add_finding(
                findings,
                &format!("{label}_UNVERIFIED"),
                FindingClass::Unverified,
                Some(evidence_ref),
            );
        }
        AttestationStatus::Revoked => {
            add_finding(
                findings,
                &format!("{label}_REVOKED"),
                FindingClass::Invalid,
                Some(evidence_ref),
            );
        }
    }
}

fn validate_record(
    record: &EvidenceRecord,
    claim: &Claim,
    findings: &mut BTreeSet<Finding>,
) {
    let evidence_ref = record.evidence_ref.as_str();
    for (field, value) in [
        ("evidence_ref", record.evidence_ref.as_str()),
        ("role", record.role.as_str()),
        ("subject_ref", record.subject_ref.as_str()),
        ("configuration_ref", record.configuration_ref.as_str()),
        ("scope_ref", record.scope_ref.as_str()),
        (
            "uncertainty_budget_ref",
            record.uncertainty_budget.budget_ref.as_str(),
        ),
    ] {
        if !valid_identifier(value) {
            add_finding(
                findings,
                &format!("EVIDENCE_{field.to_ascii_uppercase()}_INVALID"),
                FindingClass::Invalid,
                Some(evidence_ref),
            );
        }
    }

    if !valid_sha256(&record.record_sha256) {
        add_finding(
            findings,
            "EVIDENCE_RECORD_DIGEST_INVALID",
            FindingClass::Invalid,
            Some(evidence_ref),
        );
    }

    if record.recorded_at_unix_ms > claim.knowledge_cut_unix_ms {
        add_finding(
            findings,
            "EVIDENCE_NOT_KNOWN_AT_CLAIM_CUT",
            FindingClass::Missing,
            Some(evidence_ref),
        );
    }

    if record
        .effective_until_exclusive_unix_ms
        .is_some_and(|end| end <= record.effective_from_unix_ms)
    {
        add_finding(
            findings,
            "EVIDENCE_EFFECTIVE_INTERVAL_INVALID",
            FindingClass::Invalid,
            Some(evidence_ref),
        );
    } else {
        let overlaps = record.effective_from_unix_ms <= claim.acquisition_end_unix_ms
            && record
                .effective_until_exclusive_unix_ms
                .is_none_or(|end| end > claim.acquisition_start_unix_ms);
        let covers = record.effective_from_unix_ms <= claim.acquisition_start_unix_ms
            && record
                .effective_until_exclusive_unix_ms
                .is_none_or(|end| end > claim.acquisition_end_unix_ms);

        if !overlaps {
            add_finding(
                findings,
                "EVIDENCE_NOT_EFFECTIVE_DURING_CLAIM_INTERVAL",
                FindingClass::OutOfScope,
                Some(evidence_ref),
            );
        } else if !covers {
            add_finding(
                findings,
                "EVIDENCE_EFFECTIVE_INTERVAL_DOES_NOT_COVER_CLAIM",
                FindingClass::Indeterminate,
                Some(evidence_ref),
            );
        }
    }

    validate_attestation(
        record.producer_authority.status,
        &record.producer_authority.identity_ref,
        record.producer_authority.as_of_unix_ms,
        record.producer_authority.record_sha256.as_deref(),
        claim.knowledge_cut_unix_ms,
        evidence_ref,
        "PRODUCER_AUTHORITY",
        findings,
    );
    validate_attestation(
        record.trust_snapshot.status,
        &record.trust_snapshot.policy_ref,
        record.trust_snapshot.as_of_unix_ms,
        record.trust_snapshot.policy_sha256.as_deref(),
        claim.knowledge_cut_unix_ms,
        evidence_ref,
        "TRUST_SNAPSHOT",
        findings,
    );

    if !valid_sha256(&record.uncertainty_budget.digest_sha256) {
        add_finding(
            findings,
            "UNCERTAINTY_BUDGET_DIGEST_INVALID",
            FindingClass::Invalid,
            Some(evidence_ref),
        );
    }
    if !record.uncertainty_budget.resolved {
        add_finding(
            findings,
            "UNCERTAINTY_BUDGET_UNRESOLVED",
            FindingClass::Indeterminate,
            Some(evidence_ref),
        );
    }
}

fn visit_dependency(
    evidence_ref: &str,
    depth: usize,
    claim: &Claim,
    by_ref: &BTreeMap<String, Vec<&EvidenceRecord>>,
    states: &mut BTreeMap<String, u8>,
    reached: &mut BTreeSet<String>,
    findings: &mut BTreeSet<Finding>,
) {
    if depth > MAX_GRAPH_DEPTH {
        add_finding(
            findings,
            "DEPENDENCY_GRAPH_DEPTH_LIMIT_EXCEEDED",
            FindingClass::Indeterminate,
            Some(evidence_ref),
        );
        return;
    }

    match states.get(evidence_ref).copied() {
        Some(1) => {
            add_finding(
                findings,
                "DEPENDENCY_CYCLE_DETECTED",
                FindingClass::Invalid,
                Some(evidence_ref),
            );
            return;
        }
        Some(2) => return,
        _ => {}
    }

    let Some(records) = by_ref.get(evidence_ref) else {
        add_finding(
            findings,
            "DEPENDENCY_EVIDENCE_MISSING",
            FindingClass::Missing,
            Some(evidence_ref),
        );
        return;
    };
    if records.len() != 1 {
        add_finding(
            findings,
            "DUPLICATE_EVIDENCE_REFERENCE",
            FindingClass::Invalid,
            Some(evidence_ref),
        );
        return;
    }

    let record = records[0];
    states.insert(evidence_ref.to_owned(), 1);
    reached.insert(evidence_ref.to_owned());
    validate_record(record, claim, findings);

    let mut dependencies = record.dependencies.iter().collect::<Vec<_>>();
    dependencies.sort_by(|left, right| {
        (
            left.evidence_ref.as_str(),
            left.expected_record_sha256.as_str(),
        )
            .cmp(&(
                right.evidence_ref.as_str(),
                right.expected_record_sha256.as_str(),
            ))
    });

    let mut dependency_refs = BTreeSet::new();
    for dependency in dependencies {
        if !dependency_refs.insert(dependency.evidence_ref.as_str()) {
            add_finding(
                findings,
                "DUPLICATE_DEPENDENCY_EDGE",
                FindingClass::Invalid,
                Some(evidence_ref),
            );
            continue;
        }

        if !valid_identifier(&dependency.evidence_ref)
            || !valid_sha256(&dependency.expected_record_sha256)
        {
            add_finding(
                findings,
                "DEPENDENCY_BINDING_INVALID",
                FindingClass::Invalid,
                Some(evidence_ref),
            );
            continue;
        }

        match by_ref.get(&dependency.evidence_ref) {
            None => {
                add_finding(
                    findings,
                    "DEPENDENCY_EVIDENCE_MISSING",
                    FindingClass::Missing,
                    Some(&dependency.evidence_ref),
                );
            }
            Some(parent_records) if parent_records.len() != 1 => {
                add_finding(
                    findings,
                    "DUPLICATE_EVIDENCE_REFERENCE",
                    FindingClass::Invalid,
                    Some(&dependency.evidence_ref),
                );
            }
            Some(parent_records) => {
                if parent_records[0].record_sha256 != dependency.expected_record_sha256 {
                    add_finding(
                        findings,
                        "DEPENDENCY_RECORD_DIGEST_MISMATCH",
                        FindingClass::Invalid,
                        Some(&dependency.evidence_ref),
                    );
                }
                visit_dependency(
                    &dependency.evidence_ref,
                    depth + 1,
                    claim,
                    by_ref,
                    states,
                    reached,
                    findings,
                );
            }
        }
    }

    states.insert(evidence_ref.to_owned(), 2);
}

pub fn evaluate(input: &ClaimClosureInput) -> Result<ClosureReport, String> {
    if input.evidence_records.len() > MAX_EVIDENCE_RECORDS {
        return Err(format!(
            "evidence record limit exceeded: {} > {}",
            input.evidence_records.len(),
            MAX_EVIDENCE_RECORDS
        ));
    }

    let mut edge_count = 0_usize;
    for record in &input.evidence_records {
        edge_count = edge_count
            .checked_add(record.dependencies.len())
            .ok_or_else(|| "dependency edge count overflow".to_owned())?;
    }
    if edge_count > MAX_DEPENDENCY_EDGES {
        return Err(format!(
            "dependency edge limit exceeded: {edge_count} > {MAX_DEPENDENCY_EDGES}"
        ));
    }

    let canonical_input_sha256 = canonical_input_sha256(input)?;
    let mut findings = BTreeSet::new();

    if input.schema_version != INPUT_SCHEMA_VERSION {
        add_finding(
            &mut findings,
            "INPUT_SCHEMA_VERSION_UNSUPPORTED",
            FindingClass::Invalid,
            None,
        );
    }

    validate_claim(&input.claim, &mut findings);

    let mut by_ref: BTreeMap<String, Vec<&EvidenceRecord>> = BTreeMap::new();
    for record in &input.evidence_records {
        by_ref
            .entry(record.evidence_ref.clone())
            .or_default()
            .push(record);
    }

    let mut requirement_refs = BTreeSet::new();
    let mut states = BTreeMap::new();
    let mut reached = BTreeSet::new();

    for requirement in &input.claim.requirements {
        if !requirement_refs.insert(requirement.evidence_ref.as_str()) {
            add_finding(
                &mut findings,
                "DUPLICATE_CLAIM_REQUIREMENT",
                FindingClass::Invalid,
                Some(&requirement.evidence_ref),
            );
            continue;
        }

        if !valid_identifier(&requirement.evidence_ref)
            || !valid_identifier(&requirement.role)
            || !valid_identifier(&requirement.uncertainty_budget_ref)
            || !valid_sha256(&requirement.expected_record_sha256)
            || !valid_sha256(&requirement.uncertainty_budget_sha256)
        {
            add_finding(
                &mut findings,
                "CLAIM_REQUIREMENT_BINDING_INVALID",
                FindingClass::Invalid,
                Some(&requirement.evidence_ref),
            );
            continue;
        }

        let Some(records) = by_ref.get(&requirement.evidence_ref) else {
            add_finding(
                &mut findings,
                "REQUIRED_EVIDENCE_MISSING",
                FindingClass::Missing,
                Some(&requirement.evidence_ref),
            );
            continue;
        };
        if records.len() != 1 {
            add_finding(
                &mut findings,
                "DUPLICATE_EVIDENCE_REFERENCE",
                FindingClass::Invalid,
                Some(&requirement.evidence_ref),
            );
            continue;
        }

        let record = records[0];
        if record.record_sha256 != requirement.expected_record_sha256 {
            add_finding(
                &mut findings,
                "REQUIRED_EVIDENCE_DIGEST_MISMATCH",
                FindingClass::Invalid,
                Some(&requirement.evidence_ref),
            );
        }
        if record.role != requirement.role {
            add_finding(
                &mut findings,
                "REQUIRED_EVIDENCE_ROLE_MISMATCH",
                FindingClass::Invalid,
                Some(&requirement.evidence_ref),
            );
        }
        if record.subject_ref != input.claim.subject_ref
            || record.configuration_ref != input.claim.configuration_ref
            || record.scope_ref != input.claim.scope_ref
        {
            add_finding(
                &mut findings,
                "REQUIRED_EVIDENCE_CLAIM_CONTEXT_MISMATCH",
                FindingClass::OutOfScope,
                Some(&requirement.evidence_ref),
            );
        }
        if record.uncertainty_budget.budget_ref != requirement.uncertainty_budget_ref
            || record.uncertainty_budget.digest_sha256
                != requirement.uncertainty_budget_sha256
        {
            add_finding(
                &mut findings,
                "REQUIRED_EVIDENCE_UNCERTAINTY_BINDING_MISMATCH",
                FindingClass::Invalid,
                Some(&requirement.evidence_ref),
            );
        }

        visit_dependency(
            &requirement.evidence_ref,
            0,
            &input.claim,
            &by_ref,
            &mut states,
            &mut reached,
            &mut findings,
        );
    }

    let findings = findings.into_iter().collect::<Vec<_>>();
    let disposition = if findings.is_empty() {
        ClosureDisposition::ClosedForIndependentReview
    } else {
        ClosureDisposition::Blocked
    };

    Ok(ClosureReport {
        schema_version: REPORT_SCHEMA_VERSION.to_owned(),
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        claim_id: input.claim.claim_id.clone(),
        canonical_input_sha256,
        disposition,
        reached_evidence_refs: reached.into_iter().collect(),
        findings,
    })
}

pub fn verify_report(
    input: &ClaimClosureInput,
    supplied_report: &ClosureReport,
) -> Result<bool, String> {
    let recomputed = evaluate(input)?;
    Ok(&recomputed == supplied_report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const POSITIVE_FIXTURE: &str = include_str!("../fixtures/positive-v0.1.json");

    fn digest(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn evidence_record(
        evidence_ref: &str,
        record_digest: char,
        role: &str,
        subject_ref: &str,
        configuration_ref: &str,
        scope_ref: &str,
        recorded_at_unix_ms: u64,
    ) -> EvidenceRecord {
        EvidenceRecord {
            evidence_ref: evidence_ref.to_owned(),
            record_sha256: digest(record_digest),
            role: role.to_owned(),
            subject_ref: subject_ref.to_owned(),
            configuration_ref: configuration_ref.to_owned(),
            scope_ref: scope_ref.to_owned(),
            recorded_at_unix_ms,
            effective_from_unix_ms: 90,
            effective_until_exclusive_unix_ms: Some(130),
            producer_authority: ProducerAuthority {
                identity_ref: format!("producer:{evidence_ref}"),
                status: AttestationStatus::Verified,
                as_of_unix_ms: Some(recorded_at_unix_ms),
                record_sha256: Some(digest('1')),
            },
            trust_snapshot: TrustSnapshot {
                policy_ref: "policy:engineering-v1".to_owned(),
                status: AttestationStatus::Verified,
                as_of_unix_ms: Some(recorded_at_unix_ms),
                policy_sha256: Some(digest('2')),
            },
            uncertainty_budget: UncertaintyBudget {
                budget_ref: format!("uncertainty:{evidence_ref}"),
                digest_sha256: digest('3'),
                resolved: true,
            },
            dependencies: Vec::new(),
        }
    }

    fn sample_input() -> ClaimClosureInput {
        let mut measurement = evidence_record(
            "evidence:measurement",
            'a',
            "measurement",
            "part:wing-001",
            "configuration:r17",
            "scope:hot-static",
            150,
        );
        measurement.dependencies.push(DependencyBinding {
            evidence_ref: "evidence:calibration".to_owned(),
            expected_record_sha256: digest('b'),
        });

        let mut calibration = evidence_record(
            "evidence:calibration",
            'b',
            "calibration",
            "instrument:load-cell-01",
            "configuration:calibration-v3",
            "scope:force-range-100kn",
            110,
        );
        calibration.dependencies.push(DependencyBinding {
            evidence_ref: "evidence:reference".to_owned(),
            expected_record_sha256: digest('c'),
        });

        let reference = evidence_record(
            "evidence:reference",
            'c',
            "reference_standard",
            "standard:force-100kn",
            "configuration:reference-v2",
            "scope:force-range-100kn",
            90,
        );

        ClaimClosureInput {
            schema_version: INPUT_SCHEMA_VERSION.to_owned(),
            claim: Claim {
                claim_id: "claim:wing-panel-001".to_owned(),
                subject_ref: "part:wing-001".to_owned(),
                configuration_ref: "configuration:r17".to_owned(),
                scope_ref: "scope:hot-static".to_owned(),
                acquisition_start_unix_ms: 100,
                acquisition_end_unix_ms: 120,
                knowledge_cut_unix_ms: 160,
                evaluated_at_unix_ms: 180,
                requirements: vec![EvidenceRequirement {
                    evidence_ref: "evidence:measurement".to_owned(),
                    expected_record_sha256: digest('a'),
                    role: "measurement".to_owned(),
                    uncertainty_budget_ref: "uncertainty:evidence:measurement".to_owned(),
                    uncertainty_budget_sha256: digest('3'),
                }],
            },
            evidence_records: vec![measurement, calibration, reference],
        }
    }

    fn has_code(report: &ClosureReport, code: &str) -> bool {
        report.findings.iter().any(|finding| finding.code == code)
    }

    fn record_mut<'a>(
        input: &'a mut ClaimClosureInput,
        evidence_ref: &str,
    ) -> &'a mut EvidenceRecord {
        input
            .evidence_records
            .iter_mut()
            .find(|record| record.evidence_ref == evidence_ref)
            .expect("record exists")
    }

    #[test]
    fn frozen_positive_fixture_is_closed_for_independent_review() {
        let input: ClaimClosureInput =
            serde_json::from_str(POSITIVE_FIXTURE).expect("positive fixture parses");
        let report = evaluate(&input).expect("evaluation succeeds");
        assert_eq!(
            report.disposition,
            ClosureDisposition::ClosedForIndependentReview
        );
        assert_eq!(report.reached_evidence_refs.len(), 3);
        assert!(verify_report(&input, &report).unwrap());
    }

    #[test]
    fn recursively_closed_graph_is_accepted() {
        let report = evaluate(&sample_input()).unwrap();
        assert_eq!(
            report.disposition,
            ClosureDisposition::ClosedForIndependentReview
        );
        assert_eq!(
            report.reached_evidence_refs,
            vec![
                "evidence:calibration".to_owned(),
                "evidence:measurement".to_owned(),
                "evidence:reference".to_owned()
            ]
        );
    }

    #[test]
    fn evidence_record_order_does_not_change_the_report() {
        let input = sample_input();
        let original = evaluate(&input).unwrap();
        let mut permuted = input;
        permuted.evidence_records.reverse();
        let repeated = evaluate(&permuted).unwrap();
        assert_eq!(original, repeated);
    }

    #[test]
    fn dependency_order_does_not_change_the_report() {
        let mut input = sample_input();
        input.evidence_records[0].dependencies.push(DependencyBinding {
            evidence_ref: "evidence:calibration".to_owned(),
            expected_record_sha256: digest('b'),
        });
        let mut first = input.clone();
        let original = evaluate(&first).unwrap();
        first.evidence_records[0].dependencies.reverse();
        let reordered = evaluate(&first).unwrap();
        assert_eq!(original, reordered);
        assert!(has_code(&reordered, "DUPLICATE_DEPENDENCY_EDGE"));
    }

    #[test]
    fn missing_required_evidence_blocks_closure() {
        let mut input = sample_input();
        input.evidence_records.retain(|record| record.evidence_ref != "evidence:measurement");
        let report = evaluate(&input).unwrap();
        assert_eq!(report.disposition, ClosureDisposition::Blocked);
        assert!(has_code(&report, "REQUIRED_EVIDENCE_MISSING"));
    }

    #[test]
    fn missing_transitive_ancestor_blocks_closure() {
        let mut input = sample_input();
        input.evidence_records.retain(|record| record.evidence_ref != "evidence:reference");
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "DEPENDENCY_EVIDENCE_MISSING"));
    }

    #[test]
    fn required_record_digest_substitution_blocks_closure() {
        let mut input = sample_input();
        input.claim.requirements[0].expected_record_sha256 = digest('f');
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "REQUIRED_EVIDENCE_DIGEST_MISMATCH"));
    }

    #[test]
    fn dependency_digest_substitution_blocks_closure() {
        let mut input = sample_input();
        input.evidence_records[0].dependencies[0].expected_record_sha256 = digest('f');
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "DEPENDENCY_RECORD_DIGEST_MISMATCH"));
    }

    #[test]
    fn role_substitution_is_invalid() {
        let mut input = sample_input();
        input.evidence_records[0].role = "model_output".to_owned();
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "REQUIRED_EVIDENCE_ROLE_MISMATCH"));
    }

    #[test]
    fn subject_substitution_is_out_of_scope() {
        let mut input = sample_input();
        input.evidence_records[0].subject_ref = "part:other".to_owned();
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "REQUIRED_EVIDENCE_CLAIM_CONTEXT_MISMATCH"));
        assert_eq!(
            report
                .findings
                .iter()
                .find(|finding| finding.code == "REQUIRED_EVIDENCE_CLAIM_CONTEXT_MISMATCH")
                .unwrap()
                .class,
            FindingClass::OutOfScope
        );
    }

    #[test]
    fn configuration_substitution_is_out_of_scope() {
        let mut input = sample_input();
        input.evidence_records[0].configuration_ref = "configuration:r18".to_owned();
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "REQUIRED_EVIDENCE_CLAIM_CONTEXT_MISMATCH"));
    }

    #[test]
    fn scope_substitution_is_out_of_scope() {
        let mut input = sample_input();
        input.evidence_records[0].scope_ref = "scope:cold-static".to_owned();
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "REQUIRED_EVIDENCE_CLAIM_CONTEXT_MISMATCH"));
    }

    #[test]
    fn evidence_recorded_after_knowledge_cut_is_not_admissible() {
        let mut input = sample_input();
        record_mut(&mut input, "evidence:measurement").recorded_at_unix_ms = 161;
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "EVIDENCE_NOT_KNOWN_AT_CLAIM_CUT"));
    }

    #[test]
    fn future_trust_snapshot_is_not_admissible_at_claim_cut() {
        let mut input = sample_input();
        record_mut(&mut input, "evidence:measurement")
            .trust_snapshot
            .as_of_unix_ms = Some(161);
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "TRUST_SNAPSHOT_NOT_KNOWN_AT_CLAIM_CUT"));
    }

    #[test]
    fn revoked_producer_authority_is_invalid() {
        let mut input = sample_input();
        record_mut(&mut input, "evidence:measurement")
            .producer_authority
            .status = AttestationStatus::Revoked;
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "PRODUCER_AUTHORITY_REVOKED"));
    }

    #[test]
    fn unverified_trust_snapshot_cannot_close_the_graph() {
        let mut input = sample_input();
        record_mut(&mut input, "evidence:measurement")
            .trust_snapshot
            .status = AttestationStatus::Unverified;
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "TRUST_SNAPSHOT_UNVERIFIED"));
    }

    #[test]
    fn missing_verified_producer_receipt_digest_is_invalid() {
        let mut input = sample_input();
        record_mut(&mut input, "evidence:measurement")
            .producer_authority
            .record_sha256 = None;
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "PRODUCER_AUTHORITY_DIGEST_INVALID"));
    }

    #[test]
    fn unresolved_uncertainty_budget_is_indeterminate() {
        let mut input = sample_input();
        record_mut(&mut input, "evidence:calibration")
            .uncertainty_budget
            .resolved = false;
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "UNCERTAINTY_BUDGET_UNRESOLVED"));
        assert_eq!(
            report
                .findings
                .iter()
                .find(|finding| finding.code == "UNCERTAINTY_BUDGET_UNRESOLVED")
                .unwrap()
                .class,
            FindingClass::Indeterminate
        );
    }

    #[test]
    fn uncertainty_budget_identity_substitution_is_invalid() {
        let mut input = sample_input();
        input.claim.requirements[0].uncertainty_budget_ref = "uncertainty:other".to_owned();
        let report = evaluate(&input).unwrap();
        assert!(has_code(
            &report,
            "REQUIRED_EVIDENCE_UNCERTAINTY_BINDING_MISMATCH"
        ));
    }

    #[test]
    fn partially_overlapping_effective_interval_is_indeterminate() {
        let mut input = sample_input();
        record_mut(&mut input, "evidence:calibration").effective_from_unix_ms = 110;
        let report = evaluate(&input).unwrap();
        assert!(has_code(
            &report,
            "EVIDENCE_EFFECTIVE_INTERVAL_DOES_NOT_COVER_CLAIM"
        ));
    }

    #[test]
    fn exclusive_effectivity_end_must_be_after_acquisition_end() {
        let mut input = sample_input();
        record_mut(&mut input, "evidence:measurement")
            .effective_until_exclusive_unix_ms = Some(120);
        let report = evaluate(&input).unwrap();
        assert!(has_code(
            &report,
            "EVIDENCE_EFFECTIVE_INTERVAL_DOES_NOT_COVER_CLAIM"
        ));
    }

    #[test]
    fn non_overlapping_effective_interval_is_out_of_scope() {
        let mut input = sample_input();
        record_mut(&mut input, "evidence:calibration").effective_from_unix_ms = 140;
        let report = evaluate(&input).unwrap();
        assert!(has_code(
            &report,
            "EVIDENCE_NOT_EFFECTIVE_DURING_CLAIM_INTERVAL"
        ));
    }

    #[test]
    fn dependency_cycle_is_rejected() {
        let mut input = sample_input();
        input.evidence_records[2].dependencies.push(DependencyBinding {
            evidence_ref: "evidence:measurement".to_owned(),
            expected_record_sha256: digest('a'),
        });
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "DEPENDENCY_CYCLE_DETECTED"));
    }

    #[test]
    fn duplicate_evidence_reference_is_rejected() {
        let mut input = sample_input();
        input.evidence_records.push(input.evidence_records[0].clone());
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "DUPLICATE_EVIDENCE_REFERENCE"));
    }

    #[test]
    fn duplicate_requirement_is_rejected() {
        let mut input = sample_input();
        input.claim.requirements.push(input.claim.requirements[0].clone());
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "DUPLICATE_CLAIM_REQUIREMENT"));
    }

    #[test]
    fn report_tampering_is_detected_by_independent_recomputation() {
        let input = sample_input();
        let mut report = evaluate(&input).unwrap();
        report.reached_evidence_refs.push("evidence:forged".to_owned());
        assert!(!verify_report(&input, &report).unwrap());
    }

    #[test]
    fn changing_input_after_report_creation_invalidates_report() {
        let mut input = sample_input();
        let report = evaluate(&input).unwrap();
        record_mut(&mut input, "evidence:measurement").recorded_at_unix_ms = 155;
        assert!(!verify_report(&input, &report).unwrap());
    }

    #[test]
    fn unrelated_evidence_does_not_enter_the_reachable_closure() {
        let mut input = sample_input();
        input.evidence_records.push(evidence_record(
            "evidence:unrelated",
            'f',
            "unrelated",
            "part:unrelated",
            "configuration:other",
            "scope:other",
            100,
        ));
        let report = evaluate(&input).unwrap();
        assert_eq!(report.disposition, ClosureDisposition::ClosedForIndependentReview);
        assert_eq!(report.reached_evidence_refs.len(), 3);
        assert!(!report.reached_evidence_refs.contains(&"evidence:unrelated".to_owned()));
    }

    #[test]
    fn empty_requirement_set_is_not_vacuously_closed() {
        let mut input = sample_input();
        input.claim.requirements.clear();
        let report = evaluate(&input).unwrap();
        assert_eq!(report.disposition, ClosureDisposition::Blocked);
        assert!(has_code(&report, "CLAIM_HAS_NO_REQUIRED_EVIDENCE"));
    }

    #[test]
    fn reversed_acquisition_interval_is_invalid() {
        let mut input = sample_input();
        input.claim.acquisition_start_unix_ms = 121;
        input.claim.acquisition_end_unix_ms = 120;
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "CLAIM_ACQUISITION_INTERVAL_REVERSED"));
    }

    #[test]
    fn graph_depth_limit_fails_closed_without_unbounded_recursion() {
        let mut input = sample_input();
        input.claim.requirements[0].evidence_ref = "evidence:chain-0".to_owned();
        input.claim.requirements[0].expected_record_sha256 = digest('a');
        let mut chain = Vec::new();
        for index in 0..=MAX_GRAPH_DEPTH + 2 {
            let character = char::from_u32('a' as u32 + (index % 20) as u32).unwrap();
            let mut record = evidence_record(
                &format!("evidence:chain-{index}"),
                character,
                "chain",
                "part:wing-001",
                "configuration:r17",
                "scope:hot-static",
                100,
            );
            if index < MAX_GRAPH_DEPTH + 2 {
                let next_character =
                    char::from_u32('a' as u32 + ((index + 1) % 20) as u32).unwrap();
                record.dependencies.push(DependencyBinding {
                    evidence_ref: format!("evidence:chain-{}", index + 1),
                    expected_record_sha256: digest(next_character),
                });
            }
            chain.push(record);
        }
        input.evidence_records = chain;
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "DEPENDENCY_GRAPH_DEPTH_LIMIT_EXCEEDED"));
    }

    #[test]
    fn unsupported_schema_version_is_invalid() {
        let mut input = sample_input();
        input.schema_version = "engineering-claim-closure-v9".to_owned();
        let report = evaluate(&input).unwrap();
        assert!(has_code(&report, "INPUT_SCHEMA_VERSION_UNSUPPORTED"));
    }

    #[test]
    fn canonical_input_digest_changes_when_semantics_change() {
        let input = sample_input();
        let before = canonical_input_sha256(&input).unwrap();
        let mut changed = input;
        changed.claim.scope_ref = "scope:other".to_owned();
        let after = canonical_input_sha256(&changed).unwrap();
        assert_ne!(before, after);
    }
}
