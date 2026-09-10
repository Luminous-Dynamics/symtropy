use symtropy_evolution_core::{
    haldane_odd_parity_probability_ppm, HALDANE_MAX_ODD_PARITY_PPM,
    HALDANE_SATURATION_DISTANCE_MICROMORGANS,
};

#[test]
fn golden_vectors_match_independent_high_precision_reference() {
    // Expected values were generated independently at high precision from
    // round(500_000 * (1 - exp(-2 * delta_um / 1_000_000))).
    let vectors = [
        (0_u64, 0_u32),
        (1, 1),
        (10, 10),
        (100, 100),
        (1_000, 999),
        (10_000, 9_901),
        (50_000, 47_581),
        (100_000, 90_635),
        (177_160, 149_175),
        // Adversarial near-half-ppm boundary: true ~= 149175.500000699 ppm.
        (177_161, 149_176),
        (250_000, 196_735),
        (500_000, 316_060),
        (1_000_000, 432_332),
        (2_000_000, 490_842),
        (3_000_000, 498_761),
        (5_000_000, 499_977),
        // Adversarial near-half-ppm boundary: true ~= 499995.500000087 ppm.
        (5_809_143, 499_996),
        (6_000_000, 499_997),
        (6_900_000, 499_999),
        (6_907_755, 499_999),
        (6_907_756, 500_000),
        (6_999_999, 500_000),
        (7_000_000, 500_000),
        (u64::MAX, 500_000),
    ];

    for (distance, expected) in vectors {
        assert_eq!(
            haldane_odd_parity_probability_ppm(distance),
            expected,
            "distance={distance} micromorgans"
        );
    }
}

#[test]
fn probability_is_monotone_bounded_and_saturates() {
    let mut previous = 0_u32;
    for distance in (0..=HALDANE_SATURATION_DISTANCE_MICROMORGANS).step_by(1_000) {
        let observed = haldane_odd_parity_probability_ppm(distance);
        assert!(observed >= previous, "non-monotone at {distance}");
        assert!(observed <= HALDANE_MAX_ODD_PARITY_PPM);
        previous = observed;
    }

    assert_eq!(haldane_odd_parity_probability_ppm(0), 0);
    for distance in [
        HALDANE_SATURATION_DISTANCE_MICROMORGANS,
        HALDANE_SATURATION_DISTANCE_MICROMORGANS + 1,
        10_000_000,
        u64::MAX,
    ] {
        assert_eq!(
            haldane_odd_parity_probability_ppm(distance),
            HALDANE_MAX_ODD_PARITY_PPM
        );
    }
}

#[test]
fn sampled_differential_matches_standard_formula_rounding() {
    // f64 is deliberately test-only here. The authoritative implementation is
    // integer/Q48 and has no platform libm dependency.
    for distance in (0..HALDANE_SATURATION_DISTANCE_MICROMORGANS).step_by(997) {
        let d_morgans = distance as f64 / 1_000_000.0;
        let reference = (500_000.0 * (1.0 - (-2.0 * d_morgans).exp())).round() as u32;
        assert_eq!(
            haldane_odd_parity_probability_ppm(distance),
            reference,
            "sampled differential mismatch at {distance} micromorgans"
        );
    }
}

#[test]
fn adversarial_rounding_neighborhoods_are_stable() {
    let vectors = [
        (177_159, 149_174),
        (177_160, 149_175),
        (177_161, 149_176),
        (177_162, 149_176),
        (5_809_141, 499_995),
        (5_809_142, 499_995),
        (5_809_143, 499_996),
        (5_809_144, 499_996),
        (5_809_145, 499_996),
    ];
    for (distance, expected) in vectors {
        assert_eq!(haldane_odd_parity_probability_ppm(distance), expected);
    }
}
