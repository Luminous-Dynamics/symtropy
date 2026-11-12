use nonlinear_midpoint_error_envelope::{
    InconclusiveReasonV1, JacobianLipschitzWitnessV1, KantorovichProfileV1,
    MidpointKktStateV1, MidpointParametersV1, QualificationStatusV1,
    qualify_midpoint_state_v1,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = KantorovichProfileV1::default();
    let lipschitz = JacobianLipschitzWitnessV1::for_quadratic_midpoint_fixture();

    println!("mass,time_step,residual_norm,newton_correction,beta_estimate,h,radius,status");
    for mass in [1.0, 10.0, 100.0, 1000.0] {
        for time_step in [0.25, 0.5, 1.0, 2.0] {
            let parameters = MidpointParametersV1 {
                q0: 0.0,
                v0: 0.0,
                time_step,
                mass,
                applied_force: 0.0,
            };
            let epsilon = 2.0_f64.powi(-12);
            let state = MidpointKktStateV1 {
                q1: 2.0 + epsilon,
                v1: 4.0 / time_step - epsilon / (2.0 * time_step),
                multiplier: 2.0 * mass / time_step - epsilon * mass / (2.0 * time_step),
            };
            let receipt =
                qualify_midpoint_state_v1(parameters, state, lipschitz, profile)?;
            let status = match receipt.status {
                QualificationStatusV1::AdmissibleLocalRootEstimate => "ADMISSIBLE_ESTIMATE",
                QualificationStatusV1::Inconclusive(InconclusiveReasonV1::SingularKktJacobian) => {
                    "INCONCLUSIVE_SINGULAR"
                }
                QualificationStatusV1::Inconclusive(
                    InconclusiveReasonV1::KantorovichProductTooLarge,
                ) => "INCONCLUSIVE_KANTOROVICH",
                QualificationStatusV1::Inconclusive(
                    InconclusiveReasonV1::CandidateBallLeavesPositiveConstraintBranch,
                ) => "INCONCLUSIVE_DOMAIN",
                QualificationStatusV1::Inconclusive(
                    InconclusiveReasonV1::CandidateRadiusExceedsProfile,
                ) => "INCONCLUSIVE_RADIUS",
            };
            println!(
                "{mass:.4},{time_step:.4},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{status}",
                receipt.residual_norm,
                receipt.newton_correction_norm,
                receipt.inverse_jacobian_norm_upper_estimate,
                receipt.kantorovich_product,
                receipt.local_radius_estimate
            );
        }
    }
    Ok(())
}
