// SPDX-License-Identifier: AGPL-3.0-or-later
//! Metamorphic checks for dimensional scaling and caller-side unit normalization.
//!
//! These tests do not add a unit parser or claim general solver validity. The
//! public mechanics functions accept SI values; unit-equivalence cases check
//! that common source-unit conversions preserve the same canonical SI result.

use physical_transformation_reference::mechanics::{
    axial_bar, cantilever_tip_load, simply_supported_uniform_load,
};

fn close(actual: f64, expected: f64, label: &str) {
    let allowed = (expected.abs() * 1.0e-10).max(1.0e-14);
    assert!(
        actual.is_finite() && (actual - expected).abs() <= allowed,
        "{label}: actual={actual:.17e}, expected={expected:.17e}, allowed={allowed:.17e}"
    );
}

#[test]
fn axial_bar_obeys_force_homogeneity_and_geometric_scaling() {
    let base = axial_bar(4_000.0, 3.0, 0.002, 70.0e9).expect("valid SI bar");
    let doubled_force = axial_bar(8_000.0, 3.0, 0.002, 70.0e9).expect("valid doubled-force bar");
    close(
        doubled_force.axial_stress_pa,
        2.0 * base.axial_stress_pa,
        "bar stress vs force",
    );
    close(
        doubled_force.axial_strain,
        2.0 * base.axial_strain,
        "bar strain vs force",
    );
    close(
        doubled_force.extension_m,
        2.0 * base.extension_m,
        "bar extension vs force",
    );

    // Geometrically similar bar: L -> 2L and A -> 4A at fixed force/material.
    let scaled_geometry =
        axial_bar(4_000.0, 6.0, 0.008, 70.0e9).expect("valid scaled-geometry bar");
    close(
        scaled_geometry.axial_stress_pa,
        base.axial_stress_pa / 4.0,
        "bar stress under geometric scaling",
    );
    close(
        scaled_geometry.axial_strain,
        base.axial_strain / 4.0,
        "bar strain under geometric scaling",
    );
    close(
        scaled_geometry.extension_m,
        base.extension_m / 2.0,
        "bar extension under geometric scaling",
    );
}

#[test]
fn cantilever_obeys_force_span_and_stiffness_scaling() {
    let base = cantilever_tip_load(100.0, 2.0, 200.0e9, 8.0e-6, 0.1).expect("valid SI cantilever");
    let doubled_force =
        cantilever_tip_load(200.0, 2.0, 200.0e9, 8.0e-6, 0.1).expect("doubled load");
    close(
        doubled_force.tip_deflection_m,
        2.0 * base.tip_deflection_m,
        "cantilever deflection vs force",
    );
    close(
        doubled_force.maximum_bending_stress_pa,
        2.0 * base.maximum_bending_stress_pa,
        "cantilever stress vs force",
    );

    let doubled_span = cantilever_tip_load(100.0, 4.0, 200.0e9, 8.0e-6, 0.1).expect("doubled span");
    close(
        doubled_span.tip_deflection_m,
        8.0 * base.tip_deflection_m,
        "cantilever deflection vs span cubed",
    );
    close(
        doubled_span.maximum_bending_stress_pa,
        2.0 * base.maximum_bending_stress_pa,
        "cantilever stress vs span",
    );

    let doubled_stiffness = cantilever_tip_load(100.0, 2.0, 400.0e9, 16.0e-6, 0.1)
        .expect("doubled modulus and second moment");
    close(
        doubled_stiffness.tip_deflection_m,
        base.tip_deflection_m / 4.0,
        "cantilever deflection vs E times I",
    );
    close(
        doubled_stiffness.maximum_bending_stress_pa,
        base.maximum_bending_stress_pa / 2.0,
        "cantilever stress vs second moment",
    );
}

#[test]
fn simply_supported_beam_obeys_load_span_and_stiffness_scaling() {
    let base = simply_supported_uniform_load(1_000.0, 4.0, 200.0e9, 8.0e-6, 0.1)
        .expect("valid SI simply supported beam");
    let doubled_load = simply_supported_uniform_load(2_000.0, 4.0, 200.0e9, 8.0e-6, 0.1)
        .expect("doubled uniform load");
    close(
        doubled_load.midspan_deflection_m,
        2.0 * base.midspan_deflection_m,
        "beam deflection vs uniform load",
    );
    close(
        doubled_load.maximum_bending_stress_pa,
        2.0 * base.maximum_bending_stress_pa,
        "beam stress vs uniform load",
    );

    let doubled_span =
        simply_supported_uniform_load(1_000.0, 8.0, 200.0e9, 8.0e-6, 0.1).expect("doubled span");
    close(
        doubled_span.midspan_deflection_m,
        16.0 * base.midspan_deflection_m,
        "beam deflection vs span to fourth power",
    );
    close(
        doubled_span.maximum_bending_stress_pa,
        4.0 * base.maximum_bending_stress_pa,
        "beam stress vs span squared",
    );

    let doubled_modulus = simply_supported_uniform_load(1_000.0, 4.0, 400.0e9, 8.0e-6, 0.1)
        .expect("doubled Young's modulus");
    close(
        doubled_modulus.midspan_deflection_m,
        base.midspan_deflection_m / 2.0,
        "beam deflection vs Young's modulus",
    );
    close(
        doubled_modulus.maximum_bending_stress_pa,
        base.maximum_bending_stress_pa,
        "beam stress is independent of Young's modulus in this model",
    );
}

#[test]
fn common_source_unit_encodings_normalize_to_identical_si_responses() {
    let bar_si = axial_bar(4_000.0, 3.0, 0.002, 70.0e9).expect("SI bar");
    // 4 kN, 3000 mm, 2000 mm^2, 70 GPa converted to the API's SI contract.
    let bar_converted = axial_bar(
        4.0 * 1_000.0,
        3_000.0 / 1_000.0,
        2_000.0 * 1.0e-6,
        70.0 * 1.0e9,
    )
    .expect("converted bar");
    close(
        bar_converted.axial_stress_pa,
        bar_si.axial_stress_pa,
        "converted bar stress",
    );
    close(
        bar_converted.axial_strain,
        bar_si.axial_strain,
        "converted bar strain",
    );
    close(
        bar_converted.extension_m,
        bar_si.extension_m,
        "converted bar extension",
    );

    let cantilever_si =
        cantilever_tip_load(100.0, 1.5, 70.0e9, 8.0e-6, 0.1).expect("SI cantilever");
    // 0.1 kN, 1500 mm, 70 GPa, 8e6 mm^4, and 100 mm converted to SI.
    let cantilever_converted = cantilever_tip_load(
        0.1 * 1_000.0,
        1_500.0 / 1_000.0,
        70.0 * 1.0e9,
        8_000_000.0 * 1.0e-12,
        100.0 / 1_000.0,
    )
    .expect("converted cantilever");
    close(
        cantilever_converted.tip_deflection_m,
        cantilever_si.tip_deflection_m,
        "converted cantilever deflection",
    );
    close(
        cantilever_converted.maximum_bending_stress_pa,
        cantilever_si.maximum_bending_stress_pa,
        "converted cantilever stress",
    );

    let beam_si = simply_supported_uniform_load(800.0, 4.5, 70.0e9, 6.0e-5, 0.15).expect("SI beam");
    // 0.8 kN/m, 4500 mm, 70 GPa, 6e7 mm^4, and 150 mm converted to SI.
    let beam_converted = simply_supported_uniform_load(
        0.8 * 1_000.0,
        4_500.0 / 1_000.0,
        70.0 * 1.0e9,
        60_000_000.0 * 1.0e-12,
        150.0 / 1_000.0,
    )
    .expect("converted beam");
    close(
        beam_converted.midspan_deflection_m,
        beam_si.midspan_deflection_m,
        "converted beam deflection",
    );
    close(
        beam_converted.maximum_bending_stress_pa,
        beam_si.maximum_bending_stress_pa,
        "converted beam stress",
    );
}

fn next_unit_interval(seed: &mut u64) -> f64 {
    // Fixed 64-bit LCG: deterministic test sequence, not a cryptographic RNG.
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*seed >> 11) as f64 / 9_007_199_254_740_992.0
}

fn sample_between(seed: &mut u64, minimum: f64, maximum: f64) -> f64 {
    minimum + (maximum - minimum) * next_unit_interval(seed)
}

#[test]
fn fixed_seed_parameter_sweep_preserves_declared_mechanics_relations() {
    // Freeze both seed and ranges so failures reproduce exactly across machines.
    let mut seed = 0x5359_4D54_524F_5059_u64;
    const CASES: usize = 128;

    for case_index in 0..CASES {
        let force = sample_between(&mut seed, 10.0, 10_000.0);
        let length = sample_between(&mut seed, 0.1, 10.0);
        let area = sample_between(&mut seed, 1.0e-4, 5.0e-2);
        let modulus = sample_between(&mut seed, 50.0e9, 210.0e9);
        let geometry_scale = sample_between(&mut seed, 1.2, 1.8);
        let force_scale = sample_between(&mut seed, 0.5, 3.0);

        let bar = axial_bar(force, length, area, modulus)
            .unwrap_or_else(|error| panic!("bar case {case_index} rejected: {error}"));
        let bar_scaled_force = axial_bar(force * force_scale, length, area, modulus)
            .unwrap_or_else(|error| panic!("scaled bar case {case_index} rejected: {error}"));
        close(
            bar_scaled_force.axial_stress_pa,
            force_scale * bar.axial_stress_pa,
            "swept bar stress homogeneity",
        );
        close(
            bar_scaled_force.axial_strain,
            force_scale * bar.axial_strain,
            "swept bar strain homogeneity",
        );
        close(
            bar_scaled_force.extension_m,
            force_scale * bar.extension_m,
            "swept bar extension homogeneity",
        );

        let bar_scaled_geometry = axial_bar(
            force,
            length * geometry_scale,
            area * geometry_scale * geometry_scale,
            modulus,
        )
        .unwrap_or_else(|error| panic!("scaled geometry case {case_index} rejected: {error}"));
        close(
            bar_scaled_geometry.axial_stress_pa,
            bar.axial_stress_pa / geometry_scale.powi(2),
            "swept bar stress geometric similarity",
        );
        close(
            bar_scaled_geometry.extension_m,
            bar.extension_m / geometry_scale,
            "swept bar extension geometric similarity",
        );

        let beam_force = sample_between(&mut seed, 1.0, 500.0);
        let beam_length = sample_between(&mut seed, 0.2, 5.0);
        let beam_modulus = sample_between(&mut seed, 20.0e9, 210.0e9);
        let second_moment = sample_between(&mut seed, 1.0e-6, 1.0e-3);
        let outer_fiber = sample_between(&mut seed, 0.01, 0.5);
        let beam_scale = sample_between(&mut seed, 1.2, 1.8);
        let cantilever = cantilever_tip_load(
            beam_force,
            beam_length,
            beam_modulus,
            second_moment,
            outer_fiber,
        )
        .unwrap_or_else(|error| panic!("cantilever case {case_index} rejected: {error}"));
        let cantilever_scaled = cantilever_tip_load(
            beam_force,
            beam_length * beam_scale,
            beam_modulus,
            second_moment,
            outer_fiber,
        )
        .unwrap_or_else(|error| panic!("scaled cantilever case {case_index} rejected: {error}"));
        close(
            cantilever_scaled.tip_deflection_m,
            cantilever.tip_deflection_m * beam_scale.powi(3),
            "swept cantilever cubic span scaling",
        );
        close(
            cantilever_scaled.maximum_bending_stress_pa,
            cantilever.maximum_bending_stress_pa * beam_scale,
            "swept cantilever linear span scaling",
        );

        let distributed_load = sample_between(&mut seed, 1.0, 5_000.0);
        let supported = simply_supported_uniform_load(
            distributed_load,
            beam_length,
            beam_modulus,
            second_moment,
            outer_fiber,
        )
        .unwrap_or_else(|error| panic!("supported beam case {case_index} rejected: {error}"));
        let supported_scaled = simply_supported_uniform_load(
            distributed_load,
            beam_length * beam_scale,
            beam_modulus,
            second_moment,
            outer_fiber,
        )
        .unwrap_or_else(|error| {
            panic!("scaled supported beam case {case_index} rejected: {error}")
        });
        close(
            supported_scaled.midspan_deflection_m,
            supported.midspan_deflection_m * beam_scale.powi(4),
            "swept simply-supported fourth-power span scaling",
        );
        close(
            supported_scaled.maximum_bending_stress_pa,
            supported.maximum_bending_stress_pa * beam_scale.powi(2),
            "swept simply-supported square span scaling",
        );
    }
}
