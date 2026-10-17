// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Emit the canonical, synthetic solar-thermal mechanics receipt.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", solar_thermal_kernel::canonical_demo_receipt()?);
    Ok(())
}
