use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::{env, fs, path::Path};

const DEFAULT_PATH: &str =
    "docs/research/port-energy-compute/OCEAN_ENVIRONMENT_MEASUREMENT_BUNDLE_V0_1.json";
const REQUIRED_ROLES: [&str; 5] = [
    "intake", "outfall", "near_field_plume", "far_field_impact",
    "independent_reference_control",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    schema_version: String,
    fixture_id: String,
    as_of: String,
    synthetic_only: bool,
    claim_ceiling: String,
    standards_context: StandardsContext,
    station_roles: Vec<String>,
    pairing_contract: PairingContract,
    equation_contract: EquationContract,
    ogc_sensorthings_mapping: OgcMapping,
    uncertainty_simulation: UncertaintySimulation,
    interval_records: Vec<IntervalRecord>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StandardsContext {
    uncertainty_method: String,
    uncertainty_method_reference: String,
    nonlinear_model_policy: String,
    water_sampling_reference: String,
    water_sampling_url: String,
    observation_interoperability: String,
    observation_interoperability_url: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PairingContract {
    timestamp_format: String,
    required_time_sync_state: String,
    maximum_clock_offset_ms: u64,
    pair_rule: String,
    flow_closure_boundary: FlowClosureBoundary,
    flow_closure_screen: WaterBalanceScreen,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FlowClosureBoundary {
    mode: String,
    included_streams: Vec<String>,
    excluded_side_streams: Vec<String>,
    storage_change_assumption: String,
    boundary_completeness_state: String,
    rule: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WaterBalanceScreen {
    method: String,
    maximum_absolute_z_score: f64,
    purpose: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EquationContract {
    thermal_power_kw: String,
    temperature_difference_standard_uncertainty_c: String,
    thermal_power_standard_uncertainty_kw: String,
    pollutant_mass_flux_g_s: String,
    pollutant_mass_flux_kg_day: String,
    net_pollutant_mass_flux_g_s: String,
    net_pollutant_mass_flux_standard_uncertainty_g_s: String,
    covariance_assumption: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OgcMapping {
    standard: String,
    standard_url: String,
    publication_date: String,
    entity_mapping: EntityMapping,
    datastream_result_encoding: DatastreamResultEncoding,
    data_model_policy: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntityMapping {
    station_or_platform: String,
    station_location: String,
    sensor_and_calibration_identity: String,
    measured_property: String,
    sensor_and_property_series: String,
    individual_measurement: String,
    station_or_sampled_environment_feature: String,
    observation_proximate_feature_link: String,
    datastream_proximate_feature_link: String,
    datastream_ultimate_feature_link: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatastreamResultEncoding {
    result_type: String,
    result_encoding: String,
    unit_location: String,
    legacy_unit_of_measurement: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UncertaintySimulation {
    framework: String,
    framework_references: Vec<String>,
    algorithm: String,
    seed: u32,
    draws_per_interval: usize,
    lower_95_percentile: f64,
    upper_95_percentile: f64,
    maximum_relative_standard_uncertainty_difference_from_first_order: f64,
    distribution_assumption: String,
    acceptance_rule: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntervalRecord {
    interval_id: String,
    interval_start_utc: String,
    interval_end_utc: String,
    outfall_window_start_utc: String,
    outfall_window_end_utc: String,
    hydraulic_pairing_state: String,
    hydraulic_residence_time_seconds: u64,
    hydraulic_residence_time_standard_uncertainty_seconds: f64,
    transport_lag_basis: String,
    time_sync_state: String,
    measured_clock_offset_ms: u64,
    maximum_clock_offset_ms: u64,
    rights_state: String,
    quality_state: String,
    evidence_state: String,
    uncertainty_model_state: String,
    uncertainty_covariance_state: String,
    fluid_properties: FluidProperties,
    thermal_observations: Vec<ThermalObservation>,
    flow_samples: Vec<FlowSample>,
    concentration_pairs: Vec<ConcentrationPair>,
    plume_concentration_observations: Vec<PlumeConcentrationObservation>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FluidProperties {
    density_kg_m3: f64,
    density_standard_uncertainty_kg_m3: f64,
    specific_heat_capacity_j_kg_k: f64,
    cp_standard_uncertainty_j_kg_k: f64,
    property_source: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    sample_id: String,
    interval_id: String,
    interval_start_utc: String,
    interval_end_utc: String,
    sensor_id: String,
    calibration_id: String,
    calibration_state: String,
    quality_state: String,
    evidence_state: String,
    source_rights_state: String,
    calibration_independence_domain: String,
    calibration_valid_from_utc: String,
    calibration_valid_until_utc: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThermalObservation {
    evidence: Evidence,
    station_id: String,
    station_role: String,
    distance_from_outfall_m: f64,
    position_evidence_state: String,
    temperature_deg_c: f64,
    standard_uncertainty_deg_c: f64,
    temporal_rate_deg_c_per_s: f64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FlowSample {
    evidence: Evidence,
    station_id: String,
    station_role: String,
    flow_m3_s: f64,
    standard_uncertainty_m3_s: f64,
    temporal_rate_m3_s_per_s: f64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConcentrationPair {
    analyte_id: String,
    method_id: String,
    method_quality_state: String,
    intake: ConcentrationSample,
    outfall: ConcentrationSample,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConcentrationSample {
    evidence: Evidence,
    station_id: String,
    station_role: String,
    concentration_mg_l: f64,
    standard_uncertainty_mg_l: f64,
    detection_limit_mg_l: f64,
    sample_collection_method: String,
    sample_preservation_state: String,
    chain_of_custody_id: String,
    laboratory_batch_id: String,
    temporal_rate_mg_l_per_s: f64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlumeConcentrationObservation {
    analyte_id: String,
    method_id: String,
    method_quality_state: String,
    evidence: Evidence,
    station_id: String,
    station_role: String,
    concentration_mg_l: f64,
    standard_uncertainty_mg_l: f64,
    detection_limit_mg_l: f64,
    sample_collection_method: String,
    sample_preservation_state: String,
    chain_of_custody_id: String,
    laboratory_batch_id: String,
    temporal_rate_mg_l_per_s: f64,
}

#[derive(Debug, Serialize)]
struct StationAnomaly {
    station_role: String,
    distance_from_outfall_m: f64,
    delta_from_reference_deg_c: f64,
    standard_uncertainty_deg_c: f64,
    hydraulic_alignment_standard_uncertainty_deg_c: f64,
}
#[derive(Debug, Serialize)]
struct StationConcentrationAnomaly {
    analyte_id: String,
    station_role: String,
    distance_from_outfall_m: f64,
    delta_from_reference_mg_l: f64,
    standard_uncertainty_mg_l: f64,
    hydraulic_alignment_standard_uncertainty_mg_l: f64,
}
#[derive(Debug, Serialize)]
struct PollutantResult {
    analyte_id: String,
    intake_g_s: f64,
    intake_standard_uncertainty_g_s: f64,
    outfall_g_s: f64,
    outfall_standard_uncertainty_g_s: f64,
    outfall_hydraulic_alignment_standard_uncertainty_g_s: f64,
    net_added_g_s: f64,
    net_added_standard_uncertainty_g_s: f64,
    net_added_kg_day: f64,
    net_added_standard_uncertainty_kg_day: f64,
    net_added_to_standard_uncertainty_ratio: f64,
    net_flux_resolution_status: String,
    net_flux_monte_carlo: DistributionSummary,
    net_flux_uncertainty_comparison: UncertaintyComparison,
}
#[derive(Debug, Serialize)]
struct DistributionSummary {
    draw_count: usize,
    seed: u32,
    mean: f64,
    standard_uncertainty: f64,
    lower_95_percentile: f64,
    upper_95_percentile: f64,
}
#[derive(Debug, Serialize)]
struct UncertaintyComparison {
    first_order_standard_uncertainty: f64,
    monte_carlo_standard_uncertainty: f64,
    relative_standard_uncertainty_difference: f64,
    status: String,
}
#[derive(Debug, Serialize)]
struct IntervalResult {
    interval_id: String,
    intake_temperature_deg_c: f64,
    outfall_temperature_deg_c: f64,
    delta_temperature_deg_c: f64,
    delta_temperature_standard_uncertainty_deg_c: f64,
    delta_temperature_hydraulic_alignment_standard_uncertainty_deg_c: f64,
    intake_flow_m3_s: f64,
    outfall_flow_m3_s: f64,
    flow_closure_residual_m3_s: f64,
    flow_closure_standard_uncertainty_m3_s: f64,
    outfall_flow_hydraulic_alignment_standard_uncertainty_m3_s: f64,
    flow_closure_z_score: f64,
    flow_closure_status: String,
    thermal_power_kw: f64,
    thermal_power_standard_uncertainty_kw: f64,
    thermal_power_hydraulic_alignment_standard_uncertainty_kw: f64,
    hydraulic_residence_time_standard_uncertainty_seconds: f64,
    thermal_power_monte_carlo: DistributionSummary,
    thermal_power_uncertainty_comparison: UncertaintyComparison,
    station_anomalies: Vec<StationAnomaly>,
    pollutant_station_anomalies: Vec<StationConcentrationAnomaly>,
    pollutant_results: Vec<PollutantResult>,
}
#[derive(Debug, Serialize)]
struct TimeGap {
    start_utc: String,
    end_utc: String,
    duration_seconds: u64,
}
#[derive(Debug)]
struct TimeCoverage {
    observed_interval_seconds: u64,
    unobserved_gap_seconds: u64,
    gaps: Vec<TimeGap>,
}
#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    tool_version: String,
    fixture_id: String,
    input_sha256: String,
    input_byte_length: u64,
    input_hash_scope: String,
    as_of: String,
    synthetic_only: bool,
    status: String,
    interval_count: usize,
    observed_interval_seconds: u64,
    unobserved_gap_seconds: u64,
    unobserved_gap_count: usize,
    time_coverage_status: String,
    energy_scope: String,
    unobserved_gaps: Vec<TimeGap>,
    total_thermal_energy_kwh_th: f64,
    interval_results: Vec<IntervalResult>,
    environmental_compliance_status: String,
    ecological_impact_status: String,
    site_clearance_status: String,
    claim_ceiling: String,
}

#[derive(Debug)]
struct DeterministicNormalRng {
    state: u32,
    spare: Option<f64>,
}
impl DeterministicNormalRng {
    fn new(seed: u32) -> Self {
        Self { state: if seed == 0 { 0xA341_316C } else { seed }, spare: None }
    }
    fn uniform_open(&mut self) -> f64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state as f64 + 0.5) / 4_294_967_296.0
    }
    fn standard_normal(&mut self) -> f64 {
        if let Some(value) = self.spare.take() {
            return value;
        }
        let u1 = self.uniform_open().max(f64::MIN_POSITIVE);
        let u2 = self.uniform_open();
        let radius = (-2.0 * u1.ln()).sqrt();
        let angle = std::f64::consts::TAU * u2;
        self.spare = Some(radius * angle.sin());
        radius * angle.cos()
    }
}
fn percentile(sorted: &[f64], p: f64) -> f64 {
    let position = (sorted.len().saturating_sub(1)) as f64 * p;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = position - lower as f64;
        sorted[lower] + (sorted[upper] - sorted[lower]) * fraction
    }
}
fn summarize_distribution(values: &mut [f64], seed: u32, lower: f64, upper: f64)
    -> Result<DistributionSummary, String> {
    if values.len() < 2 {
        return Err("Monte Carlo sample requires at least two draws".into());
    }
    for value in values.iter() {
        finite(*value, "Monte Carlo output")?;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let sum_sq = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>();
    let standard_uncertainty = (sum_sq / (values.len() - 1) as f64).sqrt();
    values.sort_by(|a, b| a.total_cmp(b));
    let lo = percentile(values, lower);
    let hi = percentile(values, upper);
    finite(mean, "Monte Carlo mean")?;
    positive(standard_uncertainty, "Monte Carlo standard uncertainty")?;
    Ok(DistributionSummary {
        draw_count: values.len(),
        seed,
        mean,
        standard_uncertainty,
        lower_95_percentile: lo,
        upper_95_percentile: hi,
    })
}
fn compare_uncertainty(first_order: f64, monte_carlo: f64, tolerance: f64)
    -> Result<UncertaintyComparison, String> {
    positive(first_order, "first-order standard uncertainty")?;
    positive(monte_carlo, "Monte Carlo standard uncertainty")?;
    let relative = (monte_carlo - first_order).abs() / first_order;
    finite(relative, "relative uncertainty difference")?;
    if relative > tolerance {
        return Err(format!(
            "Monte Carlo/first-order uncertainty disagreement {:.6} exceeds declared tolerance {:.6}; uncertainty model review required",
            relative, tolerance
        ));
    }
    Ok(UncertaintyComparison {
        first_order_standard_uncertainty: first_order,
        monte_carlo_standard_uncertainty: monte_carlo,
        relative_standard_uncertainty_difference: relative,
        status: "MONTE_CARLO_CROSSCHECK_WITHIN_DECLARED_TOLERANCE".into(),
    })
}
fn finite(value: f64, name: &str) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("{name} must be finite"));
    }
    Ok(())
}
fn positive(value: f64, name: &str) -> Result<(), String> {
    finite(value, name)?;
    if value <= 0.0 {
        return Err(format!("{name} must be positive"));
    }
    Ok(())
}
fn nonnegative(value: f64, name: &str) -> Result<(), String> {
    finite(value, name)?;
    if value < 0.0 {
        return Err(format!("{name} must be nonnegative"));
    }
    Ok(())
}
fn parse_utc_timestamp(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-' || bytes[7] != b'-'
        || bytes[10] != b'T' || bytes[13] != b':'
        || bytes[16] != b':' || bytes[19] != b'Z'
        || ![0usize,1,2,3,5,6,8,9,11,12,14,15,17,18]
            .iter().all(|index| bytes[*index].is_ascii_digit()) {
        return None;
    }
    let year = value[0..4].parse::<i64>().ok()?;
    let month = value[5..7].parse::<i64>().ok()?;
    let day = value[8..10].parse::<i64>().ok()?;
    let hour = value[11..13].parse::<i64>().ok()?;
    let minute = value[14..16].parse::<i64>().ok()?;
    let second = value[17..19].parse::<i64>().ok()?;
    if year < 1 || !(1..=12).contains(&month) || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute) || !(0..=59).contains(&second) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [31, if leap {29} else {28}, 31,30,31,30,31,31,30,31,30,31];
    if day < 1 || day > month_days[(month - 1) as usize] {
        return None;
    }
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146097 + day_of_era - 719468;
    Some(days * 86400 + hour * 3600 + minute * 60 + second)
}
fn validate_evidence(
    e: &Evidence,
    interval: &IntervalRecord,
    expected_start: &str,
    expected_end: &str,
    ids: &mut HashSet<String>,
    calibration_domains: &mut HashSet<String>,
) -> Result<(), String> {
    if e.sample_id.trim().is_empty() || !ids.insert(e.sample_id.clone()) {
        return Err(format!("empty or duplicate sample_id {}", e.sample_id));
    }
    if e.interval_id != interval.interval_id
        || e.interval_start_utc != expected_start
        || e.interval_end_utc != expected_end {
        return Err(format!("sample {} is not paired to the residence-time-adjusted window for interval {}", e.sample_id, interval.interval_id));
    }
    for (name, value) in [
        ("sensor_id", e.sensor_id.as_str()),
        ("calibration_id", e.calibration_id.as_str()),
        ("calibration_independence_domain", e.calibration_independence_domain.as_str()),
        ("calibration_valid_from_utc", e.calibration_valid_from_utc.as_str()),
        ("calibration_valid_until_utc", e.calibration_valid_until_utc.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("sample {} missing {name}", e.sample_id));
        }
    }
    if !calibration_domains.insert(e.calibration_independence_domain.as_str()) {
        return Err(format!("sample {} shares a calibration independence domain; covariance model required", e.sample_id));
    }
    if e.calibration_state != "in_calibration" {
        return Err(format!("sample {} calibration is not current", e.sample_id));
    }
    let valid_from = parse_utc_timestamp(&e.calibration_valid_from_utc)
        .ok_or_else(|| format!("sample {} has invalid calibration start", e.sample_id))?;
    let valid_until = parse_utc_timestamp(&e.calibration_valid_until_utc)
        .ok_or_else(|| format!("sample {} has invalid calibration end", e.sample_id))?;
    let sample_start = parse_utc_timestamp(expected_start)
        .ok_or_else(|| format!("sample {} has invalid sample-window start", e.sample_id))?;
    let sample_end = parse_utc_timestamp(expected_end)
        .ok_or_else(|| format!("sample {} has invalid sample-window end", e.sample_id))?;
    if valid_until <= valid_from || valid_from > sample_start || valid_until < sample_end {
        return Err(format!("sample {} calibration validity does not cover observation interval", e.sample_id));
    }
    if e.quality_state != "valid" {
        return Err(format!("sample {} quality state is not valid", e.sample_id));
    }
    if e.evidence_state != "synthetic" || e.source_rights_state != "synthetic_test_data_only" {
        return Err(format!("sample {} exceeds the synthetic fixture claim boundary", e.sample_id));
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn monte_carlo_thermal_distribution(
    config: &UncertaintySimulation,
    seed: u32,
    rho: f64, u_rho: f64,
    cp: f64, u_cp: f64,
    intake_temp: f64, u_intake_temp: f64,
    outfall_temp: f64, u_outfall_temp: f64, outfall_temp_rate: f64,
    outfall_flow: f64, u_outfall_flow: f64, outfall_flow_rate: f64,
    lag_standard_uncertainty_seconds: f64,
) -> Result<DistributionSummary, String> {
    if config.draws_per_interval < 10_000 || seed == 0
        || config.algorithm != "deterministic_xorshift32_box_muller_v0_1" {
        return Err("unsupported or insufficient Monte Carlo configuration".into());
    }
    let mut rng = DeterministicNormalRng::new(seed);
    let mut thermal = Vec::with_capacity(config.draws_per_interval);
    for _ in 0..config.draws_per_interval {
        let rho_draw = rho + u_rho * rng.standard_normal();
        let cp_draw = cp + u_cp * rng.standard_normal();
        let outfall_flow_instrument_draw = outfall_flow + u_outfall_flow * rng.standard_normal();
        let lag_draw = lag_standard_uncertainty_seconds * rng.standard_normal();
        let outfall_flow_draw = outfall_flow_instrument_draw + outfall_flow_rate * lag_draw;
        let intake_temp_draw = intake_temp + u_intake_temp * rng.standard_normal();
        let outfall_temp_draw = outfall_temp + u_outfall_temp * rng.standard_normal()
            + outfall_temp_rate * lag_draw;
        for (name, value) in [
            ("sampled density", rho_draw), ("sampled heat capacity", cp_draw),
            ("sampled outfall flow", outfall_flow_draw),
        ] {
            positive(value, name)?;
        }
        let thermal_kw = rho_draw * cp_draw * outfall_flow_draw
            * (outfall_temp_draw - intake_temp_draw) / 1000.0;
        finite(thermal_kw, "Monte Carlo thermal output")?;
        thermal.push(thermal_kw);
    }
    summarize_distribution(
        &mut thermal,
        seed,
        config.lower_95_percentile,
        config.upper_95_percentile,
    )
}
fn measure_time_coverage(intervals: &[IntervalRecord]) -> Result<TimeCoverage, String> {
    let mut observed_interval_seconds = 0u64;
    let mut unobserved_gap_seconds = 0u64;
    let mut gaps = Vec::new();
    let mut previous_start: Option<i64> = None;
    let mut previous_end: Option<i64> = None;
    for interval in intervals {
        let start = parse_utc_timestamp(&interval.interval_start_utc)
            .ok_or_else(|| format!("interval {} start timestamp invalid", interval.interval_id))?;
        let end = parse_utc_timestamp(&interval.interval_end_utc)
            .ok_or_else(|| format!("interval {} end timestamp invalid", interval.interval_id))?;
        if end <= start {
            return Err(format!("interval {} has non-positive input-side observation window", interval.interval_id));
        }
        if let Some(last_start) = previous_start {
            if start <= last_start {
                return Err("input-side observation windows are not strictly chronological".into());
            }
        }
        if let Some(last_end) = previous_end {
            if start < last_end {
                return Err(format!("interval {} overlaps a prior input-side observation window", interval.interval_id));
            }
            if start > last_end {
                let gap_seconds = (start - last_end) as u64;
                unobserved_gap_seconds = unobserved_gap_seconds.checked_add(gap_seconds)
                    .ok_or("unobserved gap duration overflow")?;
                gaps.push(TimeGap {
                    start_utc:intervals
                        .iter()
                        .find(|candidate| parse_utc_timestamp(&candidate.interval_end_utc) == Some(last_end))
                        .map(|candidate| candidate.interval_end_utc.clone())
                        .unwrap_or_default(),
                    end_utc:interval.interval_start_utc.clone(),
                    duration_seconds:gap_seconds,
                });
            }
        }
        let duration = (end - start) as u64;
        observed_interval_seconds = observed_interval_seconds.checked_add(duration)
            .ok_or("observed interval duration overflow")?;
        previous_start = Some(start);
        previous_end = Some(end);
    }
    Ok(TimeCoverage {
        observed_interval_seconds,
        unobserved_gap_seconds,
        gaps,
    })
}
fn validate_interval(bundle: &Bundle, interval: &IntervalRecord, interval_index: usize) -> Result<IntervalResult, String> {
    let start_seconds = parse_utc_timestamp(&interval.interval_start_utc)
        .ok_or_else(|| format!("interval {} has invalid UTC start", interval.interval_id))?;
    let end_seconds = parse_utc_timestamp(&interval.interval_end_utc)
        .ok_or_else(|| format!("interval {} has invalid UTC end", interval.interval_id))?;
    let outfall_start = parse_utc_timestamp(&interval.outfall_window_start_utc)
        .ok_or_else(|| format!("interval {} has invalid outfall UTC start", interval.interval_id))?;
    let outfall_end = parse_utc_timestamp(&interval.outfall_window_end_utc)
        .ok_or_else(|| format!("interval {} has invalid outfall UTC end", interval.interval_id))?;
    if interval.interval_id.trim().is_empty() || end_seconds <= start_seconds
        || outfall_end <= outfall_start {
        return Err(format!("interval {} has invalid UTC window", interval.interval_id));
    }
    if interval.hydraulic_pairing_state != "residence_time_aligned_synthetic"
        || interval.transport_lag_basis != "synthetic_tracer_characterization_assumption_not_measured"
        || interval.hydraulic_residence_time_seconds == 0
        || !((outfall_start - start_seconds) == interval.hydraulic_residence_time_seconds as i64)
        || !((outfall_end - end_seconds) == interval.hydraulic_residence_time_seconds as i64)
        || (end_seconds - start_seconds) != (outfall_end - outfall_start) {
        return Err(format!("interval {} is not a valid residence-time-aligned intake/outfall pair", interval.interval_id));
    }
    nonnegative(interval.hydraulic_residence_time_standard_uncertainty_seconds,
        "hydraulic_residence_time_standard_uncertainty_seconds")?;
    if interval.time_sync_state != bundle.pairing_contract.required_time_sync_state {
        return Err(format!("interval {} time sync is not qualified", interval.interval_id));
    }
    let limit = bundle.pairing_contract.maximum_clock_offset_ms.min(interval.maximum_clock_offset_ms);
    if interval.measured_clock_offset_ms > limit {
        return Err(format!("interval {} clock offset exceeds tolerance", interval.interval_id));
    }
    if interval.rights_state != "synthetic_test_data_only"
        || interval.quality_state != "valid"
        || interval.evidence_state != "synthetic" {
        return Err(format!("interval {} has unqualified rights, QC or evidence state", interval.interval_id));
    }
    if interval.uncertainty_model_state != "first_order_linearization_adequate_for_synthetic_fixture" {
        return Err(format!("interval {} has no qualified uncertainty-model state; use a validated nonlinear/Monte Carlo path", interval.interval_id));
    }

    let rho = interval.fluid_properties.density_kg_m3;
    let u_rho = interval.fluid_properties.density_standard_uncertainty_kg_m3;
    let cp = interval.fluid_properties.specific_heat_capacity_j_kg_k;
    let u_cp = interval.fluid_properties.cp_standard_uncertainty_j_kg_k;
    positive(rho, "density_kg_m3")?;
    positive(cp, "specific_heat_capacity_j_kg_k")?;
    nonnegative(u_rho, "density_standard_uncertainty_kg_m3")?;
    nonnegative(u_cp, "cp_standard_uncertainty_j_kg_k")?;
    if interval.fluid_properties.property_source.trim().is_empty() {
        return Err(format!("interval {} lacks fluid-property source", interval.interval_id));
    }

    if interval.uncertainty_covariance_state
        != "independent_instrument_domains_with_common_hydraulic_lag_covariance_propagated_via_temporal_sensitivities" {
        return Err(format!("interval {} lacks an explicit paired-input covariance assessment", interval.interval_id));
    }
    let mut sample_ids = HashSet::new();
    let mut calibration_domains = HashSet::new();
    let mut temperatures = std::collections::HashMap::new();
    let mut station_ids = HashSet::new();
    let mut thermal_calibration_domains = HashSet::new();
    for observation in &interval.thermal_observations {
        let (expected_start, expected_end) = if observation.station_role == "intake" {
            (&interval.interval_start_utc, &interval.interval_end_utc)
        } else {
            (&interval.outfall_window_start_utc, &interval.outfall_window_end_utc)
        };
        validate_evidence(&observation.evidence, interval, expected_start, expected_end,
            &mut sample_ids, &mut calibration_domains)?;
        if !REQUIRED_ROLES.contains(&observation.station_role.as_str()) {
            return Err(format!("unknown thermal station role {}", observation.station_role));
        }
        if temperatures.contains_key(observation.station_role.as_str()) {
            return Err(format!("duplicate thermal station role {}", observation.station_role));
        }
        if observation.station_id.trim().is_empty()
            || !station_ids.insert(observation.station_id.as_str())
            || !thermal_calibration_domains.insert(observation.evidence.calibration_independence_domain.as_str())
            || observation.position_evidence_state != "synthetic_relative_layout" {
            return Err(format!("station {} lacks synthetic position context", observation.station_id));
        }
        nonnegative(observation.distance_from_outfall_m, "distance_from_outfall_m")?;
        finite(observation.temperature_deg_c, "temperature_deg_c")?;
        nonnegative(observation.standard_uncertainty_deg_c, "temperature standard uncertainty")?;
        finite(observation.temporal_rate_deg_c_per_s, "temperature temporal sensitivity")?;
        temperatures.insert(observation.station_role.as_str(), observation);
    }
    if temperatures.len() != REQUIRED_ROLES.len()
        || REQUIRED_ROLES.iter().any(|role| !temperatures.contains_key(*role)) {
        return Err(format!("interval {} must contain each required thermal station role exactly once", interval.interval_id));
    }

    if interval.flow_samples.len() != 2 {
        return Err(format!("interval {} requires exactly intake and outfall flow samples", interval.interval_id));
    }
    let mut flows = std::collections::HashMap::new();
    for flow in &interval.flow_samples {
        let (expected_start, expected_end) = if flow.station_role == "intake" {
            (&interval.interval_start_utc, &interval.interval_end_utc)
        } else {
            (&interval.outfall_window_start_utc, &interval.outfall_window_end_utc)
        };
        validate_evidence(&flow.evidence, interval, expected_start, expected_end,
            &mut sample_ids, &mut calibration_domains)?;
        if flow.station_id.trim().is_empty()
            || !["intake", "outfall"].contains(&flow.station_role.as_str())
            || flows.contains_key(flow.station_role.as_str()) {
            return Err(format!("interval {} has invalid or duplicate flow station", interval.interval_id));
        }
        positive(flow.flow_m3_s, "flow_m3_s")?;
        nonnegative(flow.standard_uncertainty_m3_s, "flow standard uncertainty")?;
        finite(flow.temporal_rate_m3_s_per_s, "flow temporal sensitivity")?;
        flows.insert(flow.station_role.as_str(), flow);
    }
    let intake_flow = flows.get("intake").copied().ok_or("missing intake flow")?;
    let outfall_flow = flows.get("outfall").copied().ok_or("missing outfall flow")?;
    if temperatures["intake"].station_id != intake_flow.station_id
        || temperatures["outfall"].station_id != outfall_flow.station_id {
        return Err(format!("interval {} flow meters do not match the paired thermal stations", interval.interval_id));
    }

    if interval.concentration_pairs.is_empty() {
        return Err(format!("interval {} has no paired pollutant samples", interval.interval_id));
    }
    if interval.plume_concentration_observations.len() != interval.concentration_pairs.len() * 3 {
        return Err(format!("interval {} must have exactly three plume/reference samples per analyte", interval.interval_id));
    }
    let intake_temp = temperatures["intake"].temperature_deg_c;
    let outfall_temp = temperatures["outfall"].temperature_deg_c;
    let reference = temperatures["independent_reference_control"];
    let u_lag = interval.hydraulic_residence_time_standard_uncertainty_seconds;
    let delta_temp = outfall_temp - intake_temp;
    let q_out = outfall_flow.flow_m3_s;
    let q_in = intake_flow.flow_m3_s;
    let u_q_out_instrument = outfall_flow.standard_uncertainty_m3_s;
    let u_q_in = intake_flow.standard_uncertainty_m3_s;
    let u_delta_temp_instrument = (temperatures["intake"].standard_uncertainty_deg_c.powi(2)
        + temperatures["outfall"].standard_uncertainty_deg_c.powi(2)).sqrt();
    let delta_temp_lag_sensitivity =
        temperatures["outfall"].temporal_rate_deg_c_per_s;
    let u_delta_temp_lag = delta_temp_lag_sensitivity.abs() * u_lag;
    let u_delta_temp = (u_delta_temp_instrument.powi(2) + u_delta_temp_lag.powi(2)).sqrt();

    let thermal_kw = rho * cp * q_out * delta_temp / 1000.0;
    let d_rho = cp * q_out * delta_temp / 1000.0;
    let d_cp = rho * q_out * delta_temp / 1000.0;
    let d_flow = rho * cp * delta_temp / 1000.0;
    let d_delta = rho * cp * q_out / 1000.0;
    let u_thermal_kw_instrument = ((d_rho * u_rho).powi(2)
        + (d_cp * u_cp).powi(2)
        + (d_flow * u_q_out_instrument).powi(2)
        + (d_delta * u_delta_temp_instrument).powi(2)).sqrt();
    let thermal_lag_sensitivity = rho * cp / 1000.0
        * (q_out * temperatures["outfall"].temporal_rate_deg_c_per_s
            + delta_temp * outfall_flow.temporal_rate_m3_s_per_s);
    let u_thermal_kw_lag = thermal_lag_sensitivity.abs() * u_lag;
    let u_thermal_kw = (u_thermal_kw_instrument.powi(2) + u_thermal_kw_lag.powi(2)).sqrt();
    let u_q_out_lag = outfall_flow.temporal_rate_m3_s_per_s.abs() * u_lag;
    let u_q_out_total = (u_q_out_instrument.powi(2) + u_q_out_lag.powi(2)).sqrt();
    finite(thermal_kw, "thermal_power_kw")?;
    let simulation_seed = bundle.uncertainty_simulation.seed.wrapping_add(interval_index as u32);
    let thermal_monte_carlo = monte_carlo_thermal_distribution(
        &bundle.uncertainty_simulation,
        simulation_seed,
        rho, u_rho, cp, u_cp,
        intake_temp, temperatures["intake"].standard_uncertainty_deg_c,
        outfall_temp, temperatures["outfall"].standard_uncertainty_deg_c,
        temperatures["outfall"].temporal_rate_deg_c_per_s,
        q_out, u_q_out_instrument, outfall_flow.temporal_rate_m3_s_per_s,
        u_lag,
    )?;
    let thermal_uncertainty_comparison = compare_uncertainty(
        u_thermal_kw,
        thermal_monte_carlo.standard_uncertainty,
        bundle.uncertainty_simulation.maximum_relative_standard_uncertainty_difference_from_first_order,
    )?;
    nonnegative(u_thermal_kw, "thermal_power_standard_uncertainty_kw")?;

    let flow_residual = q_out - q_in;
    let u_flow_closure = (u_q_out_total.powi(2) + u_q_in.powi(2)).sqrt();
    let flow_z = if u_flow_closure == 0.0 {
        if flow_residual == 0.0 { 0.0 } else { f64::INFINITY }
    } else {
        flow_residual.abs() / u_flow_closure
    };
    if !flow_z.is_finite() || flow_z > bundle.pairing_contract.flow_closure_screen.maximum_absolute_z_score {
        return Err(format!("interval {} intake/outfall flow closure is outside uncertainty screen", interval.interval_id));
    }

    let mut station_anomalies = Vec::new();
    for role in REQUIRED_ROLES {
        let station = temperatures[role];
        let anomaly = station.temperature_deg_c - reference.temperature_deg_c;
        let u_anomaly_instrument = if role == "independent_reference_control" {
            0.0
        } else {
            (station.standard_uncertainty_deg_c.powi(2)
                + reference.standard_uncertainty_deg_c.powi(2)).sqrt()
        };
        let lag_sensitivity = if role == "independent_reference_control" {
            0.0
        } else if role == "intake" {
            -reference.temporal_rate_deg_c_per_s
        } else {
            station.temporal_rate_deg_c_per_s - reference.temporal_rate_deg_c_per_s
        };
        let u_anomaly_lag = lag_sensitivity.abs() * u_lag;
        let u_anomaly = (u_anomaly_instrument.powi(2) + u_anomaly_lag.powi(2)).sqrt();
        finite(anomaly, "station_temperature_anomaly")?;
        nonnegative(u_anomaly, "station_temperature_anomaly_uncertainty")?;
        station_anomalies.push(StationAnomaly {
            station_role:role.to_string(),
            distance_from_outfall_m:station.distance_from_outfall_m,
            delta_from_reference_deg_c:anomaly,
            standard_uncertainty_deg_c:u_anomaly,
            hydraulic_alignment_standard_uncertainty_deg_c:u_anomaly_lag,
        });
    }

    let mut pollutant_results = Vec::new();
    let mut pollutant_station_anomalies = Vec::new();
    let mut analytes = HashSet::new();
    for pair in &interval.concentration_pairs {
        if pair.analyte_id.trim().is_empty() || !analytes.insert(pair.analyte_id.as_str())
            || pair.method_id.trim().is_empty() || pair.method_quality_state != "valid" {
            return Err(format!("interval {} has invalid/duplicate analyte or method", interval.interval_id));
        }
        validate_evidence(&pair.intake.evidence, interval,
            &interval.interval_start_utc, &interval.interval_end_utc,
            &mut sample_ids, &mut calibration_domains)?;
        validate_evidence(&pair.outfall.evidence, interval,
            &interval.outfall_window_start_utc, &interval.outfall_window_end_utc,
            &mut sample_ids, &mut calibration_domains)?;
        for (sample, role) in [(&pair.intake, "intake"), (&pair.outfall, "outfall")] {
            if sample.sample_collection_method != "synthetic_flow_proportional_composite"
                || sample.sample_preservation_state != "synthetic_documented"
                || sample.chain_of_custody_id.trim().is_empty()
                || sample.laboratory_batch_id.trim().is_empty() {
                return Err(format!("analyte {} {role} sample lacks collection, preservation or custody metadata", pair.analyte_id));
            }
        }
        if pair.intake.station_role != "intake" || pair.outfall.station_role != "outfall"
            || pair.intake.station_id != intake_flow.station_id
            || pair.outfall.station_id != outfall_flow.station_id {
            return Err(format!("analyte {} concentration samples do not match flow stations", pair.analyte_id));
        }
        for (sample, name) in [(&pair.intake, "intake"), (&pair.outfall, "outfall")] {
            nonnegative(sample.concentration_mg_l, "concentration_mg_l")?;
            nonnegative(sample.standard_uncertainty_mg_l, "concentration standard uncertainty")?;
            finite(sample.temporal_rate_mg_l_per_s, "concentration temporal sensitivity")?;
            positive(sample.detection_limit_mg_l, "detection_limit_mg_l")?;
            if sample.concentration_mg_l < sample.detection_limit_mg_l {
                return Err(format!("{} concentration for analyte {} is below detection limit; censored-data method required", name, pair.analyte_id));
            }
        }
        let fin = pair.intake.concentration_mg_l * q_in;
        let fout = pair.outfall.concentration_mg_l * q_out;
        let u_fin = ((q_in * pair.intake.standard_uncertainty_mg_l).powi(2)
            + (pair.intake.concentration_mg_l * u_q_in).powi(2)).sqrt();
        let u_fout_instrument = ((q_out * pair.outfall.standard_uncertainty_mg_l).powi(2)
            + (pair.outfall.concentration_mg_l * u_q_out_instrument).powi(2)).sqrt();
        let outfall_flux_lag_sensitivity = q_out * pair.outfall.temporal_rate_mg_l_per_s
            + pair.outfall.concentration_mg_l * outfall_flow.temporal_rate_m3_s_per_s;
        let u_fout_lag = outfall_flux_lag_sensitivity.abs() * u_lag;
        let u_fout = (u_fout_instrument.powi(2) + u_fout_lag.powi(2)).sqrt();
        let net = fout - fin;
        let u_net = (u_fin.powi(2) + u_fout.powi(2)).sqrt();
        positive(u_net, "net pollutant flux standard uncertainty")?;
        let mut mc_rng = DeterministicNormalRng::new(simulation_seed);
        let mut net_flux_draws = Vec::with_capacity(bundle.uncertainty_simulation.draws_per_interval);
        for _ in 0..bundle.uncertainty_simulation.draws_per_interval {
            let qin_draw = q_in + u_q_in * mc_rng.standard_normal();
            let qout_instr_draw = q_out + u_q_out_instrument * mc_rng.standard_normal();
            let lag_draw = u_lag * mc_rng.standard_normal();
            let qout_draw = qout_instr_draw + outfall_flow.temporal_rate_m3_s_per_s * lag_draw;
            let cin_draw = pair.intake.concentration_mg_l + pair.intake.standard_uncertainty_mg_l * mc_rng.standard_normal();
            let cout_draw = pair.outfall.concentration_mg_l
                + pair.outfall.standard_uncertainty_mg_l * mc_rng.standard_normal()
                + pair.outfall.temporal_rate_mg_l_per_s * lag_draw;
            positive(qin_draw, "Monte Carlo intake flow")?;
            positive(qout_draw, "Monte Carlo outfall flow")?;
            positive(cin_draw, "Monte Carlo intake concentration")?;
            positive(cout_draw, "Monte Carlo outfall concentration")?;
            let draw = cout_draw * qout_draw - cin_draw * qin_draw;
            finite(draw, "Monte Carlo net pollutant flux")?;
            net_flux_draws.push(draw);
        }
        let net_flux_monte_carlo = summarize_distribution(
            &mut net_flux_draws,
            simulation_seed,
            bundle.uncertainty_simulation.lower_95_percentile,
            bundle.uncertainty_simulation.upper_95_percentile,
        )?;
        let net_flux_uncertainty_comparison = compare_uncertainty(
            u_net,
            net_flux_monte_carlo.standard_uncertainty,
            bundle.uncertainty_simulation.maximum_relative_standard_uncertainty_difference_from_first_order,
        )?;
        let resolution_ratio = net.abs() / u_net;
        let resolution_status = if resolution_ratio < 1.0 {
            "BELOW_1_STANDARD_UNCERTAINTY_UNRESOLVED"
        } else {
            "ABOVE_1_STANDARD_UNCERTAINTY_NOT_COMPLIANCE"
        };
        for (name, value) in [
            ("intake_pollutant_flux", fin), ("outfall_pollutant_flux", fout),
            ("net_pollutant_flux", net), ("net_pollutant_flux_uncertainty", u_net),
        ] {
            finite(value, name)?;
        }
        pollutant_results.push(PollutantResult {
            analyte_id:pair.analyte_id.clone(),
            intake_g_s:fin,
            intake_standard_uncertainty_g_s:u_fin,
            outfall_g_s:fout,
            outfall_standard_uncertainty_g_s:u_fout,
            outfall_hydraulic_alignment_standard_uncertainty_g_s:u_fout_lag,
            net_added_g_s:net,
            net_added_standard_uncertainty_g_s:u_net,
            net_added_kg_day:net * 86.4,
            net_added_standard_uncertainty_kg_day:u_net * 86.4,
            net_added_to_standard_uncertainty_ratio:resolution_ratio,
            net_flux_resolution_status:resolution_status.into(),
            net_flux_monte_carlo,
            net_flux_uncertainty_comparison,
        });
        let matching_plume: Vec<&PlumeConcentrationObservation> =
            interval.plume_concentration_observations.iter()
                .filter(|sample| sample.analyte_id == pair.analyte_id)
                .collect();
        let required_plume_roles = ["near_field_plume", "far_field_impact", "independent_reference_control"];
        if matching_plume.len() != required_plume_roles.len() {
            return Err(format!("analyte {} requires near-field, far-field and reference concentration samples", pair.analyte_id));
        }
        let mut plume_by_role = std::collections::HashMap::new();
        for sample in matching_plume {
            validate_evidence(&sample.evidence, interval,
                &interval.outfall_window_start_utc, &interval.outfall_window_end_utc,
                &mut sample_ids, &mut calibration_domains)?;
            if sample.method_id != pair.method_id || sample.method_quality_state != "valid"
                || sample.sample_collection_method != "synthetic_flow_proportional_composite"
                || sample.sample_preservation_state != "synthetic_documented"
                || sample.chain_of_custody_id.trim().is_empty()
                || sample.laboratory_batch_id.trim().is_empty()
                || !required_plume_roles.contains(&sample.station_role.as_str())
                || plume_by_role.contains_key(sample.station_role.as_str()) {
                return Err(format!("analyte {} has invalid or duplicate plume sample", pair.analyte_id));
            }
            let station = temperatures.get(sample.station_role.as_str())
                .ok_or_else(|| format!("analyte {} plume sample has no thermal station", pair.analyte_id))?;
            if sample.station_id != station.station_id {
                return Err(format!("analyte {} plume sample station identity mismatch", pair.analyte_id));
            }
            nonnegative(sample.concentration_mg_l, "plume concentration_mg_l")?;
            nonnegative(sample.standard_uncertainty_mg_l, "plume concentration uncertainty")?;
            finite(sample.temporal_rate_mg_l_per_s, "plume concentration temporal sensitivity")?;
            positive(sample.detection_limit_mg_l, "plume detection limit")?;
            if sample.concentration_mg_l < sample.detection_limit_mg_l {
                return Err(format!("analyte {} plume concentration below detection limit; censored-data method required", pair.analyte_id));
            }
            plume_by_role.insert(sample.station_role.as_str(), sample);
        }
        let reference_sample = *plume_by_role.get("independent_reference_control")
            .ok_or_else(|| format!("analyte {} missing reference chemistry sample", pair.analyte_id))?;
        let mut station_concentrations: Vec<(&str, &str, f64, f64)> = vec![
            ("intake", &pair.intake.station_id, pair.intake.concentration_mg_l, pair.intake.standard_uncertainty_mg_l),
            ("outfall", &pair.outfall.station_id, pair.outfall.concentration_mg_l, pair.outfall.standard_uncertainty_mg_l),
        ];
        for role in ["near_field_plume", "far_field_impact", "independent_reference_control"] {
            let sample = *plume_by_role.get(role)
                .ok_or_else(|| format!("analyte {} missing {role} chemistry sample", pair.analyte_id))?;
            station_concentrations.push((role, &sample.station_id, sample.concentration_mg_l, sample.standard_uncertainty_mg_l));
        }
        for (role, station_id, concentration, uncertainty) in station_concentrations {
            let station = temperatures.get(role)
                .ok_or_else(|| format!("analyte {} has no station metadata for {role}", pair.analyte_id))?;
            if station_id != station.station_id.as_str() {
                return Err(format!("analyte {} concentration station mismatch for {role}", pair.analyte_id));
            }
            let delta = concentration - reference_sample.concentration_mg_l;
            let station_rate = match role {
                "intake" => pair.intake.temporal_rate_mg_l_per_s,
                "outfall" => pair.outfall.temporal_rate_mg_l_per_s,
                _ => plume_by_role.get(role)
                    .map(|sample| sample.temporal_rate_mg_l_per_s)
                    .unwrap_or(0.0),
            };
            let lag_sensitivity = if role == "independent_reference_control" {
                0.0
            } else if role == "intake" {
                -reference_sample.temporal_rate_mg_l_per_s
            } else {
                station_rate - reference_sample.temporal_rate_mg_l_per_s
            };
            let alignment_uncertainty = lag_sensitivity.abs() * u_lag;
            let delta_uncertainty_instrument = if role == "independent_reference_control" {
                0.0
            } else {
                (uncertainty.powi(2)
                    + reference_sample.standard_uncertainty_mg_l.powi(2)).sqrt()
            };
            let delta_uncertainty = (delta_uncertainty_instrument.powi(2)
                + alignment_uncertainty.powi(2)).sqrt();
            finite(delta, "station concentration anomaly")?;
            nonnegative(delta_uncertainty, "station concentration anomaly uncertainty")?;
            pollutant_station_anomalies.push(StationConcentrationAnomaly {
                analyte_id:pair.analyte_id.clone(),
                station_role:role.to_string(),
                distance_from_outfall_m:station.distance_from_outfall_m,
                delta_from_reference_mg_l:delta,
                standard_uncertainty_mg_l:delta_uncertainty,
                hydraulic_alignment_standard_uncertainty_mg_l:alignment_uncertainty,
            });
        }
    }

    Ok(IntervalResult {
        interval_id:interval.interval_id.clone(),
        intake_temperature_deg_c:intake_temp,
        outfall_temperature_deg_c:outfall_temp,
        delta_temperature_deg_c:delta_temp,
        delta_temperature_standard_uncertainty_deg_c:u_delta_temp,
        delta_temperature_hydraulic_alignment_standard_uncertainty_deg_c:u_delta_temp_lag,
        intake_flow_m3_s:q_in,
        outfall_flow_m3_s:q_out,
        flow_closure_residual_m3_s:flow_residual,
        flow_closure_standard_uncertainty_m3_s:u_flow_closure,
        outfall_flow_hydraulic_alignment_standard_uncertainty_m3_s:u_q_out_lag,
        flow_closure_z_score:flow_z,
        flow_closure_status:"WITHIN_DECLARED_FLOW_CLOSURE_SCREEN_NOT_FULL_MASS_BALANCE_OR_PERMIT_COMPLIANCE".into(),
        thermal_power_kw:thermal_kw,
        thermal_power_standard_uncertainty_kw:u_thermal_kw,
        thermal_power_hydraulic_alignment_standard_uncertainty_kw:u_thermal_kw_lag,
        hydraulic_residence_time_standard_uncertainty_seconds:u_lag,
        thermal_power_monte_carlo:thermal_monte_carlo,
        thermal_power_uncertainty_comparison:thermal_uncertainty_comparison,
        station_anomalies,
        pollutant_station_anomalies,
        pollutant_results,
    })
}

fn validate(bundle: &Bundle) -> Result<Report, String> {
    if !bundle.synthetic_only || bundle.interval_records.is_empty()
        || bundle.schema_version.trim().is_empty() || bundle.fixture_id.trim().is_empty()
        || bundle.as_of.trim().is_empty() || bundle.claim_ceiling.trim().is_empty() {
        return Err("bundle must be nonempty and explicitly synthetic".into());
    }
    if bundle.station_roles.len() != REQUIRED_ROLES.len()
        || REQUIRED_ROLES.iter().any(|role| !bundle.station_roles.iter().any(|x| x.as_str() == *role)) {
        return Err("station role contract must contain exactly the five required roles".into());
    }
    if bundle.pairing_contract.timestamp_format != "YYYY-MM-DDTHH:MM:SSZ"
        || bundle.pairing_contract.required_time_sync_state != "synchronized"
        || bundle.pairing_contract.maximum_clock_offset_ms == 0
        || !bundle.pairing_contract.pair_rule.contains("exact same interval_id") {
        return Err("unsupported pairing contract".into());
    }
    positive(bundle.pairing_contract.flow_closure_screen.maximum_absolute_z_score,
        "maximum_absolute_z_score")?;
    if !bundle.standards_context.uncertainty_method.contains("GUM")
        || bundle.standards_context.uncertainty_method_reference.trim().is_empty()
        || !bundle.standards_context.nonlinear_model_policy.contains("Monte Carlo")
        || !bundle.standards_context.water_sampling_reference.contains("ISO 5667-10:2026")
        || bundle.standards_context.water_sampling_url.trim().is_empty()
        || !bundle.standards_context.observation_interoperability.contains("SensorThings API 2.0")
        || bundle.standards_context.observation_interoperability_url.trim().is_empty() {
        return Err("standards context incomplete".into());
    }
    if !bundle.equation_contract.thermal_power_kw.contains("density")
        || !bundle.equation_contract.thermal_power_kw.contains("specific_heat_capacity")
        || !bundle.equation_contract.pollutant_mass_flux_g_s.contains("concentration_mg_l")
        || !bundle.equation_contract.pollutant_mass_flux_g_s.contains("flow_m3_s")
        || !bundle.equation_contract.net_pollutant_mass_flux_g_s.contains("outfall")
        || !bundle.equation_contract.net_pollutant_mass_flux_g_s.contains("intake")
        || !bundle.equation_contract.covariance_assumption.contains("independent") {
        return Err("equation and covariance contract incomplete".into());
    }
    for rule in [
        bundle.equation_contract.temperature_difference_standard_uncertainty_c.as_str(),
        bundle.equation_contract.thermal_power_standard_uncertainty_kw.as_str(),
        bundle.equation_contract.pollutant_mass_flux_kg_day.as_str(),
        bundle.equation_contract.net_pollutant_mass_flux_standard_uncertainty_g_s.as_str(),
    ] {
        if rule.trim().is_empty() {
            return Err("uncertainty or unit-conversion equation missing".into());
        }
    }
    let boundary = &bundle.pairing_contract.flow_closure_boundary;
    if boundary.mode != "single_pass_once_through_no_side_streams_synthetic"
        || boundary.included_streams != vec!["intake".to_string(), "outfall".to_string()]
        || !boundary.excluded_side_streams.is_empty()
        || boundary.storage_change_assumption != "zero_within_each_synthetic_interval"
        || boundary.boundary_completeness_state != "synthetic_assumed_complete"
        || !boundary.rule.contains("requires a full signed multi-stream mass balance") {
        return Err("two-meter flow closure is not valid outside the declared single-pass/no-side-stream boundary".into());
    }
    if !bundle.pairing_contract.flow_closure_screen.method.contains("uncertainty")
        || !bundle.pairing_contract.flow_closure_screen.purpose.contains("not a full water mass balance")
        || !bundle.pairing_contract.flow_closure_screen.purpose.contains("not a discharge permit") {
        return Err("flow-closure screen must remain a bounded metrology check, not a full mass balance or permit test".into());
    }
    let simulation = &bundle.uncertainty_simulation;
    if !simulation.framework.contains("JCGM 101:2008")
        || !simulation.framework.contains("JCGM 100:2008/Amd.1:2026")
        || simulation.framework_references.len() < 2
        || simulation.framework_references.iter().any(|url| !url.starts_with("https://"))
        || simulation.algorithm != "deterministic_xorshift32_box_muller_v0_1"
        || simulation.seed == 0
        || simulation.draws_per_interval < 10_000
        || simulation.lower_95_percentile != 0.025
        || simulation.upper_95_percentile != 0.975
        || !(0.0 < simulation.maximum_relative_standard_uncertainty_difference_from_first_order
            && simulation.maximum_relative_standard_uncertainty_difference_from_first_order <= 0.20)
        || !simulation.distribution_assumption.contains("common normal hydraulic-residence-time")
        || !simulation.acceptance_rule.contains("no qualified measurement receipt") {
        return Err("Monte Carlo uncertainty simulation contract incomplete or out of bounds".into());
    }
    let mapping = &bundle.ogc_sensorthings_mapping;
    if mapping.standard != "OGC SensorThings API 2.0"
        || mapping.standard_url != "https://docs.ogc.org/is/23-019/23-019.html"
        || mapping.publication_date != "2026-11-04"
        || mapping.entity_mapping.station_or_platform != "Thing"
        || mapping.entity_mapping.station_location != "Location"
        || mapping.entity_mapping.sensor_and_calibration_identity != "Sensor"
        || mapping.entity_mapping.measured_property != "ObservedProperty"
        || mapping.entity_mapping.sensor_and_property_series != "Datastream"
        || mapping.entity_mapping.individual_measurement != "Observation"
        || mapping.entity_mapping.station_or_sampled_environment_feature != "Feature"
        || mapping.entity_mapping.observation_proximate_feature_link != "ProximateFeatureOfInterest"
        || mapping.entity_mapping.datastream_proximate_feature_link != "ProximateFeatureOfInterest"
        || mapping.entity_mapping.datastream_ultimate_feature_link != "UltimateFeatureOfInterest"
        || mapping.datastream_result_encoding.result_type != "SWE-Common JSON structure description"
        || mapping.datastream_result_encoding.result_encoding != "SWE-Common encoding descriptor"
        || !mapping.datastream_result_encoding.unit_location.contains("part of resultType")
        || !mapping.datastream_result_encoding.legacy_unit_of_measurement.contains("not used in v2.0")
        || mapping.data_model_policy.trim().is_empty() {
        return Err("OGC SensorThings 2.0 entity, Feature relationship or SWE-Common result mapping incomplete".into());
    }

    let time_coverage = measure_time_coverage(&bundle.interval_records)?;
    let mut interval_ids = HashSet::new();
    let mut results = Vec::new();
    let mut total_thermal_kwh = 0.0;
    for (interval_index, interval) in bundle.interval_records.iter().enumerate() {
        if !interval_ids.insert(interval.interval_id.as_str()) {
            return Err(format!("duplicate interval_id {}", interval.interval_id));
        }
        let result = validate_interval(bundle, interval, interval_index)?;
        let start = parse_utc_timestamp(&interval.interval_start_utc)
            .ok_or_else(|| format!("interval {} start timestamp invalid", interval.interval_id))?;
        let end = parse_utc_timestamp(&interval.interval_end_utc)
            .ok_or_else(|| format!("interval {} end timestamp invalid", interval.interval_id))?;
        let duration_hours = (end - start) as f64 / 3600.0;
        total_thermal_kwh += result.thermal_power_kw * duration_hours;
        finite(total_thermal_kwh, "total_thermal_energy_kwh_th")?;
        results.push(result);
    }

    Ok(Report {
        schema_version:bundle.schema_version.clone(),
        tool_version:env!("CARGO_PKG_VERSION").to_string(),
        fixture_id:bundle.fixture_id.clone(),
        input_sha256:String::new(),
        input_byte_length:0,
        input_hash_scope:"unset_until_cli_reads_exact_input_bytes".into(),
        as_of:bundle.as_of.clone(),
        synthetic_only:true,
        status:"SYNTHETIC_MEASUREMENT_BUNDLE_VALID_NO_SITE_OR_COMPLIANCE_CLAIM".into(),
        interval_count:results.len(),
        observed_interval_seconds:time_coverage.observed_interval_seconds,
        unobserved_gap_seconds:time_coverage.unobserved_gap_seconds,
        unobserved_gap_count:time_coverage.gaps.len(),
        time_coverage_status:if time_coverage.gaps.is_empty() {
            "CONTIGUOUS_OBSERVED_WINDOWS_NO_INTERPOLATION".into()
        } else {
            "GAPS_EXPLICIT_NO_INTERPOLATION".into()
        },
        energy_scope:"sum of observed input-side intervals only; unobserved gaps are not interpolated or counted as zero load".into(),
        unobserved_gaps:time_coverage.gaps,
        total_thermal_energy_kwh_th:total_thermal_kwh,
        interval_results:results,
        environmental_compliance_status:"NOT_ASSESSED_NO_PERMIT_LIMIT_OR_FIELD_BASELINE".into(),
        ecological_impact_status:"NOT_ASSESSED_NO_FIELD_ECOLOGICAL_EVIDENCE".into(),
        site_clearance_status:"UNKNOWN_BLOCKED".into(),
        claim_ceiling:bundle.claim_ceiling.clone(),
    })
}

fn run(path: &Path) -> Result<Report, String> {
    let input = fs::read(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let digest = Sha256::digest(&input);
    let input_sha256 = format!("{digest:x}");
    let bundle: Bundle = serde_json::from_slice(&input)
        .map_err(|e| format!("invalid measurement bundle JSON: {e}"))?;
    let mut report = validate(&bundle)?;
    report.input_sha256 = input_sha256;
    report.input_byte_length = input.len() as u64;
    report.input_hash_scope =
        "SHA-256 over exact input file bytes; proves input-byte identity only, not sensor authenticity or acquisition integrity".into();
    Ok(report)
}
fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| DEFAULT_PATH.into());
    match run(Path::new(&path)) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(e) => { eprintln!("cannot serialize metrology receipt: {e}"); std::process::exit(2); }
        },
        Err(e) => { eprintln!("OCEAN_ENV_METROLOGY_REJECTED: {e}"); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Bundle {
        serde_json::from_str(include_str!(
            "../../../docs/research/port-energy-compute/OCEAN_ENVIRONMENT_MEASUREMENT_BUNDLE_V0_1.json"
        )).expect("measurement bundle parses")
    }
    #[test]
    fn first_order_uncertainty_disagreement_above_tolerance_is_rejected() {
        assert!(compare_uncertainty(1.0, 1.06, 0.05).is_err());
        assert!(compare_uncertainty(1.0, 1.04, 0.05).is_ok());
    }
    #[test]
    fn ogc_sensor_things_v2_mapping_rejects_legacy_feature_of_interest_contract() {
        let mut b = fixture();
        b.ogc_sensorthings_mapping.entity_mapping.station_or_sampled_environment_feature =
            "FeatureOfInterest".into();
        assert!(validate(&b).unwrap_err().contains("OGC SensorThings 2.0"));
    }
    #[test]
    fn monte_carlo_distribution_is_deterministic_for_a_fixed_seed() {
        let mut a = DeterministicNormalRng::new(20261120);
        let mut b = DeterministicNormalRng::new(20261120);
        for _ in 0..100 {
            assert_eq!(a.standard_normal().to_bits(), b.standard_normal().to_bits());
        }
        let report = validate(&fixture()).expect("fixture validates");
        assert!(report.interval_results.iter().all(|interval|
            interval.thermal_power_monte_carlo.draw_count == 50_000
                && interval.thermal_power_uncertainty_comparison.relative_standard_uncertainty_difference <= 0.05
                && interval.pollutant_results.iter().all(|p|
                    p.net_flux_monte_carlo.draw_count == 50_000
                        && p.net_flux_uncertainty_comparison.relative_standard_uncertainty_difference <= 0.05
                )
        ));
    }
    #[test]
    fn sha256_receipt_uses_standard_known_vector() {
        let digest = Sha256::digest(b"abc");
        assert_eq!(format!("{digest:x}"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
    #[test]
    fn valid_synthetic_bundle_calculates_but_never_grants_clearance() {
        let report = validate(&fixture()).expect("synthetic fixture should validate");
        assert_eq!(report.interval_count, 3);
        assert!(report.total_thermal_energy_kwh_th > 0.0);
        assert_eq!(report.site_clearance_status, "UNKNOWN_BLOCKED");
        assert!(report.environmental_compliance_status.starts_with("NOT_ASSESSED"));
        assert!(report.interval_results.iter().all(|interval|
            interval.pollutant_results.iter().all(|p|
                p.net_flux_resolution_status == "BELOW_1_STANDARD_UNCERTAINTY_UNRESOLVED"
            ) && interval.pollutant_station_anomalies.len() == 5
        ));
    }
    #[test]
    fn expired_calibration_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].thermal_observations[0].evidence.calibration_state = "expired".into();
        assert!(validate(&b).unwrap_err().contains("calibration is not current"));
    }
    #[test]
    fn clock_skew_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].measured_clock_offset_ms = 251;
        assert!(validate(&b).unwrap_err().contains("clock offset exceeds"));
    }
    #[test]
    fn unpaired_sample_window_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].flow_samples[0].evidence.interval_end_utc = "2026-10-12T10:04:59Z".into();
        assert!(validate(&b).unwrap_err().contains("residence-time-adjusted window"));
    }
    #[test]
    fn intake_outfall_flow_imbalance_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].flow_samples[1].flow_m3_s = 0.024;
        assert!(validate(&b).unwrap_err().contains("flow closure is outside"));
    }
    #[test]
    fn two_meter_flow_closure_rejects_unmeasured_side_streams() {
        let mut b = fixture();
        b.pairing_contract.flow_closure_boundary.excluded_side_streams.push("unmetered_purge".into());
        assert!(validate(&b).unwrap_err().contains("single-pass/no-side-stream boundary"));
    }
    #[test]
    fn below_detection_concentration_requires_censored_data_method() {
        let mut b = fixture();
        b.interval_records[0].concentration_pairs[0].outfall.concentration_mg_l = 0.0005;
        assert!(validate(&b).unwrap_err().contains("below detection limit"));
    }
    #[test]
    fn missing_reference_station_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].thermal_observations.pop();
        assert!(validate(&b).unwrap_err().contains("each required thermal station role"));
    }
    #[test]
    fn material_nonlinearity_without_supported_method_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].uncertainty_model_state = "material_nonlinearity_unresolved".into();
        assert!(validate(&b).unwrap_err().contains("nonlinear/Monte Carlo"));
    }
    #[test]
    fn shared_calibration_domain_requires_covariance_model() {
        let mut b = fixture();
        let intake_domain = b.interval_records[0].concentration_pairs[0].intake.evidence.calibration_independence_domain.clone();
        b.interval_records[0].concentration_pairs[0].outfall.evidence.calibration_independence_domain = intake_domain;
        assert!(validate(&b).unwrap_err().contains("shares a calibration independence domain"));
    }
    #[test]
    fn expired_calibration_validity_window_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].thermal_observations[0].evidence.calibration_valid_until_utc =
            "2026-10-12T09:59:59Z".into();
        assert!(validate(&b).unwrap_err().contains("calibration validity does not cover"));
    }
    #[test]
    fn uncorrected_same_time_outfall_sample_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].thermal_observations[1].evidence.interval_start_utc =
            b.interval_records[0].interval_start_utc.clone();
        b.interval_records[0].thermal_observations[1].evidence.interval_end_utc =
            b.interval_records[0].interval_end_utc.clone();
        assert!(validate(&b).unwrap_err().contains("residence-time-adjusted window"));
    }
    #[test]
    fn missing_chain_of_custody_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].concentration_pairs[0].outfall.chain_of_custody_id.clear();
        assert!(validate(&b).unwrap_err().contains("custody metadata"));
    }
    #[test]
    fn missing_far_field_chemistry_sample_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].plume_concentration_observations.retain(|sample|
            sample.station_role != "far_field_impact"
        );
        assert!(validate(&b).unwrap_err().contains("exactly three plume/reference samples"));
    }
    #[test]
    fn missing_calibration_independence_domain_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].thermal_observations[0].evidence.calibration_independence_domain.clear();
        assert!(validate(&b).unwrap_err().contains("missing calibration_independence_domain"));
    }
    #[test]
    fn utc_interval_duration_handles_midnight() {
        let start = parse_utc_timestamp("2026-10-12T23:58:00Z").unwrap();
        let end = parse_utc_timestamp("2026-10-13T00:03:00Z").unwrap();
        assert_eq!(end - start, 300);
        assert!(parse_utc_timestamp("2026-02-30T00:00:00Z").is_none());
    }
    #[test]
    fn time_coverage_reports_gaps_without_interpolating() {
        let mut b = fixture();
        b.interval_records[1].interval_start_utc = "2026-10-12T10:10:00Z".into();
        b.interval_records[1].interval_end_utc = "2026-10-12T10:15:00Z".into();
        let coverage = measure_time_coverage(&b.interval_records).expect("ordered non-overlapping windows");
        assert_eq!(coverage.unobserved_gap_seconds, 300);
        assert_eq!(coverage.gaps.len(), 1);
        assert_eq!(coverage.gaps[0].duration_seconds, 300);
    }
    #[test]
    fn overlapping_or_out_of_order_windows_are_rejected() {
        let mut b = fixture();
        b.interval_records[1].interval_start_utc = "2026-10-12T10:04:00Z".into();
        b.interval_records[1].interval_end_utc = "2026-10-12T10:09:00Z".into();
        assert!(measure_time_coverage(&b.interval_records).unwrap_err().contains("overlaps"));
    }
    #[test]
    fn hydraulic_lag_uncertainty_is_propagated_into_thermal_output() {
        let report = validate(&fixture()).expect("fixture validates");
        assert!(report.interval_results.iter().all(|i|
            i.thermal_power_hydraulic_alignment_standard_uncertainty_kw > 0.0
                && i.delta_temperature_hydraulic_alignment_standard_uncertainty_deg_c > 0.0
        ));
    }
    #[test]
    fn hydraulic_lag_uncertainty_is_propagated_into_pollutant_flux() {
        let report = validate(&fixture()).expect("fixture validates");
        assert!(report.interval_results.iter().all(|i|
            i.pollutant_results.iter().all(|p|
                p.outfall_hydraulic_alignment_standard_uncertainty_g_s > 0.0
            )
        ));
    }
    #[test]
    fn reference_station_anomalies_have_zero_self_difference_uncertainty() {
        let report = validate(&fixture()).expect("fixture validates");
        assert!(report.interval_results.iter().all(|interval| {
            let temp = interval.station_anomalies.iter()
                .find(|x| x.station_role == "independent_reference_control").unwrap();
            let chemistry = interval.pollutant_station_anomalies.iter()
                .find(|x| x.station_role == "independent_reference_control").unwrap();
            temp.standard_uncertainty_deg_c == 0.0
                && temp.hydraulic_alignment_standard_uncertainty_deg_c == 0.0
                && chemistry.standard_uncertainty_mg_l == 0.0
                && chemistry.hydraulic_alignment_standard_uncertainty_mg_l == 0.0
        }));
    }
}
