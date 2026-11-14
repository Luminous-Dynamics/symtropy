// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, path::{Path, PathBuf}};

const PROGRAMME: &str = "WATER-MIXING-MASS-ACCOUNTING-QUAL-001";
const CASE_COUNT: usize = 18;
const EVIDENCE_CEILING: &str = "synthetic tracer mass accounting arithmetic only";

type Matrix = Vec<Vec<f64>>;

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing field: {key}"))
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?.as_str().ok_or_else(|| format!("{key} must be text"))
}
fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?.as_f64().filter(|v| v.is_finite())
        .ok_or_else(|| format!("{key} must be a finite number"))
}
fn usize_value(value: &Value, key: &str) -> Result<usize, String> {
    field(value, key)?.as_u64().and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    field(value, key)?.as_array().ok_or_else(|| format!("{key} must be an array"))
}
fn base(disposition: &str, reason: Option<&str>) -> Value {
    json!({
        "mass_accounting_disposition": disposition,
        "reason": reason,
        "observed_outlet_mass": null,
        "delayed_recovery_mass": null,
        "known_inventory_mass": null,
        "unexplained_mass_residual": null,
        "residual_standard_uncertainty": null,
        "standardized_abs_residual": null,
        "measured_recovery_fraction": null,
        "accounted_mass_fraction": null,
        "residual_interval_lower": null,
        "residual_interval_upper": null,
        "physical_mixing_qualified": false,
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}
fn disposition(state: &str, reason: Option<&str>) -> Value {
    base(state, reason)
}
fn covariance_cholesky(a: &Matrix) -> Result<Matrix, String> {
    let n = a.len();
    if n == 0 || a.iter().any(|row| row.len() != n) {
        return Err("mass covariance must be non-empty and square".into());
    }
    let scale = a.iter().enumerate().map(|(i,row)| row[i].abs()).fold(0.0_f64, f64::max);
    if scale <= 0.0 { return Err("mass covariance has no positive variance scale".into()); }
    for i in 0..n {
        for j in 0..n {
            if !a[i][j].is_finite() {
                return Err("mass covariance contains a non-finite value".into());
            }
            if (a[i][j] - a[j][i]).abs() > 1.0e-10 * scale {
                return Err("mass covariance is not symmetric".into());
            }
        }
    }
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = a[i][j];
            for k in 0..j { sum -= lower[i][k] * lower[j][k]; }
            if i == j {
                if !sum.is_finite() || sum <= 1.0e-12 * scale {
                    return Err("mass covariance is not positive definite at declared tolerance".into());
                }
                lower[i][j] = sum.sqrt();
            } else {
                lower[i][j] = sum / lower[j][j];
            }
        }
    }
    Ok(lower)
}
fn residual_variance(covariance: &Matrix) -> Result<f64, String> {
    let _ = covariance_cholesky(covariance)?;
    let n = covariance.len();
    let mut variance = 0.0;
    for i in 0..n {
        let ci = if i == 0 { 1.0 } else { -1.0 };
        for j in 0..n {
            let cj = if j == 0 { 1.0 } else { -1.0 };
            variance += ci * covariance[i][j] * cj;
        }
    }
    if !variance.is_finite() || variance <= 0.0 {
        return Err("residual uncertainty is not finite and positive".into());
    }
    Ok(variance)
}
fn compute(inputs: &Value) -> Result<Value, String> {
    let mass_unit = text(inputs, "mass_unit")?;
    let injected = number(inputs, "injected_mass")?;
    if injected <= 0.0 { return Ok(disposition("INVALID_INPUT", Some("INJECTED_MASS_NOT_POSITIVE"))); }
    let expected_outlets = usize_value(inputs, "expected_outlet_count")?;
    if expected_outlets == 0 { return Ok(disposition("INVALID_INPUT", Some("EXPECTED_OUTLET_COUNT_ZERO"))); }
    let all_outlets_observed = field(inputs, "all_outlets_observed")?.as_bool()
        .ok_or_else(|| "all_outlets_observed must be boolean".to_owned())?;
    let temporal_state = text(inputs, "temporal_boundary_state")?;
    if !["complete", "truncated"].contains(&temporal_state) {
        return Ok(disposition("REJECTED_CONTRACT", Some("UNKNOWN_TEMPORAL_BOUNDARY_STATE")));
    }
    let calibration_state = text(inputs, "calibration_state")?;
    let provenance_state = text(inputs, "provenance_state")?;

    let streams = array(inputs, "recovery_streams")?;
    let mut stream_ids = BTreeSet::new();
    let mut outlet_ids = BTreeSet::new();
    let mut outlet_mass = 0.0;
    let mut delayed_mass = 0.0;
    let mut covariance_dimension = 1usize;
    for stream in streams {
        let stream_id = text(stream, "stream_id")?;
        if !stream_ids.insert(stream_id.to_owned()) {
            return Ok(disposition("REJECTED_CONTRACT", Some("DUPLICATE_RECOVERY_STREAM_ID")));
        }
        if text(stream, "mass_unit")? != mass_unit {
            return Ok(disposition("INDETERMINATE", Some("MASS_UNIT_MISMATCH")));
        }
        let mass = number(stream, "mass")?;
        if mass < 0.0 {
            return Ok(disposition("INVALID_INPUT", Some("NEGATIVE_RECOVERY_MASS")));
        }
        match text(stream, "stream_kind")? {
            "outlet" => {
                let outlet_id = text(stream, "outlet_id")?;
                if !outlet_ids.insert(outlet_id.to_owned()) {
                    return Ok(disposition("REJECTED_CONTRACT", Some("DUPLICATE_OUTLET_ID")));
                }
                outlet_mass += mass;
            }
            "delayed_tail" => delayed_mass += mass,
            _ => return Ok(disposition("REJECTED_CONTRACT", Some("UNKNOWN_RECOVERY_STREAM_KIND"))),
        }
        covariance_dimension += 1;
    }
    if !outlet_mass.is_finite() || !delayed_mass.is_finite() {
        return Ok(disposition("INVALID_INPUT", Some("RECOVERY_MASS_SUM_OVERFLOW")));
    }
    if all_outlets_observed && outlet_ids.len() != expected_outlets {
        return Ok(disposition("REJECTED_CONTRACT", Some("DECLARED_COMPLETE_OUTLET_SET_MISMATCH")));
    }
    if outlet_ids.len() > expected_outlets {
        return Ok(disposition("REJECTED_CONTRACT", Some("OBSERVED_OUTLET_COUNT_EXCEEDS_DECLARED")));
    }

    let inventory = field(inputs, "inventory")?;
    let inventory_state = text(inventory, "state")?;
    let mut inventory_mass = 0.0;
    let mut inventory_issue: Option<&str> = None;
    match inventory_state {
        "measured" => {
            match text(inventory, "kind")? {
                "physical_inventory" => {
                    if text(inventory, "mass_unit")? != mass_unit {
                        return Ok(disposition("INDETERMINATE", Some("INVENTORY_UNIT_MISMATCH")));
                    }
                    inventory_mass = number(inventory, "mass")?;
                    if inventory_mass < 0.0 {
                        return Ok(disposition("INVALID_INPUT", Some("NEGATIVE_PHYSICAL_INVENTORY")));
                    }
                    covariance_dimension += 1;
                }
                "measurement_system_loss" => {
                    inventory_issue = Some("MEASUREMENT_SYSTEM_LOSS_IS_NOT_PHYSICAL_INVENTORY");
                }
                _ => {
                    inventory_issue = Some("INVENTORY_CLASSIFICATION_UNSUPPORTED");
                }
            }
        }
        "not_measured" => {
            if text(inventory, "kind")? == "measurement_system_loss" {
                inventory_issue = Some("MEASUREMENT_SYSTEM_LOSS_IS_NOT_PHYSICAL_INVENTORY");
            } else if text(inventory, "kind")? != "none" {
                inventory_issue = Some("UNVERIFIED_INVENTORY_CLASSIFICATION");
            }
        }
        _ => return Ok(disposition("REJECTED_CONTRACT", Some("UNKNOWN_INVENTORY_MEASUREMENT_STATE"))),
    }

    let covariance_rows = array(inputs, "mass_balance_covariance")?;
    let n = covariance_dimension;
    if covariance_rows.len() != n {
        return Ok(disposition("REJECTED_CONTRACT", Some("MASS_COVARIANCE_DIMENSION_MISMATCH")));
    }
    let mut covariance = Vec::with_capacity(n);
    for row in covariance_rows {
        let values = row.as_array().ok_or_else(|| "covariance row must be an array".to_owned())?;
        if values.len() != n {
            return Ok(disposition("REJECTED_CONTRACT", Some("MASS_COVARIANCE_DIMENSION_MISMATCH")));
        }
        covariance.push(values.iter().map(|v| v.as_f64().filter(|x| x.is_finite())
            .ok_or_else(|| "covariance contains non-finite/non-numeric value".to_owned()))
            .collect::<Result<Vec<_>,_>>()?);
    }
    let variance = match residual_variance(&covariance) {
        Ok(v) => v,
        Err(_) => return Ok(disposition("INVALID_INPUT", Some("MASS_COVARIANCE_NOT_POSITIVE_DEFINITE"))),
    };
    let sigma = variance.sqrt();
    let residual = injected - outlet_mass - delayed_mass - inventory_mass;
    let sigma_limit = number(inputs, "closure_sigma_limit")?;
    if sigma_limit <= 0.0 {
        return Ok(disposition("INVALID_INPUT", Some("CLOSURE_SIGMA_LIMIT_NOT_POSITIVE")));
    }
    let standardized = residual.abs() / sigma;
    let lower = residual - sigma_limit * sigma;
    let upper = residual + sigma_limit * sigma;
    let measured_recovery = (outlet_mass + delayed_mass) / injected;
    let accounted_fraction = (outlet_mass + delayed_mass + inventory_mass) / injected;
    if [residual, sigma, standardized, lower, upper, measured_recovery, accounted_fraction]
        .iter().any(|v| !v.is_finite()) {
        return Ok(disposition("INVALID_INPUT", Some("MASS_ACCOUNTING_RESULT_NON_FINITE")));
    }

    let (state, reason) = if let Some(issue) = inventory_issue {
        ("INDETERMINATE", Some(issue))
    } else if calibration_state != "calibrated" {
        ("INDETERMINATE", Some("MEASUREMENT_CALIBRATION_NOT_QUALIFIED"))
    } else if provenance_state != "bound" {
        ("INDETERMINATE", Some("MASS_STREAM_PROVENANCE_NOT_BOUND"))
    } else if !all_outlets_observed || temporal_state != "complete" {
        ("PARTIAL_BOUNDARY", Some("PHYSICAL_OR_TEMPORAL_OBSERVATION_BOUNDARY_INCOMPLETE"))
    } else if standardized > sigma_limit {
        ("OPEN_RESIDUAL", Some("SIGNED_RESIDUAL_EXCEEDS_DECLARED_UNCERTAINTY"))
    } else {
        ("CLOSED_WITHIN_DECLARED_UNCERTAINTY", None)
    };
    Ok(json!({
        "mass_accounting_disposition":state,
        "reason":reason,
        "observed_outlet_mass":outlet_mass,
        "delayed_recovery_mass":delayed_mass,
        "known_inventory_mass":inventory_mass,
        "unexplained_mass_residual":residual,
        "residual_standard_uncertainty":sigma,
        "standardized_abs_residual":standardized,
        "measured_recovery_fraction":measured_recovery,
        "accounted_mass_fraction":accounted_fraction,
        "residual_interval_lower":lower,
        "residual_interval_upper":upper,
        "physical_mixing_qualified":false,
        "potability_claim_permitted":false,
        "release_authorized":false
    }))
}
fn validate_contract(input: &Value, oracle: &Value, manifest: &Value) -> Result<(), String> {
    if text(input, "schema_version")? != "mixing-mass-t0-input-v0.1" {
        return Err("unexpected input schema".into());
    }
    if text(oracle, "schema_version")? != "mixing-mass-t0-oracle-v0.1" {
        return Err("unexpected oracle schema".into());
    }
    for value in [input, oracle, manifest] {
        if text(value, "programme_id")? != PROGRAMME {
            return Err("programme identity mismatch".into());
        }
    }
    for key in ["physical_mixing_qualified", "potability_claim_permitted", "release_authorized"] {
        if field(oracle, key)?.as_bool() != Some(false) {
            return Err(format!("oracle may not set {key} true"));
        }
    }
    let cases = array(input, "cases")?;
    let expected = array(oracle, "cases")?;
    let ids = array(manifest, "case_ids")?;
    if cases.len() != CASE_COUNT || expected.len() != CASE_COUNT
        || usize_value(manifest, "case_count")? != CASE_COUNT {
        return Err("frozen mass-accounting case count mismatch".into());
    }
    let mut input_ids = BTreeSet::new();
    for case in cases {
        if !input_ids.insert(text(case, "case_id")?.to_owned()) {
            return Err("duplicate input case id".into());
        }
    }
    let mut oracle_ids = BTreeSet::new();
    for case in expected {
        if !oracle_ids.insert(text(case, "case_id")?.to_owned()) {
            return Err("duplicate oracle case id".into());
        }
    }
    let mut manifest_ids = BTreeSet::new();
    for value in ids {
        let id = value.as_str().ok_or_else(|| "manifest id must be text".to_owned())?;
        if !manifest_ids.insert(id.to_owned()) { return Err("duplicate manifest case id".into()); }
    }
    if input_ids != oracle_ids || input_ids != manifest_ids {
        return Err("input/oracle/manifest case identity mismatch".into());
    }
    Ok(())
}
fn close(left: f64, right: f64) -> bool {
    (left-right).abs() <= 1.0e-8*left.abs().max(right.abs()).max(1.0)
}
fn compare_json(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => matches!((a.as_f64(),b.as_f64()),
            (Some(x),Some(y)) if close(x,y)),
        (Value::Array(a), Value::Array(b)) => a.len()==b.len() && a.iter().zip(b).all(|(x,y)|compare_json(x,y)),
        (Value::Object(a), Value::Object(b)) => a.len()==b.len() && a.iter().all(|(k,v)|b.get(k).is_some_and(|w|compare_json(v,w))),
        _ => actual==expected
    }
}
pub fn evaluate(input: &Value, oracle: &Value, manifest: &Value) -> Result<Value, String> {
    validate_contract(input, oracle, manifest)?;
    let expected = array(oracle, "cases")?;
    let mut expected_by_id = std::collections::BTreeMap::new();
    for row in expected {
        expected_by_id.insert(text(row,"case_id")?.to_owned(), field(row,"expected")?.clone());
    }
    let mut results = Vec::new();
    let mut mismatches = Vec::new();
    for case in array(input,"cases")? {
        let id = text(case,"case_id")?;
        let computed = compute(field(case,"inputs")?)?;
        let expected = expected_by_id.get(id).ok_or_else(||format!("oracle missing case {id}"))?;
        if !compare_json(&computed, expected) {
            mismatches.push(json!({"case_id":id,"expected":expected,"computed":computed}));
        }
        results.push(json!({"case_id":id,"matched":compare_json(&computed,expected),"computed":computed}));
    }
    results.sort_by(|a,b|a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    let matched = results.iter().filter(|r|r["matched"]==true).count();
    Ok(json!({
        "schema_version":"mixing-mass-t0-evaluation-v0.1",
        "verification_state":if mismatches.is_empty() {"MatchedFrozenOracle"} else {"OracleMismatch"},
        "evidence_class":"synthetic_only",
        "evidence_ceiling":EVIDENCE_CEILING,
        "case_count":results.len(),
        "matched_case_count":matched,
        "mismatches":mismatches,
        "results":results,
        "nonclaims":[
            "This evaluator checks synthetic mass-accounting arithmetic and refusal semantics only.",
            "It does not identify the physical cause of missing tracer mass.",
            "A closed synthetic balance does not qualify a physical observation boundary or instrument.",
            "No result establishes physical mixing, potability, contamination, toxicology, SANS 241 compliance or release authority."
        ]
    }))
}
fn sha256(bytes:&[u8])->String {
    Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect()
}
fn read_json(path:&Path)->Result<(Vec<u8>,Value),String> {
    let bytes=fs::read(path).map_err(|e|format!("read {}: {e}",path.display()))?;
    let value=serde_json::from_slice(&bytes).map_err(|e|format!("parse {}: {e}",path.display()))?;
    Ok((bytes,value))
}
fn valid_revision(value:&str)->bool {
    value.len()==40 && value.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))
}
fn run()->Result<i32,String> {
    let args:Vec<String>=env::args().skip(1).collect();
    if args.len()>4 {return Err("usage: mixing-mass-t0 [INPUTS] [ORACLE] [MANIFEST] [RECEIPT]".into());}
    let input_path=PathBuf::from(args.first().map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-mass-t0-cases-v0.1.json"));
    let oracle_path=PathBuf::from(args.get(1).map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-mass-t0-oracle-v0.1.json"));
    let manifest_path=PathBuf::from(args.get(2).map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-mass-t0-manifest-v0.1.json"));
    let receipt_path=PathBuf::from(args.get(3).map(String::as_str).unwrap_or("artifacts/water-mixing-mass-t0/receipt.json"));
    let (ib,iv)=read_json(&input_path)?; let (ob,ov)=read_json(&oracle_path)?; let (mb,mv)=read_json(&manifest_path)?;
    let evaluation=evaluate(&iv,&ov,&mv)?;
    let count=evaluation["case_count"].as_u64().unwrap_or(0);
    let matched=evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let success=count==CASE_COUNT as u64 && matched==CASE_COUNT as u64 && evaluation["mismatches"].as_array().is_some_and(Vec::is_empty);
    let revision=env::var("WATER_MIXING_MASS_T0_REVISION").or_else(|_|env::var("GITHUB_SHA")).ok().filter(|v|valid_revision(v));
    let lock_path=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock=fs::read(&lock_path).map_err(|e|format!("read {}: {e}",lock_path.display()))?;
    let receipt=json!({
        "schema_version":"mixing-mass-t0-run-v0.1",
        "verification_state":if !success {"OracleMismatch"} else if revision.is_some() {"PassedExactRevision"} else {"PassedLocalUnpinned"},
        "qualification_status":"NotRun",
        "implementation_revision":revision,
        "input_sha256":sha256(&ib),
        "oracle_sha256":sha256(&ob),
        "manifest_sha256":sha256(&mb),
        "cargo_lock_sha256":sha256(&lock),
        "case_count":count,
        "matched_case_count":matched,
        "mismatches":evaluation["mismatches"],
        "results":evaluation["results"],
        "evidence_class":"synthetic_only",
        "evidence_ceiling":EVIDENCE_CEILING,
        "physical_mixing_qualified":false,
        "potability_claim_permitted":false,
        "release_authorized":false
    });
    if let Some(parent)=receipt_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e|format!("create {}: {e}",parent.display()))?;
        }
    }
    let serialized=serde_json::to_string_pretty(&receipt).map_err(|e|format!("serialize receipt: {e}"))?+"\n";
    fs::write(&receipt_path,serialized.as_bytes()).map_err(|e|format!("write {}: {e}",receipt_path.display()))?;
    println!("{serialized}");
    Ok(if success {0}else{2})
}
fn main() {
    match run() {
        Ok(code)=>std::process::exit(code),
        Err(error)=>{eprintln!("WATER_MIXING_MASS_T0_ERROR: {error}");std::process::exit(1);}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn load(name:&str)->Value {
        let source=match name {
            "input"=>include_str!("../../../../research/water-intrusion-coupling/mixing-mass-t0-cases-v0.1.json"),
            "oracle"=>include_str!("../../../../research/water-intrusion-coupling/mixing-mass-t0-oracle-v0.1.json"),
            "manifest"=>include_str!("../../../../research/water-intrusion-coupling/mixing-mass-t0-manifest-v0.1.json"),
            _=>panic!("unknown fixture")
        };
        serde_json::from_str(source).expect("mass-accounting fixture must be valid JSON")
    }
    fn evaluated()->Value {
        evaluate(&load("input"),&load("oracle"),&load("manifest")).expect("mass-accounting evaluator runs")
    }
    fn row(id:&str)->Value {
        evaluated()["results"].as_array().unwrap().iter()
            .find(|r|r["case_id"]==id).unwrap()["computed"].clone()
    }
    #[test] fn all_eighteen_mass_accounting_cases_match_the_frozen_oracle() {
        let result=evaluated();
        assert_eq!(result["case_count"],CASE_COUNT);
        assert_eq!(result["matched_case_count"],CASE_COUNT);
        assert_eq!(result["verification_state"],"MatchedFrozenOracle");
        assert!(result["mismatches"].as_array().unwrap().is_empty());
    }
    #[test] fn partial_recovery_is_not_renormalized() {
        let result=row("MMA-T0-002-truncated-window-forty-percent-unresolved");
        assert_eq!(result["mass_accounting_disposition"],"PARTIAL_BOUNDARY");
        assert_eq!(result["measured_recovery_fraction"],0.6);
        assert_eq!(result["accounted_mass_fraction"],0.6);
        assert_eq!(result["unexplained_mass_residual"],40.0);
    }
    #[test] fn independently_measured_physical_inventory_is_counted_once() {
        let result=row("MMA-T0-003-measured-physical-inventory-closes");
        assert_eq!(result["mass_accounting_disposition"],"CLOSED_WITHIN_DECLARED_UNCERTAINTY");
        assert_eq!(result["known_inventory_mass"],40.0);
        assert_eq!(result["accounted_mass_fraction"],1.0);
    }
    #[test] fn delayed_tail_remains_a_separate_recovery_stream() {
        let result=row("MMA-T0-004-delayed-tail-recovery");
        assert_eq!(result["delayed_recovery_mass"],30.0);
        assert_eq!(result["observed_outlet_mass"],70.0);
        assert_eq!(result["mass_accounting_disposition"],"CLOSED_WITHIN_DECLARED_UNCERTAINTY");
    }
    #[test] fn missing_outlet_forces_partial_boundary() {
        assert_eq!(row("MMA-T0-005-missing-outlet").as_object().unwrap()["mass_accounting_disposition"],"PARTIAL_BOUNDARY");
    }
    #[test] fn sample_line_loss_cannot_be_relabelled_as_physical_inventory() {
        let result=row("MMA-T0-006-sample-line-loss-is-not-inventory");
        assert_eq!(result["mass_accounting_disposition"],"INDETERMINATE");
        assert_eq!(result["known_inventory_mass"],0.0);
    }
    #[test] fn correlated_covariance_is_used_in_residual_uncertainty() {
        let result=row("MMA-T0-007-correlated-measurement-covariance");
        assert!((result["residual_standard_uncertainty"].as_f64().unwrap()-2.0_f64.sqrt()).abs()<1.0e-10);
    }
    #[test] fn open_residual_is_reported_with_its_signed_value() {
        let result=row("MMA-T0-008-open-positive-residual");
        assert_eq!(result["mass_accounting_disposition"],"OPEN_RESIDUAL");
        assert_eq!(result["unexplained_mass_residual"],10.0);
    }
    #[test] fn within_uncertainty_residual_is_not_clamped_to_zero() {
        let result=row("MMA-T0-009-residual-within-uncertainty");
        assert_eq!(result["unexplained_mass_residual"],0.5);
        assert_eq!(result["mass_accounting_disposition"],"CLOSED_WITHIN_DECLARED_UNCERTAINTY");
    }
    #[test] fn negative_residual_is_preserved() {
        let result=row("MMA-T0-018-over_recovery-negative-residual");
        assert_eq!(result["unexplained_mass_residual"],-5.0);
        assert_eq!(result["mass_accounting_disposition"],"OPEN_RESIDUAL");
    }
    #[test] fn invalid_covariance_never_falls_back_to_diagonal_noise() {
        let result=row("MMA-T0-016-non_positive_definite_covariance");
        assert_eq!(result["mass_accounting_disposition"],"INVALID_INPUT");
    }
    #[test] fn unit_mismatch_never_produces_a_closure_claim() {
        assert_eq!(row("MMA-T0-010-unit-mismatch")["mass_accounting_disposition"],"INDETERMINATE");
    }
    #[test] fn uncalibrated_or_unbound_streams_cannot_close_the_balance() {
        assert_eq!(row("MMA-T0-011-uncalibrated-measurement")["mass_accounting_disposition"],"INDETERMINATE");
        assert_eq!(row("MMA-T0-012-unbound-provenance")["mass_accounting_disposition"],"INDETERMINATE");
    }
    #[test] fn common_positive_mass_scale_preserves_disposition_and_fractions() {
        let a=row("MMA-T0-001-full-boundary-exact-closure");
        let b=row("MMA-T0-013-positive-scale-metamorphic-control");
        assert_eq!(a["mass_accounting_disposition"],b["mass_accounting_disposition"]);
        assert!((a["measured_recovery_fraction"].as_f64().unwrap()-b["measured_recovery_fraction"].as_f64().unwrap()).abs()<1.0e-12);
        assert!((10.0*a["residual_standard_uncertainty"].as_f64().unwrap()-b["residual_standard_uncertainty"].as_f64().unwrap()).abs()<1.0e-10);
    }
    #[test] fn negative_mass_and_duplicate_identity_are_refused() {
        assert_eq!(row("MMA-T0-014-negative-recovery-mass")["mass_accounting_disposition"],"INVALID_INPUT");
        assert_eq!(row("MMA-T0-015-duplicate-stream-identity")["mass_accounting_disposition"],"REJECTED_CONTRACT");
    }
    #[test] fn declared_complete_outlet_set_must_match_observed_outlet_ids() {
        assert_eq!(row("MMA-T0-017-declared-outlet-set-contradiction")["mass_accounting_disposition"],"REJECTED_CONTRACT");
    }
}
