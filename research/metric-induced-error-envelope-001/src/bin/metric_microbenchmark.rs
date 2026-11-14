// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Deterministic microbenchmark for the metric-induced reference.
//!
//! This measures the reference layer only. It is not a Drake/MuJoCo/Pinocchio
//! comparison and does not measure an articulated-body dynamics solver.

use std::hint::black_box;
use std::time::{Duration, Instant};

use metric_induced_error_envelope_reference::{
    CartesianQ, ConstraintSpace, Cotangent, DelassusMetricV1, DenseY, Matrix, Metric,
    MetricProfileV1, Rebasis, Tangent, pairing, vector_euclidean, vector_inf,
};

const N: usize = 6;
const ITERATIONS: usize = 20_000;
const SAMPLES: usize = 31;
const WARMUP: usize = 2_000;

#[derive(Debug)]
struct Measurement {
    name: &'static str,
    iterations: usize,
    samples: usize,
    median_ns_per_op: f64,
    p95_ns_per_op: f64,
    p99_ns_per_op: f64,
    checksum: f64,
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn measure<F>(
    name: &'static str,
    iterations: usize,
    samples: usize,
    mut operation: F,
) -> Measurement
where
    F: FnMut() -> f64,
{
    let mut warmup_checksum = 0.0_f64;
    for _ in 0..WARMUP {
        warmup_checksum += black_box(operation());
    }
    black_box(warmup_checksum);

    let mut timings = Vec::with_capacity(samples);
    let mut checksum = 0.0_f64;
    for _sample in 0..samples {
        let start = Instant::now();
        let mut sample_checksum = 0.0_f64;
        for _ in 0..iterations {
            sample_checksum += black_box(operation());
        }
        let elapsed = start.elapsed();
        checksum += sample_checksum;
        timings.push(elapsed.as_secs_f64() * 1.0e9 / iterations as f64);
    }
    timings.sort_by(f64::total_cmp);

    Measurement {
        name,
        iterations,
        samples,
        median_ns_per_op: percentile(&timings, 0.50),
        p95_ns_per_op: percentile(&timings, 0.95),
        p99_ns_per_op: percentile(&timings, 0.99),
        checksum,
    }
}

const CONSTRAINTS: usize = 3;

type BenchmarkFixture = (
    Metric<CartesianQ, N>,
    Rebasis<CartesianQ, DenseY, N>,
    Tangent<CartesianQ, N>,
    Cotangent<CartesianQ, N>,
    Tangent<DenseY, N>,
    Cotangent<DenseY, N>,
    Metric<DenseY, N>,
    DelassusMetricV1<CartesianQ, ConstraintSpace, N, CONSTRAINTS>,
    [[f64; N]; CONSTRAINTS],
);

fn fixture() -> BenchmarkFixture {
    let profile = MetricProfileV1::default();
    let mass: Matrix<N> = [
        [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 3.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 4.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, 5.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 6.0],
    ];
    let basis: Matrix<N> = [
        [1.0, 2.0, -1.0, 0.0, 1.0, 0.0],
        [0.0, 1.0, 3.0, -1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 2.0, -1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0, 3.0, -1.0],
        [0.0, 0.0, 0.0, 0.0, 1.0, 2.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    ];
    let metric_q = Metric::<CartesianQ, N>::admit(mass, profile).unwrap();
    let rebasis = Rebasis::<CartesianQ, DenseY, N>::new(basis, profile).unwrap();
    let metric_y = rebasis.metric_to_to(&metric_q, profile).unwrap();

    let vector_q = Tangent::<CartesianQ, N>::new([0.5, -1.25, 2.0, 0.75, -0.2, 1.5]).unwrap();
    let covector_q = Cotangent::<CartesianQ, N>::new([2.0, -3.0, 0.75, -1.5, 0.25, 4.0]).unwrap();
    let vector_y = rebasis.tangent_to_to(&vector_q).unwrap();
    let covector_y = rebasis.cotangent_to_to(&covector_q).unwrap();

    let jacobian: [[f64; N]; CONSTRAINTS] = [
        [1.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0, 1.0, 0.0],
    ];
    let delassus =
        DelassusMetricV1::<CartesianQ, ConstraintSpace, N, CONSTRAINTS>::from_mass_jacobian(
            &metric_q, jacobian, profile,
        )
        .unwrap();

    (
        metric_q, rebasis, vector_q, covector_q, vector_y, covector_y, metric_y, delassus, jacobian,
    )
}

fn direct_delassus(
    mass: &Metric<CartesianQ, N>,
    jacobian: &[[f64; N]; CONSTRAINTS],
    profile: MetricProfileV1,
) -> Metric<ConstraintSpace, CONSTRAINTS> {
    let mut matrix = [[0.0; CONSTRAINTS]; CONSTRAINTS];
    for column in 0..CONSTRAINTS {
        let column_covector = Cotangent::<CartesianQ, N>::new(jacobian[column]).unwrap();
        let raised_column = mass.raise(&column_covector).unwrap();
        for row in 0..CONSTRAINTS {
            let row_covector = Cotangent::<CartesianQ, N>::new(jacobian[row]).unwrap();
            matrix[row][column] = pairing(&row_covector, &raised_column);
        }
    }
    Metric::<ConstraintSpace, CONSTRAINTS>::admit(matrix, profile).unwrap()
}

fn json_number(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.9}")
    } else {
        "null".to_owned()
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

fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".to_owned())
}

fn cpu_model() -> String {
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo")
        && let Some(model) = cpuinfo.lines().find_map(|line| {
            line.strip_prefix("model name")
                .or_else(|| line.strip_prefix("Hardware"))
                .and_then(|value| value.split_once(':').map(|(_, model)| model.trim()))
        })
    {
        return model.to_owned();
    }
    std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "not-exposed".to_owned())
}

fn render_report(measurements: &[Measurement], total_elapsed: Duration) -> String {
    let rows = measurements
        .iter()
        .map(|item| {
            format!(
                concat!(
                    "{{\"name\":\"{}\",\"iterations_per_sample\":{},\"samples\":{},",
                    "\"median_ns_per_op\":{},\"p95_ns_per_op\":{},",
                    "\"p99_ns_per_op\":{},\"checksum\":{}}}"
                ),
                item.name,
                item.iterations,
                item.samples,
                json_number(item.median_ns_per_op),
                json_number(item.p95_ns_per_op),
                json_number(item.p99_ns_per_op),
                json_number(item.checksum),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"luminous.metric_induced_microbenchmark.v0.1\",\n",
            "  \"fixture_id\": \"dense-6d-diagonal-mass-upper-unitriangular-rebasis-v0.1\",\n",
            "  \"crate_version\": \"{}\",\n",
            "  \"target_arch\": \"{}\",\n",
            "  \"target_os\": \"{}\",\n",
            "  \"cpu_model\": \"{}\",\n",
            "  \"rustc_version\": \"{}\",\n",
            "  \"source_commit\": \"{}\",\n",
            "  \"workflow_run_id\": \"{}\",\n",
            "  \"available_parallelism\": {},\n",
            "  \"iterations_per_sample\": {},\n",
            "  \"samples\": {},\n",
            "  \"total_elapsed_ms\": {},\n",
            "  \"measurements\": [\n{}\n  ],\n",
            "  \"claim_boundary\": \"microbenchmark of the isolated reference layer only; not an industry solver comparison\"\n",
            "}}\n"
        ),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        std::env::consts::OS,
        json_escape(&cpu_model()),
        json_escape(&rustc_version()),
        json_escape(
            &std::env::var("METRIC_BENCHMARK_SOURCE_SHA")
                .or_else(|_| std::env::var("GITHUB_SHA"))
                .unwrap_or_else(|_| "local".to_owned()),
        ),
        json_escape(&std::env::var("GITHUB_RUN_ID").unwrap_or_else(|_| "local".to_owned())),
        std::thread::available_parallelism()
            .map(|v| v.get())
            .unwrap_or(1),
        ITERATIONS,
        SAMPLES,
        total_elapsed.as_secs_f64() * 1000.0,
        rows,
    )
}

fn main() {
    let start_all = Instant::now();
    let (
        metric_q,
        rebasis,
        vector_q,
        covector_q,
        vector_y,
        covector_y,
        metric_y,
        delassus,
        jacobian,
    ) = fixture();

    let measurements = vec![
        measure("raw_euclidean_tangent_q", ITERATIONS, SAMPLES, || {
            vector_euclidean(black_box(vector_q.components()))
        }),
        measure("raw_infinity_tangent_q", ITERATIONS, SAMPLES, || {
            vector_inf(black_box(vector_q.components()))
        }),
        measure("metric_tangent_norm_q", ITERATIONS, SAMPLES, || {
            metric_q.tangent_norm(black_box(&vector_q)).unwrap()
        }),
        measure("dual_metric_cotangent_norm_q", ITERATIONS, SAMPLES, || {
            metric_q.cotangent_norm(black_box(&covector_q)).unwrap()
        }),
        measure("metric_tangent_norm_y", ITERATIONS, SAMPLES, || {
            metric_y.tangent_norm(black_box(&vector_y)).unwrap()
        }),
        measure("dual_metric_cotangent_norm_y", ITERATIONS, SAMPLES, || {
            metric_y.cotangent_norm(black_box(&covector_y)).unwrap()
        }),
        measure("tangent_pushforward_q_to_y", ITERATIONS, SAMPLES, || {
            let value = rebasis.tangent_to_to(black_box(&vector_q)).unwrap();
            vector_euclidean(value.components())
        }),
        measure("cotangent_pullback_q_to_y", ITERATIONS, SAMPLES, || {
            let value = rebasis.cotangent_to_to(black_box(&covector_q)).unwrap();
            vector_euclidean(value.components())
        }),
        measure("metric_rebasis_q_to_y", ITERATIONS / 10, SAMPLES, || {
            let metric = rebasis
                .metric_to_to(black_box(&metric_q), MetricProfileV1::default())
                .unwrap();
            metric.tangent_norm(&vector_y).unwrap()
        }),
        measure(
            "metric_admission_cholesky_q",
            ITERATIONS / 10,
            SAMPLES,
            || {
                let admitted = Metric::<CartesianQ, N>::admit(
                    *black_box(metric_q.matrix()),
                    MetricProfileV1::default(),
                )
                .unwrap();
                admitted.admission_receipt().minimum_cholesky_pivot
            },
        ),
        measure("metric_and_dual_norms_y_pair", ITERATIONS, SAMPLES, || {
            metric_y.tangent_norm(black_box(&vector_y)).unwrap()
                + metric_y.cotangent_norm(black_box(&covector_y)).unwrap()
        }),
        measure(
            "delassus_whitened_gram_construct",
            ITERATIONS / 100,
            SAMPLES,
            || {
                DelassusMetricV1::<CartesianQ, ConstraintSpace, N, CONSTRAINTS>::from_mass_jacobian(
                    black_box(&metric_q),
                    *black_box(&jacobian),
                    MetricProfileV1::default(),
                )
                .unwrap()
                .matrix()[0][0]
            },
        ),
        measure(
            "delassus_direct_mass_solve_construct",
            ITERATIONS / 100,
            SAMPLES,
            || {
                direct_delassus(
                    black_box(&metric_q),
                    black_box(&jacobian),
                    MetricProfileV1::default(),
                )
                .matrix()[0][0]
            },
        ),
        measure(
            "delassus_multiplier_metric_norm",
            ITERATIONS,
            SAMPLES,
            || {
                delassus
                    .multiplier_norm(
                        &metric_induced_error_envelope_reference::ConstraintMultiplierV1::<
                            ConstraintSpace,
                            CONSTRAINTS,
                        >::new([0.5, -1.0, 0.75])
                        .unwrap(),
                    )
                    .unwrap()
            },
        ),
        measure("delassus_residual_dual_norm", ITERATIONS, SAMPLES, || {
            delassus
                .residual_norm(
                    &metric_induced_error_envelope_reference::ConstraintResidualV1::<
                        ConstraintSpace,
                        CONSTRAINTS,
                    >::new([0.1, -0.2, 0.15])
                    .unwrap(),
                )
                .unwrap()
        }),
    ];

    let report = render_report(&measurements, start_all.elapsed());
    if let Some(path) = std::env::args().nth(1)
        && let Err(error) = std::fs::write(&path, &report)
    {
        eprintln!("failed to write benchmark report to {path}: {error}");
        std::process::exit(2);
    }
    print!("{report}");
}
