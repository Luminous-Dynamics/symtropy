// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Matched mass-Riesz kernel benchmark for the pinned dumbbell fixture.
//!
//! This runner intentionally measures only M a = f and its metric/covector
//! invariants. It does not execute the full constrained midpoint step and does
//! not execute Drake, MuJoCo, or Pinocchio.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::io;
use std::process::Command;
use std::time::Instant;

use metric_induced_error_envelope_reference::{
    CartesianQ, Cotangent, DenseY, Matrix, Metric, MetricProfileV1, Rebasis, Tangent, Vector,
    pairing, vector_euclidean, vector_inf,
};

const N: usize = 6;
const WARMUP: usize = 200;
const ITERATIONS: usize = 500;
const SAMPLES: usize = 21;
const PIVOT_TOLERANCE: f64 = 64.0 * f64::EPSILON;
const CASE_COMMIT: &str = "b9fcf688b502d602eac91297880bf3a3e484545f";
const CASE_PATH: &str =
    "research/generalized-coordinate-covariance-001/fixtures/covariance_case_v0.1.json";
const CASE_BLOB: &str = "9c4b5eb25680f126fbcdb7307ec15a87fff8f284";

#[derive(Debug, Clone, Copy)]
struct Lu6 {
    lu: Matrix<N>,
    permutation: [usize; N],
}

impl Lu6 {
    fn factor(matrix: Matrix<N>) -> Result<Self, io::Error> {
        let scale = matrix_inf(&matrix);
        if !scale.is_finite() || scale == 0.0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "zero or non-finite matrix scale",
            ));
        }
        if matrix.iter().flatten().any(|value| !value.is_finite()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "non-finite matrix input",
            ));
        }

        let threshold = PIVOT_TOLERANCE * scale;
        if !threshold.is_finite() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "non-finite pivot threshold",
            ));
        }

        let mut lu = matrix;
        let mut permutation = std::array::from_fn(|index| index);

        // Index-based elimination is intentional: row pivots and triangular
        // updates must address the same dense matrix coordinates.
        #[allow(clippy::needless_range_loop)]
        for column in 0..N {
            let pivot_row = (column..N)
                .max_by(|left, right| lu[*left][column].abs().total_cmp(&lu[*right][column].abs()))
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "empty pivot range"))?;
            let pivot = lu[pivot_row][column].abs();
            if !pivot.is_finite() || pivot <= threshold {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("singular or out-of-profile pivot at column {column}"),
                ));
            }

            if pivot_row != column {
                lu.swap(pivot_row, column);
                permutation.swap(pivot_row, column);
            }

            for row in (column + 1)..N {
                lu[row][column] /= lu[column][column];
                for trailing in (column + 1)..N {
                    lu[row][trailing] -= lu[row][column] * lu[column][trailing];
                }
            }
        }

        if lu.iter().flatten().any(|value| !value.is_finite()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "non-finite LU intermediate",
            ));
        }

        Ok(Self { lu, permutation })
    }

    fn solve(&self, rhs: &Vector<N>) -> Result<Vector<N>, io::Error> {
        if rhs.iter().any(|value| !value.is_finite()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "non-finite right-hand side",
            ));
        }
        let mut solution = std::array::from_fn(|row| rhs[self.permutation[row]]);

        // Forward and backward substitution are deliberately separate from
        // the Cholesky/Riesz implementation in the depended-on reference crate.
        #[allow(clippy::needless_range_loop)]
        for row in 0..N {
            for column in 0..row {
                solution[row] -= self.lu[row][column] * solution[column];
            }
        }
        #[allow(clippy::needless_range_loop)]
        for row in (0..N).rev() {
            for column in (row + 1)..N {
                solution[row] -= self.lu[row][column] * solution[column];
            }
            solution[row] /= self.lu[row][row];
        }

        if solution.iter().any(|value| !value.is_finite()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "non-finite LU solution",
            ));
        }
        Ok(solution)
    }
}

#[derive(Debug, Clone, Copy)]
struct Timing {
    median_ns: f64,
    p95_ns: f64,
    p99_ns: f64,
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    let index = ((sorted.len() - 1) as f64 * fraction).ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn measure<F>(mut operation: F) -> Timing
where
    F: FnMut() -> f64,
{
    let mut warmup_checksum = 0.0;
    for _ in 0..WARMUP {
        warmup_checksum += black_box(operation());
    }
    black_box(warmup_checksum);

    let mut timings = Vec::with_capacity(SAMPLES);
    let mut checksum = 0.0;
    for _ in 0..SAMPLES {
        let start = Instant::now();
        let mut sample_checksum = 0.0;
        for _ in 0..ITERATIONS {
            sample_checksum += black_box(operation());
        }
        timings.push(start.elapsed().as_secs_f64() * 1.0e9 / ITERATIONS as f64);
        checksum += sample_checksum;
    }
    black_box(checksum);
    timings.sort_by(f64::total_cmp);

    Timing {
        median_ns: percentile(&timings, 0.50),
        p95_ns: percentile(&timings, 0.95),
        p99_ns: percentile(&timings, 0.99),
    }
}

fn matrix_inf(matrix: &Matrix<N>) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn matrix_apply(matrix: &Matrix<N>, vector: &Vector<N>) -> Vector<N> {
    std::array::from_fn(|row| {
        matrix[row]
            .iter()
            .zip(vector.iter())
            .map(|(left, right)| left * right)
            .sum()
    })
}

fn matrix_transpose(matrix: &Matrix<N>) -> Matrix<N> {
    std::array::from_fn(|row| std::array::from_fn(|column| matrix[column][row]))
}

fn matrix_multiply(left: &Matrix<N>, right: &Matrix<N>) -> Matrix<N> {
    let mut result = [[0.0; N]; N];
    for (row, output_row) in result.iter_mut().enumerate() {
        for (column, output) in output_row.iter_mut().enumerate() {
            for (inner, left_value) in left[row].iter().enumerate() {
                *output += left_value * right[inner][column];
            }
        }
    }
    result
}

fn vector_sub(left: &Vector<N>, right: &Vector<N>) -> Vector<N> {
    std::array::from_fn(|index| left[index] - right[index])
}

fn vector_difference_l2(left: &Vector<N>, right: &Vector<N>) -> f64 {
    vector_euclidean(&vector_sub(left, right))
}

fn relative_difference(left: f64, right: f64) -> f64 {
    (left - right).abs() / 1.0_f64.max(left.abs()).max(right.abs())
}

fn matrix_relative_defect(left: &Matrix<N>, right: &Matrix<N>) -> f64 {
    let mut maximum = 0.0_f64;
    for (left_row, right_row) in left.iter().zip(right.iter()) {
        for (left_value, right_value) in left_row.iter().zip(right_row.iter()) {
            maximum = maximum.max((left_value - right_value).abs());
        }
    }
    maximum / 1.0_f64.max(matrix_inf(left)).max(matrix_inf(right))
}

fn normwise_backward_error(matrix: &Matrix<N>, solution: &Vector<N>, rhs: &Vector<N>) -> f64 {
    let residual = vector_sub(rhs, &matrix_apply(matrix, solution));
    let scale = matrix_inf(matrix) * vector_inf(solution) + vector_inf(rhs);
    if scale == 0.0 {
        if vector_inf(&residual) == 0.0 {
            0.0
        } else {
            f64::INFINITY
        }
    } else {
        vector_inf(&residual) / scale
    }
}

fn componentwise_backward_error(matrix: &Matrix<N>, solution: &Vector<N>, rhs: &Vector<N>) -> f64 {
    let residual = vector_sub(rhs, &matrix_apply(matrix, solution));
    let mut maximum = 0.0_f64;
    for (row, matrix_row) in matrix.iter().enumerate() {
        let denominator = rhs[row].abs()
            + matrix_row
                .iter()
                .zip(solution.iter())
                .map(|(coefficient, value)| coefficient.abs() * value.abs())
                .sum::<f64>();
        let numerator = residual[row].abs();
        if denominator == 0.0 {
            if numerator != 0.0 {
                return f64::INFINITY;
            }
        } else {
            maximum = maximum.max(numerator / denominator);
        }
    }
    maximum
}

fn mass_matrix() -> Matrix<N> {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            if row == column {
                if row < 3 { 1.3 } else { 2.1 }
            } else {
                0.0
            }
        })
    })
}

fn basis_matrix(family: usize) -> Matrix<N> {
    match family {
        0 => [
            [1.0, 1.0, 1.0, -1.0, 1.0, 1.0],
            [1.0, 2.0, 2.0, 0.0, 0.0, 2.0],
            [1.0, 2.0, 3.0, 1.0, 1.0, 1.0],
            [-1.0, 0.0, 1.0, 4.0, 0.0, 0.0],
            [1.0, 0.0, 1.0, 0.0, 5.0, 1.0],
            [1.0, 2.0, 1.0, 0.0, 1.0, 6.0],
        ],
        2 => [
            [1.0, -1.0, 1.0, 1.0, -1.0, 1.0],
            [-1.0, 2.0, -2.0, 0.0, 2.0, -2.0],
            [1.0, -2.0, 3.0, -1.0, -1.0, 3.0],
            [1.0, 0.0, -1.0, 4.0, -2.0, 0.0],
            [-1.0, 2.0, -1.0, -2.0, 5.0, -3.0],
            [1.0, -2.0, 3.0, 0.0, -3.0, 6.0],
        ],
        _ => panic!("unfrozen coordinate family {family}"),
    }
}

fn condition_metadata(family: usize) -> (f64, f64) {
    match family {
        0 => (387.32736441766446, 206_371.94480768955),
        2 => (210.94105538055962, 60_244.438948182105),
        _ => unreachable!("unfrozen coordinate family"),
    }
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control.is_control() => escaped.push(' '),
            ordinary => escaped.push(ordinary),
        }
    }
    escaped
}

fn json_quote(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

fn json_number(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.17e}")
    } else {
        "null".to_owned()
    }
}

fn source_commit() -> Option<String> {
    std::env::var("METRIC_BENCHMARK_SOURCE_SHA")
        .ok()
        .filter(|value| value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn cpu_model() -> String {
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo")
        && let Some(model) = cpuinfo.lines().find_map(|line| {
            line.strip_prefix("model name")
                .or_else(|| line.strip_prefix("Hardware"))
                .and_then(|value| value.split_once(':').map(|(_, item)| item.trim()))
        })
    {
        return model.to_owned();
    }
    std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "not-exposed".to_owned())
}

fn compiler_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".to_owned())
}

struct RunMetrics {
    tangent_drift: f64,
    cotangent_drift: f64,
    pairing_drift: f64,
    congruence_defect: f64,
    raw_inf_change: f64,
    normwise_backward: f64,
    componentwise_backward: f64,
    metric_residual_norm: f64,
    lifted_acceleration_error: f64,
    wrong_variance_error: f64,
    solve_timing: Timing,
    factor_timing: Timing,
    evidence_timing: Timing,
}

fn solver_solution(
    backend: &str,
    metric: &Metric<DenseY, N>,
    factor: &Lu6,
    force: &Cotangent<DenseY, N>,
) -> Result<Tangent<DenseY, N>, Box<dyn Error>> {
    match backend {
        "symtropy_reference_cholesky" => Ok(metric.raise(force)?),
        "symtropy_reference_lu" => Ok(Tangent::new(factor.solve(force.components())?)?),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown backend").into()),
    }
}

struct RunInput<'a> {
    backend: &'a str,
    family: usize,
    profile: MetricProfileV1,
    metric_q: &'a Metric<CartesianQ, N>,
    rebasis: &'a Rebasis<CartesianQ, DenseY, N>,
    metric_y: &'a Metric<DenseY, N>,
    force_q: &'a Cotangent<CartesianQ, N>,
    force_y: &'a Cotangent<DenseY, N>,
    factor_y: &'a Lu6,
    acceleration_q: &'a Tangent<CartesianQ, N>,
    acceleration_y: &'a Tangent<DenseY, N>,
}

fn run_metrics(input: RunInput<'_>) -> Result<RunMetrics, Box<dyn Error>> {
    let RunInput {
        backend,
        family: _,
        profile,
        metric_q,
        rebasis,
        metric_y,
        force_q,
        force_y,
        factor_y,
        acceleration_q,
        acceleration_y,
    } = input;
    let initial_velocity_q = Tangent::<CartesianQ, N>::new([
        0.4711764705882353,
        -0.2476470588235294,
        -0.14705882352941174,
        -0.04882352941176471,
        0.07235294117647058,
        0.2529411764705882,
    ])?;
    let initial_velocity_y = rebasis.tangent_to_to(&initial_velocity_q)?;

    let acceleration_y_lifted = rebasis.tangent_to_from(acceleration_y)?;
    let lifted_acceleration_error = vector_difference_l2(
        acceleration_q.components(),
        acceleration_y_lifted.components(),
    ) / 1.0_f64.max(vector_euclidean(acceleration_q.components()));
    let tangent_norm_q = metric_q.tangent_norm(acceleration_q)?;
    let tangent_norm_y = metric_y.tangent_norm(acceleration_y)?;
    let cotangent_norm_q = metric_q.cotangent_norm(force_q)?;
    let cotangent_norm_y = metric_y.cotangent_norm(force_y)?;
    let pairing_q = pairing(force_q, &initial_velocity_q);
    let pairing_y = pairing(force_y, &initial_velocity_y);

    let tangent_drift = relative_difference(tangent_norm_q, tangent_norm_y);
    let cotangent_drift = relative_difference(cotangent_norm_q, cotangent_norm_y);
    let pairing_drift = relative_difference(pairing_q, pairing_y);

    let manual_congruence = matrix_multiply(
        &matrix_multiply(&matrix_transpose(rebasis.matrix()), metric_q.matrix()),
        rebasis.matrix(),
    );
    let congruence_defect = matrix_relative_defect(metric_y.matrix(), &manual_congruence);

    let raw_inf_change = relative_difference(
        vector_inf(initial_velocity_q.components()),
        vector_inf(initial_velocity_y.components()),
    );

    let residual_q = vector_sub(
        force_q.components(),
        &matrix_apply(metric_q.matrix(), acceleration_q.components()),
    );
    let residual_y = vector_sub(
        force_y.components(),
        &matrix_apply(metric_y.matrix(), acceleration_y.components()),
    );
    let normwise_backward = normwise_backward_error(
        metric_q.matrix(),
        acceleration_q.components(),
        force_q.components(),
    )
    .max(normwise_backward_error(
        metric_y.matrix(),
        acceleration_y.components(),
        force_y.components(),
    ));
    let componentwise_backward = componentwise_backward_error(
        metric_q.matrix(),
        acceleration_q.components(),
        force_q.components(),
    )
    .max(componentwise_backward_error(
        metric_y.matrix(),
        acceleration_y.components(),
        force_y.components(),
    ));
    let residual_q_covector = Cotangent::<CartesianQ, N>::new(residual_q)?;
    let residual_y_covector = Cotangent::<DenseY, N>::new(residual_y)?;
    let metric_residual_norm = metric_q
        .cotangent_norm(&residual_q_covector)?
        .max(metric_y.cotangent_norm(&residual_y_covector)?);

    // Negative control: incorrectly transform a covector as though it were a
    // tangent vector. The frame type prevents accidental mixing in normal use;
    // this explicit adversarial conversion measures the consequence.
    let force_as_tangent = Tangent::<CartesianQ, N>::new(*force_q.components())?;
    let incorrectly_pushed = rebasis.tangent_to_to(&force_as_tangent)?;
    let wrong_force_y = Cotangent::<DenseY, N>::new(*incorrectly_pushed.components())?;
    let wrong_acceleration_y = solver_solution(backend, metric_y, factor_y, &wrong_force_y)?;
    let wrong_acceleration_q = rebasis.tangent_to_from(&wrong_acceleration_y)?;
    let wrong_variance_error = vector_difference_l2(
        acceleration_q.components(),
        wrong_acceleration_q.components(),
    ) / 1.0_f64.max(vector_euclidean(acceleration_q.components()));

    let matrix_y = *metric_y.matrix();
    let solve_timing = match backend {
        "symtropy_reference_cholesky" => measure(|| {
            vector_euclidean(
                metric_y
                    .raise(black_box(force_y))
                    .expect("admitted Cholesky solve")
                    .components(),
            )
        }),
        "symtropy_reference_lu" => measure(|| {
            vector_euclidean(
                &factor_y
                    .solve(black_box(force_y.components()))
                    .expect("admitted LU solve"),
            )
        }),
        _ => unreachable!("backend validated by caller"),
    };
    let factor_timing = match backend {
        "symtropy_reference_cholesky" => measure(|| {
            Metric::<DenseY, N>::admit(matrix_y, profile)
                .expect("admitted SPD metric")
                .admission_receipt()
                .minimum_cholesky_pivot
        }),
        "symtropy_reference_lu" => {
            measure(|| Lu6::factor(matrix_y).expect("admitted dense operator").lu[0][0].abs())
        }
        _ => unreachable!("backend validated by caller"),
    };
    let evidence_timing = measure(|| {
        metric_y
            .tangent_norm(black_box(acceleration_y))
            .expect("finite tangent norm")
            + metric_y
                .cotangent_norm(black_box(force_y))
                .expect("finite cotangent norm")
            + pairing(force_y, &initial_velocity_y).abs()
    });

    Ok(RunMetrics {
        tangent_drift,
        cotangent_drift,
        pairing_drift,
        congruence_defect,
        raw_inf_change,
        normwise_backward,
        componentwise_backward,
        metric_residual_norm,
        lifted_acceleration_error,
        wrong_variance_error,
        solve_timing,
        factor_timing,
        evidence_timing,
    })
}

fn render_result(
    backend: &str,
    family: usize,
    metrics: &RunMetrics,
    source_sha: Option<&str>,
    run_id: &str,
) -> String {
    let (basis_condition_2, _) = condition_metadata(family);
    let (implementation_name, reason) = match backend {
        "symtropy_reference_cholesky" => (
            "Symtropy frame-typed Cholesky/Riesz reference",
            "Kernel-only matched result; full constrained step and independent high-precision forward-error coverage are not executed.",
        ),
        "symtropy_reference_lu" => (
            "Independent partial-pivoted LU reference",
            "Kernel-only matched result; full constrained step and independent high-precision forward-error coverage are not executed.",
        ),
        _ => unreachable!("backend validated by caller"),
    };
    let source_commit_json = source_sha
        .map(json_quote)
        .unwrap_or_else(|| "null".to_owned());
    let unique_run_id = json_quote(&format!("{run_id}-{backend}-family-{family}"));

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"luminous.metric_physics_benchmark_result.v0.1\",\n",
            "  \"record_kind\": \"measured_result\",\n",
            "  \"run_id\": {},\n",
            "  \"case_binding\": {{\"case_id\":\"DUMBBELL-DENSE-REBASIS-001\",\"repository\":\"Luminous-Dynamics/symtropy\",\"commit_sha\":\"{}\",\"path\":\"{}\",\"git_blob_sha\":\"{}\"}},\n",
            "  \"implementation\": {{\"backend_id\":{},\"implementation_name\":{},\"version\":\"0.1.0\",\"source_commit_sha\":{}}},\n",
            "  \"model_mapping\": {{\n",
            "    \"status\":\"MATCHED\",\n",
            "    \"comparison_scope\":\"mass_metric_riesz_solve_kernel_only\",\n",
            "    \"physical_model\":\"two 3D point masses joined by one massless fixed-length link; mass block only\",\n",
            "    \"coordinate_order\":\"q1_x,q1_y,q1_z,q2_x,q2_y,q2_z\",\n",
            "    \"velocity_convention\":\"Cartesian point-mass velocity components in the frozen case frame\",\n",
            "    \"force_covector_convention\":\"generalized force covector; dense coordinates use S^T f_q\",\n",
            "    \"constraint_equations\":\"fixed-distance link; constraint solve not executed in this kernel slice\",\n",
            "    \"initial_state\":\"pinned DUMBBELL-DENSE-REBASIS-001 case; only its mass/force operator is exercised\",\n",
            "    \"time_step_seconds\":0.02,\n",
            "    \"integration_method\":\"mass_metric_riesz_solve_kernel_only; no integration step executed\",\n",
            "    \"units\":\"SI\",\n",
            "    \"solver_tolerances\":\"MetricProfileV1 default: relative pivot 64 eps; relative symmetry 256 eps\",\n",
            "    \"notes\":[\"The two algorithms solve the same frozen M a = f subproblem.\",\"No full constrained midpoint step or vendor backend is executed.\"]\n",
            "  }},\n",
            "  \"execution\": {{\n",
            "    \"status\":\"COMPLETED\",\"cpu_model\":{},\"os\":{},\"architecture\":{},\"compiler\":{},\n",
            "    \"floating_point_mode\":\"IEEE754_binary64\",\"thread_count\":1,\"warmup_steps\":{},\"repetitions\":{},\"random_seed\":0\n",
            "  }},\n",
            "  \"metrics\": {{\n",
            "    \"coordinate_invariance\": {{\"tangent_norm_relative_drift\":{},\"cotangent_norm_relative_drift\":{},\"canonical_pairing_relative_drift\":{},\"mass_congruence_relative_defect\":{},\"constraint_jacobian_relative_defect\":null,\"raw_coordinate_inf_norm_change\":{}}},\n",
            "    \"numerical_quality\": {{\"coordinate_condition_2\":{},\"metric_induced_operator_condition\":1.0,\"normwise_backward_error\":{},\"componentwise_backward_error\":{},\"metric_residual_norm\":{},\"forward_error_bound\":null,\"observed_high_precision_forward_error\":null,\"forward_bound_covers_observed\":null}},\n",
            "    \"physical_fidelity\": {{\"lifted_acceleration_relative_error\":{},\"lifted_position_l2_error\":null,\"lifted_velocity_l2_error\":null,\"momentum_l2_error\":null,\"constraint_residual\":null,\"energy_work_defect_j\":null,\"discrete_step_equivariance_defect\":null}},\n",
            "    \"decision_quality\": {{\"negative_controls_run\":1,\"false_accept_count\":null,\"false_reject_count\":null,\"inconclusive_case_count\":1,\"wrong_law_negative_control_relative_error\":{}}},\n",
            "    \"systems_cost\": {{\"solve_median_ns\":{},\"solve_p95_ns\":{},\"solve_p99_ns\":{},\"factorization_median_ns\":{},\"evidence_overhead_median_ns\":{},\"peak_memory_bytes\":null}}\n",
            "  }},\n",
            "  \"decision\": {{\"acceptance\":\"INCONCLUSIVE\",\"comparison_status\":\"COMPARABLE\",\"reason\":{}}},\n",
            "  \"claim_boundary\":[\n",
            "    \"This measured result covers the mass-metric Riesz solve kernel only, not the full constrained midpoint step.\",\n",
            "    \"The metric-induced condition 1 applies only to the mass Riesz map M:T->T*, not to arbitrary physical operators.\",\n",
            "    \"No Drake, MuJoCo or Pinocchio adapter was executed by this runner.\",\n",
            "    \"Forward-error envelope and high-precision coverage remain INCONCLUSIVE until an independent oracle is attached.\"\n",
            "  ]\n",
            "}}\n"
        ),
        unique_run_id,
        CASE_COMMIT,
        CASE_PATH,
        CASE_BLOB,
        json_quote(backend),
        json_quote(implementation_name),
        source_commit_json,
        json_quote(&cpu_model()),
        json_quote(std::env::consts::OS),
        json_quote(std::env::consts::ARCH),
        json_quote(&compiler_version()),
        WARMUP,
        SAMPLES,
        json_number(metrics.tangent_drift),
        json_number(metrics.cotangent_drift),
        json_number(metrics.pairing_drift),
        json_number(metrics.congruence_defect),
        json_number(metrics.raw_inf_change),
        json_number(basis_condition_2),
        json_number(metrics.normwise_backward),
        json_number(metrics.componentwise_backward),
        json_number(metrics.metric_residual_norm),
        json_number(metrics.lifted_acceleration_error),
        json_number(metrics.wrong_variance_error),
        json_number(metrics.solve_timing.median_ns),
        json_number(metrics.solve_timing.p95_ns),
        json_number(metrics.solve_timing.p99_ns),
        json_number(metrics.factor_timing.median_ns),
        json_number(metrics.evidence_timing.median_ns),
        json_quote(reason),
    )
}

fn run_one_family(
    backend: &str,
    family: usize,
    profile: MetricProfileV1,
    metric_q: &Metric<CartesianQ, N>,
    force_q: &Cotangent<CartesianQ, N>,
) -> Result<String, Box<dyn Error>> {
    let basis = basis_matrix(family);
    let rebasis = Rebasis::<CartesianQ, DenseY, N>::new(basis, profile)?;
    let metric_y = rebasis.metric_to_to(metric_q, profile)?;
    let force_y = rebasis.cotangent_to_to(force_q)?;
    let matrix_y = *metric_y.matrix();

    let lu_q = Lu6::factor(*metric_q.matrix())?;
    let lu_y = Lu6::factor(matrix_y)?;

    let acceleration_q = match backend {
        "symtropy_reference_cholesky" => metric_q.raise(force_q)?,
        "symtropy_reference_lu" => Tangent::new(lu_q.solve(force_q.components())?)?,
        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown backend").into()),
    };
    let acceleration_y = solver_solution(backend, &metric_y, &lu_y, &force_y)?;

    let metrics = run_metrics(RunInput {
        backend,
        family,
        profile,
        metric_q,
        rebasis: &rebasis,
        metric_y: &metric_y,
        force_q,
        force_y: &force_y,
        factor_y: &lu_y,
        acceleration_q: &acceleration_q,
        acceleration_y: &acceleration_y,
    })?;

    let run_id = std::env::var("GITHUB_RUN_ID").unwrap_or_else(|_| "local".to_owned());
    let source_sha = source_commit();
    Ok(render_result(
        backend,
        family,
        &metrics,
        source_sha.as_deref(),
        &run_id,
    ))
}

fn main() -> Result<(), Box<dyn Error>> {
    let output_path = std::env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: matched-kernel-runner <output-json-path>",
        )
    })?;

    let profile = MetricProfileV1::default();
    let mass = mass_matrix();
    let metric_q = Metric::<CartesianQ, N>::admit(mass, profile)?;
    let force_q = Cotangent::<CartesianQ, N>::new([0.18, -0.07, 0.11, 0.0, 0.0, 0.0])?;

    let mut records = Vec::with_capacity(4);
    for backend in ["symtropy_reference_cholesky", "symtropy_reference_lu"] {
        for family in [0, 2] {
            records.push(run_one_family(
                backend, family, profile, &metric_q, &force_q,
            )?);
        }
    }

    let report = format!("[\n{}\n]\n", records.join(",\n"));
    fs::write(output_path, report)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_lu_and_cholesky_agree_for_canonical_mass_solve() {
        let profile = MetricProfileV1::default();
        let metric = Metric::<CartesianQ, N>::admit(mass_matrix(), profile).unwrap();
        let force = Cotangent::<CartesianQ, N>::new([0.18, -0.07, 0.11, 0.0, 0.0, 0.0]).unwrap();
        let cholesky = metric.raise(&force).unwrap();
        let lu = Lu6::factor(*metric.matrix())
            .unwrap()
            .solve(force.components())
            .unwrap();
        assert!(vector_difference_l2(cholesky.components(), &lu) < 1.0e-14);
    }

    #[test]
    fn wrong_covector_variance_is_detectably_different_after_dense_rebasis() {
        let profile = MetricProfileV1::default();
        let metric_q = Metric::<CartesianQ, N>::admit(mass_matrix(), profile).unwrap();
        let force_q = Cotangent::<CartesianQ, N>::new([0.18, -0.07, 0.11, 0.0, 0.0, 0.0]).unwrap();
        let basis = basis_matrix(2);
        let rebasis = Rebasis::<CartesianQ, DenseY, N>::new(basis, profile).unwrap();
        let metric_y = rebasis.metric_to_to(&metric_q, profile).unwrap();
        let force_y = rebasis.cotangent_to_to(&force_q).unwrap();
        let correct = metric_y.raise(&force_y).unwrap();
        let wrong_as_tangent = Tangent::<CartesianQ, N>::new(*force_q.components()).unwrap();
        let wrong_push = rebasis.tangent_to_to(&wrong_as_tangent).unwrap();
        let wrong_force = Cotangent::<DenseY, N>::new(*wrong_push.components()).unwrap();
        let wrong = metric_y.raise(&wrong_force).unwrap();
        let correct_lift = rebasis.tangent_to_from(&correct).unwrap();
        let wrong_lift = rebasis.tangent_to_from(&wrong).unwrap();
        let relative = vector_difference_l2(correct_lift.components(), wrong_lift.components())
            / vector_euclidean(correct_lift.components()).max(1.0);
        assert!(relative > 1.0e-6);
    }
}
