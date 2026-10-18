// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Machine-checkable preflight for farmer-service field-study reports.
//!
//! This module audits the study cohort and missingness bounds. It deliberately
//! does not duplicate the canonical request/quote/booking/service lifecycle.
//! All identifiers are opaque references; this is not a privacy or identity proof.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const FIELD_EVAL_REPORT_SCHEMA_VERSION_V1: u16 = 1;
const CLAIM_CEILING: &str = "Field-report accounting and identification-bound preflight only. Does not prove source authenticity, consent, identity, provider delivery, production lifecycle correctness, field representativeness, causality, or agricultural impact.";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldStudyManifestV1 {
    pub schema_version: u16,
    pub report_ref: String,
    pub protocol_ref: String,
    pub analysis_plan_ref: String,
    /// Frozen rule identifiers from the declared analysis plan. This cross-checks
    /// exclusion references but does not itself prove when the plan was registered.
    pub preregistered_exclusion_rule_refs: Vec<String>,
    /// Every screened candidate, including those explicitly excluded.
    pub screened_request_refs: Vec<String>,
    /// The fixed primary denominator, frozen before outcome inspection.
    pub eligible_request_refs: Vec<String>,
    /// Every screened but ineligible request, with a preregistered rule reference.
    pub exclusions: Vec<StudyExclusionV1>,
    /// Exactly one outcome classification for each eligible request.
    pub outcome_records: Vec<RequestOutcomeRecordV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudyExclusionV1 {
    pub request_ref: String,
    pub reason_code: StudyExclusionReasonV1,
    pub preregistered_rule_ref: String,
    pub exclusion_evidence_ref: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudyExclusionReasonV1 {
    OutsideDeclaredScope,
    IneligibleServiceTask,
    DuplicateBeforeEnrollment,
    WithdrawnUnderApprovedRule,
    TestOrAdministrativeRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceDecisionModeV1 {
    FarmerDirect,
    AuthorizedIntermediaryRecorded,
}

fn serialize_u64_decimal_string<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_u64_decimal_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse::<u64>()
        .map_err(serde::de::Error::custom)
}

mod optional_u64_decimal_string {
    use super::*;

    pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&value.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|raw| raw.parse::<u64>().map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeEvidenceKindV1 {
    FarmerDecisionEvent,
    AuthorizedRecordedFarmerDecisionEvent,
    ProviderAvailabilityCheck,
    ProviderDeclineEvent,
    ProviderCancellationEvent,
    ProviderNoShowEvent,
    ProviderCompletionEvent,
    NoAcceptanceAtWindowClose,
    DisputeOrReworkEvent,
    ReconciliationAttempt,
    FollowUpAttempt,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestOutcomeRecordV1 {
    pub request_ref: String,
    /// Event reference for the farmer-declared useful-window deadline.
    pub useful_window_evidence_ref: String,
    /// Event reference supporting the outcome classification (including an
    /// explicit unknown or follow-up-attempt record when no authoritative outcome exists).
    pub outcome_evidence_ref: String,
    pub outcome_evidence_kind: OutcomeEvidenceKindV1,
    /// Minute value from the same declared period origin used by service_access.
    #[serde(
        serialize_with = "serialize_u64_decimal_string",
        deserialize_with = "deserialize_u64_decimal_string"
    )]
    pub useful_window_deadline_minute: u64,
    /// Present only when authoritative evidence supports a farmer acceptance.
    #[serde(
        serialize_with = "optional_u64_decimal_string::serialize",
        deserialize_with = "optional_u64_decimal_string::deserialize"
    )]
    pub acceptance_at_minute: Option<u64>,
    pub acceptance_evidence_ref: Option<String>,
    pub acceptance_decision_mode: Option<AcceptanceDecisionModeV1>,
    pub acceptance_authority_ref: Option<String>,
    pub outcome: RequestOutcomeClassV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "class", rename_all = "snake_case", deny_unknown_fields)]
pub enum RequestOutcomeClassV1 {
    AcceptedWithinUsefulWindow,
    AcceptedLate,
    KnownNotAccepted {
        reason_code: KnownNonAcceptanceReasonV1,
    },
    ProviderCompletionAwaitingFarmerDecision {
        useful_window_status: UsefulWindowStatusV1,
    },
    DisputedOrRework {
        resolution: DisputeResolutionV1,
    },
    OutcomeUnknown,
    FollowUpMissingOrInaccessible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnownNonAcceptanceReasonV1 {
    NoProviderAvailable,
    ProviderDeclined,
    FarmerDeclinedQuote,
    FarmerCancelled,
    ProviderCancelled,
    ProviderNoShow,
    ProviderCompletionNotAccepted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsefulWindowStatusV1 {
    Open,
    Closed,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisputeResolutionV1 {
    AcceptedWithinUsefulWindow,
    AcceptedLate,
    ResolvedWithoutInWindowAcceptance,
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldEvalPreflightStatusV1 {
    Calculated,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldEvalBlockerCodeV1 {
    UnsupportedSchema,
    InvalidReportReference,
    InvalidProtocolReference,
    InvalidAnalysisPlanReference,
    EmptyScreenedCohort,
    EmptyEligibleCohort,
    InvalidRequestReference,
    DuplicateScreenedRequest,
    DuplicateEligibleRequest,
    EligibleRequestNotScreened,
    EligibleRequestAlsoExcluded,
    ExclusionNotScreened,
    DuplicateExclusion,
    MissingExclusion,
    ExclusionRuleReferenceMissing,
    EmptyPreregisteredRuleCatalog,
    InvalidPreregisteredRuleReference,
    DuplicatePreregisteredRule,
    ExclusionRuleNotPreregistered,
    MissingExclusionEvidenceReference,
    InvalidExclusionEvidenceReference,
    InvalidOutcomeEvidenceReference,
    OutcomeEvidenceKindMismatch,
    InvalidUsefulWindowEvidenceReference,
    OutcomeRequestNotEligible,
    DuplicateOutcomeRecord,
    MissingOutcomeRecord,
    InvalidOutcomeShape,
    AcceptanceEvidenceIncomplete,
    UnexpectedAcceptanceEvidence,
    InvalidAcceptanceEventReference,
    AcceptanceTimeContradictsWindow,
    IntermediaryAcceptanceAuthorityMissing,
    DirectAcceptanceUnexpectedAuthority,
    CountOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEvalBlockerV1 {
    pub code: FieldEvalBlockerCodeV1,
    pub request_ref: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactRateV1 {
    pub numerator: u64,
    pub denominator: u64,
    pub interpretation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownNonAcceptanceReasonCountsV1 {
    pub no_provider_available: u64,
    pub provider_declined: u64,
    pub farmer_declined_quote: u64,
    pub farmer_cancelled: u64,
    pub provider_cancelled: u64,
    pub provider_no_show: u64,
    pub provider_completion_not_accepted: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisputeResolutionCountsV1 {
    pub accepted_within_useful_window: u64,
    pub accepted_late: u64,
    pub resolved_without_in_window_acceptance: u64,
    pub unresolved: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEvalOutcomeCountsV1 {
    pub accepted_within_useful_window: u64,
    pub accepted_late: u64,
    pub known_not_accepted: u64,
    pub known_non_acceptance_reasons: KnownNonAcceptanceReasonCountsV1,
    pub provider_completion_awaiting_decision: u64,
    pub disputed_or_rework: u64,
    pub dispute_resolutions: DisputeResolutionCountsV1,
    pub outcome_unknown: u64,
    pub follow_up_missing_or_inaccessible: u64,
    /// Counts in-window acceptance including a dispute resolved after rework.
    /// This matches the numerator of the evidence-confirmed lower bound.
    pub confirmed_in_window_acceptances_including_resolved_rework: u64,
    pub genuinely_unascertainable_for_upper_bound: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEvalPreflightReportV1 {
    pub schema_version: u16,
    pub report_ref: String,
    pub status: FieldEvalPreflightStatusV1,
    pub screened_count: u64,
    pub eligible_denominator: u64,
    pub excluded_count: u64,
    pub outcome_counts: Option<FieldEvalOutcomeCountsV1>,
    /// Confirmed in-window acceptance count / fixed eligible denominator.
    /// This is an evidence lower bound, not a statistical confidence interval.
    pub evidence_confirmed_lower_bound: Option<ExactRateV1>,
    /// The lower bound plus only cases whose in-window acceptance is genuinely
    /// unascertainable. This identification bound is not a confidence interval.
    pub missingness_upper_identification_bound: Option<ExactRateV1>,
    pub outcome_ascertainment_coverage: Option<ExactRateV1>,
    pub blockers: Vec<FieldEvalBlockerV1>,
    pub claim_ceiling: String,
}

fn valid_opaque_reference(value: &str, prefix: &str, min_suffix: usize, max_suffix: usize) -> bool {
    let Some(suffix) = value.strip_prefix(prefix) else {
        return false;
    };
    (min_suffix..=max_suffix).contains(&suffix.len())
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !suffix.starts_with('-')
        && !suffix.ends_with('-')
        && !suffix.contains("--")
}

fn push_blocker(
    blockers: &mut Vec<FieldEvalBlockerV1>,
    code: FieldEvalBlockerCodeV1,
    request_ref: Option<&str>,
    detail: impl Into<String>,
) {
    blockers.push(FieldEvalBlockerV1 {
        code,
        request_ref: request_ref.map(str::to_owned),
        detail: detail.into(),
    });
}

/// Returns true only for a record that can plausibly contain an unobserved
/// in-window acceptance at the declared report cut-off. Known late, no-provider,
/// cancelled, no-show and resolved non-acceptance cases never enter this set.
fn outcome_is_genuinely_unascertainable(outcome: &RequestOutcomeClassV1) -> bool {
    matches!(
        outcome,
        RequestOutcomeClassV1::ProviderCompletionAwaitingFarmerDecision {
            useful_window_status: UsefulWindowStatusV1::Open | UsefulWindowStatusV1::Unknown
        } | RequestOutcomeClassV1::DisputedOrRework {
            resolution: DisputeResolutionV1::Unresolved
        } | RequestOutcomeClassV1::OutcomeUnknown
            | RequestOutcomeClassV1::FollowUpMissingOrInaccessible
    )
}

fn outcome_class_valid(outcome: &RequestOutcomeClassV1) -> bool {
    match outcome {
        RequestOutcomeClassV1::AcceptedWithinUsefulWindow
        | RequestOutcomeClassV1::AcceptedLate
        | RequestOutcomeClassV1::KnownNotAccepted { .. }
        | RequestOutcomeClassV1::OutcomeUnknown
        | RequestOutcomeClassV1::FollowUpMissingOrInaccessible => true,
        RequestOutcomeClassV1::ProviderCompletionAwaitingFarmerDecision {
            useful_window_status,
        } => matches!(
            useful_window_status,
            UsefulWindowStatusV1::Open | UsefulWindowStatusV1::Unknown
        ),
        RequestOutcomeClassV1::DisputedOrRework { resolution } => matches!(
            resolution,
            DisputeResolutionV1::AcceptedWithinUsefulWindow
                | DisputeResolutionV1::AcceptedLate
                | DisputeResolutionV1::ResolvedWithoutInWindowAcceptance
                | DisputeResolutionV1::Unresolved
        ),
    }
}

fn expected_in_window_acceptance(outcome: &RequestOutcomeClassV1) -> Option<bool> {
    match outcome {
        RequestOutcomeClassV1::AcceptedWithinUsefulWindow => Some(true),
        RequestOutcomeClassV1::AcceptedLate => Some(false),
        RequestOutcomeClassV1::DisputedOrRework { resolution } => match resolution {
            DisputeResolutionV1::AcceptedWithinUsefulWindow => Some(true),
            DisputeResolutionV1::AcceptedLate => Some(false),
            DisputeResolutionV1::ResolvedWithoutInWindowAcceptance
            | DisputeResolutionV1::Unresolved => None,
        },
        RequestOutcomeClassV1::KnownNotAccepted { .. }
        | RequestOutcomeClassV1::ProviderCompletionAwaitingFarmerDecision { .. }
        | RequestOutcomeClassV1::OutcomeUnknown
        | RequestOutcomeClassV1::FollowUpMissingOrInaccessible => None,
    }
}

fn outcome_evidence_kind_valid(
    outcome: &RequestOutcomeClassV1,
    evidence_kind: OutcomeEvidenceKindV1,
) -> bool {
    match outcome {
        RequestOutcomeClassV1::AcceptedWithinUsefulWindow
        | RequestOutcomeClassV1::AcceptedLate => matches!(
            evidence_kind,
            OutcomeEvidenceKindV1::FarmerDecisionEvent
                | OutcomeEvidenceKindV1::AuthorizedRecordedFarmerDecisionEvent
        ),
        RequestOutcomeClassV1::KnownNotAccepted { reason_code } => match reason_code {
            KnownNonAcceptanceReasonV1::NoProviderAvailable => {
                evidence_kind == OutcomeEvidenceKindV1::ProviderAvailabilityCheck
            }
            KnownNonAcceptanceReasonV1::ProviderDeclined => {
                evidence_kind == OutcomeEvidenceKindV1::ProviderDeclineEvent
            }
            KnownNonAcceptanceReasonV1::FarmerDeclinedQuote
            | KnownNonAcceptanceReasonV1::FarmerCancelled => {
                evidence_kind == OutcomeEvidenceKindV1::FarmerDecisionEvent
            }
            KnownNonAcceptanceReasonV1::ProviderCancelled => {
                evidence_kind == OutcomeEvidenceKindV1::ProviderCancellationEvent
            }
            KnownNonAcceptanceReasonV1::ProviderNoShow => {
                evidence_kind == OutcomeEvidenceKindV1::ProviderNoShowEvent
            }
            KnownNonAcceptanceReasonV1::ProviderCompletionNotAccepted => {
                evidence_kind == OutcomeEvidenceKindV1::NoAcceptanceAtWindowClose
            }
        },
        RequestOutcomeClassV1::ProviderCompletionAwaitingFarmerDecision { .. } => {
            evidence_kind == OutcomeEvidenceKindV1::ProviderCompletionEvent
        }
        RequestOutcomeClassV1::DisputedOrRework { .. } => {
            evidence_kind == OutcomeEvidenceKindV1::DisputeOrReworkEvent
        }
        RequestOutcomeClassV1::OutcomeUnknown => {
            evidence_kind == OutcomeEvidenceKindV1::ReconciliationAttempt
        }
        RequestOutcomeClassV1::FollowUpMissingOrInaccessible => {
            evidence_kind == OutcomeEvidenceKindV1::FollowUpAttempt
        }
    }
}

fn validate_acceptance_evidence(
    record: &RequestOutcomeRecordV1,
    blockers: &mut Vec<FieldEvalBlockerV1>,
) {
    let expected_in_window = expected_in_window_acceptance(&record.outcome);
    let has_any_acceptance_evidence = record.acceptance_at_minute.is_some()
        || record.acceptance_evidence_ref.is_some()
        || record.acceptance_decision_mode.is_some()
        || record.acceptance_authority_ref.is_some();

    let Some(expected_in_window) = expected_in_window else {
        if has_any_acceptance_evidence {
            push_blocker(
                blockers,
                FieldEvalBlockerCodeV1::UnexpectedAcceptanceEvidence,
                Some(&record.request_ref),
                "non-acceptance or unresolved outcome carries an acceptance event/time/mode",
            );
        }
        return;
    };

    let complete = record.acceptance_at_minute.is_some()
        && record.acceptance_evidence_ref.is_some()
        && record.acceptance_decision_mode.is_some();
    if !complete {
        push_blocker(
            blockers,
            FieldEvalBlockerCodeV1::AcceptanceEvidenceIncomplete,
            Some(&record.request_ref),
            "accepted outcomes require event reference, acceptance time and decision mode",
        );
    }

    if let Some(event_ref) = &record.acceptance_evidence_ref {
        if !valid_opaque_reference(event_ref, "evt-", 3, 64) {
            push_blocker(
                blockers,
                FieldEvalBlockerCodeV1::InvalidAcceptanceEventReference,
                Some(&record.request_ref),
                "acceptance evidence must use an opaque evt-* event reference",
            );
        }
    }

    match record.acceptance_decision_mode {
        Some(AcceptanceDecisionModeV1::FarmerDirect) => {
            if record.outcome_evidence_kind
                != OutcomeEvidenceKindV1::FarmerDecisionEvent
                && matches!(
                    &record.outcome,
                    RequestOutcomeClassV1::AcceptedWithinUsefulWindow
                        | RequestOutcomeClassV1::AcceptedLate
                )
            {
                push_blocker(
                    blockers,
                    FieldEvalBlockerCodeV1::OutcomeEvidenceKindMismatch,
                    Some(&record.request_ref),
                    "direct acceptance outcome must cite farmer-decision evidence",
                );
            }
            if record.acceptance_authority_ref.is_some() {
                push_blocker(
                    blockers,
                    FieldEvalBlockerCodeV1::DirectAcceptanceUnexpectedAuthority,
                    Some(&record.request_ref),
                    "direct farmer acceptance must not claim an intermediary mandate",
                );
            }
        }
        Some(AcceptanceDecisionModeV1::AuthorizedIntermediaryRecorded) => {
            if matches!(
                &record.outcome,
                RequestOutcomeClassV1::AcceptedWithinUsefulWindow
                    | RequestOutcomeClassV1::AcceptedLate
            ) && record.outcome_evidence_kind
                != OutcomeEvidenceKindV1::AuthorizedRecordedFarmerDecisionEvent
            {
                push_blocker(
                    blockers,
                    FieldEvalBlockerCodeV1::OutcomeEvidenceKindMismatch,
                    Some(&record.request_ref),
                    "intermediary-recorded acceptance must cite authorized-recorded decision evidence",
                );
            }
            if !record
                .acceptance_authority_ref
                .as_deref()
                .is_some_and(|reference| valid_opaque_reference(reference, "mandate-", 2, 64))
            {
                push_blocker(
                    blockers,
                    FieldEvalBlockerCodeV1::IntermediaryAcceptanceAuthorityMissing,
                    Some(&record.request_ref),
                    "intermediary-recorded acceptance requires an opaque mandate-* reference",
                );
            }
        }
        None => {}
    }

    if let Some(accepted_at) = record.acceptance_at_minute {
        let actual_in_window = accepted_at <= record.useful_window_deadline_minute;
        if actual_in_window != expected_in_window {
            push_blocker(
                blockers,
                FieldEvalBlockerCodeV1::AcceptanceTimeContradictsWindow,
                Some(&record.request_ref),
                "acceptance timestamp contradicts the claimed useful-window outcome",
            );
        }
    }
}

/// Validate cohort accounting and compute exact rational identification bounds.
/// Any accounting ambiguity blocks all derived rates: no partial denominator is returned.
pub fn preflight_field_study_v1(manifest: &FieldStudyManifestV1) -> FieldEvalPreflightReportV1 {
    let mut blockers = Vec::new();

    if manifest.schema_version != FIELD_EVAL_REPORT_SCHEMA_VERSION_V1 {
        push_blocker(
            &mut blockers,
            FieldEvalBlockerCodeV1::UnsupportedSchema,
            None,
            "unsupported field-study manifest schema version",
        );
    }
    if !valid_opaque_reference(&manifest.report_ref, "study-", 3, 64) {
        push_blocker(
            &mut blockers,
            FieldEvalBlockerCodeV1::InvalidReportReference,
            None,
            "report_ref must be an opaque study-* reference",
        );
    }
    if !valid_opaque_reference(&manifest.protocol_ref, "protocol-", 2, 64) {
        push_blocker(
            &mut blockers,
            FieldEvalBlockerCodeV1::InvalidProtocolReference,
            None,
            "protocol_ref must be an opaque protocol-* reference",
        );
    }
    if !valid_opaque_reference(&manifest.analysis_plan_ref, "analysis-", 2, 64) {
        push_blocker(
            &mut blockers,
            FieldEvalBlockerCodeV1::InvalidAnalysisPlanReference,
            None,
            "analysis_plan_ref must be an opaque analysis-* reference",
        );
    }
    if manifest.screened_request_refs.is_empty() {
        push_blocker(
            &mut blockers,
            FieldEvalBlockerCodeV1::EmptyScreenedCohort,
            None,
            "screened candidate request set cannot be empty",
        );
    }
    if manifest.eligible_request_refs.is_empty() {
        push_blocker(
            &mut blockers,
            FieldEvalBlockerCodeV1::EmptyEligibleCohort,
            None,
            "empty eligible cohort is not a zero-rate result",
        );
    }

    let mut screened: BTreeSet<String> = BTreeSet::new();
    for request_ref in &manifest.screened_request_refs {
        if !valid_opaque_reference(request_ref, "req-", 3, 64) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::InvalidRequestReference,
                Some(request_ref),
                "request reference must be an opaque req-* reference",
            );
        }
        if !screened.insert(request_ref.clone()) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::DuplicateScreenedRequest,
                Some(request_ref),
                "screened request appears more than once",
            );
        }
    }

    let mut eligible: BTreeSet<String> = BTreeSet::new();
    for request_ref in &manifest.eligible_request_refs {
        if !valid_opaque_reference(request_ref, "req-", 3, 64) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::InvalidRequestReference,
                Some(request_ref),
                "eligible request reference must be an opaque req-* reference",
            );
        }
        if !eligible.insert(request_ref.clone()) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::DuplicateEligibleRequest,
                Some(request_ref),
                "eligible request appears more than once in the fixed denominator",
            );
        }
        if !screened.contains(request_ref.as_str()) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::EligibleRequestNotScreened,
                Some(request_ref),
                "eligible request is absent from the screened candidate set",
            );
        }
    }

    let mut preregistered_rules: BTreeSet<String> = BTreeSet::new();
    if !manifest.exclusions.is_empty() && manifest.preregistered_exclusion_rule_refs.is_empty() {
        push_blocker(
            &mut blockers,
            FieldEvalBlockerCodeV1::EmptyPreregisteredRuleCatalog,
            None,
            "exclusions require a declared preregistered rule inventory",
        );
    }
    for rule_ref in &manifest.preregistered_exclusion_rule_refs {
        if !valid_opaque_reference(rule_ref, "rule-", 2, 64) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::InvalidPreregisteredRuleReference,
                None,
                "preregistered exclusion rules must be opaque rule-* references",
            );
        }
        if !preregistered_rules.insert(rule_ref.clone()) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::DuplicatePreregisteredRule,
                None,
                "preregistered exclusion rule inventory contains a duplicate",
            );
        }
    }

    let mut exclusions: BTreeMap<String, &StudyExclusionV1> = BTreeMap::new();
    for exclusion in &manifest.exclusions {
        if !screened.contains(exclusion.request_ref.as_str()) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::ExclusionNotScreened,
                Some(&exclusion.request_ref),
                "exclusion does not belong to the screened candidate set",
            );
        }
        if eligible.contains(exclusion.request_ref.as_str()) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::EligibleRequestAlsoExcluded,
                Some(&exclusion.request_ref),
                "eligible request cannot also be excluded",
            );
        }
        if !valid_opaque_reference(&exclusion.preregistered_rule_ref, "rule-", 2, 64) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::ExclusionRuleReferenceMissing,
                Some(&exclusion.request_ref),
                "exclusion requires a preregistered opaque rule-* reference",
            );
        }
        if !valid_opaque_reference(&exclusion.exclusion_evidence_ref, "evt-", 3, 64) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::InvalidExclusionEvidenceReference,
                Some(&exclusion.request_ref),
                "exclusion requires an opaque evt-* source-event reference",
            );
        }
        if !preregistered_rules.contains(&exclusion.preregistered_rule_ref) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::ExclusionRuleNotPreregistered,
                Some(&exclusion.request_ref),
                "exclusion rule reference is not listed in the frozen rule inventory",
            );
        }
        if exclusions
            .insert(exclusion.request_ref.clone(), exclusion)
            .is_some()
        {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::DuplicateExclusion,
                Some(&exclusion.request_ref),
                "screened request has more than one exclusion row",
            );
        }
    }

    for request_ref in &manifest.screened_request_refs {
        let is_eligible = eligible.contains(request_ref.as_str());
        let is_excluded = exclusions.contains_key(request_ref.as_str());
        if !is_eligible && !is_excluded {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::MissingExclusion,
                Some(request_ref),
                "screened ineligible request has no explicit exclusion reason",
            );
        }
        if is_eligible && is_excluded {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::EligibleRequestAlsoExcluded,
                Some(request_ref),
                "eligible request cannot also be present in exclusions",
            );
        }
    }

    let mut outcomes: BTreeMap<String, &RequestOutcomeRecordV1> = BTreeMap::new();
    for record in &manifest.outcome_records {
        if !eligible.contains(record.request_ref.as_str()) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::OutcomeRequestNotEligible,
                Some(&record.request_ref),
                "outcome record does not belong to the fixed eligible cohort",
            );
        }
        if !outcome_class_valid(&record.outcome) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::InvalidOutcomeShape,
                Some(&record.request_ref),
                "outcome classification conflicts with its useful-window state",
            );
        }
        if !valid_opaque_reference(&record.useful_window_evidence_ref, "evt-", 3, 64) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::InvalidUsefulWindowEvidenceReference,
                Some(&record.request_ref),
                "useful-window deadline requires an opaque evt-* source-event reference",
            );
        }
        if !valid_opaque_reference(&record.outcome_evidence_ref, "evt-", 3, 64) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::InvalidOutcomeEvidenceReference,
                Some(&record.request_ref),
                "outcome classification requires an opaque evt-* source-event reference",
            );
        }
        if !outcome_evidence_kind_valid(&record.outcome, record.outcome_evidence_kind) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::OutcomeEvidenceKindMismatch,
                Some(&record.request_ref),
                "outcome evidence kind does not support the declared outcome class",
            );
        }
        validate_acceptance_evidence(record, &mut blockers);
        if outcomes
            .insert(record.request_ref.clone(), record)
            .is_some()
        {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::DuplicateOutcomeRecord,
                Some(&record.request_ref),
                "eligible request has multiple primary outcome classifications",
            );
        }
    }

    for request_ref in &manifest.eligible_request_refs {
        if !outcomes.contains_key(request_ref.as_str()) {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::MissingOutcomeRecord,
                Some(request_ref),
                "eligible request has no outcome classification",
            );
        }
    }

    blockers.sort_by(|left, right| {
        (format!("{:?}", left.code), &left.request_ref, &left.detail).cmp(&(
            format!("{:?}", right.code),
            &right.request_ref,
            &right.detail,
        ))
    });

    let screened_count = match u64::try_from(manifest.screened_request_refs.len()) {
        Ok(value) => value,
        Err(_) => {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::CountOverflow,
                None,
                "screened cohort count does not fit report integer type",
            );
            0
        }
    };
    let eligible_denominator = match u64::try_from(manifest.eligible_request_refs.len()) {
        Ok(value) => value,
        Err(_) => {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::CountOverflow,
                None,
                "eligible cohort count does not fit report integer type",
            );
            0
        }
    };
    let excluded_count = match u64::try_from(manifest.exclusions.len()) {
        Ok(value) => value,
        Err(_) => {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::CountOverflow,
                None,
                "excluded cohort count does not fit report integer type",
            );
            0
        }
    };

    if !blockers.is_empty() || eligible_denominator == 0 {
        return FieldEvalPreflightReportV1 {
            schema_version: FIELD_EVAL_REPORT_SCHEMA_VERSION_V1,
            report_ref: manifest.report_ref.clone(),
            status: FieldEvalPreflightStatusV1::Blocked,
            screened_count,
            eligible_denominator,
            excluded_count,
            outcome_counts: None,
            evidence_confirmed_lower_bound: None,
            missingness_upper_identification_bound: None,
            outcome_ascertainment_coverage: None,
            blockers,
            claim_ceiling: CLAIM_CEILING.to_owned(),
        };
    }

    let mut counts = FieldEvalOutcomeCountsV1 {
        accepted_within_useful_window: 0,
        accepted_late: 0,
        known_not_accepted: 0,
        known_non_acceptance_reasons: KnownNonAcceptanceReasonCountsV1 {
            no_provider_available: 0,
            provider_declined: 0,
            farmer_declined_quote: 0,
            farmer_cancelled: 0,
            provider_cancelled: 0,
            provider_no_show: 0,
            provider_completion_not_accepted: 0,
        },
        provider_completion_awaiting_decision: 0,
        disputed_or_rework: 0,
        dispute_resolutions: DisputeResolutionCountsV1 {
            accepted_within_useful_window: 0,
            accepted_late: 0,
            resolved_without_in_window_acceptance: 0,
            unresolved: 0,
        },
        outcome_unknown: 0,
        follow_up_missing_or_inaccessible: 0,
        confirmed_in_window_acceptances_including_resolved_rework: 0,
        genuinely_unascertainable_for_upper_bound: 0,
    };
    let mut confirmed_in_window = 0_u64;
    let mut genuinely_unascertainable = 0_u64;

    for record in manifest.outcome_records.iter() {
        match &record.outcome {
            RequestOutcomeClassV1::AcceptedWithinUsefulWindow => {
                counts.accepted_within_useful_window += 1;
                confirmed_in_window += 1;
                counts.confirmed_in_window_acceptances_including_resolved_rework += 1;
            }
            RequestOutcomeClassV1::AcceptedLate => counts.accepted_late += 1,
            RequestOutcomeClassV1::KnownNotAccepted { reason_code } => {
                counts.known_not_accepted += 1;
                match reason_code {
                    KnownNonAcceptanceReasonV1::NoProviderAvailable => {
                        counts.known_non_acceptance_reasons.no_provider_available += 1;
                    }
                    KnownNonAcceptanceReasonV1::ProviderDeclined => {
                        counts.known_non_acceptance_reasons.provider_declined += 1;
                    }
                    KnownNonAcceptanceReasonV1::FarmerDeclinedQuote => {
                        counts.known_non_acceptance_reasons.farmer_declined_quote += 1;
                    }
                    KnownNonAcceptanceReasonV1::FarmerCancelled => {
                        counts.known_non_acceptance_reasons.farmer_cancelled += 1;
                    }
                    KnownNonAcceptanceReasonV1::ProviderCancelled => {
                        counts.known_non_acceptance_reasons.provider_cancelled += 1;
                    }
                    KnownNonAcceptanceReasonV1::ProviderNoShow => {
                        counts.known_non_acceptance_reasons.provider_no_show += 1;
                    }
                    KnownNonAcceptanceReasonV1::ProviderCompletionNotAccepted => {
                        counts
                            .known_non_acceptance_reasons
                            .provider_completion_not_accepted += 1;
                    }
                }
            }
            RequestOutcomeClassV1::ProviderCompletionAwaitingFarmerDecision { .. } => {
                counts.provider_completion_awaiting_decision += 1;
            }
            RequestOutcomeClassV1::DisputedOrRework { resolution } => {
                counts.disputed_or_rework += 1;
                match resolution {
                    DisputeResolutionV1::AcceptedWithinUsefulWindow => {
                        counts.dispute_resolutions.accepted_within_useful_window += 1;
                        confirmed_in_window += 1;
                        counts.confirmed_in_window_acceptances_including_resolved_rework += 1;
                    }
                    DisputeResolutionV1::AcceptedLate => {
                        counts.dispute_resolutions.accepted_late += 1;
                    }
                    DisputeResolutionV1::ResolvedWithoutInWindowAcceptance => {
                        counts
                            .dispute_resolutions
                            .resolved_without_in_window_acceptance += 1;
                    }
                    DisputeResolutionV1::Unresolved => {
                        counts.dispute_resolutions.unresolved += 1;
                    }
                }
            }
            RequestOutcomeClassV1::OutcomeUnknown => {
                counts.outcome_unknown += 1;
            }
            RequestOutcomeClassV1::FollowUpMissingOrInaccessible => {
                counts.follow_up_missing_or_inaccessible += 1;
            }
        }
        if outcome_is_genuinely_unascertainable(&record.outcome) {
            genuinely_unascertainable += 1;
        }
    }
    counts.genuinely_unascertainable_for_upper_bound = genuinely_unascertainable;

    let ascertainable = eligible_denominator.saturating_sub(genuinely_unascertainable);
    let upper_numerator = match confirmed_in_window.checked_add(genuinely_unascertainable) {
        Some(value) if value <= eligible_denominator => value,
        _ => {
            push_blocker(
                &mut blockers,
                FieldEvalBlockerCodeV1::CountOverflow,
                None,
                "identification-bound numerator exceeds or overflows the fixed denominator",
            );
            return FieldEvalPreflightReportV1 {
                schema_version: FIELD_EVAL_REPORT_SCHEMA_VERSION_V1,
                report_ref: manifest.report_ref.clone(),
                status: FieldEvalPreflightStatusV1::Blocked,
                screened_count,
                eligible_denominator,
                excluded_count,
                outcome_counts: None,
                evidence_confirmed_lower_bound: None,
                missingness_upper_identification_bound: None,
                outcome_ascertainment_coverage: None,
                blockers,
                claim_ceiling: CLAIM_CEILING.to_owned(),
            };
        }
    };

    FieldEvalPreflightReportV1 {
        schema_version: FIELD_EVAL_REPORT_SCHEMA_VERSION_V1,
        report_ref: manifest.report_ref.clone(),
        status: FieldEvalPreflightStatusV1::Calculated,
        screened_count,
        eligible_denominator,
        excluded_count,
        outcome_counts: Some(counts),
        evidence_confirmed_lower_bound: Some(ExactRateV1 {
            numerator: confirmed_in_window,
            denominator: eligible_denominator,
            interpretation: "evidence-confirmed in-window acceptance / all eligible requests; identification lower bound, not a confidence interval".to_owned(),
        }),
        missingness_upper_identification_bound: Some(ExactRateV1 {
            numerator: upper_numerator,
            denominator: eligible_denominator,
            interpretation: "in-window acceptance upper identification bound adds only outcomes whose in-window result is genuinely unascertainable; not a confidence interval".to_owned(),
        }),
        outcome_ascertainment_coverage: Some(ExactRateV1 {
            numerator: ascertainable,
            denominator: eligible_denominator,
            interpretation: "requests with ascertainable in-window acceptance outcome / all eligible requests".to_owned(),
        }),
        blockers,
        claim_ceiling: CLAIM_CEILING.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome_record(
        request_ref: &str,
        outcome: RequestOutcomeClassV1,
    ) -> RequestOutcomeRecordV1 {
        let acceptance_at_minute = match &outcome {
            RequestOutcomeClassV1::AcceptedWithinUsefulWindow
            | RequestOutcomeClassV1::DisputedOrRework {
                resolution: DisputeResolutionV1::AcceptedWithinUsefulWindow,
            } => Some(80),
            RequestOutcomeClassV1::AcceptedLate
            | RequestOutcomeClassV1::DisputedOrRework {
                resolution: DisputeResolutionV1::AcceptedLate,
            } => Some(120),
            _ => None,
        };
        let suffix = request_ref.strip_prefix("req-").expect("opaque request ref");
        let acceptance_evidence_ref =
            acceptance_at_minute.map(|_| format!("evt-{suffix}-acceptance"));
        let outcome_evidence_kind = match &outcome {
            RequestOutcomeClassV1::AcceptedWithinUsefulWindow => {
                OutcomeEvidenceKindV1::FarmerDecisionEvent
            }
            RequestOutcomeClassV1::AcceptedLate => OutcomeEvidenceKindV1::FarmerDecisionEvent,
            RequestOutcomeClassV1::KnownNotAccepted { reason_code } => match reason_code {
                KnownNonAcceptanceReasonV1::NoProviderAvailable => {
                    OutcomeEvidenceKindV1::ProviderAvailabilityCheck
                }
                KnownNonAcceptanceReasonV1::ProviderDeclined => {
                    OutcomeEvidenceKindV1::ProviderDeclineEvent
                }
                KnownNonAcceptanceReasonV1::FarmerDeclinedQuote
                | KnownNonAcceptanceReasonV1::FarmerCancelled => {
                    OutcomeEvidenceKindV1::FarmerDecisionEvent
                }
                KnownNonAcceptanceReasonV1::ProviderCancelled => {
                    OutcomeEvidenceKindV1::ProviderCancellationEvent
                }
                KnownNonAcceptanceReasonV1::ProviderNoShow => {
                    OutcomeEvidenceKindV1::ProviderNoShowEvent
                }
                KnownNonAcceptanceReasonV1::ProviderCompletionNotAccepted => {
                    OutcomeEvidenceKindV1::NoAcceptanceAtWindowClose
                }
            },
            RequestOutcomeClassV1::ProviderCompletionAwaitingFarmerDecision { .. } => {
                OutcomeEvidenceKindV1::ProviderCompletionEvent
            }
            RequestOutcomeClassV1::DisputedOrRework { .. } => {
                OutcomeEvidenceKindV1::DisputeOrReworkEvent
            }
            RequestOutcomeClassV1::OutcomeUnknown => OutcomeEvidenceKindV1::ReconciliationAttempt,
            RequestOutcomeClassV1::FollowUpMissingOrInaccessible => {
                OutcomeEvidenceKindV1::FollowUpAttempt
            }
        };
        RequestOutcomeRecordV1 {
            request_ref: request_ref.to_owned(),
            useful_window_evidence_ref: format!("evt-{suffix}-window"),
            outcome_evidence_ref: format!("evt-{suffix}-outcome"),
            outcome_evidence_kind,
            useful_window_deadline_minute: 100,
            acceptance_at_minute,
            acceptance_evidence_ref,
            acceptance_decision_mode: acceptance_at_minute
                .map(|_| AcceptanceDecisionModeV1::FarmerDirect),
            acceptance_authority_ref: None,
            outcome,
        }
    }

    fn valid_manifest() -> FieldStudyManifestV1 {
        let screened_request_refs = (1..=8)
            .map(|number| format!("req-study-{number:03}"))
            .collect::<Vec<_>>();
        let eligible_request_refs = screened_request_refs[..6].to_vec();
        let exclusions = vec![
            StudyExclusionV1 {
                request_ref: "req-study-007".to_owned(),
                reason_code: StudyExclusionReasonV1::OutsideDeclaredScope,
                preregistered_rule_ref: "rule-scope-eligibility".to_owned(),
                exclusion_evidence_ref: "evt-study-007-exclusion".to_owned(),
            },
            StudyExclusionV1 {
                request_ref: "req-study-008".to_owned(),
                reason_code: StudyExclusionReasonV1::IneligibleServiceTask,
                preregistered_rule_ref: "rule-task-eligibility".to_owned(),
                exclusion_evidence_ref: "evt-study-008-exclusion".to_owned(),
            },
        ];
        let outcome_records = vec![
            outcome_record(
                "req-study-001",
                RequestOutcomeClassV1::AcceptedWithinUsefulWindow,
            ),
            outcome_record("req-study-002", RequestOutcomeClassV1::AcceptedLate),
            outcome_record(
                "req-study-003",
                RequestOutcomeClassV1::KnownNotAccepted {
                    reason_code: KnownNonAcceptanceReasonV1::NoProviderAvailable,
                },
            ),
            outcome_record(
                "req-study-004",
                RequestOutcomeClassV1::ProviderCompletionAwaitingFarmerDecision {
                    useful_window_status: UsefulWindowStatusV1::Open,
                },
            ),
            outcome_record(
                "req-study-005",
                RequestOutcomeClassV1::DisputedOrRework {
                    resolution: DisputeResolutionV1::Unresolved,
                },
            ),
            outcome_record("req-study-006", RequestOutcomeClassV1::OutcomeUnknown),
        ];
        FieldStudyManifestV1 {
            schema_version: FIELD_EVAL_REPORT_SCHEMA_VERSION_V1,
            report_ref: "study-sa-irrigation-feasibility-v1".to_owned(),
            protocol_ref: "protocol-agri-service-007-v1".to_owned(),
            analysis_plan_ref: "analysis-sa-irrigation-v1".to_owned(),
            preregistered_exclusion_rule_refs: vec![
                "rule-scope-eligibility".to_owned(),
                "rule-task-eligibility".to_owned(),
            ],
            screened_request_refs,
            eligible_request_refs,
            exclusions,
            outcome_records,
        }
    }

    #[test]
    fn valid_fixture_yields_exact_bounds_without_floating_point() {
        let report = preflight_field_study_v1(&valid_manifest());
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Calculated);
        assert!(report.blockers.is_empty());
        assert_eq!(report.eligible_denominator, 6);
        let counts = report
            .outcome_counts
            .as_ref()
            .expect("typed outcome counts");
        assert_eq!(counts.known_non_acceptance_reasons.no_provider_available, 1);
        assert_eq!(counts.known_non_acceptance_reasons.provider_no_show, 0);
        assert_eq!(counts.dispute_resolutions.unresolved, 1);
        assert_eq!(
            counts.confirmed_in_window_acceptances_including_resolved_rework,
            1
        );
        assert_eq!(counts.genuinely_unascertainable_for_upper_bound, 3);
        assert_eq!(
            report
                .evidence_confirmed_lower_bound
                .as_ref()
                .map(|rate| (rate.numerator, rate.denominator)),
            Some((1, 6))
        );
        assert_eq!(
            report
                .missingness_upper_identification_bound
                .as_ref()
                .map(|rate| (rate.numerator, rate.denominator)),
            Some((4, 6))
        );
        assert_eq!(
            report
                .outcome_ascertainment_coverage
                .as_ref()
                .map(|rate| (rate.numerator, rate.denominator)),
            Some((3, 6))
        );
    }

    #[test]
    fn known_negative_or_late_outcomes_do_not_inflate_upper_bound() {
        let mut manifest = valid_manifest();
        manifest.outcome_records.retain(|record| {
            !matches!(
                &record.outcome,
                RequestOutcomeClassV1::ProviderCompletionAwaitingFarmerDecision { .. }
                    | RequestOutcomeClassV1::DisputedOrRework {
                        resolution: DisputeResolutionV1::Unresolved
                    }
                    | RequestOutcomeClassV1::OutcomeUnknown
            )
        });
        manifest.outcome_records.extend([
            outcome_record(
                "req-study-004",
                RequestOutcomeClassV1::KnownNotAccepted {
                    reason_code: KnownNonAcceptanceReasonV1::ProviderCancelled,
                },
            ),
            outcome_record(
                "req-study-005",
                RequestOutcomeClassV1::DisputedOrRework {
                    resolution: DisputeResolutionV1::ResolvedWithoutInWindowAcceptance,
                },
            ),
            outcome_record("req-study-006", RequestOutcomeClassV1::AcceptedLate),
        ]);
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Calculated);
        assert_eq!(
            report
                .missingness_upper_identification_bound
                .as_ref()
                .map(|rate| (rate.numerator, rate.denominator)),
            Some((1, 6))
        );
    }

    #[test]
    fn eligible_request_missing_from_outcomes_blocks_all_rates() {
        let mut manifest = valid_manifest();
        manifest.outcome_records.pop();
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.evidence_confirmed_lower_bound.is_none());
        assert!(report.missingness_upper_identification_bound.is_none());
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| { blocker.code == FieldEvalBlockerCodeV1::MissingOutcomeRecord })
        );
    }

    #[test]
    fn dropped_ineligible_candidate_blocks_report() {
        let mut manifest = valid_manifest();
        manifest.exclusions.pop();
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| { blocker.code == FieldEvalBlockerCodeV1::MissingExclusion })
        );
        assert!(report.evidence_confirmed_lower_bound.is_none());
    }

    #[test]
    fn duplicate_and_unscreened_references_block_report() {
        let mut manifest = valid_manifest();
        manifest
            .screened_request_refs
            .push("req-study-001".to_owned());
        manifest
            .eligible_request_refs
            .push("req-unscreened-01".to_owned());
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(
            report.blockers.iter().any(|blocker| {
                blocker.code == FieldEvalBlockerCodeV1::DuplicateScreenedRequest
            })
        );
        assert!(
            report.blockers.iter().any(|blocker| {
                blocker.code == FieldEvalBlockerCodeV1::EligibleRequestNotScreened
            })
        );
    }

    #[test]
    fn duplicate_outcome_is_not_silently_double_counted() {
        let mut manifest = valid_manifest();
        manifest
            .outcome_records
            .push(manifest.outcome_records[0].clone());
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| { blocker.code == FieldEvalBlockerCodeV1::DuplicateOutcomeRecord })
        );
    }

    #[test]
    fn no_provider_label_cannot_use_a_farmer_acceptance_evidence_kind() {
        let mut manifest = valid_manifest();
        manifest.outcome_records[2].outcome_evidence_kind =
            OutcomeEvidenceKindV1::FarmerDecisionEvent;
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.evidence_confirmed_lower_bound.is_none());
        assert!(report.blockers.iter().any(|blocker| {
            blocker.code == FieldEvalBlockerCodeV1::OutcomeEvidenceKindMismatch
        }));
    }

    #[test]
    fn claimed_in_window_acceptance_must_match_the_timestamp() {
        let mut manifest = valid_manifest();
        manifest.outcome_records[0].acceptance_at_minute = Some(120);
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.evidence_confirmed_lower_bound.is_none());
        assert!(report.blockers.iter().any(|blocker| {
            blocker.code == FieldEvalBlockerCodeV1::AcceptanceTimeContradictsWindow
        }));
    }

    #[test]
    fn accepted_outcome_without_event_reference_is_blocked() {
        let mut manifest = valid_manifest();
        manifest.outcome_records[0].acceptance_evidence_ref = None;
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.evidence_confirmed_lower_bound.is_none());
        assert!(report.blockers.iter().any(|blocker| {
            blocker.code == FieldEvalBlockerCodeV1::AcceptanceEvidenceIncomplete
        }));
    }

    #[test]
    fn intermediary_recorded_acceptance_requires_mandate_reference() {
        let mut manifest = valid_manifest();
        manifest.outcome_records[0].acceptance_decision_mode =
            Some(AcceptanceDecisionModeV1::AuthorizedIntermediaryRecorded);
        manifest.outcome_records[0].acceptance_authority_ref = None;
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.blockers.iter().any(|blocker| {
            blocker.code == FieldEvalBlockerCodeV1::IntermediaryAcceptanceAuthorityMissing
        }));
    }

    #[test]
    fn exclusion_without_source_event_reference_is_blocked() {
        let mut manifest = valid_manifest();
        manifest.exclusions[0].exclusion_evidence_ref = "missing".to_owned();
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.evidence_confirmed_lower_bound.is_none());
        assert!(report.blockers.iter().any(|blocker| {
            blocker.code == FieldEvalBlockerCodeV1::InvalidExclusionEvidenceReference
        }));
    }

    #[test]
    fn outcome_without_window_event_reference_is_blocked() {
        let mut manifest = valid_manifest();
        manifest.outcome_records[0].useful_window_evidence_ref = "missing".to_owned();
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.evidence_confirmed_lower_bound.is_none());
        assert!(report.blockers.iter().any(|blocker| {
            blocker.code == FieldEvalBlockerCodeV1::InvalidUsefulWindowEvidenceReference
        }));
    }

    #[test]
    fn exclusion_must_reference_a_listed_preregistered_rule() {
        let mut manifest = valid_manifest();
        manifest
            .preregistered_exclusion_rule_refs
            .retain(|rule_ref| rule_ref != "rule-scope-eligibility");
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.blockers.iter().any(|blocker| {
            blocker.code == FieldEvalBlockerCodeV1::ExclusionRuleNotPreregistered
        }));
        assert!(report.evidence_confirmed_lower_bound.is_none());
    }

    #[test]
    fn duplicate_preregistered_rules_block_report() {
        let mut manifest = valid_manifest();
        manifest
            .preregistered_exclusion_rule_refs
            .push("rule-scope-eligibility".to_owned());
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(
            report.blockers.iter().any(|blocker| {
                blocker.code == FieldEvalBlockerCodeV1::DuplicatePreregisteredRule
            })
        );
    }

    #[test]
    fn closed_window_cannot_remain_in_pending_farmer_decision_class() {
        let mut manifest = valid_manifest();
        let record = manifest
            .outcome_records
            .iter_mut()
            .find(|record| record.request_ref == "req-study-004")
            .expect("pending farmer decision record");
        record.outcome = RequestOutcomeClassV1::ProviderCompletionAwaitingFarmerDecision {
            useful_window_status: UsefulWindowStatusV1::Closed,
        };
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.evidence_confirmed_lower_bound.is_none());
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| { blocker.code == FieldEvalBlockerCodeV1::InvalidOutcomeShape })
        );
    }

    #[test]
    fn rework_accepted_inside_window_counts_as_confirmed_acceptance() {
        let mut manifest = valid_manifest();
        let record = manifest
            .outcome_records
            .iter_mut()
            .find(|record| record.request_ref == "req-study-005")
            .expect("dispute record");
        *record = outcome_record(
            "req-study-005",
            RequestOutcomeClassV1::DisputedOrRework {
                resolution: DisputeResolutionV1::AcceptedWithinUsefulWindow,
            },
        );
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Calculated);
        assert_eq!(
            report
                .evidence_confirmed_lower_bound
                .as_ref()
                .map(|rate| (rate.numerator, rate.denominator)),
            Some((2, 6))
        );
        assert_eq!(
            report
                .missingness_upper_identification_bound
                .as_ref()
                .map(|rate| (rate.numerator, rate.denominator)),
            Some((4, 6))
        );
        assert_eq!(
            report
                .outcome_counts
                .as_ref()
                .map(|counts| counts.confirmed_in_window_acceptances_including_resolved_rework),
            Some(2)
        );
    }

    #[test]
    fn exclusion_must_reference_a_preregistered_rule() {
        let mut manifest = valid_manifest();
        manifest.exclusions[0].preregistered_rule_ref = "none".to_owned();
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.blockers.iter().any(|blocker| {
            blocker.code == FieldEvalBlockerCodeV1::ExclusionRuleReferenceMissing
        }));
    }

    #[test]
    fn empty_eligible_cohort_is_not_reported_as_zero_percent() {
        let mut manifest = valid_manifest();
        manifest.eligible_request_refs.clear();
        manifest.outcome_records.clear();
        manifest.preregistered_exclusion_rule_refs =
            vec!["rule-outside-scope".to_owned()];
        manifest.exclusions = manifest
            .screened_request_refs
            .iter()
            .map(|request_ref| StudyExclusionV1 {
                request_ref: request_ref.clone(),
                reason_code: StudyExclusionReasonV1::OutsideDeclaredScope,
                preregistered_rule_ref: "rule-outside-scope".to_owned(),
                exclusion_evidence_ref: format!(
                    "evt-{}-exclusion",
                    request_ref.strip_prefix("req-").expect("request reference")
                ),
            })
            .collect();
        let report = preflight_field_study_v1(&manifest);
        assert_eq!(report.status, FieldEvalPreflightStatusV1::Blocked);
        assert!(report.evidence_confirmed_lower_bound.is_none());
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| { blocker.code == FieldEvalBlockerCodeV1::EmptyEligibleCohort })
        );
    }
}
