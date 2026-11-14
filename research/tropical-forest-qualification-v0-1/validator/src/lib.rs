// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Offline semantic verifier for the synthetic tropical-forest qualification fixture.
//!
//! This validates arithmetic and causal-isolation properties of a declared synthetic
//! case. It does not estimate ecological parameters, validate a biological model, or
//! turn a synthetic pass into field evidence.

pub mod canopy_kernel;

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

fn usize_at(root: &Value, path: &[&str]) -> Result<usize, String> {
    let value = at(root, path)?
        .as_u64()
        .ok_or_else(|| format!("expected a non-negative integer at {}", path.join(".")))?;
    usize::try_from(value)
        .map_err(|_| format!("integer at {} exceeds platform usize", path.join(".")))
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

fn parse_coordinates(
    value: &Value,
    label: &str,
    width: usize,
    height: usize,
) -> Result<(BTreeSet<(usize, usize)>, usize), String> {
    let coordinates = value
        .as_array()
        .ok_or_else(|| format!("{label} must be an array"))?;
    let mut set = BTreeSet::new();
    let mut duplicates = 0;
    for (index, coordinate) in coordinates.iter().enumerate() {
        let pair = coordinate
            .as_array()
            .ok_or_else(|| format!("{label}[{index}] must be a two-item coordinate"))?;
        if pair.len() != 2 {
            return Err(format!("{label}[{index}] must contain exactly two coordinates"));
        }
        let x_raw = pair[0]
            .as_u64()
            .ok_or_else(|| format!("{label}[{index}][0] must be a non-negative integer"))?;
        let y_raw = pair[1]
            .as_u64()
            .ok_or_else(|| format!("{label}[{index}][1] must be a non-negative integer"))?;
        let x = usize::try_from(x_raw)
            .map_err(|_| format!("{label}[{index}][0] exceeds platform usize"))?;
        let y = usize::try_from(y_raw)
            .map_err(|_| format!("{label}[{index}][1] exceeds platform usize"))?;
        if x >= width || y >= height {
            return Err(format!("{label}[{index}] lies outside the declared grid"));
        }
        if !set.insert((x, y)) {
            duplicates += 1;
        }
    }
    Ok((set, duplicates))
}

fn open_component_count(
    open_cells: &BTreeSet<(usize, usize)>,
) -> usize {
    let mut remaining = open_cells.clone();
    let mut components = 0;
    while let Some(start) = remaining.pop_first() {
        components += 1;
        let mut queue = VecDeque::new();
        queue.push_back(start);
        while let Some((x, y)) = queue.pop_front() {
            let neighbors = [
                (x.wrapping_sub(1), y),
                (x.saturating_add(1), y),
                (x, y.wrapping_sub(1)),
                (x, y.saturating_add(1)),
            ];
            for neighbor in neighbors {
                if remaining.remove(&neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
    }
    components
}

fn adjudicate_observation_case(case: &Value) -> Result<&'static str, String> {
    let id = string_at(case, &["case_id"])?;
    match id.as_str() {
        "PLOT-GEDI-SUPPORT-MISMATCH-001" => {
            let left = string_at(case, &["input", "left_support"])?;
            let right = string_at(case, &["input", "right_support"])?;
            let overlap_applied = at(case, &["input", "overlap_operator_applied"])?
                .as_bool().ok_or_else(|| "overlap_operator_applied must be boolean".to_string())?;
            let conversion_declared = at(case, &["input", "support_conversion_declared"])?
                .as_bool().ok_or_else(|| "support_conversion_declared must be boolean".to_string())?;
            Ok(if left != right && !overlap_applied && !conversion_declared {
                "BLOCKED_SUPPORT_MISMATCH"
            } else {
                "SUPPORT_MISMATCH_CONTROL_NOT_TRIGGERED"
            })
        }
        "GEDI-ISS-STORAGE-GAP-001" => {
            let status = string_at(case, &["input", "acquisition_status"])?;
            Ok(if status == "no_acquisition" {
                "UNOBSERVED_NO_ACQUISITION"
            } else {
                "ACQUISITION_PRESENT_REQUIRES_QUALITY_REVIEW"
            })
        }
        "TMF-DEGRADATION-NOT-MORTALITY-001" => {
            let source_class = string_at(case, &["input", "source_class"])?;
            let target = string_at(case, &["input", "target_variable"])?;
            Ok(if source_class == "degraded_forest" && target == "tree_mortality" {
                "REJECT_LABEL_SEMANTIC_MISMATCH"
            } else {
                "LABEL_SEMANTIC_MISMATCH_CONTROL_NOT_TRIGGERED"
            })
        }
        "MAPBIOMAS-BRAZIL-TRANSFER-001" => {
            let source = string_at(case, &["input", "source_coverage_country"])?;
            let target = string_at(case, &["input", "target_country"])?;
            Ok(if source != target { "OUT_OF_DOMAIN" } else { "TARGET_WITHIN_DECLARED_COUNTRY" })
        }
        "CLOUD-OR-NODATA-NO-CHANGE-001" => {
            let validity = string_at(case, &["input", "validity_state"])?;
            Ok(if validity == "cloud_masked" || validity == "no_data" || validity == "qa_invalid" {
                "UNOBSERVED_CLOUD_OR_QA_MASK"
            } else {
                "OBSERVATION_VALID_REQUIRES_SOURCE_OPERATOR"
            })
        }
        "PLOT-CAMPAIGN-LEAKAGE-001" => {
            let unit = string_at(case, &["input", "split_group_unit"])?;
            if unit.trim().is_empty() {
                return Err("split_group_unit must not be empty".to_string());
            }
            let same_plot = at(case, &["input", "same_plot_across_splits"])?
                .as_bool().ok_or_else(|| "same_plot_across_splits must be boolean".to_string())?;
            Ok(if same_plot { "REJECT_LEAKAGE" } else { "NO_WITHIN_PLOT_SPLIT_CONFLICT_DETECTED" })
        }
        "DRC-INACCESSIBLE-SAMPLE-UNITS-001" => {
            let status = string_at(case, &["input", "survey_status"])?;
            let reason = string_at(case, &["input", "unsurveyed_reason"])?;
            Ok(if status == "not_surveyed" && (reason == "inaccessible" || reason == "conflict_affected" || reason == "inaccessible_conflict_affected") {
                "UNOBSERVED_INACCESSIBLE_OR_CONFLICT_AFFECTED"
            } else {
                "SURVEYED_OR_OTHER_MISSINGNESS_REQUIRES_REVIEW"
            })
        }
        "PNG-SECURITY-LOGISTICS-OMISSIONS-001" => {
            let status = string_at(case, &["input", "survey_status"])?;
            let reason = string_at(case, &["input", "unsurveyed_reason"])?;
            Ok(if status == "not_surveyed" && (reason == "security" || reason == "logistics" || reason == "security_or_logistics") {
                "UNOBSERVED_SECURITY_OR_LOGISTICS"
            } else {
                "SURVEYED_OR_OTHER_MISSINGNESS_REQUIRES_REVIEW"
            })
        }
        "SINGLE-CENSUS-AS-LONGITUDINAL-PANEL-001" => {
            let count = usize_at(case, &["input", "census_count"])?;
            let repeats = at(case, &["input", "matched_repeat_measurements"])?
                .as_bool().ok_or_else(|| "matched_repeat_measurements must be boolean".to_string())?;
            let rate_claim = at(case, &["input", "longitudinal_rate_claim"])?
                .as_bool().ok_or_else(|| "longitudinal_rate_claim must be boolean".to_string())?;
            Ok(if rate_claim && count < 2 && !repeats {
                "REJECT_UNSUPPORTED_LONGITUDINAL_INFERENCE"
            } else {
                "LONGITUDINAL_SUPPORT_REQUIRES_PROTOCOL_REVIEW"
            })
        }
        _ => Err(format!("unsupported observation case id: {id}")),
    }
}

fn validate_observation_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let expected: BTreeMap<&'static str, &'static str> = [
        ("PLOT-GEDI-SUPPORT-MISMATCH-001", "BLOCKED_SUPPORT_MISMATCH"),
        ("GEDI-ISS-STORAGE-GAP-001", "UNOBSERVED_NO_ACQUISITION"),
        ("TMF-DEGRADATION-NOT-MORTALITY-001", "REJECT_LABEL_SEMANTIC_MISMATCH"),
        ("MAPBIOMAS-BRAZIL-TRANSFER-001", "OUT_OF_DOMAIN"),
        ("CLOUD-OR-NODATA-NO-CHANGE-001", "UNOBSERVED_CLOUD_OR_QA_MASK"),
        ("PLOT-CAMPAIGN-LEAKAGE-001", "REJECT_LEAKAGE"),
        ("DRC-INACCESSIBLE-SAMPLE-UNITS-001", "UNOBSERVED_INACCESSIBLE_OR_CONFLICT_AFFECTED"),
        ("PNG-SECURITY-LOGISTICS-OMISSIONS-001", "UNOBSERVED_SECURITY_OR_LOGISTICS"),
        ("SINGLE-CENSUS-AS-LONGITUDINAL-PANEL-001", "REJECT_UNSUPPORTED_LONGITUDINAL_INFERENCE"),
    ]
    .into_iter()
    .collect();

    let version = string_at(manifest, &["schema_version"])?;
    let benchmark = string_at(manifest, &["benchmark_id"])?;
    let corpus_id = string_at(manifest, &["corpus_id"])?;
    let status = string_at(manifest, &["status"])?;
    let ceiling = string_at(manifest, &["claim_ceiling"])?;
    let cases = at(manifest, &["cases"])?
        .as_array()
        .ok_or_else(|| "cases must be an array".to_string())?;

    let mut seen = BTreeSet::new();
    let mut checks = Vec::new();
    push_check(&mut checks, "OBS-CORPUS-VERSION-001", version == "0.1.0",
        format!("schema_version={version}"));
    push_check(&mut checks, "OBS-CORPUS-IDENTITY-001",
        benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-OBSERVATION-OPERATOR-NEGATIVE-CONTROLS-001",
        format!("benchmark={benchmark}; corpus={corpus_id}"));
    push_check(&mut checks, "OBS-CORPUS-CLAIM-CEILING-001",
        status == "SYNTHETIC_RULE_FIXTURES_NOT_REAL_OBSERVATIONS"
            && ceiling.contains("Only the deterministic rules")
            && ceiling.contains("No real source data ingested"),
        "corpus must remain explicitly synthetic and non-production");

    for (index, case) in cases.iter().enumerate() {
        let id = case.get("case_id").and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].case_id must be a string"))?;
        let disposition = case.get("expected_disposition").and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].expected_disposition must be a string"))?;
        let condition = case.get("condition").and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].condition must be a string"))?;
        let invariant = case.get("invariant").and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].invariant must be a string"))?;
        let unique = seen.insert(id.to_string());
        let expected_disposition = expected
            .iter()
            .find_map(|(candidate_id, value)| (*candidate_id == id).then_some(*value));
        let derived_disposition = adjudicate_observation_case(case)?;
        let exact_disposition = expected_disposition == Some(disposition)
            && derived_disposition == disposition;
        push_check(
            &mut checks,
            "OBS-CASE-RULE-EXECUTION-001",
            unique && exact_disposition && !condition.trim().is_empty() && !invariant.trim().is_empty(),
            format!(
                "case={id}; derived={derived_disposition}; expected={}; declared={disposition}; unique={unique}",
                expected_disposition.unwrap_or("UNKNOWN_CASE")
            ),
        );
    }
    push_check(&mut checks, "OBS-CASE-COVERAGE-001",
        seen.len() == expected.len() && cases.len() == expected.len(),
        format!("{} unique cases; {} required", seen.len(), expected.len()));

    let runner = string_at(manifest, &["execution", "runner"])?;
    let adapter = string_at(manifest, &["execution", "production_observation_adapter"])?;
    let external = at(manifest, &["execution", "external_data_ingested"])?
        .as_bool().ok_or_else(|| "external_data_ingested must be boolean".to_string())?;
    let synthetic_executed = at(manifest, &["execution", "synthetic_rule_cases_executed"])?
        .as_bool().ok_or_else(|| "synthetic_rule_cases_executed must be boolean".to_string())?;
    let rights = string_at(manifest, &["execution", "rights_adjudication"])?;
    push_check(&mut checks, "OBS-SYNTHETIC-EXECUTION-BOUNDARY-001",
        runner == "SYNTHETIC_RULE_ENGINE" && adapter == "NOT_IMPLEMENTED"
            && !external && synthetic_executed && rights == "NOT_RUN",
        "the Rust rule engine executes synthetic descriptors only; production adapters, external data and rights decisions remain unexecuted");

    let passed = checks.iter().all(|check| check.passed);
    let check_values: Vec<Value> = checks.iter().map(|check| json!({
        "id": check.id, "passed": check.passed, "detail": check.detail
    })).collect();
    Ok(json!({
        "receipt_version": "0.1.0",
        "validator": "symtropy-tropical-forest-qualifier",
        "benchmark_id": benchmark,
        "corpus_id": corpus_id,
        "source_manifest_sha256": sha256_hex(raw),
        "result": if passed { "PASS_SYNTHETIC_RULES" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "synthetic_rule_cases_executed": true,
        "production_observation_adapter": "NOT_IMPLEMENTED",
        "external_data_ingested": false,
        "ecological_observation_validation": "NOT_ESTABLISHED"
    }))
}

fn validate_canopy_fixture(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let version = string_at(manifest, &["schema_version"])?;
    let benchmark = string_at(manifest, &["benchmark_id"])?;
    let corpus_id = string_at(manifest, &["case_corpus_id"])?;
    let status = string_at(manifest, &["status"])?;
    let ceiling = string_at(manifest, &["claim_ceiling"])?;
    let width = usize_at(manifest, &["grid", "width"])?;
    let height = usize_at(manifest, &["grid", "height"])?;
    if width == 0 || height == 0 || width.checked_mul(height).is_none() {
        return Err("grid dimensions must be non-zero and fit addressable memory".to_string());
    }

    let law = string_at(manifest, &["synthetic_light_law", "equation"])?;
    let k = number_at(manifest, &["synthetic_light_law", "extinction_coefficient_k"])?;
    let calibrated = at(manifest, &["synthetic_light_law", "calibrated"])?
        .as_bool().ok_or_else(|| "synthetic_light_law.calibrated must be boolean".to_string())?;
    let arrangements = at(manifest, &["arrangements"])?
        .as_array().ok_or_else(|| "arrangements must be an array".to_string())?;
    if arrangements.len() != 2 {
        return Err("canopy v0.1 requires exactly two arrangements".to_string());
    }

    let mut checks = Vec::new();
    push_check(&mut checks, "CANOPY-FIXTURE-IDENTITY-001",
        version == "0.1.0" && benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "CANOPY-GAP-REPRESENTATION-001",
        format!("version={version}; benchmark={benchmark}; corpus={corpus_id}"));
    push_check(&mut checks, "CANOPY-CLAIM-CEILING-001",
        status == "ANALYTICAL_SYNTHETIC_FIXTURES_NOT_YET_EXECUTED"
            && !calibrated
            && ceiling.contains("not calibrated"),
        "light rule and fixture must remain synthetic and uncalibrated");
    push_check(&mut checks, "CANOPY-LIGHT-LAW-001",
        law == "T(x,y) = exp(-k * LAI(x,y))" && k >= 0.0,
        format!("law={law}; k={k}; calibrated={calibrated}"));

    let expected_cells = width * height;
    let mut outputs = Vec::new();
    let mut total_lai_reference: Option<f64> = None;
    let mut mean_lai_reference: Option<f64> = None;
    let mut component_counts = Vec::new();
    for (index, arrangement) in arrangements.iter().enumerate() {
        let id = string_at(arrangement, &["arrangement_id"])?;
        let occupied_expected = usize_at(arrangement, &["occupied_cell_count"])?;
        let open_expected = usize_at(arrangement, &["open_cell_count"])?;
        let occupied_lai = number_at(arrangement, &["occupied_lai"])?;
        let open_lai = number_at(arrangement, &["open_lai"])?;
        let declared_total_lai = number_at(arrangement, &["total_lai"])?;
        let declared_mean_lai = number_at(arrangement, &["mean_lai"])?;
        let declared_components = usize_at(arrangement, &["open_component_count_four_neighbour"])?;

        let (occupied, occupied_duplicates) = parse_coordinates(
            at(arrangement, &["occupied_coordinates"])?,
            &format!("arrangements[{index}].occupied_coordinates"),
            width, height,
        )?;
        let (open, open_duplicates) = parse_coordinates(
            at(arrangement, &["open_coordinates"])?,
            &format!("arrangements[{index}].open_coordinates"),
            width, height,
        )?;
        let disjoint = occupied.is_disjoint(&open);
        let full_coverage = occupied.len() + open.len() == expected_cells
            && occupied.union(&open).count() == expected_cells;
        let counts_match = occupied.len() == occupied_expected
            && open.len() == open_expected
            && occupied_duplicates == 0
            && open_duplicates == 0;
        let total_lai = occupied.len() as f64 * occupied_lai
            + open.len() as f64 * open_lai;
        let mean_lai = total_lai / expected_cells as f64;
        let components = open_component_count(&open);
        let occupied_transmittance = (-k * occupied_lai).exp();
        let open_transmittance = (-k * open_lai).exp();
        let expected_occupied_transmittance =
            number_at(arrangement, &["expected_spatial_transmittance", "occupied"])?;
        let expected_open_transmittance =
            number_at(arrangement, &["expected_spatial_transmittance", "open"])?;
        let expected_uniform_transmittance =
            number_at(arrangement, &["expected_uniform_patch_transmittance"])?;
        let baseline_mean_lai =
            number_at(manifest, &["uniform_patch_baseline", "mean_lai"])?;
        let baseline_transmittance =
            number_at(manifest, &["uniform_patch_baseline", "predicted_transmittance_every_cell"])?;
        let uniform_calculated = (-k * mean_lai).exp();
        let spatial_mean = (occupied_transmittance * occupied.len() as f64
            + open_transmittance * open.len() as f64)
            / expected_cells as f64;

        let valid = disjoint
            && full_coverage
            && counts_match
            && occupied_lai >= 0.0
            && open_lai >= 0.0
            && close_enough(total_lai, declared_total_lai, 1e-9)
            && close_enough(mean_lai, declared_mean_lai, 1e-9)
            && components == declared_components
            && close_enough(occupied_transmittance, expected_occupied_transmittance, 1e-9)
            && close_enough(open_transmittance, expected_open_transmittance, 1e-9)
            && close_enough(uniform_calculated, expected_uniform_transmittance, 1e-9)
            && close_enough(uniform_calculated, baseline_transmittance, 1e-9)
            && close_enough(mean_lai, baseline_mean_lai, 1e-9);

        push_check(&mut checks, "CANOPY-ARRANGEMENT-GEOMETRY-001", valid,
            format!("arrangement={id}; cells={expected_cells}; occupied={}, open={}, disjoint={disjoint}, full_coverage={full_coverage}, components={components}/{declared_components}, total_lai={total_lai}, mean_lai={mean_lai}",
                occupied.len(), open.len()));

        if let Some(reference) = total_lai_reference {
            push_check(&mut checks, "CANOPY-AGGREGATE-EQUIVALENCE-001",
                close_enough(reference, total_lai, 1e-9),
                format!("arrangement={id}; total_lai={total_lai}; reference={reference}"));
        } else {
            total_lai_reference = Some(total_lai);
        }
        if let Some(reference) = mean_lai_reference {
            push_check(&mut checks, "CANOPY-MEAN-EQUIVALENCE-001",
                close_enough(reference, mean_lai, 1e-9),
                format!("arrangement={id}; mean_lai={mean_lai}; reference={reference}"));
        } else {
            mean_lai_reference = Some(mean_lai);
        }
        component_counts.push(components);
        outputs.push(json!({
            "arrangement_id": id,
            "occupied_cells": occupied.len(),
            "open_cells": open.len(),
            "total_lai": total_lai,
            "mean_lai": mean_lai,
            "open_components_four_neighbour": components,
            "occupied_transmittance": occupied_transmittance,
            "open_transmittance": open_transmittance,
            "spatial_mean_transmittance": spatial_mean,
            "uniform_patch_transmittance": uniform_calculated
        }));
    }

    let expected_challenges: BTreeMap<&'static str, &'static str> = [
        ("CANOPY-AGGREGATE-EQUAL-TOPOLOGY-DIFFERENT-001", "SAME_AGGREGATES_DIFFERENT_SPATIAL_GAP_TOPOLOGY"),
        ("CANOPY-MORTALITY-LOCAL-GAP-001", "LOCAL_CANOPY_SUPPORT_CHANGES_ONLY"),
        ("CANOPY-CLUSTERED-VS-DISPERSED-MORTALITY-001", "EQUAL_LOSS_DIFFERENT_GAP_GEOMETRY"),
        ("CANOPY-RECRUITMENT-LOCAL-LIGHT-001", "LOCAL_LIGHT_REQUIRED_FOR_RECRUITMENT_CLAIM"),
        ("CANOPY-GAP-CLOSURE-CAUSAL-001", "GAP_CLOSURE_REQUIRES_EXPLICIT_GROWTH_SETTLEMENT"),
        ("CANOPY-TRANSLATION-SYMMETRY-001", "TRANSLATED_CASE_OUTPUTS_EQUIVALENT"),
        ("CANOPY-ROTATION-SYMMETRY-001", "ROTATED_CASE_OUTPUTS_EQUIVALENT"),
        ("CANOPY-DISCONNECTED-CONTROL-001", "DISCONNECTED_PATCH_UNCHANGED"),
        ("CANOPY-RENDERER-INDEPENDENCE-001", "CANONICAL_ECOLOGY_IDENTICAL"),
        ("CANOPY-COARSE-FIDELITY-PROMOTION-001", "PROMOTE_SPATIAL_REPRESENTATION_OR_ABSTAIN"),
    ]
    .into_iter()
    .collect();

    let challenge_cases = at(manifest, &["challenge_cases"])?
        .as_array()
        .ok_or_else(|| "challenge_cases must be an array".to_string())?;
    let mut seen_challenges = BTreeSet::new();
    for (index, case) in challenge_cases.iter().enumerate() {
        let id = case
            .get("case_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("challenge_cases[{index}].case_id must be a string"))?;
        let family = case
            .get("family")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("challenge_cases[{index}].family must be a string"))?;
        let condition = case
            .get("condition")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("challenge_cases[{index}].condition must be a string"))?;
        let disposition = case
            .get("expected_disposition")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("challenge_cases[{index}].expected_disposition must be a string"))?;
        let unique = seen_challenges.insert(id.to_string());
        let expected_disposition = expected_challenges
            .iter()
            .find_map(|(candidate_id, value)| (*candidate_id == id).then_some(*value));
        push_check(
            &mut checks,
            "CANOPY-CHALLENGE-CASE-SEMANTICS-001",
            unique
                && expected_disposition == Some(disposition)
                && !family.trim().is_empty()
                && !condition.trim().is_empty(),
            format!(
                "case={id}; expected={}; declared={disposition}; unique={unique}",
                expected_disposition.unwrap_or("UNKNOWN_CASE")
            ),
        );
    }
    let declared_challenge_count = usize_at(manifest, &["execution", "challenge_case_count"])?;
    push_check(
        &mut checks,
        "CANOPY-CHALLENGE-CASE-COVERAGE-001",
        seen_challenges.len() == expected_challenges.len()
            && challenge_cases.len() == expected_challenges.len()
            && declared_challenge_count == expected_challenges.len(),
        format!(
            "{} unique cases; {} required; declared count={declared_challenge_count}",
            seen_challenges.len(),
            expected_challenges.len()
        ),
    );

    let components_differ = component_counts.len() == 2 && component_counts[0] != component_counts[1];
    let spatial_vs_uniform_differs = outputs.iter().all(|result| {
        let spatial = result["spatial_mean_transmittance"].as_f64().unwrap_or(f64::NAN);
        let uniform = result["uniform_patch_transmittance"].as_f64().unwrap_or(f64::NAN);
        (spatial - uniform).abs() > 0.05
    });
    push_check(&mut checks, "CANOPY-TOPOLOGY-DIFFERENCE-001", components_differ,
        format!("open component counts={component_counts:?}"));
    push_check(&mut checks, "CANOPY-UNIFORM-BASELINE-LOSS-001", spatial_vs_uniform_differs,
        "spatially heterogeneous transmittance mean must differ from uniform transmission at the aggregate mean LAI");

    let passed = checks.iter().all(|check| check.passed);
    let check_values: Vec<Value> = checks.iter().map(|check| json!({
        "id": check.id, "passed": check.passed, "detail": check.detail
    })).collect();
    Ok(json!({
        "receipt_version": "0.1.0",
        "validator": "symtropy-tropical-forest-qualifier",
        "benchmark_id": benchmark,
        "case_corpus_id": corpus_id,
        "source_manifest_sha256": sha256_hex(raw),
        "result": if passed { "PASS_SYNTHETIC_GEOMETRY" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "arrangements": outputs,
        "challenge_case_count": seen_challenges.len(),
        "challenge_cases_executed": false,
        "ecological_light_model_calibration": "NOT_ESTABLISHED",
        "field_validation": "NOT_ESTABLISHED"
    }))
}


fn validate_decision_value_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let expected_cases: BTreeMap<&'static str, &'static str> = [
        ("FOREST-DECISION-VALUE-001", "HIGH_INFORMATION_ZERO_DECISION_VALUE"),
        ("FOREST-DECISION-VALUE-002", "POSITIVE_NET_DECISION_VALUE"),
        ("FOREST-DECISION-VALUE-003", "RANK_BY_EXPECTED_REGRET_REDUCTION"),
        ("FOREST-DECISION-VALUE-004", "SAMPLE_PROPAGULE_LIMITATION_IF_DECISION_RELEVANT"),
        ("FOREST-DECISION-VALUE-005", "ZERO_DECISION_VALUE_DESPITE_VARIANCE"),
        ("FOREST-DECISION-VALUE-006", "SELECT_FEASIBLE_LOWER_TOTAL_COST"),
        ("FOREST-DECISION-VALUE-007", "DECISION_FOCUSED_VERIFICATION"),
        ("FOREST-DECISION-VALUE-008", "NO_EXTRA_SURVEY_FOR_ZERO_ACTION_VALUE"),
        ("FOREST-DECISION-VALUE-009", "MISSINGNESS_IS_NOT_NEGATIVE_EVIDENCE"),
        ("FOREST-DECISION-VALUE-010", "PRIORITIZE_DECISION_IMPACT_NOT_VARIANCE"),
        ("FOREST-DECISION-VALUE-011", "ZERO_MARGINAL_DECISION_VALUE"),
        ("FOREST-DECISION-VALUE-012", "FEASIBILITY_GATE_BEFORE_RANKING"),
        ("FOREST-DECISION-VALUE-013", "SELECT_IF_EXPECTED_DECISION_REGRET_FALLS"),
        ("FOREST-DECISION-VALUE-014", "DO_NOT_RANK_UNAUTHORIZED_OBSERVATION"),
        ("FOREST-DECISION-VALUE-015", "PRESERVE_MODEL_IDENTITY_AND_DISCRIMINATION_VALUE"),
        ("FOREST-DECISION-VALUE-016", "DECISION_VALUE_WITH_MODEL_SET_ADEQUACY"),
        ("FOREST-DECISION-VALUE-017", "DO_NOT_OVER-SAMPLE_LOW_DECISION_VALUE"),
        ("FOREST-DECISION-VALUE-018", "SAMPLE_TO_RESOLVE_OBSERVATION_PROCESS"),
        ("FOREST-DECISION-VALUE-019", "EXCLUDE_UNAUTHORIZED_CANDIDATE"),
        ("FOREST-DECISION-VALUE-020", "ACCOUNT_FOR_DEPENDENT_OBSERVATIONS"),
    ]
    .into_iter()
    .collect();

    let version = string_at(manifest, &["schema_version"])?;
    let benchmark = string_at(manifest, &["benchmark_id"])?;
    let corpus_id = string_at(manifest, &["corpus_id"])?;
    let status = string_at(manifest, &["status"])?;
    let ceiling = string_at(manifest, &["claim_ceiling"])?;
    let hidden_outcomes = at(manifest, &["hidden_outcomes_published"])?
        .as_bool().ok_or_else(|| "hidden_outcomes_published must be boolean".to_string())?;
    let blind_status = string_at(manifest, &["independent_blind_corpus"])?;
    let cases = at(manifest, &["cases"])?
        .as_array().ok_or_else(|| "cases must be an array".to_string())?;

    let mut checks = Vec::new();
    push_check(&mut checks, "DECISION-VALUE-CORPUS-VERSION-001",
        version == "0.1.0", format!("schema_version={version}"));
    push_check(&mut checks, "DECISION-VALUE-CORPUS-IDENTITY-001",
        benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-DECISION-VALUE-001",
        format!("benchmark={benchmark}; corpus={corpus_id}"));
    push_check(&mut checks, "DECISION-VALUE-PUBLIC-NOT-BLIND-001",
        status == "PUBLIC_SYNTHETIC_DEVELOPMENT_CASES_NOT_BLIND"
            && !hidden_outcomes
            && blind_status == "NOT_CREATED_REQUIRES_INDEPENDENT_CUSTODY"
            && ceiling.contains("No real site"),
        "public decision-value fixtures must not claim a blind or field evaluation");

    let mut seen = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let id = case.get("case_id").and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].case_id must be a string"))?;
        let family = case.get("family").and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].family must be a string"))?;
        let scenario = case.get("scenario_id").and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].scenario_id must be a string"))?;
        let condition = case.get("condition").and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].condition must be a string"))?;
        let disposition = case.get("expected_disposition").and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].expected_disposition must be a string"))?;
        let unique = seen.insert(id.to_string());
        let expected_disposition = expected_cases.iter()
            .find_map(|(candidate_id, value)| (*candidate_id == id).then_some(*value));
        push_check(&mut checks, "DECISION-VALUE-CASE-SEMANTICS-001",
            unique && expected_disposition == Some(disposition)
                && !family.trim().is_empty() && !scenario.trim().is_empty()
                && !condition.trim().is_empty(),
            format!("case={id}; expected={}; declared={disposition}; unique={unique}",
                expected_disposition.unwrap_or("UNKNOWN_CASE")));
    }
    push_check(&mut checks, "DECISION-VALUE-CASE-COVERAGE-001",
        seen.len() == expected_cases.len() && cases.len() == expected_cases.len(),
        format!("{} unique cases; {} required", seen.len(), expected_cases.len()));

    let zero = at(manifest, &["worked_examples", "high_information_zero_decision_value"])?;
    let zero_prior = number_at(zero, &["prior_probability_high_turnover"])?;
    let zero_protect_high = number_at(zero, &["loss_matrix", "protect", "high_turnover"])?;
    let zero_protect_low = number_at(zero, &["loss_matrix", "protect", "low_turnover"])?;
    let zero_none_high = number_at(zero, &["loss_matrix", "do_not_protect", "high_turnover"])?;
    let zero_none_low = number_at(zero, &["loss_matrix", "do_not_protect", "low_turnover"])?;
    let zero_evsi = number_at(zero, &["expected_gross_evsi"])?;
    let zero_entropy_claim = string_at(zero, &["ideal_measurement_information_gain"])?;
    let zero_disposition = string_at(zero, &["expected_disposition"])?;
    let action_losses_are_state_invariant = close_enough(zero_protect_high, zero_protect_low, 1e-12)
        && close_enough(zero_none_high, zero_none_low, 1e-12);
    push_check(&mut checks, "DECISION-VALUE-HIGH-UNCERTAINTY-ZERO-EVSI-001",
        (0.0..=1.0).contains(&zero_prior)
            && action_losses_are_state_invariant
            && close_enough(zero_evsi, 0.0, 1e-12)
            && zero_entropy_claim == "positive"
            && zero_disposition == "HIGH_INFORMATION_ZERO_DECISION_VALUE",
        "state information cannot improve an action when the declared loss is invariant across states");

    let positive = at(manifest, &["worked_examples", "moderate_uncertainty_positive_decision_value"])?;
    let prior = number_at(positive, &["prior_probability_degraded"])?;
    let protect_stable = number_at(positive, &["loss_matrix", "protect", "stable"])?;
    let protect_degraded = number_at(positive, &["loss_matrix", "protect", "degraded"])?;
    let no_action_stable = number_at(positive, &["loss_matrix", "do_not_protect", "stable"])?;
    let no_action_degraded = number_at(positive, &["loss_matrix", "do_not_protect", "degraded"])?;
    let sensitivity = number_at(positive, &["measurement", "sensitivity"])?;
    let specificity = number_at(positive, &["measurement", "specificity"])?;
    let cost = number_at(positive, &["measurement", "cost"])?;
    let declared_without = number_at(positive, &["expected_loss_without_observation"])?;
    let declared_p_positive = number_at(positive, &["probability_positive"])?;
    let declared_post_positive = number_at(positive, &["posterior_degraded_given_positive"])?;
    let declared_p_negative = number_at(positive, &["probability_negative"])?;
    let declared_post_negative = number_at(positive, &["posterior_degraded_given_negative"])?;
    let declared_after = number_at(positive, &["expected_loss_after_observation"])?;
    let declared_gross = number_at(positive, &["gross_evsi"])?;
    let declared_net = number_at(positive, &["net_expected_value_after_cost"])?;
    let declared_positive_disposition = string_at(positive, &["expected_disposition"])?;

    let p_positive = prior * sensitivity + (1.0 - prior) * (1.0 - specificity);
    let p_negative = 1.0 - p_positive;
    let posterior_positive = if p_positive > 0.0 { prior * sensitivity / p_positive } else { f64::NAN };
    let posterior_negative = if p_negative > 0.0 { prior * (1.0 - sensitivity) / p_negative } else { f64::NAN };
    let baseline_protect_loss = protect_stable * (1.0 - prior) + protect_degraded * prior;
    let baseline_no_action_loss = no_action_stable * (1.0 - prior) + no_action_degraded * prior;
    let post_positive_protect_loss =
        protect_stable * (1.0 - posterior_positive) + protect_degraded * posterior_positive;
    let post_positive_no_action_loss =
        no_action_stable * (1.0 - posterior_positive) + no_action_degraded * posterior_positive;
    let post_negative_protect_loss =
        protect_stable * (1.0 - posterior_negative) + protect_degraded * posterior_negative;
    let post_negative_no_action_loss =
        no_action_stable * (1.0 - posterior_negative) + no_action_degraded * posterior_negative;
    let baseline_loss = baseline_protect_loss.min(baseline_no_action_loss);
    let post_positive_loss = post_positive_protect_loss.min(post_positive_no_action_loss);
    let post_negative_loss = post_negative_protect_loss.min(post_negative_no_action_loss);
    let expected_after = p_positive * post_positive_loss + p_negative * post_negative_loss;
    let gross_evsi = baseline_loss - expected_after;
    let net_value = gross_evsi - cost;
    let action_switches = baseline_protect_loss < baseline_no_action_loss
        && post_positive_protect_loss < post_positive_no_action_loss
        && post_negative_no_action_loss < post_negative_protect_loss;
    let valid_probabilities = (0.0..=1.0).contains(&prior)
        && (0.0..=1.0).contains(&sensitivity)
        && (0.0..=1.0).contains(&specificity)
        && cost >= 0.0
        && close_enough(p_positive + p_negative, 1.0, 1e-12);
    let worked_example_matches = valid_probabilities
        && action_switches
        && close_enough(baseline_loss, declared_without, 1e-9)
        && close_enough(p_positive, declared_p_positive, 1e-9)
        && close_enough(p_negative, declared_p_negative, 1e-9)
        && close_enough(posterior_positive, declared_post_positive, 1e-9)
        && close_enough(posterior_negative, declared_post_negative, 1e-9)
        && close_enough(expected_after, declared_after, 1e-9)
        && close_enough(gross_evsi, declared_gross, 1e-9)
        && close_enough(net_value, declared_net, 1e-9)
        && declared_positive_disposition == "POSITIVE_NET_DECISION_VALUE";
    push_check(&mut checks, "DECISION-VALUE-EVSI-ARITHMETIC-001",
        worked_example_matches,
        format!("baseline={baseline_loss}; p+={p_positive}; p-={p_negative}; post+={posterior_positive}; post-={posterior_negative}; after={expected_after}; gross={gross_evsi}; net={net_value}; action_switches={action_switches}"));

    let execution = at(manifest, &["execution"])?;
    let evaluator = string_at(execution, &["evaluator"])?;
    let uses_existing = at(execution, &["uses_existing_evsi_kernel"])?
        .as_bool().ok_or_else(|| "uses_existing_evsi_kernel must be boolean".to_string())?;
    let duplicate = at(execution, &["duplicate_evsi_kernel_created"])?
        .as_bool().ok_or_else(|| "duplicate_evsi_kernel_created must be boolean".to_string())?;
    let real_data = at(execution, &["real_field_data_ingested"])?
        .as_bool().ok_or_else(|| "real_field_data_ingested must be boolean".to_string())?;
    let action_authorized = at(execution, &["real_management_action_authorized"])?
        .as_bool().ok_or_else(|| "real_management_action_authorized must be boolean".to_string())?;
    push_check(&mut checks, "DECISION-VALUE-NO-DUPLICATE-KERNEL-001",
        evaluator == "NOT_IMPLEMENTED" && uses_existing && !duplicate
            && !real_data && !action_authorized,
        "public fixture must not claim a second EVSI engine, field data, or decision authority");

    let passed = checks.iter().all(|check| check.passed);
    let check_values: Vec<Value> = checks.iter().map(|check| json!({
        "id": check.id, "passed": check.passed, "detail": check.detail
    })).collect();
    Ok(json!({
        "receipt_version": "0.1.0",
        "validator": "symtropy-tropical-forest-qualifier",
        "benchmark_id": benchmark,
        "corpus_id": corpus_id,
        "source_manifest_sha256": sha256_hex(raw),
        "result": if passed { "MANIFEST_VALID" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "public_case_count": seen.len(),
        "expected_gross_evsi_example": gross_evsi,
        "expected_net_value_after_cost_example": net_value,
        "decision_policy_performance": "NOT_ESTABLISHED",
        "existing_evsi_kernel_execution": "NOT_RUN"
    }))
}

fn validate_model_set_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let expected: BTreeMap<&'static str, &'static str> = [
        ("FOREST-MSA-GROWTH-MORTALITY-COMPENSATION-001", "PRESERVE_STRUCTURAL_DISAGREEMENT_OR_REQUEST_SIZE_RESOLVED_EVIDENCE"),
        ("FOREST-MSA-RECRUITMENT-GROWTH-ALIASING-001", "DO_NOT_TREAT_AGGREGATE_BIOMASS_FIT_AS_DEMOGRAPHIC_ADEQUACY"),
        ("FOREST-MSA-DELAYED-RESPONSE-001", "FLAG_MODEL_SET_ADEQUACY_UNRESOLVED"),
        ("FOREST-MSA-SIZE-DEPENDENT-MORTALITY-001", "REQUEST_SIZE_STRATIFIED_OBSERVATION_OR_ABSTAIN"),
        ("FOREST-MSA-SEED-LIMITATION-001", "PRESERVE_COMPETING_MECHANISM_AND_SEEK_DISCRIMINATING_EVIDENCE"),
        ("FOREST-MSA-SPATIAL-CANOPY-TOPOLOGY-001", "PROMOTE_SPATIAL_REPRESENTATION_OR_MARK_PROCESS_UNSUPPORTED"),
        ("FOREST-MSA-DISTURBANCE-CLIMATE-INTERACTION-001", "FLAG_UNTESTED_INTERACTION_AND_LIMIT_CLAIM"),
        ("FOREST-MSA-OBSERVATION-PROCESS-SHIFT-001", "CLASSIFY_OBSERVATION_SHIFT_NOT_ECOLOGICAL_CHANGE"),
        ("FOREST-MSA-CROSS-REGION-TRANSFER-001", "OUT_OF_DOMAIN_OR_RECALIBRATION_REQUIRED"),
        ("FOREST-MSA-COMMON-STRUCTURAL-OMISSION-001", "DO_NOT_CERTIFY_MODEL_SET_ADEQUACY;REQUEST_DISCRIMINATING_EVIDENCE_OR_ABSTAIN"),
    ]
    .into_iter()
    .collect();

    let version = string_at(manifest, &["schema_version"])?;
    let benchmark = string_at(manifest, &["benchmark_id"])?;
    let corpus_id = string_at(manifest, &["corpus_id"])?;
    let status = string_at(manifest, &["status"])?;
    let claim_ceiling = string_at(manifest, &["claim_ceiling"])?;
    let hidden_truth_published = at(manifest, &["hidden_truth_published"])?
        .as_bool()
        .ok_or_else(|| "hidden_truth_published must be boolean".to_string())?;
    let blind_status = string_at(manifest, &["blind_test_status"])?;
    let cases = at(manifest, &["cases"])?
        .as_array()
        .ok_or_else(|| "cases must be an array".to_string())?;

    let mut seen = BTreeSet::new();
    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "MODEL-SET-CORPUS-VERSION-001",
        version == "0.1.0",
        format!("schema_version={version}"),
    );
    push_check(
        &mut checks,
        "MODEL-SET-CORPUS-IDENTITY-001",
        benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-MODEL-SET-ADEQUACY-001",
        format!("benchmark={benchmark}; corpus={corpus_id}"),
    );
    push_check(
        &mut checks,
        "MODEL-SET-PUBLIC-NOT-BLIND-001",
        status == "PUBLIC_DEVELOPMENT_CASE_FAMILIES_NOT_BLIND"
            && !hidden_truth_published
            && blind_status == "NOT_CREATED_REQUIRES_INDEPENDENT_CUSTODY"
            && claim_ceiling.contains("not a blind test"),
        "public development corpus must not claim sealed/blind generator truth",
    );

    for (index, case) in cases.iter().enumerate() {
        let id = case
            .get("case_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].case_id must be a string"))?;
        let family = case
            .get("family")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].family must be a string"))?;
        let condition = case
            .get("condition")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].condition must be a string"))?;
        let disposition = case
            .get("expected_disposition")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].expected_disposition must be a string"))?;
        let unique = seen.insert(id.to_string());
        let expected_disposition = expected
            .iter()
            .find_map(|(candidate_id, value)| (*candidate_id == id).then_some(*value));
        let known_case = expected_disposition == Some(disposition);
        push_check(
            &mut checks,
            "MODEL-SET-CASE-SEMANTICS-001",
            unique && known_case && !family.trim().is_empty() && !condition.trim().is_empty(),
            format!(
                "case={id}; expected={}; declared={disposition}; unique={unique}",
                expected_disposition.unwrap_or("UNKNOWN_CASE")
            ),
        );
    }
    push_check(
        &mut checks,
        "MODEL-SET-CASE-COVERAGE-001",
        seen.len() == expected.len() && cases.len() == expected.len(),
        format!("{} unique cases; {} required", seen.len(), expected.len()),
    );

    let runner = string_at(manifest, &["execution", "validator"])?;
    let external_data = at(manifest, &["execution", "external_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "external_data_ingested must be boolean".to_string())?;
    let blind_generator = string_at(manifest, &["execution", "blind_generator_identity"])?;
    let custodian = string_at(manifest, &["execution", "independent_custodian"])?;
    let hidden_test = string_at(manifest, &["execution", "hidden_test"])?;
    push_check(
        &mut checks,
        "MODEL-SET-NO-EXECUTION-CLAIM-001",
        runner == "NOT_IMPLEMENTED_FOR_THIS_CORPUS"
            && !external_data
            && blind_generator == "NOT_PUBLISHED"
            && custodian == "NOT_APPOINTED"
            && hidden_test == "NOT_CREATED",
        "public contract validation must not imply a blind experiment or external data execution",
    );

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
        "benchmark_id": benchmark,
        "corpus_id": corpus_id,
        "source_manifest_sha256": sha256_hex(raw),
        "result": if passed { "MANIFEST_VALID" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "blind_test_created": false,
        "hidden_truth_published": false,
        "empirical_model_set_adequacy": "NOT_ESTABLISHED"
    }))
}

/// Dispatch a JSON fixture to its domain-specific verifier and return a deterministic receipt.
///
/// This command verifies manifests and analytical/synthetic invariants only. It does not
/// imply ecological calibration, observation-model validation, or field performance.
pub fn evaluate(raw: &[u8]) -> Result<Value, String> {
    let manifest: Value = serde_json::from_slice(raw)
        .map_err(|error| format!("invalid JSON manifest: {error}"))?;
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-OBSERVATION-OPERATOR-NEGATIVE-CONTROLS-001")
    {
        return validate_observation_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-MODEL-SET-ADEQUACY-001")
    {
        return validate_model_set_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-DECISION-VALUE-001")
    {
        return validate_decision_value_corpus(raw, &manifest);
    }
    if manifest.get("case_corpus_id").is_some() {
        return validate_canopy_fixture(raw, &manifest);
    }
    evaluate_biomass(raw, manifest)
}

fn evaluate_biomass(raw: &[u8], manifest: Value) -> Result<Value, String> {

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

    const OBSERVATION_FIXTURE: &str =
        include_str!("../../cases/forest-observation-operator-negative-controls-v0.1.json");
    const CANOPY_FIXTURE: &str =
        include_str!("../../cases/canopy-gap-representation-synthetic-v0.1.json");
    const MODEL_SET_FIXTURE: &str =
        include_str!("../../cases/forest-model-set-adequacy-development-v0.1.json");
    const DECISION_VALUE_FIXTURE: &str =
        include_str!("../../cases/forest-decision-value-development-v0.1.json");

    #[test]
    fn observation_corpus_executes_nine_synthetic_rule_cases() {
        let receipt = evaluate(OBSERVATION_FIXTURE.as_bytes()).expect("observation corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_RULES");
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(receipt["synthetic_rule_cases_executed"], true);
        assert_eq!(receipt["production_observation_adapter"], "NOT_IMPLEMENTED");
        assert_eq!(receipt["external_data_ingested"], false);
    }

    #[test]
    fn observation_corpus_semantic_mutation_is_rejected() {
        let mut manifest: Value = serde_json::from_str(OBSERVATION_FIXTURE).expect("valid corpus");
        manifest["cases"][1]["expected_disposition"] = json!("OBSERVED_ABSENCE");
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn observation_input_mutation_changes_rule_disposition_and_fails() {
        let mut manifest: Value = serde_json::from_str(OBSERVATION_FIXTURE).expect("valid corpus");
        manifest["cases"][4]["input"]["validity_state"] = json!("valid");
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn no_acquisition_is_preserved_as_missing_not_zero() {
        let receipt = evaluate(OBSERVATION_FIXTURE.as_bytes()).expect("observation corpus parses");
        let check = receipt["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| {
                check["detail"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("GEDI-ISS-STORAGE-GAP-001")
            })
            .expect("GEDI storage-gap check is present");
        assert_eq!(check["passed"], true);
        assert!(check["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("derived=UNOBSERVED_NO_ACQUISITION"));
        assert_eq!(receipt["external_data_ingested"], false);
    }

    #[test]
    fn canopy_geometry_fixture_preserves_aggregate_and_gap_distinctions() {
        let receipt = evaluate(CANOPY_FIXTURE.as_bytes()).expect("canopy fixture parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_GEOMETRY");
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(
            receipt["arrangements"][0]["total_lai"],
            receipt["arrangements"][1]["total_lai"]
        );
        assert_ne!(
            receipt["arrangements"][0]["open_components_four_neighbour"],
            receipt["arrangements"][1]["open_components_four_neighbour"]
        );
        assert_eq!(receipt["ecological_light_model_calibration"], "NOT_ESTABLISHED");
    }

    #[test]
    fn canopy_ten_case_manifest_has_expected_dispositions() {
        let receipt = evaluate(CANOPY_FIXTURE.as_bytes()).expect("canopy fixture parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_GEOMETRY");
        assert_eq!(receipt["challenge_case_count"], 10);
        assert_eq!(receipt["challenge_cases_executed"], false);
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
    }

    #[test]
    fn canopy_challenge_disposition_mutation_is_rejected() {
        let mut manifest: Value =
            serde_json::from_str(CANOPY_FIXTURE).expect("valid canopy fixture");
        manifest["challenge_cases"][9]["expected_disposition"] = json!("EVERYTHING_IS_SUFFICIENT");
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated canopy fixture");
        let receipt = evaluate(&bytes).expect("well-formed fixture returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn canopy_duplicate_coordinate_is_rejected() {
        let mut manifest: Value = serde_json::from_str(CANOPY_FIXTURE).expect("valid canopy fixture");
        let duplicate = manifest["arrangements"][0]["occupied_coordinates"][0].clone();
        manifest["arrangements"][0]["occupied_coordinates"]
            .as_array_mut().unwrap().push(duplicate);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated canopy fixture");
        let receipt = evaluate(&bytes).expect("well-formed fixture returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }



    #[test]
    fn model_set_public_manifest_has_ten_case_families_and_no_blind_claim() {
        let receipt = evaluate(MODEL_SET_FIXTURE.as_bytes()).expect("model-set manifest parses");
        assert_eq!(receipt["result"], "MANIFEST_VALID");
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(receipt["blind_test_created"], false);
        assert_eq!(receipt["hidden_truth_published"], false);
    }

    #[test]
    fn model_set_disposition_mutation_is_rejected() {
        let mut manifest: Value =
            serde_json::from_str(MODEL_SET_FIXTURE).expect("valid model-set manifest");
        manifest["cases"][9]["expected_disposition"] = json!("THE_ENSEMBLE_IS_COMPLETE");
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated manifest");
        let receipt = evaluate(&bytes).expect("well-formed manifest returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn decision_value_manifest_validates_twenty_cases_and_worked_evsi() {
        let receipt = evaluate(DECISION_VALUE_FIXTURE.as_bytes()).expect("decision-value corpus parses");
        assert_eq!(receipt["result"], "MANIFEST_VALID");
        assert_eq!(receipt["public_case_count"], 20);
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert!((receipt["expected_gross_evsi_example"].as_f64().unwrap() - 8.8).abs() < 1e-9);
        assert!((receipt["expected_net_value_after_cost_example"].as_f64().unwrap() - 5.8).abs() < 1e-9);
        assert_eq!(receipt["decision_policy_performance"], "NOT_ESTABLISHED");
        assert_eq!(receipt["existing_evsi_kernel_execution"], "NOT_RUN");
    }

    #[test]
    fn decision_value_evsi_arithmetic_mutation_is_rejected() {
        let mut manifest: Value =
            serde_json::from_str(DECISION_VALUE_FIXTURE).expect("valid decision-value corpus");
        manifest["worked_examples"]["moderate_uncertainty_positive_decision_value"]
            ["net_expected_value_after_cost"] = json!(9.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated decision-value corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn high_information_without_action_change_has_zero_decision_value() {
        let receipt = evaluate(DECISION_VALUE_FIXTURE.as_bytes()).expect("decision-value corpus parses");
        let check = receipt["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["id"] == "DECISION-VALUE-HIGH-UNCERTAINTY-ZERO-EVSI-001")
            .expect("zero-EVSI check is present");
        assert_eq!(check["passed"], true);
    }

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
