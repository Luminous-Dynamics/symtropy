// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Instrument-aware observation contract. This is an evidence/comparison gate,
// not a CFD/FE solver, sensor deconvolution package, or modal-identification tool.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const DPW8_CATALOG: &str =
    include_str!("../../dpw8-aepw4-reference-intake/dpw8-reference-catalog-v0.2.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ComparisonKind {
    Pointwise,
    Rms,
    Psd,
    Coherence,
    ModalValidation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum DecisionStatus {
    Allow,
    Abstain,
    Reject,
    NotAssessed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct PsdSpec {
    window: Option<String>,
    segment_length_samples: Option<usize>,
    overlap_samples: Option<usize>,
    detrend: Option<String>,
    estimator: Option<String>,
    normalization: Option<String>,
    output_unit: Option<String>,
}

impl PsdSpec {
    fn is_complete(&self) -> bool {
        let Some(segment_length) = self.segment_length_samples else {
            return false;
        };
        self.window
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            && segment_length >= 2
            && self
                .overlap_samples
                .is_some_and(|overlap| overlap < segment_length)
            && self
                .detrend
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && self
                .estimator
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && self
                .normalization
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && self
                .output_unit
                .as_deref()
                .is_some_and(|value| !value.is_empty())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ObservationChannel {
    channel_id: String,
    quantity: String,
    unit: String,
    coordinate_frame: String,
    spatial_operator_id: String,
    temporal_operator_id: String,
    source_admitted: bool,
    calibration_admitted: bool,
    calibration_operator_id: Option<String>,
    standard_uncertainty: Option<f64>,
    uncertainty_budget_id: Option<String>,
    mask_required: bool,
    mask_applied: bool,
    mask_operator_id: Option<String>,
    timebase_id: Option<String>,
    sample_rate_hz: Option<f64>,
    synchronization_uncertainty_s: Option<f64>,
    bandwidth_hz: Option<[f64; 2]>,
    psd: PsdSpec,
    structural_mode_evidence_admitted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Decision {
    status: DecisionStatus,
    reasons: Vec<String>,
}

impl Decision {
    fn allow() -> Self {
        Self {
            status: DecisionStatus::Allow,
            reasons: vec!["all declared observation-operator gates passed".to_owned()],
        }
    }

    fn abstain(reason: impl Into<String>) -> Self {
        Self {
            status: DecisionStatus::Abstain,
            reasons: vec![reason.into()],
        }
    }

    fn reject(reason: impl Into<String>) -> Self {
        Self {
            status: DecisionStatus::Reject,
            reasons: vec![reason.into()],
        }
    }

    fn not_assessed(reason: impl Into<String>) -> Self {
        Self {
            status: DecisionStatus::NotAssessed,
            reasons: vec![reason.into()],
        }
    }
}

fn finite_positive(value: Option<f64>) -> bool {
    value.is_some_and(|number| number.is_finite() && number > 0.0)
}

fn valid_bandwidth(bandwidth: Option<[f64; 2]>) -> bool {
    bandwidth.is_some_and(|band| {
        band[0].is_finite() && band[1].is_finite() && band[0] >= 0.0 && band[1] > band[0]
    })
}

fn assess_pair(
    left: &ObservationChannel,
    right: &ObservationChannel,
    kind: ComparisonKind,
    maximum_sync_uncertainty_s: f64,
) -> Decision {
    if !left.source_admitted || !right.source_admitted {
        return Decision::abstain("source bytes, terms and applicability are not admitted");
    }
    if !left.calibration_admitted || !right.calibration_admitted {
        return Decision::abstain("instrument calibration/uncertainty is not admitted");
    }
    for channel in [left, right] {
        if channel
            .calibration_operator_id
            .as_deref()
            .is_none_or(|identity| identity.trim().is_empty())
        {
            return Decision::abstain("calibration operator identity is missing");
        }
        if !finite_positive(channel.standard_uncertainty)
            || channel
                .uncertainty_budget_id
                .as_deref()
                .is_none_or(|identity| identity.trim().is_empty())
        {
            return Decision::abstain(
                "standard uncertainty or its traceable uncertainty-budget identity is missing",
            );
        }
    }
    if left.quantity != right.quantity {
        return Decision::reject("physical quantities differ");
    }
    if left.unit != right.unit {
        return Decision::reject("units differ and no explicit canonical conversion was supplied");
    }
    if left.coordinate_frame != right.coordinate_frame {
        return Decision::reject("coordinate/reference frames differ");
    }
    if left.spatial_operator_id != right.spatial_operator_id {
        return Decision::reject(
            "spatial sampling/mapping operators differ; project both products through one admitted operator",
        );
    }
    if (left.mask_required && (!left.mask_applied || left.mask_operator_id.is_none()))
        || (right.mask_required && (!right.mask_applied || right.mask_operator_id.is_none()))
    {
        return Decision::reject("a required reliability mask is missing or unapplied");
    }
    if left.mask_operator_id != right.mask_operator_id {
        return Decision::reject("reliability mask identities differ");
    }
    if left.temporal_operator_id != right.temporal_operator_id {
        return Decision::reject(
            "temporal instrument responses differ; direct comparison would mix measurement bandwidths",
        );
    }
    if !valid_bandwidth(left.bandwidth_hz) || !valid_bandwidth(right.bandwidth_hz) {
        return Decision::abstain("measurement bandwidth is unknown or invalid");
    }
    if left.bandwidth_hz != right.bandwidth_hz {
        return Decision::reject("declared comparison bandwidths differ");
    }

    match kind {
        ComparisonKind::Pointwise | ComparisonKind::Rms => Decision::allow(),
        ComparisonKind::Psd => {
            if !finite_positive(left.sample_rate_hz) || !finite_positive(right.sample_rate_hz) {
                return Decision::abstain("sample rate is missing or invalid");
            }
            if left.sample_rate_hz != right.sample_rate_hz {
                return Decision::reject(
                    "sample rates differ; a separately declared and replayable resampling transform is required",
                );
            }
            if !left.psd.is_complete() || !right.psd.is_complete() {
                return Decision::abstain(
                    "PSD window, segment length, overlap, detrend, estimator, normalization or units are missing",
                );
            }
            if left.psd != right.psd {
                return Decision::reject("PSD estimator configurations are not identical");
            }
            Decision::allow()
        }
        ComparisonKind::Coherence => {
            if !finite_positive(left.sample_rate_hz) || !finite_positive(right.sample_rate_hz) {
                return Decision::abstain("sample rate is missing or invalid");
            }
            if left.sample_rate_hz != right.sample_rate_hz {
                return Decision::reject("coherence sample rates differ");
            }
            if left.timebase_id.is_none() || right.timebase_id.is_none() {
                return Decision::abstain("a common acquisition timebase has not been identified");
            }
            if left.timebase_id != right.timebase_id {
                return Decision::reject("channels do not share the same admitted timebase");
            }
            let Some(left_sync) = left.synchronization_uncertainty_s else {
                return Decision::abstain("left-channel synchronization uncertainty is unknown");
            };
            let Some(right_sync) = right.synchronization_uncertainty_s else {
                return Decision::abstain("right-channel synchronization uncertainty is unknown");
            };
            if !left_sync.is_finite()
                || !right_sync.is_finite()
                || left_sync < 0.0
                || right_sync < 0.0
            {
                return Decision::reject("synchronization uncertainty is invalid");
            }
            if left_sync > maximum_sync_uncertainty_s || right_sync > maximum_sync_uncertainty_s {
                return Decision::abstain(
                    "synchronization uncertainty exceeds the predeclared phase/coherence budget",
                );
            }
            if !left.psd.is_complete() || !right.psd.is_complete() {
                return Decision::abstain("coherence spectral estimator metadata is incomplete");
            }
            if left.psd != right.psd {
                return Decision::reject("coherence estimator configurations differ");
            }
            Decision::allow()
        }
        ComparisonKind::ModalValidation => {
            if !left.structural_mode_evidence_admitted || !right.structural_mode_evidence_admitted {
                return Decision::not_assessed(
                    "admitted structural mode/deformation evidence is absent; pressure or buffet indicators alone do not establish a structural mode",
                );
            }
            if left.timebase_id.is_none() || left.timebase_id != right.timebase_id {
                return Decision::abstain(
                    "modal comparison requires a shared admitted timebase and phase reference",
                );
            }
            Decision::allow()
        }
    }
}

fn classify_jaxa_spectral_feature(
    instrument: &str,
    frequency_hz: f64,
    source_attributed_confounds_hz: &[(String, f64, f64)],
) -> &'static str {
    if !frequency_hz.is_finite() || frequency_hz < 0.0 {
        return "INVALID_FREQUENCY";
    }
    if source_attributed_confounds_hz
        .iter()
        .any(|(name, lower, upper)| {
            instrument.eq_ignore_ascii_case(name)
                && frequency_hz >= *lower
                && frequency_hz <= *upper
        })
    {
        return "SOURCE_ATTRIBUTED_CONFOUNDER_NOT_A_STRUCTURAL_MODE";
    }
    "UNCLASSIFIED_SPECTRAL_FEATURE_REQUIRES_INDEPENDENT_EVIDENCE"
}

fn classify_buffet_indicator(channel: &str) -> &'static str {
    if channel.eq_ignore_ascii_case("C_B") || channel.eq_ignore_ascii_case("buffet_intensity") {
        "BUFFET_ONSET_INDICATOR_ONLY"
    } else {
        "NOT_A_DECLARED_BUFFET_INDICATOR"
    }
}

fn source_admitted(source: &Value) -> bool {
    source.get("exact_bytes_acquired").and_then(Value::as_bool) == Some(true)
        && source.get("acquisition_state").and_then(Value::as_str)
            == Some("bytes_acquired_and_verified")
        && source
            .get("terms_status")
            .and_then(Value::as_str)
            .is_some_and(|status| {
                status == "reviewed_permitted" || status == "public_domain_or_open_license_verified"
            })
        && source
            .get("sha256")
            .and_then(Value::as_str)
            .is_some_and(|digest| {
                digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
}

fn case3_audit(catalog: &Value) -> Result<Value, String> {
    let schema = catalog
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| "catalog schema_version missing".to_owned())?;
    if schema != "aero-reference-catalog/0.2.0" {
        return Err(format!(
            "unsupported DPW8 reference catalog schema {schema}"
        ));
    }

    let cases = catalog
        .get("benchmark_cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "benchmark_cases missing".to_owned())?;
    let case = cases
        .iter()
        .find(|item| item.get("case_id").and_then(Value::as_str) == Some("DPW8-CRM-DYNAMIC-FSI-3"))
        .ok_or_else(|| "DPW8-CRM-DYNAMIC-FSI-3 missing".to_owned())?;

    let sources = catalog
        .get("sources")
        .and_then(Value::as_array)
        .ok_or_else(|| "sources missing".to_owned())?;
    let source_map: BTreeMap<&str, &Value> = sources
        .iter()
        .filter_map(|source| {
            source
                .get("source_id")
                .and_then(Value::as_str)
                .map(|id| (id, source))
        })
        .collect();

    let channels = case
        .get("measurement_channels")
        .and_then(Value::as_array)
        .ok_or_else(|| "Test Case 3 measurement_channels missing".to_owned())?;

    let mut channel_reports = Vec::new();
    let mut source_ids = BTreeSet::new();
    for channel in channels {
        let family = channel
            .get("channel_family")
            .and_then(Value::as_str)
            .ok_or_else(|| "measurement channel family missing".to_owned())?;
        let source_id = channel
            .get("source_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{family}: source_id missing"))?;
        source_ids.insert(source_id.to_owned());
        let source = source_map
            .get(source_id)
            .ok_or_else(|| format!("{family}: dangling source ID {source_id}"))?;
        let admitted = source_admitted(source);
        let state = source
            .get("acquisition_state")
            .and_then(Value::as_str)
            .unwrap_or("UNKNOWN");
        let exact_bytes = source
            .get("exact_bytes_acquired")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let digest_present = source.get("sha256").and_then(Value::as_str).is_some();

        channel_reports.push(json!({
            "channel_family": family,
            "instrument_or_method": channel.get("instrument_or_method"),
            "source_id": source_id,
            "acquisition_state": state,
            "exact_bytes_acquired": exact_bytes,
            "sha256_present": digest_present,
            "source_admitted": admitted,
            "quantitative_comparison": if admitted { "REQUIRES_OPERATOR_VALIDATION" } else { "ABSTAIN_SOURCE_NOT_ADMITTED" },
            "source_limitations": channel.get("limitations").cloned().unwrap_or_else(|| json!([]))
        }));
    }

    let spectral = case
        .get("spectral_processing")
        .ok_or_else(|| "spectral_processing missing".to_owned())?;
    let spectral_fields = [
        "time_step_s",
        "sampling_rate_hz",
        "window",
        "segment_length",
        "overlap",
        "psd_normalization",
    ];
    let missing_spectral = spectral_fields
        .iter()
        .filter(|field| spectral.get(*field).is_none_or(Value::is_null))
        .map(|field| (*field).to_owned())
        .collect::<Vec<_>>();

    let source_bytes_all_admitted = channel_reports
        .iter()
        .all(|report| report["source_admitted"].as_bool() == Some(true));
    let deformation = case
        .get("deformation_evidence")
        .ok_or_else(|| "deformation_evidence missing".to_owned())?;
    let official_deformation_pending = deformation
        .get("official_measurements_status")
        .and_then(Value::as_str)
        == Some("pending");

    let kulite = channels.iter().find(|channel| {
        channel.get("channel_family").and_then(Value::as_str) == Some("unsteady_point_pressure")
    });
    let upsp = channels.iter().find(|channel| {
        channel.get("channel_family").and_then(Value::as_str) == Some("unsteady_surface_pressure")
    });
    let c_b = channels.iter().find(|channel| {
        channel.get("channel_family").and_then(Value::as_str) == Some("buffet_intensity")
    });
    let balance = channels.iter().find(|channel| {
        channel.get("channel_family").and_then(Value::as_str) == Some("integrated_balance")
    });
    if kulite.is_none() || upsp.is_none() || c_b.is_none() || balance.is_none() {
        return Err("Test Case 3 is missing a required measurement channel family".to_owned());
    }
    let kulite_limits = kulite
        .and_then(|channel| channel.get("limitations"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase()
        })
        .unwrap_or_default();
    if !kulite_limits.contains("9 khz") || !kulite_limits.contains("wind-tunnel noise") {
        return Err(
            "JAXA Kulite approximately 9 kHz wind-tunnel-noise confounder must remain source-attributed"
                .to_owned(),
        );
    }

    let case_optional = case.get("required").and_then(Value::as_bool) == Some(false);
    if !case_optional {
        return Err("DPW8 Test Case 3 optional status was changed".to_owned());
    }

    Ok(json!({
        "schema_version": "aero-dynamic-observation-audit/0.1.0",
        "case_id": "DPW8-CRM-DYNAMIC-FSI-3",
        "case_status": "OPTIONAL",
        "source_admission": if source_bytes_all_admitted { "CHANNEL_BYTES_ADMITTED" } else { "NOT_ADMITTED" },
        "channel_count": channel_reports.len(),
        "channel_reports": channel_reports,
        "spectral_processing": {
            "status": if missing_spectral.is_empty() { "METADATA_PRESENT_REQUIRES_REVIEW" } else { "ABSTAIN_MISSING_SPECTRAL_METADATA" },
            "missing_fields": missing_spectral
        },
        "comparison_decisions": {
            "psd_comparison": "ABSTAIN_MISSING_SAMPLING_AND_ESTIMATOR_METADATA",
            "cross_channel_coherence": "ABSTAIN_NO_ADMITTED_COMMON_TIMEBASE_OR_SYNCHRONIZATION",
            "kulite_vs_upsp_raw_rms": "REJECT_DIRECT_EQUALITY_DIFFERENT_OBSERVATION_OPERATORS",
            "buffet_coefficient_c_b": "INDICATOR_ONLY_NOT_MODAL_OR_DURABILITY_EVIDENCE",
            "psp_simultaneous_drag": "NULL_NOT_IMPUTED",
            "quantitative_deformation": if official_deformation_pending { "NOT_ASSESSED_OFFICIAL_DEFORMATION_PENDING" } else { "REQUIRES_SOURCE_REVIEW" },
            "structural_modal_validation": "NOT_ASSESSED_NO_ADMITTED_STRUCTURAL_MODE_TIME_HISTORY"
        },
        "known_confounders": [{
            "instrument": "Kulite",
            "frequency_hz": 9000.0,
            "classification": "WIND_TUNNEL_NOISE",
            "promotion_to_structural_mode": false
        }],
        "preliminary_tip_deflection": {
            "value_mm": deformation.get("preliminary_tip_deflection_mm"),
            "angle_of_attack_deg": deformation.get("preliminary_angle_of_attack_deg"),
            "evidence_class": deformation.get("preliminary_evidence_class"),
            "quantitative_acceptance_threshold": deformation.get("allow_as_quantitative_acceptance_threshold")
        },
        "claim_ceiling": "measurement_operator_contract_and_source_admission_only"
    }))
}

fn synthetic_channel(
    id: &str,
    quantity: &str,
    unit: &str,
    spatial_operator: &str,
    temporal_operator: &str,
) -> ObservationChannel {
    ObservationChannel {
        channel_id: id.to_owned(),
        quantity: quantity.to_owned(),
        unit: unit.to_owned(),
        coordinate_frame: "CRM-jig-metric-v1".to_owned(),
        spatial_operator_id: spatial_operator.to_owned(),
        temporal_operator_id: temporal_operator.to_owned(),
        source_admitted: true,
        calibration_admitted: true,
        calibration_operator_id: Some("synthetic-calibration-A".to_owned()),
        standard_uncertainty: Some(0.01),
        uncertainty_budget_id: Some("synthetic-uncertainty-budget-A".to_owned()),
        mask_required: false,
        mask_applied: true,
        mask_operator_id: None,
        timebase_id: Some("synthetic-clock-A".to_owned()),
        sample_rate_hz: Some(20_000.0),
        synchronization_uncertainty_s: Some(1.0e-6),
        bandwidth_hz: Some([0.0, 8_000.0]),
        psd: PsdSpec {
            window: Some("hann".to_owned()),
            segment_length_samples: Some(2048),
            overlap_samples: Some(1024),
            detrend: Some("constant".to_owned()),
            estimator: Some("welch-mean".to_owned()),
            normalization: Some("density".to_owned()),
            output_unit: Some("Pa^2/Hz".to_owned()),
        },
        structural_mode_evidence_admitted: false,
    }
}

fn main() {
    let catalog: Value = match serde_json::from_str(DPW8_CATALOG) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("observation_catalog_parse_error={error}");
            std::process::exit(2);
        }
    };
    let report = match case3_audit(&catalog) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("observation_audit_error={error}");
            std::process::exit(2);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("JSON serialization")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_admitted_observation_operators_allow_rms_comparison() {
        let left = synthetic_channel(
            "pressure-measurement",
            "pressure",
            "Pa",
            "sensor-grid-A",
            "bandpass-A",
        );
        let right = synthetic_channel(
            "pressure-prediction-through-sensor",
            "pressure",
            "Pa",
            "sensor-grid-A",
            "bandpass-A",
        );
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Rms, 1.0e-5).status,
            DecisionStatus::Allow
        );
    }

    #[test]
    fn different_temporal_responses_reject_raw_rms_equality() {
        let left = synthetic_channel(
            "kulite",
            "pressure",
            "Pa",
            "sensor-grid-A",
            "broadband-kulite",
        );
        let right = synthetic_channel(
            "upsp",
            "pressure",
            "Pa",
            "sensor-grid-A",
            "camera-exposure-lowpass",
        );
        let result = assess_pair(&left, &right, ComparisonKind::Rms, 1.0e-5);
        assert_eq!(result.status, DecisionStatus::Reject);
        assert!(result.reasons[0].contains("temporal instrument responses differ"));
    }

    #[test]
    fn unadmitted_source_abstains_even_when_other_metadata_matches() {
        let left = synthetic_channel("raw-data", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        let mut right = left.clone();
        right.source_admitted = false;
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Rms, 1.0e-5).status,
            DecisionStatus::Abstain
        );
    }

    #[test]
    fn incompatible_units_reject() {
        let left = synthetic_channel("pressure", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        let mut right = left.clone();
        right.unit = "Cp".to_owned();
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Pointwise, 1.0e-5).status,
            DecisionStatus::Reject
        );
    }

    #[test]
    fn coordinate_frame_mismatch_rejects() {
        let left = synthetic_channel("pressure", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        let mut right = left.clone();
        right.coordinate_frame = "wind-tunnel-frame".to_owned();
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Pointwise, 1.0e-5).status,
            DecisionStatus::Reject
        );
    }

    #[test]
    fn missing_calibration_identity_or_uncertainty_budget_abstains() {
        let left = synthetic_channel("measurement", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        let mut no_calibration_id = left.clone();
        no_calibration_id.calibration_operator_id = None;
        assert_eq!(
            assess_pair(&left, &no_calibration_id, ComparisonKind::Rms, 1.0e-5).status,
            DecisionStatus::Abstain
        );
        let mut no_uncertainty = left.clone();
        no_uncertainty.standard_uncertainty = None;
        assert_eq!(
            assess_pair(&left, &no_uncertainty, ComparisonKind::Rms, 1.0e-5).status,
            DecisionStatus::Abstain
        );
        let mut no_budget = left.clone();
        no_budget.uncertainty_budget_id = None;
        assert_eq!(
            assess_pair(&left, &no_budget, ComparisonKind::Rms, 1.0e-5).status,
            DecisionStatus::Abstain
        );
    }

    #[test]
    fn different_reliability_masks_reject_direct_comparison() {
        let left = synthetic_channel("measurement", "Cp", "1", "surface-map-A", "camera-lowpass-A");
        let mut right = left.clone();
        right.mask_required = true;
        right.mask_operator_id = Some("uPSP-mask-rev-A".to_owned());
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Pointwise, 1.0e-5).status,
            DecisionStatus::Reject
        );
        let mut same_mask = left.clone();
        same_mask.mask_required = true;
        same_mask.mask_operator_id = Some("uPSP-mask-rev-A".to_owned());
        let mut prediction = same_mask.clone();
        prediction.mask_operator_id = Some("uPSP-mask-rev-B".to_owned());
        assert_eq!(
            assess_pair(&same_mask, &prediction, ComparisonKind::Pointwise, 1.0e-5).status,
            DecisionStatus::Reject
        );
    }

    #[test]
    fn required_surface_mask_must_be_applied() {
        let left = synthetic_channel(
            "upsp-measured",
            "Cp",
            "1",
            "surface-map-A",
            "camera-lowpass-A",
        );
        let mut right = left.clone();
        right.mask_required = true;
        right.mask_applied = false;
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Pointwise, 1.0e-5).status,
            DecisionStatus::Reject
        );
    }

    #[test]
    fn missing_sample_rate_or_psd_spec_abstains() {
        let left = synthetic_channel("a", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        let mut right = left.clone();
        right.sample_rate_hz = None;
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Psd, 1.0e-5).status,
            DecisionStatus::Abstain
        );
        let mut right = left.clone();
        right.psd.window = None;
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Psd, 1.0e-5).status,
            DecisionStatus::Abstain
        );
    }

    #[test]
    fn matching_complete_psd_contract_allows_comparison() {
        let left = synthetic_channel("a", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        let right = synthetic_channel("b", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Psd, 1.0e-5).status,
            DecisionStatus::Allow
        );
    }

    #[test]
    fn coherence_abstains_without_timebase_and_sync_uncertainty() {
        let left = synthetic_channel("a", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        let mut right = left.clone();
        right.timebase_id = None;
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Coherence, 1.0e-5).status,
            DecisionStatus::Abstain
        );
        let mut right = left.clone();
        right.synchronization_uncertainty_s = None;
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Coherence, 1.0e-5).status,
            DecisionStatus::Abstain
        );
    }

    #[test]
    fn coherence_allows_only_with_common_clock_and_bounded_sync_error() {
        let left = synthetic_channel("a", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        let right = synthetic_channel("b", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Coherence, 1.0e-5).status,
            DecisionStatus::Allow
        );
        let mut bad = right.clone();
        bad.synchronization_uncertainty_s = Some(1.0e-3);
        assert_eq!(
            assess_pair(&left, &bad, ComparisonKind::Coherence, 1.0e-5).status,
            DecisionStatus::Abstain
        );
    }

    #[test]
    fn modal_claim_needs_admitted_structural_mode_evidence() {
        let left = synthetic_channel("strain", "strain", "1", "root-gauge", "strain-gauge-H");
        let right = synthetic_channel("fsi", "strain", "1", "root-gauge", "strain-gauge-H");
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::ModalValidation, 1.0e-5).status,
            DecisionStatus::NotAssessed
        );
    }

    #[test]
    fn nine_khz_kulite_peak_is_source_attributed_confounder() {
        let confounds = vec![("Kulite".to_owned(), 8_800.0, 9_200.0)];
        assert_eq!(
            classify_jaxa_spectral_feature("Kulite", 9_000.0, &confounds),
            "SOURCE_ATTRIBUTED_CONFOUNDER_NOT_A_STRUCTURAL_MODE"
        );
        assert_eq!(
            classify_jaxa_spectral_feature("uPSP", 9_000.0, &confounds),
            "UNCLASSIFIED_SPECTRAL_FEATURE_REQUIRES_INDEPENDENT_EVIDENCE"
        );
    }

    #[test]
    fn buffet_coefficient_is_an_indicator_not_a_modal_result() {
        assert_eq!(
            classify_buffet_indicator("C_B"),
            "BUFFET_ONSET_INDICATOR_ONLY"
        );
    }

    #[test]
    fn case3_catalog_audit_rejects_kulite_noise_promotion() {
        let mut catalog: Value = serde_json::from_str(DPW8_CATALOG).unwrap();
        let cases = catalog["benchmark_cases"].as_array_mut().unwrap();
        let case = cases
            .iter_mut()
            .find(|item| item["case_id"] == "DPW8-CRM-DYNAMIC-FSI-3")
            .unwrap();
        let channels = case["measurement_channels"].as_array_mut().unwrap();
        let kulite = channels
            .iter_mut()
            .find(|item| item["channel_family"] == "unsteady_point_pressure")
            .unwrap();
        kulite["limitations"] = json!(["9 kHz is the primary structural mode"]);
        assert!(case3_audit(&catalog).is_err());
    }

    #[test]
    fn case3_catalog_audit_preserves_pending_and_optional_claims() {
        let catalog: Value = serde_json::from_str(DPW8_CATALOG).unwrap();
        let report = case3_audit(&catalog).unwrap();
        assert_eq!(report["case_status"], "OPTIONAL");
        assert_eq!(report["source_admission"], "NOT_ADMITTED");
        assert_eq!(
            report["comparison_decisions"]["psd_comparison"],
            "ABSTAIN_MISSING_SAMPLING_AND_ESTIMATOR_METADATA"
        );
        assert_eq!(
            report["comparison_decisions"]["quantitative_deformation"],
            "NOT_ASSESSED_OFFICIAL_DEFORMATION_PENDING"
        );
        assert_eq!(
            report["comparison_decisions"]["psp_simultaneous_drag"],
            "NULL_NOT_IMPUTED"
        );
    }

    #[test]
    fn common_bandwidth_must_be_identical() {
        let left = synthetic_channel("a", "pressure", "Pa", "sensor-grid-A", "bandpass-A");
        let mut right = left.clone();
        right.bandwidth_hz = Some([0.0, 5_000.0]);
        assert_eq!(
            assess_pair(&left, &right, ComparisonKind::Rms, 1.0e-5).status,
            DecisionStatus::Reject
        );
    }
}
