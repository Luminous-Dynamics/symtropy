use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    AnalyticPath, Vec3, chord_collision, endpoints_only_collision,
};

fn percentile(values: &[Duration], percentile: usize) -> Duration {
    let index = ((values.len() - 1) * percentile).div_ceil(100);
    values[index]
}

fn measure(label: &str, mut operation: impl FnMut()) {
    let mut durations = Vec::with_capacity(101);
    for _ in 0..101 {
        let start = Instant::now();
        operation();
        durations.push(start.elapsed());
    }
    durations.sort_unstable();
    println!(
        "{label},p50_ns={},p95_ns={},p99_ns={}",
        percentile(&durations, 50).as_nanos(),
        percentile(&durations, 95).as_nanos(),
        percentile(&durations, 99).as_nanos()
    );
}

fn main() {
    let linear = AnalyticPath::Linear {
        start: Vec3::new(-1_000.0, 0.0, 0.0),
        end: Vec3::new(1_000.0, 0.0, 0.0),
    };
    let arc = AnalyticPath::UpperSemicircle {
        radius_mm: 1_000.0,
        z_mm: 0.0,
    };
    let linear_obstacle = Vec3::new(0.0, 0.0, 0.0);
    let arc_obstacle = Vec3::new(0.0, 1_000.0, 0.0);

    println!("case,metric,value");
    println!(
        "line-thin-barrier,analytic_collision,{}",
        linear.collides_with_stationary_sphere(10.0, linear_obstacle, 5.0)
    );
    println!(
        "line-thin-barrier,endpoint_collision,{}",
        endpoints_only_collision(linear, 10.0, linear_obstacle, 5.0)
    );
    println!(
        "arc-chord-miss,analytic_collision,{}",
        arc.collides_with_stationary_sphere(20.0, arc_obstacle, 20.0)
    );
    println!(
        "arc-chord-miss,chord_collision,{}",
        chord_collision(arc, 20.0, arc_obstacle, 20.0)
    );

    measure("linear-analytic-ccd", || {
        black_box(linear.collides_with_stationary_sphere(
            10.0,
            linear_obstacle,
            5.0,
        ));
    });
    measure("semicircle-analytic-ccd", || {
        black_box(arc.collides_with_stationary_sphere(
            20.0,
            arc_obstacle,
            20.0,
        ));
    });
}
