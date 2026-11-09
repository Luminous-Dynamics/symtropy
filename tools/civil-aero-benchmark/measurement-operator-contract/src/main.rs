// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Measurement-operator identity and compatibility only. No signal is processed,
// filtered, interpolated, deconvolved, calibrated or physically validated here.

use serde::{Deserialize, Serialize};
#[cfg(test)]
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const FIXTURE: &[u8] = include_bytes!("../data/measurement-operator-fixture-v0.1.json");

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct StateIdentity {
    configuration_sha256: String,
    geometry_sha256: String,
    loading_state_sha256: String,
    environment_sha256: String,
    mission_phase_id: String,
    test_article_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Calibration {
    status: String,
    calibration_id: Option<String>,
    calibration_artifact_sha256: Option<String>,
    validity_envelope_sha256: Option<String>,
    valid_at_test_state: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sampling {
    statistic: String,
    time_window_id: String,
    sample_rate_hz: Option<f64>,
    time_base_id: Option<String>,
    synchronization_status: String,
    synchronization_receipt_sha256: Option<String>,
    frequency_grid_sha256: Option<String>,
    spectral_estimator_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilterOperator {
    transfer_function_sha256: Option<String>,
    bandwidth_hz: Option<f64>,
    low_pass: bool,
    integration_time_s: Option<f64>,
    exposure_time_s: Option<f64>,
    mask_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    status: String,
    value: Option<f64>,
    missing_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MeasurementChannel {
    id: String,
    instrument_family: String,
    instrument_id: String,
    quantity: String,
    units: String,
    sign_convention: String,
    spatial_support: String,
    spatial_location_id: String,
    coordinate_frame: String,
    registration_sha256: Option<String>,
    identity: StateIdentity,
    measurement_run_id: String,
    source_artifact_sha256: String,
    raw_data_sha256: String,
    derived_processing_sha256: String,
    calibration: Calibration,
    sampling: Sampling,
    filter: FilterOperator,
    observation: Observation,
    interpretation_limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ComparisonRequest {
    id: String,
    left_channel_id: String,
    right_channel_id: String,
    mode: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dataset {
    schema_version: String,
    dataset_id: String,
    evidence_class: String,
    claim_ceiling: String,
    channels: Vec<MeasurementChannel>,
    comparisons: Vec<ComparisonRequest>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct ComparisonResult {
    id: String,
    left_channel_id: String,
    right_channel_id: String,
    status: String,
    reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ChannelResult {
    id: String,
    observation_status: String,
    calibration_status: String,
    claim_status: String,
    reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct Evaluation {
    schema_version: &'static str,
    dataset_id: String,
    source_fixture_sha256: String,
    channel_results: Vec<ChannelResult>,
    comparisons: Vec<ComparisonResult>,
    admitted_comparisons: usize,
    abstained_comparisons: usize,
    incompatible_comparisons: usize,
    unsynchronized_comparisons: usize,
    claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContractError(String);

impl std::fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ContractError {}

type Result<T> = std::result::Result<T, ContractError>;

fn error<T>(message: impl Into<String>) -> Result<T> {
    Err(ContractError(message.into()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_sha256(value: &str, label: &str) -> Result<()> {
    if !is_sha256(value) {
        return error(format!("{label}: expected lowercase SHA-256"));
    }
    Ok(())
}

fn require_nonempty(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() {
        return error(format!("{label}: must be non-empty"));
    }
    Ok(())
}

fn finite_positive(value: f64, label: &str) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return error(format!("{label}: must be finite and positive"));
    }
    Ok(())
}

fn validate_identity(identity: &StateIdentity, label: &str) -> Result<()> {
    for (value, field) in [
        (&identity.configuration_sha256, "configuration"),
        (&identity.geometry_sha256, "geometry"),
        (&identity.loading_state_sha256, "loading state"),
        (&identity.environment_sha256, "environment"),
    ] {
        require_sha256(value, &format!("{label} {field} digest"))?;
    }
    require_nonempty(
        &identity.mission_phase_id,
        &format!("{label} mission phase"),
    )?;
    require_nonempty(&identity.test_article_id, &format!("{label} test article"))?;
    Ok(())
}

fn validate_calibration(calibration: &Calibration, channel_id: &str) -> Result<()> {
    match calibration.status.as_str() {
        "VALID" => {
            require_nonempty(
                calibration.calibration_id.as_deref().unwrap_or(""),
                &format!("{channel_id} calibration ID"),
            )?;
            require_sha256(
                calibration
                    .calibration_artifact_sha256
                    .as_deref()
                    .unwrap_or(""),
                &format!("{channel_id} calibration artifact"),
            )?;
            require_sha256(
                calibration
                    .validity_envelope_sha256
                    .as_deref()
                    .unwrap_or(""),
                &format!("{channel_id} calibration validity envelope"),
            )?;
            if !calibration.valid_at_test_state {
                return error(format!(
                    "{channel_id}: VALID calibration is outside its declared test-state envelope"
                ));
            }
        }
        "EXPIRED" | "UNKNOWN" => {
            if calibration.valid_at_test_state {
                return error(format!(
                    "{channel_id}: expired/unknown calibration cannot be valid at the test state"
                ));
            }
        }
        other => {
            return error(format!(
                "{channel_id}: unsupported calibration status {other:?}"
            ));
        }
    }
    Ok(())
}

fn validate_sampling(sampling: &Sampling, channel_id: &str) -> Result<()> {
    require_nonempty(&sampling.statistic, &format!("{channel_id} statistic"))?;
    require_nonempty(
        &sampling.time_window_id,
        &format!("{channel_id} time window"),
    )?;
    match sampling.synchronization_status.as_str() {
        "SYNCHRONIZED" => {
            require_nonempty(
                sampling.time_base_id.as_deref().unwrap_or(""),
                &format!("{channel_id} time-base ID"),
            )?;
            require_sha256(
                sampling
                    .synchronization_receipt_sha256
                    .as_deref()
                    .unwrap_or(""),
                &format!("{channel_id} synchronization receipt"),
            )?;
        }
        "UNSYNCHRONIZED" | "UNKNOWN" => {
            if sampling.synchronization_receipt_sha256.is_some() {
                return error(format!(
                    "{channel_id}: unsynchronized/unknown clock cannot carry a synchronization receipt"
                ));
            }
        }
        other => {
            return error(format!(
                "{channel_id}: unsupported synchronization status {other:?}"
            ));
        }
    }
    if let Some(rate) = sampling.sample_rate_hz {
        finite_positive(rate, &format!("{channel_id} sample rate"))?;
    }
    if let Some(digest) = &sampling.frequency_grid_sha256 {
        require_sha256(digest, &format!("{channel_id} frequency-grid digest"))?;
    }
    if let Some(digest) = &sampling.spectral_estimator_sha256 {
        require_sha256(digest, &format!("{channel_id} spectral-estimator digest"))?;
    }
    if matches!(
        sampling.statistic.as_str(),
        "PSD" | "CROSS_SPECTRUM" | "COHERENCE"
    ) {
        finite_positive(
            sampling.sample_rate_hz.unwrap_or(f64::NAN),
            &format!("{channel_id} spectral sample rate"),
        )?;
        require_sha256(
            sampling.frequency_grid_sha256.as_deref().unwrap_or(""),
            &format!("{channel_id} spectral frequency grid"),
        )?;
        require_sha256(
            sampling.spectral_estimator_sha256.as_deref().unwrap_or(""),
            &format!("{channel_id} spectral estimator"),
        )?;
    }
    Ok(())
}

fn validate_filter(filter: &FilterOperator, channel_id: &str) -> Result<()> {
    if let Some(digest) = &filter.transfer_function_sha256 {
        require_sha256(digest, &format!("{channel_id} transfer-function digest"))?;
    }
    if let Some(bandwidth) = filter.bandwidth_hz {
        finite_positive(bandwidth, &format!("{channel_id} bandwidth"))?;
    }
    if let Some(value) = filter.integration_time_s {
        finite_positive(value, &format!("{channel_id} integration time"))?;
    }
    if let Some(value) = filter.exposure_time_s {
        finite_positive(value, &format!("{channel_id} exposure time"))?;
    }
    if let Some(digest) = &filter.mask_sha256 {
        require_sha256(digest, &format!("{channel_id} mask digest"))?;
    }
    if filter.low_pass && filter.bandwidth_hz.is_none() && filter.exposure_time_s.is_none() {
        return error(format!(
            "{channel_id}: low-pass operator requires bandwidth or exposure metadata"
        ));
    }
    Ok(())
}

fn validate_observation(observation: &Observation, channel_id: &str) -> Result<()> {
    match observation.status.as_str() {
        "PRESENT" => {
            let Some(value) = observation.value else {
                return error(format!(
                    "{channel_id}: PRESENT observation requires a numeric value"
                ));
            };
            if !value.is_finite() {
                return error(format!("{channel_id}: PRESENT observation is non-finite"));
            }
            if observation.missing_reason.is_some() {
                return error(format!(
                    "{channel_id}: PRESENT observation cannot carry a missingness reason"
                ));
            }
        }
        "MASKED" | "UNAVAILABLE" | "NOT_COLLECTED" | "UNKNOWN" => {
            if observation.value.is_some() {
                return error(format!(
                    "{channel_id}: {} observation must remain non-numeric (no zero-fill)",
                    observation.status
                ));
            }
            require_nonempty(
                observation.missing_reason.as_deref().unwrap_or(""),
                &format!("{channel_id} missingness reason"),
            )?;
        }
        other => {
            return error(format!(
                "{channel_id}: unsupported observation status {other:?}"
            ));
        }
    }
    Ok(())
}

fn validate_channel(channel: &MeasurementChannel) -> Result<()> {
    let id = &channel.id;
    for (value, label) in [
        (&channel.id, "channel ID"),
        (&channel.instrument_family, "instrument family"),
        (&channel.instrument_id, "instrument ID"),
        (&channel.quantity, "quantity"),
        (&channel.units, "units"),
        (&channel.sign_convention, "sign convention"),
        (&channel.spatial_support, "spatial support"),
        (&channel.spatial_location_id, "spatial location ID"),
        (&channel.coordinate_frame, "coordinate frame"),
        (&channel.measurement_run_id, "measurement run ID"),
    ] {
        require_nonempty(value, &format!("{id} {label}"))?;
    }
    for (value, label) in [
        (&channel.source_artifact_sha256, "source artifact"),
        (&channel.raw_data_sha256, "raw data"),
        (&channel.derived_processing_sha256, "derived processing"),
    ] {
        require_sha256(value, &format!("{id} {label} digest"))?;
    }
    validate_identity(&channel.identity, id)?;
    if let Some(digest) = &channel.registration_sha256 {
        require_sha256(digest, &format!("{id} registration digest"))?;
    }
    validate_calibration(&channel.calibration, id)?;
    validate_sampling(&channel.sampling, id)?;
    validate_filter(&channel.filter, id)?;
    validate_observation(&channel.observation, id)?;
    for limitation in &channel.interpretation_limitations {
        require_nonempty(limitation, &format!("{id} interpretation limitation"))?;
    }
    Ok(())
}

fn incompatible(
    id: &str,
    left: &MeasurementChannel,
    right: &MeasurementChannel,
    reasons: Vec<String>,
) -> ComparisonResult {
    ComparisonResult {
        id: id.to_owned(),
        left_channel_id: left.id.clone(),
        right_channel_id: right.id.clone(),
        status: "INCOMPATIBLE".to_owned(),
        reason_codes: reasons,
    }
}

fn abstain(
    id: &str,
    left: &MeasurementChannel,
    right: &MeasurementChannel,
    reasons: Vec<String>,
) -> ComparisonResult {
    ComparisonResult {
        id: id.to_owned(),
        left_channel_id: left.id.clone(),
        right_channel_id: right.id.clone(),
        status: "ABSTAIN".to_owned(),
        reason_codes: reasons,
    }
}

fn compare_pair(
    request: &ComparisonRequest,
    left: &MeasurementChannel,
    right: &MeasurementChannel,
) -> Result<ComparisonResult> {
    if left.observation.status != "PRESENT" || right.observation.status != "PRESENT" {
        let mut reasons = Vec::new();
        if left.observation.status != "PRESENT" {
            reasons.push(format!("LEFT_{}", left.observation.status));
        }
        if right.observation.status != "PRESENT" {
            reasons.push(format!("RIGHT_{}", right.observation.status));
        }
        return Ok(abstain(&request.id, left, right, reasons));
    }

    if left.calibration.status != "VALID" || right.calibration.status != "VALID" {
        let mut reasons = Vec::new();
        if left.calibration.status != "VALID" {
            reasons.push(format!("LEFT_CALIBRATION_{}", left.calibration.status));
        }
        if right.calibration.status != "VALID" {
            reasons.push(format!("RIGHT_CALIBRATION_{}", right.calibration.status));
        }
        return Ok(abstain(&request.id, left, right, reasons));
    }

    let mut reasons = Vec::new();
    if left.identity != right.identity {
        reasons.push("STATE_IDENTITY_MISMATCH".to_owned());
    }
    if left.instrument_family != right.instrument_family {
        reasons.push("INSTRUMENT_FAMILY_MISMATCH".to_owned());
    }
    if left.quantity != right.quantity {
        reasons.push("QUANTITY_MISMATCH".to_owned());
    }
    if left.units != right.units {
        reasons.push("UNIT_MISMATCH".to_owned());
    }
    if left.sign_convention != right.sign_convention {
        reasons.push("SIGN_CONVENTION_MISMATCH".to_owned());
    }
    if left.spatial_support != right.spatial_support {
        reasons.push("SPATIAL_SUPPORT_MISMATCH".to_owned());
    }
    if left.spatial_location_id != right.spatial_location_id {
        reasons.push("SPATIAL_LOCATION_MISMATCH".to_owned());
    }
    if left.coordinate_frame != right.coordinate_frame {
        reasons.push("COORDINATE_FRAME_MISMATCH".to_owned());
    }
    if left.registration_sha256 != right.registration_sha256 {
        reasons.push("REGISTRATION_MISMATCH".to_owned());
    }
    if left.sampling.statistic != right.sampling.statistic {
        reasons.push("STATISTIC_MISMATCH".to_owned());
    }
    if left.sampling.time_window_id != right.sampling.time_window_id {
        reasons.push("TIME_WINDOW_MISMATCH".to_owned());
    }
    if left.sampling.sample_rate_hz != right.sampling.sample_rate_hz {
        reasons.push("SAMPLE_RATE_MISMATCH".to_owned());
    }
    if left.measurement_run_id != right.measurement_run_id {
        reasons.push("MEASUREMENT_RUN_MISMATCH".to_owned());
    }
    if left.filter.transfer_function_sha256 != right.filter.transfer_function_sha256
        || left.filter.bandwidth_hz != right.filter.bandwidth_hz
        || left.filter.low_pass != right.filter.low_pass
        || left.filter.integration_time_s != right.filter.integration_time_s
        || left.filter.exposure_time_s != right.filter.exposure_time_s
    {
        reasons.push("MEASUREMENT_FILTER_MISMATCH".to_owned());
    }
    if left.filter.mask_sha256 != right.filter.mask_sha256 {
        reasons.push("SPATIAL_MASK_MISMATCH".to_owned());
    }
    if matches!(
        left.sampling.statistic.as_str(),
        "PSD" | "CROSS_SPECTRUM" | "COHERENCE"
    ) && (left.sampling.sample_rate_hz != right.sampling.sample_rate_hz
        || left.sampling.frequency_grid_sha256 != right.sampling.frequency_grid_sha256
        || left.sampling.spectral_estimator_sha256 != right.sampling.spectral_estimator_sha256)
    {
        reasons.push("SPECTRAL_PROCESSING_MISMATCH".to_owned());
    }

    if !reasons.is_empty() {
        return Ok(incompatible(&request.id, left, right, reasons));
    }

    match request.mode.as_str() {
        "DIRECT_SCALAR" => {}
        "COHERENCE_OR_PHASE" => {
            if left.sampling.synchronization_status != "SYNCHRONIZED"
                || right.sampling.synchronization_status != "SYNCHRONIZED"
                || left.sampling.time_base_id != right.sampling.time_base_id
                || left.sampling.synchronization_receipt_sha256
                    != right.sampling.synchronization_receipt_sha256
                || left.sampling.sample_rate_hz != right.sampling.sample_rate_hz
            {
                return Ok(ComparisonResult {
                    id: request.id.clone(),
                    left_channel_id: left.id.clone(),
                    right_channel_id: right.id.clone(),
                    status: "UNSYNCHRONIZED".to_owned(),
                    reason_codes: vec![
                        "COMMON_TIMEBASE_OR_SYNC_RECEIPT_NOT_ESTABLISHED".to_owned(),
                    ],
                });
            }
        }
        other => {
            return error(format!(
                "{}: unsupported comparison mode {other:?}",
                request.id
            ));
        }
    }

    Ok(ComparisonResult {
        id: request.id.clone(),
        left_channel_id: left.id.clone(),
        right_channel_id: right.id.clone(),
        status: "DIRECT_COMPARISON_ADMITTED".to_owned(),
        reason_codes: vec!["OPERATOR_IDENTITIES_MATCH_DECLARED_V0_1_RULES".to_owned()],
    })
}

fn validate_dataset(dataset: &Dataset) -> Result<()> {
    if dataset.schema_version != "aero-measurement-operator/0.1.0" {
        return error("unsupported measurement-operator schema version");
    }
    require_nonempty(&dataset.dataset_id, "dataset ID")?;
    if dataset.evidence_class != "synthetic_contract_fixture" {
        return error("v0.1 executable only admits the declared synthetic contract fixture");
    }
    if dataset.claim_ceiling != "measurement_operator_compatibility_only" {
        return error("measurement-operator claim ceiling is missing or weakened");
    }
    if dataset.channels.is_empty() {
        return error("measurement dataset has no channels");
    }
    let mut ids = BTreeSet::new();
    for channel in &dataset.channels {
        if !ids.insert(channel.id.clone()) {
            return error(format!("duplicate measurement channel ID {}", channel.id));
        }
        validate_channel(channel)?;
    }
    let channel_ids = ids;
    let mut comparison_ids = BTreeSet::new();
    for comparison in &dataset.comparisons {
        require_nonempty(&comparison.id, "comparison ID")?;
        if !comparison_ids.insert(comparison.id.clone()) {
            return error(format!("duplicate comparison ID {}", comparison.id));
        }
        if !channel_ids.contains(&comparison.left_channel_id)
            || !channel_ids.contains(&comparison.right_channel_id)
        {
            return error(format!(
                "{}: comparison references unknown channel",
                comparison.id
            ));
        }
        if comparison.left_channel_id == comparison.right_channel_id {
            return error(format!(
                "{}: self-comparison is not admitted",
                comparison.id
            ));
        }
    }
    Ok(())
}

fn evaluate_dataset(dataset: &Dataset, source_bytes: &[u8]) -> Result<Evaluation> {
    validate_dataset(dataset)?;
    let channels = dataset
        .channels
        .iter()
        .map(|channel| (channel.id.as_str(), channel))
        .collect::<BTreeMap<_, _>>();

    let mut channel_results = Vec::new();
    for channel in &dataset.channels {
        let status = if channel.observation.status == "PRESENT" {
            "OBSERVATION_PRESENT"
        } else {
            "OBSERVATION_WITHHELD"
        };
        let reasons = if channel.observation.status == "PRESENT" {
            vec!["RAW_OR_DECLARED_DERIVED_VALUE_RETAINED".to_owned()]
        } else {
            vec![format!("OBSERVATION_{}", channel.observation.status)]
        };
        channel_results.push(ChannelResult {
            id: channel.id.clone(),
            observation_status: channel.observation.status.clone(),
            calibration_status: channel.calibration.status.clone(),
            claim_status: status.to_owned(),
            reason_codes: reasons,
        });
    }

    let mut comparisons = Vec::new();
    for request in &dataset.comparisons {
        let left = channels[request.left_channel_id.as_str()];
        let right = channels[request.right_channel_id.as_str()];
        comparisons.push(compare_pair(request, left, right)?);
    }

    let admitted = comparisons
        .iter()
        .filter(|item| item.status == "DIRECT_COMPARISON_ADMITTED")
        .count();
    let abstained = comparisons
        .iter()
        .filter(|item| item.status == "ABSTAIN")
        .count();
    let incompatible_count = comparisons
        .iter()
        .filter(|item| item.status == "INCOMPATIBLE")
        .count();
    let unsynchronized = comparisons
        .iter()
        .filter(|item| item.status == "UNSYNCHRONIZED")
        .count();

    Ok(Evaluation {
        schema_version: "aero-measurement-operator-result/0.1.0",
        dataset_id: dataset.dataset_id.clone(),
        source_fixture_sha256: sha256_hex(source_bytes),
        channel_results,
        comparisons,
        admitted_comparisons: admitted,
        abstained_comparisons: abstained,
        incompatible_comparisons: incompatible_count,
        unsynchronized_comparisons: unsynchronized,
        claim_ceiling: "measurement_operator_compatibility_only",
    })
}

fn main() {
    let dataset: Dataset = match serde_json::from_slice(FIXTURE) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("measurement_operator_parse_error={error}");
            std::process::exit(2);
        }
    };
    let result = match evaluate_dataset(&dataset, FIXTURE) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("measurement_operator_contract_error={error}");
            std::process::exit(2);
        }
    };
    match serde_json::to_string_pretty(&result) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("measurement_operator_serialization_error={error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_value() -> Value {
        serde_json::from_slice(FIXTURE).expect("synthetic fixture JSON must parse")
    }

    fn dataset(value: &Value) -> Dataset {
        serde_json::from_value(value.clone()).expect("mutated fixture must deserialize")
    }

    fn result_for(value: &Value) -> Evaluation {
        let source = serde_json::to_vec(value).unwrap();
        evaluate_dataset(&dataset(value), &source).unwrap()
    }

    fn comparison<'a>(result: &'a Evaluation, id: &str) -> &'a ComparisonResult {
        result
            .comparisons
            .iter()
            .find(|item| item.id == id)
            .unwrap()
    }

    #[test]
    fn frozen_operator_compatibility_known_answers_match() {
        let value = fixture_value();
        let result = result_for(&value);
        assert_eq!(
            comparison(&result, "healthy_same_point_mean").status,
            "DIRECT_COMPARISON_ADMITTED"
        );
        assert_eq!(
            comparison(&result, "kulite_vs_upsp_rms").status,
            "INCOMPATIBLE"
        );
        assert_eq!(
            comparison(&result, "phase_pair_unsynchronized").status,
            "UNSYNCHRONIZED"
        );
        assert_eq!(
            comparison(&result, "psp_simultaneous_drag_not_available").status,
            "ABSTAIN"
        );
        assert_eq!(
            comparison(&result, "derived_buffet_index_vs_deflection").status,
            "INCOMPATIBLE"
        );
        assert_eq!(comparison(&result, "masked_upsp_channel").status, "ABSTAIN");
        assert_eq!(
            comparison(&result, "stale_geometry_pair").status,
            "INCOMPATIBLE"
        );
        assert_eq!(
            comparison(&result, "expired_calibration_pair").status,
            "ABSTAIN"
        );
    }

    #[test]
    fn missing_measurement_cannot_be_zero_filled() {
        let mut value = fixture_value();
        let channel = value["channels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "balance_psp_simultaneous_cd")
            .unwrap();
        channel["observation"]["value"] = Value::from(0.0);
        assert!(validate_dataset(&dataset(&value)).is_err());
    }

    #[test]
    fn present_measurement_requires_a_finite_value() {
        let mut value = fixture_value();
        let channel = value["channels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "kulite_point_a")
            .unwrap();
        channel["observation"]["value"] = Value::Null;
        assert!(validate_dataset(&dataset(&value)).is_err());
    }

    #[test]
    fn stale_registration_digest_is_not_directly_comparable() {
        let mut value = fixture_value();
        let channel = value["channels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "healthy_point_mean_b")
            .unwrap();
        channel["registration_sha256"] = Value::from("f".repeat(64));
        let result = result_for(&value);
        assert_eq!(
            comparison(&result, "healthy_same_point_mean").status,
            "INCOMPATIBLE"
        );
    }

    #[test]
    fn unknown_calibration_abstains() {
        let mut value = fixture_value();
        let channel = value["channels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "healthy_point_mean_b")
            .unwrap();
        channel["calibration"]["status"] = Value::from("UNKNOWN");
        channel["calibration"]["valid_at_test_state"] = Value::Bool(false);
        let result = result_for(&value);
        assert_eq!(
            comparison(&result, "healthy_same_point_mean").status,
            "ABSTAIN"
        );
    }

    #[test]
    fn changed_filter_response_prevents_rms_comparison() {
        let mut value = fixture_value();
        let channel = value["channels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "healthy_point_mean_b")
            .unwrap();
        channel["filter"]["transfer_function_sha256"] = Value::from("d".repeat(64));
        let result = result_for(&value);
        assert_eq!(
            comparison(&result, "healthy_same_point_mean").status,
            "INCOMPATIBLE"
        );
    }

    #[test]
    fn different_instrument_families_are_not_directly_compared() {
        let mut value = fixture_value();
        let channel = value["channels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "healthy_point_mean_b")
            .unwrap();
        channel["instrument_family"] = Value::from("esp");
        let result = result_for(&value);
        assert_eq!(
            comparison(&result, "healthy_same_point_mean").status,
            "INCOMPATIBLE"
        );
    }

    #[test]
    fn coherence_requires_common_synchronized_clock() {
        let mut value = fixture_value();
        let channel = value["channels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "healthy_point_mean_b")
            .unwrap();
        channel["sampling"]["time_base_id"] = Value::from("clock-other");
        let result = result_for(&value);
        assert_eq!(
            comparison(&result, "phase_pair_unsynchronized").status,
            "UNSYNCHRONIZED"
        );
    }

    #[test]
    fn mask_cannot_be_removed_without_changing_operator_identity() {
        let mut value = fixture_value();
        let channel = value["channels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "upsp_surface_rms")
            .unwrap();
        channel["filter"]["mask_sha256"] = Value::Null;
        let result = result_for(&value);
        assert_eq!(
            comparison(&result, "kulite_vs_upsp_rms").status,
            "INCOMPATIBLE"
        );
    }

    #[test]
    fn sha256_helper_matches_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sample_rate_must_be_positive_when_present() {
        let mut value = fixture_value();
        let channel = value["channels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "kulite_point_a")
            .unwrap();
        channel["sampling"]["sample_rate_hz"] = Value::from(0.0);
        assert!(validate_dataset(&dataset(&value)).is_err());
    }

    #[test]
    fn low_pass_channel_requires_exposure_or_bandwidth() {
        let mut value = fixture_value();
        let channel = value["channels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "upsp_surface_rms")
            .unwrap();
        channel["filter"]["bandwidth_hz"] = Value::Null;
        channel["filter"]["exposure_time_s"] = Value::Null;
        assert!(validate_dataset(&dataset(&value)).is_err());
    }

    #[test]
    fn derived_buffet_index_is_not_a_displacement_channel() {
        let value = fixture_value();
        let result = result_for(&value);
        assert!(
            comparison(&result, "derived_buffet_index_vs_deflection")
                .reason_codes
                .contains(&"QUANTITY_MISMATCH".to_owned())
        );
    }

    #[test]
    fn unavailable_balance_drag_is_retained_as_null() {
        let value = fixture_value();
        let result = result_for(&value);
        let channel = result
            .channel_results
            .iter()
            .find(|item| item.id == "balance_psp_simultaneous_cd")
            .unwrap();
        assert_eq!(channel.observation_status, "UNAVAILABLE");
        assert_eq!(
            comparison(&result, "psp_simultaneous_drag_not_available").status,
            "ABSTAIN"
        );
    }
}
