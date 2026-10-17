//! Emit the canonical, synthetic solar-thermal mechanics receipt.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", solar_thermal_kernel::canonical_demo_receipt()?);
    Ok(())
}
