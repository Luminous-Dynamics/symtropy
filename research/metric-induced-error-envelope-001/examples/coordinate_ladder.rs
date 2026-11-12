use metric_induced_error_envelope_reference::{
    CartesianQ, Cotangent, DenseY, Metric, MetricProfileV1, Rebasis, Tangent, vector_inf,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = MetricProfileV1::default();
    let metric_q = Metric::<CartesianQ, 3>::admit(
        [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
        profile,
    )?;
    let vector_y = Tangent::<DenseY, 3>::new([1.0, -0.5, 0.25])?;
    let covector_q = Cotangent::<CartesianQ, 3>::new([1.0, -2.0, 0.5])?;

    println!(
        "shear,condition_inf,vector_inf_q,vector_inf_y,covector_inf_q,covector_inf_y,tangent_norm_q,tangent_norm_y,cotangent_norm_q,cotangent_norm_y,pairing_q,pairing_y"
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
        let metric_y = rebasis.metric_to_to(&metric_q, profile)?;
        let vector_q = rebasis.tangent_to_from(&vector_y)?;
        let covector_y = rebasis.cotangent_to_to(&covector_q)?;

        let tangent_norm_q = metric_q.tangent_norm(&vector_q)?;
        let tangent_norm_y = metric_y.tangent_norm(&vector_y)?;
        let cotangent_norm_q = metric_q.cotangent_norm(&covector_q)?;
        let cotangent_norm_y = metric_y.cotangent_norm(&covector_y)?;
        let pairing_q = metric_induced_error_envelope_reference::pairing(&covector_q, &vector_q);
        let pairing_y = metric_induced_error_envelope_reference::pairing(&covector_y, &vector_y);

        println!(
            "{shear:.1},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}",
            rebasis.condition_inf_estimate()?,
            vector_inf(vector_q.components()),
            vector_inf(vector_y.components()),
            vector_inf(covector_q.components()),
            vector_inf(covector_y.components()),
            tangent_norm_q,
            tangent_norm_y,
            cotangent_norm_q,
            cotangent_norm_y,
            pairing_q,
            pairing_y
        );
    }
    Ok(())
}
