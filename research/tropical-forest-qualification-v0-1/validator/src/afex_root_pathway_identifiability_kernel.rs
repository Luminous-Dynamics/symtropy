// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Exact structural identifiability for a simplified AFEX root nutrient pathway model.
//!
//! Rank and nullspace claims are algebraic properties of the declared synthetic
//! observation matrix only. This module is not an ecological uptake estimator.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    fn new(mut numerator: i128, mut denominator: i128) -> Result<Self, String> {
        if denominator == 0 {
            return Err("zero denominator in exact rank arithmetic".to_string());
        }
        if denominator < 0 {
            numerator = -numerator;
            denominator = -denominator;
        }
        let divisor = gcd(numerator.abs(), denominator);
        Ok(Self { numerator: numerator / divisor, denominator: denominator / divisor })
    }

    fn zero() -> Self { Self { numerator: 0, denominator: 1 } }
    fn is_zero(self) -> bool { self.numerator == 0 }

    fn subtract(self, other: Self) -> Result<Self, String> {
        Self::new(
            self.numerator * other.denominator - other.numerator * self.denominator,
            self.denominator * other.denominator
        )
    }

    fn multiply(self, other: Self) -> Result<Self, String> {
        Self::new(self.numerator * other.numerator, self.denominator * other.denominator)
    }

    fn divide(self, other: Self) -> Result<Self, String> {
        if other.is_zero() { return Err("division by zero in exact rank elimination".to_string()); }
        Self::new(self.numerator * other.denominator, self.denominator * other.numerator)
    }
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    if a == 0 { 1 } else { a }
}

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("String write cannot fail");
    }
    output
}

fn integer_matrix(value: &Value, columns: usize) -> Result<Vec<Vec<i128>>, String> {
    let rows = value.as_array().ok_or_else(|| "observation_matrix must be an array".to_string())?;
    let mut matrix = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let entries = row.as_array().ok_or_else(|| format!("observation_matrix[{index}] must be an array"))?;
        if entries.len() != columns {
            return Err(format!("observation_matrix[{index}] must have {columns} columns"));
        }
        let mut parsed = Vec::with_capacity(columns);
        for item in entries {
            let number = item.as_i64().ok_or_else(|| "matrix coefficients must be signed integers".to_string())?;
            parsed.push(i128::from(number));
        }
        matrix.push(parsed);
    }
    Ok(matrix)
}

fn exact_rank(matrix: &[Vec<i128>], columns: usize) -> Result<usize, String> {
    if matrix.is_empty() { return Ok(0); }
    let mut work = Vec::with_capacity(matrix.len());
    for row in matrix {
        if row.len() != columns { return Err("ragged matrix is invalid".to_string()); }
        work.push(row.iter().map(|value| Rational::new(*value, 1)).collect::<Result<Vec<_>,_>>()?);
    }
    let rows = work.len();
    let mut pivot_row = 0usize;
    for column in 0..columns {
        let Some(found) = (pivot_row..rows).find(|row| !work[*row][column].is_zero()) else {
            continue;
        };
        work.swap(pivot_row, found);
        let pivot = work[pivot_row][column];
        for value in column..columns {
            work[pivot_row][value] = work[pivot_row][value].divide(pivot)?;
        }
        for row in 0..rows {
            if row == pivot_row { continue; }
            let factor = work[row][column];
            if factor.is_zero() { continue; }
            for value in column..columns {
                let product = factor.multiply(work[pivot_row][value])?;
                work[row][value] = work[row][value].subtract(product)?;
            }
        }
        pivot_row += 1;
        if pivot_row == rows { break; }
    }
    Ok(pivot_row)
}

fn validate_nullspace(matrix: &[Vec<i128>], basis: &Value, columns: usize) -> Result<usize, String> {
    let vectors = basis.as_array().ok_or_else(|| "expected_nullspace_basis must be an array".to_string())?;
    for (index, vector) in vectors.iter().enumerate() {
        let values = vector.as_array().ok_or_else(|| format!("nullspace vector {index} must be an array"))?;
        if values.len() != columns {
            return Err(format!("nullspace vector {index} must contain {columns} coefficients"));
        }
        let parsed = values.iter().map(|x| x.as_i64()
            .map(i128::from).ok_or_else(|| "nullspace coefficients must be signed integers".to_string()))
            .collect::<Result<Vec<_>,_>>()?;
        if parsed.iter().all(|x| *x == 0) {
            return Err(format!("nullspace vector {index} cannot be zero"));
        }
        for row in matrix {
            let dot = row.iter().zip(parsed.iter()).map(|(a,b)| a*b).sum::<i128>();
            if dot != 0 {
                return Err(format!("declared nullspace vector {index} is not in the matrix nullspace"));
            }
        }
    }
    Ok(vectors.len())
}

pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    if string_at(manifest, "schema_version")? != "0.1.0"
        || string_at(manifest, "benchmark_id")? != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || string_at(manifest, "corpus_id")? != "AFEX-ROOT-PATHWAY-IDENTIFIABILITY-001"
    {
        return Err("AFEX pathway identifiability corpus identity mismatch".to_string());
    }
    let ceiling = string_at(manifest, "claim_ceiling")?;
    if !ceiling.contains("AFEX raw data remain un-ingested")
        || !ceiling.contains("No AFEX pathway partition or fungal carbon cost is established")
    {
        return Err("AFEX pathway identifiability claim ceiling is incomplete".to_string());
    }
    let unknowns = manifest.get("unknown_parameters").and_then(Value::as_array)
        .ok_or_else(|| "unknown_parameters must be an array".to_string())?;
    let unknown_ids = unknowns.iter().map(|x| string_at(x, "id"))
        .collect::<Result<Vec<_>,_>>()?;
    if unknown_ids != [
        "k_root_inorganic",
        "k_myc_inorganic",
        "k_myc_organic",
        "c_host_to_fungal"
    ] {
        return Err("unknown pathway parameter identities/order are invalid".to_string());
    }
    let operator = manifest.get("observation_operator")
        .ok_or_else(|| "observation_operator required".to_string())?;
    if string_at(operator, "colonization_semantics")?
        != "AM colonisation fraction is a covariate in the observation operator, not itself an equation measuring pathway flux."
    {
        return Err("colonisation proxy must not be represented as a pathway flux equation".to_string());
    }

    let current = manifest.get("afex_current_observations").and_then(Value::as_array)
        .ok_or_else(|| "afex_current_observations must be an array".to_string())?;
    if current.is_empty() || current.iter().any(|observation| observation.get("direct_pathway_flux").and_then(Value::as_bool) != Some(false)) {
        return Err("current AFEX proxies must remain explicitly non-flux observations".to_string());
    }

    let cases = manifest.get("cases").and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_string())?;
    let expected_count = manifest.get("expected_case_count").and_then(Value::as_u64)
        .ok_or_else(|| "expected_case_count must be an integer".to_string())? as usize;
    if cases.len() != expected_count || expected_count != 4 {
        return Err("expected exactly four pathway identifiability cases".to_string());
    }

    let mut case_ids = BTreeSet::new();
    let mut results = Vec::new();
    for case in cases {
        let case_id = string_at(case, "case_id")?;
        if !case_ids.insert(case_id.clone()) { return Err("duplicate pathway case ID".to_string()); }
        let matrix = integer_matrix(
            case.get("observation_matrix").ok_or_else(|| "observation_matrix required".to_string())?,
            unknown_ids.len()
        )?;
        let rank = exact_rank(&matrix, unknown_ids.len())?;
        let nullity = unknown_ids.len() - rank;
        let expected_rank = case.get("expected_rank").and_then(Value::as_u64)
            .ok_or_else(|| format!("case {case_id} expected_rank must be integer"))? as usize;
        let expected_nullity = case.get("expected_nullity").and_then(Value::as_u64)
            .ok_or_else(|| format!("case {case_id} expected_nullity must be integer"))? as usize;
        if rank != expected_rank || nullity != expected_nullity {
            return Err(format!("case {case_id} exact rank/nullity mismatch: got {rank}/{nullity}, expected {expected_rank}/{expected_nullity}"));
        }
        let basis_count = validate_nullspace(
            &matrix,
            case.get("expected_nullspace_basis").ok_or_else(|| "expected_nullspace_basis required".to_string())?,
            unknown_ids.len()
        )?;
        if basis_count != nullity {
            return Err(format!("case {case_id} nullspace witness count {basis_count} does not equal nullity {nullity}"));
        }
        let status = string_at(case, "evidence_status")?;
        let channel_text = case.get("observed_channels").and_then(Value::as_array)
            .ok_or_else(|| "observed_channels must be an array".to_string())?
            .iter().filter_map(Value::as_str).collect::<Vec<_>>().join(";");
        if case_id == "CURRENT_AFEX_PROXY_ONLY" && !channel_text.contains("AM colonisation fraction") {
            return Err("current AFEX proxy case must identify colonisation fraction".to_string());
        }
        if status.contains("HYPOTHETICAL") && status.contains("CURRENT_AFEX") {
            return Err(format!("case {case_id} has contradictory evidence status"));
        }
        results.push(json!({
            "case_id": case_id,
            "evidence_status": status,
            "matrix_rows": matrix.len(),
            "parameter_count": unknown_ids.len(),
            "exact_rank": rank,
            "nullity": nullity,
            "nullspace_witness_count": basis_count,
            "conclusion": string_at(case, "expected_conclusion")?
        }));
    }
    if !case_ids.contains("CURRENT_AFEX_PROXY_ONLY")
        || !case_ids.contains("TOTAL_UPTAKE_WITH_VARIATION_IN_COLONISATION")
        || !case_ids.contains("ADD_SOURCE_SPECIFIC_MYCORRHIZAL_INORGANIC_TRACER")
        || !case_ids.contains("ADD_HOST_TO_FUNGAL_CARBON_ALLOCATION_TRACER")
    {
        return Err("required current, aggregate, pathway-tracer and carbon-tracer cases are missing".to_string());
    }

    Ok(json!({
        "schema_version": "0.1.0",
        "benchmark_id": "LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id": "AFEX-ROOT-PATHWAY-IDENTIFIABILITY-001",
        "result": "PASS_SYNTHETIC_PATHWAY_IDENTIFIABILITY",
        "source_manifest_sha256": sha256_hex(raw),
        "unknown_parameter_count": unknown_ids.len(),
        "case_count": results.len(),
        "ranks": results.iter().map(|x| x["exact_rank"].clone()).collect::<Vec<_>>(),
        "nullities": results.iter().map(|x| x["nullity"].clone()).collect::<Vec<_>>(),
        "current_afex_pathway_flux_identifiability": "NONE_ESTABLISHED_FROM_CURRENT_REGISTERED_TRAITS",
        "mycorrhizal_inorganic_organic_alias": "UNRESOLVED_WITH_TOTAL_UPTAKE_ONLY",
        "host_to_fungal_carbon_cost": "UNIDENTIFIABLE_WITHOUT_INDEPENDENT_C_ALLOCATION_OBSERVATION",
        "empirical_pathway_partition": "NOT_ESTABLISHED",
        "case_results": results
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../cases/afex-root-pathway-identifiability-synthetic-v0.1.json");

    #[test]
    fn exact_ranks_and_nullities_match_the_four_frozen_cases() {
        let manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest).expect("receipt");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_PATHWAY_IDENTIFIABILITY");
        assert_eq!(receipt["ranks"], json!([0, 2, 3, 4]));
        assert_eq!(receipt["nullities"], json!([4, 2, 1, 0]));
        assert_eq!(receipt["empirical_pathway_partition"], "NOT_ESTABLISHED");
    }

    #[test]
    fn colonisation_proxy_cannot_be_relabelled_as_a_flux() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture");
        manifest["afex_current_observations"][0]["direct_pathway_flux"] = json!(true);
        let raw = serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw, &manifest).is_err());
    }

    #[test]
    fn missing_fungal_organic_pathway_alias_witness_fails() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture");
        manifest["cases"][1]["expected_nullspace_basis"][0] = json!([0, 1, 0, 0]);
        let raw = serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw, &manifest).is_err());
    }

    #[test]
    fn carbon_cost_cannot_be_claimed_identified_without_tracer_row() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture");
        manifest["cases"][2]["expected_rank"] = json!(4);
        manifest["cases"][2]["expected_nullity"] = json!(0);
        manifest["cases"][2]["expected_nullspace_basis"] = json!([]);
        let raw = serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw, &manifest).is_err());
    }
}
