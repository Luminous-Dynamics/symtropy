// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Per-joint command attenuation helper.
//!
//! This module is deliberately dependency-light. Its tiers are local
//! attenuation categories, not the Phi-derived tiers in the AGPL research
//! crate, and not a safety-rated control system. Platform-specific safety
//! logic must independently own hard limits, fault handling and the
//! physical safe state.

/// Local authority/attenuation tier used by the robotics bridge core.
///
/// This enum is intentionally separate from the AGPL research layer's
/// SafetyTier. The gain values are convenience policy defaults; they are
/// not derived from a standard, a hazard analysis or a calibrated actuator
/// envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JointAuthorityTier {
    /// Full requested command is passed through.
    Green,
    /// Requested command is scaled to 60%.
    Yellow,
    /// Requested command is scaled to 30%.
    Orange,
    /// Requested command is scaled to zero.
    Red,
}

impl JointAuthorityTier {
    /// Scalar attenuation multiplier in [0.0, 1.0].
    #[inline]
    pub fn motor_gain(&self) -> f64 {
        match self {
            Self::Green => 1.0,
            Self::Yellow => 0.6,
            Self::Orange => 0.3,
            Self::Red => 0.0,
        }
    }

    /// Whether the helper permits a non-zero command.
    ///
    /// This is only a policy query. A zero command is not a universally safe
    /// state for a physical robot (for example, gravity-loaded joints may
    /// require active holding or a controlled descent).
    #[inline]
    pub fn allows_output(&self) -> bool {
        *self != Self::Red
    }
}

/// Legacy per-joint command attenuation helper.
///
/// Despite the historical type name, this is not an independent or
/// safety-rated safety authority. It scales a caller-provided command vector;
/// it does not know actuator units, torque/current/velocity limits, contact
/// state, platform safe states or a hazard analysis. Do not use it as the
/// sole protective function on physical hardware.
pub struct JointSafetyAuthority {
    /// Per-joint attenuation categories.
    pub joint_tiers: Vec<JointAuthorityTier>,
}

impl JointSafetyAuthority {
    /// Create a new helper for the given number of actuators.
    pub fn new(num_actuators: usize) -> Self {
        Self {
            joint_tiers: vec![JointAuthorityTier::Green; num_actuators],
        }
    }

    /// Update per-joint attenuation categories from localized prediction errors.
    ///
    /// These thresholds are demo heuristics, not calibrated safety thresholds.
    pub fn update_from_surprise(&mut self, surprises: &[f64]) {
        for (i, &surprise) in surprises.iter().enumerate() {
            if i >= self.joint_tiers.len() {
                break;
            }

            self.joint_tiers[i] = if surprise < 1.0 {
                JointAuthorityTier::Green
            } else if surprise < 3.0 {
                JointAuthorityTier::Yellow
            } else if surprise < 5.0 {
                JointAuthorityTier::Orange
            } else {
                JointAuthorityTier::Red
            };
        }
    }

    /// Apply the configured scalar attenuation to the matching commands.
    ///
    /// Commands without a corresponding tier are left untouched for
    /// backwards compatibility; this helper must not be treated as a
    /// fail-closed hardware safety gate.
    pub fn apply_gains(&self, commands: &mut [f64]) {
        for (i, command) in commands.iter_mut().enumerate() {
            if i >= self.joint_tiers.len() {
                break;
            }
            *command *= self.joint_tiers[i].motor_gain();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attenuation_tiers_have_explicit_gains() {
        assert_eq!(JointAuthorityTier::Green.motor_gain(), 1.0);
        assert_eq!(JointAuthorityTier::Yellow.motor_gain(), 0.6);
        assert_eq!(JointAuthorityTier::Orange.motor_gain(), 0.3);
        assert_eq!(JointAuthorityTier::Red.motor_gain(), 0.0);
    }

    #[test]
    fn surprise_thresholds_select_the_expected_tier() {
        let mut authority = JointSafetyAuthority::new(4);
        authority.update_from_surprise(&[0.9, 1.0, 3.0, 5.0]);
        assert_eq!(
            authority.joint_tiers,
            vec![
                JointAuthorityTier::Green,
                JointAuthorityTier::Yellow,
                JointAuthorityTier::Orange,
                JointAuthorityTier::Red,
            ]
        );
    }

    #[test]
    fn gains_are_applied_per_joint_without_changing_unmatched_values() {
        let mut authority = JointSafetyAuthority::new(3);
        authority.joint_tiers = vec![
            JointAuthorityTier::Green,
            JointAuthorityTier::Yellow,
            JointAuthorityTier::Red,
        ];
        let mut commands = [1.0, 1.0, 1.0, 0.5];
        authority.apply_gains(&mut commands);
        assert_eq!(commands, [1.0, 0.6, 0.0, 0.5]);
    }
}
