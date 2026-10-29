//! Strict parsers for the byte-pinned NASA TMR NACA 0012 text datasets.
//!
//! Raw source files remain committed separately and immutable by Git revision. These parsers
//! preserve source zones and original line numbers; they do not normalize away differences in
//! Reynolds number, Mach number, transition treatment, or data quality.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ForceObservation {
    pub zone: String,
    pub alpha_deg: f64,
    pub cl: f64,
    pub cd: f64,
    pub source_line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PressureObservation {
    pub zone: String,
    pub x_over_c: f64,
    pub cp: f64,
    pub source_line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedReference<T> {
    pub dataset_id: &'static str,
    pub variables_header: String,
    pub observations: Vec<T>,
    pub zone_counts: BTreeMap<String, usize>,
}

/// Parse NASA TMR's CLCD_Ladson_expdata.dat.
///
/// Its header declares Ladson/NASA TM 4074 (1988), Re=6 million, M=0.15, and transition
/// tripped. The 80/120/180 grit zones remain separate because they are distinct source records.
pub fn parse_ladson_force_data(input: &str) -> Result<ParsedReference<ForceObservation>, String> {
    if !input.contains("Data from Ladson, NASA TM 4074, 1988")
        || !input.contains("Re=6 million, with transition tripped")
        || !input.contains("M=0.15")
    {
        return Err("force reference is missing the expected Ladson source/condition headers".to_owned());
    }
    parse_zone_data(
        input,
        "NASA-TMR-NACA0012-LADSON-FORCES",
        "alpha, deg",
        3,
        |zone, values, source_line| {
            if values[2] < 0.0 {
                return Err(format!(
                    "line {source_line}: drag coefficient must be non-negative"
                ));
            }
            Ok(ForceObservation {
                zone: zone.to_owned(),
                alpha_deg: values[0],
                cl: values[1],
                cd: values[2],
                source_line,
            })
        },
    )
}

/// Parse a NASA TMR CP_*.dat pressure-distribution file.
///
/// Zone names retain source alpha, Reynolds-number and transition text. This function validates
/// the two-column x/c, Cp structure but intentionally does not imply that pressure datasets
/// share Reynolds-number, Mach-number, or two-dimensional applicability.
pub fn parse_pressure_data(
    input: &str,
    dataset_id: &'static str,
) -> Result<ParsedReference<PressureObservation>, String> {
    if !input.contains("variables=\"x/c\",\"cp\"") {
        return Err("pressure reference must declare variables=\"x/c\",\"cp\"".to_owned());
    }
    parse_zone_data(input, dataset_id, "x/c", 2, |zone, values, source_line| {
        if !(0.0..=1.0).contains(&values[0]) {
            return Err(format!(
                "line {source_line}: x/c must be in [0, 1], got {}",
                values[0]
            ));
        }
        Ok(PressureObservation {
            zone: zone.to_owned(),
            x_over_c: values[0],
            cp: values[1],
            source_line,
        })
    })
}

fn parse_zone_data<T>(
    input: &str,
    dataset_id: &'static str,
    first_variable: &str,
    expected_columns: usize,
    mut make_observation: impl FnMut(&str, &[f64], usize) -> Result<T, String>,
) -> Result<ParsedReference<T>, String> {
    let mut variables_header = None;
    let mut zone: Option<String> = None;
    let mut observations = Vec::new();
    let mut zone_counts = BTreeMap::<String, usize>::new();

    for (index, raw_line) in input.lines().enumerate() {
        let source_line = index + 1;
        let line = raw_line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(header) = line.strip_prefix("variables=") {
            if variables_header.replace(header.to_owned()).is_some() {
                return Err(format!("line {source_line}: duplicate variables header"));
            }
            if !header.contains(first_variable) {
                return Err(format!(
                    "line {source_line}: variables header does not declare {first_variable}"
                ));
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("zone,") {
            let marker = "t=\"";
            let start = rest
                .find(marker)
                .ok_or_else(|| format!("line {source_line}: zone is missing a quoted t= label"))?
                + marker.len();
            let label_end = rest[start..]
                .find('"')
                .ok_or_else(|| format!("line {source_line}: zone label is not terminated"))?
                + start;
            if !rest[label_end + 1..].trim().is_empty() {
                return Err(format!("line {source_line}: unexpected text after zone label"));
            }
            let label = rest[start..label_end].to_owned();
            if label.trim().is_empty() {
                return Err(format!("line {source_line}: zone label is empty"));
            }
            if zone_counts.insert(label.clone(), 0).is_some() {
                return Err(format!("line {source_line}: duplicate zone label {label}"));
            }
            zone = Some(label);
            continue;
        }

        let active_zone = zone
            .as_deref()
            .ok_or_else(|| format!("line {source_line}: numeric observation before first zone"))?;
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != expected_columns {
            return Err(format!(
                "line {source_line}: expected exactly {expected_columns} numeric columns, got {}",
                tokens.len()
            ));
        }
        let values: Vec<f64> = tokens
            .iter()
            .map(|token| {
                token.parse::<f64>().map_err(|_| {
                    format!("line {source_line}: invalid numeric token {token:?}")
                })
            })
            .collect::<Result<_, _>>()?;
        if values.iter().any(|value| !value.is_finite()) {
            return Err(format!("line {source_line}: non-finite observation"));
        }
        let observation = make_observation(active_zone, &values, source_line)?;
        observations.push(observation);
        *zone_counts
            .get_mut(active_zone)
            .expect("zone counter created when zone parsed") += 1;
    }

    let variables_header = variables_header
        .ok_or_else(|| "reference file has no variables header".to_owned())?;
    if zone_counts.is_empty() {
        return Err("reference file has no data zones".to_owned());
    }
    if observations.is_empty() || zone_counts.values().any(|count| *count == 0) {
        return Err("every reference zone must contain at least one observation".to_owned());
    }

    Ok(ParsedReference {
        dataset_id,
        variables_header,
        observations,
        zone_counts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const REFERENCE_DIR: &str = "data/reference/nasa-tmr/";

    fn fixture(path: &str) -> String {
        fs::read_to_string(format!("{REFERENCE_DIR}{path}")).expect("pinned reference fixture")
    }

    #[test]
    fn parses_all_ladson_force_zones_without_merging_roughness_records() {
        let parsed = parse_ladson_force_data(&fixture("CLCD_Ladson_expdata.dat"))
            .expect("valid force data");
        assert_eq!(parsed.dataset_id, "NASA-TMR-NACA0012-LADSON-FORCES");
        assert_eq!(parsed.observations.len(), 53);
        assert_eq!(parsed.zone_counts.get("80 grit"), Some(&17));
        assert_eq!(parsed.zone_counts.get("120 grit"), Some(&18));
        assert_eq!(parsed.zone_counts.get("180 grit"), Some(&18));
        assert_eq!(parsed.observations[0].source_line, 6);
        assert!((parsed.observations[0].alpha_deg - (-4.04)).abs() < 1e-12);
        assert!((parsed.observations[0].cl - (-0.4417)).abs() < 1e-12);
        assert!((parsed.observations[0].cd - 0.00871).abs() < 1e-12);
    }

    #[test]
    fn parses_ladson_pressure_data_and_preserves_nine_condition_zones() {
        let parsed = parse_pressure_data(
            &fixture("CP_Ladson.dat"),
            "NASA-TMR-NACA0012-LADSON-PRESSURE",
        )
        .expect("valid pressure data");
        assert_eq!(parsed.observations.len(), 405);
        assert_eq!(parsed.zone_counts.len(), 9);
        assert_eq!(
            parsed.zone_counts.get("Re=6 million, alpha=.0169, free transition"),
            Some(&45)
        );
        assert_eq!(
            parsed.zone_counts.get("Re=3 million, alpha=14.9994, fixed transition"),
            Some(&45)
        );
        assert_eq!(
            parsed.zone_counts.get("Re=9 million, alpha=14.9911, fixed transition"),
            Some(&45)
        );
    }

    #[test]
    fn parses_gregory_pressure_but_keeps_each_alpha_zone_separate() {
        let parsed = parse_pressure_data(
            &fixture("CP_Gregory_expdata.dat"),
            "NASA-TMR-NACA0012-GREGORY-PRESSURE",
        )
        .expect("valid Gregory pressure data");
        assert_eq!(parsed.observations.len(), 73);
        assert_eq!(parsed.zone_counts.get("alpha=0"), Some(&25));
        assert_eq!(parsed.zone_counts.get("alpha=10"), Some(&24));
        assert_eq!(parsed.zone_counts.get("alpha=15"), Some(&24));
    }

    #[test]
    fn rejects_missing_condition_header_duplicate_header_malformed_rows_and_bad_x() {
        let forces = fixture("CLCD_Ladson_expdata.dat");
        assert!(parse_ladson_force_data(&forces.replace("M=0.15", "M=unknown")).is_err());
        let duplicate_header = forces.replace(
            "variables=\"alpha, deg\",\"cl\",\"cd\"",
            "variables=\"alpha, deg\",\"cl\",\"cd\"\nvariables=\"alpha, deg\",\"cl\",\"cd\"",
        );
        assert!(parse_ladson_force_data(&duplicate_header).is_err());

        let pressure = fixture("CP_Gregory_expdata.dat");
        let malformed = pressure.replace("0.0023497 0.847673", "0.0023497 not-a-number");
        assert!(parse_pressure_data(&malformed, "bad-pressure-fixture").is_err());
        let out_of_bounds = pressure.replace("0.0023497 0.847673", "1.0023497 0.847673");
        assert!(parse_pressure_data(&out_of_bounds, "out-of-chord-fixture").is_err());
        assert!(parse_pressure_data(
            "variables=\"x/c\",\"cp\"\nzone, t=\"empty\"\n",
            "empty-zone-fixture"
        )
        .is_err());
    }
}
