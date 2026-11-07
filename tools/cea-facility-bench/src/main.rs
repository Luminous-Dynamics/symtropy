use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::ExitCode;

const REQUIRED_KEYS: [&str; 20] = [
    "schema_version",
    "scenario_id",
    "evidence_class",
    "period_days",
    "facility_area_m2",
    "lighting_electricity_wh",
    "hvac_electricity_wh",
    "pumping_electricity_wh",
    "water_treatment_electricity_wh",
    "co2_system_electricity_wh",
    "cold_storage_electricity_wh",
    "controls_electricity_wh",
    "gross_harvest_g",
    "accepted_edible_g",
    "rejected_g",
    "spoiled_g",
    "inedible_residue_g",
    "makeup_water_ml",
    "useful_heat_wh_th",
    "heat_rejected_wh_th",
];

#[derive(Debug, Clone, PartialEq)]
struct FacilityInput {
    scenario_id: String,
    evidence_class: String,
    period_days: u64,
    facility_area_m2: u64,
    electricity_wh: [u64; 7],
    gross_harvest_g: u64,
    accepted_edible_g: u64,
    rejected_g: u64,
    spoiled_g: u64,
    inedible_residue_g: u64,
    makeup_water_ml: u64,
    useful_heat_wh_th: u64,
    heat_rejected_wh_th: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct FacilityReport {
    scenario_id: String,
    evidence_class: String,
    period_days: u64,
    facility_area_m2: u64,
    total_electricity_wh: u64,
    electricity_wh_by_subsystem: [u64; 7],
    accepted_edible_g: u64,
    gross_harvest_g: u64,
    rejected_g: u64,
    spoiled_g: u64,
    inedible_residue_g: u64,
    makeup_water_ml: u64,
    useful_heat_wh_th: u64,
    heat_rejected_wh_th: u64,
}

fn parse_u64(values: &BTreeMap<String, String>, key: &str) -> Result<u64, String> {
    let raw = values
        .get(key)
        .ok_or_else(|| format!("missing required key {key}"))?;
    raw.parse::<u64>()
        .map_err(|_| format!("{key} must be a non-negative integer"))
}

fn parse_input(source: &str) -> Result<FacilityInput, String> {
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

    let evidence_class = values
        .get("evidence_class")
        .cloned()
        .ok_or_else(|| "missing evidence_class".to_owned())?;
    if evidence_class != "synthetic" {
        return Err("v0.1 executable fixture profile accepts synthetic evidence only".to_owned());
    }

    let electricity_keys = [
        "lighting_electricity_wh",
        "hvac_electricity_wh",
        "pumping_electricity_wh",
        "water_treatment_electricity_wh",
        "co2_system_electricity_wh",
        "cold_storage_electricity_wh",
        "controls_electricity_wh",
    ];
    let mut electricity_wh = [0_u64; 7];
    for (index, key) in electricity_keys.iter().enumerate() {
        electricity_wh[index] = parse_u64(&values, key)?;
    }

    Ok(FacilityInput {
        scenario_id,
        evidence_class,
        period_days: parse_u64(&values, "period_days")?,
        facility_area_m2: parse_u64(&values, "facility_area_m2")?,
        electricity_wh,
        gross_harvest_g: parse_u64(&values, "gross_harvest_g")?,
        accepted_edible_g: parse_u64(&values, "accepted_edible_g")?,
        rejected_g: parse_u64(&values, "rejected_g")?,
        spoiled_g: parse_u64(&values, "spoiled_g")?,
        inedible_residue_g: parse_u64(&values, "inedible_residue_g")?,
        makeup_water_ml: parse_u64(&values, "makeup_water_ml")?,
        useful_heat_wh_th: parse_u64(&values, "useful_heat_wh_th")?,
        heat_rejected_wh_th: parse_u64(&values, "heat_rejected_wh_th")?,
    })
}

fn checked_sum(values: &[u64], at: &str) -> Result<u64, String> {
    values.iter().try_fold(0_u64, |total, value| {
        total
            .checked_add(*value)
            .ok_or_else(|| format!("{at}: integer overflow"))
    })
}

fn evaluate(input: &FacilityInput) -> Result<FacilityReport, String> {
    if input.period_days == 0 {
        return Err("period_days must be greater than zero".to_owned());
    }
    if input.facility_area_m2 == 0 {
        return Err("facility_area_m2 must be greater than zero".to_owned());
    }
    if input.gross_harvest_g == 0 || input.accepted_edible_g == 0 {
        return Err("gross and accepted edible harvest must be greater than zero".to_owned());
    }

    let accounted_harvest_g = checked_sum(
        &[
            input.accepted_edible_g,
            input.rejected_g,
            input.spoiled_g,
            input.inedible_residue_g,
        ],
        "harvest mass balance",
    )?;
    if accounted_harvest_g != input.gross_harvest_g {
        return Err(format!(
            "harvest mass imbalance: gross {} g, accounted {} g",
            input.gross_harvest_g, accounted_harvest_g
        ));
    }

    let total_electricity_wh = checked_sum(&input.electricity_wh, "electricity ledger")?;
    if total_electricity_wh == 0 {
        return Err("total facility electricity must be greater than zero".to_owned());
    }

    Ok(FacilityReport {
        scenario_id: input.scenario_id.clone(),
        evidence_class: input.evidence_class.clone(),
        period_days: input.period_days,
        facility_area_m2: input.facility_area_m2,
        total_electricity_wh,
        electricity_wh_by_subsystem: input.electricity_wh,
        accepted_edible_g: input.accepted_edible_g,
        gross_harvest_g: input.gross_harvest_g,
        rejected_g: input.rejected_g,
        spoiled_g: input.spoiled_g,
        inedible_residue_g: input.inedible_residue_g,
        makeup_water_ml: input.makeup_water_ml,
        useful_heat_wh_th: input.useful_heat_wh_th,
        heat_rejected_wh_th: input.heat_rejected_wh_th,
    })
}

fn ratio(numerator: u128, denominator: u128) -> String {
    let value = numerator as f64 / denominator as f64;
    format!("{value:.6}")
}

fn render_json(report: &FacilityReport) -> Result<String, String> {
    let annualized_electricity_numerator = u128::from(report.total_electricity_wh)
        .checked_mul(365)
        .ok_or_else(|| "annualized electricity overflow".to_owned())?;
    let annualized_area_denominator = u128::from(report.period_days)
        .checked_mul(u128::from(report.facility_area_m2))
        .and_then(|value| value.checked_mul(1000))
        .ok_or_else(|| "annualized area denominator overflow".to_owned())?;

    let kwh_per_kg_accepted = ratio(
        u128::from(report.total_electricity_wh),
        u128::from(report.accepted_edible_g),
    );

    let kwh_per_m2_year = ratio(
        annualized_electricity_numerator,
        annualized_area_denominator,
    );

    let water_l_per_kg_accepted = ratio(
        u128::from(report.makeup_water_ml),
        u128::from(report.accepted_edible_g),
    );

    let accepted_percent = ratio(
        u128::from(report.accepted_edible_g) * 100,
        u128::from(report.gross_harvest_g),
    );

    Ok(format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"0.1\",\n",
            "  \"evidence_class\": \"{}\",\n",
            "  \"scenario_id\": \"{}\",\n",
            "  \"period_days\": {},\n",
            "  \"facility_area_m2\": {},\n",
            "  \"total_electricity_wh\": {},\n",
            "  \"electricity_wh_by_subsystem\": {{\n",
            "    \"lighting\": {},\n",
            "    \"hvac\": {},\n",
            "    \"pumping\": {},\n",
            "    \"water_treatment\": {},\n",
            "    \"co2_system\": {},\n",
            "    \"cold_storage\": {},\n",
            "    \"controls\": {}\n",
            "  }},\n",
            "  \"accepted_edible_g\": {},\n",
            "  \"gross_harvest_g\": {},\n",
            "  \"rejected_g\": {},\n",
            "  \"spoiled_g\": {},\n",
            "  \"inedible_residue_g\": {},\n",
            "  \"makeup_water_ml\": {},\n",
            "  \"useful_heat_wh_th\": {},\n",
            "  \"heat_rejected_wh_th\": {},\n",
            "  \"metrics\": {{\n",
            "    \"electricity_kwh_per_kg_accepted\": {},\n",
            "    \"electricity_kwh_per_m2_year\": {},\n",
            "    \"makeup_water_l_per_kg_accepted\": {},\n",
            "    \"accepted_fraction_of_gross_percent\": {}\n",
            "  }},\n",
            "  \"claim_ceiling\": \"facility electricity and harvest accounting only; no crop-yield prediction, nutrition adequacy, food safety, primary-energy or lifecycle claim\"\n",
            "}}"
        ),
        report.evidence_class,
        report.scenario_id,
        report.period_days,
        report.facility_area_m2,
        report.total_electricity_wh,
        report.electricity_wh_by_subsystem[0],
        report.electricity_wh_by_subsystem[1],
        report.electricity_wh_by_subsystem[2],
        report.electricity_wh_by_subsystem[3],
        report.electricity_wh_by_subsystem[4],
        report.electricity_wh_by_subsystem[5],
        report.electricity_wh_by_subsystem[6],
        report.accepted_edible_g,
        report.gross_harvest_g,
        report.rejected_g,
        report.spoiled_g,
        report.inedible_residue_g,
        report.makeup_water_ml,
        report.useful_heat_wh_th,
        report.heat_rejected_wh_th,
        kwh_per_kg_accepted,
        kwh_per_m2_year,
        water_l_per_kg_accepted,
        accepted_percent,
    ))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let input_path = match args.as_slice() {
        [] => "fixtures/synthetic-lettuce-facility-v0-1.input".to_owned(),
        [flag, path] if flag == "--input" => path.clone(),
        _ => return Err("usage: cea-facility-bench [--input PATH]".to_owned()),
    };

    let source = fs::read_to_string(&input_path)
        .map_err(|error| format!("could not read {input_path}: {error}"))?;
    let input = parse_input(&source)?;
    let report = evaluate(&input)?;
    println!("{}", render_json(&report)?);
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

    const FIXTURE: &str = include_str!("../fixtures/synthetic-lettuce-facility-v0-1.input");

    #[test]
    fn synthetic_fixture_closes_mass_and_matches_hand_calculated_intensities() {
        let input = parse_input(FIXTURE).expect("valid synthetic fixture");
        let report = evaluate(&input).expect("valid facility report");
        assert_eq!(report.total_electricity_wh, 1_750_000);
        assert_eq!(report.accepted_edible_g, 120_000);

        let rendered = render_json(&report).expect("render report");
        assert!(rendered.contains("\"electricity_kwh_per_kg_accepted\": 14.583333"));
        assert!(rendered.contains("\"electricity_kwh_per_m2_year\": 875.000000"));
        assert!(rendered.contains("\"makeup_water_l_per_kg_accepted\": 10.000000"));
        assert!(rendered.contains("\"accepted_fraction_of_gross_percent\": 66.666667"));
    }

    #[test]
    fn rejects_harvest_mass_imbalance() {
        let bad = FIXTURE.replace("spoiled_g=5000", "spoiled_g=4000");
        let input = parse_input(&bad).expect("syntactically valid fixture");
        assert!(
            evaluate(&input)
                .unwrap_err()
                .contains("harvest mass imbalance")
        );
    }

    #[test]
    fn rejects_unknown_and_duplicate_keys() {
        let unknown = format!("{FIXTURE}generic_crop_yield=999\n");
        assert!(parse_input(&unknown).unwrap_err().contains("unknown key"));

        let duplicate = format!("{FIXTURE}period_days=365\n");
        assert!(
            parse_input(&duplicate)
                .unwrap_err()
                .contains("duplicate key")
        );
    }

    #[test]
    fn rejects_measured_claims_in_the_synthetic_reference_runner() {
        let measured = FIXTURE.replace("evidence_class=synthetic", "evidence_class=measured");
        assert!(
            parse_input(&measured)
                .unwrap_err()
                .contains("accepts synthetic evidence only")
        );
    }

    #[test]
    fn rejects_zero_period_or_area() {
        let zero_period = FIXTURE.replace("period_days=365", "period_days=0");
        let input = parse_input(&zero_period).expect("syntactically valid fixture");
        assert!(evaluate(&input).unwrap_err().contains("period_days"));

        let zero_area = FIXTURE.replace("facility_area_m2=2", "facility_area_m2=0");
        let input = parse_input(&zero_area).expect("syntactically valid fixture");
        assert!(evaluate(&input).unwrap_err().contains("facility_area_m2"));
    }

    #[test]
    fn keeps_thermal_energy_separate_from_electrical_energy() {
        let input = parse_input(FIXTURE).expect("valid synthetic fixture");
        let report = evaluate(&input).expect("valid facility report");
        assert_eq!(report.total_electricity_wh, 1_750_000);
        assert_eq!(report.useful_heat_wh_th, 100_000);
        assert_eq!(report.heat_rejected_wh_th, 700_000);
    }
}
