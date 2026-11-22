#![forbid(unsafe_code)]

//! T0 arithmetic auditor for synthetic sanitation benchmark cases.
//!
//! This crate checks declared balances and derived load arithmetic only.
//! It is not a treatment-process model, water-quality assessment, pathogen
//! model, permit checker, or physical performance certification.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AuditResult {
    pub case_id: String,
    pub actual_disposition: String,
    pub expected_disposition: String,
    pub actual_reason_codes: Vec<String>,
    pub expected_reason_codes: Vec<String>,
    pub oracle_match: bool,
    pub diagnostics: Vec<String>,
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing required field: {key}"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    let n = field(value, key)?
        .as_f64()
        .ok_or_else(|| format!("field {key} must be numeric"))?;
    if !n.is_finite() {
        return Err(format!("field {key} must be finite"));
    }
    Ok(n)
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?
        .as_str()
        .ok_or_else(|| format!("field {key} must be a string"))
}

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance
}

fn push_reason(reasons: &mut Vec<String>, code: &str) {
    if !reasons.iter().any(|existing| existing == code) {
        reasons.push(code.to_owned());
    }
}

fn require_unit(
    object: &Value,
    expected_unit: &str,
    diagnostics: &mut Vec<String>,
    reasons: &mut Vec<String>,
) -> Result<(), String> {
    let actual = string(object, "unit")?;
    if actual != expected_unit {
        diagnostics.push(format!(
            "unit mismatch: expected {expected_unit}, received {actual}"
        ));
        push_reason(reasons, "INVALID_OR_MISSING_REQUIRED_INPUT");
    }
    Ok(())
}

/// Audit a single JSON fixture and compare recomputed results with its oracle.
pub fn audit_case(input: &str) -> Result<AuditResult, String> {
    let root: Value = serde_json::from_str(input)
        .map_err(|error| format!("invalid JSON: {error}"))?;

    let case_id = string(&root, "case_id")?.to_owned();
    if string(&root, "schema_version")? != "0.1.0" {
        return Err("unsupported schema_version".to_owned());
    }
    if string(&root, "case_class")? != "synthetic_accounting_only" {
        return Err("unsupported case_class; this auditor accepts synthetic accounting fixtures only".to_owned());
    }
    if string(&root, "evidence_ceiling")?
        != "arithmetic_conformance_only_no_physical_or_safety_claim"
    {
        return Err("unsupported evidence ceiling".to_owned());
    }

    let expected_disposition = string(&root, "expected_disposition")?.to_owned();
    let expected_reasons_value = field(&root, "expected_reason_codes")?
        .as_array()
        .ok_or_else(|| "expected_reason_codes must be an array".to_owned())?;
    let mut expected_reason_codes = Vec::with_capacity(expected_reasons_value.len());
    for value in expected_reasons_value {
        let code = value
            .as_str()
            .ok_or_else(|| "expected reason codes must be strings".to_owned())?;
        expected_reason_codes.push(code.to_owned());
    }
    expected_reason_codes.sort();

    let mut actual_reason_codes = Vec::new();
    let mut diagnostics = Vec::new();

    // Water source/sink closure.
    let water = field(&root, "water_balance")?;
    require_unit(water, "L/day", &mut diagnostics, &mut actual_reason_codes)?;
    let water_in = number(water, "influent_l_day")?;
    let water_effluent = number(water, "effluent_l_day")?;
    let water_sludge = number(water, "sludge_water_l_day")?;
    let water_evaporation = number(water, "evaporation_l_day")?;
    let water_expected = number(water, "expected_closure_l_day")?;
    let water_tolerance = number(water, "closure_tolerance_l_day")?;
    if [water_in, water_effluent, water_sludge, water_evaporation]
        .iter()
        .any(|value| *value < 0.0)
    {
        return Err("water flow values cannot be negative".to_owned());
    }
    if water_tolerance < 0.0 {
        return Err("water closure tolerance cannot be negative".to_owned());
    }
    let water_residual = water_in - water_effluent - water_sludge - water_evaporation;
    diagnostics.push(format!("water residual: {water_residual:.12} L/day"));
    if !close(water_residual, water_expected, water_tolerance.max(1e-12)) {
        diagnostics.push("recomputed water residual differs from declared closure".to_owned());
        push_reason(&mut actual_reason_codes, "WATER_BALANCE_EXCEEDS_TOLERANCE");
    }
    if water_residual.abs() > water_tolerance {
        diagnostics.push(format!(
            "absolute water residual exceeds tolerance {} L/day",
            water_tolerance
        ));
        push_reason(&mut actual_reason_codes, "WATER_BALANCE_EXCEEDS_TOLERANCE");
    }

    // Conserved load balances. Concentration removal is evaluated separately below.
    let constituents = field(&root, "conserved_constituent_balances")?
        .as_array()
        .ok_or_else(|| "conserved_constituent_balances must be an array".to_owned())?;
    if constituents.is_empty() {
        return Err("at least one constituent balance is required".to_owned());
    }
    for item in constituents {
        let name = string(item, "constituent")?;
        require_unit(item, "kg/day", &mut diagnostics, &mut actual_reason_codes)?;
        let inflow = number(item, "influent_kg_day")?;
        let effluent = number(item, "effluent_kg_day")?;
        let sludge = number(item, "sludge_kg_day")?;
        let other = number(item, "other_outputs_kg_day")?;
        let expected = number(item, "expected_closure_kg_day")?;
        let tolerance = number(item, "closure_tolerance_kg_day")?;
        if [inflow, effluent, sludge, other]
            .iter()
            .any(|value| *value < 0.0)
        {
            return Err(format!("{name} constituent loads cannot be negative"));
        }
        if tolerance < 0.0 {
            return Err(format!("{name} closure tolerance cannot be negative"));
        }
        let residual = inflow - effluent - sludge - other;
        diagnostics.push(format!("{name} residual: {residual:.12} kg/day"));
        if !close(residual, expected, tolerance.max(1e-12)) || residual.abs() > tolerance {
            push_reason(
                &mut actual_reason_codes,
                "CONSTITUENT_BALANCE_EXCEEDS_TOLERANCE",
            );
        }
    }

    // BOD5/COD are non-conservative performance metrics; compare loads, not concentrations alone.
    let performance = field(&root, "nonconservative_performance_metrics")?
        .as_array()
        .ok_or_else(|| "nonconservative_performance_metrics must be an array".to_owned())?;
    if performance.is_empty() {
        return Err("at least one non-conservative performance metric is required".to_owned());
    }
    for item in performance {
        let parameter = string(item, "parameter")?;
        if parameter != "BOD5" && parameter != "COD" {
            return Err(format!("unsupported non-conservative parameter: {parameter}"));
        }
        if string(item, "interpretation")?
            != "calculated_metric_not_a_conserved_species_or_compliance_result"
        {
            return Err(format!("{parameter} interpretation/evidence ceiling mismatch"));
        }
        require_unit(item, "mg/L", &mut diagnostics, &mut actual_reason_codes)?;
        let influent_concentration = number(item, "influent_mg_l")?;
        let effluent_concentration = number(item, "effluent_mg_l")?;
        let influent_flow = number(item, "influent_flow_l_day")?;
        let effluent_flow = number(item, "effluent_flow_l_day")?;
        let expected = number(item, "expected_load_removal_percent")?;
        if influent_concentration < 0.0
            || effluent_concentration < 0.0
            || influent_flow <= 0.0
            || effluent_flow < 0.0
        {
            return Err(format!("{parameter} concentration/flow values outside supported domain"));
        }
        let influent_load = influent_concentration * influent_flow / 1_000_000.0;
        let effluent_load = effluent_concentration * effluent_flow / 1_000_000.0;
        if influent_load <= 0.0 {
            return Err(format!("{parameter} influent load must be greater than zero"));
        }
        let removal = (1.0 - effluent_load / influent_load) * 100.0;
        diagnostics.push(format!("{parameter} load removal: {removal:.9}%"));
        if !close(removal, expected, 1e-8) {
            diagnostics.push(format!("{parameter} expected load-removal arithmetic mismatch"));
            push_reason(&mut actual_reason_codes, "INVALID_OR_MISSING_REQUIRED_INPUT");
        }
    }

    // Gross import, recovered energy and net import are kept separately.
    let energy = field(&root, "energy")?;
    require_unit(energy, "kWh/day", &mut diagnostics, &mut actual_reason_codes)?;
    if string(energy, "status")?
        != "synthetic_arithmetic_only_not_an_energy_performance_claim"
    {
        return Err("energy evidence ceiling mismatch".to_owned());
    }
    let gross = number(energy, "gross_import_kwh_day")?;
    let recovered = number(energy, "recovered_kwh_day")?;
    let net = number(energy, "net_import_kwh_day")?;
    if gross < 0.0 || recovered < 0.0 || net < 0.0 || !close(gross - recovered, net, 1e-9) {
        diagnostics.push("gross/recovered/net energy arithmetic mismatch".to_owned());
        push_reason(&mut actual_reason_codes, "INVALID_OR_MISSING_REQUIRED_INPUT");
    }

    // Reuse allocation is not the same as actual delivery; storage change is explicit.
    let disposition = field(&root, "effluent_disposition")?;
    require_unit(disposition, "L/day", &mut diagnostics, &mut actual_reason_codes)?;
    let produced = number(disposition, "effluent_produced_l_day")?;
    let allocated = number(disposition, "allocated_for_reuse_l_day")?;
    let discharged = number(disposition, "discharged_l_day")?;
    let delivered = number(disposition, "reuse_delivered_l_day")?;
    let storage_change = number(disposition, "reuse_storage_change_l_day")?;
    if [produced, allocated, discharged, delivered]
        .iter()
        .any(|value| *value < 0.0)
        || !close(produced, allocated + discharged, 1e-9)
        || !close(allocated, delivered + storage_change, 1e-9)
    {
        diagnostics.push("effluent/reuse allocation arithmetic mismatch".to_owned());
        push_reason(&mut actual_reason_codes, "INVALID_OR_MISSING_REQUIRED_INPUT");
    }

    actual_reason_codes.sort();
    actual_reason_codes.dedup();
    let actual_disposition = if actual_reason_codes.is_empty() {
        "Accepted"
    } else {
        "Rejected"
    }
    .to_owned();

    let oracle_match = actual_disposition == expected_disposition
        && actual_reason_codes == expected_reason_codes;

    Ok(AuditResult {
        case_id,
        actual_disposition,
        expected_disposition,
        actual_reason_codes,
        expected_reason_codes,
        oracle_match,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::audit_case;
    use serde_json::Value;

    const POSITIVE: &str =
        include_str!("../../fixtures/synthetic-accounting-only-v0.1.json");
    const NEGATIVE: &str =
        include_str!("../../fixtures/synthetic-water-balance-negative-v0.1.json");

    #[test]
    fn positive_accounting_fixture_matches_oracle() {
        let report = audit_case(POSITIVE).expect("positive fixture should parse and audit");
        assert_eq!(report.actual_disposition, "Accepted");
        assert!(report.oracle_match, "{report:#?}");
        assert!(report.actual_reason_codes.is_empty());
    }

    #[test]
    fn negative_water_balance_fixture_is_rejected() {
        let report = audit_case(NEGATIVE).expect("negative fixture should parse and audit");
        assert_eq!(report.actual_disposition, "Rejected");
        assert_eq!(
            report.actual_reason_codes,
            vec!["WATER_BALANCE_EXCEEDS_TOLERANCE".to_owned()]
        );
        assert!(report.oracle_match, "{report:#?}");
    }

    #[test]
    fn tampered_positive_fixture_fails_closed() {
        let mut value: Value = serde_json::from_str(POSITIVE).expect("fixture JSON");
        value["water_balance"]["effluent_l_day"] = serde_json::json!(851.0);
        let tampered = serde_json::to_string(&value).expect("serialize tampered fixture");
        let report = audit_case(&tampered).expect("tampered fixture should be auditable");
        assert_eq!(report.actual_disposition, "Rejected");
        assert!(!report.oracle_match);
        assert!(report
            .actual_reason_codes
            .iter()
            .any(|code| code == "WATER_BALANCE_EXCEEDS_TOLERANCE"));
    }

    #[test]
    fn malformed_json_is_rejected() {
        assert!(audit_case("{not-json").is_err());
    }

    #[test]
    fn unsupported_evidence_ceiling_is_rejected() {
        let mut value: Value = serde_json::from_str(POSITIVE).expect("fixture JSON");
        value["evidence_ceiling"] = serde_json::json!("potable_water_certified");
        let tampered = serde_json::to_string(&value).expect("serialize tampered fixture");
        assert!(audit_case(&tampered).is_err());
    }
}
