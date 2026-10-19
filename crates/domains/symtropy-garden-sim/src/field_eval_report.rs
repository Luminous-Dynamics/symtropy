// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Reconciles the primary outcome denominator for AGRI-ROBOT-SERVICE-007.
//!
//! This measurement-layer checker does not re-run the request/quote/booking
//! evaluator. It checks only a study-level partition, reason reconciliation,
//! and conservative outcome-identification bounds. It cannot authenticate
//! source records or provide causal/statistical inference.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const FIELD_EVAL_REPORT_SCHEMA_VERSION_V1: u16 = 1;
pub const FIELD_EVAL_REPORT_CLAIM_CEILING_V1: &str = "Study-report arithmetic and declared cohort partition only. Does not authenticate source evidence, validate individual service lifecycles, estimate sampling uncertainty, establish causal effects, or prove field impact.";

fn serialize_option_u64<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(value) => serializer.serialize_some(&value.to_string()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_option_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|value| value.parse::<u64>().map_err(serde::de::Error::custom))
        .transpose()
}

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEvalReportInputV1 {
    pub schema_version: u16,
    pub study_ref: String,
    pub protocol_ref: String,
    pub analysis_plan_ref: String,
    pub source_ledger_ref: String,
    pub task_code: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub useful_window_rule_ref: String,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub screened_request_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub eligible_request_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub protocol_excluded_count: u64,
    pub outcomes: FieldEvalOutcomePartitionV1,
    pub nonacceptance_reasons: FieldEvalNonacceptanceReasonsV1,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEvalOutcomePartitionV1 {
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub accepted_within_window_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub accepted_after_window_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub known_not_accepted_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub disputed_or_rework_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub completed_pending_decision_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub outcome_unknown_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub follow_up_missing_count: u64,
}

impl FieldEvalOutcomePartitionV1 {
    fn checked_total(&self) -> Option<u64> {
        [
            self.accepted_within_window_count,
            self.accepted_after_window_count,
            self.known_not_accepted_count,
            self.disputed_or_rework_count,
            self.completed_pending_decision_count,
            self.outcome_unknown_count,
            self.follow_up_missing_count,
        ]
        .into_iter()
        .try_fold(0_u64, u64::checked_add)
    }

    fn uncertain_count(&self) -> Option<u64> {
        self.completed_pending_decision_count
            .checked_add(self.outcome_unknown_count)?
            .checked_add(self.follow_up_missing_count)
    }

    fn ascertained_count(&self, eligible: u64) -> Option<u64> {
        eligible.checked_sub(self.uncertain_count()?)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEvalNonacceptanceReasonsV1 {
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub no_provider_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub provider_declined_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub farmer_declined_quote_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub farmer_cancelled_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub provider_cancelled_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub provider_no_show_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub abandoned_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub other_typed_reason_count: u64,
}

impl FieldEvalNonacceptanceReasonsV1 {
    fn checked_total(&self) -> Option<u64> {
        [
            self.no_provider_count,
            self.provider_declined_count,
            self.farmer_declined_quote_count,
            self.farmer_cancelled_count,
            self.provider_cancelled_count,
            self.provider_no_show_count,
            self.abandoned_count,
            self.other_typed_reason_count,
        ]
        .into_iter()
        .try_fold(0_u64, u64::checked_add)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldEvalReportDispositionV1 {
    Calculated,
    NotAssessable,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldEvalReportBlockerCodeV1 {
    UnsupportedSchema,
    MissingOrUnsafeReference,
    CountOverflow,
    ScreenedCountMismatch,
    OutcomePartitionMismatch,
    NonacceptanceReasonMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldEvalReportBlockerV1 {
    pub code: FieldEvalReportBlockerCodeV1,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldEvalRateBoundsV1 {
    /// Confirmed within-window acceptances only.
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub lower_numerator: u64,
    /// Confirmed in-window acceptances plus pending decisions and unascertained outcomes.
    /// This is an identification bound, not a statistical confidence interval.
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub upper_numerator: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub denominator: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldEvalReportAssessmentV1 {
    pub study_ref: String,
    pub disposition: FieldEvalReportDispositionV1,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub eligible_request_count: u64,
    #[serde(
        serialize_with = "serialize_option_u64",
        deserialize_with = "deserialize_option_u64"
    )]
    pub classified_request_count: Option<u64>,
    pub evidence_confirmed_attainment: Option<FieldEvalRateBoundsV1>,
    pub blockers: Vec<FieldEvalReportBlockerV1>,
    pub claim_ceiling: String,
}

fn is_safe_opaque_ref(value: &str) -> bool {
    (3..=96).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value.as_bytes()[0].is_ascii_lowercase()
}

fn push_blocker(
    blockers: &mut Vec<FieldEvalReportBlockerV1>,
    code: FieldEvalReportBlockerCodeV1,
    detail: impl Into<String>,
) {
    let blocker = FieldEvalReportBlockerV1 {
        code,
        detail: detail.into(),
    };
    if !blockers.contains(&blocker) {
        blockers.push(blocker);
    }
}

/// Validate denominator closure and calculate a conservative interval for
/// evidence-confirmed attainment. Unknown/missing outcomes are not assumed to
/// be success or failure; known-late, known-not-accepted, and disputed outcomes
/// do not enter the upper bound. All arithmetic remains integer-exact.
pub fn assess_field_eval_report_v1(input: &FieldEvalReportInputV1) -> FieldEvalReportAssessmentV1 {
    let mut blockers = Vec::new();

    if input.schema_version != FIELD_EVAL_REPORT_SCHEMA_VERSION_V1 {
        push_blocker(
            &mut blockers,
            FieldEvalReportBlockerCodeV1::UnsupportedSchema,
            "unsupported field-evaluation report schema version",
        );
    }

    for (field, value) in [
        ("study_ref", input.study_ref.as_str()),
        ("protocol_ref", input.protocol_ref.as_str()),
        ("analysis_plan_ref", input.analysis_plan_ref.as_str()),
        ("source_ledger_ref", input.source_ledger_ref.as_str()),
        ("task_code", input.task_code.as_str()),
        ("scope_ref", input.scope_ref.as_str()),
        ("period_ref", input.period_ref.as_str()),
        (
            "useful_window_rule_ref",
            input.useful_window_rule_ref.as_str(),
        ),
    ] {
        if !is_safe_opaque_ref(value) {
            push_blocker(
                &mut blockers,
                FieldEvalReportBlockerCodeV1::MissingOrUnsafeReference,
                format!("{field} must be a nonempty opaque lowercase identifier"),
            );
        }
    }

    let expected_screened = input
        .eligible_request_count
        .checked_add(input.protocol_excluded_count);
    let outcome_total = input.outcomes.checked_total();
    let reason_total = input.nonacceptance_reasons.checked_total();
    let uncertain_total = input.outcomes.uncertain_count();
    let classified_total = input
        .outcomes
        .ascertained_count(input.eligible_request_count);

    if expected_screened.is_none()
        || outcome_total.is_none()
        || reason_total.is_none()
        || uncertain_total.is_none()
    {
        push_blocker(
            &mut blockers,
            FieldEvalReportBlockerCodeV1::CountOverflow,
            "one or more count sums overflowed u64",
        );
    }

    if expected_screened != Some(input.screened_request_count) {
        push_blocker(
            &mut blockers,
            FieldEvalReportBlockerCodeV1::ScreenedCountMismatch,
            "screened requests must equal eligible requests plus protocol exclusions",
        );
    }

    if outcome_total != Some(input.eligible_request_count) {
        push_blocker(
            &mut blockers,
            FieldEvalReportBlockerCodeV1::OutcomePartitionMismatch,
            "mutually exclusive outcome classes must account for every eligible request exactly once",
        );
    }

    if reason_total != Some(input.outcomes.known_not_accepted_count) {
        push_blocker(
            &mut blockers,
            FieldEvalReportBlockerCodeV1::NonacceptanceReasonMismatch,
            "typed nonacceptance reasons must reconcile exactly to known_not_accepted_count",
        );
    }

    let disposition = if !blockers.is_empty() {
        FieldEvalReportDispositionV1::Blocked
    } else if input.eligible_request_count == 0 {
        FieldEvalReportDispositionV1::NotAssessable
    } else {
        FieldEvalReportDispositionV1::Calculated
    };

    let evidence_confirmed_attainment = if disposition == FieldEvalReportDispositionV1::Calculated {
        let outcomes = &input.outcomes;
        let upper_uncertain = uncertain_total.expect("validated count sum");
        outcomes
            .accepted_within_window_count
            .checked_add(upper_uncertain)
            .map(|upper| FieldEvalRateBoundsV1 {
                lower_numerator: outcomes.accepted_within_window_count,
                upper_numerator: upper,
                denominator: input.eligible_request_count,
            })
    } else {
        None
    };

    let classified_request_count = (disposition != FieldEvalReportDispositionV1::Blocked)
        .then_some(classified_total.unwrap_or(0));
    FieldEvalReportAssessmentV1 {
        study_ref: input.study_ref.clone(),
        disposition,
        eligible_request_count: input.eligible_request_count,
        classified_request_count,
        evidence_confirmed_attainment,
        blockers,
        claim_ceiling: FIELD_EVAL_REPORT_CLAIM_CEILING_V1.to_owned(),
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
