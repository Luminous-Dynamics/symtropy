//! Independent precondition checks for route-qualified production planning.
//!
//! These checks bind a scheduled job to an exact route receipt, material lot,
//! input mass, receiving specification and receipt digest. They do not
//! authenticate the receipt issuer, fetch or hash its source artifact, or
//! establish the underlying physical facts. The upstream evidence verifier
//! must verify the receipt bytes before a route is admitted here.

use std::collections::{BTreeMap, BTreeSet};

use crate::{Finding, Problem, Schedule, ValidationReport, validate};

/// Route receipt state exported by the upstream evidence-qualified recovery system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteReceiptStatus {
    /// All route-specific evidence gates passed.
    Eligible,
    /// More work, inspection or a conditional waiver is required.
    Conditional,
    /// Required evidence is missing or unresolved.
    InsufficientEvidence,
    /// A hard route requirement or receiving test failed.
    Ineligible,
}

/// Route precondition pinned to one scheduled production job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteRequirement {
    /// Production job identifier from the frozen scheduling problem.
    pub job_id: String,
    /// Exact physical lot identifier.
    pub lot_id: String,
    /// Candidate route identifier, versioned by the route vocabulary owner.
    pub route_id: String,
    /// Versioned receiving-process specification required by this job.
    pub receiving_specification_id: String,
    /// Independently measured incoming lot mass in milligrams.
    pub input_mass_mg: u64,
    /// Exact upstream route receipt identifier.
    pub receipt_id: String,
    /// Expected SHA-256 digest of the exact route receipt bytes.
    pub receipt_sha256: String,
}

/// Upstream route qualification receipt consumed by the schedule validator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteReceipt {
    /// Route receipt schema/version.
    pub receipt_version: String,
    /// Stable receipt identifier.
    pub receipt_id: String,
    /// Physical material lot identifier.
    pub lot_id: String,
    /// Qualified route identifier.
    pub route_id: String,
    /// Exact receiving-process specification identifier/version.
    pub receiving_specification_id: String,
    /// Incoming mass used by the upstream route qualification, in milligrams.
    pub input_mass_mg: u64,
    /// Upstream route qualification disposition.
    pub status: RouteReceiptStatus,
    /// SHA-256 digest of this exact serialized receipt.
    pub receipt_sha256: String,
    /// SHA-256 of the evidence manifest consumed by the upstream verifier.
    pub evidence_manifest_sha256: String,
    /// Version of the upstream route qualification contract.
    pub source_contract_version: String,
    /// Issuer responsible for the receipt.
    pub issuer_id: String,
    /// Independent verifier identity recorded by the upstream system.
    pub verifier_id: String,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

fn route_finding(
    findings: &mut Vec<Finding>,
    code: &'static str,
    entity_id: impl Into<String>,
    message: impl Into<String>,
) {
    findings.push(Finding {
        code,
        entity_id: entity_id.into(),
        message: message.into(),
    });
}

/// Validates lot/route/specification eligibility for all jobs in a textile problem.
///
/// Every job must have exactly one route requirement. The referenced receipt
/// must be unique, exactly match the required lot/route/specification/input
/// mass/digest, and have status Eligible. Conditional, unknown and rejected
/// routes fail closed. This function performs structural binding only; it does
/// not recompute the SHA-256 of an artifact it has not been given.
pub fn validate_route_gates(
    problem: &Problem,
    requirements: &[RouteRequirement],
    receipts: &[RouteReceipt],
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let jobs: BTreeSet<&str> = problem
        .operations
        .iter()
        .map(|operation| operation.job_id.as_str())
        .collect();

    let mut requirements_by_job = BTreeMap::<&str, Vec<&RouteRequirement>>::new();
    for requirement in requirements {
        requirements_by_job
            .entry(requirement.job_id.as_str())
            .or_default()
            .push(requirement);
        if !jobs.contains(requirement.job_id.as_str()) {
            route_finding(
                &mut findings,
                "ROUTE_REQUIREMENT_UNKNOWN_JOB",
                &requirement.job_id,
                "route requirement references a job absent from the frozen problem",
            );
        }
    }

    for job_id in &jobs {
        match requirements_by_job.get(job_id).map(Vec::len).unwrap_or(0) {
            0 => route_finding(
                &mut findings,
                "ROUTE_REQUIREMENT_MISSING",
                *job_id,
                "every scheduled textile job requires exactly one qualified route",
            ),
            1 => {}
            _ => route_finding(
                &mut findings,
                "ROUTE_REQUIREMENT_DUPLICATE",
                *job_id,
                "job has more than one route requirement; choose one route before scheduling",
            ),
        }
    }

    let mut receipts_by_id = BTreeMap::<&str, Vec<&RouteReceipt>>::new();
    for receipt in receipts {
        receipts_by_id
            .entry(receipt.receipt_id.as_str())
            .or_default()
            .push(receipt);
    }
    for (receipt_id, matches) in &receipts_by_id {
        if matches.len() > 1 {
            route_finding(
                &mut findings,
                "ROUTE_RECEIPT_DUPLICATE",
                *receipt_id,
                "route receipt IDs must be unique in the validation input",
            );
        }
    }

    for requirement in requirements {
        let entity = if requirement.job_id.trim().is_empty() {
            "<empty-job-id>"
        } else {
            requirement.job_id.as_str()
        };
        if requirement.lot_id.trim().is_empty()
            || requirement.route_id.trim().is_empty()
            || requirement.receiving_specification_id.trim().is_empty()
            || requirement.receipt_id.trim().is_empty()
            || requirement.input_mass_mg == 0
        {
            route_finding(
                &mut findings,
                "ROUTE_REQUIREMENT_INVALID",
                entity,
                "lot, route, receiving specification, receipt ID and positive input mass are required",
            );
        }
        if !valid_sha256(&requirement.receipt_sha256) {
            route_finding(
                &mut findings,
                "ROUTE_REQUIREMENT_INVALID_RECEIPT_DIGEST",
                entity,
                "expected receipt digest must be 64 lowercase hexadecimal characters",
            );
        }

        let Some(matches) = receipts_by_id.get(requirement.receipt_id.as_str()) else {
            route_finding(
                &mut findings,
                "ROUTE_RECEIPT_MISSING",
                entity,
                format!(
                    "required route receipt {} is absent",
                    requirement.receipt_id
                ),
            );
            continue;
        };
        if matches.len() != 1 {
            continue;
        }
        let receipt = matches[0];

        if receipt.status != RouteReceiptStatus::Eligible {
            route_finding(
                &mut findings,
                "ROUTE_NOT_ELIGIBLE",
                entity,
                format!(
                    "route receipt {} has non-eligible status {:?}",
                    receipt.receipt_id, receipt.status
                ),
            );
        }
        if receipt.receipt_version.trim().is_empty()
            || receipt.source_contract_version.trim().is_empty()
            || receipt.issuer_id.trim().is_empty()
            || receipt.verifier_id.trim().is_empty()
            || receipt.issuer_id == receipt.verifier_id
        {
            route_finding(
                &mut findings,
                "ROUTE_RECEIPT_PROVENANCE_INVALID",
                entity,
                "receipt version, contract version and distinct non-empty issuer/verifier IDs are required",
            );
        }
        if !valid_sha256(&receipt.receipt_sha256)
            || !valid_sha256(&receipt.evidence_manifest_sha256)
        {
            route_finding(
                &mut findings,
                "ROUTE_RECEIPT_DIGEST_INVALID",
                entity,
                "receipt and evidence-manifest digests must be lowercase SHA-256 values",
            );
        }
        if receipt.receipt_sha256 != requirement.receipt_sha256 {
            route_finding(
                &mut findings,
                "ROUTE_RECEIPT_DIGEST_MISMATCH",
                entity,
                "schedule requirement does not pin the supplied receipt bytes",
            );
        }
        if receipt.lot_id != requirement.lot_id {
            route_finding(
                &mut findings,
                "ROUTE_LOT_MISMATCH",
                entity,
                "route receipt lot differs from the scheduled lot",
            );
        }
        if receipt.route_id != requirement.route_id {
            route_finding(
                &mut findings,
                "ROUTE_ID_MISMATCH",
                entity,
                "route receipt does not qualify the scheduled route",
            );
        }
        if receipt.receiving_specification_id != requirement.receiving_specification_id {
            route_finding(
                &mut findings,
                "ROUTE_SPECIFICATION_MISMATCH",
                entity,
                "route receipt was qualified against a different receiving specification",
            );
        }
        if receipt.input_mass_mg != requirement.input_mass_mg {
            route_finding(
                &mut findings,
                "ROUTE_INPUT_MASS_MISMATCH",
                entity,
                "route receipt input mass differs from the scheduled physical lot mass",
            );
        }
    }

    findings.sort_by(|left, right| {
        (&left.code, &left.entity_id, &left.message).cmp(&(
            &right.code,
            &right.entity_id,
            &right.message,
        ))
    });
    findings
}

/// Validates schedule feasibility and route evidence gates as one report.
///
/// A schedule is valid only when both the ordinary machine/operator/precedence
/// checks and all route qualification preconditions pass.
pub fn validate_with_route_gates(
    problem: &Problem,
    schedule: &Schedule,
    requirements: &[RouteRequirement],
    receipts: &[RouteReceipt],
) -> ValidationReport {
    let mut report = validate(problem, schedule);
    report
        .findings
        .extend(validate_route_gates(problem, requirements, receipts));
    report.findings.sort_by(|left, right| {
        (&left.code, &left.entity_id, &left.message).cmp(&(
            &right.code,
            &right.entity_id,
            &right.message,
        ))
    });
    report.valid = report.findings.is_empty();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Operation, Problem};

    fn problem() -> Problem {
        Problem {
            name: "route-gate-fixture".to_owned(),
            machines: Vec::new(),
            operators: Vec::new(),
            operations: vec![
                Operation {
                    id: "JOB-1-OP-1".to_owned(),
                    job_id: "JOB-1".to_owned(),
                    stage_index: 0,
                    duration_ticks: 2,
                    release_tick: 0,
                    due_tick: None,
                    required_capability: "inspect".to_owned(),
                    required_skill: "textile-inspection".to_owned(),
                    required_certification: None,
                    product_family: "cotton".to_owned(),
                },
                Operation {
                    id: "JOB-2-OP-1".to_owned(),
                    job_id: "JOB-2".to_owned(),
                    stage_index: 0,
                    duration_ticks: 2,
                    release_tick: 0,
                    due_tick: None,
                    required_capability: "inspect".to_owned(),
                    required_skill: "textile-inspection".to_owned(),
                    required_certification: None,
                    product_family: "polyester".to_owned(),
                },
            ],
            precedences: Vec::new(),
        }
    }

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn receipt(status: RouteReceiptStatus) -> RouteReceipt {
        RouteReceipt {
            receipt_version: "textile-recovery-receipt-v0.1".to_owned(),
            receipt_id: "ROUTE-RECEIPT-1".to_owned(),
            lot_id: "LOT-1".to_owned(),
            route_id: "mechanical_fibre_recycling".to_owned(),
            receiving_specification_id: "SPINNING-SPEC-V1".to_owned(),
            input_mass_mg: 1_000_000,
            status,
            receipt_sha256: digest('a'),
            evidence_manifest_sha256: digest('b'),
            source_contract_version: "symthaea-textile-recovery-v0.1".to_owned(),
            issuer_id: "Luminous-Symthaea".to_owned(),
            verifier_id: "Independent-Receiver".to_owned(),
        }
    }

    fn requirement(job_id: &str) -> RouteRequirement {
        RouteRequirement {
            job_id: job_id.to_owned(),
            lot_id: "LOT-1".to_owned(),
            route_id: "mechanical_fibre_recycling".to_owned(),
            receiving_specification_id: "SPINNING-SPEC-V1".to_owned(),
            input_mass_mg: 1_000_000,
            receipt_id: "ROUTE-RECEIPT-1".to_owned(),
            receipt_sha256: digest('a'),
        }
    }

    #[test]
    fn eligible_receipt_binds_exact_lot_route_spec_mass_and_digest() {
        let p = problem();
        let requirements = vec![requirement("JOB-1"), {
            let mut value = requirement("JOB-2");
            value.receipt_id = "ROUTE-RECEIPT-2".to_owned();
            value.receipt_sha256 = digest('c');
            value.lot_id = "LOT-2".to_owned();
            value.route_id = "reuse_as_is".to_owned();
            value.receiving_specification_id = "REUSE-ACCEPTANCE-V1".to_owned();
            value.input_mass_mg = 400_000;
            value
        }];
        let mut second = receipt(RouteReceiptStatus::Eligible);
        second.receipt_id = "ROUTE-RECEIPT-2".to_owned();
        second.receipt_sha256 = digest('c');
        second.lot_id = "LOT-2".to_owned();
        second.route_id = "reuse_as_is".to_owned();
        second.receiving_specification_id = "REUSE-ACCEPTANCE-V1".to_owned();
        second.input_mass_mg = 400_000;
        assert!(
            validate_route_gates(
                &p,
                &requirements,
                &[receipt(RouteReceiptStatus::Eligible), second]
            )
            .is_empty()
        );
    }

    #[test]
    fn every_job_requires_a_route_receipt() {
        let findings = validate_route_gates(&problem(), &[], &[]);
        assert!(
            findings
                .iter()
                .any(|f| f.code == "ROUTE_REQUIREMENT_MISSING")
        );
    }

    #[test]
    fn missing_upstream_receipt_blocks_job() {
        let findings = validate_route_gates(
            &problem(),
            &[requirement("JOB-1"), requirement("JOB-2")],
            &[],
        );
        assert!(findings.iter().any(|f| f.code == "ROUTE_RECEIPT_MISSING"));
    }

    #[test]
    fn conditional_route_is_not_schedulable() {
        let findings = validate_route_gates(
            &problem(),
            &[requirement("JOB-1")],
            &[receipt(RouteReceiptStatus::Conditional)],
        );
        assert!(findings.iter().any(|f| f.code == "ROUTE_NOT_ELIGIBLE"));
    }

    #[test]
    fn insufficient_evidence_route_is_not_schedulable() {
        let findings = validate_route_gates(
            &problem(),
            &[requirement("JOB-1")],
            &[receipt(RouteReceiptStatus::InsufficientEvidence)],
        );
        assert!(findings.iter().any(|f| f.code == "ROUTE_NOT_ELIGIBLE"));
    }

    #[test]
    fn ineligible_route_is_not_schedulable() {
        let findings = validate_route_gates(
            &problem(),
            &[requirement("JOB-1")],
            &[receipt(RouteReceiptStatus::Ineligible)],
        );
        assert!(findings.iter().any(|f| f.code == "ROUTE_NOT_ELIGIBLE"));
    }

    #[test]
    fn stale_or_different_receipt_digest_is_rejected() {
        let mut req = requirement("JOB-1");
        req.receipt_sha256 = digest('c');
        let findings = validate_route_gates(
            &problem(),
            &[req],
            &[receipt(RouteReceiptStatus::Eligible)],
        );
        assert!(
            findings
                .iter()
                .any(|f| f.code == "ROUTE_RECEIPT_DIGEST_MISMATCH")
        );
    }

    #[test]
    fn lot_mismatch_is_rejected() {
        let mut value = receipt(RouteReceiptStatus::Eligible);
        value.lot_id = "LOT-OTHER".to_owned();
        let findings = validate_route_gates(&problem(), &[requirement("JOB-1")], &[value]);
        assert!(findings.iter().any(|f| f.code == "ROUTE_LOT_MISMATCH"));
    }

    #[test]
    fn route_mismatch_is_rejected() {
        let mut value = receipt(RouteReceiptStatus::Eligible);
        value.route_id = "chemical_fibre_recycling".to_owned();
        let findings = validate_route_gates(&problem(), &[requirement("JOB-1")], &[value]);
        assert!(findings.iter().any(|f| f.code == "ROUTE_ID_MISMATCH"));
    }

    #[test]
    fn receiving_specification_mismatch_is_rejected() {
        let mut value = receipt(RouteReceiptStatus::Eligible);
        value.receiving_specification_id = "OTHER-SPEC".to_owned();
        let findings = validate_route_gates(&problem(), &[requirement("JOB-1")], &[value]);
        assert!(
            findings
                .iter()
                .any(|f| f.code == "ROUTE_SPECIFICATION_MISMATCH")
        );
    }

    #[test]
    fn input_mass_mismatch_is_rejected() {
        let mut value = receipt(RouteReceiptStatus::Eligible);
        value.input_mass_mg = 900_000;
        let findings = validate_route_gates(&problem(), &[requirement("JOB-1")], &[value]);
        assert!(
            findings
                .iter()
                .any(|f| f.code == "ROUTE_INPUT_MASS_MISMATCH")
        );
    }

    #[test]
    fn malformed_receipt_or_manifest_digest_is_rejected() {
        let mut value = receipt(RouteReceiptStatus::Eligible);
        value.evidence_manifest_sha256 = "invalid".to_owned();
        let findings = validate_route_gates(&problem(), &[requirement("JOB-1")], &[value]);
        assert!(
            findings
                .iter()
                .any(|f| f.code == "ROUTE_RECEIPT_DIGEST_INVALID")
        );
    }

    #[test]
    fn same_issuer_and_verifier_is_not_independent() {
        let mut value = receipt(RouteReceiptStatus::Eligible);
        value.verifier_id = value.issuer_id.clone();
        let findings = validate_route_gates(&problem(), &[requirement("JOB-1")], &[value]);
        assert!(
            findings
                .iter()
                .any(|f| f.code == "ROUTE_RECEIPT_PROVENANCE_INVALID")
        );
    }

    #[test]
    fn duplicate_route_requirements_for_a_job_are_rejected() {
        let req = requirement("JOB-1");
        let findings = validate_route_gates(
            &problem(),
            &[req.clone(), req],
            &[receipt(RouteReceiptStatus::Eligible)],
        );
        assert!(
            findings
                .iter()
                .any(|f| f.code == "ROUTE_REQUIREMENT_DUPLICATE")
        );
    }

    #[test]
    fn duplicate_receipt_ids_are_rejected() {
        let value = receipt(RouteReceiptStatus::Eligible);
        let findings =
            validate_route_gates(&problem(), &[requirement("JOB-1")], &[value.clone(), value]);
        assert!(findings.iter().any(|f| f.code == "ROUTE_RECEIPT_DUPLICATE"));
    }

    #[test]
    fn route_requirement_for_unknown_job_is_rejected() {
        let findings = validate_route_gates(
            &problem(),
            &[requirement("JOB-NOT-IN-PROBLEM")],
            &[receipt(RouteReceiptStatus::Eligible)],
        );
        assert!(
            findings
                .iter()
                .any(|f| f.code == "ROUTE_REQUIREMENT_UNKNOWN_JOB")
        );
    }

    #[test]
    fn combined_validation_includes_route_gate_findings() {
        let p = problem();
        let report = validate_with_route_gates(
            &p,
            &Schedule {
                assignments: Vec::new(),
                setup_reservations: Vec::new(),
            },
            &[],
            &[],
        );
        assert!(!report.valid);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.code == "ROUTE_REQUIREMENT_MISSING")
        );
    }
}
