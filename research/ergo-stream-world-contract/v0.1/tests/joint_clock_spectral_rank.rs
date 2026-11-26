use std::collections::BTreeMap;

use ergo_stream_microcosm::joint_clock::{
    analyze_joint_clock_design_v0, JointClockProblemV0, SensorEventObservationV0,
    JOINT_CLOCK_RANK_TOLERANCE_V0,
};

fn two_stream_problem(mask: u32) -> JointClockProblemV0 {
    let stream_ids = vec![10_u16, 20_u16];
    let event_ids = vec![101_u32, 202_u32, 303_u32];
    let local_times = [[0_i64, 10, 30], [5_i64, 10, 20]];
    let mut observations = Vec::new();

    for (stream_index, (stream_id, times)) in
        stream_ids.iter().copied().zip(local_times.iter()).enumerate()
    {
        for (event_index, (event_id, local_timestamp)) in
            event_ids.iter().copied().zip(times.iter()).enumerate()
        {
            let bit = 1_u32 << (stream_index * event_ids.len() + event_index);
            if mask & bit != 0 {
                observations.push(SensorEventObservationV0 {
                    stream_id,
                    event_id,
                    local_timestamp: *local_timestamp,
                });
            }
        }
    }

    JointClockProblemV0 {
        stream_ids,
        event_ids,
        reference_stream_id: 10,
        observations,
    }
}

fn multistream_problem() -> JointClockProblemV0 {
    let stream_ids = vec![30_u16, 10_u16, 20_u16];
    let event_ids = vec![404_u32, 101_u32, 303_u32, 202_u32];
    let local_times = [
        [100_i64, 120, 140, 160],
        [0_i64, 10, 20, 30],
        [4_i64, 9, 14, 19],
    ];
    let mut observations = Vec::new();

    for (stream_id, times) in stream_ids.iter().copied().zip(local_times.iter()) {
        for (event_id, local_timestamp) in event_ids.iter().copied().zip(times.iter()) {
            observations.push(SensorEventObservationV0 {
                stream_id,
                event_id,
                local_timestamp: *local_timestamp,
            });
        }
    }

    JointClockProblemV0 {
        stream_ids,
        event_ids,
        reference_stream_id: 10,
        observations,
    }
}

/// Rebuild the gauge-fixed design from the public problem contract rather than
/// reading the production module's private design matrix.
fn independently_build_column_scaled_design(
    problem: &JointClockProblemV0,
) -> Vec<Vec<f64>> {
    let mut stream_ids = problem.stream_ids.clone();
    stream_ids.sort_unstable();
    let mut event_ids = problem.event_ids.clone();
    event_ids.sort_unstable();

    let event_columns = event_ids
        .iter()
        .enumerate()
        .map(|(column, event_id)| (*event_id, column))
        .collect::<BTreeMap<_, _>>();

    let non_reference_streams = stream_ids
        .iter()
        .copied()
        .filter(|stream_id| *stream_id != problem.reference_stream_id)
        .collect::<Vec<_>>();

    let clock_columns = non_reference_streams
        .iter()
        .enumerate()
        .map(|(index, stream_id)| {
            (
                *stream_id,
                (event_ids.len() + 2 * index, event_ids.len() + 2 * index + 1),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let parameter_count = event_ids.len() + 2 * non_reference_streams.len();
    let mut observations = problem.observations.clone();
    observations.sort_unstable();

    let mut matrix = Vec::with_capacity(observations.len());
    for observation in observations {
        let mut row = vec![0.0_f64; parameter_count];
        row[event_columns[&observation.event_id]] = 1.0;

        if observation.stream_id != problem.reference_stream_id {
            let (rate_column, offset_column) = clock_columns[&observation.stream_id];
            row[rate_column] = -(observation.local_timestamp as f64);
            row[offset_column] = -1.0;
        }

        matrix.push(row);
    }

    // Match the production rank audit's column scaling, but not its QR
    // factorization or pivot/rank calculation.
    let mut squared_norms = vec![0.0_f64; parameter_count];
    for row in &matrix {
        for (squared_norm, value) in squared_norms.iter_mut().zip(row) {
            *squared_norm += value * value;
        }
    }
    for row in &mut matrix {
        for (value, squared_norm) in row.iter_mut().zip(&squared_norms) {
            if *squared_norm > 0.0 {
                *value /= squared_norm.sqrt();
            }
        }
    }

    matrix
}

/// Independent one-sided Jacobi SVD for small test matrices. Orthogonalizing
/// columns directly avoids forming A^T A and squaring its condition number.
fn singular_values_one_sided_jacobi(matrix: &[Vec<f64>]) -> Vec<f64> {
    assert!(!matrix.is_empty(), "the design must contain observations");
    let row_count = matrix.len();
    let column_count = matrix[0].len();
    assert!(column_count > 0, "the design must contain parameters");
    assert!(
        matrix.iter().all(|row| row.len() == column_count),
        "the design must be rectangular"
    );

    let mut columns = (0..column_count)
        .map(|column_index| {
            matrix
                .iter()
                .map(|row| row[column_index])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let rotation_tolerance = 1.0e-14_f64;
    let maximum_sweeps = 128 * column_count.max(1);
    let mut converged = false;

    for _ in 0..maximum_sweeps {
        let mut rotated_in_sweep = false;

        for left in 0..column_count {
            for right in (left + 1)..column_count {
                let mut alpha = 0.0_f64;
                let mut beta = 0.0_f64;
                let mut gamma = 0.0_f64;

                for (&x, &y) in columns[left].iter().zip(&columns[right]) {
                    alpha += x * x;
                    beta += y * y;
                    gamma += x * y;
                }

                if alpha == 0.0 || beta == 0.0 {
                    continue;
                }

                let scale = alpha.sqrt() * beta.sqrt();
                if gamma.abs() <= rotation_tolerance * scale {
                    continue;
                }

                let tau = (beta - alpha) / (2.0 * gamma);
                let denominator = tau.abs() + tau.hypot(1.0);
                let tangent = if tau >= 0.0 {
                    1.0 / denominator
                } else {
                    -1.0 / denominator
                };
                let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
                let sine = cosine * tangent;

                let (left_columns, right_columns) = columns.split_at_mut(right);
                for (x, y) in left_columns[left]
                    .iter_mut()
                    .zip(right_columns[0].iter_mut())
                {
                    let old_x = *x;
                    let old_y = *y;
                    *x = cosine * old_x - sine * old_y;
                    *y = sine * old_x + cosine * old_y;
                }

                rotated_in_sweep = true;
            }
        }

        if !rotated_in_sweep {
            converged = true;
            break;
        }
    }

    assert!(
        converged,
        "independent one-sided Jacobi SVD did not converge"
    );

    let mut singular_values = columns
        .iter()
        .map(|column| column.iter().map(|value| value * value).sum::<f64>().sqrt())
        .collect::<Vec<_>>();
    singular_values.sort_by(|left, right| right.total_cmp(left));
    singular_values
}

fn independent_svd_rank(matrix: &[Vec<f64>]) -> usize {
    let row_count = matrix.len();
    let column_count = matrix[0].len();
    let threshold = JOINT_CLOCK_RANK_TOLERANCE_V0 * row_count.max(column_count) as f64;

    singular_values_one_sided_jacobi(matrix)
        .into_iter()
        .filter(|singular_value| *singular_value > threshold)
        .count()
}

#[test]
fn qr_rank_matches_independent_svd_for_every_two_stream_anchor_subset() {
    for mask in 1_u32..(1_u32 << 6) {
        let problem = two_stream_problem(mask);
        let report =
            analyze_joint_clock_design_v0(&problem, JOINT_CLOCK_RANK_TOLERANCE_V0)
                .expect("non-empty anchor subset should produce a design report");
        let independent_design = independently_build_column_scaled_design(&problem);
        let svd_rank = independent_svd_rank(&independent_design);

        assert_eq!(
            report.rank, svd_rank,
            "QR/SVD rank disagreement for two-stream anchor mask {mask:#08b}"
        );
        assert_eq!(
            report.full_column_rank,
            svd_rank == report.parameter_count,
            "full-column-rank disposition disagrees for mask {mask:#08b}"
        );
    }
}

#[test]
fn full_multistream_design_matches_independent_svd() {
    let problem = multistream_problem();
    let report =
        analyze_joint_clock_design_v0(&problem, JOINT_CLOCK_RANK_TOLERANCE_V0)
            .expect("complete multi-stream design should be analyzable");
    let independent_design = independently_build_column_scaled_design(&problem);
    let singular_values = singular_values_one_sided_jacobi(&independent_design);

    assert_eq!(report.rank, report.parameter_count);
    assert_eq!(
        independent_svd_rank(&independent_design),
        report.parameter_count
    );
    assert!(
        singular_values.last().copied().unwrap_or(0.0)
            > JOINT_CLOCK_RANK_TOLERANCE_V0
                * independent_design.len().max(report.parameter_count) as f64
    );
}

#[test]
fn single_shared_event_remains_rank_deficient_in_both_auditors() {
    // One paired event supplies two observations but cannot identify the
    // event-time vector plus both inverse-clock parameters.
    let problem = two_stream_problem((1_u32 << 0) | (1_u32 << 3));
    let report =
        analyze_joint_clock_design_v0(&problem, JOINT_CLOCK_RANK_TOLERANCE_V0)
            .expect("the deficient design still has a report");
    let independent_design = independently_build_column_scaled_design(&problem);

    assert!(report.rank < report.parameter_count);
    assert_eq!(report.rank, independent_svd_rank(&independent_design));
}
