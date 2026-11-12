use metric_induced_error_envelope_reference::{
    CoordinateChange, FrameMetric, MetricProfileV1, Tangent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cartesian;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Dense;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = MetricProfileV1 {
        relative_symmetry_tolerance: 1.0e-12,
        relative_pivot_tolerance: 1.0e-14,
    };
    let metric_q = FrameMetric::<Cartesian, 3>::new(
        [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
        profile,
    )?;
    let vector_y = Tangent::<Dense, 3>::new([1.0, -0.5, 0.25]);

    println!(
        "shear,condition_inf,coordinate_inf_q,coordinate_inf_y,metric_norm_q,metric_norm_y,relative_metric_error"
    );
    for shear in [1.0, 2.0, 4.0, 8.0, 12.0] {
        let change = CoordinateChange::<Cartesian, Dense, 3>::new(
            [
                [1.0, shear, 0.5 * shear * shear],
                [0.0, 1.0, shear],
                [0.0, 0.0, 1.0],
            ],
            profile,
        )?;
        let metric_y = change.pullback_metric(&metric_q)?;
        let vector_q = change.pushforward_tangent(&vector_y)?;
        let norm_q = metric_q.tangent_norm(&vector_q)?;
        let norm_y = metric_y.tangent_norm(&vector_y)?;
        let relative_error = (norm_q - norm_y).abs() / norm_q.abs().max(norm_y.abs()).max(1.0e-300);
        let q_inf = vector_q
            .components()
            .iter()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        let y_inf = vector_y
            .components()
            .iter()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        println!(
            "{shear:.1},{:.8e},{q_inf:.8e},{y_inf:.8e},{norm_q:.12e},{norm_y:.12e},{relative_error:.8e}",
            change.condition_inf_estimate()
        );
    }
    Ok(())
}
