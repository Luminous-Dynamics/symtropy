// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Farmer-confirmed need episodes that may or may not become service requests.
//!
//! This module is separate from the canonical service-access lifecycle. It
//! accounts only for explicit need observations and their provenance; it does
//! not infer need from non-use, create requests, dispatch providers, authenticate
//! source evidence, or estimate causal impact.

use serde::{Deserialize, Serialize};

pub const NEED_OBSERVATION_SCHEMA_VERSION_V1: u16 = 1;
pub const NEED_OBSERVATION_CLAIM_CEILING_V1: &str = "Synthetic need-episode accounting and contract consistency only. Does not establish the prevalence of unmet need, authenticate farmer or intermediary identity, verify service delivery, infer causality, trigger service actions, or prove South African field outcomes.";

fn serialize_u64<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse::<u64>()
        .map_err(serde::de::Error::custom)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedReportBasisV1 {
    FarmerConfirmedProspective,
    FarmerConfirmedRetrospective,
    AuthorizedRelayWithFarmerConfirmation,
    UnverifiedIntermediaryRelay,
    InferredFromNonUseTelemetry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedTimingV1 {
    Anticipated,
    Current,
    Retrospective,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedRequestPresenceV1 {
    LinkedCanonicalRequest,
    FarmerConfirmsNoRequest,
    NoCanonicalRequestLocated,
    RequestExistenceUnknown,
    NotAssessed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedDispositionV1 {
    AdequateServiceWithinWindow,
    ServiceLateOrInadequate,
    NeedStillOpenNoAdequateService,
    FarmerDeferredNeed,
    ResolvedWithoutService,
    OutcomeUnknown,
    FollowUpMissing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedOutcomeSourceV1 {
    CanonicalServiceLifecycle,
    FarmerReportedOtherChannel,
    AuthorizedRelayOtherChannel,
    NotApplicable,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoRequestReasonV1 {
    ExpectedUnaffordable,
    NoProviderKnown,
    TimingMismatch,
    TaskMismatch,
    ChannelInaccessible,
    TrustOrPrivacyConcern,
    RequiredInputsUnavailable,
    ExistingTrustedChannelChosen,
    ChoseNotToProceed,
    Other,
    Unknown,
    Unanswered,
    NotAsked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedExclusionReasonV1 {
    OutsideDeclaredTask,
    OutsideDeclaredPeriod,
    DuplicateCandidate,
    TrainingOrTestRecord,
    ConsentNotGranted,
    OtherPredeclaredRule,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedExclusionCountsV1 {
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub outside_declared_task_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub outside_declared_period_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub duplicate_candidate_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub training_or_test_record_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub consent_not_granted_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub other_predeclared_rule_count: u64,
}

impl NeedExclusionCountsV1 {
    fn checked_total(&self) -> Option<u64> {
        [
            self.outside_declared_task_count,
            self.outside_declared_period_count,
            self.duplicate_candidate_count,
            self.training_or_test_record_count,
            self.consent_not_granted_count,
            self.other_predeclared_rule_count,
        ]
        .into_iter()
        .try_fold(0_u64, u64::checked_add)
    }

    pub fn is_empty(&self) -> bool {
        self.checked_total() == Some(0)
    }
}

impl Default for NeedExclusionCountsV1 {
    fn default() -> Self {
        Self {
            outside_declared_task_count: 0,
            outside_declared_period_count: 0,
            duplicate_candidate_count: 0,
            training_or_test_record_count: 0,
            consent_not_granted_count: 0,
            other_predeclared_rule_count: 0,
        }
    }
}

/// One confirmed or attempted-to-be-confirmed need episode. This is not a
/// request record and must not copy price, booking, completion or payment state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedEpisodeV1 {
    pub need_ref: String,
    pub task_code: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub useful_window_rule_ref: String,
    pub report_basis: NeedReportBasisV1,
    /// Separates anticipated, current and recalled need even within one report method.
    pub need_timing: NeedTimingV1,
    /// Required only for an explicitly authorized intermediary relay; opaque.
    pub authority_ref: Option<String>,
    pub request_presence: NeedRequestPresenceV1,
    /// Points to the canonical Symtropy request only when one actually exists.
    pub request_ref: Option<String>,
    pub disposition: NeedDispositionV1,
    pub outcome_source: NeedOutcomeSourceV1,
    /// Only meaningful when the farmer explicitly confirms that no request was made.
    pub no_request_reason: Option<NoRequestReasonV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedObservationInputV1 {
    pub schema_version: u16,
    pub study_ref: String,
    pub task_code: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub useful_window_rule_ref: String,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub screened_candidate_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub protocol_excluded_count: u64,
    pub exclusions: NeedExclusionCountsV1,
    pub episodes: Vec<NeedEpisodeV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedObservationDispositionV1 {
    Calculated,
    NotAssessable,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedObservationBlockerCodeV1 {
    UnsupportedSchema,
    MissingOrUnsafeReference,
    CountOverflow,
    ScreenedCountMismatch,
    ExclusionCountMismatch,
    DuplicateNeedReference,
    ScopeOrTaskMismatch,
    NeedTimingMismatch,
    UnqualifiedNeedProvenance,
    AuthorityReferenceMismatch,
    RequestReferenceMismatch,
    NoRequestReasonMismatch,
    OutcomeSourceMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedObservationBlockerV1 {
    pub code: NeedObservationBlockerCodeV1,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedReasonCountV1 {
    pub reason: NoRequestReasonV1,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedObservationSummaryV1 {
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub screened_candidate_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub eligible_need_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub protocol_excluded_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub prospective_farmer_report_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub retrospective_farmer_recall_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub authorized_relay_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub anticipated_need_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub current_need_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub retrospective_timing_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub linked_canonical_request_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub farmer_confirms_no_request_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub no_canonical_request_located_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub request_existence_unknown_or_not_assessed_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub adequate_service_within_window_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub canonical_service_attainment_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub other_channel_reported_attainment_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub late_or_inadequate_service_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub open_need_without_adequate_service_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub farmer_deferred_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub resolved_without_service_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub outcome_unknown_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub follow_up_missing_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub no_request_reason_not_recorded_count: u64,
    pub no_request_reason_counts: Vec<NeedReasonCountV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedObservationAssessmentV1 {
    pub study_ref: String,
    pub disposition: NeedObservationDispositionV1,
    pub summary: Option<NeedObservationSummaryV1>,
    pub blockers: Vec<NeedObservationBlockerV1>,
    pub claim_ceiling: String,
}

fn is_safe_opaque_ref(value: &str) -> bool {
    (3..=96).contains(&value.len())
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn has_prefix(value: &str, prefix: &str) -> bool {
    is_safe_opaque_ref(value)
        && value
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.len() >= 3)
}

fn push_blocker(
    blockers: &mut Vec<NeedObservationBlockerV1>,
    code: NeedObservationBlockerCodeV1,
    detail: impl Into<String>,
) {
    let blocker = NeedObservationBlockerV1 {
        code,
        detail: detail.into(),
    };
    if !blockers.contains(&blocker) {
        blockers.push(blocker);
    }
}

fn valid_positive_outcome(disposition: NeedDispositionV1) -> bool {
    matches!(
        disposition,
        NeedDispositionV1::AdequateServiceWithinWindow
            | NeedDispositionV1::ServiceLateOrInadequate
    )
}

fn validate_episode(
    episode: &NeedEpisodeV1,
    input: &NeedObservationInputV1,
    blockers: &mut Vec<NeedObservationBlockerV1>,
) {
    for (field, value) in [
        ("need_ref", episode.need_ref.as_str()),
        ("task_code", episode.task_code.as_str()),
        ("scope_ref", episode.scope_ref.as_str()),
        ("period_ref", episode.period_ref.as_str()),
        ("useful_window_rule_ref", episode.useful_window_rule_ref.as_str()),
    ] {
        if !is_safe_opaque_ref(value) {
            push_blocker(
                blockers,
                NeedObservationBlockerCodeV1::MissingOrUnsafeReference,
                format!("{field} must be a safe opaque lowercase identifier"),
            );
        }
    }
    if !has_prefix(&episode.need_ref, "need-") {
        push_blocker(
            blockers,
            NeedObservationBlockerCodeV1::MissingOrUnsafeReference,
            "need_ref must use the need- namespace",
        );
    }
    if episode.task_code != input.task_code
        || episode.scope_ref != input.scope_ref
        || episode.period_ref != input.period_ref
        || episode.useful_window_rule_ref != input.useful_window_rule_ref
    {
        push_blocker(
            blockers,
            NeedObservationBlockerCodeV1::ScopeOrTaskMismatch,
            "episode task, scope, period and useful-window rule must match the declared study input",
        );
    }

    match episode.report_basis {
        NeedReportBasisV1::FarmerConfirmedProspective
        | NeedReportBasisV1::FarmerConfirmedRetrospective => {
            if episode.authority_ref.is_some() {
                push_blocker(
                    blockers,
                    NeedObservationBlockerCodeV1::AuthorityReferenceMismatch,
                    "direct farmer reports must not carry an intermediary authority reference",
                );
            }
            let timing_matches = match episode.report_basis {
                NeedReportBasisV1::FarmerConfirmedProspective => {
                    episode.need_timing != NeedTimingV1::Retrospective
                }
                NeedReportBasisV1::FarmerConfirmedRetrospective => {
                    episode.need_timing == NeedTimingV1::Retrospective
                }
                _ => true,
            };
            if !timing_matches {
                push_blocker(
                    blockers,
                    NeedObservationBlockerCodeV1::NeedTimingMismatch,
                    "need_timing must agree with prospective versus retrospective report basis",
                );
            }
        }
        NeedReportBasisV1::AuthorizedRelayWithFarmerConfirmation => {
            if !episode
                .authority_ref
                .as_deref()
                .is_some_and(|value| has_prefix(value, "auth-"))
            {
                push_blocker(
                    blockers,
                    NeedObservationBlockerCodeV1::AuthorityReferenceMismatch,
                    "an authorized relay requires an opaque auth- reference",
                );
            }
        }
        NeedReportBasisV1::UnverifiedIntermediaryRelay
        | NeedReportBasisV1::InferredFromNonUseTelemetry => {
            push_blocker(
                blockers,
                NeedObservationBlockerCodeV1::UnqualifiedNeedProvenance,
                "unverified relay and inference from non-use are not eligible need observations",
            );
        }
    }

    if let Some(request_ref) = episode.request_ref.as_deref() {
        if !has_prefix(request_ref, "req-") {
            push_blocker(
                blockers,
                NeedObservationBlockerCodeV1::MissingOrUnsafeReference,
                "request_ref must use the canonical req- namespace",
            );
        }
    }
    let request_ref_present = episode.request_ref.is_some();
    if (episode.request_presence == NeedRequestPresenceV1::LinkedCanonicalRequest)
        != request_ref_present
    {
        push_blocker(
            blockers,
            NeedObservationBlockerCodeV1::RequestReferenceMismatch,
            "only LinkedCanonicalRequest may carry request_ref, and that state requires one",
        );
    }

    if episode.no_request_reason.is_some()
        && episode.request_presence != NeedRequestPresenceV1::FarmerConfirmsNoRequest
    {
        push_blocker(
            blockers,
            NeedObservationBlockerCodeV1::NoRequestReasonMismatch,
            "a no-request reason is valid only when the farmer explicitly confirms no request was made",
        );
    }

    let positive_outcome = valid_positive_outcome(episode.disposition);
    if positive_outcome {
        let source_ok = match episode.outcome_source {
            NeedOutcomeSourceV1::CanonicalServiceLifecycle => {
                episode.request_presence == NeedRequestPresenceV1::LinkedCanonicalRequest
            }
            NeedOutcomeSourceV1::FarmerReportedOtherChannel => {
                episode.request_presence != NeedRequestPresenceV1::FarmerConfirmsNoRequest
                    && matches!(
                        episode.report_basis,
                        NeedReportBasisV1::FarmerConfirmedProspective
                            | NeedReportBasisV1::FarmerConfirmedRetrospective
                    )
            }
            NeedOutcomeSourceV1::AuthorizedRelayOtherChannel => {
                episode.request_presence != NeedRequestPresenceV1::FarmerConfirmsNoRequest
                    && episode.report_basis
                        == NeedReportBasisV1::AuthorizedRelayWithFarmerConfirmation
            }
            NeedOutcomeSourceV1::NotApplicable | NeedOutcomeSourceV1::Unknown => false,
        };
        if !source_ok {
            push_blocker(
                blockers,
                NeedObservationBlockerCodeV1::OutcomeSourceMismatch,
                "service-attainment claims need a compatible source; other-channel reports must not become canonical platform receipts",
            );
        }
    } else if matches!(
        episode.outcome_source,
        NeedOutcomeSourceV1::CanonicalServiceLifecycle
            | NeedOutcomeSourceV1::FarmerReportedOtherChannel
            | NeedOutcomeSourceV1::AuthorizedRelayOtherChannel
    ) {
        push_blocker(
            blockers,
            NeedObservationBlockerCodeV1::OutcomeSourceMismatch,
            "a non-attainment disposition cannot carry a source that asserts delivered service",
        );
    }
}

pub fn assess_need_observation_v1(
    input: &NeedObservationInputV1,
) -> NeedObservationAssessmentV1 {
    let mut blockers = Vec::new();
    if input.schema_version != NEED_OBSERVATION_SCHEMA_VERSION_V1 {
        push_blocker(
            &mut blockers,
            NeedObservationBlockerCodeV1::UnsupportedSchema,
            "unsupported need-observation schema version",
        );
    }

    for (field, value) in [
        ("study_ref", input.study_ref.as_str()),
        ("task_code", input.task_code.as_str()),
        ("scope_ref", input.scope_ref.as_str()),
        ("period_ref", input.period_ref.as_str()),
        ("useful_window_rule_ref", input.useful_window_rule_ref.as_str()),
    ] {
        if !is_safe_opaque_ref(value) {
            push_blocker(
                &mut blockers,
                NeedObservationBlockerCodeV1::MissingOrUnsafeReference,
                format!("{field} must be a safe opaque lowercase identifier"),
            );
        }
    }
    if !has_prefix(&input.study_ref, "study-") {
        push_blocker(
            &mut blockers,
            NeedObservationBlockerCodeV1::MissingOrUnsafeReference,
            "study_ref must use the study- namespace",
        );
    }

    let eligible_count = u64::try_from(input.episodes.len()).ok();
    let exclusion_total = input.exclusions.checked_total();
    let expected_screened = eligible_count.and_then(|eligible| {
        input.protocol_excluded_count.checked_add(eligible)
    });
    if eligible_count.is_none()
        || exclusion_total.is_none()
        || expected_screened.is_none()
    {
        push_blocker(
            &mut blockers,
            NeedObservationBlockerCodeV1::CountOverflow,
            "candidate, eligible or exclusion counts exceed supported arithmetic",
        );
    }
    if exclusion_total != Some(input.protocol_excluded_count) {
        push_blocker(
            &mut blockers,
            NeedObservationBlockerCodeV1::ExclusionCountMismatch,
            "declared protocol exclusions must reconcile to the typed exclusion counts",
        );
    }
    if expected_screened != Some(input.screened_candidate_count) {
        push_blocker(
            &mut blockers,
            NeedObservationBlockerCodeV1::ScreenedCountMismatch,
            "screened candidates must equal eligible need episodes plus protocol exclusions",
        );
    }

    let mut seen_need_refs = std::collections::BTreeSet::new();
    for episode in &input.episodes {
        if !seen_need_refs.insert(episode.need_ref.as_str()) {
            push_blocker(
                &mut blockers,
                NeedObservationBlockerCodeV1::DuplicateNeedReference,
                "need_ref values must be unique within one study report",
            );
        }
        validate_episode(episode, input, &mut blockers);
    }
    blockers.sort_by_key(|blocker| (blocker.code as u8, blocker.detail.clone()));

    let disposition = if !blockers.is_empty() {
        NeedObservationDispositionV1::Blocked
    } else if input.episodes.is_empty() {
        NeedObservationDispositionV1::NotAssessable
    } else {
        NeedObservationDispositionV1::Calculated
    };

    let summary = if disposition == NeedObservationDispositionV1::Calculated {
        Some(build_summary(input))
    } else {
        None
    };

    NeedObservationAssessmentV1 {
        study_ref: input.study_ref.clone(),
        disposition,
        summary,
        blockers,
        claim_ceiling: NEED_OBSERVATION_CLAIM_CEILING_V1.to_owned(),
    }
}

fn build_summary(input: &NeedObservationInputV1) -> NeedObservationSummaryV1 {
    use NeedDispositionV1 as D;
    use NeedOutcomeSourceV1 as S;
    use NeedReportBasisV1 as B;
    use NeedRequestPresenceV1 as R;

    let mut summary = NeedObservationSummaryV1 {
        screened_candidate_count: input.screened_candidate_count,
        eligible_need_count: input.episodes.len() as u64,
        protocol_excluded_count: input.protocol_excluded_count,
        prospective_farmer_report_count: 0,
        retrospective_farmer_recall_count: 0,
        authorized_relay_count: 0,
        anticipated_need_count: 0,
        current_need_count: 0,
        retrospective_timing_count: 0,
        linked_canonical_request_count: 0,
        farmer_confirms_no_request_count: 0,
        no_canonical_request_located_count: 0,
        request_existence_unknown_or_not_assessed_count: 0,
        adequate_service_within_window_count: 0,
        canonical_service_attainment_count: 0,
        other_channel_reported_attainment_count: 0,
        late_or_inadequate_service_count: 0,
        open_need_without_adequate_service_count: 0,
        farmer_deferred_count: 0,
        resolved_without_service_count: 0,
        outcome_unknown_count: 0,
        follow_up_missing_count: 0,
        no_request_reason_not_recorded_count: 0,
        no_request_reason_counts: Vec::new(),
    };
    let mut reason_counts = std::collections::BTreeMap::<NoRequestReasonV1, u64>::new();

    for episode in &input.episodes {
        match episode.report_basis {
            B::FarmerConfirmedProspective => summary.prospective_farmer_report_count += 1,
            B::FarmerConfirmedRetrospective => summary.retrospective_farmer_recall_count += 1,
            B::AuthorizedRelayWithFarmerConfirmation => summary.authorized_relay_count += 1,
            B::UnverifiedIntermediaryRelay | B::InferredFromNonUseTelemetry => unreachable!("blocked above"),
        }

        match episode.need_timing {
            NeedTimingV1::Anticipated => summary.anticipated_need_count += 1,
            NeedTimingV1::Current => summary.current_need_count += 1,
            NeedTimingV1::Retrospective => summary.retrospective_timing_count += 1,
        }

        match episode.request_presence {
            R::LinkedCanonicalRequest => summary.linked_canonical_request_count += 1,
            R::FarmerConfirmsNoRequest => summary.farmer_confirms_no_request_count += 1,
            R::NoCanonicalRequestLocated => summary.no_canonical_request_located_count += 1,
            R::RequestExistenceUnknown | R::NotAssessed => {
                summary.request_existence_unknown_or_not_assessed_count += 1
            }
        }

        if episode.request_presence == R::FarmerConfirmsNoRequest {
            if let Some(reason) = episode.no_request_reason {
                *reason_counts.entry(reason).or_default() += 1;
            } else {
                summary.no_request_reason_not_recorded_count += 1;
            }
        }

        match episode.disposition {
            D::AdequateServiceWithinWindow => {
                summary.adequate_service_within_window_count += 1;
                match episode.outcome_source {
                    S::CanonicalServiceLifecycle => summary.canonical_service_attainment_count += 1,
                    S::FarmerReportedOtherChannel | S::AuthorizedRelayOtherChannel => {
                        summary.other_channel_reported_attainment_count += 1
                    }
                    S::NotApplicable | S::Unknown => unreachable!("blocked above"),
                }
            }
            D::ServiceLateOrInadequate => summary.late_or_inadequate_service_count += 1,
            D::NeedStillOpenNoAdequateService => summary.open_need_without_adequate_service_count += 1,
            D::FarmerDeferredNeed => summary.farmer_deferred_count += 1,
            D::ResolvedWithoutService => summary.resolved_without_service_count += 1,
            D::OutcomeUnknown => summary.outcome_unknown_count += 1,
            D::FollowUpMissing => summary.follow_up_missing_count += 1,
        }
    }

    summary.no_request_reason_counts = reason_counts
        .into_iter()
        .map(|(reason, count)| NeedReasonCountV1 { reason, count })
        .collect();
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusion_counts_use_checked_arithmetic() {
        let counts = NeedExclusionCountsV1 {
            outside_declared_task_count: u64::MAX,
            ..NeedExclusionCountsV1::default()
        };
        assert_eq!(counts.checked_total(), Some(u64::MAX));
    }
}
