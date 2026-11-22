//! Offline integrity and comparability audit for the public EHD reference atlas.
//!
//! This checks source transcription structure and cohort boundaries. It does
//! not reproduce a plasma solver, validate physical measurements, or certify
//! that a source-reported result is physically correct.

use serde_json::Value;
use std::{env, fs, process};

mod exchange;
mod geometry;

const EMBEDDED_ATLAS: &str =
    include_str!("../../../data/research/ehd_reference_atlas_v0_1.json");
const EMBEDDED_FRONTIER_PATTERNS: &str =
    include_str!("../../../data/research/ehd_frontier_design_patterns_v0_1.json");
const EMBEDDED_EXCHANGE_CONTRACT: &str =
    include_str!("../../../data/research/ehd_plasma_neutral_exchange_contract_v0_1.json");
const EMBEDDED_ARRAY_INTERACTION: &str =
    include_str!("../../../data/research/ehd_array_interaction_benchmark_v0_1.json");

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

    // Wang et al. is a 10-Torr numerical needle-ring cohort. The
    // reported grid differences are not physical thrust uncertainty, and
    // the model's thermal-equilibrium / no-heating / chemistry limits remain
    // part of the solver applicability envelope.
    let wang = records.and_then(|rows| rows.iter().find(|r| {
        text_at(r, &["source_id"]) == Some("WANG_2026_NEEDLE_RING_10_TORR")
    }));
    push_if_missing(
        &mut findings,
        wang.is_some(),
        "WANG_RECORD",
        "missing 10-Torr needle-ring numerical EHD cohort",
    );
    if let Some(record) = wang {
        push_if_missing(
            &mut findings,
            text_at(record, &["doi"]) == Some("10.1088/1674-1056/adf829")
                && text_at(record, &["cohort"]) == Some("low_pressure_air_corona_needle_ring_numerical")
                && number_at(record, &["operating_envelope", "pressure_torr"]) == Some(10.0)
                && number_at(record, &["operating_envelope", "pressure_pa"]) == Some(1333.22),
            "WANG_COHORT_IDENTITY",
            "Wang needle-ring cohort DOI, regime or pressure identity changed",
        );
        push_if_missing(
            &mut findings,
            number_at(record, &["method", "species_count"]) == Some(12.0)
                && number_at(record, &["method", "reaction_count"]) == Some(26.0)
                && text_at(record, &["method", "dimensionality"]) == Some("2D axisymmetric cylindrical")
                && text_at(record, &["method", "solver"]) == Some("COMSOL Multiphysics"),
            "WANG_MODEL_IDENTITY",
            "source model dimensionality, solver or 12-species/26-reaction chemistry changed",
        );
        push_if_missing(
            &mut findings,
            number_at(record, &["reported_results", "nitrogen_oxygen_comparison", "current_ma", "baseline"])
                == Some(0.00244)
                && number_at(record, &["reported_results", "nitrogen_oxygen_comparison", "current_ma", "comparison"])
                    == Some(0.00179)
                && number_at(record, &["reported_results", "nitrogen_oxygen_comparison", "thrust_to_power_mn_kw", "baseline"])
                    == Some(29.38)
                && number_at(record, &["reported_results", "nitrogen_oxygen_comparison", "thrust_to_power_mn_kw", "comparison"])
                    == Some(41.52),
            "WANG_GAS_COMPOSITION_RESULTS",
            "source-reported 4:1/5:1 current and thrust-to-power values changed",
        );
        push_if_missing(
            &mut findings,
            number_at(record, &["reported_results", "secondary_electron_emission_sweep", "reported_thrust_increase_percent"])
                == Some(34.5)
                && number_at(record, &["reported_results", "secondary_electron_emission_sweep", "reported_thrust_to_power_increase_percent"])
                    == Some(142.2)
                && number_at(record, &["reported_results", "polarity_comparison", "negative_corona_thrust_percent_of_positive"])
                    == Some(0.57)
                && number_at(record, &["reported_results", "polarity_comparison", "negative_corona_thrust_to_power_percent_of_positive"])
                    == Some(0.78),
            "WANG_SENSITIVITY_RESULTS",
            "source-reported secondary-emission or polarity sensitivity values changed",
        );
        push_if_missing(
            &mut findings,
            number_at(record, &["reported_results", "grid_refinement", "thrust_difference_percent"])
                == Some(0.15)
                && number_at(record, &["reported_results", "grid_refinement", "current_difference_percent"])
                    == Some(0.36)
                && contains_any(record, &["reported_results", "grid_refinement", "claim_boundary"], "not total predictive uncertainty")
                && at(record, &["measurement", "physical_force_balance_performed"])
                    .and_then(Value::as_bool) == Some(false)
                && text_at(record, &["measurement", "evidence_class"]) == Some("simulation_output"),
            "WANG_GRID_AND_EVIDENCE_BOUNDARY",
            "grid-refinement differences must not be promoted to physical accuracy or force-balance evidence",
        );
        push_if_missing(
            &mut findings,
            contains_any(record, &["method", "physical_assumptions"], "thermal equilibrium")
                && contains_any(record, &["method", "physical_assumptions"], "corona discharge heating is neglected")
                && contains_any(record, &["method", "physical_assumptions"], "excited-state quenching")
                && contains_any(record, &["method", "model_limitations"], "not suitable for arc-discharge")
                && at(record, &["measurement", "physical_velocity_field_measurement_performed"])
                    .and_then(Value::as_bool) == Some(false)
                && text_at(record, &["measurement", "thrust_estimator"])
                    .is_some_and(|value| value.contains("outlet velocity profile")),
            "WANG_MODEL_LIMITS",
            "thermal, quenching, arc-regime and outlet-profile thrust limitations must remain explicit",
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
    let mut findings = audit_atlas(&atlas);
    findings.extend(audit_frontier_patterns(&frontier));
    findings.extend(audit_exchange_contract(&exchange));
    findings.extend(audit_array_interaction(&array_interaction));
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
        println!("scope: source structure and cohort comparability only; no physical V&V claim");
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
    fn wang_low_pressure_numerical_cohort_passes_source_contract() {
        assert!(audit_atlas(&atlas()).is_empty());
    }

    #[test]
    fn wang_grid_refinement_cannot_be_promoted_to_physical_uncertainty() {
        let mut value = atlas();
        let wang = value["source_records"].as_array_mut().unwrap()
            .iter_mut().find(|record| record["source_id"] == "WANG_2026_NEEDLE_RING_10_TORR")
            .unwrap();
        wang["reported_results"]["grid_refinement"]["claim_boundary"] =
            Value::String("physical thrust uncertainty".into());
        assert!(has_code(&audit_atlas(&value), "WANG_GRID_AND_EVIDENCE_BOUNDARY"));
    }

    #[test]
    fn wang_thermal_and_quenching_limits_cannot_be_removed() {
        let mut value = atlas();
        let wang = value["source_records"].as_array_mut().unwrap()
            .iter_mut().find(|record| record["source_id"] == "WANG_2026_NEEDLE_RING_10_TORR")
            .unwrap();
        wang["method"]["physical_assumptions"] = Value::Array(vec![]);
        assert!(has_code(&audit_atlas(&value), "WANG_MODEL_LIMITS"));
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
