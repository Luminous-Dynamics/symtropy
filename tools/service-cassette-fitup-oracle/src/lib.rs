//! Synthetic-first service-cassette tolerance-stack and fit-up oracle.
//!
//! V0.1 uses exact integer nanometres internally, but requires every input
//! quantity to use the same declared canonical unit for the interface. This
//! prevents an implicit unit conversion from laundering an input-contract error.
//! It implements bounded interval arithmetic only; no RSS/statistical model is
//! inferred. It does not qualify physical equipment or authorize field work.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_ID: &str = "SERVICE-CASSETTE-FITUP-CORPUS-V0.1";
const CLAIM_CEILING: &str = "synthetic fit-up evaluator semantics only; no real building datum, instrument calibration, engineering approval, code compliance, physical safety, product certification or field modification authority";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Interval {
    lower_nm: i128,
    upper_nm: i128,
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn scale_to_nm(unit: &str) -> Option<i128> {
    match unit {
        "nm" => Some(1),
        "um" => Some(1_000),
        "mm" => Some(1_000_000),
        "m" => Some(1_000_000_000),
        "in" => Some(25_400_000),
        _ => None,
    }
}

fn quantity_nm(value: &Value, canonical_unit: &str) -> Result<i128, String> {
    let unit = text(value, "unit");
    if unit.is_empty() || unit != canonical_unit {
        return Err("DIMENSION_UNIT_MISMATCH".to_owned());
    }
    let raw = value
        .get("value")
        .and_then(Value::as_i64)
        .ok_or_else(|| "DIMENSION_VALUE_NOT_INTEGER".to_owned())?;
    let scale = scale_to_nm(unit)
        .ok_or_else(|| "DIMENSION_UNIT_UNSUPPORTED".to_owned())?;
    i128::from(raw)
        .checked_mul(scale)
        .ok_or_else(|| "DIMENSION_VALUE_OVERFLOW".to_owned())
}

fn interval_nm(value: &Value, canonical_unit: &str) -> Result<Interval, String> {
    let lower = quantity_nm(value.get("lower").unwrap_or(&Value::Null), canonical_unit)?;
    let upper = quantity_nm(value.get("upper").unwrap_or(&Value::Null), canonical_unit)?;
    if lower > upper {
        return Err("INTERVAL_LOWER_EXCEEDS_UPPER".to_owned());
    }
    Ok(Interval {
        lower_nm: lower,
        upper_nm: upper,
    })
}

fn add_invalid(reasons: &mut BTreeSet<String>, code: &str) {
    reasons.insert(code.to_owned());
}

fn add_incomplete(reasons: &mut BTreeSet<String>, code: &str) {
    reasons.insert(code.to_owned());
}

fn is_invalid_reason(reason: &str) -> bool {
    [
        "MISMATCH",
        "INVALID",
        "OVERFLOW",
        "DUPLICATED",
        "UNDECLARED",
        "UNSUPPORTED",
        "EXCEEDS",
        "NOT_INTEGER",
    ]
    .iter()
    .any(|token| reason.contains(token))
}

fn checked_add(
    left: i128,
    right: i128,
    reasons: &mut BTreeSet<String>,
) -> Option<i128> {
    match left.checked_add(right) {
        Some(value) => Some(value),
        None => {
            add_invalid(reasons, "TOLERANCE_STACK_ARITHMETIC_OVERFLOW");
            None
        }
    }
}

fn apply_coefficient(interval: Interval, coefficient: i64) -> Option<Interval> {
    match coefficient {
        1 => Some(interval),
        -1 => Some(Interval {
            lower_nm: interval.upper_nm.checked_neg()?,
            upper_nm: interval.lower_nm.checked_neg()?,
        }),
        _ => None,
    }
}

fn calibration_state(
    measurement: &Value,
    label: &str,
    reasons: &mut BTreeSet<String>,
) -> bool {
    let required = [
        "measurement_receipt_id",
        "instrument_id",
        "calibration_receipt_id",
        "measured_at",
        "calibration_valid_from",
        "calibration_valid_until",
        "calibration_state",
    ];
    if required
        .iter()
        .any(|field| measurement.get(*field).and_then(Value::as_str).is_none())
    {
        add_incomplete(
            reasons,
            &format!("{label}_MEASUREMENT_OR_CALIBRATION_BINDING_INCOMPLETE"),
        );
        return false;
    }

    let measured_at = text(measurement, "measured_at");
    let valid_from = text(measurement, "calibration_valid_from");
    let valid_until = text(measurement, "calibration_valid_until");
    if !canonical_utc(measured_at)
        || !canonical_utc(valid_from)
        || !canonical_utc(valid_until)
        || valid_from >= valid_until
    {
        add_invalid(
            reasons,
            &format!("{label}_CALIBRATION_TIME_ENVELOPE_INVALID"),
        );
        return false;
    }
    if measured_at < valid_from || measured_at >= valid_until {
        add_incomplete(
            reasons,
            &format!("{label}_CALIBRATION_NOT_VALID_AT_MEASUREMENT"),
        );
        return false;
    }
    if text(measurement, "calibration_state") != "verified" {
        add_incomplete(
            reasons,
            &format!("{label}_CALIBRATION_NOT_VERIFIED"),
        );
        return false;
    }
    true
}

fn canonical_utc(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    for index in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
    }
    let year = value[0..4].parse::<u32>().ok();
    let month = value[5..7].parse::<u32>().ok();
    let day = value[8..10].parse::<u32>().ok();
    let hour = value[11..13].parse::<u32>().ok();
    let minute = value[14..16].parse::<u32>().ok();
    let second = value[17..19].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) =
        (year, month, day, hour, minute, second)
    else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    day >= 1 && day <= max_day && hour <= 23 && minute <= 59 && second <= 59
}

fn validate_common_bindings(
    input: &Value,
    reasons: &mut BTreeSet<String>,
) -> String {
    let scope = input.get("scope").unwrap_or(&Value::Null);
    let cassette = input.get("cassette").unwrap_or(&Value::Null);
    let host = input.get("host_measurement").unwrap_or(&Value::Null);
    let module = input.get("cassette_measurement").unwrap_or(&Value::Null);
    let required_scope = [
        "facility_id",
        "design_revision",
        "reference_frame_id",
        "datum_revision",
        "required_host_as_built_revision",
        "canonical_length_unit",
        "tolerance_method_id",
        "tolerance_method_revision",
        "evaluation_at",
    ];
    if required_scope
        .iter()
        .any(|field| scope.get(*field).and_then(Value::as_str).is_none())
    {
        add_incomplete(reasons, "COMMON_SCOPE_BINDING_INCOMPLETE");
    }
    let unit = text(scope, "canonical_length_unit").to_owned();
    if unit.is_empty() {
        add_incomplete(reasons, "CANONICAL_LENGTH_UNIT_MISSING");
    } else if scale_to_nm(&unit).is_none() {
        add_invalid(reasons, "CANONICAL_LENGTH_UNIT_UNSUPPORTED");
    }
    if !canonical_utc(text(scope, "evaluation_at")) {
        add_invalid(reasons, "EVALUATION_TIME_INVALID");
    }

    if text(cassette, "cassette_id").is_empty()
        || text(cassette, "manufacturing_revision").is_empty()
        || text(cassette, "target_design_revision").is_empty()
        || text(cassette, "interface_id").is_empty()
    {
        add_incomplete(reasons, "CASSETTE_IDENTITY_INCOMPLETE");
    }
    if !text(cassette, "target_design_revision").is_empty()
        && !text(scope, "design_revision").is_empty()
        && text(cassette, "target_design_revision") != text(scope, "design_revision")
    {
        add_invalid(reasons, "CASSETTE_TARGET_DESIGN_REVISION_MISMATCH");
    }
    for (measurement, label) in [(host, "HOST"), (module, "CASSETTE")] {
        if text(measurement, "reference_frame_id").is_empty()
            || text(measurement, "datum_revision").is_empty()
        {
            add_incomplete(reasons, &format!("{label}_REFERENCE_FRAME_OR_DATUM_INCOMPLETE"));
        } else if text(measurement, "reference_frame_id") != text(scope, "reference_frame_id")
            || text(measurement, "datum_revision") != text(scope, "datum_revision")
        {
            add_invalid(reasons, &format!("{label}_REFERENCE_FRAME_OR_DATUM_MISMATCH"));
        }
    }
    if text(host, "as_built_revision").is_empty() {
        add_incomplete(reasons, "HOST_AS_BUILT_REVISION_MISSING");
    } else if text(host, "as_built_revision") != text(scope, "required_host_as_built_revision") {
        add_invalid(reasons, "HOST_AS_BUILT_REVISION_MISMATCH");
    }
    if text(host, "design_revision").is_empty() {
        add_incomplete(reasons, "HOST_DESIGN_REVISION_MISSING");
    } else if text(host, "design_revision") != text(scope, "design_revision") {
        add_invalid(reasons, "HOST_DESIGN_REVISION_MISMATCH");
    }
    if text(module, "manufacturing_revision").is_empty() {
        add_incomplete(reasons, "CASSETTE_MEASUREMENT_REVISION_MISSING");
    } else if text(module, "manufacturing_revision") != text(cassette, "manufacturing_revision") {
        add_invalid(reasons, "CASSETTE_MANUFACTURING_REVISION_MISMATCH");
    }

    calibration_state(host, "HOST", reasons);
    calibration_state(module, "CASSETTE", reasons);
    unit
}

fn declared_axes(allowance: &Value) -> BTreeSet<String> {
    allowance
        .as_object()
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default()
}

fn build_stack(
    input: &Value,
    canonical_unit: &str,
    reasons: &mut BTreeSet<String>,
) -> BTreeMap<String, Interval> {
    let scope = input.get("scope").unwrap_or(&Value::Null);
    let model = input.get("tolerance_model").unwrap_or(&Value::Null);
    let allowance = model.get("interface_allowance").unwrap_or(&Value::Null);
    let required = array(scope, "required_tolerance_term_ids");
    let terms = array(model, "stack_terms");

    if text(model, "method_id").is_empty()
        || text(model, "method_revision").is_empty()
        || text(model, "method").is_empty()
    {
        add_incomplete(reasons, "TOLERANCE_METHOD_BINDING_INCOMPLETE");
    } else if text(model, "method_id") != text(scope, "tolerance_method_id")
        || text(model, "method_revision") != text(scope, "tolerance_method_revision")
    {
        add_invalid(reasons, "TOLERANCE_METHOD_BINDING_MISMATCH");
    } else if text(model, "method") != "worst_case_interval_v0.1" {
        add_invalid(reasons, "TOLERANCE_METHOD_UNSUPPORTED");
    }

    let axes = declared_axes(allowance);
    if axes.is_empty() {
        add_incomplete(reasons, "INTERFACE_ALLOWANCE_MISSING");
    }
    let required_ids = required
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if scope.get("required_tolerance_term_ids").is_none() {
        add_incomplete(reasons, "REQUIRED_TOLERANCE_TERM_INVENTORY_MISSING");
    } else if required_ids.is_empty() || required_ids.len() != required.len() {
        add_invalid(reasons, "REQUIRED_TOLERANCE_TERM_INVENTORY_INVALID");
    }
    if terms.is_empty() {
        add_incomplete(reasons, "TOLERANCE_STACK_TERMS_MISSING");
    }

    let host = input.get("host_measurement").unwrap_or(&Value::Null);
    let cassette = input.get("cassette_measurement").unwrap_or(&Value::Null);
    let host_receipt = text(host, "measurement_receipt_id");
    let cassette_receipt = text(cassette, "measurement_receipt_id");

    let mut seen = BTreeSet::new();
    let mut sums = axes
        .iter()
        .map(|axis| {
            (
                axis.clone(),
                Interval {
                    lower_nm: 0,
                    upper_nm: 0,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    for term in terms {
        let id = text(term, "term_id");
        let axis = text(term, "axis");
        if id.is_empty() || !seen.insert(id.to_owned()) {
            add_invalid(reasons, "TOLERANCE_TERM_ID_EMPTY_OR_DUPLICATED");
            continue;
        }
        if !axes.contains(axis) {
            add_invalid(reasons, "TOLERANCE_TERM_AXIS_UNDECLARED");
            continue;
        }
        let source = text(term, "source");
        let expected_receipt = match source {
            "host" => host_receipt,
            "cassette" => cassette_receipt,
            _ => {
                add_invalid(reasons, "TOLERANCE_TERM_SOURCE_INVALID");
                continue;
            }
        };
        if expected_receipt.is_empty() || text(term, "measurement_receipt_id").is_empty() {
            add_incomplete(reasons, "TOLERANCE_TERM_MEASUREMENT_RECEIPT_INCOMPLETE");
        } else if text(term, "measurement_receipt_id") != expected_receipt {
            add_invalid(reasons, "TOLERANCE_TERM_MEASUREMENT_RECEIPT_MISMATCH");
        }

        let coefficient = term.get("coefficient").and_then(Value::as_i64).unwrap_or(0);
        if coefficient != 1 && coefficient != -1 {
            add_invalid(reasons, "TOLERANCE_TERM_COEFFICIENT_INVALID");
            continue;
        }
        let interval = match interval_nm(term, canonical_unit) {
            Ok(value) => value,
            Err(reason) => {
                add_invalid(reasons, &reason);
                continue;
            }
        };
        let Some(contribution) = apply_coefficient(interval, coefficient) else {
            add_invalid(reasons, "TOLERANCE_TERM_COEFFICIENT_OVERFLOW");
            continue;
        };
        let current = sums.get(axis).copied().unwrap_or(Interval {
            lower_nm: 0,
            upper_nm: 0,
        });
        let Some(lower) = checked_add(current.lower_nm, contribution.lower_nm, reasons) else {
            continue;
        };
        let Some(upper) = checked_add(current.upper_nm, contribution.upper_nm, reasons) else {
            continue;
        };
        sums.insert(
            axis.to_owned(),
            Interval {
                lower_nm: lower,
                upper_nm: upper,
            },
        );
    }

    if !required_ids.is_subset(&seen) {
        add_incomplete(reasons, "TOLERANCE_REQUIRED_TERM_MISSING");
    }
    if !seen.is_subset(&required_ids) {
        add_invalid(reasons, "TOLERANCE_UNEXPECTED_TERM_PRESENT");
    }
    for axis in &axes {
        if !sums.contains_key(axis) {
            add_incomplete(reasons, "TOLERANCE_STACK_AXIS_MISSING");
        }
    }
    sums
}

fn allowance_intervals(
    input: &Value,
    canonical_unit: &str,
    reasons: &mut BTreeSet<String>,
) -> BTreeMap<String, Interval> {
    let allowance = input
        .get("tolerance_model")
        .and_then(|v| v.get("interface_allowance"))
        .unwrap_or(&Value::Null);
    let mut result = BTreeMap::new();
    if let Some(map) = allowance.as_object() {
        for (axis, value) in map {
            match interval_nm(value, canonical_unit) {
                Ok(interval) => {
                    result.insert(axis.clone(), interval);
                }
                Err(reason) => {
                    add_invalid(reasons, &reason);
                }
            }
        }
    }
    result
}

fn inside(inner: Interval, outer: Interval) -> bool {
    inner.lower_nm >= outer.lower_nm && inner.upper_nm <= outer.upper_nm
}

fn overlaps(left: Interval, right: Interval) -> bool {
    left.lower_nm <= right.upper_nm && right.lower_nm <= left.upper_nm
}

fn buffer_covers_required_correction(
    input: &Value,
    stack: &BTreeMap<String, Interval>,
    canonical_unit: &str,
    reasons: &mut BTreeSet<String>,
) -> bool {
    let buffer = input
        .get("tolerance_model")
        .and_then(|v| v.get("buffer"))
        .unwrap_or(&Value::Null);
    if buffer.get("qualified").and_then(Value::as_bool) != Some(true)
        || text(buffer, "buffer_id").is_empty()
        || text(buffer, "buffer_revision").is_empty()
        || text(buffer, "qualified_revision") != text(buffer, "buffer_revision")
    {
        return false;
    }
    let axes = buffer.get("axes").unwrap_or(&Value::Null);
    let Some(axis_map) = axes.as_object() else {
        return false;
    };

    for (axis, bounds) in stack {
        let Some(qualified_range) = axis_map.get(axis) else {
            return false;
        };
        let range = match interval_nm(qualified_range, canonical_unit) {
            Ok(value) => value,
            Err(reason) => {
                add_invalid(reasons, &reason);
                return false;
            }
        };
        let required_correction = Interval {
            lower_nm: match bounds.upper_nm.checked_neg() {
                Some(v) => v,
                None => {
                    add_invalid(reasons, "BUFFER_CORRECTION_OVERFLOW");
                    return false;
                }
            },
            upper_nm: match bounds.lower_nm.checked_neg() {
                Some(v) => v,
                None => {
                    add_invalid(reasons, "BUFFER_CORRECTION_OVERFLOW");
                    return false;
                }
            },
        };
        if !inside(required_correction, range) {
            return false;
        }
        let directions = array(qualified_range, "permitted_directions")
            .iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        if (required_correction.lower_nm < 0 && !directions.contains("negative"))
            || (required_correction.upper_nm > 0 && !directions.contains("positive"))
        {
            return false;
        }
    }
    true
}

fn evaluate_installed(
    input: &Value,
    canonical_unit: &str,
    allowance: &BTreeMap<String, Interval>,
    reasons: &mut BTreeSet<String>,
) -> (String, Value) {
    let installed = input
        .get("final_installed_measurement")
        .unwrap_or(&Value::Null);
    if installed.is_null() {
        return ("not_measured".to_owned(), Value::Null);
    }
    if text(installed, "reference_frame_id")
        != text(input.get("scope").unwrap_or(&Value::Null), "reference_frame_id")
        || text(installed, "datum_revision")
            != text(input.get("scope").unwrap_or(&Value::Null), "datum_revision")
    {
        add_invalid(reasons, "FINAL_INSTALLED_REFERENCE_FRAME_OR_DATUM_MISMATCH");
    }
    let calibration_ok = calibration_state(installed, "FINAL_INSTALLED", reasons);
    let residuals = installed
        .get("residuals")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut per_axis = serde_json::Map::new();
    let mut all_inside = true;
    let mut any_disjoint = false;
    for (axis, limit) in allowance {
        let Some(raw) = residuals.get(axis) else {
            add_incomplete(reasons, "FINAL_INSTALLED_AXIS_MEASUREMENT_MISSING");
            return ("insufficient_evidence".to_owned(), Value::Null);
        };
        let measured = match interval_nm(raw, canonical_unit) {
            Ok(v) => v,
            Err(reason) => {
                add_invalid(reasons, &reason);
                return ("invalid_input".to_owned(), Value::Null);
            }
        };
        all_inside &= inside(measured, *limit);
        any_disjoint |= !overlaps(measured, *limit);
        per_axis.insert(
            axis.clone(),
            json!({
                "measured_lower_nm": measured.lower_nm.to_string(),
                "measured_upper_nm": measured.upper_nm.to_string(),
                "accepted": inside(measured, *limit)
            }),
        );
    }
    if !calibration_ok {
        return ("insufficient_evidence".to_owned(), Value::Null);
    }
    let state = if all_inside {
        "accepted"
    } else if any_disjoint {
        "rejected"
    } else {
        "unknown_boundary_crossing"
    };
    (
        state.to_owned(),
        Value::Object(per_axis),
    )
}

/// Evaluate a single synthetic fit-up case. Expected values are never read.
pub fn evaluate_case(case: &Value) -> Value {
    let case_id = text(case, "case_id");
    let input = case.get("input").unwrap_or(&Value::Null);
    let mut reasons = BTreeSet::new();
    let unit = validate_common_bindings(input, &mut reasons);
    let stack = build_stack(input, &unit, &mut reasons);
    let allowance = allowance_intervals(input, &unit, &mut reasons);
    let stack_available = !stack.is_empty() && !allowance.is_empty();

    let mut stack_json = serde_json::Map::new();
    for (axis, interval) in &stack {
        stack_json.insert(
            axis.clone(),
            json!({
                "lower_nm": interval.lower_nm.to_string(),
                "upper_nm": interval.upper_nm.to_string()
            }),
        );
    }

    let input_state = if reasons.iter().any(|reason| is_invalid_reason(reason)) {
        "invalid_input"
    } else if reasons.iter().any(|reason| {
        reason.contains("INCOMPLETE")
            || reason.contains("NOT_VERIFIED")
            || reason.contains("NOT_VALID")
    }) {
        "insufficient_evidence"
    } else {
        "valid"
    };

    let fit_up = if input_state == "invalid_input" {
        "invalid_input"
    } else if input_state == "insufficient_evidence" || !stack_available {
        "unknown"
    } else {
        let mut all_inside = true;
        let mut any_disjoint = false;
        for (axis, limit) in &allowance {
            let Some(stack_interval) = stack.get(axis).copied() else {
                all_inside = false;
                continue;
            };
            all_inside &= inside(stack_interval, *limit);
            any_disjoint |= !overlaps(stack_interval, *limit);
        }
        if all_inside {
            "fit"
        } else if buffer_covers_required_correction(input, &stack, &unit, &mut reasons) {
            "fit_with_qualified_adjustment"
        } else if any_disjoint {
            "blocked"
        } else {
            "unknown"
        }
    };

    let installed_allowance = allowance_intervals(input, &unit, &mut BTreeSet::new());
    let (installed_acceptance, installed_axes) =
        evaluate_installed(input, &unit, &installed_allowance, &mut reasons);

    let final_fit_up = if input_state == "invalid_input" {
        "invalid_input"
    } else {
        fit_up
    };

    json!({
        "case_id": case_id,
        "input_state": input_state,
        "fit_up_disposition": final_fit_up,
        "installed_acceptance": installed_acceptance,
        "canonical_length_unit": unit,
        "stack_bounds_nm": stack_json,
        "installed_axis_results": installed_axes,
        "reason_codes": reasons.into_iter().collect::<Vec<_>>(),
        "factory_acceptance_implies_installation": false,
        "fit_up_implies_installed_acceptance": false,
        "action_authority": "none",
        "claim_ceiling": CLAIM_CEILING
    })
}

fn equivalent(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(left), Value::Number(right)) => {
            left.as_i64().zip(right.as_i64()).is_some_and(|(a, b)| a == b)
                || left.as_u64().zip(right.as_u64()).is_some_and(|(a, b)| a == b)
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(a, b)| equivalent(a, b))
        }
        (Value::Object(left), Value::Object(right)) => right.iter().all(|(key, value)| {
            left.get(key).is_some_and(|actual_value| equivalent(actual_value, value))
        }),
        _ => actual == expected,
    }
}

pub fn matches_expected(actual: &Value, expected: &Value) -> bool {
    equivalent(actual, expected)
}

#[cfg(test)]
mod tests {
    use super::{evaluate_case, matches_expected};
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn corpus() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/research/service-cassette/SERVICE_CASSETTE_FITUP_CORPUS_V0_1.json");
        let bytes = std::fs::read(path).expect("fit-up corpus fixture must exist");
        serde_json::from_slice(&bytes).expect("fit-up corpus JSON must parse")
    }

    #[test]
    fn all_fitup_cases_match_independent_gold() {
        for case in corpus()["cases"].as_array().unwrap() {
            let actual = evaluate_case(case);
            assert!(
                matches_expected(&actual, &case["expected"]),
                "{} mismatch\nactual: {}\nexpected: {}",
                case["case_id"].as_str().unwrap(),
                serde_json::to_string_pretty(&actual).unwrap(),
                serde_json::to_string_pretty(&case["expected"]).unwrap()
            );
        }
    }

    #[test]
    fn corpus_has_28_unique_cases() {
        let cases = corpus()["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 28);
        let ids = cases
            .iter()
            .map(|case| case["case_id"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 28);
    }
}
