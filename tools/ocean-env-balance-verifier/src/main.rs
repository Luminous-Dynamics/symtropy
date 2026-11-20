use serde::{Deserialize, Serialize};
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
    water_balance_screen: WaterBalanceScreen,
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
    entity_mapping: EntityMapping,
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
    intake_outfall_or_reference_station: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntervalRecord {
    interval_id: String,
    interval_start_utc: String,
    interval_end_utc: String,
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
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FlowSample {
    evidence: Evidence,
    station_id: String,
    station_role: String,
    flow_m3_s: f64,
    standard_uncertainty_m3_s: f64,
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
}

#[derive(Debug, Serialize)]
struct StationAnomaly {
    station_role: String,
    distance_from_outfall_m: f64,
    delta_from_reference_deg_c: f64,
    standard_uncertainty_deg_c: f64,
}
#[derive(Debug, Serialize)]
struct PollutantResult {
    analyte_id: String,
    intake_g_s: f64,
    intake_standard_uncertainty_g_s: f64,
    outfall_g_s: f64,
    outfall_standard_uncertainty_g_s: f64,
    net_added_g_s: f64,
    net_added_standard_uncertainty_g_s: f64,
    net_added_kg_day: f64,
    net_added_standard_uncertainty_kg_day: f64,
    net_added_to_standard_uncertainty_ratio: f64,
    net_flux_resolution_status: String,
}
#[derive(Debug, Serialize)]
struct IntervalResult {
    interval_id: String,
    intake_temperature_deg_c: f64,
    outfall_temperature_deg_c: f64,
    delta_temperature_deg_c: f64,
    delta_temperature_standard_uncertainty_deg_c: f64,
    intake_flow_m3_s: f64,
    outfall_flow_m3_s: f64,
    water_balance_residual_m3_s: f64,
    water_balance_standard_uncertainty_m3_s: f64,
    water_balance_z_score: f64,
    water_balance_status: String,
    thermal_power_kw: f64,
    thermal_power_standard_uncertainty_kw: f64,
    station_anomalies: Vec<StationAnomaly>,
    pollutant_results: Vec<PollutantResult>,
}
#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    fixture_id: String,
    as_of: String,
    synthetic_only: bool,
    status: String,
    interval_count: usize,
    total_thermal_energy_kwh_th: f64,
    interval_results: Vec<IntervalResult>,
    environmental_compliance_status: String,
    ecological_impact_status: String,
    site_clearance_status: String,
    claim_ceiling: String,
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
fn validate_evidence(e: &Evidence, interval: &IntervalRecord, ids: &mut HashSet<String>) -> Result<(), String> {
    if e.sample_id.trim().is_empty() || !ids.insert(e.sample_id.clone()) {
        return Err(format!("empty or duplicate sample_id {}", e.sample_id));
    }
    if e.interval_id != interval.interval_id
        || e.interval_start_utc != interval.interval_start_utc
        || e.interval_end_utc != interval.interval_end_utc {
        return Err(format!("sample {} is not exactly paired to interval {}", e.sample_id, interval.interval_id));
    }
    for (name, value) in [
        ("sensor_id", e.sensor_id.as_str()),
        ("calibration_id", e.calibration_id.as_str()),
        ("calibration_independence_domain", e.calibration_independence_domain.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("sample {} missing {name}", e.sample_id));
        }
    }
    if e.calibration_state != "in_calibration" {
        return Err(format!("sample {} calibration is not current", e.sample_id));
    }
    if e.quality_state != "valid" {
        return Err(format!("sample {} quality state is not valid", e.sample_id));
    }
    if e.evidence_state != "synthetic" || e.source_rights_state != "synthetic_test_data_only" {
        return Err(format!("sample {} exceeds the synthetic fixture claim boundary", e.sample_id));
    }
    Ok(())
}
fn validate_interval(bundle: &Bundle, interval: &IntervalRecord) -> Result<IntervalResult, String> {
    let start_seconds = parse_utc_timestamp(&interval.interval_start_utc)
        .ok_or_else(|| format!("interval {} has invalid UTC start", interval.interval_id))?;
    let end_seconds = parse_utc_timestamp(&interval.interval_end_utc)
        .ok_or_else(|| format!("interval {} has invalid UTC end", interval.interval_id))?;
    if interval.interval_id.trim().is_empty() || end_seconds <= start_seconds {
        return Err(format!("interval {} has invalid UTC window", interval.interval_id));
    }
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

    if interval.uncertainty_covariance_state != "paired_measurement_calibration_domains_independent" {
        return Err(format!("interval {} lacks an explicit paired-input covariance assessment", interval.interval_id));
    }
    let mut sample_ids = HashSet::new();
    let mut temperatures = std::collections::HashMap::new();
    let mut station_ids = HashSet::new();
    for observation in &interval.thermal_observations {
        validate_evidence(&observation.evidence, interval, &mut sample_ids)?;
        if !REQUIRED_ROLES.contains(&observation.station_role.as_str()) {
            return Err(format!("unknown thermal station role {}", observation.station_role));
        }
        if temperatures.contains_key(observation.station_role.as_str()) {
            return Err(format!("duplicate thermal station role {}", observation.station_role));
        }
        if observation.station_id.trim().is_empty()
            || !station_ids.insert(observation.station_id.as_str())
            || observation.position_evidence_state != "synthetic_relative_layout" {
            return Err(format!("station {} lacks synthetic position context", observation.station_id));
        }
        nonnegative(observation.distance_from_outfall_m, "distance_from_outfall_m")?;
        finite(observation.temperature_deg_c, "temperature_deg_c")?;
        nonnegative(observation.standard_uncertainty_deg_c, "temperature standard uncertainty")?;
        temperatures.insert(observation.station_role.as_str(), observation);
    }
    if temperatures.len() != REQUIRED_ROLES.len()
        || REQUIRED_ROLES.iter().any(|role| !temperatures.contains_key(role)) {
        return Err(format!("interval {} must contain each required thermal station role exactly once", interval.interval_id));
    }

    if interval.flow_samples.len() != 2 {
        return Err(format!("interval {} requires exactly intake and outfall flow samples", interval.interval_id));
    }
    let mut flows = std::collections::HashMap::new();
    for flow in &interval.flow_samples {
        validate_evidence(&flow.evidence, interval, &mut sample_ids)?;
        if flow.station_id.trim().is_empty()
            || !["intake", "outfall"].contains(&flow.station_role.as_str())
            || flows.contains_key(flow.station_role.as_str()) {
            return Err(format!("interval {} has invalid or duplicate flow station", interval.interval_id));
        }
        positive(flow.flow_m3_s, "flow_m3_s")?;
        nonnegative(flow.standard_uncertainty_m3_s, "flow standard uncertainty")?;
        flows.insert(flow.station_role.as_str(), flow);
    }
    let intake_flow = flows.get("intake").copied().ok_or("missing intake flow")?;
    let outfall_flow = flows.get("outfall").copied().ok_or("missing outfall flow")?;
    if intake_flow.evidence.calibration_independence_domain == outfall_flow.evidence.calibration_independence_domain {
        return Err("shared calibration independence domain for intake/outfall flow requires covariance model".into());
    }
    if temperatures["intake"].station_id != intake_flow.station_id
        || temperatures["outfall"].station_id != outfall_flow.station_id {
        return Err(format!("interval {} flow meters do not match the paired thermal stations", interval.interval_id));
    }

    if interval.concentration_pairs.is_empty() {
        return Err(format!("interval {} has no paired pollutant samples", interval.interval_id));
    }
    if temperatures["intake"].evidence.calibration_independence_domain
        == temperatures["outfall"].evidence.calibration_independence_domain {
        return Err("shared calibration independence domain for paired thermal inputs requires covariance model".into());
    }
    let intake_temp = temperatures["intake"].temperature_deg_c;
    let outfall_temp = temperatures["outfall"].temperature_deg_c;
    let reference = temperatures["independent_reference_control"];
    let u_delta_temp = (temperatures["intake"].standard_uncertainty_deg_c.powi(2)
        + temperatures["outfall"].standard_uncertainty_deg_c.powi(2)).sqrt();
    let delta_temp = outfall_temp - intake_temp;
    let q_out = outfall_flow.flow_m3_s;
    let q_in = intake_flow.flow_m3_s;
    let u_q_out = outfall_flow.standard_uncertainty_m3_s;
    let u_q_in = intake_flow.standard_uncertainty_m3_s;

    let thermal_kw = rho * cp * q_out * delta_temp / 1000.0;
    let d_rho = cp * q_out * delta_temp / 1000.0;
    let d_cp = rho * q_out * delta_temp / 1000.0;
    let d_flow = rho * cp * delta_temp / 1000.0;
    let d_delta = rho * cp * q_out / 1000.0;
    let u_thermal_kw = ((d_rho * u_rho).powi(2)
        + (d_cp * u_cp).powi(2)
        + (d_flow * u_q_out).powi(2)
        + (d_delta * u_delta_temp).powi(2)).sqrt();
    finite(thermal_kw, "thermal_power_kw")?;
    nonnegative(u_thermal_kw, "thermal_power_standard_uncertainty_kw")?;

    let water_residual = q_out - q_in;
    let u_water_balance = (u_q_out.powi(2) + u_q_in.powi(2)).sqrt();
    let water_z = if u_water_balance == 0.0 {
        if water_residual == 0.0 { 0.0 } else { f64::INFINITY }
    } else {
        water_residual.abs() / u_water_balance
    };
    if !water_z.is_finite() || water_z > bundle.pairing_contract.water_balance_screen.maximum_absolute_z_score {
        return Err(format!("interval {} intake/outfall flow balance is outside uncertainty screen", interval.interval_id));
    }

    let mut station_anomalies = Vec::new();
    for role in REQUIRED_ROLES {
        let station = temperatures[role];
        let anomaly = station.temperature_deg_c - reference.temperature_deg_c;
        let u_anomaly = (station.standard_uncertainty_deg_c.powi(2)
            + reference.standard_uncertainty_deg_c.powi(2)).sqrt();
        finite(anomaly, "station_temperature_anomaly")?;
        nonnegative(u_anomaly, "station_temperature_anomaly_uncertainty")?;
        station_anomalies.push(StationAnomaly {
            station_role:role.to_string(),
            distance_from_outfall_m:station.distance_from_outfall_m,
            delta_from_reference_deg_c:anomaly,
            standard_uncertainty_deg_c:u_anomaly,
        });
    }

    let mut pollutant_results = Vec::new();
    let mut analytes = HashSet::new();
    for pair in &interval.concentration_pairs {
        if pair.analyte_id.trim().is_empty() || !analytes.insert(pair.analyte_id.as_str())
            || pair.method_id.trim().is_empty() || pair.method_quality_state != "valid" {
            return Err(format!("interval {} has invalid/duplicate analyte or method", interval.interval_id));
        }
        validate_evidence(&pair.intake.evidence, interval, &mut sample_ids)?;
        validate_evidence(&pair.outfall.evidence, interval, &mut sample_ids)?;
        if pair.intake.station_role != "intake" || pair.outfall.station_role != "outfall"
            || pair.intake.station_id != intake_flow.station_id
            || pair.outfall.station_id != outfall_flow.station_id {
            return Err(format!("analyte {} concentration samples do not match flow stations", pair.analyte_id));
        }
        if pair.intake.evidence.calibration_independence_domain
            == pair.outfall.evidence.calibration_independence_domain {
            return Err(format!("analyte {} shares a calibration domain; covariance model required", pair.analyte_id));
        }
        for (sample, name) in [(&pair.intake, "intake"), (&pair.outfall, "outfall")] {
            nonnegative(sample.concentration_mg_l, "concentration_mg_l")?;
            nonnegative(sample.standard_uncertainty_mg_l, "concentration standard uncertainty")?;
            positive(sample.detection_limit_mg_l, "detection_limit_mg_l")?;
            if sample.concentration_mg_l < sample.detection_limit_mg_l {
                return Err(format!("{} concentration for analyte {} is below detection limit; censored-data method required", name, pair.analyte_id));
            }
        }
        let fin = pair.intake.concentration_mg_l * q_in;
        let fout = pair.outfall.concentration_mg_l * q_out;
        let u_fin = ((q_in * pair.intake.standard_uncertainty_mg_l).powi(2)
            + (pair.intake.concentration_mg_l * u_q_in).powi(2)).sqrt();
        let u_fout = ((q_out * pair.outfall.standard_uncertainty_mg_l).powi(2)
            + (pair.outfall.concentration_mg_l * u_q_out).powi(2)).sqrt();
        let net = fout - fin;
        let u_net = (u_fin.powi(2) + u_fout.powi(2)).sqrt();
        positive(u_net, "net pollutant flux standard uncertainty")?;
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
            net_added_g_s:net,
            net_added_standard_uncertainty_g_s:u_net,
            net_added_kg_day:net * 86.4,
            net_added_standard_uncertainty_kg_day:u_net * 86.4,
            net_added_to_standard_uncertainty_ratio:resolution_ratio,
            net_flux_resolution_status:resolution_status.into(),
        });
    }

    Ok(IntervalResult {
        interval_id:interval.interval_id.clone(),
        intake_temperature_deg_c:intake_temp,
        outfall_temperature_deg_c:outfall_temp,
        delta_temperature_deg_c:delta_temp,
        delta_temperature_standard_uncertainty_deg_c:u_delta_temp,
        intake_flow_m3_s:q_in,
        outfall_flow_m3_s:q_out,
        water_balance_residual_m3_s:water_residual,
        water_balance_standard_uncertainty_m3_s:u_water_balance,
        water_balance_z_score:water_z,
        water_balance_status:"WITHIN_DECLARED_INSTRUMENT_UNCERTAINTY_SCREEN_NOT_PERMIT_COMPLIANCE".into(),
        thermal_power_kw:thermal_kw,
        thermal_power_standard_uncertainty_kw:u_thermal_kw,
        station_anomalies,
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
        || REQUIRED_ROLES.iter().any(|role| !bundle.station_roles.iter().any(|x| x == role)) {
        return Err("station role contract must contain exactly the five required roles".into());
    }
    if bundle.pairing_contract.timestamp_format != "YYYY-MM-DDTHH:MM:SSZ"
        || bundle.pairing_contract.required_time_sync_state != "synchronized"
        || bundle.pairing_contract.maximum_clock_offset_ms == 0
        || !bundle.pairing_contract.pair_rule.contains("exact same interval_id") {
        return Err("unsupported pairing contract".into());
    }
    positive(bundle.pairing_contract.water_balance_screen.maximum_absolute_z_score,
        "maximum_absolute_z_score")?;
    if !bundle.standards_context.uncertainty_method.contains("GUM")
        || bundle.standards_context.uncertainty_method_reference.trim().is_empty()
        || !bundle.standards_context.nonlinear_model_policy.contains("Monte Carlo")
        || !bundle.standards_context.water_sampling_reference.contains("ISO 5667-10:2026")
        || bundle.standards_context.water_sampling_url.trim().is_empty()
        || !bundle.standards_context.observation_interoperability.contains("SensorThings")
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
    if !bundle.pairing_contract.water_balance_screen.method.contains("uncertainty")
        || !bundle.pairing_contract.water_balance_screen.purpose.contains("not a discharge permit") {
        return Err("water-balance screen must remain a metrology check, not permit compliance".into());
    }
    let mapping = &bundle.ogc_sensorthings_mapping;
    if mapping.standard != "OGC SensorThings API Part 1: Sensing 1.1"
        || mapping.entity_mapping.station_or_platform != "Thing"
        || mapping.entity_mapping.station_location != "Location"
        || mapping.entity_mapping.sensor_and_calibration_identity != "Sensor"
        || mapping.entity_mapping.measured_property != "ObservedProperty"
        || mapping.entity_mapping.sensor_and_property_series != "Datastream"
        || mapping.entity_mapping.individual_measurement != "Observation"
        || mapping.entity_mapping.intake_outfall_or_reference_station != "FeatureOfInterest"
        || mapping.data_model_policy.trim().is_empty() {
        return Err("OGC SensorThings entity mapping incomplete or unsupported".into());
    }

    let mut interval_ids = HashSet::new();
    let mut results = Vec::new();
    let mut total_thermal_kwh = 0.0;
    for interval in &bundle.interval_records {
        if !interval_ids.insert(interval.interval_id.as_str()) {
            return Err(format!("duplicate interval_id {}", interval.interval_id));
        }
        let result = validate_interval(bundle, interval)?;
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
        fixture_id:bundle.fixture_id.clone(),
        as_of:bundle.as_of.clone(),
        synthetic_only:true,
        status:"SYNTHETIC_MEASUREMENT_BUNDLE_VALID_NO_SITE_OR_COMPLIANCE_CLAIM".into(),
        interval_count:results.len(),
        total_thermal_energy_kwh_th:total_thermal_kwh,
        interval_results:results,
        environmental_compliance_status:"NOT_ASSESSED_NO_PERMIT_LIMIT_OR_FIELD_BASELINE".into(),
        ecological_impact_status:"NOT_ASSESSED_NO_FIELD_ECOLOGICAL_EVIDENCE".into(),
        site_clearance_status:"UNKNOWN_BLOCKED".into(),
        claim_ceiling:bundle.claim_ceiling.clone(),
    })
}

fn load(path: &Path) -> Result<Bundle, String> {
    let input = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&input).map_err(|e| format!("invalid measurement bundle JSON: {e}"))
}
fn run(path: &Path) -> Result<Report, String> {
    let bundle = load(path)?;
    validate(&bundle)
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
    fn valid_synthetic_bundle_calculates_but_never_grants_clearance() {
        let report = validate(&fixture()).expect("synthetic fixture should validate");
        assert_eq!(report.interval_count, 3);
        assert!(report.total_thermal_energy_kwh_th > 0.0);
        assert_eq!(report.site_clearance_status, "UNKNOWN_BLOCKED");
        assert!(report.environmental_compliance_status.starts_with("NOT_ASSESSED"));
        assert!(report.interval_results.iter().all(|interval|
            interval.pollutant_results.iter().all(|p|
                p.net_flux_resolution_status == "BELOW_1_STANDARD_UNCERTAINTY_UNRESOLVED"
            )
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
        assert!(validate(&b).unwrap_err().contains("not exactly paired"));
    }
    #[test]
    fn intake_outfall_flow_imbalance_is_rejected() {
        let mut b = fixture();
        b.interval_records[0].flow_samples[1].flow_m3_s = 0.024;
        assert!(validate(&b).unwrap_err().contains("flow balance is outside"));
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
        assert!(validate(&b).unwrap_err().contains("shares a calibration domain"));
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
}
