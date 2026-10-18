// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bounded repair/outage chronology composed over canonical service requests.
//! It separates machine breakdowns from scheduled work, weather/site access and
//! operator availability. Source references are not authenticated here.

use crate::field_robot_cost::{CostEvidenceDispositionV1, CostEvidenceStateV1};
use crate::service_access::{
    ServiceAccessCalculationDispositionV1, ServiceAccessEvidenceV1, ServiceAccessInputV1,
    assess_service_access_v1,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const REPAIR_RECOVERY_SCHEMA_VERSION_V1: u16 = 1;

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
mod option_u64_string {
    use super::*;
    pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match value {
            Some(v) => serializer.serialize_some(&v.to_string()),
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
pub enum InterruptionCauseV1 {
    Breakdown,
    ScheduledMaintenance,
    WeatherOrSiteAccess,
    OperatorUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartRequirementV1 {
    NotAssessed,
    NotRequired,
    Required,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairRecoveryBlockerCodeV1 {
    UnsupportedSchema,
    CanonicalServiceAccessBlocked,
    EmptyIdentity,
    ScenarioMismatch,
    ScopeMismatch,
    PeriodMismatch,
    CurrencyMismatch,
    DuplicateEpisodeId,
    UnknownAffectedRequest,
    DuplicateAffectedRequestLink,
    InvalidInterruptionShape,
    InvalidPartRequirementShape,
    InvalidEventOrder,
    EventAfterAsOf,
    MissingEvidence,
    StaleEvidence,
    ConflictingEvidence,
    MalformedEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRecoveryBlockerV1 {
    pub code: RepairRecoveryBlockerCodeV1,
    pub episode_id: Option<String>,
    pub detail: String,
}

/// One repair or availability interruption. Timestamps are integer minutes from
/// the declared period origin. Open episodes have no asset-verified timestamp.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairEpisodeV1 {
    pub episode_id: String,
    pub scenario_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub currency_code: String,
    pub asset_id: String,
    pub asset_configuration_ref: String,
    pub cause: InterruptionCauseV1,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub outage_started_at_minute: u64,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub failure_detected_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub asset_removed_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub diagnosis_completed_at_minute: Option<u64>,
    pub part_requirement: PartRequirementV1,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub part_required_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub part_ordered_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub part_received_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub repair_started_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub repair_completed_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub service_restored_at_minute: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub asset_verified_available_at_minute: Option<u64>,
    pub affected_request_ids: Vec<String>,
    pub evidence: ServiceAccessEvidenceV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRecoveryInputV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub currency_code: String,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub as_of_minute: u64,
    /// Declaration about the supplied log window only; no authentication is implied.
    pub declared_log_complete: bool,
    pub episodes: Vec<RepairEpisodeV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRecoveryBreakdownV1 {
    pub declared_log_complete: bool,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub episode_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub breakdown_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub scheduled_maintenance_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub external_interruption_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub verified_episode_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub open_episode_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub parts_required_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub parts_received_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub open_parts_wait_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub affected_request_link_count: u64,
    #[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
    pub unique_affected_request_count: u64,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub breakdown_restore_p50_minutes: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub breakdown_restore_p90_minutes: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub parts_wait_p50_minutes: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub parts_wait_p90_minutes: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub external_interruption_p50_minutes: Option<u64>,
    #[serde(
        serialize_with = "option_u64_string::serialize",
        deserialize_with = "option_u64_string::deserialize"
    )]
    pub external_interruption_p90_minutes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairRecoveryCalculationDispositionV1 {
    Calculated,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRecoveryAssessmentV1 {
    pub scenario_id: String,
    pub calculation_disposition: RepairRecoveryCalculationDispositionV1,
    pub evidence_disposition: CostEvidenceDispositionV1,
    pub blockers: Vec<RepairRecoveryBlockerV1>,
    pub breakdown: Option<RepairRecoveryBreakdownV1>,
}

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
fn valid_currency_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn push_blocker(
    blockers: &mut Vec<RepairRecoveryBlockerV1>,
    code: RepairRecoveryBlockerCodeV1,
    episode_id: Option<&str>,
    detail: &str,
) {
    let blocker = RepairRecoveryBlockerV1 {
        code,
        episode_id: episode_id.map(str::to_owned),
        detail: detail.to_owned(),
    };
    if !blockers.contains(&blocker) {
        blockers.push(blocker);
    }
}
fn episode_times(episode: &RepairEpisodeV1) -> [Option<u64>; 11] {
    [
        Some(episode.outage_started_at_minute),
        episode.failure_detected_at_minute,
        episode.asset_removed_at_minute,
        episode.diagnosis_completed_at_minute,
        episode.part_required_at_minute,
        episode.part_ordered_at_minute,
        episode.part_received_at_minute,
        episode.repair_started_at_minute,
        episode.repair_completed_at_minute,
        episode.service_restored_at_minute,
        episode.asset_verified_available_at_minute,
    ]
}
fn validate_evidence(episode: &RepairEpisodeV1, blockers: &mut Vec<RepairRecoveryBlockerV1>) {
    let id = Some(episode.episode_id.as_str());
    if episode
        .evidence
        .artifact_sha256
        .as_deref()
        .is_some_and(|v| !valid_sha256(v))
    {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::MalformedEvidence,
            id,
            "evidence digest is not 64 hexadecimal characters",
        );
    }
    match episode.evidence.state {
        CostEvidenceStateV1::SourceReferenced => {
            if episode
                .evidence
                .source_ref
                .as_deref()
                .is_none_or(|v| v.trim().is_empty())
                || episode
                    .evidence
                    .artifact_sha256
                    .as_deref()
                    .is_none_or(|v| !valid_sha256(v))
            {
                push_blocker(
                    blockers,
                    RepairRecoveryBlockerCodeV1::MalformedEvidence,
                    id,
                    "source-referenced evidence requires source reference and SHA-256-shaped digest",
                );
            }
        }
        CostEvidenceStateV1::Estimated => {
            if episode
                .evidence
                .source_ref
                .as_deref()
                .is_none_or(|v| v.trim().is_empty())
            {
                push_blocker(
                    blockers,
                    RepairRecoveryBlockerCodeV1::MalformedEvidence,
                    id,
                    "estimated evidence requires source reference",
                );
            }
        }
        CostEvidenceStateV1::SyntheticFixture => {
            if !episode
                .evidence
                .source_ref
                .as_deref()
                .is_some_and(|v| v.starts_with("synthetic-fixture:"))
            {
                push_blocker(
                    blockers,
                    RepairRecoveryBlockerCodeV1::MalformedEvidence,
                    id,
                    "synthetic evidence must use explicit fixture reference",
                );
            }
        }
        CostEvidenceStateV1::Missing => push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::MissingEvidence,
            id,
            "repair event evidence is missing",
        ),
        CostEvidenceStateV1::Stale => push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::StaleEvidence,
            id,
            "repair event evidence is stale",
        ),
        CostEvidenceStateV1::Conflicting => push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::ConflictingEvidence,
            id,
            "repair event evidence conflicts",
        ),
    }
}
fn validate_episode(
    input: &RepairRecoveryInputV1,
    episode: &RepairEpisodeV1,
    known_requests: &BTreeSet<&str>,
    seen_episode_ids: &mut BTreeSet<String>,
    blockers: &mut Vec<RepairRecoveryBlockerV1>,
) {
    let id = Some(episode.episode_id.as_str());
    if episode.episode_id.trim().is_empty()
        || episode.asset_id.trim().is_empty()
        || episode.asset_configuration_ref.trim().is_empty()
    {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::EmptyIdentity,
            id,
            "episode, asset and configuration identities must be non-empty",
        );
    }
    if !seen_episode_ids.insert(episode.episode_id.clone()) {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::DuplicateEpisodeId,
            id,
            "episode IDs must be unique",
        );
    }
    if episode.scenario_id != input.scenario_id {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::ScenarioMismatch,
            id,
            "episode scenario must match assessment",
        );
    }
    if episode.scope_ref != input.scope_ref {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::ScopeMismatch,
            id,
            "episode scope must match assessment",
        );
    }
    if episode.period_ref != input.period_ref {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::PeriodMismatch,
            id,
            "episode period must match assessment",
        );
    }
    if episode.currency_code != input.currency_code || !valid_currency_code(&episode.currency_code)
    {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::CurrencyMismatch,
            id,
            "episode currency must match assessment and be uppercase three-letter code",
        );
    }
    for time in episode_times(episode).into_iter().flatten() {
        if time > input.as_of_minute {
            push_blocker(
                blockers,
                RepairRecoveryBlockerCodeV1::EventAfterAsOf,
                id,
                "event timestamp is after assessment as-of time",
            );
            break;
        }
    }
    let mut previous = None;
    for time in episode_times(episode).into_iter().flatten() {
        if previous.is_some_and(|last| time < last) {
            push_blocker(
                blockers,
                RepairRecoveryBlockerCodeV1::InvalidEventOrder,
                id,
                "event timestamps must be monotone in the declared lifecycle",
            );
            break;
        }
        previous = Some(time);
    }

    match episode.cause {
        InterruptionCauseV1::Breakdown => {
            if episode.failure_detected_at_minute.is_none()
                || episode.asset_removed_at_minute.is_none()
            {
                push_blocker(
                    blockers,
                    RepairRecoveryBlockerCodeV1::InvalidInterruptionShape,
                    id,
                    "breakdown requires failure detection and asset removal timestamps",
                );
            }
        }
        InterruptionCauseV1::ScheduledMaintenance => {
            if episode.failure_detected_at_minute.is_some()
                || episode.asset_removed_at_minute.is_none()
            {
                push_blocker(
                    blockers,
                    RepairRecoveryBlockerCodeV1::InvalidInterruptionShape,
                    id,
                    "scheduled maintenance must not claim a failure-detection time and requires asset removal",
                );
            }
        }
        InterruptionCauseV1::WeatherOrSiteAccess | InterruptionCauseV1::OperatorUnavailable => {
            if episode.failure_detected_at_minute.is_some()
                || episode.asset_removed_at_minute.is_some()
                || episode.diagnosis_completed_at_minute.is_some()
                || episode.part_required_at_minute.is_some()
                || episode.part_ordered_at_minute.is_some()
                || episode.part_received_at_minute.is_some()
                || episode.repair_started_at_minute.is_some()
                || episode.repair_completed_at_minute.is_some()
                || episode.part_requirement != PartRequirementV1::NotAssessed
            {
                push_blocker(
                    blockers,
                    RepairRecoveryBlockerCodeV1::InvalidInterruptionShape,
                    id,
                    "weather/access and operator interruptions must not claim machine failure or repair events",
                );
            }
        }
    }

    match episode.part_requirement {
        PartRequirementV1::NotAssessed | PartRequirementV1::NotRequired => {
            if episode.part_required_at_minute.is_some()
                || episode.part_ordered_at_minute.is_some()
                || episode.part_received_at_minute.is_some()
            {
                push_blocker(
                    blockers,
                    RepairRecoveryBlockerCodeV1::InvalidPartRequirementShape,
                    id,
                    "part timestamps require a declared part requirement",
                );
            }
        }
        PartRequirementV1::Required => {
            if episode.part_required_at_minute.is_none() {
                push_blocker(
                    blockers,
                    RepairRecoveryBlockerCodeV1::InvalidPartRequirementShape,
                    id,
                    "required part needs a required-at timestamp",
                );
            }
        }
    }
    if episode.part_ordered_at_minute.is_some()
        && (episode.part_requirement != PartRequirementV1::Required
            || episode.part_required_at_minute.is_none())
    {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::InvalidPartRequirementShape,
            id,
            "part order requires a recorded part requirement",
        );
    }
    if episode.part_received_at_minute.is_some() && episode.part_ordered_at_minute.is_none() {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::InvalidPartRequirementShape,
            id,
            "part receipt requires a part-order timestamp",
        );
    }
    if episode.repair_started_at_minute.is_some() && episode.diagnosis_completed_at_minute.is_none()
    {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::InvalidEventOrder,
            id,
            "repair start requires completed diagnosis",
        );
    }
    if episode.part_requirement == PartRequirementV1::Required
        && episode.repair_started_at_minute.is_some()
        && episode.part_received_at_minute.is_none()
    {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::InvalidPartRequirementShape,
            id,
            "repair cannot start before a declared required part is received",
        );
    }
    if episode.repair_completed_at_minute.is_some() && episode.repair_started_at_minute.is_none() {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::InvalidEventOrder,
            id,
            "repair completion requires repair start",
        );
    }
    if matches!(
        episode.cause,
        InterruptionCauseV1::Breakdown | InterruptionCauseV1::ScheduledMaintenance
    ) && episode.service_restored_at_minute.is_some()
        && episode.repair_completed_at_minute.is_none()
    {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::InvalidEventOrder,
            id,
            "machine service restoration requires recorded repair or maintenance completion",
        );
    }
    if episode.asset_verified_available_at_minute.is_some()
        && episode.service_restored_at_minute.is_none()
    {
        push_blocker(
            blockers,
            RepairRecoveryBlockerCodeV1::InvalidEventOrder,
            id,
            "asset availability verification requires service-restored event",
        );
    }

    let mut episode_requests = BTreeSet::new();
    for request_id in &episode.affected_request_ids {
        if !known_requests.contains(request_id.as_str()) {
            push_blocker(
                blockers,
                RepairRecoveryBlockerCodeV1::UnknownAffectedRequest,
                id,
                "affected request ID is absent from the canonical service-access ledger",
            );
        }
        if !episode_requests.insert(request_id.as_str()) {
            push_blocker(
                blockers,
                RepairRecoveryBlockerCodeV1::DuplicateAffectedRequestLink,
                id,
                "affected request ID is repeated within the episode",
            );
        }
    }
    validate_evidence(episode, blockers);
}
fn nearest_rank(values: &mut [u64], percentile: u8) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let len = u128::try_from(values.len()).ok()?;
    let rank = (u128::from(percentile) * len).div_ceil(100);
    let index = usize::try_from(rank.max(1) - 1).ok()?;
    values.get(index).copied()
}

/// Validate all episodes before publishing a summary. Missing end events remain open;
/// they are not assigned zero duration or silently dropped.
pub fn assess_repair_recovery_v1(
    service_access: &ServiceAccessInputV1,
    input: &RepairRecoveryInputV1,
) -> RepairRecoveryAssessmentV1 {
    let mut blockers = Vec::new();
    let canonical = assess_service_access_v1(service_access);
    if canonical.calculation_disposition != ServiceAccessCalculationDispositionV1::Calculated {
        push_blocker(
            &mut blockers,
            RepairRecoveryBlockerCodeV1::CanonicalServiceAccessBlocked,
            None,
            "canonical service-access ledger is blocked",
        );
    }
    if input.schema_version != REPAIR_RECOVERY_SCHEMA_VERSION_V1 {
        push_blocker(
            &mut blockers,
            RepairRecoveryBlockerCodeV1::UnsupportedSchema,
            None,
            "unsupported repair-recovery schema",
        );
    }
    if input.scenario_id.trim().is_empty()
        || input.scope_ref.trim().is_empty()
        || input.period_ref.trim().is_empty()
    {
        push_blocker(
            &mut blockers,
            RepairRecoveryBlockerCodeV1::EmptyIdentity,
            None,
            "assessment scenario, scope and period must be non-empty",
        );
    }
    if input.scenario_id != service_access.scenario_id {
        push_blocker(
            &mut blockers,
            RepairRecoveryBlockerCodeV1::ScenarioMismatch,
            None,
            "assessment scenario differs from canonical service-access ledger",
        );
    }
    if input.scope_ref != service_access.scope_ref {
        push_blocker(
            &mut blockers,
            RepairRecoveryBlockerCodeV1::ScopeMismatch,
            None,
            "assessment scope differs from canonical service-access ledger",
        );
    }
    if input.period_ref != service_access.period_ref {
        push_blocker(
            &mut blockers,
            RepairRecoveryBlockerCodeV1::PeriodMismatch,
            None,
            "assessment period differs from canonical service-access ledger",
        );
    }
    if input.currency_code != service_access.currency_code
        || !valid_currency_code(&input.currency_code)
    {
        push_blocker(
            &mut blockers,
            RepairRecoveryBlockerCodeV1::CurrencyMismatch,
            None,
            "assessment currency differs from canonical ledger or is malformed",
        );
    }
    let known_requests: BTreeSet<&str> = service_access
        .requests
        .iter()
        .map(|r| r.request_id.as_str())
        .collect();
    let mut episode_ids = BTreeSet::new();
    for episode in &input.episodes {
        validate_episode(
            input,
            episode,
            &known_requests,
            &mut episode_ids,
            &mut blockers,
        );
    }
    if !blockers.is_empty() {
        return RepairRecoveryAssessmentV1 {
            scenario_id: input.scenario_id.clone(),
            calculation_disposition: RepairRecoveryCalculationDispositionV1::Blocked,
            evidence_disposition: CostEvidenceDispositionV1::Blocked,
            blockers,
            breakdown: None,
        };
    }

    let breakdowns: Vec<_> = input
        .episodes
        .iter()
        .filter(|e| e.cause == InterruptionCauseV1::Breakdown)
        .collect();
    let scheduled: Vec<_> = input
        .episodes
        .iter()
        .filter(|e| e.cause == InterruptionCauseV1::ScheduledMaintenance)
        .collect();
    let external: Vec<_> = input
        .episodes
        .iter()
        .filter(|e| {
            matches!(
                e.cause,
                InterruptionCauseV1::WeatherOrSiteAccess | InterruptionCauseV1::OperatorUnavailable
            )
        })
        .collect();
    let verified = input
        .episodes
        .iter()
        .filter(|e| e.asset_verified_available_at_minute.is_some())
        .count();
    let open = input.episodes.len().saturating_sub(verified);
    let required = input
        .episodes
        .iter()
        .filter(|e| e.part_requirement == PartRequirementV1::Required)
        .count();
    let received = input
        .episodes
        .iter()
        .filter(|e| e.part_received_at_minute.is_some())
        .count();
    let open_parts = input
        .episodes
        .iter()
        .filter(|e| {
            e.part_requirement == PartRequirementV1::Required && e.part_received_at_minute.is_none()
        })
        .count();
    let link_count: usize = input
        .episodes
        .iter()
        .map(|e| e.affected_request_ids.len())
        .sum();
    let unique_affected: BTreeSet<&str> = input
        .episodes
        .iter()
        .flat_map(|e| e.affected_request_ids.iter().map(String::as_str))
        .collect();
    let mut repair_waits: Vec<u64> = breakdowns
        .iter()
        .filter_map(|e| Some(e.asset_verified_available_at_minute? - e.failure_detected_at_minute?))
        .collect();
    let mut part_waits: Vec<u64> = input
        .episodes
        .iter()
        .filter_map(|e| Some(e.part_received_at_minute? - e.part_ordered_at_minute?))
        .collect();
    let mut external_waits: Vec<u64> = external
        .iter()
        .filter_map(|e| Some(e.asset_verified_available_at_minute? - e.outage_started_at_minute))
        .collect();
    let restore_p50 = nearest_rank(&mut repair_waits, 50);
    let restore_p90 = nearest_rank(&mut repair_waits, 90);
    let parts_p50 = nearest_rank(&mut part_waits, 50);
    let parts_p90 = nearest_rank(&mut part_waits, 90);
    let external_p50 = nearest_rank(&mut external_waits, 50);
    let external_p90 = nearest_rank(&mut external_waits, 90);
    let source_only = canonical.evidence_disposition
        == CostEvidenceDispositionV1::SourceReferencedInputs
        && input
            .episodes
            .iter()
            .all(|e| e.evidence.state == CostEvidenceStateV1::SourceReferenced);
    let breakdown = RepairRecoveryBreakdownV1 {
        declared_log_complete: input.declared_log_complete,
        episode_count: count(input.episodes.len()),
        breakdown_count: count(breakdowns.len()),
        scheduled_maintenance_count: count(scheduled.len()),
        external_interruption_count: count(external.len()),
        verified_episode_count: count(verified),
        open_episode_count: count(open),
        parts_required_count: count(required),
        parts_received_count: count(received),
        open_parts_wait_count: count(open_parts),
        affected_request_link_count: count(link_count),
        unique_affected_request_count: count(unique_affected.len()),
        breakdown_restore_p50_minutes: restore_p50,
        breakdown_restore_p90_minutes: restore_p90,
        parts_wait_p50_minutes: parts_p50,
        parts_wait_p90_minutes: parts_p90,
        external_interruption_p50_minutes: external_p50,
        external_interruption_p90_minutes: external_p90,
    };
    RepairRecoveryAssessmentV1 {
        scenario_id: input.scenario_id.clone(),
        calculation_disposition: RepairRecoveryCalculationDispositionV1::Calculated,
        evidence_disposition: if source_only {
            CostEvidenceDispositionV1::SourceReferencedInputs
        } else {
            CostEvidenceDispositionV1::ProvisionalInputs
        },
        blockers: Vec::new(),
        breakdown: Some(breakdown),
    }
}
