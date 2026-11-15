// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

const PROGRAMME: &str = "WATER-MIXING-CONSTRAINED-IDENTIFICATION-001";
const CASE_COUNT: usize = 8;
const EVIDENCE_CEILING: &str =
    "synthetic passive-kernel constrained-identification arithmetic only";
type Matrix = Vec<Vec<f64>>;
type Vector = Vec<f64>;

fn field<'a>(v: &'a Value, k: &str) -> Result<&'a Value, String> {
    v.get(k).ok_or_else(|| format!("missing field: {k}"))
}
fn number(v: &Value, k: &str) -> Result<f64, String> {
    field(v, k)?
        .as_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| format!("{k} must be finite numeric"))
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    field(v, k)?
        .as_str()
        .ok_or_else(|| format!("{k} must be text"))
}
fn usize_value(v: &Value, k: &str) -> Result<usize, String> {
    field(v, k)?
        .as_u64()
        .and_then(|x| usize::try_from(x).ok())
        .ok_or_else(|| format!("{k} must be non-negative integer"))
}
fn array<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>, String> {
    field(v, k)?
        .as_array()
        .ok_or_else(|| format!("{k} must be array"))
}
fn matrix(v: &Value, key: &str, rows: usize, cols: usize) -> Result<Matrix, String> {
    let raw = array(v, key)?;
    if raw.len() != rows {
        return Err(format!("{key} row count mismatch"));
    }
    let mut result = Vec::with_capacity(rows);
    for row in raw {
        let vals = row
            .as_array()
            .ok_or_else(|| format!("{key} row must be array"))?;
        if vals.len() != cols {
            return Err(format!("{key} column count mismatch"));
        }
        result.push(
            vals.iter()
                .map(|x| {
                    x.as_f64()
                        .filter(|n| n.is_finite())
                        .ok_or_else(|| format!("{key} contains non-finite value"))
                })
                .collect::<Result<Vector, _>>()?,
        );
    }
    Ok(result)
}
fn vector(v: &Value, key: &str, len: usize) -> Result<Vector, String> {
    let raw = array(v, key)?;
    if raw.len() != len {
        return Err(format!("{key} length mismatch"));
    }
    raw.iter()
        .map(|x| {
            x.as_f64()
                .filter(|n| n.is_finite())
                .ok_or_else(|| format!("{key} contains non-finite value"))
        })
        .collect()
}
fn base(state: &str, reason: &str) -> Value {
    json!({
        "identification_disposition":state,"reason":reason,
        "design_rank":null,"design_condition_number":null,
        "unconstrained_coefficients":null,"clipped_coefficients":null,"constrained_coefficients":null,
        "unconstrained_training_mse":null,"clipped_training_mse":null,"constrained_training_mse":null,
        "unconstrained_holdout_mse":null,"clipped_holdout_mse":null,"constrained_holdout_mse":null,
        "unconstrained_min_coefficient":null,"clipped_min_coefficient":null,"constrained_min_coefficient":null,
        "unconstrained_gain_by_group":null,"clipped_gain_by_group":null,"constrained_gain_by_group":null,
        "constrained_gain_target_by_group":null,"solver_iterations":null,"solver_converged":false,
        "constraint_satisfied":false,"blind_holdout_improvement_vs_clip":false,
        "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
    })
}
fn transpose_multiply(x: &Matrix, y: &Vector) -> Result<Vector, String> {
    if x.is_empty() || x.len() != y.len() {
        return Err("normal-equation row count mismatch".into());
    }
    let p = x[0].len();
    if x.iter().any(|r| r.len() != p) {
        return Err("design matrix must be rectangular".into());
    }
    let mut out = vec![0.0; p];
    for i in 0..x.len() {
        for j in 0..p {
            out[j] += x[i][j] * y[i];
        }
    }
    if out.iter().any(|v| !v.is_finite()) {
        return Err("X^T y overflow".into());
    }
    Ok(out)
}
fn gram(x: &Matrix) -> Result<Matrix, String> {
    if x.is_empty() {
        return Err("design matrix empty".into());
    }
    let p = x[0].len();
    if p == 0 || x.iter().any(|r| r.len() != p) {
        return Err("design matrix dimensions invalid".into());
    }
    let mut g = vec![vec![0.0; p]; p];
    for row in x {
        for i in 0..p {
            for j in 0..p {
                g[i][j] += row[i] * row[j];
            }
        }
    }
    Ok(g)
}
fn predict(x: &Matrix, h: &Vector) -> Result<Vector, String> {
    if x.iter().any(|r| r.len() != h.len()) {
        return Err("prediction parameter dimension mismatch".into());
    }
    let y = x
        .iter()
        .map(|r| r.iter().zip(h).map(|(a, b)| a * b).sum::<f64>())
        .collect::<Vector>();
    if y.iter().any(|v| !v.is_finite()) {
        return Err("prediction overflow".into());
    }
    Ok(y)
}
fn weighted_system(x: &Matrix, y: &Vector, weights: &Vector) -> Result<(Matrix, Vector), String> {
    if x.len() != y.len() || x.len() != weights.len() || x.is_empty() {
        return Err("weighted design dimensions invalid".into());
    }
    let columns = x[0].len();
    if x.iter().any(|r| r.len() != columns) {
        return Err("weighted design is not rectangular".into());
    }
    let mut wx = Vec::with_capacity(x.len());
    let mut wy = Vec::with_capacity(y.len());
    for ((row, value), weight) in x.iter().zip(y).zip(weights) {
        if !weight.is_finite() || *weight <= 0.0 {
            return Err("sample weights must be positive finite".into());
        }
        let scale = weight.sqrt();
        wx.push(row.iter().map(|v| v * scale).collect());
        wy.push(value * scale);
    }
    Ok((wx, wy))
}
fn weighted_mse(actual: &Vector, predicted: &Vector, weights: &Vector) -> Result<f64, String> {
    if actual.len() != predicted.len() || actual.len() != weights.len() || actual.is_empty() {
        return Err("weighted MSE length mismatch or empty vector".into());
    }
    let total = weights.iter().sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return Err("weighted MSE total weight invalid".into());
    }
    Ok(actual
        .iter()
        .zip(predicted)
        .zip(weights)
        .map(|((a, b), w)| w * (a - b).powi(2))
        .sum::<f64>()
        / total)
}
#[allow(clippy::needless_range_loop)]
fn solve_linear(mut a: Matrix, mut b: Vector, tol: f64) -> Result<Vector, String> {
    let n = b.len();
    if a.len() != n || a.iter().any(|r| r.len() != n) {
        return Err("linear solve dimensions invalid".into());
    }
    let scale = a
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0e-300);
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|i, j| a[*i][col].abs().total_cmp(&a[*j][col].abs()))
            .ok_or_else(|| "pivot search failed".to_owned())?;
        if a[pivot][col].abs() <= tol * scale {
            return Err("normal matrix singular at solve tolerance".into());
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let diag = a[col][col];
        for j in col..n {
            a[col][j] /= diag;
        }
        b[col] /= diag;
        for i in 0..n {
            if i == col {
                continue;
            }
            let factor = a[i][col];
            for j in col..n {
                a[i][j] -= factor * a[col][j];
            }
            b[i] -= factor * b[col];
        }
    }
    if b.iter().any(|v| !v.is_finite()) {
        return Err("linear solve non-finite result".into());
    }
    Ok(b)
}
#[allow(clippy::needless_range_loop)]
fn eigenvalues_symmetric(mut a: Matrix) -> Result<Vector, String> {
    let n = a.len();
    if n == 0 || a.iter().any(|r| r.len() != n) {
        return Err("eigenvalue matrix must be square".into());
    }
    let scale = a
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0e-300);
    for i in 0..n {
        for j in 0..n {
            if (a[i][j] - a[j][i]).abs() > 1.0e-10 * scale {
                return Err("Gram matrix is not symmetric".into());
            }
        }
    }
    if n == 1 {
        return Ok(vec![a[0][0]]);
    }
    let mut converged = false;
    for _ in 0..(100 * n * n) {
        let mut p = 0;
        let mut q = 1;
        let mut largest = 0.0_f64;
        for i in 0..n {
            for j in (i + 1)..n {
                if a[i][j].abs() > largest {
                    largest = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if largest <= 1.0e-13 * scale {
            converged = true;
            break;
        }
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let sign = if tau >= 0.0 { 1.0 } else { -1.0 };
        let t = sign / (tau.abs() + (1.0 + tau * tau).sqrt());
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        for k in 0..n {
            if k != p && k != q {
                let kp = a[k][p];
                let kq = a[k][q];
                a[k][p] = c * kp - s * kq;
                a[p][k] = a[k][p];
                a[k][q] = s * kp + c * kq;
                a[q][k] = a[k][q];
            }
        }
        a[p][p] = app - t * apq;
        a[q][q] = aqq + t * apq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
    }
    if !converged {
        return Err("symmetric Jacobi eigensolver did not converge".into());
    }
    let mut values = (0..n).map(|i| a[i][i]).collect::<Vector>();
    values.sort_by(|x, y| y.total_cmp(x));
    if values.iter().any(|v| *v < -1.0e-9 * scale) {
        return Err("Gram matrix has materially negative eigenvalue".into());
    }
    for v in &mut values {
        if *v < 0.0 {
            *v = 0.0;
        }
    }
    Ok(values)
}
fn design_metrics(x: &Matrix, rank_tol: f64) -> Result<(usize, Option<f64>, f64), String> {
    let eigen = eigenvalues_symmetric(gram(x)?)?;
    let max = eigen.first().copied().unwrap_or(0.0);
    if max <= 0.0 {
        return Ok((0, None, 0.0));
    }
    let singular = eigen.iter().map(|v| v.sqrt()).collect::<Vector>();
    let threshold = singular[0] * rank_tol;
    let rank = singular.iter().filter(|s| **s > threshold).count();
    let condition = if rank == singular.len() && singular.last().copied().unwrap_or(0.0) > 0.0 {
        Some(singular[0] / singular[singular.len() - 1])
    } else {
        None
    };
    Ok((rank, condition, singular[0] * singular[0]))
}
fn project_simplex(z: &[f64], target: f64) -> Vector {
    if target <= 0.0 {
        return vec![0.0; z.len()];
    }
    let mut sorted = z.to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a));
    let mut sum = 0.0;
    let mut theta = 0.0;
    let mut rho = 0usize;
    for (i, value) in sorted.iter().enumerate() {
        sum += value;
        let candidate = (sum - target) / (i as f64 + 1.0);
        if *value > candidate {
            rho = i + 1;
            theta = candidate;
        }
    }
    if rho == 0 {
        return vec![target / z.len() as f64; z.len()];
    }
    z.iter().map(|v| (v - theta).max(0.0)).collect()
}
fn project_groups(z: &Vector, groups: &[Vec<usize>], targets: &[f64]) -> Result<Vector, String> {
    if groups.len() != targets.len() {
        return Err("group target count mismatch".into());
    }
    let mut result = vec![0.0; z.len()];
    let mut seen = BTreeSet::new();
    for (group, target) in groups.iter().zip(targets) {
        if group.is_empty() || *target < 0.0 || *target > 1.0 + 1.0e-12 {
            return Err("invalid mass group or gain target".into());
        }
        let mut values = Vec::new();
        for index in group {
            if *index >= z.len() || !seen.insert(*index) {
                return Err("mass groups overlap or index is out of range".into());
            }
            values.push(z[*index]);
        }
        let projected = project_simplex(&values, *target);
        for (index, value) in group.iter().zip(projected) {
            result[*index] = value;
        }
    }
    if seen.len() != z.len() {
        return Err("mass groups do not cover all kernel coefficients".into());
    }
    Ok(result)
}
fn dot(a: &Vector, b: &Vector) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn mat_vec(a: &Matrix, x: &Vector) -> Vector {
    a.iter().map(|r| dot(r, x)).collect()
}
fn norm_diff(a: &Vector, b: &Vector) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}
fn constrained_fit(
    x: &Matrix,
    y: &Vector,
    groups: &[Vec<usize>],
    targets: &[f64],
    tol: f64,
    max_iter: usize,
) -> Result<(Vector, usize, bool), String> {
    let g = gram(x)?;
    let b = transpose_multiply(x, y)?;
    let eig = eigenvalues_symmetric(g.clone())?;
    let lipschitz = eig.first().copied().unwrap_or(0.0);
    if lipschitz <= 0.0 {
        return Err("non-positive gradient Lipschitz constant".into());
    }
    let n = b.len();
    let mut h = project_groups(&vec![0.0; n], groups, targets)?;
    let mut z = h.clone();
    let mut t: f64 = 1.0;
    let mut converged = false;
    let mut iterations = 0;
    for iteration in 0..max_iter {
        let gz = mat_vec(&g, &z);
        let grad = gz.iter().zip(&b).map(|(a, c)| a - c).collect::<Vector>();
        let step = z
            .iter()
            .zip(&grad)
            .map(|(a, c)| a - c / lipschitz)
            .collect::<Vector>();
        let next = project_groups(&step, groups, targets)?;
        let delta = norm_diff(&next, &h);
        let t_next = (1.0 + (1.0 + 4.0 * t * t).sqrt()) / 2.0;
        let momentum = (t - 1.0) / t_next;
        z = next
            .iter()
            .zip(&h)
            .map(|(a, b)| a + momentum * (a - b))
            .collect();
        h = next;
        t = t_next;
        iterations = iteration + 1;
        if delta <= tol * (1.0 + norm_diff(&h, &vec![0.0; n])) {
            converged = true;
            break;
        }
    }
    Ok((h, iterations, converged))
}
fn group_gains(h: &Vector, groups: &[Vec<usize>]) -> Vector {
    groups
        .iter()
        .map(|g| g.iter().map(|i| h[*i]).sum())
        .collect()
}
fn coefficients_min(h: &Vector) -> f64 {
    h.iter().copied().fold(f64::INFINITY, f64::min)
}
fn coefficient_rmse(a: &Vector, b: &Vector) -> f64 {
    (a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f64>() / a.len() as f64).sqrt()
}
fn evaluate_fit(
    x: &Matrix,
    y: &Vector,
    weights: &Vector,
    holdout_x: &Matrix,
    holdout_y: &Vector,
    holdout_weights: &Vector,
    h: &Vector,
) -> Result<(f64, f64), String> {
    Ok((
        weighted_mse(y, &predict(x, h)?, weights)?,
        weighted_mse(holdout_y, &predict(holdout_x, h)?, holdout_weights)?,
    ))
}
fn compute(case: &Value) -> Result<Value, String> {
    let x_rows = array(case, "design_matrix")?;
    if x_rows.is_empty() {
        return Ok(base("INVALID_INPUT", "EMPTY_DESIGN_MATRIX"));
    }
    let m = x_rows.len();
    let p = x_rows[0].as_array().map(Vec::len).unwrap_or(0);
    if p == 0 {
        return Ok(base("INVALID_INPUT", "DESIGN_MATRIX_HAS_ZERO_COLUMNS"));
    }
    let x = matrix(case, "design_matrix", m, p)?;
    let y = vector(case, "observations", m)?;
    if x.iter().flatten().any(|v| *v < 0.0) || y.iter().any(|v| *v < 0.0) {
        return Ok(base(
            "INVALID_INPUT",
            "PASSIVE_TRACER_DESIGN_AND_OBSERVATIONS_MUST_BE_NONNEGATIVE",
        ));
    }
    let noise_contract = field(case, "observation_noise_contract")?
        .as_str()
        .ok_or_else(|| "observation_noise_contract must be text".to_owned())?;
    if noise_contract != "independent_diagonal_variance" {
        return Ok(base(
            "REJECTED_CONTRACT",
            "UNSUPPORTED_OR_UNDECLARED_OBSERVATION_COVARIANCE",
        ));
    }
    let variance = vector(case, "observation_variance", m)?;
    let weights = vector(case, "sample_weights", m)?;
    if variance
        .iter()
        .zip(&weights)
        .any(|(v, w)| *v <= 0.0 || *w <= 0.0)
    {
        return Ok(base(
            "INVALID_INPUT",
            "OBSERVATION_VARIANCE_AND_WEIGHTS_MUST_BE_POSITIVE",
        ));
    }
    if variance
        .iter()
        .zip(&weights)
        .any(|(v, w)| (v * w - 1.0).abs() > 1.0e-8 * (v * w).abs().max(1.0))
    {
        return Ok(base(
            "INVALID_INPUT",
            "SAMPLE_WEIGHTS_MUST_BE_INVERSE_VARIANCE",
        ));
    }
    let hx_rows = array(case, "holdout_design_matrix")?;
    if hx_rows.is_empty() {
        return Ok(base("INVALID_INPUT", "EMPTY_HOLDOUT_DESIGN"));
    }
    let hm = hx_rows.len();
    let holdout_x = matrix(case, "holdout_design_matrix", hm, p)?;
    let holdout_y = vector(case, "holdout_observations", hm)?;
    if holdout_x.iter().flatten().any(|v| *v < 0.0) || holdout_y.iter().any(|v| *v < 0.0) {
        return Ok(base(
            "INVALID_INPUT",
            "PASSIVE_TRACER_HOLDOUT_MUST_BE_NONNEGATIVE",
        ));
    }
    let holdout_variance = vector(case, "holdout_observation_variance", hm)?;
    let holdout_weights = vector(case, "holdout_sample_weights", hm)?;
    if holdout_variance
        .iter()
        .zip(&holdout_weights)
        .any(|(v, w)| *v <= 0.0 || *w <= 0.0)
    {
        return Ok(base(
            "INVALID_INPUT",
            "HOLDOUT_VARIANCE_AND_WEIGHTS_MUST_BE_POSITIVE",
        ));
    }
    if holdout_variance
        .iter()
        .zip(&holdout_weights)
        .any(|(v, w)| (v * w - 1.0).abs() > 1.0e-8 * (v * w).abs().max(1.0))
    {
        return Ok(base(
            "INVALID_INPUT",
            "HOLDOUT_WEIGHTS_MUST_BE_INVERSE_VARIANCE",
        ));
    }
    let (weighted_x, weighted_y) = weighted_system(&x, &y, &weights)?;
    let truth = vector(case, "true_coefficients", p)?;
    if truth.iter().any(|v| *v < 0.0) {
        return Ok(base("INVALID_INPUT", "TRUE_KERNEL_NEGATIVE"));
    }
    let groups_raw = array(case, "mass_groups")?;
    let mut groups = Vec::new();
    for g in groups_raw {
        let raw = g
            .as_array()
            .ok_or_else(|| "mass group must be array".to_owned())?;
        let indices = raw
            .iter()
            .map(|v| {
                v.as_u64()
                    .and_then(|x| usize::try_from(x).ok())
                    .ok_or_else(|| "mass group index must be usize".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        groups.push(indices);
    }
    let targets = vector(case, "steady_gain_targets", groups.len())?;
    if targets.iter().any(|g| *g < 0.0 || *g > 1.0) {
        return Ok(base(
            "INVALID_INPUT",
            "STEADY_GAIN_TARGET_OUTSIDE_PASSIVE_RANGE",
        ));
    }
    let rank_tol = number(case, "rank_relative_tolerance")?;
    let max_condition = number(case, "maximum_condition_number")?;
    let solve_tol = number(case, "solver_tolerance")?;
    let max_iterations = usize_value(case, "maximum_iterations")?;
    if rank_tol <= 0.0
        || rank_tol >= 0.1
        || max_condition <= 1.0
        || solve_tol <= 0.0
        || max_iterations == 0
    {
        return Ok(base(
            "INVALID_INPUT",
            "RANK_SOLVER_OR_ITERATION_THRESHOLD_INVALID",
        ));
    }
    let (rank, condition, _) = design_metrics(&weighted_x, rank_tol)?;
    if rank < p {
        return Ok(
            json!({"identification_disposition":"UNIDENTIFIABLE","reason":"DESIGN_MATRIX_RANK_DEFICIENT","design_rank":rank,"design_condition_number":condition,"physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false}),
        );
    }
    if condition.is_none_or(|v| v > max_condition) {
        return Ok(
            json!({"identification_disposition":"ILL_CONDITIONED","reason":"DESIGN_MATRIX_CONDITION_EXCEEDS_LIMIT","design_rank":rank,"design_condition_number":condition,"physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false}),
        );
    }
    let xtx = gram(&weighted_x)?;
    let xty = transpose_multiply(&weighted_x, &weighted_y)?;
    let unconstrained = match solve_linear(xtx, xty, rank_tol * rank_tol) {
        Ok(v) => v,
        Err(_) => {
            return Ok(base(
                "UNIDENTIFIABLE",
                "UNCONSTRAINED_NORMAL_EQUATION_SINGULAR",
            ));
        }
    };
    let clipped = unconstrained.iter().map(|v| v.max(0.0)).collect::<Vector>();
    let (constrained, iterations, converged) = constrained_fit(
        &weighted_x,
        &weighted_y,
        &groups,
        &targets,
        solve_tol,
        max_iterations,
    )?;
    if !converged {
        return Ok(
            json!({"identification_disposition":"SOLVER_NOT_CONVERGED","reason":"PROJECTED_GRADIENT_DID_NOT_CONVERGE","design_rank":rank,"design_condition_number":condition,"solver_iterations":iterations,"solver_converged":false,"physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false}),
        );
    }
    let (u_train, u_hold) = evaluate_fit(
        &x,
        &y,
        &weights,
        &holdout_x,
        &holdout_y,
        &holdout_weights,
        &unconstrained,
    )?;
    let (c_train, c_hold) = evaluate_fit(
        &x,
        &y,
        &weights,
        &holdout_x,
        &holdout_y,
        &holdout_weights,
        &clipped,
    )?;
    let (n_train, n_hold) = evaluate_fit(
        &x,
        &y,
        &weights,
        &holdout_x,
        &holdout_y,
        &holdout_weights,
        &constrained,
    )?;
    let ug = group_gains(&unconstrained, &groups);
    let cg = group_gains(&clipped, &groups);
    let ng = group_gains(&constrained, &groups);
    let coeff_tol = number(case, "constraint_tolerance")?;
    let constraint_ok = constrained.iter().all(|v| *v >= -coeff_tol)
        && ng
            .iter()
            .zip(&targets)
            .all(|(a, b)| (a - b).abs() <= coeff_tol);
    let holdout_improvement = n_hold + coeff_tol < c_hold;
    let state = if !constraint_ok {
        "CONSTRAINT_VIOLATION"
    } else {
        "IDENTIFIED_SYNTHETIC"
    };
    Ok(json!({
        "identification_disposition":state,"reason":null,
        "design_rank":rank,"design_condition_number":condition,
        "observation_noise_contract":noise_contract,
        "total_training_weight":weights.iter().sum::<f64>(),
        "total_holdout_weight":holdout_weights.iter().sum::<f64>(),
        "unconstrained_coefficients":unconstrained,"clipped_coefficients":clipped,"constrained_coefficients":constrained,
        "unconstrained_training_mse":u_train,"clipped_training_mse":c_train,"constrained_training_mse":n_train,
        "unconstrained_holdout_mse":u_hold,"clipped_holdout_mse":c_hold,"constrained_holdout_mse":n_hold,
        "unconstrained_min_coefficient":coefficients_min(&unconstrained),
        "clipped_min_coefficient":coefficients_min(&clipped),
        "constrained_min_coefficient":coefficients_min(&constrained),
        "unconstrained_gain_by_group":ug,"clipped_gain_by_group":cg,"constrained_gain_by_group":ng,
        "constrained_gain_target_by_group":targets,"solver_iterations":iterations,"solver_converged":converged,
        "constraint_satisfied":constraint_ok,"blind_holdout_improvement_vs_clip":holdout_improvement,
        "true_coefficient_rmse_unconstrained":coefficient_rmse(&unconstrained,&truth),
        "true_coefficient_rmse_clipped":coefficient_rmse(&clipped,&truth),
        "true_coefficient_rmse_constrained":coefficient_rmse(&constrained,&truth),
        "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
    }))
}
fn validate(input: &Value, oracle: &Value, manifest: &Value) -> Result<(), String> {
    if text(input, "schema_version")? != "constrained-kernel-id-t0-input-v0.1" {
        return Err("unexpected input schema".into());
    }
    if text(oracle, "schema_version")? != "constrained-kernel-id-t0-oracle-v0.1" {
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
            return Err(format!("oracle cannot claim {k}"));
        }
    }
    let cases = array(input, "cases")?;
    let expected = array(oracle, "cases")?;
    let ids = array(manifest, "case_ids")?;
    if cases.len() != CASE_COUNT
        || expected.len() != CASE_COUNT
        || usize_value(manifest, "case_count")? != CASE_COUNT
    {
        return Err("frozen constrained ID case count mismatch".into());
    }
    let inp = cases
        .iter()
        .map(|x| text(x, "case_id").map(str::to_owned))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let exp = expected
        .iter()
        .map(|x| text(x, "case_id").map(str::to_owned))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mid = ids
        .iter()
        .map(|x| {
            x.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "manifest ID must be text".to_owned())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if inp.len() != cases.len()
        || exp.len() != expected.len()
        || mid.len() != ids.len()
        || inp != exp
        || inp != mid
    {
        return Err("input/oracle/manifest identity mismatch".into());
    }
    Ok(())
}
fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            matches!((x.as_f64(),y.as_f64()),(Some(u),Some(v)) if (u-v).abs()<=1e-8*u.abs().max(v.abs()).max(1.0))
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
    for case in array(input, "cases")? {
        let id = text(case, "case_id")?;
        let actual = compute(field(case, "inputs")?)?;
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
        "schema_version":"constrained-kernel-id-t0-evaluation-v0.1",
        "verification_state":if mismatches.is_empty(){"MatchedFrozenOracle"}else{"OracleMismatch"},
        "evidence_class":"synthetic_only","evidence_ceiling":EVIDENCE_CEILING,
        "case_count":rows.len(),"matched_case_count":matched,"mismatches":mismatches,"results":rows,
        "nonclaims":[
            "This evaluator estimates a discrete kernel on synthetic linear regression fixtures only.",
            "The constrained estimator uses exact supplied per-group steady-gain constraints; it does not estimate physical P uncertainty.",
            "A full-rank design and converged optimizer do not validate upstream sensor calibration or model structure.",
            "No output qualifies a physical junction, tracer, sensor, potability or release authority."
        ]
    }))
}
fn digest(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}
fn read_json(p: &Path) -> Result<(Vec<u8>, Value), String> {
    let b = fs::read(p).map_err(|e| format!("read {}: {e}", p.display()))?;
    let v = serde_json::from_slice(&b).map_err(|e| format!("parse {}: {e}", p.display()))?;
    Ok((b, v))
}
fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 4 {
        return Err("usage: constrained-kernel-id-t0 [INPUT] [ORACLE] [MANIFEST] [RECEIPT]".into());
    }
    let ip =
        PathBuf::from(args.first().map(String::as_str).unwrap_or(
            "research/water-intrusion-coupling/constrained-kernel-id-t0-cases-v0.1.json",
        ));
    let op =
        PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
            "research/water-intrusion-coupling/constrained-kernel-id-t0-oracle-v0.1.json",
        ));
    let mp = PathBuf::from(args.get(2).map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/constrained-kernel-id-t0-manifest-v0.1.json",
    ));
    let rp = PathBuf::from(
        args.get(3)
            .map(String::as_str)
            .unwrap_or("artifacts/water-mixing-constrained-id-t0/receipt.json"),
    );
    let (ib, iv) = read_json(&ip)?;
    let (ob, ov) = read_json(&op)?;
    let (mb, mv) = read_json(&mp)?;
    let eval = evaluate(&iv, &ov, &mv)?;
    let success = eval["verification_state"] == "MatchedFrozenOracle";
    let revision = env::var("WATER_MIXING_CONSTRAINED_ID_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|x| {
            x.len() == 40
                && x.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        });
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock = fs::read(&lock_path).map_err(|e| format!("read {}: {e}", lock_path.display()))?;
    let receipt = json!({
        "schema_version":"constrained-kernel-id-t0-run-v0.1",
        "verification_state":if !success{"OracleMismatch"}else if revision.is_some(){"PassedExactRevision"}else{"PassedLocalUnpinned"},
        "implementation_revision":revision,"input_sha256":digest(&ib),"oracle_sha256":digest(&ob),
        "manifest_sha256":digest(&mb),"cargo_lock_sha256":digest(&lock),
        "case_count":eval["case_count"],"matched_case_count":eval["matched_case_count"],
        "mismatches":eval["mismatches"],"results":eval["results"],
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
            eprintln!("WATER_MIXING_CONSTRAINED_ID_T0_ERROR: {e}");
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
                "../../../../research/water-intrusion-coupling/constrained-kernel-id-t0-cases-v0.1.json"
            ),
            "oracle" => include_str!(
                "../../../../research/water-intrusion-coupling/constrained-kernel-id-t0-oracle-v0.1.json"
            ),
            "manifest" => include_str!(
                "../../../../research/water-intrusion-coupling/constrained-kernel-id-t0-manifest-v0.1.json"
            ),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(raw).expect("constrained identification fixture parses")
    }

    fn result() -> Value {
        evaluate(&load("input"), &load("oracle"), &load("manifest"))
            .expect("constrained identification T0 runs")
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
    fn all_eight_identification_cases_match_oracle() {
        let x = result();
        assert_eq!(x["case_count"], CASE_COUNT);
        assert_eq!(
            x["matched_case_count"], CASE_COUNT,
            "oracle mismatches: {}",
            x["mismatches"]
        );
        assert_eq!(x["verification_state"], "MatchedFrozenOracle");
    }

    #[test]
    fn constrained_estimator_beats_posthoc_clip_on_blind_fixture() {
        let x = row("CKI-T0-001-noisy-negative-fit");
        assert_eq!(x["identification_disposition"], "IDENTIFIED_SYNTHETIC");
        assert_eq!(x["blind_holdout_improvement_vs_clip"], true);
        assert!(x["clipped_gain_by_group"][0].as_f64().unwrap() > 1.0);
        assert!((x["constrained_gain_by_group"][0].as_f64().unwrap() - 0.8).abs() < 1e-8);
    }

    #[test]
    fn true_substochastic_kernel_is_not_forced_to_unit_gain() {
        let x = row("CKI-T0-002-substochastic-preserved");
        assert!((x["constrained_gain_by_group"][0].as_f64().unwrap() - 0.8).abs() < 1e-8);
        assert_eq!(x["total_training_weight"], 2.25);
        assert_eq!(x["total_holdout_weight"], 1.25);
        assert!(x["constraint_satisfied"].as_bool().unwrap());
    }

    #[test]
    fn rank_deficient_excitation_refuses_unique_identification() {
        assert_eq!(
            row("CKI-T0-003-rank-deficient")["identification_disposition"],
            "UNIDENTIFIABLE"
        );
    }

    #[test]
    fn ill_conditioned_full_rank_design_is_not_reported_as_robust() {
        assert_eq!(
            row("CKI-T0-004-ill-conditioned")["identification_disposition"],
            "ILL_CONDITIONED"
        );
    }

    #[test]
    fn static_zero_lag_kernel_is_recovered() {
        let x = row("CKI-T0-005-static-zero-lag");
        assert_eq!(x["identification_disposition"], "IDENTIFIED_SYNTHETIC");
        assert!(
            x["constrained_coefficients"]
                .as_array()
                .unwrap()
                .iter()
                .all(|v| v.as_f64().unwrap() >= 0.0)
        );
    }

    #[test]
    fn invalid_steady_gain_target_is_refused() {
        assert_eq!(
            row("CKI-T0-006-invalid-gain-target")["identification_disposition"],
            "INVALID_INPUT"
        );
    }

    #[test]
    fn unsupported_full_covariance_is_not_treated_as_diagonal() {
        assert_eq!(
            row("CKI-T0-007-unsupported-full-covariance")["identification_disposition"],
            "REJECTED_CONTRACT"
        );
    }

    #[test]
    fn nonpositive_sample_weight_is_refused() {
        assert_eq!(
            row("CKI-T0-008-invalid-sample-weight")["identification_disposition"],
            "INVALID_INPUT"
        );
    }
}
