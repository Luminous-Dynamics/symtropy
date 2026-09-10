/// Versioned deterministic Haldane odd-crossover-parity threshold oracle.
///
/// The public result is expressed in the crate's existing parts-per-million
/// probability scale. The authoritative path uses integer/fixed-point arithmetic
/// only; floating point is deliberately absent from this module.
pub const HALDANE_PARITY_ORACLE_VERSION: u32 = 1;

/// At and beyond this genetic distance, the rounded Haldane odd-parity
/// probability is exactly 500_000 ppm at V1 output resolution.
pub const HALDANE_SATURATION_DISTANCE_MICROMORGANS: u64 = 7_000_000;

/// Limiting odd/even crossover-parity probability under Haldane/no interference.
pub const HALDANE_MAX_ODD_PARITY_PPM: u32 = 500_000;

const MICROMORGANS_PER_MORGAN: u128 = 1_000_000;
const Q48_ONE: u128 = 1_u128 << 48;
const Q48_REDUCTION_LIMIT: u128 = Q48_ONE / 8;
const EXP_NEG_TAYLOR_TERMS: u128 = 12;

/// Return the Haldane odd-crossover-parity probability, rounded to integer ppm,
/// for a genetic-map distance expressed in micromorgans.
///
/// Under `PoissonCrossoversNoInterferenceV1`, if `K ~ Poisson(d)` for genetic
/// distance `d` Morgans, then `P(K odd) = (1 - exp(-2d)) / 2`.
///
/// V1 evaluates that law with Q48 fixed-point integer arithmetic and explicit
/// round-half-up operations. Distances at or above 7 Morgans saturate to the
/// correctly rounded 500_000 ppm limit before any multiplication, so every
/// `u64` input is safe.
pub fn haldane_odd_parity_probability_ppm(distance_micromorgans: u64) -> u32 {
    if distance_micromorgans == 0 {
        return 0;
    }
    if distance_micromorgans >= HALDANE_SATURATION_DISTANCE_MICROMORGANS {
        return HALDANE_MAX_ODD_PARITY_PPM;
    }

    // x = 2d in Q48, where d is measured in Morgans.
    let x_q48 = round_half_up_u128(
        2_u128 * u128::from(distance_micromorgans) * Q48_ONE,
        MICROMORGANS_PER_MORGAN,
    );

    // Range-reduce x until y <= 1/8. Each halving is itself part of the V1
    // deterministic grammar and therefore uses explicit round-half-up.
    let mut y_q48 = x_q48;
    let mut squarings = 0_u32;
    while y_q48 > Q48_REDUCTION_LIMIT {
        y_q48 = (y_q48 + 1) / 2;
        squarings += 1;
    }

    // exp(-y) = sum_n (-y)^n / n!, evaluated through a fixed recurrence.
    // y <= 1/8, so term magnitudes monotonically decrease. Saturating subtraction
    // makes this private numerical path total even if that invariant is ever
    // accidentally violated by a future refactor; it does not change valid V1
    // results under the frozen reduction contract.
    let mut exp_neg_q48 = Q48_ONE;
    let mut term_q48 = Q48_ONE;
    for n in 1..=EXP_NEG_TAYLOR_TERMS {
        term_q48 = round_half_up_u128(term_q48 * y_q48, Q48_ONE * n);
        if n % 2 == 1 {
            exp_neg_q48 = exp_neg_q48.saturating_sub(term_q48);
        } else {
            exp_neg_q48 += term_q48;
        }
    }

    // Undo range reduction by repeated squaring, again with fixed rounding.
    for _ in 0..squarings {
        exp_neg_q48 = round_half_up_u128(exp_neg_q48 * exp_neg_q48, Q48_ONE);
    }

    let one_minus_exp_q48 = Q48_ONE.saturating_sub(exp_neg_q48.min(Q48_ONE));
    let ppm = round_half_up_u128(
        u128::from(HALDANE_MAX_ODD_PARITY_PPM) * one_minus_exp_q48,
        Q48_ONE,
    );
    u32::try_from(ppm)
        .unwrap_or(HALDANE_MAX_ODD_PARITY_PPM)
        .min(HALDANE_MAX_ODD_PARITY_PPM)
}

fn round_half_up_u128(numerator: u128, denominator: u128) -> u128 {
    // All V1 callers use positive compile-time denominators. Keeping this helper
    // private avoids turning denominator validity into a public authority surface.
    (numerator + denominator / 2) / denominator
}
