use metric_induced_error_envelope_reference::{
    outward_interval::IntervalV1,
    scalar_krawczyk::{ScalarKrawczykStatusV1, scalar_krawczyk_inclusion},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // F(x) = eta + x + gamma*x^2/2 with exact dyadic eta/gamma.
    let eta = 0.125;
    let gamma = 2.0;
    let center = -0.125;
    let search_box = IntervalV1::new(-0.2, -0.05)?;

    // Evaluate F(center) and enclose F'(X) using only outward interval ops.
    let eta_interval = IntervalV1::point(eta)?;
    let x = IntervalV1::point(center)?;
    let quadratic_scale = IntervalV1::point(0.5 * gamma)?;
    let function_value = eta_interval
        .add(x)?
        .add(quadratic_scale.mul(x.square()?)?)?;
    let derivative = IntervalV1::point(1.0)?
        .add(IntervalV1::point(gamma)?.mul(search_box)?)?;

    let receipt = scalar_krawczyk_inclusion(
        center,
        1.25,
        function_value,
        derivative,
        search_box,
    )?;
    let exact_root = (-1.0 + (1.0 - 2.0 * eta * gamma).sqrt()) / gamma;

    println!(
        "function_value=[{:.17e},{:.17e}]",
        function_value.lower(),
        function_value.upper()
    );
    println!(
        "derivative_box=[{:.17e},{:.17e}]",
        derivative.lower(),
        derivative.upper()
    );
    println!(
        "krawczyk_image=[{:.17e},{:.17e}]",
        receipt.krawczyk_image.lower(),
        receipt.krawczyk_image.upper()
    );
    println!(
        "search_box=[{:.17e},{:.17e}]",
        search_box.lower(),
        search_box.upper()
    );
    println!("exact_reference_root={exact_root:.17e}");
    println!("status={:?}", receipt.status);

    assert_eq!(
        receipt.status,
        ScalarKrawczykStatusV1::ConditionalUniqueRoot
    );
    assert!(search_box.contains(exact_root));
    assert!(search_box.contains_interval(receipt.krawczyk_image));
    Ok(())
}
