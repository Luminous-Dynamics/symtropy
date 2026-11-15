use metric_induced_error_envelope_reference::{
    CartesianQ, ConstraintMultiplierV1, ConstraintResidualV1, ConstraintSpace, CoordinateFrame,
    Cotangent, DelassusMetricV1, DenseY, Metric, MetricProfileV1, Rebasis, Tangent,
    certify_linear_constrained_solution, rebase_constraint_jacobian, vector_inf,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RebasedConstraintSpace;
impl CoordinateFrame for RebasedConstraintSpace {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = MetricProfileV1::default();
    let mass_q = Metric::<CartesianQ, 3>::admit(
        [[2.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 8.0]],
        profile,
    )?;
    let jacobian_q = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
    let delassus_q = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
        &mass_q, jacobian_q, profile,
    )?;

    let exact_acceleration = Tangent::<CartesianQ, 3>::new([1.0, -2.0, 0.5])?;
    let exact_multiplier = [0.25, -0.5];
    let mass_force = mass_q.lower(&exact_acceleration)?;
    let mut force_components = [0.0; 3];
    for dof in 0..3 {
        force_components[dof] = mass_force.components()[dof]
            + jacobian_q[0][dof] * exact_multiplier[0]
            + jacobian_q[1][dof] * exact_multiplier[1];
    }
    let force_q = Cotangent::<CartesianQ, 3>::new(force_components)?;
    let constraint_rhs = ConstraintResidualV1::<ConstraintSpace, 2>::new([
        jacobian_q[0][0] * exact_acceleration.components()[0]
            + jacobian_q[0][1] * exact_acceleration.components()[1]
            + jacobian_q[0][2] * exact_acceleration.components()[2],
        jacobian_q[1][0] * exact_acceleration.components()[0]
            + jacobian_q[1][1] * exact_acceleration.components()[1]
            + jacobian_q[1][2] * exact_acceleration.components()[2],
    ])?;

    let computed_acceleration_q = Tangent::<CartesianQ, 3>::new([1.1, -2.2, 0.8])?;
    let computed_multiplier_q = ConstraintMultiplierV1::<ConstraintSpace, 2>::new([0.3, -0.6])?;
    let receipt_q = certify_linear_constrained_solution(
        &delassus_q,
        &force_q,
        &constraint_rhs,
        &computed_acceleration_q,
        &computed_multiplier_q,
    )?;

    println!(
        "coordinate_shear,constraint_shear,coordinate_condition_inf,raw_force_residual_inf,raw_constraint_residual_inf,rho,acceleration_bound,multiplier_bound,combined_state_bound,relative_rho_drift"
    );

    for coordinate_shear in [0.0, 1.0, 2.0, 4.0, 8.0] {
        let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [
                [1.0, coordinate_shear, 1.0],
                [0.0, 1.0, 3.0 * coordinate_shear],
                [0.0, 0.0, 1.0],
            ],
            profile,
        )?;
        let mass_y = rebasis.metric_to_to(&mass_q, profile)?;
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &rebasis)?;
        let force_y = rebasis.cotangent_to_to(&force_q)?;
        let computed_acceleration_y = rebasis.tangent_to_to(&computed_acceleration_q)?;

        for constraint_shear in [0.0, 1.0, 2.0, 4.0] {
            // T = [[1, t], [0, 1]], J_new = T J, b_new = T b,
            // lambda_new = T^-T lambda.
            let jacobian_y_t = [
                std::array::from_fn(|dof| {
                    jacobian_y[0][dof] + constraint_shear * jacobian_y[1][dof]
                }),
                jacobian_y[1],
            ];
            let target_y_t = ConstraintResidualV1::<RebasedConstraintSpace, 2>::new([
                constraint_rhs.components()[0] + constraint_shear * constraint_rhs.components()[1],
                constraint_rhs.components()[1],
            ])?;
            let multiplier_y_t = ConstraintMultiplierV1::<RebasedConstraintSpace, 2>::new([
                computed_multiplier_q.components()[0],
                computed_multiplier_q.components()[1]
                    - constraint_shear * computed_multiplier_q.components()[0],
            ])?;
            let delassus_y_t =
                DelassusMetricV1::<DenseY, RebasedConstraintSpace, 3, 2>::from_mass_jacobian(
                    &mass_y,
                    jacobian_y_t,
                    profile,
                )?;
            let receipt_y_t = certify_linear_constrained_solution(
                &delassus_y_t,
                &force_y,
                &target_y_t,
                &computed_acceleration_y,
                &multiplier_y_t,
            )?;
            let drift = (receipt_q.paired_residual_norm - receipt_y_t.paired_residual_norm).abs()
                / receipt_q.paired_residual_norm.max(1.0e-300);

            println!(
                "{coordinate_shear:.1},{constraint_shear:.1},{:.8e},{:.8e},{:.8e},{:.12e},{:.12e},{:.12e},{:.12e},{:.8e}",
                rebasis.condition_inf_estimate()?,
                vector_inf(receipt_q.mass_equilibrium_residual.components()),
                vector_inf(receipt_q.constraint_residual.components()),
                receipt_q.paired_residual_norm,
                receipt_q.acceleration_error_bound,
                receipt_q.multiplier_error_bound,
                receipt_q.combined_state_error_bound,
                drift
            );
        }
    }
    Ok(())
}
