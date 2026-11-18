// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Protocol-bound food-quality evidence assessment.
//!
//! Instrumental measurements, blinded consumer responses and external safety
//! evidence remain independent lanes. This module computes descriptive results
//! for a declared protocol only. It never issues a safety certificate or permit.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const FOOD_QUALITY_PROTOCOL_SCHEMA_VERSION: u32 = 1;
pub const FOOD_QUALITY_REQUEST_SCHEMA_VERSION: u32 = 1;
pub const FOOD_QUALITY_REPORT_SCHEMA_VERSION: u32 = 1;
pub const FOOD_QUALITY_ASSESSOR_VERSION_REF: &str = "symtropy-food-quality-assessor-v1";
pub const FOOD_QUALITY_CLAIM_CEILING: &str = "Descriptive evidence assessment for the exact declared samples and protocol only; not food-safety or allergen qualification, nutrition/medical suitability, universal taste, regulatory conformity, a serving permit, or product superiority.";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityDispositionV1 {
    Complete,
    NotQualified,
    Indeterminate,
    Blocked,
    Conflicted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityIssueCodeV1 {
    UnsupportedSchemaVersion,
    MissingRequiredValue,
    InvalidClaimCeiling,
    InvalidProtocolDigest,
    InvalidProtocol,
    DuplicateSampleRef,
    DuplicateMetricRequirement,
    InvalidMetricRequirement,
    InvalidScale,
    InvalidThreshold,
    StudyNotBlinded,
    StudyNotRandomized,
    DuplicateObservation,
    UnknownSample,
    UnknownMetric,
    ObservationMethodMismatch,
    MissingCalibration,
    InvalidMeasurementDigest,
    InvalidObservationTime,
    ObservationRunMismatch,
    MeasurementOutsideQualityLimit,
    MeasurementUncertaintyOverlapsLimit,
    MissingRequiredMeasurement,
    DuplicateResponse,
    DuplicateRespondentSample,
    DuplicatePresentationOrder,
    InvalidResponse,
    MissingResponseConsent,
    InvalidResponseDigest,
    InsufficientConsumerResponses,
    ConsumerScoreBelowThreshold,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QualityIssueV1 {
    pub code: QualityIssueCodeV1,
    pub reference: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstrumentalMetricRequirementV1 {
    pub metric_id: String,
    pub unit: String,
    pub basis: String,
    pub method_ref: String,
    pub decimal_places: u8,
    pub required: bool,
    pub minimum_value_scaled: Option<i64>,
    pub maximum_value_scaled: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SensoryProtocolV1 {
    pub method_ref: String,
    pub population_scope_ref: String,
    pub scale_min: i32,
    pub scale_max: i32,
    pub minimum_responses_per_sample: u32,
    /// Protocol-specific minimum mean liking, multiplied by 1,000.
    pub minimum_mean_liking_milli: Option<i64>,
    pub blinded: bool,
    pub blinding_protocol_ref: Option<String>,
    pub randomized: bool,
    pub randomization_protocol_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FoodQualityProtocolV1 {
    pub schema_version: u32,
    pub protocol_id: String,
    pub revision: u32,
    pub dish_ref: String,
    pub sample_refs: Vec<String>,
    pub sensory: SensoryProtocolV1,
    pub instrumental_requirements: Vec<InstrumentalMetricRequirementV1>,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstrumentalQualityObservationV1 {
    pub observation_id: String,
    pub run_ref: String,
    pub sample_ref: String,
    pub metric_id: String,
    pub value_scaled: i64,
    pub decimal_places: u8,
    pub unit: String,
    pub basis: String,
    pub method_ref: String,
    pub sample_position_ref: String,
    pub instrument_ref: String,
    pub calibration_ref: String,
    /// Absolute uncertainty in the same scaled unit as value_scaled.
    pub uncertainty_scaled: u64,
    pub observed_at_epoch_ms: u64,
    pub raw_observation_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerResponseV1 {
    pub response_id: String,
    pub run_ref: String,
    pub sample_ref: String,
    /// Study-scoped, one-time token; never a global or public identity.
    pub respondent_token_ref: String,
    pub response_value: i32,
    pub presentation_order: u32,
    pub consent_receipt_ref: String,
    pub raw_response_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalSafetyGateStateV1 {
    NotProvided,
    Unknown,
    ExternallyQualified,
    ExternallyFailed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalSafetyGateEvidenceV1 {
    pub state: ExternalSafetyGateStateV1,
    pub scope_ref: Option<String>,
    pub competent_authority_ref: Option<String>,
    pub evidence_ref: Option<String>,
    pub valid_until_epoch_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FoodQualityAssessmentRequestV1 {
    pub schema_version: u32,
    pub assessment_id: String,
    pub run_ref: String,
    pub assessed_at_epoch_ms: u64,
    pub protocol_sha256: String,
    pub protocol: FoodQualityProtocolV1,
    pub instrumental_observations: Vec<InstrumentalQualityObservationV1>,
    pub consumer_responses: Vec<ConsumerResponseV1>,
    /// Echoed independently; the evaluator does not authenticate this claim.
    pub external_safety_gate: ExternalSafetyGateEvidenceV1,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponseBucketV1 {
    pub score: i32,
    pub count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SampleSensorySummaryV1 {
    pub sample_ref: String,
    pub response_count: u32,
    pub mean_liking_milli: Option<i64>,
    pub median_liking_milli: Option<i64>,
    pub minimum_response: Option<i32>,
    pub maximum_response: Option<i32>,
    pub response_histogram: Vec<ResponseBucketV1>,
    pub threshold_met: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstrumentalMetricSummaryV1 {
    pub sample_ref: String,
    pub metric_id: String,
    pub observation_count: u32,
    pub minimum_value_scaled: i64,
    pub maximum_value_scaled: i64,
    pub mean_value_scaled: i64,
    pub maximum_uncertainty_scaled: u64,
    pub declared_limits_met: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FoodQualityAssessmentReportV1 {
    pub schema_version: u32,
    pub assessment_id: String,
    pub run_ref: String,
    pub protocol_id: String,
    pub protocol_revision: u32,
    pub protocol_sha256: String,
    pub assessor_version_ref: String,
    pub disposition: QualityDispositionV1,
    pub issues: Vec<QualityIssueV1>,
    pub sensory_summaries: Vec<SampleSensorySummaryV1>,
    pub instrumental_summaries: Vec<InstrumentalMetricSummaryV1>,
    pub external_safety_gate: ExternalSafetyGateEvidenceV1,
    pub raw_evidence_sha256: Vec<String>,
    pub claim_ceiling: String,
}

impl FoodQualityAssessmentReportV1 {
    pub fn has_complete_quality_evidence(&self) -> bool {
        self.disposition == QualityDispositionV1::Complete
            && self.issues.is_empty()
            && !self.sensory_summaries.is_empty()
    }

    /// The food-quality assessor never authorizes serving or appliance action.
    pub fn grants_serving_permit(&self) -> bool {
        false
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn add_issue(
    issues: &mut Vec<QualityIssueV1>,
    code: QualityIssueCodeV1,
    reference: Option<&str>,
    detail: impl Into<String>,
) {
    issues.push(QualityIssueV1 {
        code,
        reference: reference.map(str::to_owned),
        detail: detail.into(),
    });
}

fn protocol_digest(protocol: &FoodQualityProtocolV1) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(protocol)?;
    let digest = Sha256::digest(bytes);
    Ok(digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(""))
}

fn validate_protocol(
    protocol: &FoodQualityProtocolV1,
    issues: &mut Vec<QualityIssueV1>,
) -> BTreeMap<String, InstrumentalMetricRequirementV1> {
    if protocol.schema_version != FOOD_QUALITY_PROTOCOL_SCHEMA_VERSION {
        add_issue(
            issues,
            QualityIssueCodeV1::UnsupportedSchemaVersion,
            Some("protocol.schema_version"),
            "only FoodQualityProtocolV1 schema version 1 is supported",
        );
    }
    if !nonempty(&protocol.protocol_id)
        || protocol.revision == 0
        || !nonempty(&protocol.dish_ref)
        || protocol.sample_refs.is_empty()
    {
        add_issue(
            issues,
            QualityIssueCodeV1::InvalidProtocol,
            Some(&protocol.protocol_id),
            "protocol identity, revision, dish and at least one sample are required",
        );
    }
    if protocol.claim_ceiling != FOOD_QUALITY_CLAIM_CEILING {
        add_issue(
            issues,
            QualityIssueCodeV1::InvalidClaimCeiling,
            Some("protocol.claim_ceiling"),
            "protocol claim ceiling does not match the fixed non-safety contract",
        );
    }

    let mut samples = BTreeSet::new();
    for sample_ref in &protocol.sample_refs {
        if !nonempty(sample_ref) || !samples.insert(sample_ref.as_str()) {
            add_issue(
                issues,
                QualityIssueCodeV1::DuplicateSampleRef,
                Some(sample_ref),
                "sample references must be non-empty and unique",
            );
        }
    }

    let sensory = &protocol.sensory;
    if !nonempty(&sensory.method_ref)
        || !nonempty(&sensory.population_scope_ref)
        || sensory.minimum_responses_per_sample == 0
    {
        add_issue(
            issues,
            QualityIssueCodeV1::InvalidProtocol,
            Some("protocol.sensory"),
            "sensory method, population and positive minimum response count are required",
        );
    }
    if sensory.scale_min >= sensory.scale_max {
        add_issue(
            issues,
            QualityIssueCodeV1::InvalidScale,
            Some("protocol.sensory"),
            "sensory scale minimum must be below maximum",
        );
    }
    if sensory.minimum_mean_liking_milli.is_some_and(|threshold| {
        threshold < i64::from(sensory.scale_min) * 1_000
            || threshold > i64::from(sensory.scale_max) * 1_000
    }) {
        add_issue(
            issues,
            QualityIssueCodeV1::InvalidThreshold,
            Some("protocol.sensory.minimum_mean_liking_milli"),
            "liking threshold must be within the protocol's declared response scale",
        );
    }
    if !sensory.blinded
        || sensory
            .blinding_protocol_ref
            .as_deref()
            .is_none_or(|reference| !nonempty(reference))
    {
        add_issue(
            issues,
            QualityIssueCodeV1::StudyNotBlinded,
            Some("protocol.sensory"),
            "consumer comparisons require a documented blinding protocol",
        );
    }
    if !sensory.randomized
        || sensory
            .randomization_protocol_ref
            .as_deref()
            .is_none_or(|reference| !nonempty(reference))
    {
        add_issue(
            issues,
            QualityIssueCodeV1::StudyNotRandomized,
            Some("protocol.sensory"),
            "consumer comparisons require a documented randomization protocol",
        );
    }

    let mut requirements = BTreeMap::new();
    for requirement in &protocol.instrumental_requirements {
        if !nonempty(&requirement.metric_id)
            || !nonempty(&requirement.unit)
            || !nonempty(&requirement.basis)
            || !nonempty(&requirement.method_ref)
            || requirement.decimal_places > 6
            || requirement
                .minimum_value_scaled
                .zip(requirement.maximum_value_scaled)
                .is_some_and(|(minimum, maximum)| minimum > maximum)
        {
            add_issue(
                issues,
                QualityIssueCodeV1::InvalidMetricRequirement,
                Some(&requirement.metric_id),
                "instrumental requirement has missing metadata, invalid scale or reversed limits",
            );
        }
        if requirements
            .insert(requirement.metric_id.clone(), requirement.clone())
            .is_some()
        {
            add_issue(
                issues,
                QualityIssueCodeV1::DuplicateMetricRequirement,
                Some(&requirement.metric_id),
                "instrumental metric requirements must be unique",
            );
        }
    }
    requirements
}

fn summarize_sensory(
    request: &FoodQualityAssessmentRequestV1,
    issues: &mut Vec<QualityIssueV1>,
    raw_digests: &mut BTreeSet<String>,
) -> Vec<SampleSensorySummaryV1> {
    let protocol = &request.protocol;
    let valid_samples: BTreeSet<&str> =
        protocol.sample_refs.iter().map(String::as_str).collect();
    let mut by_sample: BTreeMap<String, Vec<&ConsumerResponseV1>> = BTreeMap::new();
    let mut response_ids = BTreeSet::new();
    let mut respondent_sample_pairs = BTreeSet::new();
    let mut respondent_orders = BTreeSet::new();

    for response in &request.consumer_responses {
        if !response_ids.insert(response.response_id.as_str()) {
            add_issue(
                issues,
                QualityIssueCodeV1::DuplicateResponse,
                Some(&response.response_id),
                "consumer response IDs must be unique",
            );
        }
        if !valid_samples.contains(response.sample_ref.as_str()) {
            add_issue(
                issues,
                QualityIssueCodeV1::UnknownSample,
                Some(&response.response_id),
                "consumer response references a sample absent from the protocol",
            );
            continue;
        }
        if !nonempty(&response.response_id)
            || !nonempty(&response.run_ref)
            || !nonempty(&response.respondent_token_ref)
            || response.presentation_order == 0
            || response.run_ref != request.run_ref
            || response.response_value < protocol.sensory.scale_min
            || response.response_value > protocol.sensory.scale_max
        {
            add_issue(
                issues,
                QualityIssueCodeV1::InvalidResponse,
                Some(&response.response_id),
                "response fields, run binding, presentation order or scale value are invalid",
            );
        }
        if response.consent_receipt_ref.trim().is_empty() {
            add_issue(
                issues,
                QualityIssueCodeV1::MissingResponseConsent,
                Some(&response.response_id),
                "each consumer response requires a consent receipt reference",
            );
        }
        if !is_sha256(&response.raw_response_sha256) {
            add_issue(
                issues,
                QualityIssueCodeV1::InvalidResponseDigest,
                Some(&response.response_id),
                "raw response digest must be 64 lowercase hexadecimal characters",
            );
        } else {
            raw_digests.insert(response.raw_response_sha256.clone());
        }
        if !respondent_sample_pairs.insert((
            response.respondent_token_ref.as_str(),
            response.sample_ref.as_str(),
        )) {
            add_issue(
                issues,
                QualityIssueCodeV1::DuplicateRespondentSample,
                Some(&response.sample_ref),
                "a study-scoped respondent token may rate a sample only once",
            );
        }
        if !respondent_orders.insert((
            response.respondent_token_ref.as_str(),
            response.presentation_order,
        )) {
            add_issue(
                issues,
                QualityIssueCodeV1::DuplicatePresentationOrder,
                Some(&response.respondent_token_ref),
                "presentation order must be unique within each respondent session",
            );
        }
        by_sample
            .entry(response.sample_ref.clone())
            .or_default()
            .push(response);
    }

    let mut summaries = Vec::new();
    for sample_ref in &protocol.sample_refs {
        let mut sample_responses = by_sample.remove(sample_ref).unwrap_or_default();
        sample_responses.sort_by(|left, right| {
            (left.response_value, left.response_id.as_str())
                .cmp(&(right.response_value, right.response_id.as_str()))
        });
        let count = u32::try_from(sample_responses.len()).unwrap_or(u32::MAX);
        let minimum = sample_responses
            .iter()
            .map(|response| response.response_value)
            .min();
        let maximum = sample_responses
            .iter()
            .map(|response| response.response_value)
            .max();
        let mean_liking_milli = if sample_responses.is_empty() {
            None
        } else {
            let sum: i128 = sample_responses
                .iter()
                .map(|response| i128::from(response.response_value))
                .sum();
            let divisor = i128::from(count);
            i64::try_from((sum * 1_000 + divisor / 2) / divisor).ok()
        };
        let median_liking_milli = if sample_responses.is_empty() {
            None
        } else {
            let midpoint = sample_responses.len() / 2;
            let doubled_median = if sample_responses.len() % 2 == 0 {
                i64::from(sample_responses[midpoint - 1].response_value)
                    + i64::from(sample_responses[midpoint].response_value)
            } else {
                i64::from(sample_responses[midpoint].response_value) * 2
            };
            Some(doubled_median * 500)
        };
        let mut histogram = BTreeMap::<i32, u32>::new();
        for response in &sample_responses {
            *histogram.entry(response.response_value).or_default() += 1;
        }
        let threshold_met = protocol
            .sensory
            .minimum_mean_liking_milli
            .zip(mean_liking_milli)
            .map(|(threshold, mean)| mean >= threshold);
        if count < protocol.sensory.minimum_responses_per_sample {
            add_issue(
                issues,
                QualityIssueCodeV1::InsufficientConsumerResponses,
                Some(sample_ref),
                format!(
                    "sample has {count} consumer responses; protocol requires {}",
                    protocol.sensory.minimum_responses_per_sample
                ),
            );
        }
        if threshold_met == Some(false) {
            add_issue(
                issues,
                QualityIssueCodeV1::ConsumerScoreBelowThreshold,
                Some(sample_ref),
                "consumer mean liking is below the protocol-specific threshold",
            );
        }
        summaries.push(SampleSensorySummaryV1 {
            sample_ref: sample_ref.clone(),
            response_count: count,
            mean_liking_milli,
            median_liking_milli,
            minimum_response: minimum,
            maximum_response: maximum,
            response_histogram: histogram
                .into_iter()
                .map(|(score, count)| ResponseBucketV1 { score, count })
                .collect(),
            threshold_met,
        });
    }
    summaries
}

fn summarize_instrumental(
    request: &FoodQualityAssessmentRequestV1,
    requirements: &BTreeMap<String, InstrumentalMetricRequirementV1>,
    issues: &mut Vec<QualityIssueV1>,
    raw_digests: &mut BTreeSet<String>,
) -> Vec<InstrumentalMetricSummaryV1> {
    let protocol = &request.protocol;
    let valid_samples: BTreeSet<&str> =
        protocol.sample_refs.iter().map(String::as_str).collect();
    let mut observed = BTreeMap::<(String, String), Vec<&InstrumentalQualityObservationV1>>::new();
    let mut seen = BTreeSet::new();

    for observation in &request.instrumental_observations {
        if !nonempty(&observation.observation_id)
            || !nonempty(&observation.run_ref)
            || !nonempty(&observation.sample_ref)
            || !nonempty(&observation.metric_id)
        {
            add_issue(
                issues,
                QualityIssueCodeV1::MissingRequiredValue,
                Some(&observation.observation_id),
                "observation identity, run, sample and metric references are required",
            );
            continue;
        }
        if observation.run_ref != request.run_ref {
            add_issue(
                issues,
                QualityIssueCodeV1::ObservationRunMismatch,
                Some(&observation.observation_id),
                "instrumental observation run must match the assessment run",
            );
        }
        if !valid_samples.contains(observation.sample_ref.as_str()) {
            add_issue(
                issues,
                QualityIssueCodeV1::UnknownSample,
                Some(&observation.observation_id),
                "instrumental observation references a sample absent from the protocol",
            );
            continue;
        }
        let Some(requirement) = requirements.get(&observation.metric_id) else {
            add_issue(
                issues,
                QualityIssueCodeV1::UnknownMetric,
                Some(&observation.observation_id),
                "instrumental metric is not declared in the protocol",
            );
            continue;
        };
        let key = (observation.sample_ref.clone(), observation.metric_id.clone());
        if !seen.insert(key.clone()) {
            add_issue(
                issues,
                QualityIssueCodeV1::DuplicateObservation,
                Some(&observation.sample_ref),
                "V1 permits one summary observation per sample and metric; replicated designs need an explicit later schema",
            );
        }
        if observation.unit != requirement.unit
            || observation.basis != requirement.basis
            || observation.method_ref != requirement.method_ref
            || observation.decimal_places != requirement.decimal_places
        {
            add_issue(
                issues,
                QualityIssueCodeV1::ObservationMethodMismatch,
                Some(&observation.observation_id),
                "observation unit, basis, method and scale must exactly match the protocol",
            );
        }
        if !nonempty(&observation.sample_position_ref) || !nonempty(&observation.instrument_ref) {
            add_issue(
                issues,
                QualityIssueCodeV1::MissingRequiredValue,
                Some(&observation.observation_id),
                "sample position and instrument references are required",
            );
        }
        if !nonempty(&observation.calibration_ref) {
            add_issue(
                issues,
                QualityIssueCodeV1::MissingCalibration,
                Some(&observation.observation_id),
                "measured instrumental evidence requires a calibration reference",
            );
        }
        if observation.observed_at_epoch_ms == 0
            || observation.observed_at_epoch_ms > request.assessed_at_epoch_ms
        {
            add_issue(
                issues,
                QualityIssueCodeV1::InvalidObservationTime,
                Some(&observation.observation_id),
                "observation time must be positive and no later than assessment time",
            );
        }
        if !is_sha256(&observation.raw_observation_sha256) {
            add_issue(
                issues,
                QualityIssueCodeV1::InvalidMeasurementDigest,
                Some(&observation.observation_id),
                "raw observation digest must be 64 lowercase hexadecimal characters",
            );
        } else {
            raw_digests.insert(observation.raw_observation_sha256.clone());
        }
        let value = i128::from(observation.value_scaled);
        let uncertainty = i128::from(observation.uncertainty_scaled);
        let lower = value - uncertainty;
        let upper = value + uncertainty;
        if let Some(minimum) = requirement.minimum_value_scaled {
            let limit = i128::from(minimum);
            if upper < limit {
                add_issue(
                    issues,
                    QualityIssueCodeV1::MeasurementOutsideQualityLimit,
                    Some(&observation.observation_id),
                    "the full measurement interval is below the declared minimum",
                );
            } else if lower < limit {
                add_issue(
                    issues,
                    QualityIssueCodeV1::MeasurementUncertaintyOverlapsLimit,
                    Some(&observation.observation_id),
                    "measurement uncertainty overlaps the declared minimum",
                );
            }
        }
        if let Some(maximum) = requirement.maximum_value_scaled {
            let limit = i128::from(maximum);
            if lower > limit {
                add_issue(
                    issues,
                    QualityIssueCodeV1::MeasurementOutsideQualityLimit,
                    Some(&observation.observation_id),
                    "the full measurement interval is above the declared maximum",
                );
            } else if upper > limit {
                add_issue(
                    issues,
                    QualityIssueCodeV1::MeasurementUncertaintyOverlapsLimit,
                    Some(&observation.observation_id),
                    "measurement uncertainty overlaps the declared maximum",
                );
            }
        }
        observed.entry(key).or_default().push(observation);
    }

    let mut summaries = Vec::new();
    for sample_ref in &protocol.sample_refs {
        for (metric_id, requirement) in requirements {
            let key = (sample_ref.clone(), metric_id.clone());
            let values = observed.remove(&key).unwrap_or_default();
            if requirement.required && values.is_empty() {
                add_issue(
                    issues,
                    QualityIssueCodeV1::MissingRequiredMeasurement,
                    Some(sample_ref),
                    format!("required instrumental metric {metric_id} has no observation"),
                );
                continue;
            }
            if values.is_empty() {
                continue;
            }
            let divisor = i128::try_from(values.len()).unwrap_or(i128::MAX);
            let sum: i128 = values
                .iter()
                .map(|observation| i128::from(observation.value_scaled))
                .sum();
            let mean = (sum + divisor / 2) / divisor;
            let minimum = values
                .iter()
                .map(|observation| observation.value_scaled)
                .min()
                .unwrap_or_default();
            let maximum = values
                .iter()
                .map(|observation| observation.value_scaled)
                .max()
                .unwrap_or_default();
            let maximum_uncertainty = values
                .iter()
                .map(|observation| observation.uncertainty_scaled)
                .max()
                .unwrap_or_default();
            let limits_met = values.iter().all(|observation| {
                let value = i128::from(observation.value_scaled);
                let uncertainty = i128::from(observation.uncertainty_scaled);
                requirement
                    .minimum_value_scaled
                    .is_none_or(|limit| value - uncertainty >= i128::from(limit))
                    && requirement
                        .maximum_value_scaled
                        .is_none_or(|limit| value + uncertainty <= i128::from(limit))
            });
            summaries.push(InstrumentalMetricSummaryV1 {
                sample_ref: sample_ref.clone(),
                metric_id: metric_id.clone(),
                observation_count: u32::try_from(values.len()).unwrap_or(u32::MAX),
                minimum_value_scaled: minimum,
                maximum_value_scaled: maximum,
                mean_value_scaled: i64::try_from(mean).unwrap_or_else(|_| {
                    if mean.is_negative() { i64::MIN } else { i64::MAX }
                }),
                maximum_uncertainty_scaled: maximum_uncertainty,
                declared_limits_met: Some(limits_met),
            });
        }
    }
    summaries
}

/// Assess descriptive evidence for one exact run and protocol revision.
/// Complete means only that the supplied quality evidence meets the declared
/// quality protocol. The external safety lane is echoed, never authenticated.
pub fn assess_food_quality(
    request: &FoodQualityAssessmentRequestV1,
) -> FoodQualityAssessmentReportV1 {
    let mut issues = Vec::new();
    if request.schema_version != FOOD_QUALITY_REQUEST_SCHEMA_VERSION {
        add_issue(
            &mut issues,
            QualityIssueCodeV1::UnsupportedSchemaVersion,
            Some("request.schema_version"),
            "only FoodQualityAssessmentRequestV1 schema version 1 is supported",
        );
    }
    if !nonempty(&request.assessment_id)
        || !nonempty(&request.run_ref)
        || request.assessed_at_epoch_ms == 0
    {
        add_issue(
            &mut issues,
            QualityIssueCodeV1::MissingRequiredValue,
            Some("request"),
            "assessment ID, run reference and explicit assessment time are required",
        );
    }
    if request.claim_ceiling != FOOD_QUALITY_CLAIM_CEILING {
        add_issue(
            &mut issues,
            QualityIssueCodeV1::InvalidClaimCeiling,
            Some("request.claim_ceiling"),
            "assessment claim ceiling does not match the fixed non-safety contract",
        );
    }
    let calculated_digest = protocol_digest(&request.protocol).unwrap_or_default();
    if request.protocol_sha256 != calculated_digest || !is_sha256(&request.protocol_sha256) {
        add_issue(
            &mut issues,
            QualityIssueCodeV1::InvalidProtocolDigest,
            Some("protocol_sha256"),
            "protocol digest must match the exact typed protocol serialization",
        );
    }

    let requirements = validate_protocol(&request.protocol, &mut issues);
    let mut raw_digests = BTreeSet::new();
    let sensory_summaries = summarize_sensory(request, &mut issues, &mut raw_digests);
    let instrumental_summaries =
        summarize_instrumental(request, &requirements, &mut issues, &mut raw_digests);

    issues.sort_by(|left, right| {
        (
            left.code,
            left.reference.as_deref().unwrap_or(""),
            left.detail.as_str(),
        )
            .cmp(&(
                right.code,
                right.reference.as_deref().unwrap_or(""),
                right.detail.as_str(),
            ))
    });
    let disposition = if issues.is_empty() {
        QualityDispositionV1::Complete
    } else if issues.iter().any(|entry| {
        matches!(
            entry.code,
            QualityIssueCodeV1::DuplicateSampleRef
                | QualityIssueCodeV1::DuplicateMetricRequirement
                | QualityIssueCodeV1::DuplicateObservation
                | QualityIssueCodeV1::DuplicateResponse
                | QualityIssueCodeV1::DuplicateRespondentSample
                | QualityIssueCodeV1::DuplicatePresentationOrder
                | QualityIssueCodeV1::InvalidProtocolDigest
        )
    }) {
        QualityDispositionV1::Conflicted
    } else if issues.iter().any(|entry| {
        matches!(
            entry.code,
            QualityIssueCodeV1::StudyNotBlinded | QualityIssueCodeV1::StudyNotRandomized
        )
    }) {
        QualityDispositionV1::Blocked
    } else if issues.iter().any(|entry| {
        matches!(
            entry.code,
            QualityIssueCodeV1::MeasurementOutsideQualityLimit
                | QualityIssueCodeV1::ConsumerScoreBelowThreshold
        )
    }) {
        QualityDispositionV1::NotQualified
    } else {
        QualityDispositionV1::Indeterminate
    };

    FoodQualityAssessmentReportV1 {
        schema_version: FOOD_QUALITY_REPORT_SCHEMA_VERSION,
        assessment_id: request.assessment_id.clone(),
        run_ref: request.run_ref.clone(),
        protocol_id: request.protocol.protocol_id.clone(),
        protocol_revision: request.protocol.revision,
        protocol_sha256: calculated_digest,
        assessor_version_ref: FOOD_QUALITY_ASSESSOR_VERSION_REF.to_owned(),
        disposition,
        issues,
        sensory_summaries,
        instrumental_summaries,
        external_safety_gate: request.external_safety_gate.clone(),
        raw_evidence_sha256: raw_digests.into_iter().collect(),
        claim_ceiling: FOOD_QUALITY_CLAIM_CEILING.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: &impl Serialize) -> String {
        let bytes = serde_json::to_vec(value).unwrap();
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join("")
    }

    fn valid_request() -> FoodQualityAssessmentRequestV1 {
        let protocol = FoodQualityProtocolV1 {
            schema_version: 1,
            protocol_id: "protocol:rice-acceptance-v1".to_owned(),
            revision: 1,
            dish_ref: "dish:rice".to_owned(),
            sample_refs: vec!["sample:control".to_owned(), "sample:workcell".to_owned()],
            sensory: SensoryProtocolV1 {
                method_ref: "method:iso-11136-consumer-hedonic".to_owned(),
                population_scope_ref: "population:adult-rice-consumers".to_owned(),
                scale_min: 1,
                scale_max: 9,
                minimum_responses_per_sample: 3,
                minimum_mean_liking_milli: Some(5_500),
                blinded: true,
                blinding_protocol_ref: Some("blinding:codes-v1".to_owned()),
                randomized: true,
                randomization_protocol_ref: Some("randomization:balanced-order-v1".to_owned()),
            },
            instrumental_requirements: vec![InstrumentalMetricRequirementV1 {
                metric_id: "moisture".to_owned(),
                unit: "percent".to_owned(),
                basis: "wet-basis".to_owned(),
                method_ref: "method:moisture-reference-v1".to_owned(),
                decimal_places: 2,
                required: true,
                minimum_value_scaled: Some(500),
                maximum_value_scaled: Some(750),
            }],
            claim_ceiling: FOOD_QUALITY_CLAIM_CEILING.to_owned(),
        };
        let protocol_sha256 = digest(&protocol);
        let mut instrumental_observations = Vec::new();
        let mut consumer_responses = Vec::new();
        for sample_ref in ["sample:control", "sample:workcell"] {
            instrumental_observations.push(InstrumentalQualityObservationV1 {
                observation_id: format!("obs:{sample_ref}:moisture"),
                run_ref: "run:rice-comparison-01".to_owned(),
                sample_ref: sample_ref.to_owned(),
                metric_id: "moisture".to_owned(),
                value_scaled: 620,
                decimal_places: 2,
                unit: "percent".to_owned(),
                basis: "wet-basis".to_owned(),
                method_ref: "method:moisture-reference-v1".to_owned(),
                sample_position_ref: "position:center-mixed".to_owned(),
                instrument_ref: "instrument:moisture-meter-01".to_owned(),
                calibration_ref: "calibration:moisture-meter-2026-11".to_owned(),
                uncertainty_scaled: 10,
                observed_at_epoch_ms: 1_750_000_000_000,
                raw_observation_sha256: "a".repeat(64),
            });
            for index in 0..3 {
                consumer_responses.push(ConsumerResponseV1 {
                    response_id: format!("response:{sample_ref}:{index}"),
                    run_ref: "run:rice-comparison-01".to_owned(),
                    sample_ref: sample_ref.to_owned(),
                    respondent_token_ref: format!("token:{index}"),
                    response_value: if sample_ref == "sample:workcell" { 7 } else { 6 },
                    presentation_order: if sample_ref == "sample:workcell" { 2 } else { 1 },
                    consent_receipt_ref: format!("consent:session:{index}"),
                    raw_response_sha256: format!("{:064x}", index + 1),
                });
            }
        }
        FoodQualityAssessmentRequestV1 {
            schema_version: 1,
            assessment_id: "assessment:rice-01".to_owned(),
            run_ref: "run:rice-comparison-01".to_owned(),
            assessed_at_epoch_ms: 1_750_000_000_100,
            protocol_sha256,
            protocol,
            instrumental_observations,
            consumer_responses,
            external_safety_gate: ExternalSafetyGateEvidenceV1 {
                state: ExternalSafetyGateStateV1::Unknown,
                scope_ref: Some("scope:separate-food-safety-review".to_owned()),
                competent_authority_ref: None,
                evidence_ref: None,
                valid_until_epoch_ms: None,
            },
            claim_ceiling: FOOD_QUALITY_CLAIM_CEILING.to_owned(),
        }
    }

    #[test]
    fn complete_quality_evidence_never_grants_a_serving_permit() {
        let report = assess_food_quality(&valid_request());
        assert_eq!(report.disposition, QualityDispositionV1::Complete, "{:?}", report.issues);
        assert!(report.has_complete_quality_evidence());
        assert!(!report.grants_serving_permit());
        assert_eq!(report.external_safety_gate.state, ExternalSafetyGateStateV1::Unknown);
        assert_eq!(report.sensory_summaries.len(), 2);
        assert_eq!(report.instrumental_summaries.len(), 2);
    }

    #[test]
    fn unblinded_or_unrandomized_study_is_blocked() {
        let mut request = valid_request();
        request.protocol.sensory.blinded = false;
        request.protocol.sensory.randomized = false;
        request.protocol_sha256 = digest(&request.protocol);
        let report = assess_food_quality(&request);
        assert_eq!(report.disposition, QualityDispositionV1::Blocked);
    }

    #[test]
    fn missing_consumer_responses_remain_indeterminate() {
        let mut request = valid_request();
        request.consumer_responses.retain(|response| response.sample_ref != "sample:workcell");
        let report = assess_food_quality(&request);
        assert_eq!(report.disposition, QualityDispositionV1::Indeterminate);
        assert!(report.issues.iter().any(|entry| {
            entry.code == QualityIssueCodeV1::InsufficientConsumerResponses
        }));
    }

    #[test]
    fn low_liking_cannot_be_offset_by_instrumental_quality() {
        let mut request = valid_request();
        for response in &mut request.consumer_responses {
            if response.sample_ref == "sample:workcell" {
                response.response_value = 2;
            }
        }
        let report = assess_food_quality(&request);
        assert_eq!(report.disposition, QualityDispositionV1::NotQualified);
        assert!(report.issues.iter().any(|entry| {
            entry.code == QualityIssueCodeV1::ConsumerScoreBelowThreshold
        }));
    }

    #[test]
    fn uncertainty_crossing_a_limit_is_indeterminate() {
        let mut request = valid_request();
        request.instrumental_observations[0].value_scaled = 505;
        request.instrumental_observations[0].uncertainty_scaled = 20;
        let report = assess_food_quality(&request);
        assert_eq!(report.disposition, QualityDispositionV1::Indeterminate);
        assert!(report.issues.iter().any(|entry| {
            entry.code == QualityIssueCodeV1::MeasurementUncertaintyOverlapsLimit
        }));
    }

    #[test]
    fn missing_calibration_is_not_measured_quality_evidence() {
        let mut request = valid_request();
        request.instrumental_observations[0].calibration_ref.clear();
        let report = assess_food_quality(&request);
        assert_eq!(report.disposition, QualityDispositionV1::Indeterminate);
        assert!(report.issues.iter().any(|entry| {
            entry.code == QualityIssueCodeV1::MissingCalibration
        }));
    }

    #[test]
    fn duplicate_respondent_sample_pair_is_conflicted() {
        let mut request = valid_request();
        request.consumer_responses.push(request.consumer_responses[0].clone());
        let report = assess_food_quality(&request);
        assert_eq!(report.disposition, QualityDispositionV1::Conflicted);
    }

    #[test]
    fn protocol_revision_change_invalidates_the_bound_digest() {
        let mut request = valid_request();
        request.protocol.revision = 2;
        let report = assess_food_quality(&request);
        assert_eq!(report.disposition, QualityDispositionV1::Conflicted);
        assert!(report.issues.iter().any(|entry| {
            entry.code == QualityIssueCodeV1::InvalidProtocolDigest
        }));
    }

    #[test]
    fn external_failure_remains_visible_but_is_not_rewritten_as_quality() {
        let mut request = valid_request();
        request.external_safety_gate.state = ExternalSafetyGateStateV1::ExternallyFailed;
        request.external_safety_gate.evidence_ref = Some("external:evidence:failed".to_owned());
        let report = assess_food_quality(&request);
        assert_eq!(report.disposition, QualityDispositionV1::Complete);
        assert_eq!(report.external_safety_gate.state, ExternalSafetyGateStateV1::ExternallyFailed);
        assert!(!report.grants_serving_permit());
    }
}
