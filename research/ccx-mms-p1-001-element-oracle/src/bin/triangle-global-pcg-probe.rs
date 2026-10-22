//! Compare unpreconditioned CG with symmetric Jacobi-preconditioned CG on
//! small oracle-assembled, boundary-constrained CST stiffness systems.
//!
//! This isolates linear-solver behavior. It does not run CalculiX, measure
//! discretization error, or establish that Jacobi preconditioning is superior
//! for general meshes or production problem sizes.

#![allow(clippy::needless_range_loop)]

use ccx_mms_p1_element_oracle::{Point2, TriangleElement};
use ccx_mms_p1_mesh::generate_mesh;
use ccx_mms_p1_oracle::PlaneStrainMaterial;
use std::env;
use std::process::ExitCode;

type Matrix = Vec<Vec<f64>>;

const RELATIVE_RESIDUAL_TOLERANCE: f64 = 1.0e-10;

#[derive(Clone, Copy, Debug)]
struct SolveResult {
    iterations: usize,
    relative_residual: f64,
}

#[derive(Clone, Copy, Debug)]
struct SweepPoint {
    mesh_family: &'static str,
    cells_per_side: usize,
    free_dof_count: usize,
    condition_number_unscaled: f64,
    condition_number_jacobi_scaled: f64,
    cg: SolveResult,
    pcg: SolveResult,
}

fn is_lowercase_sha40(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(a, b)| a * b)
        .sum::<f64>()
}

fn norm(values: &[f64]) -> f64 {
    values.iter().fold(0.0_f64, |acc, value| acc.hypot(*value))
}

fn mat_vec(matrix: &Matrix, vector: &[f64]) -> Result<Vec<f64>, String> {
    if matrix.is_empty()
        || matrix.len() != vector.len()
        || matrix.iter().any(|row| row.len() != vector.len())
        || matrix.iter().flatten().any(|value| !value.is_finite())
        || vector.iter().any(|value| !value.is_finite())
    {
        return Err("matrix/vector shape or finite-value check failed".to_owned());
    }

    let result: Vec<f64> = matrix
        .iter()
        .map(|row| row.iter().zip(vector).map(|(a, x)| a * x).sum())
        .collect();
    if result.iter().any(|value: &f64| !value.is_finite()) {
        return Err("matrix-vector product is non-finite".to_owned());
    }
    Ok(result)
}

/// Assemble K_ff after eliminating both displacement DOFs on the fixed boundary.
fn assemble_constrained_stiffness(
    cells_per_side: usize,
    quadratically_graded: bool,
) -> Result<Matrix, String> {
    let mut mesh = generate_mesh(cells_per_side)
        .map_err(|error| format!("mesh generation failed: {error:?}"))?;
    if quadratically_graded {
        // A monotone coordinate map clusters nodes toward the origin while
        // preserving the unit-square boundary and structured connectivity.
        // This provides a deterministic nonuniform-diagonal case for Jacobi scaling.
        for node in &mut mesh.nodes {
            node.x *= node.x;
            node.y *= node.y;
        }
    }
    let mut reduced_map = vec![None; 2 * mesh.nodes.len()];
    let mut free_dof_count = 0_usize;

    for node in &mesh.nodes {
        if node.boundary {
            continue;
        }
        let full_x = 2 * (node.id - 1);
        reduced_map[full_x] = Some(free_dof_count);
        free_dof_count += 1;
        reduced_map[full_x + 1] = Some(free_dof_count);
        free_dof_count += 1;
    }
    if free_dof_count == 0 {
        return Err("mesh has no free degrees of freedom".to_owned());
    }

    let mut stiffness = vec![vec![0.0_f64; free_dof_count]; free_dof_count];
    let material = PlaneStrainMaterial::new(1.0, 0.3)
        .map_err(|error| format!("reference material is invalid: {error:?}"))?;

    for triangle in &mesh.triangles {
        let vertices = triangle.nodes.map(|node_id| {
            let node = mesh.nodes[node_id - 1];
            Point2 { x: node.x, y: node.y }
        });
        let element = TriangleElement::new(vertices, material, 1.0)
            .map_err(|error| format!("element construction failed: {error}"))?;

        for local_i in 0..6 {
            let node_i = triangle.nodes[local_i / 2] - 1;
            let full_i = 2 * node_i + local_i % 2;
            let Some(global_i) = reduced_map[full_i] else {
                continue;
            };
            for local_j in 0..6 {
                let node_j = triangle.nodes[local_j / 2] - 1;
                let full_j = 2 * node_j + local_j % 2;
                let Some(global_j) = reduced_map[full_j] else {
                    continue;
                };
                stiffness[global_i][global_j] += element.stiffness[local_i][local_j];
            }
        }
    }

    if stiffness.iter().flatten().any(|value| !value.is_finite()) {
        return Err("assembled stiffness has non-finite entries".to_owned());
    }
    for i in 0..free_dof_count {
        if stiffness[i][i] <= 0.0 {
            return Err(format!("non-positive stiffness diagonal at DOF {i}"));
        }
        for j in 0..free_dof_count {
            let scale = stiffness[i][j].abs().max(stiffness[j][i].abs()).max(1.0);
            if (stiffness[i][j] - stiffness[j][i]).abs() > 1.0e-12 * scale {
                return Err(format!("assembled stiffness is not symmetric at ({i}, {j})"));
            }
        }
    }
    Ok(stiffness)
}

/// Return D^(-1/2) K D^(-1/2), the symmetric matrix for Jacobi scaling.
/// This allows its spectral condition number to be computed without using a
/// nonsymmetric left-preconditioned matrix D^(-1) K.
fn jacobi_scaled_matrix(matrix: &Matrix) -> Result<Matrix, String> {
    let n = matrix.len();
    if n == 0
        || matrix.iter().any(|row| row.len() != n)
        || matrix.iter().flatten().any(|value| !value.is_finite())
    {
        return Err("invalid stiffness matrix shape or values".to_owned());
    }

    let mut diagonal_roots = Vec::with_capacity(n);
    for i in 0..n {
        if !matrix[i][i].is_finite() || matrix[i][i] <= 0.0 {
            return Err(format!("Jacobi preconditioner requires positive diagonal at {i}"));
        }
        diagonal_roots.push(matrix[i][i].sqrt());
    }

    let mut scaled = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..n {
            scaled[i][j] = (matrix[i][j] / diagonal_roots[i]) / diagonal_roots[j];
        }
    }
    if scaled.iter().flatten().any(|value| !value.is_finite()) {
        return Err("Jacobi-scaled matrix contains non-finite values".to_owned());
    }
    Ok(scaled)
}

/// Dense symmetric Jacobi eigensolver, used only for these small verification
/// matrices. This is not a scalable production eigensolver.
fn symmetric_eigenvalues(matrix: &Matrix) -> Result<Vec<f64>, String> {
    let n = matrix.len();
    if n == 0
        || matrix.iter().any(|row| row.len() != n)
        || matrix.iter().flatten().any(|value| !value.is_finite())
    {
        return Err("invalid matrix for eigensolver".to_owned());
    }
    let mut a = matrix.clone();
    for i in 0..n {
        for j in (i + 1)..n {
            let average = 0.5 * (a[i][j] + a[j][i]);
            a[i][j] = average;
            a[j][i] = average;
        }
    }

    let max_rotations = 64_usize
        .checked_mul(n)
        .and_then(|value| value.checked_mul(n))
        .ok_or_else(|| "eigensolver iteration budget overflow".to_owned())?;
    for _ in 0..max_rotations {
        let mut pivot = (0_usize, 0_usize);
        let mut largest_off_diagonal = 0.0_f64;
        let mut scale = 0.0_f64;
        for i in 0..n {
            scale = scale.max(a[i][i].abs());
            for j in (i + 1)..n {
                if a[i][j].abs() > largest_off_diagonal {
                    largest_off_diagonal = a[i][j].abs();
                    pivot = (i, j);
                }
            }
        }
        if !scale.is_finite() || !largest_off_diagonal.is_finite() {
            return Err("non-finite Jacobi state".to_owned());
        }
        if largest_off_diagonal <= 32.0 * f64::EPSILON * scale.max(f64::MIN_POSITIVE) {
            let mut values: Vec<f64> = (0..n).map(|index| a[index][index]).collect();
            if values.iter().any(|value| !value.is_finite()) {
                return Err("non-finite eigenvalue".to_owned());
            }
            values.sort_by(f64::total_cmp);
            return Ok(values);
        }

        let (p, q) = pivot;
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        if apq == 0.0 {
            continue;
        }
        let tau = (aqq - app) / (2.0 * apq);
        if !tau.is_finite() {
            return Err("non-finite Jacobi rotation parameter".to_owned());
        }
        let t = if tau >= 0.0 {
            1.0 / (tau + tau.hypot(1.0))
        } else {
            -1.0 / (-tau + tau.hypot(1.0))
        };
        let cosine = 1.0 / (1.0 + t * t).sqrt();
        let sine = t * cosine;
        a[p][p] = app - t * apq;
        a[q][q] = aqq + t * apq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for k in 0..n {
            if k == p || k == q {
                continue;
            }
            let akp = a[k][p];
            let akq = a[k][q];
            let new_kp = cosine * akp - sine * akq;
            let new_kq = sine * akp + cosine * akq;
            a[k][p] = new_kp;
            a[p][k] = new_kp;
            a[k][q] = new_kq;
            a[q][k] = new_kq;
        }
    }
    Err("symmetric Jacobi eigensolver reached iteration limit".to_owned())
}

fn spectral_condition_number(matrix: &Matrix) -> Result<f64, String> {
    let eigenvalues = symmetric_eigenvalues(matrix)?;
    let smallest = *eigenvalues
        .first()
        .ok_or_else(|| "empty eigenvalue spectrum".to_owned())?;
    let largest = *eigenvalues
        .last()
        .ok_or_else(|| "empty eigenvalue spectrum".to_owned())?;
    if !smallest.is_finite()
        || !largest.is_finite()
        || smallest <= 0.0
        || largest < smallest
    {
        return Err(format!(
            "matrix is not numerically SPD: min={smallest:e}, max={largest:e}"
        ));
    }
    let condition = largest / smallest;
    if !condition.is_finite() || condition < 1.0 {
        return Err("invalid spectral condition number".to_owned());
    }
    Ok(condition)
}

fn conjugate_gradient(
    matrix: &Matrix,
    rhs: &[f64],
    jacobi_preconditioned: bool,
) -> Result<SolveResult, String> {
    let n = matrix.len();
    if n == 0
        || rhs.len() != n
        || matrix.iter().any(|row| row.len() != n)
        || matrix.iter().flatten().any(|value| !value.is_finite())
        || rhs.iter().any(|value| !value.is_finite())
    {
        return Err("invalid linear system".to_owned());
    }
    let rhs_norm = norm(rhs);
    if !rhs_norm.is_finite() || rhs_norm <= 0.0 {
        return Err("right-hand side must have a positive finite norm".to_owned());
    }
    let diagonal: Vec<f64> = (0..n).map(|i| matrix[i][i]).collect();
    if jacobi_preconditioned
        && diagonal
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err("Jacobi preconditioner requires positive finite diagonal".to_owned());
    }

    let mut solution = vec![0.0_f64; n];
    let mut residual = rhs.to_vec();
    let mut z: Vec<f64> = (0..n)
        .map(|i| if jacobi_preconditioned { residual[i] / diagonal[i] } else { residual[i] })
        .collect();
    let mut direction = z.clone();
    let mut rho = dot(&residual, &z);
    if !rho.is_finite() || rho <= 0.0 {
        return Err("initial CG inner product is invalid".to_owned());
    }
    let max_iterations = n.saturating_mul(4).max(32);

    for iteration in 0..=max_iterations {
        let relative = norm(&residual) / rhs_norm;
        if relative <= RELATIVE_RESIDUAL_TOLERANCE {
            let product = mat_vec(matrix, &solution)?;
            let true_residual: Vec<f64> = rhs
                .iter()
                .zip(product)
                .map(|(b, ax)| b - ax)
                .collect();
            let true_relative = norm(&true_residual) / rhs_norm;
            if true_relative.is_finite() && true_relative <= RELATIVE_RESIDUAL_TOLERANCE {
                return Ok(SolveResult {
                    iterations: iteration,
                    relative_residual: true_relative,
                });
            }
        }
        if iteration == max_iterations {
            break;
        }

        let applied = mat_vec(matrix, &direction)?;
        let denominator = dot(&direction, &applied);
        if !denominator.is_finite() || denominator <= 0.0 {
            return Err("CG encountered a non-positive search curvature".to_owned());
        }
        let alpha = rho / denominator;
        if !alpha.is_finite() {
            return Err("CG step length is non-finite".to_owned());
        }
        for i in 0..n {
            solution[i] += alpha * direction[i];
            residual[i] -= alpha * applied[i];
        }
        if solution.iter().any(|value| !value.is_finite())
            || residual.iter().any(|value| !value.is_finite())
        {
            return Err("CG iterate became non-finite".to_owned());
        }
        if norm(&residual) / rhs_norm <= RELATIVE_RESIDUAL_TOLERANCE {
            let product = mat_vec(matrix, &solution)?;
            let true_residual: Vec<f64> = rhs
                .iter()
                .zip(product)
                .map(|(b, ax)| b - ax)
                .collect();
            let true_relative = norm(&true_residual) / rhs_norm;
            if true_relative.is_finite() && true_relative <= RELATIVE_RESIDUAL_TOLERANCE {
                return Ok(SolveResult {
                    iterations: iteration + 1,
                    relative_residual: true_relative,
                });
            }
        }

        z = (0..n)
            .map(|i| if jacobi_preconditioned { residual[i] / diagonal[i] } else { residual[i] })
            .collect();
        let new_rho = dot(&residual, &z);
        if !new_rho.is_finite() || new_rho <= 0.0 {
            return Err("CG residual inner product is invalid".to_owned());
        }
        let beta = new_rho / rho;
        if !beta.is_finite() {
            return Err("CG direction coefficient is non-finite".to_owned());
        }
        for i in 0..n {
            direction[i] = z[i] + beta * direction[i];
        }
        rho = new_rho;
    }
    Err(format!(
        "{}CG did not achieve relative residual <= {RELATIVE_RESIDUAL_TOLERANCE:e}",
        if jacobi_preconditioned { "preconditioned " } else { "" }
    ))
}

fn run_sweep() -> Result<Vec<SweepPoint>, String> {
    let mut points = Vec::with_capacity(10);
    for (mesh_family, quadratically_graded) in [("uniform", false), ("quadratic-graded", true)] {
        for cells_per_side in [2, 3, 4, 5, 6] {
            let matrix = assemble_constrained_stiffness(cells_per_side, quadratically_graded)?;
            let n = matrix.len();
            let scaled = jacobi_scaled_matrix(&matrix)?;
            let condition_number_unscaled = spectral_condition_number(&matrix)?;
            let condition_number_jacobi_scaled = spectral_condition_number(&scaled)?;

            // A deterministic, nontrivial RHS provides a linear-solver diagnostic
            // independent of a selected physical load vector.
            let rhs: Vec<f64> = (0..n)
                .map(|i| {
                    let coordinate = (i + 1) as f64;
                    coordinate.sin() + 0.25 * (0.37 * coordinate).cos()
                })
                .collect();
            let cg = conjugate_gradient(&matrix, &rhs, false)?;
            let pcg = conjugate_gradient(&matrix, &rhs, true)?;
            if cg.relative_residual > RELATIVE_RESIDUAL_TOLERANCE
                || pcg.relative_residual > RELATIVE_RESIDUAL_TOLERANCE
                || cg.iterations > n.saturating_mul(4).max(32)
                || pcg.iterations > n.saturating_mul(4).max(32)
            {
                return Err("linear solver residual/iteration gate failed".to_owned());
            }
            points.push(SweepPoint {
                mesh_family,
                cells_per_side,
                free_dof_count: n,
                condition_number_unscaled,
                condition_number_jacobi_scaled,
                cg,
                pcg,
            });
        }
    }
    Ok(points)
}

fn main() -> ExitCode {
    let source_sha = env::var("MMS_BENCH_SOURCE_SHA").unwrap_or_default();
    let checkout_sha = env::var("MMS_BENCH_CHECKOUT_SHA").unwrap_or_default();
    if !is_lowercase_sha40(&source_sha) || !is_lowercase_sha40(&checkout_sha) {
        eprintln!(
            "MMS_BENCH_SOURCE_SHA and MMS_BENCH_CHECKOUT_SHA must be lowercase 40-character Git SHAs"
        );
        return ExitCode::FAILURE;
    }

    let sweep = match run_sweep() {
        Ok(points) => points,
        Err(error) => {
            eprintln!("global PCG probe failed closed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let uniform_condition_control_pass = sweep
        .iter()
        .filter(|point| point.mesh_family == "uniform")
        .all(|point| {
            let ratio = point.condition_number_jacobi_scaled / point.condition_number_unscaled;
            (0.95..=1.05).contains(&ratio)
        });
    let graded_condition_improvement_pass = sweep.iter().any(|point| {
        point.mesh_family == "quadratic-graded"
            && point.cells_per_side == 6
            && point.condition_number_jacobi_scaled
                < 0.75 * point.condition_number_unscaled
    });
    let solver_diagnostics_pass = sweep.len() == 10
        && uniform_condition_control_pass
        && graded_condition_improvement_pass
        && sweep.iter().all(|point| {
            point.condition_number_unscaled.is_finite()
                && point.condition_number_unscaled >= 1.0
                && point.condition_number_jacobi_scaled.is_finite()
                && point.condition_number_jacobi_scaled >= 1.0
                && point.cg.relative_residual <= RELATIVE_RESIDUAL_TOLERANCE
                && point.pcg.relative_residual <= RELATIVE_RESIDUAL_TOLERANCE
        });

    let records = sweep
        .iter()
        .map(|point| {
            format!(
                concat!(
                    "{{\"mesh_family\":\"{}\",\"cells_per_side\":{},\"free_dof_count\":{},",
                    "\"condition_number_unscaled\":{:.17e},",
                    "\"condition_number_jacobi_scaled\":{:.17e},",
                    "\"cg_iterations\":{},\"cg_relative_residual\":{:.17e},",
                    "\"pcg_iterations\":{},\"pcg_relative_residual\":{:.17e}}}"
                ),
                point.mesh_family,
                point.cells_per_side,
                point.free_dof_count,
                point.condition_number_unscaled,
                point.condition_number_jacobi_scaled,
                point.cg.iterations,
                point.cg.relative_residual,
                point.pcg.iterations,
                point.pcg.relative_residual
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    println!(
        concat!(
            "{{\"schema\":\"ccx-mms-p1-global-pcg-v1\",",
            "\"source_sha\":\"{}\",\"checkout_sha\":\"{}\",",
            "\"solver_executed\":false,\"qualification_pass\":false,",
            "\"linear_solver\":\"independent dense CG/PCG\",",
            "\"relative_residual_tolerance\":{:.1e},",
            "\"solver_diagnostics_pass\":{},\"sweep\":[{}]}}"
        ),
        source_sha,
        checkout_sha,
        RELATIVE_RESIDUAL_TOLERANCE,
        solver_diagnostics_pass,
        records
    );

    if solver_diagnostics_pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symmetric_jacobi_scaling_preserves_spd_and_has_finite_spectrum() {
        let matrix = vec![
            vec![4.0, -1.0, 0.0],
            vec![-1.0, 3.0, -1.0],
            vec![0.0, -1.0, 2.0],
        ];
        let scaled = jacobi_scaled_matrix(&matrix).unwrap();
        let condition = spectral_condition_number(&scaled).unwrap();
        assert!(condition.is_finite() && condition >= 1.0);
        assert!((scaled[0][0] - 1.0).abs() <= 1.0e-15);
        assert!((scaled[1][1] - 1.0).abs() <= 1.0e-15);
        assert!((scaled[2][2] - 1.0).abs() <= 1.0e-15);
    }

    #[test]
    fn preconditioner_fails_closed_on_nonpositive_diagonal() {
        let invalid = vec![vec![1.0, 0.0], [0.0, 0.0].to_vec()];
        assert!(jacobi_scaled_matrix(&invalid).is_err());
        let rhs = [1.0, 1.0];
        assert!(conjugate_gradient(&invalid, &rhs, true).is_err());
    }

    #[test]
    fn cg_and_pcg_recover_a_known_small_spd_solution_to_residual_tolerance() {
        let matrix = vec![
            vec![4.0, -1.0, 0.0],
            vec![-1.0, 3.0, -1.0],
            vec![0.0, -1.0, 2.0],
        ];
        let exact = [0.5, -1.25, 2.0];
        let rhs = mat_vec(&matrix, &exact).unwrap();
        let cg = conjugate_gradient(&matrix, &rhs, false).unwrap();
        let pcg = conjugate_gradient(&matrix, &rhs, true).unwrap();
        assert!(cg.relative_residual <= RELATIVE_RESIDUAL_TOLERANCE);
        assert!(pcg.relative_residual <= RELATIVE_RESIDUAL_TOLERANCE);
        assert!(cg.iterations <= 3);
        assert!(pcg.iterations <= 3);
    }

    #[test]
    fn assembled_global_solver_sweep_is_repeatable_and_residual_bounded() {
        let first = run_sweep().unwrap();
        let second = run_sweep().unwrap();
        assert_eq!(first.len(), 5);
        assert_eq!(first.len(), second.len());
        for (left, right) in first.iter().zip(&second) {
            assert_eq!(left.cells_per_side, right.cells_per_side);
            assert_eq!(left.free_dof_count, right.free_dof_count);
            assert_eq!(left.cg.iterations, right.cg.iterations);
            assert_eq!(left.pcg.iterations, right.pcg.iterations);
            assert!(left.cg.relative_residual <= RELATIVE_RESIDUAL_TOLERANCE);
            assert!(left.pcg.relative_residual <= RELATIVE_RESIDUAL_TOLERANCE);
            assert!(left.condition_number_unscaled.is_finite());
            assert!(left.condition_number_jacobi_scaled.is_finite());
        }
    }
}
