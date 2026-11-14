use std::hint::black_box;
use std::time::Instant;

use metric_induced_error_envelope_reference::{
    CartesianQ, DenseY, Matrix, Metric, MetricProfileV1, Rebasis,
};

fn fixture<const N: usize>(
    profile: MetricProfileV1,
) -> (
    Metric<CartesianQ, N>,
    Rebasis<CartesianQ, DenseY, N>,
) {
    let mass: Matrix<N> = std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            if row == column {
                1.0 + (row % 7) as f64 * 0.25
            } else {
                0.0
            }
        })
    });
    let basis: Matrix<N> = std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            if row == column {
                1.0
            } else if column > row {
                let signed = ((row * 17 + column * 13) % 7) as i32 - 3;
                signed as f64 * 0.025
            } else {
                0.0
            }
        })
    });
    (
        Metric::<CartesianQ, N>::admit(mass, profile).unwrap(),
        Rebasis::<CartesianQ, DenseY, N>::new(basis, profile).unwrap(),
    )
}

/// Deliberately simple O(N^4) four-index congruence oracle followed by the
/// same metric admission and Cholesky work as the optimized implementation.
fn reference_congruence<const N: usize>(
    metric: &Metric<CartesianQ, N>,
    rebasis: &Rebasis<CartesianQ, DenseY, N>,
    profile: MetricProfileV1,
) -> Metric<DenseY, N> {
    let mut result = [[0.0; N]; N];
    for (row, result_row) in result.iter_mut().enumerate() {
        for (column, output) in result_row.iter_mut().enumerate() {
            let mut value = 0.0;
            for (basis_left, mass_row) in rebasis.matrix().iter().zip(metric.matrix().iter()) {
                let left_factor = basis_left[row];
                for (mass_value, basis_right) in mass_row.iter().zip(rebasis.matrix().iter()) {
                    value += left_factor * mass_value * basis_right[column];
                }
            }
            *output = value;
        }
    }
    Metric::<DenseY, N>::admit(result, profile).unwrap()
}

fn median_ns<F>(samples: usize, iterations: usize, mut operation: F) -> f64
where
    F: FnMut() -> f64,
{
    let mut timings = Vec::with_capacity(samples);
    let mut checksum = 0.0_f64;
    for _ in 0..2 {
        checksum += black_box(operation());
    }
    black_box(checksum);

    for _ in 0..samples {
        let start = Instant::now();
        let mut local_checksum = 0.0_f64;
        for _ in 0..iterations {
            local_checksum += black_box(operation());
        }
        let elapsed = start.elapsed();
        checksum += local_checksum;
        timings.push(elapsed.as_secs_f64() * 1.0e9 / iterations as f64);
    }
    black_box(checksum);
    timings.sort_by(f64::total_cmp);
    timings[timings.len() / 2]
}

fn run<const N: usize>(iterations: usize) {
    let profile = MetricProfileV1::default();
    let (metric, rebasis) = fixture::<N>(profile);
    let samples = 9;
    let optimized = median_ns(samples, iterations, || {
        let result = rebasis
            .metric_to_to(black_box(&metric), profile)
            .unwrap();
        result.matrix()[0][0] + result.matrix()[N - 1][N - 1]
    });
    let reference = median_ns(samples, iterations, || {
        let result = reference_congruence(black_box(&metric), black_box(&rebasis), profile);
        result.matrix()[0][0] + result.matrix()[N - 1][N - 1]
    });
    println!(
        "{N},{iterations},{optimized:.3},{reference:.3},{:.3}",
        reference / optimized
    );
}

fn main() {
    println!("dimension,iterations_per_sample,optimized_o_n3_median_ns,reference_o_n4_median_ns,reference_over_optimized");
    run::<6>(500);
    run::<12>(100);
    run::<24>(10);
    run::<48>(2);
}
