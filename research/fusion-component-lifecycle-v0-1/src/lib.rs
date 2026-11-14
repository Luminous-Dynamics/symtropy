//! Fusion component lifecycle evidence contract, v0.1.
//!
//! This crate validates identity and evidence-gate semantics only. It does
//! not calculate physical properties, validate a reactor component, or grant
//! manufacturing, safety, or operating authority.

use std::collections::BTreeSet;

/// Contract version carried by any future serialized adapter.
pub const CONTRACT_VERSION: &str = "fusion-component-lifecycle/0.1";

/// Source class for an evidence reference.
///
/// These classes are intentionally distinct. In particular, a synthetic
/// fixture is never a measured observation or a physical qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticFixture,
    ExternalReference,
    MeasuredObservation,
    IndependentVerification,
    PhysicalQualification,
}

/// Immutable reference to evidence owned by another subsystem or source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRef {
    pub id: String,
    pub subject_ref: String,
    pub subject_revision: String,
    pub scope_ref: String,
    pub content_sha256: String,
    pub class: EvidenceClass,
}

/// A hard engineering gate. A passing gate must cite at least one evidence
/// reference; a failed or unknown gate must explain why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    pub state: GateState,
    pub reason_code: Option<String>,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateState {
    Pass,
    Fail,
    Unknown,
}

/// A frozen component candidate package. References point to canonical
/// material and process records; this crate does not copy those records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentCandidate {
    pub candidate_id: String,
    pub design_revision: String,
    pub geometry_sha256: String,
    pub material_record_ref: String,
    pub process_plan_ref: String,
    pub process_capability: Gate,
    pub tolerance_stack: Gate,
    pub inspection_access: Gate,
    pub assembly_access: Gate,
    pub replacement_access: Gate,
    pub evidence: Vec<EvidenceRef>,
}

impl ComponentCandidate {
    fn gates(&self) -> [(&'static str, &Gate); 5] {
        [
            ("process_capability", &self.process_capability),
            ("tolerance_stack", &self.tolerance_stack),
            ("inspection_access", &self.inspection_access),
            ("assembly_access", &self.assembly_access),
            ("replacement_access", &self.replacement_access),
        ]
    }
}

/// The only dispositions this contract may return.
///
/// There is intentionally no `ManufacturingRelease`, `SafetyApproved`,
/// `PhysicallyQualified`, or `ReactorValidated` disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Blocked,
    Indeterminate,
    EligibleForFurtherEngineeringReview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateFinding {
    pub gate: &'static str,
    pub state: GateState,
    pub reason_code: Option<String>,
}

/// Contract-level assessment. The claim ceiling is fixed and cannot be
/// increased by supplying more PASS gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    pub contract_version: &'static str,
    pub candidate_id: String,
    pub disposition: Disposition,
    pub findings: Vec<GateFinding>,
    pub claim_ceiling: ClaimCeiling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimCeiling {
    ContractAndSyntheticFixtureOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    EmptyField(&'static str),
    InvalidSha256(&'static str),
    DuplicateEvidenceId(String),
    DuplicateGateEvidenceId {
        gate: &'static str,
        evidence_id: String,
    },
    DanglingEvidenceReference {
        gate: &'static str,
        evidence_id: String,
    },
    PassingGateWithoutEvidence(&'static str),
    NonPassingGateWithoutReason(&'static str),
}

/// Validate a candidate package and compute its bounded disposition.
///
/// A failed gate dominates unknown gates. Unknown dominates a fully
/// admissible result. Even when every gate passes, the result is only
/// eligible for further engineering review.
pub fn assess(candidate: &ComponentCandidate) -> Result<Assessment, Vec<ValidationError>> {
    let mut errors = Vec::new();

    require_nonempty(&candidate.candidate_id, "candidate_id", &mut errors);
    require_nonempty(&candidate.design_revision, "design_revision", &mut errors);
    require_nonempty(
        &candidate.material_record_ref,
        "material_record_ref",
        &mut errors,
    );
    require_nonempty(
        &candidate.process_plan_ref,
        "process_plan_ref",
        &mut errors,
    );

    if !is_sha256(&candidate.geometry_sha256) {
        errors.push(ValidationError::InvalidSha256("geometry_sha256"));
    }

    let mut evidence_ids = BTreeSet::new();
    for evidence in &candidate.evidence {
        require_nonempty(&evidence.id, "evidence.id", &mut errors);
        require_nonempty(&evidence.subject_ref, "evidence.subject_ref", &mut errors);
        require_nonempty(
            &evidence.subject_revision,
            "evidence.subject_revision",
            &mut errors,
        );
        require_nonempty(&evidence.scope_ref, "evidence.scope_ref", &mut errors);
        if !is_sha256(&evidence.content_sha256) {
            errors.push(ValidationError::InvalidSha256("evidence.content_sha256"));
        }
        if !evidence_ids.insert(evidence.id.as_str()) {
            errors.push(ValidationError::DuplicateEvidenceId(evidence.id.clone()));
        }
    }

    for (gate_name, gate) in candidate.gates() {
        let mut gate_evidence_ids = BTreeSet::new();
        for evidence_id in &gate.evidence_ids {
            if !gate_evidence_ids.insert(evidence_id.as_str()) {
                errors.push(ValidationError::DuplicateGateEvidenceId {
                    gate: gate_name,
                    evidence_id: evidence_id.clone(),
                });
            }
            if !evidence_ids.contains(evidence_id.as_str()) {
                errors.push(ValidationError::DanglingEvidenceReference {
                    gate: gate_name,
                    evidence_id: evidence_id.clone(),
                });
            }
        }

        match gate.state {
            GateState::Pass if gate.evidence_ids.is_empty() => {
                errors.push(ValidationError::PassingGateWithoutEvidence(gate_name));
            }
            GateState::Fail | GateState::Unknown
                if gate.reason_code.as_deref().is_none_or(str::is_empty) =>
            {
                errors.push(ValidationError::NonPassingGateWithoutReason(gate_name));
            }
            _ => {}
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let findings = candidate
        .gates()
        .into_iter()
        .map(|(gate, value)| GateFinding {
            gate,
            state: value.state,
            reason_code: value.reason_code.clone(),
        })
        .collect::<Vec<_>>();

    let has_failure = findings
        .iter()
        .any(|finding| finding.state == GateState::Fail);
    let has_unknown = findings
        .iter()
        .any(|finding| finding.state == GateState::Unknown);

    let disposition = if has_failure {
        Disposition::Blocked
    } else if has_unknown {
        Disposition::Indeterminate
    } else {
        Disposition::EligibleForFurtherEngineeringReview
    };

    Ok(Assessment {
        contract_version: CONTRACT_VERSION,
        candidate_id: candidate.candidate_id.clone(),
        disposition,
        findings,
        claim_ceiling: ClaimCeiling::ContractAndSyntheticFixtureOnly,
    })
}

fn require_nonempty(value: &str, field: &'static str, errors: &mut Vec<ValidationError>) {
    if value.trim().is_empty() {
        errors.push(ValidationError::EmptyField(field));
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn evidence(id: &str, scope_ref: &str, hash: &str) -> EvidenceRef {
        EvidenceRef {
            id: id.to_owned(),
            subject_ref: "component/mock-assembly".to_owned(),
            subject_revision: "design-r1".to_owned(),
            scope_ref: scope_ref.to_owned(),
            content_sha256: hash.to_owned(),
            class: EvidenceClass::SyntheticFixture,
        }
    }

    fn passing_gate(evidence_id: &str) -> Gate {
        Gate {
            state: GateState::Pass,
            reason_code: None,
            evidence_ids: vec![evidence_id.to_owned()],
        }
    }

    fn candidate() -> ComponentCandidate {
        ComponentCandidate {
            candidate_id: "synthetic-candidate-a".to_owned(),
            design_revision: "design-r1".to_owned(),
            geometry_sha256: HASH_A.to_owned(),
            material_record_ref: "symthaea://materials/synthetic-mat-01".to_owned(),
            process_plan_ref: "symthaea://process-plans/synthetic-route-01".to_owned(),
            process_capability: passing_gate("ev-process"),
            tolerance_stack: passing_gate("ev-tolerance"),
            inspection_access: passing_gate("ev-inspection"),
            assembly_access: passing_gate("ev-assembly"),
            replacement_access: passing_gate("ev-replacement"),
            evidence: vec![
                evidence("ev-process", "process-capability", HASH_A),
                evidence("ev-tolerance", "tolerance-stack", HASH_B),
                evidence("ev-inspection", "inspection-access", HASH_A),
                evidence("ev-assembly", "assembly-access", HASH_B),
                evidence("ev-replacement", "replacement-access", HASH_A),
            ],
        }
    }

    #[test]
    fn all_synthetic_gates_pass_only_allows_further_review() {
        let result = assess(&candidate()).expect("synthetic package should be valid");
        assert_eq!(
            result.disposition,
            Disposition::EligibleForFurtherEngineeringReview
        );
        assert_eq!(
            result.claim_ceiling,
            ClaimCeiling::ContractAndSyntheticFixtureOnly
        );
    }

    #[test]
    fn unknown_process_capability_is_indeterminate_not_feasible() {
        let mut package = candidate();
        package.process_capability = Gate {
            state: GateState::Unknown,
            reason_code: Some("PROCESS_PROFILE_NOT_ADMITTED".to_owned()),
            evidence_ids: vec![],
        };
        let result = assess(&package).expect("unknown is a valid explicit disposition");
        assert_eq!(result.disposition, Disposition::Indeterminate);
    }

    #[test]
    fn failed_inspection_access_blocks_candidate() {
        let mut package = candidate();
        package.inspection_access = Gate {
            state: GateState::Fail,
            reason_code: Some("CRITICAL_SURFACE_NOT_INSPECTABLE".to_owned()),
            evidence_ids: vec!["ev-inspection".to_owned()],
        };
        let result = assess(&package).expect("failed is a valid explicit disposition");
        assert_eq!(result.disposition, Disposition::Blocked);
    }

    #[test]
    fn failure_dominates_unknown() {
        let mut package = candidate();
        package.process_capability = Gate {
            state: GateState::Unknown,
            reason_code: Some("CAPABILITY_UNKNOWN".to_owned()),
            evidence_ids: vec![],
        };
        package.replacement_access = Gate {
            state: GateState::Fail,
            reason_code: Some("REMOVAL_ENVELOPE_INTERFERENCE".to_owned()),
            evidence_ids: vec!["ev-replacement".to_owned()],
        };
        let result = assess(&package).expect("explicit failure and unknown are valid");
        assert_eq!(result.disposition, Disposition::Blocked);
    }

    #[test]
    fn passing_gate_without_evidence_is_rejected() {
        let mut package = candidate();
        package.inspection_access.evidence_ids.clear();
        let errors = assess(&package).expect_err("unsupported PASS must fail closed");
        assert!(errors.contains(&ValidationError::PassingGateWithoutEvidence(
            "inspection_access"
        )));
    }

    #[test]
    fn dangling_evidence_reference_is_rejected() {
        let mut package = candidate();
        package.process_plan_ref = "symthaea://process-plans/route-unknown".to_owned();
        package.process_capability.evidence_ids = vec!["missing-evidence".to_owned()];
        let errors = assess(&package).expect_err("dangling evidence must be rejected");
        assert!(errors.iter().any(|error| matches!(
            error,
            ValidationError::DanglingEvidenceReference {
                gate: "process_capability",
                evidence_id
            } if evidence_id == "missing-evidence"
        )));
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let mut package = candidate();
        package.geometry_sha256 = "not-a-digest".to_owned();
        let errors = assess(&package).expect_err("bad geometry digest must be rejected");
        assert!(errors.contains(&ValidationError::InvalidSha256("geometry_sha256")));
    }

    #[test]
    fn duplicate_evidence_identity_is_rejected() {
        let mut package = candidate();
        package.evidence.push(evidence("ev-process", "duplicate", HASH_B));
        let errors = assess(&package).expect_err("duplicate evidence identity must be rejected");
        assert!(errors.contains(&ValidationError::DuplicateEvidenceId(
            "ev-process".to_owned()
        )));
    }

    #[test]
    fn unknown_must_have_a_reason_code() {
        let mut package = candidate();
        package.process_capability = Gate {
            state: GateState::Unknown,
            reason_code: None,
            evidence_ids: vec![],
        };
        let errors = assess(&package).expect_err("unknown without a reason must be rejected");
        assert!(errors.contains(&ValidationError::NonPassingGateWithoutReason(
            "process_capability"
        )));
    }
}
