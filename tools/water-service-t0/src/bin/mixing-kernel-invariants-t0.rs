// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

const PROGRAMME: &str = "WATER-MIXING-PHYSICAL-OPERATOR-INVARIANTS-001";
const CASE_COUNT: usize = 10;
const EVIDENCE_CEILING: &str = "synthetic physical-operator invariant arithmetic only";

type Matrix = Vec<Vec<f64>>;
type Tensor3 = Vec<Vec<Vec<f64>>>;

fn field<'a>(v: &'a Value, k: &str) -> Result<&'a Value, String> {
    v.get(k).ok_or_else(|| format!("missing field: {k}"))
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    field(v, k)?
        .as_str()
        .ok_or_else(|| format!("{k} must be text"))
}
fn number(v: &Value, k: &str) -> Result<f64, String> {
    field(v, k)?
        .as_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| format!("{k} must be finite numeric"))
}
fn array<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>, String> {
    field(v, k)?
        .as_array()
        .ok_or_else(|| format!("{k} must be array"))
}
fn parse_matrix(v: &Value, key: &str, rows: usize, cols: usize) -> Result<Matrix, String> {
    let raw = array(v, key)?;
    if raw.len() != rows {
        return Err(format!("{key} row count mismatch"));
    }
    let mut out = Vec::new();
    for row in raw {
        let values = row
            .as_array()
            .ok_or_else(|| format!("{key} row must be array"))?;
        if values.len() != cols {
            return Err(format!("{key} column count mismatch"));
        }
        out.push(
            values
                .iter()
                .map(|x| {
                    x.as_f64()
                        .filter(|n| n.is_finite())
                        .ok_or_else(|| format!("{key} contains non-finite value"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    Ok(out)
}
fn parse_tensor(v: &Value, key: &str, nout: usize, nin: usize) -> Result<Tensor3, String> {
    let outs = array(v, key)?;
    if outs.len() != nout {
        return Err(format!("{key} outlet count mismatch"));
    }
    let mut lag_count = None;
    let mut result = Vec::new();
    for out in outs {
        let ins = out
            .as_array()
            .ok_or_else(|| format!("{key} outlet must be array"))?;
        if ins.len() != nin {
            return Err(format!("{key} inlet count mismatch"));
        }
        let mut row = Vec::new();
        for kernel in ins {
            let values = kernel
                .as_array()
                .ok_or_else(|| format!("{key} kernel must be array"))?;
            if values.is_empty() {
                return Err(format!("{key} kernel cannot be empty"));
            }
            if lag_count.is_some_and(|n| n != values.len()) {
                return Err(format!("{key} lag length mismatch"));
            }
            lag_count = Some(values.len());
            row.push(
                values
                    .iter()
                    .map(|x| {
                        x.as_f64()
                            .filter(|n| n.is_finite())
                            .ok_or_else(|| format!("{key} non-finite coefficient"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        result.push(row);
    }
    Ok(result)
}
#[allow(clippy::needless_range_loop)]
fn conv(input: &Matrix, kernel: &Tensor3) -> Result<Matrix, String> {
    let nt = input.first().map(Vec::len).unwrap_or(0);
    let nin = input.len();
    let nout = kernel.len();
    if nt == 0 || nin == 0 || nout == 0 || kernel.iter().any(|r| r.len() != nin) {
        return Err("convolution dimensions invalid".into());
    }
    let lag = kernel[0][0].len();
    let mut out = vec![vec![0.0; nt + lag - 1]; nout];
    for j in 0..nout {
        for i in 0..nin {
            for t in 0..nt {
                for l in 0..lag {
                    out[j][t + l] += kernel[j][i][l] * input[i][t];
                }
            }
        }
    }
    if out.iter().flatten().any(|x| !x.is_finite()) {
        return Err("convolution overflow".into());
    }
    Ok(out)
}
fn base(state: &str, reason: &str) -> Value {
    json!({
        "invariant_disposition":state,"reason":reason,
        "kernel_area_by_inlet":null,"steady_gain_max_abs_error":null,
        "superposition_max_abs_error":null,"time_shift_max_abs_error":null,
        "port_permutation_max_abs_error":null,"scale_max_abs_error":null,
        "grid_area_max_abs_error":null,"grid_centroid_max_abs_error_seconds":null,
        "state_space_disposition":"NOT_SUPPLIED",
        "positive_output_minimum":null,"predicted_total_output_mass":null,
        "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
    })
}
#[allow(clippy::needless_range_loop)]
fn check_state_space(case: &Value, tol: f64) -> Result<String, String> {
    let Some(ss) = case.get("state_space") else {
        return Ok("NOT_SUPPLIED".into());
    };
    let a_raw = array(ss, "A")?;
    let n = a_raw.len();
    if n == 0 {
        return Ok("INVALID_STATE_SPACE_DIMENSION".into());
    }
    let a = parse_matrix(ss, "A", n, n)?;
    for i in 0..n {
        for j in 0..n {
            if i != j && a[i][j] < -tol {
                return Ok("INVALID_STATE_SPACE_NOT_METZLER".into());
            }
        }
    }
    let b_rows = array(ss, "B")?;
    let nb = b_rows
        .first()
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if nb == 0 {
        return Ok("INVALID_STATE_SPACE_DIMENSION".into());
    }
    let b = parse_matrix(ss, "B", n, nb)?;
    let c_rows = array(ss, "C")?;
    let no = c_rows.len();
    if no == 0 {
        return Ok("INVALID_STATE_SPACE_DIMENSION".into());
    }
    let c = parse_matrix(ss, "C", no, n)?;
    let d = parse_matrix(ss, "D", no, nb)?;
    if b.iter()
        .flatten()
        .chain(c.iter().flatten())
        .chain(d.iter().flatten())
        .any(|x| *x < -tol)
    {
        return Ok("INVALID_STATE_SPACE_NEGATIVE_COEFFICIENT".into());
    }
    let v_raw = field(ss, "stability_witness")?
        .as_array()
        .ok_or_else(|| "stability_witness must be array".to_owned())?;
    if v_raw.len() != n {
        return Ok("INVALID_STATE_SPACE_DIMENSION".into());
    }
    let v = v_raw
        .iter()
        .map(|x| {
            x.as_f64()
                .filter(|n| n.is_finite())
                .ok_or_else(|| "stability witness nonfinite".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if v.iter().any(|x| *x <= 0.0) {
        return Ok("UNSTABLE_OR_UNCERTIFIED_STATE_SPACE".into());
    }
    let margin = number(ss, "stability_margin")?;
    if margin <= 0.0 {
        return Ok("INVALID_STATE_SPACE_MARGIN".into());
    }
    let av = (0..n)
        .map(|i| (0..n).map(|j| a[i][j] * v[j]).sum::<f64>())
        .collect::<Vec<_>>();
    if av.iter().any(|x| *x >= -margin) {
        return Ok("UNSTABLE_OR_UNCERTIFIED_STATE_SPACE".into());
    }
    Ok("POSITIVE_STABLE_WITH_WITNESS".into())
}
fn grid_refinement(case: &Value, tol: f64) -> Result<(Option<f64>, Option<f64>, bool), String> {
    let Some(grid) = case.get("grid_refinement") else {
        return Ok((None, None, true));
    };
    let coarse_dt = number(grid, "coarse_dt_seconds")?;
    let fine_dt = number(grid, "fine_dt_seconds")?;
    if coarse_dt <= 0.0 || fine_dt <= 0.0 || fine_dt >= coarse_dt {
        return Ok((None, None, false));
    }
    let coarse = parse_tensor(grid, "coarse_kernel", 1, 1)?;
    let fine = parse_tensor(grid, "fine_kernel", 1, 1)?;
    let ca = coarse[0][0].iter().sum::<f64>();
    let fa = fine[0][0].iter().sum::<f64>();
    let centroid = |kernel: &[f64], dt: f64| -> f64 {
        let area = kernel.iter().sum::<f64>();
        if area <= 0.0 {
            0.0
        } else {
            kernel
                .iter()
                .enumerate()
                .map(|(i, w)| *w * ((i as f64) + 0.5) * dt)
                .sum::<f64>()
                / area
        }
    };
    let area_error = (ca - fa).abs();
    let time_error = (centroid(&coarse[0][0], coarse_dt) - centroid(&fine[0][0], fine_dt)).abs();
    let max_time = number(grid, "maximum_centroid_difference_seconds")?;
    Ok((
        Some(area_error),
        Some(time_error),
        area_error <= tol && time_error <= max_time,
    ))
}
#[allow(clippy::needless_range_loop)]
fn compute(case: &Value) -> Result<Value, String> {
    let input_rows = array(case, "input_a")?;
    if input_rows.is_empty() {
        return Ok(base("INVALID_INPUT", "NO_INLET_INPUTS"));
    }
    let nin = input_rows.len();
    let nt = input_rows[0].as_array().map(Vec::len).unwrap_or(0);
    if nt == 0 {
        return Ok(base("INVALID_INPUT", "EMPTY_INPUT_HISTORY"));
    }
    let input_a = parse_matrix(case, "input_a", nin, nt)?;
    let input_b = parse_matrix(case, "input_b", nin, nt)?;
    if input_a
        .iter()
        .chain(input_b.iter())
        .flatten()
        .any(|x| !x.is_finite() || *x < 0.0)
    {
        return Ok(base("INVALID_INPUT", "NEGATIVE_OR_NONFINITE_TRACER_INPUT"));
    }
    let kernel_raw = array(case, "kernel")?;
    let nout = kernel_raw.len();
    let kernel = parse_tensor(case, "kernel", nout, nin)?;
    let tol = number(case, "invariant_tolerance")?;
    let gain_tol = number(case, "steady_gain_tolerance")?;
    if tol <= 0.0 || gain_tol <= 0.0 {
        return Ok(base("INVALID_INPUT", "INVARIANT_TOLERANCE_NOT_POSITIVE"));
    }
    if kernel.iter().flatten().flatten().any(|x| *x < -tol) {
        return Ok(base(
            "INVALID_KERNEL_NEGATIVE_COEFFICIENT",
            "PHYSICAL_KERNEL_NOT_NONNEGATIVE",
        ));
    }
    let negative_lag = array(case, "negative_lag_coefficients")?;
    if negative_lag
        .iter()
        .any(|x| x.as_f64().is_none_or(|v| !v.is_finite() || v.abs() > tol))
    {
        return Ok(base(
            "NONCAUSAL_NEGATIVE_LAG",
            "NEGATIVE_LAG_COEFFICIENT_PRESENT",
        ));
    }
    let area_by_inlet = (0..nin)
        .map(|i| {
            (0..nout)
                .map(|j| kernel[j][i].iter().sum::<f64>())
                .sum::<f64>()
        })
        .collect::<Vec<_>>();
    if area_by_inlet
        .iter()
        .any(|x| !x.is_finite() || *x > 1.0 + tol)
    {
        return Ok(base(
            "INVALID_KERNEL_MASS_GAIN_ABOVE_ONE",
            "PASSIVE_TRACER_KERNEL_CREATES_OR_DOES_NOT_ACCOUNT_FOR_MASS",
        ));
    }
    let p = parse_matrix(case, "steady_gain", nout, nin)?;
    if p.iter().flatten().any(|x| !x.is_finite() || *x < -tol) {
        return Ok(base(
            "INVALID_STEADY_GAIN",
            "STEADY_GAIN_MUST_BE_NONNEGATIVE",
        ));
    }
    let mut gain_error = 0.0_f64;
    for j in 0..nout {
        for i in 0..nin {
            gain_error = gain_error.max((p[j][i] - kernel[j][i].iter().sum::<f64>()).abs());
        }
    }
    if gain_error > gain_tol {
        return Ok(base(
            "STATIC_GAIN_MISMATCH",
            "STEADY_MATRIX_DIFFERS_FROM_INTEGRATED_KERNEL_GAIN",
        ));
    }
    let ya = conv(&input_a, &kernel)?;
    let yb = conv(&input_b, &kernel)?;
    let sum_input = input_a
        .iter()
        .zip(&input_b)
        .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x + y).collect())
        .collect::<Matrix>();
    let ysum = conv(&sum_input, &kernel)?;
    let mut super_error = 0.0_f64;
    for j in 0..nout {
        for t in 0..ysum[j].len() {
            super_error = super_error.max((ysum[j][t] - ya[j][t] - yb[j][t]).abs());
        }
    }
    let shifted = input_a
        .iter()
        .map(|r| {
            let mut v = vec![0.0];
            v.extend(r.iter().copied());
            v
        })
        .collect::<Matrix>();
    let yshift = conv(&shifted, &kernel)?;
    let mut shift_error = 0.0_f64;
    for j in 0..nout {
        for t in 0..yshift[j].len() {
            let expected = if t == 0 { 0.0 } else { ya[j][t - 1] };
            shift_error = shift_error.max((yshift[j][t] - expected).abs());
        }
    }
    let reversed_kernel = kernel.iter().rev().cloned().collect::<Tensor3>();
    let yperm = conv(&input_a, &reversed_kernel)?;
    let mut perm_error = 0.0_f64;
    for j in 0..nout {
        for t in 0..ya[j].len() {
            perm_error = perm_error.max((yperm[nout - 1 - j][t] - ya[j][t]).abs());
        }
    }
    let alpha = number(case, "scale_factor")?;
    if alpha <= 0.0 {
        return Ok(base("INVALID_INPUT", "SCALE_FACTOR_NOT_POSITIVE"));
    }
    let scaled_input = input_a
        .iter()
        .map(|r| r.iter().map(|x| x * alpha).collect())
        .collect::<Matrix>();
    let scaled_output = conv(&scaled_input, &kernel)?;
    let mut scale_error = 0.0_f64;
    for j in 0..nout {
        for t in 0..ya[j].len() {
            scale_error = scale_error.max((scaled_output[j][t] - alpha * ya[j][t]).abs());
        }
    }
    let min_output = ya
        .iter()
        .flatten()
        .chain(yb.iter().flatten())
        .copied()
        .fold(f64::INFINITY, f64::min);
    if min_output < -tol {
        return Ok(base(
            "POSITIVITY_VIOLATION",
            "NONNEGATIVE_INPUT_PRODUCED_NEGATIVE_OUTPUT",
        ));
    }
    let dt = number(case, "sample_interval_seconds")?;
    if dt <= 0.0 {
        return Ok(base("INVALID_INPUT", "SAMPLE_INTERVAL_NOT_POSITIVE"));
    }
    let input_mass = input_a
        .iter()
        .chain(input_b.iter())
        .map(|r| r.iter().sum::<f64>())
        .sum::<f64>()
        * dt;
    let output_mass = ya
        .iter()
        .chain(yb.iter())
        .map(|r| r.iter().sum::<f64>())
        .sum::<f64>()
        * dt;
    let state_space = check_state_space(case, tol)?;
    if state_space.starts_with("INVALID") || state_space.starts_with("UNSTABLE") {
        return Ok(base(
            &state_space,
            "STATE_SPACE_REALIZATION_FAILED_POSITIVE_STABILITY_CONTRACT",
        ));
    }
    let (grid_area, grid_time, grid_ok) = grid_refinement(case, tol)?;
    if !grid_ok {
        return Ok(base(
            "GRID_REFINEMENT_MISMATCH",
            "COARSE_FINE_KERNEL_AREA_OR_CENTROID_DIFFERENCE_EXCEEDS_LIMIT",
        ));
    }
    if super_error > tol || shift_error > tol || perm_error > tol || scale_error > tol {
        return Ok(base(
            "METAMORPHIC_INVARIANT_FAILURE",
            "LINEARITY_SHIFT_PERMUTATION_OR_SCALING_INVARIANT_FAILED",
        ));
    }
    Ok(json!({
        "invariant_disposition":"PASS_SYNTHETIC_INVARIANTS","reason":null,
        "kernel_area_by_inlet":area_by_inlet,
        "steady_gain_max_abs_error":gain_error,
        "superposition_max_abs_error":super_error,
        "time_shift_max_abs_error":shift_error,
        "port_permutation_max_abs_error":perm_error,
        "scale_max_abs_error":scale_error,
        "grid_area_max_abs_error":grid_area,
        "grid_centroid_max_abs_error_seconds":grid_time,
        "state_space_disposition":state_space,
        "positive_output_minimum":min_output,
        "predicted_total_output_mass":output_mass,
        "input_mass_across_two_histories":input_mass,
        "synthetic_input_mass_not_returned_by_kernel":(input_mass-output_mass).max(0.0),
        "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
    }))
}
fn ids(rows: &[Value], key: &str) -> Result<BTreeSet<String>, String> {
    rows.iter()
        .map(|x| {
            field(x, key)?
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} must be text"))
        })
        .collect()
}
fn validate(input: &Value, oracle: &Value, manifest: &Value) -> Result<(), String> {
    if text(input, "schema_version")? != "operator-invariants-t0-input-v0.1" {
        return Err("unexpected input schema".into());
    }
    if text(oracle, "schema_version")? != "operator-invariants-t0-oracle-v0.1" {
        return Err("unexpected oracle schema".into());
    }
    for x in [input, oracle, manifest] {
        if text(x, "programme_id")? != PROGRAMME {
            return Err("programme identity mismatch".into());
        }
    }
    for k in [
        "physical_operator_qualified",
        "potability_claim_permitted",
        "release_authorized",
    ] {
        if field(oracle, k)?.as_bool() != Some(false) {
            return Err(format!("oracle cannot claim {k} true"));
        }
    }
    let a = array(input, "cases")?;
    let b = array(oracle, "cases")?;
    let m = array(manifest, "case_ids")?;
    if a.len() != CASE_COUNT
        || b.len() != CASE_COUNT
        || number(manifest, "case_count")? != CASE_COUNT as f64
    {
        return Err("frozen operator-invariant case count mismatch".into());
    }
    let aid = ids(a, "case_id")?;
    let bid = ids(b, "case_id")?;
    let mid = m
        .iter()
        .map(|x| {
            x.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "manifest ID must be text".to_owned())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if aid.len() != a.len()
        || bid.len() != b.len()
        || mid.len() != m.len()
        || aid != bid
        || aid != mid
    {
        return Err("input/oracle/manifest identity mismatch".into());
    }
    Ok(())
}
fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            matches!((x.as_f64(),y.as_f64()),(Some(u),Some(v)) if (u-v).abs()<=1.0e-8*u.abs().max(v.abs()).max(1.0))
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(u, v)| close(u, v))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| close(v, w)))
        }
        _ => a == b,
    }
}
pub fn evaluate(input: &Value, oracle: &Value, manifest: &Value) -> Result<Value, String> {
    validate(input, oracle, manifest)?;
    let mut expected = std::collections::BTreeMap::new();
    for x in array(oracle, "cases")? {
        expected.insert(
            text(x, "case_id")?.to_owned(),
            field(x, "expected")?.clone(),
        );
    }
    let mut rows = Vec::new();
    let mut mismatches = Vec::new();
    for c in array(input, "cases")? {
        let id = text(c, "case_id")?;
        let actual = compute(field(c, "inputs")?)?;
        let exp = expected
            .get(id)
            .ok_or_else(|| format!("oracle missing {id}"))?;
        if !close(&actual, exp) {
            mismatches.push(json!({"case_id":id,"expected":exp,"actual":actual}));
        }
        rows.push(json!({"case_id":id,"matched":close(&actual,exp),"result":actual}));
    }
    rows.sort_by(|a, b| a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    let matched = rows.iter().filter(|x| x["matched"] == true).count();
    Ok(json!({
        "schema_version":"operator-invariants-t0-evaluation-v0.1",
        "verification_state":if mismatches.is_empty(){"MatchedFrozenOracle"}else{"OracleMismatch"},
        "evidence_class":"synthetic_only","evidence_ceiling":EVIDENCE_CEILING,
        "case_count":rows.len(),"matched_case_count":matched,"mismatches":mismatches,"results":rows,
        "nonclaims":[
            "This evaluator checks discrete synthetic kernel and positive-realization invariants only.",
            "The sufficient stability witness is not a physical system identification result.",
            "Fixed-regime superposition and time invariance are not assumed across hydraulic transitions or reactive chemistry.",
            "No output qualifies a physical operator, sensor, tracer, junction, tank or drinking-water system."
        ]
    }))
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn read_json(p: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(p).map_err(|e| format!("read {}: {e}", p.display()))?;
    let value =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse {}: {e}", p.display()))?;
    Ok((bytes, value))
}
fn valid_revision(x: &str) -> bool {
    x.len() == 40
        && x.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 4 {
        return Err(
            "usage: mixing-kernel-invariants-t0 [INPUT] [ORACLE] [MANIFEST] [RECEIPT]".into(),
        );
    }
    let ip = PathBuf::from(
        args.first()
            .map(String::as_str)
            .unwrap_or("research/water-intrusion-coupling/operator-invariants-t0-cases-v0.1.json"),
    );
    let op =
        PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
            "research/water-intrusion-coupling/operator-invariants-t0-oracle-v0.1.json",
        ));
    let mp =
        PathBuf::from(args.get(2).map(String::as_str).unwrap_or(
            "research/water-intrusion-coupling/operator-invariants-t0-manifest-v0.1.json",
        ));
    let rp = PathBuf::from(
        args.get(3)
            .map(String::as_str)
            .unwrap_or("artifacts/water-mixing-operator-invariants-t0/receipt.json"),
    );
    let (ib, iv) = read_json(&ip)?;
    let (ob, ov) = read_json(&op)?;
    let (mb, mv) = read_json(&mp)?;
    let evaluation = evaluate(&iv, &ov, &mv)?;
    let success = evaluation["verification_state"] == "MatchedFrozenOracle";
    let revision = env::var("WATER_MIXING_INVARIANTS_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|x| valid_revision(x));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock = fs::read(&lock_path).map_err(|e| format!("read {}: {e}", lock_path.display()))?;
    let receipt = json!({
        "schema_version":"operator-invariants-t0-run-v0.1",
        "verification_state":if !success{"OracleMismatch"}else if revision.is_some(){"PassedExactRevision"}else{"PassedLocalUnpinned"},
        "implementation_revision":revision,
        "input_sha256":digest(&ib),"oracle_sha256":digest(&ob),"manifest_sha256":digest(&mb),"cargo_lock_sha256":digest(&lock),
        "case_count":evaluation["case_count"],"matched_case_count":evaluation["matched_case_count"],
        "mismatches":evaluation["mismatches"],"results":evaluation["results"],
        "evidence_class":"synthetic_only","evidence_ceiling":EVIDENCE_CEILING,
        "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
    });
    if let Some(parent) = rp.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let bytes = serde_json::to_string_pretty(&receipt)
        .map_err(|e| format!("serialize receipt: {e}"))?
        + "\n";
    fs::write(&rp, bytes.as_bytes()).map_err(|e| format!("write {}: {e}", rp.display()))?;
    println!("{bytes}");
    Ok(if success { 0 } else { 2 })
}
fn main() {
    match run() {
        Ok(c) => std::process::exit(c),
        Err(e) => {
            eprintln!("WATER_MIXING_INVARIANTS_T0_ERROR: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Value {
        let raw = match name {
            "input" => include_str!(
                "../../../../research/water-intrusion-coupling/operator-invariants-t0-cases-v0.1.json"
            ),
            "oracle" => include_str!(
                "../../../../research/water-intrusion-coupling/operator-invariants-t0-oracle-v0.1.json"
            ),
            "manifest" => include_str!(
                "../../../../research/water-intrusion-coupling/operator-invariants-t0-manifest-v0.1.json"
            ),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(raw).expect("operator invariant fixture parses")
    }

    fn result() -> Value {
        evaluate(&load("input"), &load("oracle"), &load("manifest"))
            .expect("operator invariant T0 runs")
    }

    fn row(id: &str) -> Value {
        result()["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["case_id"] == id)
            .unwrap()["result"]
            .clone()
    }

    #[test]
    fn all_ten_operator_invariant_cases_match_oracle() {
        let x = result();
        assert_eq!(x["case_count"], CASE_COUNT);
        assert_eq!(x["matched_case_count"], CASE_COUNT);
        assert_eq!(x["verification_state"], "MatchedFrozenOracle");
    }

    #[test]
    fn negative_kernel_coefficients_are_rejected() {
        assert_eq!(
            row("OI-T0-002-negative-coefficient")["invariant_disposition"],
            "INVALID_KERNEL_NEGATIVE_COEFFICIENT"
        );
    }

    #[test]
    fn noncausal_negative_lag_is_rejected() {
        assert_eq!(
            row("OI-T0-003-negative-lag")["invariant_disposition"],
            "NONCAUSAL_NEGATIVE_LAG"
        );
    }

    #[test]
    fn passive_mass_gain_above_one_is_rejected() {
        assert_eq!(
            row("OI-T0-004-mass-gain-above-one")["invariant_disposition"],
            "INVALID_KERNEL_MASS_GAIN_ABOVE_ONE"
        );
    }

    #[test]
    fn integrated_kernel_gain_must_match_steady_transfer() {
        assert_eq!(
            row("OI-T0-005-static-gain-mismatch")["invariant_disposition"],
            "STATIC_GAIN_MISMATCH"
        );
    }

    #[test]
    fn superposition_time_shift_scale_and_port_permutation_pass() {
        let x = row("OI-T0-001-positive-fixed-regime");
        assert_eq!(x["invariant_disposition"], "PASS_SYNTHETIC_INVARIANTS");
    }

    #[test]
    fn positive_state_space_requires_metzler_and_nonnegative_coefficients() {
        assert_eq!(
            row("OI-T0-006-positive-stable-realization")["state_space_disposition"],
            "POSITIVE_STABLE_WITH_WITNESS"
        );
        assert_eq!(
            row("OI-T0-007-non-metzler")["invariant_disposition"],
            "INVALID_STATE_SPACE_NOT_METZLER"
        );
    }

    #[test]
    fn positive_state_space_requires_stability_witness() {
        assert_eq!(
            row("OI-T0-008-unstable-metzler")["invariant_disposition"],
            "UNSTABLE_OR_UNCERTIFIED_STATE_SPACE"
        );
    }

    #[test]
    fn coarse_fine_kernel_area_and_centroid_must_converge() {
        let x = row("OI-T0-009-grid-refinement");
        assert_eq!(x["invariant_disposition"], "PASS_SYNTHETIC_INVARIANTS");
        assert_eq!(x["grid_area_max_abs_error"], 0.0);
    }

    #[test]
    fn invalid_grid_refinement_is_rejected() {
        assert_eq!(
            row("OI-T0-010-grid-refinement-mismatch")["invariant_disposition"],
            "GRID_REFINEMENT_MISMATCH"
        );
    }
}
