//! Version-pinned standards/evidence scope auditor.
//!
//! This reference binds declared claims to public standard scope and exact
//! evidence receipts. It is not a conformity-assessment tool, safety assessor,
//! legal applicability engine, or certification authority.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Version-pinned standard identity. Editions are not interchangeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StandardId {
    /// Legged service-robot locomotion performance and test methods.
    Iso18646Part5_2026,
    /// Industrial mobile robot communications and interoperability.
    Iso21423_2026,
    /// Industrial robot product-level safety requirements.
    Iso10218Part1_2025,
    /// Industrial robot application/cell integration and lifecycle safety.
    Iso10218Part2_2025,
    /// Collaborative industrial robot application safety specification.
    IsoTs15066_2016,
}

/// Independently qualified claim domains.
///
/// Robot-level, application-level and collaborative-application safety are
/// intentionally distinct. A result in one cannot satisfy another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimDomain {
    /// Locomotion/task performance under a declared measurement protocol.
    Performance,
    /// Safety evidence for the robot as a product / partly completed machine.
    RobotSafety,
    /// Safety evidence for an integrated robot application or cell.
    ApplicationCellSafety,
    /// Safety evidence for a declared collaborative industrial application.
    CollaborativeApplicationSafety,
    /// Communication and information-exchange interoperability.
    Interoperability,
    /// Navigation performance/behavior, distinct from communication exchange.
    Navigation,
}

/// Explicit disposition supplied by the accountable applicability assessor.
///
/// This crate does not decide legal applicability from a robot-type label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicabilityDisposition {
    /// The named standard is declared applicable to this exact deployment scope.
    Applicable,
    /// The standard is explicitly not applicable, with rationale.
    NotApplicable {
        /// Non-empty rationale.
        reason: String,
    },
    /// The deployment is explicitly outside the published standard scope.
    OutOfScope {
        /// Non-empty rationale.
        reason: String,
    },
    /// Applicability has not been established.
    Unknown,
}

/// Assessment of one claim-domain receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDisposition {
    /// The responsible evidence owner qualified the receipt for this scope.
    Qualified,
    /// Evidence is missing, incomplete or unresolved.
    Unknown,
    /// The evidence was explicitly rejected or found contradictory.
    Rejected,
}

/// What an evidence owner actually concluded within a declared scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceResult {
    /// The named test/evidence passed within its exact declared scope.
    PassWithinDeclaredScope,
    /// The named test/evidence failed within its exact declared scope.
    Fail,
    /// Evidence is incomplete or could not establish the requested result.
    Incomplete,
    /// The test was not performed or was not assessed.
    NotAssessed,
}

/// Accountable owner class for an evidence receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceOwner {
    /// Robot manufacturer or robot-product authority.
    RobotManufacturer,
    /// Integrator / application or cell owner.
    ApplicationIntegrator,
    /// Collaborative-application risk assessor / validation owner.
    CollaborativeApplicationAssessor,
    /// Independent performance-test laboratory / benchmark owner.
    PerformanceEvaluator,
    /// Protocol participant / interoperability test authority.
    InteroperabilityEvaluator,
    /// Other owner; exact role is still bound by receipt identity.
    Other,
}

/// Exact scope common to the whole standards claim profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimScope {
    /// Declared robot / fleet identity.
    pub robot_identity_digest: String,
    /// Exact physical robot and equipment configuration.
    pub robot_configuration_digest: String,
    /// Exact application/workcell/deployment configuration.
    pub application_scope_digest: String,
    /// Declared deployment context and operating environment.
    pub deployment_context_digest: String,
    /// Exact software/controller configuration where relevant.
    pub software_configuration_digest: String,
    /// Exact benchmark/test campaign manifest.
    pub test_manifest_digest: String,
}

/// One standard's declared applicability and public-scope reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardApplicabilityRecord {
    /// Exact standard edition.
    pub standard: StandardId,
    /// Assessor's explicit scope decision.
    pub disposition: ApplicabilityDisposition,
    /// Digest of the assessor's scope/applicability record.
    pub applicability_receipt_digest: String,
    /// Source URL or controlled source reference for the public scope statement.
    pub public_scope_source: String,
    /// Digest of the public source snapshot used by this profile.
    pub public_scope_source_digest: String,
}

/// Evidence artifact receipt for one specific claim domain and standard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainEvidenceReceipt {
    /// Standard whose declared scope this evidence is mapped to.
    pub standard: StandardId,
    /// Single claim domain assessed by this receipt.
    pub domain: ClaimDomain,
    /// Owner disposition.
    pub disposition: EvidenceDisposition,
    /// Result inside the declared evidence scope.
    pub result: EvidenceResult,
    /// Exact applicability receipt to which this evidence is attached.
    pub applicability_receipt_digest: String,
    /// Exact robot identity.
    pub robot_identity_digest: String,
    /// Exact physical robot / equipment configuration.
    pub robot_configuration_digest: String,
    /// Exact application/workcell/deployment scope.
    pub application_scope_digest: String,
    /// Exact deployment context.
    pub deployment_context_digest: String,
    /// Exact software/controller identity.
    pub software_configuration_digest: String,
    /// Exact test/campaign manifest.
    pub test_manifest_digest: String,
    /// Evidence artifact digest.
    pub evidence_artifact_digest: String,
    /// Evidence owner.
    pub owner: EvidenceOwner,
    /// Evidence authority / responsible reviewer identity.
    pub authority_identity: String,
}

/// Complete profile for a bounded standards-related claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardsEvidenceProfile {
    /// Profile identifier.
    pub profile_id: String,
    /// Profile version.
    pub profile_version: String,
    /// Exact declared scope.
    pub scope: ClaimScope,
    /// Standards/applicability records; one entry per standard.
    pub standards: Vec<StandardApplicabilityRecord>,
    /// Receipts for each separately claimed evidence domain.
    pub evidence: Vec<DomainEvidenceReceipt>,
}

/// Per-domain outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainAssessment {
    /// Standard.
    pub standard: StandardId,
    /// Independently typed domain.
    pub domain: ClaimDomain,
    /// Final disposition.
    pub result: EvidenceResult,
    /// Receipt disposition.
    pub evidence_disposition: Option<EvidenceDisposition>,
}

/// Final profile assessment.
///
/// This is a scope-completeness report, not a statement of conformity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardsEvidenceReport {
    /// Profile identity.
    pub profile_id: String,
    /// Overall disposition; Pass is deliberately not representable.
    pub complete_within_declared_scope: bool,
    /// Domain-level assessments.
    pub domains: Vec<DomainAssessment>,
    /// Always false: this profile grants no safety or actuation authority.
    pub conformity_certified: bool,
    /// Always false: this profile grants no robot actuation authority.
    pub actuation_authorized: bool,
}

/// Structural, scope or evidence-contract error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrosswalkError {
    /// Required identity is empty.
    EmptyIdentity(&'static str),
    /// Duplicate standard entry.
    DuplicateStandard(StandardId),
    /// Duplicate evidence for one standard/domain pair.
    DuplicateDomainEvidence {
        standard: StandardId,
        domain: ClaimDomain,
    },
    /// Evidence names a standard without an applicability record.
    EvidenceForUnknownStandard(StandardId),
    /// Evidence is supplied for a standard explicitly marked not applicable/out of scope.
    EvidenceForNonApplicableStandard(StandardId),
    /// Applicability record is incomplete.
    IncompleteApplicabilityRecord(StandardId),
    /// NotApplicable or OutOfScope record has no rationale.
    MissingApplicabilityRationale(StandardId),
    /// The standard's public scope does not support the supplied claim domain.
    DomainOutsideStandardScope {
        standard: StandardId,
        domain: ClaimDomain,
    },
    /// Evidence receipt refers to a different applicability receipt.
    ApplicabilityReceiptMismatch(StandardId),
    /// Evidence receipt does not bind the exact profile scope.
    EvidenceScopeMismatch {
        standard: StandardId,
        domain: ClaimDomain,
        field: &'static str,
    },
    /// A qualified result is missing its artifact, authority or owner identity.
    IncompleteQualifiedEvidence {
        standard: StandardId,
        domain: ClaimDomain,
    },
    /// A non-qualified receipt cannot claim a Pass result.
    UnqualifiedEvidenceCannotPass {
        standard: StandardId,
        domain: ClaimDomain,
    },
}

/// Publicly declared domains supported by each standard edition in this profile.
///
/// These are deliberately high-level scope mappings; they are not clause-level
/// interpretations of the copyrighted standards.
pub fn standard_domains(standard: StandardId) -> &'static [ClaimDomain] {
    match standard {
        StandardId::Iso18646Part5_2026 => &[ClaimDomain::Performance],
        StandardId::Iso21423_2026 => &[ClaimDomain::Interoperability],
        StandardId::Iso10218Part1_2025 => &[ClaimDomain::RobotSafety],
        StandardId::Iso10218Part2_2025 => &[ClaimDomain::ApplicationCellSafety],
        StandardId::IsoTs15066_2016 => &[ClaimDomain::CollaborativeApplicationSafety],
    }
}

fn standard_requires_owner(standard: StandardId) -> EvidenceOwner {
    match standard {
        StandardId::Iso18646Part5_2026 => EvidenceOwner::PerformanceEvaluator,
        StandardId::Iso21423_2026 => EvidenceOwner::InteroperabilityEvaluator,
        StandardId::Iso10218Part1_2025 => EvidenceOwner::RobotManufacturer,
        StandardId::Iso10218Part2_2025 => EvidenceOwner::ApplicationIntegrator,
        StandardId::IsoTs15066_2016 => EvidenceOwner::CollaborativeApplicationAssessor,
    }
}

fn require_nonempty(
    field: &'static str,
    value: &str,
) -> Result<(), CrosswalkError> {
    if value.trim().is_empty() {
        return Err(CrosswalkError::EmptyIdentity(field));
    }
    Ok(())
}

fn validate_scope(scope: &ClaimScope) -> Result<(), CrosswalkError> {
    for (field, value) in [
        ("robot_identity_digest", scope.robot_identity_digest.as_str()),
        (
            "robot_configuration_digest",
            scope.robot_configuration_digest.as_str(),
        ),
        (
            "application_scope_digest",
            scope.application_scope_digest.as_str(),
        ),
        (
            "deployment_context_digest",
            scope.deployment_context_digest.as_str(),
        ),
        (
            "software_configuration_digest",
            scope.software_configuration_digest.as_str(),
        ),
        ("test_manifest_digest", scope.test_manifest_digest.as_str()),
    ] {
        require_nonempty(field, value)?;
    }
    Ok(())
}

fn validate_profile(profile: &StandardsEvidenceProfile) -> Result<(), CrosswalkError> {
    require_nonempty("profile_id", &profile.profile_id)?;
    require_nonempty("profile_version", &profile.profile_version)?;
    validate_scope(&profile.scope)?;

    let mut standards = BTreeSet::new();
    for record in &profile.standards {
        if !standards.insert(record.standard) {
            return Err(CrosswalkError::DuplicateStandard(record.standard));
        }
        for (field, value) in [
            (
                "applicability_receipt_digest",
                record.applicability_receipt_digest.as_str(),
            ),
            ("public_scope_source", record.public_scope_source.as_str()),
            (
                "public_scope_source_digest",
                record.public_scope_source_digest.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(CrosswalkError::IncompleteApplicabilityRecord(record.standard));
            }
            let _ = field;
        }
        match &record.disposition {
            ApplicabilityDisposition::NotApplicable { reason }
            | ApplicabilityDisposition::OutOfScope { reason }
                if reason.trim().is_empty() =>
            {
                return Err(CrosswalkError::MissingApplicabilityRationale(
                    record.standard,
                ));
            }
            _ => {}
        }
    }

    let mut seen = BTreeSet::new();
    for receipt in &profile.evidence {
        if !seen.insert((receipt.standard, receipt.domain)) {
            return Err(CrosswalkError::DuplicateDomainEvidence {
                standard: receipt.standard,
                domain: receipt.domain,
            });
        }
        let record = profile
            .standards
            .iter()
            .find(|record| record.standard == receipt.standard)
            .ok_or(CrosswalkError::EvidenceForUnknownStandard(receipt.standard))?;
        match record.disposition {
            ApplicabilityDisposition::Applicable => {}
            ApplicabilityDisposition::Unknown => {
                return Err(CrosswalkError::EvidenceForNonApplicableStandard(
                    receipt.standard,
                ));
            }
            ApplicabilityDisposition::NotApplicable { .. }
            | ApplicabilityDisposition::OutOfScope { .. } => {
                return Err(CrosswalkError::EvidenceForNonApplicableStandard(
                    receipt.standard,
                ));
            }
        }
        if !standard_domains(receipt.standard).contains(&receipt.domain) {
            return Err(CrosswalkError::DomainOutsideStandardScope {
                standard: receipt.standard,
                domain: receipt.domain,
            });
        }
        if receipt.applicability_receipt_digest != record.applicability_receipt_digest {
            return Err(CrosswalkError::ApplicabilityReceiptMismatch(receipt.standard));
        }
        for (field, observed, expected) in [
            (
                "robot_identity_digest",
                receipt.robot_identity_digest.as_str(),
                profile.scope.robot_identity_digest.as_str(),
            ),
            (
                "robot_configuration_digest",
                receipt.robot_configuration_digest.as_str(),
                profile.scope.robot_configuration_digest.as_str(),
            ),
            (
                "application_scope_digest",
                receipt.application_scope_digest.as_str(),
                profile.scope.application_scope_digest.as_str(),
            ),
            (
                "deployment_context_digest",
                receipt.deployment_context_digest.as_str(),
                profile.scope.deployment_context_digest.as_str(),
            ),
            (
                "software_configuration_digest",
                receipt.software_configuration_digest.as_str(),
                profile.scope.software_configuration_digest.as_str(),
            ),
            (
                "test_manifest_digest",
                receipt.test_manifest_digest.as_str(),
                profile.scope.test_manifest_digest.as_str(),
            ),
        ] {
            if observed != expected {
                return Err(CrosswalkError::EvidenceScopeMismatch {
                    standard: receipt.standard,
                    domain: receipt.domain,
                    field,
                });
            }
        }
        if receipt.disposition == EvidenceDisposition::Qualified {
            if receipt.evidence_artifact_digest.trim().is_empty()
                || receipt.authority_identity.trim().is_empty()
                || receipt.owner != standard_requires_owner(receipt.standard)
            {
                return Err(CrosswalkError::IncompleteQualifiedEvidence {
                    standard: receipt.standard,
                    domain: receipt.domain,
                });
            }
        } else if receipt.result == EvidenceResult::PassWithinDeclaredScope {
            return Err(CrosswalkError::UnqualifiedEvidenceCannotPass {
                standard: receipt.standard,
                domain: receipt.domain,
            });
        }
    }
    Ok(())
}

/// Assess completeness of the explicit profile.
///
/// This does not say that a standard is satisfied. It says only whether every
/// Applicable standard has one qualified, scope-matched domain receipt and
/// each receipt reports a pass within its declared evidence boundary.
pub fn assess_standards_evidence(
    profile: &StandardsEvidenceProfile,
) -> Result<StandardsEvidenceReport, CrosswalkError> {
    validate_profile(profile)?;
    let mut domains = Vec::new();
    let mut complete = true;

    for record in &profile.standards {
        match &record.disposition {
            ApplicabilityDisposition::Applicable => {
                for domain in standard_domains(record.standard) {
                    let receipt = profile
                        .evidence
                        .iter()
                        .find(|receipt| receipt.standard == record.standard && receipt.domain == *domain);
                    match receipt {
                        Some(receipt)
                            if receipt.disposition == EvidenceDisposition::Qualified
                                && receipt.result == EvidenceResult::PassWithinDeclaredScope =>
                        {
                            domains.push(DomainAssessment {
                                standard: record.standard,
                                domain: *domain,
                                result: receipt.result,
                                evidence_disposition: Some(receipt.disposition),
                            });
                        }
                        Some(receipt) => {
                            complete = false;
                            domains.push(DomainAssessment {
                                standard: record.standard,
                                domain: *domain,
                                result: receipt.result,
                                evidence_disposition: Some(receipt.disposition),
                            });
                        }
                        None => {
                            complete = false;
                            domains.push(DomainAssessment {
                                standard: record.standard,
                                domain: *domain,
                                result: EvidenceResult::NotAssessed,
                                evidence_disposition: None,
                            });
                        }
                    }
                }
            }
            ApplicabilityDisposition::Unknown => {
                complete = false;
            }
            ApplicabilityDisposition::NotApplicable { .. }
            | ApplicabilityDisposition::OutOfScope { .. } => {}
        }
    }

    domains.sort_by_key(|assessment| (assessment.standard, assessment.domain));
    Ok(StandardsEvidenceReport {
        profile_id: profile.profile_id.clone(),
        complete_within_declared_scope: complete,
        domains,
        conformity_certified: false,
        actuation_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> ClaimScope {
        ClaimScope {
            robot_identity_digest: "robot-sha256".into(),
            robot_configuration_digest: "robot-config-sha256".into(),
            application_scope_digest: "application-sha256".into(),
            deployment_context_digest: "deployment-sha256".into(),
            software_configuration_digest: "software-sha256".into(),
            test_manifest_digest: "test-manifest-sha256".into(),
        }
    }

    fn standard(standard: StandardId) -> StandardApplicabilityRecord {
        StandardApplicabilityRecord {
            standard,
            disposition: ApplicabilityDisposition::Applicable,
            applicability_receipt_digest: format!("applicability-{standard:?}"),
            public_scope_source: format!("https://www.iso.org/{standard:?}"),
            public_scope_source_digest: format!("public-scope-{standard:?}"),
        }
    }

    fn evidence(
        standard: StandardId,
        domain: ClaimDomain,
        result: EvidenceResult,
    ) -> DomainEvidenceReceipt {
        DomainEvidenceReceipt {
            standard,
            domain,
            disposition: EvidenceDisposition::Qualified,
            result,
            applicability_receipt_digest: format!("applicability-{standard:?}"),
            robot_identity_digest: "robot-sha256".into(),
            robot_configuration_digest: "robot-config-sha256".into(),
            application_scope_digest: "application-sha256".into(),
            deployment_context_digest: "deployment-sha256".into(),
            software_configuration_digest: "software-sha256".into(),
            test_manifest_digest: "test-manifest-sha256".into(),
            evidence_artifact_digest: format!("evidence-{standard:?}-{domain:?}"),
            owner: standard_requires_owner(standard),
            authority_identity: "independent-owner".into(),
        }
    }

    fn profile() -> StandardsEvidenceProfile {
        StandardsEvidenceProfile {
            profile_id: "industrial-and-legged-robot-profile".into(),
            profile_version: "0.1.0".into(),
            scope: scope(),
            standards: vec![
                standard(StandardId::Iso18646Part5_2026),
                standard(StandardId::Iso21423_2026),
                standard(StandardId::Iso10218Part1_2025),
                standard(StandardId::Iso10218Part2_2025),
                standard(StandardId::IsoTs15066_2016),
            ],
            evidence: vec![
                evidence(
                    StandardId::Iso18646Part5_2026,
                    ClaimDomain::Performance,
                    EvidenceResult::PassWithinDeclaredScope,
                ),
                evidence(
                    StandardId::Iso21423_2026,
                    ClaimDomain::Interoperability,
                    EvidenceResult::PassWithinDeclaredScope,
                ),
                evidence(
                    StandardId::Iso10218Part1_2025,
                    ClaimDomain::RobotSafety,
                    EvidenceResult::PassWithinDeclaredScope,
                ),
                evidence(
                    StandardId::Iso10218Part2_2025,
                    ClaimDomain::ApplicationCellSafety,
                    EvidenceResult::PassWithinDeclaredScope,
                ),
                evidence(
                    StandardId::IsoTs15066_2016,
                    ClaimDomain::CollaborativeApplicationSafety,
                    EvidenceResult::PassWithinDeclaredScope,
                ),
            ],
        }
    }

    #[test]
    fn separate_standards_domains_can_be_reported_without_certification() {
        let report = assess_standards_evidence(&profile()).unwrap();
        assert!(report.complete_within_declared_scope);
        assert!(!report.conformity_certified);
        assert!(!report.actuation_authorized);
        assert_eq!(report.domains.len(), 5);
    }

    #[test]
    fn performance_standard_cannot_satisfy_safety_domain() {
        let mut audit = profile();
        audit.evidence[0].domain = ClaimDomain::RobotSafety;
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::DomainOutsideStandardScope {
                standard: StandardId::Iso18646Part5_2026,
                domain: ClaimDomain::RobotSafety,
            })
        );
    }

    #[test]
    fn industrial_mobile_robot_communication_standard_cannot_satisfy_navigation_or_safety() {
        let mut audit = profile();
        audit.evidence[1].domain = ClaimDomain::Navigation;
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::DomainOutsideStandardScope {
                standard: StandardId::Iso21423_2026,
                domain: ClaimDomain::Navigation,
            })
        );
        audit.evidence[1].domain = ClaimDomain::ApplicationCellSafety;
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::DomainOutsideStandardScope {
                standard: StandardId::Iso21423_2026,
                domain: ClaimDomain::ApplicationCellSafety,
            })
        );
    }

    #[test]
    fn robot_product_safety_does_not_satisfy_application_cell_safety() {
        let mut audit = profile();
        audit.evidence[2].domain = ClaimDomain::ApplicationCellSafety;
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::DomainOutsideStandardScope {
                standard: StandardId::Iso10218Part1_2025,
                domain: ClaimDomain::ApplicationCellSafety,
            })
        );
    }

    #[test]
    fn application_cell_safety_does_not_satisfy_robot_product_safety() {
        let mut audit = profile();
        audit.evidence[3].domain = ClaimDomain::RobotSafety;
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::DomainOutsideStandardScope {
                standard: StandardId::Iso10218Part2_2025,
                domain: ClaimDomain::RobotSafety,
            })
        );
    }

    #[test]
    fn collaborative_application_standard_is_not_generic_robot_safety() {
        let mut audit = profile();
        audit.evidence[4].domain = ClaimDomain::RobotSafety;
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::DomainOutsideStandardScope {
                standard: StandardId::IsoTs15066_2016,
                domain: ClaimDomain::RobotSafety,
            })
        );
    }

    #[test]
    fn unknown_applicability_is_incomplete_not_pass() {
        let mut audit = profile();
        audit.standards[0].disposition = ApplicabilityDisposition::Unknown;
        let report = assess_standards_evidence(&audit).unwrap();
        assert!(!report.complete_within_declared_scope);
    }

    #[test]
    fn non_applicable_requires_rationale_and_has_no_evidence() {
        let mut audit = profile();
        audit.standards[0].disposition =
            ApplicabilityDisposition::NotApplicable { reason: "".into() };
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::MissingApplicabilityRationale(
                StandardId::Iso18646Part5_2026
            ))
        );
        audit.standards[0].disposition = ApplicabilityDisposition::NotApplicable {
            reason: "No legged locomotion in this application profile".into(),
        };
        assert!(matches!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::EvidenceForNonApplicableStandard(
                StandardId::Iso18646Part5_2026
            ))
        ));
    }

    #[test]
    fn evidence_cannot_pass_with_unknown_or_rejected_receipt() {
        let mut audit = profile();
        audit.evidence[0].disposition = EvidenceDisposition::Unknown;
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::UnqualifiedEvidenceCannotPass {
                standard: StandardId::Iso18646Part5_2026,
                domain: ClaimDomain::Performance,
            })
        );
    }

    #[test]
    fn exact_robot_application_and_test_identity_are_required() {
        let mut audit = profile();
        audit.evidence[0].robot_configuration_digest = "another-robot".into();
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::EvidenceScopeMismatch {
                standard: StandardId::Iso18646Part5_2026,
                domain: ClaimDomain::Performance,
                field: "robot_configuration_digest",
            })
        );
    }

    #[test]
    fn missing_applicable_domain_receipt_is_not_assessed() {
        let mut audit = profile();
        audit.evidence.retain(|receipt| receipt.standard != StandardId::Iso21423_2026);
        let report = assess_standards_evidence(&audit).unwrap();
        assert!(!report.complete_within_declared_scope);
        let interop = report
            .domains
            .iter()
            .find(|assessment| assessment.standard == StandardId::Iso21423_2026)
            .unwrap();
        assert_eq!(interop.result, EvidenceResult::NotAssessed);
    }

    #[test]
    fn duplicate_standard_record_is_rejected() {
        let mut audit = profile();
        audit.standards.push(audit.standards[0].clone());
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::DuplicateStandard(
                StandardId::Iso18646Part5_2026
            ))
        );
    }

    #[test]
    fn duplicate_domain_receipt_is_rejected() {
        let mut audit = profile();
        audit.evidence.push(audit.evidence[0].clone());
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::DuplicateDomainEvidence {
                standard: StandardId::Iso18646Part5_2026,
                domain: ClaimDomain::Performance,
            })
        );
    }

    #[test]
    fn out_of_scope_standard_cannot_receive_a_pass_evidence_receipt() {
        let mut audit = profile();
        audit.standards[0].disposition = ApplicabilityDisposition::OutOfScope {
            reason: "This application is not a legged robot".into(),
        };
        assert_eq!(
            assess_standards_evidence(&audit),
            Err(CrosswalkError::EvidenceForNonApplicableStandard(
                StandardId::Iso18646Part5_2026
            ))
        );
    }
}
