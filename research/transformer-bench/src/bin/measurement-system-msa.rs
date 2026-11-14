//! Balanced crossed ANOVA Gage R&R diagnostic for continuous measurement systems.
//! It provides a study-specific noise estimate; it does not authorize measurement,
//! product release, or standards conformity.
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const OBSERVATION_HEADER: [&str; 9] = [
    "study_id", "part_id", "operator_id", "repeat_no", "measured_utc",
    "observed_value", "unit", "instrument_id", "calibration_id",
];
const SPEC_HEADER: [&str; 9] = [
    "study_id", "characteristic", "unit", "lower_limit", "upper_limit",
    "calibration_class", "min_parts", "min_operators", "min_repeats",
];
const CALIBRATION_HEADER: [&str; 8] = [
    "calibration_id", "instrument_id", "calibration_class", "valid_from_utc",
    "valid_to_utc", "status", "certificate_artifact_id", "certificate_sha256",
];

#[derive(Clone, Debug)]
struct Observation {
    study: String,
    part: String,
    operator: String,
    repeat_no: usize,
    measured_at: String,
    value: f64,
    unit: String,
    instrument: String,
    calibration_id: String,
}

#[derive(Clone, Debug)]
struct Spec {
    characteristic: String,
    unit: String,
    lower: f64,
    upper: f64,
    calibration_class: String,
    min_parts: usize,
    min_operators: usize,
    min_repeats: usize,
}

#[derive(Clone, Debug)]
struct Calibration {
    instrument: String,
    class: String,
    valid_from: String,
    valid_to: String,
    status: String,
}

#[derive(Clone, Debug, Default)]
struct VarianceComponents {
    repeatability: f64,
    interaction: f64,
    operator: f64,
    part: f64,
    grr: f64,
}

#[derive(Clone, Debug)]
struct Report {
    study: String,
    characteristic: String,
    unit: String,
    parts: usize,
    operators: usize,
    repeats: usize,
    observations: usize,
    invalid_metrology_records: usize,
    repeatability_sd: Option<f64>,
    interaction_sd: Option<f64>,
    operator_sd: Option<f64>,
    reproducibility_sd: Option<f64>,
    part_to_part_sd: Option<f64>,
    grr_sd: Option<f64>,
    study_variation_pct: Option<f64>,
    tolerance_pct: Option<f64>,
    ndc: Option<usize>,
    guidance_band: String,
    ndc_guidance: String,
    diagnostic: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("measurement-system-msa: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        return Err(format!(
            "usage: {} <observations.csv> <study-specs.csv> <calibrations.csv> <report.csv>",
            args.first().map(String::as_str).unwrap_or("measurement-system-msa")
        ));
    }
    let observations = read_observations(&args[1])?;
    let specs = read_specs(&args[2])?;
    let calibrations = read_calibrations(&args[3])?;
    let reports = evaluate(&observations, &specs, &calibrations)?;
    write_report(&args[4], &reports)?;
    for r in &reports {
        eprintln!(
            "study={} parts={} operators={} repeats={} grr_sd={} study_variation_pct={} tolerance_pct={} ndc={} guidance={} diagnostic={}",
            r.study, r.parts, r.operators, r.repeats, optional(r.grr_sd),
            optional(r.study_variation_pct), optional(r.tolerance_pct),
            r.ndc.map(|v| v.to_string()).unwrap_or_else(|| "NA".into()),
            r.guidance_band, r.diagnostic
        );
    }
    eprintln!("measurement-system MSA complete: studies={}; guidance bands are not automatic approval", reports.len());
    Ok(())
}

fn read_table(path: &str, expected: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing header"))?.trim_end_matches('\r');
    if header != expected.join(",") {
        return Err(format!("{path}: unexpected header; expected {}", expected.join(",")));
    }
    let mut rows = Vec::new();
    for (i, raw) in lines.enumerate() {
        let line_no = i + 2;
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            return Err(format!("{path}:{line_no}: blank row"));
        }
        if line.contains('"') {
            return Err(format!("{path}:{line_no}: quoted CSV fields are unsupported"));
        }
        let fields: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();
        if fields.len() != expected.len() {
            return Err(format!("{path}:{line_no}: expected {} fields, found {}", expected.len(), fields.len()));
        }
        rows.push(fields);
    }
    if rows.is_empty() {
        return Err(format!("{path}: no rows"));
    }
    Ok(rows)
}

fn required(value: &str, field: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err(format!("{field}: empty value"))
    } else {
        Ok(value.trim().to_string())
    }
}

fn number(value: &str, field: &str) -> Result<f64, String> {
    let v: f64 = value.parse().map_err(|_| format!("{field}: invalid number {value:?}"))?;
    if v.is_finite() { Ok(v) } else { Err(format!("{field}: non-finite value")) }
}

fn sha256_syntax(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn valid_utc(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T'
        || b[13] != b':' || b[16] != b':' || b[19] != b'Z' {
        return false;
    }
    if !b.iter().enumerate().all(|(i, byte)| {
        matches!(i, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
    }) {
        return false;
    }
    let parse = |range: std::ops::Range<usize>| {
        std::str::from_utf8(&b[range]).ok().and_then(|s| s.parse::<u32>().ok())
    };
    let Some(y) = parse(0..4) else { return false; };
    let Some(m) = parse(5..7) else { return false; };
    let Some(d) = parse(8..10) else { return false; };
    let Some(h) = parse(11..13) else { return false; };
    let Some(min) = parse(14..16) else { return false; };
    let Some(sec) = parse(17..19) else { return false; };
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1900..=2200).contains(&y) && (1..=12).contains(&m) && (1..=mdays).contains(&d)
        && h <= 23 && min <= 59 && sec <= 59
}

fn read_observations(path: &str) -> Result<Vec<Observation>, String> {
    let rows = read_table(path, &OBSERVATION_HEADER)?;
    let mut out = Vec::new();
    let mut keys = BTreeSet::new();
    for (i, f) in rows.iter().enumerate() {
        let row = i + 2;
        let study = required(&f[0], "study_id")?;
        let part = required(&f[1], "part_id")?;
        let operator = required(&f[2], "operator_id")?;
        let repeat_no: usize = f[3].parse().map_err(|_| format!("{path}:{row}: invalid repeat_no"))?;
        if repeat_no == 0 {
            return Err(format!("{path}:{row}: repeat_no must be positive"));
        }
        let key = (study.clone(), part.clone(), operator.clone(), repeat_no);
        if !keys.insert(key) {
            return Err(format!("{path}:{row}: duplicate part/operator/repeat observation"));
        }
        let measured_at = required(&f[4], "measured_utc")?;
        if !valid_utc(&measured_at) {
            return Err(format!("{path}:{row}: invalid canonical UTC timestamp"));
        }
        out.push(Observation {
            study, part, operator, repeat_no, measured_at,
            value: number(&f[5], "observed_value")?,
            unit: required(&f[6], "unit")?,
            instrument: required(&f[7], "instrument_id")?,
            calibration_id: required(&f[8], "calibration_id")?,
        });
    }
    Ok(out)
}

fn read_specs(path: &str) -> Result<BTreeMap<String, Spec>, String> {
    let rows = read_table(path, &SPEC_HEADER)?;
    let mut out = BTreeMap::new();
    for f in rows {
        let study = required(&f[0], "spec study_id")?;
        let lower = number(&f[3], "lower_limit")?;
        let upper = number(&f[4], "upper_limit")?;
        let min_parts: usize = f[6].parse().map_err(|_| format!("{path}: min_parts invalid"))?;
        let min_operators: usize = f[7].parse().map_err(|_| format!("{path}: min_operators invalid"))?;
        let min_repeats: usize = f[8].parse().map_err(|_| format!("{path}: min_repeats invalid"))?;
        if lower >= upper || min_parts < 2 || min_operators < 2 || min_repeats < 2 {
            return Err(format!("{path}: invalid limits or minimum balanced study dimensions"));
        }
        let spec = Spec {
            characteristic: required(&f[1], "characteristic")?,
            unit: required(&f[2], "unit")?,
            lower, upper,
            calibration_class: required(&f[5], "calibration_class")?,
            min_parts, min_operators, min_repeats,
        };
        if out.insert(study.clone(), spec).is_some() {
            return Err(format!("{path}: duplicate study spec {study:?}"));
        }
    }
    Ok(out)
}

fn read_calibrations(path: &str) -> Result<BTreeMap<String, Calibration>, String> {
    let rows = read_table(path, &CALIBRATION_HEADER)?;
    let mut out = BTreeMap::new();
    for f in rows {
        let id = required(&f[0], "calibration_id")?;
        let from = required(&f[3], "valid_from_utc")?;
        let to = required(&f[4], "valid_to_utc")?;
        if !valid_utc(&from) || !valid_utc(&to) || to <= from {
            return Err(format!("{path}: invalid calibration validity interval for {id:?}"));
        }
        if !matches!(f[5].as_str(), "CURRENT" | "EXPIRED" | "SUSPENDED" | "REVOKED") {
            return Err(format!("{path}: invalid calibration status {:?}", f[5]));
        }
        if !sha256_syntax(&f[7]) {
            return Err(format!("{path}: malformed certificate SHA-256 for {id:?}"));
        }
        let calibration = Calibration {
            instrument: required(&f[1], "instrument_id")?,
            class: required(&f[2], "calibration_class")?,
            valid_from: from, valid_to: to, status: f[5].clone(),
        };
        if out.insert(id.clone(), calibration).is_some() {
            return Err(format!("{path}: duplicate calibration_id {id:?}"));
        }
    }
    Ok(out)
}

fn calibration_valid(cal: &Calibration, observation: &Observation, spec: &Spec) -> bool {
    cal.status == "CURRENT"
        && cal.instrument == observation.instrument
        && cal.class == spec.calibration_class
        && observation.measured_at >= cal.valid_from
        && observation.measured_at <= cal.valid_to
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn analyze_crossed(cells: &BTreeMap<(String, String), Vec<f64>>) -> Result<VarianceComponents, String> {
    let parts: BTreeSet<&str> = cells.keys().map(|(part, _)| part.as_str()).collect();
    let operators: BTreeSet<&str> = cells.keys().map(|(_, operator)| operator.as_str()).collect();
    let p = parts.len();
    let o = operators.len();
    if p < 2 || o < 2 {
        return Err("crossed ANOVA requires at least two parts and two operators".into());
    }
    let repeats: BTreeSet<usize> = cells.values().map(Vec::len).collect();
    if repeats.len() != 1 {
        return Err("crossed ANOVA requires a balanced repeat count in every cell".into());
    }
    let r = *repeats.iter().next().expect("non-empty cell map");
    if r < 2 {
        return Err("crossed ANOVA requires at least two repeats".into());
    }
    if cells.len() != p * o {
        return Err("crossed ANOVA requires every part/operator cell".into());
    }

    let all: Vec<f64> = cells.values().flatten().copied().collect();
    let grand = mean(&all);
    let mut part_means = BTreeMap::new();
    for part in &parts {
        let vals: Vec<f64> = cells.iter().filter(|((p_id, _), _)| p_id == part)
            .flat_map(|(_, values)| values.iter().copied()).collect();
        part_means.insert(*part, mean(&vals));
    }
    let mut operator_means = BTreeMap::new();
    for operator in &operators {
        let vals: Vec<f64> = cells.iter().filter(|((_, op_id), _)| op_id == operator)
            .flat_map(|(_, values)| values.iter().copied()).collect();
        operator_means.insert(*operator, mean(&vals));
    }

    let mut ss_part = 0.0;
    for value in part_means.values() {
        ss_part += (o * r) as f64 * (value - grand).powi(2);
    }
    let mut ss_operator = 0.0;
    for value in operator_means.values() {
        ss_operator += (p * r) as f64 * (value - grand).powi(2);
    }
    let mut ss_interaction = 0.0;
    let mut ss_repeatability = 0.0;
    for ((part, operator), values) in cells {
        let cell_mean = mean(values);
        let interaction = cell_mean - part_means[part.as_str()] - operator_means[operator.as_str()] + grand;
        ss_interaction += r as f64 * interaction.powi(2);
        for value in values {
            ss_repeatability += (value - cell_mean).powi(2);
        }
    }

    let df_part = (p - 1) as f64;
    let df_operator = (o - 1) as f64;
    let df_interaction = ((p - 1) * (o - 1)) as f64;
    let df_repeatability = (p * o * (r - 1)) as f64;
    let ms_part = ss_part / df_part;
    let ms_operator = ss_operator / df_operator;
    let ms_interaction = ss_interaction / df_interaction;
    let ms_repeatability = ss_repeatability / df_repeatability;

    let repeatability = ms_repeatability.max(0.0);
    let interaction = ((ms_interaction - ms_repeatability) / r as f64).max(0.0);
    let operator = ((ms_operator - ms_interaction) / (p * r) as f64).max(0.0);
    let part = ((ms_part - ms_interaction) / (o * r) as f64).max(0.0);
    let grr = repeatability + interaction + operator;
    Ok(VarianceComponents {
        repeatability,
        interaction,
        operator,
        part,
        grr,
    })
}

fn guidance_band(study_variation_pct: Option<f64>) -> &'static str {
    match study_variation_pct {
        Some(value) if value.is_finite() && value < 10.0 => "GUIDANCE_LT_10_PERCENT",
        Some(value) if value.is_finite() && value <= 30.0 => "GUIDANCE_10_TO_30_PERCENT",
        Some(value) if value.is_finite() => "GUIDANCE_GT_30_PERCENT",
        _ => "NOT_ESTIMABLE",
    }
}

fn ndc_guidance(ndc: Option<usize>) -> &'static str {
    match ndc {
        Some(value) if value >= 5 => "NDC_AT_LEAST_5",
        Some(_) => "NDC_BELOW_5",
        None => "NDC_NOT_ESTIMABLE",
    }
}

fn evaluate(
    observations: &[Observation],
    specs: &BTreeMap<String, Spec>,
    calibrations: &BTreeMap<String, Calibration>,
) -> Result<Vec<Report>, String> {
    let mut groups: BTreeMap<String, Vec<&Observation>> = BTreeMap::new();
    for observation in observations {
        let spec = specs.get(&observation.study)
            .ok_or_else(|| format!("observation references unknown study {:?}", observation.study))?;
        if observation.unit != spec.unit {
            return Err(format!("study {:?}: observation unit differs from study specification", observation.study));
        }
        groups.entry(observation.study.clone()).or_default().push(observation);
    }

    let mut reports = Vec::new();
    for (study_id, group) in groups {
        let spec = specs.get(&study_id).expect("spec validated above");
        let parts: BTreeSet<String> = group.iter().map(|o| o.part.clone()).collect();
        let operators: BTreeSet<String> = group.iter().map(|o| o.operator.clone()).collect();
        let instruments: BTreeSet<String> = group.iter().map(|o| o.instrument.clone()).collect();
        if instruments.len() != 1 {
            return Err(format!("study {study_id:?}: crossed study must use one declared instrument"));
        }

        let mut cells: BTreeMap<(String, String), Vec<&Observation>> = BTreeMap::new();
        let mut invalid_metrology_records = 0usize;
        for observation in &group {
            let valid = calibrations.get(&observation.calibration_id)
                .is_some_and(|cal| calibration_valid(cal, observation, spec));
            if !valid {
                invalid_metrology_records += 1;
            }
            cells.entry((observation.part.clone(), observation.operator.clone()))
                .or_default().push(observation);
        }
        for values in cells.values_mut() {
            values.sort_by_key(|o| o.repeat_no);
            let repeat_numbers: Vec<usize> = values.iter().map(|o| o.repeat_no).collect();
            if repeat_numbers.iter().enumerate().any(|(i, value)| *value != i + 1) {
                return Err(format!("study {study_id:?}: repeat numbers must be contiguous from 1 in every cell"));
            }
        }
        if cells.len() != parts.len() * operators.len() {
            return Err(format!("study {study_id:?}: incomplete crossed part/operator grid"));
        }
        let repeats_per_cell: BTreeSet<usize> = cells.values().map(Vec::len).collect();
        if repeats_per_cell.len() != 1 {
            return Err(format!("study {study_id:?}: unbalanced number of repeats across cells"));
        }
        let repeats = *repeats_per_cell.iter().next().expect("non-empty observations");
        let diagnostic = if invalid_metrology_records > 0 {
            "BLOCKED_INVALID_METROLOGY"
        } else if parts.len() < spec.min_parts || operators.len() < spec.min_operators
            || repeats < spec.min_repeats {
            "INSUFFICIENT_STUDY_DESIGN"
        } else {
            "MEASUREMENT_SYSTEM_ANALYZED"
        };

        let components = if diagnostic == "MEASUREMENT_SYSTEM_ANALYZED" {
            let mut numeric_cells = BTreeMap::new();
            for (key, values) in &cells {
                numeric_cells.insert(key.clone(), values.iter().map(|o| o.value).collect());
            }
            Some(analyze_crossed(&numeric_cells)?)
        } else {
            None
        };

        let (repeatability_sd, interaction_sd, operator_sd, reproducibility_sd, part_sd,
            grr_sd, study_pct, tolerance_pct, ndc, guidance_band, ndc_guidance) =
            if let Some(v) = components {
                let repeatability_sd = v.repeatability.sqrt();
                let interaction_sd = v.interaction.sqrt();
                let operator_sd = v.operator.sqrt();
                let reproducibility_sd = (v.interaction + v.operator).sqrt();
                let part_sd = v.part.sqrt();
                let grr_sd = v.grr.sqrt();
                let total_sd = (v.grr + v.part).sqrt();
                let study_pct = if total_sd > 0.0 { Some(100.0 * grr_sd / total_sd) } else { None };
                let tolerance_pct = Some(100.0 * 6.0 * grr_sd / (spec.upper - spec.lower));
                let ndc = if grr_sd > 0.0 { Some((1.41 * part_sd / grr_sd).floor() as usize) } else { None };
                let band = guidance_band(study_pct);
                let ndc_band = ndc_guidance(ndc);
                (Some(repeatability_sd), Some(interaction_sd), Some(operator_sd),
                 Some(reproducibility_sd), Some(part_sd), Some(grr_sd), study_pct,
                 tolerance_pct, ndc, band.to_string(), ndc_band.to_string())
            } else {
                (None, None, None, None, None, None, None, None, None,
                 "NOT_ANALYZED".to_string(), "NOT_ANALYZED".to_string())
            };

        reports.push(Report {
            study: study_id,
            characteristic: spec.characteristic.clone(),
            unit: spec.unit.clone(),
            parts: parts.len(),
            operators: operators.len(),
            repeats,
            observations: group.len(),
            invalid_metrology_records,
            repeatability_sd,
            interaction_sd,
            operator_sd,
            reproducibility_sd,
            part_to_part_sd: part_sd,
            grr_sd,
            study_variation_pct: study_pct,
            tolerance_pct,
            ndc,
            guidance_band,
            ndc_guidance,
            diagnostic: diagnostic.to_string(),
        });
    }
    Ok(reports)
}

fn optional(value: Option<f64>) -> String {
    value.map(|v| format!("{v:.8}")).unwrap_or_else(|| "NA".into())
}

fn write_report(path: &str, reports: &[Report]) -> Result<(), String> {
    let mut out = String::from(
        "study_id,characteristic,unit,parts,operators,repeats,observations,invalid_metrology_records,repeatability_sd,interaction_sd,operator_sd,reproducibility_sd,part_to_part_sd,grr_measurement_noise_std_dev,study_variation_pct,tolerance_pct,ndc,guidance_band,ndc_guidance,diagnostic\n"
    );
    for r in reports {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.study, r.characteristic, r.unit, r.parts, r.operators, r.repeats,
            r.observations, r.invalid_metrology_records, optional(r.repeatability_sd),
            optional(r.interaction_sd), optional(r.operator_sd), optional(r.reproducibility_sd),
            optional(r.part_to_part_sd), optional(r.grr_sd), optional(r.study_variation_pct),
            optional(r.tolerance_pct), r.ndc.map(|v| v.to_string()).unwrap_or_else(|| "NA".into()),
            r.guidance_band, r.ndc_guidance, r.diagnostic
        ));
    }
    fs::write(path, out).map_err(|e| format!("cannot write {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells_from_rows(rows: Vec<(String, String, Vec<f64>)>) -> BTreeMap<(String, String), Vec<f64>> {
        rows.into_iter().map(|(part, operator, values)| ((part, operator), values)).collect()
    }

    #[test]
    fn crossed_anova_recovers_a_small_measurement_system_variation() {
        let mut rows = Vec::new();
        for part in 0..10 {
            for operator in 0..3 {
                let part_base = part as f64 * 0.1;
                let operator_bias = (operator as f64 - 1.0) * 0.001;
                rows.push((
                    format!("P{part}"),
                    format!("O{operator}"),
                    vec![100.0 + part_base + operator_bias - 0.001, 100.0 + part_base + operator_bias + 0.001],
                ));
            }
        }
        let comps = analyze_crossed(&cells_from_rows(rows)).unwrap();
        let grr = comps.grr.sqrt();
        let part = comps.part.sqrt();
        assert!(grr > 0.0);
        assert!(part > grr);
    }

    #[test]
    fn crossed_anova_separates_large_operator_reproducibility_shift() {
        let mut rows = Vec::new();
        for part in 0..10 {
            for operator in 0..3 {
                let part_base = part as f64 * 0.1;
                let operator_bias = (operator as f64 - 1.0) * 0.12;
                rows.push((
                    format!("P{part}"),
                    format!("O{operator}"),
                    vec![
                        100.0 + part_base + operator_bias - 0.002,
                        100.0 + part_base + operator_bias + 0.002,
                        100.0 + part_base + operator_bias,
                    ],
                ));
            }
        }
        let components = analyze_crossed(&cells_from_rows(rows)).unwrap();
        assert!(components.operator.sqrt() > 0.1);
        assert!(components.repeatability.sqrt() < 0.01);
        assert!(components.grr >= components.operator);
    }

    #[test]
    fn crossed_anova_rejects_unbalanced_and_missing_cells() {
        let unbalanced = cells_from_rows(vec![
            ("P1".into(), "O1".into(), vec![1.0, 1.1]),
            ("P1".into(), "O2".into(), vec![1.0]),
            ("P2".into(), "O1".into(), vec![2.0, 2.1]),
            ("P2".into(), "O2".into(), vec![2.0, 2.1]),
        ]);
        assert!(analyze_crossed(&unbalanced).is_err());
        let missing = cells_from_rows(vec![
            ("P1".into(), "O1".into(), vec![1.0, 1.1]),
            ("P2".into(), "O1".into(), vec![2.0, 2.1]),
            ("P2".into(), "O2".into(), vec![2.0, 2.1]),
        ]);
        assert!(analyze_crossed(&missing).is_err());
    }

    #[test]
    fn guidance_bands_and_ndc_are_reported_separately_without_release_authority() {
        assert_eq!(guidance_band(Some(9.99)), "GUIDANCE_LT_10_PERCENT");
        assert_eq!(guidance_band(Some(10.0)), "GUIDANCE_10_TO_30_PERCENT");
        assert_eq!(guidance_band(Some(30.0)), "GUIDANCE_10_TO_30_PERCENT");
        assert_eq!(guidance_band(Some(30.01)), "GUIDANCE_GT_30_PERCENT");
        assert_eq!(guidance_band(None), "NOT_ESTIMABLE");
        assert_eq!(ndc_guidance(Some(5)), "NDC_AT_LEAST_5");
        assert_eq!(ndc_guidance(Some(4)), "NDC_BELOW_5");
        assert_eq!(ndc_guidance(None), "NDC_NOT_ESTIMABLE");
    }

    #[test]
    fn strict_utc_parser_rejects_bad_dates_offsets_and_leap_day_errors() {
        assert!(valid_utc("2024-02-29T23:59:59Z"));
        assert!(!valid_utc("2025-02-29T23:59:59Z"));
        assert!(!valid_utc("2026-13-01T00:00:00Z"));
        assert!(!valid_utc("2026-01-01T00:00:00+02:00"));
        assert!(!valid_utc("2026-01-01T00:00:60Z"));
    }
}
