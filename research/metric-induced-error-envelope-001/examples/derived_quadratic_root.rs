use metric_induced_error_envelope_reference::{
    nonlinear_kkt_certificate::{
        NewtonKantorovichProfileV1,
        NewtonKantorovichStatusV1,
        certify_newton_kantorovich_local_root,
    },
    outward_interval::IntervalV1,
    scalar_krawczyk::{
        ScalarKrawczykStatusV1,
        scalar_krawczyk_inclusion,
    },
    scalar_quadratic_bounds::derive_scalar_quadratic_bounds,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // F(x) = 1/8 + x + x^2. The exact root nearest x0=-1/8 is
    // (-1 + sqrt(1/2))/2, and F'(x0)=3/4.
    let evidence = derive_scalar_quadratic_bounds(
        "analytic-quadratic-root-v0.1",
        "quadratic-linearization-at-minus-one-eighth",
        "scalar-absolute-value-v0.1",
        0.125,
        1.0,
        2.0,
        -0.125,
    )?;

    let newton = certify_newton_kantorovich_local_root(
        &evidence.newton_kantorovich_inputs(),
        NewtonKantorovichProfileV1::default(),
    )?;
    assert_eq!(
        newton.status,
        NewtonKantorovichStatusV1::ConditionalLocalRootCertificate
    );

    // Independently construct F'(X) for a fixed interval X and use Krawczyk.
    let search_box = IntervalV1::new(-0.2, -0.05)?;
    let derivative_over_box = IntervalV1::point(1.0)?
        .add(IntervalV1::point(2.0)?.mul(search_box)?)?;
    let preconditioner = 4.0 / 3.0;
    let krawczyk = scalar_krawczyk_inclusion(
        evidence.center,
        preconditioner,
        evidence.function_value_at_center,
        derivative_over_box,
        search_box,
    )?;
    assert_eq!(
        krawczyk.status,
        ScalarKrawczykStatusV1::ConditionalUniqueRoot
    );

    let exact_root = (-1.0 + (0.5_f64).sqrt()) / 2.0;
    let radius = newton.local_solution_radius.unwrap();
    assert!((exact_root - evidence.center).abs() <= radius);
    assert!(search_box.contains(exact_root));
    assert!(krawczyk.krawczyk_image.contains(exact_root));

    println!("polynomial_model={}", evidence.polynomial_model_id);
    println!(
        "F(x0)=[{:.17e},{:.17e}]",
        evidence.function_value_at_center.lower(),
        evidence.function_value_at_center.upper()
    );
    println!(
        "F'(x0)=[{:.17e},{:.17e}]",
        evidence.derivative_at_center.lower(),
        evidence.derivative_at_center.upper()
    );
    println!(
        "eta_interval=[{:.17e},{:.17e}] eta_upper={:.17e}",
        evidence.eta_interval.lower(),
        evidence.eta_interval.upper(),
        evidence.eta_upper_bound
    );
    println!(
        "gamma_interval=[{:.17e},{:.17e}] gamma_upper={:.17e}",
        evidence.gamma_interval.lower(),
        evidence.gamma_interval.upper(),
        evidence.gamma_upper_bound
    );
    println!(
        "newton_h={:.17e} radius={:.17e} q={:.17e} status={:?}",
        newton.kantorovich_parameter,
        radius,
        newton.contraction_factor.unwrap(),
        newton.status
    );
    println!(
        "krawczyk_image=[{:.17e},{:.17e}] status={:?}",
        krawczyk.krawczyk_image.lower(),
        krawczyk.krawczyk_image.upper(),
        krawczyk.status
    );
    println!("exact_root={exact_root:.17e}");
    Ok(())
}
