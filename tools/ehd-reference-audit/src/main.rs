//! Offline integrity and comparability audit for the public EHD reference atlas.
//!
//! This checks source transcription structure and cohort boundaries. It does
//! not reproduce a plasma solver, validate physical measurements, or certify
//! that a source-reported result is physically correct.

use serde_json::Value;
use std::{env, fs, process};

mod exchange;
mod geometry;
mod gcl;
mod exchange_receipt;
mod polynomial_moments;
mod drag_dissipation;
mod drag_trajectory;
mod control_volume;
mod identifiability;
mod discrepancy;

const EMBEDDED_ATLAS: &str =
    include_str!("../../../data/research/ehd_reference_atlas_v0_1.json");
const EMBEDDED_FRONTIER_PATTERNS: &str =
    include_str!("../../../data/research/ehd_frontier_design_patterns_v0_1.json");
const EMBEDDED_EXCHANGE_CONTRACT: &str =
    include_str!("../../../data/research/ehd_plasma_neutral_exchange_contract_v0_1.json");
const EMBEDDED_ARRAY_INTERACTION: &str =
    include_str!("../../../data/research/ehd_array_interaction_benchmark_v0_1.json");
const EMBEDDED_GCL_BENCHMARK: &str =
    include_str!("../../../data/research/ehd_moving_mesh_gcl_benchmark_v0_1.json");
const EMBEDDED_EXCHANGE_RECEIPT: &str =
    include_str!("../../../data/research/ehd_exchange_event_receipt_v0_1.json");
const EMBEDDED_POLYNOMIAL_MOMENTS: &str =
    include_str!("../../../data/research/ehd_polynomial_moments_v0_1.json");
const EMBEDDED_DRAG_DISSIPATION: &str =
    include_str!("../../../data/research/ehd_drag_dissipation_v0_1.json");
const EMBEDDED_DRAG_TRAJECTORY: &str =
    include_str!("../../../data/research/ehd_drag_trajectory_v0_1.json");
const EMBEDDED_THRUST_OBSERVABILITY: &str =
    include_str!("../../../data/research/ehd_thrust_observability_v0_1.json");
const EMBEDDED_CONTROL_VOLUME: &str =
    include_str!("../../../data/research/ehd_control_volume_v0_1.json");
const EMBEDDED_IDENTIFIABILITY: &str =
    include_str!("../../../data/research/ehd_experiment_identifiability_kernel_v0_1.json");
const EMBEDDED_DISCREPANCY_D0: &str =
    include_str!("../../../data/research/ehd_discrepancy_d0_corpus_v0_1.json");

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

fn find_record<'a>(records: Option<&'a Vec<Value>>, wanted: &str) -> Option<&'a Value> {
    records.and_then(|rows| rows.iter().find(|row| text_at(row, &["id"]) == Some(wanted)))
}

fn row_matches(row: &Value, expected: &[(&str, f64)]) -> bool {
    expected
        .iter()
        .all(|(field, value)| number_at(row, &[*field]) == Some(*value))
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
        let figure3 = at(record, &["extracted_figure_data", "figure3_ignition_voltage", "rows"])
            .and_then(Value::as_array);
        let figure4 = at(record, &["extracted_figure_data", "figure4_breakdown_voltage", "rows"])
            .and_then(Value::as_array);
        let figure5 = at(record, &["extracted_figure_data", "figure5_ignition_to_breakdown_voltage_delta", "rows"])
            .and_then(Value::as_array);
        let figure12_ta = at(record, &["extracted_figure_data", "figure12_altitude_projection", "thrust_density_variation"])
            .and_then(Value::as_array);
        let figure12_tp = at(record, &["extracted_figure_data", "figure12_altitude_projection", "thrust_to_power_variation"])
            .and_then(Value::as_array);

        let source_points_match = figure3.is_some_and(|rows| {
            rows.len() == 55
                && rows.first().is_some_and(|row| row_matches(row, &[("gap_mm", 20.0), ("pressure_pa", 100.0), ("ignition_voltage_kv", 7.29)]))
                && rows.get(44).is_some_and(|row| row_matches(row, &[("gap_mm", 60.0), ("pressure_pa", 20.0), ("ignition_voltage_kv", 5.12)]))
        }) && figure4.is_some_and(|rows| {
            rows.len() == 30
                && rows.first().is_some_and(|row| row_matches(row, &[("pressure_gap_pa_m", 400.0), ("breakdown_voltage_kv", 7.5395)]))
                && rows.get(29).is_some_and(|row| row_matches(row, &[("pressure_gap_pa_m", 3500.0), ("breakdown_voltage_kv", 24.7899)]))
        }) && figure5.is_some_and(|rows| {
            rows.len() == 29
                && rows.first().is_some_and(|row| row_matches(row, &[("gap_mm", 20.0), ("pressure_pa", 20000.0), ("delta_voltage_kv", 4.5195)]))
                && rows.get(28).is_some_and(|row| row_matches(row, &[("gap_mm", 50.0), ("pressure_pa", 70000.0), ("delta_voltage_kv", 16.6149)]))
        }) && figure12_ta.is_some_and(|rows| {
            rows.len() == 17
                && rows.first().is_some_and(|row| row_matches(row, &[("altitude_km", 0.11090105), ("percentage_variation", 0.0)]))
                && rows.get(16).is_some_and(|row| row_matches(row, &[("altitude_km", 11.7763701), ("percentage_variation", -80.33792326)]))
        }) && figure12_tp.is_some_and(|rows| {
            rows.len() == 17
                && rows.first().is_some_and(|row| row_matches(row, &[("altitude_km", 0.11090105), ("percentage_variation", 0.0)]))
                && rows.get(16).is_some_and(|row| row_matches(row, &[("altitude_km", 11.7763701), ("percentage_variation", -78.04621025)]))
        });
        push_if_missing(
            &mut findings,
            source_points_match,
            "USUELLI_SOURCE_POINTS",
            "source-pinned first/last points no longer match the Zenodo raw text assets",
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

    let integration_refs = at(root, &["system_integration_references"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        integration_refs.is_some_and(|rows| rows.iter().any(|r| text_at(r, &["source_id"]) == Some("TAO_2026_ION_MICROROBOT_CONTROLLED_HOVER"))),
        "TAO_SYSTEM_REFERENCE",
        "missing integrated ion-wind microrobot reference",
    );
    let tao = integration_refs.and_then(|rows| rows.iter().find(|r| {
        text_at(r, &["source_id"]) == Some("TAO_2026_ION_MICROROBOT_CONTROLLED_HOVER")
    }));
    if let Some(reference) = tao {
        push_if_missing(
            &mut findings,
            number_at(reference, &["platform", "reported_robot_mass_mg"]) == Some(36.7)
                && number_at(reference, &["performance", "reported_thrust_to_weight_ratio"]) == Some(5.0)
                && number_at(reference, &["performance", "reported_hover_duration_hours"]) == Some(1.0),
            "TAO_SOURCE_VALUES",
            "microrobot mass, T/W or tethered hover values changed",
        );
        push_if_missing(
            &mut findings,
            at(reference, &["operating_boundary", "hover_power_boundary"]).and_then(Value::as_str) == Some("tethered")
                && at(reference, &["operating_boundary", "self_powered_free_flight_demonstrated"]).and_then(Value::as_bool) == Some(false),
            "TAO_POWER_BOUNDARY",
            "tethered hover must not be promoted to self-powered free flight",
        );
        push_if_missing(
            &mut findings,
            at(reference, &["comparability", "leaderboard_eligible_against_bare_thrusters"]).and_then(Value::as_bool) == Some(false),
            "TAO_COMPARABILITY",
            "integrated microrobot control result must remain separate from bare-thruster leaderboard",
        );
    }

    let comsol = solvers.and_then(|rows| rows.iter().find(|s| text_at(s, &["solver_id"]) == Some("COMSOL_MULTIPHYSICS")));
    push_if_missing(
        &mut findings,
        comsol.is_some(),
        "COMSOL_PROFILE",
        "missing commercial COMSOL solver profile",
    );
    if let Some(profile) = comsol {
        push_if_missing(
            &mut findings,
            text_at(profile, &["license"]) == Some("commercial"),
            "COMSOL_LICENSE",
            "COMSOL must retain a commercial-license boundary",
        );
        push_if_missing(
            &mut findings,
            contains_any(profile, &["important_limits"], "omits the ionisation layer")
                && contains_any(profile, &["important_limits"], "electron dynamics are not solved"),
            "COMSOL_CORONA_SCOPE",
            "simplified Corona Discharge interface must retain its ionisation-layer / electron-dynamics limitations",
        );
        push_if_missing(
            &mut findings,
            contains_any(profile, &["capabilities"], "CFD-module electrohydrodynamics multiphysics capability"),
            "COMSOL_EHD_CAPABILITY",
            "commercial EHD flow-coupling capability must remain identified as a distinct profile",
        );
    }

    let reference_cases = at(root, &["solver_reference_cases"]).and_then(Value::as_array);
    let merlino_wire_cylinder = reference_cases.and_then(|rows| rows.iter().find(|case| {
        text_at(case, &["case_id"]) == Some("MERLINO2D_WIRE_CYLINDER_KIOUSIS_2014_IV")
    }));
    push_if_missing(
        &mut findings,
        merlino_wire_cylinder.is_some(),
        "MERLINO_REFERENCE_CASE",
        "missing source-pinned wire-cylinder I-V reference case",
    );
    if let Some(case) = merlino_wire_cylinder {
        push_if_missing(
            &mut findings,
            text_at(case, &["upstream", "commit"]) == Some("08f70dbeba5c9a8cccacbd21f5511896110581f8"),
            "MERLINO_UPSTREAM_PIN",
            "Merlino2D upstream source commit changed; requalify all pinned files",
        );
        push_if_missing(
            &mut findings,
            number_at(case, &["operating_conditions", "pressure_pa"]) == Some(101325.0)
                && number_at(case, &["operating_conditions", "temperature_k"]) == Some(300.0),
            "MERLINO_REFERENCE_CONDITIONS",
            "wire-cylinder baseline must retain its declared default pressure and temperature",
        );
        push_if_missing(
            &mut findings,
            number_at(case, &["model", "resistor_ohms"]) == Some(0.001)
                && text_at(case, &["model", "ode_solver"]) == Some("IDAS"),
            "MERLINO_REFERENCE_CIRCUIT",
            "external-circuit resistance or solver choice changed; rebuild exact baseline identity",
        );
        let csv_rows = at(case, &["experimental_comparator", "source_rows"])
            .and_then(Value::as_array);
        let csv_points_match = csv_rows.is_some_and(|rows| {
            rows.len() == 10
                && rows.first().is_some_and(|row| row_matches(row, &[
                    ("voltage_kv", 10.99520219328307),
                    ("current_ma", 0.04150943396226414),
                    ("current_ua", 41.50943396226414),
                ]))
                && rows.get(9).is_some_and(|row| row_matches(row, &[
                    ("voltage_kv", 20.021932830705964),
                    ("current_ma", 0.40452830188679245),
                    ("current_ua", 404.52830188679246),
                ]))
        });
        push_if_missing(
            &mut findings,
            csv_points_match,
            "MERLINO_REFERENCE_DATA",
            "repository-supplied ten-point I-V comparator changed or its current-unit conversion is inconsistent",
        );
        push_if_missing(
            &mut findings,
            text_at(case, &["claim_boundary"]).is_some_and(|s| {
                s.contains("discharge I-V curve")
                    && s.contains("no solver accuracy, thrust prediction")
            }),
            "MERLINO_REFERENCE_CLAIM",
            "wire-cylinder I-V case must not be promoted to thrust or neutral-flow validation",
        );
        push_if_missing(
            &mut findings,
            text_at(case, &["pinned_files", "experimental_csv", "git_blob_sha"])
                == Some("00d64d517ce18c1779477eb3a02ee8c55c4080d2"),
            "MERLINO_CSV_PIN",
            "experimental comparator CSV blob identity changed",
        );
        push_if_missing(
            &mut findings,
            text_at(case, &["experimental_comparator", "evidence_class"])
                == Some("repository_supplied_experimental_csv_linked_by_upstream_plot_script_to_Kiousis_2014"),
            "MERLINO_EVIDENCE_CLASS",
            "repository CSV must not be promoted to independently authenticated raw acquisition",
        );
        push_if_missing(
            &mut findings,
            number_at(case, &["geometry", "geo_distance_parameter_mm"]) == Some(30.0)
                && number_at(case, &["geometry", "derived_minimum_electrode_surface_clearance_mm"]) == Some(29.95),
            "MERLINO_GEOMETRY_CLEARANCE",
            "source geometry parameter and derived metal-surface clearance must remain distinct",
        );
        push_if_missing(
            &mut findings,
            text_at(case, &["experimental_comparator", "plot_conversion", "simulated_integrated_current_to_microamp"])
                .is_some_and(|s| s.contains("2e6") && s.contains("half-domain")),
            "MERLINO_CURRENT_SCALING",
            "half-domain model-current ×2 conversion must remain explicit",
        );
    }

    findings
}


/// Audit the separate 2026 frontier cohort without merging it into the older
/// source atlas. These checks preserve source-local claims and evidence limits.
pub fn audit_frontier_patterns(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();

    push_if_missing(
        &mut findings,
        text_at(root, &["artifact_id"]) == Some("EHD-FRONTIER-DESIGN-PATTERNS-2026")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["as_of"]) == Some("2026-11-21"),
        "FRONTIER_IDENTITY",
        "frontier artifact identity, schema version or as-of date changed",
    );

    let records = at(root, &["records"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        records.is_some_and(|rows| rows.len() == 10),
        "FRONTIER_RECORD_COUNT",
        "frontier cohort must retain ten separately qualified source records",
    );

    let mut ids = std::collections::BTreeSet::new();
    if let Some(rows) = records {
        for (index, record) in rows.iter().enumerate() {
            let id = text_at(record, &["id"]);
            push_if_missing(
                &mut findings,
                id.is_some_and(|value| ids.insert(value.to_owned())),
                "FRONTIER_SOURCE_ID",
                &format!("frontier record {index} has a missing or duplicate id"),
            );
            push_if_missing(
                &mut findings,
                text_at(record, &["doi"]).is_some_and(|value| !value.trim().is_empty())
                    && text_at(record, &["url"]).is_some_and(|value| value.starts_with("https://")),
                "FRONTIER_SOURCE_PROVENANCE",
                &format!("frontier record {index} lacks a DOI or HTTPS source URL"),
            );
        }
    }

    let jiang = find_record(records, "jiang-2026-sub-unit-electrode");
    push_if_missing(
        &mut findings,
        jiang.is_some(),
        "FRONTIER_JIANG_RECORD",
        "missing 2026 sub-unit electrode source",
    );
    if let Some(row) = jiang {
        let baseline = number_at(row, &["reported_values", "baseline_thrust_mn"]);
        let sub_unit = number_at(row, &["reported_values", "sub_unit_thrust_mn"]);
        let plus = number_at(row, &["reported_values", "sub_unit_plus_thrust_mn"]);
        let sub_gain = number_at(row, &["reported_values", "sub_unit_gain_pct_recomputed"]);
        let plus_increment = number_at(
            row,
            &["reported_values", "sub_unit_plus_increment_over_sub_unit_pct_recomputed"],
        );
        let plus_gain = number_at(
            row,
            &["reported_values", "sub_unit_plus_gain_over_baseline_pct_recomputed"],
        );
        let arithmetic_matches = baseline.is_some_and(|b| b > 0.0)
            && sub_unit.is_some_and(|s| s > 0.0)
            && plus.is_some_and(|p| p > 0.0)
            && sub_gain.is_some_and(|v| {
                (sub_unit.unwrap() / baseline.unwrap() - 1.0) * 100.0 - v < 0.02
                    && v - (sub_unit.unwrap() / baseline.unwrap() - 1.0) * 100.0 < 0.02
            })
            && plus_increment.is_some_and(|v| {
                (plus.unwrap() / sub_unit.unwrap() - 1.0) * 100.0 - v < 0.02
                    && v - (plus.unwrap() / sub_unit.unwrap() - 1.0) * 100.0 < 0.02
            })
            && plus_gain.is_some_and(|v| {
                (plus.unwrap() / baseline.unwrap() - 1.0) * 100.0 - v < 0.02
                    && v - (plus.unwrap() / baseline.unwrap() - 1.0) * 100.0 < 0.02
            });
        push_if_missing(
            &mut findings,
            arithmetic_matches
                && number_at(row, &["reported_operating_point", "voltage_kv"]) == Some(22.0),
            "FRONTIER_JIANG_ARITHMETIC",
            "sub-unit thrust points, 22 kV condition or within-paper percentage arithmetic changed",
        );
        push_if_missing(
            &mut findings,
            text_at(row, &["reported_tradeoff"]).is_some_and(|value| {
                value.contains("trade-off") && value.contains("thrust-to-power")
            }),
            "FRONTIER_JIANG_TP_TRADEOFF",
            "sub-unit absolute thrust versus thrust-to-power trade-off must remain explicit",
        );
    }

    let nonaka = find_record(records, "nonaka-2026-guide-electrode");
    push_if_missing(
        &mut findings,
        nonaka.is_some(),
        "FRONTIER_NONAKA_RECORD",
        "missing guide-electrode source",
    );
    if let Some(row) = nonaka {
        push_if_missing(
            &mut findings,
            number_at(row, &["reported_values", "maximum_thrust_improvement_pct"]) == Some(36.6)
                && at(row, &["reported_values", "absolute_thrust_mn"]).is_some_and(Value::is_null)
                && at(row, &["reported_values", "absolute_thrust_to_power"]).is_some_and(Value::is_null),
            "FRONTIER_NONAKA_SOURCE_LOCAL",
            "guide-electrode gain must remain source-local with unknown absolute thrust/T/P",
        );
    }

    let tao = find_record(records, "tao-2026-controlled-microrobot");
    push_if_missing(
        &mut findings,
        tao.is_some(),
        "FRONTIER_TAO_RECORD",
        "missing integrated microrobot comparator",
    );
    if let Some(row) = tao {
        push_if_missing(
            &mut findings,
            at(row, &["reported_values", "hover_is_tethered"]).and_then(Value::as_bool)
                == Some(true)
                && number_at(row, &["reported_values", "hover_duration_hours"]) == Some(1.0)
                && number_at(row, &["reported_values", "robot_mass_mg"]) == Some(36.7)
                && number_at(row, &["reported_values", "reported_thrust_to_weight"]) == Some(5.0)
                && text_at(row, &["system_boundary"]).is_some_and(|value| value.contains("tethered")),
            "FRONTIER_TAO_BOUNDARY",
            "microrobot one-hour hover must remain tethered and separate from onboard-energy flight",
        );
    }

    let shibata = find_record(records, "shibata-2026-multistage-ducted-optimization");
    push_if_missing(
        &mut findings,
        shibata.is_some(),
        "FRONTIER_SHIBATA_RECORD",
        "missing 2026 automated multistage ducted-thruster optimisation comparator",
    );
    if let Some(row) = shibata {
        push_if_missing(
            &mut findings,
            number_at(row, &["reported_values", "thrust_per_span_gain_vs_initial_design_approx_factor"]) == Some(6.0)
                && number_at(row, &["reported_values", "thrust_to_power_ratio_vs_initial_design_approx_factor"]) == Some(0.5)
                && text_at(row, &["reported_values", "metric_unit_n_per_m"]) == Some("N/m"),
            "FRONTIER_SHIBATA_METRICS",
            "Shibata approximate relative gains and per-span N/m unit must remain distinct",
        );
        push_if_missing(
            &mut findings,
            text_at(row, &["method", "operating_regime"]) == Some("stationary zero-freestream")
                && number_at(row, &["method", "latent_design_variables"]) == Some(4.0)
                && number_at(row, &["method", "population_per_generation"]) == Some(7.0)
                && at(row, &["thrust_estimation", "physical_force_measurement_performed"])
                    .and_then(Value::as_bool) == Some(false),
            "FRONTIER_SHIBATA_MODEL_SCOPE",
            "the 2D stationary optimiser comparator must not be promoted to physical or flight validation",
        );
        push_if_missing(
            &mut findings,
            at(row, &["thrust_estimation", "excluded"]).and_then(Value::as_array)
                .is_some_and(|items| items.iter().any(|value| value.as_str()
                    .is_some_and(|value| value.contains("friction drag on electrodes")))
                    && items.iter().any(|value| value.as_str()
                    .is_some_and(|value| value.contains("friction drag on duct")))),
            "FRONTIER_SHIBATA_DRAG_BOUNDARY",
            "omitted electrode and duct friction drag must remain explicit",
        );
    }

    let dong = find_record(records, "dong-2026-ion-mobility-regulation");
    push_if_missing(
        &mut findings,
        dong.is_some(),
        "FRONTIER_DONG_RECORD",
        "missing ion-mobility early-source watchlist entry",
    );
    if let Some(row) = dong {
        let issue_date = text_at(row, &["issue_date"]);
        let as_of = text_at(root, &["as_of"]);
        push_if_missing(
            &mut findings,
            issue_date.zip(as_of).is_some_and(|(issue, cutoff)| issue > cutoff)
                && text_at(row, &["evidence_class"])
                    == Some("online_early_source_modelled_prediction_not_ranked")
                && at(row, &["reported_values", "prescribed_scaled_current_condition"])
                    .and_then(Value::as_bool)
                    == Some(true)
                && text_at(row, &["comparability"])
                    .is_some_and(|value| value.contains("watchlist only")),
            "FRONTIER_DONG_DATE_GUARD",
            "future issue-date mobility CFD result must remain an early, scaled-current, non-ranked watchlist item",
        );
    }

    let wakimoto = find_record(records, "wakimoto-2025-booster-electrode");
    push_if_missing(
        &mut findings,
        wakimoto.is_some(),
        "FRONTIER_WAKIMOTO_RECORD",
        "missing experimentally validated booster-electrode comparator",
    );
    if let Some(row) = wakimoto {
        push_if_missing(
            &mut findings,
            at(row, &["method", "experimental_validation"]).and_then(Value::as_bool) == Some(true)
                && number_at(row, &["reported_values", "thrust_density_gain_vs_no_booster_at_15_kv_approx_factor"]) == Some(4.46)
                && number_at(row, &["reported_values", "thrust_density_gain_vs_no_booster_at_20_kv_approx_factor"]) == Some(2.22)
                && number_at(row, &["reported_values", "thrust_density_gain_vs_no_booster_at_25_kv_approx_factor"]) == Some(1.84)
                && at(row, &["reported_values", "metric_unit"]).is_some_and(Value::is_null),
            "FRONTIER_WAKIMOTO_SOURCE_SCOPE",
            "booster-electrode voltage-specific gains and unknown metric unit must remain source-local",
        );
    }

    let anemone = find_record(records, "shibata-2025-anemone-3d");
    push_if_missing(
        &mut findings,
        anemone.is_some(),
        "FRONTIER_ANEMONE_RECORD",
        "missing 3D ANEMONE solver comparator",
    );
    if let Some(row) = anemone {
        push_if_missing(
            &mut findings,
            text_at(row, &["method", "language"]) == Some("C++")
                && text_at(row, &["method", "license"]) == Some("MIT")
                && at(row, &["method", "charged_species"]).and_then(Value::as_array)
                    .is_some_and(|species| species.len() == 3)
                && at(row, &["method", "claimed_outputs"]).and_then(Value::as_array)
                    .is_some_and(|outputs| outputs.len() == 2
                        && outputs.iter().any(|v| v.as_str() == Some("corona inception voltage"))
                        && outputs.iter().any(|v| v.as_str() == Some("energy conversion efficiency")))
                && text_at(row, &["validation_boundary"]).is_some_and(|value| {
                    value.contains("does not claim independent validation")
                        && value.contains("net thrust")
                }),
            "FRONTIER_ANEMONE_SCOPE",
            "ANEMONE must remain a 3D three-species C++/MIT solver with a limited output/validation boundary",
        );
    }

    let integral = find_record(records, "shibata-2025-integral-three-component-plasma");
    push_if_missing(
        &mut findings,
        integral.is_some(),
        "FRONTIER_INTEGRAL_MODEL_RECORD",
        "missing integral three-component plasma-fluid model comparator",
    );
    if let Some(row) = integral {
        push_if_missing(
            &mut findings,
            at(row, &["method", "experimental_comparison_geometries"]).and_then(Value::as_array)
                .is_some_and(|geometries| geometries.len() == 3
                    && geometries.iter().any(|v| v.as_str().is_some_and(|s| s.contains("coaxial cylinder")))
                    && geometries.iter().any(|v| v.as_str().is_some_and(|s| s.contains("point-to-plate")))
                    && geometries.iter().any(|v| v.as_str().is_some_and(|s| s.contains("single-stage thruster"))))
                && text_at(row, &["validation_boundary"]).is_some_and(|value| value.contains("full neutral-flow")),
            "FRONTIER_INTEGRAL_MODEL_SCOPE",
            "integral plasma-fluid validation geometries and neutral-flow claim boundary must remain explicit",
        );
    }

    let array = find_record(records, "kitamura-2023-multianode-force-vector");
    push_if_missing(
        &mut findings,
        array.is_some(),
        "FRONTIER_ARRAY_RECORD",
        "missing paired-anode force-vector interaction experiment",
    );
    if let Some(row) = array {
        push_if_missing(
            &mut findings,
            at(row, &["reported_values", "orthogonal_anodes_force_superposition"])
                .and_then(Value::as_bool) == Some(false)
                && number_at(row, &["reported_values", "orthogonal_anodes_resultant_direction_degrees_approx"]) == Some(45.0)
                && at(row, &["reported_values", "horizontal_anodes_force_superposition"])
                    .and_then(Value::as_bool) == Some(true)
                && text_at(row, &["comparability"]).is_some_and(|value| value.contains("not a universal")),
            "FRONTIER_ARRAY_NONADDITIVITY",
            "paired-anode superposition must remain geometry-dependent, not a universal array assumption",
        );
    }

    let switching = find_record(records, "yashita-2025-switched-corona-thrust-control");
    push_if_missing(
        &mut findings,
        switching.is_some(),
        "FRONTIER_SWITCHING_RECORD",
        "missing switched-corona thrust control comparator",
    );
    if let Some(row) = switching {
        push_if_missing(
            &mut findings,
            number_at(row, &["method", "experiments_reported"]) == Some(2.0)
                && text_at(row, &["method", "circuit_topology"]).is_some_and(|value| {
                    value.contains("series") && value.contains("parallel")
                })
                && text_at(row, &["method", "demonstrated_control"]).is_some_and(|value| {
                    value.contains("duty ratio")
                })
                && contains_any(row, &["missing_or_unverified"], "switching loss"),
            "FRONTIER_SWITCHING_SCOPE",
            "duty-ratio thrust control must retain its circuit topology, two-experiment evidence and unresolved switching-loss boundary",
        );
    }

    let factors = at(root, &["proposed_design_factors"]).and_then(Value::as_array);
    let required_factors = [
        "guide_electrode",
        "sub_unit_geometry",
        "auxiliary_electrode",
        "collector_topology",
        "drive_architecture",
        "multistage_duct_geometry",
        "booster_electrode",
        "array_orientation_and_spacing",
        "switching_duty_ratio",
    ];
    let factors_match = factors.is_some_and(|items| {
        items.len() == required_factors.len()
            && required_factors.iter().all(|wanted| {
                items.iter().any(|item| text_at(item, &["id"]) == Some(*wanted))
            })
    });
    push_if_missing(
        &mut findings,
        factors_match,
        "FRONTIER_DESIGN_FACTOR_SET",
        "topology-screen design factor set is incomplete or duplicated",
    );

    for policy in [
        "Do not sum or multiply source-local percentage improvements",
        "Keep bare thruster, integrated platform and tethered system boundaries separate",
        "A missing condition remains null",
    ] {
        push_if_missing(
            &mut findings,
            contains_any(root, &["comparability_rules"], policy),
            "FRONTIER_COMPARABILITY_POLICY",
            &format!("missing required frontier comparability rule: {policy}"),
        );
    }

    push_if_missing(
        &mut findings,
        text_at(root, &["claim_ceiling"]).is_some_and(|value| {
            value.contains("No Luminous simulation")
                && value.contains("physical measurement")
                && value.contains("SOTA improvement")
        }),
        "FRONTIER_CLAIM_CEILING",
        "frontier dataset must retain an explicit non-achievement claim ceiling",
    );

    findings
}


/// Audit the conservative plasma-neutral exchange interface contract. This
/// validates only the manifest specification, not a coupled solver result.
pub fn audit_exchange_contract(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();

    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_plasma_neutral_exchange")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["contract_id"]) == Some("EHD-PLASMA-NEUTRAL-EXCHANGE-001")
            && text_at(root, &["status"]) == Some("protocol_only_not_implemented_or_validated"),
        "EXCHANGE_IDENTITY",
        "exchange contract identity, version or protocol-only status changed",
    );

    let methods = at(root, &["force_transfer_methods"]).and_then(Value::as_array);
    let explicit = methods.and_then(|rows| rows.iter().find(|row| {
        text_at(row, &["method_id"]) == Some("explicit_species_collision_drag")
    }));
    let reduced = methods.and_then(|rows| rows.iter().find(|row| {
        text_at(row, &["method_id"]) == Some("reduced_coulomb_deposition")
    }));
    push_if_missing(
        &mut findings,
        methods.is_some_and(|rows| rows.len() == 2) && explicit.is_some() && reduced.is_some(),
        "EXCHANGE_METHOD_SET",
        "exactly two distinct alternative force-transfer methods must be declared",
    );
    if let Some(row) = explicit {
        push_if_missing(
            &mut findings,
            text_at(row, &["forbidden_combination"])
                .is_some_and(|value| value.contains("Do not also apply rho_c*E")),
            "EXCHANGE_EXPLICIT_DOUBLE_COUNT_GUARD",
            "explicit species drag must prohibit a second direct Coulomb neutral source",
        );
        push_if_missing(
            &mut findings,
            text_at(row, &["plasma_reaction"])
                .is_some_and(|value| value.contains("equal-and-opposite")),
            "EXCHANGE_EXPLICIT_REACTION",
            "explicit species drag must account for its equal-and-opposite plasma reaction",
        );
    }
    if let Some(row) = reduced {
        push_if_missing(
            &mut findings,
            text_at(row, &["forbidden_combination"])
                .is_some_and(|value| value.contains("Do not also apply explicit species drag")),
            "EXCHANGE_REDUCED_DOUBLE_COUNT_GUARD",
            "reduced Coulomb deposition must prohibit explicit drag for the same transfer",
        );
        push_if_missing(
            &mut findings,
            at(row, &["required_coefficients"]).and_then(Value::as_array)
                .is_some_and(|fields| fields.iter().any(|v| v.as_str() == Some("omitted_term_bounds"))
                    && fields.iter().any(|v| v.as_str() == Some("closure_validation_receipt"))),
            "EXCHANGE_REDUCED_QUALIFICATION",
            "reduced Coulomb closure must include omitted-term bounds and validation receipt",
        );
    }

    push_if_missing(
        &mut findings,
        at(root, &["exchange_mapping", "interpolation_only_is_sufficient"])
            .and_then(Value::as_bool) == Some(false)
            && at(root, &["exchange_mapping", "spatial_meshes_may_differ"])
                .and_then(Value::as_bool) == Some(true)
            && at(root, &["exchange_mapping", "temporal_grids_may_differ"])
                .and_then(Value::as_bool) == Some(true),
        "EXCHANGE_MAPPING_POLICY",
        "different meshes/time grids are permitted, but simple interpolation alone is not a conservative exchange contract",
    );
    let reference_kernel = at(root, &["exchange_mapping", "reference_kernel"]);
    push_if_missing(
        &mut findings,
        reference_kernel.is_some_and(|value| {
            text_at(value, &["implementation"])
                == Some("tools/ehd-reference-audit/src/exchange.rs")
                && text_at(value, &["quantity"])
                    .is_some_and(|text| text.contains("already-integrated vector impulse"))
                && text_at(value, &["source_reconstruction"])
                    .is_some_and(|text| text.contains("first_order"))
                && text_at(value, &["coverage_policy"])
                    .is_some_and(|text| text.contains("no implicit renormalization"))
                && at(value, &["limitations"]).and_then(Value::as_array)
                    .is_some_and(|rows| rows.iter().any(|v| v.as_str()
                        == Some("does_not_compute_geometric_intersections")))
        }),
        "EXCHANGE_REFERENCE_KERNEL",
        "reference impulse kernel must retain its first-order, no-renormalization and no-geometry-engine limits",
    );
    push_if_missing(
        &mut findings,
        reference_kernel.and_then(|value| at(value, &["geometry_backend"])).is_some_and(|value| {
            text_at(value, &["implementation"])
                == Some("tools/ehd-reference-audit/src/geometry.rs")
                && text_at(value, &["geometry"]) == Some("axis_aligned_space_time_boxes")
                && at(value, &["supported_dimensions"]).and_then(Value::as_array)
                    .is_some_and(|dims| dims.len() == 3
                        && dims[0].as_u64() == Some(2)
                        && dims[1].as_u64() == Some(3)
                        && dims[2].as_u64() == Some(4))
                && at(value, &["checks"]).and_then(Value::as_array)
                    .is_some_and(|checks| checks.iter().any(|v| v.as_str()
                        == Some("pairwise_disjoint_destination_intersections_within_source"))
                        && checks.iter().any(|v| v.as_str()
                        == Some("complete_source_measure_coverage")))
                && at(value, &["limitations"]).and_then(Value::as_array)
                    .is_some_and(|limits| limits.iter().any(|v| v.as_str()
                        == Some("axis_aligned_boxes_only"))
                        && limits.iter().any(|v| v.as_str()
                        == Some("not_general_unstructured_polyhedra")))
        }),
        "EXCHANGE_GEOMETRY_BACKEND",
        "axis-aligned space-time intersection backend must enforce pairwise overlap and full coverage and retain its limited geometry scope",
    );

    let interoperability = at(root, &["exchange_mapping", "industry_interoperability"]);
    push_if_missing(
        &mut findings,
        interoperability.is_some_and(|value| {
            text_at(value, &["reference"]) == Some("preCICE")
                && text_at(value, &["documentation"])
                    .is_some_and(|url| url.starts_with("https://precice.org/"))
                && text_at(value, &["rule"])
                    .is_some_and(|text| text.contains("extensive quantities")
                        && text.contains("intensive quantities"))
        }),
        "EXCHANGE_INDUSTRY_INTEROP",
        "preCICE conservative-vs-consistent mapping boundary must be retained as an interoperability reference",
    );

    let mapping_fields = at(root, &["exchange_mapping", "record"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        mapping_fields.is_some_and(|fields| {
            ["source_mesh_hash", "destination_mesh_hash", "operator_identity",
             "mapped_impulse_ns", "source_impulse_ns", "mapping_residual_ns"]
            .iter().all(|wanted| fields.iter().any(|field| field.as_str() == Some(*wanted)))
        }),
        "EXCHANGE_MAPPING_RECEIPT",
        "mesh mapping receipt lacks source/destination identity or integral residual",
    );

    let required_invariants = [
        "equal_opposite_momentum",
        "exclusive_force_closure",
        "global_charge_conservation",
        "space_time_mapping_conservation",
        "dissipative_collision_sign",
        "independent_thrust_observables",
        "force_boundary_completeness",
        "unknown_is_not_zero",
    ];
    let invariants = at(root, &["invariants"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        invariants.is_some_and(|rows| {
            rows.len() == required_invariants.len()
                && required_invariants.iter().all(|wanted| {
                    rows.iter().any(|row| text_at(row, &["id"]) == Some(*wanted))
                })
        }),
        "EXCHANGE_INVARIANTS",
        "the eight declared charge/momentum/energy and claim-boundary invariants must remain present",
    );

    let manifest = at(root, &["run_manifest_required_fields"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        manifest.is_some_and(|rows| {
            ["selected_method_id", "chemistry_and_transport_hashes", "plasma_mesh_identity",
             "neutral_mesh_identity", "plasma_time_grid_identity", "neutral_time_grid_identity",
             "space_time_transfer_operator", "charge_ledger", "energy_ledger",
             "uncertainty_budget", "qualification_disposition"]
            .iter().all(|wanted| rows.iter().any(|field| field.as_str() == Some(*wanted)))
        }),
        "EXCHANGE_RUN_MANIFEST",
        "required run manifest is missing solver, mesh/time, exchange, ledger or uncertainty identity",
    );

    for (ledger, required) in [
        ("charge_ledger", &["initial_total_charge_c", "final_total_charge_c",
            "integrated_boundary_current_c", "charge_closure_residual_c", "uncertainty_c"][..]),
        ("momentum_ledger", &["initial_charged_momentum_ns", "final_neutral_momentum_ns",
            "boundary_momentum_flux_ns", "interphase_exchange_impulse_ns",
            "closure_residual_ns", "uncertainty_ns"][..]),
        ("energy_ledger", &["terminal_electrical_input_j", "change_neutral_kinetic_energy_j",
            "joule_and_collisional_heating_j", "electrode_wall_heat_j",
            "closure_residual_j", "uncertainty_j"][..]),
    ] {
        let fields = at(root, &[ledger, "required_fields"]).and_then(Value::as_array);
        push_if_missing(
            &mut findings,
            fields.is_some_and(|rows| required.iter().all(|wanted| {
                rows.iter().any(|field| field.as_str() == Some(*wanted))
            })),
            "EXCHANGE_LEDGER_FIELDS",
            &format!("{ledger} is missing required accounting fields"),
        );
    }

    let verification = at(root, &["verification_matrix"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        verification.is_some_and(|rows| rows.len() == 10
            && rows.iter().any(|v| v.as_str() == Some("reject_double_counted_force_closure"))
            && rows.iter().any(|v| v.as_str() == Some("space_time_mapping_partition_invariance"))
            && rows.iter().any(|v| v.as_str() == Some("nonnegative_dissipative_collision_heating"))),
        "EXCHANGE_VERIFICATION_MATRIX",
        "ten analytic, conservation, mapping and negative-control tests must remain declared",
    );

    push_if_missing(
        &mut findings,
        at(root, &["acceptance_policy", "universal_numeric_tolerance"]).is_some_and(Value::is_null)
            && at(root, &["acceptance_policy", "case_specific_tolerance_required"])
                .and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance_policy", "thresholds_must_be_predeclared"])
                .and_then(Value::as_bool) == Some(true),
        "EXCHANGE_TOLERANCE_POLICY",
        "do not impose a universal residual threshold; require predeclared case-specific tolerances",
    );

    push_if_missing(
        &mut findings,
        text_at(root, &["claim_ceiling"]).is_some_and(|value| {
            value.contains("No plasma-neutral solver coupling")
                && value.contains("physical measurement")
                && value.contains("SOTA improvement")
        }),
        "EXCHANGE_CLAIM_CEILING",
        "contract must state that implementation, physical validation and SOTA improvement are not yet claimed",
    );

    findings
}


/// Audit the EHD array-interaction measurement protocol. It validates the
/// data contract and claim boundaries, not measured coupling or controller performance.
pub fn audit_array_interaction(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();

    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_array_interaction_benchmark")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["benchmark_id"]) == Some("EHD-ARRAY-INTERACTION-001")
            && text_at(root, &["status"]) == Some("measurement_protocol_only_no_luminous_array_measurements"),
        "ARRAY_BENCHMARK_IDENTITY",
        "array benchmark identity, version or protocol-only status changed",
    );

    push_if_missing(
        &mut findings,
        text_at(root, &["source_anchor", "source_id"])
            == Some("kitamura-2023-multianode-force-vector")
            && text_at(root, &["source_anchor", "doi"])
                == Some("10.1109/ISIE51358.2023.10228184")
            && text_at(root, &["source_anchor", "evidence_boundary"])
                .is_some_and(|value| value.contains("not a universal array law")),
        "ARRAY_SOURCE_ANCHOR",
        "paired-anode evidence and configuration-specific limitation must remain explicit",
    );

    let stages = at(root, &["experiment_hierarchy"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        stages.is_some_and(|rows| rows.len() == 5
            && rows.iter().any(|row| text_at(row, &["stage"]) == Some("isolated_cell_baseline"))
            && rows.iter().any(|row| text_at(row, &["stage"]) == Some("paired_cell_interaction"))
            && rows.iter().any(|row| text_at(row, &["stage"]) == Some("small_array_transfer"))
            && rows.iter().any(|row| text_at(row, &["stage"]) == Some("dynamic_drive_interaction"))
            && rows.iter().any(|row| text_at(row, &["stage"]) == Some("holdout_transfer"))),
        "ARRAY_EXPERIMENT_HIERARCHY",
        "five-stage isolated/pair/array/dynamic/holdout experiment ladder is required",
    );

    let measured = at(root, &["measured_configuration_fields"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        measured.is_some_and(|rows| {
            ["geometry_hash", "cell_positions_m", "orientation_angles_deg",
             "pairwise_spacing_m", "drive_waveform_hash_by_cell",
             "voltage_waveform_by_cell", "current_waveform_by_cell",
             "terminal_power_by_cell", "gas_composition", "freestream_velocity_m_s",
             "force_balance_calibration_id", "session_id", "run_order"]
            .iter().all(|wanted| rows.iter().any(|field| field.as_str() == Some(*wanted)))
        }),
        "ARRAY_MEASURED_CONFIGURATION_FIELDS",
        "array configuration manifest must bind geometry, orientation, spacing, per-cell electrical waveforms, environment, calibration and session identity",
    );

    let observations = at(root, &["observation_fields"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        observations.is_some_and(|rows| {
            ["measured_total_force_vector_n", "isolated_cell_force_vectors_n",
             "measured_pairwise_or_array_force_vector_n", "force_vector_covariance_n2",
             "terminal_electrical_energy_j"]
            .iter().all(|wanted| rows.iter().any(|field| field.as_str() == Some(*wanted)))
        }),
        "ARRAY_OBSERVATION_FIELDS",
        "vector force, isolated baselines, combined force, covariance and electrical energy are required",
    );

    let model = at(root, &["model_contract"]);
    push_if_missing(
        &mut findings,
        model.is_some_and(|value| {
            at(value, &["isolated_cell_additivity_is_default"]).and_then(Value::as_bool) == Some(false)
                && text_at(value, &["optional_local_linearization"])
                    .is_some_and(|v| v.contains("J(configuration, operating_point)"))
                && text_at(value, &["linearity_scope"]).is_some_and(|v| {
                    v.contains("local") && v.contains("must not be extrapolated")
                })
                && at(value, &["training_holdouts"]).and_then(Value::as_array)
                    .is_some_and(|rows| rows.len() >= 4
                        && rows.iter().any(|v| v.as_str() == Some("orientation_family"))
                        && rows.iter().any(|v| v.as_str() == Some("spacing_family"))
                        && rows.iter().any(|v| v.as_str() == Some("experimental_session")))
        }),
        "ARRAY_MODEL_SCOPE",
        "array model must reject default additivity and limit local Jacobians to a held-out qualified envelope",
    );

    let factors = at(root, &["design_factors"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        factors.is_some_and(|rows| rows.len() == 6
            && rows.iter().any(|r| text_at(r, &["id"]) == Some("anode_orientation"))
            && rows.iter().any(|r| text_at(r, &["id"]) == Some("inter_cell_spacing"))
            && rows.iter().any(|r| text_at(r, &["id"]) == Some("polarity"))
            && rows.iter().any(|r| text_at(r, &["id"]) == Some("voltage_and_duty"))
            && rows.iter().any(|r| text_at(r, &["id"]) == Some("freestream"))
            && rows.iter().any(|r| text_at(r, &["id"]) == Some("cell_count"))),
        "ARRAY_DESIGN_FACTORS",
        "orientation, spacing, polarity, drive, freestream and cell-count factors are required",
    );

    let gates = at(root, &["proposed_acceptance_gates"]);
    push_if_missing(
        &mut findings,
        gates.is_some_and(|value| {
            number_at(value, &["force_direction_error_degrees_max"]) == Some(5.0)
                && number_at(value, &["force_magnitude_relative_error_max"]) == Some(0.10)
                && at(value, &["all_targets_prospective"]).and_then(Value::as_bool) == Some(true)
                && at(value, &["holdout_requirements"]).and_then(Value::as_array)
                    .is_some_and(|rows| rows.len() == 3)
        }),
        "ARRAY_PROSPECTIVE_GATES",
        "array accuracy targets and all three independent holdout requirements must remain prospective",
    );

    push_if_missing(
        &mut findings,
        contains_any(root, &["exclusion_rules"], "Do not assume isolated-cell force vectors superpose")
            && contains_any(root, &["exclusion_rules"], "Do not use a static force map as a transient switching model")
            && text_at(root, &["claim_ceiling"]).is_some_and(|value| {
                value.contains("No Luminous multi-anode force map")
                    && value.contains("physical result")
            }),
        "ARRAY_CLAIM_BOUNDARY",
        "do not assume superposition, reuse static maps for transients, or claim Luminous physical results",
    );

    findings
}

/// Audit the moving-mesh GCL benchmark identity and its deliberately narrow
/// analytic-reference claim boundary. It does not certify a production ALE solver.
pub fn audit_gcl_benchmark(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_moving_mesh_gcl_benchmark")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["benchmark_id"]) == Some("EHD-MOVING-GCL-001")
            && text_at(root, &["status"])
                == Some("analytic_reference_oracle_authored_tests_rust_execution_pending"),
        "GCL_BENCHMARK_IDENTITY",
        "moving-mesh GCL benchmark identity/version/status changed",
    );
    push_if_missing(
        &mut findings,
        text_at(root, &["identity", "equation"])
            .is_some_and(|value| value.contains("V(t1)-V(t0)") && value.contains("w dot n"))
            && text_at(root, &["identity", "residual"])
                == Some("integrated_grid_flux_minus_cell_volume_change"),
        "GCL_BENCHMARK_IDENTITY_EQUATION",
        "GCL must compare integrated outward grid flux with the endpoint volume change",
    );
    let implementation = at(root, &["reference_implementation"]);
    push_if_missing(
        &mut findings,
        implementation.is_some_and(|value| {
            text_at(value, &["path"]) == Some("tools/ehd-reference-audit/src/gcl.rs")
                && text_at(value, &["geometry"]) == Some("single_axis_aligned_spatial_box")
                && text_at(value, &["motion"]) == Some("lower_and_upper_bounds_linear_in_normalized_time")
                && text_at(value, &["face_area_integral"])
                    .is_some_and(|method| method.contains("exact_polynomial"))
                && at(value, &["spatial_dimensions"]).and_then(Value::as_array)
                    .is_some_and(|dims| dims.len() == 3
                        && dims[0].as_u64() == Some(1)
                        && dims[1].as_u64() == Some(2)
                        && dims[2].as_u64() == Some(3))
                && at(value, &["reports"]).and_then(Value::as_array)
                    .is_some_and(|fields| fields.iter().any(|field| field.as_str() == Some("integrated_grid_flux"))
                        && fields.iter().any(|field| field.as_str() == Some("gcl_residual")))
        }),
        "GCL_REFERENCE_SCOPE",
        "GCL reference implementation must preserve exact polynomial integration and the 1D/2D/3D AABB scope",
    );
    let test_families = at(root, &["test_families"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        test_families.is_some_and(|rows| rows.len() == 11
            && rows.iter().any(|row| row.as_str() == Some("independent_temporal_subdivision_additivity"))
            && rows.iter().any(|row| row.as_str() == Some("uniform_scalar_ale_free_stream"))
            && rows.iter().any(|row| row.as_str() == Some("forward_reverse_flux_antisymmetry"))),
        "GCL_TEST_MATRIX",
        "GCL benchmark must retain all eleven authored test families",
    );
    push_if_missing(
        &mut findings,
        at(root, &["acceptance", "universal_numeric_tolerance"]).is_some_and(Value::is_null)
            && at(root, &["acceptance", "case_specific_absolute_plus_relative_tolerance_required"])
                .and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance", "geometric_residual_separate_from_impulse_charge_momentum_energy_ledgers"])
                .and_then(Value::as_bool) == Some(true),
        "GCL_TOLERANCE_AND_LEDGER_POLICY",
        "GCL tolerances must be case-specific and GCL residuals must remain separate from physical ledgers",
    );
    push_if_missing(
        &mut findings,
        at(root, &["limitations"]).and_then(Value::as_array)
            .is_some_and(|limits| limits.iter().any(|item| item.as_str() == Some("axis_aligned_cells_only"))
                && limits.iter().any(|item| item.as_str() == Some("no_general_polyhedra"))
                && limits.iter().any(|item| item.as_str() == Some("not_a_production_ale_solver")))
            && text_at(root, &["claim_ceiling"])
                .is_some_and(|claim| claim.contains("No arbitrary moving-mesh GCL compliance")
                    && claim.contains("No production ALE solver")),
        "GCL_CLAIM_BOUNDARY",
        "GCL artifact must not promote an analytic AABB oracle into general ALE solver qualification",
    );
    findings
}

/// Audit the event-level exchange receipt contract and ensure that its scope
/// does not claim durable exactly-once semantics.
pub fn audit_exchange_receipt_contract(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_exchange_event_receipt")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["benchmark_id"]) == Some("EHD-EXCHANGE-RECEIPT-001")
            && text_at(root, &["status"])
                == Some("in_memory_reference_validator_authored_tests_rust_execution_pending"),
        "EXCHANGE_RECEIPT_IDENTITY",
        "event receipt benchmark identity/version/status changed",
    );
    let implementation = at(root, &["reference_implementation"]);
    push_if_missing(
        &mut findings,
        implementation.is_some_and(|value| {
            text_at(value, &["path"]) == Some("tools/ehd-reference-audit/src/exchange_receipt.rs")
                && text_at(value, &["identity_scope"])
                    == Some("unique_event_id_within_one_in_memory_batch")
                && text_at(value, &["frame_policy"])
                    .is_some_and(|policy| policy.contains("implicit transforms rejected"))
                && text_at(value, &["path_policy"])
                    .is_some_and(|policy| policy.contains("one_force_transfer_method"))
                && at(value, &["compensated_summation"]).and_then(Value::as_bool) == Some(true)
        }),
        "EXCHANGE_RECEIPT_IMPLEMENTATION",
        "event receipt implementation path, batch scope, frame policy, path policy or compensated summation boundary changed",
    );
    let methods = at(root, &["methods"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        methods.is_some_and(|rows| rows.len() == 2
            && rows.iter().any(|row| row.as_str() == Some("explicit_species_collision_drag"))
            && rows.iter().any(|row| row.as_str() == Some("reduced_coulomb_deposition"))),
        "EXCHANGE_RECEIPT_METHODS",
        "both mutually exclusive physical transfer methods must remain represented",
    );
    let fields = at(root, &["event_fields"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        fields.is_some_and(|rows| {
            ["event_id", "transfer_path_id", "method", "charged_solver_id",
             "neutral_solver_id", "charged_frame_id", "neutral_frame_id",
             "charged_reaction_impulse_ns", "neutral_impulse_ns"]
                .iter().all(|wanted| rows.iter().any(|row| row.as_str() == Some(*wanted)))
        }),
        "EXCHANGE_RECEIPT_EVENT_FIELDS",
        "event identity, solver, coordinate frame or paired impulse fields are missing",
    );
    let invariants = at(root, &["required_invariants"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        invariants.is_some_and(|rows| rows.len() == 8
            && rows.iter().any(|row| row.as_str() == Some("equal_and_opposite_impulse_per_event"))
            && rows.iter().any(|row| row.as_str() == Some("one_transfer_method_per_physical_path"))
            && rows.iter().any(|row| row.as_str() == Some("matching_coordinate_frame_per_event"))
            && rows.iter().any(|row| row.as_str() == Some("stable_solver_and_frame_identity_per_transfer_path"))),
        "EXCHANGE_RECEIPT_INVARIANTS",
        "event-local balance, path method exclusivity and frame identity are mandatory",
    );
    let tests = at(root, &["tests"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        tests.is_some_and(|rows| rows.len() == 12
            && rows.iter().any(|row| row.as_str() == Some("reject_event_residuals_that_cancel_globally"))
            && rows.iter().any(|row| row.as_str() == Some("reject_duplicate_event_ids"))
            && rows.iter().any(|row| row.as_str() == Some("reject_solver_or_frame_identity_changes_within_path"))),
        "EXCHANGE_RECEIPT_TEST_MATRIX",
        "event receipt must retain all eleven authored test families",
    );
    push_if_missing(
        &mut findings,
        at(root, &["acceptance", "universal_numeric_tolerance"]).is_some_and(Value::is_null)
            && at(root, &["acceptance", "per_event_check_required"]).and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance", "aggregate_check_required"]).and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance", "durable_exactly_once_not_implemented"])
                .and_then(Value::as_bool) == Some(true),
        "EXCHANGE_RECEIPT_ACCEPTANCE",
        "per-event and aggregate checks require predeclared case tolerances; durable exactly-once must remain unimplemented",
    );
    push_if_missing(
        &mut findings,
        at(root, &["limitations"]).and_then(Value::as_array)
            .is_some_and(|rows| rows.iter().any(|row| row.as_str() == Some("in_memory_batch_only"))
                && rows.iter().any(|row| row.as_str() == Some("no_durable_idempotency_across_restarts"))
                && rows.iter().any(|row| row.as_str() == Some("no_atomic_paired_solver_side_commit")))
            && text_at(root, &["claim_ceiling"])
                .is_some_and(|claim| claim.contains("does not provide durable exactly-once semantics")),
        "EXCHANGE_RECEIPT_CLAIM_BOUNDARY",
        "in-memory event checks must not be represented as durable exactly-once transaction semantics",
    );
    findings
}

/// Audit the P0/P1/P2 polynomial-moment oracle's contract and explicit limits.
/// It must not be promoted to reconstruction, moving-mesh TPE, or a production remapper.
pub fn audit_polynomial_moments_contract(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_polynomial_moment_remap")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["benchmark_id"]) == Some("EHD-POLYNOMIAL-MOMENTS-001")
            && text_at(root, &["status"])
                == Some("analytic_p0_p1_p2_integral_oracle_authored_tests_rust_execution_pending"),
        "POLY_MOMENT_IDENTITY",
        "polynomial-moment benchmark identity/version/status changed",
    );
    let implementation = at(root, &["reference_implementation"]);
    push_if_missing(
        &mut findings,
        implementation.is_some_and(|value| {
            text_at(value, &["path"]) == Some("tools/ehd-reference-audit/src/polynomial_moments.rs")
                && text_at(value, &["geometry_backend"]) == Some("tools/ehd-reference-audit/src/geometry.rs")
                && text_at(value, &["supported_geometry"])
                    == Some("axis_aligned_boxes_in_compile_time_dimensions")
                && text_at(value, &["reconstruction"])
                    .is_some_and(|scope| scope.contains("caller_supplies_coefficients"))
                && at(value, &["checks"]).and_then(Value::as_array)
                    .is_some_and(|checks| checks.iter().any(|item| item.as_str() == Some("destination_integral_per_cell"))
                        && checks.iter().any(|item| item.as_str() == Some("global_destination_integral")))
        }),
        "POLY_MOMENT_IMPLEMENTATION_SCOPE",
        "polynomial-moment path, geometry scope, supplied-coefficient boundary or local/global checks changed",
    );
    push_if_missing(
        &mut findings,
        at(root, &["supported_polynomial_degrees"]).and_then(Value::as_array)
            .is_some_and(|degrees| degrees.len() == 3
                && degrees[0].as_u64() == Some(0)
                && degrees[1].as_u64() == Some(1)
                && degrees[2].as_u64() == Some(2)),
        "POLY_MOMENT_DEGREES",
        "P0/P1/P2 polynomial degree support must remain explicit",
    );
    let tests = at(root, &["test_families"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        tests.is_some_and(|rows| rows.len() == 10
            && rows.iter().any(|item| item.as_str() == Some("p1_linear_exact_destination_integrals"))
            && rows.iter().any(|item| item.as_str() == Some("p2_quadratic_exact_destination_integrals"))
            && rows.iter().any(|item| item.as_str() == Some("reject_requested_maximum_degree_above_p2_scope"))
            && rows.iter().any(|item| item.as_str() == Some("reject_geometry_gap_before_transfer"))),
        "POLY_MOMENT_TEST_MATRIX",
        "nine P0/P1/P2 analytic and adversarial tests must remain declared",
    );
    push_if_missing(
        &mut findings,
        at(root, &["acceptance", "universal_numeric_tolerance"]).is_some_and(Value::is_null)
            && at(root, &["acceptance", "per_destination_integral_checked"]).and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance", "global_integral_checked"]).and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance", "polynomial_exactness_not_equivalent_to_tpe"]).and_then(Value::as_bool) == Some(true),
        "POLY_MOMENT_ACCEPTANCE",
        "local and global integral checks plus case-specific tolerances and TPE distinction are mandatory",
    );
    let limitations = at(root, &["limitations"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        limitations.is_some_and(|rows| rows.iter().any(|item| item.as_str() == Some("caller_supplies_polynomial_coefficients"))
            && rows.iter().any(|item| item.as_str() == Some("no_cell_average_reconstruction"))
            && rows.iter().any(|item| item.as_str() == Some("no_tpe_1_or_tpe_2_implementation"))
            && rows.iter().any(|item| item.as_str() == Some("no_production_remapping_solver")))
            && text_at(root, &["claim_ceiling"])
                .is_some_and(|claim| claim.contains("does not establish production higher-order remapping")
                    && claim.contains("TPE(1/2)")),
        "POLY_MOMENT_CLAIM_BOUNDARY",
        "analytic moment integration must not be represented as a reconstruction or TPE solver",
    );
    findings
}

/// Audit the constant-drift ion-neutral drag/dissipation contract. This is
/// an analytic reference case, not a general collision or thermal model.
pub fn audit_drag_dissipation_contract(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_drag_dissipation")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["benchmark_id"]) == Some("EHD-DRAG-DISSIPATION-001")
            && text_at(root, &["status"])
                == Some("constant_drift_analytic_reference_authored_tests_rust_execution_pending"),
        "DRAG_DISSIPATION_IDENTITY",
        "drag/dissipation benchmark identity/version/status changed",
    );
    let closure = at(root, &["physical_closure"]);
    push_if_missing(
        &mut findings,
        closure.is_some_and(|value| {
            text_at(value, &["neutral_impulse"])
                == Some("I_n = Gamma * relative_velocity")
                && text_at(value, &["charged_reaction_impulse"])
                    == Some("I_c = -I_n")
                && text_at(value, &["integrated_frictional_heating"])
                    .is_some_and(|expr| expr.contains("Gamma*|relative_velocity|^2")
                        && expr.contains(">= 0"))
        }),
        "DRAG_DISSIPATION_PHYSICS",
        "drag impulse, equal-opposite reaction and non-negative frictional heating identity must remain explicit",
    );
    let implementation = at(root, &["reference_implementation"]);
    push_if_missing(
        &mut findings,
        implementation.is_some_and(|value| {
            text_at(value, &["path"]) == Some("tools/ehd-reference-audit/src/drag_dissipation.rs")
                && text_at(value, &["motion"])
                    == Some("constant_relative_velocity_over_event_space_time_slab")
                && text_at(value, &["summation"])
                    == Some("event_id_ordered_compensated_summation")
                && at(value, &["checks"]).and_then(Value::as_array)
                    .is_some_and(|checks| checks.iter().any(|item| item.as_str() == Some("impulse_work_is_nonnegative"))
                        && checks.iter().any(|item| item.as_str() == Some("impulse_work_matches_predicted_frictional_heating")))
        }),
        "DRAG_DISSIPATION_SCOPE",
        "reference implementation must retain constant-drift scope, deterministic summation and energy-sign checks",
    );
    let tests = at(root, &["test_families"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        tests.is_some_and(|rows| rows.len() == 10
            && rows.iter().any(|item| item.as_str() == Some("anti_drag_impulse_rejected"))
            && rows.iter().any(|item| item.as_str() == Some("negative_frictional_work_rejected_even_with_relaxed_impulse_tolerance"))
            && rows.iter().any(|item| item.as_str() == Some("equal_opposite_wrong_magnitude_rejected"))),
        "DRAG_DISSIPATION_TESTS",
        "nine analytic and adversarial drag/dissipation test families must remain declared",
    );
    push_if_missing(
        &mut findings,
        at(root, &["acceptance", "universal_numeric_tolerance"]).is_some_and(Value::is_null)
            && at(root, &["acceptance", "per_event_momentum_and_energy_checks_required"])
                .and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance", "relative_velocity_constant_within_event_required"])
                .and_then(Value::as_bool) == Some(true),
        "DRAG_DISSIPATION_ACCEPTANCE",
        "case-specific per-event momentum/energy tolerances and constant-drift scope are mandatory",
    );
    let limits = at(root, &["limitations"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        limits.is_some_and(|rows| rows.iter().any(|item| item.as_str() == Some("constant_relative_velocity_only"))
            && rows.iter().any(|item| item.as_str() == Some("no_thermal_partition_between_species"))
            && rows.iter().any(|item| item.as_str() == Some("no_full_fluid_or_plasma_equations")))
            && text_at(root, &["claim_ceiling"])
                .is_some_and(|claim| claim.contains("constant-drift analytic drag/dissipation oracle only")
                    && claim.contains("No general time-varying")),
        "DRAG_DISSIPATION_CLAIM_BOUNDARY",
        "constant-drift oracle must not be promoted to a general collision/thermal solver",
    );
    findings
}

/// Audit the time-varying drag quadrature receipt scope and claim boundary.
pub fn audit_drag_trajectory_contract(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_drag_trajectory_quadrature")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["benchmark_id"]) == Some("EHD-DRAG-TRAJECTORY-001")
            && text_at(root, &["status"])
                == Some("space_time_quadrature_receipt_validator_authored_tests_rust_execution_pending"),
        "DRAG_TRAJECTORY_IDENTITY",
        "time-varying drag quadrature benchmark identity/version/status changed",
    );
    let physics = at(root, &["physics"]);
    push_if_missing(
        &mut findings,
        physics.is_some_and(|value| {
            text_at(value, &["coefficient_density"])
                .is_some_and(|expr| expr.contains(">= 0"))
                && text_at(value, &["neutral_impulse"])
                    .is_some_and(|expr| expr.contains("alpha*relative_velocity"))
                && text_at(value, &["frictional_heating"])
                    .is_some_and(|expr| expr.contains("alpha*|relative_velocity|^2")
                        && expr.contains(">= 0"))
                && at(value, &["discrete_receipt"]).is_some_and(|receipt| {
                    text_at(receipt, &["neutral_impulse"])
                        .is_some_and(|expr| expr.contains("delta_Gamma_q*relative_velocity_q"))
                        && text_at(receipt, &["frictional_heating"])
                            .is_some_and(|expr| expr.contains("delta_Gamma_q*|relative_velocity_q|^2"))
                })
        }),
        "DRAG_TRAJECTORY_PHYSICS",
        "impulse and frictional heating must use the same non-negative space-time drag measure",
    );
    let implementation = at(root, &["reference_implementation"]);
    push_if_missing(
        &mut findings,
        implementation.is_some_and(|value| {
            text_at(value, &["path"]) == Some("tools/ehd-reference-audit/src/drag_trajectory.rs")
                && at(value, &["input_sample_fields"]).and_then(Value::as_array)
                    .is_some_and(|fields| ["sample_id","time_fraction","integrated_drag_weight_kg","relative_velocity_m_s"]
                        .iter().all(|wanted| fields.iter().any(|item| item.as_str() == Some(*wanted))))
                && at(value, &["event_fields"]).and_then(Value::as_array)
                    .is_some_and(|fields| fields.iter().any(|item| item.as_str() == Some("charged_frame_id"))
                        && fields.iter().any(|item| item.as_str() == Some("neutral_frame_id")))
                && at(value, &["checks"]).and_then(Value::as_array)
                    .is_some_and(|checks| checks.iter().any(|item| item.as_str() == Some("neutral_impulse_reconstructed_from_same_samples"))
                        && checks.iter().any(|item| item.as_str() == Some("reported_frictional_heating_matches_same_samples"))
                        && checks.iter().any(|item| item.as_str() == Some("deterministic_event_and_sample_ordering")))
                && at(value, &["receipt_fields"]).and_then(Value::as_array)
                    .is_some_and(|fields| fields.iter().any(|item| item.as_str() == Some("quadrature_rule_ids_by_event"))
                        && fields.iter().any(|item| item.as_str() == Some("coordinate_frame_ids_by_event")))
        }),
        "DRAG_TRAJECTORY_IMPLEMENTATION",
        "time-varying drag implementation must retain sample identity, shared momentum/energy quadrature and rule provenance",
    );
    let manufactured = at(root, &["manufactured_solution"]);
    push_if_missing(
        &mut findings,
        manufactured.is_some_and(|value| {
            number_at(value, &["duration_seconds"]) == Some(1.0)
                && number_at(value, &["drag_rate_kg_per_s"]) == Some(2.0)
                && number_at(value, &["expected_frictional_heating_j"])
                    .is_some_and(|heat| (heat - 59.0 / 6.0).abs() < 1.0e-12)
                && at(value, &["expected_neutral_impulse_ns"]).and_then(Value::as_array)
                    .is_some_and(|impulse| impulse.len() == 3
                        && impulse[0].as_f64() == Some(4.0)
                        && impulse[1].as_f64() == Some(-1.0)
                        && impulse[2].as_f64() == Some(1.0))
        }),
        "DRAG_TRAJECTORY_MANUFACTURED_CASE",
        "variable-drift Simpson manufactured solution and its analytic impulse/heating known answers changed",
    );
    let tests = at(root, &["test_families"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        tests.is_some_and(|rows| rows.len() == 13
            && rows.iter().any(|item| item.as_str() == Some("incorrect_impulse_rejected_even_when_heating_matches"))
            && rows.iter().any(|item| item.as_str() == Some("incorrect_heating_rejected_even_when_impulse_matches"))
            && rows.iter().any(|item| item.as_str() == Some("event_error_cancellation_rejected"))
            && rows.iter().any(|item| item.as_str() == Some("implicit_coordinate_frame_mismatch_rejected"))
            && rows.iter().any(|item| item.as_str() == Some("offline_reference_self_check"))),
        "DRAG_TRAJECTORY_TEST_MATRIX",
        "twelve variable-drift analytic, adversarial and reference tests must remain declared",
    );
    push_if_missing(
        &mut findings,
        at(root, &["acceptance", "universal_numeric_tolerance"]).is_some_and(Value::is_null)
            && at(root, &["acceptance", "same_space_time_quadrature_for_impulse_and_heating"])
                .and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance", "quadrature_accuracy_requires_independent_refinement"])
                .and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance", "frame_identity_match_required"])
                .and_then(Value::as_bool) == Some(true),
        "DRAG_TRAJECTORY_ACCEPTANCE",
        "case-specific tolerances and independent quadrature refinement are mandatory",
    );
    let limits = at(root, &["limitations"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        limits.is_some_and(|rows| rows.iter().any(|item| item.as_str() == Some("validates_supplied_quadrature_receipt_only"))
            && rows.iter().any(|item| item.as_str() == Some("elastic_drag_frictional_heating_only"))
            && rows.iter().any(|item| item.as_str() == Some("no_full_collisional_energy_closure")))
            && text_at(root, &["claim_ceiling"])
                .is_some_and(|claim| claim.contains("does not qualify quadrature accuracy")
                    && claim.contains("full multi-fluid collisional energy closure")),
        "DRAG_TRAJECTORY_CLAIM_BOUNDARY",
        "supplied quadrature receipt must not be promoted to quadrature accuracy or complete collisional-energy validation",
    );
    findings
}

pub fn audit_thrust_observability_contract(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_thrust_observability")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["benchmark_id"]) == Some("EHD-THRUST-OBSERVABILITY-001")
            && text_at(root, &["status"])
                == Some("measurement_protocol_only_no_luminous_measurement"),
        "THRUST_OBSERVABILITY_IDENTITY",
        "thrust observability protocol identity/version/status changed",
    );
    let observables = at(root, &["independent_observables"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        observables.is_some_and(|rows| rows.len() == 4
            && rows.iter().any(|row| text_at(row, &["id"]) == Some("direct_force_balance"))
            && rows.iter().any(|row| text_at(row, &["id"]) == Some("velocity_field"))
            && rows.iter().any(|row| text_at(row, &["id"]) == Some("control_volume_momentum"))
            && rows.iter().any(|row| text_at(row, &["id"]) == Some("solver_prediction"))),
        "THRUST_OBSERVABILITY_METHODS",
        "direct balance, field measurement, control volume and solver prediction must remain independent observables",
    );
    let cv = at(root, &["momentum_equation"]);
    push_if_missing(
        &mut findings,
        cv.is_some_and(|value| {
            at(value, &["one_exit_plane_rho_u2_sufficient_for_net_thrust"])
                .and_then(Value::as_bool) == Some(false)
                && text_at(value, &["inferred_fluid_on_solid_sign"])
                    == Some("negative_of_solid_to_fluid")
                && at(value, &["terms_must_be_measured_or_bounded"]).and_then(Value::as_array)
                    .is_some_and(|terms| terms.len() == 6
                        && terms.iter().any(|item| item.as_str() == Some("momentum_storage"))
                        && terms.iter().any(|item| item.as_str() == Some("pressure_traction"))
                        && terms.iter().any(|item| item.as_str() == Some("viscous_traction")))
        }),
        "THRUST_CONTROL_VOLUME_COMPLETENESS",
        "control-volume momentum, traction and boundary terms/sign convention must remain explicit",
    );
    let electrical = at(root, &["electrical_boundary"]);
    push_if_missing(
        &mut findings,
        electrical.is_some_and(|value| {
            text_at(value, &["real_power_definition"])
                .is_some_and(|expr| expr.contains("integral_v_terminal(t)*i_terminal(t)"))
                && at(value, &["synchronous_voltage_current_required"])
                    .and_then(Value::as_bool) == Some(true)
                && at(value, &["separately_report"]).and_then(Value::as_array)
                    .is_some_and(|fields| fields.iter().any(|item| item.as_str() == Some("electrode_terminal_power"))
                        && fields.iter().any(|item| item.as_str() == Some("switching_loss")))
        }),
        "THRUST_ELECTRICAL_BOUNDARY",
        "real terminal power and separate conversion/switching losses must be required",
    );
    push_if_missing(
        &mut findings,
        at(root, &["mass_boundary", "separate_bare_thruster_and_integrated_platform_cohorts"])
            .and_then(Value::as_bool) == Some(true)
            && at(root, &["mass_boundary", "tethered_or_externally_powered_not_self_powered_flight"])
                .and_then(Value::as_bool) == Some(true),
        "THRUST_MASS_BOUNDARY",
        "bare/integrated and tethered/self-powered system boundaries must remain separate",
    );
    let holdouts = at(root, &["holdouts"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        holdouts.is_some_and(|rows| rows.len() == 3
            && rows.iter().any(|item| item.as_str() == Some("entire_experimental_session"))
            && rows.iter().any(|item| item.as_str() == Some("geometry_family"))
            && rows.iter().any(|item| item.as_str() == Some("waveform_family")))
            && text_at(root, &["holdout_policy"]).is_some_and(|policy| {
                policy.contains("Do not split time samples") && policy.contains("before unblinding")
            }),
        "THRUST_HOLDOUT_POLICY",
        "session/geometry/waveform holdouts must be enforced before unblinding",
    );
    let gates = at(root, &["proposed_acceptance_gates"]);
    push_if_missing(
        &mut findings,
        gates.is_some_and(|value| {
            at(value, &["targets_prospective"]).and_then(Value::as_bool) == Some(true)
                && text_at(value, &["cross_observable_rule"])
                    .is_some_and(|rule| rule.contains("combined_expanded_uncertainty"))
        }),
        "THRUST_RECONCILIATION_POLICY",
        "independent measurement estimates must reconcile within combined uncertainty and targets must remain prospective",
    );
    let limits = at(root, &["limitations"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        limits.is_some_and(|rows| rows.iter().any(|item| item.as_str() == Some("protocol_only_no_measurement_data"))
            && rows.iter().any(|item| item.as_str() == Some("no_control_volume_integral_implementation"))
            && rows.iter().any(|item| item.as_str() == Some("no_sota_superiority_claim")))
            && text_at(root, &["claim_ceiling"])
                .is_some_and(|claim| claim.contains("No Luminous physical EHD thrust measurement")
                    && claim.contains("no cross-observable agreement")),
        "THRUST_OBSERVABILITY_CLAIM_BOUNDARY",
        "measurement protocol must not be represented as an executed thrust test or SOTA result",
    );
    findings
}


pub fn audit_control_volume_contract(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_control_volume_reducer")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["benchmark_id"]) == Some("EHD-CV-001")
            && text_at(root, &["status"])
                == Some("vector_bookkeeping_reducer_authored_tests_rust_execution_pending"),
        "CONTROL_VOLUME_IDENTITY",
        "control-volume reducer identity/version/status changed",
    );
    let equation = at(root, &["equation"]);
    push_if_missing(
        &mut findings,
        equation.is_some_and(|value| {
            text_at(value, &["form"])
                == Some("S + A = P + V + B + F_solid_to_fluid")
                && text_at(value, &["reconstructed_solid_on_fluid"])
                    == Some("S + A - P - V - B")
                && text_at(value, &["inferred_fluid_on_solid"])
                    == Some("negative_of_reconstructed_solid_on_fluid")
                && at(value, &["sign_convention_id_required"]).and_then(Value::as_bool) == Some(true)
        }),
        "CONTROL_VOLUME_SIGN_CONVENTION",
        "control-volume equation/sign convention must remain explicit and fixed",
    );
    let required_terms = at(root, &["required_vector_terms"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        required_terms.is_some_and(|rows| rows.len() == 6
            && ["momentum_storage","advective_momentum_flux_all_surfaces","pressure_traction_all_surfaces",
                "viscous_traction_all_surfaces","body_force","direct_force_balance_fluid_on_solid"]
                .iter().all(|wanted| rows.iter().any(|item| item.as_str() == Some(*wanted)))),
        "CONTROL_VOLUME_REQUIRED_TERMS",
        "all five fluid-side terms and independent balance measurement must be mandatory",
    );
    let uncertainty = at(root, &["uncertainty_policy"]);
    push_if_missing(
        &mut findings,
        uncertainty.is_some_and(|value| {
            text_at(value, &["cv_uncertainty"])
                == Some("U_storage + U_advective + U_pressure + U_viscous + U_body")
                && text_at(value, &["combined_uncertainty"])
                    == Some("U_cv + U_direct_balance")
                && text_at(value, &["method"])
                    .is_some_and(|method| method.contains("worst_case_triangle_inequality"))
                && at(value, &["rss_allowed_by_default"]).and_then(Value::as_bool) == Some(false)
                && at(value, &["covariance_aware_future_method_requires_validated_covariance_artifact"])
                    .and_then(Value::as_bool) == Some(true)
        }),
        "CONTROL_VOLUME_UNCERTAINTY_POLICY",
        "uncertainty must use conservative worst-case addition; RSS is not the default",
    );
    let implementation = at(root, &["reference_implementation"]);
    push_if_missing(
        &mut findings,
        implementation.is_some_and(|value| {
            text_at(value, &["path"]) == Some("tools/ehd-reference-audit/src/control_volume.rs")
                && text_at(value, &["missing_term_policy"]) == Some("reject_not_zero_fill")
                && text_at(value, &["duplicate_evidence_id_policy"]) == Some("reject")
                && at(value, &["outputs"]).and_then(Value::as_array)
                    .is_some_and(|fields| fields.iter().any(|item| item.as_str() == Some("combined_uncertainty_bound_n"))
                        && fields.iter().any(|item| item.as_str() == Some("overall_within_combined_bound")))
        }),
        "CONTROL_VOLUME_IMPLEMENTATION_SCOPE",
        "reducer must reject missing terms, reject duplicate evidence and emit uncertainty-aware reconciliation",
    );
    let tests = at(root, &["test_families"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        tests.is_some_and(|rows| rows.len() == 10
            && rows.iter().any(|item| item.as_str() == Some("reject_each_missing_required_term"))
            && rows.iter().any(|item| item.as_str() == Some("verify_worst_case_uncertainty_sum_not_rss"))
            && rows.iter().any(|item| item.as_str() == Some("flag_discrepancy_beyond_uncertainty"))
            && rows.iter().any(|item| item.as_str() == Some("reject_empty_run_and_control_volume_identities"))
            && rows.iter().any(|item| item.as_str() == Some("offline_reference_self_check"))),
        "CONTROL_VOLUME_TEST_MATRIX",
        "nine control-volume analytic/adversarial test families must remain declared",
    );
    push_if_missing(
        &mut findings,
        at(root, &["acceptance", "all_six_terms_required"]).and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance", "unknown_term_is_not_zero"]).and_then(Value::as_bool) == Some(true)
            && at(root, &["acceptance", "worst_case_sum_is_default"]).and_then(Value::as_bool) == Some(true),
        "CONTROL_VOLUME_ACCEPTANCE",
        "all six terms must be present and unknown terms must not become zero",
    );
    let limitations = at(root, &["limitations"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        limitations.is_some_and(|rows| rows.iter().any(|item| item.as_str() == Some("no_piv_ldv_image_processing"))
            && rows.iter().any(|item| item.as_str() == Some("no_velocity_field_surface_integration"))
            && rows.iter().any(|item| item.as_str() == Some("no_physical_thrust_measurement")))
            && text_at(root, &["claim_ceiling"])
                .is_some_and(|claim| claim.contains("not a PIV/LDV pipeline")
                    && claim.contains("physical thrust measurement")),
        "CONTROL_VOLUME_CLAIM_BOUNDARY",
        "vector reducer must not be promoted into field processing or physical thrust validation",
    );
    findings
}


/// Audit the two-parameter manufactured identifiability benchmark contract.
/// This is a contract check for the reference fixture, not a physical EHD model.
pub fn audit_identifiability_contract(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_experiment_identifiability")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["benchmark_id"]) == Some("EHD-OED-001")
            && text_at(root, &["status"])
                == Some("two_parameter_manufactured_reference_kernel_authored_tests_rust_execution_pending"),
        "IDENTIFIABILITY_IDENTITY",
        "identifiability kernel benchmark identity/version/status changed",
    );
    push_if_missing(
        &mut findings,
        at(root, &["parameter_contract", "parameter_order"]).and_then(Value::as_array)
            .is_some_and(|rows| rows.len() == 2
                && rows[0].as_str() == Some("a")
                && rows[1].as_str() == Some("b"))
            && at(root, &["parameter_contract", "correlated_covariance_supported"])
                .and_then(Value::as_bool) == Some(false),
        "IDENTIFIABILITY_PARAMETER_CONTRACT",
        "two-parameter order and diagonal-covariance limitation must remain explicit",
    );
    let policy = at(root, &["numerical_policy"]);
    push_if_missing(
        &mut findings,
        policy.is_some_and(|value| {
            text_at(value, &["singular_value_rank_rule"])
                == Some("singular_value_i > relative_tolerance * largest_singular_value")
                && number_at(value, &["relative_rank_tolerance"]) == Some(1.0e-12)
                && at(value, &["rank_tolerance_is_input_and_receipt_field"])
                    .and_then(Value::as_bool) == Some(true)
                && text_at(value, &["parameter_estimate_policy"])
                    == Some("emit_only_when_numerical_rank_equals_two")
        }),
        "IDENTIFIABILITY_RANK_POLICY",
        "rank threshold, input provenance and no-estimate-on-rank-deficiency policy changed",
    );
    let cases = at(root, &["manufactured_cases"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        cases.is_some_and(|rows| rows.len() == 3),
        "IDENTIFIABILITY_CASE_COUNT",
        "exactly one rank-deficient and one full-rank manufactured case are required",
    );
    let repeated = cases.and_then(|rows| rows.iter().find(|row| {
        text_at(row, &["case_id"]) == Some("E0-REPEATED-CONDITION-RANK-1")
    }));
    let balanced = cases.and_then(|rows| rows.iter().find(|row| {
        text_at(row, &["case_id"]) == Some("E0-BALANCED-CONDITION-RANK-2")
    }));
    push_if_missing(
        &mut findings,
        repeated.is_some(),
        "IDENTIFIABILITY_REPEATED_CASE",
        "missing repeated-condition rank-deficiency control",
    );
    if let Some(case) = repeated {
        let design = at(case, &["design_matrix"]).and_then(Value::as_array);
        let design_matches = design.is_some_and(|rows| rows.len() == 3 && rows.iter().all(|row| {
            row.as_array().is_some_and(|cols| cols.len() == 2
                && cols[0].as_f64() == Some(1.0)
                && cols[1].as_f64() == Some(1.0))
        }));
        push_if_missing(
            &mut findings,
            design_matches
                && at(case, &["expected_information_matrix"]).and_then(Value::as_array)
                    .is_some_and(|rows| rows.len() == 2
                        && rows[0].as_array().is_some_and(|cols| cols.len() == 2
                            && cols[0].as_f64() == Some(3.0) && cols[1].as_f64() == Some(3.0))
                        && rows[1].as_array().is_some_and(|cols| cols.len() == 2
                            && cols[0].as_f64() == Some(3.0) && cols[1].as_f64() == Some(3.0)))
                && at(case, &["expected_rank"]).and_then(Value::as_u64) == Some(1)
                && text_at(case, &["expected_disposition"]) == Some("not_identifiable")
                && at(case, &["expected_parameter_estimate"]).is_some_and(Value::is_null),
            "IDENTIFIABILITY_REPEATED_EXPECTATION",
            "repeated identical conditions must remain rank one and must not emit parameters",
        );
    }
    push_if_missing(
        &mut findings,
        balanced.is_some(),
        "IDENTIFIABILITY_BALANCED_CASE",
        "missing balanced full-rank positive control",
    );
    if let Some(case) = balanced {
        let design = at(case, &["design_matrix"]).and_then(Value::as_array);
        let design_matches = design.is_some_and(|rows| rows.len() == 3
            && rows[0].as_array().is_some_and(|cols| cols.len() == 2
                && cols[0].as_f64() == Some(-1.0) && cols[1].as_f64() == Some(1.0))
            && rows[1].as_array().is_some_and(|cols| cols.len() == 2
                && cols[0].as_f64() == Some(0.0) && cols[1].as_f64() == Some(1.0))
            && rows[2].as_array().is_some_and(|cols| cols.len() == 2
                && cols[0].as_f64() == Some(1.0) && cols[1].as_f64() == Some(1.0)));
        push_if_missing(
            &mut findings,
            design_matches
                && at(case, &["expected_rank"]).and_then(Value::as_u64) == Some(2)
                && text_at(case, &["expected_disposition"]) == Some("identifiable")
                && at(case, &["expected_parameter_estimate"]).and_then(Value::as_array)
                    .is_some_and(|rows| rows.len() == 2
                        && rows[0].as_f64() == Some(2.0)
                        && rows[1].as_f64() == Some(0.5)),
            "IDENTIFIABILITY_BALANCED_EXPECTATION",
            "balanced design must retain its full-rank expected parameter recovery",
        );
    }
    let near_collinear = cases.and_then(|rows| rows.iter().find(|row| {
        text_at(row, &["case_id"]) == Some("E0-NEAR-COLLINEAR-RANK-2")
    }));
    push_if_missing(
        &mut findings,
        near_collinear.is_some(),
        "IDENTIFIABILITY_NEAR_COLLINEAR_CASE",
        "missing near-collinear full-rank numerical-stability control",
    );
    if let Some(case) = near_collinear {
        push_if_missing(
            &mut findings,
            at(case, &["expected_rank"]).and_then(Value::as_u64) == Some(2)
                && text_at(case, &["expected_disposition"]) == Some("identifiable")
                && number_at(case, &["expected_condition_number_approx"])
                    .is_some_and(|value| (value - 2.0e9).abs() / 2.0e9 < 1.0e-6)
                && at(case, &["practical_warning_required"])
                    .and_then(Value::as_bool) == Some(true),
            "IDENTIFIABILITY_NEAR_COLLINEAR_EXPECTATION",
            "near-collinear fixture must remain numerically full-rank while exposing severe conditioning",
        );
    }
    push_if_missing(
        &mut findings,
        at(root, &["test_families"]).and_then(Value::as_array)
            .is_some_and(|rows| rows.len() == 16
                && rows.iter().any(|row| row.as_str() == Some("repeated_identical_conditions_do_not_repair_rank_deficiency"))
                && rows.iter().any(|row| row.as_str() == Some("independent_variance_weights_are_applied"))
                && rows.iter().any(|row| row.as_str() == Some("jacobi_spectrum_retains_a_small_but_resolvable_singular_direction"))
                && rows.iter().any(|row| row.as_str() == Some("d_and_e_optimal_designs_can_differ_at_the_same_budget"))
                && rows.iter().any(|row| row.as_str() == Some("design_tournament_rejects_search_space_over_limit"))
                && rows.iter().any(|row| row.as_str() == Some("offline_reference_self_check"))),
        "IDENTIFIABILITY_TEST_MATRIX",
        "identifiability benchmark must retain all fifteen rank, stability, tournament and self-check families",
    );
    let tournament = at(root, &["reference_implementation", "design_tournament"]);
    push_if_missing(
        &mut findings,
        tournament.is_some_and(|value| {
            text_at(value, &["algorithm"])
                == Some("deterministic_exhaustive_discrete_design_enumeration")
                && at(value, &["criteria"]).and_then(Value::as_array)
                    .is_some_and(|rows| rows.len() == 2
                        && rows[0].as_str() == Some("D_optimal_maximize_log_determinant_information")
                        && rows[1].as_str() == Some("E_optimal_maximize_minimum_information_eigenvalue"))
                && at(value, &["observation_budget"]).and_then(Value::as_u64) == Some(3)
                && at(value, &["evaluated_design_count"]).and_then(Value::as_u64) == Some(20)
                && at(value, &["max_observation_budget"]).and_then(Value::as_u64) == Some(32)
                && at(value, &["candidate_conditions"]).and_then(Value::as_array)
                    .is_some_and(|rows| rows.len() == 4
                        && rows[0].get("index").and_then(Value::as_u64) == Some(0)
                        && rows[0].get("sensitivity").and_then(Value::as_array)
                            .is_some_and(|cols| cols.len() == 2
                                && cols[0].as_f64() == Some(-3.0)
                                && cols[1].as_f64() == Some(-3.0))
                        && rows[1].get("index").and_then(Value::as_u64) == Some(1)
                        && rows[1].get("sensitivity").and_then(Value::as_array)
                            .is_some_and(|cols| cols.len() == 2
                                && cols[0].as_f64() == Some(-3.0)
                                && cols[1].as_f64() == Some(-2.0))
                        && rows[2].get("index").and_then(Value::as_u64) == Some(2)
                        && rows[2].get("sensitivity").and_then(Value::as_array)
                            .is_some_and(|cols| cols.len() == 2
                                && cols[0].as_f64() == Some(-3.0)
                                && cols[1].as_f64() == Some(-1.0))
                        && rows[3].get("index").and_then(Value::as_u64) == Some(3)
                        && rows[3].get("sensitivity").and_then(Value::as_array)
                            .is_some_and(|cols| cols.len() == 2
                                && cols[0].as_f64() == Some(-2.0)
                                && cols[1].as_f64() == Some(0.0))
                        && rows.iter().all(|row| row.get("variance").and_then(Value::as_f64) == Some(1.0)))
                && at(value, &["expected_d_optimal", "candidate_indices"]).and_then(Value::as_array)
                    .is_some_and(|rows| rows.len() == 3
                        && rows[0].as_u64() == Some(0)
                        && rows[1].as_u64() == Some(2)
                        && rows[2].as_u64() == Some(3))
                && at(value, &["expected_e_optimal", "candidate_indices"]).and_then(Value::as_array)
                    .is_some_and(|rows| rows.len() == 3
                        && rows[0].as_u64() == Some(0)
                        && rows[1].as_u64() == Some(3)
                        && rows[2].as_u64() == Some(3))
                && number_at(value, &["expected_e_optimal", "minimum_information_eigenvalue"])
                    .is_some_and(|value| (value - 3.1511421982038947).abs() < 1.0e-10)
                && number_at(value, &["expected_d_optimal", "minimum_information_eigenvalue"])
                    .is_some_and(|value| (value - 2.583592135001262).abs() < 1.0e-10)
                && number_at(value, &["expected_d_optimal", "determinant_information"]) == Some(76.0)
                && number_at(value, &["expected_e_optimal", "determinant_information"]) == Some(72.0)
                && number_at(value, &["expected_e_optimal", "minimum_information_eigenvalue"])
                    .zip(number_at(value, &["expected_d_optimal", "minimum_information_eigenvalue"]))
                    .is_some_and(|(e, d)| e > d)
                && number_at(value, &["expected_d_optimal", "log_determinant_information"])
                    .zip(number_at(value, &["expected_e_optimal", "log_determinant_information"]))
                    .is_some_and(|(d, e)| d > e)
                && text_at(value, &["claim_boundary"])
                    .is_some_and(|claim| claim.contains("not EHD operating points")))
        }),
        "IDENTIFIABILITY_DESIGN_TOURNAMENT",
        "matched-budget D/E tournament outcomes, criteria and manufactured claim boundary must remain explicit",
    );
    let limits = at(root, &["limitations"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        limits.is_some_and(|rows| rows.iter().any(|row| row.as_str() == Some("two_parameters_only"))
            && rows.iter().any(|row| row.as_str() == Some("diagonal_observation_covariance_only"))
            && rows.iter().any(|row| row.as_str() == Some("no_nonlinear_profile_likelihood"))
            && rows.iter().any(|row| row.as_str() == Some("no_physical_ehd_parameter_estimation")))
            && text_at(root, &["claim_ceiling"]).is_some_and(|claim| {
                claim.contains("not an EHD physical parameter estimator")
                    && claim.contains("not an experiment authorization")
            }),
        "IDENTIFIABILITY_CLAIM_BOUNDARY",
        "manufactured linear kernel must retain explicit covariance, nonlinear and physical claim limits",
    );
    findings
}




/// Audit the frozen D0 manufactured error-source separation corpus.
pub fn audit_discrepancy_d0_contract(root: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    push_if_missing(
        &mut findings,
        text_at(root, &["schema"]) == Some("luminous.ehd_discrepancy_d0_corpus")
            && text_at(root, &["schema_version"]) == Some("0.1.0")
            && text_at(root, &["benchmark_id"]) == Some("EHD-D0-001")
            && text_at(root, &["status"])
                == Some("manufactured_d0_residual_attribution_kernel_authored_tests_execution_pending"),
        "D0_IDENTITY",
        "D0 error-source corpus identity, version or execution status changed",
    );
    push_if_missing(
        &mut findings,
        at(root, &["reference_basis", "vector_length"]).and_then(Value::as_u64) == Some(8)
            && at(root, &["reference_basis", "systematic_signatures"]).and_then(Value::as_array)
                .is_some_and(|rows| rows.len() == 4),
        "D0_REFERENCE_BASIS",
        "D0 must retain four systematic signatures over eight residual components",
    );
    push_if_missing(
        &mut findings,
        text_at(root, &["reference_implementation", "path"])
            == Some("tools/ehd-reference-audit/src/discrepancy.rs")
            && text_at(root, &["reference_implementation", "function"])
                == Some("analyse_d0_case")
            && at(root, &["reference_implementation", "tests"]).and_then(Value::as_u64)
                == Some(7),
        "D0_IMPLEMENTATION_SCOPE",
        "D0 implementation path, entry point and seven authored tests must remain declared",
    );
    let signatures = at(root, &["reference_basis", "systematic_signatures"])
        .and_then(Value::as_array);
    if let Some(rows) = signatures {
        let vectors: Option<Vec<Vec<f64>>> = rows
            .iter()
            .map(|row| {
                let values = at(row, &["values"])?.as_array()?;
                if values.len() != 8 {
                    return None;
                }
                values.iter().map(Value::as_f64).collect()
            })
            .collect();
        let orthogonal = vectors.as_ref().is_some_and(|basis| {
            basis.iter().enumerate().all(|(i, left)| {
                basis.iter().enumerate().all(|(j, right)| {
                    i == j || left.iter().zip(right).map(|(a, b)| a * b).sum::<f64>().abs() < 1.0e-12
                })
            })
        });
        push_if_missing(
            &mut findings,
            orthogonal,
            "D0_SIGNATURE_ORTHOGONALITY",
            "frozen systematic signatures must remain pairwise orthogonal",
        );
    }
    push_if_missing(
        &mut findings,
        text_at(root, &["attribution_policy", "confounding"]) == Some("reject_and_abstain")
            && text_at(root, &["attribution_policy", "conservation_firewall"])
                == Some("physical_ledger_failure_has_priority_over_any_discrepancy_fit")
            && text_at(root, &["attribution_policy", "validation_envelope"])
                == Some("explicit_external_boolean_gate_not_inferred_from_residual"),
        "D0_ATTRIBUTION_POLICY",
        "confounding, independent-ledger and external-envelope policies changed",
    );
    let cases = at(root, &["manufactured_cases"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        cases.is_some_and(|rows| rows.len() == 6
            && text_at(&rows[0], &["case_id"]) == Some("D0-MIXED-ERRORS")
            && text_at(&rows[0], &["expected_disposition"])
                == Some("parameter_or_numerical_error_detected")
            && text_at(&rows[1], &["expected_disposition"])
                == Some("no_systematic_source_detected")
            && text_at(&rows[2], &["expected_disposition"]) == Some("not_identifiable_abstain")
            && text_at(&rows[3], &["expected_disposition"]) == Some("physical_ledger_failure")
            && text_at(&rows[4], &["expected_disposition"]) == Some("out_of_envelope")
            && text_at(&rows[5], &["expected_disposition"]) == Some("not_evaluable")),
        "D0_MANUFACTURED_CASES",
        "mixed, noise-only, confounded, ledger-failure, OOD and unexplained-residual cases are required",
    );
    push_if_missing(
        &mut findings,
        at(root, &["conservation_firewall", "charge_mass_momentum_energy_ledgers_remain_independent_gates"])
            .and_then(Value::as_bool) == Some(true)
            && at(root, &["conservation_firewall", "discrepancy_can_override_failed_ledger"])
                .and_then(Value::as_bool) == Some(false),
        "D0_CONSERVATION_FIREWALL",
        "discrepancy may never promote a failed independent physical ledger",
    );
    push_if_missing(
        &mut findings,
        at(root, &["test_families"]).and_then(Value::as_array)
            .is_some_and(|tests| tests.len() == 7
                && tests.iter().any(|item| item.as_str() == Some("mixed_parameter_numerical_artifact_and_model_errors_are_separated"))
                && tests.iter().any(|item| item.as_str() == Some("correlated_parameter_and_numerical_signatures_abstain"))
                && tests.iter().any(|item| item.as_str() == Some("discrepancy_fit_never_repairs_failed_conservation_ledger"))
                && tests.iter().any(|item| item.as_str() == Some("out_of_envelope_case_abstains_even_with_small_residual"))
                && tests.iter().any(|item| item.as_str() == Some("unexplained_residual_above_measurement_noise_bound_is_not_evaluable"))),
        "D0_TEST_MATRIX",
        "all seven D0 manufactured and adversarial Rust test families must remain declared",
    );
    let limits = at(root, &["limitations"]).and_then(Value::as_array);
    push_if_missing(
        &mut findings,
        limits.is_some_and(|rows| {
            rows.iter().any(|item| item.as_str() == Some("manufactured_hadamard_signatures_only"))
                && rows.iter().any(|item| item.as_str() == Some("orthogonal_systematic_signatures_required"))
                && rows.iter().any(|item| item.as_str() == Some("no_general_discrepancy_function"))
                && rows.iter().any(|item| item.as_str() == Some("no_physical_ehd_measurements"))
        }) && text_at(root, &["claim_ceiling"]).is_some_and(|claim| {
            claim.contains("not a general EHD discrepancy model")
                && claim.contains("not a physical validation")
        }),
        "D0_CLAIM_BOUNDARY",
        "manufactured-only basis and no physical calibration/validation claim must remain explicit",
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

    let frontier: Value = match serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded frontier JSON: {error}");
            process::exit(2);
        }
    };
    let exchange: Value = match serde_json::from_str(EMBEDDED_EXCHANGE_CONTRACT) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded exchange contract JSON: {error}");
            process::exit(2);
        }
    };
    let array_interaction: Value = match serde_json::from_str(EMBEDDED_ARRAY_INTERACTION) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded array-interaction JSON: {error}");
            process::exit(2);
        }
    };
    let gcl_benchmark: Value = match serde_json::from_str(EMBEDDED_GCL_BENCHMARK) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded moving-mesh GCL benchmark JSON: {error}");
            process::exit(2);
        }
    };
    let mut findings = audit_atlas(&atlas);
    findings.extend(audit_frontier_patterns(&frontier));
    findings.extend(audit_exchange_contract(&exchange));
    findings.extend(audit_array_interaction(&array_interaction));
    findings.extend(audit_gcl_benchmark(&gcl_benchmark));
    let exchange_receipt_contract: Value = match serde_json::from_str(EMBEDDED_EXCHANGE_RECEIPT) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded exchange event receipt JSON: {error}");
            process::exit(2);
        }
    };
    findings.extend(audit_exchange_receipt_contract(&exchange_receipt_contract));
    let polynomial_moments_contract: Value = match serde_json::from_str(EMBEDDED_POLYNOMIAL_MOMENTS) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded polynomial-moment contract JSON: {error}");
            process::exit(2);
        }
    };
    findings.extend(audit_polynomial_moments_contract(&polynomial_moments_contract));
    let drag_dissipation_contract: Value = match serde_json::from_str(EMBEDDED_DRAG_DISSIPATION) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded drag-dissipation contract JSON: {error}");
            process::exit(2);
        }
    };
    findings.extend(audit_drag_dissipation_contract(&drag_dissipation_contract));
    let drag_trajectory_contract: Value = match serde_json::from_str(EMBEDDED_DRAG_TRAJECTORY) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded time-varying drag quadrature contract JSON: {error}");
            process::exit(2);
        }
    };
    findings.extend(audit_drag_trajectory_contract(&drag_trajectory_contract));
    let thrust_observability_contract: Value = match serde_json::from_str(EMBEDDED_THRUST_OBSERVABILITY) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded thrust observability contract JSON: {error}");
            process::exit(2);
        }
    };
    findings.extend(audit_thrust_observability_contract(&thrust_observability_contract));
    let control_volume_contract: Value = match serde_json::from_str(EMBEDDED_CONTROL_VOLUME) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded control-volume reducer contract JSON: {error}");
            process::exit(2);
        }
    };
    findings.extend(audit_control_volume_contract(&control_volume_contract));
    let identifiability_contract: Value = match serde_json::from_str(EMBEDDED_IDENTIFIABILITY) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded experiment-identifiability JSON: {error}");
            process::exit(2);
        }
    };
    findings.extend(audit_identifiability_contract(&identifiability_contract));
    let discrepancy_d0_contract: Value = match serde_json::from_str(EMBEDDED_DISCREPANCY_D0) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid embedded D0 discrepancy corpus JSON: {error}");
            process::exit(2);
        }
    };
    findings.extend(audit_discrepancy_d0_contract(&discrepancy_d0_contract));
    let exchange_kernel_ok = match exchange::reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("EXCHANGE_KERNEL_SELF_CHECK: {error}"));
            false
        }
    };
    let geometry_kernel_ok = match geometry::reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("GEOMETRY_KERNEL_SELF_CHECK: {error}"));
            false
        }
    };
    let gcl_kernel_ok = match gcl::reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("GCL_KERNEL_SELF_CHECK: {error}"));
            false
        }
    };
    let exchange_receipt_ok = match exchange_receipt::reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("EXCHANGE_RECEIPT_SELF_CHECK: {error}"));
            false
        }
    };
    let polynomial_moments_ok = match polynomial_moments::reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("POLYNOMIAL_MOMENTS_SELF_CHECK: {error}"));
            false
        }
    };
    let drag_dissipation_ok = match drag_dissipation::reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("DRAG_DISSIPATION_SELF_CHECK: {error}"));
            false
        }
    };
    let drag_trajectory_ok = match drag_trajectory::reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("DRAG_TRAJECTORY_SELF_CHECK: {error}"));
            false
        }
    };
    let control_volume_ok = match control_volume::reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("CONTROL_VOLUME_SELF_CHECK: {error}"));
            false
        }
    };
    let identifiability_ok = match identifiability::reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("IDENTIFIABILITY_SELF_CHECK: {error}"));
            false
        }
    };
    let discrepancy_d0_ok = match discrepancy::reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("D0_DISCREPANCY_SELF_CHECK: {error}"));
            false
        }
    };
    let design_tournament_ok = match identifiability::tournament_reference_self_check() {
        Ok(()) => true,
        Err(error) => {
            findings.push(format!("IDENTIFIABILITY_DESIGN_TOURNAMENT_SELF_CHECK: {error}"));
            false
        }
    };
    if findings.is_empty() {
        let count = len_at(&atlas, &["source_records"]).unwrap_or(0);
        let system_count = len_at(&atlas, &["system_integration_references"]).unwrap_or(0);
        println!("PASS: {label}");
        println!("physical source cohorts: {count}");
        println!("integrated-system references: {system_count}");
        let frontier_count = len_at(&frontier, &["records"]).unwrap_or(0);
        println!("frontier design-pattern references: {frontier_count}");
        println!("plasma-neutral exchange contract: audited");
        println!("array interaction protocol: audited");
        if exchange_kernel_ok {
            println!("conservative impulse remap kernel self-check: passed");
        }
        if geometry_kernel_ok {
            println!("axis-aligned space-time geometry self-check: passed");
        }
        if gcl_kernel_ok {
            println!("moving-AABB geometric conservation reference self-check: passed");
        }
        if exchange_receipt_ok {
            println!("event-level equal-and-opposite exchange receipt self-check: passed");
        }
        if polynomial_moments_ok {
            println!("P0/P1/P2 exact polynomial-moment integration self-check: passed");
        }
        if drag_dissipation_ok {
            println!("constant-drift ion-neutral drag dissipation self-check: passed");
        }
        if drag_trajectory_ok {
            println!("time-varying ion-neutral drag quadrature receipt self-check: passed");
        }
        println!("thrust observability protocol: audited (no physical measurement performed)");
        if control_volume_ok {
            println!("control-volume momentum and force-balance reducer self-check: passed");
        }
        if identifiability_ok {
            println!("two-parameter experiment-identifiability reference self-check: passed");
        }
        if design_tournament_ok {
            println!("matched-budget D/E discrete experiment-design tournament self-check: passed");
        }
        if discrepancy_d0_ok {
            println!("D0 manufactured error-source separation and conservation-firewall self-check: passed");
        }
        println!("scope: source structure and analytic reference checks only; no physical V&V, production ALE, TPE(1/2), nonlinear profile likelihood, continuous design optimization, complete collisional-energy closure or durable exactly-once claim");
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
    fn source_figure_endpoints_match_known_answers() {
        let mut value = atlas();
        value["source_records"][1]["extracted_figure_data"]["figure3_ignition_voltage"]["rows"][0]["ignition_voltage_kv"] =
            Value::from(7.31);
        assert!(has_code(&audit_atlas(&value), "USUELLI_SOURCE_POINTS"));
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
    fn tethered_microrobot_hover_cannot_be_promoted_to_free_flight() {
        let mut value = atlas();
        value["system_integration_references"][0]["operating_boundary"]
            ["self_powered_free_flight_demonstrated"] = Value::Bool(true);
        assert!(has_code(&audit_atlas(&value), "TAO_POWER_BOUNDARY"));
    }

    #[test]
    fn missing_integrated_system_reference_is_rejected() {
        let mut value = atlas();
        value["system_integration_references"] = Value::Array(Vec::new());
        assert!(has_code(&audit_atlas(&value), "TAO_SYSTEM_REFERENCE"));
    }

    #[test]
    fn missing_commercial_comsol_profile_is_rejected() {
        let mut value = atlas();
        value["solver_profiles"].as_array_mut().unwrap().retain(|p| {
            p["solver_id"].as_str() != Some("COMSOL_MULTIPHYSICS")
        });
        assert!(has_code(&audit_atlas(&value), "COMSOL_PROFILE"));
    }

    #[test]
    fn simplified_comsol_corona_interface_cannot_be_promoted_to_full_discharge() {
        let mut value = atlas();
        value["solver_profiles"][3]["important_limits"] = Value::Array(Vec::new());
        assert!(has_code(&audit_atlas(&value), "COMSOL_CORONA_SCOPE"));
    }

    #[test]
    fn comsol_ehd_capability_remains_explicit() {
        let mut value = atlas();
        value["solver_profiles"][3]["capabilities"] = Value::Array(Vec::new());
        assert!(has_code(&audit_atlas(&value), "COMSOL_EHD_CAPABILITY"));
    }

    #[test]
    fn missing_merlino_wire_cylinder_reference_case_is_rejected() {
        let mut value = atlas();
        value["solver_reference_cases"] = Value::Array(Vec::new());
        assert!(has_code(&audit_atlas(&value), "MERLINO_REFERENCE_CASE"));
    }

    #[test]
    fn repository_iva_csv_known_answer_is_enforced() {
        let mut value = atlas();
        value["solver_reference_cases"][0]["experimental_comparator"]["source_rows"][0]
            ["current_ma"] = Value::from(0.05);
        assert!(has_code(&audit_atlas(&value), "MERLINO_REFERENCE_DATA"));
    }

    #[test]
    fn wire_cylinder_iv_case_cannot_be_promoted_to_thrust_validation() {
        let mut value = atlas();
        value["solver_reference_cases"][0]["claim_boundary"] =
            Value::String("full thrust and neutral-flow validation".into());
        assert!(has_code(&audit_atlas(&value), "MERLINO_REFERENCE_CLAIM"));
    }

    #[test]
    fn half_domain_current_correction_is_mandatory() {
        let mut value = atlas();
        value["solver_reference_cases"][0]["experimental_comparator"]["plot_conversion"]
            ["simulated_integrated_current_to_microamp"] = Value::String("multiply by 1e6".into());
        assert!(has_code(&audit_atlas(&value), "MERLINO_CURRENT_SCALING"));
    }

    #[test]
    fn wire_cylinder_surface_gap_is_not_conflated_with_geo_parameter() {
        let mut value = atlas();
        value["solver_reference_cases"][0]["geometry"]
            ["derived_minimum_electrode_surface_clearance_mm"] = Value::from(30.0);
        assert!(has_code(&audit_atlas(&value), "MERLINO_GEOMETRY_CLEARANCE"));
    }

    #[test]
    fn empty_source_record_set_is_rejected() {
        let mut value = atlas();
        value["source_records"] = Value::Array(Vec::new());
        assert!(has_code(&audit_atlas(&value), "SOURCE_RECORDS"));
    }

    #[test]
    fn frontier_design_patterns_pass_declared_contract() {
        let value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS)
            .expect("embedded frontier data must be valid JSON");
        assert!(audit_frontier_patterns(&value).is_empty());
    }

    #[test]
    fn sub_unit_gain_arithmetic_is_enforced() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][0]["reported_values"]["sub_unit_gain_pct_recomputed"] = Value::from(10.0);
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_JIANG_ARITHMETIC"));
    }

    #[test]
    fn guide_electrode_absolute_metrics_cannot_be_invented() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][1]["reported_values"]["absolute_thrust_to_power"] = Value::from(9.0);
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_NONAKA_SOURCE_LOCAL"));
    }

    #[test]
    fn tethered_microrobot_hover_cannot_be_promoted_to_self_powered() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][2]["reported_values"]["hover_is_tethered"] = Value::Bool(false);
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_TAO_BOUNDARY"));
    }

    #[test]
    fn future_issue_date_mobility_result_cannot_enter_leaderboard() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][3]["evidence_class"] = Value::String("published_measured_benchmark".into());
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_DONG_DATE_GUARD"));
    }

    #[test]
    fn all_frontier_topology_factors_are_required() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["proposed_design_factors"].as_array_mut().unwrap().retain(|item| {
            item["id"].as_str() != Some("guide_electrode")
        });
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_DESIGN_FACTOR_SET"));
    }


    #[test]
    fn geometry_backend_scope_and_overlap_checks_are_mandatory() {
        let mut value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_CONTRACT).unwrap();
        value["exchange_mapping"]["reference_kernel"]["geometry_backend"]["limitations"] =
            Value::Array(vec![Value::String("none".into())]);
        assert!(has_code(&audit_exchange_contract(&value), "EXCHANGE_GEOMETRY_BACKEND"));
    }

    #[test]
    fn exchange_reference_kernel_keeps_its_limited_first_order_scope() {
        let mut value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_CONTRACT).unwrap();
        value["exchange_mapping"]["reference_kernel"]["limitations"] = Value::Array(Vec::new());
        assert!(has_code(&audit_exchange_contract(&value), "EXCHANGE_REFERENCE_KERNEL"));
    }

    #[test]
    fn precice_interoperability_boundary_is_retained() {
        let mut value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_CONTRACT).unwrap();
        value["exchange_mapping"]["industry_interoperability"]["reference"] =
            Value::String("nearest-neighbor".into());
        assert!(has_code(&audit_exchange_contract(&value), "EXCHANGE_INDUSTRY_INTEROP"));
    }

    #[test]
    fn plasma_neutral_exchange_contract_passes_declared_contract() {
        let value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_CONTRACT)
            .expect("embedded exchange contract must be valid JSON");
        assert!(audit_exchange_contract(&value).is_empty());
    }

    #[test]
    fn explicit_drag_cannot_be_combined_with_direct_coulomb_deposition() {
        let mut value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_CONTRACT).unwrap();
        value["force_transfer_methods"][0]["forbidden_combination"] =
            Value::String("no restrictions".into());
        assert!(has_code(&audit_exchange_contract(&value), "EXCHANGE_EXPLICIT_DOUBLE_COUNT_GUARD"));
    }

    #[test]
    fn reduced_coulomb_closure_requires_omitted_term_bounds() {
        let mut value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_CONTRACT).unwrap();
        value["force_transfer_methods"][1]["required_coefficients"] = Value::Array(Vec::new());
        assert!(has_code(&audit_exchange_contract(&value), "EXCHANGE_REDUCED_QUALIFICATION"));
    }

    #[test]
    fn interpolation_only_mesh_transfer_is_rejected() {
        let mut value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_CONTRACT).unwrap();
        value["exchange_mapping"]["interpolation_only_is_sufficient"] = Value::Bool(true);
        assert!(has_code(&audit_exchange_contract(&value), "EXCHANGE_MAPPING_POLICY"));
    }

    #[test]
    fn charge_momentum_and_energy_ledgers_are_mandatory() {
        let mut value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_CONTRACT).unwrap();
        value["energy_ledger"]["required_fields"] = Value::Array(Vec::new());
        assert!(has_code(&audit_exchange_contract(&value), "EXCHANGE_LEDGER_FIELDS"));
    }

    #[test]
    fn universal_tolerance_cannot_replace_case_specific_gates() {
        let mut value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_CONTRACT).unwrap();
        value["acceptance_policy"]["universal_numeric_tolerance"] = Value::from(1e-6);
        assert!(has_code(&audit_exchange_contract(&value), "EXCHANGE_TOLERANCE_POLICY"));
    }


    #[test]
    fn multistage_optimizer_per_span_metric_cannot_be_relabelled_as_area_density() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][4]["reported_values"]["metric_unit_n_per_m"] = Value::String("N/m^2".into());
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_SHIBATA_METRICS"));
    }

    #[test]
    fn multistage_optimizer_is_not_physical_validation() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][4]["thrust_estimation"]["physical_force_measurement_performed"] =
            Value::Bool(true);
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_SHIBATA_MODEL_SCOPE"));
    }


    #[test]
    fn booster_electrode_gains_retain_experimental_source_boundary() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][5]["method"]["experimental_validation"] = Value::Bool(false);
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_WAKIMOTO_SOURCE_SCOPE"));
    }

    #[test]
    fn anemone_three_dimensional_solver_scope_cannot_be_promoted_to_net_thrust() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][6]["validation_boundary"] = Value::String("Complete neutral-flow and net-thrust validation".into());
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_ANEMONE_SCOPE"));
    }

    #[test]
    fn integral_model_experimental_geometry_cohort_is_required() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][7]["method"]["experimental_comparison_geometries"] = Value::Array(Vec::new());
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_INTEGRAL_MODEL_SCOPE"));
    }

    #[test]
    fn multi_anode_interaction_cannot_be_assumed_additive_for_all_geometries() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][8]["reported_values"]["orthogonal_anodes_force_superposition"] = Value::Bool(true);
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_ARRAY_NONADDITIVITY"));
    }

    #[test]
    fn switched_thrust_comparator_must_retain_switching_loss_unknowns() {
        let mut value: Value = serde_json::from_str(EMBEDDED_FRONTIER_PATTERNS).unwrap();
        value["records"][9]["missing_or_unverified"] = Value::Array(Vec::new());
        assert!(has_code(&audit_frontier_patterns(&value), "FRONTIER_SWITCHING_SCOPE"));
    }


    #[test]
    fn array_interaction_protocol_passes_declared_contract() {
        let value: Value = serde_json::from_str(EMBEDDED_ARRAY_INTERACTION)
            .expect("embedded array-interaction contract must be valid JSON");
        assert!(audit_array_interaction(&value).is_empty());
    }

    #[test]
    fn isolated_cell_force_additivity_is_not_a_default() {
        let mut value: Value = serde_json::from_str(EMBEDDED_ARRAY_INTERACTION).unwrap();
        value["model_contract"]["isolated_cell_additivity_is_default"] = Value::Bool(true);
        assert!(has_code(&audit_array_interaction(&value), "ARRAY_MODEL_SCOPE"));
    }

    #[test]
    fn local_array_jacobian_requires_orientation_spacing_and_session_holdouts() {
        let mut value: Value = serde_json::from_str(EMBEDDED_ARRAY_INTERACTION).unwrap();
        value["model_contract"]["training_holdouts"] = Value::Array(vec![
            Value::String("random_rows".into())
        ]);
        assert!(has_code(&audit_array_interaction(&value), "ARRAY_MODEL_SCOPE"));
    }

    #[test]
    fn array_experiment_requires_per_cell_waveforms_and_calibration_identity() {
        let mut value: Value = serde_json::from_str(EMBEDDED_ARRAY_INTERACTION).unwrap();
        value["measured_configuration_fields"] = Value::Array(vec![
            Value::String("configuration_id".into())
        ]);
        assert!(has_code(&audit_array_interaction(&value), "ARRAY_MEASURED_CONFIGURATION_FIELDS"));
    }

    #[test]
    fn array_direction_and_magnitude_gates_must_remain_prospective() {
        let mut value: Value = serde_json::from_str(EMBEDDED_ARRAY_INTERACTION).unwrap();
        value["proposed_acceptance_gates"]["all_targets_prospective"] = Value::Bool(false);
        assert!(has_code(&audit_array_interaction(&value), "ARRAY_PROSPECTIVE_GATES"));
    }

    #[test]
    fn array_claims_must_not_promote_protocol_to_measurement() {
        let mut value: Value = serde_json::from_str(EMBEDDED_ARRAY_INTERACTION).unwrap();
        value["claim_ceiling"] = Value::String("Luminous measured a global array law".into());
        assert!(has_code(&audit_array_interaction(&value), "ARRAY_CLAIM_BOUNDARY"));
    }

}

#[cfg(test)]
mod moving_gcl_contract_tests {
    use super::*;

    #[test]
    fn moving_mesh_gcl_benchmark_passes_declared_contract() {
        let value: Value = serde_json::from_str(EMBEDDED_GCL_BENCHMARK)
            .expect("embedded moving-mesh GCL benchmark must be valid JSON");
        assert!(audit_gcl_benchmark(&value).is_empty());
    }

    #[test]
    fn moving_mesh_gcl_claim_cannot_be_promoted_to_general_ale_validation() {
        let mut value: Value = serde_json::from_str(EMBEDDED_GCL_BENCHMARK).unwrap();
        value["limitations"] = Value::Array(Vec::new());
        assert!(audit_gcl_benchmark(&value).iter().any(|finding| {
            finding.starts_with("GCL_CLAIM_BOUNDARY")
        }));
    }

    #[test]
    fn moving_mesh_gcl_requires_case_specific_tolerances() {
        let mut value: Value = serde_json::from_str(EMBEDDED_GCL_BENCHMARK).unwrap();
        value["acceptance"]["universal_numeric_tolerance"] = Value::from(1.0e-6);
        assert!(audit_gcl_benchmark(&value).iter().any(|finding| {
            finding.starts_with("GCL_TOLERANCE_AND_LEDGER_POLICY")
        }));
    }
}

#[cfg(test)]
mod exchange_event_receipt_contract_tests {
    use super::*;

    #[test]
    fn event_level_exchange_receipt_contract_passes() {
        let value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_RECEIPT)
            .expect("embedded exchange receipt contract must be valid JSON");
        assert!(audit_exchange_receipt_contract(&value).is_empty());
    }

    #[test]
    fn event_level_exchange_receipt_cannot_claim_durable_exactly_once() {
        let mut value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_RECEIPT).unwrap();
        value["acceptance"]["durable_exactly_once_not_implemented"] = Value::Bool(false);
        assert!(audit_exchange_receipt_contract(&value).iter().any(|finding| {
            finding.starts_with("EXCHANGE_RECEIPT_ACCEPTANCE")
        }));
    }

    #[test]
    fn event_level_exchange_receipt_must_check_per_event_balance() {
        let mut value: Value = serde_json::from_str(EMBEDDED_EXCHANGE_RECEIPT).unwrap();
        value["required_invariants"] = Value::Array(vec![
            Value::String("aggregate_impulse_balance".into())
        ]);
        assert!(audit_exchange_receipt_contract(&value).iter().any(|finding| {
            finding.starts_with("EXCHANGE_RECEIPT_INVARIANTS")
        }));
    }
}

#[cfg(test)]
mod polynomial_moment_contract_tests {
    use super::*;

    #[test]
    fn polynomial_moment_contract_passes_declared_scope() {
        let value: Value = serde_json::from_str(EMBEDDED_POLYNOMIAL_MOMENTS)
            .expect("embedded polynomial-moment contract must be valid JSON");
        assert!(audit_polynomial_moments_contract(&value).is_empty());
    }

    #[test]
    fn polynomial_moment_oracle_cannot_claim_tpe_1_or_tpe_2() {
        let mut value: Value = serde_json::from_str(EMBEDDED_POLYNOMIAL_MOMENTS).unwrap();
        value["limitations"] = Value::Array(Vec::new());
        assert!(audit_polynomial_moments_contract(&value).iter().any(|finding| {
            finding.starts_with("POLY_MOMENT_CLAIM_BOUNDARY")
        }));
    }

    #[test]
    fn polynomial_moment_oracle_requires_local_and_global_integral_checks() {
        let mut value: Value = serde_json::from_str(EMBEDDED_POLYNOMIAL_MOMENTS).unwrap();
        value["acceptance"]["per_destination_integral_checked"] = Value::Bool(false);
        assert!(audit_polynomial_moments_contract(&value).iter().any(|finding| {
            finding.starts_with("POLY_MOMENT_ACCEPTANCE")
        }));
    }
}

#[cfg(test)]
mod drag_dissipation_contract_tests {
    use super::*;

    #[test]
    fn constant_drift_drag_dissipation_contract_passes() {
        let value: Value = serde_json::from_str(EMBEDDED_DRAG_DISSIPATION)
            .expect("embedded drag-dissipation contract must be valid JSON");
        assert!(audit_drag_dissipation_contract(&value).is_empty());
    }

    #[test]
    fn drag_dissipation_contract_rejects_time_varying_claim_without_quadrature() {
        let mut value: Value = serde_json::from_str(EMBEDDED_DRAG_DISSIPATION).unwrap();
        value["limitations"] = Value::Array(Vec::new());
        assert!(audit_drag_dissipation_contract(&value).iter().any(|finding| {
            finding.starts_with("DRAG_DISSIPATION_CLAIM_BOUNDARY")
        }));
    }

    #[test]
    fn drag_dissipation_contract_requires_event_level_energy_checks() {
        let mut value: Value = serde_json::from_str(EMBEDDED_DRAG_DISSIPATION).unwrap();
        value["acceptance"]["per_event_momentum_and_energy_checks_required"] = Value::Bool(false);
        assert!(audit_drag_dissipation_contract(&value).iter().any(|finding| {
            finding.starts_with("DRAG_DISSIPATION_ACCEPTANCE")
        }));
    }
}

#[cfg(test)]
mod drag_trajectory_contract_tests {
    use super::*;

    #[test]
    fn time_varying_drag_quadrature_contract_passes_declared_scope() {
        let value: Value = serde_json::from_str(EMBEDDED_DRAG_TRAJECTORY)
            .expect("embedded time-varying drag quadrature contract must be valid JSON");
        assert!(audit_drag_trajectory_contract(&value).is_empty());
    }

    #[test]
    fn time_varying_drag_contract_requires_same_quadrature_for_impulse_and_heating() {
        let mut value: Value = serde_json::from_str(EMBEDDED_DRAG_TRAJECTORY).unwrap();
        value["acceptance"]["same_space_time_quadrature_for_impulse_and_heating"] = Value::Bool(false);
        assert!(audit_drag_trajectory_contract(&value).iter().any(|finding| {
            finding.starts_with("DRAG_TRAJECTORY_ACCEPTANCE")
        }));
    }

    #[test]
    fn time_varying_drag_contract_cannot_claim_full_collisional_energy_closure() {
        let mut value: Value = serde_json::from_str(EMBEDDED_DRAG_TRAJECTORY).unwrap();
        value["limitations"] = Value::Array(Vec::new());
        assert!(audit_drag_trajectory_contract(&value).iter().any(|finding| {
            finding.starts_with("DRAG_TRAJECTORY_CLAIM_BOUNDARY")
        }));
    }
}

#[cfg(test)]
mod drag_trajectory_frame_contract_tests {
    use super::*;

    #[test]
    fn time_varying_drag_contract_requires_explicit_frame_identity() {
        let mut value: Value = serde_json::from_str(EMBEDDED_DRAG_TRAJECTORY).unwrap();
        value["reference_implementation"]["event_fields"] = Value::Array(vec![
            Value::String("event_id".into()),
            Value::String("quadrature_rule_id".into()),
        ]);
        assert!(audit_drag_trajectory_contract(&value).iter().any(|finding| {
            finding.starts_with("DRAG_TRAJECTORY_IMPLEMENTATION")
        }));
    }
}

#[cfg(test)]
mod thrust_observability_contract_tests {
    use super::*;

    #[test]
    fn thrust_observability_protocol_passes_declared_contract() {
        let value: Value = serde_json::from_str(EMBEDDED_THRUST_OBSERVABILITY)
            .expect("embedded thrust observability protocol must be valid JSON");
        assert!(audit_thrust_observability_contract(&value).is_empty());
    }

    #[test]
    fn thrust_observability_rejects_single_plane_momentum_shortcut() {
        let mut value: Value = serde_json::from_str(EMBEDDED_THRUST_OBSERVABILITY).unwrap();
        value["momentum_equation"]["one_exit_plane_rho_u2_sufficient_for_net_thrust"] = Value::Bool(true);
        assert!(audit_thrust_observability_contract(&value).iter().any(|finding| {
            finding.starts_with("THRUST_CONTROL_VOLUME_COMPLETENESS")
        }));
    }

    #[test]
    fn thrust_observability_protocol_cannot_claim_measurements_or_sota() {
        let mut value: Value = serde_json::from_str(EMBEDDED_THRUST_OBSERVABILITY).unwrap();
        value["limitations"] = Value::Array(Vec::new());
        assert!(audit_thrust_observability_contract(&value).iter().any(|finding| {
            finding.starts_with("THRUST_OBSERVABILITY_CLAIM_BOUNDARY")
        }));
    }
}

#[cfg(test)]
mod control_volume_contract_tests {
    use super::*;

    #[test]
    fn control_volume_reducer_contract_passes() {
        let value: Value = serde_json::from_str(EMBEDDED_CONTROL_VOLUME)
            .expect("embedded control-volume reducer contract must be valid JSON");
        assert!(audit_control_volume_contract(&value).is_empty());
    }

    #[test]
    fn control_volume_contract_requires_complete_six_term_manifest() {
        let mut value: Value = serde_json::from_str(EMBEDDED_CONTROL_VOLUME).unwrap();
        value["required_vector_terms"].as_array_mut().unwrap().pop();
        assert!(audit_control_volume_contract(&value).iter().any(|finding| {
            finding.starts_with("CONTROL_VOLUME_REQUIRED_TERMS")
        }));
    }

    #[test]
    fn control_volume_contract_rejects_root_sum_square_as_default() {
        let mut value: Value = serde_json::from_str(EMBEDDED_CONTROL_VOLUME).unwrap();
        value["uncertainty_policy"]["rss_allowed_by_default"] = Value::Bool(true);
        assert!(audit_control_volume_contract(&value).iter().any(|finding| {
            finding.starts_with("CONTROL_VOLUME_UNCERTAINTY_POLICY")
        }));
    }
}


#[cfg(test)]
mod identifiability_contract_tests {
    use super::*;

    #[test]
    fn discrepancy_d0_contract_passes() {
        let value: Value = serde_json::from_str(EMBEDDED_DISCREPANCY_D0)
            .expect("embedded D0 discrepancy corpus must be valid JSON");
        assert!(audit_discrepancy_d0_contract(&value).is_empty());
    }

    #[test]
    fn discrepancy_d0_contract_rejects_conservation_laundering() {
        let mut value: Value = serde_json::from_str(EMBEDDED_DISCREPANCY_D0).unwrap();
        value["conservation_firewall"]["discrepancy_can_override_failed_ledger"] =
            Value::Bool(true);
        assert!(audit_discrepancy_d0_contract(&value).iter().any(|finding| {
            finding.starts_with("D0_CONSERVATION_FIREWALL")
        }));
    }

    #[test]
    fn identifiability_contract_passes() {
        let value: Value = serde_json::from_str(EMBEDDED_IDENTIFIABILITY)
            .expect("embedded identifiability benchmark must be valid JSON");
        assert!(audit_identifiability_contract(&value).is_empty());
    }

    #[test]
    fn identifiability_contract_rejects_parameter_estimate_for_rank_deficient_case() {
        let mut value: Value = serde_json::from_str(EMBEDDED_IDENTIFIABILITY).unwrap();
        value["manufactured_cases"][0]["expected_parameter_estimate"] =
            serde_json::json!([2.0, 0.5]);
        assert!(audit_identifiability_contract(&value).iter().any(|finding| {
            finding.starts_with("IDENTIFIABILITY_REPEATED_EXPECTATION")
        }));
    }

    #[test]
    fn identifiability_contract_requires_diagonal_covariance_boundary() {
        let mut value: Value = serde_json::from_str(EMBEDDED_IDENTIFIABILITY).unwrap();
        value["parameter_contract"]["correlated_covariance_supported"] = Value::Bool(true);
        assert!(audit_identifiability_contract(&value).iter().any(|finding| {
            finding.starts_with("IDENTIFIABILITY_PARAMETER_CONTRACT")
        }));
    }


    #[test]
    fn identifiability_contract_rejects_candidate_sensitivity_change() {
        let mut value: Value = serde_json::from_str(EMBEDDED_IDENTIFIABILITY).unwrap();
        value["reference_implementation"]["design_tournament"]["candidate_conditions"][0]
            ["sensitivity"][0] = serde_json::json!(-4.0);
        assert!(audit_identifiability_contract(&value).iter().any(|finding| {
            finding.starts_with("IDENTIFIABILITY_DESIGN_TOURNAMENT")
        }));
    }

    #[test]
    fn identifiability_contract_requires_distinct_d_and_e_optimal_selections() {
        let mut value: Value = serde_json::from_str(EMBEDDED_IDENTIFIABILITY).unwrap();
        value["reference_implementation"]["design_tournament"]["expected_e_optimal"]
            ["candidate_indices"] = serde_json::json!([0, 2, 3]);
        assert!(audit_identifiability_contract(&value).iter().any(|finding| {
            finding.starts_with("IDENTIFIABILITY_DESIGN_TOURNAMENT")
        }));
    }
}
