// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent known-answer checks for named, typed SI conversions.

use physical_transformation_reference::mechanics::{
    axial_bar as raw_axial_bar, cantilever_tip_load as raw_cantilever_tip_load,
    simply_supported_uniform_load as raw_simply_supported_uniform_load,
};
use physical_transformation_reference::units::{
    self, AreaM2, DistributedLoadNPerM, ForceN, LengthM, SecondMomentM4, YoungsModulusPa,
};

fn close(actual: f64, expected: f64, label: &str) {
    let allowed = (expected.abs() * 1.0e-12).max(1.0e-15);
    assert!(
        actual.is_finite() && (actual - expected).abs() <= allowed,
        "{label}: actual={actual:.17e}, expected={expected:.17e}, allowed={allowed:.17e}"
    );
}

#[test]
fn named_source_unit_conversions_match_canonical_si_values() {
    close(
        LengthM::from_millimeters(1_500.0).expect("valid mm length").as_meters(),
        1.5,
        "mm to m",
    );
    close(
        ForceN::from_kilonewtons(0.1).expect("valid kN force").as_newtons(),
        100.0,
        "kN to N",
    );
    close(
        AreaM2::from_square_millimeters(2_000.0)
            .expect("valid mm^2 area")
            .as_square_meters(),
        0.002,
        "mm^2 to m^2",
    );
    close(
        YoungsModulusPa::from_gigapascals(70.0)
            .expect("valid GPa modulus")
            .as_pascals(),
        70.0e9,
        "GPa to Pa",
    );
    close(
        SecondMomentM4::from_millimeters4(8_000_000.0)
            .expect("valid mm^4 second moment")
            .as_meters4(),
        8.0e-6,
        "mm^4 to m^4",
    );
    close(
        DistributedLoadNPerM::from_kilonewtons_per_meter(0.8)
            .expect("valid kN/m load")
            .as_newtons_per_meter(),
        800.0,
        "kN/m to N/m",
    );
}

#[test]
fn typed_solver_wrappers_match_independent_si_call_sites() {
    let typed_bar = units::axial_bar(
        ForceN::from_kilonewtons(4.0).unwrap(),
        LengthM::from_millimeters(3_000.0).unwrap(),
        AreaM2::from_square_millimeters(2_000.0).unwrap(),
        YoungsModulusPa::from_gigapascals(70.0).unwrap(),
    )
    .expect("typed SI bar");
    let raw_bar = raw_axial_bar(4_000.0, 3.0, 0.002, 70.0e9).expect("canonical SI bar");
    close(typed_bar.axial_stress_pa, raw_bar.axial_stress_pa, "bar stress");
    close(typed_bar.axial_strain, raw_bar.axial_strain, "bar strain");
    close(typed_bar.extension_m, raw_bar.extension_m, "bar extension");

    let typed_cantilever = units::cantilever_tip_load(
        ForceN::from_kilonewtons(0.1).unwrap(),
        LengthM::from_millimeters(1_500.0).unwrap(),
        YoungsModulusPa::from_gigapascals(70.0).unwrap(),
        SecondMomentM4::from_millimeters4(8_000_000.0).unwrap(),
        LengthM::from_millimeters(100.0).unwrap(),
    )
    .expect("typed SI cantilever");
    let raw_cantilever =
        raw_cantilever_tip_load(100.0, 1.5, 70.0e9, 8.0e-6, 0.1).expect("canonical SI cantilever");
    close(
        typed_cantilever.tip_deflection_m,
        raw_cantilever.tip_deflection_m,
        "cantilever deflection",
    );
    close(
        typed_cantilever.maximum_bending_stress_pa,
        raw_cantilever.maximum_bending_stress_pa,
        "cantilever stress",
    );

    let typed_beam = units::simply_supported_uniform_load(
        DistributedLoadNPerM::from_kilonewtons_per_meter(0.8).unwrap(),
        LengthM::from_millimeters(4_500.0).unwrap(),
        YoungsModulusPa::from_gigapascals(70.0).unwrap(),
        SecondMomentM4::from_millimeters4(60_000_000.0).unwrap(),
        LengthM::from_millimeters(150.0).unwrap(),
    )
    .expect("typed SI simply-supported beam");
    let raw_beam =
        raw_simply_supported_uniform_load(800.0, 4.5, 70.0e9, 6.0e-5, 0.15)
            .expect("canonical SI simply-supported beam");
    close(
        typed_beam.midspan_deflection_m,
        raw_beam.midspan_deflection_m,
        "beam deflection",
    );
    close(
        typed_beam.maximum_bending_stress_pa,
        raw_beam.maximum_bending_stress_pa,
        "beam stress",
    );
}

#[test]
fn boundary_rejects_invalid_positive_quantities_and_conversion_overflow() {
    assert!(LengthM::from_meters(0.0).is_err());
    assert!(LengthM::from_millimeters(-1.0).is_err());
    assert!(LengthM::from_meters(f64::NAN).is_err());
    assert!(ForceN::from_kilonewtons(f64::MAX).is_err());
    assert!(AreaM2::from_square_millimeters(f64::NAN).is_err());
    assert!(AreaM2::from_square_millimeters(0.0).is_err());
    assert!(YoungsModulusPa::from_gigapascals(f64::MAX).is_err());
    assert!(SecondMomentM4::from_millimeters4(f64::INFINITY).is_err());
    assert!(SecondMomentM4::from_millimeters4(0.0).is_err());
    assert!(DistributedLoadNPerM::from_kilonewtons_per_meter(f64::MAX).is_err());
}

#[test]
fn signed_force_and_distributed_load_preserve_direction() {
    close(
        ForceN::from_kilonewtons(-4.0)
            .expect("signed compression force")
            .as_newtons(),
        -4_000.0,
        "signed kN force",
    );
    close(
        DistributedLoadNPerM::from_kilonewtons_per_meter(-0.8)
            .expect("signed distributed load")
            .as_newtons_per_meter(),
        -800.0,
        "signed kN/m load",
    );
}
