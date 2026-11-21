pub mod event_replay;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt::Write as _;

const CORPUS_SCHEMA: &str = "luminous.orbital-interface-lifecycle.corpus.v0.1";
const EVIDENCE_CLASS: &str = "synthetic";
const CLAIM_CEILING: &str = "synthetic-lifecycle-state-semantics-only";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    MechanicalCapture,
    PreparedStorableFluid,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    QualifiedWithinDeclaredEnvelope,
    Restricted,
    RequalificationRequired,
    Withdrawn,
    Rejected,
    Indeterminate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<LifecycleCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleCase {
    pub case_id: String,
    pub at_tick: u64,
    pub pair_id: String,
    pub qualified_pair_id: String,
    pub service_kind: ServiceKind,
    pub qualification_service_kind: ServiceKind,
    pub provider_configuration: String,
    pub qualified_configuration: String,
    pub current_procedure_revision: String,
    pub qualified_procedure_revision: String,
    pub cycle_count: u64,
    pub qualified_cycle_limit: Option<u64>,
    pub inspection_age_ticks: Option<u64>,
    pub inspection_interval_ticks: Option<u64>,
    pub qualification_valid_until_tick: Option<u64>,
    pub calibration_valid: Option<bool>,
    pub wear_within_limit: Option<bool>,
    pub environment_within_envelope: Option<bool>,
    pub uncertainty_clears_margin: Option<bool>,
    pub inspection_coverage_complete: Option<bool>,
    pub lineage_reconciled: Option<bool>,
    pub raw_artifact_valid: Option<bool>,
    pub critical_defect_for_service: bool,
    pub maintenance_after_qualification: bool,
    pub qualification_revoked: bool,
    pub fluid_interface_observation_present: bool,
    pub restricted: bool,
    pub expected_status: Disposition,
    pub expected_reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub actual_status: Disposition,
    pub actual_reason: String,
    pub expected_status: Disposition,
    pub expected_reason: String,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub qualified_within_declared_envelope: usize,
    pub restricted: usize,
    pub requalification_required: usize,
    pub withdrawn: usize,
    pub rejected: usize,
    pub indeterminate: usize,
    pub expected_result_mismatches: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Decision {
    status: Disposition,
    reason: &'static str,
}

impl Decision {
    const fn new(status: Disposition, reason: &'static str) -> Self {
        Self { status, reason }
    }
}

fn decide(case: &LifecycleCase) -> Decision {
    if case.pair_id.is_empty() || case.qualified_pair_id.is_empty() {
        return Decision::new(Disposition::Indeterminate, "pair_identity_missing");
    }
    if case.pair_id != case.qualified_pair_id {
        return Decision::new(Disposition::Rejected, "pair_identity_mismatch");
    }
    if case.service_kind != case.qualification_service_kind {
        return Decision::new(Disposition::Rejected, "service_kind_mismatch");
    }
    if case.provider_configuration.is_empty() || case.qualified_configuration.is_empty() {
        return Decision::new(Disposition::Indeterminate, "configuration_identity_missing");
    }
    if case.provider_configuration != case.qualified_configuration {
        return Decision::new(
            Disposition::RequalificationRequired,
            "configuration_changed_since_qualification",
        );
    }
    if case.current_procedure_revision.is_empty() || case.qualified_procedure_revision.is_empty() {
        return Decision::new(Disposition::Indeterminate, "procedure_revision_missing");
    }
    if case.current_procedure_revision != case.qualified_procedure_revision {
        return Decision::new(
            Disposition::RequalificationRequired,
            "procedure_revision_changed",
        );
    }
    if case.qualification_revoked {
        return Decision::new(Disposition::Withdrawn, "qualification_revoked");
    }
    if case.critical_defect_for_service {
        return Decision::new(Disposition::Withdrawn, "unresolved_critical_defect");
    }
    if case.wear_within_limit == Some(false) {
        return Decision::new(Disposition::Withdrawn, "wear_exceeds_declared_limit");
    }
    if case.environment_within_envelope == Some(false) {
        return Decision::new(
            Disposition::RequalificationRequired,
            "environment_exceeds_qualified_envelope",
        );
    }
    if case.maintenance_after_qualification {
        return Decision::new(
            Disposition::RequalificationRequired,
            "post_qualification_maintenance_requires_requalification",
        );
    }
    let Some(valid_until) = case.qualification_valid_until_tick else {
        return Decision::new(Disposition::Indeterminate, "qualification_validity_missing");
    };
    if case.at_tick > valid_until {
        return Decision::new(
            Disposition::RequalificationRequired,
            "qualification_expired",
        );
    }
    let Some(cycle_limit) = case.qualified_cycle_limit else {
        return Decision::new(Disposition::Indeterminate, "qualified_cycle_limit_missing");
    };
    if case.cycle_count > cycle_limit {
        return Decision::new(
            Disposition::RequalificationRequired,
            "qualified_cycle_limit_exceeded",
        );
    }
    let Some(inspection_age) = case.inspection_age_ticks else {
        return Decision::new(Disposition::Indeterminate, "inspection_age_missing");
    };
    let Some(inspection_interval) = case.inspection_interval_ticks else {
        return Decision::new(Disposition::Indeterminate, "inspection_interval_missing");
    };
    if inspection_age > inspection_interval {
        return Decision::new(
            Disposition::RequalificationRequired,
            "inspection_interval_expired",
        );
    }
    if case.calibration_valid != Some(true) {
        return Decision::new(Disposition::Indeterminate, "calibration_evidence_not_valid");
    }
    if case.wear_within_limit != Some(true) {
        return Decision::new(Disposition::Indeterminate, "wear_observation_missing");
    }
    if case.environment_within_envelope != Some(true) {
        return Decision::new(Disposition::Indeterminate, "environment_evidence_missing");
    }
    if case.uncertainty_clears_margin != Some(true) {
        return Decision::new(Disposition::Indeterminate, "measurement_margin_unproven");
    }
    if case.inspection_coverage_complete != Some(true) {
        return Decision::new(Disposition::Indeterminate, "inspection_coverage_incomplete");
    }
    if case.lineage_reconciled != Some(true) {
        return Decision::new(Disposition::Indeterminate, "lifecycle_lineage_unreconciled");
    }
    if case.raw_artifact_valid != Some(true) {
        return Decision::new(Disposition::Indeterminate, "raw_artifact_not_admitted");
    }
    if case.service_kind == ServiceKind::PreparedStorableFluid
        && !case.fluid_interface_observation_present
    {
        return Decision::new(
            Disposition::Indeterminate,
            "fluid_interface_observation_missing",
        );
    }
    if case.restricted {
        return Decision::new(
            Disposition::Restricted,
            "only_restricted_envelope_is_qualified",
        );
    }
    Decision::new(
        Disposition::QualifiedWithinDeclaredEnvelope,
        "all_lifecycle_gates_satisfied",
    )
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != CORPUS_SCHEMA {
        return Err("unsupported lifecycle corpus schema".to_owned());
    }
    if corpus.evidence_class != EVIDENCE_CLASS {
        return Err("v0.1 accepts only synthetic evidence".to_owned());
    }
    if corpus.claim_ceiling != CLAIM_CEILING {
        return Err("unsupported lifecycle claim ceiling".to_owned());
    }
    if corpus.cases.is_empty() {
        return Err("lifecycle corpus must contain at least one case".to_owned());
    }

    let mut seen = HashSet::with_capacity(corpus.cases.len());
    let mut results = Vec::with_capacity(corpus.cases.len());
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        qualified_within_declared_envelope: 0,
        restricted: 0,
        requalification_required: 0,
        withdrawn: 0,
        rejected: 0,
        indeterminate: 0,
        expected_result_mismatches: 0,
    };

    for case in &corpus.cases {
        if case.case_id.is_empty() || !seen.insert(case.case_id.as_str()) {
            return Err("case IDs must be non-empty and unique".to_owned());
        }
        let decision = decide(case);
        match decision.status {
            Disposition::QualifiedWithinDeclaredEnvelope => {
                summary.qualified_within_declared_envelope += 1;
            }
            Disposition::Restricted => summary.restricted += 1,
            Disposition::RequalificationRequired => summary.requalification_required += 1,
            Disposition::Withdrawn => summary.withdrawn += 1,
            Disposition::Rejected => summary.rejected += 1,
            Disposition::Indeterminate => summary.indeterminate += 1,
        }
        let expected_result_matched =
            decision.status == case.expected_status && decision.reason == case.expected_reason;
        if !expected_result_matched {
            summary.expected_result_mismatches += 1;
        }
        results.push(CaseResult {
            case_id: case.case_id.clone(),
            actual_status: decision.status,
            actual_reason: decision.reason.to_owned(),
            expected_status: case.expected_status,
            expected_reason: case.expected_reason.clone(),
            expected_result_matched,
        });
    }

    Ok(Report {
        result: if summary.expected_result_mismatches == 0 {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
        evidence_class: EVIDENCE_CLASS.to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        cases: results,
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

pub fn is_valid_source_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
