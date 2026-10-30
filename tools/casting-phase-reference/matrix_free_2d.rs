//! Matrix-free 2D active-phase operator and Jacobi-PCG reference.
//!
//! This is a linear-algebra kernel for the implicit enthalpy flag-update method.
//! It does not perform flag iteration or thermal time evolution.
//! No production sparse matrix is assembled; dense assembly exists only in tests.
//!
//! Compile:
//!   rustc --edition=2024 -D warnings --test tools/casting-phase-reference/matrix_free_2d.rs -o /tmp/matrix-free-2d-tests
//!   rustc --edition=2024 -D warnings tools/casting-phase-reference/matrix_free_2d.rs -o /tmp/matrix-free-2d
//!   /tmp/matrix-free-2d --verify
//!   /tmp/matrix-free-2d --csv

const RELATIVE_TOLERANCE: f64 = 2.0e-11;
const MAX_ITERATIONS: usize = 20_000;
const EPSILON_SCALE: f64 = 64.0 * f64::EPSILON;

#[derive(Clone, Debug)]
struct GridOperator {
    nx: usize,
    ny: usize,
    dx_m: f64,
    dy_m: f64,
    dt_s: f64,
    // gamma > 0 means solid/liquid active; gamma == 0 means mixed (v=0).
    gamma: Vec<f64>,
}

impl GridOperator {
    fn validate(&self) -> Result<(), String> {
        if self.nx < 2 || self.ny < 2 {
            return Err("GRID_TOO_SMALL: require nx,ny >= 2".into());
        }
        if !self.dx_m.is_finite() || self.dx_m <= 0.0
            || !self.dy_m.is_finite() || self.dy_m <= 0.0
            || !self.dt_s.is_finite() || self.dt_s <= 0.0
        {
            return Err("GEOMETRY_INVALID: dx,dy,dt must be finite and positive".into());
        }
        let expected = self.nx.checked_mul(self.ny)
            .ok_or("GRID_OVERFLOW: nx*ny overflow")?;
        if self.gamma.len() != expected {
            return Err("FIELD_SIZE_MISMATCH: gamma length differs from nx*ny".into());
        }
        if self.gamma.iter().any(|g| !g.is_finite() || *g < 0.0) {
            return Err("GAMMA_INVALID: gamma must be finite and nonnegative".into());
        }
        if !self.gamma.iter().any(|g| *g > 0.0) {
            return Err("NO_ACTIVE_NODES: at least one non-mixed node is required".into());
        }
        Ok(())
    }

    fn len(&self) -> usize {
        self.nx * self.ny
    }

    fn active_nodes(&self) -> usize {
        self.gamma.iter().filter(|g| **g > 0.0).count()
    }

    fn mixed_nodes(&self) -> usize {
        self.gamma.len() - self.active_nodes()
    }

    fn conductance_x(&self) -> f64 {
        self.dt_s / (self.dx_m * self.dx_m)
    }

    fn conductance_y(&self) -> f64 {
        self.dt_s / (self.dy_m * self.dy_m)
    }

    fn diagonal_entry(&self, index: usize) -> Result<f64, String> {
        if index >= self.len() {
            return Err("INDEX_OUT_OF_RANGE".into());
        }
        let gamma = self.gamma[index];
        if gamma == 0.0 {
            return Ok(1.0); // Identity row enforces v=0 for mixed nodes.
        }

        let x = index % self.nx;
        let y = index / self.nx;
        let rx = self.conductance_x();
        let ry = self.conductance_y();

        let mut diagonal = gamma;
        if x == 0 {
            diagonal += 2.0 * rx; // West cold Dirichlet face, half-cell distance.
        } else {
            diagonal += rx;
        }
        if x + 1 < self.nx {
            diagonal += rx;
        }
        if y > 0 {
            diagonal += ry;
        }
        if y + 1 < self.ny {
            diagonal += ry;
        }
        if diagonal.is_finite() && diagonal > 0.0 {
            Ok(diagonal)
        } else {
            Err("DIAGONAL_INVALID: active diagonal must be finite and positive".into())
        }
    }

    fn apply(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
        self.validate()?;
        if x.iter().any(|v| !v.is_finite()) {
            return Err("INPUT_NONFINITE: matrix-free input contains NaN or infinity".into());
        }
        self.apply_validated(x, out)
    }

    fn apply_validated(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
        if x.len() != self.len() || out.len() != self.len() {
            return Err("VECTOR_SIZE_MISMATCH".into());
        }

        let rx = self.conductance_x();
        let ry = self.conductance_y();
        for index in 0..self.len() {
            let gamma = self.gamma[index];
            if gamma == 0.0 {
                if !x[index].is_finite() {
                    return Err("INPUT_NONFINITE: mixed-node vector entry is invalid".into());
                }
                out[index] = x[index];
                continue;
            }

            let x_index = index % self.nx;
            let y_index = index / self.nx;
            let mut value = gamma * x[index];

            if x_index == 0 {
                value += 2.0 * rx * x[index];
            } else {
                value += rx * x[index];
                let west = index - 1;
                if self.gamma[west] > 0.0 {
                    value -= rx * x[west];
                }
            }

            if x_index + 1 < self.nx {
                value += rx * x[index];
                let east = index + 1;
                if self.gamma[east] > 0.0 {
                    value -= rx * x[east];
                }
            }

            if y_index > 0 {
                value += ry * x[index];
                let south = index - self.nx;
                if self.gamma[south] > 0.0 {
                    value -= ry * x[south];
                }
            }

            if y_index + 1 < self.ny {
                value += ry * x[index];
                let north = index + self.nx;
                if self.gamma[north] > 0.0 {
                    value -= ry * x[north];
                }
            }

            if !value.is_finite() {
                return Err("OPERATOR_NONFINITE: matrix-free stencil produced invalid value".into());
            }
            out[index] = value;
        }
        Ok(())
    }

    fn diagonal(&self, out: &mut [f64]) -> Result<(), String> {
        if out.len() != self.len() {
            return Err("DIAGONAL_SIZE_MISMATCH".into());
        }
        for (index, value) in out.iter_mut().enumerate() {
            *value = self.diagonal_entry(index)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct SolveReport {
    solution: Vec<f64>,
    iterations: usize,
    operator_applications: usize,
    relative_residual: f64,
}

fn dot(a: &[f64], b: &[f64]) -> Result<f64, String> {
    if a.len() != b.len() {
        return Err("DOT_SIZE_MISMATCH".into());
    }
    let mut sum = 0.0_f64;
    for (left, right) in a.iter().zip(b.iter()) {
        sum += left * right;
    }
    if sum.is_finite() {
        Ok(sum)
    } else {
        Err("DOT_NONFINITE".into())
    }
}

fn norm(a: &[f64]) -> Result<f64, String> {
    let value = dot(a, a)?;
    if value < 0.0 {
        return Err("NORM_NEGATIVE".into());
    }
    Ok(value.sqrt())
}

fn pcg(
    operator: &GridOperator,
    rhs: &[f64],
    initial: Option<&[f64]>,
    relative_tolerance: f64,
    max_iterations: usize,
) -> Result<SolveReport, String> {
    operator.validate()?;
    let n = operator.len();
    if rhs.len() != n || initial.is_some_and(|x| x.len() != n) {
        return Err("PCG_VECTOR_SIZE_MISMATCH".into());
    }
    if rhs.iter().any(|v| !v.is_finite())
        || initial.is_some_and(|x| x.iter().any(|v| !v.is_finite()))
    {
        return Err("PCG_NONFINITE_INPUT".into());
    }
    let rhs_scale = rhs.iter().fold(1.0_f64, |scale, value| scale.max(value.abs()));
    for (index, gamma) in operator.gamma.iter().enumerate() {
        if *gamma == 0.0 && rhs[index].abs() > EPSILON_SCALE * rhs_scale {
            return Err("MIXED_ROW_RHS_NONZERO: mixed-phase rows must impose v=0".into());
        }
    }
    if !relative_tolerance.is_finite()
        || relative_tolerance <= 0.0
        || max_iterations == 0
    {
        return Err("PCG_SETTINGS_INVALID".into());
    }

    let mut x = initial.map_or_else(|| vec![0.0; n], |values| values.to_vec());
    for (index, gamma) in operator.gamma.iter().enumerate() {
        if *gamma == 0.0 {
            x[index] = 0.0;
        }
    }

    let mut ax = vec![0.0; n];
    operator.apply_validated(&x, &mut ax)?;
    let mut operator_applications = 1_usize;
    let mut residual = vec![0.0; n];
    for i in 0..n {
        residual[i] = rhs[i] - ax[i];
        if operator.gamma[i] == 0.0 {
            residual[i] = 0.0;
        }
    }

    let rhs_norm = norm(rhs)?;
    let residual_norm = norm(&residual)?;
    let normalizer = rhs_norm.max(1.0);
    let mut relative_residual = residual_norm / normalizer;
    if relative_residual <= relative_tolerance {
        return Ok(SolveReport {
            solution: x,
            iterations: 0,
            operator_applications,
            relative_residual,
        });
    }

    let mut diagonal = vec![0.0; n];
    operator.diagonal(&mut diagonal)?;
    let mut z = vec![0.0; n];
    for i in 0..n {
        if operator.gamma[i] > 0.0 {
            z[i] = residual[i] / diagonal[i];
        }
    }
    let mut direction = z.clone();
    drop(ax);
    let mut applied_direction = vec![0.0; n];
    let mut rho = dot(&residual, &z)?;
    if !rho.is_finite() || rho <= 0.0 {
        return Err("PCG_PRECONDITIONER_BREAKDOWN: r^T M^-1 r must be positive".into());
    }

    for iteration in 1..=max_iterations {
        operator.apply_validated(&direction, &mut applied_direction)?;
        operator_applications += 1;
        let curvature = dot(&direction, &applied_direction)?;
        let curvature_scale = norm(&direction)? * norm(&applied_direction)?;
        if !curvature.is_finite()
            || !curvature_scale.is_finite()
            || curvature <= EPSILON_SCALE * curvature_scale
        {
            return Err("PCG_NOT_POSITIVE_DEFINITE: p^T A p is nonpositive or numerically unsafe".into());
        }

        let alpha = rho / curvature;
        if !alpha.is_finite() {
            return Err("PCG_ALPHA_NONFINITE".into());
        }
        for i in 0..n {
            x[i] += alpha * direction[i];
            residual[i] -= alpha * applied_direction[i];
            if !x[i].is_finite() || !residual[i].is_finite() {
                return Err("PCG_ITERATE_NONFINITE".into());
            }
            if operator.gamma[i] == 0.0 {
                x[i] = 0.0;
                residual[i] = 0.0;
            }
        }

        relative_residual = norm(&residual)? / normalizer;
        if relative_residual <= relative_tolerance {
            return Ok(SolveReport {
                solution: x,
                iterations: iteration,
                operator_applications,
                relative_residual,
            });
        }

        for i in 0..n {
            z[i] = if operator.gamma[i] > 0.0 {
                residual[i] / diagonal[i]
            } else {
                0.0
            };
        }
        let rho_new = dot(&residual, &z)?;
        if !rho_new.is_finite() || rho_new <= 0.0 {
            return Err("PCG_RHO_BREAKDOWN".into());
        }
        let beta = rho_new / rho;
        if !beta.is_finite() {
            return Err("PCG_BETA_NONFINITE".into());
        }
        for i in 0..n {
            direction[i] = z[i] + beta * direction[i];
            if operator.gamma[i] == 0.0 {
                direction[i] = 0.0;
            }
        }
        rho = rho_new;
    }

    Err(format!(
        "PCG_NO_CONVERGENCE: residual {relative_residual:.6e} exceeds tolerance after {max_iterations} iterations"
    ))
}

fn fixture(nx: usize, ny: usize) -> GridOperator {
    let mut gamma = vec![0.0; nx * ny];
    for y in 0..ny {
        for x in 0..nx {
            let index = y * nx + x;
            // Mixed band plus solid/liquid active regions; values represent
            // inverse phase diffusivity in this dimensionless operator test.
            if (x + 2 * y) % 5 != 0 {
                gamma[index] = if (x + y) % 2 == 0 { 1.0e6 } else { 0.5e6 };
            }
        }
    }
    // Ensure a nonempty active sub-block at all admitted tiny sizes.
    gamma[0] = 1.0e6;
    GridOperator {
        nx,
        ny,
        dx_m: 0.01,
        dy_m: 0.0125,
        dt_s: 0.2,
        gamma,
    }
}

fn known_solution(operator: &GridOperator) -> Vec<f64> {
    let mut solution = vec![0.0; operator.len()];
    for (index, gamma) in operator.gamma.iter().enumerate() {
        if *gamma > 0.0 {
            let x = (index % operator.nx) as f64;
            let y = (index / operator.nx) as f64;
            solution[index] = 0.2 + 0.01 * x - 0.015 * y + 0.001 * x * y;
        }
    }
    solution
}

fn verify() -> Result<(), String> {
    for (nx, ny) in [(4, 4), (8, 8), (16, 12), (32, 24)] {
        let operator = fixture(nx, ny);
        operator.validate()?;
        let exact = known_solution(&operator);
        let mut rhs = vec![0.0; operator.len()];
        operator.apply(&exact, &mut rhs)?;
        let report = pcg(&operator, &rhs, None, RELATIVE_TOLERANCE, MAX_ITERATIONS)?;
        let mut error = vec![0.0; operator.len()];
        for i in 0..operator.len() {
            error[i] = report.solution[i] - exact[i];
        }
        let relative_error = norm(&error)? / norm(&exact)?.max(1.0);
        if relative_error > 2.0e-10 || report.relative_residual > RELATIVE_TOLERANCE {
            return Err(format!("PCG_KNOWN_ANSWER_FAILED: {nx}x{ny}, error={relative_error:.6e}"));
        }
        for (i, gamma) in operator.gamma.iter().enumerate() {
            if *gamma == 0.0 && report.solution[i] != 0.0 {
                return Err("MIXED_NODE_NOT_ZERO".into());
            }
        }
    }
    Ok(())
}

fn emit_csv() -> Result<(), String> {
    println!("nx,ny,cells,active_nodes,mixed_nodes,field_bytes_estimate,iterations,operator_applications,relative_residual,status");
    for (nx, ny) in [(64, 64), (128, 128), (256, 256), (512, 512)] {
        let operator = fixture(nx, ny);
        let exact = known_solution(&operator);
        let mut rhs = vec![0.0; operator.len()];
        operator.apply(&exact, &mut rhs)?;
        let report = pcg(&operator, &rhs, None, RELATIVE_TOLERANCE, MAX_ITERATIONS)?;
        let cells = operator.len();
        // gamma + rhs + x + r + z + p + q + diagonal: eight f64 arrays.
        let field_bytes = cells * 8 * std::mem::size_of::<f64>()
            + cells * std::mem::size_of::<u8>();
        println!(
            "{},{},{},{},{},{},{},{},{:.12e},PASS",
            nx,
            ny,
            cells,
            operator.active_nodes(),
            operator.mixed_nodes(),
            field_bytes,
            report.iterations,
            report.operator_applications,
            report.relative_residual
        );
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MATRIX_FREE_2D_OK: mask-aware SPD stencil and Jacobi-PCG")),
        [arg] if arg == "--verify" => verify().map(|()| println!("MATRIX_FREE_2D_OK: mask-aware SPD stencil and Jacobi-PCG")),
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: matrix-free-2d [--verify|--csv|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("MATRIX_FREE_2D_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dense_matrix(operator: &GridOperator) -> Vec<Vec<f64>> {
        let n = operator.len();
        let rx = operator.conductance_x();
        let ry = operator.conductance_y();
        let mut matrix = vec![vec![0.0; n]; n];

        for i in 0..n {
            if operator.gamma[i] == 0.0 {
                matrix[i][i] = 1.0;
                continue;
            }
            let x = i % operator.nx;
            let y = i / operator.nx;
            let mut diagonal = operator.gamma[i];

            if x == 0 {
                diagonal += 2.0 * rx;
            } else {
                diagonal += rx;
                let j = i - 1;
                if operator.gamma[j] > 0.0 { matrix[i][j] = -rx; }
            }
            if x + 1 < operator.nx {
                diagonal += rx;
                let j = i + 1;
                if operator.gamma[j] > 0.0 { matrix[i][j] = -rx; }
            }
            if y > 0 {
                diagonal += ry;
                let j = i - operator.nx;
                if operator.gamma[j] > 0.0 { matrix[i][j] = -ry; }
            }
            if y + 1 < operator.ny {
                diagonal += ry;
                let j = i + operator.nx;
                if operator.gamma[j] > 0.0 { matrix[i][j] = -ry; }
            }
            matrix[i][i] = diagonal;
        }
        matrix
    }

    fn dense_apply(matrix: &[Vec<f64>], x: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; x.len()];
        for i in 0..x.len() {
            for j in 0..x.len() {
                out[i] += matrix[i][j] * x[j];
            }
        }
        out
    }

    fn gaussian_solve(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Vec<f64> {
        let n = rhs.len();
        for col in 0..n {
            let mut pivot = col;
            for row in (col + 1)..n {
                if matrix[row][col].abs() > matrix[pivot][col].abs() {
                    pivot = row;
                }
            }
            matrix.swap(col, pivot);
            rhs.swap(col, pivot);
            let diagonal = matrix[col][col];
            assert!(diagonal.abs() > 1.0e-18);
            for j in col..n {
                matrix[col][j] /= diagonal;
            }
            rhs[col] /= diagonal;
            for row in 0..n {
                if row == col { continue; }
                let factor = matrix[row][col];
                for j in col..n {
                    matrix[row][j] -= factor * matrix[col][j];
                }
                rhs[row] -= factor * rhs[col];
            }
        }
        rhs
    }

    #[test]
    fn matrix_free_stencil_matches_independent_dense_assembly() {
        for (nx, ny) in [(3, 3), (4, 5), (8, 8)] {
            let operator = fixture(nx, ny);
            let matrix = dense_matrix(&operator);
            let x: Vec<f64> = (0..operator.len())
                .map(|i| if operator.gamma[i] == 0.0 { 0.0 } else { 0.03 * (i + 1) as f64 })
                .collect();
            let dense = dense_apply(&matrix, &x);
            let mut stencil = vec![0.0; operator.len()];
            operator.apply(&x, &mut stencil).unwrap();
            for i in 0..operator.len() {
                assert!((dense[i] - stencil[i]).abs() <= 2.0e-10 * dense[i].abs().max(1.0));
            }
        }
    }

    #[test]
    fn operator_is_symmetric_and_positive_on_admitted_vectors() {
        let operator = fixture(8, 7);
        let n = operator.len();
        let x: Vec<f64> = (0..n).map(|i| if operator.gamma[i] > 0.0 { (i as f64 + 1.0).sin() } else { 0.0 }).collect();
        let y: Vec<f64> = (0..n).map(|i| if operator.gamma[i] > 0.0 { (0.7 * i as f64 + 0.3).cos() } else { 0.0 }).collect();
        let mut ax = vec![0.0; n];
        let mut ay = vec![0.0; n];
        operator.apply(&x, &mut ax).unwrap();
        operator.apply(&y, &mut ay).unwrap();
        let lhs = dot(&x, &ay).unwrap();
        let rhs = dot(&y, &ax).unwrap();
        assert!((lhs - rhs).abs() < 1.0e-10 * lhs.abs().max(rhs.abs()).max(1.0));
        assert!(dot(&x, &ax).unwrap() > 0.0);
    }

    #[test]
    fn pcg_recovers_known_solution_on_multiple_masked_grids() {
        verify().unwrap();
    }

    #[test]
    fn dense_solver_agrees_with_matrix_free_pcg_on_tiny_grid() {
        let operator = fixture(4, 4);
        let matrix = dense_matrix(&operator);
        let exact = known_solution(&operator);
        let rhs = dense_apply(&matrix, &exact);
        let dense = gaussian_solve(matrix, rhs.clone());
        let pcg_report = pcg(&operator, &rhs, None, 1.0e-13, MAX_ITERATIONS).unwrap();
        for i in 0..exact.len() {
            assert!((dense[i] - pcg_report.solution[i]).abs() < 1.0e-9);
        }
    }

    #[test]
    fn mixed_nodes_are_exact_zero_constraints() {
        let operator = fixture(6, 6);
        let exact = known_solution(&operator);
        let mut rhs = vec![0.0; operator.len()];
        operator.apply(&exact, &mut rhs).unwrap();
        let result = pcg(&operator, &rhs, None, RELATIVE_TOLERANCE, MAX_ITERATIONS).unwrap();
        for (i, gamma) in operator.gamma.iter().enumerate() {
            if *gamma == 0.0 {
                assert_eq!(result.solution[i], 0.0);
            }
        }
    }

    #[test]
    fn invalid_geometry_masks_and_nonfinite_vectors_reject() {
        let mut bad = fixture(4, 4);
        bad.dx_m = 0.0;
        assert!(bad.validate().is_err());

        let mut bad = fixture(4, 4);
        bad.gamma[0] = -1.0;
        assert!(bad.validate().is_err());

        let bad = GridOperator {
            nx: 2, ny: 2, dx_m: 1.0, dy_m: 1.0, dt_s: 1.0, gamma: vec![0.0; 4]
        };
        assert!(bad.validate().is_err());

        let operator = fixture(4, 4);
        let mut x = vec![1.0; operator.len()];
        x[2] = f64::NAN;
        let mut y = vec![0.0; operator.len()];
        assert!(operator.apply(&x, &mut y).is_err());
    }

    #[test]
    fn nonzero_mixed_row_rhs_rejects() {
        let operator = fixture(4, 4);
        let exact = known_solution(&operator);
        let mut rhs = vec![0.0; operator.len()];
        operator.apply(&exact, &mut rhs).unwrap();
        let mixed = operator.gamma.iter().position(|gamma| *gamma == 0.0).unwrap();
        rhs[mixed] = 1.0;
        assert!(pcg(&operator, &rhs, None, RELATIVE_TOLERANCE, MAX_ITERATIONS).is_err());
    }

    #[test]
    fn exhausted_iteration_budget_rejects() {
        let operator = fixture(8, 8);
        let exact = known_solution(&operator);
        let mut rhs = vec![0.0; operator.len()];
        operator.apply(&exact, &mut rhs).unwrap();
        assert!(pcg(&operator, &rhs, None, 1.0e-15, 1).is_err());
    }
}
