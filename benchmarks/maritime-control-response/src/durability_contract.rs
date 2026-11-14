#![forbid(unsafe_code)]

//! Exposure-based maritime repair-durability outcome contract.
//!
//! This module validates follow-up chronology, cumulative operating exposure,
//! counter epochs/resets, typed recurrence/competing/censoring events and
//! evidence/acceptance-envelope lineage. It does not estimate survival curves,
//! hazards, RUL or causal repair effects.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const DURABILITY_SCHEMA: &str = "symtropy-marine-repair-durability-v0.1";
const TIME_TOLERANCE_SECONDS: f64 = 1e-9;
const EXPOSURE_TOLERANCE: f64 = 1e-9;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExposureClock {
    EngineHours,
    OperatingCycles,
    Starts,
    DutyCycles,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceQuality {
    Valid,
    Missing,
    Stale,
    Rejected,
    Conflicted,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationStatus {
    Verified,
    Unknown,
    Expired,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DurabilityEventType {
    TargetCapabilityRecurrence,
    RepeatCorrectiveRepair,
    NewComponentFailure,
    PreventiveReplacement,
    SensorOrMeasurementChange,
    ControllerOrConfigurationChange,
    AssetCompetingEvent,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FollowupEndKind {
    ObservedEvent,
    CompetingEvent,
    Censoring,
    AdministrativeEnd,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DurabilityDisposition {
    CompleteThroughHorizon,
    TargetRecurrenceObserved,
    OtherEventObserved,
    CompetingEvent,
    CensoredBeforeHorizon,
    AdministrativeEndBeforeHorizon,
    NotEvaluable,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DurabilityProfile {
    pub schema_version: String,
    pub case_id: String,
    pub asset_id: String,
    pub component_id: String,
    pub capability_id: String,
    pub intervention_id: String,
    pub intervention_record_sha256: String,
    pub intervention_completed_at_seconds: f64,
    pub restoration_verified_at_seconds: f64,
    pub restoration_report_sha256: String,
    pub acceptance_envelope_owner_ref: String,
    pub acceptance_envelope_sha256: String,
    pub followup_started_at_seconds: f64,
    pub analysis_as_of_seconds: f64,
    pub exposure_clock: ExposureClock,
    pub exposure_unit: String,
    pub target_horizon_exposure: f64,
    pub maximum_calendar_followup_seconds: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExposureEpoch {
    pub epoch_id: String,
    pub counter_source_id: String,
    pub clock: ExposureClock,
    pub unit: String,
    pub started_at_seconds: f64,
    pub ended_at_seconds: f64,
    pub counter_value_start: f64,
    pub counter_value_end: f64,
    /// Cumulative exposure since the index intervention at this epoch's start.
    pub cumulative_exposure_at_start: f64,
    pub reset_from_epoch_id: Option<String>,
    pub reset_receipt_sha256: Option<String>,
    pub epoch_record_sha256: String,
    pub evidence_available_at_seconds: f64,
    pub quality: EvidenceQuality,
    pub calibration_status: CalibrationStatus,
    pub calibration_record_sha256: String,
    pub calibration_valid_from_seconds: f64,
    pub calibration_valid_until_seconds: f64,
}

impl ExposureEpoch {
    pub fn cumulative_exposure_end(&self) -> f64 {
        self.cumulative_exposure_at_start
            + (self.counter_value_end - self.counter_value_start)
    }

    pub fn cumulative_exposure_at(&self, counter_value: f64) -> f64 {
        self.cumulative_exposure_at_start + (counter_value - self.counter_value_start)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DurabilityEvent {
    pub event_id: String,
    pub event_record_sha256: String,
    pub event_type: DurabilityEventType,
    pub asset_id: String,
    pub component_id: String,
    pub capability_id: String,
    pub intervention_id: String,
    pub restoration_report_sha256: String,
    pub acceptance_envelope_sha256: String,
    pub comparability_review_sha256: Option<String>,
    pub comparability_reviewer_ref: Option<String>,
    pub epoch_id: String,
    pub counter_value: f64,
    pub observed_at_seconds: f64,
    pub evidence_available_at_seconds: f64,
    pub evidence_sha256: String,
    pub adjudication_rule_sha256: String,
    pub reviewer_ref: String,
    pub source_id: String,
    pub measurement_reference_path_id: Option<String>,
    pub measurement_quality: EvidenceQuality,
    pub calibration_status: CalibrationStatus,
    pub calibration_record_sha256: Option<String>,
    pub calibration_valid_from_seconds: Option<f64>,
    pub calibration_valid_until_seconds: Option<f64>,
    pub preceding_event_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FollowupEnd {
    pub kind: FollowupEndKind,
    pub ended_at_seconds: f64,
    pub epoch_id: String,
    pub counter_value: f64,
    pub evidence_sha256: String,
    pub reason_ref: String,
    pub linked_event_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DurabilityCase {
    pub profile: DurabilityProfile,
    pub exposure_epochs: Vec<ExposureEpoch>,
    pub events: Vec<DurabilityEvent>,
    pub followup_end: FollowupEnd,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DurabilityReport {
    pub schema_version: String,
    pub case_id: String,
    pub disposition: DurabilityDisposition,
    pub observed_exposure_through: f64,
    pub target_horizon_exposure: f64,
    pub target_recurrence_count: usize,
    pub repeat_corrective_repair_count: usize,
    pub new_component_failure_count: usize,
    pub preventive_replacement_count: usize,
    pub sensor_or_measurement_change_count: usize,
    pub controller_or_configuration_change_count: usize,
    pub competing_event_count: usize,
    pub blockers: Vec<String>,
    pub future_safe_life_estimated: bool,
    pub survival_or_hazard_estimated: bool,
    pub causal_repair_effect_estimated: bool,
    pub maintenance_authorized: bool,
    pub physical_restoration_authorized: bool,
    pub operating_restriction_clearance_authorized: bool,
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn finite_non_negative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_calibration_interval(
    status: CalibrationStatus,
    digest: Option<&str>,
    valid_from: Option<f64>,
    valid_until: Option<f64>,
    observed_at: f64,
    available_at: f64,
) -> bool {
    status == CalibrationStatus::Verified
        && digest.is_some_and(is_sha256_hex)
        && valid_from.is_some_and(|value| {
            finite_non_negative(value) && value <= observed_at + TIME_TOLERANCE_SECONDS
        })
        && valid_until.is_some_and(|value| {
            finite_non_negative(value) && value + TIME_TOLERANCE_SECONDS >= available_at
        })
}

fn epoch_map(case: &DurabilityCase) -> BTreeMap<&str, &ExposureEpoch> {
    case.exposure_epochs
        .iter()
        .map(|epoch| (epoch.epoch_id.as_str(), epoch))
        .collect()
}

fn validate_profile(profile: &DurabilityProfile, blockers: &mut Vec<String>) {
    if profile.schema_version != DURABILITY_SCHEMA {
        blockers.push("unexpected durability schema version".into());
    }
    for (name, identity) in [
        ("case", &profile.case_id),
        ("asset", &profile.asset_id),
        ("component", &profile.component_id),
        ("capability", &profile.capability_id),
        ("intervention", &profile.intervention_id),
        ("acceptance-envelope owner", &profile.acceptance_envelope_owner_ref),
        ("exposure unit", &profile.exposure_unit),
    ] {
        if identity.trim().is_empty() {
            blockers.push(format!("{name} identity is empty"));
        }
    }
    for (name, digest) in [
        ("intervention record", &profile.intervention_record_sha256),
        ("restoration report", &profile.restoration_report_sha256),
        ("acceptance envelope", &profile.acceptance_envelope_sha256),
    ] {
        if !is_sha256_hex(digest) {
            blockers.push(format!("{name} digest is invalid"));
        }
    }
    for (name, value) in [
        ("intervention completion", profile.intervention_completed_at_seconds),
        ("restoration verification", profile.restoration_verified_at_seconds),
        ("follow-up start", profile.followup_started_at_seconds),
        ("analysis as-of", profile.analysis_as_of_seconds),
        ("target exposure horizon", profile.target_horizon_exposure),
    ] {
        if !finite_non_negative(value) {
            blockers.push(format!("{name} value is non-finite or negative"));
        }
    }
    if profile.target_horizon_exposure <= 0.0 {
        blockers.push("target exposure horizon must be positive".into());
    }
    if profile.restoration_verified_at_seconds
        < profile.intervention_completed_at_seconds - TIME_TOLERANCE_SECONDS
        || profile.followup_started_at_seconds
            < profile.restoration_verified_at_seconds - TIME_TOLERANCE_SECONDS
        || profile.followup_started_at_seconds > profile.analysis_as_of_seconds
    {
        blockers.push("intervention, restoration and follow-up chronology is invalid".into());
    }
    if let Some(maximum) = profile.maximum_calendar_followup_seconds {
        if !finite_non_negative(maximum) || maximum <= 0.0 {
            blockers.push("maximum calendar follow-up horizon is invalid".into());
        }
    }
}

fn validate_epochs(case: &DurabilityCase, blockers: &mut Vec<String>) {
    let profile = &case.profile;
    if case.exposure_epochs.is_empty() {
        blockers.push("at least one exposure-counter epoch is required".into());
        return;
    }
    let mut ids = BTreeSet::new();
    let mut ordered = case.exposure_epochs.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.started_at_seconds.total_cmp(&right.started_at_seconds));

    let mut previous: Option<&ExposureEpoch> = None;
    for epoch in ordered {
        if epoch.epoch_id.trim().is_empty()
            || epoch.counter_source_id.trim().is_empty()
            || !ids.insert(epoch.epoch_id.as_str())
        {
            blockers.push(format!("exposure epoch identity is empty or duplicated: {}", epoch.epoch_id));
        }
        if epoch.clock != profile.exposure_clock || epoch.unit != profile.exposure_unit {
            blockers.push(format!("exposure epoch {} uses a different clock or unit", epoch.epoch_id));
        }
        let numeric = [
            epoch.started_at_seconds,
            epoch.ended_at_seconds,
            epoch.counter_value_start,
            epoch.counter_value_end,
            epoch.cumulative_exposure_at_start,
            epoch.evidence_available_at_seconds,
            epoch.calibration_valid_from_seconds,
            epoch.calibration_valid_until_seconds,
        ];
        if !numeric.iter().all(|value| finite_non_negative(*value))
            || !is_sha256_hex(&epoch.epoch_record_sha256)
            || !is_sha256_hex(&epoch.calibration_record_sha256)
            || epoch.quality != EvidenceQuality::Valid
            || epoch.calibration_status != CalibrationStatus::Verified
            || epoch.calibration_valid_from_seconds
                > epoch.started_at_seconds + TIME_TOLERANCE_SECONDS
            || epoch.calibration_valid_until_seconds
                + TIME_TOLERANCE_SECONDS
                < epoch.evidence_available_at_seconds
            || epoch.evidence_available_at_seconds > profile.analysis_as_of_seconds
            || epoch.ended_at_seconds < epoch.started_at_seconds
            || epoch.counter_value_end < epoch.counter_value_start
        {
            blockers.push(format!("exposure epoch {} has invalid evidence, calibration or counter values", epoch.epoch_id));
        }
        if let Some(previous_epoch) = previous {
            if epoch.reset_from_epoch_id.as_deref() != Some(previous_epoch.epoch_id.as_str())
                || !epoch.reset_receipt_sha256.as_deref().is_some_and(is_sha256_hex)
            {
                blockers.push(format!(
                    "exposure epoch {} is missing a linked reset receipt",
                    epoch.epoch_id
                ));
            }
            if epoch.started_at_seconds
                < previous_epoch.ended_at_seconds - TIME_TOLERANCE_SECONDS
            {
                blockers.push(format!("exposure epoch {} overlaps its predecessor", epoch.epoch_id));
            }
            if (epoch.cumulative_exposure_at_start
                - previous_epoch.cumulative_exposure_end())
                .abs()
                > EXPOSURE_TOLERANCE
            {
                blockers.push(format!(
                    "exposure epoch {} does not continue cumulative exposure from its predecessor",
                    epoch.epoch_id
                ));
            }
        } else if epoch.reset_from_epoch_id.is_some()
            || epoch.reset_receipt_sha256.is_some()
            || epoch.cumulative_exposure_at_start.abs() > EXPOSURE_TOLERANCE
        {
            blockers.push("first exposure epoch must start at zero without a reset predecessor".into());
        }
        previous = Some(epoch);
    }

    if let Some(first) = case
        .exposure_epochs
        .iter()
        .min_by(|left, right| left.started_at_seconds.total_cmp(&right.started_at_seconds))
    {
        if first.started_at_seconds > profile.followup_started_at_seconds + TIME_TOLERANCE_SECONDS {
            blockers.push("first exposure epoch starts after follow-up begins".into());
        }
    }
}

fn validate_events(case: &DurabilityCase, blockers: &mut Vec<String>) {
    let profile = &case.profile;
    let epochs = epoch_map(case);
    let mut event_ids = BTreeSet::new();
    let mut ordered = case.events.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.observed_at_seconds.total_cmp(&right.observed_at_seconds));

    let mut previous_exposure = profile.followup_started_at_seconds;
    let mut previous_event_at = profile.followup_started_at_seconds;
    let mut target_recurrence_ids = Vec::<String>::new();

    for event in ordered {
        if event.event_id.trim().is_empty()
            || !event_ids.insert(event.event_id.as_str())
            || !is_sha256_hex(&event.event_record_sha256)
            || !is_sha256_hex(&event.evidence_sha256)
            || !is_sha256_hex(&event.adjudication_rule_sha256)
            || event.reviewer_ref.trim().is_empty()
            || event.source_id.trim().is_empty()
            || event.asset_id.trim().is_empty()
            || event.component_id.trim().is_empty()
            || event.capability_id.trim().is_empty()
            || event.intervention_id.trim().is_empty()
        {
            blockers.push(format!("event {} has incomplete identity or evidence receipts", event.event_id));
        }
        if event.asset_id != profile.asset_id || event.intervention_id != profile.intervention_id {
            blockers.push(format!("event {} refers to a different asset or index intervention", event.event_id));
        }
        if event.restoration_report_sha256 != profile.restoration_report_sha256
            || !is_sha256_hex(&event.restoration_report_sha256)
        {
            blockers.push(format!("event {} does not bind the index restoration report", event.event_id));
        }
        if !finite_non_negative(event.observed_at_seconds)
            || !finite_non_negative(event.evidence_available_at_seconds)
            || !finite_non_negative(event.counter_value)
            || event.observed_at_seconds
                < profile.followup_started_at_seconds - TIME_TOLERANCE_SECONDS
            || event.observed_at_seconds
                < profile.intervention_completed_at_seconds - TIME_TOLERANCE_SECONDS
            || event.evidence_available_at_seconds
                < event.observed_at_seconds - TIME_TOLERANCE_SECONDS
            || event.evidence_available_at_seconds
                > profile.analysis_as_of_seconds + TIME_TOLERANCE_SECONDS
        {
            blockers.push(format!("event {} has invalid time chronology or evidence availability", event.event_id));
        }
        if event.observed_at_seconds + TIME_TOLERANCE_SECONDS < previous_event_at {
            blockers.push(format!("event {} is out of chronological order", event.event_id));
        }
        previous_event_at = event.observed_at_seconds;

        let Some(epoch) = epochs.get(event.epoch_id.as_str()) else {
            blockers.push(format!("event {} references an unknown exposure epoch", event.event_id));
            continue;
        };
        if event.observed_at_seconds
            < epoch.started_at_seconds - TIME_TOLERANCE_SECONDS
            || event.observed_at_seconds
                > epoch.ended_at_seconds + TIME_TOLERANCE_SECONDS
            || event.counter_value < epoch.counter_value_start - EXPOSURE_TOLERANCE
            || event.counter_value > epoch.counter_value_end + EXPOSURE_TOLERANCE
        {
            blockers.push(format!("event {} is outside its exposure epoch bounds", event.event_id));
        }
        let exposure = epoch.cumulative_exposure_at(event.counter_value);
        if exposure + EXPOSURE_TOLERANCE < previous_exposure {
            blockers.push(format!("event {} decreases cumulative operating exposure", event.event_id));
        }
        previous_exposure = exposure;

        if event.acceptance_envelope_sha256 != profile.acceptance_envelope_sha256 {
            let review_valid = event
                .comparability_review_sha256
                .as_deref()
                .is_some_and(is_sha256_hex)
                && event
                    .comparability_reviewer_ref
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty());
            if !review_valid {
                blockers.push(format!(
                    "event {} changes acceptance-envelope lineage without a comparability review",
                    event.event_id
                ));
            }
        }
        if event.event_type == DurabilityEventType::TargetCapabilityRecurrence {
            if event.component_id != profile.component_id
                || event.capability_id != profile.capability_id
            {
                blockers.push(format!(
                    "event {} claims target recurrence for a different component or capability",
                    event.event_id
                ));
            }
            if event.acceptance_envelope_sha256 == profile.acceptance_envelope_sha256
                && event.comparability_review_sha256.is_some()
            {
                // An extra review is allowed, but still must be a valid receipt.
                if !event
                    .comparability_review_sha256
                    .as_deref()
                    .is_some_and(is_sha256_hex)
                    || !event
                        .comparability_reviewer_ref
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty())
                {
                    blockers.push(format!("event {} has malformed envelope review metadata", event.event_id));
                }
            }
            if event.measurement_reference_path_id.as_deref().is_none_or(|value| value.trim().is_empty())
                || event.measurement_quality != EvidenceQuality::Valid
                || !valid_calibration_interval(
                    event.calibration_status,
                    event.calibration_record_sha256.as_deref(),
                    event.calibration_valid_from_seconds,
                    event.calibration_valid_until_seconds,
                    event.observed_at_seconds,
                    event.evidence_available_at_seconds,
                )
            {
                blockers.push(format!(
                    "target recurrence {} lacks a valid calibrated physical reference measurement",
                    event.event_id
                ));
            }
            if let Some(previous_id) = event.preceding_event_id.as_deref() {
                if !target_recurrence_ids.iter().any(|id| id == previous_id) {
                    blockers.push(format!(
                        "recurrence {} does not link to a preceding target recurrence",
                        event.event_id
                    ));
                }
            } else if !target_recurrence_ids.is_empty() {
                blockers.push(format!(
                    "recurrence {} omits its preceding recurrence link",
                    event.event_id
                ));
            }
            target_recurrence_ids.push(event.event_id.clone());
        } else if event.preceding_event_id.as_deref().is_some_and(|previous_id| {
            !event_ids.contains(previous_id)
        }) {
            // The full predecessor ordering is checked below after all event IDs are known.
            blockers.push(format!("event {} has an unknown predecessor", event.event_id));
        }
    }

    let all_ids = case
        .events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect::<BTreeSet<_>>();
    for event in &case.events {
        if let Some(previous_id) = event.preceding_event_id.as_deref() {
            let Some(previous) = case.events.iter().find(|candidate| candidate.event_id == previous_id) else {
                blockers.push(format!("event {} references a missing predecessor", event.event_id));
                continue;
            };
            if previous.observed_at_seconds >= event.observed_at_seconds
                || previous.asset_id != event.asset_id
            {
                blockers.push(format!("event {} has an invalid predecessor chronology", event.event_id));
            }
        }
    }
    let _ = all_ids;
}

fn validate_followup_end(case: &DurabilityCase, blockers: &mut Vec<String>) -> (f64, DurabilityDisposition) {
    let end = &case.followup_end;
    let profile = &case.profile;
    if !finite_non_negative(end.ended_at_seconds)
        || !finite_non_negative(end.counter_value)
        || !is_sha256_hex(&end.evidence_sha256)
        || end.reason_ref.trim().is_empty()
        || end.ended_at_seconds < profile.followup_started_at_seconds
        || end.ended_at_seconds > profile.analysis_as_of_seconds + TIME_TOLERANCE_SECONDS
    {
        blockers.push("follow-up end has invalid time or evidence metadata".into());
        return (0.0, DurabilityDisposition::NotEvaluable);
    }
    let epochs = epoch_map(case);
    let Some(epoch) = epochs.get(end.epoch_id.as_str()) else {
        blockers.push("follow-up end references an unknown exposure epoch".into());
        return (0.0, DurabilityDisposition::NotEvaluable);
    };
    if end.ended_at_seconds < epoch.started_at_seconds - TIME_TOLERANCE_SECONDS
        || end.ended_at_seconds > epoch.ended_at_seconds + TIME_TOLERANCE_SECONDS
        || end.counter_value < epoch.counter_value_start - EXPOSURE_TOLERANCE
        || end.counter_value > epoch.counter_value_end + EXPOSURE_TOLERANCE
    {
        blockers.push("follow-up end lies outside its exposure epoch".into());
        return (0.0, DurabilityDisposition::NotEvaluable);
    }
    let exposure = epoch.cumulative_exposure_at(end.counter_value);
    let linked_event = end
        .linked_event_id
        .as_deref()
        .and_then(|id| case.events.iter().find(|event| event.event_id == id));

    let disposition = match end.kind {
        FollowupEndKind::ObservedEvent => match linked_event {
            Some(event) if event.event_type == DurabilityEventType::TargetCapabilityRecurrence => {
                DurabilityDisposition::TargetRecurrenceObserved
            }
            Some(_) => DurabilityDisposition::OtherEventObserved,
            None => {
                blockers.push("observed-event follow-up end does not link to a known event".into());
                DurabilityDisposition::NotEvaluable
            }
        },
        FollowupEndKind::CompetingEvent => match linked_event {
            Some(event) if event.event_type == DurabilityEventType::AssetCompetingEvent => {
                DurabilityDisposition::CompetingEvent
            }
            _ => {
                blockers.push("competing-event end must link to an asset competing event".into());
                DurabilityDisposition::NotEvaluable
            }
        },
        FollowupEndKind::Censoring => {
            if end.linked_event_id.is_some() {
                blockers.push("censoring end must not be linked to a physical event".into());
                DurabilityDisposition::NotEvaluable
            } else if exposure + EXPOSURE_TOLERANCE >= profile.target_horizon_exposure {
                DurabilityDisposition::CompleteThroughHorizon
            } else {
                DurabilityDisposition::CensoredBeforeHorizon
            }
        }
        FollowupEndKind::AdministrativeEnd => {
            if end.linked_event_id.is_some() {
                blockers.push("administrative end must not be linked to a physical event".into());
                DurabilityDisposition::NotEvaluable
            } else if exposure + EXPOSURE_TOLERANCE >= profile.target_horizon_exposure {
                DurabilityDisposition::CompleteThroughHorizon
            } else {
                DurabilityDisposition::AdministrativeEndBeforeHorizon
            }
        }
    };
    if let Some(event) = linked_event {
        if event.observed_at_seconds > end.ended_at_seconds + TIME_TOLERANCE_SECONDS
            || event.epoch_id != end.epoch_id
            || (event.counter_value - end.counter_value).abs() > EXPOSURE_TOLERANCE
        {
            blockers.push("follow-up terminal event does not match the end timestamp/counter".into());
        }
    }
    if let Some(maximum_calendar) = profile.maximum_calendar_followup_seconds {
        if end.ended_at_seconds - profile.followup_started_at_seconds
            > maximum_calendar + TIME_TOLERANCE_SECONDS
        {
            blockers.push("follow-up exceeds its declared maximum calendar horizon".into());
        }
    }
    (exposure, disposition)
}

/// Validate one post-restoration follow-up record. No survival model or causal effect
/// is computed; the report describes only the evidence and exposure actually observed.
pub fn evaluate_durability_case(case: &DurabilityCase) -> DurabilityReport {
    let mut blockers = Vec::new();
    validate_profile(&case.profile, &mut blockers);
    validate_epochs(case, &mut blockers);
    validate_events(case, &mut blockers);
    let (observed_exposure_through, mut disposition) =
        validate_followup_end(case, &mut blockers);

    let mut target_recurrence_count = 0;
    let mut repeat_corrective_repair_count = 0;
    let mut new_component_failure_count = 0;
    let mut preventive_replacement_count = 0;
    let mut sensor_or_measurement_change_count = 0;
    let mut controller_or_configuration_change_count = 0;
    let mut competing_event_count = 0;
    for event in &case.events {
        match event.event_type {
            DurabilityEventType::TargetCapabilityRecurrence => target_recurrence_count += 1,
            DurabilityEventType::RepeatCorrectiveRepair => repeat_corrective_repair_count += 1,
            DurabilityEventType::NewComponentFailure => new_component_failure_count += 1,
            DurabilityEventType::PreventiveReplacement => preventive_replacement_count += 1,
            DurabilityEventType::SensorOrMeasurementChange => sensor_or_measurement_change_count += 1,
            DurabilityEventType::ControllerOrConfigurationChange => {
                controller_or_configuration_change_count += 1
            }
            DurabilityEventType::AssetCompetingEvent => competing_event_count += 1,
        }
    }
    if !blockers.is_empty() {
        disposition = DurabilityDisposition::NotEvaluable;
    }
    DurabilityReport {
        schema_version: DURABILITY_SCHEMA.to_owned(),
        case_id: case.profile.case_id.clone(),
        disposition,
        observed_exposure_through,
        target_horizon_exposure: case.profile.target_horizon_exposure,
        target_recurrence_count,
        repeat_corrective_repair_count,
        new_component_failure_count,
        preventive_replacement_count,
        sensor_or_measurement_change_count,
        controller_or_configuration_change_count,
        competing_event_count,
        blockers,
        future_safe_life_estimated: false,
        survival_or_hazard_estimated: false,
        causal_repair_effect_estimated: false,
        maintenance_authorized: false,
        physical_restoration_authorized: false,
        operating_restriction_clearance_authorized: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn base_case() -> DurabilityCase {
        DurabilityCase {
            profile: DurabilityProfile {
                schema_version: DURABILITY_SCHEMA.into(),
                case_id: "durability-synthetic-001".into(),
                asset_id: "engine-001".into(),
                component_id: "fuel-pump-001".into(),
                capability_id: "propulsion-delivery-pressure".into(),
                intervention_id: "repair-001".into(),
                intervention_record_sha256: digest('a'),
                intervention_completed_at_seconds: 100.0,
                restoration_verified_at_seconds: 180.0,
                restoration_report_sha256: digest('b'),
                acceptance_envelope_owner_ref: "OEM-envelope-rev-7".into(),
                acceptance_envelope_sha256: digest('c'),
                followup_started_at_seconds: 200.0,
                analysis_as_of_seconds: 700.0,
                exposure_clock: ExposureClock::EngineHours,
                exposure_unit: "engine_hours".into(),
                target_horizon_exposure: 100.0,
                maximum_calendar_followup_seconds: Some(10_000.0),
            },
            exposure_epochs: vec![ExposureEpoch {
                epoch_id: "epoch-0".into(),
                counter_source_id: "engine-hour-meter-001".into(),
                clock: ExposureClock::EngineHours,
                unit: "engine_hours".into(),
                started_at_seconds: 200.0,
                ended_at_seconds: 600.0,
                counter_value_start: 5_000.0,
                counter_value_end: 5_150.0,
                cumulative_exposure_at_start: 0.0,
                reset_from_epoch_id: None,
                reset_receipt_sha256: None,
                epoch_record_sha256: digest('d'),
                evidence_available_at_seconds: 610.0,
                quality: EvidenceQuality::Valid,
                calibration_status: CalibrationStatus::Verified,
                calibration_record_sha256: digest('e'),
                calibration_valid_from_seconds: 100.0,
                calibration_valid_until_seconds: 1_000.0,
            }],
            events: Vec::new(),
            followup_end: FollowupEnd {
                kind: FollowupEndKind::AdministrativeEnd,
                ended_at_seconds: 500.0,
                epoch_id: "epoch-0".into(),
                counter_value: 5_100.0,
                evidence_sha256: digest('f'),
                reason_ref: "study-horizon".into(),
                linked_event_id: None,
            },
        }
    }

    fn target_recurrence_event() -> DurabilityEvent {
        DurabilityEvent {
            event_id: "event-recurrence-001".into(),
            event_record_sha256: digest('1'),
            event_type: DurabilityEventType::TargetCapabilityRecurrence,
            asset_id: "engine-001".into(),
            component_id: "fuel-pump-001".into(),
            capability_id: "propulsion-delivery-pressure".into(),
            intervention_id: "repair-001".into(),
            restoration_report_sha256: digest('b'),
            acceptance_envelope_sha256: digest('c'),
            comparability_review_sha256: None,
            comparability_reviewer_ref: None,
            epoch_id: "epoch-0".into(),
            counter_value: 5_040.0,
            observed_at_seconds: 300.0,
            evidence_available_at_seconds: 310.0,
            evidence_sha256: digest('2'),
            adjudication_rule_sha256: digest('3'),
            reviewer_ref: "marine-reviewer-001".into(),
            source_id: "reference-daq-001".into(),
            measurement_reference_path_id: Some("independent-shaft-torque-path".into()),
            measurement_quality: EvidenceQuality::Valid,
            calibration_status: CalibrationStatus::Verified,
            calibration_record_sha256: Some(digest('4')),
            calibration_valid_from_seconds: Some(250.0),
            calibration_valid_until_seconds: Some(500.0),
            preceding_event_id: None,
        }
    }

    #[test]
    fn complete_exposure_followup_is_qualified_only_through_observed_horizon() {
        let report = evaluate_durability_case(&base_case());
        assert_eq!(report.disposition, DurabilityDisposition::CompleteThroughHorizon);
        assert_eq!(report.observed_exposure_through, 100.0);
        assert!(!report.future_safe_life_estimated);
        assert!(!report.survival_or_hazard_estimated);
    }

    #[test]
    fn target_recurrence_requires_calibrated_reference_evidence_and_adjudication() {
        let mut case = base_case();
        case.events.push(target_recurrence_event());
        case.followup_end.kind = FollowupEndKind::ObservedEvent;
        case.followup_end.ended_at_seconds = 300.0;
        case.followup_end.counter_value = 5_040.0;
        case.followup_end.linked_event_id = Some("event-recurrence-001".into());
        let report = evaluate_durability_case(&case);
        assert_eq!(report.disposition, DurabilityDisposition::TargetRecurrenceObserved);
        assert_eq!(report.target_recurrence_count, 1);
    }

    #[test]
    fn sensor_only_change_is_not_counted_as_target_recurrence() {
        let mut case = base_case();
        case.events.push(DurabilityEvent {
            event_type: DurabilityEventType::SensorOrMeasurementChange,
            measurement_reference_path_id: None,
            calibration_status: CalibrationStatus::Unknown,
            calibration_record_sha256: None,
            calibration_valid_from_seconds: None,
            calibration_valid_until_seconds: None,
            ..target_recurrence_event()
        });
        case.followup_end.kind = FollowupEndKind::ObservedEvent;
        case.followup_end.ended_at_seconds = 300.0;
        case.followup_end.counter_value = 5_040.0;
        case.followup_end.linked_event_id = Some("event-recurrence-001".into());
        let report = evaluate_durability_case(&case);
        assert_eq!(report.disposition, DurabilityDisposition::OtherEventObserved);
        assert_eq!(report.target_recurrence_count, 0);
        assert_eq!(report.sensor_or_measurement_change_count, 1);
    }

    #[test]
    fn recurrence_requires_valid_calibration_interval() {
        let mut case = base_case();
        let mut event = target_recurrence_event();
        event.calibration_valid_from_seconds = Some(305.0);
        case.events.push(event);
        case.followup_end.kind = FollowupEndKind::ObservedEvent;
        case.followup_end.ended_at_seconds = 300.0;
        case.followup_end.counter_value = 5_040.0;
        case.followup_end.linked_event_id = Some("event-recurrence-001".into());
        let report = evaluate_durability_case(&case);
        assert_eq!(report.disposition, DurabilityDisposition::NotEvaluable);
        assert!(report.blockers.iter().any(|item| item.contains("calibrated physical reference")));
    }

    #[test]
    fn changed_acceptance_envelope_requires_comparability_review() {
        let mut case = base_case();
        let mut event = target_recurrence_event();
        event.acceptance_envelope_sha256 = digest('5');
        case.events.push(event);
        case.followup_end.kind = FollowupEndKind::ObservedEvent;
        case.followup_end.ended_at_seconds = 300.0;
        case.followup_end.counter_value = 5_040.0;
        case.followup_end.linked_event_id = Some("event-recurrence-001".into());
        let report = evaluate_durability_case(&case);
        assert!(report.blockers.iter().any(|item| item.contains("comparability review")));
    }

    #[test]
    fn envelope_change_with_receipt_can_be_adjudicated() {
        let mut case = base_case();
        let mut event = target_recurrence_event();
        event.acceptance_envelope_sha256 = digest('5');
        event.comparability_review_sha256 = Some(digest('6'));
        event.comparability_reviewer_ref = Some("envelope-comparability-reviewer".into());
        case.events.push(event);
        case.followup_end.kind = FollowupEndKind::ObservedEvent;
        case.followup_end.ended_at_seconds = 300.0;
        case.followup_end.counter_value = 5_040.0;
        case.followup_end.linked_event_id = Some("event-recurrence-001".into());
        let report = evaluate_durability_case(&case);
        assert_eq!(report.disposition, DurabilityDisposition::TargetRecurrenceObserved);
    }

    #[test]
    fn missing_reset_receipt_invalidates_counter_epoch_transition() {
        let mut case = base_case();
        let mut second = case.exposure_epochs[0].clone();
        second.epoch_id = "epoch-1".into();
        second.started_at_seconds = 600.0;
        second.ended_at_seconds = 800.0;
        second.counter_value_start = 0.0;
        second.counter_value_end = 100.0;
        second.cumulative_exposure_at_start = 150.0;
        second.reset_from_epoch_id = Some("epoch-0".into());
        second.reset_receipt_sha256 = None;
        second.epoch_record_sha256 = digest('7');
        second.evidence_available_at_seconds = 810.0;
        case.exposure_epochs.push(second);
        let report = evaluate_durability_case(&case);
        assert!(report.blockers.iter().any(|item| item.contains("missing a linked reset receipt")));
    }

    #[test]
    fn linked_counter_reset_must_preserve_cumulative_exposure() {
        let mut case = base_case();
        let mut second = case.exposure_epochs[0].clone();
        second.epoch_id = "epoch-1".into();
        second.started_at_seconds = 600.0;
        second.ended_at_seconds = 800.0;
        second.counter_value_start = 0.0;
        second.counter_value_end = 100.0;
        second.cumulative_exposure_at_start = 999.0;
        second.reset_from_epoch_id = Some("epoch-0".into());
        second.reset_receipt_sha256 = Some(digest('8'));
        second.epoch_record_sha256 = digest('9');
        second.evidence_available_at_seconds = 810.0;
        case.exposure_epochs.push(second);
        let report = evaluate_durability_case(&case);
        assert!(report.blockers.iter().any(|item| item.contains("does not continue cumulative exposure")));
    }

    #[test]
    fn decreasing_counter_without_new_epoch_is_rejected() {
        let mut case = base_case();
        case.exposure_epochs[0].counter_value_end = 4_900.0;
        let report = evaluate_durability_case(&case);
        assert!(report.blockers.iter().any(|item| item.contains("counter values")));
    }

    #[test]
    fn event_before_intervention_completion_is_rejected() {
        let mut case = base_case();
        let mut event = target_recurrence_event();
        event.observed_at_seconds = 90.0;
        event.evidence_available_at_seconds = 95.0;
        event.counter_value = 5_010.0;
        case.events.push(event);
        let report = evaluate_durability_case(&case);
        assert!(report.blockers.iter().any(|item| item.contains("time chronology")));
    }

    #[test]
    fn future_event_evidence_is_not_used_before_analysis_as_of() {
        let mut case = base_case();
        let mut event = target_recurrence_event();
        event.evidence_available_at_seconds = 701.0;
        case.events.push(event);
        let report = evaluate_durability_case(&case);
        assert!(report.blockers.iter().any(|item| item.contains("time chronology")));
    }

    #[test]
    fn censoring_before_target_horizon_is_not_a_durability_pass() {
        let mut case = base_case();
        case.followup_end.kind = FollowupEndKind::Censoring;
        case.followup_end.ended_at_seconds = 400.0;
        case.followup_end.counter_value = 5_050.0;
        case.followup_end.reason_ref = "telemetry-lost".into();
        let report = evaluate_durability_case(&case);
        assert_eq!(report.disposition, DurabilityDisposition::CensoredBeforeHorizon);
    }

    #[test]
    fn competing_event_is_not_misclassified_as_censoring_or_success() {
        let mut case = base_case();
        let mut event = target_recurrence_event();
        event.event_type = DurabilityEventType::AssetCompetingEvent;
        event.component_id = "vessel-asset".into();
        event.capability_id = "asset-in-service".into();
        event.measurement_reference_path_id = None;
        event.calibration_status = CalibrationStatus::Unknown;
        event.calibration_record_sha256 = None;
        event.calibration_valid_from_seconds = None;
        event.calibration_valid_until_seconds = None;
        case.events.push(event);
        case.followup_end.kind = FollowupEndKind::CompetingEvent;
        case.followup_end.ended_at_seconds = 300.0;
        case.followup_end.counter_value = 5_040.0;
        case.followup_end.linked_event_id = Some("event-recurrence-001".into());
        let report = evaluate_durability_case(&case);
        assert_eq!(report.disposition, DurabilityDisposition::CompetingEvent);
        assert_eq!(report.competing_event_count, 1);
    }

    #[test]
    fn premature_administrative_end_is_incomplete() {
        let mut case = base_case();
        case.followup_end.ended_at_seconds = 400.0;
        case.followup_end.counter_value = 5_050.0;
        let report = evaluate_durability_case(&case);
        assert_eq!(
            report.disposition,
            DurabilityDisposition::AdministrativeEndBeforeHorizon
        );
    }

    #[test]
    fn repeated_target_recurrence_requires_predecessor_chain() {
        let mut case = base_case();
        let first = target_recurrence_event();
        let mut second = target_recurrence_event();
        second.event_id = "event-recurrence-002".into();
        second.event_record_sha256 = digest('a');
        second.observed_at_seconds = 350.0;
        second.evidence_available_at_seconds = 360.0;
        second.counter_value = 5_060.0;
        second.preceding_event_id = None;
        case.events.extend([first, second]);
        case.followup_end.kind = FollowupEndKind::ObservedEvent;
        case.followup_end.ended_at_seconds = 350.0;
        case.followup_end.counter_value = 5_060.0;
        case.followup_end.linked_event_id = Some("event-recurrence-002".into());
        let report = evaluate_durability_case(&case);
        assert!(report.blockers.iter().any(|item| item.contains("omits its preceding recurrence link")));
    }

    #[test]
    fn terminal_event_must_match_followup_end_link() {
        let mut case = base_case();
        case.events.push(target_recurrence_event());
        case.followup_end.kind = FollowupEndKind::ObservedEvent;
        case.followup_end.linked_event_id = Some("unknown-event".into());
        let report = evaluate_durability_case(&case);
        assert!(report.blockers.iter().any(|item| item.contains("does not link to a known event")));
    }

    #[test]
    fn report_never_claims_survival_causal_effect_or_authority() {
        let report = evaluate_durability_case(&base_case());
        assert!(!report.future_safe_life_estimated);
        assert!(!report.survival_or_hazard_estimated);
        assert!(!report.causal_repair_effect_estimated);
        assert!(!report.maintenance_authorized);
        assert!(!report.physical_restoration_authorized);
        assert!(!report.operating_restriction_clearance_authorized);
    }

    #[test]
    fn profile_requires_explicit_owner_and_evidence_digests() {
        let mut case = base_case();
        case.profile.acceptance_envelope_owner_ref.clear();
        case.profile.intervention_record_sha256 = "bad".into();
        let report = evaluate_durability_case(&case);
        assert!(report.blockers.iter().any(|item| item.contains("acceptance-envelope owner")));
        assert!(report.blockers.iter().any(|item| item.contains("intervention record digest")));
    }
}
