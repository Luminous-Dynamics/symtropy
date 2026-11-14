use metric_induced_error_envelope_reference::nonlinear_kkt_certificate::{
    NewtonKantorovichBoundEvidenceV1, NewtonKantorovichInputsV1,
    NewtonKantorovichProfileV1, NewtonKantorovichStatusV1,
    certify_newton_kantorovich_local_root,
};

fn evidence(
    problem: &str,
    linearization: &str,
    norm: &str,
    receipt: &str,
    bound: f64,
) -> NewtonKantorovichBoundEvidenceV1 {
    NewtonKantorovichBoundEvidenceV1 {
        problem_id: problem.to_owned(),
        linearization_id: linearization.to_owned(),
        norm_identity_id: norm.to_owned(),
        receipt_id: receipt.to_owned(),
        bound_value: bound,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = NewtonKantorovichProfileV1::default();
    let problem = "synthetic-scalar-newton-kantorovich-v0.1";
    let linearization = "synthetic-preconditioned-jacobian-v0.1";
    let norm = "synthetic-mass-delassus-block-norm-v0.1";

    println!(
        "eta_upper,gamma_upper,h,margin_to_half,radius,q,exact_scalar_root,root_residual,status"
    );
    for (eta, gamma) in [
        (0.10, 2.00),
        (0.20, 2.00),
        (0.249, 2.00),
        (0.25, 2.00),
        (0.30, 2.00),
        (0.40, 0.00),
        (0.00, 100.0),
    ] {
        let inputs = NewtonKantorovichInputsV1 {
            problem_id: problem.to_owned(),
            linearization_id: linearization.to_owned(),
            norm_identity_id: norm.to_owned(),
            eta_problem_id: problem.to_owned(),
            gamma_problem_id: problem.to_owned(),
            eta_linearization_id: linearization.to_owned(),
            gamma_linearization_id: linearization.to_owned(),
            eta_norm_identity_id: norm.to_owned(),
            gamma_norm_identity_id: norm.to_owned(),
            eta_receipt_id: "analytic-scalar-eta".to_owned(),
            gamma_bound_receipt_id: "analytic-scalar-gamma".to_owned(),
            eta_upper_bound: eta,
            preconditioned_jacobian_lipschitz_upper_bound: gamma,
        };

        let receipt = certify_newton_kantorovich_local_root(&inputs, profile)?;
        let radius = receipt.local_solution_radius.unwrap_or(f64::NAN);
        let contraction = receipt.contraction_factor.unwrap_or(f64::NAN);
        let exact_root = if receipt.local_solution_radius.is_some() {
            if gamma == 0.0 {
                Some(-eta)
            } else {
                Some(-2.0 * eta / (1.0 + (1.0 - 2.0 * eta * gamma).sqrt()))
            }
        } else {
            None
        };
        let root_residual = exact_root
            .map(|root| eta + root + 0.5 * gamma * root * root)
            .unwrap_or(f64::NAN);

        if let Some(root) = exact_root {
            assert!((root.abs() - radius).abs() <= 2.0e-12 * radius.abs().max(1.0));
            assert!(root_residual.abs() <= 2.0e-12);
            assert!(contraction < 1.0);
            assert!(contraction <= profile.maximum_contraction_factor);
        }

        println!(
            "{eta:.6},{gamma:.6},{:.8e},{:.8e},{radius:.8e},{contraction:.8e},{:.8e},{root_residual:.8e},{:?}",
            receipt.kantorovich_parameter,
            receipt.margin_to_half,
            exact_root.unwrap_or(f64::NAN),
            receipt.status
        );
    }
    Ok(())
}
