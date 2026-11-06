//! Evidence-bounded fragmentation-to-transport validation.
//!
//! The crate validates measured interval records and computes descriptive
//! mass-balance, particle-size closure, output-rate and installed-stage capacity
//! comparisons. It is not a fluid/particle solver, blockage classifier, safety
//! authority or machine-control interface.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const STAGES: [&str; 4] = ["CAPTURE", "CONVEY", "SEPARATE", "DISCHARGE"];

#[derive(Clone, Debug, PartialEq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntervalReport {
    pub interval_id: String,
    pub generated_mass_kg: f64,
    pub recovered_mass_kg: f64,
    pub residual_mass_kg: f64,
    pub rejected_mass_kg: f64,
    pub mass_balance_residual_kg: f64,
    pub mass_balance_uncertainty_envelope_kg: f64,
    pub mass_balance_within_uncertainty: bool,
    pub fragment_bins_total_kg: f64,
    pub fragment_bins_within_uncertainty: bool,
    pub extraction_efficiency: f64,
    pub recovered_rate_kg_s: f64,
    pub generation_rate_lower_kg_s: f64,
    pub generation_rate_upper_kg_s: f64,
    pub capacity_disposition: String,
    pub definite_bottleneck_stages: Vec<String>,
    pub possible_bottleneck_stages: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub evidence_class: String,
    pub interval_count: usize,
    pub accounting_disposition: String,
    pub interval_reports: Vec<IntervalReport>,
    pub warnings: Vec<String>,
}

fn push(
    errors: &mut Vec<Violation>,
    code: &str,
    path: &str,
    detail: impl Into<String>,
) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn non_empty<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn millis(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn independent_role(role: Option<&str>) -> bool {
    matches!(
        role,
        Some("INDEPENDENT_MEASUREMENT" | "DERIVED_FROM_INDEPENDENT_MEASUREMENTS")
    )
}

fn check_measurement(
    value: &Value,
    path: &str,
    errors: &mut Vec<Violation>,
) -> Option<(f64, f64)> {
    let amount = finite(value, "value_kg");
    let uncertainty = finite(value, "standard_uncertainty_kg");

    match amount {
        Some(v) if v >= 0.0 => {}
        _ => push(
            errors,
            "INVALID_MASS_VALUE",
            &format!("{path}.value_kg"),
            "mass must be finite and non-negative",
        ),
    }
    match uncertainty {
        Some(v) if v >= 0.0 => {}
        _ => push(
            errors,
            "INVALID_MASS_UNCERTAINTY",
            &format!("{path}.standard_uncertainty_kg"),
            "standard uncertainty must be finite and non-negative",
        ),
    }

    if !independent_role(non_empty(value, "evidence_role")) {
        push(
            errors,
            "NON_INDEPENDENT_MASS_EVIDENCE",
            &format!("{path}.evidence_role"),
            "primary mass observations require independent measurement or a declared derivation from independent measurements",
        );
    }
    for key in ["calibration_ref", "source_ref"] {
        if non_empty(value, key).is_none() {
            push(
                errors,
                "MISSING_MASS_PROVENANCE",
                &format!("{path}.{key}"),
                "non-empty calibration/source reference required",
            );
        }
    }
    if non_empty(value, "evidence_role")
        == Some("DERIVED_FROM_INDEPENDENT_MEASUREMENTS")
        && non_empty(value, "lineage_ref").is_none()
    {
        push(
            errors,
            "MISSING_MASS_LINEAGE",
            &format!("{path}.lineage_ref"),
            "derived mass requires upstream measurement lineage",
        );
    }

    match (amount, uncertainty) {
        (Some(v), Some(u)) if v >= 0.0 && u >= 0.0 => Some((v, u)),
        _ => None,
    }
}

fn check_required_measurement(
    parent: &Value,
    key: &str,
    path: &str,
    errors: &mut Vec<Violation>,
) -> Option<(f64, f64)> {
    match parent.get(key) {
        Some(value) => check_measurement(value, &format!("{path}.{key}"), errors),
        None => {
            push(
                errors,
                "MISSING_MASS_MEASUREMENT",
                &format!("{path}.{key}"),
                "required mass measurement is missing",
            );
            None
        }
    }
}

fn check_fragment_bins(
    interval: &Value,
    path: &str,
    generated_mass: Option<(f64, f64)>,
    errors: &mut Vec<Violation>,
) -> Option<(f64, f64, bool)> {
    let Some(bins) = interval.get("fragment_bins").and_then(Value::as_array) else {
        push(
            errors,
            "MISSING_FRAGMENT_BINS",
            &format!("{path}.fragment_bins"),
            "fragment_bins must be an array",
        );
        return None;
    };
    if bins.is_empty() {
        push(
            errors,
            "EMPTY_FRAGMENT_BINS",
            &format!("{path}.fragment_bins"),
            "at least one non-overlapping size bin is required",
        );
        return None;
    }

    let mut total = 0.0;
    let mut uncertainty_total = 0.0;
    let mut previous_upper: Option<f64> = None;
    let mut all_valid = true;

    for (index, bin) in bins.iter().enumerate() {
        let bin_path = format!("{path}.fragment_bins[{index}]");
        let lower = finite(bin, "lower_mm");
        let upper_value = bin.get("upper_mm");
        if upper_value.is_none() {
            all_valid = false;
            push(
                errors,
                "MISSING_PSD_UPPER_BOUND",
                &format!("{bin_path}.upper_mm"),
                "upper_mm is required; use null only for the final open-ended bin",
            );
        }
        let upper = match upper_value {
            Some(Value::Null) => None,
            Some(_) => finite(bin, "upper_mm"),
            None => None,
        };
        let (mass, uncertainty) = match (
            finite(bin, "mass_kg"),
            finite(bin, "standard_uncertainty_kg"),
        ) {
            (Some(m), Some(u)) if m >= 0.0 && u >= 0.0 => (Some(m), Some(u)),
            _ => {
                all_valid = false;
                push(
                    errors,
                    "INVALID_FRAGMENT_BIN_MASS",
                    &format!("{bin_path}.mass_kg"),
                    "bin mass and uncertainty must be finite and non-negative",
                );
                (None, None)
            }
        };

        if !independent_role(non_empty(bin, "evidence_role")) {
            all_valid = false;
            push(
                errors,
                "NON_INDEPENDENT_PSD_EVIDENCE",
                &format!("{bin_path}.evidence_role"),
                "particle-size mass bins require independent measurement or declared derivation from independent measurements",
            );
        }
        for key in ["calibration_ref", "source_ref"] {
            if non_empty(bin, key).is_none() {
                all_valid = false;
                push(
                    errors,
                    "MISSING_PSD_PROVENANCE",
                    &format!("{bin_path}.{key}"),
                    "non-empty calibration/source reference required",
                );
            }
        }
        if non_empty(bin, "evidence_role")
            == Some("DERIVED_FROM_INDEPENDENT_MEASUREMENTS")
            && non_empty(bin, "lineage_ref").is_none()
        {
            all_valid = false;
            push(
                errors,
                "MISSING_PSD_LINEAGE",
                &format!("{bin_path}.lineage_ref"),
                "derived particle-size mass requires upstream lineage",
            );
        }

        match (lower, upper) {
            (Some(lo), Some(hi)) if lo >= 0.0 && hi > lo => {
                if index == 0 {
                    if !close(lo, 0.0) {
                        all_valid = false;
                        push(
                            errors,
                            "PSD_FIRST_BIN_NOT_ZERO",
                            &format!("{bin_path}.lower_mm"),
                            "first size bin must start at 0 mm",
                        );
                    }
                } else if previous_upper.is_none_or(|prev| !close(prev, lo)) {
                    all_valid = false;
                    push(
                        errors,
                        "PSD_BIN_GAP_OR_OVERLAP",
                        &format!("{bin_path}.lower_mm"),
                        "bins must be ordered, contiguous and non-overlapping",
                    );
                }
                previous_upper = Some(hi);
                if index + 1 == bins.len() {
                    all_valid = false;
                    push(
                        errors,
                        "PSD_LAST_BIN_NOT_OPEN",
                        &format!("{bin_path}.upper_mm"),
                        "last bin must be open-ended with upper_mm = null",
                    );
                }
            }
            (Some(lo), None) if lo >= 0.0 && index + 1 == bins.len() => {
                if index == 0 {
                    if !close(lo, 0.0) {
                        all_valid = false;
                        push(
                            errors,
                            "PSD_FIRST_BIN_NOT_ZERO",
                            &format!("{bin_path}.lower_mm"),
                            "first size bin must start at 0 mm",
                        );
                    }
                } else if previous_upper.is_none_or(|prev| !close(prev, lo)) {
                    all_valid = false;
                    push(
                        errors,
                        "PSD_BIN_GAP_OR_OVERLAP",
                        &format!("{bin_path}.lower_mm"),
                        "bins must be ordered, contiguous and non-overlapping",
                    );
                }
                previous_upper = None;
            }
            _ => {
                all_valid = false;
                push(
                    errors,
                    "INVALID_PSD_BIN_BOUNDARY",
                    &bin_path,
                    "finite bins need upper_mm > lower_mm; only the final bin may use upper_mm = null",
                );
            }
        }

        if let Some(m) = mass {
            total += m;
        }
        if let Some(u) = uncertainty {
            uncertainty_total += u;
        }
    }

    if previous_upper.is_some() {
        all_valid = false;
        push(
            errors,
            "PSD_LAST_BIN_NOT_OPEN",
            &format!("{path}.fragment_bins"),
            "the final bin must terminate at infinity",
        );
    }

    if !all_valid {
        return None;
    }

    let (generated, generated_uncertainty) = generated_mass?;
    let within = (total - generated).abs() <= uncertainty_total + generated_uncertainty;
    Some((total, uncertainty_total, within))
}

fn validate_stage_capacities(
    interval: &Value,
    path: &str,
    transport_configuration_ref: &str,
    transport_capacity_profile_ref: &str,
    stage_order: &[String],
    errors: &mut Vec<Violation>,
) -> Option<Vec<(String, f64, f64)>> {
    let Some(stages) = interval.get("stage_capacities").and_then(Value::as_array) else {
        push(
            errors,
            "MISSING_STAGE_CAPACITIES",
            &format!("{path}.stage_capacities"),
            "stage_capacities must be an array",
        );
        return None;
    };
    if stages.len() != STAGES.len() {
        push(
            errors,
            "STAGE_COUNT_MISMATCH",
            &format!("{path}.stage_capacities"),
            "exactly one capacity interval for capture, convey, separate and discharge is required",
        );
    }

    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for (index, stage) in stages.iter().enumerate() {
        let stage_path = format!("{path}.stage_capacities[{index}]");
        let Some(name) = non_empty(stage, "stage") else {
            push(
                errors,
                "MISSING_STAGE_ID",
                &format!("{stage_path}.stage"),
                "stage is required",
            );
            continue;
        };
        if !STAGES.contains(&name) {
            push(
                errors,
                "UNKNOWN_STAGE",
                &format!("{stage_path}.stage"),
                format!("unknown stage {name}"),
            );
            continue;
        }
        if !seen.insert(name.to_owned()) {
            push(
                errors,
                "DUPLICATE_STAGE",
                &format!("{stage_path}.stage"),
                format!("duplicate stage {name}"),
            );
        }

        if non_empty(stage, "transport_configuration_ref") != Some(transport_configuration_ref) {
            push(
                errors,
                "INSTALLED_CONFIGURATION_MISMATCH",
                &format!("{stage_path}.transport_configuration_ref"),
                "capacity evidence must match the exact installed transport configuration, not a factory or nameplate reference",
            );
        }
        if non_empty(stage, "transport_capacity_profile_ref") != Some(transport_capacity_profile_ref) {
            push(
                errors,
                "CAPACITY_PROFILE_MISMATCH",
                &format!("{stage_path}.transport_capacity_profile_ref"),
                "stage capacity evidence must bind to the declared transport capacity profile",
            );
        }
        for key in ["capacity_window_ref", "calibration_ref", "source_ref"] {
            if non_empty(stage, key).is_none() {
                push(
                    errors,
                    "MISSING_CAPACITY_PROVENANCE",
                    &format!("{stage_path}.{key}"),
                    "capacity requires window, calibration and source references",
                );
            }
        }
        if !independent_role(non_empty(stage, "evidence_role")) {
            push(
                errors,
                "NON_INDEPENDENT_CAPACITY_EVIDENCE",
                &format!("{stage_path}.evidence_role"),
                "installed capacity bounds require independent measurement or a declared derivation from independent measurements; model and nameplate claims are not accepted",
            );
        }
        if non_empty(stage, "evidence_role")
            == Some("DERIVED_FROM_INDEPENDENT_MEASUREMENTS")
            && non_empty(stage, "lineage_ref").is_none()
        {
            push(
                errors,
                "MISSING_CAPACITY_LINEAGE",
                &format!("{stage_path}.lineage_ref"),
                "derived capacity bounds require upstream measurement lineage",
            );
        }

        let lower = finite(stage, "capacity_lower_kg_s");
        let upper = finite(stage, "capacity_upper_kg_s");
        match (lower, upper) {
            (Some(lo), Some(hi)) if lo >= 0.0 && hi >= lo => {
                result.push((name.to_owned(), lo, hi));
            }
            _ => push(
                errors,
                "INVALID_CAPACITY_INTERVAL",
                &stage_path,
                "capacity interval must be finite, non-negative and lower <= upper",
            ),
        }
    }

    for expected in STAGES {
        if !seen.contains(expected) {
            push(
                errors,
                "MISSING_STAGE",
                &format!("{path}.stage_capacities"),
                format!("required stage {expected} is missing"),
            );
        }
    }

    let actual_order = stages
        .iter()
        .filter_map(|stage| non_empty(stage, "stage").map(str::to_owned))
        .collect::<Vec<_>>();
    if actual_order.len() != stage_order.len()
        || actual_order.iter().zip(stage_order).any(|(actual, expected)| actual != expected)
    {
        push(
            errors,
            "STAGE_ORDER_MISMATCH",
            &format!("{path}.stage_capacities"),
            "stage records must follow the profile's declared ordered stages",
        );
    }
    Some(result)
}

pub fn evaluate_json(source: &str) -> Result<EvaluationReport, Vec<Violation>> {
    let value: Value = serde_json::from_str(source).map_err(|error| {
        vec![Violation {
            code: "INVALID_JSON".to_owned(),
            path: "$".to_owned(),
            detail: error.to_string(),
        }]
    })?;
    evaluate(&value)
}

pub fn evaluate(value: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if !value.is_object() {
        return Err(vec![Violation {
            code: "ROOT_NOT_OBJECT".to_owned(),
            path: "$".to_owned(),
            detail: "case root must be a JSON object".to_owned(),
        }]);
    }
    if value.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut errors, "UNSUPPORTED_SCHEMA", "schema_version", "schema_version must equal 1");
    }
    let case_id = non_empty(value, "case_id").map(str::to_owned);
    if case_id.is_none() {
        push(&mut errors, "MISSING_CASE_ID", "case_id", "case_id must be non-empty");
    }
    let evidence_class = non_empty(value, "evidence_class").unwrap_or("");
    match evidence_class {
        "SYNTHETIC_CONFORMANCE_FIXTURE" => {
            warnings.push("synthetic conformance fixture: no physical performance or equipment qualification claim is supported".to_owned());
        }
        "MEASURED_CAMPAIGN" => {}
        _ => push(
            &mut errors,
            "INVALID_EVIDENCE_CLASS",
            "evidence_class",
            "evidence_class must be SYNTHETIC_CONFORMANCE_FIXTURE or MEASURED_CAMPAIGN",
        ),
    }

    let Some(profile) = value.get("profile") else {
        push(&mut errors, "MISSING_PROFILE", "profile", "profile is required");
        return Err(errors);
    };
    let profile_fields = [
        "profile_id",
        "task_profile_ref",
        "material_cohort_ref",
        "acceptance_profile_ref",
        "site_context_ref",
        "machine_configuration_ref",
        "transport_configuration_ref",
        "transport_capacity_profile_ref",
        "energy_boundary_ref",
        "resource_ledger_ref",
        "clock_sync_protocol_ref",
    ];
    for key in profile_fields {
        if non_empty(profile, key).is_none() {
            push(
                &mut errors,
                "MISSING_PROFILE_REFERENCE",
                &format!("profile.{key}"),
                "non-empty reference required",
            );
        }
    }
    if non_empty(profile, "mass_basis") != Some("DRY_SOLID_MASS") {
        push(
            &mut errors,
            "INVALID_MASS_BASIS",
            "profile.mass_basis",
            "v0.1 requires DRY_SOLID_MASS; wet and loose-spoil bases are separate measures",
        );
    }

    let declared_stage_order = profile
        .get("stage_order")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if declared_stage_order.len() != STAGES.len()
        || declared_stage_order.iter().zip(STAGES).any(|(actual, expected)| actual.as_str() != expected)
    {
        push(
            &mut errors,
            "INVALID_PROFILE_STAGE_ORDER",
            "profile.stage_order",
            "stage order must be exactly CAPTURE, CONVEY, SEPARATE, DISCHARGE",
        );
    }

    let transport_configuration_ref =
        non_empty(profile, "transport_configuration_ref").unwrap_or("");
    let Some(intervals) = value.get("intervals").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_INTERVALS", "intervals", "intervals must be an array");
        return Err(errors);
    };
    if intervals.is_empty() {
        push(&mut errors, "EMPTY_INTERVALS", "intervals", "at least one interval is required");
        return Err(errors);
    }

    let mut interval_ids = BTreeSet::new();
    let mut trial_windows: BTreeMap<String, Vec<(u64, u64)>> = BTreeMap::new();
    let mut reports = Vec::new();
    let mut any_indeterminate_accounting = false;

    for (index, interval) in intervals.iter().enumerate() {
        let path = format!("intervals[{index}]");
        let interval_id = non_empty(interval, "interval_id").map(str::to_owned);
        let trial_id = non_empty(interval, "trial_id");
        if let Some(id) = interval_id.as_deref() {
            if !interval_ids.insert(id.to_owned()) {
                push(
                    &mut errors,
                    "DUPLICATE_INTERVAL_ID",
                    &format!("{path}.interval_id"),
                    format!("duplicate interval_id {id}"),
                );
            }
        } else {
            push(
                &mut errors,
                "MISSING_INTERVAL_ID",
                &format!("{path}.interval_id"),
                "interval_id must be non-empty",
            );
        }
        if trial_id.is_none() {
            push(&mut errors, "MISSING_TRIAL_ID", &format!("{path}.trial_id"), "trial_id is required");
        }
        if non_empty(interval, "configuration_ref")
            != non_empty(profile, "machine_configuration_ref")
        {
            push(
                &mut errors,
                "MACHINE_CONFIGURATION_MISMATCH",
                &format!("{path}.configuration_ref"),
                "interval must bind to the profile's machine configuration",
            );
        }
        if non_empty(interval, "clock_sync_ref").is_none() {
            push(
                &mut errors,
                "MISSING_CLOCK_SYNC",
                &format!("{path}.clock_sync_ref"),
                "a synchronized clock reference is required",
            );
        }

        let start = millis(interval, "start_ms");
        let end = millis(interval, "end_ms");
        let time_uncertainty_ms = finite(interval, "time_uncertainty_ms");
        let duration_ms = match (start, end) {
            (Some(s), Some(e)) if e > s => Some((e - s) as f64),
            _ => {
                push(
                    &mut errors,
                    "INVALID_TIME_WINDOW",
                    &format!("{path}.start_ms"),
                    "start_ms and end_ms must be integers with end_ms > start_ms",
                );
                None
            }
        };
        if let (Some(trial), Some(s), Some(e)) = (trial_id, start, end) {
            let prior_windows = trial_windows.entry(trial.to_owned()).or_default();
            if prior_windows.iter().any(|(prior_start, prior_end)| s < *prior_end && *prior_start < e) {
                push(
                    &mut errors,
                    "TIME_WINDOW_OVERLAP",
                    &format!("{path}.start_ms"),
                    "observation windows for the same trial must not overlap; overlapping windows would double-count material flow",
                );
            }
            prior_windows.push((s, e));
        }
        let duration_seconds = duration_ms.map(|d| d / 1000.0);
        let time_uncertainty_seconds = match time_uncertainty_ms {
            Some(u) if u >= 0.0 => Some(u / 1000.0),
            _ => {
                push(
                    &mut errors,
                    "INVALID_TIME_UNCERTAINTY",
                    &format!("{path}.time_uncertainty_ms"),
                    "time uncertainty must be finite and non-negative",
                );
                None
            }
        };
        if let (Some(d), Some(u)) = (duration_seconds, time_uncertainty_seconds) {
            if u >= d {
                push(
                    &mut errors,
                    "TIME_UNCERTAINTY_EXCEEDS_WINDOW",
                    &format!("{path}.time_uncertainty_ms"),
                    "time uncertainty must be smaller than interval duration",
                );
            }
        }

        let generated = check_required_measurement(interval, "generated_mass", &path, &mut errors);
        let recovered = check_required_measurement(interval, "recovered_mass", &path, &mut errors);
        let residual = check_required_measurement(interval, "residual_mass", &path, &mut errors);
        let rejected = check_required_measurement(interval, "rejected_mass", &path, &mut errors);

        let psd = check_fragment_bins(interval, &path, generated, &mut errors);
        let stages = validate_stage_capacities(
            interval,
            &path,
            transport_configuration_ref,
            non_empty(profile, "transport_capacity_profile_ref").unwrap_or(""),
            &declared_stage_order,
            &mut errors,
        );

        if let (Some(g), Some(r), Some(res), Some(rej), Some(duration), Some(time_u), Some(psd), Some(stages)) =
            (generated, recovered, residual, rejected, duration_seconds, time_uncertainty_seconds, psd, stages)
        {
            if g.0 <= 0.0 {
                push(
                    &mut errors,
                    "NON_POSITIVE_GENERATED_MASS",
                    &format!("{path}.generated_mass.value_kg"),
                    "generated mass must be positive for rates and extraction efficiency",
                );
                continue;
            }
            if duration <= 0.0 || time_u >= duration {
                continue;
            }

            let mass_residual = g.0 - r.0 - res.0 - rej.0;
            let mass_uncertainty_envelope = g.1 + r.1 + res.1 + rej.1;
            let mass_balance_ok = mass_residual.abs() <= mass_uncertainty_envelope;
            let psd_ok = psd.2;
            if !mass_balance_ok || !psd_ok {
                any_indeterminate_accounting = true;
            }

            let rate_lower = (g.0 - g.1).max(0.0) / (duration + time_u);
            let rate_upper = (g.0 + g.1) / (duration - time_u);
            let recovered_rate = r.0 / duration;
            let extraction_efficiency = r.0 / g.0;

            let mut definite = Vec::new();
            let mut possible = Vec::new();
            for (stage, cap_lower, cap_upper) in stages {
                if cap_upper < rate_lower {
                    definite.push(stage);
                } else if cap_lower <= rate_upper && cap_upper >= rate_lower {
                    possible.push(stage);
                }
            }
            let capacity_disposition = if !definite.is_empty() {
                "DEFINITE_CAPACITY_BOTTLENECK"
            } else if !possible.is_empty() {
                "CAPACITY_COMPARISON_AMBIGUOUS"
            } else {
                "NO_STAGE_CAPACITY_BOTTLENECK_EVIDENCED"
            };

            reports.push(IntervalReport {
                interval_id: interval_id.unwrap_or_else(|| format!("interval-{index}")),
                generated_mass_kg: g.0,
                recovered_mass_kg: r.0,
                residual_mass_kg: res.0,
                rejected_mass_kg: rej.0,
                mass_balance_residual_kg: mass_residual,
                mass_balance_uncertainty_envelope_kg: mass_uncertainty_envelope,
                mass_balance_within_uncertainty: mass_balance_ok,
                fragment_bins_total_kg: psd.0,
                fragment_bins_within_uncertainty: psd_ok,
                extraction_efficiency,
                recovered_rate_kg_s: recovered_rate,
                generation_rate_lower_kg_s: rate_lower,
                generation_rate_upper_kg_s: rate_upper,
                capacity_disposition: capacity_disposition.to_owned(),
                definite_bottleneck_stages: definite,
                possible_bottleneck_stages: possible,
            });
        } else {
            // The individual validation helpers already recorded the precise reason.
            continue;
        }
    }

    if !errors.is_empty() {
        errors.sort_by(|a, b| (&a.path, &a.code, &a.detail).cmp(&(&b.path, &b.code, &b.detail)));
        errors.dedup();
        return Err(errors);
    }
    if reports.len() != intervals.len() {
        push(
            &mut errors,
            "INCOMPLETE_INTERVAL_REPORT",
            "intervals",
            "not all intervals produced a complete descriptive report",
        );
        return Err(errors);
    }

    let accounting_disposition = if any_indeterminate_accounting {
        "INDETERMINATE_ACCOUNTING"
    } else {
        "MEASURED_ACCOUNTING_CLOSED"
    };
    if evidence_class == "SYNTHETIC_CONFORMANCE_FIXTURE" {
        warnings.push("all numeric observations are fabricated and must not be interpreted as mine or equipment performance".to_owned());
    }
    warnings.push("uncertainty envelope is the conservative sum of stated magnitudes, not a probabilistic coverage interval".to_owned());
    warnings.push("capacity disposition compares measured interval bounds only; it does not identify a real-time control action or establish breakage-limited operation".to_owned());

    Ok(EvaluationReport {
        case_id: case_id.unwrap_or_default(),
        evidence_class: evidence_class.to_owned(),
        interval_count: reports.len(),
        accounting_disposition: accounting_disposition.to_owned(),
        interval_reports: reports,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-fragment-transport-case.v0_1.json"
        ))
        .expect("fixture JSON must parse")
    }

    fn codes(value: &Value) -> Vec<String> {
        evaluate(value)
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.code)
            .collect()
    }

    fn has(value: &Value, code: &str) -> bool {
        codes(value).iter().any(|c| c == code)
    }

    fn interval_mut(value: &mut Value, index: usize) -> &mut Value {
        &mut value["intervals"][index]
    }

    fn stage_mut(value: &mut Value, index: usize, stage: &str) -> &mut Value {
        interval_mut(value, index)["stage_capacities"]
            .as_array_mut()
            .expect("stages")
            .iter_mut()
            .find(|s| s["stage"] == stage)
            .expect("stage")
    }

    #[test]
    fn accepts_synthetic_case_and_replays_two_windows() {
        let report = evaluate(&fixture()).expect("valid fixture");
        assert_eq!(report.interval_count, 2);
        assert_eq!(report.accounting_disposition, "MEASURED_ACCOUNTING_CLOSED");
        assert_eq!(report.interval_reports[0].definite_bottleneck_stages, vec!["CONVEY"]);
        assert_eq!(report.interval_reports[1].capacity_disposition, "CAPACITY_COMPARISON_AMBIGUOUS");
    }

    #[test]
    fn parses_json_entry_point() {
        let source = serde_json::to_string(&fixture()).expect("serialize");
        assert_eq!(evaluate_json(&source).expect("valid").interval_count, 2);
    }

    #[test]
    fn rejects_invalid_json() {
        assert_eq!(evaluate_json("{").expect_err("invalid").first().unwrap().code, "INVALID_JSON");
    }

    #[test]
    fn rejects_non_object_root() {
        assert_eq!(evaluate(&json!([])).expect_err("root").first().unwrap().code, "ROOT_NOT_OBJECT");
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let mut v = fixture();
        v["schema_version"] = json!(2);
        assert!(has(&v, "UNSUPPORTED_SCHEMA"));
    }

    #[test]
    fn rejects_missing_resource_ledger_reference() {
        let mut v = fixture();
        v["profile"].as_object_mut().unwrap().remove("resource_ledger_ref");
        assert!(has(&v, "MISSING_PROFILE_REFERENCE"));
    }

    #[test]
    fn rejects_non_dry_mass_basis() {
        let mut v = fixture();
        v["profile"]["mass_basis"] = json!("WET_MASS");
        assert!(has(&v, "INVALID_MASS_BASIS"));
    }

    #[test]
    fn rejects_duplicate_interval_id() {
        let mut v = fixture();
        v["intervals"][1]["interval_id"] = json!("WINDOW-001");
        assert!(has(&v, "DUPLICATE_INTERVAL_ID"));
    }

    #[test]
    fn rejects_wrong_machine_configuration() {
        let mut v = fixture();
        v["intervals"][0]["configuration_ref"] = json!("machine:other");
        assert!(has(&v, "MACHINE_CONFIGURATION_MISMATCH"));
    }

    #[test]
    fn rejects_missing_clock_sync() {
        let mut v = fixture();
        v["intervals"][0].as_object_mut().unwrap().remove("clock_sync_ref");
        assert!(has(&v, "MISSING_CLOCK_SYNC"));
    }

    #[test]
    fn rejects_non_positive_time_window() {
        let mut v = fixture();
        v["intervals"][0]["end_ms"] = json!(1000000);
        assert!(has(&v, "INVALID_TIME_WINDOW"));
    }

    #[test]
    fn rejects_time_uncertainty_exceeding_interval() {
        let mut v = fixture();
        v["intervals"][0]["time_uncertainty_ms"] = json!(60000.0);
        assert!(has(&v, "TIME_UNCERTAINTY_EXCEEDS_WINDOW"));
    }

    #[test]
    fn rejects_negative_generated_mass() {
        let mut v = fixture();
        v["intervals"][0]["generated_mass"]["value_kg"] = json!(-1.0);
        assert!(has(&v, "INVALID_MASS_VALUE"));
    }

    #[test]
    fn rejects_non_independent_mass_role() {
        let mut v = fixture();
        v["intervals"][0]["recovered_mass"]["evidence_role"] = json!("MODEL_ESTIMATE");
        assert!(has(&v, "NON_INDEPENDENT_MASS_EVIDENCE"));
    }

    #[test]
    fn rejects_missing_calibration_reference() {
        let mut v = fixture();
        v["intervals"][0]["generated_mass"].as_object_mut().unwrap().remove("calibration_ref");
        assert!(has(&v, "MISSING_MASS_PROVENANCE"));
    }

    #[test]
    fn derived_mass_requires_lineage() {
        let mut v = fixture();
        v["intervals"][0]["residual_mass"].as_object_mut().unwrap().remove("lineage_ref");
        assert!(has(&v, "MISSING_MASS_LINEAGE"));
    }

    #[test]
    fn rejects_first_bin_not_starting_at_zero() {
        let mut v = fixture();
        v["intervals"][0]["fragment_bins"][0]["lower_mm"] = json!(0.1);
        assert!(has(&v, "PSD_FIRST_BIN_NOT_ZERO"));
    }

    #[test]
    fn rejects_psd_bin_gap() {
        let mut v = fixture();
        v["intervals"][0]["fragment_bins"][1]["lower_mm"] = json!(5.1);
        assert!(has(&v, "PSD_BIN_GAP_OR_OVERLAP"));
    }

    #[test]
    fn rejects_psd_bin_overlap() {
        let mut v = fixture();
        v["intervals"][0]["fragment_bins"][1]["lower_mm"] = json!(4.9);
        assert!(has(&v, "PSD_BIN_GAP_OR_OVERLAP"));
    }

    #[test]
    fn rejects_last_bin_with_finite_upper_bound() {
        let mut v = fixture();
        v["intervals"][0]["fragment_bins"][3]["upper_mm"] = json!(30.0);
        assert!(has(&v, "PSD_LAST_BIN_NOT_OPEN"));
    }

    #[test]
    fn rejects_non_independent_psd_evidence() {
        let mut v = fixture();
        v["intervals"][0]["fragment_bins"][0]["evidence_role"] = json!("SIMULATION");
        assert!(has(&v, "NON_INDEPENDENT_PSD_EVIDENCE"));
    }

    #[test]
    fn reports_psd_mass_mismatch_as_indeterminate_accounting() {
        let mut v = fixture();
        v["intervals"][0]["fragment_bins"][0]["mass_kg"] = json!(30.0);
        let report = evaluate(&v).expect("well-formed but inconsistent accounting");
        assert_eq!(report.accounting_disposition, "INDETERMINATE_ACCOUNTING");
        assert!(!report.interval_reports[0].fragment_bins_within_uncertainty);
    }

    #[test]
    fn mass_balance_closes_within_conservative_uncertainty_envelope() {
        let report = evaluate(&fixture()).expect("fixture");
        assert!(report.interval_reports[0].mass_balance_within_uncertainty);
        assert!((report.interval_reports[0].mass_balance_residual_kg - 2.0).abs() < 1e-9);
    }

    #[test]
    fn mass_balance_failure_is_indeterminate_not_zero_filled() {
        let mut v = fixture();
        v["intervals"][0]["recovered_mass"]["value_kg"] = json!(40.0);
        let report = evaluate(&v).expect("well-formed but non-closing balance");
        assert_eq!(report.accounting_disposition, "INDETERMINATE_ACCOUNTING");
        assert!(!report.interval_reports[0].mass_balance_within_uncertainty);
    }

    #[test]
    fn rejects_zero_generated_mass() {
        let mut v = fixture();
        v["intervals"][0]["generated_mass"]["value_kg"] = json!(0.0);
        v["intervals"][0]["fragment_bins"] = json!([{
            "lower_mm": 0.0, "upper_mm": null, "mass_kg": 0.0,
            "standard_uncertainty_kg": 0.0, "evidence_role": "INDEPENDENT_MEASUREMENT",
            "calibration_ref": "cal:psd", "source_ref": "artifact:psd"
        }]);
        assert!(has(&v, "NON_POSITIVE_GENERATED_MASS"));
    }

    #[test]
    fn rejects_negative_residual_mass() {
        let mut v = fixture();
        v["intervals"][0]["residual_mass"]["value_kg"] = json!(-1.0);
        assert!(has(&v, "INVALID_MASS_VALUE"));
    }

    #[test]
    fn rejects_missing_stage_capacity() {
        let mut v = fixture();
        v["intervals"][0]["stage_capacities"].as_array_mut().unwrap().pop();
        assert!(has(&v, "STAGE_COUNT_MISMATCH"));
        assert!(has(&v, "MISSING_STAGE"));
    }

    #[test]
    fn rejects_duplicate_stage() {
        let mut v = fixture();
        v["intervals"][0]["stage_capacities"][1]["stage"] = json!("CAPTURE");
        assert!(has(&v, "DUPLICATE_STAGE"));
    }

    #[test]
    fn rejects_wrong_stage_order() {
        let mut v = fixture();
        v["intervals"][0]["stage_capacities"].as_array_mut().unwrap().reverse();
        assert!(has(&v, "STAGE_ORDER_MISMATCH"));
    }

    #[test]
    fn rejects_factory_capacity_for_installed_path() {
        let mut v = fixture();
        v["intervals"][0]["stage_capacities"][0]["transport_configuration_ref"] = json!("transport:factory-test-line");
        assert!(has(&v, "INSTALLED_CONFIGURATION_MISMATCH"));
    }

    #[test]
    fn rejects_model_estimated_stage_capacity() {
        let mut v = fixture();
        v["intervals"][0]["stage_capacities"][1]["evidence_role"] = json!("MODEL_ESTIMATE");
        assert!(has(&v, "NON_INDEPENDENT_CAPACITY_EVIDENCE"));
    }

    #[test]
    fn rejects_capacity_lower_greater_than_upper() {
        let mut v = fixture();
        v["intervals"][0]["stage_capacities"][1]["capacity_lower_kg_s"] = json!(2.0);
        v["intervals"][0]["stage_capacities"][1]["capacity_upper_kg_s"] = json!(1.0);
        assert!(has(&v, "INVALID_CAPACITY_INTERVAL"));
    }

    #[test]
    fn identifies_definite_conveying_bottleneck() {
        let report = evaluate(&fixture()).expect("fixture");
        let interval = &report.interval_reports[0];
        assert_eq!(interval.capacity_disposition, "DEFINITE_CAPACITY_BOTTLENECK");
        assert_eq!(interval.definite_bottleneck_stages, vec!["CONVEY"]);
    }

    #[test]
    fn identifies_ambiguous_capture_capacity() {
        let report = evaluate(&fixture()).expect("fixture");
        let interval = &report.interval_reports[1];
        assert_eq!(interval.capacity_disposition, "CAPACITY_COMPARISON_AMBIGUOUS");
        assert_eq!(interval.possible_bottleneck_stages, vec!["CAPTURE"]);
    }

    #[test]
    fn no_bottleneck_is_only_a_capacity_envelope_statement() {
        let mut v = fixture();
        for stage in v["intervals"][0]["stage_capacities"].as_array_mut().unwrap() {
            stage["capacity_lower_kg_s"] = json!(3.0);
            stage["capacity_upper_kg_s"] = json!(3.5);
        }
        let report = evaluate(&v).expect("fixture");
        assert_eq!(
            report.interval_reports[0].capacity_disposition,
            "NO_STAGE_CAPACITY_BOTTLENECK_EVIDENCED"
        );
    }

    #[test]
    fn rejects_empty_intervals() {
        let mut v = fixture();
        v["intervals"] = json!([]);
        assert_eq!(evaluate(&v).expect_err("empty").first().unwrap().code, "EMPTY_INTERVALS");
    }


    #[test]
    fn rejects_missing_fragment_upper_boundary() {
        let mut v = fixture();
        v["intervals"][0]["fragment_bins"][0].as_object_mut().unwrap().remove("upper_mm");
        assert!(has(&v, "MISSING_PSD_UPPER_BOUND"));
    }

    #[test]
    fn rejects_capacity_profile_mismatch() {
        let mut v = fixture();
        v["intervals"][0]["stage_capacities"][0]["transport_capacity_profile_ref"] = json!("capacity:factory-line");
        assert!(has(&v, "CAPACITY_PROFILE_MISMATCH"));
    }

    #[test]
    fn rejects_missing_required_mass_measurement() {
        let mut v = fixture();
        v["intervals"][0].as_object_mut().unwrap().remove("generated_mass");
        assert!(has(&v, "MISSING_MASS_MEASUREMENT"));
    }


    #[test]
    fn rejects_overlapping_windows_within_same_trial() {
        let mut v = fixture();
        v["intervals"][1]["start_ms"] = json!(1050000);
        assert!(has(&v, "TIME_WINDOW_OVERLAP"));
    }

    #[test]
    fn allows_equal_clock_windows_for_distinct_trials() {
        let mut v = fixture();
        v["intervals"][1]["trial_id"] = json!("TRIAL-002");
        v["intervals"][1]["start_ms"] = json!(1000000);
        v["intervals"][1]["end_ms"] = json!(1060000);
        let report = evaluate(&v).expect("different trials may use the same clock interval");
        assert_eq!(report.interval_count, 2);
    }

    #[test]
    fn warnings_preserve_claim_ceiling() {
        let report = evaluate(&fixture()).expect("fixture");
        assert!(report.warnings.iter().any(|w| w.contains("fabricated")));
        assert!(report.warnings.iter().any(|w| w.contains("not a probabilistic")));
    }
}
