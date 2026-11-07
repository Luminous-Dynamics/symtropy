use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::ExitCode;

const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;
const LIGHT_YEAR_M: f64 = 9.460_730_472_580_8e15;
const JOULES_PER_KWH: f64 = 3_600_000.0;
const REQUIRED_KEYS: [&str; 9] = [
    "schema_version",
    "scenario_id",
    "evidence_class",
    "spacecraft_projected_area_m2",
    "path_distance_ly",
    "speed_fraction_c",
    "grain_radius_m",
    "grain_density_kg_m3",
    "grain_number_density_m3",
];

#[derive(Debug, Clone, PartialEq)]
struct ExposureInput {
    scenario_id: String,
    spacecraft_projected_area_m2: f64,
    path_distance_ly: f64,
    speed_fraction_c: f64,
    grain_radius_m: f64,
    grain_density_kg_m3: f64,
    grain_number_density_m3: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct ExposureReport {
    scenario_id: String,
    path_distance_ly: f64,
    speed_fraction_c: f64,
    spacecraft_projected_area_m2: f64,
    grain_radius_m: f64,
    grain_density_kg_m3: f64,
    grain_number_density_m3: f64,
    path_distance_m: f64,
    grain_mass_kg: f64,
    lorentz_factor: f64,
    grain_kinetic_energy_j: f64,
    grain_momentum_kg_m_s: f64,
    newtonian_kinetic_energy_j: f64,
    relativistic_to_newtonian_energy_ratio: f64,
    expected_impacts: f64,
    probability_at_least_one_impact: f64,
    expected_intercepted_mass_kg: f64,
    expected_incident_kinetic_energy_j: f64,
    expected_incident_energy_per_area_j_m2: f64,
    expected_incident_energy_kwh: f64,
}

fn parse_f64(values: &BTreeMap<String, String>, key: &str) -> Result<f64, String> {
    let raw = values
        .get(key)
        .ok_or_else(|| format!("missing required key {key}"))?;
    let value = raw
        .parse::<f64>()
        .map_err(|_| format!("{key} must be a number"))?;
    if !value.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    Ok(value)
}

fn positive(value: f64, field: &str) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{field} must be finite and greater than zero"))
    }
}

fn non_negative(value: f64, field: &str) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!("{field} must be finite and non-negative"))
    }
}

fn parse_input(source: &str) -> Result<ExposureInput, String> {
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

    if values.get("schema_version").map(String::as_str) != Some("0.1") {
        return Err("unsupported schema_version; expected 0.1".to_owned());
    }
    if values.get("evidence_class").map(String::as_str) != Some("synthetic") {
        return Err("v0.1 reference runner accepts synthetic fixtures only".to_owned());
    }

    let scenario_id = values
        .get("scenario_id")
        .cloned()
        .ok_or_else(|| "missing scenario_id".to_owned())?;
    if scenario_id.is_empty()
        || !scenario_id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err("scenario_id must use lowercase ASCII, digits and hyphens".to_owned());
    }

    Ok(ExposureInput {
        scenario_id,
        spacecraft_projected_area_m2: parse_f64(&values, "spacecraft_projected_area_m2")?,
        path_distance_ly: parse_f64(&values, "path_distance_ly")?,
        speed_fraction_c: parse_f64(&values, "speed_fraction_c")?,
        grain_radius_m: parse_f64(&values, "grain_radius_m")?,
        grain_density_kg_m3: parse_f64(&values, "grain_density_kg_m3")?,
        grain_number_density_m3: parse_f64(&values, "grain_number_density_m3")?,
    })
}

fn validate(input: &ExposureInput) -> Result<(), String> {
    positive(
        input.spacecraft_projected_area_m2,
        "spacecraft_projected_area_m2",
    )?;
    positive(input.path_distance_ly, "path_distance_ly")?;
    if !input.speed_fraction_c.is_finite()
        || !(0.0..1.0).contains(&input.speed_fraction_c)
    {
        return Err("speed_fraction_c must be within [0, 1)".to_owned());
    }
    positive(input.grain_radius_m, "grain_radius_m")?;
    positive(input.grain_density_kg_m3, "grain_density_kg_m3")?;
    non_negative(input.grain_number_density_m3, "grain_number_density_m3")?;
    Ok(())
}

fn finite(value: f64, field: &str) -> Result<f64, String> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(format!("computed {field} is non-finite or negative"))
    }
}

fn evaluate(input: &ExposureInput) -> Result<ExposureReport, String> {
    validate(input)?;

    let beta = input.speed_fraction_c;
    let velocity_m_s = beta * SPEED_OF_LIGHT_M_S;
    let lorentz_factor = 1.0 / (1.0 - beta * beta).sqrt();
    let path_distance_m = input.path_distance_ly * LIGHT_YEAR_M;
    let grain_mass_kg =
        (4.0 / 3.0) * std::f64::consts::PI * input.grain_radius_m.powi(3)
            * input.grain_density_kg_m3;
    let grain_kinetic_energy_j =
        (lorentz_factor - 1.0) * grain_mass_kg * SPEED_OF_LIGHT_M_S.powi(2);
    let grain_momentum_kg_m_s = lorentz_factor * grain_mass_kg * velocity_m_s;
    let newtonian_kinetic_energy_j =
        0.5 * grain_mass_kg * velocity_m_s.powi(2);
    let relativistic_to_newtonian_energy_ratio = if beta == 0.0 {
        1.0
    } else {
        grain_kinetic_energy_j / newtonian_kinetic_energy_j
    };

    // Homogeneous, independent Poisson encounter model. It is not valid for
    // clustered or otherwise correlated dust without a different spatial model.
    let expected_impacts = input.grain_number_density_m3
        * input.spacecraft_projected_area_m2
        * path_distance_m;
    let probability_at_least_one_impact = -(-expected_impacts).exp_m1();
    let expected_intercepted_mass_kg = expected_impacts * grain_mass_kg;
    let expected_incident_kinetic_energy_j = expected_impacts * grain_kinetic_energy_j;
    let expected_incident_energy_per_area_j_m2 =
        expected_incident_kinetic_energy_j / input.spacecraft_projected_area_m2;
    let expected_incident_energy_kwh =
        expected_incident_kinetic_energy_j / JOULES_PER_KWH;

    let computed = [
        ("velocity_m_s", velocity_m_s),
        ("lorentz_factor", lorentz_factor),
        ("path_distance_m", path_distance_m),
        ("grain_mass_kg", grain_mass_kg),
        ("grain_kinetic_energy_j", grain_kinetic_energy_j),
        ("grain_momentum_kg_m_s", grain_momentum_kg_m_s),
        ("newtonian_kinetic_energy_j", newtonian_kinetic_energy_j),
        (
            "relativistic_to_newtonian_energy_ratio",
            relativistic_to_newtonian_energy_ratio,
        ),
        ("expected_impacts", expected_impacts),
        (
            "probability_at_least_one_impact",
            probability_at_least_one_impact,
        ),
        ("expected_intercepted_mass_kg", expected_intercepted_mass_kg),
        (
            "expected_incident_kinetic_energy_j",
            expected_incident_kinetic_energy_j,
        ),
        (
            "expected_incident_energy_per_area_j_m2",
            expected_incident_energy_per_area_j_m2,
        ),
        ("expected_incident_energy_kwh", expected_incident_energy_kwh),
    ];
    for (name, value) in computed {
        finite(value, name)?;
    }

    Ok(ExposureReport {
        scenario_id: input.scenario_id.clone(),
        path_distance_ly: input.path_distance_ly,
        speed_fraction_c: beta,
        spacecraft_projected_area_m2: input.spacecraft_projected_area_m2,
        grain_radius_m: input.grain_radius_m,
        grain_density_kg_m3: input.grain_density_kg_m3,
        grain_number_density_m3: input.grain_number_density_m3,
        path_distance_m,
        grain_mass_kg,
        lorentz_factor,
        grain_kinetic_energy_j,
        grain_momentum_kg_m_s,
        newtonian_kinetic_energy_j,
        relativistic_to_newtonian_energy_ratio,
        expected_impacts,
        probability_at_least_one_impact,
        expected_intercepted_mass_kg,
        expected_incident_kinetic_energy_j,
        expected_incident_energy_per_area_j_m2,
        expected_incident_energy_kwh,
    })
}

fn render_json(report: &ExposureReport) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"0.1\",\n",
            "  \"evidence_class\": \"synthetic\",\n",
            "  \"scenario_id\": \"{}\",\n",
            "  \"path_distance_ly\": {:.12},\n",
            "  \"speed_fraction_c\": {:.12},\n",
            "  \"spacecraft_projected_area_m2\": {:.12e},\n",
            "  \"grain_radius_m\": {:.12e},\n",
            "  \"grain_density_kg_m3\": {:.12},\n",
            "  \"grain_number_density_m3\": {:.12e},\n",
            "  \"path_distance_m\": {:.12e},\n",
            "  \"grain_mass_kg\": {:.12e},\n",
            "  \"lorentz_factor\": {:.12},\n",
            "  \"grain_kinetic_energy_j\": {:.12e},\n",
            "  \"grain_momentum_kg_m_s\": {:.12e},\n",
            "  \"newtonian_kinetic_energy_j\": {:.12e},\n",
            "  \"relativistic_to_newtonian_energy_ratio\": {:.12},\n",
            "  \"expected_impacts_homogeneous_poisson\": {:.12},\n",
            "  \"probability_at_least_one_impact_poisson\": {:.12},\n",
            "  \"expected_intercepted_mass_kg\": {:.12e},\n",
            "  \"expected_incident_kinetic_energy_j\": {:.12e},\n",
            "  \"expected_incident_energy_per_area_j_m2\": {:.12e},\n",
            "  \"expected_incident_energy_kwh\": {:.12e},\n",
            "  \"claim_ceiling\": \"analytical exposure bookkeeping only; no erosion, crater, material damage, shielding or survival prediction\"\n",
            "}}"
        ),
        report.scenario_id,
        report.path_distance_ly,
        report.speed_fraction_c,
        report.spacecraft_projected_area_m2,
        report.grain_radius_m,
        report.grain_density_kg_m3,
        report.grain_number_density_m3,
        report.path_distance_m,
        report.grain_mass_kg,
        report.lorentz_factor,
        report.grain_kinetic_energy_j,
        report.grain_momentum_kg_m_s,
        report.newtonian_kinetic_energy_j,
        report.relativistic_to_newtonian_energy_ratio,
        report.expected_impacts,
        report.probability_at_least_one_impact,
        report.expected_intercepted_mass_kg,
        report.expected_incident_kinetic_energy_j,
        report.expected_incident_energy_per_area_j_m2,
        report.expected_incident_energy_kwh,
    )
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let input_path = match args.as_slice() {
        [] => "fixtures/synthetic-4p24ly-10nm-dust-v0-1.input".to_owned(),
        [flag, path] if flag == "--input" => path.clone(),
        _ => return Err("usage: interstellar-medium-reference [--input PATH]".to_owned()),
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

    const FIXTURE: &str =
        include_str!("../fixtures/synthetic-4p24ly-10nm-dust-v0-1.input");

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual}, expected {expected}, tolerance {tolerance}"
        );
    }

    #[test]
    fn fixture_reproduces_grain_mass_and_expected_encounter_count() {
        let input = parse_input(FIXTURE).expect("valid synthetic fixture");
        let report = evaluate(&input).expect("valid exposure report");
        close(report.grain_mass_kg, 1.256_637_061_435_917_3e-20, 1e-32);
        close(report.expected_impacts, 2.005_674_860_187_129_5, 1e-12);
        close(
            report.probability_at_least_one_impact,
            0.865_430_550_519_151_5,
            1e-12,
        );
    }

    #[test]
    fn relativistic_energy_and_momentum_are_consistent() {
        let input = parse_input(FIXTURE).expect("valid synthetic fixture");
        let report = evaluate(&input).expect("valid exposure report");
        assert!(report.grain_kinetic_energy_j > report.newtonian_kinetic_energy_j);
        assert!(report.relativistic_to_newtonian_energy_ratio > 1.0);
        close(
            report.grain_momentum_kg_m_s,
            report.lorentz_factor
                * report.grain_mass_kg
                * report.speed_fraction_c
                * SPEED_OF_LIGHT_M_S,
            1e-25,
        );
    }

    #[test]
    fn zero_density_means_zero_encounters_without_assuming_zero_dust_by_default() {
        let zero_density = FIXTURE.replace(
            "grain_number_density_m3=1.0e-6",
            "grain_number_density_m3=0.0",
        );
        let input = parse_input(&zero_density).expect("valid zero-density control");
        let report = evaluate(&input).expect("valid exposure report");
        close(report.expected_impacts, 0.0, 0.0);
        close(report.probability_at_least_one_impact, 0.0, 0.0);
        close(report.expected_incident_kinetic_energy_j, 0.0, 0.0);
    }

    #[test]
    fn newtonian_limit_is_recovered_at_low_speed() {
        let slow = FIXTURE.replace("speed_fraction_c=0.2", "speed_fraction_c=0.00001");
        let input = parse_input(&slow).expect("valid low-speed control");
        let report = evaluate(&input).expect("valid exposure report");
        close(report.relativistic_to_newtonian_energy_ratio, 1.0, 1e-9);
    }

    #[test]
    fn doubling_projected_area_doubles_expected_hits_but_not_energy_fluence() {
        let input = parse_input(FIXTURE).expect("valid synthetic fixture");
        let base = evaluate(&input).expect("valid exposure report");
        let double_area = ExposureInput {
            spacecraft_projected_area_m2: input.spacecraft_projected_area_m2 * 2.0,
            ..input
        };
        let doubled = evaluate(&double_area).expect("valid exposure report");
        close(doubled.expected_impacts, base.expected_impacts * 2.0, 1e-12);
        close(
            doubled.expected_incident_energy_per_area_j_m2,
            base.expected_incident_energy_per_area_j_m2,
            1e-9,
        );
    }

    #[test]
    fn doubling_grain_radius_increases_grain_mass_and_energy_eightfold() {
        let input = parse_input(FIXTURE).expect("valid synthetic fixture");
        let base = evaluate(&input).expect("valid exposure report");
        let larger = ExposureInput {
            grain_radius_m: input.grain_radius_m * 2.0,
            ..input
        };
        let doubled = evaluate(&larger).expect("valid exposure report");
        close(doubled.grain_mass_kg / base.grain_mass_kg, 8.0, 1e-12);
        close(
            doubled.grain_kinetic_energy_j / base.grain_kinetic_energy_j,
            8.0,
            1e-12,
        );
    }

    #[test]
    fn rejects_superluminal_non_finite_and_negative_density_inputs() {
        let superluminal = FIXTURE.replace("speed_fraction_c=0.2", "speed_fraction_c=1.0");
        let input = parse_input(&superluminal).expect("syntactically valid input");
        assert!(validate(&input).unwrap_err().contains("[0, 1)"));

        let non_finite = FIXTURE.replace("path_distance_ly=4.24", "path_distance_ly=NaN");
        assert!(parse_input(&non_finite).unwrap_err().contains("finite"));

        let negative_density =
            FIXTURE.replace("grain_number_density_m3=1.0e-6", "grain_number_density_m3=-1.0");
        let input = parse_input(&negative_density).expect("syntactically valid input");
        assert!(validate(&input).unwrap_err().contains("non-negative"));
    }

    #[test]
    fn rejects_unknown_and_duplicate_input_keys() {
        let unknown = format!("{FIXTURE}unknown_shield_efficiency=0.9\n");
        assert!(parse_input(&unknown).unwrap_err().contains("unknown key"));

        let duplicate = format!("{FIXTURE}speed_fraction_c=0.2\n");
        assert!(parse_input(&duplicate).unwrap_err().contains("duplicate key"));
    }
}
