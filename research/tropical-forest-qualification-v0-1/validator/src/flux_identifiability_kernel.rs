// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Exact structural identifiability and finite measurement-design enumeration.
//!
//! This module uses exact rational row reduction on a bounded integer observation
//! operator. It diagnoses structural non-identifiability only: measurement noise,
//! practical parameter identifiability, spatial dependence and ecological validity
//! require separate empirical analysis.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;

const MAX_VARIABLES: usize = 16;
const MAX_ROWS: usize = 128;
const MAX_MEASUREMENT_OPTIONS: usize = 16;
const MAX_ABSOLUTE_COEFFICIENT: i64 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    fn new(mut numerator: i128, mut denominator: i128) -> Result<Self, String> {
        if denominator == 0 {
            return Err("rational denominator must be non-zero".to_string());
        }
        if denominator < 0 {
            numerator = numerator
                .checked_neg()
                .ok_or_else(|| "rational sign normalization overflow".to_string())?;
            denominator = denominator
                .checked_neg()
                .ok_or_else(|| "rational denominator normalization overflow".to_string())?;
        }
        if numerator == 0 {
            return Ok(Self::ZERO);
        }
        let absolute_numerator = numerator
            .checked_abs()
            .ok_or_else(|| "rational absolute-value overflow".to_string())?;
        let divisor = gcd(absolute_numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn from_i64(value: i64) -> Self {
        Self {
            numerator: i128::from(value),
            denominator: 1,
        }
    }

    fn is_zero(self) -> bool {
        self.numerator == 0
    }

    fn add(self, other: Self) -> Result<Self, String> {
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or_else(|| "rational addition overflow".to_string())?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or_else(|| "rational addition overflow".to_string())?;
        let numerator = left
            .checked_add(right)
            .ok_or_else(|| "rational addition overflow".to_string())?;
        let denominator = self
            .denominator
            .checked_mul(other.denominator)
            .ok_or_else(|| "rational denominator overflow".to_string())?;
        Self::new(numerator, denominator)
    }

    fn sub(self, other: Self) -> Result<Self, String> {
        let negated = other
            .numerator
            .checked_neg()
            .ok_or_else(|| "rational subtraction overflow".to_string())?;
        self.add(Self {
            numerator: negated,
            denominator: other.denominator,
        })
    }

    fn mul(self, other: Self) -> Result<Self, String> {
        let numerator = self
            .numerator
            .checked_mul(other.numerator)
            .ok_or_else(|| "rational multiplication overflow".to_string())?;
        let denominator = self
            .denominator
            .checked_mul(other.denominator)
            .ok_or_else(|| "rational denominator overflow".to_string())?;
        Self::new(numerator, denominator)
    }

    fn div(self, other: Self) -> Result<Self, String> {
        if other.is_zero() {
            return Err("division by zero during exact row reduction".to_string());
        }
        let numerator = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or_else(|| "rational division overflow".to_string())?;
        let denominator = self
            .denominator
            .checked_mul(other.numerator)
            .ok_or_else(|| "rational denominator overflow".to_string())?;
        Self::new(numerator, denominator)
    }
}

fn gcd(mut left: i128, mut right: i128) -> i128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Exact rational rank shared with noise-aware practical-identifiability diagnostics.
pub(crate) fn exact_structural_rank_for_observation_rows(
    matrix: &[Vec<i64>],
    column_count: usize,
) -> Result<usize, String> {
    exact_rank(matrix, column_count)
}

fn exact_rank(matrix: &[Vec<i64>], column_count: usize) -> Result<usize, String> {
    if column_count == 0 || column_count > MAX_VARIABLES {
        return Err(format!("column count must be in 1..={MAX_VARIABLES}"));
    }
    if matrix.len() > MAX_ROWS {
        return Err(format!("row count exceeds {MAX_ROWS}"));
    }
    for (row_index, row) in matrix.iter().enumerate() {
        if row.len() != column_count {
            return Err(format!(
                "matrix row {row_index} has {} columns; expected {column_count}",
                row.len()
            ));
        }
        if row.iter().any(|value| *value < -MAX_ABSOLUTE_COEFFICIENT || *value > MAX_ABSOLUTE_COEFFICIENT) {
            return Err(format!(
                "matrix row {row_index} contains a coefficient outside the supported range"
            ));
        }
    }

    let mut reduced: Vec<Vec<Rational>> = matrix
        .iter()
        .map(|row| row.iter().copied().map(Rational::from_i64).collect())
        .collect();

    let mut pivot_row = 0usize;
    for column in 0..column_count {
        let Some(found_row) = (pivot_row..reduced.len())
            .find(|row| !reduced[*row][column].is_zero())
        else {
            continue;
        };
        reduced.swap(pivot_row, found_row);

        let pivot = reduced[pivot_row][column];
        for entry in column..column_count {
            reduced[pivot_row][entry] = reduced[pivot_row][entry].div(pivot)?;
        }

        let pivot_values = reduced[pivot_row].clone();
        for row_index in 0..reduced.len() {
            if row_index == pivot_row {
                continue;
            }
            let factor = reduced[row_index][column];
            if factor.is_zero() {
                continue;
            }
            for entry in column..column_count {
                let product = factor.mul(pivot_values[entry])?;
                reduced[row_index][entry] = reduced[row_index][entry].sub(product)?;
            }
        }

        pivot_row += 1;
        if pivot_row == reduced.len() {
            break;
        }
    }

    Ok(pivot_row)
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn usize_at(value: &Value, key: &str) -> Result<usize, String> {
    let number = value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))?;
    usize::try_from(number).map_err(|_| format!("{key} exceeds platform usize"))
}

fn u64_at(value: &Value, key: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}

fn matrix_at(value: &Value, key: &str, columns: usize) -> Result<Vec<Vec<i64>>, String> {
    let rows = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array of integer rows"))?;
    if rows.is_empty() || rows.len() > MAX_ROWS {
        return Err(format!("{key} row count must be in 1..={MAX_ROWS}"));
    }
    let mut matrix = Vec::with_capacity(rows.len());
    for (row_index, row) in rows.iter().enumerate() {
        let entries = row
            .as_array()
            .ok_or_else(|| format!("{key}[{row_index}] must be an array"))?;
        if entries.len() != columns {
            return Err(format!(
                "{key}[{row_index}] has {} columns; expected {columns}",
                entries.len()
            ));
        }
        let mut parsed = Vec::with_capacity(columns);
        for (column_index, entry) in entries.iter().enumerate() {
            let number = entry.as_i64().ok_or_else(|| {
                format!("{key}[{row_index}][{column_index}] must be an integer")
            })?;
            if number < -MAX_ABSOLUTE_COEFFICIENT || number > MAX_ABSOLUTE_COEFFICIENT {
                return Err(format!(
                    "{key}[{row_index}][{column_index}] exceeds coefficient bounds"
                ));
            }
            parsed.push(number);
        }
        matrix.push(parsed);
    }
    Ok(matrix)
}

fn validate_nullspace_witnesses(
    matrix: &[Vec<i64>],
    witnesses: &[Vec<i64>],
    variable_count: usize,
    expected_nullity: usize,
) -> Result<bool, String> {
    if witnesses.len() != expected_nullity {
        return Ok(false);
    }
    for (witness_index, witness) in witnesses.iter().enumerate() {
        if witness.len() != variable_count {
            return Err(format!("nullspace witness {witness_index} has wrong width"));
        }
        if witness.iter().all(|value| *value == 0) {
            return Ok(false);
        }
        for row in matrix {
            let dot = row
                .iter()
                .zip(witness.iter())
                .try_fold(0_i128, |sum, (coefficient, value)| {
                    let product = i128::from(*coefficient)
                        .checked_mul(i128::from(*value))
                        .ok_or_else(|| "nullspace dot-product overflow".to_string())?;
                    sum.checked_add(product)
                        .ok_or_else(|| "nullspace dot-product overflow".to_string())
                })?;
            if dot != 0 {
                return Ok(false);
            }
        }
    }
    Ok(exact_rank(witnesses, variable_count)? == expected_nullity)
}

#[derive(Clone, Debug)]
struct MeasurementOption {
    id: String,
    cost: u64,
    row: Vec<i64>,
}

fn parse_measurement_options(
    value: &Value,
    variable_count: usize,
) -> Result<Vec<MeasurementOption>, String> {
    let options = value
        .get("measurement_options")
        .and_then(Value::as_array)
        .ok_or_else(|| "measurement_options must be an array".to_string())?;
    if options.is_empty() || options.len() > MAX_MEASUREMENT_OPTIONS {
        return Err(format!(
            "measurement option count must be in 1..={MAX_MEASUREMENT_OPTIONS}"
        ));
    }

    let mut seen = BTreeSet::new();
    let mut parsed = Vec::with_capacity(options.len());
    for (index, option) in options.iter().enumerate() {
        let id = string_at(option, "measurement_id")?;
        if id.trim().is_empty() || !seen.insert(id.clone()) {
            return Err(format!(
                "measurement option {index} has empty or duplicate identity"
            ));
        }
        let cost = u64_at(option, "synthetic_effort_cost")?;
        let row_value = option
            .get("row")
            .ok_or_else(|| format!("measurement option {id} requires row"))?;
        let row = row_value
            .as_array()
            .ok_or_else(|| format!("measurement option {id} row must be an array"))?
            .iter()
            .map(|entry| {
                let coefficient = entry
                    .as_i64()
                    .ok_or_else(|| format!("measurement option {id} row must contain integers"))?;
                if coefficient < -MAX_ABSOLUTE_COEFFICIENT || coefficient > MAX_ABSOLUTE_COEFFICIENT {
                    return Err(format!("measurement option {id} coefficient exceeds bounds"));
                }
                Ok(coefficient)
            })
            .collect::<Result<Vec<_>, String>>()?;
        if row.len() != variable_count {
            return Err(format!(
                "measurement option {id} has {} columns; expected {variable_count}",
                row.len()
            ));
        }
        parsed.push(MeasurementOption { id, cost, row });
    }
    Ok(parsed)
}

fn enumerate_designs(
    base_matrix: &[Vec<i64>],
    options: &[MeasurementOption],
    variable_count: usize,
) -> Result<Vec<Value>, String> {
    let subset_count = 1_u32
        .checked_shl(u32::try_from(options.len()).map_err(|_| "too many options")?)
        .ok_or_else(|| "measurement subset count overflow".to_string())?;
    let subset_capacity = usize::try_from(subset_count)
        .map_err(|_| "measurement subset count exceeds platform usize".to_string())?;
    let mut designs = Vec::with_capacity(subset_capacity);

    for mask in 0..subset_count {
        let mut matrix = base_matrix.to_vec();
        let mut selected = Vec::new();
        let mut cost = 0_u64;
        for (index, option) in options.iter().enumerate() {
            if mask & (1_u32 << index) != 0 {
                matrix.push(option.row.clone());
                selected.push(option.id.clone());
                cost = cost
                    .checked_add(option.cost)
                    .ok_or_else(|| "measurement design cost overflow".to_string())?;
            }
        }
        let rank = exact_rank(&matrix, variable_count)?;
        designs.push(json!({
            "measurement_ids": selected,
            "measurement_count": selected.len(),
            "synthetic_effort_cost": cost,
            "rank": rank,
            "nullity": variable_count - rank,
            "structurally_identifies_all_fluxes": rank == variable_count
        }));
    }
    Ok(designs)
}

fn summarize_designs(designs: &[Value], variable_count: usize) -> Result<Value, String> {
    let full_rank: Vec<&Value> = designs
        .iter()
        .filter(|design| {
            design["structurally_identifies_all_fluxes"].as_bool() == Some(true)
        })
        .collect();
    if full_rank.is_empty() {
        return Err("no measurement subset identifies all unknown fluxes".to_string());
    }

    let minimum_cost = full_rank
        .iter()
        .filter_map(|design| design["synthetic_effort_cost"].as_u64())
        .min()
        .ok_or_else(|| "full-rank design lacks a valid cost".to_string())?;
    let minimum_measurement_count = full_rank
        .iter()
        .filter_map(|design| design["measurement_count"].as_u64())
        .min()
        .ok_or_else(|| "full-rank design lacks a valid measurement count".to_string())?;
    let optimal_designs: Vec<Value> = full_rank
        .iter()
        .filter(|design| {
            design["synthetic_effort_cost"].as_u64() == Some(minimum_cost)
        })
        .map(|design| {
            json!({
                "measurement_ids": design["measurement_ids"],
                "measurement_count": design["measurement_count"],
                "synthetic_effort_cost": design["synthetic_effort_cost"],
                "rank": variable_count,
                "nullity": 0
            })
        })
        .collect();

    Ok(json!({
        "full_rank_design_count": full_rank.len(),
        "minimum_synthetic_effort_cost": minimum_cost,
        "minimum_measurement_count": minimum_measurement_count,
        "optimal_designs": optimal_designs
    }))
}

/// Verify exact structural rank, nullspace witnesses and exhaustive measurement designs.
pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let schema_version = string_at(manifest, "schema_version")?;
    let benchmark_id = string_at(manifest, "benchmark_id")?;
    let corpus_id = string_at(manifest, "corpus_id")?;
    let claim_ceiling = string_at(manifest, "claim_ceiling")?;
    if schema_version != "0.1.0"
        || benchmark_id != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || corpus_id != "FOREST-FLUX-IDENTIFIABILITY-001"
        || !claim_ceiling.contains("does not establish practical identifiability")
    {
        return Err("flux-identifiability corpus identity or claim ceiling is invalid".to_string());
    }

    let variables = manifest
        .get("unknown_fluxes")
        .and_then(Value::as_array)
        .ok_or_else(|| "unknown_fluxes must be an array".to_string())?;
    let variable_names: Vec<String> = variables
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| "unknown_fluxes entries must be strings".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let variable_count = variable_names.len();
    if variable_count == 0 || variable_count > MAX_VARIABLES {
        return Err(format!("unknown flux count must be in 1..={MAX_VARIABLES}"));
    }
    let mut variable_ids = BTreeSet::new();
    if variable_names.iter().any(|name| name.trim().is_empty() || !variable_ids.insert(name)) {
        return Err("unknown flux names must be non-empty and unique".to_string());
    }

    let base_matrix = matrix_at(manifest, "base_observation_operator", variable_count)?;
    let baseline_rank = exact_rank(&base_matrix, variable_count)?;
    let baseline_nullity = variable_count - baseline_rank;

    let witness_values = manifest
        .get("nullspace_witnesses")
        .and_then(Value::as_array)
        .ok_or_else(|| "nullspace_witnesses must be an array".to_string())?;
    let mut witnesses = Vec::with_capacity(witness_values.len());
    for (index, witness) in witness_values.iter().enumerate() {
        let values = witness
            .as_array()
            .ok_or_else(|| format!("nullspace_witnesses[{index}] must be an array"))?;
        if values.len() != variable_count {
            return Err(format!("nullspace_witnesses[{index}] has wrong width"));
        }
        let parsed = values
            .iter()
            .map(|value| {
                value
                    .as_i64()
                    .ok_or_else(|| "nullspace witness coefficients must be integers".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if parsed
            .iter()
            .any(|value| *value < -MAX_ABSOLUTE_COEFFICIENT || *value > MAX_ABSOLUTE_COEFFICIENT)
        {
            return Err(format!("nullspace_witnesses[{index}] exceeds coefficient bounds"));
        }
        witnesses.push(parsed);
    }
    let witnesses_valid = validate_nullspace_witnesses(
        &base_matrix,
        &witnesses,
        variable_count,
        baseline_nullity,
    )?;

    let options = parse_measurement_options(manifest, variable_count)?;
    let designs = enumerate_designs(&base_matrix, &options, variable_count)?;
    let design_summary = summarize_designs(&designs, variable_count)?;

    let expected = manifest
        .get("expected")
        .ok_or_else(|| "expected object is required".to_string())?;
    let expected_rank = usize_at(expected, "baseline_rank")?;
    let expected_nullity = usize_at(expected, "baseline_nullity")?;
    let expected_witness_count = usize_at(expected, "independent_nullspace_witness_count")?;
    let expected_subset_count = usize_at(expected, "subset_count")?;
    let expected_full_rank_count = usize_at(expected, "full_rank_design_count")?;
    let expected_minimum_cost = u64_at(expected, "minimum_synthetic_effort_cost")?;
    let expected_minimum_count = usize_at(expected, "minimum_measurement_count")?;
    let expected_optimal_designs = expected
        .get("optimal_designs")
        .ok_or_else(|| "expected.optimal_designs is required".to_string())?;

    let actual_subset_count = designs.len();
    let actual_optimal_designs = design_summary["optimal_designs"].clone();
    let expected_full_rank_count_u64 = u64::try_from(expected_full_rank_count)
        .map_err(|_| "expected full-rank design count exceeds u64".to_string())?;
    let expected_minimum_count_u64 = u64::try_from(expected_minimum_count)
        .map_err(|_| "expected minimum measurement count exceeds u64".to_string())?;
    let checks = vec![
        json!({
            "id": "BASE-OPERATOR-RANK-001",
            "passed": baseline_rank == expected_rank && baseline_nullity == expected_nullity,
            "actual_rank": baseline_rank,
            "expected_rank": expected_rank,
            "actual_nullity": baseline_nullity,
            "expected_nullity": expected_nullity
        }),
        json!({
            "id": "NULLSPACE-WITNESSES-001",
            "passed": witnesses_valid && witnesses.len() == expected_witness_count,
            "witness_count": witnesses.len(),
            "expected_witness_count": expected_witness_count,
            "witnesses_are_independent_and_annihilated": witnesses_valid
        }),
        json!({
            "id": "MEASUREMENT-SUBSET-ENUMERATION-001",
            "passed": actual_subset_count == expected_subset_count,
            "actual_subset_count": actual_subset_count,
            "expected_subset_count": expected_subset_count
        }),
        json!({
            "id": "FULL-RANK-DESIGN-COUNT-001",
            "passed": design_summary["full_rank_design_count"].as_u64()
                == Some(expected_full_rank_count_u64),
            "actual_full_rank_design_count": design_summary["full_rank_design_count"],
            "expected_full_rank_design_count": expected_full_rank_count
        }),
        json!({
            "id": "MINIMUM-COST-MEASUREMENT-DESIGN-001",
            "passed": design_summary["minimum_synthetic_effort_cost"].as_u64()
                == Some(expected_minimum_cost),
            "actual_minimum_cost": design_summary["minimum_synthetic_effort_cost"],
            "expected_minimum_cost": expected_minimum_cost
        }),
        json!({
            "id": "MINIMUM-MEASUREMENT-COUNT-001",
            "passed": design_summary["minimum_measurement_count"].as_u64()
                == Some(expected_minimum_count_u64),
            "actual_minimum_measurement_count": design_summary["minimum_measurement_count"],
            "expected_minimum_measurement_count": expected_minimum_count
        }),
        json!({
            "id": "OPTIMAL-DESIGN-IDENTITIES-001",
            "passed": actual_optimal_designs == *expected_optimal_designs,
            "actual_optimal_designs": actual_optimal_designs,
            "expected_optimal_designs": expected_optimal_designs
        }),
    ];
    let checks_passed = checks
        .iter()
        .filter(|check| check["passed"].as_bool() == Some(true))
        .count();
    let passed = checks_passed == checks.len();

    Ok(json!({
        "receipt_version": "0.1.0",
        "validator": "symtropy-tropical-forest-qualifier",
        "benchmark_id": benchmark_id,
        "corpus_id": corpus_id,
        "source_manifest_sha256": sha256_hex(raw),
        "result": if passed { "PASS_SYNTHETIC_FLUX_IDENTIFIABILITY" } else { "FAIL" },
        "checks_passed": checks_passed,
        "checks_total": checks.len(),
        "checks": checks,
        "baseline": {
            "rank": baseline_rank,
            "nullity": baseline_nullity,
            "unknown_fluxes": variable_names,
            "nullspace_witnesses": witnesses
        },
        "measurement_design": design_summary,
        "designs": designs,
        "structural_identifiability": if baseline_nullity == 0 { "STRUCTURALLY_IDENTIFIABLE" } else { "STRUCTURALLY_NON_IDENTIFIABLE" },
        "practical_identifiability": "NOT_EVALUATED",
        "measurement_error": "NOT_MODELLED",
        "empirical_ecological_validity": "NOT_ESTABLISHED",
        "external_data_ingested": false
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../cases/forest-flux-identifiability-synthetic-v0.1.json");

    fn manifest() -> Value {
        serde_json::from_str(FIXTURE).expect("valid flux-identifiability fixture")
    }

    #[test]
    fn stock_only_observations_have_rank_two_and_nullity_four() {
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("valid corpus");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_FLUX_IDENTIFIABILITY");
        assert_eq!(receipt["baseline"]["rank"], 2);
        assert_eq!(receipt["baseline"]["nullity"], 4);
    }

    #[test]
    fn supplied_nullspace_witnesses_are_independent_and_invisible_to_stocks() {
        let value = manifest();
        let matrix = matrix_at(&value, "base_observation_operator", 6).expect("base matrix");
        let witnesses = value["nullspace_witnesses"]
            .as_array().unwrap()
            .iter()
            .map(|row| row.as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect())
            .collect::<Vec<Vec<i64>>>();
        assert!(validate_nullspace_witnesses(&matrix, &witnesses, 6, 4).unwrap());
    }

    #[test]
    fn exhaustive_measurement_tournament_finds_two_minimum_cost_designs() {
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("valid corpus");
        assert_eq!(receipt["measurement_design"]["full_rank_design_count"], 18);
        assert_eq!(receipt["measurement_design"]["minimum_synthetic_effort_cost"], 14);
        assert_eq!(receipt["measurement_design"]["minimum_measurement_count"], 4);
        assert_eq!(receipt["measurement_design"]["optimal_designs"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn corrupted_nullspace_witness_is_rejected() {
        let mut value = manifest();
        value["nullspace_witnesses"][0][0] = json!(0);
        let raw = serde_json::to_vec(&value).expect("serialize corrupted witness");
        let receipt = validate_corpus(&raw, &value).expect("well-formed corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn dependent_nullspace_witness_set_is_rejected() {
        let mut value = manifest();
        value["nullspace_witnesses"][1] = value["nullspace_witnesses"][0].clone();
        let raw = serde_json::to_vec(&value).expect("serialize dependent witnesses");
        let receipt = validate_corpus(&raw, &value).expect("well-formed corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn measurement_cost_mutation_changes_optimum_and_fails_oracle() {
        let mut value = manifest();
        value["measurement_options"][4]["synthetic_effort_cost"] = json!(50);
        let raw = serde_json::to_vec(&value).expect("serialize changed cost");
        let receipt = validate_corpus(&raw, &value).expect("well-formed corpus returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn duplicate_measurement_identity_is_rejected() {
        let mut value = manifest();
        value["measurement_options"][1]["measurement_id"] =
            value["measurement_options"][0]["measurement_id"].clone();
        let raw = serde_json::to_vec(&value).expect("serialize duplicate identity");
        assert!(validate_corpus(&raw, &value).is_err());
    }

    #[test]
    fn invalid_measurement_row_width_is_rejected() {
        let mut value = manifest();
        value["measurement_options"][0]["row"].as_array_mut().unwrap().pop();
        let raw = serde_json::to_vec(&value).expect("serialize malformed row");
        assert!(validate_corpus(&raw, &value).is_err());
    }

    #[test]
    fn integer_rank_is_exact_for_dependent_and_independent_rows() {
        assert_eq!(exact_rank(&[vec![1, 1, 0], vec![2, 2, 0]], 3).unwrap(), 1);
        assert_eq!(exact_rank(&[vec![1, 1, 0], vec![0, 1, 1]], 3).unwrap(), 2);
    }
}
