use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::process::ExitCode;

const REQUIRED_IDS: [&str; 11] = [
    "PORS_100KM_APEX_5DEG_PROGRADE",
    "PORS_100KM_APEX_20DEG_PROGRADE",
    "PORS_300KM_APEX_5DEG_PROGRADE",
    "PORS_300KM_APEX_10DEG_PROGRADE",
    "PORS_300KM_APEX_20DEG_PROGRADE",
    "PORS_300KM_APEX_30DEG_PROGRADE",
    "PORS_600KM_APEX_10DEG_PROGRADE",
    "PORS_600KM_APEX_20DEG_PROGRADE",
    "PORS_600KM_APEX_30DEG_RETROGRADE",
    "INVALID_ZERO_APEX",
    "INVALID_HALF_ANGLE_LIMIT",
];

const EXPECTED_MU: f64 = 3.986_004_418e14;
const EXPECTED_RADIUS: f64 = 6_371_000.0;
const EXPECTED_EARTH_ROTATION: f64 = 7.292_115e-5;
const GOLDEN_REL_TOLERANCE: f64 = 2.0e-9;
const ENERGY_ABS_TOLERANCE: f64 = 2.0e-5;
const ANGULAR_MOMENTUM_REL_TOLERANCE: f64 = 2.0e-12;

type Check<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct PorsResult {
    id: String,
    status: String,
    reason_code: Option<String>,
    values: Value,
}

fn object<'a>(value: &'a Value, at: &str) -> Check<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{at}: expected object"))
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let actual = object(value, at)?;
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&expected_set).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!(
            "{at}: missing keys {missing:?}; unknown keys {unknown:?}"
        ));
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(value, at)?
        .get(key)
        .ok_or_else(|| format!("{at}: missing field {key}"))
}

fn req_text(value: &Value, key: &str, at: &str) -> Check<String> {
    field(value, key, at)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key}: expected string"))
}

fn req_f64(value: &Value, key: &str, at: &str) -> Check<f64> {
    let number = field(value, key, at)?
        .as_f64()
        .ok_or_else(|| format!("{at}.{key}: expected finite number"))?;
    if !number.is_finite() {
        return Err(format!("{at}.{key}: expected finite number"));
    }
    Ok(number)
}

fn valid_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn evaluate_arc(
    id: &str,
    apex_height_m: f64,
    half_angle_deg: f64,
    direction: &str,
    mu: f64,
    radius_m: f64,
    earth_rotation_rad_s: f64,
) -> Check<PorsResult> {
    if apex_height_m <= 0.0 {
        return Ok(PorsResult {
            id: id.to_owned(),
            status: "invalid".to_owned(),
            reason_code: Some("APEX_HEIGHT_NOT_POSITIVE".to_owned()),
            values: Value::Null,
        });
    }
    if !(0.0 < half_angle_deg && half_angle_deg < 90.0) {
        return Ok(PorsResult {
            id: id.to_owned(),
            status: "invalid".to_owned(),
            reason_code: Some("HALF_ANGLE_OUT_OF_RANGE".to_owned()),
            values: Value::Null,
        });
    }
    let direction_sign = match direction {
        "prograde" => 1.0,
        "retrograde" => -1.0,
        other => return Err(format!("{id}: unsupported direction {other}")),
    };
    if !mu.is_finite()
        || mu <= 0.0
        || !radius_m.is_finite()
        || radius_m <= 0.0
        || !earth_rotation_rad_s.is_finite()
        || earth_rotation_rad_s < 0.0
    {
        return Err(format!("{id}: invalid planetary constants"));
    }

    // A symmetric two-body conic segment with its apogee at the arc apex.
    // The continuation of the mathematical ellipse beyond the two terminals
    // would pass through Earth; only the exterior segment is being evaluated.
    let alpha = half_angle_deg.to_radians();
    let z = apex_height_m / radius_m;
    let c = 1.0 - alpha.cos();
    let eccentricity = z / (z + c);
    if !(0.0..1.0).contains(&eccentricity) {
        return Err(format!("{id}: conic eccentricity is not elliptic"));
    }

    let apex_radius = radius_m + apex_height_m;
    let semi_major_axis_m = apex_radius / (1.0 + eccentricity);
    let semi_latus_rectum_m = semi_major_axis_m * (1.0 - eccentricity * eccentricity);
    let apex_speed_mps =
        (mu * (1.0 - eccentricity) / apex_radius).sqrt();
    let ground_inertial_speed_mps =
        (mu * (2.0 / radius_m - 1.0 / semi_major_axis_m)).sqrt();

    // At the arrival terminal, true anomaly is pi + alpha. The flight-path
    // angle magnitude is symmetric about the apex.
    let gamma = (eccentricity * alpha.sin())
        .atan2(1.0 - eccentricity * alpha.cos());

    // Convert the first endpoint's true anomaly f = pi - alpha to eccentric
    // anomaly, then use Kepler's equation over the symmetric arc.
    let true_anomaly_start = std::f64::consts::PI - alpha;
    let denominator = 1.0 + eccentricity * true_anomaly_start.cos();
    let cos_e = (eccentricity + true_anomaly_start.cos()) / denominator;
    let sin_e = (1.0 - eccentricity * eccentricity).sqrt()
        * true_anomaly_start.sin()
        / denominator;
    let mut eccentric_anomaly_start = sin_e.atan2(cos_e);
    if eccentric_anomaly_start < 0.0 {
        eccentric_anomaly_start += std::f64::consts::TAU;
    }
    let mean_anomaly_start =
        eccentric_anomaly_start - eccentricity * eccentric_anomaly_start.sin();
    let eccentric_anomaly_end =
        std::f64::consts::TAU - eccentric_anomaly_start;
    let mean_anomaly_end =
        eccentric_anomaly_end - eccentricity * eccentric_anomaly_end.sin();
    let mean_motion = (mu / semi_major_axis_m.powi(3)).sqrt();
    let flight_time_s = (mean_anomaly_end - mean_anomaly_start) / mean_motion;
    if !flight_time_s.is_finite() || flight_time_s <= 0.0 {
        return Err(format!("{id}: invalid Keplerian flight time"));
    }

    let inertial_arc_deg = direction_sign * 2.0 * half_angle_deg;
    let earth_rotation_during_flight_deg =
        earth_rotation_rad_s * flight_time_s.to_degrees();
    let earth_fixed_endpoint_delta_deg =
        inertial_arc_deg - earth_rotation_during_flight_deg;
    let surface_arc_distance_km =
        earth_fixed_endpoint_delta_deg.abs().to_radians() * radius_m / 1000.0;

    let ground_relative_tangential_speed_mps =
        direction_sign * ground_inertial_speed_mps * gamma.cos()
            - earth_rotation_rad_s * radius_m;
    let ground_relative_radial_speed_mps =
        -ground_inertial_speed_mps * gamma.sin();

    let endpoint_radius_m =
        semi_latus_rectum_m / (1.0 - eccentricity * alpha.cos());
    let endpoint_radius_error_m = (endpoint_radius_m - radius_m).abs();

    let apex_specific_energy_j_kg =
        0.5 * apex_speed_mps.powi(2) - mu / apex_radius;
    let ground_specific_energy_j_kg =
        0.5 * ground_inertial_speed_mps.powi(2) - mu / radius_m;
    let specific_energy_error_j_kg =
        (apex_specific_energy_j_kg - ground_specific_energy_j_kg).abs();

    let apex_angular_momentum_m2_s = apex_radius * apex_speed_mps;
    let ground_angular_momentum_m2_s =
        radius_m * ground_inertial_speed_mps * gamma.cos();
    let angular_momentum_relative_error =
        (apex_angular_momentum_m2_s - ground_angular_momentum_m2_s).abs()
            / apex_angular_momentum_m2_s.abs().max(1.0);

    if endpoint_radius_error_m > 1.0e-5
        || specific_energy_error_j_kg > ENERGY_ABS_TOLERANCE
        || angular_momentum_relative_error > ANGULAR_MOMENTUM_REL_TOLERANCE
    {
        return Err(format!("{id}: analytic conservation closure failed"));
    }

    let values = json!({
        "eccentricity": eccentricity,
        "semi_major_axis_m": semi_major_axis_m,
        "inertial_arc_deg": inertial_arc_deg.abs(),
        "earth_rotation_during_flight_deg": earth_rotation_during_flight_deg,
        "earth_fixed_endpoint_delta_deg": earth_fixed_endpoint_delta_deg,
        "surface_arc_distance_km": surface_arc_distance_km,
        "flight_time_s": flight_time_s,
        "apex_speed_mps": apex_speed_mps,
        "ground_inertial_speed_mps": ground_inertial_speed_mps,
        "ground_relative_tangential_speed_mps": ground_relative_tangential_speed_mps,
        "ground_relative_radial_speed_mps": ground_relative_radial_speed_mps,
        "ground_flight_path_angle_deg": gamma.to_degrees(),
        "endpoint_radius_error_m": endpoint_radius_error_m,
        "specific_energy_error_j_kg": specific_energy_error_j_kg,
        "angular_momentum_relative_error": angular_momentum_relative_error
    });
    Ok(PorsResult {
        id: id.to_owned(),
        status: "pass".to_owned(),
        reason_code: None,
        values,
    })
}

fn compare_expected(actual: &PorsResult, expected: &Value, at: &str) -> Check<()> {
    let expected_status = req_text(expected, "status", at)?;
    if expected_status != actual.status {
        return Err(format!(
            "{at}: expected status {expected_status}, got {}",
            actual.status
        ));
    }
    if actual.status == "invalid" {
        let expected_reason = req_text(expected, "reason_code", at)?;
        if actual.reason_code.as_deref() != Some(expected_reason.as_str()) {
            return Err(format!("{at}: expected reason code {expected_reason}"));
        }
        return Ok(());
    }

    let numeric_keys = [
        "eccentricity",
        "semi_major_axis_m",
        "inertial_arc_deg",
        "earth_rotation_during_flight_deg",
        "earth_fixed_endpoint_delta_deg",
        "surface_arc_distance_km",
        "flight_time_s",
        "apex_speed_mps",
        "ground_inertial_speed_mps",
        "ground_relative_tangential_speed_mps",
        "ground_relative_radial_speed_mps",
        "ground_flight_path_angle_deg",
    ];
    for key in numeric_keys {
        let expected_value = req_f64(expected, key, at)?;
        let actual_value = actual.values[key]
            .as_f64()
            .ok_or_else(|| format!("{at}.{key}: missing actual number"))?;
        let tolerance =
            GOLDEN_REL_TOLERANCE * expected_value.abs().max(1.0);
        if (actual_value - expected_value).abs() > tolerance {
            return Err(format!(
                "{at}.{key}: expected {expected_value}, got {actual_value}, tolerance {tolerance}"
            ));
        }
    }
    Ok(())
}

fn evaluate_scenario(
    scenario: &Value,
    mu: f64,
    radius_m: f64,
    earth_rotation_rad_s: f64,
) -> Check<PorsResult> {
    let id = req_text(scenario, "id", "scenario")?;
    let at = format!("scenario.{id}");
    exact_keys(
        scenario,
        &[
            "id",
            "apex_height_m",
            "half_central_angle_deg",
            "direction",
            "expected",
        ],
        &at,
    )?;
    let expected = field(scenario, "expected", &at)?;
    let result = evaluate_arc(
        &id,
        req_f64(scenario, "apex_height_m", &at)?,
        req_f64(scenario, "half_central_angle_deg", &at)?,
        &req_text(scenario, "direction", &at)?,
        mu,
        radius_m,
        earth_rotation_rad_s,
    )?;
    compare_expected(&result, expected, &format!("{at}.expected"))?;
    Ok(result)
}

fn validate(root: &Value, source_commit: &str, corpus_sha256: &str) -> Check<Value> {
    exact_keys(
        root,
        &[
            "suite_id",
            "suite_version",
            "evidence_class",
            "model_scope",
            "claim_ceiling",
            "constants",
            "scenarios",
        ],
        "root",
    )?;
    if req_text(root, "suite_id", "root")? != "pors-two-body-geometry"
        || req_text(root, "suite_version", "root")? != "0.1.0"
        || req_text(root, "evidence_class", "root")? != "analytic_synthetic_reference"
    {
        return Err("root: unsupported PORS suite identity/version/evidence class".to_owned());
    }
    if req_text(root, "claim_ceiling", "root")?.len() < 60 {
        return Err("root: missing PORS claim ceiling".to_owned());
    }
    let constants = field(root, "constants", "root")?;
    exact_keys(
        constants,
        &[
            "earth_mu_m3_s2",
            "earth_mean_radius_m",
            "earth_rotation_rate_rad_s",
        ],
        "root.constants",
    )?;
    let mu = req_f64(constants, "earth_mu_m3_s2", "root.constants")?;
    let radius_m = req_f64(constants, "earth_mean_radius_m", "root.constants")?;
    let earth_rotation_rad_s =
        req_f64(constants, "earth_rotation_rate_rad_s", "root.constants")?;
    if (mu - EXPECTED_MU).abs() > 1.0
        || (radius_m - EXPECTED_RADIUS).abs() > 1.0e-6
        || (earth_rotation_rad_s - EXPECTED_EARTH_ROTATION).abs() > 1.0e-12
    {
        return Err("root.constants: unsupported Earth reference constants".to_owned());
    }

    let scenarios = field(root, "scenarios", "root")?
        .as_array()
        .ok_or_else(|| "root.scenarios: expected array".to_owned())?;
    if scenarios.len() != REQUIRED_IDS.len() {
        return Err(format!(
            "root.scenarios: expected {} cases, found {}",
            REQUIRED_IDS.len(),
            scenarios.len()
        ));
    }
    let mut by_id = std::collections::BTreeMap::new();
    for scenario in scenarios {
        let id = req_text(scenario, "id", "scenario")?;
        if !REQUIRED_IDS.contains(&id.as_str()) || by_id.insert(id.clone(), scenario).is_some() {
            return Err(format!("root.scenarios: unknown or duplicate id {id}"));
        }
    }

    let mut reports = Vec::new();
    for id in REQUIRED_IDS {
        let scenario = by_id
            .get(id)
            .ok_or_else(|| format!("root.scenarios: missing case {id}"))?;
        let result = evaluate_scenario(
            scenario,
            mu,
            radius_m,
            earth_rotation_rad_s,
        )?;
        reports.push(json!({
            "scenario_id": result.id,
            "status": result.status,
            "reason_code": result.reason_code,
            "values": result.values
        }));
    }
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "validator": "pors_two_body_geometry_reference",
        "suite_id": "pors-two-body-geometry",
        "suite_version": "0.1.0",
        "source_commit": source_commit,
        "corpus_sha256": corpus_sha256,
        "scenario_count": reports.len(),
        "pass_count": reports.iter().filter(|r| r["status"] == "pass").count(),
        "invalid_count": reports.iter().filter(|r| r["status"] == "invalid").count(),
        "result": "pass",
        "claim_ceiling": root["claim_ceiling"],
        "scenarios": reports
    }))
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err(
            "usage: pors_geometry_reference --source-commit <40-char-lowercase-git-sha>"
                .to_owned(),
        );
    }
    if !valid_commit_sha(&args[2]) {
        return Err("source commit must be a 40-character lowercase SHA-1".to_owned());
    }
    let bytes =
        include_bytes!("../../../docs/tech/pors-two-body-geometry-corpus-v0.1.json");
    let digest = format!("{:x}", Sha256::digest(bytes));
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid PORS corpus JSON: {error}"))?;
    let receipt = validate(&root, &args[2], &digest)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pors_geometry_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Value {
        serde_json::from_slice(include_bytes!(
            "../../../docs/tech/pors-two-body-geometry-corpus-v0.1.json"
        ))
        .expect("PORS analytic fixture corpus parses")
    }

    fn result_for(root: &Value, id: &str) -> PorsResult {
        let scenario = root["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scenario| scenario["id"] == id)
            .expect("PORS fixture exists");
        evaluate_scenario(
            scenario,
            EXPECTED_MU,
            EXPECTED_RADIUS,
            EXPECTED_EARTH_ROTATION,
        )
        .expect("PORS fixture validates")
    }

    #[test]
    fn all_eleven_frozen_pors_cases_match_the_independent_golden_reference() {
        let root = corpus();
        let bytes =
            include_bytes!("../../../docs/tech/pors-two-body-geometry-corpus-v0.1.json");
        let digest = format!("{:x}", Sha256::digest(bytes));
        let receipt = validate(&root, &"a".repeat(40), &digest)
            .expect("all PORS cases match the golden reference");
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["scenario_count"], 11);
        assert_eq!(receipt["pass_count"], 9);
        assert_eq!(receipt["invalid_count"], 2);
    }

    #[test]
    fn endpoint_and_apex_speeds_conserve_specific_energy_and_angular_momentum() {
        let root = corpus();
        for scenario in root["scenarios"].as_array().unwrap() {
            if scenario["expected"]["status"] != "pass" {
                continue;
            }
            let result = result_for(&root, scenario["id"].as_str().unwrap());
            assert!(result.values["endpoint_radius_error_m"].as_f64().unwrap() < 1.0e-5);
            assert!(result.values["specific_energy_error_j_kg"].as_f64().unwrap()
                < ENERGY_ABS_TOLERANCE);
            assert!(result.values["angular_momentum_relative_error"].as_f64().unwrap()
                < ANGULAR_MOMENTUM_REL_TOLERANCE);
        }
    }

    #[test]
    fn Earth_rotation_changes_the_paired_ground_terminal_longitude() {
        let root = corpus();
        let prograde = result_for(&root, "PORS_300KM_APEX_20DEG_PROGRADE");
        let retrograde = result_for(&root, "PORS_600KM_APEX_30DEG_RETROGRADE");
        assert!(prograde.values["earth_rotation_during_flight_deg"].as_f64().unwrap() > 0.0);
        assert!(
            prograde.values["earth_fixed_endpoint_delta_deg"].as_f64().unwrap()
                < prograde.values["inertial_arc_deg"].as_f64().unwrap()
        );
        assert!(
            retrograde.values["earth_fixed_endpoint_delta_deg"].as_f64().unwrap()
                < -retrograde.values["inertial_arc_deg"].as_f64().unwrap()
        );
    }

    #[test]
    fn ground_reversal_station_faces_large_velocity_components() {
        let root = corpus();
        let case = result_for(&root, "PORS_300KM_APEX_5DEG_PROGRADE");
        assert!(
            case.values["ground_relative_radial_speed_mps"]
                .as_f64()
                .unwrap()
                < -2_000.0
        );
        assert!(
            case.values["ground_relative_tangential_speed_mps"]
                .as_f64()
                .unwrap()
                > 1_000.0
        );
    }

    #[test]
    fn invalid_apex_and_half_angle_fail_closed() {
        let root = corpus();
        assert_eq!(
            result_for(&root, "INVALID_ZERO_APEX").reason_code.as_deref(),
            Some("APEX_HEIGHT_NOT_POSITIVE")
        );
        assert_eq!(
            result_for(&root, "INVALID_HALF_ANGLE_LIMIT").reason_code.as_deref(),
            Some("HALF_ANGLE_OUT_OF_RANGE")
        );
    }

    #[test]
    fn rejects_duplicate_and_unknown_case_ids() {
        let mut root = corpus();
        root["scenarios"][1]["id"] = root["scenarios"][0]["id"].clone();
        let bytes =
            include_bytes!("../../../docs/tech/pors-two-body-geometry-corpus-v0.1.json");
        let digest = format!("{:x}", Sha256::digest(bytes));
        assert!(validate(&root, &"a".repeat(40), &digest).is_err());

        let mut root = corpus();
        root["scenarios"][0]["id"] = json!("UNREGISTERED_CASE");
        assert!(validate(&root, &"a".repeat(40), &digest).is_err());
    }

    #[test]
    fn rejects_modified_golden_geometry() {
        let mut root = corpus();
        root["scenarios"][0]["expected"]["flight_time_s"] = json!(123.0);
        assert!(evaluate_scenario(
            &root["scenarios"][0],
            EXPECTED_MU,
            EXPECTED_RADIUS,
            EXPECTED_EARTH_ROTATION
        )
        .is_err());
    }
}
