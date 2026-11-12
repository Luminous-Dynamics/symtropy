// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic accounting-only evaluator for the sanitation T0 corpus.
//!
//! This crate does not model treatment kinetics, pathogens, compliance,
//! hydraulics, plant operation, or physical measurements. It verifies only
//! declared synthetic arithmetic and explicit ledger closure.

use serde_json::{json, Value};
use std::collections::BTreeSet;

const ARITHMETIC_EPSILON: f64 = 1.0e-10;

#[derive(Debug, Clone, PartialEq)]
pub struct ConstituentResult {
    pub constituent: String,
    pub residual_kg_day: f64,
    pub tolerance_kg_day: f64,
    pub within_tolerance: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceResult {
    pub parameter: String,
    pub calculated_load_removal_percent: f64,
    pub declared_load_removal_percent: f64,
    pub within_arithmetic_tolerance: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct T0Report {
    pub evaluator_version: &'static str,
    pub case_id: String,
    pub disposition: &'static str,
    pub reason_codes: Vec<String>,
    pub water_residual_l_day: f64,
    pub water_tolerance_l_day: f64,
    pub water_within_tolerance: bool,
    pub constituent_results: Vec<ConstituentResult>,
    pub performance_results: Vec<PerformanceResult>,
    pub energy_residual_kwh_day: f64,
    pub effluent_allocation_residual_l_day: f64,
    pub reuse_allocation_residual_l_day: f64,
    pub expected_disposition: String,
    pub expected_reason_codes: Vec<String>,
    pub oracle_matches: bool,
    pub evidence_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationError(pub String);

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EvaluationError {}

fn object<'a>(value: &'a Value, name: &str) -> Result<&'a serde_json::Map<String, Value>, EvaluationError> {
    value.as_object().ok_or_else(|| EvaluationError(format!("{name} must be an object")))
}

fn child<'a>(value: &'a Value, key: &str, parent: &str) -> Result<&'a Value, EvaluationError> {
    value
        .get(key)
        .ok_or_else(|| EvaluationError(format!("missing required field {parent}.{key}")))
}

fn string(value: &Value, name: &str) -> Result<String, EvaluationError> {
    let s = value
        .as_str()
        .ok_or_else(|| EvaluationError(format!("{name} must be a string")))?;
    if s.trim().is_empty() {
        return Err(EvaluationError(format!("{name} must not be empty")));
    }
    Ok(s.to_owned())
}

fn number(value: &Value, name: &str) -> Result<f64, EvaluationError> {
    let n = value
        .as_f64()
        .ok_or_else(|| EvaluationError(format!("{name} must be a finite JSON number")))?;
    if !n.is_finite() {
        return Err(EvaluationError(format!("{name} must be finite")));
    }
    Ok(n)
}

fn nonnegative(value: &Value, name: &str) -> Result<f64, EvaluationError> {
    let n = number(value, name)?;
    if n < 0.0 {
        return Err(EvaluationError(format!("{name} must be non-negative")));
    }
    Ok(n)
}

fn field_number(parent: &Value, key: &str, name: &str) -> Result<f64, EvaluationError> {
    number(child(parent, key, name)?, &format!("{name}.{key}"))
}

fn field_nonnegative(parent: &Value, key: &str, name: &str) -> Result<f64, EvaluationError> {
    nonnegative(child(parent, key, name)?, &format!("{name}.{key}"))
}

fn close_enough(actual: f64, expected: f64, scale: f64) -> bool {
    (actual - expected).abs() <= ARITHMETIC_EPSILON * scale.abs().max(1.0)
}

fn expected_codes(value: &Value) -> Result<Vec<String>, EvaluationError> {
    let values = value
        .as_array()
        .ok_or_else(|| EvaluationError("expected_reason_codes must be an array".into()))?;
    let mut codes = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        codes.push(string(value, &format!("expected_reason_codes[{index}]"))?);
    }
    codes.sort();
    codes.dedup();
    Ok(codes)
}

fn require_unit(parent: &Value, key: &str, expected: &str, name: &str) -> Result<(), EvaluationError> {
    let actual = string(child(parent, key, name)?, &format!("{name}.{key}"))?;
    if actual != expected {
        return Err(EvaluationError(format!(
            "unit mismatch at {name}.{key}: expected {expected}, received {actual}"
        )));
    }
    Ok(())
}

fn push_reason(reasons: &mut Vec<String>, code: &str) {
    if !reasons.iter().any(|existing| existing == code) {
        reasons.push(code.to_owned());
    }
}

/// Evaluate one JSON case. A valid negative-control case returns a Rejected
/// report with oracle_matches=true; that is a passing benchmark outcome.
pub fn evaluate_case(input: &str) -> Result<T0Report, EvaluationError> {
    let root: Value = serde_json::from_str(input)
        .map_err(|e| EvaluationError(format!("invalid JSON: {e}")))?;
    let root_obj = object(&root, "case")?;

    let case_id = string(child(&root, "case_id", "case")?, "case.case_id")?;
    let scope = child(&root, "model_scope", "case")?;
    let scope_obj = object(scope, "model_scope")?;
    let process_kinetics = string(child(scope, "process_kinetics", "model_scope")?, "model_scope.process_kinetics")?;
    let pathogen_model = string(child(scope, "pathogen_model", "model_scope")?, "model_scope.pathogen_model")?;
    let physical_measurements = string(child(scope, "physical_measurements", "model_scope")?, "model_scope.physical_measurements")?;
    let purpose = string(child(scope, "purpose", "model_scope")?, "model_scope.purpose")?;

    let expected_disposition = string(
        child(&root, "expected_disposition", "case")?,
        "case.expected_disposition",
    )?;
    if !["Accepted", "Rejected", "Indeterminate", "Unsupported"].contains(&expected_disposition.as_str()) {
        return Err(EvaluationError("unknown expected_disposition".into()));
    }
    let declared_reasons = expected_codes(child(&root, "expected_reason_codes", "case")?)?;

    let mut reasons = Vec::new();
    if process_kinetics != "not_modelled"
        || pathogen_model != "not_modelled"
        || physical_measurements != "none_synthetic_values_only"
        || purpose != "check_declared_arithmetic_and_units"
    {
        push_reason(&mut reasons, "UNSUPPORTED_PROCESS_OR_EVIDENCE_CLASS");
    }

    // Water ledger.
    let water = child(&root, "water_balance", "case")?;
    object(water, "water_balance")?;
    require_unit(water, "unit", "L/day", "water_balance")?;
    let water_in = field_nonnegative(water, "influent_l_day", "water_balance")?;
    let water_effluent = field_nonnegative(water, "effluent_l_day", "water_balance")?;
    let water_sludge = field_nonnegative(water, "sludge_water_l_day", "water_balance")?;
    let water_evaporation = field_nonnegative(water, "evaporation_l_day", "water_balance")?;
    let declared_water_residual = field_number(water, "expected_closure_l_day", "water_balance")?;
    let water_tolerance = field_nonnegative(water, "closure_tolerance_l_day", "water_balance")?;
    let water_residual = water_in - water_effluent - water_sludge - water_evaporation;
    let water_scale = water_in.abs() + water_effluent.abs() + water_sludge.abs() + water_evaporation.abs();
    if !close_enough(water_residual, declared_water_residual, water_scale) {
        return Err(EvaluationError("declared water closure does not match recomputed residual".into()));
    }
    let water_within_tolerance = water_residual.abs() <= water_tolerance;
    if !water_within_tolerance {
        push_reason(&mut reasons, "WATER_BALANCE_EXCEEDS_TOLERANCE");
    }

    // Conserved constituent ledgers.
    let constituent_values = child(&root, "conserved_constituent_balances", "case")?
        .as_array()
        .ok_or_else(|| EvaluationError("conserved_constituent_balances must be an array".into()))?;
    if constituent_values.is_empty() {
        return Err(EvaluationError("conserved_constituent_balances must not be empty".into()));
    }
    let mut seen_constituents = BTreeSet::new();
    let mut constituent_results = Vec::with_capacity(constituent_values.len());
    for (index, balance) in constituent_values.iter().enumerate() {
        let name = format!("conserved_constituent_balances[{index}]");
        object(balance, &name)?;
        let constituent = string(child(balance, "constituent", &name)?, &format!("{name}.constituent"))?;
        if !["TSS", "total_nitrogen", "total_phosphorus"].contains(&constituent.as_str()) {
            return Err(EvaluationError(format!("unsupported conserved constituent: {constituent}")));
        }
        if !seen_constituents.insert(constituent.clone()) {
            return Err(EvaluationError(format!("duplicate constituent: {constituent}")));
        }
        require_unit(balance, "unit", "kg/day", &name)?;
        let input = field_nonnegative(balance, "influent_kg_day", &name)?;
        let effluent = field_nonnegative(balance, "effluent_kg_day", &name)?;
        let sludge = field_nonnegative(balance, "sludge_kg_day", &name)?;
        let other = field_nonnegative(balance, "other_outputs_kg_day", &name)?;
        let declared = field_number(balance, "expected_closure_kg_day", &name)?;
        let tolerance = field_nonnegative(balance, "closure_tolerance_kg_day", &name)?;
        let residual = input - effluent - sludge - other;
        let scale = input.abs() + effluent.abs() + sludge.abs() + other.abs();
        if !close_enough(residual, declared, scale) {
            return Err(EvaluationError(format!(
                "{constituent} declared closure does not match recomputed residual"
            )));
        }
        let within_tolerance = residual.abs() <= tolerance;
        if !within_tolerance {
            push_reason(&mut reasons, "CONSTITUENT_BALANCE_EXCEEDS_TOLERANCE");
        }
        constituent_results.push(ConstituentResult {
            constituent,
            residual_kg_day: residual,
            tolerance_kg_day: tolerance,
            within_tolerance,
        });
    }

    // BOD5/COD are load-based performance indicators, not conserved species.
    let metrics = child(&root, "nonconservative_performance_metrics", "case")?
        .as_array()
        .ok_or_else(|| EvaluationError("nonconservative_performance_metrics must be an array".into()))?;
    if metrics.is_empty() {
        return Err(EvaluationError("nonconservative_performance_metrics must not be empty".into()));
    }
    let mut seen_metrics = BTreeSet::new();
    let mut performance_results = Vec::with_capacity(metrics.len());
    for (index, metric) in metrics.iter().enumerate() {
        let name = format!("nonconservative_performance_metrics[{index}]");
        object(metric, &name)?;
        let parameter = string(child(metric, "parameter", &name)?, &format!("{name}.parameter"))?;
        if !["BOD5", "COD"].contains(&parameter.as_str()) {
            return Err(EvaluationError(format!("unsupported performance parameter: {parameter}")));
        }
        if !seen_metrics.insert(parameter.clone()) {
            return Err(EvaluationError(format!("duplicate performance parameter: {parameter}")));
        }
        require_unit(metric, "unit", "mg/L", &name)?;
        let influent_concentration = field_nonnegative(metric, "influent_mg_l", &name)?;
        let effluent_concentration = field_nonnegative(metric, "effluent_mg_l", &name)?;
        let influent_flow = field_nonnegative(metric, "influent_flow_l_day", &name)?;
        let effluent_flow = field_nonnegative(metric, "effluent_flow_l_day", &name)?;
        if influent_flow <= 0.0 || influent_concentration <= 0.0 {
            return Err(EvaluationError(format!(
                "{parameter} load removal is undefined for zero influent load or flow"
            )));
        }
        let declared = field_number(metric, "expected_load_removal_percent", &name)?;
        let interpretation = string(child(metric, "interpretation", &name)?, &format!("{name}.interpretation"))?;
        if interpretation != "calculated_metric_not_a_conserved_species_or_compliance_result" {
            return Err(EvaluationError(format!("invalid interpretation boundary for {parameter}")));
        }
        let influent_load = influent_concentration * influent_flow;
        let effluent_load = effluent_concentration * effluent_flow;
        let calculated = (1.0 - effluent_load / influent_load) * 100.0;
        if !calculated.is_finite() || !declared.is_finite() {
            return Err(EvaluationError(format!("{parameter} load calculation is non-finite")));
        }
        let within_arithmetic_tolerance = close_enough(calculated, declared, calculated.abs().max(declared.abs()).max(100.0));
        if !within_arithmetic_tolerance {
            return Err(EvaluationError(format!(
                "{parameter} declared load-removal value does not match recomputed arithmetic"
            )));
        }
        performance_results.push(PerformanceResult {
            parameter,
            calculated_load_removal_percent: calculated,
            declared_load_removal_percent: declared,
            within_arithmetic_tolerance,
        });
    }

    // Meter-boundary arithmetic. This v0.1 contract represents recovered
    // energy as offsetting imports and has no export stream; reject impossible
    // values until an explicit export field is added in a new schema version.
    let energy = child(&root, "energy", "case")?;
    object(energy, "energy")?;
    require_unit(energy, "unit", "kWh/day", "energy")?;
    let gross_import = field_nonnegative(energy, "gross_import_kwh_day", "energy")?;
    let recovered = field_nonnegative(energy, "recovered_kwh_day", "energy")?;
    let net_import = field_nonnegative(energy, "net_import_kwh_day", "energy")?;
    let meter_boundary = string(child(energy, "meter_boundary", "energy")?, "energy.meter_boundary")?;
    if meter_boundary.trim().is_empty() {
        return Err(EvaluationError("energy meter boundary must be declared".into()));
    }
    let energy_status = string(child(energy, "status", "energy")?, "energy.status")?;
    if energy_status != "synthetic_arithmetic_only_not_an_energy_performance_claim" {
        return Err(EvaluationError("energy evidence ceiling is invalid".into()));
    }
    if recovered > gross_import {
        return Err(EvaluationError("recovered energy exceeds imports but v0.1 has no export stream".into()));
    }
    let energy_residual = gross_import - recovered - net_import;
    if !close_enough(energy_residual, 0.0, gross_import.abs() + recovered.abs() + net_import.abs()) {
        return Err(EvaluationError("energy ledger does not close".into()));
    }

    // Effluent is allocated to reuse or discharge; reused water is either
    // delivered or changes storage. These are disposition balances, not
    // quality or permission decisions.
    let disposition = child(&root, "effluent_disposition", "case")?;
    object(disposition, "effluent_disposition")?;
    require_unit(disposition, "unit", "L/day", "effluent_disposition")?;
    let produced = field_nonnegative(disposition, "effluent_produced_l_day", "effluent_disposition")?;
    let reuse_allocated = field_nonnegative(disposition, "allocated_for_reuse_l_day", "effluent_disposition")?;
    let discharged = field_nonnegative(disposition, "discharged_l_day", "effluent_disposition")?;
    let reuse_delivered = field_nonnegative(disposition, "reuse_delivered_l_day", "effluent_disposition")?;
    let storage_change = field_number(disposition, "reuse_storage_change_l_day", "effluent_disposition")?;
    let effluent_allocation_residual = produced - reuse_allocated - discharged;
    let reuse_allocation_residual = reuse_allocated - reuse_delivered - storage_change;
    if !close_enough(effluent_allocation_residual, 0.0, produced.abs() + reuse_allocated.abs() + discharged.abs()) {
        return Err(EvaluationError("effluent allocation does not close".into()));
    }
    if !close_enough(reuse_allocation_residual, 0.0, reuse_allocated.abs() + reuse_delivered.abs() + storage_change.abs()) {
        return Err(EvaluationError("reuse allocation does not close".into()));
    }
    
    reasons.sort();
    reasons.dedup();
    let disposition = if reasons.iter().any(|code| code == "UNSUPPORTED_PROCESS_OR_EVIDENCE_CLASS") {
        "Unsupported"
    } else if reasons.is_empty() {
        "Accepted"
    } else {
        "Rejected"
    };
    let oracle_matches = disposition == expected_disposition
        && reasons == declared_reasons;

    // Silence unused-binding warnings without introducing semantic defaults.
    let _ = root_obj;

    Ok(T0Report {
        evaluator_version: "0.1.0",
        case_id,
        disposition,
        reason_codes: reasons,
        water_residual_l_day: water_residual,
        water_tolerance_l_day: water_tolerance,
        water_within_tolerance,
        constituent_results,
        performance_results,
        energy_residual_kwh_day: energy_residual,
        effluent_allocation_residual_l_day: effluent_allocation_residual,
        reuse_allocation_residual_l_day: reuse_allocation_residual,
        expected_disposition,
        expected_reason_codes: declared_reasons,
        oracle_matches,
        evidence_ceiling: "synthetic_arithmetic_only_no_physical_or_safety_claim",
    })
}

impl T0Report {
    pub fn to_json(&self) -> Value {
        json!({
            "evaluator_version": self.evaluator_version,
            "case_id": self.case_id,
            "disposition": self.disposition,
            "reason_codes": self.reason_codes,
            "water": {
                "residual_l_day": self.water_residual_l_day,
                "tolerance_l_day": self.water_tolerance_l_day,
                "within_tolerance": self.water_within_tolerance
            },
            "constituents": self.constituent_results.iter().map(|r| json!({
                "constituent": r.constituent,
                "residual_kg_day": r.residual_kg_day,
                "tolerance_kg_day": r.tolerance_kg_day,
                "within_tolerance": r.within_tolerance
            })).collect::<Vec<_>>(),
            "performance_metrics": self.performance_results.iter().map(|r| json!({
                "parameter": r.parameter,
                "calculated_load_removal_percent": r.calculated_load_removal_percent,
                "declared_load_removal_percent": r.declared_load_removal_percent,
                "within_arithmetic_tolerance": r.within_arithmetic_tolerance
            })).collect::<Vec<_>>(),
            "energy_residual_kwh_day": self.energy_residual_kwh_day,
            "effluent_allocation_residual_l_day": self.effluent_allocation_residual_l_day,
            "reuse_allocation_residual_l_day": self.reuse_allocation_residual_l_day,
            "expected_disposition": self.expected_disposition,
            "expected_reason_codes": self.expected_reason_codes,
            "oracle_matches": self.oracle_matches,
            "evidence_ceiling": self.evidence_ceiling
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const POSITIVE: &str = include_str!("../../fixtures/synthetic-accounting-only-v0.1.json");
    const NEGATIVE: &str = include_str!("../../fixtures/synthetic-water-balance-negative-v0.1.json");

    fn mutated_case(input: &str, mutate: impl FnOnce(&mut Value)) -> String {
        let mut value: Value = serde_json::from_str(input).expect("fixture JSON must parse");
        mutate(&mut value);
        serde_json::to_string(&value).expect("mutated JSON must serialize")
    }

    #[test]
    fn accepts_balanced_synthetic_reference_case() {
        let report = evaluate_case(POSITIVE).expect("positive fixture is structurally valid");
        assert_eq!(report.disposition, "Accepted");
        assert!(report.oracle_matches);
        assert!(report.water_within_tolerance);
        assert_eq!(report.constituent_results.len(), 3);
        assert_eq!(report.performance_results.len(), 2);
    }

    #[test]
    fn rejects_unbalanced_negative_control_as_expected() {
        let report = evaluate_case(NEGATIVE).expect("negative fixture is structurally valid");
        assert_eq!(report.disposition, "Rejected");
        assert_eq!(report.reason_codes, vec!["WATER_BALANCE_EXCEEDS_TOLERANCE"]);
        assert!(report.oracle_matches);
        assert!(!report.water_within_tolerance);
    }

    #[test]
    fn catches_expected_disposition_tampering() {
        let changed = mutated_case(NEGATIVE, |case| {
            case["expected_disposition"] = Value::String("Accepted".into());
            case["expected_reason_codes"] = json!([]);
        });
        let report = evaluate_case(&changed).expect("arithmetic remains parseable");
        assert_eq!(report.disposition, "Rejected");
        assert!(!report.oracle_matches);
    }

    #[test]
    fn rejects_unit_mismatch() {
        let changed = mutated_case(POSITIVE, |case| {
            case["water_balance"]["unit"] = Value::String("m3/day".into());
        });
        assert!(evaluate_case(&changed).unwrap_err().to_string().contains("unit mismatch"));
    }

    #[test]
    fn rejects_negative_mass_input() {
        let changed = mutated_case(POSITIVE, |case| {
            case["conserved_constituent_balances"][0]["influent_kg_day"] = json!(-0.1);
        });
        assert!(evaluate_case(&changed).is_err());
    }

    #[test]
    fn rejects_duplicate_constituent_entries() {
        let changed = mutated_case(POSITIVE, |case| {
            let first = case["conserved_constituent_balances"][0].clone();
            case["conserved_constituent_balances"].as_array_mut().unwrap().push(first);
        });
        assert!(evaluate_case(&changed).unwrap_err().to_string().contains("duplicate constituent"));
    }

    #[test]
    fn rejects_bod_load_removal_arithmetic_tampering() {
        let changed = mutated_case(POSITIVE, |case| {
            case["nonconservative_performance_metrics"][0]["expected_load_removal_percent"] = json!(99.0);
        });
        assert!(evaluate_case(&changed).unwrap_err().to_string().contains("declared load-removal"));
    }

    #[test]
    fn rejects_energy_ledger_mismatch() {
        let changed = mutated_case(POSITIVE, |case| {
            case["energy"]["net_import_kwh_day"] = json!(0.01);
        });
        assert!(evaluate_case(&changed).unwrap_err().to_string().contains("energy ledger"));
    }

    #[test]
    fn rejects_effluent_reuse_allocation_mismatch() {
        let changed = mutated_case(POSITIVE, |case| {
            case["effluent_disposition"]["reuse_delivered_l_day"] = json!(750.0);
        });
        assert!(evaluate_case(&changed).unwrap_err().to_string().contains("reuse allocation"));
    }

    #[test]
    fn never_certifies_physical_or_pathogen_safety() {
        let report = evaluate_case(POSITIVE).expect("fixture evaluates");
        assert_eq!(report.evidence_ceiling, "synthetic_arithmetic_only_no_physical_or_safety_claim");
    }
}
