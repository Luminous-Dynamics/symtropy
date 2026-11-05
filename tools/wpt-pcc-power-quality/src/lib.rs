// SPDX-License-Identifier: AGPL-3.0-or-later
//! Standards-aware profile applicability and point metrics for WPT power quality.
//!
//! This crate does not reproduce normative limit tables or issue a conformance
//! certificate. It checks whether a declared run matches a pinned profile's
//! boundary/scope and computes THD and IEEE-519-style TDD from supplied RMS
//! harmonic lines. "Applicable scope only" is deliberately not a pass result.

use serde_json::Value;
use std::collections::BTreeSet;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    for key in object(value, path)?.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn number_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = field(value, key, path)?
        .as_f64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be numeric"))?;
    if !number.is_finite() {
        return Err(fail(&format!("{path}.{key}"), "must be finite"));
    }
    Ok(number)
}

fn nullable_number(value: &Value, key: &str, path: &str) -> Result<Option<f64>, String> {
    let item = field(value, key, path)?;
    if item.is_null() {
        return Ok(None);
    }
    let number = item
        .as_f64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be numeric or null"))?;
    if !number.is_finite() {
        return Err(fail(&format!("{path}.{key}"), "must be finite"));
    }
    Ok(Some(number))
}

fn integer_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-negative integer"))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Compute a conventional current THD percentage from integer harmonic RMS
/// currents, normalized by the measured fundamental current.
pub fn thd_percent(harmonics_rms_a: &[f64], fundamental_rms_a: f64) -> Result<f64, String> {
    validate_currents(harmonics_rms_a, fundamental_rms_a, "THD")?;
    let square_sum = checked_square_sum(harmonics_rms_a, "THD")?;
    let result = square_sum.sqrt() / fundamental_rms_a * 100.0;
    if !result.is_finite() {
        return Err("THD calculation overflowed".into());
    }
    Ok(result)
}

/// Compute IEEE-519-style total demand distortion as a point metric.
///
/// The caller must supply the maximum-demand-current reference I_L appropriate
/// to the pinned PCC profile. This function does not infer I_L from the
/// instantaneous fundamental or from charger nameplate current.
pub fn tdd_percent(harmonics_rms_a: &[f64], i_l_rms_a: f64) -> Result<f64, String> {
    validate_currents(harmonics_rms_a, i_l_rms_a, "TDD")?;
    let square_sum = checked_square_sum(harmonics_rms_a, "TDD")?;
    let result = square_sum.sqrt() / i_l_rms_a * 100.0;
    if !result.is_finite() {
        return Err("TDD calculation overflowed".into());
    }
    Ok(result)
}

fn validate_currents(harmonics: &[f64], reference_a: f64, metric: &str) -> Result<(), String> {
    if !reference_a.is_finite() || reference_a <= 0.0 {
        return Err(format!(
            "{metric} reference current must be finite and positive"
        ));
    }
    if harmonics
        .iter()
        .any(|current| !current.is_finite() || *current < 0.0)
    {
        return Err(format!(
            "{metric} harmonic currents must be finite and non-negative"
        ));
    }
    Ok(())
}

fn checked_square_sum(values: &[f64], metric: &str) -> Result<f64, String> {
    let mut sum = 0.0;
    for value in values {
        sum += value * value;
        if !sum.is_finite() {
            return Err(format!("{metric} harmonic power accumulation overflowed"));
        }
    }
    Ok(sum)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileApplicability {
    ApplicableScopeOnly,
    /// Equipment/network class is broadly in scope, but this edition does not
    /// provide limit values for the declared nominal voltage/frequency system.
    ScopeOnlyLimitDomainNotCovered,
    NotApplicable,
    MeasurementMethodOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualityReport {
    pub run_id: String,
    pub standard_id: String,
    pub applicability: ProfileApplicability,
    pub maximum_harmonic_order: u64,
    /// THD computed through maximum_harmonic_order; it does not include higher orders.
    pub thd_through_order_percent: f64,
    /// TDD uses the same declared harmonic coverage and the PCC I_L reference.
    pub tdd_through_order_percent: Option<f64>,
    pub short_circuit_ratio_isc_over_il: Option<f64>,
    pub harmonic_components: usize,
    pub claim_ceiling: &'static str,
}

fn validate_measurement(run: &Value, profile_id: &str) -> Result<(), String> {
    let measurement = field(run, "measurement", "$")?;
    exact_keys(
        measurement,
        &[
            "measurement_standard_id",
            "measurement_edition",
            "measurement_class",
            "instrument_id",
            "calibration_receipt_id",
            "aggregation_window_id",
            "bandwidth_max_hz",
            "raw_artifact_sha256",
        ],
        "$.measurement",
    )?;
    let method_id = string_field(measurement, "measurement_standard_id", "$.measurement")?;
    let edition = string_field(measurement, "measurement_edition", "$.measurement")?;
    if ![
        "IEC_61000_4_30_2025",
        "IEC_61000_4_7_2002_AMD1_2008",
        "OTHER",
    ]
    .contains(&method_id)
    {
        return Err(fail(
            "$.measurement",
            "unknown measurement-method identifier",
        ));
    }
    if edition.is_empty() {
        return Err(fail(
            "$.measurement.measurement_edition",
            "edition must be explicit",
        ));
    }
    let class = string_field(measurement, "measurement_class", "$.measurement")?;
    if !["CLASS_A", "CLASS_S", "EQUIPMENT_TEST", "OTHER"].contains(&class) {
        return Err(fail(
            "$.measurement.measurement_class",
            "unknown measurement class",
        ));
    }
    string_field(measurement, "instrument_id", "$.measurement")?;
    string_field(measurement, "calibration_receipt_id", "$.measurement")?;
    string_field(measurement, "aggregation_window_id", "$.measurement")?;
    let bandwidth = number_field(measurement, "bandwidth_max_hz", "$.measurement")?;
    if bandwidth <= 0.0 {
        return Err(fail("$.measurement.bandwidth_max_hz", "must be positive"));
    }
    let digest = string_field(measurement, "raw_artifact_sha256", "$.measurement")?;
    if !valid_sha256(digest) {
        return Err(fail(
            "$.measurement.raw_artifact_sha256",
            "must be lowercase SHA-256 hex",
        ));
    }

    if (profile_id == "IEEE_519_2022" || profile_id == "IEC_61000_4_30_2025")
        && (method_id != "IEC_61000_4_30_2025"
            || edition != "IEC 61000-4-30:2025 COR1:2026"
            || !["CLASS_A", "CLASS_S"].contains(&class))
    {
        return Err(fail(
            "$.measurement",
            "in-situ PCC power-quality assessment requires IEC 61000-4-30:2025 COR1:2026 with Class A or Class S",
        ));
    }
    if profile_id == "IEC_61000_3_12_2011_AMD1_2021"
        && (method_id != "IEC_61000_4_7_2002_AMD1_2008"
            || edition != "IEC 61000-4-7:2002+AMD1:2008"
            || class != "EQUIPMENT_TEST")
    {
        return Err(fail(
            "$.measurement",
            "IEC 61000-3-12 equipment testing requires the pinned IEC 61000-4-7:2002+AMD1:2008 equipment-test method",
        ));
    }
    Ok(())
}

fn validate_quality_flags(run: &Value, evidence_kind: &str) -> Result<(), String> {
    let flags = field(run, "quality_flags", "$")?
        .as_array()
        .ok_or_else(|| fail("$.quality_flags", "must be an array"))?;
    let mut seen = BTreeSet::new();
    let mut synthetic = false;
    for flag in flags {
        let value = flag
            .as_str()
            .ok_or_else(|| fail("$.quality_flags", "each flag must be a string"))?;
        if !seen.insert(value) {
            return Err(fail("$.quality_flags", "duplicate quality flag"));
        }
        if ![
            "SYNTHETIC_ONLY",
            "CALIBRATION_MISSING",
            "CLOCK_UNCERTAIN",
            "BACKGROUND_DISTORTION_NOT_CHARACTERIZED",
            "LIMIT_TABLE_NOT_LOADED",
        ]
        .contains(&value)
        {
            return Err(fail(
                "$.quality_flags",
                format!("unknown quality flag {value}"),
            ));
        }
        synthetic |= value == "SYNTHETIC_ONLY";
    }
    if evidence_kind == "SYNTHETIC_KNOWN_TRUTH" && !synthetic {
        return Err(fail(
            "$.quality_flags",
            "synthetic evidence requires SYNTHETIC_ONLY",
        ));
    }
    if evidence_kind != "SYNTHETIC_KNOWN_TRUTH" && synthetic {
        return Err(fail(
            "$.quality_flags",
            "SYNTHETIC_ONLY cannot label non-synthetic evidence",
        ));
    }
    Ok(())
}

/// Validate one profile/run, recompute THD and (when a PCC current reference is
/// available) TDD, and classify profile applicability. It never compares
/// against normative limit tables and never returns a standards-compliance pass.
pub fn evaluate_run(run: &Value) -> Result<QualityReport, String> {
    exact_keys(
        run,
        &[
            "schema_version",
            "run_id",
            "evidence_kind",
            "profile",
            "measurement",
            "current_spectrum",
            "quality_flags",
        ],
        "$",
    )?;
    if string_field(run, "schema_version", "$")? != "wpt-pcc-power-quality-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let run_id = string_field(run, "run_id", "$")?.to_string();
    let evidence_kind = string_field(run, "evidence_kind", "$")?;
    if ![
        "MEASURED",
        "PUBLISHED_MEASURED",
        "SIMULATED",
        "SYNTHETIC_KNOWN_TRUTH",
    ]
    .contains(&evidence_kind)
    {
        return Err(fail("$.evidence_kind", "unknown evidence kind"));
    }
    validate_quality_flags(run, evidence_kind)?;

    let profile = field(run, "profile", "$")?;
    exact_keys(
        profile,
        &[
            "standard_id",
            "edition",
            "standard_role",
            "assessment_boundary",
            "network_type",
            "phase_count",
            "wire_count",
            "grid_frequency_hz",
            "nominal_voltage_v",
            "rated_input_current_a_per_phase",
            "pcc",
        ],
        "$.profile",
    )?;
    let standard_id = string_field(profile, "standard_id", "$.profile")?.to_string();
    let edition = string_field(profile, "edition", "$.profile")?;
    let role = string_field(profile, "standard_role", "$.profile")?;
    if ![
        "SYSTEM_PCC_LIMITS",
        "PRODUCT_EMISSION_LIMITS",
        "MEASUREMENT_METHOD_ONLY",
    ]
    .contains(&role)
    {
        return Err(fail("$.profile.standard_role", "unknown standard role"));
    }
    let boundary = string_field(profile, "assessment_boundary", "$.profile")?;
    if !["SITE_PCC", "EQUIPMENT_TERMINAL"].contains(&boundary) {
        return Err(fail(
            "$.profile.assessment_boundary",
            "unknown assessment boundary",
        ));
    }
    let network_type = string_field(profile, "network_type", "$.profile")?;
    if ![
        "PUBLIC_LOW_VOLTAGE",
        "PRIVATE_LOW_VOLTAGE",
        "MEDIUM_VOLTAGE",
        "HIGH_VOLTAGE",
        "OTHER",
    ]
    .contains(&network_type)
    {
        return Err(fail("$.profile.network_type", "unknown network type"));
    }
    let phase_count = integer_field(profile, "phase_count", "$.profile")?;
    let wire_count = integer_field(profile, "wire_count", "$.profile")?;
    let grid_frequency = number_field(profile, "grid_frequency_hz", "$.profile")?;
    let nominal_voltage = number_field(profile, "nominal_voltage_v", "$.profile")?;
    if ![1, 3].contains(&phase_count)
        || ![2, 3, 4].contains(&wire_count)
        || grid_frequency <= 0.0
        || nominal_voltage <= 0.0
    {
        return Err(fail(
            "$.profile",
            "invalid phase count, wire count, grid frequency or nominal voltage",
        ));
    }
    if (phase_count == 1 && ![2, 3].contains(&wire_count))
        || (phase_count == 3 && ![3, 4].contains(&wire_count))
    {
        return Err(fail(
            "$.profile.wire_count",
            "wire count is inconsistent with the declared phase count",
        ));
    }
    if ![50.0, 60.0].contains(&grid_frequency) {
        return Err(fail(
            "$.profile.grid_frequency_hz",
            "v0 supports 50 Hz or 60 Hz profiles",
        ));
    }
    let rated_input_current =
        nullable_number(profile, "rated_input_current_a_per_phase", "$.profile")?;
    let pcc = field(profile, "pcc", "$.profile")?;

    validate_measurement(run, &standard_id)?;

    let applicability = match standard_id.as_str() {
        "IEEE_519_2022" => {
            if edition != "IEEE 519-2022" || role != "SYSTEM_PCC_LIMITS" {
                return Err(fail("$.profile", "IEEE 519 edition/role mismatch"));
            }
            if boundary != "SITE_PCC" {
                ProfileApplicability::NotApplicable
            } else {
                let pcc_value = field(profile, "pcc", "$.profile")?;
                exact_keys(
                    pcc_value,
                    &[
                        "pcc_id",
                        "voltage_kv",
                        "i_sc_rms_a",
                        "i_sc_method_id",
                        "i_sc_source_artifact_sha256",
                        "i_l_rms_a",
                        "i_l_definition_id",
                        "i_l_window_seconds",
                        "i_l_reference_period_id",
                        "i_l_source_artifact_sha256",
                    ],
                    "$.profile.pcc",
                )?;
                string_field(pcc_value, "pcc_id", "$.profile.pcc")?;
                let voltage_kv = number_field(pcc_value, "voltage_kv", "$.profile.pcc")?;
                let isc = number_field(pcc_value, "i_sc_rms_a", "$.profile.pcc")?;
                let il = number_field(pcc_value, "i_l_rms_a", "$.profile.pcc")?;
                string_field(pcc_value, "i_sc_method_id", "$.profile.pcc")?;
                string_field(pcc_value, "i_l_definition_id", "$.profile.pcc")?;
                string_field(pcc_value, "i_l_reference_period_id", "$.profile.pcc")?;
                let isc_digest =
                    string_field(pcc_value, "i_sc_source_artifact_sha256", "$.profile.pcc")?;
                let il_digest =
                    string_field(pcc_value, "i_l_source_artifact_sha256", "$.profile.pcc")?;
                if !valid_sha256(isc_digest) || !valid_sha256(il_digest) {
                    return Err(fail(
                        "$.profile.pcc",
                        "Isc and IL source artifacts must be lowercase SHA-256 digests",
                    ));
                }
                let il_window = number_field(pcc_value, "i_l_window_seconds", "$.profile.pcc")?;
                if voltage_kv <= 0.0 || isc <= 0.0 || il <= 0.0 || il_window <= 0.0 {
                    return Err(fail(
                        "$.profile.pcc",
                        "PCC voltage, Isc, IL and IL averaging window must be positive",
                    ));
                }
                ProfileApplicability::ApplicableScopeOnly
            }
        }
        "IEC_61000_3_12_2011_AMD1_2021" => {
            if edition != "IEC 61000-3-12:2011+AMD1:2021" || role != "PRODUCT_EMISSION_LIMITS" {
                return Err(fail("$.profile", "IEC 61000-3-12 edition/role mismatch"));
            }
            if boundary != "EQUIPMENT_TERMINAL" || !pcc.is_null() {
                ProfileApplicability::NotApplicable
            } else {
                let in_current_scope =
                    rated_input_current.is_some_and(|current| current > 16.0 && current <= 75.0);
                let in_voltage_scope = match phase_count {
                    1 => nominal_voltage <= 240.0,
                    3 => nominal_voltage <= 690.0,
                    _ => false,
                };
                if network_type != "PUBLIC_LOW_VOLTAGE" || !in_current_scope || !in_voltage_scope {
                    ProfileApplicability::NotApplicable
                } else if (phase_count == 1 && (grid_frequency != 50.0 || nominal_voltage != 230.0))
                    || (phase_count == 3 && (grid_frequency != 50.0 || nominal_voltage != 400.0))
                {
                    ProfileApplicability::ScopeOnlyLimitDomainNotCovered
                } else {
                    ProfileApplicability::ApplicableScopeOnly
                }
            }
        }
        "IEC_61000_4_30_2025" => {
            if edition != "IEC 61000-4-30:2025 COR1:2026" || role != "MEASUREMENT_METHOD_ONLY" {
                return Err(fail("$.profile", "IEC 61000-4-30 edition/role mismatch"));
            }
            ProfileApplicability::MeasurementMethodOnly
        }
        _ => return Err(fail("$.profile.standard_id", "unknown standard profile")),
    };

    let spectrum = field(run, "current_spectrum", "$")?;
    exact_keys(
        spectrum,
        &["fundamental_rms_a", "maximum_harmonic_order", "harmonics"],
        "$.current_spectrum",
    )?;
    let maximum_harmonic_order =
        integer_field(spectrum, "maximum_harmonic_order", "$.current_spectrum")?;
    if !(2..=50).contains(&maximum_harmonic_order) {
        return Err(fail(
            "$.current_spectrum.maximum_harmonic_order",
            "v0 requires complete harmonic coverage from order 2 through a maximum of 50",
        ));
    }
    let fundamental = number_field(spectrum, "fundamental_rms_a", "$.current_spectrum")?;
    if fundamental <= 0.0 {
        return Err(fail(
            "$.current_spectrum.fundamental_rms_a",
            "must be positive",
        ));
    }
    let harmonic_lines = field(spectrum, "harmonics", "$.current_spectrum")?
        .as_array()
        .ok_or_else(|| fail("$.current_spectrum.harmonics", "must be an array"))?;
    let mut seen_orders = BTreeSet::new();
    let mut previous_order = None::<u64>;
    let mut harmonic_currents = Vec::with_capacity(harmonic_lines.len());
    for (index, line) in harmonic_lines.iter().enumerate() {
        let path = format!("$.current_spectrum.harmonics[{index}]");
        exact_keys(
            line,
            &["harmonic_order", "rms_current_a", "expanded_uncertainty_a"],
            &path,
        )?;
        let order = integer_field(line, "harmonic_order", &path)?;
        if order < 2 || order > maximum_harmonic_order || !seen_orders.insert(order) {
            return Err(fail(
                &path,
                "harmonic orders must be unique integers within the declared coverage",
            ));
        }
        if previous_order.is_some_and(|previous| order <= previous) {
            return Err(fail(&path, "harmonic orders must be strictly increasing"));
        }
        previous_order = Some(order);
        let current = number_field(line, "rms_current_a", &path)?;
        let uncertainty = number_field(line, "expanded_uncertainty_a", &path)?;
        if current < 0.0 || uncertainty < 0.0 {
            return Err(fail(
                &path,
                "harmonic current and uncertainty must be non-negative",
            ));
        }
        harmonic_currents.push(current);
    }
    let expected_orders = (2..=maximum_harmonic_order).collect::<BTreeSet<_>>();
    if seen_orders != expected_orders {
        return Err(fail(
            "$.current_spectrum.harmonics",
            format!(
                "spectrum must include every integer harmonic order from 2 through {maximum_harmonic_order}; represent a measured zero explicitly"
            ),
        ));
    }

    let thd = thd_percent(&harmonic_currents, fundamental)?;
    if standard_id == "IEEE_519_2022"
        && applicability == ProfileApplicability::ApplicableScopeOnly
        && maximum_harmonic_order != 50
    {
        return Err(fail(
            "$.current_spectrum.maximum_harmonic_order",
            "IEEE 519-2022 PCC TDD profile requires complete current-harmonic coverage through order 50",
        ));
    }
    let (tdd, ratio) = if standard_id == "IEEE_519_2022"
        && applicability == ProfileApplicability::ApplicableScopeOnly
    {
        let pcc = field(profile, "pcc", "$.profile")?;
        let il = number_field(pcc, "i_l_rms_a", "$.profile.pcc")?;
        let isc = number_field(pcc, "i_sc_rms_a", "$.profile.pcc")?;
        let ratio = isc / il;
        if !ratio.is_finite() {
            return Err(fail("$.profile.pcc", "Isc/IL ratio overflowed"));
        }
        (Some(tdd_percent(&harmonic_currents, il)?), Some(ratio))
    } else {
        (None, None)
    };

    Ok(QualityReport {
        run_id,
        standard_id,
        applicability,
        maximum_harmonic_order,
        thd_through_order_percent: thd,
        tdd_through_order_percent: tdd,
        short_circuit_ratio_isc_over_il: ratio,
        harmonic_components: harmonic_currents.len(),
        claim_ceiling: "Profile scope and point metrics only. No normative limit tables are loaded and no standards-compliance decision is made.",
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-pcc-quality-v0/pcc-quality-run.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic PCC fixture JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    #[test]
    fn computes_thd_tdd_and_isc_over_il_from_synthetic_pcc_fixture() {
        let report = evaluate_run(&fixture()).unwrap();
        assert_eq!(
            report.applicability,
            ProfileApplicability::ApplicableScopeOnly
        );
        assert!((report.thd_through_order_percent - 5.5).abs() < 1e-12);
        assert!((report.tdd_through_order_percent.unwrap() - 5.5).abs() < 1e-12);
        assert!((report.short_circuit_ratio_isc_over_il.unwrap() - 250.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_missing_harmonic_bin_within_declared_thd_coverage() {
        let mut run = fixture();
        run["current_spectrum"]["harmonics"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("must include every integer harmonic order from 2 through 50")
        );
    }

    #[test]
    fn ieee_519_tdd_rejects_partial_harmonic_coverage() {
        let mut run = fixture();
        run["current_spectrum"]["maximum_harmonic_order"] = Value::from(40);
        run["current_spectrum"]["harmonics"]
            .as_array_mut()
            .unwrap()
            .truncate(39);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("requires complete current-harmonic coverage through order 50")
        );
    }

    #[test]
    fn rejects_undeclared_harmonic_order_above_coverage() {
        let mut run = fixture();
        run["current_spectrum"]["maximum_harmonic_order"] = Value::from(40);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("unique integers within the declared coverage")
        );
    }

    #[test]
    fn same_spectrum_different_il_changes_tdd_but_not_thd() {
        let mut run = fixture();
        run["profile"]["pcc"]["i_l_rms_a"] = Value::from(50.0);
        let report = evaluate_run(&run).unwrap();
        assert!((report.thd_through_order_percent - 5.5).abs() < 1e-12);
        assert!((report.tdd_through_order_percent.unwrap() - 11.0).abs() < 1e-12);
        assert!((report.short_circuit_ratio_isc_over_il.unwrap() - 500.0).abs() < 1e-12);
    }

    #[test]
    fn ieee_519_rejects_missing_il_source_artifact_digest() {
        let mut run = fixture();
        run["profile"]["pcc"]["i_l_source_artifact_sha256"] = Value::String("bad".into());
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("source artifacts must be lowercase SHA-256 digests")
        );
    }

    #[test]
    fn ieee_519_rejects_zero_il_demand_window() {
        let mut run = fixture();
        run["profile"]["pcc"]["i_l_window_seconds"] = Value::from(0.0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("IL averaging window must be positive")
        );
    }

    #[test]
    fn ieee_519_equipment_terminal_result_is_not_a_pcc_result() {
        let mut run = fixture();
        run["profile"]["assessment_boundary"] = Value::String("EQUIPMENT_TERMINAL".into());
        run["profile"]["pcc"] = Value::Null;
        let report = evaluate_run(&run).unwrap();
        assert_eq!(report.applicability, ProfileApplicability::NotApplicable);
        assert!(report.tdd_through_order_percent.is_none());
    }

    #[test]
    fn ieee_519_requires_pcc_data_for_pcc_claim() {
        let mut run = fixture();
        run["profile"]["pcc"] = Value::Null;
        assert!(evaluate_run(&run).is_err());
    }

    #[test]
    fn iec_61000_3_12_30a_public_lv_equipment_is_scope_applicable_not_certified() {
        let mut run = fixture();
        run["profile"]["standard_id"] = Value::String("IEC_61000_3_12_2011_AMD1_2021".into());
        run["profile"]["edition"] = Value::String("IEC 61000-3-12:2011+AMD1:2021".into());
        run["profile"]["standard_role"] = Value::String("PRODUCT_EMISSION_LIMITS".into());
        run["profile"]["assessment_boundary"] = Value::String("EQUIPMENT_TERMINAL".into());
        run["profile"]["rated_input_current_a_per_phase"] = Value::from(30.0);
        run["profile"]["pcc"] = Value::Null;
        run["measurement"]["measurement_standard_id"] =
            Value::String("IEC_61000_4_7_2002_AMD1_2008".into());
        run["measurement"]["measurement_edition"] =
            Value::String("IEC 61000-4-7:2002+AMD1:2008".into());
        run["measurement"]["measurement_class"] = Value::String("EQUIPMENT_TEST".into());
        let report = evaluate_run(&run).unwrap();
        assert_eq!(
            report.applicability,
            ProfileApplicability::ApplicableScopeOnly
        );
        assert!(report.tdd_through_order_percent.is_none());
    }

    #[test]
    fn iec_61000_3_12_80a_is_not_applicable() {
        let mut run = fixture();
        run["profile"]["standard_id"] = Value::String("IEC_61000_3_12_2011_AMD1_2021".into());
        run["profile"]["edition"] = Value::String("IEC 61000-3-12:2011+AMD1:2021".into());
        run["profile"]["standard_role"] = Value::String("PRODUCT_EMISSION_LIMITS".into());
        run["profile"]["assessment_boundary"] = Value::String("EQUIPMENT_TERMINAL".into());
        run["profile"]["rated_input_current_a_per_phase"] = Value::from(80.0);
        run["profile"]["pcc"] = Value::Null;
        run["measurement"]["measurement_standard_id"] =
            Value::String("IEC_61000_4_7_2002_AMD1_2008".into());
        run["measurement"]["measurement_edition"] =
            Value::String("IEC 61000-4-7:2002+AMD1:2008".into());
        run["measurement"]["measurement_class"] = Value::String("EQUIPMENT_TEST".into());
        let report = evaluate_run(&run).unwrap();
        assert_eq!(report.applicability, ProfileApplicability::NotApplicable);
    }

    #[test]
    fn iec_61000_4_30_is_classified_as_measurement_method_only() {
        let mut run = fixture();
        run["profile"]["standard_id"] = Value::String("IEC_61000_4_30_2025".into());
        run["profile"]["edition"] = Value::String("IEC 61000-4-30:2025 COR1:2026".into());
        run["profile"]["standard_role"] = Value::String("MEASUREMENT_METHOD_ONLY".into());
        let report = evaluate_run(&run).unwrap();
        assert_eq!(
            report.applicability,
            ProfileApplicability::MeasurementMethodOnly
        );
        assert!(report.tdd_through_order_percent.is_none());
    }

    #[test]
    fn rejects_standard_edition_mismatch() {
        let mut run = fixture();
        run["profile"]["edition"] = Value::String("IEEE 519-2014".into());
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("edition/role mismatch")
        );
    }

    #[test]
    fn iec_61000_3_12_60hz_public_lv_is_broad_scope_but_limit_domain_not_covered() {
        let mut run = fixture();
        run["profile"]["standard_id"] = Value::String("IEC_61000_3_12_2011_AMD1_2021".into());
        run["profile"]["edition"] = Value::String("IEC 61000-3-12:2011+AMD1:2021".into());
        run["profile"]["standard_role"] = Value::String("PRODUCT_EMISSION_LIMITS".into());
        run["profile"]["assessment_boundary"] = Value::String("EQUIPMENT_TERMINAL".into());
        run["profile"]["rated_input_current_a_per_phase"] = Value::from(30.0);
        run["profile"]["grid_frequency_hz"] = Value::from(60.0);
        run["profile"]["pcc"] = Value::Null;
        run["measurement"]["measurement_standard_id"] =
            Value::String("IEC_61000_4_7_2002_AMD1_2008".into());
        run["measurement"]["measurement_edition"] =
            Value::String("IEC 61000-4-7:2002+AMD1:2008".into());
        run["measurement"]["measurement_class"] = Value::String("EQUIPMENT_TEST".into());
        let report = evaluate_run(&run).unwrap();
        assert_eq!(
            report.applicability,
            ProfileApplicability::ScopeOnlyLimitDomainNotCovered
        );
    }

    #[test]
    fn iec_61000_3_12_480v_public_lv_is_not_in_this_editions_limit_domain() {
        let mut run = fixture();
        run["profile"]["standard_id"] = Value::String("IEC_61000_3_12_2011_AMD1_2021".into());
        run["profile"]["edition"] = Value::String("IEC 61000-3-12:2011+AMD1:2021".into());
        run["profile"]["standard_role"] = Value::String("PRODUCT_EMISSION_LIMITS".into());
        run["profile"]["assessment_boundary"] = Value::String("EQUIPMENT_TERMINAL".into());
        run["profile"]["rated_input_current_a_per_phase"] = Value::from(30.0);
        run["profile"]["nominal_voltage_v"] = Value::from(480.0);
        run["profile"]["pcc"] = Value::Null;
        run["measurement"]["measurement_standard_id"] =
            Value::String("IEC_61000_4_7_2002_AMD1_2008".into());
        run["measurement"]["measurement_edition"] =
            Value::String("IEC 61000-4-7:2002+AMD1:2008".into());
        run["measurement"]["measurement_class"] = Value::String("EQUIPMENT_TEST".into());
        let report = evaluate_run(&run).unwrap();
        assert_eq!(
            report.applicability,
            ProfileApplicability::ScopeOnlyLimitDomainNotCovered
        );
    }

    #[test]
    fn iec_61000_3_12_single_phase_230v_50hz_is_limit_domain_applicable() {
        let mut run = fixture();
        run["profile"]["standard_id"] = Value::String("IEC_61000_3_12_2011_AMD1_2021".into());
        run["profile"]["edition"] = Value::String("IEC 61000-3-12:2011+AMD1:2021".into());
        run["profile"]["standard_role"] = Value::String("PRODUCT_EMISSION_LIMITS".into());
        run["profile"]["assessment_boundary"] = Value::String("EQUIPMENT_TERMINAL".into());
        run["profile"]["phase_count"] = Value::from(1);
        run["profile"]["wire_count"] = Value::from(2);
        run["profile"]["nominal_voltage_v"] = Value::from(230.0);
        run["profile"]["rated_input_current_a_per_phase"] = Value::from(30.0);
        run["profile"]["pcc"] = Value::Null;
        run["measurement"]["measurement_standard_id"] =
            Value::String("IEC_61000_4_7_2002_AMD1_2008".into());
        run["measurement"]["measurement_edition"] =
            Value::String("IEC 61000-4-7:2002+AMD1:2008".into());
        run["measurement"]["measurement_class"] = Value::String("EQUIPMENT_TEST".into());
        let report = evaluate_run(&run).unwrap();
        assert_eq!(
            report.applicability,
            ProfileApplicability::ApplicableScopeOnly
        );
    }

    #[test]
    fn rejects_phase_and_wire_count_mismatch() {
        let mut run = fixture();
        run["profile"]["wire_count"] = Value::from(2);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("wire count is inconsistent with the declared phase count")
        );
    }

    #[test]
    fn iec_61000_3_12_16a_boundary_is_not_applicable() {
        let mut run = fixture();
        run["profile"]["standard_id"] = Value::String("IEC_61000_3_12_2011_AMD1_2021".into());
        run["profile"]["edition"] = Value::String("IEC 61000-3-12:2011+AMD1:2021".into());
        run["profile"]["standard_role"] = Value::String("PRODUCT_EMISSION_LIMITS".into());
        run["profile"]["assessment_boundary"] = Value::String("EQUIPMENT_TERMINAL".into());
        run["profile"]["rated_input_current_a_per_phase"] = Value::from(16.0);
        run["profile"]["pcc"] = Value::Null;
        run["measurement"]["measurement_standard_id"] =
            Value::String("IEC_61000_4_7_2002_AMD1_2008".into());
        run["measurement"]["measurement_edition"] =
            Value::String("IEC 61000-4-7:2002+AMD1:2008".into());
        run["measurement"]["measurement_class"] = Value::String("EQUIPMENT_TEST".into());
        let report = evaluate_run(&run).unwrap();
        assert_eq!(report.applicability, ProfileApplicability::NotApplicable);
    }

    #[test]
    fn rejects_pcc_measurement_method_edition_mismatch() {
        let mut run = fixture();
        run["measurement"]["measurement_edition"] = Value::String("IEC 61000-4-30:2015".into());
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("requires IEC 61000-4-30:2025 COR1:2026")
        );
    }

    #[test]
    fn rejects_unsorted_harmonic_orders() {
        let mut run = fixture();
        run["current_spectrum"]["harmonics"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("strictly increasing")
        );
    }

    #[test]
    fn tdd_rejects_zero_or_non_finite_reference_current() {
        assert!(tdd_percent(&[1.0, 2.0], 0.0).is_err());
        assert!(tdd_percent(&[1.0], f64::NAN).is_err());
    }

    #[test]
    fn tdd_rejects_harmonic_accumulation_overflow() {
        assert!(tdd_percent(&[1.0e308], 100.0).is_err());
    }

    #[test]
    fn rejects_duplicate_harmonic_orders() {
        let mut run = fixture();
        run["current_spectrum"]["harmonics"][1]["harmonic_order"] = Value::from(2);
        assert!(evaluate_run(&run).unwrap_err().contains("unique integers"));
    }

    #[test]
    fn rejects_negative_harmonic_currents() {
        let mut run = fixture();
        run["current_spectrum"]["harmonics"][0]["rms_current_a"] = Value::from(-1.0);
        assert!(evaluate_run(&run).unwrap_err().contains("non-negative"));
    }
}
