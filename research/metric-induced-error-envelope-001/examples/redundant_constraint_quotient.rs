use metric_induced_error_envelope_reference::{
    ConstraintMultiplierV1, ConstraintQuotientProfileV1, ConstraintResidualStatusV1,
    ConstraintResidualV1, CartesianQ, CoordinateFrame, Metric, MetricProfileV1,
    RankDeficientDelassusQuotientV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConstraintRows;
impl CoordinateFrame for ConstraintRows {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metric_profile = MetricProfileV1::default();
    let quotient_profile = ConstraintQuotientProfileV1::default();
    let mass = Metric::<CartesianQ, 3>::admit(
        [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
        metric_profile,
    )?;
    let residual_metric =
        Metric::<ConstraintRows, 3>::admit(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            metric_profile,
        )?;
    let jacobian = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0], [1.0, 0.0, 1.0]];
    let quotient = RankDeficientDelassusQuotientV1::<
        CartesianQ,
        OldConstraintSpace,
        3,
        3,
    >::new(&mass, jacobian, &residual_metric, quotient_profile)?;

    let compatible = ConstraintResidualV1::<ConstraintRows, 3>::new([0.4, -0.7, 0.4])?;
    let incompatible =
        ConstraintResidualV1::<OldConstraintSpace, 3>::new([0.4, -0.7, 0.41])?;
    let accepted = quotient.solve_residual(&compatible)?;
    let rejected = quotient.solve_residual(&incompatible)?;

    println!(
        "rank={},multiplier_nullity={},compatible_status={:?},compatibility_defect={:.8e},residual_dual_norm={:.12e},minimum_kinetic_norm={:.12e}",
        quotient.rank(),
        quotient.nullity(),
        accepted.status(),
        accepted.compatibility_relative_defect(),
        accepted.residual_dual_norm().unwrap_or(f64::NAN),
        accepted.minimum_kinetic_correction_norm().unwrap_or(f64::NAN),
    );
    println!(
        "incompatible_status={:?},incompatible_defect={:.8e},correction_authorized={}",
        rejected.status(),
        rejected.compatibility_relative_defect(),
        rejected.minimum_kinetic_correction().is_some()
    );

    let lambda = ConstraintMultiplierV1::<ConstraintRows, 3>::new([0.5, -0.75, 1.25])?;
    let shifted =
        ConstraintMultiplierV1::<OldConstraintSpace, 3>::new([3.5, -0.75, -1.75])?;
    println!(
        "multiplier_quotient_norm={:.12e},null_shifted_norm={:.12e}",
        quotient.multiplier_quotient_norm(&lambda)?,
        quotient.multiplier_quotient_norm(&shifted)?
    );

    if accepted.status() != ConstraintResidualStatusV1::Compatible
        || rejected.status() != ConstraintResidualStatusV1::Incompatible
        || accepted.minimum_kinetic_correction().is_none()
        || rejected.minimum_kinetic_correction().is_some()
    {
        return Err("rank-deficient constraint quotient acceptance contract failed".into());
    }

    Ok(())
}
