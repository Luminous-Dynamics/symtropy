use metric_induced_error_envelope_reference::{
    CartesianQ, ConstraintMultiplierV1, ConstraintResidualV1, ConstraintSpace, DelassusMetricV1,
    DenseY, Metric, MetricProfileV1, Rebasis, rebase_constraint_jacobian,
};

fn max_abs<const N: usize>(values: &[f64; N]) -> f64 {
    values.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = MetricProfileV1::default();
    let mass_q = Metric::<CartesianQ, 3>::admit(
        [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
        profile,
    )?;
    let jacobian_q = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
    let delassus_q = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
        &mass_q, jacobian_q, profile,
    )?;
    let multiplier = ConstraintMultiplierV1::<ConstraintSpace, 2>::new([0.75, -1.25])?;
    let residual = ConstraintResidualV1::<ConstraintSpace, 2>::new([0.25, -0.4])?;
    let multiplier_norm_q = delassus_q.multiplier_norm(&multiplier)?;
    let residual_norm_q = delassus_q.residual_norm(&residual)?;

    println!(
        "shear,condition_inf,jacobian_inf_q,jacobian_inf_y,delassus_max_abs_defect,multiplier_norm_q,multiplier_norm_y,residual_norm_q,residual_norm_y,min_correction_kinetic_q,min_correction_kinetic_y"
    );

    for shear in [1.0, 2.0, 4.0, 8.0, 12.0, 16.0, 24.0] {
        let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [
                [1.0, shear, 0.5 * shear * shear],
                [0.0, 1.0, shear],
                [0.0, 0.0, 1.0],
            ],
            profile,
        )?;
        let mass_y = rebasis.metric_to_to(&mass_q, profile)?;
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &rebasis)?;
        let delassus_y = DelassusMetricV1::<DenseY, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_y, jacobian_y, profile,
        )?;

        let mut delassus_defect = 0.0_f64;
        for row in 0..2 {
            for column in 0..2 {
                delassus_defect = delassus_defect.max(
                    (delassus_q.matrix()[row][column] - delassus_y.matrix()[row][column]).abs(),
                );
            }
        }

        let multiplier_norm_y = delassus_y.multiplier_norm(&multiplier)?;
        let residual_norm_y = delassus_y.residual_norm(&residual)?;
        let correction_q = delassus_q.minimum_kinetic_correction(&residual)?;
        let correction_y = delassus_y.minimum_kinetic_correction(&residual)?;
        let correction_norm_q = mass_q.tangent_norm(&correction_q)?;
        let correction_norm_y = mass_y.tangent_norm(&correction_y)?;

        let relative_scale = delassus_q
            .matrix()
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(1.0_f64, f64::max);
        assert!(
            delassus_defect <= 2.0e-8 * relative_scale,
            "Delassus rebasing defect {delassus_defect:e} at shear {shear}"
        );
        assert!(
            (multiplier_norm_q - multiplier_norm_y).abs() <= 2.0e-8,
            "multiplier norm mismatch at shear {shear}"
        );
        assert!(
            (residual_norm_q - residual_norm_y).abs() <= 2.0e-8,
            "residual norm mismatch at shear {shear}"
        );

        println!(
            "{shear:.1},{:.8e},{:.8e},{:.8e},{:.8e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}",
            rebasis.condition_inf_estimate()?,
            jacobian_q.iter().map(max_abs).fold(0.0_f64, f64::max),
            jacobian_y.iter().map(max_abs).fold(0.0_f64, f64::max),
            delassus_defect,
            multiplier_norm_q,
            multiplier_norm_y,
            residual_norm_q,
            residual_norm_y,
            correction_norm_q,
            correction_norm_y
        );
    }

    Ok(())
}
