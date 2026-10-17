// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Safety tier system: Φ-gated motor authority.
//!
//! Maps integrated information (Φ) to motor gain. Higher Φ (more integrated
//! information processing) grants more force authority. This is a formal
//! coupling, not a philosophical claim — it could equally be called
//! "integration-gated motor authority."
//!
//! # Two mappings live here, and the NRC one is NOT what platforms run
//!
//! Read this before citing the 4-tier table below as "the safety system."
//!
//! - [`SafetyTier::from_phi`] + [`SafetyTier::motor_gain`] implement an
//!   NRC-inspired 4-tier ladder with **hardcoded** cut points at Φ =
//!   0.6 / 0.3 / 0.1. **No platform demo in this workspace uses it**
//!   (`grep -rln '\.motor_gain()' crates/apps/` → no matches).
//! - [`sprint_floor_gain`] is a two-level sprint/floor gate taking
//!   platform-specific thresholds. **All six instrumented platform demos use
//!   this instead** — AUV, humanoid, flight, vehicle, helicopter, and the CLI
//!   calibration path.
//!
//! The reason is empirical, not stylistic. Measured Φ on these platforms sits
//! in a narrow band around 0.1, so every one of them calibrates its sprint
//! threshold there:
//!
//! | Demo       | `SPRINT_THRESHOLD` | `FLOOR_GAIN` |
//! |------------|--------------------|--------------|
//! | auv        | 0.130              | 0.3          |
//! | humanoid   | 0.130              | 0.3          |
//! | flight     | 0.110              | 0.3          |
//! | vehicle    | 0.101              | 0.3          |
//! | helicopter | 0.100              | 0.3          |
//!
//! Against `from_phi`'s cut points that band is entirely inside Red/Orange —
//! its Green threshold is 5-6x above anything these platforms actually reach.
//! Feeding real platform Φ into `from_phi` therefore pins motor gain at 0.0 or
//! 0.3 permanently, which is why the demos route around it. Use
//! `symtropy-cli`'s calibration output (it prints a suggested
//! `SPRINT_THRESHOLD`) or the `MANIP_BENCH_PHI_TRACE=1` pattern in the
//! manipulator benchmark to measure your own platform's band before choosing.
//!
//! `from_phi` is nonetheless still live inside this crate's own coupling field
//! (`simple_field.rs`, `coupling.rs`), which has **not** been measured against
//! the same yardstick. Whether the band there resembles the robotics platforms'
//! is an open question — see `SYMTROPY_EXECUTION_PLAN_2026-07-28.md` §C2. Do
//! not assume either way.

/// Φ-gated safety tier for motor/force output.
///
/// Based on NRC (Nuclear Regulatory Commission) 4-tier safety model,
/// mapped to integrated information level (Φ):
///
/// | Tier   | Φ Range   | Motor Gain | Behavior                |
/// |--------|-----------|------------|-------------------------|
/// | Green  | > 0.6     | 100%       | Full authority           |
/// | Yellow | 0.3–0.6   | 60%        | Reduced speed/force      |
/// | Orange | 0.1–0.3   | 30%        | Retreat to safe pose     |
/// | Red    | ≤ 0.1     | 0%         | Emergency stop           |
///
/// **These cut points do not match any measured platform's Φ band** — see the
/// module-level docs. Every platform demo in this workspace uses
/// [`sprint_floor_gain`] with its own calibrated threshold instead. Reach for
/// this type only after measuring your signal's actual distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SafetyTier {
    Green,
    Yellow,
    Orange,
    Red,
}

/// Why a Φ value cannot be classified against the safety-tier domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidPhi {
    /// The value is NaN or an infinity.
    NonFinite,
    /// The finite value is outside the declared inclusive domain [0, 1].
    OutOfRange,
}

impl SafetyTier {
    /// Determine safety tier from Φ value, using the hardcoded 0.6 / 0.3 / 0.1
    /// cut points.
    ///
    /// This compatibility wrapper fails closed: invalid values map to Red.
    /// Call [`try_from_phi`] when invalid telemetry should be distinguishable
    /// from a valid low-Φ state. These cut points are not calibrated for any
    /// platform demo; see the module-level docs and prefer
    /// [`sprint_floor_gain`] with a measured threshold.
    pub fn from_phi(phi: f64) -> Self {
        Self::try_from_phi(phi).unwrap_or(Self::Red)
    }

    /// Classify a finite Φ in the declared inclusive domain [0, 1].
    ///
    /// Non-finite values and finite values outside the domain are rejected
    /// rather than allowing values such as +∞ to grant Green-tier authority.
    /// Strict threshold behavior is preserved: equality at 0.1 is Red, at
    /// 0.3 is Orange, and at 0.6 is Yellow.
    pub fn try_from_phi(phi: f64) -> Result<Self, InvalidPhi> {
        if !phi.is_finite() {
            return Err(InvalidPhi::NonFinite);
        }
        if !(0.0..=1.0).contains(&phi) {
            return Err(InvalidPhi::OutOfRange);
        }

        Ok(if phi > 0.6 {
            Self::Green
        } else if phi > 0.3 {
            Self::Yellow
        } else if phi > 0.1 {
            Self::Orange
        } else {
            Self::Red
        })
    }

    /// Motor gain multiplier [0.0, 1.0].
    #[inline]
    pub fn motor_gain(&self) -> f64 {
        match self {
            Self::Green => 1.0,
            Self::Yellow => 0.6,
            Self::Orange => 0.3,
            Self::Red => 0.0,
        }
    }

    /// Whether this tier allows any motor output.
    #[inline]
    pub fn allows_output(&self) -> bool {
        *self != Self::Red
    }
}

/// Minimal 2-part signal → gain mapping, empirically derived.
///
/// # What `signal` actually is
///
/// The function takes an arbitrary scalar `signal ∈ [0, 1]` and gates
/// motor authority on whether it exceeds `sprint_threshold`. In the
/// Φ-gated-safety paper the signal is the output of
/// `MasterConsciousnessEquation::compute()` — a consciousness-inspired
/// integration index that aggregates 10 sub-signals (IIT Φ, broadcast,
/// working memory, attention, recurrence, embodiment, knowledge, HOT,
/// narrative, social) via softmin bottleneck plus rescaling. The
/// function is **signal-agnostic**: any scalar correlate that
/// separates "confident cognitive state" from "uncertain state" works.
/// We documented it with IIT-adjacent Φ because that's what the
/// consciousness equation produces, but the method stands or falls on
/// the empirical gating behavior, not on IIT axioms.
///
/// # Sufficient condition (empirical)
///
/// For a given platform the consciousness equation tends to produce
/// signal values concentrated in a narrow band rather than spanning
/// [0, 1]. The 5-variant Monte Carlo study in
/// `symtropy-manipulator-demo/examples/manipulator_benchmark.rs`
/// (commits `38dc8b1fd9` through `317baad595`, Apr 2026) found that a
/// signal → gain mapping beats binary ISO/TS 15066 SSM only when it
/// satisfies **both**:
///
///   1. A **sprint threshold**: at or below the top of the platform's
///      empirical signal band, above which `gain = 1.0` so the
///      controller commits to full authority in brief confident windows.
///   2. A **crawl-rate floor**: a non-zero gain below the sprint
///      threshold, large enough to keep IK convergence alive. The
///      surgical/manipulator demos use 0.3.
///
/// Intermediate tiers (0.6, 0.3 steps as in `SafetyTier::motor_gain`)
/// proved empirically superfluous — a two-level `sprint_floor_gain`
/// matched the 4-tier `Recalibrated` variant to three decimal places.
///
/// **`SafetyTier::from_phi` uses hardcoded thresholds (0.6 / 0.3 /
/// 0.1) that don't match any real platform's empirical signal band.**
/// For a working demo, measure your platform's signal distribution
/// first (e.g. via the `MANIP_BENCH_PHI_TRACE=1` pattern in the
/// manipulator benchmark), then pass a platform-specific
/// `sprint_threshold` and `floor` to this function.
///
/// # Example
///
/// ```
/// use symtropy_consciousness_physics::safety::sprint_floor_gain;
///
/// // Platform's empirical signal band is [0.099, 0.145].
/// // Pick sprint threshold near the top, floor above the crawl limit.
/// let gain = sprint_floor_gain(0.14, 0.135, 0.3);
/// assert!((gain - 1.0).abs() < 1e-9);  // signal above sprint → full authority.
///
/// let gain = sprint_floor_gain(0.11, 0.135, 0.3);
/// assert!((gain - 0.3).abs() < 1e-9);  // signal below sprint → floor.
/// ```
///
/// Input parameter being rejected by the checked sprint/floor mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SprintFloorParameter {
    Signal,
    SprintThreshold,
    Floor,
}

/// Invalid input for the checked sprint/floor mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidSprintFloorInput {
    NonFinite(SprintFloorParameter),
    OutOfRange(SprintFloorParameter),
}

/// Compute the two-level gain after validating all inputs are finite and in [0, 1].
///
/// Equality at the threshold remains on the floor: sprint activates only when
/// `signal > sprint_threshold`. This function validates numeric domains only;
/// choosing a suitable threshold/floor still requires platform-specific
/// calibration and safety analysis.
pub fn try_sprint_floor_gain(
    signal: f64,
    sprint_threshold: f64,
    floor: f64,
) -> Result<f64, InvalidSprintFloorInput> {
    for (value, parameter) in [
        (signal, SprintFloorParameter::Signal),
        (sprint_threshold, SprintFloorParameter::SprintThreshold),
        (floor, SprintFloorParameter::Floor),
    ] {
        if !value.is_finite() {
            return Err(InvalidSprintFloorInput::NonFinite(parameter));
        }
        if !(0.0..=1.0).contains(&value) {
            return Err(InvalidSprintFloorInput::OutOfRange(parameter));
        }
    }

    Ok(if signal > sprint_threshold {
        1.0
    } else {
        floor
    })
}

/// Compatibility wrapper for the checked two-level signal → gain mapping.
///
/// Invalid inputs fail closed to zero gain. Call [`try_sprint_floor_gain`]
/// when callers need to distinguish invalid configuration/telemetry from a
/// valid zero-gain result. Numeric validation does not replace platform
/// calibration or a safety case.
#[inline]
pub fn sprint_floor_gain(signal: f64, sprint_threshold: f64, floor: f64) -> f64 {
    try_sprint_floor_gain(signal, sprint_threshold, floor).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_thresholds() {
        assert_eq!(SafetyTier::from_phi(0.8), SafetyTier::Green);
        assert_eq!(SafetyTier::from_phi(0.5), SafetyTier::Yellow);
        assert_eq!(SafetyTier::from_phi(0.2), SafetyTier::Orange);
        assert_eq!(SafetyTier::from_phi(0.05), SafetyTier::Red);
        assert_eq!(SafetyTier::from_phi(0.0), SafetyTier::Red);
        assert_eq!(SafetyTier::from_phi(1.0), SafetyTier::Green);
    }

    #[test]
    fn tier_thresholds_are_strict_and_stable_at_adjacent_floats() {
        let below_01 = f64::from_bits(0.1_f64.to_bits() - 1);
        let above_01 = f64::from_bits(0.1_f64.to_bits() + 1);
        let below_03 = f64::from_bits(0.3_f64.to_bits() - 1);
        let above_03 = f64::from_bits(0.3_f64.to_bits() + 1);
        let below_06 = f64::from_bits(0.6_f64.to_bits() - 1);
        let above_06 = f64::from_bits(0.6_f64.to_bits() + 1);

        assert_eq!(SafetyTier::from_phi(below_01), SafetyTier::Red);
        assert_eq!(SafetyTier::from_phi(0.1), SafetyTier::Red);
        assert_eq!(SafetyTier::from_phi(above_01), SafetyTier::Orange);

        assert_eq!(SafetyTier::from_phi(below_03), SafetyTier::Orange);
        assert_eq!(SafetyTier::from_phi(0.3), SafetyTier::Orange);
        assert_eq!(SafetyTier::from_phi(above_03), SafetyTier::Yellow);

        assert_eq!(SafetyTier::from_phi(below_06), SafetyTier::Yellow);
        assert_eq!(SafetyTier::from_phi(0.6), SafetyTier::Yellow);
        assert_eq!(SafetyTier::from_phi(above_06), SafetyTier::Green);
    }

    #[test]
    fn checked_phi_rejects_invalid_domain_and_nonfinite_values() {
        assert_eq!(
            SafetyTier::try_from_phi(f64::NAN),
            Err(InvalidPhi::NonFinite)
        );
        assert_eq!(
            SafetyTier::try_from_phi(f64::INFINITY),
            Err(InvalidPhi::NonFinite)
        );
        assert_eq!(
            SafetyTier::try_from_phi(f64::NEG_INFINITY),
            Err(InvalidPhi::NonFinite)
        );
        assert_eq!(
            SafetyTier::try_from_phi(-f64::EPSILON),
            Err(InvalidPhi::OutOfRange)
        );
        assert_eq!(
            SafetyTier::try_from_phi(1.0 + f64::EPSILON),
            Err(InvalidPhi::OutOfRange)
        );

        // Legacy callers remain fail-closed without a signature change.
        assert_eq!(SafetyTier::from_phi(f64::NAN), SafetyTier::Red);
        assert_eq!(SafetyTier::from_phi(f64::INFINITY), SafetyTier::Red);
        assert_eq!(SafetyTier::from_phi(-f64::EPSILON), SafetyTier::Red);
    }

    #[test]
    fn motor_gain_values() {
        assert_eq!(SafetyTier::Green.motor_gain(), 1.0);
        assert_eq!(SafetyTier::Yellow.motor_gain(), 0.6);
        assert_eq!(SafetyTier::Orange.motor_gain(), 0.3);
        assert_eq!(SafetyTier::Red.motor_gain(), 0.0);
    }

    #[test]
    fn red_blocks_output() {
        assert!(SafetyTier::Green.allows_output());
        assert!(SafetyTier::Yellow.allows_output());
        assert!(SafetyTier::Orange.allows_output());
        assert!(!SafetyTier::Red.allows_output());
    }

    #[test]
    fn monotonic_phi_to_gain() {
        // Higher phi should never give lower gain
        let phis = [0.0, 0.05, 0.1, 0.15, 0.3, 0.35, 0.6, 0.65, 1.0];
        for window in phis.windows(2) {
            let gain_low = SafetyTier::from_phi(window[0]).motor_gain();
            let gain_high = SafetyTier::from_phi(window[1]).motor_gain();
            assert!(
                gain_high >= gain_low,
                "phi {} (gain {}) > phi {} (gain {})",
                window[1],
                gain_high,
                window[0],
                gain_low
            );
        }
    }

    // ── sprint_floor_gain (empirically-derived minimal mapping) ────────

    #[test]
    fn checked_sprint_floor_gain_accepts_domain_endpoints() {
        assert_eq!(try_sprint_floor_gain(0.0, 0.0, 0.0), Ok(0.0));
        assert_eq!(try_sprint_floor_gain(1.0, 0.0, 0.0), Ok(1.0));
        assert_eq!(try_sprint_floor_gain(0.5, 0.5, 1.0), Ok(1.0));
        assert_eq!(try_sprint_floor_gain(0.5, 0.5, 0.0), Ok(0.0));
    }

    #[test]
    fn checked_sprint_floor_gain_rejects_nonfinite_each_parameter() {
        use InvalidSprintFloorInput::NonFinite;
        use SprintFloorParameter::*;

        assert_eq!(
            try_sprint_floor_gain(f64::NAN, 0.5, 0.3),
            Err(NonFinite(Signal))
        );
        assert_eq!(
            try_sprint_floor_gain(0.5, f64::INFINITY, 0.3),
            Err(NonFinite(SprintThreshold))
        );
        assert_eq!(
            try_sprint_floor_gain(0.5, 0.5, f64::NEG_INFINITY),
            Err(NonFinite(Floor))
        );
        assert_eq!(sprint_floor_gain(f64::INFINITY, 0.5, 0.3), 0.0);
        assert_eq!(sprint_floor_gain(0.5, f64::NAN, 0.3), 0.0);
        assert_eq!(sprint_floor_gain(0.5, 0.5, f64::NAN), 0.0);
    }

    #[test]
    fn checked_sprint_floor_gain_rejects_out_of_range_each_parameter() {
        use InvalidSprintFloorInput::OutOfRange;
        use SprintFloorParameter::*;

        assert_eq!(
            try_sprint_floor_gain(-f64::EPSILON, 0.5, 0.3),
            Err(OutOfRange(Signal))
        );
        assert_eq!(
            try_sprint_floor_gain(0.5, 1.0 + f64::EPSILON, 0.3),
            Err(OutOfRange(SprintThreshold))
        );
        assert_eq!(
            try_sprint_floor_gain(0.5, 0.5, -0.1),
            Err(OutOfRange(Floor))
        );
        assert_eq!(sprint_floor_gain(0.5, 0.5, 1.1), 0.0);
    }

    #[test]
    fn checked_sprint_floor_gain_preserves_strict_threshold() {
        let threshold = 0.135_f64;
        let below = f64::from_bits(threshold.to_bits() - 1);
        let above = f64::from_bits(threshold.to_bits() + 1);

        assert_eq!(try_sprint_floor_gain(below, threshold, 0.3), Ok(0.3));
        assert_eq!(try_sprint_floor_gain(threshold, threshold, 0.3), Ok(0.3));
        assert_eq!(try_sprint_floor_gain(above, threshold, 0.3), Ok(1.0));
    }

    #[test]
    fn sprint_floor_gain_full_above_threshold() {
        // Strictly above the sprint threshold → full authority.
        assert_eq!(sprint_floor_gain(0.14, 0.135, 0.3), 1.0);
        assert_eq!(sprint_floor_gain(1.0, 0.135, 0.3), 1.0);
        assert_eq!(sprint_floor_gain(0.50, 0.13, 0.25), 1.0);
    }

    #[test]
    fn sprint_floor_gain_floor_at_and_below_threshold() {
        // At exactly the sprint threshold, strict inequality keeps us at floor
        // (the safety margin that would let a paranoid reviewer confirm
        // "sprint doesn't fire until Φ STRICTLY exceeds the threshold").
        assert_eq!(sprint_floor_gain(0.135, 0.135, 0.3), 0.3);
        assert_eq!(sprint_floor_gain(0.134, 0.135, 0.3), 0.3);
        assert_eq!(sprint_floor_gain(0.0, 0.135, 0.3), 0.3);
    }

    #[test]
    fn sprint_floor_gain_with_zero_floor_still_sprints() {
        // Floor of 0.0 is allowed (recovers a sharp-cutoff binary mapping).
        assert_eq!(sprint_floor_gain(0.14, 0.135, 0.0), 1.0);
        assert_eq!(sprint_floor_gain(0.10, 0.135, 0.0), 0.0);
    }

    #[test]
    fn sprint_floor_gain_is_two_level_only() {
        // No intermediate values — every output is either floor or 1.0.
        // This is the "middle tiers are superfluous" finding locked in.
        let floor = 0.3;
        for phi_x100 in 0..=100 {
            let phi = phi_x100 as f64 / 100.0;
            let g = sprint_floor_gain(phi, 0.135, floor);
            assert!(
                (g - 1.0).abs() < 1e-12 || (g - floor).abs() < 1e-12,
                "sprint_floor_gain produced intermediate value {} at phi={}",
                g,
                phi
            );
        }
    }
}
