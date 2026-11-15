// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Synthetic AFEX randomized-block factorial design and unit-of-analysis gate.
//!
//! This validates allocation completeness and the distinction between plot-level
//! treatment assignment and nested tree/time observations. It is not a causal estimator.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn usize_at(value: &Value, key: &str) -> Result<usize, String> {
    let number = value.get(key).and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))?;
    usize::try_from(number).map_err(|_| format!("{key} exceeds platform usize"))
}

fn bool_at(value: &Value, key: &str) -> Result<bool, String> {
    value.get(key).and_then(Value::as_bool)
        .ok_or_else(|| format!("{key} must be a boolean"))
}

fn number_at(value: &Value, key: &str) -> Result<f64, String> {
    let number = value.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key} must be a number"))?;
    if !number.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    Ok(number)
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn push_check(checks: &mut Vec<Value>, id: &str, passed: bool, detail: impl Into<String>) {
    checks.push(json!({"id":id,"passed":passed,"detail":detail.into()}));
}

/// Validate a synthetic complete 2x2x2 N/P/cation factorial crossed with four blocks.
pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let schema_version = string_at(manifest, "schema_version")?;
    let benchmark_id = string_at(manifest, "benchmark_id")?;
    let corpus_id = string_at(manifest, "corpus_id")?;
    let evidence_class = string_at(manifest, "evidence_class")?;
    let claim_ceiling = string_at(manifest, "claim_ceiling")?;
    if schema_version != "0.1.0"
        || benchmark_id != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || corpus_id != "AFEX-FACTORIAL-DESIGN-001"
        || evidence_class != "synthetic_factorial_design"
        || !claim_ceiling.contains("EMPIRICAL_TREATMENT_EFFECTS_NOT_ESTABLISHED")
    {
        return Err("AFEX fixture identity, evidence class, or claim ceiling is invalid".to_string());
    }

    let design = manifest.get("design").ok_or_else(|| "design is required".to_string())?;
    let factors = design.get("factors").and_then(Value::as_array)
        .ok_or_else(|| "design.factors must be an array".to_string())?;
    let factor_names: Vec<&str> = factors.iter().filter_map(Value::as_str).collect();
    if factor_names.len() != 3 || factor_names[0] != "N" || factor_names[1] != "P" || factor_names[2] != "C" {
        return Err("design.factors must be exactly [N, P, C]".to_string());
    }
    let expected_blocks = usize_at(design, "expected_block_count")?;
    let expected_treatments = usize_at(design, "expected_treatment_count")?;
    let expected_plots = usize_at(design, "expected_plot_count")?;
    let expected_rank = usize_at(design, "expected_full_design_rank")?;
    let expected_residual_df = usize_at(design, "expected_residual_degrees_of_freedom")?;
    if expected_blocks != 4 || expected_treatments != 8 || expected_plots != 32
        || expected_rank != 11 || expected_residual_df != 21
    {
        return Err("AFEX v0.1 expected design dimensions are outside the supported contract".to_string());
    }
    if string_at(design, "assignment_unit")? != "plot" {
        return Err("treatment assignment unit must be plot".to_string());
    }

    let declared_blocks = design.get("blocks").and_then(Value::as_array)
        .ok_or_else(|| "design.blocks must be an array".to_string())?;
    let mut blocks = BTreeSet::new();
    for block in declared_blocks {
        let id = block.as_str().ok_or_else(|| "block IDs must be strings".to_string())?;
        if id.trim().is_empty() || !blocks.insert(id.to_string()) {
            return Err("block IDs must be non-empty and unique".to_string());
        }
    }

    let declared_treatments = design.get("treatments").and_then(Value::as_array)
        .ok_or_else(|| "design.treatments must be an array".to_string())?;
    let mut treatments = BTreeSet::new();
    for treatment in declared_treatments {
        let id = treatment.as_str().ok_or_else(|| "treatment IDs must be strings".to_string())?;
        if id.trim().is_empty() || !treatments.insert(id.to_string()) {
            return Err("treatment IDs must be non-empty and unique".to_string());
        }
    }

    let plots = manifest.get("plots").and_then(Value::as_array)
        .ok_or_else(|| "plots must be an array".to_string())?;
    let mut plot_ids = BTreeSet::new();
    let mut cell_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut block_plot_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut factor_combinations = BTreeSet::new();
    let mut factor_values_valid = true;
    let mut treatment_identity_valid = true;

    for (index, plot) in plots.iter().enumerate() {
        let plot_id = string_at(plot, "plot_id")
            .map_err(|error| format!("plots[{index}]: {error}"))?;
        let block_id = string_at(plot, "block_id")?;
        let treatment_id = string_at(plot, "treatment_id")?;
        if plot_id.trim().is_empty() || !plot_ids.insert(plot_id.clone()) {
            return Err(format!("plots[{index}] has an empty or duplicate plot_id"));
        }
        if !blocks.contains(&block_id) {
            return Err(format!("plots[{index}] references an undeclared block {block_id}"));
        }
        let factors = plot.get("factors").ok_or_else(|| format!("plots[{index}].factors required"))?;
        let mut values = [0_u64; 3];
        for (slot, name) in ["N", "P", "C"].into_iter().enumerate() {
            let value = factors.get(name).and_then(Value::as_u64);
            match value {
                Some(v @ 0..=1) => values[slot] = v,
                _ => factor_values_valid = false,
            }
        }
        let derived_treatment = format!("N{}-P{}-C{}", values[0], values[1], values[2]);
        if treatment_id != derived_treatment || !treatments.contains(&treatment_id) {
            treatment_identity_valid = false;
        }
        factor_combinations.insert(derived_treatment.clone());
        *cell_counts.entry((block_id.clone(), derived_treatment)).or_insert(0) += 1;
        *block_plot_counts.entry(block_id).or_insert(0) += 1;
    }

    let all_cells_once = blocks.iter().all(|block| treatments.iter().all(|treatment| {
        cell_counts.get(&(block.clone(), treatment.clone())).copied() == Some(1)
    }));
    let each_block_has_eight = blocks.iter().all(|block| {
        block_plot_counts.get(block).copied() == Some(8)
    });
    let exact_factorial = factor_combinations.len() == 8
        && treatments.iter().all(|t| factor_combinations.contains(t));

    let mut checks = Vec::new();
    push_check(&mut checks, "AFEX-FACTORIAL-IDENTITY-001",
        schema_version == "0.1.0" && benchmark_id == "LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "fixture belongs to the declared tropical-forest qualification programme");
    push_check(&mut checks, "AFEX-FOUR-BLOCKS-001",
        blocks.len() == expected_blocks && declared_blocks.len() == expected_blocks,
        format!("observed {} unique blocks; expected {expected_blocks}", blocks.len()));
    push_check(&mut checks, "AFEX-EIGHT-TREATMENTS-001",
        treatments.len() == expected_treatments && declared_treatments.len() == expected_treatments,
        format!("observed {} unique treatment labels; expected {expected_treatments}", treatments.len()));
    push_check(&mut checks, "AFEX-UNIQUE-PLOT-IDENTITY-001",
        plot_ids.len() == expected_plots && plots.len() == expected_plots,
        format!("observed {} unique plot IDs over {} rows; expected {expected_plots}", plot_ids.len(), plots.len()));
    push_check(&mut checks, "AFEX-BINARY-FACTOR-LEVELS-001",
        factor_values_valid && exact_factorial,
        format!("observed {} distinct N/P/C combinations; expected 8 binary combinations", factor_combinations.len()));
    push_check(&mut checks, "AFEX-TREATMENT-LABEL-CONSISTENCY-001",
        treatment_identity_valid,
        "each treatment label must agree with its N/P/C factor values");
    push_check(&mut checks, "AFEX-COMPLETE-BLOCK-CROSSING-001",
        all_cells_once && each_block_has_eight,
        "each block must contain exactly one plot for every factorial treatment");
    let calculated_rank = if blocks.len() == 4 && treatments.len() == 8
        && all_cells_once && exact_factorial && factor_values_valid && treatment_identity_valid {
        Some(11_usize)
    } else { None };
    push_check(&mut checks, "AFEX-DESIGN-RANK-001",
        calculated_rank == Some(expected_rank),
        format!("balanced 2x2x2 cell means (rank 8) plus three block contrasts; rank={calculated_rank:?}"));
    push_check(&mut checks, "AFEX-RESIDUAL-DF-001",
        plots.len() >= expected_rank && plots.len().saturating_sub(expected_rank) == expected_residual_df,
        format!("plot units={}, model rank={expected_rank}, residual df={}", plots.len().saturating_sub(expected_rank)));

    let nested = manifest.get("nested_measurements")
        .ok_or_else(|| "nested_measurements is required".to_string())?;
    let nested_contract = string_at(nested, "treatment_assignment_unit")? == "plot"
        && string_at(nested, "observation_unit")? == "tree"
        && string_at(nested, "repeated_measurement_unit")? == "tree_by_measurement_date"
        && usize_at(nested, "independent_treatment_unit_count")? == plots.len()
        && !bool_at(nested, "tree_rows_are_independent_treatment_replicates")?
        && !bool_at(nested, "repeated_readings_are_independent_treatment_replicates")?;
    let metadata_tree_count = usize_at(nested, "source_metadata_tree_count")?;
    let source_count_not_recounted = string_at(nested, "source_metadata_tree_count_status")?
        == "DESCRIBED_BY_OFFICIAL_DATASET_RECORD_NOT_RECOUNTED_FROM_INGESTED_BYTES";
    push_check(&mut checks, "AFEX-ANTI-PSEUDOREPLICATION-001",
        nested_contract && source_count_not_recounted && metadata_tree_count == 1299,
        format!("tree metadata count={metadata_tree_count}; treatment assignment remains at plot level"));

    let source = manifest.get("source_metadata")
        .ok_or_else(|| "source_metadata is required".to_string())?;
    let source_identity = string_at(source, "doi")? == "10.5285/c2587e20-ba4a-4444-8ce9-ccdec15b0aa3"
        && !bool_at(source, "external_data_ingested")?
        && !bool_at(source, "archive_checksum_verified")?;
    push_check(&mut checks, "AFEX-NO-INGESTION-CLAIM-001", source_identity,
        "archive is a candidate source; fixture does not claim data bytes or checksum verification");

    let measurement = manifest.get("measurement_contract")
        .ok_or_else(|| "measurement_contract is required".to_string())?;
    let measurement_contract_valid =
        (number_at(measurement, "inventory_dbh_lower_threshold_cm")? - 10.0).abs() < 1e-12
        && string_at(measurement, "inventory_inclusion_operator")? == "strictly_greater_than"
        && (number_at(measurement, "inventory_measurement_height_m")? - 1.3).abs() < 1e-12
        && string_at(measurement, "measurement_height_exception")?
            == "measure_above_buttress_or_deformity_when_needed"
        && string_at(measurement, "recruitment_definition")?
            == "new_individual_reaches_10_cm_dbh"
        && string_at(measurement, "raw_dendrometer_quantity")?
            == "circumference_window_distance_mm"
        && string_at(measurement, "raw_dendrometer_instrument")? == "digital_caliper"
        && string_at(measurement, "dendrometer_time_support")?
            == "irregular_repeated_measurements"
        && string_at(measurement, "rainy_collection_policy")?
            == "NOT_COLLECTED_HUMIDITY_SENSITIVE_INSTRUMENT"
        && !bool_at(measurement, "rainy_missingness_numeric_zero")?;
    push_check(&mut checks, "AFEX-MEASUREMENT-OPERATOR-001",
        measurement_contract_valid,
        "preserve DBH threshold/height, source recruit definition, raw dendrometer quantity, irregular cadence, and rain-related missingness");

    let passed = checks.iter().all(|check| check["passed"].as_bool() == Some(true));
    let check_count = checks.len();
    let passed_count = checks.iter().filter(|check| check["passed"].as_bool() == Some(true)).count();
    let result = if passed { "PASS_SYNTHETIC_AFEX_DESIGN" } else { "FAIL" };
    let mut report = json!({
        "schema_version":"0.1.0",
        "benchmark_id":"LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id":"AFEX-FACTORIAL-DESIGN-001",
        "result":result,
        "source_manifest_sha256":sha256_hex(raw),
        "case_count":1,
        "plot_row_count":plots.len(),
        "unique_plot_count":plot_ids.len(),
        "block_count":blocks.len(),
        "treatment_count":treatments.len(),
        "complete_block_treatment_cells":cell_counts.len(),
        "factorial_treatment_rank":if exact_factorial {8} else {0},
        "block_contrast_rank":if all_cells_once && factor_values_valid && exact_factorial {3} else {0},
        "full_design_rank":calculated_rank,
        "residual_degrees_of_freedom":if calculated_rank.is_some() {Some(plots.len().saturating_sub(expected_rank))} else {None::<usize>},
        "checks_total":check_count,
        "checks_passed":passed_count,
        "external_data_ingested":false,
        "empirical_treatment_effects":"NOT_ESTABLISHED",
        "claim_ceiling":claim_ceiling,
        "checks":checks
    });
    if passed && report["checks_passed"] == report["checks_total"] {
        report["design_status"] = json!("BALANCED_COMPLETE_BLOCK_FACTORIAL_SYNTHETIC");
    } else {
        report["design_status"] = json!("DESIGN_GATE_FAILED");
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../cases/afex-factorial-design-synthetic-v0.1.json");

    #[test]
    fn complete_factorial_design_passes_and_reports_plot_level_replication() {
        let manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest).expect("receipt");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_AFEX_DESIGN");
        assert_eq!(receipt["plot_row_count"], 32);
        assert_eq!(receipt["full_design_rank"], 11);
        assert_eq!(receipt["residual_degrees_of_freedom"], 21);
        assert_eq!(receipt["checks_passed"], receipt["checks_total"]);
        assert_eq!(receipt["empirical_treatment_effects"], "NOT_ESTABLISHED");
    }

    #[test]
    fn duplicated_plot_identity_fails_closed() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let first_plot_id = manifest["plots"][0]["plot_id"].clone();
        manifest["plots"][1]["plot_id"] = first_plot_id;
        let raw = serde_json::to_vec(&manifest).expect("serialize mutation");
        assert!(validate_corpus(&raw, &manifest).is_err());
    }

    #[test]
    fn missing_block_treatment_cell_fails() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        manifest["plots"].as_array_mut().expect("plots")[0]["factors"]["N"] = json!(1);
        let raw = serde_json::to_vec(&manifest).expect("serialize mutation");
        let receipt = validate_corpus(&raw, &manifest).expect("semantic failure receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn tree_level_pseudoreplication_claim_fails() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        manifest["nested_measurements"]["tree_rows_are_independent_treatment_replicates"] = json!(true);
        let raw = serde_json::to_vec(&manifest).expect("serialize mutation");
        let receipt = validate_corpus(&raw, &manifest).expect("semantic failure receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn raw_dendrometer_distance_cannot_be_relabelled_as_dbh_growth() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        manifest["measurement_contract"]["raw_dendrometer_quantity"] = json!("dbh_increment");
        let raw = serde_json::to_vec(&manifest).expect("serialize mutation");
        let receipt = validate_corpus(&raw, &manifest).expect("semantic failure receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn rain_related_missingness_cannot_be_converted_to_zero() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        manifest["measurement_contract"]["rainy_missingness_numeric_zero"] = json!(true);
        let raw = serde_json::to_vec(&manifest).expect("serialize mutation");
        let receipt = validate_corpus(&raw, &manifest).expect("semantic failure receipt");
        assert_eq!(receipt["result"], "FAIL");
    }
}
