//! Offline integrity and comparability audit for the public EHD reference atlas.
//!
//! This checks source transcription structure and cohort boundaries. It does
//! not reproduce a plasma solver, validate physical measurements, or certify
//! that a source-reported result is physically correct.

use serde_json::Value;
use std::{env, fs, process};

const EMBEDDED_ATLAS: &str =
    include_str!("../../../data/research/ehd_reference_atlas_v0_1.json");

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |node, key| node.get(*key))
}

fn text_at(value: &Value, path: &[&str]) -> Option<&str> {
    at(value, path)?.as_str()
}

fn number_at(value: &Value, path: &[&str]) -> Option<f64> {
    at(value, path)?.as_f64()
}

fn len_at(value: &Value, path: &[&str]) -> Option<usize> {
    at(value, path)?.as_array().map(Vec::len)
}

fn contains_any(value: &Value, path: &[&str], needle: &str) -> bool {
    match at(value, path) {
        Some(Value::String(text)) => text.contains(needle),
        Some(Value::Array(items)) => items
            .iter()
            .any(|item| item.as_str().is_some_and(|text| text.contains(needle))),
        _ => false,
    }
}

fn push_if_missing(report: &mut Vec<String>, condition: bool, code: &str, detail: &str) {
    if !condition {
        report.push(format!("{code}: {detail}"));
    }
}

/// Return structural / comparability findings. An empty vector means that the
/// atlas satisfies this auditor's declared contract; it is not physical V&V.
pub fn audit_atlas(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();

    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_reference_atlas"),
        "ATLAS_SCHEMA",
        "unexpected or missing schema identifier",
    );
    push_if_missing(
        &mut findings,
        text_at(root, &["schema_version"]) == Some("0.1.0"),
        "ATLAS_VERSION",
        "unsupported or missing schema version",
    );

    for policy in [
        "Never merge atmospheric air corona",
        "Keep maximum values from different electrode configurations",
        "A source figure export or interpolating pickle is not a raw instrument time-series",
        "Unknown values are null, not zero",
        "Power boundary and system-mass boundary are required",
    ] {
        let found = at(root, &["comparability_policy"])
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().any(|v| v.as_str().is_some_and(|s| s.contains(policy))));
        push_if_missing(
            &mut findings,
            found,
            "COMPARABILITY_POLICY",
            &format!("missing required policy phrase: {policy}"),
        );
    }

    let mut ids = std::collections::BTreeSet::new();
    let records = at(root, &["source_records"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        records.is_some_and(|rows| !rows.is_empty()),
        "SOURCE_RECORDS",
        "source_records must be a non-empty array",
    );

    if let Some(records) = records {
        for (index, record) in records.iter().enumerate() {
            let id = text_at(record, &["source_id"]);
            push_if_missing(
                &mut findings,
                id.is_some_and(|id| ids.insert(id.to_owned())),
                "SOURCE_ID",
                &format!("record {index} has a missing or duplicate source_id"),
            );
            push_if_missing(
                &mut findings,
                text_at(record, &["cohort"]).is_some_and(|s| !s.trim().is_empty()),
                "SOURCE_COHORT",
                &format!("record {index} has no cohort"),
            );
        }
    }

    for (metric, unit) in [
        ("thrust_absolute", "N"),
        ("thrust_per_span", "N/m"),
        ("thrust_density", "N/m^2"),
        ("thrust_to_power", "N/kW"),
        ("thrust_to_weight", "1"),
        ("effective_thrust", "N or N/m"),
        ("real_ac_power", "W"),
        ("electrical_terminal_power", "W"),
        ("system_wall_power", "W"),
    ] {
        push_if_missing(
            &mut findings,
            text_at(root, &["metric_ontology", metric, "unit"]) == Some(unit),
            "METRIC_ONTOLOGY",
            &format!("missing or changed unit for {metric}; expected {unit}"),
        );
    }

    // Usuelli pressure/gap figure data must retain the source-recorded row
    // counts. The exported values are figure data, not instrument time series.
    let usuelli = records.and_then(|rows| rows.iter().find(|r| {
        text_at(r, &["source_id"]) == Some("USUELLI_2025_GRID_PRESSURE_DATA_V2")
    }));
    push_if_missing(
        &mut findings,
        usuelli.is_some(),
        "USUELLI_RECORD",
        "missing pressure/gap figure-data source",
    );
    if let Some(record) = usuelli {
        for (path, expected, name) in [
            (&["extracted_figure_data", "figure3_ignition_voltage", "rows"][..], 45, "Figure 3"),
            (&["extracted_figure_data", "figure4_breakdown_voltage", "rows"][..], 30, "Figure 4"),
            (&["extracted_figure_data", "figure5_ignition_to_breakdown_voltage_delta", "rows"][..], 29, "Figure 5"),
            (&["extracted_figure_data", "figure12_altitude_projection", "thrust_density_variation"][..], 17, "Figure 12 T/A"),
            (&["extracted_figure_data", "figure12_altitude_projection", "thrust_to_power_variation"][..], 17, "Figure 12 T/P"),
        ] {
            push_if_missing(
                &mut findings,
                len_at(record, path) == Some(expected),
                "USUELLI_ROW_COUNT",
                &format!("{name} expected {expected} source rows"),
            );
        }
        push_if_missing(
            &mut findings,
            text_at(record, &["evidence_class"]) == Some("figure_data_export_not_raw_acquisition"),
            "USUELLI_EVIDENCE_CLASS",
            "figure exports must not be promoted to raw instrument acquisition",
        );
        push_if_missing(
            &mut findings,
            contains_any(record, &["provenance", "limitations"], "Not raw instrument acquisition"),
            "USUELLI_RAW_DATA_BOUNDARY",
            "missing figure-data versus raw-acquisition limitation",
        );
        push_if_missing(
            &mut findings,
            contains_any(record, &["provenance", "limitations"], "Do not count repeated figure data as independent samples"),
            "USUELLI_DUPLICATION_BOUNDARY",
            "repeated figures must not be counted as independent observations",
        );
    }

    // The Terenzi paper's headline maxima belong to distinct configurations
    // and its published T/W mass basis is collector-only.
    let terenzi = records.and_then(|rows| rows.iter().find(|r| {
        text_at(r, &["source_id"]) == Some("TERENZI_2025_GRID_COLLECTOR")
    }));
    push_if_missing(
        &mut findings,
        terenzi.is_some(),
        "TERENZI_RECORD",
        "missing grid-collector source record",
    );
    if let Some(record) = terenzi {
        let ta_config = text_at(record, &["reported_configuration_specific_results", "maximum_thrust_density_n_m2", "configuration"]);
        let tp_config = text_at(record, &["reported_configuration_specific_results", "maximum_thrust_to_power_n_kw", "configuration"]);
        push_if_missing(
            &mut findings,
            ta_config == Some("III") && tp_config == Some("I") && ta_config != tp_config,
            "TERENZI_MIXED_MAXIMA",
            "maximum T/A (Config III) and maximum T/P (Config I) must remain separate",
        );
        push_if_missing(
            &mut findings,
            (number_at(record, &["reported_configuration_specific_results", "maximum_thrust_density_n_m2", "value"]) == Some(7.03))
                && (number_at(record, &["reported_configuration_specific_results", "maximum_thrust_to_power_n_kw", "value"]) == Some(6.52)),
            "TERENZI_SOURCE_VALUES",
            "source-reported headline values have changed; re-audit source transcription",
        );
        push_if_missing(
            &mut findings,
            text_at(record, &["mass_boundary", "included"]) == Some("collector mass only"),
            "TERENZI_MASS_BOUNDARY",
            "published T/W mass boundary must remain collector-only",
        );
        push_if_missing(
            &mut findings,
            contains_any(record, &["mass_boundary", "excluded"], "high-voltage converter")
                && contains_any(record, &["mass_boundary", "excluded"], "frame"),
            "TERENZI_MASS_EXCLUSIONS",
            "integrated-system mass exclusions must remain explicit",
        );
        push_if_missing(
            &mut findings,
            text_at(record, &["public_data", "data_kind"]) == Some("figure pickle assets, not disclosed raw instrument time-series"),
            "TERENZI_DATA_KIND",
            "figure assets must not be represented as raw acquisition traces",
        );
    }

    // The Mars/low-pressure cohort is per-span thrust, not total thrust, and
    // its power boundary is the real AC cycle-average.
    let liu = records.and_then(|rows| rows.iter().find(|r| {
        text_at(r, &["source_id"]) == Some("LIU_2026_MARS_CO2_DBD")
    }));
    push_if_missing(
        &mut findings,
        liu.is_some(),
        "LIU_RECORD",
        "missing low-pressure CO2 DBD source record",
    );
    if let Some(record) = liu {
        push_if_missing(
            &mut findings,
            number_at(record, &["reported_results", "at_600_pa_co2", "thrust_per_span_mn_m"]) == Some(5.498)
                && number_at(record, &["reported_results", "at_600_pa_air", "thrust_per_span_mn_m"]) == Some(1.451),
            "LIU_PER_SPAN_VALUES",
            "600 Pa CO2/air per-span values changed; re-audit source",
        );
        push_if_missing(
            &mut findings,
            text_at(record, &["measurement", "power"]).is_some_and(|s| s.contains("integral of u(t)*i(t)")),
            "LIU_REAL_POWER",
            "AC power must remain a cycle-average real-power integral",
        );
        push_if_missing(
            &mut findings,
            text_at(record, &["comparability", "not_vacuum_propellantless"]) == Some("true")
                || at(record, &["comparability", "not_vacuum_propellantless"]).and_then(Value::as_bool) == Some(true),
            "LIU_COHORT_BOUNDARY",
            "low-pressure CO2 DBD must not be treated as vacuum propellantless thrust",
        );
    }

    // Specialist solver scopes are intentionally narrow.
    let solvers = at(root, &["solver_profiles"]).and_then(Value::as_array);
    let merlino = solvers.and_then(|rows| rows.iter().find(|s| text_at(s, &["solver_id"]) == Some("MERLINO2D")));
    push_if_missing(
        &mut findings,
        merlino.is_some(),
        "MERLINO_PROFILE",
        "missing Merlino2D profile",
    );
    if let Some(profile) = merlino {
        push_if_missing(
            &mut findings,
            text_at(profile, &["license"]) == Some("GPL-3.0") && text_at(profile, &["language"]) == Some("MATLAB"),
            "MERLINO_DEPENDENCY_SCOPE",
            "Merlino2D MATLAB/GPL-3.0 dependency boundary changed",
        );
        push_if_missing(
            &mut findings,
            contains_any(profile, &["important_limits"], "does not by itself solve the full neutral-gas Navier–Stokes"),
            "MERLINO_FLOW_LIMIT",
            "Merlino2D must not be presented as a complete neutral-flow/thrust solver",
        );
        push_if_missing(
            &mut findings,
            text_at(profile, &["recommended_role"]).is_some_and(|s| s.contains("external discharge-physics engine")),
            "MERLINO_ROLE",
            "Merlino2D must remain an external discharge solver",
        );
    }

    let zapdos = solvers.and_then(|rows| rows.iter().find(|s| text_at(s, &["solver_id"]) == Some("ZAPDOS")));
    push_if_missing(
        &mut findings,
        zapdos.is_some(),
        "ZAPDOS_PROFILE",
        "missing Zapdos profile",
    );
    if let Some(profile) = zapdos {
        push_if_missing(
            &mut findings,
            text_at(profile, &["published_validation_envelope"]).is_some_and(|s| s.contains("0.1–1 Torr")),
            "ZAPDOS_VALIDATION_SCOPE",
            "Zapdos validation envelope must retain the published RF-CCP 0.1–1 Torr scope",
        );
        push_if_missing(
            &mut findings,
            contains_any(profile, &["important_limit"], "not automatic validation for atmospheric corona EHD"),
            "ZAPDOS_TRANSFER_LIMIT",
            "Zapdos CCP validation must not be silently transferred to atmospheric EHD",
        );
    }

    let rigid = solvers.and_then(|rows| rows.iter().find(|s| text_at(s, &["solver_id"]) == Some("SYMTROPY_RIGID_BODY")));
    push_if_missing(
        &mut findings,
        rigid.is_some_and(|s| text_at(s, &["current_claim_boundary"]).is_some_and(|v| v.contains("not a plasma"))),
        "SYMTROPY_CLAIM_BOUNDARY",
        "Symtropy rigid-body engine must not be represented as a plasma solver",
    );

    findings
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next();
    if args.next().is_some() {
        eprintln!("usage: ehd-reference-audit [path-to-atlas.json]");
        process::exit(2);
    }

    let (label, contents) = match path {
        Some(path) => match fs::read_to_string(&path) {
            Ok(contents) => (path, contents),
            Err(error) => {
                eprintln!("cannot read atlas: {error}");
                process::exit(2);
            }
        },
        None => ("embedded EHD reference atlas".to_owned(), EMBEDDED_ATLAS.to_owned()),
    };

    let atlas: Value = match serde_json::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid JSON: {error}");
            process::exit(2);
        }
    };

    let findings = audit_atlas(&atlas);
    if findings.is_empty() {
        let count = len_at(&atlas, &["source_records"]).unwrap_or(0);
        println!("PASS: {label}");
        println!("source records: {count}");
        println!("scope: source integrity and cohort comparability only; no physical V&V claim");
    } else {
        eprintln!("FAIL: {label} ({} finding(s))", findings.len());
        for finding in findings {
            eprintln!("- {finding}");
        }
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atlas() -> Value {
        serde_json::from_str(EMBEDDED_ATLAS).expect("embedded reference atlas must be valid JSON")
    }

    fn has_code(findings: &[String], code: &str) -> bool {
        findings.iter().any(|finding| finding.starts_with(code))
    }

    #[test]
    fn source_atlas_passes_declared_contract() {
        assert!(audit_atlas(&atlas()).is_empty());
    }

    #[test]
    fn duplicate_source_ids_are_rejected() {
        let mut value = atlas();
        let first = value["source_records"][0].clone();
        value["source_records"].as_array_mut().unwrap().push(first);
        assert!(has_code(&audit_atlas(&value), "SOURCE_ID"));
    }

    #[test]
    fn missing_metric_unit_is_rejected() {
        let mut value = atlas();
        value["metric_ontology"]["thrust_to_power"]["unit"] = Value::Null;
        assert!(has_code(&audit_atlas(&value), "METRIC_ONTOLOGY"));
    }

    #[test]
    fn mixing_grid_collector_maxima_is_rejected() {
        let mut value = atlas();
        value["source_records"][2]["reported_configuration_specific_results"]
            ["maximum_thrust_to_power_n_kw"]["configuration"] = Value::String("III".into());
        assert!(has_code(&audit_atlas(&value), "TERENZI_MIXED_MAXIMA"));
    }

    #[test]
    fn integrated_mass_boundary_cannot_be_silently_promoted() {
        let mut value = atlas();
        value["source_records"][2]["mass_boundary"]["included"] =
            Value::String("integrated aircraft mass".into());
        assert!(has_code(&audit_atlas(&value), "TERENZI_MASS_BOUNDARY"));
    }

    #[test]
    fn figure_data_cannot_be_relabelled_as_raw_acquisition() {
        let mut value = atlas();
        value["source_records"][1]["evidence_class"] =
            Value::String("raw_instrument_acquisition".into());
        assert!(has_code(&audit_atlas(&value), "USUELLI_EVIDENCE_CLASS"));
    }

    #[test]
    fn repeated_figure_points_cannot_be_dropped_without_detection() {
        let mut value = atlas();
        value["source_records"][1]["extracted_figure_data"]["figure3_ignition_voltage"]["rows"]
            .as_array_mut().unwrap().pop();
        assert!(has_code(&audit_atlas(&value), "USUELLI_ROW_COUNT"));
    }

    #[test]
    fn co2_dbd_power_boundary_is_required() {
        let mut value = atlas();
        value["source_records"][4]["measurement"]["power"] = Value::String("RMS volts times RMS amps".into());
        assert!(has_code(&audit_atlas(&value), "LIU_REAL_POWER"));
    }

    #[test]
    fn merlino_cannot_be_misrepresented_as_full_flow_solver() {
        let mut value = atlas();
        value["solver_profiles"][0]["important_limits"] =
            Value::Array(vec![Value::String("complete neutral-flow solver".into())]);
        assert!(has_code(&audit_atlas(&value), "MERLINO_FLOW_LIMIT"));
    }

    #[test]
    fn zapdos_ccp_validation_cannot_be_promoted_to_atmospheric_ehd() {
        let mut value = atlas();
        value["solver_profiles"][1]["published_validation_envelope"] =
            Value::String("atmospheric corona EHD".into());
        assert!(has_code(&audit_atlas(&value), "ZAPDOS_VALIDATION_SCOPE"));
    }

    #[test]
    fn rigid_body_engine_cannot_be_promoted_to_plasma_solver() {
        let mut value = atlas();
        value["solver_profiles"][2]["current_claim_boundary"] =
            Value::String("qualified plasma PDE solver".into());
        assert!(has_code(&audit_atlas(&value), "SYMTROPY_CLAIM_BOUNDARY"));
    }

    #[test]
    fn empty_source_record_set_is_rejected() {
        let mut value = atlas();
        value["source_records"] = Value::Array(Vec::new());
        assert!(has_code(&audit_atlas(&value), "SOURCE_RECORDS"));
    }
}
