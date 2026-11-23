// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Configuration-bound conservative flight-energy envelope evaluator.
//!
//! This is a proposal qualification screen over a finite, declared operating
//! envelope. It never authorizes flight or physical actuation. Synthetic data
//! may exercise arithmetic and abstention logic, but cannot support an
//! operational qualification claim.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::battery_service_network::{EvidenceClass, EvidenceEnvelope};

const MAX_PHASES: usize = 16;
const MAX_CAMPAIGNS: usize = 64;
const MAX_ENERGY_WH: u64 = 1_000_000_000_000;
const MAX_PAYLOAD_G: u32 = 1_000_000;
const MAX_WIND_MILLI_MPS: u32 = 100_000;
const MAX_TEMPERATURE_MILLI_C: i32 = 150_000;
const MIN_HOLDOUT_SAMPLES: u32 = 100;
const MAX_HOLDOUT_SAMPLES: u32 = 10_000_000;
const MAX_MISSION_DURATION_SECONDS: u32 = 86_400;
const CLAIM_CEILING: &str = "Finite model-level conservative energy-envelope check only; no battery safety qualification, flight authorization, dispatch, return-to-service, aviation permission or physical performance claim.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnergyMeasurementBoundary {
    PackTerminal,
    DcBus,
    AcInput,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatingEnvelope {
    pub payload_mass_min_g: u32,
    pub payload_mass_max_g: u32,
    /// Signed wind component along the aircraft's direction of travel.
    pub headwind_min_milli_mps: i32,
    pub headwind_max_milli_mps: i32,
    pub crosswind_abs_max_milli_mps: u32,
    pub gust_abs_max_milli_mps: u32,
    pub ambient_temperature_min_milli_c: i32,
    pub ambient_temperature_max_milli_c: i32,
    pub battery_temperature_min_milli_c: i32,
    pub battery_temperature_max_milli_c: i32,
    pub state_of_health_min_permille: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MissionEnergyPhase {
    pub phase_id: String,
    pub duration_seconds: u32,
    pub energy_upper_bound_wh: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HoldoutQualification {
    pub campaign_ids: Vec<String>,
    pub sample_manifest_digest: String,
    pub total_samples: u32,
    pub covered_samples: u32,
    pub required_lower_coverage_permille: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlightEnergyModel {
    pub model_id: String,
    pub model_digest: String,
    pub aircraft_id: String,
    pub aircraft_configuration_digest: String,
    pub pack_id: String,
    pub pack_configuration_digest: String,
    pub bms_firmware_digest: String,
    pub route_profile_digest: String,
    pub measurement_boundary: EnergyMeasurementBoundary,
    pub operating_envelope: OperatingEnvelope,
    pub mission_duration_seconds: u32,
    pub mission_energy_upper_bound_wh: u64,
    pub phases: Vec<MissionEnergyPhase>,
    pub holdout: HoldoutQualification,
    pub model_evidence: EvidenceEnvelope,
    pub aircraft_configuration_evidence: EvidenceEnvelope,
    pub pack_configuration_evidence: EvidenceEnvelope,
    pub bms_configuration_evidence: EvidenceEnvelope,
    pub route_profile_evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlightEnergyCandidate {
    pub mission_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub aircraft_id: String,
    pub aircraft_configuration_digest: String,
    pub pack_id: String,
    pub pack_configuration_digest: String,
    pub bms_firmware_digest: String,
    pub route_profile_digest: String,
    pub measurement_boundary: EnergyMeasurementBoundary,
    pub mission_start_unix_ms: u64,
    pub mission_end_unix_ms: u64,
    pub payload_mass_g: u32,
    pub headwind_milli_mps: i32,
    pub crosswind_abs_milli_mps: u32,
    pub gust_abs_milli_mps: u32,
    pub ambient_temperature_milli_c: i32,
    pub battery_temperature_milli_c: i32,
    pub state_of_health_permille: u16,
    pub available_energy_lower_bound_wh: u64,
    pub mission_reserve_wh: u64,
    pub safety_outage_reserve_wh: u64,
    pub uncertainty_allowance_wh: u64,
    pub payload_evidence: EvidenceEnvelope,
    pub weather_evidence: EvidenceEnvelope,
    pub battery_temperature_evidence: EvidenceEnvelope,
    pub state_of_health_evidence: EvidenceEnvelope,
    pub available_energy_evidence: EvidenceEnvelope,
    pub reserve_evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlightEnergyEnvelopeInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub decision_time_unix_ms: u64,
    pub model: FlightEnergyModel,
    pub candidate: FlightEnergyCandidate,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FlightEnergyEnvelopeReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub mission_id: String,
    pub model_id: String,
    pub model_digest: String,
    pub aircraft_id: String,
    pub pack_id: String,
    pub measurement_boundary: EnergyMeasurementBoundary,
    pub model_energy_upper_bound_wh: u64,
    pub phase_energy_sum_wh: u64,
    pub reserve_total_wh: u64,
    pub required_energy_wh: u64,
    pub available_energy_lower_bound_wh: u64,
    pub energy_margin_wh: i64,
    pub holdout_sample_count: u32,
    pub holdout_covered_count: u32,
    pub holdout_lower_coverage_bound_permille: u16,
    pub required_lower_coverage_permille: u16,
    pub in_operating_envelope: bool,
    pub evidence_current: bool,
    pub all_evidence_source_grade: bool,
    pub all_evidence_synthetic: bool,
    pub candidate_model_feasible: bool,
    pub abstention_reasons: Vec<String>,
    pub flight_authority_granted: bool,
    pub operational_qualification_granted: bool,
    pub claim_ceiling: &'static str,
}

fn add_error(errors: &mut Vec<String>, value: impl Into<String>) {
    errors.push(value.into());
}

fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn evidence_covers(
    evidence: &EvidenceEnvelope,
    asset_id: &str,
    configuration_digest: &str,
    expected_reference: Option<&str>,
    decision_time_unix_ms: u64,
    mission_start_unix_ms: u64,
    mission_end_unix_ms: u64,
) -> bool {
    if evidence.asset_id != asset_id
        || evidence.configuration_digest != configuration_digest
        || evidence.asset_id.trim().is_empty()
        || evidence.configuration_digest.trim().is_empty()
        || evidence.producer_id.trim().is_empty()
        || evidence.method_id.trim().is_empty()
        || evidence.evidence_reference.trim().is_empty()
        || expected_reference.is_some_and(|value| evidence.evidence_reference != value)
        || evidence.evidence_class == EvidenceClass::Unknown
    {
        return false;
    }

    match (
        evidence.observed_at_unix_ms,
        evidence.valid_from_unix_ms,
        evidence.valid_until_unix_ms,
    ) {
        (Some(observed), Some(valid_from), Some(valid_until)) => {
            valid_from <= observed
                && observed <= decision_time_unix_ms
                && valid_from <= mission_start_unix_ms
                && mission_end_unix_ms <= valid_until
        }
        _ => false,
    }
}

fn is_source_grade(evidence: &EvidenceEnvelope) -> bool {
    matches!(
        evidence.evidence_class,
        EvidenceClass::Measured | EvidenceClass::IndependentTest
    )
}

fn wilson_lower_bound_permille(successes: u32, total: u32) -> Option<u16> {
    if total == 0 || successes > total {
        return None;
    }
    let n = f64::from(total);
    let p = f64::from(successes) / n;
    // Two-sided 95% Wilson lower confidence bound (z = 1.9599639845).
    let z = 1.959_963_984_540_054_f64;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = p + z2 / (2.0 * n);
    let spread = z
        * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt());
    let lower = ((center - spread) / denominator).clamp(0.0, 1.0);
    let permille = (lower * 1_000.0).floor();
    u16::try_from(permille as u32).ok()
}

fn signed_margin(available: u64, required: u64) -> i64 {
    if available >= required {
        i64::try_from(available - required).unwrap_or(i64::MAX)
    } else {
        -i64::try_from(required - available).unwrap_or(i64::MAX)
    }
}

fn validate_operating_envelope(
    envelope: &OperatingEnvelope,
    errors: &mut Vec<String>,
) {
    if envelope.payload_mass_min_g > envelope.payload_mass_max_g
        || envelope.payload_mass_max_g > MAX_PAYLOAD_G
        || envelope.headwind_min_milli_mps > envelope.headwind_max_milli_mps
        || envelope.headwind_min_milli_mps.unsigned_abs() > MAX_WIND_MILLI_MPS
        || envelope.headwind_max_milli_mps.unsigned_abs() > MAX_WIND_MILLI_MPS
        || envelope.crosswind_abs_max_milli_mps > MAX_WIND_MILLI_MPS
        || envelope.gust_abs_max_milli_mps > MAX_WIND_MILLI_MPS
        || envelope.ambient_temperature_min_milli_c
            > envelope.ambient_temperature_max_milli_c
        || envelope.ambient_temperature_min_milli_c.abs() > MAX_TEMPERATURE_MILLI_C
        || envelope.ambient_temperature_max_milli_c.abs() > MAX_TEMPERATURE_MILLI_C
        || envelope.battery_temperature_min_milli_c
            > envelope.battery_temperature_max_milli_c
        || envelope.battery_temperature_min_milli_c.abs() > MAX_TEMPERATURE_MILLI_C
        || envelope.battery_temperature_max_milli_c.abs() > MAX_TEMPERATURE_MILLI_C
        || envelope.state_of_health_min_permille == 0
        || envelope.state_of_health_min_permille > 1_000
    {
        add_error(errors, "model_operating_envelope_bounds_invalid");
    }
}

fn candidate_inside_envelope(
    envelope: &OperatingEnvelope,
    candidate: &FlightEnergyCandidate,
) -> bool {
    candidate.payload_mass_g >= envelope.payload_mass_min_g
        && candidate.payload_mass_g <= envelope.payload_mass_max_g
        && candidate.headwind_milli_mps >= envelope.headwind_min_milli_mps
        && candidate.headwind_milli_mps <= envelope.headwind_max_milli_mps
        && candidate.crosswind_abs_milli_mps <= envelope.crosswind_abs_max_milli_mps
        && candidate.gust_abs_milli_mps <= envelope.gust_abs_max_milli_mps
        && candidate.ambient_temperature_milli_c >= envelope.ambient_temperature_min_milli_c
        && candidate.ambient_temperature_milli_c <= envelope.ambient_temperature_max_milli_c
        && candidate.battery_temperature_milli_c >= envelope.battery_temperature_min_milli_c
        && candidate.battery_temperature_milli_c <= envelope.battery_temperature_max_milli_c
        && candidate.state_of_health_permille >= envelope.state_of_health_min_permille
        && candidate.state_of_health_permille <= 1_000
}

pub fn evaluate_flight_energy_envelope(
    input: &FlightEnergyEnvelopeInput,
) -> Result<FlightEnergyEnvelopeReport, Vec<String>> {
    let mut errors = Vec::new();
    let model = &input.model;
    let candidate = &input.candidate;

    if input.schema_version != "0.1.0"
        || input.benchmark_id != "SUSTAINABLE-FLIGHT-ENERGY-ENVELOPE-001"
        || input.site_id.trim().is_empty()
        || input.site_configuration_digest.trim().is_empty()
        || input.decision_time_unix_ms == 0
        || input.claim_ceiling.trim().is_empty()
    {
        add_error(&mut errors, "flight_energy_input_identity_invalid");
    }

    let digest_fields = [
        model.model_digest.as_str(),
        model.aircraft_configuration_digest.as_str(),
        model.pack_configuration_digest.as_str(),
        model.bms_firmware_digest.as_str(),
        model.route_profile_digest.as_str(),
        candidate.aircraft_configuration_digest.as_str(),
        candidate.pack_configuration_digest.as_str(),
        candidate.bms_firmware_digest.as_str(),
        candidate.route_profile_digest.as_str(),
    ];
    if digest_fields.iter().any(|value| !is_sha256_digest(value)) {
        add_error(&mut errors, "flight_energy_configuration_digest_invalid");
    }

    if model.model_id.trim().is_empty()
        || model.aircraft_id.trim().is_empty()
        || model.pack_id.trim().is_empty()
        || model.mission_duration_seconds == 0
        || model.mission_duration_seconds > MAX_MISSION_DURATION_SECONDS
        || model.mission_energy_upper_bound_wh == 0
        || model.mission_energy_upper_bound_wh > MAX_ENERGY_WH
        || model.phases.is_empty()
        || model.phases.len() > MAX_PHASES
        || model.holdout.campaign_ids.is_empty()
        || model.holdout.campaign_ids.len() > MAX_CAMPAIGNS
        || model.holdout.total_samples < MIN_HOLDOUT_SAMPLES
        || model.holdout.total_samples > MAX_HOLDOUT_SAMPLES
        || model.holdout.covered_samples > model.holdout.total_samples
        || !(900..=1_000).contains(&model.holdout.required_lower_coverage_permille)
        || !is_sha256_digest(&model.holdout.sample_manifest_digest)
    {
        add_error(&mut errors, "flight_energy_model_identity_or_bounds_invalid");
    }
    validate_operating_envelope(&model.operating_envelope, &mut errors);

    if candidate.mission_id.trim().is_empty()
        || candidate.site_id != input.site_id
        || candidate.site_configuration_digest != input.site_configuration_digest
        || candidate.aircraft_id != model.aircraft_id
        || candidate.aircraft_configuration_digest != model.aircraft_configuration_digest
        || candidate.pack_id != model.pack_id
        || candidate.pack_configuration_digest != model.pack_configuration_digest
        || candidate.bms_firmware_digest != model.bms_firmware_digest
        || candidate.route_profile_digest != model.route_profile_digest
        || candidate.measurement_boundary != model.measurement_boundary
        || candidate.mission_start_unix_ms <= input.decision_time_unix_ms
        || candidate.mission_end_unix_ms <= candidate.mission_start_unix_ms
        || candidate.mission_end_unix_ms - candidate.mission_start_unix_ms
            > u64::from(model.mission_duration_seconds) * 1_000
        || candidate.payload_mass_g > MAX_PAYLOAD_G
        || candidate.headwind_milli_mps.unsigned_abs() > MAX_WIND_MILLI_MPS
        || candidate.crosswind_abs_milli_mps > MAX_WIND_MILLI_MPS
        || candidate.gust_abs_milli_mps > MAX_WIND_MILLI_MPS
        || candidate.ambient_temperature_milli_c.abs() > MAX_TEMPERATURE_MILLI_C
        || candidate.battery_temperature_milli_c.abs() > MAX_TEMPERATURE_MILLI_C
        || candidate.state_of_health_permille == 0
        || candidate.state_of_health_permille > 1_000
        || candidate.available_energy_lower_bound_wh > MAX_ENERGY_WH
        || candidate.available_energy_lower_bound_wh == 0
        || candidate.mission_reserve_wh > MAX_ENERGY_WH
        || candidate.safety_outage_reserve_wh > MAX_ENERGY_WH
        || candidate.uncertainty_allowance_wh > MAX_ENERGY_WH
    {
        add_error(&mut errors, "flight_energy_candidate_identity_or_bounds_invalid");
    }

    if model.measurement_boundary != candidate.measurement_boundary {
        add_error(&mut errors, "flight_energy_measurement_boundary_mismatch");
    }

    let mut phase_ids = BTreeSet::new();
    let mut phase_total = 0_u64;
    let mut phase_duration = 0_u64;
    for phase in &model.phases {
        if phase.phase_id.trim().is_empty()
            || !phase_ids.insert(phase.phase_id.as_str())
            || phase.duration_seconds == 0
            || phase.duration_seconds > MAX_MISSION_DURATION_SECONDS
            || phase.energy_upper_bound_wh == 0
            || phase.energy_upper_bound_wh > MAX_ENERGY_WH
        {
            add_error(
                &mut errors,
                format!("flight_energy_phase_identity_or_bounds_invalid:{}", phase.phase_id),
            );
            continue;
        }
        phase_total = match phase_total.checked_add(phase.energy_upper_bound_wh) {
            Some(value) => value,
            None => {
                add_error(&mut errors, "flight_energy_phase_sum_overflow");
                phase_total
            }
        };
        phase_duration = match phase_duration.checked_add(u64::from(phase.duration_seconds)) {
            Some(value) => value,
            None => {
                add_error(&mut errors, "flight_energy_phase_duration_overflow");
                phase_duration
            }
        };
    }
    if phase_total != model.mission_energy_upper_bound_wh
        || phase_duration != u64::from(model.mission_duration_seconds)
    {
        add_error(&mut errors, "flight_energy_phase_profile_does_not_match_mission");
    }

    let mut campaign_ids = BTreeSet::new();
    for campaign in &model.holdout.campaign_ids {
        if campaign.trim().is_empty() || !campaign_ids.insert(campaign.as_str()) {
            add_error(&mut errors, "flight_energy_holdout_campaign_id_empty_or_duplicate");
        }
    }
    let holdout_lower = wilson_lower_bound_permille(
        model.holdout.covered_samples,
        model.holdout.total_samples,
    );
    if holdout_lower.is_none_or(|value| value < model.holdout.required_lower_coverage_permille) {
        add_error(&mut errors, "flight_energy_holdout_coverage_lower_bound_below_target");
    }

    let evidence_end = candidate.mission_end_unix_ms;
    let model_evidence = [
        (
            &model.model_evidence,
            model.model_id.as_str(),
            model.model_digest.as_str(),
            Some(model.holdout.sample_manifest_digest.as_str()),
        ),
        (
            &model.aircraft_configuration_evidence,
            model.aircraft_id.as_str(),
            model.aircraft_configuration_digest.as_str(),
            None,
        ),
        (
            &model.pack_configuration_evidence,
            model.pack_id.as_str(),
            model.pack_configuration_digest.as_str(),
            None,
        ),
        (
            &model.bms_configuration_evidence,
            model.pack_id.as_str(),
            model.bms_firmware_digest.as_str(),
            None,
        ),
        (
            &model.route_profile_evidence,
            model.route_profile_digest.as_str(),
            model.route_profile_digest.as_str(),
            None,
        ),
    ];
    let mut all_source_grade = true;
    let mut all_synthetic = true;
    for (evidence, asset, digest, reference) in model_evidence {
        if !evidence_covers(
            evidence,
            asset,
            digest,
            reference,
            input.decision_time_unix_ms,
            candidate.mission_start_unix_ms,
            evidence_end,
        ) {
            add_error(
                &mut errors,
                format!("flight_energy_model_evidence_invalid:{}", evidence.evidence_reference),
            );
        }
        all_source_grade &= is_source_grade(evidence);
        all_synthetic &= evidence.evidence_class == EvidenceClass::Synthetic;
    }

    let candidate_evidence = [
        (
            &candidate.payload_evidence,
            candidate.mission_id.as_str(),
            model.route_profile_digest.as_str(),
        ),
        (
            &candidate.weather_evidence,
            candidate.site_id.as_str(),
            candidate.site_configuration_digest.as_str(),
        ),
        (
            &candidate.battery_temperature_evidence,
            candidate.pack_id.as_str(),
            candidate.pack_configuration_digest.as_str(),
        ),
        (
            &candidate.state_of_health_evidence,
            candidate.pack_id.as_str(),
            candidate.pack_configuration_digest.as_str(),
        ),
        (
            &candidate.available_energy_evidence,
            candidate.pack_id.as_str(),
            candidate.pack_configuration_digest.as_str(),
        ),
        (
            &candidate.reserve_evidence,
            candidate.mission_id.as_str(),
            model.route_profile_digest.as_str(),
        ),
    ];
    for (evidence, asset, digest) in candidate_evidence {
        if !evidence_covers(
            evidence,
            asset,
            digest,
            None,
            input.decision_time_unix_ms,
            candidate.mission_start_unix_ms,
            evidence_end,
        ) {
            add_error(
                &mut errors,
                format!("flight_energy_candidate_evidence_invalid:{}", evidence.evidence_reference),
            );
        }
        all_source_grade &= is_source_grade(evidence);
        all_synthetic &= evidence.evidence_class == EvidenceClass::Synthetic;
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let reserve_total = candidate
        .mission_reserve_wh
        .checked_add(candidate.safety_outage_reserve_wh)
        .and_then(|value| value.checked_add(candidate.uncertainty_allowance_wh))
        .ok_or_else(|| vec!["flight_energy_reserve_sum_overflow".to_owned()])?;
    let required_energy = model
        .mission_energy_upper_bound_wh
        .checked_add(reserve_total)
        .ok_or_else(|| vec!["flight_energy_required_energy_overflow".to_owned()])?;
    let in_operating_envelope =
        candidate_inside_envelope(&model.operating_envelope, candidate);
    let mut abstention_reasons = BTreeSet::new();
    if !in_operating_envelope {
        abstention_reasons.insert("candidate_outside_measured_operating_envelope".to_owned());
    }
    if candidate.available_energy_lower_bound_wh < required_energy {
        abstention_reasons.insert("available_energy_below_mission_plus_reserves".to_owned());
    }
    if !all_source_grade {
        abstention_reasons.insert("evidence_not_entirely_measured_or_independent_test".to_owned());
    }

    let candidate_model_feasible = in_operating_envelope
        && candidate.available_energy_lower_bound_wh >= required_energy;
    Ok(FlightEnergyEnvelopeReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        mission_id: candidate.mission_id.clone(),
        model_id: model.model_id.clone(),
        model_digest: model.model_digest.clone(),
        aircraft_id: model.aircraft_id.clone(),
        pack_id: model.pack_id.clone(),
        measurement_boundary: model.measurement_boundary,
        model_energy_upper_bound_wh: model.mission_energy_upper_bound_wh,
        phase_energy_sum_wh: phase_total,
        reserve_total_wh: reserve_total,
        required_energy_wh: required_energy,
        available_energy_lower_bound_wh: candidate.available_energy_lower_bound_wh,
        energy_margin_wh: signed_margin(candidate.available_energy_lower_bound_wh, required_energy),
        holdout_sample_count: model.holdout.total_samples,
        holdout_covered_count: model.holdout.covered_samples,
        holdout_lower_coverage_bound_permille: holdout_lower.unwrap_or(0),
        required_lower_coverage_permille: model.holdout.required_lower_coverage_permille,
        in_operating_envelope,
        evidence_current: true,
        all_evidence_source_grade: all_source_grade,
        all_evidence_synthetic: all_synthetic,
        candidate_model_feasible,
        abstention_reasons: abstention_reasons.into_iter().collect(),
        flight_authority_granted: false,
        operational_qualification_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> FlightEnergyEnvelopeInput {
        serde_json::from_str(include_str!(
            "../fixtures/sustainable_flight_energy_envelope_v0_1.json"
        ))
        .expect("frozen flight energy fixture must parse")
    }

    #[test]
    fn wilson_lower_bound_is_conservative_and_monotonic() {
        let lower = wilson_lower_bound_permille(9_995, 10_000).expect("coverage bound");
        assert!(lower < 1_000);
        assert!(lower >= 990);
        assert!(
            wilson_lower_bound_permille(9_990, 10_000).expect("lower coverage")
                < lower
        );
    }

    #[test]
    fn synthetic_known_answer_reports_model_feasibility_without_authority() {
        let report = evaluate_flight_energy_envelope(&fixture()).expect("synthetic fixture");
        assert!(report.candidate_model_feasible);
        assert!(report.in_operating_envelope);
        assert_eq!(report.model_energy_upper_bound_wh, 1_200);
        assert_eq!(report.phase_energy_sum_wh, 1_200);
        assert_eq!(report.reserve_total_wh, 400);
        assert_eq!(report.required_energy_wh, 1_600);
        assert_eq!(report.available_energy_lower_bound_wh, 1_800);
        assert_eq!(report.energy_margin_wh, 200);
        assert!(report.all_evidence_synthetic);
        assert!(!report.all_evidence_source_grade);
        assert!(!report.flight_authority_granted);
        assert!(!report.operational_qualification_granted);
        assert!(!report.abstention_reasons.is_empty());
    }

    #[test]
    fn insufficient_energy_fails_closed() {
        let mut input = fixture();
        input.candidate.available_energy_lower_bound_wh = 1_599;
        let report = evaluate_flight_energy_envelope(&input).expect("model remains structurally valid");
        assert!(!report.candidate_model_feasible);
        assert!(report.abstention_reasons.iter().any(|reason| {
            reason == "available_energy_below_mission_plus_reserves"
        }));
    }

    #[test]
    fn payload_outside_envelope_is_rejected_as_infeasible() {
        let mut input = fixture();
        input.candidate.payload_mass_g = 2_001;
        let report = evaluate_flight_energy_envelope(&input).expect("operating point can be evaluated");
        assert!(!report.in_operating_envelope);
        assert!(!report.candidate_model_feasible);
    }

    #[test]
    fn headwind_outside_envelope_abstains() {
        let mut input = fixture();
        input.candidate.headwind_milli_mps = 8_001;
        let report = evaluate_flight_energy_envelope(&input).expect("headwind is bounded");
        assert!(!report.candidate_model_feasible);
    }

    #[test]
    fn crosswind_or_gust_outside_envelope_abstains() {
        let mut input = fixture();
        input.candidate.crosswind_abs_milli_mps = 4_001;
        assert!(
            !evaluate_flight_energy_envelope(&input)
                .expect("crosswind is bounded")
                .candidate_model_feasible
        );
        let mut input = fixture();
        input.candidate.gust_abs_milli_mps = 8_001;
        assert!(
            !evaluate_flight_energy_envelope(&input)
                .expect("gust is bounded")
                .candidate_model_feasible
        );
    }

    #[test]
    fn temperature_or_battery_health_outside_envelope_abstains() {
        let mut input = fixture();
        input.candidate.ambient_temperature_milli_c = 45_001;
        assert!(!evaluate_flight_energy_envelope(&input).expect("ambient").candidate_model_feasible);
        let mut input = fixture();
        input.candidate.battery_temperature_milli_c = 50_001;
        assert!(!evaluate_flight_energy_envelope(&input).expect("battery temp").candidate_model_feasible);
        let mut input = fixture();
        input.candidate.state_of_health_permille = 899;
        assert!(!evaluate_flight_energy_envelope(&input).expect("soh").candidate_model_feasible);
    }

    #[test]
    fn aircraft_pack_bms_route_and_measurement_boundary_must_match_exactly() {
        let mut input = fixture();
        input.candidate.aircraft_configuration_digest = format!("sha256:{}", "f".repeat(64));
        assert!(evaluate_flight_energy_envelope(&input).is_err());
        let mut input = fixture();
        input.candidate.bms_firmware_digest = format!("sha256:{}", "e".repeat(64));
        assert!(evaluate_flight_energy_envelope(&input).is_err());
        let mut input = fixture();
        input.candidate.route_profile_digest = format!("sha256:{}", "d".repeat(64));
        assert!(evaluate_flight_energy_envelope(&input).is_err());
        let mut input = fixture();
        input.candidate.measurement_boundary = EnergyMeasurementBoundary::DcBus;
        assert!(evaluate_flight_energy_envelope(&input).is_err());
    }

    #[test]
    fn phase_energy_and_duration_must_sum_to_declared_mission() {
        let mut input = fixture();
        input.model.phases[0].energy_upper_bound_wh += 1;
        assert!(evaluate_flight_energy_envelope(&input).is_err());
        let mut input = fixture();
        input.model.phases[0].duration_seconds += 1;
        assert!(evaluate_flight_energy_envelope(&input).is_err());
    }

    #[test]
    fn source_evidence_must_cover_future_mission_without_future_observation() {
        let mut input = fixture();
        input.model.model_evidence.observed_at_unix_ms =
            Some(input.candidate.mission_start_unix_ms + 1);
        assert!(evaluate_flight_energy_envelope(&input).is_err());
        let mut input = fixture();
        input.candidate.weather_evidence.valid_until_unix_ms =
            Some(input.candidate.mission_end_unix_ms - 1);
        assert!(evaluate_flight_energy_envelope(&input).is_err());
    }

    #[test]
    fn source_grade_evidence_is_required_for_operational_support() {
        let mut input = fixture();
        for evidence in [
            &mut input.model.model_evidence,
            &mut input.model.aircraft_configuration_evidence,
            &mut input.model.pack_configuration_evidence,
            &mut input.model.bms_configuration_evidence,
            &mut input.model.route_profile_evidence,
            &mut input.candidate.payload_evidence,
            &mut input.candidate.weather_evidence,
            &mut input.candidate.battery_temperature_evidence,
            &mut input.candidate.state_of_health_evidence,
            &mut input.candidate.available_energy_evidence,
            &mut input.candidate.reserve_evidence,
        ] {
            evidence.evidence_class = EvidenceClass::Measured;
        }
        let report = evaluate_flight_energy_envelope(&input).expect("evidence classes changed only");
        assert!(report.all_evidence_source_grade);
        assert!(report.candidate_model_feasible);
        assert!(!report.operational_qualification_granted);
    }

    #[test]
    fn duplicate_holdout_campaign_ids_and_invalid_counts_are_rejected() {
        let mut input = fixture();
        input.model.holdout.campaign_ids[1] =
            input.model.holdout.campaign_ids[0].clone();
        assert!(evaluate_flight_energy_envelope(&input).is_err());
        let mut input = fixture();
        input.model.holdout.covered_samples = input.model.holdout.total_samples + 1;
        assert!(evaluate_flight_energy_envelope(&input).is_err());
    }

    #[test]
    fn insufficient_statistical_coverage_bound_is_rejected() {
        let mut input = fixture();
        input.model.holdout.covered_samples = 9_900;
        let errors = evaluate_flight_energy_envelope(&input)
            .expect_err("Wilson lower bound falls below target");
        assert!(errors.iter().any(|error| {
            error == "flight_energy_holdout_coverage_lower_bound_below_target"
        }));
    }

    #[test]
    fn future_or_stale_observation_is_rejected_for_each_input_class() {
        let mut input = fixture();
        input.candidate.available_energy_evidence.observed_at_unix_ms =
            Some(input.decision_time_unix_ms + 1);
        assert!(evaluate_flight_energy_envelope(&input).is_err());
        let mut input = fixture();
        input.candidate.state_of_health_evidence.valid_until_unix_ms =
            Some(input.candidate.mission_end_unix_ms - 1);
        assert!(evaluate_flight_energy_envelope(&input).is_err());
    }

    #[test]
    fn model_sample_manifest_reference_must_match_evidence() {
        let mut input = fixture();
        input.model.model_evidence.evidence_reference = "sha256:other".to_owned();
        assert!(evaluate_flight_energy_envelope(&input).is_err());
    }
}
