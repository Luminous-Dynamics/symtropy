use metric_induced_error_envelope_reference::{
    CartesianQ, ConstraintEquationRebasisV1, ConstraintMultiplierV1, ConstraintResidualV1,
    ConstraintSpace, CoordinateFrame, DelassusMetricV1, Metric, MetricProfileV1,
    constraint_pairing,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConstraintSpaceScaled;
impl CoordinateFrame for ConstraintSpaceScaled {}

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
        "row_scale,condition_inf,jacobian_component_growth,delassus_relative_defect,multiplier_norm_drift,residual_norm_drift,pairing_drift,minimum_correction_defect"
    );

    for scale in [0.01_f64, 0.1, 0.5, 1.0, 2.0, 10.0, 100.0] {
        let change = ConstraintEquationRebasisV1::<ConstraintSpaceScaled, ConstraintSpace, 2>::new(
            [[scale, 0.0], [0.0, 1.0]],
            profile,
        )?;
        let jacobian_new = change.jacobian_to_new(&jacobian_old)?;
        let delassus_new =
            DelassusMetricV1::<CartesianQ, ConstraintSpaceScaled, 3, 2>::from_mass_jacobian(
                &mass,
                jacobian_new,
                profile,
            )?;
        let multiplier_new = change.multiplier_to_new(&multiplier_old, profile)?;
        let residual_new = change.residual_to_new(&residual_old)?;

        let transform = change.matrix();
        let mut delassus_relative_defect = 0.0_f64;
        for row in 0..2 {
            for column in 0..2 {
                let expected = transform[row][0]
                    * (delassus_old.matrix()[0][0] * transform[column][0]
                        + delassus_old.matrix()[0][1] * transform[column][1])
                    + transform[row][1]
                        * (delassus_old.matrix()[1][0] * transform[column][0]
                            + delassus_old.matrix()[1][1] * transform[column][1]);
                let scale = expected.abs().max(1.0);
                delassus_relative_defect = delassus_relative_defect
                    .max((delassus_new.matrix()[row][column] - expected).abs() / scale);
            }
        }
        let multiplier_norm_drift =
            (delassus_new.multiplier_norm(&multiplier_new)? - multiplier_norm_old).abs();
        let residual_norm_drift =
            (delassus_new.residual_norm(&residual_new)? - residual_norm_old).abs();
        let pairing_drift =
            (constraint_pairing(&multiplier_new, &residual_new)? - pairing_old).abs();

        let correction_old = delassus_old.minimum_kinetic_correction(&residual_old)?;
        let correction_new = delassus_new.minimum_kinetic_correction(&residual_new)?;
        let minimum_correction_defect = correction_old
            .components()
            .iter()
            .zip(correction_new.components())
            .map(|(left, right)| (left - right).abs())
            .fold(0.0_f64, f64::max);

        assert!(delassus_relative_defect <= 3.0e-10);
        assert!(multiplier_norm_drift <= 3.0e-10);
        assert!(residual_norm_drift <= 3.0e-10);
        assert!(pairing_drift <= 3.0e-10);
        assert!(minimum_correction_defect <= 3.0e-10);

        println!(
            "{scale:.6e},{:.8e},{:.8e},{delassus_relative_defect:.8e},{multiplier_norm_drift:.8e},{residual_norm_drift:.8e},{pairing_drift:.8e},{minimum_correction_defect:.8e}",
            change.condition_inf_estimate()?,
            jacobian_new
                .iter()
                .map(|row| row.iter().map(|v| v.abs()).fold(0.0, f64::max))
                .fold(0.0, f64::max)
                / jacobian_old
                    .iter()
                    .map(|row| row.iter().map(|v| v.abs()).fold(0.0, f64::max))
                    .fold(1.0, f64::max),
        );
    }

    Ok(())
}
