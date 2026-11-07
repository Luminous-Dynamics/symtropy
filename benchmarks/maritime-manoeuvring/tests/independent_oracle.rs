use maritime_manoeuvring_contract::{
    Trajectory, TrajectorySample, ZigZagCommand, stopping_metrics, turning_circle_metrics,
    zig_zag_metrics,
};

const TOLERANCE: f64 = 1.0e-8;
const DEG: f64 = std::f64::consts::PI / 180.0;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "actual {actual:.12}, expected {expected:.12}"
    );
}

fn sample(
    time_s: f64,
    x_m: f64,
    y_m: f64,
    heading_rad: f64,
    surge_mps: f64,
    sway_mps: f64,
    yaw_rate_rad_s: f64,
) -> TrajectorySample {
    TrajectorySample {
        time_s,
        x_m,
        y_m,
        heading_rad,
        surge_mps,
        sway_mps,
        yaw_rate_rad_s,
    }
}

// This test-only reference path is deliberately implemented here rather than
// calling any production metric/helper. Its interpolation is performed on
// explicit sample pairs and its angular unwrapping uses degree-domain deltas.
fn reference_unwrapped_heading_deg(samples: &[TrajectorySample]) -> Vec<f64> {
    let mut result = Vec::with_capacity(samples.len());
    let mut previous_raw = samples[0].heading_rad.to_degrees();
    result.push(previous_raw);

    for item in samples.iter().skip(1) {
        let current_raw = item.heading_rad.to_degrees();
        let mut step = current_raw - previous_raw;
        if step > 180.0 {
            step -= 360.0;
        } else if step < -180.0 {
            step += 360.0;
        }
        result.push(result.last().copied().unwrap_or(previous_raw) + step);
        previous_raw = current_raw;
    }

    result
}

#[derive(Clone, Copy)]
struct ReferencePoint {
    time_s: f64,
    x_m: f64,
    y_m: f64,
}

fn reference_heading_intersection(
    samples: &[TrajectorySample],
    headings_deg: &[f64],
    from_index: usize,
    direction: f64,
    target_deg: f64,
) -> Option<ReferencePoint> {
    for right in (from_index + 1)..samples.len() {
        let left = right - 1;
        let before = direction * (headings_deg[left] - headings_deg[0]);
        let after = direction * (headings_deg[right] - headings_deg[0]);

        if before <= target_deg && after >= target_deg && after > before {
            let ratio = (target_deg - before) / (after - before);
            let a = samples[left];
            let b = samples[right];
            return Some(ReferencePoint {
                time_s: a.time_s + ratio * (b.time_s - a.time_s),
                x_m: a.x_m + ratio * (b.x_m - a.x_m),
                y_m: a.y_m + ratio * (b.y_m - a.y_m),
            });
        }
    }
    None
}

fn reference_turning_circle(trajectory: &Trajectory) -> [f64; 5] {
    let samples = &trajectory.samples;
    let headings = reference_unwrapped_heading_deg(samples);
    let direction = headings
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .find(|delta| delta.abs() > 1.0e-12)
        .expect("fixture turns")
        .signum();

    let at_90 = reference_heading_intersection(samples, &headings, 0, direction, 90.0)
        .expect("fixture reaches 90 degrees");
    let at_180 = reference_heading_intersection(samples, &headings, 0, direction, 180.0)
        .expect("fixture reaches 180 degrees");

    let initial = samples[0];
    let (sin_h, cos_h) = initial.heading_rad.sin_cos();
    let dx90 = at_90.x_m - initial.x_m;
    let dy90 = at_90.y_m - initial.y_m;
    let dx180 = at_180.x_m - initial.x_m;
    let dy180 = at_180.y_m - initial.y_m;

    [
        dx90 * cos_h + dy90 * sin_h,
        -dx90 * sin_h + dy90 * cos_h,
        (-dx180 * sin_h + dy180 * cos_h).abs(),
        at_90.time_s - initial.time_s,
        at_180.time_s - initial.time_s,
    ]
}

fn circular_trajectory(direction: f64, initial_heading_deg: f64) -> Trajectory {
    let offsets_deg: [f64; 6] = [0.0, 31.0, 74.0, 111.0, 149.0, 203.0];
    let times_s: [f64; 6] = [4.0, 4.4, 5.6, 7.1, 9.8, 12.0];
    let radius_m = 42.0;
    let initial = initial_heading_deg * DEG;
    let (sin_h, cos_h) = initial.sin_cos();

    let samples = offsets_deg
        .iter()
        .zip(times_s)
        .map(|(offset_deg, time_s)| {
            let offset = direction * offset_deg * DEG;
            let local_x = direction * radius_m * offset.sin();
            let local_y = direction * radius_m * (1.0 - offset.cos());
            let x_m = 123.0 + local_x * cos_h - local_y * sin_h;
            let y_m = -41.0 + local_x * sin_h + local_y * cos_h;
            let heading = (initial_heading_deg + direction * offset_deg).rem_euclid(360.0);
            sample(time_s, x_m, y_m, heading * DEG, 4.0, 0.0, direction * 0.1)
        })
        .collect();

    Trajectory {
        vessel_id: "oracle-turning-vessel".to_owned(),
        vessel_length_m: 84.0,
        samples,
    }
}

#[test]
fn turning_circle_matches_independent_oracle_for_both_directions_and_heading_wrap() {
    for (direction, initial_heading) in [(1.0, 300.0), (-1.0, 40.0)] {
        let trajectory = circular_trajectory(direction, initial_heading);
        let expected = reference_turning_circle(&trajectory);
        let actual = turning_circle_metrics(&trajectory).expect("turning circle");

        assert_close(actual.advance_at_90_m, expected[0]);
        assert_close(actual.transfer_at_90_m, expected[1]);
        assert_close(actual.tactical_diameter_at_180_m, expected[2]);
        assert_close(actual.time_to_90_s, expected[3]);
        assert_close(actual.time_to_180_s, expected[4]);
    }
}

fn reference_speed(item: TrajectorySample) -> f64 {
    (item.surge_mps * item.surge_mps + item.sway_mps * item.sway_mps).sqrt()
}

fn reference_stopping(
    trajectory: &Trajectory,
    command_index: usize,
    threshold_mps: f64,
) -> [f64; 4] {
    let samples = &trajectory.samples;
    let command = samples[command_index];
    let mut intersection = None;

    for pair in samples[command_index..].windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let speed_a = reference_speed(a);
        let speed_b = reference_speed(b);
        if speed_a > threshold_mps && speed_b <= threshold_mps {
            let fraction = (speed_a - threshold_mps) / (speed_a - speed_b);
            intersection = Some(ReferencePoint {
                time_s: a.time_s + fraction * (b.time_s - a.time_s),
                x_m: a.x_m + fraction * (b.x_m - a.x_m),
                y_m: a.y_m + fraction * (b.y_m - a.y_m),
            });
            break;
        }
    }

    let stop = intersection.expect("fixture crosses stop threshold");
    let (sin_h, cos_h) = command.heading_rad.sin_cos();
    let dx = stop.x_m - command.x_m;
    let dy = stop.y_m - command.y_m;
    [
        stop.time_s - command.time_s,
        dx * cos_h + dy * sin_h,
        -dx * sin_h + dy * cos_h,
        threshold_mps,
    ]
}

#[test]
fn stopping_metrics_match_independent_oracle_with_nonuniform_sampling_and_rotated_track() {
    let trajectory = Trajectory {
        vessel_id: "oracle-stop-vessel".to_owned(),
        vessel_length_m: 51.0,
        samples: vec![
            sample(10.0, 100.0, 200.0, 35.0 * DEG, 11.0, 0.0, 0.0),
            sample(10.3, 110.0, 207.0, 35.0 * DEG, 9.0, 0.0, 0.0),
            sample(11.5, 118.0, 213.0, 35.0 * DEG, 6.0, 0.0, 0.0),
            sample(13.0, 123.0, 218.0, 35.0 * DEG, 2.0, 0.0, 0.0),
            sample(14.0, 126.0, 220.0, 35.0 * DEG, 0.5, 0.0, 0.0),
        ],
    };
    let expected = reference_stopping(&trajectory, 1, 1.25);
    let actual = stopping_metrics(&trajectory, 1, 1.25).expect("stopping threshold");

    assert_close(actual.time_from_command_s, expected[0]);
    assert_close(actual.along_track_displacement_m, expected[1]);
    assert_close(actual.cross_track_displacement_m, expected[2]);
    assert_close(actual.speed_at_threshold_mps, expected[3]);
}

fn reference_zig_zag_peak(
    samples: &[TrajectorySample],
    headings_deg: &[f64],
    from_index: usize,
    through_index: usize,
    direction: f64,
) -> (f64, f64) {
    for right in (from_index + 1)..=through_index {
        let left = right - 1;
        let before = samples[left].yaw_rate_rad_s * direction;
        let after = samples[right].yaw_rate_rad_s * direction;
        let fraction = if before > 1.0e-12 && after < -1.0e-12 {
            Some(before / (before - after))
        } else if before > 1.0e-12
            && after.abs() <= 1.0e-12
            && right < through_index
            && samples[right + 1].yaw_rate_rad_s * direction < -1.0e-12
        {
            Some(1.0)
        } else {
            None
        };

        if let Some(ratio) = fraction {
            let a = samples[left];
            let b = samples[right];
            let time = a.time_s + ratio * (b.time_s - a.time_s);
            let heading = headings_deg[left] + ratio * (headings_deg[right] - headings_deg[left]);
            return (time, heading);
        }
    }
    panic!("fixture has a sampled yaw-rate reversal");
}

fn reference_zig_zag(trajectory: &Trajectory, commands: &[ZigZagCommand], target: f64) -> [f64; 4] {
    let samples = &trajectory.samples;
    let headings = reference_unwrapped_heading_deg(samples);
    let command_indices: Vec<usize> = commands
        .iter()
        .map(|command| {
            samples
                .iter()
                .position(|item| (item.time_s - command.time_s).abs() < 1.0e-12)
                .expect("fixture command is sample-aligned")
        })
        .collect();

    let first_direction = headings
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .find(|delta| delta.abs() > 1.0e-12)
        .expect("fixture turns")
        .signum();

    let first_peak = reference_zig_zag_peak(
        samples,
        &headings,
        command_indices[1],
        command_indices[2],
        first_direction,
    );
    let second_peak = reference_zig_zag_peak(
        samples,
        &headings,
        command_indices[2],
        samples.len() - 1,
        -first_direction,
    );

    [
        first_direction * (first_peak.1 - headings[0]) - target,
        -first_direction * (second_peak.1 - headings[0]) - target,
        first_peak.0,
        second_peak.0,
    ]
}

#[test]
fn zig_zag_metrics_match_independent_oracle_across_heading_wrap_and_irregular_time_steps() {
    let times: [f64; 12] = [0.0, 0.4, 0.9, 1.6, 2.4, 3.6, 4.2, 5.4, 6.8, 7.7, 9.1, 10.5];
    let offsets_deg: [f64; 12] = [
        0.0, 4.0, 11.0, 15.0, 14.0, 8.0, 0.0, -10.0, -14.0, -16.0, -15.0, -10.0,
    ];
    let yaw_rates_deg_s: [f64; 12] = [
        3.0, 2.0, 1.0, 0.6, 0.0, -1.0, -2.0, -3.0, -1.0, 0.0, 1.0, 2.0,
    ];
    let samples = (0..times.len())
        .map(|index| {
            let heading_deg = (350.0 + offsets_deg[index]).rem_euclid(360.0);
            sample(
                times[index],
                index as f64 * 2.0,
                index as f64 * 0.3,
                heading_deg * DEG,
                5.0,
                0.0,
                yaw_rates_deg_s[index] * DEG,
            )
        })
        .collect();
    let trajectory = Trajectory {
        vessel_id: "oracle-zig-zag-vessel".to_owned(),
        vessel_length_m: 100.0,
        samples,
    };
    let commands = [
        ZigZagCommand {
            time_s: 0.0,
            rudder_deg: 10.0,
        },
        ZigZagCommand {
            time_s: 0.9,
            rudder_deg: -10.0,
        },
        ZigZagCommand {
            time_s: 5.4,
            rudder_deg: 10.0,
        },
    ];

    let expected = reference_zig_zag(&trajectory, &commands, 10.0);
    let actual = zig_zag_metrics(&trajectory, &commands, 10.0).expect("zig-zag overshoots");

    assert_close(actual.first_overshoot_deg, expected[0]);
    assert_close(actual.second_overshoot_deg, expected[1]);
    assert_close(actual.first_peak_time_s, expected[2]);
    assert_close(actual.second_peak_time_s, expected[3]);
}
