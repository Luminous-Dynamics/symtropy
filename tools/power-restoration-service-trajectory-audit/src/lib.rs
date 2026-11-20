use power_restoration_benchmark_case::{
    BenchmarkCase, canonical_case_sha256, validate_case,
};
use power_restoration_run_receipt::{
    RunReceipt, RunStatus, ServiceOutcome, StepDisposition, qualify as qualify_run_receipt,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const FLOAT_TOLERANCE: f64 = 1e-9;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStreamKind {
    PlannedForecast,
    ObservedWork,
    ModelledService,
    MeasuredService,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceSampleStatus {
    Known,
    Unavailable,
    Unknown,
    Conflicting,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkMilestoneKind {
    InspectionStarted,
    InspectionCompleted,
    RepairStarted,
    RepairCompleted,
    CommissioningStarted,
    CommissioningPassed,
    CommissioningFailed,
    EnergizationObserved,
    Other,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceTimebase {
    pub timezone_id: String,
    /// Unix epoch milliseconds; this is the unambiguous time authority.
    pub first_interval_start_unix_ms: u64,
    pub interval_duration_seconds: u32,
    pub horizon_steps: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceProducer {
    pub producer_id: String,
    pub source_commit: String,
    pub configuration_sha256: String,
    /// Required for modelled service; identifies the qualified domain model.
    pub model_id: Option<String>,
    /// Required for measured service; identifies the metering system.
    pub instrument_id: Option<String>,
    /// Required for measured service; calibration evidence is not inferred.
    pub calibration_artifact_ref: Option<String>,
    pub calibration_artifact_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceSourceArtifact {
    pub artifact_ref: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceSample {
    pub sample_id: String,
    pub load_id: String,
    pub step: u32,
    pub interval_start_unix_ms: u64,
    /// Offset is recorded per sample so UTC epoch time remains unambiguous
    /// across daylight-saving transitions.
    pub utc_offset_minutes: i16,
    pub demand_kwh: f64,
    pub delivered_kwh: Option<f64>,
    pub status: ServiceSampleStatus,
    pub evidence_ref: Option<String>,
    pub evidence_sha256: Option<String>,
    pub uncertainty_kwh: Option<f64>,
    pub conflict_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkMilestone {
    pub milestone_id: String,
    pub kind: WorkMilestoneKind,
    pub asset_id: String,
    pub step: u32,
    pub observed_at_unix_ms: u64,
    pub evidence_ref: String,
    pub evidence_sha256: String,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceTrajectoryRecord {
    pub schema_version: String,
    pub trajectory_id: String,
    pub artifact_uri: String,
    pub run_id: String,
    pub case_ref: String,
    pub case_sha256: String,
    pub stream_kind: ServiceStreamKind,
    pub service_id: String,
    pub service_type: String,
    pub service_boundary_id: String,
    /// v0.1 is electricity-only: native energy is kWh per evaluation interval.
    pub native_unit: String,
    pub timebase: ServiceTimebase,
    pub producer: ServiceProducer,
    pub source_artifacts: Vec<ServiceSourceArtifact>,
    pub samples: Vec<ServiceSample>,
    pub work_milestones: Vec<WorkMilestone>,
    /// Per-step service artifacts from the run receipt that this aggregate
    /// trajectory reconciles. This list is provenance only, not service values.
    pub parent_service_artifacts: Vec<ServiceSourceArtifact>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoadServiceSummary {
    pub load_id: String,
    pub essential_service_class: String,
    pub demand_kwh: f64,
    pub delivered_kwh: Option<f64>,
    pub unserved_kwh: Option<f64>,
    pub service_fraction_by_step: Option<Vec<f64>>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceTrajectoryAuditReport {
    pub trajectory_id: String,
    pub trajectory_sha256: String,
    pub run_id: String,
    pub case_sha256: String,
    pub stream_kind: ServiceStreamKind,
    pub interval_duration_seconds: u32,
    pub horizon_steps: u32,
    pub sample_count: usize,
    pub known_sample_count: usize,
    pub unavailable_sample_count: usize,
    pub unknown_sample_count: usize,
    pub conflicting_sample_count: usize,
    pub work_milestone_count: usize,
    pub total_demand_kwh: Option<f64>,
    pub total_delivered_kwh: Option<f64>,
    pub total_unserved_kwh: Option<f64>,
    pub weighted_delivered_kwh: Option<f64>,
    pub weighted_unserved_kwh: Option<f64>,
    pub aggregate_score_match: Option<bool>,
    pub load_summaries: Vec<LoadServiceSummary>,
    pub result_status: String,
    pub claim_boundary: String,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn nearly_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= FLOAT_TOLERANCE * left.abs().max(right.abs()).max(1.0)
}

pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    let canonical_value = serde_json::to_value(value)
        .map_err(|error| format!("JSON serialization failed: {error}"))?;
    let bytes = serde_json::to_vec(&canonical_value)
        .map_err(|error| format!("canonical JSON serialization failed: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn case_interval_duration_ms(case: &BenchmarkCase) -> Result<u64, String> {
    let unit_ms = match case.timebase.unit.as_str() {
        "second" | "seconds" | "s" => 1_000_u64,
        "minute" | "minutes" | "min" => 60_000_u64,
        "hour" | "hours" | "h" => 3_600_000_u64,
        other => return Err(format!("unsupported case timebase unit {other}")),
    };
    unit_ms
        .checked_mul(u64::from(case.timebase.step_size))
        .ok_or_else(|| "case timebase interval overflowed".to_string())
}

fn expected_demand_kwh(active_power_kw: f64, interval_duration_seconds: u32) -> f64 {
    active_power_kw * f64::from(interval_duration_seconds) / 3_600.0
}

fn validate_producer(
    trajectory: &ServiceTrajectoryRecord,
    errors: &mut Vec<String>,
) {
    let producer = &trajectory.producer;
    if !valid_id(&producer.producer_id)
        || !is_lower_hex(&producer.source_commit, 40)
        || !is_lower_hex(&producer.configuration_sha256, 64)
    {
        errors.push("service producer identity, source commit or configuration digest is invalid".to_string());
    }

    match trajectory.stream_kind {
        ServiceStreamKind::ModelledService => {
            if producer.model_id.as_deref().is_none_or(|id| !valid_id(id)) {
                errors.push("modelled service requires an explicit model ID".to_string());
            }
            if producer.instrument_id.is_some()
                || producer.calibration_artifact_ref.is_some()
                || producer.calibration_artifact_sha256.is_some()
            {
                errors.push("modelled service must not claim a metering instrument/calibration identity".to_string());
            }
        }
        ServiceStreamKind::MeasuredService => {
            if producer.instrument_id.as_deref().is_none_or(|id| !valid_id(id))
                || producer.calibration_artifact_ref.as_deref().is_none_or(|id| !valid_id(id))
                || producer.calibration_artifact_sha256.as_deref().is_none_or(|hash| !is_lower_hex(hash, 64))
            {
                errors.push("measured service requires an instrument ID and calibration artifact/digest".to_string());
            }
            if producer.model_id.is_some() {
                errors.push("measured service must not substitute a model ID for metering identity".to_string());
            }
        }
        ServiceStreamKind::PlannedForecast | ServiceStreamKind::ObservedWork => {}
    }
}

fn validate_sample_evidence(sample: &ServiceSample, errors: &mut Vec<String>) {
    let evidence_pair_valid = match (
        sample.evidence_ref.as_deref(),
        sample.evidence_sha256.as_deref(),
    ) {
        (Some(reference), Some(digest)) => valid_id(reference) && is_lower_hex(digest, 64),
        (None, None) => false,
        _ => false,
    };

    match sample.status {
        ServiceSampleStatus::Known | ServiceSampleStatus::Unavailable => {
            if !evidence_pair_valid {
                errors.push(format!(
                    "known/unavailable sample {} lacks a complete source evidence reference/digest",
                    sample.sample_id
                ));
            }
        }
        ServiceSampleStatus::Unknown => {
            if sample.delivered_kwh.is_some() {
                errors.push(format!(
                    "unknown sample {} must not contain a numeric delivered-energy value",
                    sample.sample_id
                ));
            }
        }
        ServiceSampleStatus::Conflicting => {
            if sample.delivered_kwh.is_some() {
                errors.push(format!(
                    "conflicting sample {} must not contain an averaged/selected delivered-energy value",
                    sample.sample_id
                ));
            }
            if sample.conflict_reason.as_deref().is_none_or(|reason| !valid_id(reason)) {
                errors.push(format!(
                    "conflicting sample {} requires an explicit conflict reason",
                    sample.sample_id
                ));
            }
        }
    }

    if let Some(hash) = sample.evidence_sha256.as_deref() {
        if !is_lower_hex(hash, 64) {
            errors.push(format!("sample {} has an invalid evidence digest", sample.sample_id));
        }
    }
}

fn validate_work_milestones(
    trajectory: &ServiceTrajectoryRecord,
    case: &BenchmarkCase,
    errors: &mut Vec<String>,
) {
    let mut ids = BTreeSet::new();
    for milestone in &trajectory.work_milestones {
        if !valid_id(&milestone.milestone_id)
            || !ids.insert(milestone.milestone_id.as_str())
            || !valid_id(&milestone.asset_id)
            || !valid_id(&milestone.evidence_ref)
            || !is_lower_hex(&milestone.evidence_sha256, 64)
            || milestone.step >= case.timebase.horizon_steps
        {
            errors.push(format!("work milestone {} has invalid identity, evidence or time", milestone.milestone_id));
        }
    }
}

fn validate_parent_service_artifacts(
    trajectory: &ServiceTrajectoryRecord,
    receipt: &RunReceipt,
    errors: &mut Vec<String>,
) {
    let mut expected: Vec<(String, String)> = receipt
        .decision_steps
        .iter()
        .filter(|step| {
            step.disposition == StepDisposition::Executed
                && step.service_outcome == ServiceOutcome::Scored
        })
        .filter_map(|step| {
            step.service_trajectory.as_ref().map(|artifact| {
                (artifact.artifact_ref.clone(), artifact.sha256.clone())
            })
        })
        .collect();
    let mut actual: Vec<(String, String)> = trajectory
        .parent_service_artifacts
        .iter()
        .map(|artifact| (artifact.artifact_ref.clone(), artifact.sha256.clone()))
        .collect();

    expected.sort();
    actual.sort();
    if expected != actual {
        errors.push("aggregate service trajectory parent set does not exactly match per-step scored service artifacts".to_string());
    }
    let mut unique = BTreeSet::new();
    for (uri, digest) in actual {
        if !valid_id(&uri)
            || !is_lower_hex(&digest, 64)
            || !unique.insert(uri.clone())
        {
            errors.push(format!("aggregate service trajectory has invalid or duplicate parent artifact {uri}"));
        }
    }
}

fn validate_common(
    trajectory: &ServiceTrajectoryRecord,
    receipt: &RunReceipt,
    case: &BenchmarkCase,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let case_sha = canonical_case_sha256(case)
        .map_err(|error| vec![format!("case canonicalization failed: {error}")])?;

    if trajectory.schema_version != "0.1.0"
        || !valid_id(&trajectory.trajectory_id)
        || !valid_id(&trajectory.artifact_uri)
        || trajectory.run_id != receipt.run_id
        || trajectory.case_ref != receipt.case.case_ref
        || trajectory.case_sha256 != case_sha
        || trajectory.case_sha256 != receipt.case.case_sha256
        || !valid_id(&trajectory.service_id)
        || !valid_id(&trajectory.service_type)
        || !valid_id(&trajectory.service_boundary_id)
        || trajectory.native_unit != "kWh"
    {
        errors.push("service trajectory identity, case/run binding, service boundary or native unit is invalid".to_string());
    }

    if !valid_id(&trajectory.timebase.timezone_id)
        || trajectory.timebase.horizon_steps != case.timebase.horizon_steps
        || trajectory.timebase.interval_duration_seconds != case_interval_duration_ms(case).unwrap_or(0) as u32 / 1_000
    {
        errors.push("service trajectory timezone, horizon or interval does not match the frozen case timebase".to_string());
    }

    validate_producer(trajectory, &mut errors);
    if trajectory.source_artifacts.is_empty()
        && matches!(
            trajectory.stream_kind,
            ServiceStreamKind::ModelledService | ServiceStreamKind::MeasuredService
        )
    {
        errors.push("modelled/measured service requires at least one source artifact".to_string());
    }
    let mut source_uris = BTreeSet::new();
    for source in &trajectory.source_artifacts {
        if !valid_id(&source.artifact_ref)
            || !is_lower_hex(&source.sha256, 64)
            || !source_uris.insert(source.artifact_ref.as_str())
        {
            errors.push(format!("service source artifact {} is invalid or duplicated", source.artifact_ref));
        }
    }

    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok(())
    }
}

fn validate_stream_separation(
    trajectory: &ServiceTrajectoryRecord,
    case: &BenchmarkCase,
) -> Result<Option<String>, Vec<String>> {
    let mut errors = Vec::new();
    match trajectory.stream_kind {
        ServiceStreamKind::ObservedWork => {
            if !trajectory.samples.is_empty() {
                errors.push("observed_work stream must not contain delivered-service samples".to_string());
            }
            if trajectory.work_milestones.is_empty() {
                errors.push("observed_work stream requires at least one work milestone".to_string());
            }
            validate_work_milestones(trajectory, case, &mut errors);
            if errors.is_empty() {
                Ok(Some("observed_work_only_not_service".to_string()))
            } else {
                Err(errors)
            }
        }
        ServiceStreamKind::PlannedForecast => {
            if !trajectory.work_milestones.is_empty() {
                errors.push("planned_forecast stream must not contain observed work milestones".to_string());
            }
            if trajectory.samples.len()
                != case.network.loads.len().saturating_mul(case.timebase.horizon_steps as usize)
            {
                errors.push("planned_forecast must account for every load and horizon interval".to_string());
            }
            if errors.is_empty() {
                Ok(Some("forecast_only_not_realized_service".to_string()))
            } else {
                Err(errors)
            }
        }
        ServiceStreamKind::ModelledService | ServiceStreamKind::MeasuredService => {
            if !trajectory.work_milestones.is_empty() {
                errors.push("modelled/measured service stream must not carry work milestones".to_string());
            }
            if errors.is_empty() {
                Ok(None)
            } else {
                Err(errors)
            }
        }
    }
}

fn verify_no_forecast_work_as_final_service(
    trajectory: &ServiceTrajectoryRecord,
    receipt: &RunReceipt,
) -> Result<(), Vec<String>> {
    if matches!(
        trajectory.stream_kind,
        ServiceStreamKind::PlannedForecast | ServiceStreamKind::ObservedWork
    ) && receipt.final_service_trajectory.as_ref().is_some_and(|final_ref| {
        final_ref.artifact_ref == trajectory.artifact_uri
    }) {
        return Err(vec![
            "planned_forecast/observed_work artifact cannot satisfy final delivered-service trajectory reference".to_string()
        ]);
    }
    Ok(())
}

fn summarize_service(
    trajectory: &ServiceTrajectoryRecord,
    receipt: &RunReceipt,
    case: &BenchmarkCase,
) -> Result<ServiceTrajectoryAuditReport, Vec<String>> {
    let interval_duration_seconds = trajectory.timebase.interval_duration_seconds;
    let interval_duration_ms = u64::from(interval_duration_seconds)
        .checked_mul(1_000)
        .ok_or_else(|| vec!["service interval duration overflows milliseconds".to_string()])?;

    let mut errors = Vec::new();
    let mut sample_ids = BTreeSet::new();
    let mut sample_keys = BTreeSet::new();
    let loads: BTreeMap<&str, _> = case.network.loads.iter()
        .map(|load| (load.load_id.as_str(), load))
        .collect();
    let expected_count = case.network.loads.len()
        .checked_mul(case.timebase.horizon_steps as usize)
        .ok_or_else(|| vec!["expected load/interval sample count overflowed".to_string()])?;

    if trajectory.samples.len() != expected_count {
        errors.push(format!(
            "service trajectory must contain exactly {expected_count} load/interval samples; got {}",
            trajectory.samples.len()
        ));
    }

    let mut per_load_demand = BTreeMap::<String, f64>::new();
    let mut per_load_delivered = BTreeMap::<String, f64>::new();
    let mut per_load_fraction = BTreeMap::<String, Vec<f64>>::new();
    let mut known_count = 0;
    let mut unavailable_count = 0;
    let mut unknown_count = 0;
    let mut conflicting_count = 0;
    let mut total_demand = 0.0;
    let mut total_delivered = 0.0;
    let mut total_unserved = 0.0;
    let mut weighted_delivered = 0.0;
    let mut weighted_unserved = 0.0;

    for sample in &trajectory.samples {
        if !valid_id(&sample.sample_id) || !sample_ids.insert(sample.sample_id.as_str()) {
            errors.push(format!("sample {} has an empty or duplicate ID", sample.sample_id));
        }
        if sample.utc_offset_minutes.unsigned_abs() > 14 * 60 {
            errors.push(format!("sample {} has an invalid UTC offset", sample.sample_id));
        }
        if sample.step >= case.timebase.horizon_steps {
            errors.push(format!("sample {} is outside the case horizon", sample.sample_id));
            continue;
        }

        let Some(load) = loads.get(sample.load_id.as_str()) else {
            errors.push(format!("sample {} references unknown load {}", sample.sample_id, sample.load_id));
            continue;
        };
        if !sample_keys.insert((sample.load_id.as_str(), sample.step)) {
            errors.push(format!(
                "duplicate service sample for load {} at step {}",
                sample.load_id, sample.step
            ));
        }

        let expected_time = trajectory.timebase.first_interval_start_unix_ms
            .checked_add(u64::from(sample.step).saturating_mul(interval_duration_ms));
        if expected_time != Some(sample.interval_start_unix_ms) {
            errors.push(format!(
                "sample {} timestamp does not match its declared interval index",
                sample.sample_id
            ));
        }

        let expected_demand = expected_demand_kwh(load.active_power_kw, interval_duration_seconds);
        if !sample.demand_kwh.is_finite()
            || sample.demand_kwh < 0.0
            || !nearly_equal(sample.demand_kwh, expected_demand)
        {
            errors.push(format!(
                "sample {} demand denominator differs from frozen case load kW × interval hours",
                sample.sample_id
            ));
        }
        if sample.uncertainty_kwh.is_some_and(|uncertainty| {
            !uncertainty.is_finite() || uncertainty < 0.0
        }) {
            errors.push(format!("sample {} has invalid uncertainty", sample.sample_id));
        }
        validate_sample_evidence(sample, &mut errors);

        *per_load_demand.entry(sample.load_id.clone()).or_default() += sample.demand_kwh;
        match sample.status {
            ServiceSampleStatus::Known => {
                known_count += 1;
                let Some(delivered) = sample.delivered_kwh else {
                    errors.push(format!("known sample {} is missing delivered_kwh", sample.sample_id));
                    continue;
                };
                if !delivered.is_finite() || delivered <= 0.0 || delivered > sample.demand_kwh + FLOAT_TOLERANCE {
                    errors.push(format!("known sample {} has delivered_kwh outside (0, demand]", sample.sample_id));
                    continue;
                }
                let fraction = (delivered / sample.demand_kwh).clamp(0.0, 1.0);
                per_load_fraction.entry(sample.load_id.clone()).or_default().push(fraction);
                *per_load_delivered.entry(sample.load_id.clone()).or_default() += delivered;
                total_delivered += delivered;
                total_unserved += (sample.demand_kwh - delivered).max(0.0);
                weighted_delivered += delivered * load.priority_weight;
                weighted_unserved += (sample.demand_kwh - delivered).max(0.0) * load.priority_weight;
                total_demand += sample.demand_kwh;
            }
            ServiceSampleStatus::Unavailable => {
                unavailable_count += 1;
                if sample.delivered_kwh != Some(0.0) {
                    errors.push(format!("unavailable sample {} must have delivered_kwh=0", sample.sample_id));
                    continue;
                }
                per_load_fraction.entry(sample.load_id.clone()).or_default().push(0.0);
                per_load_delivered.entry(sample.load_id.clone()).or_default();
                total_demand += sample.demand_kwh;
                total_unserved += sample.demand_kwh;
                weighted_unserved += sample.demand_kwh * load.priority_weight;
            }
            ServiceSampleStatus::Unknown => unknown_count += 1,
            ServiceSampleStatus::Conflicting => conflicting_count += 1,
        }
    }

    if sample_keys.len() != expected_count {
        errors.push(format!(
            "load/interval mapping is incomplete: {} unique rows for {expected_count} expected pairs",
            sample_keys.len()
        ));
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let unknown_or_conflicting = unknown_count + conflicting_count;
    let complete = unknown_or_conflicting == 0;
    let aggregate_score_match = if complete {
        match (&receipt.aggregate_score, &receipt.final_service_trajectory) {
            (Some(score), Some(final_ref)) => {
                let score_matches = score.metric_id == case.objective.metric
                    && score.metric_id == receipt.objective.metric
                    && score.unit == receipt.objective.unit
                    && nearly_equal(score.value, weighted_delivered);
                let final_matches = final_ref.artifact_ref == trajectory.artifact_uri
                    && final_ref.sha256 == canonical_sha256(trajectory).unwrap_or_default();
                if !score_matches {
                    errors.push("aggregate scorer metric/unit/value does not equal independently recomputed weighted delivered kWh".to_string());
                }
                if !final_matches {
                    errors.push("run receipt final service trajectory URI/digest does not match this canonical trajectory".to_string());
                }
                Some(score_matches && final_matches)
            }
            _ => {
                errors.push("completed service claim requires both aggregate score and final trajectory reference".to_string());
                Some(false)
            }
        }
    } else {
        if receipt.aggregate_score.is_some() {
            errors.push("run receipt publishes an aggregate score despite unknown/conflicting delivered-service intervals".to_string());
        }
        None
    };
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut load_summaries = Vec::new();
    for load in &case.network.loads {
        let demand = per_load_demand.get(&load.load_id).copied().unwrap_or(0.0);
        let delivered = if complete {
            Some(per_load_delivered.get(&load.load_id).copied().unwrap_or(0.0))
        } else {
            None
        };
        let unserved = delivered.map(|value| (demand - value).max(0.0));
        let fractions = if complete {
            per_load_fraction.get(&load.load_id).cloned()
        } else {
            None
        };
        load_summaries.push(LoadServiceSummary {
            load_id: load.load_id.clone(),
            essential_service_class: load.essential_service_class.clone(),
            demand_kwh: demand,
            delivered_kwh: delivered,
            unserved_kwh: unserved,
            service_fraction_by_step: fractions,
        });
    }

    let status = if !complete {
        "inconclusive_unknown_or_conflicting_service".to_string()
    } else if aggregate_score_match == Some(true) {
        match trajectory.stream_kind {
            ServiceStreamKind::ModelledService => "modelled_service_and_score_reconciled".to_string(),
            ServiceStreamKind::MeasuredService => "measured_service_and_score_reconciled".to_string(),
            _ => "invalid_service_stream".to_string(),
        }
    } else {
        "service_trajectory_recomputed_without_completed_score".to_string()
    };

    Ok(ServiceTrajectoryAuditReport {
        trajectory_id: trajectory.trajectory_id.clone(),
        trajectory_sha256: canonical_sha256(trajectory)
            .map_err(|error| vec![format!("trajectory canonicalization failed: {error}")])?,
        run_id: receipt.run_id.clone(),
        case_sha256: receipt.case.case_sha256.clone(),
        stream_kind: trajectory.stream_kind,
        interval_duration_seconds,
        horizon_steps: case.timebase.horizon_steps,
        sample_count: trajectory.samples.len(),
        known_sample_count: known_count,
        unavailable_sample_count: unavailable_count,
        unknown_sample_count: unknown_count,
        conflicting_sample_count: conflicting_count,
        work_milestone_count: trajectory.work_milestones.len(),
        total_demand_kwh: complete.then_some(total_demand),
        total_delivered_kwh: complete.then_some(total_delivered),
        total_unserved_kwh: complete.then_some(total_unserved),
        weighted_delivered_kwh: complete.then_some(weighted_delivered),
        weighted_unserved_kwh: complete.then_some(weighted_unserved),
        aggregate_score_match,
        load_summaries,
        result_status: status,
        claim_boundary: "Source-bound electricity service trajectory and priority-weighted kWh reconciliation only. Modelled/measured provenance labels and signatures do not prove physical truth, metering authenticity, AC/three-phase feasibility, safe switching, protection, operator authority or field restoration. Planned forecasts and observed work are never delivered-service evidence.".to_string(),
    })
}

pub fn audit_service_trajectory(
    trajectory: &ServiceTrajectoryRecord,
    receipt: &RunReceipt,
    case: &BenchmarkCase,
) -> Result<ServiceTrajectoryAuditReport, Vec<String>> {
    validate_case(case);
    qualify_run_receipt(receipt, case)
        .map_err(|errors| errors.into_iter().map(|error| format!("run receipt: {error}")).collect::<Vec<_>>())?;
    validate_common(trajectory, receipt, case)?;
    verify_no_forecast_work_as_final_service(trajectory, receipt)?;

    if let Some(result_status) = validate_stream_separation(trajectory, case)? {
        return Ok(ServiceTrajectoryAuditReport {
            trajectory_id: trajectory.trajectory_id.clone(),
            trajectory_sha256: canonical_sha256(trajectory)
                .map_err(|error| vec![format!("trajectory canonicalization failed: {error}")])?,
            run_id: receipt.run_id.clone(),
            case_sha256: receipt.case.case_sha256.clone(),
            stream_kind: trajectory.stream_kind,
            interval_duration_seconds: trajectory.timebase.interval_duration_seconds,
            horizon_steps: case.timebase.horizon_steps,
            sample_count: trajectory.samples.len(),
            known_sample_count: 0,
            unavailable_sample_count: 0,
            unknown_sample_count: 0,
            conflicting_sample_count: 0,
            work_milestone_count: trajectory.work_milestones.len(),
            total_demand_kwh: None,
            total_delivered_kwh: None,
            total_unserved_kwh: None,
            weighted_delivered_kwh: None,
            weighted_unserved_kwh: None,
            aggregate_score_match: None,
            load_summaries: Vec::new(),
            result_status,
            claim_boundary: "Forecast and observed work streams are preserved as their own evidence classes and cannot establish delivered service.".to_string(),
        });
    }

    validate_parent_service_artifacts(trajectory, receipt, &mut Vec::new());
    summarize_service(trajectory, receipt, case)
}

pub fn per_load_cascade_availability(
    report: &ServiceTrajectoryAuditReport,
) -> Result<BTreeMap<String, Vec<f64>>, Vec<String>> {
    if !matches!(
        report.result_status.as_str(),
        "modelled_service_and_score_reconciled" | "measured_service_and_score_reconciled"
    ) {
        return Err(vec![
            "CIVDYN availability export requires a complete reconciled modelled/measured service trajectory".to_string()
        ]);
    }
    let mut result = BTreeMap::new();
    for load in &report.load_summaries {
        let Some(values) = load.service_fraction_by_step.as_ref() else {
            return Err(vec![format!("load {} has no complete service fraction series", load.load_id)]);
        };
        if values.len() != report.horizon_steps as usize
            || values.iter().any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(vec![format!("load {} availability series has invalid horizon or values", load.load_id)]);
        }
        result.insert(load.load_id.clone(), values.clone());
    }
    Ok(result)
}

pub fn parse_and_audit(
    trajectory_json: &str,
    run_receipt_json: &str,
    case_json: &str,
) -> Result<ServiceTrajectoryAuditReport, Vec<String>> {
    let trajectory: ServiceTrajectoryRecord = serde_json::from_str(trajectory_json)
        .map_err(|error| vec![format!("service trajectory JSON decode error: {error}")])?;
    let receipt: RunReceipt = serde_json::from_str(run_receipt_json)
        .map_err(|error| vec![format!("run receipt JSON decode error: {error}")])?;
    let case: BenchmarkCase = serde_json::from_str(case_json)
        .map_err(|error| vec![format!("benchmark case JSON decode error: {error}")])?;
    audit_service_trajectory(&trajectory, &receipt, &case)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case() -> BenchmarkCase {
        serde_json::from_str(include_str!(
            "../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"
        )).unwrap()
    }

    fn receipt(case: &BenchmarkCase) -> RunReceipt {
        let mut receipt: RunReceipt = serde_json::from_str(include_str!(
            "../../power-restoration-run-receipt/fixtures/synthetic-restoration-run-receipt-v0.1.json"
        )).unwrap();
        receipt.case.case_sha256 = canonical_case_sha256(case).unwrap();
        receipt.case.case_ref = format!("power-restoration-case://{}", case.case_id);
        receipt
    }

    fn fully_served_trajectory(case: &BenchmarkCase, receipt: &RunReceipt) -> ServiceTrajectoryRecord {
        let interval_duration_seconds = case.timebase.step_size * 60;
        let interval_duration_ms = u64::from(interval_duration_seconds) * 1_000;
        let mut samples = Vec::new();
        for step in 0..case.timebase.horizon_steps {
            for load in &case.network.loads {
                let demand_kwh = expected_demand_kwh(load.active_power_kw, interval_duration_seconds);
                samples.push(ServiceSample {
                    sample_id: format!("sample-{}-{step}", load.load_id),
                    load_id: load.load_id.clone(),
                    step,
                    interval_start_unix_ms: 1_800_000_000_000 + u64::from(step) * interval_duration_ms,
                    utc_offset_minutes: 0,
                    demand_kwh,
                    delivered_kwh: Some(demand_kwh),
                    status: ServiceSampleStatus::Known,
                    evidence_ref: Some(format!("evidence://meter/{}/{}", load.load_id, step)),
                    evidence_sha256: Some("1".repeat(64)),
                    uncertainty_kwh: Some(0.01),
                    conflict_reason: None,
                });
            }
        }
        ServiceTrajectoryRecord {
            schema_version: "0.1.0".to_string(),
            trajectory_id: "service-trajectory-0001".to_string(),
            artifact_uri: "synthetic://final-service-trajectory".to_string(),
            run_id: receipt.run_id.clone(),
            case_ref: receipt.case.case_ref.clone(),
            case_sha256: receipt.case.case_sha256.clone(),
            stream_kind: ServiceStreamKind::MeasuredService,
            service_id: "electricity-service".to_string(),
            service_type: "electricity".to_string(),
            service_boundary_id: "load-terminal".to_string(),
            native_unit: "kWh".to_string(),
            timebase: ServiceTimebase {
                timezone_id: "UTC".to_string(),
                first_interval_start_unix_ms: 1_800_000_000_000,
                interval_duration_seconds,
                horizon_steps: case.timebase.horizon_steps,
            },
            producer: ServiceProducer {
                producer_id: "utility-metering-system".to_string(),
                source_commit: "1".repeat(40),
                configuration_sha256: "2".repeat(64),
                model_id: None,
                instrument_id: Some("meter-set-001".to_string()),
                calibration_artifact_ref: Some("calibration://meter-set-001".to_string()),
                calibration_artifact_sha256: Some("3".repeat(64)),
            },
            source_artifacts: vec![ServiceSourceArtifact {
                artifact_ref: "meter-capture://synthetic-run-0001".to_string(),
                sha256: "4".repeat(64),
            }],
            samples,
            work_milestones: Vec::new(),
            parent_service_artifacts: vec![ServiceSourceArtifact {
                artifact_ref: "synthetic://service-trajectory/step-0".to_string(),
                sha256: "b".repeat(64),
            }],
        }
    }

    fn bind_final_and_score(
        trajectory: &ServiceTrajectoryRecord,
        receipt: &mut RunReceipt,
        score: f64,
    ) {
        receipt.final_service_trajectory = Some(power_restoration_run_receipt::ArtifactRef {
            artifact_ref: trajectory.artifact_uri.clone(),
            sha256: canonical_sha256(trajectory).unwrap(),
        });
        receipt.objective.metric = "cumulative_time_weighted_priority_energy_restored".to_string();
        receipt.objective.unit = "weighted_kWh".to_string();
        receipt.aggregate_score = Some(power_restoration_run_receipt::AggregateScore {
            metric_id: "cumulative_time_weighted_priority_energy_restored".to_string(),
            unit: "weighted_kWh".to_string(),
            value: score,
            scorer_receipt: power_restoration_run_receipt::ArtifactRef {
                artifact_ref: "scorer://receipt".to_string(),
                sha256: "5".repeat(64),
            },
        });
    }

    #[test]
    fn measured_service_recomputes_weighted_kwh_and_exports_cascade_input() {
        let case = case();
        let mut receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        let expected = case.network.loads.iter().map(|load| {
            load.active_power_kw
                * f64::from(case.timebase.step_size) / 60.0
                * f64::from(case.timebase.horizon_steps)
                * load.priority_weight
        }).sum::<f64>();
        bind_final_and_score(&trajectory, &mut receipt, expected);
        trajectory.parent_service_artifacts = receipt.decision_steps.iter()
            .filter(|step| step.disposition == StepDisposition::Executed
                && step.service_outcome == ServiceOutcome::Scored)
            .filter_map(|step| step.service_trajectory.as_ref())
            .map(|artifact| ServiceSourceArtifact {
                artifact_ref: artifact.artifact_ref.clone(),
                sha256: artifact.sha256.clone(),
            }).collect();

        let report = audit_service_trajectory(&trajectory, &receipt, &case).unwrap();
        assert_eq!(report.result_status, "measured_service_and_score_reconciled");
        assert!(nearly_equal(report.weighted_delivered_kwh.unwrap(), expected));
        let cascade = per_load_cascade_availability(&report).unwrap();
        assert_eq!(cascade["L5"], vec![1.0; 24]);
    }

    #[test]
    fn planned_forecast_never_qualifies_as_delivered_service() {
        let case = case();
        let mut receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.stream_kind = ServiceStreamKind::PlannedForecast;
        trajectory.artifact_uri = "synthetic://forecast".to_string();
        let report = audit_service_trajectory(&trajectory, &receipt, &case).unwrap();
        assert_eq!(report.result_status, "forecast_only_not_realized_service");
        assert!(report.weighted_delivered_kwh.is_none());
        assert!(per_load_cascade_availability(&report).is_err());
    }

    #[test]
    fn observed_repair_completion_never_generates_service_samples() {
        let case = case();
        let receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.stream_kind = ServiceStreamKind::ObservedWork;
        trajectory.samples.clear();
        trajectory.parent_service_artifacts.clear();
        trajectory.work_milestones = vec![WorkMilestone {
            milestone_id: "repair-0001".to_string(),
            kind: WorkMilestoneKind::RepairCompleted,
            asset_id: "F23-A".to_string(),
            step: 4,
            observed_at_unix_ms: 1_800_001_200_000,
            evidence_ref: "field://repair-ticket/0001".to_string(),
            evidence_sha256: "6".repeat(64),
            notes: Some("crew reports work complete; commissioning not implied".to_string()),
        }];
        trajectory.artifact_uri = "synthetic://observed-work".to_string();
        let report = audit_service_trajectory(&trajectory, &receipt, &case).unwrap();
        assert_eq!(report.result_status, "observed_work_only_not_service");
        assert!(report.total_delivered_kwh.is_none());
    }

    #[test]
    fn unknown_interval_is_not_coerced_to_zero() {
        let case = case();
        let mut receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        let sample = &mut trajectory.samples[0];
        sample.status = ServiceSampleStatus::Unknown;
        sample.delivered_kwh = None;
        sample.evidence_ref = None;
        sample.evidence_sha256 = None;
        trajectory.parent_service_artifacts = receipt.decision_steps.iter()
            .filter(|step| step.disposition == StepDisposition::Executed
                && step.service_outcome == ServiceOutcome::Scored)
            .filter_map(|step| step.service_trajectory.as_ref())
            .map(|artifact| ServiceSourceArtifact {
                artifact_ref: artifact.artifact_ref.clone(),
                sha256: artifact.sha256.clone(),
            }).collect();
        // Unknown service cannot coexist with a published completed aggregate score.
        receipt.aggregate_score = None;
        bind_final_and_score(&trajectory, &mut receipt, 0.0);
        receipt.aggregate_score = None;
        let report = audit_service_trajectory(&trajectory, &receipt, &case).unwrap();
        assert_eq!(report.result_status, "inconclusive_unknown_or_conflicting_service");
        assert!(report.weighted_delivered_kwh.is_none());
        assert!(report.load_summaries[0].delivered_kwh.is_none());
    }

    #[test]
    fn conflicting_interval_requires_reason_and_has_no_numeric_value() {
        let case = case();
        let receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.samples[0].status = ServiceSampleStatus::Conflicting;
        trajectory.samples[0].delivered_kwh = None;
        trajectory.samples[0].conflict_reason = Some("two meters disagree".to_string());
        trajectory.parent_service_artifacts = receipt.decision_steps.iter()
            .filter(|step| step.disposition == StepDisposition::Executed
                && step.service_outcome == ServiceOutcome::Scored)
            .filter_map(|step| step.service_trajectory.as_ref())
            .map(|artifact| ServiceSourceArtifact {
                artifact_ref: artifact.artifact_ref.clone(),
                sha256: artifact.sha256.clone(),
            }).collect();
        let mut receipt = receipt;
        receipt.aggregate_score = None;
        let report = audit_service_trajectory(&trajectory, &receipt, &case).unwrap();
        assert_eq!(report.conflicting_sample_count, 1);
        assert_eq!(report.result_status, "inconclusive_unknown_or_conflicting_service");
    }

    #[test]
    fn missing_load_interval_row_is_rejected_not_imputed() {
        let case = case();
        let receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.samples.pop();
        assert!(audit_service_trajectory(&trajectory, &receipt, &case).is_err());
    }

    #[test]
    fn duplicate_load_interval_row_is_rejected() {
        let case = case();
        let receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.samples.push(trajectory.samples[0].clone());
        assert!(audit_service_trajectory(&trajectory, &receipt, &case).is_err());
    }

    #[test]
    fn demand_denominator_must_match_case_kw_and_interval() {
        let case = case();
        let receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.samples[0].demand_kwh += 1.0;
        assert!(audit_service_trajectory(&trajectory, &receipt, &case).is_err());
    }

    #[test]
    fn sample_timestamp_must_match_declared_epoch_and_step() {
        let case = case();
        let receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.samples[0].interval_start_unix_ms += 60_000;
        assert!(audit_service_trajectory(&trajectory, &receipt, &case).is_err());
    }

    #[test]
    fn measured_service_requires_calibration_and_per_sample_evidence() {
        let case = case();
        let receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.producer.calibration_artifact_sha256 = None;
        assert!(audit_service_trajectory(&trajectory, &receipt, &case).is_err());

        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.samples[0].evidence_ref = None;
        trajectory.samples[0].evidence_sha256 = None;
        assert!(audit_service_trajectory(&trajectory, &receipt, &case).is_err());
    }

    #[test]
    fn aggregate_score_must_equal_recomputed_weighted_kwh() {
        let case = case();
        let mut receipt = receipt(&case);
        let trajectory = fully_served_trajectory(&case, &receipt);
        bind_final_and_score(&trajectory, &mut receipt, 1.0);
        assert!(audit_service_trajectory(&trajectory, &receipt, &case).is_err());
    }

    #[test]
    fn measured_service_requires_exact_parent_service_artifact_set() {
        let case = case();
        let mut receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        let expected = case.network.loads.iter().map(|load| {
            load.active_power_kw
                * f64::from(case.timebase.step_size) / 60.0
                * f64::from(case.timebase.horizon_steps)
                * load.priority_weight
        }).sum::<f64>();
        bind_final_and_score(&trajectory, &mut receipt, expected);
        trajectory.parent_service_artifacts.clear();
        assert!(audit_service_trajectory(&trajectory, &receipt, &case).is_err());
    }

    #[test]
    fn measured_service_requires_source_artifact() {
        let case = case();
        let receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.source_artifacts.clear();
        assert!(audit_service_trajectory(&trajectory, &receipt, &case).is_err());
    }

    #[test]
    fn planned_forecast_cannot_be_bound_as_run_final_service() {
        let case = case();
        let mut receipt = receipt(&case);
        let mut trajectory = fully_served_trajectory(&case, &receipt);
        trajectory.stream_kind = ServiceStreamKind::PlannedForecast;
        trajectory.artifact_uri = "synthetic://forecast".to_string();
        receipt.final_service_trajectory = Some(power_restoration_run_receipt::ArtifactRef {
            artifact_ref: trajectory.artifact_uri.clone(),
            sha256: canonical_sha256(&trajectory).unwrap(),
        });
        assert!(audit_service_trajectory(&trajectory, &receipt, &case).is_err());
    }
}
