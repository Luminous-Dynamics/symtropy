use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::ExitCode;

const C_M_S: f64 = 299_792_458.0;
const LIGHT_YEAR_M: f64 = 9.460_730_472_580_8e15;
const JOULES_PER_KWH: f64 = 3_600_000.0;
const SECONDS_PER_JULIAN_YEAR: f64 = 365.25 * 24.0 * 60.0 * 60.0;
const REQUIRED_KEYS: [&str; 6] = [
    "schema_version",
    "scenario_id",
    "distance_ly",
    "spacecraft_mass_kg",
    "peak_speed_fraction_c",
    "proper_acceleration_m_s2",
];

#[derive(Debug, Clone, PartialEq)]
struct MissionInput {
    scenario_id: String,
    distance_ly: f64,
    spacecraft_mass_kg: f64,
    peak_speed_fraction_c: f64,
    proper_acceleration_m_s2: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct MissionReport {
    scenario_id: String,
    distance_ly: f64,
    peak_speed_fraction_c: f64,
    lorentz_factor: f64,
    acceleration_distance_ly: f64,
    deceleration_distance_ly: f64,
    cruise_distance_ly: f64,
    coordinate_flight_years: f64,
    spacecraft_proper_years: f64,
    peak_kinetic_energy_j: f64,
    peak_kinetic_energy_kwh: f64,
}

fn finite_positive(value: f64, field: &str) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{field} must be finite and greater than zero"))
    }
}

/// Parse the deliberately small key=value fixture format.
/// Unknown keys and duplicate keys are rejected to keep the fixture contract strict.
fn parse_input(source: &str) -> Result<MissionInput, String> {
    let mut values = BTreeMap::<String, String>::new();

    for (line_index, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {}: expected key=value", line_index + 1))?;
        let key = key.trim();
        let value = value.trim();

        if key.is_empty() || value.is_empty() {
            return Err(format!(
                "line {}: key and value must be non-empty",
                line_index + 1
            ));
        }
        if !REQUIRED_KEYS.contains(&key) {
            return Err(format!("line {}: unknown key {key}", line_index + 1));
        }
        if values.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!("line {}: duplicate key {key}", line_index + 1));
        }
    }

    for key in REQUIRED_KEYS {
        if !values.contains_key(key) {
            return Err(format!("missing required key {key}"));
        }
    }
    if values.len() != REQUIRED_KEYS.len() {
        return Err("fixture has an unexpected key count".to_owned());
    }
    if values.get("schema_version").map(String::as_str) != Some("0.1") {
        return Err("unsupported schema_version; expected 0.1".to_owned());
    }

    let scenario_id = values
        .remove("scenario_id")
        .ok_or_else(|| "missing scenario_id".to_owned())?;
    if !scenario_id
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(
            "scenario_id must contain only lowercase ASCII, digits, and hyphens".to_owned(),
        );
    }

    let parse_number = |key: &str| -> Result<f64, String> {
        let raw = values.get(key).ok_or_else(|| format!("missing {key}"))?;
        let number = raw
            .parse::<f64>()
            .map_err(|_| format!("{key} must be a number"))?;
        if !number.is_finite() {
            return Err(format!("{key} must be finite"));
        }
        Ok(number)
    };

    Ok(MissionInput {
        scenario_id,
        distance_ly: parse_number("distance_ly")?,
        spacecraft_mass_kg: parse_number("spacecraft_mass_kg")?,
        peak_speed_fraction_c: parse_number("peak_speed_fraction_c")?,
        proper_acceleration_m_s2: parse_number("proper_acceleration_m_s2")?,
    })
}

/// Evaluate a 1D flat-spacetime profile:
/// constant proper acceleration, inertial cruise, constant proper deceleration.
/// The target is stationary in the departure frame. No propulsion or vehicle model is implied.
fn evaluate(input: &MissionInput) -> Result<MissionReport, String> {
    finite_positive(input.distance_ly, "distance_ly")?;
    finite_positive(input.spacecraft_mass_kg, "spacecraft_mass_kg")?;
    finite_positive(input.proper_acceleration_m_s2, "proper_acceleration_m_s2")?;

    let beta = input.peak_speed_fraction_c;
    if !beta.is_finite() || beta <= 0.0 || beta >= 1.0 {
        return Err("peak_speed_fraction_c must be strictly between zero and one".to_owned());
    }

    let gamma = 1.0 / (1.0 - beta * beta).sqrt();
    let rapidity = beta.atanh();
    let acceleration = input.proper_acceleration_m_s2;

    // Exact constant-proper-acceleration expressions in 1D Minkowski spacetime.
    let acceleration_coordinate_seconds = (C_M_S / acceleration) * gamma * beta;
    let acceleration_proper_seconds = (C_M_S / acceleration) * rapidity;
    let one_leg_distance_m = (C_M_S * C_M_S / acceleration) * (gamma - 1.0);

    let total_distance_m = input.distance_ly * LIGHT_YEAR_M;
    let acceleration_and_deceleration_m = 2.0 * one_leg_distance_m;
    let cruise_distance_m = total_distance_m - acceleration_and_deceleration_m;
    if cruise_distance_m < 0.0 {
        return Err(
            "mission is too short to reach peak speed and then decelerate symmetrically".to_owned(),
        );
    }

    let cruise_coordinate_seconds = cruise_distance_m / (beta * C_M_S);
    let coordinate_seconds = 2.0 * acceleration_coordinate_seconds + cruise_coordinate_seconds;
    let proper_seconds = 2.0 * acceleration_proper_seconds + cruise_coordinate_seconds / gamma;

    // This is kinetic energy at peak cruise speed, not total propulsion-system energy.
    // It excludes conversion losses, propulsion dry mass, reaction mass and braking recovery.
    let peak_kinetic_energy_j = (gamma - 1.0) * input.spacecraft_mass_kg * C_M_S * C_M_S;

    let computed_values = [
        ("lorentz_factor", gamma),
        (
            "acceleration_coordinate_seconds",
            acceleration_coordinate_seconds,
        ),
        ("acceleration_proper_seconds", acceleration_proper_seconds),
        ("one_leg_distance_m", one_leg_distance_m),
        ("total_distance_m", total_distance_m),
        ("cruise_distance_m", cruise_distance_m),
        ("cruise_coordinate_seconds", cruise_coordinate_seconds),
        ("coordinate_seconds", coordinate_seconds),
        ("proper_seconds", proper_seconds),
        ("peak_kinetic_energy_j", peak_kinetic_energy_j),
    ];
    for (name, value) in computed_values {
        if !value.is_finite() {
            return Err(format!(
                "computed {name} is not finite; input exceeds numeric range"
            ));
        }
    }

    Ok(MissionReport {
        scenario_id: input.scenario_id.clone(),
        distance_ly: input.distance_ly,
        peak_speed_fraction_c: beta,
        lorentz_factor: gamma,
        acceleration_distance_ly: one_leg_distance_m / LIGHT_YEAR_M,
        deceleration_distance_ly: one_leg_distance_m / LIGHT_YEAR_M,
        cruise_distance_ly: cruise_distance_m / LIGHT_YEAR_M,
        coordinate_flight_years: coordinate_seconds / SECONDS_PER_JULIAN_YEAR,
        spacecraft_proper_years: proper_seconds / SECONDS_PER_JULIAN_YEAR,
        peak_kinetic_energy_j,
        peak_kinetic_energy_kwh: peak_kinetic_energy_j / JOULES_PER_KWH,
    })
}

fn render_json(report: &MissionReport) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"0.1\",\n",
            "  \"evidence_class\": \"illustrative_model_output\",\n",
            "  \"scenario_id\": \"{}\",\n",
            "  \"distance_ly\": {:.12},\n",
            "  \"peak_speed_fraction_c\": {:.12},\n",
            "  \"lorentz_factor\": {:.12},\n",
            "  \"acceleration_distance_ly\": {:.12},\n",
            "  \"deceleration_distance_ly\": {:.12},\n",
            "  \"cruise_distance_ly\": {:.12},\n",
            "  \"coordinate_flight_years\": {:.12},\n",
            "  \"spacecraft_proper_years\": {:.12},\n",
            "  \"peak_kinetic_energy_j\": {:.12e},\n",
            "  \"peak_kinetic_energy_kwh\": {:.12e},\n",
            "  \"claim_ceiling\": \"1D kinematics only; no propulsion, braking hardware, shielding, dust, navigation, power, or flight qualification\"\n",
            "}}"
        ),
        report.scenario_id,
        report.distance_ly,
        report.peak_speed_fraction_c,
        report.lorentz_factor,
        report.acceleration_distance_ly,
        report.deceleration_distance_ly,
        report.cruise_distance_ly,
        report.coordinate_flight_years,
        report.spacecraft_proper_years,
        report.peak_kinetic_energy_j,
        report.peak_kinetic_energy_kwh,
    )
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let input_path = match args.as_slice() {
        [] => "fixtures/proxima-4p24ly-stop-at-target-0p1c.input".to_owned(),
        [flag, path] if flag == "--input" => path.clone(),
        _ => return Err("usage: interstellar-precursor-bench [--input PATH]".to_owned()),
    };

    let source = fs::read_to_string(&input_path)
        .map_err(|error| format!("could not read {input_path}: {error}"))?;
    let input = parse_input(&source)?;
    let report = evaluate(&input)?;
    println!("{}", render_json(&report));
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/proxima-4p24ly-stop-at-target-0p1c.input");

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual}, expected {expected}, tolerance {tolerance}"
        );
    }

    #[test]
    fn fixture_parses_and_replays_reference_case() {
        let input = parse_input(FIXTURE).expect("valid frozen fixture");
        let report = evaluate(&input).expect("valid mission profile");
        assert_eq!(report.scenario_id, "proxima-centauri-stop-at-target-0p1c");
        close(report.lorentz_factor, 1.005_037_815_259_212, 1e-12);
        close(report.coordinate_flight_years, 43.352_371_885_495_91, 1e-9);
        close(report.spacecraft_proper_years, 43.141_436_900_950_91, 1e-9);
        close(report.cruise_distance_ly, 4.144_283_024_088_708, 1e-12);
    }

    #[test]
    fn acceleration_and_deceleration_are_both_accounted_for() {
        let input = parse_input(FIXTURE).expect("valid frozen fixture");
        let report = evaluate(&input).expect("valid mission profile");
        close(
            report.acceleration_distance_ly,
            report.deceleration_distance_ly,
            0.0,
        );
        assert!(report.acceleration_distance_ly > 0.0);
        assert!(report.cruise_distance_ly < report.distance_ly);
    }

    #[test]
    fn peak_kinetic_energy_matches_relativistic_reference() {
        let input = parse_input(FIXTURE).expect("valid frozen fixture");
        let report = evaluate(&input).expect("valid mission profile");
        close(report.peak_kinetic_energy_j, 4.527_762_553_736_236e17, 1e7);
        close(
            report.peak_kinetic_energy_kwh,
            1.257_711_820_482_287_8e11,
            1e-1,
        );
    }

    #[test]
    fn rejects_duplicate_and_unknown_fixture_keys() {
        let duplicate = format!("{FIXTURE}distance_ly=4.24\n");
        assert!(
            parse_input(&duplicate)
                .unwrap_err()
                .contains("duplicate key")
        );
        let unknown = format!("{FIXTURE}warp_factor=2\n");
        assert!(parse_input(&unknown).unwrap_err().contains("unknown key"));
    }

    #[test]
    fn rejects_non_finite_and_superluminal_values() {
        let non_finite = FIXTURE.replace("distance_ly=4.24", "distance_ly=NaN");
        assert!(parse_input(&non_finite).unwrap_err().contains("finite"));
        let superluminal =
            FIXTURE.replace("peak_speed_fraction_c=0.1", "peak_speed_fraction_c=1.0");
        let input = parse_input(&superluminal).expect("numeric fixture is parseable");
        assert!(evaluate(&input).unwrap_err().contains("strictly between"));
    }

    #[test]
    fn rejects_a_mission_too_short_to_accelerate_and_decelerate() {
        let short = FIXTURE.replace("distance_ly=4.24", "distance_ly=0.001");
        let input = parse_input(&short).expect("numeric fixture is parseable");
        assert!(evaluate(&input).unwrap_err().contains("too short"));
    }

    #[test]
    fn rejects_zero_mass_or_acceleration() {
        let zero_mass = FIXTURE.replace("spacecraft_mass_kg=1000", "spacecraft_mass_kg=0");
        let input = parse_input(&zero_mass).expect("numeric fixture is parseable");
        assert!(evaluate(&input).unwrap_err().contains("spacecraft_mass_kg"));

        let zero_accel =
            FIXTURE.replace("proper_acceleration_m_s2=1.0", "proper_acceleration_m_s2=0");
        let input = parse_input(&zero_accel).expect("numeric fixture is parseable");
        assert!(
            evaluate(&input)
                .unwrap_err()
                .contains("proper_acceleration_m_s2")
        );
    }
}
