//! Cross-field semantic validation for Subterra site-evidence claims.
//!
//! JSON Schema validates record shape. This module validates relationships
//! between claims so a site cannot simultaneously claim direct interior
//! connectivity and no demonstrated access, or qualified use without a
//! declared engineering envelope and evidence.
//!
//! This validator checks consistency of a record, not truth of its evidence.
//! It cannot independently authenticate observations, validate geomechanics,
//! approve planetary-protection activity, or certify habitation.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteBody {
    Earth,
    Moon,
    Mars,
    Venus,
    Mercury,
    IcyMoon,
    Titan,
    Asteroid,
    Comet,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimState {
    Candidate,
    Supported,
    DirectlyObserved,
    Contradicted,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityState {
    Unknown,
    Candidate,
    GeophysicallySupported,
    DirectlyObservedConnected,
    Contradicted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityState {
    Unknown,
    Candidate,
    RobotAccessDemonstrated,
    HumanAccessAssessed,
    Inaccessible,
    OutOfDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteriorSurveyState {
    NotSurveyed,
    Partial,
    SurveyedWithinDeclaredExtent,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringUseCase {
    Science,
    RobotStaging,
    Storage,
    ResourceProcessing,
    Industrial,
    PressurizedModule,
    Habitation,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringClaimState {
    NotAssessed,
    Candidate,
    AssessedWithLimitations,
    QualifiedForDeclaredEnvelope,
    Rejected,
    OutOfDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetaryProtectionState {
    NotApplicable,
    NotAssessed,
    ReviewRequired,
    Restricted,
    AuthorizedForDeclaredActivity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityPermission {
    NotAssessed,
    Prohibited,
    ReviewRequired,
    AuthorizedForDeclaredActivity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceClaim {
    pub state: ClaimState,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OriginHypothesis {
    pub mechanism: String,
    pub state: ClaimState,
    pub rationale: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectivityClaim {
    pub state: ConnectivityState,
    pub known_extent_m: Option<f64>,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccessibilityClaim {
    pub state: AccessibilityState,
    pub access_method: Option<String>,
    pub egress_method: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InteriorSurveyClaim {
    pub state: InteriorSurveyState,
    pub surveyed_extent_m: Option<f64>,
    pub coverage_fraction: Option<f64>,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EngineeringAssessment {
    pub use_case: EngineeringUseCase,
    pub state: EngineeringClaimState,
    pub envelope_id: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreservationAssessment {
    pub planetary_protection_state: PlanetaryProtectionState,
    pub sampling_permission: ActivityPermission,
    pub biological_activity_permission: ActivityPermission,
    pub basis_refs: Vec<String>,
    /// A named review/authorization case from the responsible planetary
    /// protection authority. A string is a reference, not proof of validity.
    pub authorization_case_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SiteEvidenceAssessment {
    pub site_id: String,
    pub body: SiteBody,
    pub morphology: EvidenceClaim,
    pub origin_hypotheses: Vec<OriginHypothesis>,
    pub connectivity: ConnectivityClaim,
    pub accessibility: AccessibilityClaim,
    pub interior_survey: InteriorSurveyClaim,
    pub engineering_assessments: Vec<EngineeringAssessment>,
    pub preservation: PreservationAssessment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticViolation {
    EmptySiteId,
    SupportedClaimMissingEvidence(&'static str),
    InvalidOriginHypothesis(usize),
    InvalidConnectivityExtent,
    DemonstratedAccessMissingMethod,
    DemonstratedAccessMissingEgress,
    DemonstratedAccessMissingEvidence,
    InvalidSurveyExtent,
    InvalidSurveyCoverage,
    SurveyMissingEvidence,
    DuplicateEngineeringUseCase,
    EngineeringClaimMissingEnvelope,
    EngineeringClaimMissingEvidence,
    QualifiedPressurizedUseWithoutSurvey,
    AuthorizedActivityWithoutProtectionAuthorization,
    NonEarthActivityWithoutNamedAuthorizationCase,
    AuthorizedActivityWithoutBasis,
    ConnectivityClaimMissingEvidence,
    IcyBodyBiologicalActivityWithoutNamedReview,
    IcyBodySamplingWithoutNamedReview,
    RestrictedSiteHasAuthorizedActivity,
}

impl std::fmt::Display for SemanticViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn has_refs(refs: &[String]) -> bool {
    refs.iter().any(|value| nonempty(value))
}

fn demonstrated_access(state: AccessibilityState) -> bool {
    matches!(
        state,
        AccessibilityState::RobotAccessDemonstrated
            | AccessibilityState::HumanAccessAssessed
    )
}

fn surveyed_interior(state: InteriorSurveyState) -> bool {
    matches!(
        state,
        InteriorSurveyState::Partial | InteriorSurveyState::SurveyedWithinDeclaredExtent
    )
}

/// Validate logical dependencies across Subterra site claims.
///
/// The function does not interpret the scientific validity of a referenced
/// dataset. It only rejects inconsistent state combinations and missing
/// minimum claim dependencies.
pub fn validate_site_evidence(
    record: &SiteEvidenceAssessment,
) -> Result<(), Vec<SemanticViolation>> {
    let mut violations = Vec::new();

    if !nonempty(&record.site_id) {
        violations.push(SemanticViolation::EmptySiteId);
    }

    if matches!(
        record.morphology.state,
        ClaimState::Supported | ClaimState::DirectlyObserved
    ) && !has_refs(&record.morphology.evidence_refs)
    {
        violations.push(SemanticViolation::SupportedClaimMissingEvidence(
            "morphology",
        ));
    }

    for (index, hypothesis) in record.origin_hypotheses.iter().enumerate() {
        if !nonempty(&hypothesis.mechanism)
            || !nonempty(&hypothesis.rationale)
            || (matches!(
                hypothesis.state,
                ClaimState::Candidate | ClaimState::Supported | ClaimState::DirectlyObserved
            ) && !has_refs(&hypothesis.evidence_refs))
        {
            violations.push(SemanticViolation::InvalidOriginHypothesis(index));
        }
    }

    if record
        .connectivity
        .known_extent_m
        .is_some_and(|extent| !extent.is_finite() || extent < 0.0)
    {
        violations.push(SemanticViolation::InvalidConnectivityExtent);
    }

    if record.connectivity.state == ConnectivityState::GeophysicallySupported
        && !has_refs(&record.connectivity.evidence_refs)
    {
        violations.push(SemanticViolation::ConnectivityClaimMissingEvidence);
    }

    if record.connectivity.state == ConnectivityState::DirectlyObservedConnected
        && !has_refs(&record.connectivity.evidence_refs)
    {
        violations.push(SemanticViolation::SupportedClaimMissingEvidence(
            "direct_connectivity",
        ));
    }

    if demonstrated_access(record.accessibility.state) {
        if !record
            .accessibility
            .access_method
            .as_deref()
            .is_some_and(nonempty)
        {
            violations.push(SemanticViolation::DemonstratedAccessMissingMethod);
        }
        if !record
            .accessibility
            .egress_method
            .as_deref()
            .is_some_and(nonempty)
        {
            violations.push(SemanticViolation::DemonstratedAccessMissingEgress);
        }
        if !has_refs(&record.accessibility.evidence_refs) {
            violations.push(SemanticViolation::DemonstratedAccessMissingEvidence);
        }
    }

    if surveyed_interior(record.interior_survey.state) {
        if !record
            .interior_survey
            .surveyed_extent_m
            .is_some_and(|extent| extent.is_finite() && extent > 0.0)
        {
            violations.push(SemanticViolation::InvalidSurveyExtent);
        }
        if !record
            .interior_survey
            .coverage_fraction
            .is_some_and(|coverage| {
                coverage.is_finite() && coverage > 0.0 && coverage <= 1.0
            })
        {
            violations.push(SemanticViolation::InvalidSurveyCoverage);
        }
        if !has_refs(&record.interior_survey.evidence_refs) {
            violations.push(SemanticViolation::SurveyMissingEvidence);
        }
    } else if record.interior_survey.surveyed_extent_m.is_some()
        || record.interior_survey.coverage_fraction.is_some()
    {
        violations.push(SemanticViolation::InvalidSurveyExtent);
    }

    let mut use_cases = std::collections::BTreeSet::new();
    for assessment in &record.engineering_assessments {
        if !use_cases.insert(assessment.use_case as u8) {
            violations.push(SemanticViolation::DuplicateEngineeringUseCase);
        }

        if assessment.state == EngineeringClaimState::QualifiedForDeclaredEnvelope {
            if !assessment
                .envelope_id
                .as_deref()
                .is_some_and(nonempty)
            {
                violations.push(SemanticViolation::EngineeringClaimMissingEnvelope);
            }
            if !has_refs(&assessment.evidence_refs) {
                violations.push(SemanticViolation::EngineeringClaimMissingEvidence);
            }

            if matches!(
                assessment.use_case,
                EngineeringUseCase::PressurizedModule | EngineeringUseCase::Habitation
            ) && (!surveyed_interior(record.interior_survey.state)
                || !demonstrated_access(record.accessibility.state))
            {
                violations.push(SemanticViolation::QualifiedPressurizedUseWithoutSurvey);
            }
        }
    }

    let preservation = &record.preservation;
    let sampling_authorized = preservation.sampling_permission
        == ActivityPermission::AuthorizedForDeclaredActivity;
    let biology_authorized = preservation.biological_activity_permission
        == ActivityPermission::AuthorizedForDeclaredActivity;
    let any_activity_authorized = sampling_authorized || biology_authorized;

    if any_activity_authorized
        && record.body != SiteBody::Earth
        && preservation.planetary_protection_state
            != PlanetaryProtectionState::AuthorizedForDeclaredActivity
    {
        violations.push(
            SemanticViolation::AuthorizedActivityWithoutProtectionAuthorization,
        );
    }

    if any_activity_authorized
        && record.body != SiteBody::Earth
        && !preservation
            .authorization_case_ref
            .as_deref()
            .is_some_and(nonempty)
    {
        violations.push(SemanticViolation::NonEarthActivityWithoutNamedAuthorizationCase);
    }

    if any_activity_authorized && !has_refs(&preservation.basis_refs) {
        violations.push(SemanticViolation::AuthorizedActivityWithoutBasis);
    }

    if matches!(record.body, SiteBody::IcyMoon)
        && biology_authorized
        && !preservation
            .authorization_case_ref
            .as_deref()
            .is_some_and(nonempty)
    {
        violations.push(
            SemanticViolation::IcyBodyBiologicalActivityWithoutNamedReview,
        );
    }

    if matches!(record.body, SiteBody::IcyMoon)
        && sampling_authorized
        && !preservation
            .authorization_case_ref
            .as_deref()
            .is_some_and(nonempty)
    {
        violations.push(SemanticViolation::IcyBodySamplingWithoutNamedReview);
    }

    if preservation.planetary_protection_state == PlanetaryProtectionState::Restricted
        && any_activity_authorized
    {
        violations.push(SemanticViolation::RestrictedSiteHasAuthorizedActivity);
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_candidate() -> SiteEvidenceAssessment {
        SiteEvidenceAssessment {
            site_id: "synthetic-candidate-01".to_string(),
            body: SiteBody::Moon,
            morphology: EvidenceClaim {
                state: ClaimState::Supported,
                evidence_refs: vec!["optical-observation-01".to_string()],
            },
            origin_hypotheses: vec![OriginHypothesis {
                mechanism: "volcanic_lava_tube".to_string(),
                state: ClaimState::Candidate,
                rationale: "Morphology is compatible with a volcanic conduit.".to_string(),
                evidence_refs: vec!["context-observation-01".to_string()],
            }],
            connectivity: ConnectivityClaim {
                state: ConnectivityState::Candidate,
                known_extent_m: None,
                evidence_refs: vec![],
            },
            accessibility: AccessibilityClaim {
                state: AccessibilityState::Unknown,
                access_method: None,
                egress_method: None,
                evidence_refs: vec![],
            },
            interior_survey: InteriorSurveyClaim {
                state: InteriorSurveyState::NotSurveyed,
                surveyed_extent_m: None,
                coverage_fraction: None,
                evidence_refs: vec![],
            },
            engineering_assessments: vec![EngineeringAssessment {
                use_case: EngineeringUseCase::Science,
                state: EngineeringClaimState::Candidate,
                envelope_id: None,
                evidence_refs: vec![],
            }],
            preservation: PreservationAssessment {
                planetary_protection_state: PlanetaryProtectionState::ReviewRequired,
                sampling_permission: ActivityPermission::ReviewRequired,
                biological_activity_permission: ActivityPermission::Prohibited,
                basis_refs: vec!["policy-review-required".to_string()],
                authorization_case_ref: None,
            },
        }
    }

    #[test]
    fn candidate_with_unknown_connectivity_and_access_is_valid() {
        assert!(validate_site_evidence(&valid_candidate()).is_ok());
    }

    #[test]
    fn geophysically_supported_connectivity_requires_observation_references() {
        let mut record = valid_candidate();
        record.connectivity.state = ConnectivityState::GeophysicallySupported;
        assert!(validate_site_evidence(&record)
            .unwrap_err()
            .contains(&SemanticViolation::ConnectivityClaimMissingEvidence));
    }

    #[test]
    fn earth_not_applicable_planetary_protection_does_not_trigger_planetary_gate() {
        let mut record = valid_candidate();
        record.body = SiteBody::Earth;
        record.preservation.planetary_protection_state = PlanetaryProtectionState::NotApplicable;
        record.preservation.sampling_permission =
            ActivityPermission::AuthorizedForDeclaredActivity;
        record.preservation.basis_refs = vec!["land-manager-permit-record".to_string()];
        assert!(validate_site_evidence(&record).is_ok());
    }

    #[test]
    fn non_earth_authorized_sampling_requires_named_authorization_case() {
        let mut record = valid_candidate();
        record.preservation.planetary_protection_state =
            PlanetaryProtectionState::AuthorizedForDeclaredActivity;
        record.preservation.sampling_permission =
            ActivityPermission::AuthorizedForDeclaredActivity;
        record.preservation.basis_refs = vec!["review-basis".to_string()];
        record.preservation.authorization_case_ref = None;

        assert!(validate_site_evidence(&record)
            .unwrap_err()
            .contains(&SemanticViolation::NonEarthActivityWithoutNamedAuthorizationCase));
    }

    #[test]
    fn directly_observed_connectivity_is_independent_of_access_and_survey() {
        let mut record = valid_candidate();
        record.connectivity.state = ConnectivityState::DirectlyObservedConnected;
        record.connectivity.evidence_refs = vec!["interior-video-01".to_string()];

        // A conduit can be directly observed without a robot demonstrating
        // traversal or the complete interior having been surveyed.
        assert!(validate_site_evidence(&record).is_ok());
        assert_eq!(record.accessibility.state, AccessibilityState::Unknown);
        assert_eq!(record.interior_survey.state, InteriorSurveyState::NotSurveyed);
    }

    #[test]
    fn interior_survey_does_not_imply_demonstrated_edge_traversal() {
        let mut record = valid_candidate();
        record.accessibility.state = AccessibilityState::Unknown;
        record.interior_survey = InteriorSurveyClaim {
            state: InteriorSurveyState::Partial,
            surveyed_extent_m: Some(5.0),
            coverage_fraction: Some(0.2),
            evidence_refs: vec!["remote-interior-survey-01".to_string()],
        };

        // The survey may have been acquired by a tethered, remote or otherwise
        // non-traversal-proving instrument; access remains a separate claim.
        assert!(validate_site_evidence(&record).is_ok());
    }

    #[test]
    fn qualified_habitation_requires_access_egress_and_survey_evidence() {
        let mut record = valid_candidate();
        record.accessibility.state = AccessibilityState::RobotAccessDemonstrated;
        record.accessibility.access_method = Some("tethered rover".to_string());
        record.accessibility.egress_method = Some("reverse tether retrieval".to_string());
        record.accessibility.evidence_refs = vec!["robot-access-receipt".to_string()];
        record.connectivity.state = ConnectivityState::DirectlyObservedConnected;
        record.connectivity.evidence_refs = vec!["interior-video".to_string()];
        record.interior_survey = InteriorSurveyClaim {
            state: InteriorSurveyState::Partial,
            surveyed_extent_m: Some(22.0),
            coverage_fraction: Some(0.4),
            evidence_refs: vec!["survey-map-01".to_string()],
        };

        assert!(validate_site_evidence(&record).is_ok());
    }

    #[test]
    fn qualified_habitation_requires_survey_access_envelope_and_evidence() {
        let mut record = valid_candidate();
        record.engineering_assessments = vec![EngineeringAssessment {
            use_case: EngineeringUseCase::Habitation,
            state: EngineeringClaimState::QualifiedForDeclaredEnvelope,
            envelope_id: Some("habitation-envelope-01".to_string()),
            evidence_refs: vec!["independent-qualification-record-01".to_string()],
        }];

        assert!(validate_site_evidence(&record).unwrap_err().contains(
            &SemanticViolation::QualifiedPressurizedUseWithoutSurvey
        ));
    }

    #[test]
    fn qualified_engineering_claim_requires_envelope_and_evidence() {
        let mut record = valid_candidate();
        record.engineering_assessments = vec![EngineeringAssessment {
            use_case: EngineeringUseCase::Storage,
            state: EngineeringClaimState::QualifiedForDeclaredEnvelope,
            envelope_id: None,
            evidence_refs: vec![],
        }];

        let violations = validate_site_evidence(&record).unwrap_err();
        assert!(violations.contains(&SemanticViolation::EngineeringClaimMissingEnvelope));
        assert!(violations.contains(&SemanticViolation::EngineeringClaimMissingEvidence));
    }

    #[test]
    fn authorized_sampling_requires_planetary_protection_authorization() {
        let mut record = valid_candidate();
        record.preservation.sampling_permission =
            ActivityPermission::AuthorizedForDeclaredActivity;
        record.preservation.basis_refs.clear();

        let violations = validate_site_evidence(&record).unwrap_err();
        assert!(violations.contains(
            &SemanticViolation::AuthorizedActivityWithoutProtectionAuthorization
        ));
        assert!(violations.contains(&SemanticViolation::AuthorizedActivityWithoutBasis));
    }

    #[test]
    fn icy_body_biological_activity_requires_named_review_case() {
        let mut record = valid_candidate();
        record.body = SiteBody::IcyMoon;
        record.preservation.planetary_protection_state =
            PlanetaryProtectionState::AuthorizedForDeclaredActivity;
        record.preservation.biological_activity_permission =
            ActivityPermission::AuthorizedForDeclaredActivity;
        record.preservation.basis_refs = vec!["formal-review-record".to_string()];

        assert!(validate_site_evidence(&record).unwrap_err().contains(
            &SemanticViolation::IcyBodyBiologicalActivityWithoutNamedReview
        ));
    }

    #[test]
    fn restricted_site_cannot_authorize_an_activity() {
        let mut record = valid_candidate();
        record.preservation.planetary_protection_state =
            PlanetaryProtectionState::Restricted;
        record.preservation.sampling_permission =
            ActivityPermission::AuthorizedForDeclaredActivity;
        record.preservation.basis_refs = vec!["review-record".to_string()];

        let violations = validate_site_evidence(&record).unwrap_err();
        assert!(violations.contains(&SemanticViolation::AuthorizedActivityWithoutProtectionAuthorization));
        assert!(violations.contains(&SemanticViolation::RestrictedSiteHasAuthorizedActivity));
    }

    #[test]
    fn missing_survey_extent_and_invalid_coverage_are_rejected() {
        let mut record = valid_candidate();
        record.accessibility.state = AccessibilityState::RobotAccessDemonstrated;
        record.accessibility.access_method = Some("robot".to_string());
        record.accessibility.egress_method = Some("reverse".to_string());
        record.accessibility.evidence_refs = vec!["access".to_string()];
        record.interior_survey = InteriorSurveyClaim {
            state: InteriorSurveyState::Partial,
            surveyed_extent_m: Some(5.0),
            coverage_fraction: Some(1.5),
            evidence_refs: vec!["survey".to_string()],
        };

        assert!(validate_site_evidence(&record).unwrap_err().contains(
            &SemanticViolation::InvalidSurveyCoverage
        ));
    }

    #[test]
    fn supported_morphology_requires_evidence_reference() {
        let mut record = valid_candidate();
        record.morphology.evidence_refs.clear();

        assert!(validate_site_evidence(&record).unwrap_err().contains(
            &SemanticViolation::SupportedClaimMissingEvidence("morphology")
        ));
    }

    #[test]
    fn empty_site_id_is_rejected() {
        let mut record = valid_candidate();
        record.site_id.clear();

        assert!(validate_site_evidence(&record).unwrap_err().contains(
            &SemanticViolation::EmptySiteId
        ));
    }

    #[test]
    fn duplicated_engineering_use_case_is_rejected() {
        let mut record = valid_candidate();
        record.engineering_assessments.push(record.engineering_assessments[0].clone());

        assert!(validate_site_evidence(&record).unwrap_err().contains(
            &SemanticViolation::DuplicateEngineeringUseCase
        ));
    }
}
