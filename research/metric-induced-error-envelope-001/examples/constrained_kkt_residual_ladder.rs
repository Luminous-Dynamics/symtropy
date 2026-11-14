use metric_induced_error_envelope_reference::{
    CartesianQ, ConstraintMultiplierV1, ConstraintResidualV1, ConstraintSpace, Cotangent,
    DelassusMetricV1, DenseY, Metric, MetricProfileV1, Rebasis, Tangent,
    certify_linear_constrained_solution, rebase_constraint_jacobian, vector_inf,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = MetricProfileV1::default();
    let mass_q = Metric::<CartesianQ, 3>::admit(
        [[2.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 8.0]],
        profile,
    )?;
    let jacobian_q = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
    let delassus_q = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
        &mass_q,
        jacobian_q,
        profile,
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

    let computed_acceleration_q =
        Tangent::<CartesianQ, 3>::new([1.1, -2.2, 0.8])?;
    let computed_multiplier =
        ConstraintMultiplierV1::<ConstraintSpace, 2>::new([0.3, -0.6])?;
    let receipt_q = certify_linear_constrained_solution(
        &delassus_q,
        &force_q,
        &constraint_rhs,
        &computed_acceleration_q,
        &computed_multiplier,
    )?;

    println!(
        "shear,condition_inf,raw_force_residual_inf,raw_constraint_residual_inf,rho,acceleration_bound,multiplier_bound,combined_state_bound,rebased_rho_relative_drift"
    );
    for shear in [0.0, 1.0, 2.0, 4.0, 8.0] {
        let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [
                [1.0, shear, 1.0],
                [0.0, 1.0, 3.0 * shear],
                [0.0, 0.0, 1.0],
            ],
            profile,
        )?;
        let mass_y = rebasis.metric_to_to(&mass_q, profile)?;
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &rebasis)?;
        let delassus_y =
            DelassusMetricV1::<DenseY, ConstraintSpace, 3, 2>::from_mass_jacobian(
                &mass_y,
                jacobian_y,
                profile,
            )?;
        let force_y = rebasis.cotangent_to_to(&force_q)?;
        let computed_acceleration_y =
            rebasis.tangent_to_to(&computed_acceleration_q)?;
        let receipt_y = certify_linear_constrained_solution(
            &delassus_y,
            &force_y,
            &constraint_rhs,
            &computed_acceleration_y,
            &computed_multiplier,
        )?;
        let drift = (receipt_q.paired_residual_norm - receipt_y.paired_residual_norm).abs()
            / receipt_q.paired_residual_norm.max(1.0e-300);

        println!(
            "{shear:.1},{:.8e},{:.8e},{:.8e},{:.12e},{:.12e},{:.12e},{:.12e},{:.8e}",
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
    Ok(())
}
