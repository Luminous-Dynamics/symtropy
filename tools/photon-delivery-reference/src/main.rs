use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::ExitCode;

const REQUIRED_KEYS: [&str; 19] = [
    "schema_version",
    "scenario_id",
    "evidence_class",
    "target_dli_mol_m2_day",
    "canopy_area_m2",
    "solar_ppfd_umol_m2_s",
    "collector_area_m2",
    "illumination_hours_per_day",
    "collector_efficiency",
    "relay_transmission",
    "canopy_uniformity_floor",
    "optical_availability",
    "led_efficacy_umol_j",
    "led_hours_per_day",
    "optical_auxiliary_electricity_wh_day",
    "optical_auxiliary_peak_kw",
    "available_lighting_power_kw",
    "available_lighting_energy_kwh_day",
    "led_canopy_uniformity_floor",
];

#[derive(Debug, Clone, PartialEq)]
struct PhotonInput {
    scenario_id: String,
    target_dli_mol_m2_day: f64,
    canopy_area_m2: f64,
    solar_ppfd_umol_m2_s: f64,
    collector_area_m2: f64,
    illumination_hours_per_day: f64,
    collector_efficiency: f64,
    relay_transmission: f64,
    canopy_uniformity_floor: f64,
    optical_availability: f64,
    led_efficacy_umol_j: f64,
    led_canopy_uniformity_floor: f64,
    led_hours_per_day: f64,
    optical_auxiliary_electricity_wh_day: f64,
    optical_auxiliary_peak_kw: f64,
    available_lighting_power_kw: f64,
    available_lighting_energy_kwh_day: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct PhotonReport {
    scenario_id: String,
    target_dli_mol_m2_day: f64,
    solar_mean_dli_mol_m2_day: f64,
    solar_floor_dli_mol_m2_day: f64,
    led_supplement_dli_mol_m2_day: f64,
    led_requested_mean_dli_mol_m2_day: f64,
    led_delivered_floor_dli_mol_m2_day: f64,
    resource_constrained_solar_dli_mol_m2_day: f64,
    optical_service_fraction: f64,
    resource_constrained_optical_auxiliary_electricity_kwh_day: f64,
    resource_constrained_dli_mol_m2_day: f64,
    resource_constrained_dli_shortfall_mol_m2_day: f64,
    led_only_electricity_kwh_day: f64,
    led_supplement_electricity_kwh_day: f64,
    led_delivered_electricity_kwh_day: f64,
    resource_constrained_hybrid_electricity_kwh_day: f64,
    optical_auxiliary_electricity_kwh_day: f64,
    hybrid_lighting_electricity_kwh_day: f64,
    led_peak_power_kw: f64,
    hybrid_peak_power_kw: f64,
    available_lighting_power_kw: f64,
    available_lighting_energy_kwh_day: f64,
    requested_energy_saving_vs_led_only_percent: f64,
    dli_met: bool,
    peak_power_feasible: bool,
    daily_energy_feasible: bool,
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

fn non_negative(value: f64, field: &str) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!("{field} must be finite and non-negative"))
    }
}

fn positive(value: f64, field: &str) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{field} must be finite and greater than zero"))
    }
}

fn unit_interval(value: f64, field: &str) -> Result<(), String> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(format!("{field} must be within [0, 1]"))
    }
}

fn parse_input(source: &str) -> Result<PhotonInput, String> {
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

    Ok(PhotonInput {
        scenario_id,
        target_dli_mol_m2_day: parse_f64(&values, "target_dli_mol_m2_day")?,
        canopy_area_m2: parse_f64(&values, "canopy_area_m2")?,
        solar_ppfd_umol_m2_s: parse_f64(&values, "solar_ppfd_umol_m2_s")?,
        collector_area_m2: parse_f64(&values, "collector_area_m2")?,
        illumination_hours_per_day: parse_f64(&values, "illumination_hours_per_day")?,
        collector_efficiency: parse_f64(&values, "collector_efficiency")?,
        relay_transmission: parse_f64(&values, "relay_transmission")?,
        canopy_uniformity_floor: parse_f64(&values, "canopy_uniformity_floor")?,
        optical_availability: parse_f64(&values, "optical_availability")?,
        led_efficacy_umol_j: parse_f64(&values, "led_efficacy_umol_j")?,
        led_canopy_uniformity_floor: parse_f64(&values, "led_canopy_uniformity_floor")?,
        led_hours_per_day: parse_f64(&values, "led_hours_per_day")?,
        optical_auxiliary_electricity_wh_day: parse_f64(
            &values,
            "optical_auxiliary_electricity_wh_day",
        )?,
        optical_auxiliary_peak_kw: parse_f64(&values, "optical_auxiliary_peak_kw")?,
        available_lighting_power_kw: parse_f64(&values, "available_lighting_power_kw")?,
        available_lighting_energy_kwh_day: parse_f64(&values, "available_lighting_energy_kwh_day")?,
    })
}

fn validate(input: &PhotonInput) -> Result<(), String> {
    positive(input.target_dli_mol_m2_day, "target_dli_mol_m2_day")?;
    positive(input.canopy_area_m2, "canopy_area_m2")?;
    non_negative(input.solar_ppfd_umol_m2_s, "solar_ppfd_umol_m2_s")?;
    non_negative(input.collector_area_m2, "collector_area_m2")?;
    if !input.illumination_hours_per_day.is_finite()
        || !(0.0..=24.0).contains(&input.illumination_hours_per_day)
    {
        return Err("illumination_hours_per_day must be within [0, 24]".to_owned());
    }
    unit_interval(input.collector_efficiency, "collector_efficiency")?;
    unit_interval(input.relay_transmission, "relay_transmission")?;
    unit_interval(input.canopy_uniformity_floor, "canopy_uniformity_floor")?;
    unit_interval(input.optical_availability, "optical_availability")?;
    positive(input.led_efficacy_umol_j, "led_efficacy_umol_j")?;
    unit_interval(
        input.led_canopy_uniformity_floor,
        "led_canopy_uniformity_floor",
    )?;
    if input.led_canopy_uniformity_floor == 0.0 {
        return Err("led_canopy_uniformity_floor must be greater than zero".to_owned());
    }
    positive(input.led_hours_per_day, "led_hours_per_day")?;
    if input.led_hours_per_day > 24.0 {
        return Err("led_hours_per_day must not exceed 24".to_owned());
    }
    non_negative(
        input.optical_auxiliary_electricity_wh_day,
        "optical_auxiliary_electricity_wh_day",
    )?;
    non_negative(input.optical_auxiliary_peak_kw, "optical_auxiliary_peak_kw")?;
    non_negative(
        input.available_lighting_power_kw,
        "available_lighting_power_kw",
    )?;
    non_negative(
        input.available_lighting_energy_kwh_day,
        "available_lighting_energy_kwh_day",
    )?;
    Ok(())
}

fn checked_finite(value: f64, field: &str) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("computed {field} is non-finite"))
    }
}

fn evaluate(input: &PhotonInput) -> Result<PhotonReport, String> {
    validate(input)?;

    let solar_window_seconds = input.illumination_hours_per_day * 3600.0;
    let collected_photons_umol =
        input.solar_ppfd_umol_m2_s * input.collector_area_m2 * solar_window_seconds;
    let after_collector_umol = collected_photons_umol * input.collector_efficiency;
    let after_relay_umol = after_collector_umol * input.relay_transmission;
    let at_canopy_umol = after_relay_umol * input.optical_availability;

    // DLI is per canopy area. The uniformity floor converts the mean delivery
    // into a conservative minimum-delivery estimate for the weakest canopy zone.
    let solar_mean_dli = at_canopy_umol / input.canopy_area_m2 / 1_000_000.0;
    let solar_floor_dli = solar_mean_dli * input.canopy_uniformity_floor;
    let led_supplement_dli = (input.target_dli_mol_m2_day - solar_floor_dli).max(0.0);
    let led_requested_mean_dli = led_supplement_dli / input.led_canopy_uniformity_floor;

    // LED photon efficacy is defined at the declared electrical input boundary.
    // The LED uniformity floor converts mean LED DLI into the minimum-zone contribution.
    let led_only_mean_dli = input.target_dli_mol_m2_day / input.led_canopy_uniformity_floor;
    let led_only_photons_umol = led_only_mean_dli * 1_000_000.0 * input.canopy_area_m2;
    let led_supplement_photons_umol = led_requested_mean_dli * 1_000_000.0 * input.canopy_area_m2;
    let led_only_j = led_only_photons_umol / input.led_efficacy_umol_j;
    let led_supplement_j = led_supplement_photons_umol / input.led_efficacy_umol_j;
    let led_only_kwh = led_only_j / 3_600_000.0;
    let led_supplement_kwh = led_supplement_j / 3_600_000.0;
    let optical_aux_kwh = input.optical_auxiliary_electricity_wh_day / 1000.0;
    let hybrid_electricity_kwh = led_supplement_kwh + optical_aux_kwh;

    let led_peak_kw = led_supplement_kwh / input.led_hours_per_day;
    let hybrid_peak_kw = led_peak_kw + input.optical_auxiliary_peak_kw;
    let energy_saving_percent = (led_only_kwh - hybrid_electricity_kwh) / led_only_kwh * 100.0;

    // Allocate declared lighting resources to the optical path first. If its
    // tracking/relay auxiliaries cannot be powered, its photon contribution is
    // reduced proportionally rather than being counted as free illumination.
    let optical_power_service_fraction = if input.optical_auxiliary_peak_kw > 0.0 {
        (input.available_lighting_power_kw / input.optical_auxiliary_peak_kw).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let optical_energy_service_fraction = if optical_aux_kwh > 0.0 {
        (input.available_lighting_energy_kwh_day / optical_aux_kwh).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let optical_service_fraction =
        optical_power_service_fraction.min(optical_energy_service_fraction);
    let resource_constrained_optical_auxiliary_electricity_kwh =
        optical_aux_kwh * optical_service_fraction;
    let resource_constrained_optical_auxiliary_peak_kw =
        input.optical_auxiliary_peak_kw * optical_service_fraction;
    let resource_constrained_solar_dli = solar_floor_dli * optical_service_fraction;

    // Remaining resources are available to the LED supplement. Recompute its
    // requested energy because an underpowered optical path creates a larger gap.
    let resource_led_supplement_dli =
        (input.target_dli_mol_m2_day - resource_constrained_solar_dli).max(0.0);
    let resource_led_requested_mean_dli =
        resource_led_supplement_dli / input.led_canopy_uniformity_floor;
    let resource_led_required_photons_umol =
        resource_led_requested_mean_dli * 1_000_000.0 * input.canopy_area_m2;
    let resource_led_required_energy_kwh =
        resource_led_required_photons_umol / input.led_efficacy_umol_j / 3_600_000.0;
    let led_power_available_kw =
        (input.available_lighting_power_kw - resource_constrained_optical_auxiliary_peak_kw)
            .max(0.0);
    let led_energy_available_kwh =
        (input.available_lighting_energy_kwh_day
            - resource_constrained_optical_auxiliary_electricity_kwh)
            .max(0.0);
    let led_energy_by_power_limit_kwh = led_power_available_kw * input.led_hours_per_day;
    let led_energy_delivered_kwh = resource_led_required_energy_kwh
        .min(led_energy_available_kwh)
        .min(led_energy_by_power_limit_kwh);
    let led_delivered_mean_dli = led_energy_delivered_kwh * 3_600_000.0 * input.led_efficacy_umol_j
        / 1_000_000.0
        / input.canopy_area_m2;
    let led_delivered_floor_dli = led_delivered_mean_dli * input.led_canopy_uniformity_floor;
    let resource_constrained_hybrid_electricity_kwh =
        led_energy_delivered_kwh + resource_constrained_optical_auxiliary_electricity_kwh;
    let resource_constrained_dli =
        resource_constrained_solar_dli + led_delivered_floor_dli;
    let resource_constrained_dli_shortfall =
        (input.target_dli_mol_m2_day - resource_constrained_dli).max(0.0);

    let computed = [
        ("solar_window_seconds", solar_window_seconds),
        ("collected_photons_umol", collected_photons_umol),
        ("after_collector_umol", after_collector_umol),
        ("after_relay_umol", after_relay_umol),
        ("at_canopy_umol", at_canopy_umol),
        ("solar_mean_dli", solar_mean_dli),
        ("solar_floor_dli", solar_floor_dli),
        ("led_supplement_dli", led_supplement_dli),
        ("led_only_mean_dli", led_only_mean_dli),
        ("led_requested_mean_dli", led_requested_mean_dli),
        ("led_delivered_mean_dli", led_delivered_mean_dli),
        ("led_delivered_floor_dli", led_delivered_floor_dli),
        ("optical_service_fraction", optical_service_fraction),
        ("resource_constrained_optical_auxiliary_electricity_kwh", resource_constrained_optical_auxiliary_electricity_kwh),
        ("resource_constrained_solar_dli", resource_constrained_solar_dli),
        ("resource_constrained_dli", resource_constrained_dli),
        (
            "resource_constrained_dli_shortfall",
            resource_constrained_dli_shortfall,
        ),
        ("led_energy_delivered_kwh", led_energy_delivered_kwh),
        (
            "resource_constrained_hybrid_electricity_kwh",
            resource_constrained_hybrid_electricity_kwh,
        ),
        ("led_only_kwh", led_only_kwh),
        ("led_supplement_kwh", led_supplement_kwh),
        ("optical_aux_kwh", optical_aux_kwh),
        ("hybrid_electricity_kwh", hybrid_electricity_kwh),
        ("led_peak_kw", led_peak_kw),
        ("hybrid_peak_kw", hybrid_peak_kw),
        ("energy_saving_percent", energy_saving_percent),
    ];
    for (name, value) in computed {
        checked_finite(value, name)?;
    }

    Ok(PhotonReport {
        scenario_id: input.scenario_id.clone(),
        target_dli_mol_m2_day: input.target_dli_mol_m2_day,
        solar_mean_dli_mol_m2_day: solar_mean_dli,
        solar_floor_dli_mol_m2_day: solar_floor_dli,
        led_supplement_dli_mol_m2_day: led_supplement_dli,
        led_requested_mean_dli_mol_m2_day: led_requested_mean_dli,
        led_delivered_floor_dli_mol_m2_day: led_delivered_floor_dli,
        resource_constrained_solar_dli_mol_m2_day: resource_constrained_solar_dli,
        optical_service_fraction,
        resource_constrained_optical_auxiliary_electricity_kwh_day:
            resource_constrained_optical_auxiliary_electricity_kwh,
        resource_constrained_dli_mol_m2_day: resource_constrained_dli,
        resource_constrained_dli_shortfall_mol_m2_day: resource_constrained_dli_shortfall,
        led_only_electricity_kwh_day: led_only_kwh,
        led_supplement_electricity_kwh_day: led_supplement_kwh,
        led_delivered_electricity_kwh_day: led_energy_delivered_kwh,
        resource_constrained_hybrid_electricity_kwh_day:
            resource_constrained_hybrid_electricity_kwh,
        optical_auxiliary_electricity_kwh_day: optical_aux_kwh,
        hybrid_lighting_electricity_kwh_day: hybrid_electricity_kwh,
        led_peak_power_kw: led_peak_kw,
        hybrid_peak_power_kw: hybrid_peak_kw,
        available_lighting_power_kw: input.available_lighting_power_kw,
        available_lighting_energy_kwh_day: input.available_lighting_energy_kwh_day,
        requested_energy_saving_vs_led_only_percent: energy_saving_percent,
        dli_met: resource_constrained_dli + 1e-9 >= input.target_dli_mol_m2_day,
        peak_power_feasible: hybrid_peak_kw <= input.available_lighting_power_kw,
        daily_energy_feasible: hybrid_electricity_kwh <= input.available_lighting_energy_kwh_day,
    })
}

fn render_json(report: &PhotonReport) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"0.1\",\n",
            "  \"evidence_class\": \"synthetic\",\n",
            "  \"scenario_id\": \"{}\",\n",
            "  \"target_dli_mol_m2_day\": {:.6},\n",
            "  \"solar_mean_dli_mol_m2_day\": {:.6},\n",
            "  \"solar_floor_dli_mol_m2_day\": {:.6},\n",
            "  \"led_supplement_dli_mol_m2_day\": {:.6},\n",
            "  \"led_requested_mean_dli_mol_m2_day\": {:.6},\n",
            "  \"led_delivered_floor_dli_mol_m2_day\": {:.6},\n",
            "  \"resource_constrained_solar_dli_mol_m2_day\": {:.6},\n",
            "  \"optical_service_fraction\": {:.6},\n",
            "  \"resource_constrained_optical_auxiliary_electricity_kwh_day\": {:.6},\n",
            "  \"resource_constrained_dli_mol_m2_day\": {:.6},\n",
            "  \"resource_constrained_dli_shortfall_mol_m2_day\": {:.6},\n",
            "  \"led_only_electricity_kwh_day\": {:.6},\n",
            "  \"led_supplement_electricity_kwh_day\": {:.6},\n",
            "  \"led_delivered_electricity_kwh_day\": {:.6},\n",
            "  \"resource_constrained_hybrid_electricity_kwh_day\": {:.6},\n",
            "  \"optical_auxiliary_electricity_kwh_day\": {:.6},\n",
            "  \"hybrid_lighting_electricity_kwh_day\": {:.6},\n",
            "  \"led_peak_power_kw\": {:.6},\n",
            "  \"hybrid_peak_power_kw\": {:.6},\n",
            "  \"available_lighting_power_kw\": {:.6},\n",
            "  \"available_lighting_energy_kwh_day\": {:.6},\n",
            "  \"requested_energy_saving_vs_led_only_percent\": {:.6},\n",
            "  \"dli_met\": {},\n",
            "  \"peak_power_feasible\": {},\n",
            "  \"daily_energy_feasible\": {},\n",
            "  \"claim_ceiling\": \"synthetic photon and LED electrical budget only; no crop growth, yield, optical hardware, site or flight qualification\"\n",
            "}}"
        ),
        report.scenario_id,
        report.target_dli_mol_m2_day,
        report.solar_mean_dli_mol_m2_day,
        report.solar_floor_dli_mol_m2_day,
        report.led_supplement_dli_mol_m2_day,
        report.led_requested_mean_dli_mol_m2_day,
        report.led_delivered_floor_dli_mol_m2_day,
        report.resource_constrained_solar_dli_mol_m2_day,
        report.optical_service_fraction,
        report.resource_constrained_optical_auxiliary_electricity_kwh_day,
        report.resource_constrained_dli_mol_m2_day,
        report.resource_constrained_dli_shortfall_mol_m2_day,
        report.led_only_electricity_kwh_day,
        report.led_supplement_electricity_kwh_day,
        report.led_delivered_electricity_kwh_day,
        report.resource_constrained_hybrid_electricity_kwh_day,
        report.optical_auxiliary_electricity_kwh_day,
        report.hybrid_lighting_electricity_kwh_day,
        report.led_peak_power_kw,
        report.hybrid_peak_power_kw,
        report.available_lighting_power_kw,
        report.available_lighting_energy_kwh_day,
        report.requested_energy_saving_vs_led_only_percent,
        report.dli_met,
        report.peak_power_feasible,
        report.daily_energy_feasible,
    )
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let input_path = match args.as_slice() {
        [] => "fixtures/synthetic-lunar-cavern-hybrid-v0-1.input".to_owned(),
        [flag, path] if flag == "--input" => path.clone(),
        _ => return Err("usage: photon-delivery-reference [--input PATH]".to_owned()),
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

    const FIXTURE: &str = include_str!("../fixtures/synthetic-lunar-cavern-hybrid-v0-1.input");

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual}, expected {expected}, tolerance {tolerance}"
        );
    }

    #[test]
    fn frozen_hybrid_fixture_reproduces_source_to_canopy_budget() {
        let input = parse_input(FIXTURE).expect("valid synthetic fixture");
        let report = evaluate(&input).expect("valid photon report");
        close(report.solar_mean_dli_mol_m2_day, 15.552, 1e-12);
        close(report.solar_floor_dli_mol_m2_day, 12.4416, 1e-12);
        close(report.led_supplement_dli_mol_m2_day, 7.5584, 1e-12);
        close(report.led_delivered_floor_dli_mol_m2_day, 5.6448, 1e-12);
        close(report.resource_constrained_dli_mol_m2_day, 18.0864, 1e-12);
        close(
            report.resource_constrained_dli_shortfall_mol_m2_day,
            1.9136,
            1e-12,
        );
        assert!(!report.dli_met);
    }

    #[test]
    fn hybrid_led_energy_is_lower_than_led_only_but_includes_optical_auxiliaries() {
        let input = parse_input(FIXTURE).expect("valid synthetic fixture");
        let report = evaluate(&input).expect("valid photon report");
        close(report.led_only_electricity_kwh_day, 2222.222222222222, 1e-9);
        close(
            report.led_supplement_electricity_kwh_day,
            839.8222222222222,
            1e-9,
        );
        close(report.optical_auxiliary_electricity_kwh_day, 20.0, 1e-12);
        close(
            report.hybrid_lighting_electricity_kwh_day,
            859.8222222222222,
            1e-9,
        );
        assert!(report.requested_energy_saving_vs_led_only_percent > 61.0);
        close(report.led_delivered_electricity_kwh_day, 627.2, 1e-9);
        close(
            report.resource_constrained_hybrid_electricity_kwh_day,
            647.2,
            1e-9,
        );
    }

    #[test]
    fn power_peak_and_daily_energy_are_separate_feasibility_checks() {
        let input = parse_input(FIXTURE).expect("valid synthetic fixture");
        let report = evaluate(&input).expect("valid photon report");
        close(report.hybrid_peak_power_kw, 53.28888888888889, 1e-9);
        assert!(!report.peak_power_feasible);
        assert!(report.daily_energy_feasible);
        assert!(!report.dli_met);
        close(
            report.resource_constrained_dli_shortfall_mol_m2_day,
            1.9136,
            1e-12,
        );
    }

    #[test]
    fn led_only_case_works_with_zero_solar_aperture() {
        let led_only = FIXTURE.replace("collector_area_m2=2000.0", "collector_area_m2=0.0");
        let input = parse_input(&led_only).expect("syntactically valid fixture");
        let report = evaluate(&input).expect("valid LED-only case");
        close(report.solar_floor_dli_mol_m2_day, 0.0, 0.0);
        close(
            report.led_supplement_electricity_kwh_day,
            report.led_only_electricity_kwh_day,
            1e-9,
        );
    }

    #[test]
    fn led_uniformity_loss_increases_required_power_and_reduces_delivered_minimum_dli() {
        let uneven_led = FIXTURE.replace(
            "led_canopy_uniformity_floor=1.0",
            "led_canopy_uniformity_floor=0.5",
        );
        let input = parse_input(&uneven_led).expect("syntactically valid fixture");
        let report = evaluate(&input).expect("valid uneven LED case");
        close(
            report.led_supplement_electricity_kwh_day,
            1679.6444444444444,
            1e-9,
        );
        close(report.led_delivered_floor_dli_mol_m2_day, 2.8224, 1e-12);
        close(report.resource_constrained_dli_mol_m2_day, 15.264, 1e-12);
        close(
            report.resource_constrained_dli_shortfall_mol_m2_day,
            4.736,
            1e-12,
        );
        assert!(!report.dli_met);
    }

    #[test]
    fn unavailable_optical_auxiliary_power_reduces_the_solar_photon_contribution() {
        let underpowered_optics = FIXTURE
            .replace(
                "available_lighting_power_kw=40.0",
                "available_lighting_power_kw=0.4",
            );
        let input = parse_input(&underpowered_optics).expect("syntactically valid fixture");
        let report = evaluate(&input).expect("valid underpowered optical case");
        close(report.optical_service_fraction, 0.5, 1e-12);
        close(
            report.resource_constrained_optical_auxiliary_electricity_kwh_day,
            10.0,
            1e-12,
        );
        close(
            report.resource_constrained_solar_dli_mol_m2_day,
            6.2208,
            1e-12,
        );
        close(report.resource_constrained_dli_mol_m2_day, 6.2208, 1e-12);
        assert!(!report.dli_met);
        assert!(!report.peak_power_feasible);
        assert!(report.daily_energy_feasible);
    }

    #[test]
    fn sufficient_peak_and_daily_resource_limits_allow_target_dli() {
        let high_capacity = FIXTURE
            .replace(
                "available_lighting_power_kw=40.0",
                "available_lighting_power_kw=100.0",
            )
            .replace(
                "available_lighting_energy_kwh_day=960.0",
                "available_lighting_energy_kwh_day=3000.0",
            );
        let input = parse_input(&high_capacity).expect("syntactically valid fixture");
        let report = evaluate(&input).expect("valid high-capacity case");
        assert!(report.peak_power_feasible);
        assert!(report.daily_energy_feasible);
        assert!(report.dli_met);
        close(report.resource_constrained_dli_mol_m2_day, 20.0, 1e-9);
        close(
            report.resource_constrained_dli_shortfall_mol_m2_day,
            0.0,
            0.0,
        );
    }

    #[test]
    fn daily_energy_limit_can_bind_even_when_peak_power_is_adequate() {
        let energy_limited = FIXTURE
            .replace(
                "available_lighting_power_kw=40.0",
                "available_lighting_power_kw=100.0",
            )
            .replace(
                "available_lighting_energy_kwh_day=960.0",
                "available_lighting_energy_kwh_day=300.0",
            );
        let input = parse_input(&energy_limited).expect("syntactically valid fixture");
        let report = evaluate(&input).expect("valid energy-limited case");
        assert!(report.peak_power_feasible);
        assert!(!report.daily_energy_feasible);
        assert!(!report.dli_met);
        close(report.resource_constrained_dli_mol_m2_day, 14.9616, 1e-9);
    }

    #[test]
    fn high_optical_auxiliary_load_can_make_hybrid_worse_than_led_only() {
        let high_aux = FIXTURE
            .replace(
                "optical_auxiliary_electricity_wh_day=20000.0",
                "optical_auxiliary_electricity_wh_day=3000000.0",
            )
            .replace(
                "optical_auxiliary_peak_kw=0.8",
                "optical_auxiliary_peak_kw=10.0",
            )
            .replace(
                "available_lighting_power_kw=40.0",
                "available_lighting_power_kw=100.0",
            )
            .replace(
                "available_lighting_energy_kwh_day=960.0",
                "available_lighting_energy_kwh_day=3000.0",
            );
        let input = parse_input(&high_aux).expect("syntactically valid fixture");
        let report = evaluate(&input).expect("valid high-auxiliary case");
        assert!(report.requested_energy_saving_vs_led_only_percent < 0.0);
        assert!(report.peak_power_feasible);
        assert!(!report.daily_energy_feasible);
    }

    #[test]
    fn missing_optical_efficiency_is_not_assumed_to_be_perfect() {
        let missing = FIXTURE.replace("relay_transmission=0.5\n", "");
        assert!(
            parse_input(&missing)
                .unwrap_err()
                .contains("missing required key relay_transmission")
        );
    }

    #[test]
    fn rejects_invalid_efficiencies_and_time_windows() {
        let bad_efficiency =
            FIXTURE.replace("collector_efficiency=0.8", "collector_efficiency=1.2");
        let input = parse_input(&bad_efficiency).expect("syntactically valid fixture");
        assert!(
            validate(&input)
                .unwrap_err()
                .contains("collector_efficiency")
        );

        let bad_window = FIXTURE.replace(
            "illumination_hours_per_day=6.0",
            "illumination_hours_per_day=25.0",
        );
        let input = parse_input(&bad_window).expect("syntactically valid fixture");
        assert!(
            validate(&input)
                .unwrap_err()
                .contains("illumination_hours_per_day")
        );

        let zero_led_uniformity = FIXTURE.replace(
            "led_canopy_uniformity_floor=1.0",
            "led_canopy_uniformity_floor=0.0",
        );
        let input = parse_input(&zero_led_uniformity).expect("syntactically valid fixture");
        assert!(
            validate(&input)
                .unwrap_err()
                .contains("led_canopy_uniformity_floor")
        );
    }

    #[test]
    fn rejects_duplicate_unknown_and_measured_claim_keys() {
        let duplicate = format!("{FIXTURE}target_dli_mol_m2_day=20.0\n");
        assert!(
            parse_input(&duplicate)
                .unwrap_err()
                .contains("duplicate key")
        );

        let unknown = format!("{FIXTURE}magic_optical_gain=4\n");
        assert!(parse_input(&unknown).unwrap_err().contains("unknown key"));

        let measured = FIXTURE.replace("evidence_class=synthetic", "evidence_class=measured");
        assert!(
            parse_input(&measured)
                .unwrap_err()
                .contains("synthetic fixtures only")
        );
    }

    #[test]
    fn rejects_non_finite_and_zero_led_efficacy() {
        let non_finite = FIXTURE.replace("target_dli_mol_m2_day=20.0", "target_dli_mol_m2_day=NaN");
        assert!(parse_input(&non_finite).unwrap_err().contains("finite"));

        let zero_efficacy = FIXTURE.replace("led_efficacy_umol_j=2.5", "led_efficacy_umol_j=0");
        let input = parse_input(&zero_efficacy).expect("syntactically valid fixture");
        assert!(
            validate(&input)
                .unwrap_err()
                .contains("led_efficacy_umol_j")
        );
    }
}
