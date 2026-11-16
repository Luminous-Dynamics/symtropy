// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Offline semantic verifier for the synthetic tropical-forest qualification fixture.
//!
//! This validates arithmetic and causal-isolation properties of a declared synthetic
//! case. It does not estimate ecological parameters, validate a biological model, or
//! turn a synthetic pass into field evidence.

pub mod aggregation_kernel;
pub mod canopy_kernel;
pub mod conformal_kernel;
pub mod coverage_inference_kernel;
pub mod forecast_kernel;
pub mod forecast_usefulness_kernel;
pub mod forest_as_of_integrity_kernel;
pub mod forest_observation_cadence_kernel;
pub mod forest_corpus_receipts;
pub mod model_set_kernel;
pub mod support_kernel;
pub mod spatial_localization_kernel;
pub mod trajectory_budget_kernel;
pub mod flux_identifiability_kernel;
pub mod practical_identifiability_kernel;
pub mod forestpaths_crosswalk_kernel;
pub mod afex_design_kernel;
pub mod afex_root_turnover_kernel;
pub mod afex_ratio_estimand_kernel;
pub mod afex_root_pathway_tournament_kernel;
pub mod afex_root_pathway_identifiability_kernel;
pub mod afex_practical_design_kernel;
pub mod afex_source_identity_kernel;
pub mod afex_root_distribution_kernel;

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

type CellStateMap = BTreeMap<String, Value>;
type CellStateIndex = (BTreeSet<String>, CellStateMap, CellStateMap);

fn check_cell_ids(manifest: &Value) -> Result<CellStateIndex, String> {
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

    let initial: CellStateMap = object_at(manifest, &["initial_state"])?
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let expected: CellStateMap = object_at(manifest, &["expected_state"])?
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();

    let initial_ids: BTreeSet<String> = initial.keys().cloned().collect();
    let expected_ids: BTreeSet<String> = expected.keys().cloned().collect();
    if declared != initial_ids {
        return Err(
            "declared spatial cell ids do not exactly match initial_state keys".to_string(),
        );
    }
    if declared != expected_ids {
        return Err(
            "declared spatial cell ids do not exactly match expected_state keys".to_string(),
        );
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
            .ok_or_else(|| {
                format!("explicit_causal_links[{index}].from_cell_id must be a string")
            })?;
        let to = link
            .get("to_cell_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("explicit_causal_links[{index}].to_cell_id must be a string"))?;
        if !declared_cells.contains(from) || !declared_cells.contains(to) {
            return Err(format!(
                "causal link {from} -> {to} references an undeclared spatial cell"
            ));
        }
        graph
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
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
            return Err(format!(
                "{label}[{index}] must contain exactly two coordinates"
            ));
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

fn open_component_count(open_cells: &BTreeSet<(usize, usize)>) -> usize {
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
                .as_bool()
                .ok_or_else(|| "overlap_operator_applied must be boolean".to_string())?;
            let conversion_declared = at(case, &["input", "support_conversion_declared"])?
                .as_bool()
                .ok_or_else(|| "support_conversion_declared must be boolean".to_string())?;
            Ok(
                if left != right && !overlap_applied && !conversion_declared {
                    "BLOCKED_SUPPORT_MISMATCH"
                } else {
                    "SUPPORT_MISMATCH_CONTROL_NOT_TRIGGERED"
                },
            )
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
            Ok(
                if source_class == "degraded_forest" && target == "tree_mortality" {
                    "REJECT_LABEL_SEMANTIC_MISMATCH"
                } else {
                    "LABEL_SEMANTIC_MISMATCH_CONTROL_NOT_TRIGGERED"
                },
            )
        }
        "MAPBIOMAS-BRAZIL-TRANSFER-001" => {
            let source = string_at(case, &["input", "source_coverage_country"])?;
            let target = string_at(case, &["input", "target_country"])?;
            Ok(if source != target {
                "OUT_OF_DOMAIN"
            } else {
                "TARGET_WITHIN_DECLARED_COUNTRY"
            })
        }
        "CLOUD-OR-NODATA-NO-CHANGE-001" => {
            let validity = string_at(case, &["input", "validity_state"])?;
            Ok(
                if validity == "cloud_masked" || validity == "no_data" || validity == "qa_invalid" {
                    "UNOBSERVED_CLOUD_OR_QA_MASK"
                } else {
                    "OBSERVATION_VALID_REQUIRES_SOURCE_OPERATOR"
                },
            )
        }
        "PLOT-CAMPAIGN-LEAKAGE-001" => {
            let unit = string_at(case, &["input", "split_group_unit"])?;
            if unit.trim().is_empty() {
                return Err("split_group_unit must not be empty".to_string());
            }
            let same_plot = at(case, &["input", "same_plot_across_splits"])?
                .as_bool()
                .ok_or_else(|| "same_plot_across_splits must be boolean".to_string())?;
            Ok(if same_plot {
                "REJECT_LEAKAGE"
            } else {
                "NO_WITHIN_PLOT_SPLIT_CONFLICT_DETECTED"
            })
        }
        "DRC-INACCESSIBLE-SAMPLE-UNITS-001" => {
            let status = string_at(case, &["input", "survey_status"])?;
            let reason = string_at(case, &["input", "unsurveyed_reason"])?;
            Ok(
                if status == "not_surveyed"
                    && (reason == "inaccessible"
                        || reason == "conflict_affected"
                        || reason == "inaccessible_conflict_affected")
                {
                    "UNOBSERVED_INACCESSIBLE_OR_CONFLICT_AFFECTED"
                } else {
                    "SURVEYED_OR_OTHER_MISSINGNESS_REQUIRES_REVIEW"
                },
            )
        }
        "PNG-SECURITY-LOGISTICS-OMISSIONS-001" => {
            let status = string_at(case, &["input", "survey_status"])?;
            let reason = string_at(case, &["input", "unsurveyed_reason"])?;
            Ok(
                if status == "not_surveyed"
                    && (reason == "security"
                        || reason == "logistics"
                        || reason == "security_or_logistics")
                {
                    "UNOBSERVED_SECURITY_OR_LOGISTICS"
                } else {
                    "SURVEYED_OR_OTHER_MISSINGNESS_REQUIRES_REVIEW"
                },
            )
        }
        "SINGLE-CENSUS-AS-LONGITUDINAL-PANEL-001" => {
            let count = usize_at(case, &["input", "census_count"])?;
            let repeats = at(case, &["input", "matched_repeat_measurements"])?
                .as_bool()
                .ok_or_else(|| "matched_repeat_measurements must be boolean".to_string())?;
            let rate_claim = at(case, &["input", "longitudinal_rate_claim"])?
                .as_bool()
                .ok_or_else(|| "longitudinal_rate_claim must be boolean".to_string())?;
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
        (
            "TMF-DEGRADATION-NOT-MORTALITY-001",
            "REJECT_LABEL_SEMANTIC_MISMATCH",
        ),
        ("MAPBIOMAS-BRAZIL-TRANSFER-001", "OUT_OF_DOMAIN"),
        (
            "CLOUD-OR-NODATA-NO-CHANGE-001",
            "UNOBSERVED_CLOUD_OR_QA_MASK",
        ),
        ("PLOT-CAMPAIGN-LEAKAGE-001", "REJECT_LEAKAGE"),
        (
            "DRC-INACCESSIBLE-SAMPLE-UNITS-001",
            "UNOBSERVED_INACCESSIBLE_OR_CONFLICT_AFFECTED",
        ),
        (
            "PNG-SECURITY-LOGISTICS-OMISSIONS-001",
            "UNOBSERVED_SECURITY_OR_LOGISTICS",
        ),
        (
            "SINGLE-CENSUS-AS-LONGITUDINAL-PANEL-001",
            "REJECT_UNSUPPORTED_LONGITUDINAL_INFERENCE",
        ),
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
    push_check(
        &mut checks,
        "OBS-CORPUS-VERSION-001",
        version == "0.1.0",
        format!("schema_version={version}"),
    );
    push_check(
        &mut checks,
        "OBS-CORPUS-IDENTITY-001",
        benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-OBSERVATION-OPERATOR-NEGATIVE-CONTROLS-001",
        format!("benchmark={benchmark}; corpus={corpus_id}"),
    );
    push_check(
        &mut checks,
        "OBS-CORPUS-CLAIM-CEILING-001",
        status == "SYNTHETIC_RULE_FIXTURES_NOT_REAL_OBSERVATIONS"
            && ceiling.contains("Only the deterministic rules")
            && ceiling.contains("No real source data ingested"),
        "corpus must remain explicitly synthetic and non-production",
    );

    for (index, case) in cases.iter().enumerate() {
        let id = case
            .get("case_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].case_id must be a string"))?;
        let disposition = case
            .get("expected_disposition")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].expected_disposition must be a string"))?;
        let condition = case
            .get("condition")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].condition must be a string"))?;
        let invariant = case
            .get("invariant")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].invariant must be a string"))?;
        let unique = seen.insert(id.to_string());
        let expected_disposition = expected
            .iter()
            .find_map(|(candidate_id, value)| (*candidate_id == id).then_some(*value));
        let derived_disposition = adjudicate_observation_case(case)?;
        let exact_disposition =
            expected_disposition == Some(disposition) && derived_disposition == disposition;
        push_check(
            &mut checks,
            "OBS-CASE-RULE-EXECUTION-001",
            unique
                && exact_disposition
                && !condition.trim().is_empty()
                && !invariant.trim().is_empty(),
            format!(
                "case={id}; derived={derived_disposition}; expected={}; declared={disposition}; unique={unique}",
                expected_disposition.unwrap_or("UNKNOWN_CASE")
            ),
        );
    }
    push_check(
        &mut checks,
        "OBS-CASE-COVERAGE-001",
        seen.len() == expected.len() && cases.len() == expected.len(),
        format!("{} unique cases; {} required", seen.len(), expected.len()),
    );

    let runner = string_at(manifest, &["execution", "runner"])?;
    let adapter = string_at(manifest, &["execution", "production_observation_adapter"])?;
    let external = at(manifest, &["execution", "external_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "external_data_ingested must be boolean".to_string())?;
    let synthetic_executed = at(manifest, &["execution", "synthetic_rule_cases_executed"])?
        .as_bool()
        .ok_or_else(|| "synthetic_rule_cases_executed must be boolean".to_string())?;
    let rights = string_at(manifest, &["execution", "rights_adjudication"])?;
    push_check(
        &mut checks,
        "OBS-SYNTHETIC-EXECUTION-BOUNDARY-001",
        runner == "SYNTHETIC_RULE_ENGINE"
            && adapter == "NOT_IMPLEMENTED"
            && !external
            && synthetic_executed
            && rights == "NOT_RUN",
        "the Rust rule engine executes synthetic descriptors only; production adapters, external data and rights decisions remain unexecuted",
    );

    let passed = checks.iter().all(|check| check.passed);
    let check_values: Vec<Value> = checks
        .iter()
        .map(|check| {
            json!({
                "id": check.id, "passed": check.passed, "detail": check.detail
            })
        })
        .collect();
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
    let k = number_at(
        manifest,
        &["synthetic_light_law", "extinction_coefficient_k"],
    )?;
    let calibrated = at(manifest, &["synthetic_light_law", "calibrated"])?
        .as_bool()
        .ok_or_else(|| "synthetic_light_law.calibrated must be boolean".to_string())?;
    let arrangements = at(manifest, &["arrangements"])?
        .as_array()
        .ok_or_else(|| "arrangements must be an array".to_string())?;
    if arrangements.len() != 2 {
        return Err("canopy v0.1 requires exactly two arrangements".to_string());
    }

    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "CANOPY-FIXTURE-IDENTITY-001",
        version == "0.1.0"
            && benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "CANOPY-GAP-REPRESENTATION-001",
        format!("version={version}; benchmark={benchmark}; corpus={corpus_id}"),
    );
    push_check(
        &mut checks,
        "CANOPY-CLAIM-CEILING-001",
        status == "ANALYTICAL_SYNTHETIC_FIXTURES_NOT_YET_EXECUTED"
            && !calibrated
            && ceiling.contains("not calibrated"),
        "light rule and fixture must remain synthetic and uncalibrated",
    );
    push_check(
        &mut checks,
        "CANOPY-LIGHT-LAW-001",
        law == "T(x,y) = exp(-k * LAI(x,y))" && k >= 0.0,
        format!("law={law}; k={k}; calibrated={calibrated}"),
    );

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
            width,
            height,
        )?;
        let (open, open_duplicates) = parse_coordinates(
            at(arrangement, &["open_coordinates"])?,
            &format!("arrangements[{index}].open_coordinates"),
            width,
            height,
        )?;
        let disjoint = occupied.is_disjoint(&open);
        let full_coverage = occupied.len() + open.len() == expected_cells
            && occupied.union(&open).count() == expected_cells;
        let counts_match = occupied.len() == occupied_expected
            && open.len() == open_expected
            && occupied_duplicates == 0
            && open_duplicates == 0;
        let total_lai = occupied.len() as f64 * occupied_lai + open.len() as f64 * open_lai;
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
        let baseline_mean_lai = number_at(manifest, &["uniform_patch_baseline", "mean_lai"])?;
        let baseline_transmittance = number_at(
            manifest,
            &[
                "uniform_patch_baseline",
                "predicted_transmittance_every_cell",
            ],
        )?;
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
            && close_enough(
                occupied_transmittance,
                expected_occupied_transmittance,
                1e-9,
            )
            && close_enough(open_transmittance, expected_open_transmittance, 1e-9)
            && close_enough(uniform_calculated, expected_uniform_transmittance, 1e-9)
            && close_enough(uniform_calculated, baseline_transmittance, 1e-9)
            && close_enough(mean_lai, baseline_mean_lai, 1e-9);

        push_check(
            &mut checks,
            "CANOPY-ARRANGEMENT-GEOMETRY-001",
            valid,
            format!(
                "arrangement={id}; cells={expected_cells}; occupied={}, open={}, disjoint={disjoint}, full_coverage={full_coverage}, components={components}/{declared_components}, total_lai={total_lai}, mean_lai={mean_lai}",
                occupied.len(),
                open.len()
            ),
        );

        if let Some(reference) = total_lai_reference {
            push_check(
                &mut checks,
                "CANOPY-AGGREGATE-EQUIVALENCE-001",
                close_enough(reference, total_lai, 1e-9),
                format!("arrangement={id}; total_lai={total_lai}; reference={reference}"),
            );
        } else {
            total_lai_reference = Some(total_lai);
        }
        if let Some(reference) = mean_lai_reference {
            push_check(
                &mut checks,
                "CANOPY-MEAN-EQUIVALENCE-001",
                close_enough(reference, mean_lai, 1e-9),
                format!("arrangement={id}; mean_lai={mean_lai}; reference={reference}"),
            );
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
        (
            "CANOPY-AGGREGATE-EQUAL-TOPOLOGY-DIFFERENT-001",
            "SAME_AGGREGATES_DIFFERENT_SPATIAL_GAP_TOPOLOGY",
        ),
        (
            "CANOPY-MORTALITY-LOCAL-GAP-001",
            "LOCAL_CANOPY_SUPPORT_CHANGES_ONLY",
        ),
        (
            "CANOPY-CLUSTERED-VS-DISPERSED-MORTALITY-001",
            "EQUAL_LOSS_DIFFERENT_GAP_GEOMETRY",
        ),
        (
            "CANOPY-RECRUITMENT-LOCAL-LIGHT-001",
            "LOCAL_LIGHT_REQUIRED_FOR_RECRUITMENT_CLAIM",
        ),
        (
            "CANOPY-GAP-CLOSURE-CAUSAL-001",
            "GAP_CLOSURE_REQUIRES_EXPLICIT_GROWTH_SETTLEMENT",
        ),
        (
            "CANOPY-TRANSLATION-SYMMETRY-001",
            "TRANSLATED_CASE_OUTPUTS_EQUIVALENT",
        ),
        (
            "CANOPY-ROTATION-SYMMETRY-001",
            "ROTATED_CASE_OUTPUTS_EQUIVALENT",
        ),
        (
            "CANOPY-DISCONNECTED-CONTROL-001",
            "DISCONNECTED_PATCH_UNCHANGED",
        ),
        (
            "CANOPY-RENDERER-INDEPENDENCE-001",
            "CANONICAL_ECOLOGY_IDENTICAL",
        ),
        (
            "CANOPY-COARSE-FIDELITY-PROMOTION-001",
            "PROMOTE_SPATIAL_REPRESENTATION_OR_ABSTAIN",
        ),
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
            .ok_or_else(|| {
                format!("challenge_cases[{index}].expected_disposition must be a string")
            })?;
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

    let components_differ =
        component_counts.len() == 2 && component_counts[0] != component_counts[1];
    let spatial_vs_uniform_differs = outputs.iter().all(|result| {
        let spatial = result["spatial_mean_transmittance"]
            .as_f64()
            .unwrap_or(f64::NAN);
        let uniform = result["uniform_patch_transmittance"]
            .as_f64()
            .unwrap_or(f64::NAN);
        (spatial - uniform).abs() > 0.05
    });
    push_check(
        &mut checks,
        "CANOPY-TOPOLOGY-DIFFERENCE-001",
        components_differ,
        format!("open component counts={component_counts:?}"),
    );
    push_check(
        &mut checks,
        "CANOPY-UNIFORM-BASELINE-LOSS-001",
        spatial_vs_uniform_differs,
        "spatially heterogeneous transmittance mean must differ from uniform transmission at the aggregate mean LAI",
    );

    let passed = checks.iter().all(|check| check.passed);
    let check_values: Vec<Value> = checks
        .iter()
        .map(|check| {
            json!({
                "id": check.id, "passed": check.passed, "detail": check.detail
            })
        })
        .collect();
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
        (
            "FOREST-DECISION-VALUE-001",
            "HIGH_INFORMATION_ZERO_DECISION_VALUE",
        ),
        ("FOREST-DECISION-VALUE-002", "POSITIVE_NET_DECISION_VALUE"),
        (
            "FOREST-DECISION-VALUE-003",
            "RANK_BY_EXPECTED_REGRET_REDUCTION",
        ),
        (
            "FOREST-DECISION-VALUE-004",
            "SAMPLE_PROPAGULE_LIMITATION_IF_DECISION_RELEVANT",
        ),
        (
            "FOREST-DECISION-VALUE-005",
            "ZERO_DECISION_VALUE_DESPITE_VARIANCE",
        ),
        (
            "FOREST-DECISION-VALUE-006",
            "SELECT_FEASIBLE_LOWER_TOTAL_COST",
        ),
        ("FOREST-DECISION-VALUE-007", "DECISION_FOCUSED_VERIFICATION"),
        (
            "FOREST-DECISION-VALUE-008",
            "NO_EXTRA_SURVEY_FOR_ZERO_ACTION_VALUE",
        ),
        (
            "FOREST-DECISION-VALUE-009",
            "MISSINGNESS_IS_NOT_NEGATIVE_EVIDENCE",
        ),
        (
            "FOREST-DECISION-VALUE-010",
            "PRIORITIZE_DECISION_IMPACT_NOT_VARIANCE",
        ),
        ("FOREST-DECISION-VALUE-011", "ZERO_MARGINAL_DECISION_VALUE"),
        (
            "FOREST-DECISION-VALUE-012",
            "FEASIBILITY_GATE_BEFORE_RANKING",
        ),
        (
            "FOREST-DECISION-VALUE-013",
            "SELECT_IF_EXPECTED_DECISION_REGRET_FALLS",
        ),
        (
            "FOREST-DECISION-VALUE-014",
            "DO_NOT_RANK_UNAUTHORIZED_OBSERVATION",
        ),
        (
            "FOREST-DECISION-VALUE-015",
            "PRESERVE_MODEL_IDENTITY_AND_DISCRIMINATION_VALUE",
        ),
        (
            "FOREST-DECISION-VALUE-016",
            "DECISION_VALUE_WITH_MODEL_SET_ADEQUACY",
        ),
        (
            "FOREST-DECISION-VALUE-017",
            "DO_NOT_OVER-SAMPLE_LOW_DECISION_VALUE",
        ),
        (
            "FOREST-DECISION-VALUE-018",
            "SAMPLE_TO_RESOLVE_OBSERVATION_PROCESS",
        ),
        (
            "FOREST-DECISION-VALUE-019",
            "EXCLUDE_UNAUTHORIZED_CANDIDATE",
        ),
        (
            "FOREST-DECISION-VALUE-020",
            "ACCOUNT_FOR_DEPENDENT_OBSERVATIONS",
        ),
    ]
    .into_iter()
    .collect();

    let version = string_at(manifest, &["schema_version"])?;
    let benchmark = string_at(manifest, &["benchmark_id"])?;
    let corpus_id = string_at(manifest, &["corpus_id"])?;
    let status = string_at(manifest, &["status"])?;
    let ceiling = string_at(manifest, &["claim_ceiling"])?;
    let hidden_outcomes = at(manifest, &["hidden_outcomes_published"])?
        .as_bool()
        .ok_or_else(|| "hidden_outcomes_published must be boolean".to_string())?;
    let blind_status = string_at(manifest, &["independent_blind_corpus"])?;
    let cases = at(manifest, &["cases"])?
        .as_array()
        .ok_or_else(|| "cases must be an array".to_string())?;

    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "DECISION-VALUE-CORPUS-VERSION-001",
        version == "0.1.0",
        format!("schema_version={version}"),
    );
    push_check(
        &mut checks,
        "DECISION-VALUE-CORPUS-IDENTITY-001",
        benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-DECISION-VALUE-001",
        format!("benchmark={benchmark}; corpus={corpus_id}"),
    );
    push_check(
        &mut checks,
        "DECISION-VALUE-PUBLIC-NOT-BLIND-001",
        status == "PUBLIC_SYNTHETIC_DEVELOPMENT_CASES_NOT_BLIND"
            && !hidden_outcomes
            && blind_status == "NOT_CREATED_REQUIRES_INDEPENDENT_CUSTODY"
            && ceiling.contains("No real site"),
        "public decision-value fixtures must not claim a blind or field evaluation",
    );

    let mut seen = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let id = case
            .get("case_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].case_id must be a string"))?;
        let family = case
            .get("family")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].family must be a string"))?;
        let scenario = case
            .get("scenario_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].scenario_id must be a string"))?;
        let condition = case
            .get("condition")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].condition must be a string"))?;
        let disposition = case
            .get("expected_disposition")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].expected_disposition must be a string"))?;
        let unique = seen.insert(id.to_string());
        let expected_disposition = expected_cases
            .iter()
            .find_map(|(candidate_id, value)| (*candidate_id == id).then_some(*value));
        push_check(
            &mut checks,
            "DECISION-VALUE-CASE-SEMANTICS-001",
            unique
                && expected_disposition == Some(disposition)
                && !family.trim().is_empty()
                && !scenario.trim().is_empty()
                && !condition.trim().is_empty(),
            format!(
                "case={id}; expected={}; declared={disposition}; unique={unique}",
                expected_disposition.unwrap_or("UNKNOWN_CASE")
            ),
        );
    }
    push_check(
        &mut checks,
        "DECISION-VALUE-CASE-COVERAGE-001",
        seen.len() == expected_cases.len() && cases.len() == expected_cases.len(),
        format!(
            "{} unique cases; {} required",
            seen.len(),
            expected_cases.len()
        ),
    );

    let zero = at(
        manifest,
        &["worked_examples", "high_information_zero_decision_value"],
    )?;
    let zero_prior = number_at(zero, &["prior_probability_high_turnover"])?;
    let zero_protect_high = number_at(zero, &["loss_matrix", "protect", "high_turnover"])?;
    let zero_protect_low = number_at(zero, &["loss_matrix", "protect", "low_turnover"])?;
    let zero_none_high = number_at(zero, &["loss_matrix", "do_not_protect", "high_turnover"])?;
    let zero_none_low = number_at(zero, &["loss_matrix", "do_not_protect", "low_turnover"])?;
    let zero_evsi = number_at(zero, &["expected_gross_evsi"])?;
    let zero_entropy_claim = string_at(zero, &["ideal_measurement_information_gain"])?;
    let zero_disposition = string_at(zero, &["expected_disposition"])?;
    let action_losses_are_state_invariant =
        close_enough(zero_protect_high, zero_protect_low, 1e-12)
            && close_enough(zero_none_high, zero_none_low, 1e-12);
    push_check(
        &mut checks,
        "DECISION-VALUE-HIGH-UNCERTAINTY-ZERO-EVSI-001",
        (0.0..=1.0).contains(&zero_prior)
            && action_losses_are_state_invariant
            && close_enough(zero_evsi, 0.0, 1e-12)
            && zero_entropy_claim == "positive"
            && zero_disposition == "HIGH_INFORMATION_ZERO_DECISION_VALUE",
        "state information cannot improve an action when the declared loss is invariant across states",
    );

    let positive = at(
        manifest,
        &[
            "worked_examples",
            "moderate_uncertainty_positive_decision_value",
        ],
    )?;
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
    let posterior_positive = if p_positive > 0.0 {
        prior * sensitivity / p_positive
    } else {
        f64::NAN
    };
    let posterior_negative = if p_negative > 0.0 {
        prior * (1.0 - sensitivity) / p_negative
    } else {
        f64::NAN
    };
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
    push_check(
        &mut checks,
        "DECISION-VALUE-EVSI-ARITHMETIC-001",
        worked_example_matches,
        format!(
            "baseline={baseline_loss}; p+={p_positive}; p-={p_negative}; post+={posterior_positive}; post-={posterior_negative}; after={expected_after}; gross={gross_evsi}; net={net_value}; action_switches={action_switches}"
        ),
    );

    let execution = at(manifest, &["execution"])?;
    let evaluator = string_at(execution, &["evaluator"])?;
    let uses_existing = at(execution, &["uses_existing_evsi_kernel"])?
        .as_bool()
        .ok_or_else(|| "uses_existing_evsi_kernel must be boolean".to_string())?;
    let duplicate = at(execution, &["duplicate_evsi_kernel_created"])?
        .as_bool()
        .ok_or_else(|| "duplicate_evsi_kernel_created must be boolean".to_string())?;
    let real_data = at(execution, &["real_field_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "real_field_data_ingested must be boolean".to_string())?;
    let action_authorized = at(execution, &["real_management_action_authorized"])?
        .as_bool()
        .ok_or_else(|| "real_management_action_authorized must be boolean".to_string())?;
    push_check(
        &mut checks,
        "DECISION-VALUE-NO-DUPLICATE-KERNEL-001",
        evaluator == "NOT_IMPLEMENTED"
            && uses_existing
            && !duplicate
            && !real_data
            && !action_authorized,
        "public fixture must not claim a second EVSI engine, field data, or decision authority",
    );

    let passed = checks.iter().all(|check| check.passed);
    let check_values: Vec<Value> = checks
        .iter()
        .map(|check| {
            json!({
                "id": check.id, "passed": check.passed, "detail": check.detail
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
        "public_case_count": seen.len(),
        "expected_gross_evsi_example": gross_evsi,
        "expected_net_value_after_cost_example": net_value,
        "decision_policy_performance": "NOT_ESTABLISHED",
        "existing_evsi_kernel_execution": "NOT_RUN"
    }))
}

fn finite_array_at(root: &Value, path: &[&str]) -> Result<Vec<f64>, String> {
    let values = at(root, path)?
        .as_array()
        .ok_or_else(|| format!("expected array at {}", path.join(".")))?;
    let mut result = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let number = value
            .as_f64()
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("expected finite number at {}[{index}]", path.join(".")))?;
        result.push(number);
    }
    Ok(result)
}

fn optional_number_matches(actual: Option<f64>, expected: &Value, tolerance: f64) -> bool {
    match (actual, expected.as_f64(), expected.is_null()) {
        (Some(actual), Some(expected), _) => close_enough(actual, expected, tolerance),
        (None, None, true) => true,
        _ => false,
    }
}

fn validate_block_conformal_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let version = string_at(manifest, &["schema_version"])?;
    let benchmark = string_at(manifest, &["benchmark_id"])?;
    let corpus_id = string_at(manifest, &["corpus_id"])?;
    let status = string_at(manifest, &["status"])?;
    let claim_ceiling = string_at(manifest, &["claim_ceiling"])?;
    let scenario_value = at(manifest, &["scenario"])?;

    let training_group_ids: Vec<String> = string_set_at(scenario_value, &["training_group_ids"])?
        .into_iter()
        .collect();
    let calibration_values = at(scenario_value, &["calibration_blocks"])?
        .as_array()
        .ok_or_else(|| "scenario.calibration_blocks must be an array".to_string())?;
    let test_values = at(scenario_value, &["test_blocks"])?
        .as_array()
        .ok_or_else(|| "scenario.test_blocks must be an array".to_string())?;

    let calibration_blocks = calibration_values
        .iter()
        .map(|block| {
            Ok(conformal_kernel::CalibrationBlock {
                block_id: string_at(block, &["block_id"])?,
                group_id: string_at(block, &["group_id"])?,
                absolute_residuals: finite_array_at(block, &["absolute_residuals"])?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let test_blocks = test_values
        .iter()
        .map(|block| {
            Ok(conformal_kernel::TestBlock {
                block_id: string_at(block, &["block_id"])?,
                group_id: string_at(block, &["group_id"])?,
                regime: string_at(block, &["regime"])?,
                point_predictions: finite_array_at(block, &["point_predictions"])?,
                observations: finite_array_at(block, &["observations"])?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let alpha = number_at(scenario_value, &["alpha"])?;
    let exchangeability_basis = string_at(scenario_value, &["exchangeability_basis"])?;
    let scenario = conformal_kernel::BlockConformalScenario {
        alpha,
        training_group_ids,
        calibration_blocks,
        test_blocks,
        exchangeability_basis,
    };
    let result = conformal_kernel::evaluate_block_conformal(&scenario)
        .map_err(|error| format!("block conformal evaluation failed: {error}"))?;

    let expected = at(manifest, &["expected"])?;
    let mut matches = result.calibration_block_count
        == usize_at(expected, &["calibration_block_count"])?
        && result.observations_per_block == usize_at(expected, &["observations_per_block"])?
        && result.test_block_count == usize_at(expected, &["test_block_count"])?
        && result.test_observation_count == usize_at(expected, &["test_observation_count"])?
        && optional_number_matches(
            result.pooled_observation_radius,
            expected
                .get("pooled_observation_radius")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            result.block_maximum_radius,
            expected.get("block_maximum_radius").unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            result.pooled_interval_width,
            expected
                .get("pooled_interval_width")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            result.block_interval_width,
            expected.get("block_interval_width").unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            result.pooled_mean_interval_score,
            expected
                .get("pooled_mean_interval_score")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && optional_number_matches(
            result.block_mean_interval_score,
            expected
                .get("block_mean_interval_score")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && optional_number_matches(
            result.pooled_observation_coverage,
            expected
                .get("pooled_observation_coverage")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            result.pooled_full_block_coverage,
            expected
                .get("pooled_full_block_coverage")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            result.block_observation_coverage,
            expected
                .get("block_observation_coverage")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            result.block_full_block_coverage,
            expected
                .get("block_full_block_coverage")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            result.declared_in_domain_block_coverage,
            expected
                .get("declared_in_domain_block_coverage")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            result.declared_shift_block_coverage,
            expected
                .get("declared_shift_block_coverage")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && result.exchangeability_status
            == string_at(expected, &["exchangeability_status"])?.as_str();

    let expected_blocks = at(expected, &["test_block_results"])?
        .as_array()
        .ok_or_else(|| "expected.test_block_results must be an array".to_string())?;
    matches &= result.test_block_results.len() == expected_blocks.len();
    for (actual, declared) in result.test_block_results.iter().zip(expected_blocks) {
        matches &= actual.block_id == string_at(declared, &["block_id"])?;
        matches &= actual.observation_count == usize_at(declared, &["observation_count"])?;
        matches &=
            actual.covered_observation_count == usize_at(declared, &["covered_observation_count"])?;
        matches &= close_enough(
            actual.observation_coverage_fraction,
            number_at(declared, &["observation_coverage_fraction"])?,
            1e-12,
        );
        matches &= actual.block_fully_covered
            == at(declared, &["block_fully_covered"])?
                .as_bool()
                .ok_or_else(|| "block_fully_covered must be boolean".to_string())?;
        matches &= actual.regime_is_declared_shift
            == at(declared, &["regime_is_declared_shift"])?
                .as_bool()
                .ok_or_else(|| "regime_is_declared_shift must be boolean".to_string())?;
        matches &= optional_number_matches(
            actual.pooled_mean_interval_score,
            declared
                .get("pooled_mean_interval_score")
                .unwrap_or(&Value::Null),
            1e-9,
        );
        matches &= optional_number_matches(
            actual.block_mean_interval_score,
            declared
                .get("block_mean_interval_score")
                .unwrap_or(&Value::Null),
            1e-9,
        );
    }

    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "BLOCK-CONFORMAL-CORPUS-IDENTITY-001",
        version == "0.1.0"
            && benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-BLOCK-CONFORMAL-001"
            && status == "PUBLIC_SYNTHETIC_BLOCK_CONFORMAL_NOT_STATISTICAL_VALIDATION"
            && claim_ceiling.contains("Nine equal-sized synthetic calibration blocks")
            && claim_ceiling.contains("not statistically established"),
        format!("version={version}; benchmark={benchmark}; corpus={corpus_id}"),
    );
    push_check(
        &mut checks,
        "BLOCK-CONFORMAL-QUANTILE-AND-COVERAGE-001",
        matches,
        format!(
            "calibration_blocks={}; pooled_radius={:?}; block_maximum_radius={:?}; pooled_winkler={:?}; block_winkler={:?}; pooled_point_coverage={:?}; block_point_coverage={:?}; block_simultaneous_coverage={:?}; in_domain_blocks={:?}; shifted_blocks={:?}",
            result.calibration_block_count,
            result.pooled_observation_radius,
            result.block_maximum_radius,
            result.pooled_mean_interval_score,
            result.block_mean_interval_score,
            result.pooled_observation_coverage,
            result.block_observation_coverage,
            result.block_full_block_coverage,
            result.declared_in_domain_block_coverage,
            result.declared_shift_block_coverage
        ),
    );

    let execution = at(manifest, &["execution"])?;
    let runner = string_at(execution, &["runner"])?;
    let external = at(execution, &["external_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "execution.external_data_ingested must be boolean".to_string())?;
    let exchangeability = string_at(execution, &["exchangeability"])?;
    let ecological_coverage = string_at(execution, &["real_forest_coverage"])?;
    let shift_guarantee = string_at(execution, &["distribution_shift_guarantee"])?;
    push_check(
        &mut checks,
        "BLOCK-CONFORMAL-CLAIM-BOUNDARY-001",
        runner == "BLOCK_MAXIMUM_SPLIT_CONFORMAL_SYNTHETIC"
            && !external
            && exchangeability == "DESIGNER_DECLARED_NOT_VERIFIED"
            && ecological_coverage == "NOT_ESTABLISHED"
            && shift_guarantee == "NOT_ESTABLISHED",
        "synthetic block-calibration arithmetic only; group separation does not verify exchangeability or shift validity",
    );

    let test_block_rows: Vec<Value> = result
        .test_block_results
        .iter()
        .map(|block| {
            json!({
                "block_id": block.block_id,
                "observation_count": block.observation_count,
                "covered_observation_count": block.covered_observation_count,
                "observation_coverage_fraction": block.observation_coverage_fraction,
                "block_fully_covered": block.block_fully_covered,
                "pooled_mean_interval_score": block.pooled_mean_interval_score,
                "block_mean_interval_score": block.block_mean_interval_score,
                "regime_is_declared_shift": block.regime_is_declared_shift
            })
        })
        .collect();

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
        "result": if passed { "PASS_SYNTHETIC_BLOCK_CONFORMAL" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "calibration_block_count": result.calibration_block_count,
        "observations_per_block": result.observations_per_block,
        "pooled_observation_radius": result.pooled_observation_radius,
        "block_maximum_radius": result.block_maximum_radius,
        "pooled_interval_width": result.pooled_interval_width,
        "block_interval_width": result.block_interval_width,
        "pooled_mean_interval_score": result.pooled_mean_interval_score,
        "block_mean_interval_score": result.block_mean_interval_score,
        "test_block_count": result.test_block_count,
        "test_observation_count": result.test_observation_count,
        "pooled_observation_coverage": result.pooled_observation_coverage,
        "pooled_full_block_coverage": result.pooled_full_block_coverage,
        "block_observation_coverage": result.block_observation_coverage,
        "block_full_block_coverage": result.block_full_block_coverage,
        "declared_in_domain_block_coverage": result.declared_in_domain_block_coverage,
        "declared_shift_block_coverage": result.declared_shift_block_coverage,
        "test_block_results": test_block_rows,
        "exchangeability_status": result.exchangeability_status,
        "real_forest_coverage": "NOT_ESTABLISHED",
        "distribution_shift_guarantee": "NOT_ESTABLISHED",
        "external_data_ingested": false
    }))
}

fn validate_binary_event_brier_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let version = string_at(manifest, &["schema_version"])?;
    let benchmark = string_at(manifest, &["benchmark_id"])?;
    let corpus_id = string_at(manifest, &["corpus_id"])?;
    let status = string_at(manifest, &["status"])?;
    let ceiling = string_at(manifest, &["claim_ceiling"])?;
    let event_name = string_at(manifest, &["target", "event_name"])?;
    let horizon = u32::try_from(usize_at(manifest, &["target", "horizon_days"])?)
        .map_err(|_| "target.horizon_days exceeds u32".to_string())?;
    let baseline_kind = string_at(manifest, &["baseline_kind"])?;
    let cases = at(manifest, &["cases"])?
        .as_array()
        .ok_or_else(|| "cases must be an array".to_string())?;

    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "BRIER-CORPUS-IDENTITY-001",
        version == "0.1.0"
            && benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-BINARY-EVENT-BRIER-001"
            && status == "PUBLIC_SYNTHETIC_BINARY_EVENT_SCORE_CASES_NOT_ECOLOGICAL_CALIBRATION"
            && ceiling.contains("Six public synthetic binary-event forecasts")
            && ceiling.contains("No event prevalence")
            && baseline_kind == "FIXED_SYNTHETIC_BERNOULLI_REFERENCE",
        format!(
            "version={version}; benchmark={benchmark}; corpus={corpus_id}; event={event_name}; horizon={horizon}"
        ),
    );

    let mut event_cases = Vec::with_capacity(cases.len());
    let mut seen = BTreeSet::new();
    let mut case_rows = Vec::with_capacity(cases.len());
    for case in cases {
        let forecast_id = string_at(case, &["forecast_id"])?;
        let unique = seen.insert(forecast_id.clone());
        let event_probability = match at(case, &["event_probability"])? {
            value if value.is_null() => None,
            value => Some(
                value
                    .as_f64()
                    .filter(|number| number.is_finite())
                    .ok_or_else(|| {
                        format!(
                            "forecast_id={forecast_id}; event_probability must be finite or null"
                        )
                    })?,
            ),
        };
        let event_case = forecast_kernel::BinaryEventForecast {
            forecast_id: forecast_id.clone(),
            event_name: string_at(case, &["event_name"])?,
            horizon_days: u32::try_from(usize_at(case, &["horizon_days"])?)
                .map_err(|_| "case.horizon_days exceeds u32".to_string())?,
            event_probability,
            observed_event: at(case, &["observed_event"])?
                .as_bool()
                .ok_or_else(|| "observed_event must be boolean".to_string())?,
            baseline_probability: number_at(case, &["baseline_probability"])?,
        };
        let score = forecast_kernel::score_binary_event_case(&event_case)
            .map_err(|error| format!("forecast_id={forecast_id}; Brier scoring failed: {error}"))?;
        let expected = at(case, &["expected"])?;
        let case_matches = unique
            && optional_number_matches(
                score.model_brier,
                expected.get("model_brier").unwrap_or(&Value::Null),
                1e-12,
            )
            && close_enough(
                score.baseline_brier,
                number_at(expected, &["baseline_brier"])?,
                1e-12,
            );
        push_check(
            &mut checks,
            "BRIER-CASE-SCORE-ORACLE-001",
            case_matches,
            format!(
                "forecast_id={forecast_id}; model_brier={:?}; baseline_brier={}; unique={unique}; expected_fields_match={case_matches}",
                score.model_brier, score.baseline_brier
            ),
        );
        case_rows.push(json!({
            "forecast_id": forecast_id,
            "model_brier": score.model_brier,
            "baseline_brier": score.baseline_brier
        }));
        event_cases.push(event_case);
    }

    let summary = forecast_kernel::summarize_binary_event_forecasts(&event_cases)
        .map_err(|error| format!("binary event Brier summary failed: {error}"))?;
    let expected_summary = at(manifest, &["expected_summary"])?;
    let expected_disposition = string_at(expected_summary, &["disposition"])?;
    let expected_bins = at(expected_summary, &["reliability_bins"])?
        .as_array()
        .ok_or_else(|| "expected_summary.reliability_bins must be an array".to_string())?;
    let mut bins_match =
        expected_bins.len() == summary.reliability_bins.len() && expected_bins.len() == 5;
    for (index, (actual, expected)) in summary
        .reliability_bins
        .iter()
        .zip(expected_bins)
        .enumerate()
    {
        bins_match &= actual.case_count == usize_at(expected, &["case_count"])?;
        bins_match &= close_enough(
            actual.lower_bound,
            number_at(expected, &["lower_bound"])?,
            1e-12,
        );
        bins_match &= close_enough(
            actual.upper_bound,
            number_at(expected, &["upper_bound"])?,
            1e-12,
        );
        bins_match &= optional_number_matches(
            actual.mean_forecast_probability,
            expected
                .get("mean_forecast_probability")
                .unwrap_or(&Value::Null),
            1e-12,
        );
        bins_match &= optional_number_matches(
            actual.observed_event_frequency,
            expected
                .get("observed_event_frequency")
                .unwrap_or(&Value::Null),
            1e-12,
        );
        if index >= 5 {
            bins_match = false;
        }
    }

    let summary_matches = summary.eligible_case_count
        == usize_at(expected_summary, &["eligible_case_count"])?
        && summary.forecast_case_count == usize_at(expected_summary, &["forecast_case_count"])?
        && summary.abstained_case_count == usize_at(expected_summary, &["abstained_case_count"])?
        && close_enough(
            summary.forecast_coverage_fraction,
            number_at(expected_summary, &["forecast_coverage_fraction"])?,
            1e-12,
        )
        && optional_number_matches(
            summary.mean_model_brier,
            expected_summary
                .get("mean_model_brier")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            summary.mean_baseline_brier_paired,
            expected_summary
                .get("mean_baseline_brier_paired")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && close_enough(
            summary.mean_baseline_brier_all,
            number_at(expected_summary, &["mean_baseline_brier_all"])?,
            1e-12,
        )
        && optional_number_matches(
            summary.brier_skill_score,
            expected_summary
                .get("brier_skill_score")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && summary.disposition == expected_disposition.as_str()
        && bins_match;

    push_check(
        &mut checks,
        "BRIER-SUMMARY-RELIABILITY-001",
        summary_matches
            && summary.eligible_case_count == cases.len()
            && seen.len() == cases.len()
            && event_name
                == event_cases
                    .first()
                    .map(|case| case.event_name.as_str())
                    .unwrap_or("")
            && Some(horizon) == event_cases.first().map(|case| case.horizon_days),
        format!(
            "eligible={}; forecasts={}; abstained={}; coverage={}; model_brier={:?}; paired_baseline={:?}; skill={:?}; reliability_bins_match={bins_match}",
            summary.eligible_case_count,
            summary.forecast_case_count,
            summary.abstained_case_count,
            summary.forecast_coverage_fraction,
            summary.mean_model_brier,
            summary.mean_baseline_brier_paired,
            summary.brier_skill_score
        ),
    );

    let execution = at(manifest, &["execution"])?;
    let runner = string_at(execution, &["runner"])?;
    let external = at(execution, &["external_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "execution.external_data_ingested must be boolean".to_string())?;
    let calibration = string_at(execution, &["event_calibration"])?;
    let ecological_skill = string_at(execution, &["ecological_forecast_skill"])?;
    let blind = string_at(execution, &["blind_evaluation"])?;
    push_check(
        &mut checks,
        "BRIER-SYNTHETIC-CLAIM-BOUNDARY-001",
        runner == "BERNOULLI_BRIER_AND_FIXED_BIN_RELIABILITY"
            && !external
            && calibration == "NOT_ESTABLISHED"
            && ecological_skill == "NOT_ESTABLISHED"
            && blind == "NOT_CREATED",
        "synthetic Bernoulli scoring only; no event calibration, ecological forecast skill or blind evaluation is claimed",
    );

    let reliability_bin_rows: Vec<Value> = summary
        .reliability_bins
        .iter()
        .map(|bin| {
            json!({
                "lower_bound": bin.lower_bound,
                "upper_bound": bin.upper_bound,
                "case_count": bin.case_count,
                "mean_forecast_probability": bin.mean_forecast_probability,
                "observed_event_frequency": bin.observed_event_frequency
            })
        })
        .collect();
    let brier_summary = json!({
        "eligible_case_count": summary.eligible_case_count,
        "forecast_case_count": summary.forecast_case_count,
        "abstained_case_count": summary.abstained_case_count,
        "forecast_coverage_fraction": summary.forecast_coverage_fraction,
        "mean_model_brier": summary.mean_model_brier,
        "mean_baseline_brier_paired": summary.mean_baseline_brier_paired,
        "mean_baseline_brier_all": summary.mean_baseline_brier_all,
        "brier_skill_score": summary.brier_skill_score,
        "reliability_bins": reliability_bin_rows,
        "disposition": summary.disposition
    });

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
        "result": if passed { "PASS_SYNTHETIC_BRIER" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "summary": brier_summary,
        "case_scores": case_rows,
        "event_calibration": "NOT_ESTABLISHED",
        "ecological_forecast_skill": "NOT_ESTABLISHED",
        "external_data_ingested": false
    }))
}

fn validate_probabilistic_forecast_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let version = string_at(manifest, &["schema_version"])?;
    let benchmark = string_at(manifest, &["benchmark_id"])?;
    let corpus_id = string_at(manifest, &["corpus_id"])?;
    let status = string_at(manifest, &["status"])?;
    let ceiling = string_at(manifest, &["claim_ceiling"])?;
    let variable = string_at(manifest, &["target", "variable"])?;
    let unit = string_at(manifest, &["target", "unit"])?;
    let horizon = u32::try_from(usize_at(manifest, &["target", "horizon_days"])?)
        .map_err(|_| "target.horizon_days exceeds u32".to_string())?;
    let baseline_kind = string_at(manifest, &["baseline_kind"])?;
    let cases = at(manifest, &["cases"])?
        .as_array()
        .ok_or_else(|| "cases must be an array".to_string())?;

    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "FORECAST-CORPUS-IDENTITY-001",
        version == "0.1.0"
            && benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-PROBABILISTIC-FORECAST-SCORING-001"
            && status == "PUBLIC_SYNTHETIC_PROBABILISTIC_SCORE_CASES_NOT_ECOLOGICAL_CALIBRATION"
            && ceiling.contains("Six public synthetic forecast cases")
            && ceiling.contains("not a calibrated forecast")
            && baseline_kind == "SYNTHETIC_POINT_MASS_REFERENCE",
        format!(
            "version={version}; benchmark={benchmark}; corpus={corpus_id}; target={variable} [{unit}] horizon={horizon}"
        ),
    );

    let mut forecast_cases = Vec::with_capacity(cases.len());
    for case in cases {
        forecast_cases.push(forecast_kernel::ForecastCase {
            forecast_id: string_at(case, &["forecast_id"])?,
            variable: string_at(case, &["variable"])?,
            unit: string_at(case, &["unit"])?,
            horizon_days: u32::try_from(usize_at(case, &["horizon_days"])?)
                .map_err(|_| "case.horizon_days exceeds u32".to_string())?,
            observed: number_at(case, &["observed"])?,
            model_samples: finite_array_at(case, &["model_samples"])?,
            baseline_samples: finite_array_at(case, &["baseline_samples"])?,
        });
    }

    let summary = forecast_kernel::summarize_forecasts(&forecast_cases)
        .map_err(|error| format!("forecast scoring failed: {error}"))?;
    let mut seen_forecast_ids = BTreeSet::new();
    let mut forecast_rows = Vec::with_capacity(cases.len());
    for (manifest_case, forecast_case) in cases.iter().zip(&forecast_cases) {
        let forecast_id = string_at(manifest_case, &["forecast_id"])?;
        let unique = seen_forecast_ids.insert(forecast_id.clone());
        let expected = at(manifest_case, &["expected"])?;
        let score = forecast_kernel::score_forecast_case(forecast_case)
            .map_err(|error| format!("forecast_id={forecast_id}; scoring failed: {error}"))?;
        let case_matches = unique
            && optional_number_matches(
                score.model_crps,
                expected.get("model_crps").unwrap_or(&Value::Null),
                1e-9,
            )
            && close_enough(
                score.baseline_crps,
                number_at(expected, &["baseline_crps"])?,
                1e-9,
            )
            && optional_number_matches(
                score.model_wis,
                expected.get("model_wis").unwrap_or(&Value::Null),
                1e-9,
            )
            && close_enough(
                score.baseline_wis,
                number_at(expected, &["baseline_wis"])?,
                1e-9,
            )
            && optional_number_matches(
                score.model_mean,
                expected.get("model_mean").unwrap_or(&Value::Null),
                1e-9,
            )
            && optional_number_matches(
                score.absolute_error,
                expected.get("absolute_error").unwrap_or(&Value::Null),
                1e-9,
            )
            && optional_number_matches(
                score.median,
                expected.get("median").unwrap_or(&Value::Null),
                1e-9,
            )
            && optional_number_matches(
                score.lower_50,
                expected.get("lower_50").unwrap_or(&Value::Null),
                1e-9,
            )
            && optional_number_matches(
                score.upper_50,
                expected.get("upper_50").unwrap_or(&Value::Null),
                1e-9,
            )
            && optional_number_matches(
                score.lower_80,
                expected.get("lower_80").unwrap_or(&Value::Null),
                1e-9,
            )
            && optional_number_matches(
                score.upper_80,
                expected.get("upper_80").unwrap_or(&Value::Null),
                1e-9,
            )
            && optional_number_matches(
                score.lower_95,
                expected.get("lower_95").unwrap_or(&Value::Null),
                1e-9,
            )
            && optional_number_matches(
                score.upper_95,
                expected.get("upper_95").unwrap_or(&Value::Null),
                1e-9,
            );
        push_check(
            &mut checks,
            "FORECAST-CASE-SCORE-ORACLE-001",
            case_matches,
            format!(
                "forecast_id={forecast_id}; model_crps={:?}; baseline_crps={}; model_mean={:?}; unique={unique}; expected_fields_match={case_matches}",
                score.model_crps, score.baseline_crps, score.model_mean
            ),
        );
        forecast_rows.push(json!({
            "forecast_id": forecast_id,
            "model_crps": score.model_crps,
            "baseline_crps": score.baseline_crps,
            "model_wis": score.model_wis,
            "baseline_wis": score.baseline_wis,
            "model_mean": score.model_mean,
            "absolute_error": score.absolute_error,
            "median": score.median,
            "lower_50": score.lower_50,
            "upper_50": score.upper_50,
            "lower_80": score.lower_80,
            "upper_80": score.upper_80,
            "lower_95": score.lower_95,
            "upper_95": score.upper_95
        }));
    }
    push_check(
        &mut checks,
        "FORECAST-CASE-IDENTITY-001",
        seen_forecast_ids.len() == cases.len() && cases.len() == 6,
        format!(
            "{} unique forecast IDs; {} cases",
            seen_forecast_ids.len(),
            cases.len()
        ),
    );
    let expected_summary = at(manifest, &["expected_summary"])?;
    let summary_matches = summary.eligible_case_count
        == usize_at(expected_summary, &["eligible_case_count"])?
        && summary.forecast_case_count == usize_at(expected_summary, &["forecast_case_count"])?
        && summary.abstained_case_count == usize_at(expected_summary, &["abstained_case_count"])?
        && close_enough(
            summary.forecast_coverage_fraction,
            number_at(expected_summary, &["forecast_coverage_fraction"])?,
            1e-12,
        )
        && optional_number_matches(
            summary.mean_model_crps,
            expected_summary
                .get("mean_model_crps")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && optional_number_matches(
            summary.mean_baseline_crps_paired,
            expected_summary
                .get("mean_baseline_crps_paired")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && optional_number_matches(
            summary.mean_model_wis,
            expected_summary
                .get("mean_model_wis")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && optional_number_matches(
            summary.mean_baseline_wis_paired,
            expected_summary
                .get("mean_baseline_wis_paired")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && close_enough(
            summary.mean_baseline_wis_all,
            number_at(expected_summary, &["mean_baseline_wis_all"])?,
            1e-9,
        )
        && optional_number_matches(
            summary.wis_skill_score,
            expected_summary
                .get("wis_skill_score")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && close_enough(
            summary.mean_baseline_crps_all,
            number_at(expected_summary, &["mean_baseline_crps_all"])?,
            1e-9,
        )
        && optional_number_matches(
            summary.crps_skill_score,
            expected_summary
                .get("crps_skill_score")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && optional_number_matches(
            summary.mean_absolute_error,
            expected_summary
                .get("mean_absolute_error")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && optional_number_matches(
            summary.interval_coverage_50,
            expected_summary
                .get("interval_coverage_50")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            summary.interval_coverage_80,
            expected_summary
                .get("interval_coverage_80")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            summary.interval_coverage_95,
            expected_summary
                .get("interval_coverage_95")
                .unwrap_or(&Value::Null),
            1e-12,
        )
        && optional_number_matches(
            summary.mean_interval_width_80,
            expected_summary
                .get("mean_interval_width_80")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && optional_number_matches(
            summary.mean_interval_width_95,
            expected_summary
                .get("mean_interval_width_95")
                .unwrap_or(&Value::Null),
            1e-9,
        )
        && summary.disposition == string_at(expected_summary, &["disposition"])?.as_str();

    push_check(
        &mut checks,
        "FORECAST-PAIRED-CRPS-AND-COVERAGE-001",
        summary_matches
            && summary.eligible_case_count == cases.len()
            && variable
                == forecast_cases
                    .first()
                    .map(|case| case.variable.as_str())
                    .unwrap_or("")
            && unit
                == forecast_cases
                    .first()
                    .map(|case| case.unit.as_str())
                    .unwrap_or("")
            && Some(horizon) == forecast_cases.first().map(|case| case.horizon_days),
        format!(
            "eligible={}; forecasts={}; abstained={}; coverage={}; mean_crps={:?}; paired_baseline={:?}; crps_skill={:?}",
            summary.eligible_case_count,
            summary.forecast_case_count,
            summary.abstained_case_count,
            summary.forecast_coverage_fraction,
            summary.mean_model_crps,
            summary.mean_baseline_crps_paired,
            summary.crps_skill_score
        ),
    );

    let risk_values = at(manifest, &["risk_coverage_cases"])?
        .as_array()
        .ok_or_else(|| "risk_coverage_cases must be an array".to_string())?;
    let mut risk_cases = Vec::with_capacity(risk_values.len());
    for case in risk_values {
        risk_cases.push(forecast_kernel::SelectiveRiskCase {
            forecast_id: string_at(case, &["forecast_id"])?,
            confidence_score: number_at(case, &["confidence_score"])?,
            forecast_samples: finite_array_at(case, &["forecast_samples"])?,
            observed: number_at(case, &["observed"])?,
        });
    }
    let risk_curve = forecast_kernel::selective_risk_coverage_curve(&risk_cases)
        .map_err(|error| format!("risk-coverage curve failed: {error}"))?;
    let expected_curve = at(manifest, &["expected_risk_coverage_curve"])?
        .as_array()
        .ok_or_else(|| "expected_risk_coverage_curve must be an array".to_string())?;
    let mut risk_curve_matches =
        risk_curve.len() == expected_curve.len() && risk_curve.len() == risk_cases.len();
    for (index, (actual, expected)) in risk_curve.iter().zip(expected_curve).enumerate() {
        risk_curve_matches &= actual.selected_count == usize_at(expected, &["selected_count"])?;
        risk_curve_matches &= close_enough(
            actual.coverage_fraction,
            number_at(expected, &["coverage_fraction"])?,
            1e-12,
        );
        risk_curve_matches &= close_enough(
            actual.mean_crps_risk,
            number_at(expected, &["mean_crps_risk"])?,
            1e-9,
        );
        if index >= risk_cases.len() {
            risk_curve_matches = false;
        }
    }
    push_check(
        &mut checks,
        "FORECAST-RISK-COVERAGE-CURVE-001",
        risk_curve_matches,
        format!(
            "{} curve points; full confidence-ranked CRPS risk–coverage curve matches={risk_curve_matches}",
            risk_curve.len()
        ),
    );

    let execution = at(manifest, &["execution"])?;
    let runner = string_at(execution, &["runner"])?;
    let external = at(execution, &["external_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "execution.external_data_ingested must be boolean".to_string())?;
    let calibration = string_at(execution, &["probabilistic_calibration"])?;
    let ecological_skill = string_at(execution, &["ecological_forecast_skill"])?;
    let blind = string_at(execution, &["blind_evaluation"])?;
    push_check(
        &mut checks,
        "FORECAST-SYNTHETIC-CLAIM-BOUNDARY-001",
        runner == "EMPIRICAL_ENSEMBLE_CRPS_AND_SELECTIVE_COVERAGE"
            && !external
            && calibration == "NOT_ESTABLISHED"
            && ecological_skill == "NOT_ESTABLISHED"
            && blind == "NOT_CREATED",
        "synthetic empirical-ensemble scoring only; no calibration, ecological skill or blind evaluation is claimed",
    );

    let risk_curve_rows: Vec<Value> = risk_curve
        .iter()
        .map(|point| {
            json!({
                "selected_count": point.selected_count,
                "coverage_fraction": point.coverage_fraction,
                "mean_crps_risk": point.mean_crps_risk
            })
        })
        .collect();
    let forecast_summary = json!({
        "eligible_case_count": summary.eligible_case_count,
        "forecast_case_count": summary.forecast_case_count,
        "abstained_case_count": summary.abstained_case_count,
        "forecast_coverage_fraction": summary.forecast_coverage_fraction,
        "mean_model_crps": summary.mean_model_crps,
        "mean_baseline_crps_paired": summary.mean_baseline_crps_paired,
        "mean_baseline_crps_all": summary.mean_baseline_crps_all,
        "mean_model_wis": summary.mean_model_wis,
        "mean_baseline_wis_paired": summary.mean_baseline_wis_paired,
        "mean_baseline_wis_all": summary.mean_baseline_wis_all,
        "wis_skill_score": summary.wis_skill_score,
        "crps_skill_score": summary.crps_skill_score,
        "mean_absolute_error": summary.mean_absolute_error,
        "interval_coverage_50": summary.interval_coverage_50,
        "interval_coverage_80": summary.interval_coverage_80,
        "interval_coverage_95": summary.interval_coverage_95,
        "mean_interval_width_80": summary.mean_interval_width_80,
        "mean_interval_width_95": summary.mean_interval_width_95,
        "disposition": summary.disposition
    });

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
        "result": if passed { "PASS_SYNTHETIC_FORECAST_SCORING" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "summary": forecast_summary,
        "forecast_scores": forecast_rows,
        "risk_coverage_curve": risk_curve_rows,
        "probabilistic_calibration": "NOT_ESTABLISHED",
        "ecological_forecast_skill": "NOT_ESTABLISHED",
        "external_data_ingested": false
    }))
}

fn validate_executable_model_set_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let expected_ids: BTreeSet<&'static str> = [
        "MODELSET-KNOWN-GROWTH-MORTALITY-001",
        "MODELSET-KNOWN-RECRUITMENT-GROWTH-001",
        "MODELSET-KNOWN-DELAYED-RESPONSE-001",
        "MODELSET-KNOWN-MATCHED-CONTROL-001",
        "MODELSET-KNOWN-HISTORY-FIT-FAILURE-001",
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
        "MODELSET-DIAGNOSTIC-CORPUS-IDENTITY-001",
        version == "0.1.0"
            && benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-MODEL-SET-KNOWN-TRUTH-DIAGNOSTICS-001"
            && status == "PUBLIC_SYNTHETIC_KNOWN_TRUTH_NOT_BLIND"
            && hidden_truth_published
            && blind_status == "NOT_BLIND_PUBLIC_KNOWN_TRUTH"
            && claim_ceiling.contains("Five public analytic known-truth scenarios")
            && claim_ceiling.contains("not a blind benchmark"),
        format!(
            "version={version}; benchmark={benchmark}; corpus={corpus_id}; blind_status={blind_status}"
        ),
    );

    let mut result_rows = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let case_id = string_at(case, &["case_id"])?;
        let unique = seen.insert(case_id.clone());
        let observed_history = number_at(case, &["observed_historical_total_biomass"])?;
        let observed_validation = model_set_kernel::CohortState {
            small_live_biomass: number_at(
                case,
                &["observed_validation_state", "small_live_biomass"],
            )?,
            large_live_biomass: number_at(
                case,
                &["observed_validation_state", "large_live_biomass"],
            )?,
        };
        let historical_fit_tolerance =
            number_at(case, &["thresholds", "historical_fit_tolerance"])?;
        let structural_disagreement_threshold =
            number_at(case, &["thresholds", "structural_disagreement_threshold"])?;
        let common_omission_residual_threshold =
            number_at(case, &["thresholds", "common_omission_residual_threshold"])?;
        let candidate_values = at(case, &["candidates"])?
            .as_array()
            .ok_or_else(|| format!("cases[{index}].candidates must be an array"))?;
        let mut candidates = Vec::with_capacity(candidate_values.len());
        for candidate in candidate_values {
            candidates.push(model_set_kernel::CandidateForecast {
                model_id: string_at(candidate, &["model_id"])?,
                historical_total_biomass: number_at(candidate, &["historical_total_biomass"])?,
                predicted_validation_state: model_set_kernel::CohortState {
                    small_live_biomass: number_at(
                        candidate,
                        &["predicted_validation_state", "small_live_biomass"],
                    )?,
                    large_live_biomass: number_at(
                        candidate,
                        &["predicted_validation_state", "large_live_biomass"],
                    )?,
                },
            });
        }

        let scenario = model_set_kernel::KnownTruthScenario {
            observed_historical_total_biomass: observed_history,
            observed_validation_state: observed_validation,
            historical_fit_tolerance,
            structural_disagreement_threshold,
            common_omission_residual_threshold,
            candidates,
        };
        let result = model_set_kernel::evaluate_known_truth_scenario(&scenario)
            .map_err(|error| format!("case={case_id}; model-set diagnostic error={error}"))?;
        let expected = at(case, &["expected"])?;
        let expected_disposition = string_at(expected, &["disposition"])?;
        let disposition_matches = result.disposition.as_str() == expected_disposition;
        let metric_matches = result.candidate_count == usize_at(expected, &["candidate_count"])?
            && result.all_candidates_fit_history
                == at(expected, &["all_candidates_fit_history"])?
                    .as_bool()
                    .ok_or_else(|| {
                        "expected.all_candidates_fit_history must be boolean".to_string()
                    })?
            && close_enough(
                result.maximum_pairwise_cohort_span,
                number_at(expected, &["maximum_pairwise_cohort_span"])?,
                1e-9,
            )
            && close_enough(
                result.minimum_validation_residual,
                number_at(expected, &["minimum_validation_residual"])?,
                1e-9,
            )
            && close_enough(
                result.maximum_validation_residual,
                number_at(expected, &["maximum_validation_residual"])?,
                1e-9,
            );
        let known = expected_ids.contains(case_id.as_str());
        let passed = unique && known && disposition_matches && metric_matches;
        push_check(
            &mut checks,
            "MODELSET-DIAGNOSTIC-KNOWN-TRUTH-001",
            passed,
            format!(
                "case={case_id}; derived={}; expected={expected_disposition}; history_fit={}; span={}; residual_min={}; residual_max={}; metric_match={metric_matches}; unique={unique}",
                result.disposition.as_str(),
                result.all_candidates_fit_history,
                result.maximum_pairwise_cohort_span,
                result.minimum_validation_residual,
                result.maximum_validation_residual
            ),
        );
        result_rows.push(json!({
            "case_id": case_id,
            "disposition": result.disposition.as_str(),
            "candidate_count": result.candidate_count,
            "all_candidates_fit_history": result.all_candidates_fit_history,
            "maximum_pairwise_cohort_span": result.maximum_pairwise_cohort_span,
            "minimum_validation_residual": result.minimum_validation_residual,
            "maximum_validation_residual": result.maximum_validation_residual
        }));
    }

    push_check(
        &mut checks,
        "MODELSET-DIAGNOSTIC-CASE-COVERAGE-001",
        seen.len() == expected_ids.len()
            && cases.len() == expected_ids.len()
            && seen
                .iter()
                .all(|case_id| expected_ids.contains(case_id.as_str())),
        format!(
            "{} unique cases; {} required",
            seen.len(),
            expected_ids.len()
        ),
    );

    let execution = at(manifest, &["execution"])?;
    let runner = string_at(execution, &["runner"])?;
    let external = at(execution, &["external_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "execution.external_data_ingested must be boolean".to_string())?;
    let detector = string_at(execution, &["candidate_detector"])?;
    let custodian = string_at(execution, &["independent_custodian"])?;
    let forecast = string_at(execution, &["probabilistic_forecast"])?;
    push_check(
        &mut checks,
        "MODELSET-DIAGNOSTIC-CLAIM-BOUNDARY-001",
        runner == "SYNTHETIC_KNOWN_TRUTH_COHORT_MODEL_SET_KERNEL"
            && !external
            && detector == "TOY_DIAGNOSTIC_ONLY"
            && custodian == "NOT_APPOINTED"
            && forecast == "NOT_IMPLEMENTED",
        "toy deterministic cohort diagnostic only; no external data, independent blind custodian, calibrated ecology or probabilistic forecast",
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
        "result": if passed { "PASS_SYNTHETIC_MODEL_SET_DIAGNOSTICS" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "case_count": seen.len(),
        "scenario_results": result_rows,
        "blind_test_created": false,
        "empirical_model_set_adequacy": "NOT_ESTABLISHED",
        "ecological_calibration": "NOT_ESTABLISHED"
    }))
}

fn validate_model_set_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let expected_families: BTreeMap<&'static str, &'static str> = [
        (
            "growth_mortality_compensation",
            "PRESERVE_STRUCTURAL_DISAGREEMENT_OR_REQUEST_SIZE_RESOLVED_EVIDENCE",
        ),
        (
            "recruitment_growth_aliasing",
            "DO_NOT_TREAT_AGGREGATE_BIOMASS_FIT_AS_DEMOGRAPHIC_ADEQUACY",
        ),
        ("delayed_response", "FLAG_MODEL_SET_ADEQUACY_UNRESOLVED"),
        (
            "size_dependent_mortality",
            "REQUEST_SIZE_STRATIFIED_OBSERVATION_OR_ABSTAIN",
        ),
        (
            "seed_or_propagule_limitation",
            "PRESERVE_COMPETING_MECHANISM_AND_SEEK_DISCRIMINATING_EVIDENCE",
        ),
        (
            "spatial_canopy_topology",
            "PROMOTE_SPATIAL_REPRESENTATION_OR_MARK_PROCESS_UNSUPPORTED",
        ),
        (
            "disturbance_climate_interaction",
            "FLAG_UNTESTED_INTERACTION_AND_LIMIT_CLAIM",
        ),
        (
            "observation_process_shift",
            "CLASSIFY_OBSERVATION_SHIFT_NOT_ECOLOGICAL_CHANGE",
        ),
        (
            "cross_region_parameter_transfer",
            "OUT_OF_DOMAIN_OR_RECALIBRATION_REQUIRED",
        ),
        (
            "common_structural_omission",
            "DO_NOT_CERTIFY_MODEL_SET_ADEQUACY;REQUEST_DISCRIMINATING_EVIDENCE_OR_ABSTAIN",
        ),
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

    let mut family_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut variant_ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let id = string_at(case, &["case_id"])?;
        let family = string_at(case, &["family"])?;
        let variant_id = string_at(case, &["variant_id"])?;
        let condition = string_at(case, &["condition"])?;
        let disposition = string_at(case, &["expected_disposition"])?;
        let unique = seen.insert(id.clone());
        let unique_variant = variant_ids.insert(variant_id.clone());
        *family_counts.entry(family.clone()).or_default() += 1;
        let expected_disposition = expected_families.get(family.as_str()).copied();
        let known_case = expected_disposition == Some(disposition.as_str());
        push_check(
            &mut checks,
            "MODEL-SET-CASE-SEMANTICS-001",
            unique && unique_variant && known_case && !condition.trim().is_empty(),
            format!(
                "case={id}; family={family}; expected={}; declared={disposition}; unique={unique}; unique_variant={unique_variant}",
                expected_disposition.unwrap_or("UNKNOWN_FAMILY")
            ),
        );
    }
    let three_per_family = expected_families
        .keys()
        .all(|family| family_counts.get(*family).copied().unwrap_or_default() == 3);
    push_check(
        &mut checks,
        "MODEL-SET-THIRTY-CASE-COVERAGE-001",
        seen.len() == 30
            && variant_ids.len() == 30
            && cases.len() == 30
            && family_counts.len() == expected_families.len()
            && three_per_family,
        format!(
            "{} unique cases, {} unique variants, {} families; three variants per family required",
            seen.len(),
            variant_ids.len(),
            family_counts.len()
        ),
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
        "public_case_count": seen.len(),
        "public_family_count": family_counts.len(),
        "blind_test_created": false,
        "hidden_truth_published": false,
        "empirical_model_set_adequacy": "NOT_ESTABLISHED"
    }))
}

fn validate_observation_admission_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let expected: BTreeMap<&'static str, &'static str> = [
        ("FOREST-OBS-ADMIT-VALID-001", "ADMIT_SYNTHETIC_ONLY"),
        (
            "FOREST-OBS-CLOUD-AS-ZERO-001",
            "REJECT_MISSINGNESS_PROMOTED_TO_ZERO",
        ),
        (
            "FOREST-OBS-RIGHTS-UNCLEARED-001",
            "BLOCK_RIGHTS_NOT_CLEARED",
        ),
        (
            "FOREST-OBS-ABSENCE-UNQUALIFIED-001",
            "REJECT_UNQUALIFIED_ABSENCE",
        ),
        (
            "FOREST-OBS-ABSENCE-QUALIFIED-001",
            "ADMIT_NEGATIVE_WITHIN_QUALIFIED_ENVELOPE",
        ),
        (
            "FOREST-OBS-PROVENANCE-INCOMPLETE-001",
            "REJECT_INCOMPLETE_PROVENANCE",
        ),
        ("FOREST-OBS-SPLIT-LEAKAGE-001", "REJECT_SPLIT_LEAKAGE"),
        (
            "FOREST-OBS-LONGITUDINAL-OVERCLAIM-001",
            "REJECT_UNSUPPORTED_LONGITUDINAL_INFERENCE",
        ),
        ("FOREST-OBS-OUT-OF-DOMAIN-001", "OUT_OF_DOMAIN"),
        ("FOREST-OBS-NO-ACQUISITION-001", "PRESERVE_UNOBSERVED"),
        (
            "FOREST-OBS-TARGET-SEMANTIC-MISMATCH-001",
            "REJECT_TARGET_VARIABLE_SEMANTIC_MISMATCH",
        ),
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
    push_check(
        &mut checks,
        "OBS-ADMISSION-CORPUS-IDENTITY-001",
        version == "0.1.0"
            && benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-OBSERVATION-ADMISSION-CONTRACT-001",
        format!("version={version}; benchmark={benchmark}; corpus={corpus_id}"),
    );
    push_check(
        &mut checks,
        "OBS-ADMISSION-CLAIM-CEILING-001",
        status == "PUBLIC_SYNTHETIC_ADMISSION_CONTRACT_NOT_PRODUCTION"
            && ceiling.contains("synthetic examples")
            && ceiling.contains("No real source data ingested"),
        "record-level rules must remain synthetic and must not claim a production adapter or real-data authority",
    );

    for (index, case) in cases.iter().enumerate() {
        let id = case
            .get("case_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].case_id must be a string"))?;
        let declared = case
            .get("expected_disposition")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].expected_disposition must be a string"))?;
        let observation = at(case, &["observation"])?;

        let observation_id = string_at(observation, &["observation_id"])?;
        let subject_id = string_at(observation, &["subject_id"])?;
        let provider = string_at(observation, &["source", "provider"])?;
        let product_id = string_at(observation, &["source", "product_id"])?;
        let product_version = string_at(observation, &["source", "product_version"])?;
        let asset_digest = string_at(observation, &["source", "asset_sha256"])?;
        let operator_id = string_at(observation, &["operator", "operator_id"])?;
        let operator_version = string_at(observation, &["operator", "operator_version"])?;
        let geometry_id = string_at(observation, &["support", "geometry_id"])?;
        let crs = string_at(observation, &["support", "crs"])?;
        let valid_from = string_at(observation, &["support", "valid_from"])?;
        let valid_to = string_at(observation, &["support", "valid_to"])?;
        let area = number_at(observation, &["support", "area_m2"])?;
        let variable = string_at(observation, &["measurement", "variable"])?;
        let unit = string_at(observation, &["measurement", "unit"])?;
        let validity = string_at(observation, &["measurement", "validity_state"])?;
        let target_variable = string_at(observation, &["claim", "target_variable"])?;
        let intended_purpose = string_at(observation, &["rights", "intended_purpose"])?;
        let rights_status = string_at(observation, &["rights", "status"])?;
        let group_key = string_at(observation, &["split", "group_key"])?;
        let partition = string_at(observation, &["split", "partition"])?;
        let source_domain = string_at(observation, &["support", "source_domain"])?;
        let target_domain = string_at(observation, &["support", "target_domain"])?;
        let domain_transfer_qualified = at(observation, &["support", "domain_transfer_qualified"])?
            .as_bool()
            .ok_or_else(|| "support.domain_transfer_qualified must be boolean".to_string())?;
        let detection_qualified = at(observation, &["measurement", "detection_qualified"])?
            .as_bool()
            .ok_or_else(|| "measurement.detection_qualified must be boolean".to_string())?;
        let coverage_qualified = at(observation, &["measurement", "coverage_qualified"])?
            .as_bool()
            .ok_or_else(|| "measurement.coverage_qualified must be boolean".to_string())?;
        let longitudinal_rate = at(observation, &["claim", "longitudinal_rate"])?
            .as_bool()
            .ok_or_else(|| "claim.longitudinal_rate must be boolean".to_string())?;
        let matched_repeats = at(observation, &["support", "matched_repeat_measurements"])?
            .as_bool()
            .ok_or_else(|| "support.matched_repeat_measurements must be boolean".to_string())?;
        let repeat_count = usize_at(observation, &["support", "repeat_measurement_count"])?;
        let measurement_value = at(observation, &["measurement", "value"])?;
        let has_value = !measurement_value.is_null();
        let related_partition = observation
            .get("split")
            .and_then(|split| split.get("related_partition"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let purposes = at(observation, &["rights", "allowed_purposes"])?
            .as_array()
            .ok_or_else(|| "rights.allowed_purposes must be an array".to_string())?;
        let purpose_allowed = purposes
            .iter()
            .any(|purpose| purpose.as_str() == Some(intended_purpose.as_str()));

        let digest_valid =
            asset_digest.len() == 64 && asset_digest.bytes().all(|byte| byte.is_ascii_hexdigit());
        let provenance_complete = !observation_id.trim().is_empty()
            && !subject_id.trim().is_empty()
            && !provider.trim().is_empty()
            && !product_id.trim().is_empty()
            && !product_version.trim().is_empty()
            && digest_valid
            && !operator_id.trim().is_empty()
            && !operator_version.trim().is_empty()
            && !geometry_id.trim().is_empty()
            && !crs.trim().is_empty()
            && !valid_from.trim().is_empty()
            && !valid_to.trim().is_empty()
            && valid_from <= valid_to
            && area.is_finite()
            && area > 0.0
            && !variable.trim().is_empty()
            && !unit.trim().is_empty()
            && !group_key.trim().is_empty()
            && !partition.trim().is_empty();
        let rights_cleared = rights_status == "cleared_for_declared_use" && purpose_allowed;
        let missingness_state = matches!(
            validity.as_str(),
            "cloud_masked" | "no_acquisition" | "no_data" | "qa_invalid" | "below_detection"
        );
        let split_leakage = !related_partition.is_empty()
            && related_partition != partition
            && !group_key.trim().is_empty();
        let derived = if !provenance_complete {
            "REJECT_INCOMPLETE_PROVENANCE"
        } else if !rights_cleared {
            "BLOCK_RIGHTS_NOT_CLEARED"
        } else if (missingness_state || validity == "observed_absence") && has_value {
            "REJECT_MISSINGNESS_PROMOTED_TO_ZERO"
        } else if source_domain != target_domain && !domain_transfer_qualified {
            "OUT_OF_DOMAIN"
        } else if variable != target_variable {
            "REJECT_TARGET_VARIABLE_SEMANTIC_MISMATCH"
        } else if validity == "observed_absence" && !(detection_qualified && coverage_qualified) {
            "REJECT_UNQUALIFIED_ABSENCE"
        } else if validity == "observed_absence" {
            "ADMIT_NEGATIVE_WITHIN_QUALIFIED_ENVELOPE"
        } else if longitudinal_rate && (repeat_count < 2 || !matched_repeats) {
            "REJECT_UNSUPPORTED_LONGITUDINAL_INFERENCE"
        } else if split_leakage {
            "REJECT_SPLIT_LEAKAGE"
        } else if missingness_state {
            "PRESERVE_UNOBSERVED"
        } else if validity == "observed_valid" && measurement_value.as_f64().is_some() {
            "ADMIT_SYNTHETIC_ONLY"
        } else {
            "REJECT_UNSUPPORTED_VALIDITY_STATE"
        };

        let known_expected = expected
            .iter()
            .find_map(|(case_id, disposition)| (*case_id == id).then_some(*disposition));
        let unique = seen.insert(id.to_string());
        let disposition_matches = known_expected == Some(declared) && derived == declared;
        push_check(
            &mut checks,
            "OBS-ADMISSION-CASE-DISPOSITION-001",
            unique && disposition_matches,
            format!(
                "case={id}; derived={derived}; expected={}; declared={declared}; unique={unique}",
                known_expected.unwrap_or("UNKNOWN_CASE")
            ),
        );
    }

    push_check(
        &mut checks,
        "OBS-ADMISSION-CASE-COVERAGE-001",
        seen.len() == expected.len() && cases.len() == expected.len(),
        format!("{} unique cases; {} required", seen.len(), expected.len()),
    );

    let execution = at(manifest, &["execution"])?;
    let runner = string_at(execution, &["runner"])?;
    let adapter = string_at(execution, &["production_adapter"])?;
    let external_data = at(execution, &["external_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "external_data_ingested must be boolean".to_string())?;
    let rights_execution = string_at(execution, &["rights_authority_execution"])?;
    let blind_cases = at(execution, &["blind_cases_created"])?
        .as_bool()
        .ok_or_else(|| "blind_cases_created must be boolean".to_string())?;
    push_check(
        &mut checks,
        "OBS-ADMISSION-EXECUTION-BOUNDARY-001",
        runner == "SYNTHETIC_OBSERVATION_ADMISSION_RULES"
            && adapter == "NOT_IMPLEMENTED"
            && !external_data
            && rights_execution == "NOT_RUN"
            && !blind_cases,
        "synthetic record rules only; production adapters, real data, rights authority and blind custody remain unexecuted",
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
        "public_case_count": seen.len(),
        "production_adapter_execution": "NOT_RUN",
        "external_data_ingested": false,
        "rights_authority_execution": "NOT_RUN",
        "empirical_observation_admission": "NOT_ESTABLISHED"
    }))
}

fn rect_from_json(value: &Value) -> Result<support_kernel::Rect, support_kernel::SupportError> {
    let coordinates = value
        .as_array()
        .filter(|coordinates| coordinates.len() == 4)
        .ok_or(support_kernel::SupportError::InvalidGeometry)?;
    let mut parsed = [0.0; 4];
    for (index, coordinate) in coordinates.iter().enumerate() {
        parsed[index] = coordinate
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or(support_kernel::SupportError::InvalidGeometry)?;
    }
    support_kernel::Rect::new(parsed[0], parsed[1], parsed[2], parsed[3])
}

fn validate_support_intersection_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let expected_ids: BTreeSet<&'static str> = [
        "SUPPORT-IDENTITY-001",
        "SUPPORT-TARGET-CONTAINED-001",
        "SUPPORT-PARTIAL-EDGE-001",
        "SUPPORT-NONOVERLAP-001",
        "SUPPORT-TILED-COMPLETE-001",
        "SUPPORT-FRAGMENTED-001",
        "SUPPORT-CLOUD-FULL-GEOMETRY-001",
        "SUPPORT-MIXED-VALIDITY-001",
        "SUPPORT-REGISTRATION-SHIFT-001",
        "SUPPORT-CRS-MISMATCH-001",
        "SUPPORT-OVERLAP-DOUBLE-COUNT-001",
        "SUPPORT-DEGENERATE-GEOMETRY-001",
        "SUPPORT-EDGE-TOUCH-001",
        "SUPPORT-EMPTY-EVIDENCE-001",
        "SUPPORT-REGISTRATION-BOUNDS-ZERO-001",
        "SUPPORT-REGISTRATION-BOUNDS-SYMMETRIC-001",
        "SUPPORT-REGISTRATION-BOUNDS-EDGE-001",
        "SUPPORT-REGISTRATION-BOUNDS-WIDE-001",
    ]
    .into_iter()
    .collect();

    let version = string_at(manifest, &["schema_version"])?;
    let benchmark = string_at(manifest, &["benchmark_id"])?;
    let corpus_id = string_at(manifest, &["corpus_id"])?;
    let status = string_at(manifest, &["status"])?;
    let coordinate_model = string_at(manifest, &["coordinate_model"])?;
    let claim_ceiling = string_at(manifest, &["claim_ceiling"])?;
    let cases = at(manifest, &["cases"])?
        .as_array()
        .ok_or_else(|| "cases must be an array".to_string())?;

    let mut seen = BTreeSet::new();
    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "SUPPORT-CORPUS-IDENTITY-001",
        version == "0.1.0"
            && benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-SUPPORT-INTERSECTION-001"
            && status == "PUBLIC_SYNTHETIC_RECTANGLE_CASES_NOT_GIS"
            && coordinate_model == "LOCAL_PLANAR_METRES_ALREADY_TRANSFORMED"
            && claim_ceiling.contains("axis-aligned rectangle intersections")
            && claim_ceiling.contains("No arbitrary geometry"),
        format!(
            "version={version}; benchmark={benchmark}; corpus={corpus_id}; coordinate_model={coordinate_model}"
        ),
    );

    let mut result_rows = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let case_id = case
            .get("case_id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("cases[{index}].case_id must be a string"))?;
        let unique = seen.insert(case_id.to_string());
        let target = at(case, &["target_support"])?;
        let target_crs = string_at(target, &["crs"])?;
        let target_rect = rect_from_json(at(target, &["bounds"])?)
            .map_err(|error| format!("cases[{index}].target_support.bounds: {error}"))?;
        let sample_values = at(case, &["samples"])?
            .as_array()
            .ok_or_else(|| format!("cases[{index}].samples must be an array"))?;
        let mut samples = Vec::with_capacity(sample_values.len());
        let mut sample_geometry_error = None;
        for (sample_index, sample) in sample_values.iter().enumerate() {
            let sample_id = string_at(sample, &["sample_id"])?;
            let crs = string_at(sample, &["crs"])?;
            let bounds = match rect_from_json(at(sample, &["bounds"])?) {
                Ok(bounds) => bounds,
                Err(error) => {
                    sample_geometry_error = Some(error);
                    break;
                }
            };
            let valid = at(sample, &["valid"])?.as_bool().ok_or_else(|| {
                format!("cases[{index}].samples[{sample_index}].valid must be boolean")
            })?;
            samples.push(support_kernel::SupportSample {
                sample_id,
                crs,
                bounds,
                valid,
            });
        }

        let actual = match sample_geometry_error {
            Some(error) => Err(error),
            None => support_kernel::qualify_rectangular_support(&target_crs, target_rect, &samples),
        };
        let expected_error = case.get("expected_error").and_then(Value::as_str);
        let (case_passed, detail, row) = if let Some(expected_error) = expected_error {
            match actual {
                Err(error) => {
                    let actual_error = error.to_string();
                    let passed =
                        unique && actual_error == expected_error && expected_ids.contains(case_id);
                    (
                        passed,
                        format!(
                            "case={case_id}; error={actual_error}; expected_error={expected_error}; unique={unique}"
                        ),
                        json!({"case_id":case_id,"error":actual_error}),
                    )
                }
                Ok(report) => (
                    false,
                    format!(
                        "case={case_id}; expected error={expected_error}; got {}",
                        report.disposition.as_str()
                    ),
                    json!({"case_id":case_id,"unexpected_disposition":report.disposition.as_str()}),
                ),
            }
        } else {
            let expected = at(case, &["expected"])?;
            let report = actual.map_err(|error| {
                format!("cases[{index}] unexpectedly returned support error: {error}")
            })?;
            let expected_overlap = number_at(expected, &["geometric_overlap_area"])?;
            let expected_geometry_fraction = number_at(expected, &["geometric_coverage_fraction"])?;
            let expected_valid_area = number_at(expected, &["valid_data_area"])?;
            let expected_valid_fraction = number_at(expected, &["valid_data_coverage_fraction"])?;
            let expected_disposition = string_at(expected, &["disposition"])?;
            let derived_disposition = report.disposition.as_str();
            let passed = unique
                && expected_ids.contains(case_id)
                && close_enough(report.geometric_overlap_area, expected_overlap, 1e-9)
                && close_enough(
                    report.geometric_coverage_fraction,
                    expected_geometry_fraction,
                    1e-12,
                )
                && close_enough(report.valid_data_area, expected_valid_area, 1e-9)
                && close_enough(
                    report.valid_data_coverage_fraction,
                    expected_valid_fraction,
                    1e-12,
                )
                && derived_disposition == expected_disposition;
            (
                passed,
                format!(
                    "case={case_id}; geometric_coverage={}; valid_coverage={}; disposition={derived_disposition}; unique={unique}",
                    report.geometric_coverage_fraction, report.valid_data_coverage_fraction
                ),
                json!({
                    "case_id":case_id,
                    "target_area":report.target_area,
                    "geometric_overlap_area":report.geometric_overlap_area,
                    "geometric_coverage_fraction":report.geometric_coverage_fraction,
                    "valid_data_area":report.valid_data_area,
                    "valid_data_coverage_fraction":report.valid_data_coverage_fraction,
                    "disposition":derived_disposition
                }),
            )
        };
        push_check(
            &mut checks,
            "SUPPORT-CASE-ORACLE-AGREEMENT-001",
            case_passed,
            detail,
        );

        if let Some(shift_value) = case.get("registration_shift_bounds") {
            let expected_bounds = at(case, &["expected_registration_bounds"])?;
            let shift = support_kernel::RegistrationShiftBounds {
                min_dx: number_at(shift_value, &["min_dx"])?,
                max_dx: number_at(shift_value, &["max_dx"])?,
                min_dy: number_at(shift_value, &["min_dy"])?,
                max_dy: number_at(shift_value, &["max_dy"])?,
            };
            let reference_sample = samples.first().ok_or_else(|| {
                format!("cases[{index}] registration bounds require one reference source rectangle")
            })?;
            let registration_result = support_kernel::bound_rect_overlap_under_translation(
                target_rect,
                reference_sample.bounds,
                shift,
            );
            match registration_result {
                Ok(registration) => {
                    let minimum_area = number_at(expected_bounds, &["minimum_overlap_area_m2"])?;
                    let maximum_area = number_at(expected_bounds, &["maximum_overlap_area_m2"])?;
                    let minimum_fraction =
                        number_at(expected_bounds, &["minimum_coverage_fraction"])?;
                    let maximum_fraction =
                        number_at(expected_bounds, &["maximum_coverage_fraction"])?;
                    let registration_passed = unique
                        && close_enough(registration.minimum_overlap_area_m2, minimum_area, 1e-9)
                        && close_enough(registration.maximum_overlap_area_m2, maximum_area, 1e-9)
                        && close_enough(
                            registration.minimum_coverage_fraction,
                            minimum_fraction,
                            1e-12,
                        )
                        && close_enough(
                            registration.maximum_coverage_fraction,
                            maximum_fraction,
                            1e-12,
                        );
                    push_check(
                        &mut checks,
                        "SUPPORT-REGISTRATION-BOUNDS-ORACLE-001",
                        registration_passed,
                        format!(
                            "case={case_id}; min_area={}; max_area={}; min_fraction={}; max_fraction={}; expected_min_area={minimum_area}; expected_max_area={maximum_area}",
                            registration.minimum_overlap_area_m2,
                            registration.maximum_overlap_area_m2,
                            registration.minimum_coverage_fraction,
                            registration.maximum_coverage_fraction
                        ),
                    );
                    result_rows.push(json!({
                        "case_id": case_id,
                        "registration_minimum_overlap_area_m2": registration.minimum_overlap_area_m2,
                        "registration_maximum_overlap_area_m2": registration.maximum_overlap_area_m2,
                        "registration_minimum_coverage_fraction": registration.minimum_coverage_fraction,
                        "registration_maximum_coverage_fraction": registration.maximum_coverage_fraction
                    }));
                }
                Err(error) => {
                    push_check(
                        &mut checks,
                        "SUPPORT-REGISTRATION-BOUNDS-ORACLE-001",
                        false,
                        format!("case={case_id}; registration_bound_error={error}"),
                    );
                }
            }
        }
        result_rows.push(row);
    }

    push_check(
        &mut checks,
        "SUPPORT-CASE-COVERAGE-001",
        seen.len() == expected_ids.len()
            && cases.len() == expected_ids.len()
            && seen
                .iter()
                .all(|case_id| expected_ids.contains(case_id.as_str())),
        format!(
            "{} unique cases; {} required",
            seen.len(),
            expected_ids.len()
        ),
    );

    let execution = at(manifest, &["execution"])?;
    let runner = string_at(execution, &["runner"])?;
    let external = at(execution, &["external_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "execution.external_data_ingested must be boolean".to_string())?;
    let transform = string_at(execution, &["crs_transform"])?;
    let arbitrary = string_at(execution, &["arbitrary_polygon_support"])?;
    let uncertainty = string_at(execution, &["positional_uncertainty"])?;
    push_check(
        &mut checks,
        "SUPPORT-EXECUTION-BOUNDARY-001",
        runner == "ANALYTIC_RECTANGULAR_SUPPORT_KERNEL"
            && !external
            && transform == "NOT_IMPLEMENTED"
            && arbitrary == "NOT_IMPLEMENTED"
            && uncertainty == "PROBABILISTIC_REGISTRATION_MODEL_NOT_IMPLEMENTED",
        "pre-transformed axis-aligned synthetic rectangles and deterministic translation bounds only; no external data, arbitrary geometry, CRS transform or probabilistic registration model is claimed",
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
        "result": if passed { "PASS_SYNTHETIC_SUPPORT" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "case_count": seen.len(),
        "support_kernel": "AXIS_ALIGNED_RECTANGLES_SAME_DECLARED_CRS",
        "arbitrary_polygon_support": "NOT_IMPLEMENTED",
        "crs_transform": "NOT_IMPLEMENTED",
        "positional_uncertainty": "DETERMINISTIC_TRANSLATION_BOUNDS_ONLY",
        "probabilistic_registration_model": "NOT_IMPLEMENTED",
        "registration_bounds_case_count": cases.iter().filter(|case| case.get("registration_shift_bounds").is_some()).count(),
        "external_data_ingested": false
    }))
}

fn parse_support_sample(
    value: &Value,
    label: &str,
) -> Result<support_kernel::SupportSample, String> {
    let sample_id = string_at(value, &["sample_id"])?;
    let crs = string_at(value, &["crs"])?;
    let bounds = rect_from_json(at(value, &["bounds"])?)
        .map_err(|error| format!("{label}.bounds: {error}"))?;
    let valid = at(value, &["valid"])?
        .as_bool()
        .ok_or_else(|| format!("{label}.valid must be boolean"))?;
    Ok(support_kernel::SupportSample {
        sample_id,
        crs,
        bounds,
        valid,
    })
}

fn validate_variable_aggregation_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let expected_ids: BTreeSet<&'static str> = [
        "AGG-INTENSIVE-WEIGHTED-MEAN-001",
        "AGG-INTENSIVE-PARTIAL-NO-EXTRAPOLATION-001",
        "AGG-INTENSIVE-ALL-MASKED-001",
        "AGG-INTENSIVE-UNIT-MISMATCH-001",
        "AGG-STOCK-DENSITY-INTEGRAL-001",
        "AGG-STOCK-PARTIAL-NO-EXTRAPOLATION-001",
        "AGG-COUNT-HORVITZ-THOMPSON-001",
        "AGG-COUNT-ZERO-INCLUSION-PROBABILITY-001",
        "AGG-COUNT-NONPROBABILITY-DESIGN-001",
        "AGG-CATEGORICAL-FRACTIONS-001",
        "AGG-CATEGORICAL-MASKED-PRESERVES-GAP-001",
        "AGG-CATEGORICAL-EMPTY-CLASS-001",
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
    push_check(
        &mut checks,
        "AGG-CORPUS-IDENTITY-001",
        version == "0.1.0"
            && benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-VARIABLE-AWARE-AGGREGATION-001"
            && status == "PUBLIC_SYNTHETIC_VARIABLE_AGGREGATION_CASES"
            && ceiling.contains("Synthetic known-truth cases only")
            && ceiling.contains("No production raster"),
        format!("version={version}; benchmark={benchmark}; corpus={corpus_id}"),
    );

    let mut result_rows = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let case_id = string_at(case, &["case_id"])?;
        let kind = string_at(case, &["kind"])?;
        let unique = seen.insert(case_id.clone());
        let expected_error = case.get("expected_error").and_then(Value::as_str);
        let sample_values = at(case, &["samples"])?
            .as_array()
            .ok_or_else(|| format!("cases[{index}].samples must be an array"))?;

        let evaluation: Result<Value, String> = match kind.as_str() {
            "intensive_mean" => {
                let target_crs = string_at(case, &["target_crs"])?;
                let target = rect_from_json(at(case, &["target_bounds"])?)
                    .map_err(|error| format!("cases[{index}].target_bounds: {error}"))?;
                let mut samples = Vec::with_capacity(sample_values.len());
                for (sample_index, sample) in sample_values.iter().enumerate() {
                    samples.push(aggregation_kernel::IntensiveSample {
                        support: parse_support_sample(
                            at(sample, &["support"])?,
                            &format!("cases[{index}].samples[{sample_index}].support"),
                        )?,
                        value: number_at(sample, &["value"])?,
                        unit: string_at(sample, &["unit"])?,
                    });
                }
                let expected_unit = string_at(case, &["target_unit"])?;
                aggregation_kernel::aggregate_intensive_mean(
                    &target_crs,
                    target,
                    &samples,
                    &expected_unit,
                )
                .map(|result| {
                    json!({
                        "observed_mean": result.observed_mean,
                        "unit": result.unit,
                        "target_area_m2": result.target_area_m2,
                        "geometric_coverage_fraction": result.geometric_coverage_fraction,
                        "valid_data_coverage_fraction": result.valid_data_coverage_fraction,
                        "valid_observed_area_m2": result.valid_observed_area_m2,
                        "disposition": result.disposition.as_str()
                    })
                })
                .map_err(|error| format!("{error:?}"))
            }
            "stock_integral" => {
                let target_crs = string_at(case, &["target_crs"])?;
                let target = rect_from_json(at(case, &["target_bounds"])?)
                    .map_err(|error| format!("cases[{index}].target_bounds: {error}"))?;
                let mut samples = Vec::with_capacity(sample_values.len());
                for (sample_index, sample) in sample_values.iter().enumerate() {
                    samples.push(aggregation_kernel::DensitySample {
                        support: parse_support_sample(
                            at(sample, &["support"])?,
                            &format!("cases[{index}].samples[{sample_index}].support"),
                        )?,
                        value_mg_per_ha: number_at(sample, &["value_mg_per_ha"])?,
                    });
                }
                aggregation_kernel::integrate_density_to_observed_stock(
                    &target_crs,
                    target,
                    &samples,
                )
                .map(|result| {
                    json!({
                        "observed_stock_mg": result.observed_stock_mg,
                        "target_area_m2": result.target_area_m2,
                        "geometric_coverage_fraction": result.geometric_coverage_fraction,
                        "valid_data_coverage_fraction": result.valid_data_coverage_fraction,
                        "valid_observed_area_m2": result.valid_observed_area_m2,
                        "disposition": result.disposition.as_str()
                    })
                })
                .map_err(|error| format!("{error:?}"))
            }
            "count_total" => {
                let mut samples = Vec::with_capacity(sample_values.len());
                for sample in sample_values {
                    samples.push(aggregation_kernel::CountSample {
                        sample_unit_id: string_at(sample, &["sample_unit_id"])?,
                        sampling_frame_id: string_at(sample, &["sampling_frame_id"])?,
                        sampling_design_id: string_at(sample, &["sampling_design_id"])?,
                        design_kind: string_at(sample, &["design_kind"])?,
                        observed_count: at(sample, &["observed_count"])?.as_u64().ok_or_else(
                            || "observed_count must be a non-negative integer".to_string(),
                        )?,
                        inclusion_probability: number_at(sample, &["inclusion_probability"])?,
                    });
                }
                let frame = string_at(case, &["sampling_frame_id"])?;
                let design = string_at(case, &["sampling_design_id"])?;
                let frame_count = usize_at(case, &["population_unit_count"])?;
                aggregation_kernel::estimate_probability_sample_count_total(
                    &frame,
                    &design,
                    frame_count,
                    &samples,
                )
                .map(|result| {
                    json!({
                        "horvitz_thompson_total": result.horvitz_thompson_total,
                        "sampling_frame_id": result.sampling_frame_id,
                        "sampling_design_id": result.sampling_design_id,
                        "population_unit_count": result.population_unit_count,
                        "sampled_unit_count": result.sampled_unit_count,
                        "uncertainty_status": result.uncertainty_status
                    })
                })
                .map_err(|error| format!("{error:?}"))
            }
            "categorical_areas" => {
                let target_crs = string_at(case, &["target_crs"])?;
                let target = rect_from_json(at(case, &["target_bounds"])?)
                    .map_err(|error| format!("cases[{index}].target_bounds: {error}"))?;
                let mut samples = Vec::with_capacity(sample_values.len());
                for (sample_index, sample) in sample_values.iter().enumerate() {
                    samples.push(aggregation_kernel::CategoricalSample {
                        support: parse_support_sample(
                            at(sample, &["support"])?,
                            &format!("cases[{index}].samples[{sample_index}].support"),
                        )?,
                        class_id: string_at(sample, &["class_id"])?,
                    });
                }
                aggregation_kernel::aggregate_categorical_areas(
                    &target_crs,
                    target,
                    &samples,
                )
                .map(|result| json!({
                    "target_area_m2": result.target_area_m2,
                    "geometric_coverage_fraction": result.geometric_coverage_fraction,
                    "valid_data_coverage_fraction": result.valid_data_coverage_fraction,
                    "valid_observed_area_m2": result.valid_observed_area_m2,
                    "disposition": result.disposition.as_str(),
                    "classes": result.classes.iter().map(|class| json!({
                        "class_id": class.class_id,
                        "valid_area_m2": class.valid_area_m2,
                        "fraction_of_valid_observed_area": class.fraction_of_valid_observed_area
                    })).collect::<Vec<Value>>()
                }))
                .map_err(|error| format!("{error:?}"))
            }
            _ => Err(format!("unsupported aggregation kind: {kind}")),
        };

        let (passed, detail, row) = if let Some(expected_error) = expected_error {
            match evaluation {
                Err(actual_error) => {
                    let actual_name = actual_error
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_string();
                    let passed = unique
                        && expected_ids.contains(case_id.as_str())
                        && actual_name == expected_error;
                    (
                        passed,
                        format!(
                            "case={case_id}; actual_error={actual_name}; expected_error={expected_error}; unique={unique}"
                        ),
                        json!({"case_id":case_id,"error":actual_name}),
                    )
                }
                Ok(actual) => (
                    false,
                    format!("case={case_id}; expected_error={expected_error}; got={actual}"),
                    json!({"case_id":case_id,"unexpected_result":actual}),
                ),
            }
        } else {
            let actual = evaluation?;
            let expected = at(case, &["expected"])?;
            let mut passed = true;
            let numeric_fields = match kind.as_str() {
                "intensive_mean" => vec![
                    ("geometric_coverage_fraction", "geometric_coverage_fraction"),
                    (
                        "valid_data_coverage_fraction",
                        "valid_data_coverage_fraction",
                    ),
                    ("valid_observed_area_m2", "valid_observed_area_m2"),
                ],
                "stock_integral" => vec![
                    ("observed_stock_mg", "observed_stock_mg"),
                    ("geometric_coverage_fraction", "geometric_coverage_fraction"),
                    (
                        "valid_data_coverage_fraction",
                        "valid_data_coverage_fraction",
                    ),
                    ("valid_observed_area_m2", "valid_observed_area_m2"),
                ],
                "count_total" => vec![("horvitz_thompson_total", "horvitz_thompson_total")],
                "categorical_areas" => vec![
                    ("geometric_coverage_fraction", "geometric_coverage_fraction"),
                    (
                        "valid_data_coverage_fraction",
                        "valid_data_coverage_fraction",
                    ),
                    ("valid_observed_area_m2", "valid_observed_area_m2"),
                ],
                _ => Vec::new(),
            };
            for (field, actual_field) in numeric_fields {
                let wanted = number_at(expected, &[field])?;
                let got = number_at(&actual, &[actual_field])?;
                passed &= close_enough(got, wanted, 1e-9);
            }
            match kind.as_str() {
                "intensive_mean" => {
                    let wanted = expected.get("observed_mean").unwrap_or(&Value::Null);
                    let got = actual.get("observed_mean").unwrap_or(&Value::Null);
                    passed &= wanted == got;
                    passed &= string_at(expected, &["unit"])? == string_at(&actual, &["unit"])?;
                }
                "count_total" => {
                    passed &= string_at(expected, &["uncertainty_status"])?
                        == string_at(&actual, &["uncertainty_status"])?;
                    passed &= usize_at(expected, &["sampled_unit_count"])?
                        == usize_at(&actual, &["sampled_unit_count"])?;
                }
                "categorical_areas" => {
                    let expected_classes = at(expected, &["classes"])?
                        .as_array()
                        .ok_or_else(|| "expected.classes must be an array".to_string())?;
                    let actual_classes = at(&actual, &["classes"])?
                        .as_array()
                        .ok_or_else(|| "actual.classes must be an array".to_string())?;
                    if expected_classes.len() != actual_classes.len() {
                        passed = false;
                    } else {
                        for expected_class in expected_classes {
                            let id = string_at(expected_class, &["class_id"])?;
                            let actual_class = actual_classes.iter().find(|candidate| {
                                candidate.get("class_id").and_then(Value::as_str)
                                    == Some(id.as_str())
                            });
                            if let Some(actual_class) = actual_class {
                                passed &= close_enough(
                                    number_at(actual_class, &["valid_area_m2"])?,
                                    number_at(expected_class, &["valid_area_m2"])?,
                                    1e-9,
                                );
                                passed &= close_enough(
                                    number_at(actual_class, &["fraction_of_valid_observed_area"])?,
                                    number_at(
                                        expected_class,
                                        &["fraction_of_valid_observed_area"],
                                    )?,
                                    1e-12,
                                );
                            } else {
                                passed = false;
                            }
                        }
                    }
                }
                _ => {}
            }
            (
                unique && expected_ids.contains(case_id.as_str()) && passed,
                format!(
                    "case={case_id}; kind={kind}; expected_fields_match={passed}; unique={unique}"
                ),
                json!({"case_id":case_id,"actual":actual}),
            )
        };

        push_check(&mut checks, "AGG-CASE-ORACLE-AGREEMENT-001", passed, detail);
        result_rows.push(row);
    }

    push_check(
        &mut checks,
        "AGG-CASE-COVERAGE-001",
        seen.len() == expected_ids.len()
            && cases.len() == expected_ids.len()
            && seen
                .iter()
                .all(|case_id| expected_ids.contains(case_id.as_str())),
        format!(
            "{} unique cases; {} required",
            seen.len(),
            expected_ids.len()
        ),
    );

    let execution = at(manifest, &["execution"])?;
    let runner = string_at(execution, &["runner"])?;
    let external = at(execution, &["external_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "execution.external_data_ingested must be boolean".to_string())?;
    let variance = string_at(execution, &["survey_design_variance"])?;
    let affine = string_at(execution, &["raster_affine_transform"])?;
    let polygons = string_at(execution, &["arbitrary_polygon_support"])?;
    push_check(
        &mut checks,
        "AGG-EXECUTION-BOUNDARY-001",
        runner == "VARIABLE_AWARE_AGGREGATION_KERNEL"
            && !external
            && variance == "NOT_IMPLEMENTED"
            && affine == "NOT_IMPLEMENTED"
            && polygons == "NOT_IMPLEMENTED",
        "bounded synthetic aggregation only; no survey-design variance, raster affine transform or arbitrary polygon is claimed",
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
        "result": if passed { "PASS_SYNTHETIC_AGGREGATION" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "case_count": seen.len(),
        "uncertainty_status": "NO_UNSUPPORTED_VARIANCE_ESTIMATORS",
        "external_data_ingested": false
    }))
}

fn string_set_at(root: &Value, path: &[&str]) -> Result<BTreeSet<String>, String> {
    let values = at(root, path)?
        .as_array()
        .ok_or_else(|| format!("expected array at {}", path.join(".")))?;
    let mut result = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let item = value
            .as_str()
            .ok_or_else(|| format!("expected string at {}[{index}]", path.join(".")))?;
        if item.trim().is_empty() {
            return Err(format!("empty string at {}[{index}]", path.join(".")));
        }
        if !result.insert(item.to_string()) {
            return Err(format!("duplicate value at {}[{index}]", path.join(".")));
        }
    }
    Ok(result)
}

fn adjudicate_evidence_dependence(
    left: &Value,
    right: &Value,
    required_dimensions: &BTreeSet<String>,
) -> Result<&'static str, String> {
    let left_complete = at(left, &["lineage_complete"])?
        .as_bool()
        .ok_or_else(|| "left.lineage_complete must be boolean".to_string())?;
    let right_complete = at(right, &["lineage_complete"])?
        .as_bool()
        .ok_or_else(|| "right.lineage_complete must be boolean".to_string())?;
    let left_assessed = string_set_at(left, &["dependency_dimensions_assessed"])?;
    let right_assessed = string_set_at(right, &["dependency_dimensions_assessed"])?;

    for field in ["product_id", "site_id", "campaign_id", "sensor_id"] {
        let left_value = left.get(field).and_then(Value::as_str).unwrap_or_default();
        let right_value = right.get(field).and_then(Value::as_str).unwrap_or_default();
        if left_value.trim().is_empty() || right_value.trim().is_empty() {
            return Ok("DEPENDENCE_UNKNOWN");
        }
    }

    if !left_complete
        || !right_complete
        || !required_dimensions.is_subset(&left_assessed)
        || !required_dimensions.is_subset(&right_assessed)
    {
        return Ok("DEPENDENCE_UNKNOWN");
    }

    let left_upstream = string_set_at(left, &["upstream_asset_ids"])?;
    let right_upstream = string_set_at(right, &["upstream_asset_ids"])?;
    if !left_upstream.is_disjoint(&right_upstream) {
        return Ok("SHARED_UPSTREAM_EVIDENCE");
    }

    let left_campaign = string_at(left, &["campaign_id"])?;
    let right_campaign = string_at(right, &["campaign_id"])?;
    if left_campaign == right_campaign {
        return Ok("SHARED_CAMPAIGN");
    }

    let shared_dependencies = [
        ("calibration_dataset_ids", "calibration"),
        ("training_dataset_ids", "training"),
        ("common_cause_ids", "common cause"),
        ("processing_pipeline_ids", "processing pipeline"),
    ];
    for (field, _) in shared_dependencies {
        let left_values = string_set_at(left, &[field])?;
        let right_values = string_set_at(right, &[field])?;
        if !left_values.is_disjoint(&right_values) {
            return Ok("DEPENDENCE_MODEL_REQUIRED");
        }
    }

    let left_site = string_at(left, &["site_id"])?;
    let right_site = string_at(right, &["site_id"])?;
    let left_sensor = string_at(left, &["sensor_id"])?;
    let right_sensor = string_at(right, &["sensor_id"])?;
    if left_site == right_site || left_sensor == right_sensor {
        return Ok("DEPENDENCE_MODEL_REQUIRED");
    }

    let left_pipelines = string_set_at(left, &["processing_pipeline_ids"])?;
    let right_pipelines = string_set_at(right, &["processing_pipeline_ids"])?;
    if !left_pipelines.is_disjoint(&right_pipelines) {
        return Ok("DEPENDENCE_MODEL_REQUIRED");
    }

    Ok("DECLARED_INDEPENDENT_GIVEN_CONDITIONS")
}

fn validate_consilience_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let expected_ids: BTreeSet<&'static str> = [
        "CONSILIENCE-SHARED-UPSTREAM-001",
        "CONSILIENCE-SHARED-CAMPAIGN-001",
        "CONSILIENCE-SHARED-CALIBRATION-001",
        "CONSILIENCE-SHARED-TRAINING-001",
        "CONSILIENCE-SHARED-COMMON-CAUSE-001",
        "CONSILIENCE-SHARED-SITE-001",
        "CONSILIENCE-SHARED-SENSOR-001",
        "CONSILIENCE-SHARED-PIPELINE-001",
        "CONSILIENCE-INDEPENDENT-CONDITIONS-001",
        "CONSILIENCE-LINEAGE-UNKNOWN-001",
        "CONSILIENCE-DIMENSION-UNASSESSED-001",
        "CONSILIENCE-DIFFERENT-PRODUCT-SAME-RAW-001",
    ]
    .into_iter()
    .collect();

    let version = string_at(manifest, &["schema_version"])?;
    let benchmark = string_at(manifest, &["benchmark_id"])?;
    let corpus_id = string_at(manifest, &["corpus_id"])?;
    let status = string_at(manifest, &["status"])?;
    let ceiling = string_at(manifest, &["claim_ceiling"])?;
    let required_dimensions = string_set_at(manifest, &["required_dependency_dimensions"])?;
    let expected_dimensions: BTreeSet<String> = [
        "upstream_assets",
        "campaign",
        "site",
        "sensor",
        "calibration_data",
        "training_data",
        "common_causes",
        "processing_pipeline",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect();
    let cases = at(manifest, &["cases"])?
        .as_array()
        .ok_or_else(|| "cases must be an array".to_string())?;

    let mut seen = BTreeSet::new();
    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "CONSILIENCE-CORPUS-IDENTITY-001",
        version == "0.1.0"
            && benchmark == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
            && corpus_id == "FOREST-CONSILIENCE-DEPENDENCE-001"
            && status == "PUBLIC_SYNTHETIC_DEPENDENCE_CASES_NOT_PRODUCTION"
            && ceiling.contains("Twelve synthetic lineage pairs")
            && ceiling.contains("No real source data"),
        format!("version={version}; benchmark={benchmark}; corpus={corpus_id}"),
    );
    push_check(
        &mut checks,
        "CONSILIENCE-DEPENDENCY-DIMENSIONS-001",
        required_dimensions == expected_dimensions,
        format!(
            "{} required dependency dimensions declared",
            required_dimensions.len()
        ),
    );

    for (index, case) in cases.iter().enumerate() {
        let case_id = string_at(case, &["case_id"])?;
        let declared = string_at(case, &["expected_disposition"])?;
        let left = at(case, &["left"])?;
        let right = at(case, &["right"])?;
        let derived = adjudicate_evidence_dependence(left, right, &required_dimensions)?;
        let known = expected_ids.contains(case_id.as_str());
        let unique = seen.insert(case_id.clone());
        let matches = known && derived == declared;
        push_check(
            &mut checks,
            "CONSILIENCE-DEPENDENCE-CASE-001",
            unique && matches,
            format!("case={case_id}; derived={derived}; declared={declared}; unique={unique}"),
        );
        if !unique {
            push_check(
                &mut checks,
                "CONSILIENCE-DEPENDENCE-UNIQUE-001",
                false,
                format!("duplicate case identity at cases[{index}]"),
            );
        }
    }

    push_check(
        &mut checks,
        "CONSILIENCE-DEPENDENCE-COVERAGE-001",
        seen.len() == expected_ids.len()
            && cases.len() == expected_ids.len()
            && seen
                .iter()
                .all(|case_id| expected_ids.contains(case_id.as_str())),
        format!(
            "{} unique cases; {} required",
            seen.len(),
            expected_ids.len()
        ),
    );

    let execution = at(manifest, &["execution"])?;
    let runner = string_at(execution, &["runner"])?;
    let external = at(execution, &["external_data_ingested"])?
        .as_bool()
        .ok_or_else(|| "execution.external_data_ingested must be boolean".to_string())?;
    let registry = string_at(execution, &["lineage_registry_authentication"])?;
    let model = string_at(execution, &["probabilistic_dependence_model"])?;
    let fusion = string_at(execution, &["evidence_fusion"])?;
    push_check(
        &mut checks,
        "CONSILIENCE-EXECUTION-BOUNDARY-001",
        runner == "SYNTHETIC_EVIDENCE_DEPENDENCE_RULES"
            && !external
            && registry == "NOT_IMPLEMENTED"
            && model == "NOT_IMPLEMENTED"
            && fusion == "NOT_IMPLEMENTED",
        "deterministic synthetic lineage screening only; no authenticated registry, probability model or fusion is implemented",
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
        "result": if passed { "PASS_SYNTHETIC_CONSILIENCE" } else { "FAIL" },
        "checks_passed": checks.iter().filter(|check| check.passed).count(),
        "checks_total": checks.len(),
        "checks": check_values,
        "case_count": seen.len(),
        "lineage_registry_authentication": "NOT_IMPLEMENTED",
        "probabilistic_dependence_model": "NOT_IMPLEMENTED",
        "evidence_fusion": "NOT_IMPLEMENTED",
        "external_data_ingested": false
    }))
}

/// Dispatch a JSON fixture to its domain-specific verifier and return a deterministic receipt.
///
/// This command verifies manifests and analytical/synthetic invariants only. It does not
/// imply ecological calibration, observation-model validation, or field performance.
pub fn evaluate(raw: &[u8]) -> Result<Value, String> {
    let manifest: Value =
        serde_json::from_slice(raw).map_err(|error| format!("invalid JSON manifest: {error}"))?;
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-OBSERVATION-OPERATOR-NEGATIVE-CONTROLS-001")
    {
        return validate_observation_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-OBSERVATION-ADMISSION-CONTRACT-001")
    {
        return validate_observation_admission_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str) == Some("FOREST-SUPPORT-INTERSECTION-001")
    {
        return validate_support_intersection_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-VARIABLE-AWARE-AGGREGATION-001")
    {
        return validate_variable_aggregation_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-CONSILIENCE-DEPENDENCE-001")
    {
        return validate_consilience_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-BLOCK-COVERAGE-INFERENCE-001")
    {
        return forest_corpus_receipts::validate_block_coverage_inference(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-SPATIAL-LOCALIZATION-001")
    {
        return forest_corpus_receipts::validate_spatial_localization(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-PROCESS-TRAJECTORY-001")
    {
        return trajectory_budget_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-FLUX-IDENTIFIABILITY-001")
    {
        return flux_identifiability_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-FLUX-PRACTICAL-IDENTIFIABILITY-001")
    {
        return practical_identifiability_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("crosswalk_id").and_then(Value::as_str)
        == Some("FORESTPATHS-D-BEN-ADMISSION-001")
    {
        return forestpaths_crosswalk_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("AFEX-FACTORIAL-DESIGN-001")
    {
        return afex_design_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("AFEX-ROOT-TURNOVER-001")
    {
        return afex_root_turnover_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("AFEX-RATIO-ESTIMAND-001")
    {
        return afex_ratio_estimand_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("AFEX-ROOT-PATHWAY-MODEL-TOURNAMENT-001")
    {
        return afex_root_pathway_tournament_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("AFEX-ROOT-PATHWAY-IDENTIFIABILITY-001")
    {
        return afex_root_pathway_identifiability_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("AFEX-PRACTICAL-DESIGN-001")
    {
        return afex_practical_design_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("AFEX-SOURCE-IDENTITY-001")
    {
        return afex_source_identity_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("AFEX-ROOT-DISTRIBUTION-UQ-001")
    {
        return afex_root_distribution_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str) == Some("FOREST-BLOCK-CONFORMAL-001") {
        return validate_block_conformal_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str) == Some("FOREST-BINARY-EVENT-BRIER-001") {
        return validate_binary_event_brier_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-PROBABILISTIC-FORECAST-SCORING-001")
    {
        return validate_probabilistic_forecast_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-FORECAST-USEFULNESS-LIMIT-001")
    {
        return forecast_usefulness_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-AS-OF-FORECAST-INTEGRITY-001")
    {
        return forest_as_of_integrity_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-OBSERVATION-CADENCE-001")
    {
        return forest_observation_cadence_kernel::validate_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str)
        == Some("FOREST-MODEL-SET-KNOWN-TRUTH-DIAGNOSTICS-001")
    {
        return validate_executable_model_set_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str) == Some("FOREST-MODEL-SET-ADEQUACY-001") {
        return validate_model_set_corpus(raw, &manifest);
    }
    if manifest.get("corpus_id").and_then(Value::as_str) == Some("FOREST-DECISION-VALUE-001") {
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
        return Err(format!(
            "intervention target is not a declared cell: {target}"
        ));
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

    let tolerance = number_at(&manifest, &["invariants", "0", "tolerance_kg_dry_matter"])?;
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
            .ok_or_else(|| {
                format!("expected total biomass missing or non-numeric for {cell_id}")
            })?;
        all_stocks_nonnegative &=
            before_live >= 0.0 && before_dead >= 0.0 && after_live >= 0.0 && after_dead >= 0.0;
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
                format!(
                    "cell {cell_id} changed canonical biomass stocks despite no declared causal path"
                ),
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

    const FIXTURE: &str = include_str!("../../cases/forest-patch-accounting-synthetic-001.json");

    const OBSERVATION_FIXTURE: &str =
        include_str!("../../cases/forest-observation-operator-negative-controls-v0.1.json");
    const CANOPY_FIXTURE: &str =
        include_str!("../../cases/canopy-gap-representation-synthetic-v0.1.json");
    const MODEL_SET_FIXTURE: &str =
        include_str!("../../cases/forest-model-set-adequacy-development-v0.1.json");
    const EXECUTABLE_MODEL_SET_FIXTURE: &str =
        include_str!("../../cases/forest-model-set-known-truth-diagnostics-synthetic-v0.1.json");
    const FORECAST_SCORING_FIXTURE: &str =
        include_str!("../../cases/forest-probabilistic-forecast-scoring-synthetic-v0.1.json");
    const BINARY_EVENT_BRIER_FIXTURE: &str =
        include_str!("../../cases/forest-binary-event-brier-synthetic-v0.1.json");
    const BLOCK_CONFORMAL_FIXTURE: &str =
        include_str!("../../cases/forest-block-conformal-synthetic-v0.1.json");
    const DECISION_VALUE_FIXTURE: &str =
        include_str!("../../cases/forest-decision-value-development-v0.1.json");
    const OBSERVATION_ADMISSION_FIXTURE: &str =
        include_str!("../../cases/forest-observation-admission-contract-synthetic-v0.1.json");
    const SUPPORT_INTERSECTION_FIXTURE: &str =
        include_str!("../../cases/forest-support-intersection-synthetic-v0.1.json");
    const VARIABLE_AGGREGATION_FIXTURE: &str =
        include_str!("../../cases/forest-variable-aware-aggregation-synthetic-v0.1.json");
    const CONSILIENCE_FIXTURE: &str =
        include_str!("../../cases/forest-consilience-dependence-synthetic-v0.1.json");
    const PROCESS_TRAJECTORY_FIXTURE: &str =
        include_str!("../../cases/forest-process-trajectory-synthetic-v0.1.json");
    const FLUX_IDENTIFIABILITY_FIXTURE: &str =
        include_str!("../../cases/forest-flux-identifiability-synthetic-v0.1.json");
    const PRACTICAL_IDENTIFIABILITY_FIXTURE: &str =
        include_str!("../../cases/forest-flux-practical-identifiability-synthetic-v0.1.json");
    const FORESTPATHS_CROSSWALK_FIXTURE: &str =
        include_str!("../../cases/forestpaths-dben-source-crosswalk-v0.1.json");

    #[test]
    fn consilience_corpus_matches_twelve_synthetic_dependency_pairs() {
        let receipt = evaluate(CONSILIENCE_FIXTURE.as_bytes()).expect("consilience corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_CONSILIENCE");
        assert_eq!(receipt["case_count"], 12);
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(
            receipt["lineage_registry_authentication"],
            "NOT_IMPLEMENTED"
        );
        assert_eq!(receipt["probabilistic_dependence_model"], "NOT_IMPLEMENTED");
        assert_eq!(receipt["evidence_fusion"], "NOT_IMPLEMENTED");
    }

    #[test]
    fn consilience_corpus_rejects_false_independence_after_upstream_mutation() {
        let mut manifest: Value =
            serde_json::from_str(CONSILIENCE_FIXTURE).expect("valid consilience corpus");
        manifest["cases"][8]["left"]["upstream_asset_ids"] = json!(["shared-raw-scene"]);
        manifest["cases"][8]["right"]["upstream_asset_ids"] = json!(["shared-raw-scene"]);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn consilience_corpus_rejects_unassessed_dependency_dimension_as_independent() {
        let mut manifest: Value =
            serde_json::from_str(CONSILIENCE_FIXTURE).expect("valid consilience corpus");
        manifest["cases"][8]["left"]["dependency_dimensions_assessed"] = json!(["upstream_assets"]);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn variable_aggregation_corpus_matches_twelve_synthetic_cases() {
        let receipt = evaluate(VARIABLE_AGGREGATION_FIXTURE.as_bytes())
            .expect("variable aggregation corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_AGGREGATION");
        assert_eq!(receipt["case_count"], 12);
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(
            receipt["uncertainty_status"],
            "NO_UNSUPPORTED_VARIANCE_ESTIMATORS"
        );
        assert_eq!(receipt["external_data_ingested"], false);
    }

    #[test]
    fn variable_aggregation_corpus_rejects_changed_intensive_mean() {
        let mut manifest: Value = serde_json::from_str(VARIABLE_AGGREGATION_FIXTURE)
            .expect("valid variable aggregation corpus");
        manifest["cases"][0]["expected"]["observed_mean"] = json!(999.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn variable_aggregation_corpus_rejects_category_code_as_class_identity_change() {
        let mut manifest: Value = serde_json::from_str(VARIABLE_AGGREGATION_FIXTURE)
            .expect("valid variable aggregation corpus");
        manifest["cases"][9]["samples"][0]["class_id"] = json!("forest-999");
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn support_intersection_corpus_matches_eighteen_geometry_and_registration_cases() {
        let receipt = evaluate(SUPPORT_INTERSECTION_FIXTURE.as_bytes())
            .expect("support intersection corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_SUPPORT");
        assert_eq!(receipt["case_count"], 18);
        assert_eq!(receipt["registration_bounds_case_count"], 4);
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(receipt["arbitrary_polygon_support"], "NOT_IMPLEMENTED");
        assert_eq!(receipt["crs_transform"], "NOT_IMPLEMENTED");
        assert_eq!(
            receipt["positional_uncertainty"],
            "DETERMINISTIC_TRANSLATION_BOUNDS_ONLY"
        );
        assert_eq!(
            receipt["probabilistic_registration_model"],
            "NOT_IMPLEMENTED"
        );
    }

    #[test]
    fn support_intersection_corpus_rejects_mutated_coverage_oracle() {
        let mut manifest: Value = serde_json::from_str(SUPPORT_INTERSECTION_FIXTURE)
            .expect("valid support intersection corpus");
        manifest["cases"][2]["expected"]["geometric_coverage_fraction"] = json!(0.75);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated support corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn support_intersection_corpus_rejects_mutated_registration_bounds_oracle() {
        let mut manifest: Value = serde_json::from_str(SUPPORT_INTERSECTION_FIXTURE)
            .expect("valid support intersection corpus");
        manifest["cases"][15]["expected_registration_bounds"]["minimum_coverage_fraction"] =
            json!(0.95);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated support corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn support_intersection_corpus_rejects_crs_mismatch_mutation() {
        let mut manifest: Value = serde_json::from_str(SUPPORT_INTERSECTION_FIXTURE)
            .expect("valid support intersection corpus");
        manifest["cases"][9]["samples"][0]["crs"] = json!("LOCAL_METRE_GRID_A");
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated support corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn observation_admission_contract_executes_eleven_synthetic_cases() {
        let receipt = evaluate(OBSERVATION_ADMISSION_FIXTURE.as_bytes())
            .expect("observation admission contract parses");
        assert_eq!(receipt["result"], "MANIFEST_VALID");
        assert_eq!(receipt["public_case_count"], 11);
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(receipt["production_adapter_execution"], "NOT_RUN");
        assert_eq!(receipt["external_data_ingested"], false);
        assert_eq!(receipt["rights_authority_execution"], "NOT_RUN");
    }

    #[test]
    fn observation_admission_contract_rejects_rights_mutation() {
        let mut manifest: Value = serde_json::from_str(OBSERVATION_ADMISSION_FIXTURE)
            .expect("valid observation admission contract");
        manifest["cases"][0]["observation"]["rights"]["status"] = json!("unknown");
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated contract");
        let receipt = evaluate(&bytes).expect("well-formed contract returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn observation_admission_contract_rejects_unqualified_absence_promotion() {
        let mut manifest: Value = serde_json::from_str(OBSERVATION_ADMISSION_FIXTURE)
            .expect("valid observation admission contract");
        manifest["cases"][3]["observation"]["measurement"]["detection_qualified"] = json!(true);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated contract");
        let receipt = evaluate(&bytes).expect("well-formed contract returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

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
        assert!(
            check["detail"]
                .as_str()
                .unwrap_or_default()
                .contains("derived=UNOBSERVED_NO_ACQUISITION")
        );
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
        assert_eq!(
            receipt["ecological_light_model_calibration"],
            "NOT_ESTABLISHED"
        );
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
        let mut manifest: Value =
            serde_json::from_str(CANOPY_FIXTURE).expect("valid canopy fixture");
        let duplicate = manifest["arrangements"][0]["occupied_coordinates"][0].clone();
        manifest["arrangements"][0]["occupied_coordinates"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated canopy fixture");
        let receipt = evaluate(&bytes).expect("well-formed fixture returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn block_conformal_corpus_matches_pooled_and_block_level_coverage() {
        let receipt =
            evaluate(BLOCK_CONFORMAL_FIXTURE.as_bytes()).expect("block conformal corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_BLOCK_CONFORMAL");
        assert_eq!(receipt["calibration_block_count"], 9);
        assert_eq!(receipt["pooled_observation_radius"], 1.0);
        assert_eq!(receipt["block_maximum_radius"], 9.0);
        assert_eq!(receipt["block_full_block_coverage"], 1.0 / 3.0);
        assert_eq!(
            receipt["exchangeability_status"],
            "DESIGNER_DECLARED_NOT_STATISTICALLY_VERIFIED"
        );
    }

    #[test]
    fn block_conformal_corpus_matches_pooled_and_block_winkler_scores() {
        let receipt =
            evaluate(BLOCK_CONFORMAL_FIXTURE.as_bytes()).expect("block conformal corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_BLOCK_CONFORMAL");
        assert!(
            (receipt["pooled_mean_interval_score"].as_f64().unwrap() - (55.0 / 3.0)).abs() < 1e-9
        );
        assert!((receipt["block_mean_interval_score"].as_f64().unwrap() - 19.0).abs() < 1e-9);
        assert!(
            (receipt["test_block_results"][0]["block_mean_interval_score"]
                .as_f64()
                .unwrap()
                - 18.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn block_conformal_corpus_rejects_interval_score_oracle_mutation() {
        let mut manifest: Value =
            serde_json::from_str(BLOCK_CONFORMAL_FIXTURE).expect("valid block conformal corpus");
        manifest["expected"]["block_mean_interval_score"] = json!(1.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated Winkler corpus");
        let receipt = evaluate(&bytes).expect("well-formed conformal corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn block_conformal_corpus_rejects_radius_oracle_mutation() {
        let mut manifest: Value =
            serde_json::from_str(BLOCK_CONFORMAL_FIXTURE).expect("valid block conformal corpus");
        manifest["expected"]["block_maximum_radius"] = json!(1.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated conformal corpus");
        let receipt = evaluate(&bytes).expect("well-formed conformal corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn block_conformal_corpus_rejects_calibration_test_block_identity_reuse() {
        let mut manifest: Value =
            serde_json::from_str(BLOCK_CONFORMAL_FIXTURE).expect("valid block conformal corpus");
        manifest["scenario"]["test_blocks"][0]["block_id"] =
            manifest["scenario"]["calibration_blocks"][0]["block_id"].clone();
        let bytes = serde_json::to_vec(&manifest).expect("serialize duplicate block identity");
        assert!(evaluate(&bytes).is_err());
    }

    #[test]
    fn block_conformal_corpus_rejects_calibration_test_group_leakage() {
        let mut manifest: Value =
            serde_json::from_str(BLOCK_CONFORMAL_FIXTURE).expect("valid block conformal corpus");
        manifest["scenario"]["test_blocks"][0]["group_id"] =
            manifest["scenario"]["calibration_blocks"][0]["group_id"].clone();
        let bytes = serde_json::to_vec(&manifest).expect("serialize leaked conformal corpus");
        assert!(evaluate(&bytes).is_err());
    }

    #[test]
    fn binary_event_brier_corpus_matches_case_scores_reliability_and_abstention() {
        let receipt = evaluate(BINARY_EVENT_BRIER_FIXTURE.as_bytes())
            .expect("binary event Brier corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_BRIER");
        assert_eq!(receipt["summary"]["eligible_case_count"], 6);
        assert_eq!(receipt["summary"]["forecast_case_count"], 5);
        assert_eq!(receipt["summary"]["abstained_case_count"], 1);
        assert_eq!(
            receipt["summary"]["reliability_bins"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert_eq!(receipt["event_calibration"], "NOT_ESTABLISHED");
    }

    #[test]
    fn binary_event_brier_corpus_rejects_skill_oracle_mutation() {
        let mut manifest: Value =
            serde_json::from_str(BINARY_EVENT_BRIER_FIXTURE).expect("valid Brier corpus");
        manifest["expected_summary"]["brier_skill_score"] = json!(0.99);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated Brier corpus");
        let receipt = evaluate(&bytes).expect("well-formed Brier corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn binary_event_brier_corpus_rejects_reliability_bin_mutation() {
        let mut manifest: Value =
            serde_json::from_str(BINARY_EVENT_BRIER_FIXTURE).expect("valid Brier corpus");
        manifest["expected_summary"]["reliability_bins"][4]["observed_event_frequency"] =
            json!(1.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated Brier corpus");
        let receipt = evaluate(&bytes).expect("well-formed Brier corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn forecast_scoring_corpus_matches_crps_skill_intervals_and_abstention() {
        let receipt =
            evaluate(FORECAST_SCORING_FIXTURE.as_bytes()).expect("forecast scoring corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_FORECAST_SCORING");
        assert_eq!(receipt["summary"]["eligible_case_count"], 6);
        assert_eq!(receipt["summary"]["forecast_case_count"], 5);
        assert_eq!(receipt["summary"]["abstained_case_count"], 1);
        assert_eq!(
            receipt["summary"]["disposition"],
            "SCORED_WITH_EXPLICIT_FORECAST_COVERAGE"
        );
        assert_eq!(receipt["probabilistic_calibration"], "NOT_ESTABLISHED");
        assert_eq!(receipt["ecological_forecast_skill"], "NOT_ESTABLISHED");
    }

    #[test]
    fn forecast_scoring_corpus_matches_paired_wis_and_interval_sharpness() {
        let receipt =
            evaluate(FORECAST_SCORING_FIXTURE.as_bytes()).expect("forecast scoring corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_FORECAST_SCORING");
        assert!(
            (receipt["summary"]["mean_model_wis"].as_f64().unwrap() - 2.3142857142857145).abs()
                < 1e-9
        );
        assert!(
            (receipt["summary"]["mean_baseline_wis_paired"]
                .as_f64()
                .unwrap()
                - 2.0)
                .abs()
                < 1e-9
        );
        assert!(
            (receipt["summary"]["wis_skill_score"].as_f64().unwrap() - (-0.15714285714285725))
                .abs()
                < 1e-9
        );
        assert_eq!(receipt["forecast_scores"][4]["model_wis"], Value::Null);
    }

    #[test]
    fn forecast_scoring_corpus_rejects_per_case_wis_oracle_mutation() {
        let mut manifest: Value =
            serde_json::from_str(FORECAST_SCORING_FIXTURE).expect("valid forecast scoring corpus");
        manifest["cases"][0]["expected"]["model_wis"] = json!(0.99);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated WIS corpus");
        let receipt = evaluate(&bytes).expect("well-formed forecast corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn forecast_scoring_corpus_checks_full_risk_coverage_curve() {
        let receipt =
            evaluate(FORECAST_SCORING_FIXTURE.as_bytes()).expect("forecast scoring corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_FORECAST_SCORING");
        assert_eq!(receipt["risk_coverage_curve"].as_array().unwrap().len(), 6);
        assert!(
            (receipt["risk_coverage_curve"][5]["mean_crps_risk"]
                .as_f64()
                .unwrap()
                - (14.5 / 6.0))
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn forecast_scoring_corpus_rejects_mutated_risk_coverage_curve() {
        let mut manifest: Value =
            serde_json::from_str(FORECAST_SCORING_FIXTURE).expect("valid forecast scoring corpus");
        manifest["expected_risk_coverage_curve"][0]["mean_crps_risk"] = json!(9.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated risk-coverage corpus");
        let receipt = evaluate(&bytes).expect("well-formed forecast corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn forecast_scoring_corpus_rejects_case_level_crps_oracle_mutation() {
        let mut manifest: Value =
            serde_json::from_str(FORECAST_SCORING_FIXTURE).expect("valid forecast scoring corpus");
        manifest["cases"][0]["expected"]["model_crps"] = json!(0.99);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated forecast case");
        let receipt = evaluate(&bytes).expect("well-formed forecast corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn forecast_scoring_corpus_rejects_skill_oracle_mutation() {
        let mut manifest: Value =
            serde_json::from_str(FORECAST_SCORING_FIXTURE).expect("valid forecast scoring corpus");
        manifest["expected_summary"]["crps_skill_score"] = json!(0.99);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated forecast corpus");
        let receipt = evaluate(&bytes).expect("well-formed forecast corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn forecast_scoring_corpus_rejects_abstention_coverage_mutation() {
        let mut manifest: Value =
            serde_json::from_str(FORECAST_SCORING_FIXTURE).expect("valid forecast scoring corpus");
        manifest["expected_summary"]["forecast_coverage_fraction"] = json!(1.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated forecast corpus");
        let receipt = evaluate(&bytes).expect("well-formed forecast corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn executable_model_set_corpus_runs_five_known_truth_counterexamples() {
        let receipt = evaluate(EXECUTABLE_MODEL_SET_FIXTURE.as_bytes())
            .expect("known-truth model-set corpus parses");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_MODEL_SET_DIAGNOSTICS");
        assert_eq!(receipt["case_count"], 5);
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(receipt["blind_test_created"], false);
        assert_eq!(receipt["empirical_model_set_adequacy"], "NOT_ESTABLISHED");
    }

    #[test]
    fn executable_model_set_corpus_rejects_mutated_delayed_response_truth() {
        let mut manifest: Value = serde_json::from_str(EXECUTABLE_MODEL_SET_FIXTURE)
            .expect("valid executable model-set corpus");
        manifest["cases"][2]["observed_validation_state"]["small_live_biomass"] = json!(80.0);
        manifest["cases"][2]["observed_validation_state"]["large_live_biomass"] = json!(20.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated known-truth corpus");
        let receipt = evaluate(&bytes).expect("well-formed known-truth corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn model_set_public_manifest_has_thirty_cases_and_no_blind_claim() {
        let receipt = evaluate(MODEL_SET_FIXTURE.as_bytes()).expect("model-set manifest parses");
        assert_eq!(receipt["result"], "MANIFEST_VALID");
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(receipt["blind_test_created"], false);
        assert_eq!(receipt["hidden_truth_published"], false);
        assert_eq!(receipt["public_case_count"], 30);
        assert_eq!(receipt["public_family_count"], 10);
    }

    #[test]
    fn model_set_duplicate_variant_identity_is_rejected() {
        let mut manifest: Value =
            serde_json::from_str(MODEL_SET_FIXTURE).expect("valid model-set manifest");
        manifest["cases"][1]["variant_id"] = manifest["cases"][0]["variant_id"].clone();
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated model-set manifest");
        let receipt = evaluate(&bytes).expect("well-formed manifest returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn model_set_unknown_family_mutation_is_rejected() {
        let mut manifest: Value =
            serde_json::from_str(MODEL_SET_FIXTURE).expect("valid model-set manifest");
        manifest["cases"][0]["family"] = json!("unregistered_mechanism");
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated model-set manifest");
        let receipt = evaluate(&bytes).expect("well-formed manifest returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
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
        let receipt =
            evaluate(DECISION_VALUE_FIXTURE.as_bytes()).expect("decision-value corpus parses");
        assert_eq!(receipt["result"], "MANIFEST_VALID");
        assert_eq!(receipt["public_case_count"], 20);
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert!((receipt["expected_gross_evsi_example"].as_f64().unwrap() - 8.8).abs() < 1e-9);
        assert!(
            (receipt["expected_net_value_after_cost_example"]
                .as_f64()
                .unwrap()
                - 5.8)
                .abs()
                < 1e-9
        );
        assert_eq!(receipt["decision_policy_performance"], "NOT_ESTABLISHED");
        assert_eq!(receipt["existing_evsi_kernel_execution"], "NOT_RUN");
    }

    #[test]
    fn decision_value_evsi_arithmetic_mutation_is_rejected() {
        let mut manifest: Value =
            serde_json::from_str(DECISION_VALUE_FIXTURE).expect("valid decision-value corpus");
        manifest["worked_examples"]["moderate_uncertainty_positive_decision_value"]["net_expected_value_after_cost"] =
            json!(9.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated decision-value corpus");
        let receipt = evaluate(&bytes).expect("well-formed corpus returns a receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn high_information_without_action_change_has_zero_decision_value() {
        let receipt =
            evaluate(DECISION_VALUE_FIXTURE.as_bytes()).expect("decision-value corpus parses");
        let check = receipt["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["id"] == "DECISION-VALUE-HIGH-UNCERTAINTY-ZERO-EVSI-001")
            .expect("zero-EVSI check is present");
        assert_eq!(check["passed"], true);
    }

    #[test]
    fn process_trajectory_corpus_dispatches_through_top_level_evaluator() {
        let receipt = evaluate(PROCESS_TRAJECTORY_FIXTURE.as_bytes())
            .expect("process-trajectory corpus dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_PROCESS_TRAJECTORY");
        assert_eq!(receipt["corpus_id"], "FOREST-PROCESS-TRAJECTORY-001");
        assert_eq!(receipt["case_results"].as_array().unwrap().len(), 4);
        assert_eq!(
            receipt["source_manifest_sha256"],
            sha256_hex(PROCESS_TRAJECTORY_FIXTURE.as_bytes())
        );
    }

    #[test]
    fn flux_identifiability_corpus_dispatches_with_exact_rank_receipt() {
        let receipt = evaluate(FLUX_IDENTIFIABILITY_FIXTURE.as_bytes())
            .expect("flux-identifiability corpus dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_FLUX_IDENTIFIABILITY");
        assert_eq!(receipt["baseline"]["rank"], 2);
        assert_eq!(receipt["baseline"]["nullity"], 4);
        assert_eq!(receipt["measurement_design"]["minimum_synthetic_effort_cost"], 14);
        assert_eq!(
            receipt["source_manifest_sha256"],
            sha256_hex(FLUX_IDENTIFIABILITY_FIXTURE.as_bytes())
        );
    }

    #[test]
    fn forestpaths_crosswalk_dispatches_through_top_level_evaluator() {
        let receipt = evaluate(FORESTPATHS_CROSSWALK_FIXTURE.as_bytes())
            .expect("ForestPaths crosswalk dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_FORESTPATHS_CROSSWALK");
        assert_eq!(receipt["model_count"], 9);
        assert_eq!(receipt["quantity_count"], 24);
        assert_eq!(
            receipt["source_manifest_sha256"],
            sha256_hex(FORESTPATHS_CROSSWALK_FIXTURE.as_bytes())
        );
    }

    #[test]
    fn practical_identifiability_corpus_dispatches_with_covariance_receipt() {
        let receipt = evaluate(PRACTICAL_IDENTIFIABILITY_FIXTURE.as_bytes())
            .expect("practical-identifiability corpus dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_PRACTICAL_IDENTIFIABILITY");
        assert_eq!(receipt["baseline_structural_rank"], 2);
        assert_eq!(receipt["full_rank_design_count"], 18);
        assert_eq!(receipt["d_optimal_designs"][0].as_array().unwrap().len(), 6);
        assert_eq!(
            receipt["source_manifest_sha256"],
            sha256_hex(PRACTICAL_IDENTIFIABILITY_FIXTURE.as_bytes())
        );
    }

    #[test]
    fn committed_fixture_passes_all_synthetic_contract_checks() {
        let receipt = evaluate(FIXTURE.as_bytes()).expect("fixture should parse");
        assert_eq!(receipt["result"], "PASS");
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(receipt["regional_or_field_validation"], "NOT_ESTABLISHED");
        assert_eq!(
            receipt["ecological_parameter_calibration"],
            "NOT_ESTABLISHED"
        );
    }

    #[test]
    fn disconnected_control_mutation_is_rejected() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("valid fixture");
        manifest["expected_state"]["disconnected-control-cell"]["live_biomass_kg_dry_matter"] =
            json!(51.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated fixture");
        let receipt = evaluate(&bytes).expect("mutated fixture should remain structurally valid");
        assert_eq!(receipt["result"], "FAIL");
        assert!(
            receipt["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["id"] == "DISCONNECTED-CONTROL-001" && check["passed"] == false)
        );
    }

    #[test]
    fn inconsistent_declared_mortality_parameter_is_rejected() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("valid fixture");
        manifest["parameters"]["mortality_flux_per_step"] = json!(6.0);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutated fixture");
        let receipt = evaluate(&bytes).expect("mutated fixture should remain structurally valid");
        assert_eq!(receipt["result"], "FAIL");
        assert!(
            receipt["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["id"] == "PARAMETER-EFFECT-CONSISTENCY-001"
                    && check["passed"] == false)
        );
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
    
    #[test]
    fn afex_factorial_design_dispatches_through_top_level_evaluator() {
        let bytes = include_str!("../../cases/afex-factorial-design-synthetic-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("AFEX synthetic design corpus dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_AFEX_DESIGN");
        assert_eq!(receipt["full_design_rank"], 11);
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }

    
    #[test]
    fn afex_root_turnover_dispatches_with_stock_flow_and_summary_receipt() {
        let bytes = include_str!("../../cases/afex-root-turnover-synthetic-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("AFEX root-turnover corpus dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_ROOT_TURNOVER");
        assert_eq!(receipt["case_count"], 4);
        assert_eq!(receipt["published_summary_effect_count"], 7);
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }

    
    #[test]
    fn afex_root_distribution_dispatches_and_checks_three_logratio_oracles() {
        let bytes = include_str!("../../cases/afex-root-ratio-distribution-analytic-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("AFEX analytic ratio distribution dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_AFEX_ROOT_DISTRIBUTION_ORACLE");
        assert_eq!(receipt["scenario_count"], 3);
        assert_eq!(receipt["checks_passed"], 36);
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }

    
    #[test]
    fn afex_ratio_estimand_dispatches_and_binds_exact_fixture() {
        let bytes = include_str!("../../cases/afex-ratio-estimand-synthetic-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("AFEX ratio estimand fixture dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_RATIO_ESTIMANDS");
        assert_eq!(receipt["valid_summary_count"], 2);
        assert_eq!(receipt["support_refusal_count"], 1);
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }

    
    #[test]
    fn afex_root_pathway_tournament_dispatches_with_exact_manifest_receipt() {
        let bytes = include_str!("../../cases/afex-root-pathway-model-tournament-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("AFEX root-pathway tournament dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_ROOT_PATHWAY_TOURNAMENT_SPEC");
        assert_eq!(receipt["model_count"], 8);
        assert_eq!(receipt["no_mediation_present"], true);
        assert_eq!(receipt["omitted_mechanism_present"], true);
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }

    
    #[test]
    fn afex_root_pathway_identifiability_dispatches_with_exact_rank_receipt() {
        let bytes = include_str!("../../cases/afex-root-pathway-identifiability-synthetic-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("AFEX pathway identifiability fixture dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_PATHWAY_IDENTIFIABILITY");
        assert_eq!(receipt["ranks"], json!([0, 2, 3, 4]));
        assert_eq!(receipt["nullities"], json!([4, 2, 1, 0]));
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }

    
    #[test]
    fn afex_practical_design_dispatches_with_budget_and_pareto_receipt() {
        let bytes = include_str!("../../cases/afex-practical-design-synthetic-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("AFEX practical design fixture dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_PRACTICAL_DESIGN");
        assert_eq!(receipt["best_budgeted_portfolio"]["data_only_rank"], 4);
        assert_eq!(receipt["best_budgeted_portfolio"]["total_cost_units"], 24);
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }

    
    #[test]
    fn afex_source_identity_dispatches_with_unresolved_and_conflict_counts() {
        let bytes = include_str!("../../cases/afex-source-identity-synthetic-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("AFEX source identity fixture dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_SOURCE_IDENTITY_POLICY");
        assert_eq!(receipt["case_count"], 7);
        assert_eq!(receipt["unresolved_identity_count"], 1);
        assert_eq!(receipt["site_or_support_conflict_count"], 4);
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }


    #[test]
    fn forest_forecast_usefulness_dispatches_prefix_and_recovery_receipt() {
        let bytes = include_str!("../../cases/forest-forecast-usefulness-synthetic-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("forest forecast usefulness fixture dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_FORECAST_USEFULNESS_LIMIT");
        let results = receipt["context_results"].as_array().expect("context results");
        assert_eq!(results[0]["useful_horizon_days"], 90);
        assert_eq!(results[0]["first_failure_horizon_days"], 180);
        assert_eq!(results[1]["useful_horizon_days"], 30);
        assert_eq!(results[1]["first_failure_horizon_days"], 90);
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }


    #[test]
    fn forest_as_of_forecast_integrity_dispatches_bitemporal_receipt() {
        let bytes = include_str!("../../cases/forest-as-of-forecast-integrity-synthetic-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("forest as-of integrity fixture dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_AS_OF_FORECAST_INTEGRITY");
        assert_eq!(receipt["case_count"], 12);
        assert_eq!(receipt["valid_as_of_count"], 3);
        assert_eq!(receipt["not_estimable_count"], 1);
        assert_eq!(receipt["rights_refusal_count"], 1);
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }


    #[test]
    fn forest_observation_cadence_dispatches_profiles_and_budgeted_pareto_frontiers() {
        let bytes = include_str!("../../cases/forest-observation-cadence-synthetic-v0.1.json").as_bytes();
        let receipt = evaluate(bytes).expect("forest observation cadence fixture dispatches");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_FOREST_OBSERVATION_CADENCE");
        assert_eq!(receipt["profile_count"], 6);
        assert_eq!(receipt["context_count"], 2);
        assert_eq!(receipt["decision_scenario_count"], 6);
        assert_eq!(receipt["source_manifest_sha256"], sha256_hex(bytes));
    }

}
