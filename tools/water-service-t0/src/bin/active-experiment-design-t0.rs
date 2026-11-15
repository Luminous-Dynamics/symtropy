// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, path::{Path, PathBuf}};

const PROGRAMME: &str = "WATER-MIXING-ACTIVE-EXPERIMENT-DESIGN-001";
const CASE_COUNT: usize = 8;
const EVIDENCE_CEILING: &str = "synthetic sensor-aware Fisher-information design arithmetic only";
type Matrix = Vec<Vec<f64>>;
type Vector = Vec<f64>;

fn field<'a>(v: &'a Value, key: &str) -> Result<&'a Value, String> {
    v.get(key).ok_or_else(|| format!("missing field: {key}"))
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    field(v, key)?.as_str().ok_or_else(|| format!("{key} must be text"))
}
fn number(v: &Value, key: &str) -> Result<f64, String> {
    field(v, key)?.as_f64().filter(|x| x.is_finite()).ok_or_else(|| format!("{key} must be finite numeric"))
}
fn boolean(v: &Value, key: &str) -> Result<bool, String> {
    field(v, key)?.as_bool().ok_or_else(|| format!("{key} must be boolean"))
}
fn usize_value(v: &Value, key: &str) -> Result<usize, String> {
    field(v, key)?.as_u64().and_then(|x| usize::try_from(x).ok()).ok_or_else(|| format!("{key} must be a non-negative integer"))
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    field(v, key)?.as_array().ok_or_else(|| format!("{key} must be an array"))
}
fn parse_matrix(v: &Value, key: &str, rows: usize, cols: usize) -> Result<Matrix, String> {
    let raw = array(v, key)?;
    if raw.len() != rows { return Err(format!("{key} row count mismatch")); }
    let mut out = Vec::with_capacity(rows);
    for row in raw {
        let values = row.as_array().ok_or_else(|| format!("{key} row must be an array"))?;
        if values.len() != cols { return Err(format!("{key} column count mismatch")); }
        out.push(values.iter().map(|x| x.as_f64().filter(|n| n.is_finite()).ok_or_else(|| format!("{key} contains a non-finite value"))).collect::<Result<Vector, _>>()?);
    }
    Ok(out)
}
fn parse_vector(v: &Value, key: &str, len: usize) -> Result<Vector, String> {
    let raw = array(v, key)?;
    if raw.len() != len { return Err(format!("{key} length mismatch")); }
    raw.iter().map(|x| x.as_f64().filter(|n| n.is_finite()).ok_or_else(|| format!("{key} contains a non-finite value"))).collect()
}
fn base_candidate(state: &str, reason: &str) -> Value {
    json!({
        "candidate_disposition": state, "reason": reason,
        "observation_count": null, "parameter_count": null,
        "design_rank": null, "singular_values": null,
        "minimum_singular_value": null, "condition_number": null,
        "fisher_log_determinant": null, "fisher_trace": null,
        "total_dose": null, "duration_seconds": null,
        "information_per_cost": null, "selected": false
    })
}
fn base_case(state: &str, reason: &str, candidates: Vec<Value>) -> Value {
    json!({
        "design_disposition": state, "reason": reason,
        "selected_candidate_id": null,
        "candidate_results": candidates,
        "physical_operator_qualified": false,
        "physical_experiment_performed": false,
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}
#[allow(clippy::needless_range_loop)]
fn symmetric_eigenvalues(mut a: Matrix) -> Result<Vector, String> {
    let n = a.len();
    if n == 0 || a.iter().any(|row| row.len() != n) { return Err("Fisher matrix must be non-empty and square".into()); }
    let scale = a.iter().flatten().map(|x| x.abs()).fold(0.0_f64, f64::max).max(1.0e-300);
    for i in 0..n { for j in 0..n {
        if (a[i][j] - a[j][i]).abs() > 1.0e-10 * scale { return Err("Fisher matrix is not symmetric".into()); }
    }}
    if n == 1 { return Ok(vec![a[0][0]]); }
    let mut converged = false;
    for _ in 0..(100 * n * n) {
        let mut p = 0usize; let mut q = 1usize; let mut largest = 0.0_f64;
        for i in 0..n { for j in (i + 1)..n { if a[i][j].abs() > largest { largest = a[i][j].abs(); p = i; q = j; } } }
        if largest <= 1.0e-13 * scale { converged = true; break; }
        let app = a[p][p]; let aqq = a[q][q]; let apq = a[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let sign = if tau >= 0.0 { 1.0 } else { -1.0 };
        let t = sign / (tau.abs() + (1.0 + tau * tau).sqrt());
        let c = 1.0 / (1.0 + t * t).sqrt(); let s = t * c;
        for k in 0..n {
            if k != p && k != q {
                let kp = a[k][p]; let kq = a[k][q];
                a[k][p] = c * kp - s * kq; a[p][k] = a[k][p];
                a[k][q] = s * kp + c * kq; a[q][k] = a[k][q];
            }
        }
        a[p][p] = app - t * apq; a[q][q] = aqq + t * apq;
        a[p][q] = 0.0; a[q][p] = 0.0;
    }
    if !converged { return Err("symmetric Jacobi eigensolver did not converge".into()); }
    let mut values = (0..n).map(|i| a[i][i]).collect::<Vector>();
    values.sort_by(|x, y| y.total_cmp(x));
    if values.iter().any(|x| *x < -1.0e-9 * scale) { return Err("Fisher matrix has a materially negative eigenvalue".into()); }
    for value in &mut values { if *value < 0.0 { *value = 0.0; } }
    Ok(values)
}
fn fisher_information(jacobian: &Matrix, variances: &Vector) -> Result<Matrix, String> {
    if jacobian.is_empty() || jacobian.len() != variances.len() { return Err("Jacobian/variance row count mismatch".into()); }
    let p = jacobian[0].len();
    if p == 0 || jacobian.iter().any(|row| row.len() != p) { return Err("Jacobian dimensions invalid".into()); }
    if variances.iter().any(|v| !v.is_finite() || *v <= 0.0) { return Err("observation variances must be positive and finite".into()); }
    let mut f = vec![vec![0.0; p]; p];
    for (row, variance) in jacobian.iter().zip(variances) {
        let weight = 1.0 / variance;
        for i in 0..p { for k in 0..p { f[i][k] += weight * row[i] * row[k]; } }
    }
    if f.iter().flatten().any(|x| !x.is_finite()) { return Err("Fisher information overflow".into()); }
    Ok(f)
}
fn evaluate_candidate(candidate: &Value, constraints: &Value) -> Result<Value, String> {
    let dose = number(candidate, "tracer_dose_units")?;
    let duration = number(candidate, "duration_seconds")?;
    let sample_interval = number(candidate, "sample_interval_seconds")?;
    let min_resolvable = number(candidate, "minimum_resolvable_delay_seconds")?;
    let min_delay_interest = number(candidate, "minimum_delay_of_interest_seconds")?;
    if dose <= 0.0 || duration <= 0.0 || sample_interval <= 0.0 || min_resolvable <= 0.0 || min_delay_interest <= 0.0 {
        return Ok(base_candidate("INVALID_INPUT", "DOSE_DURATION_OR_TEMPORAL_LIMIT_NOT_POSITIVE"));
    }
    if dose > number(constraints, "maximum_tracer_dose_units")? {
        return Ok(base_candidate("DOSE_LIMIT_EXCEEDED", "TRACER_DOSE_EXCEEDS_APPROVED_DESIGN_LIMIT"));
    }
    if duration > number(constraints, "maximum_duration_seconds")? {
        return Ok(base_candidate("DURATION_LIMIT_EXCEEDED", "RUN_DURATION_EXCEEDS_DESIGN_LIMIT"));
    }
    if !boolean(candidate, "complete_outlet_boundary")? {
        return Ok(base_candidate("PARTIAL_OUTLET_BOUNDARY", "NOT_ALL_OUTLETS_ARE_INSTRUMENTED"));
    }
    if text(candidate, "sensor_calibration_state")? != "calibrated" || !boolean(candidate, "sensor_response_applied")? {
        return Ok(base_candidate("SENSOR_RESPONSE_UNRESOLVED", "SENSOR_RESPONSE_NOT_CALIBRATED_AND_APPLIED"));
    }
    if min_resolvable + 1.0e-12 < sample_interval {
        return Ok(base_candidate("INVALID_INPUT", "CLAIMED_SENSOR_RESOLUTION_IS_FINER_THAN_SAMPLE_INTERVAL"));
    }
    if min_resolvable > min_delay_interest {
        return Ok(base_candidate("TEMPORAL_RESOLUTION_INADEQUATE", "SENSOR_CANNOT_RESOLVE_MINIMUM_DELAY_OF_INTEREST"));
    }
    if text(candidate, "operating_regime_state")? != "fixed_regime" {
        return Ok(base_candidate("REGIME_TRANSITION_UNSUPPORTED", "FIXED_REGIME_IDENTIFICATION_CANNOT_CROSS_HYDRAULIC_TRANSITION"));
    }
    if boolean(candidate, "flow_reversal_present")? && !boolean(candidate, "all_port_directions_observed")? {
        return Ok(base_candidate("ABSTAIN_FLOW_DIRECTION_UNOBSERVED", "SIGNED_PORT_DIRECTION_NOT_MEASURED"));
    }
    if text(candidate, "domain_state")? != "in_domain" {
        return Ok(base_candidate("OUT_OF_DOMAIN", "EXCITATION_OPERATING_STATE_OUTSIDE_DECLARED_DOMAIN"));
    }
    let raw_j = array(candidate, "sensitivity_matrix")?;
    if raw_j.is_empty() { return Ok(base_candidate("INVALID_INPUT", "EMPTY_SENSITIVITY_MATRIX")); }
    let m = raw_j.len();
    let p = raw_j[0].as_array().map(Vec::len).unwrap_or(0);
    if p == 0 { return Ok(base_candidate("INVALID_INPUT", "SENSITIVITY_MATRIX_HAS_NO_PARAMETERS")); }
    let jacobian = parse_matrix(candidate, "sensitivity_matrix", m, p)?;
    let variances = parse_vector(candidate, "observation_variance", m)?;
    let f = match fisher_information(&jacobian, &variances) {
        Ok(value) => value,
        Err(error) => return Ok(base_candidate("INVALID_COVARIANCE_OR_SENSITIVITY", error.as_str()))
    };
    let eigen = symmetric_eigenvalues(f.clone())?;
    let largest = eigen.first().copied().unwrap_or(0.0);
    if largest <= 0.0 { return Ok(base_candidate("UNIDENTIFIABLE_DESIGN", "FISHER_INFORMATION_HAS_NO_IDENTIFIABLE_DIRECTION")); }
    let singular = eigen.iter().map(|x| x.sqrt()).collect::<Vector>();
    let rank_tol = number(constraints, "rank_relative_tolerance")?;
    if rank_tol <= 0.0 || rank_tol >= 0.1 { return Err("rank tolerance outside valid range".into()); }
    let rank = singular.iter().filter(|s| **s > singular[0] * rank_tol).count();
    if rank < p {
        return Ok(json!({
            "candidate_disposition": "UNIDENTIFIABLE_DESIGN", "reason": "SENSOR_FILTERED_FISHER_INFORMATION_RANK_DEFICIENT",
            "observation_count": m, "parameter_count": p, "design_rank": rank,
            "singular_values": singular, "minimum_singular_value": singular.last(),
            "condition_number": null, "fisher_log_determinant": null,
            "fisher_trace": (0..p).map(|i| f[i][i]).sum::<f64>(),
            "total_dose": dose, "duration_seconds": duration,
            "information_per_cost": null, "selected": false
        }));
    }
    let smallest = singular.last().copied().unwrap_or(0.0);
    let condition = singular[0] / smallest;
    if condition > number(constraints, "maximum_condition_number")? {
        return Ok(json!({
            "candidate_disposition": "ILL_CONDITIONED_DESIGN", "reason": "FISHER_CONDITION_EXCEEDS_LIMIT",
            "observation_count": m, "parameter_count": p, "design_rank": rank,
            "singular_values": singular, "minimum_singular_value": smallest,
            "condition_number": condition, "fisher_log_determinant": null,
            "fisher_trace": (0..p).map(|i| f[i][i]).sum::<f64>(),
            "total_dose": dose, "duration_seconds": duration,
            "information_per_cost": null, "selected": false
        }));
    }
    let logdet = eigen.iter().map(|v| v.ln()).sum::<f64>();
    if !logdet.is_finite() { return Ok(base_candidate("UNIDENTIFIABLE_DESIGN", "FISHER_LOG_DETERMINANT_NOT_FINITE")); }
    let trace = (0..p).map(|i| f[i][i]).sum::<f64>();
    let cost = dose + duration;
    Ok(json!({
        "candidate_disposition": "FEASIBLE_IDENTIFIABLE",
        "reason": null, "observation_count": m, "parameter_count": p,
        "design_rank": rank, "singular_values": singular,
        "minimum_singular_value": smallest, "condition_number": condition,
        "fisher_log_determinant": logdet, "fisher_trace": trace,
        "total_dose": dose, "duration_seconds": duration,
        "information_per_cost": logdet / cost, "selected": false
    }))
}
fn compute_case(case: &Value) -> Result<Value, String> {
    let constraints = field(case, "constraints")?;
    let candidates = array(case, "candidates")?;
    if candidates.is_empty() { return Ok(base_case("NO_CANDIDATES", "NO_EXCITATION_CANDIDATES_SUPPLIED", Vec::new())); }
    let mut identities = BTreeSet::new();
    let mut rows = Vec::new();
    for candidate in candidates {
        let id = text(candidate, "candidate_id")?;
        if !identities.insert(id.to_owned()) {
            return Ok(base_case("REJECTED_CONTRACT", "DUPLICATE_CANDIDATE_ID", Vec::new()));
        }
        let mut result = evaluate_candidate(candidate, constraints)?;
        result["candidate_id"] = json!(id);
        rows.push(result);
    }
    let mut feasible: Vec<usize> = rows.iter().enumerate()
        .filter(|(_, row)| row["candidate_disposition"] == "FEASIBLE_IDENTIFIABLE")
        .map(|(i, _)| i).collect();
    if feasible.is_empty() {
        let any_rank_deficient = rows.iter().any(|row| row["candidate_disposition"] == "UNIDENTIFIABLE_DESIGN" || row["candidate_disposition"] == "ILL_CONDITIONED_DESIGN");
        let state = if any_rank_deficient { "UNIDENTIFIABLE_DESIGN" } else { "NO_FEASIBLE_DESIGN" };
        return Ok(base_case(state, "NO_CANDIDATE_PASSES_THE_SENSOR_AND_PHYSICAL_DESIGN_GATES", rows));
    }
    feasible.sort_by(|a, b| {
        let la = rows[*a]["fisher_log_determinant"].as_f64().unwrap_or(f64::NEG_INFINITY);
        let lb = rows[*b]["fisher_log_determinant"].as_f64().unwrap_or(f64::NEG_INFINITY);
        let ca = rows[*a]["total_dose"].as_f64().unwrap_or(f64::INFINITY)
            + rows[*a]["duration_seconds"].as_f64().unwrap_or(f64::INFINITY);
        let cb = rows[*b]["total_dose"].as_f64().unwrap_or(f64::INFINITY)
            + rows[*b]["duration_seconds"].as_f64().unwrap_or(f64::INFINITY);
        lb.total_cmp(&la).then_with(|| ca.total_cmp(&cb))
    });
    let selected = feasible[0];
    rows[selected]["selected"] = json!(true);
    let selected_id = rows[selected]["candidate_id"].clone();
    rows.sort_by(|a, b| a["candidate_id"].as_str().cmp(&b["candidate_id"].as_str()));
    Ok(json!({
        "design_disposition": "SELECTED_SENSOR_AWARE_D_OPTIMAL_DESIGN",
        "reason": null, "selected_candidate_id": selected_id,
        "candidate_results": rows,
        "selection_rule": "max_fisher_log_determinant_then_minimum_dose_plus_duration",
        "physical_operator_qualified": false, "physical_experiment_performed": false,
        "potability_claim_permitted": false, "release_authorized": false
    }))
}
fn ids(rows: &[Value], key: &str) -> Result<BTreeSet<String>, String> {
    rows.iter().map(|x| text(x, key).map(str::to_owned)).collect()
}
fn validate(input: &Value, oracle: &Value, manifest: &Value) -> Result<(), String> {
    if text(input, "schema_version")? != "active-experiment-design-t0-input-v0.1" { return Err("unexpected input schema".into()); }
    if text(oracle, "schema_version")? != "active-experiment-design-t0-oracle-v0.1" { return Err("unexpected oracle schema".into()); }
    for item in [input, oracle, manifest] {
        if text(item, "programme_id")? != PROGRAMME { return Err("programme identity mismatch".into()); }
    }
    for key in ["physical_operator_qualified", "physical_experiment_performed", "potability_claim_permitted", "release_authorized"] {
        if field(oracle, key)?.as_bool() != Some(false) { return Err(format!("oracle cannot claim {key} true")); }
    }
    let cases = array(input, "cases")?; let expected = array(oracle, "cases")?; let manifest_ids = array(manifest, "case_ids")?;
    if cases.len() != CASE_COUNT || expected.len() != CASE_COUNT || usize_value(manifest, "case_count")? != CASE_COUNT {
        return Err("frozen active-design case count mismatch".into());
    }
    let a = ids(cases, "case_id")?; let b = ids(expected, "case_id")?;
    let c = manifest_ids.iter().map(|x| x.as_str().map(str::to_owned).ok_or_else(|| "manifest case ID must be text".to_owned())).collect::<Result<BTreeSet<_>, _>>()?;
    if a.len() != cases.len() || b.len() != expected.len() || c.len() != manifest_ids.len() || a != b || a != c {
        return Err("input/oracle/manifest identity mismatch".into());
    }
    Ok(())
}
fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => matches!((x.as_f64(), y.as_f64()), (Some(p), Some(q)) if (p-q).abs() <= 1.0e-8 * p.abs().max(q.abs()).max(1.0)),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(a,b)| close(a,b)),
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k,v)| y.get(k).is_some_and(|w| close(v,w))),
        _ => a == b
    }
}
pub fn evaluate(input: &Value, oracle: &Value, manifest: &Value) -> Result<Value, String> {
    validate(input, oracle, manifest)?;
    let mut expected = BTreeMap::new();
    for item in array(oracle, "cases")? { expected.insert(text(item, "case_id")?.to_owned(), field(item, "expected")?.clone()); }
    let mut rows = Vec::new(); let mut mismatches = Vec::new();
    for case in array(input, "cases")? {
        let id = text(case, "case_id")?;
        let actual = compute_case(field(case, "inputs")?)?;
        let exp = expected.get(id).ok_or_else(|| format!("oracle missing {id}"))?;
        if !close(&actual, exp) { mismatches.push(json!({"case_id": id, "expected": exp, "actual": actual})); }
        rows.push(json!({"case_id": id, "matched": close(&actual, exp), "result": actual}));
    }
    rows.sort_by(|a,b| a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    let matched = rows.iter().filter(|row| row["matched"] == true).count();
    Ok(json!({
        "schema_version": "active-experiment-design-t0-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() { "MatchedFrozenOracle" } else { "OracleMismatch" },
        "evidence_class": "synthetic_only", "evidence_ceiling": EVIDENCE_CEILING,
        "case_count": rows.len(), "matched_case_count": matched, "mismatches": mismatches,
        "results": rows,
        "nonclaims": [
            "This evaluator selects among supplied synthetic sensor-filtered sensitivity matrices; it does not operate a physical tracer rig.",
            "Fisher information is conditional on the supplied model sensitivities and diagonal covariance; it does not prove model correctness.",
            "A feasible design is not safe or authorized until facility-specific limits and competent approvals are supplied.",
            "No result qualifies a physical operator, sensor, tracer, potable system or release authority."
        ]
    }))
}
fn digest(bytes: &[u8]) -> String { Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect() }
fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let value = serde_json::from_slice(&bytes).map_err(|e| format!("parse {}: {e}", path.display()))?;
    Ok((bytes, value))
}
fn valid_revision(value: &str) -> bool { value.len() == 40 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }
fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 4 { return Err("usage: active-experiment-design-t0 [INPUT] [ORACLE] [MANIFEST] [RECEIPT]".into()); }
    let input_path = PathBuf::from(args.first().map(String::as_str).unwrap_or("research/water-intrusion-coupling/active-experiment-design-t0-cases-v0.1.json"));
    let oracle_path = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("research/water-intrusion-coupling/active-experiment-design-t0-oracle-v0.1.json"));
    let manifest_path = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("research/water-intrusion-coupling/active-experiment-design-t0-manifest-v0.1.json"));
    let receipt_path = PathBuf::from(args.get(3).map(String::as_str).unwrap_or("artifacts/water-mixing-active-design-t0/receipt.json"));
    let (input_bytes, input) = read_json(&input_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let evaluation = evaluate(&input, &oracle, &manifest)?;
    let success = evaluation["verification_state"] == "MatchedFrozenOracle";
    let revision = env::var("WATER_MIXING_ACTIVE_DESIGN_T0_REVISION").or_else(|_| env::var("GITHUB_SHA")).ok().filter(|x| valid_revision(x));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes = fs::read(&lock_path).map_err(|e| format!("read {}: {e}", lock_path.display()))?;
    let receipt = json!({
        "schema_version": "active-experiment-design-t0-run-v0.1",
        "verification_state": if !success { "OracleMismatch" } else if revision.is_some() { "PassedExactRevision" } else { "PassedLocalUnpinned" },
        "implementation_revision": revision, "input_sha256": digest(&input_bytes),
        "oracle_sha256": digest(&oracle_bytes), "manifest_sha256": digest(&manifest_bytes),
        "cargo_lock_sha256": digest(&lock_bytes), "case_count": evaluation["case_count"],
        "matched_case_count": evaluation["matched_case_count"], "mismatches": evaluation["mismatches"],
        "results": evaluation["results"], "evidence_class": "synthetic_only",
        "evidence_ceiling": EVIDENCE_CEILING, "physical_operator_qualified": false,
        "physical_experiment_performed": false, "potability_claim_permitted": false, "release_authorized": false
    });
    if let Some(parent) = receipt_path.parent() {
        if !parent.as_os_str().is_empty() { fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?; }
    }
    let output = serde_json::to_string_pretty(&receipt).map_err(|e| format!("serialize receipt: {e}"))? + "\n";
    fs::write(&receipt_path, output.as_bytes()).map_err(|e| format!("write {}: {e}", receipt_path.display()))?;
    println!("{output}");
    Ok(if success { 0 } else { 2 })
}
fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => { eprintln!("WATER_MIXING_ACTIVE_DESIGN_T0_ERROR: {error}"); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn load(name: &str) -> Value {
        let raw = match name {
            "input" => include_str!("../../../../research/water-intrusion-coupling/active-experiment-design-t0-cases-v0.1.json"),
            "oracle" => include_str!("../../../../research/water-intrusion-coupling/active-experiment-design-t0-oracle-v0.1.json"),
            "manifest" => include_str!("../../../../research/water-intrusion-coupling/active-experiment-design-t0-manifest-v0.1.json"),
            _ => panic!("unknown fixture")
        };
        serde_json::from_str(raw).expect("active design fixture parses")
    }
    fn result() -> Value { evaluate(&load("input"), &load("oracle"), &load("manifest")).expect("active design T0 evaluates") }
    fn row(id: &str) -> Value {
        result()["results"].as_array().unwrap().iter()
            .find(|x| x["case_id"] == id).unwrap()["result"].clone()
    }
    #[test]
    fn all_eight_active_design_cases_match_frozen_oracle() {
        let value = result();
        assert_eq!(value["case_count"], CASE_COUNT);
        assert_eq!(value["matched_case_count"], CASE_COUNT, "oracle mismatches: {}", value["mismatches"]);
        assert_eq!(value["verification_state"], "MatchedFrozenOracle");
    }
    #[test]
    fn low_dose_orthogonal_design_beats_collinear_candidate() {
        let value = row("AED-T0-001-orthogonal-versus-collinear");
        assert_eq!(value["design_disposition"], "SELECTED_SENSOR_AWARE_D_OPTIMAL_DESIGN");
        assert_eq!(value["selected_candidate_id"], "orthogonal_low_dose");
        assert_eq!(value["candidate_results"].as_array().unwrap().iter().find(|x| x["candidate_id"] == "collinear_high_dose").unwrap()["candidate_disposition"], "UNIDENTIFIABLE_DESIGN");
    }
    #[test]
    fn rank_deficient_only_candidates_are_not_identifiable() {
        assert_eq!(row("AED-T0-002-rank-deficient-only")["design_disposition"], "UNIDENTIFIABLE_DESIGN");
    }
    #[test]
    fn sensor_bandwidth_refuses_unresolvable_delay() {
        assert_eq!(row("AED-T0-003-sensor-bandwidth-limit")["candidate_results"][0]["candidate_disposition"], "TEMPORAL_RESOLUTION_INADEQUATE");
    }
    #[test]
    fn missing_outlet_channel_is_not_scored() {
        assert_eq!(row("AED-T0-004-missing-outlet-channel")["design_disposition"], "NO_FEASIBLE_DESIGN");
    }
    #[test]
    fn dose_budget_selects_only_feasible_excitation() {
        let value = row("AED-T0-005-dose-duration-budget");
        assert_eq!(value["selected_candidate_id"], "low_dose");
        assert_eq!(value["candidate_results"].as_array().unwrap().iter().find(|x| x["candidate_id"] == "high_dose").unwrap()["candidate_disposition"], "DOSE_LIMIT_EXCEEDED");
    }
    #[test]
    fn regime_transition_invalidates_fixed_regime_design() {
        assert_eq!(row("AED-T0-006-regime-transition")["design_disposition"], "NO_FEASIBLE_DESIGN");
    }
    #[test]
    fn unobserved_flow_reversal_abstains() {
        assert_eq!(row("AED-T0-007-flow-reversal-unobserved")["candidate_results"][0]["candidate_disposition"], "ABSTAIN_FLOW_DIRECTION_UNOBSERVED");
    }
    #[test]
    fn invalid_measurement_variance_is_refused() {
        assert_eq!(row("AED-T0-008-invalid-variance")["candidate_results"][0]["candidate_disposition"], "INVALID_COVARIANCE_OR_SENSITIVITY");
    }
}
