use metric_induced_error_envelope_reference::{
    CartesianQ, ConstraintEquationRebasisV1, ConstraintMultiplierV1, ConstraintResidualV1,
    ConstraintSpace, CoordinateFrame, DelassusMetricV1, Metric, MetricProfileV1,
    constraint_pairing,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConstraintSpaceSheared;
impl CoordinateFrame for ConstraintSpaceSheared {}

fn max_abs<const N: usize>(values: &[f64; N]) -> f64 {
    values.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = MetricProfileV1::default();
    let mass = Metric::<CartesianQ, 3>::admit(
        [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
        profile,
    )?;
    let jacobian_old = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
    let delassus_old = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
        &mass,
        jacobian_old,
        profile,
    )?;
    let multiplier_old = ConstraintMultiplierV1::<ConstraintSpace, 2>::new([0.75, -1.25])?;
    let residual_old = ConstraintResidualV1::<ConstraintSpace, 2>::new([0.25, -0.4])?;
    let multiplier_norm_old = delassus_old.multiplier_norm(&multiplier_old)?;
    let residual_norm_old = delassus_old.residual_norm(&residual_old)?;
    let pairing_old = constraint_pairing(&multiplier_old, &residual_old)?;

    println!(
        "constraint_shear,condition_inf,jacobian_component_growth,delassus_relative_defect,multiplier_norm_drift,residual_norm_drift,pairing_drift,induced_force_defect,min_correction_kinetic_defect"
    );

    for shear in [1.0_f64, 2.0, 4.0, 8.0, 12.0, 16.0, 24.0] {
        // r_new = T r_old, lambda_new = T^-T lambda_old, J_new = T J_old.
        let equation_rebasis = ConstraintEquationRebasisV1::<
            ConstraintSpaceSheared,
            ConstraintSpace,
            2,
        >::new([[1.0, shear], [0.0, 1.0]], profile)?;
        let jacobian_new = equation_rebasis.jacobian_to_new(&jacobian_old)?;
        let delassus_new =
            DelassusMetricV1::<CartesianQ, ConstraintSpaceSheared, 3, 2>::from_mass_jacobian(
                &mass,
                jacobian_new,
                profile,
            )?;
        let multiplier_new = equation_rebasis.multiplier_to_new(&multiplier_old, profile)?;
        let residual_new = equation_rebasis.residual_to_new(&residual_old)?;

        let transform = equation_rebasis.matrix();
        let old_metric = delassus_old.matrix();
        let new_metric = delassus_new.matrix();
        let mut delassus_relative_defect = 0.0_f64;
        for (new_row, transform_row) in new_metric.iter().zip(transform.iter()) {
            for (actual, transform_column) in new_row.iter().zip(transform.iter()) {
                let expected = transform_row
                    .iter()
                    .zip(old_metric.iter())
                    .map(|(left, old_row)| {
                        old_row
                            .iter()
                            .zip(transform_column.iter())
                            .map(|(metric_value, right)| left * metric_value * right)
                            .sum::<f64>()
                    })
                    .sum::<f64>();
                let scale = expected.abs().max(1.0);
                delassus_relative_defect =
                    delassus_relative_defect.max((*actual - expected).abs() / scale);
            }
        }

        let multiplier_norm_drift =
            (delassus_new.multiplier_norm(&multiplier_new)? - multiplier_norm_old).abs();
        let residual_norm_drift =
            (delassus_new.residual_norm(&residual_new)? - residual_norm_old).abs();
        let pairing_drift =
            (constraint_pairing(&multiplier_new, &residual_new)? - pairing_old).abs();

        let force_old = delassus_old.induced_velocity_correction(&multiplier_old)?;
        let force_new = delassus_new.induced_velocity_correction(&multiplier_new)?;
        let induced_force_defect = force_old
            .components()
            .iter()
            .zip(force_new.components())
            .map(|(left, right)| (left - right).abs())
            .fold(0.0_f64, f64::max);

        let correction_old = delassus_old.minimum_kinetic_correction(&residual_old)?;
        let correction_new = delassus_new.minimum_kinetic_correction(&residual_new)?;
        let min_correction_kinetic_defect =
            (mass.tangent_norm(&correction_old)? - mass.tangent_norm(&correction_new)?).abs();

        assert!(delassus_relative_defect <= 2.0e-10);
        assert!(multiplier_norm_drift <= 2.0e-10);
        assert!(residual_norm_drift <= 2.0e-10);
        assert!(pairing_drift <= 2.0e-10);
        assert!(induced_force_defect <= 2.0e-10);
        assert!(min_correction_kinetic_defect <= 2.0e-10);

        println!(
            "{shear:.1},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e}",
            equation_rebasis.condition_inf_estimate()?,
            jacobian_new.iter().map(max_abs).fold(0.0_f64, f64::max)
                / jacobian_old.iter().map(max_abs).fold(1.0_f64, f64::max),
            delassus_relative_defect,
            multiplier_norm_drift,
            residual_norm_drift,
            pairing_drift,
            induced_force_defect,
            min_correction_kinetic_defect,
        );
    }

    Ok(())
}
