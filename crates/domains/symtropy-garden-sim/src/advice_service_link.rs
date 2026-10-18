// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Versioned reference link between a WCARE farm-advice episode and the
//! canonical Symtropy farmer-service request.
//!
//! This module owns neither lifecycle. Service requests remain valid without
//! an advice link; service request/quote/booking/completion/farmer decision
//! state remains in `service_access`. Advice premises, decisions and observed
//! consequence claims remain owned by Symthaea/WCARE and the evidence ledger.
//!
//! This module validates explicit reference/scope consistency only. It does
//! not authenticate source references, verify Mycelix authorization, or infer
//! that temporal association means advice caused a later service outcome.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::service_access::ServiceAccessRequestV1;

pub const ADVICE_SERVICE_LINK_SCHEMA_VERSION_V1: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdviceServiceLinkReasonV1 {
    FarmerDeclared,
    ExplicitAdvice,
    AdviserRelay,
    ServiceResolutionAfterAdvice,
    PostHocFollowUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdviceServiceLinkInitiatorRoleV1 {
    Farmer,
    Adviser,
    CooperativeRelay,
    ServiceProvider,
    Evaluator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdviceServiceLinkStateV1 {
    Pending,
    Linked,
    Stale,
    Disputed,
    RevokedForFutureSharing,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdviceServiceLinkBlockerCodeV1 {
    UnsupportedSchema,
    EmptyLinkId,
    EmptyServiceRequestReference,
    EmptyAdviceEpisodeReference,
    EmptyDecisionReference,
    EmptyScopeReference,
    EmptyPeriodReference,
    EmptyTaskCode,
    EmptySourceReference,
    InvalidRecordedTimestamp,
    RequestReferenceMismatch,
    ScopeMismatch,
    PeriodMismatch,
    TaskMismatch,
    MissingAuthorizationReference,
    EmptyAuthorizationReference,
    EmptyOutcomeReference,
    DuplicateOutcomeReference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdviceServiceLinkDispositionV1 {
    /// There is no advice link. This is not a service-request failure.
    UnlinkedServiceRequest,
    /// Structural refs and scope match; downstream authorization is still required.
    LinkedReferenceValidated,
    /// A valid link exists but should be resolved/refreshed before use.
    RequiresResolution,
    /// A future-sharing revocation is recorded; do not disclose via this link.
    SharingRevoked,
    /// Required reference/consistency checks failed.
    Blocked,
}

/// Opaque references only. The scope triple records the advice-side declared
/// context and is compared to the canonical service request; it is not a
/// replacement for validating the referenced advice episode in WCARE.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdviceServiceLinkV1 {
    pub schema_version: u16,
    pub link_id: String,
    pub service_request_ref: String,
    pub advice_episode_ref: String,
    pub decision_ref: Option<String>,
    pub advice_scope_ref: String,
    pub advice_period_ref: String,
    pub advice_task_code: String,
    pub reason: AdviceServiceLinkReasonV1,
    pub initiated_by_role: AdviceServiceLinkInitiatorRoleV1,
    pub state: AdviceServiceLinkStateV1,
    /// Reference to the owning system's authorization/consent receipt.
    /// Presence is not proof that the receipt is authentic or current.
    pub authorization_ref: Option<String>,
    /// References to observed-outcome claims in the canonical evidence ledger.
    /// Do not copy numeric outcomes or prediction values here.
    pub observed_outcome_claim_refs: Vec<String>,
    pub source_ref: String,
    /// Wall-clock Unix seconds for recording this link event. Not comparable
    /// to ServiceAccessRequestV1's period-relative minute fields.
    pub recorded_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdviceServiceLinkAssessmentV1 {
    pub disposition: AdviceServiceLinkDispositionV1,
    pub blockers: Vec<AdviceServiceLinkBlockerCodeV1>,
    /// A link validation error must never invalidate/remove the service request.
    pub service_request_remains_valid: bool,
    /// Symtropy does not authenticate Mycelix permissions or authorize disclosure.
    pub disclosure_authorized_by_this_module: bool,
    /// Linking events does not establish a causal effect.
    pub causal_claim_permitted: bool,
}

impl AdviceServiceLinkAssessmentV1 {
    fn unlinked() -> Self {
        Self {
            disposition: AdviceServiceLinkDispositionV1::UnlinkedServiceRequest,
            blockers: Vec::new(),
            service_request_remains_valid: true,
            disclosure_authorized_by_this_module: false,
            causal_claim_permitted: false,
        }
    }
}

/// Assess the optional traceability link against the canonical service request.
///
/// The request itself remains valid even when the link is missing, malformed,
/// cross-scope, disputed, stale, unavailable or revoked. A structurally valid
/// `Linked` status only permits downstream systems to inspect the reference;
/// it does not authorize data disclosure or justify causal claims.
pub fn assess_advice_service_link_v1(
    request: &ServiceAccessRequestV1,
    link: Option<&AdviceServiceLinkV1>,
) -> AdviceServiceLinkAssessmentV1 {
    let Some(link) = link else {
        return AdviceServiceLinkAssessmentV1::unlinked();
    };

    let mut blockers = BTreeSet::new();
    if link.schema_version != ADVICE_SERVICE_LINK_SCHEMA_VERSION_V1 {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::UnsupportedSchema);
    }
    if link.link_id.trim().is_empty() {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::EmptyLinkId);
    }
    if link.service_request_ref.trim().is_empty() {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::EmptyServiceRequestReference);
    }
    if link.advice_episode_ref.trim().is_empty() {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::EmptyAdviceEpisodeReference);
    }
    if link.decision_ref.as_ref().is_some_and(|value| value.trim().is_empty()) {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::EmptyDecisionReference);
    }
    if link.advice_scope_ref.trim().is_empty() {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::EmptyScopeReference);
    }
    if link.advice_period_ref.trim().is_empty() {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::EmptyPeriodReference);
    }
    if link.advice_task_code.trim().is_empty() {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::EmptyTaskCode);
    }
    if link.source_ref.trim().is_empty() {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::EmptySourceReference);
    }
    if link.recorded_at_epoch_seconds == 0 {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::InvalidRecordedTimestamp);
    }
    if request.request_id.trim().is_empty() || link.service_request_ref != request.request_id {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::RequestReferenceMismatch);
    }
    if link.advice_scope_ref != request.scope_ref {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::ScopeMismatch);
    }
    if link.advice_period_ref != request.period_ref {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::PeriodMismatch);
    }
    if link.advice_task_code != request.task_code {
        blockers.insert(AdviceServiceLinkBlockerCodeV1::TaskMismatch);
    }
    match link.authorization_ref.as_deref() {
        Some(value) if value.trim().is_empty() => {
            blockers.insert(AdviceServiceLinkBlockerCodeV1::EmptyAuthorizationReference);
        }
        None if link.state == AdviceServiceLinkStateV1::Linked => {
            blockers.insert(AdviceServiceLinkBlockerCodeV1::MissingAuthorizationReference);
        }
        _ => {}
    }

    let mut outcomes = BTreeSet::new();
    for outcome_ref in &link.observed_outcome_claim_refs {
        if outcome_ref.trim().is_empty() {
            blockers.insert(AdviceServiceLinkBlockerCodeV1::EmptyOutcomeReference);
        }
        if !outcomes.insert(outcome_ref) {
            blockers.insert(AdviceServiceLinkBlockerCodeV1::DuplicateOutcomeReference);
        }
    }

    let blockers: Vec<_> = blockers.into_iter().collect();
    let disposition = if !blockers.is_empty() {
        AdviceServiceLinkDispositionV1::Blocked
    } else {
        match link.state {
            AdviceServiceLinkStateV1::Linked => AdviceServiceLinkDispositionV1::LinkedReferenceValidated,
            AdviceServiceLinkStateV1::RevokedForFutureSharing => AdviceServiceLinkDispositionV1::SharingRevoked,
            AdviceServiceLinkStateV1::Pending
            | AdviceServiceLinkStateV1::Stale
            | AdviceServiceLinkStateV1::Disputed
            | AdviceServiceLinkStateV1::Unavailable => AdviceServiceLinkDispositionV1::RequiresResolution,
        }
    };

    AdviceServiceLinkAssessmentV1 {
        disposition,
        blockers,
        service_request_remains_valid: true,
        disclosure_authorized_by_this_module: false,
        causal_claim_permitted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_robot_cost::ServiceUnitV1;
    use crate::service_access::{ServiceAccessEvidenceV1, TravelChargeTreatmentV1};

    fn request() -> ServiceAccessRequestV1 {
        ServiceAccessRequestV1 {
            request_id: "service-request-001".to_owned(),
            scenario_id: "synthetic-scenario-001".to_owned(),
            scope_ref: "opaque-site-a".to_owned(),
            period_ref: "season-2026-b".to_owned(),
            currency_code: "ZAR".to_owned(),
            task_code: "irrigation-pump-repair".to_owned(),
            service_unit: ServiceUnitV1::MissionsCompleted,
            requested_units: 1,
            requested_at_minute: 10,
            useful_window_deadline_minute: 120,
            quote_at_minute: None,
            quoted_fee_minor: None,
            price_basis_unit: None,
            quote_valid_until_minute: None,
            travel_charge_treatment: Some(TravelChargeTreatmentV1::Unknown),
            travel_fee_minor: None,
            booking_at_minute: None,
            completion_at_minute: None,
            delivered_units: None,
            actual_travel_minutes: None,
            delivery_decision_at_minute: None,
            delivery_decision: None,
            no_outcome_reason: None,
            cohort_ref: None,
            cohort_consent: false,
            evidence: ServiceAccessEvidenceV1::synthetic_fixture("service-request-001"),
        }
    }

    fn link() -> AdviceServiceLinkV1 {
        AdviceServiceLinkV1 {
            schema_version: ADVICE_SERVICE_LINK_SCHEMA_VERSION_V1,
            link_id: "link-001".to_owned(),
            service_request_ref: "service-request-001".to_owned(),
            advice_episode_ref: "advice-episode-001".to_owned(),
            decision_ref: Some("decision-001".to_owned()),
            advice_scope_ref: "opaque-site-a".to_owned(),
            advice_period_ref: "season-2026-b".to_owned(),
            advice_task_code: "irrigation-pump-repair".to_owned(),
            reason: AdviceServiceLinkReasonV1::FarmerDeclared,
            initiated_by_role: AdviceServiceLinkInitiatorRoleV1::Farmer,
            state: AdviceServiceLinkStateV1::Linked,
            authorization_ref: Some("mycelix-consent-receipt-001".to_owned()),
            observed_outcome_claim_refs: vec!["observed-flow-claim-001".to_owned()],
            source_ref: "synthetic-fixture:advice-service-link:001".to_owned(),
            recorded_at_epoch_seconds: 1_790_000_000,
        }
    }

    #[test]
    fn service_request_is_valid_without_an_advice_link() {
        let result = assess_advice_service_link_v1(&request(), None);
        assert_eq!(result.disposition, AdviceServiceLinkDispositionV1::UnlinkedServiceRequest);
        assert!(result.service_request_remains_valid);
        assert!(result.blockers.is_empty());
        assert!(!result.disclosure_authorized_by_this_module);
        assert!(!result.causal_claim_permitted);
    }

    #[test]
    fn valid_reference_link_does_not_authorize_disclosure_or_causality() {
        let result = assess_advice_service_link_v1(&request(), Some(&link()));
        assert_eq!(result.disposition, AdviceServiceLinkDispositionV1::LinkedReferenceValidated);
        assert!(result.blockers.is_empty());
        assert!(result.service_request_remains_valid);
        assert!(!result.disclosure_authorized_by_this_module);
        assert!(!result.causal_claim_permitted);
    }

    #[test]
    fn cross_scope_period_and_task_links_fail_closed() {
        let request = request();
        let cases: [(AdviceServiceLinkBlockerCodeV1, fn(&mut AdviceServiceLinkV1)); 3] = [
            (
                AdviceServiceLinkBlockerCodeV1::ScopeMismatch,
                |link| link.advice_scope_ref = "other-site".to_owned(),
            ),
            (
                AdviceServiceLinkBlockerCodeV1::PeriodMismatch,
                |link| link.advice_period_ref = "other-season".to_owned(),
            ),
            (
                AdviceServiceLinkBlockerCodeV1::TaskMismatch,
                |link| link.advice_task_code = "harvest".to_owned(),
            ),
        ];
        for (expected, mutate) in cases {
            let mut link = link();
            mutate(&mut link);
            let result = assess_advice_service_link_v1(&request, Some(&link));
            assert_eq!(result.disposition, AdviceServiceLinkDispositionV1::Blocked);
            assert!(result.blockers.contains(&expected));
            assert!(result.service_request_remains_valid);
        }
    }

    #[test]
    fn request_reference_mismatch_does_not_invalidate_service_request() {
        let mut link = link();
        link.service_request_ref = "different-service-request".to_owned();
        let result = assess_advice_service_link_v1(&request(), Some(&link));
        assert_eq!(result.disposition, AdviceServiceLinkDispositionV1::Blocked);
        assert!(result.blockers.contains(&AdviceServiceLinkBlockerCodeV1::RequestReferenceMismatch));
        assert!(result.service_request_remains_valid);
    }

    #[test]
    fn linked_state_requires_authorization_reference_but_does_not_authenticate_it() {
        let mut link = link();
        link.authorization_ref = None;
        let result = assess_advice_service_link_v1(&request(), Some(&link));
        assert_eq!(result.disposition, AdviceServiceLinkDispositionV1::Blocked);
        assert!(result.blockers.contains(&AdviceServiceLinkBlockerCodeV1::MissingAuthorizationReference));

        let mut link = link();
        link.authorization_ref = Some("sha256:".to_owned() + &"0".repeat(64));
        let result = assess_advice_service_link_v1(&request(), Some(&link));
        assert_eq!(result.disposition, AdviceServiceLinkDispositionV1::LinkedReferenceValidated);
        assert!(!result.disclosure_authorized_by_this_module);
    }

    #[test]
    fn stale_disputed_unavailable_and_revoked_links_never_grant_disclosure() {
        let states = [
            (AdviceServiceLinkStateV1::Stale, AdviceServiceLinkDispositionV1::RequiresResolution),
            (AdviceServiceLinkStateV1::Disputed, AdviceServiceLinkDispositionV1::RequiresResolution),
            (AdviceServiceLinkStateV1::Unavailable, AdviceServiceLinkDispositionV1::RequiresResolution),
            (AdviceServiceLinkStateV1::RevokedForFutureSharing, AdviceServiceLinkDispositionV1::SharingRevoked),
        ];
        for (state, expected) in states {
            let mut link = link();
            link.state = state;
            let result = assess_advice_service_link_v1(&request(), Some(&link));
            assert_eq!(result.disposition, expected);
            assert!(!result.disclosure_authorized_by_this_module);
            assert!(!result.causal_claim_permitted);
        }
    }

    #[test]
    fn outcome_refs_must_be_nonempty_and_unique() {
        let mut link = link();
        link.observed_outcome_claim_refs.push("observed-flow-claim-001".to_owned());
        let result = assess_advice_service_link_v1(&request(), Some(&link));
        assert!(result.blockers.contains(&AdviceServiceLinkBlockerCodeV1::DuplicateOutcomeReference));

        let mut link = link();
        link.observed_outcome_claim_refs.push("  ".to_owned());
        let result = assess_advice_service_link_v1(&request(), Some(&link));
        assert!(result.blockers.contains(&AdviceServiceLinkBlockerCodeV1::EmptyOutcomeReference));
    }

    #[test]
    fn digest_shaped_source_ref_does_not_change_claim_ceiling() {
        let mut link = link();
        link.source_ref = format!("sha256:{}", "0".repeat(64));
        let result = assess_advice_service_link_v1(&request(), Some(&link));
        assert_eq!(result.disposition, AdviceServiceLinkDispositionV1::LinkedReferenceValidated);
        assert!(!result.disclosure_authorized_by_this_module);
        assert!(!result.causal_claim_permitted);
    }
}
