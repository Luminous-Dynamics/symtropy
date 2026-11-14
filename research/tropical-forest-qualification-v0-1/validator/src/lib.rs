//! Offline semantic verifier for the synthetic tropical-forest qualification fixture.
//!
//! This validates arithmetic and causal-isolation properties of a declared synthetic
//! case. It does not estimate ecological parameters, validate a biological model, or
//! turn a synthetic pass into field evidence.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as FmtWrite;

#[derive(Debug, Clone)]
struct Check {
    id: &'static str,
    passed: bool,
    detail: String,
}

fn push_check(checks: &mut Vec<Check>, id: &'static str, passed: bool, detail: impl Into<String>) {
    checks.push(Check {
        id,
        passed,
        detail: detail.into(),
    });
}

fn at<'a>(root: &'a Value, path: &[&str]) -> Result<&'a Value, String> {
    let mut current = root;
    for key in path {
        let next = if let Ok(index) = key.parse::<usize>() {
            current.get(index)
        } else {
            current.get(*key)
        };
        current = next.ok_or_else(|| format!("missing required field: {}", path.join(".")))?;
    }
    Ok(current)
}

fn string_at(root: &Value, path: &[&str]) -> Result<String, String> {
    at(root, path)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("expected string at {}", path.join(".")))
}

fn number_at(root: &Value, path: &[&str]) -> Result<f64, String> {
    let value = at(root, path)?
        .as_f64()
        .ok_or_else(|| format!("expected number at {}", path.join(".")))?;
    if !value.is_finite() {
        return Err(format!("non-finite number at {}", path.join(".")));
    }
    Ok(value)
}

fn object_at<'a>(
    root: &'a Value,
    path: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, String> {
    at(root, path)?
        .as_object()
        .ok_or_else(|| format!("expected object at {}", path.join(".")))
}

fn close_enough(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance
}

fn same_biomass_stocks(before: &Value, after: &Value) -> Result<bool, String> {
    for key in [
        "live_biomass_kg_dry_matter",
        "deadwood_biomass_kg_dry_matter",
    ] {
        let before_value = before
            .get(key)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("missing or non-numeric initial stock {key}"))?;
        let after_value = after
            .get(key)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("missing or non-numeric expected stock {key}"))?;
        if before_value != after_value {
            return Ok(false);
        }
    }
    // Derived totals are checked independently; they are not canonical stocks.
    Ok(true)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn check_cell_ids(
    manifest: &Value,
) -> Result<
    (
        BTreeSet<String>,
        BTreeMap<String, Value>,
        BTreeMap<String, Value>,
    ),
    String,
> {
    let cells = at(manifest, &["spatial_support", "cells"])?
        .as_array()
        .ok_or_else(|| "spatial_support.cells must be an array".to_string())?;

    let mut declared = BTreeSet::new();
    for (index, cell) in cells.iter().enumerate() {
        let id = cell
            .get("cell_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("spatial_support.cells[{index}].cell_id must be a string"))?;
        if !declared.insert(id.to_string()) {
            return Err(format!("duplicate spatial cell id: {id}"));
        }
    }

    let initial: BTreeMap<String, Value> = object_at(manifest, &["initial_state"])?
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let expected: BTreeMap<String, Value> = object_at(manifest, &["expected_state"])?
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();

    let initial_ids: BTreeSet<String> = initial.keys().cloned().collect();
    let expected_ids: BTreeSet<String> = expected.keys().cloned().collect();
    if declared != initial_ids {
        return Err("declared spatial cell ids do not exactly match initial_state keys".to_string());
    }
    if declared != expected_ids {
        return Err("declared spatial cell ids do not exactly match expected_state keys".to_string());
    }
    Ok((declared, initial, expected))
}

fn causally_reachable_cells(
    manifest: &Value,
    start: &str,
    declared_cells: &BTreeSet<String>,
) -> Result<BTreeSet<String>, String> {
    let links = at(manifest, &["spatial_support", "explicit_causal_links"])?
        .as_array()
        .ok_or_else(|| "spatial_support.explicit_causal_links must be an array".to_string())?;

    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (index, link) in links.iter().enumerate() {
        let from = link
            .get("from_cell_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("explicit_causal_links[{index}].from_cell_id must be a string"))?;
        let to = link
            .get("to_cell_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("explicit_causal_links[{index}].to_cell_id must be a string"))?;
        if !declared_cells.contains(from) || !declared_cells.contains(to) {
            return Err(format!(
                "causal link {from} -> {to} references an undeclared spatial cell"
            ));
        }
        graph.entry(from.to_string()).or_default().push(to.to_string());
    }

    let mut reachable = BTreeSet::new();
    let mut queue = VecDeque::new();
    reachable.insert(start.to_string());
    queue.push_back(start.to_string());

    while let Some(current) = queue.pop_front() {
        if let Some(next) = graph.get(&current) {
            for destination in next {
                if reachable.insert(destination.clone()) {
                    queue.push_back(destination.clone());
                }
            }
        }
    }
    Ok(reachable)
}

/// Evaluate a complete JSON fixture and return a deterministic, machine-readable receipt.
///
/// Structurally malformed JSON returns Err. A well-formed manifest with failed semantic
/// invariants returns a receipt whose result is FAIL.
pub fn evaluate(raw: &[u8]) -> Result<Value, String> {
    let manifest: Value =
        serde_json::from_slice(raw).map_err(|error| format!("invalid JSON manifest: {error}"))?;

    let schema_version = string_at(&manifest, &["schema_version"])?;
    let benchmark_id = string_at(&manifest, &["benchmark_id"])?;
    let case_id = string_at(&manifest, &["case_id"])?;
    let evidence_class = string_at(&manifest, &["evidence_class"])?;
    let claim_ceiling = string_at(&manifest, &["claim_ceiling"])?;

    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "SCHEMA-VERSION-001",
        schema_version == "0.1.0",
        format!("schema_version={schema_version}; expected 0.1.0"),
    );
    push_check(
        &mut checks,
        "BENCHMARK-IDENTITY-001",
        benchmark_id == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        format!("benchmark_id={benchmark_id}"),
    );
    push_check(
        &mut checks,
        "SYNTHETIC-CLAIM-CEILING-001",
        evidence_class == "synthetic_known_truth"
            && claim_ceiling.contains("does not calibrate biological parameters"),
        "fixture must remain explicitly synthetic and disclaim biological calibration",
    );

    let region_kind = string_at(&manifest, &["region_profile", "kind"])?;
    let external_data_used = at(&manifest, &["provenance", "external_data_used"])?
        .as_bool()
        .ok_or_else(|| "provenance.external_data_used must be boolean".to_string())?;
    let rights_status = string_at(&manifest, &["provenance", "data_rights_status"])?;
    push_check(
        &mut checks,
        "NO-REAL-DATA-CLAIM-001",
        region_kind == "synthetic"
            && !external_data_used
            && rights_status == "not_applicable_synthetic",
        "fixture must not imply real geography, external observations, or external-data rights",
    );

    let (cell_ids, initial, expected) = check_cell_ids(&manifest)?;
    push_check(
        &mut checks,
        "SPATIAL-IDENTITY-001",
        !cell_ids.is_empty() && initial.len() == cell_ids.len() && expected.len() == cell_ids.len(),
        format!(
            "{} unique cells have exactly one initial and expected state",
            cell_ids.len()
        ),
    );

    let interventions = at(&manifest, &["interventions"])?
        .as_array()
        .ok_or_else(|| "interventions must be an array".to_string())?;
    if interventions.len() != 1 {
        return Err("v0.1 validator supports exactly one intervention per fixture".to_string());
    }
    let intervention = &interventions[0];
    let target = intervention
        .get("target_cell_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "interventions[0].target_cell_id must be a string".to_string())?;
    if !cell_ids.contains(target) {
        return Err(format!("intervention target is not a declared cell: {target}"));
    }

    let parameter_growth = number_at(&manifest, &["parameters", "growth_flux_per_step"])?;
    let parameter_mortality = number_at(&manifest, &["parameters", "mortality_flux_per_step"])?;
    let parameter_decomposition =
        number_at(&manifest, &["parameters", "decomposition_flux_per_step"])?;
    let effect_growth = number_at(
        &manifest,
        &[
            "interventions",
            "0",
            "declared_effects",
            "live_growth_kg_dry_matter",
        ],
    )?;
    let effect_transfer = number_at(
        &manifest,
        &[
            "interventions",
            "0",
            "declared_effects",
            "live_to_deadwood_transfer_kg_dry_matter",
        ],
    )?;

    let nonnegative_parameters = parameter_growth >= 0.0
        && parameter_mortality >= 0.0
        && parameter_decomposition >= 0.0
        && effect_growth >= 0.0
        && effect_transfer >= 0.0;
    push_check(
        &mut checks,
        "NONNEGATIVE-FLUXES-001",
        nonnegative_parameters,
        "all declared synthetic growth, mortality, and decomposition fluxes must be non-negative",
    );
    push_check(
        &mut checks,
        "PARAMETER-EFFECT-CONSISTENCY-001",
        close_enough(parameter_growth, effect_growth, 1e-9)
            && close_enough(parameter_mortality, effect_transfer, 1e-9),
        format!(
            "growth parameter/effect={parameter_growth}/{effect_growth}; mortality parameter/transfer={parameter_mortality}/{effect_transfer}"
        ),
    );
    push_check(
        &mut checks,
        "DECOMPOSITION-SCOPE-001",
        close_enough(parameter_decomposition, 0.0, 1e-12),
        "v0.1 represents live biomass and deadwood only; decomposition must remain zero until a receiving nutrient pool is modelled",
    );

    let initial_target = initial
        .get(target)
        .ok_or_else(|| format!("initial state missing intervention target {target}"))?;
    let expected_target = expected
        .get(target)
        .ok_or_else(|| format!("expected state missing intervention target {target}"))?;

    let start_live = initial_target
        .get("live_biomass_kg_dry_matter")
        .and_then(Value::as_f64)
        .ok_or_else(|| "target live biomass must be numeric".to_string())?;
    let start_dead = initial_target
        .get("deadwood_biomass_kg_dry_matter")
        .and_then(Value::as_f64)
        .ok_or_else(|| "target deadwood biomass must be numeric".to_string())?;
    let end_live = expected_target
        .get("live_biomass_kg_dry_matter")
        .and_then(Value::as_f64)
        .ok_or_else(|| "expected target live biomass must be numeric".to_string())?;
    let end_dead = expected_target
        .get("deadwood_biomass_kg_dry_matter")
        .and_then(Value::as_f64)
        .ok_or_else(|| "expected target deadwood biomass must be numeric".to_string())?;
    let end_total = expected_target
        .get("total_biomass_kg_dry_matter")
        .and_then(Value::as_f64)
        .ok_or_else(|| "expected target total biomass must be numeric".to_string())?;

    let tolerance = number_at(
        &manifest,
        &["invariants", "0", "tolerance_kg_dry_matter"],
    )?;
    let predicted_live = start_live + effect_growth - effect_transfer;
    let predicted_dead = start_dead + effect_transfer;
    let predicted_total = predicted_live + predicted_dead;
    let residual = predicted_total - (start_live + start_dead + effect_growth);

    let mut all_stocks_nonnegative = true;
    let mut all_expected_totals_consistent = true;
    for cell_id in &cell_ids {
        let before = initial
            .get(cell_id)
            .ok_or_else(|| format!("initial state missing {cell_id}"))?;
        let after = expected
            .get(cell_id)
            .ok_or_else(|| format!("expected state missing {cell_id}"))?;
        let before_live = before
            .get("live_biomass_kg_dry_matter")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("initial live biomass missing or non-numeric for {cell_id}"))?;
        let before_dead = before
            .get("deadwood_biomass_kg_dry_matter")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("initial deadwood missing or non-numeric for {cell_id}"))?;
        let after_live = after
            .get("live_biomass_kg_dry_matter")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("expected live biomass missing or non-numeric for {cell_id}"))?;
        let after_dead = after
            .get("deadwood_biomass_kg_dry_matter")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("expected deadwood missing or non-numeric for {cell_id}"))?;
        let after_total = after
            .get("total_biomass_kg_dry_matter")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("expected total biomass missing or non-numeric for {cell_id}"))?;
        all_stocks_nonnegative &= before_live >= 0.0
            && before_dead >= 0.0
            && after_live >= 0.0
            && after_dead >= 0.0;
        all_expected_totals_consistent &=
            close_enough(after_total, after_live + after_dead, tolerance);
    }
    push_check(
        &mut checks,
        "BIOMASS-STOCKS-NONNEGATIVE-001",
        all_stocks_nonnegative,
        "initial and expected live/deadwood stocks must be non-negative for every cell",
    );
    push_check(
        &mut checks,
        "CELL-TOTALS-CONSISTENT-001",
        all_expected_totals_consistent,
        "each expected cell total must equal live biomass plus deadwood",
    );
    push_check(
        &mut checks,
        "LIVE-BIOMASS-TRANSITION-001",
        close_enough(end_live, predicted_live, tolerance),
        format!("expected live={end_live}; derived live={predicted_live}"),
    );
    push_check(
        &mut checks,
        "DEADWOOD-TRANSITION-001",
        close_enough(end_dead, predicted_dead, tolerance),
        format!("expected deadwood={end_dead}; derived deadwood={predicted_dead}"),
    );
    push_check(
        &mut checks,
        "BIOMASS-TOTAL-001",
        close_enough(end_total, predicted_total, tolerance),
        format!("expected total={end_total}; derived total={predicted_total}"),
    );
    push_check(
        &mut checks,
        "BIOMASS-CONSERVATION-001",
        tolerance >= 0.0 && residual.abs() <= tolerance,
        format!("budget residual={residual} kg dry matter; tolerance={tolerance}"),
    );

    let transferred_from_live = start_live + effect_growth - predicted_live;
    let transferred_to_deadwood = predicted_dead - start_dead;
    push_check(
        &mut checks,
        "MORTALITY-TRANSFER-001",
        close_enough(transferred_from_live, effect_transfer, tolerance)
            && close_enough(transferred_to_deadwood, effect_transfer, tolerance),
        format!(
            "declared transfer={effect_transfer}; removed from live={transferred_from_live}; added to deadwood={transferred_to_deadwood}"
        ),
    );

    let reachable = causally_reachable_cells(&manifest, target, &cell_ids)?;
    let mut unchanged_controls = Vec::new();
    for cell_id in &cell_ids {
        if reachable.contains(cell_id) {
            continue;
        }
        let before = initial
            .get(cell_id)
            .ok_or_else(|| format!("initial state missing {cell_id}"))?;
        let after = expected
            .get(cell_id)
            .ok_or_else(|| format!("expected state missing {cell_id}"))?;
        unchanged_controls.push(cell_id.clone());
        if !same_biomass_stocks(before, after)? {
            push_check(
                &mut checks,
                "DISCONNECTED-CONTROL-001",
                false,
                format!("cell {cell_id} changed canonical biomass stocks despite no declared causal path"),
            );
        }
    }
    if !checks
        .iter()
        .any(|check| check.id == "DISCONNECTED-CONTROL-001" && !check.passed)
    {
        push_check(
            &mut checks,
            "DISCONNECTED-CONTROL-001",
            !unchanged_controls.is_empty(),
            format!(
                "{} unreachable cells remained exactly unchanged",
                unchanged_controls.len()
            ),
        );
    }

    let passed = checks.iter().all(|check| check.passed);
    let check_values: Vec<Value> = checks
        .iter()
        .map(|check| {
            json!({
                "id": check.id,
                "passed": check.passed,
                "detail": check.detail
            })
        })
        .collect();

    Ok(json!({
        "receipt_version": "0.1.0",
        "validator": "symtropy-tropical-forest-qualifier",
        "benchmark_id": benchmark_id,
        "case_id": case_id,
        "evidence_class": evidence_class,
        "claim_ceiling": claim_ceiling,
        "source_manifest_sha256": sha256_hex(raw),
        "result": if passed { "PASS" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "regional_or_field_validation": "NOT_ESTABLISHED",
        "ecological_parameter_calibration": "NOT_ESTABLISHED"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FIXTURE: &str =
        include_str!("../../cases/forest-patch-accounting-synthetic-001.json");

    #[test]
    fn committed_fixture_passes_all_synthetic_contract_checks() {
        let receipt = evaluate(FIXTURE.as_bytes()).expect("fixture should parse");
        assert_eq!(receipt["result"], "PASS");
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(receipt["regional_or_field_validation"], "NOT_ESTABLISHED");
        assert_eq!(receipt["ecological_parameter_calibration"], "NOT_ESTABLISHED");
    }

    #[test]
    fn disconnected_control_mutation_is_rejected() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("valid fixture");
        manifest["expected_state"]["disconnected-control-cell"]
            ["live_biomass_kg_dry_matter"] = json!(51.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated fixture");
        let receipt =
            evaluate(&bytes).expect("mutated fixture should remain structurally valid");
        assert_eq!(receipt["result"], "FAIL");
        assert!(receipt["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["id"] == "DISCONNECTED-CONTROL-001"
                && check["passed"] == false));
    }

    #[test]
    fn inconsistent_declared_mortality_parameter_is_rejected() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("valid fixture");
        manifest["parameters"]["mortality_flux_per_step"] = json!(6.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated fixture");
        let receipt =
            evaluate(&bytes).expect("mutated fixture should remain structurally valid");
        assert_eq!(receipt["result"], "FAIL");
        assert!(receipt["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["id"] == "PARAMETER-EFFECT-CONSISTENCY-001"
                && check["passed"] == false));
    }

    #[test]
    fn identical_inputs_produce_identical_receipts() {
        let first = evaluate(FIXTURE.as_bytes()).expect("fixture should parse");
        let second = evaluate(FIXTURE.as_bytes()).expect("fixture should parse");
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn malformed_json_is_rejected_before_scoring() {
        assert!(evaluate(b"{not-json").is_err());
    }
}
