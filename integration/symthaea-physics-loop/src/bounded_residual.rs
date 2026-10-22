//! Bounded cognitive-residual control for comparative research.
//!
//! A classical command remains the nominal action. Symthaea may contribute only
//! a configured, channel-wise bounded residual. This bounds the command delta;
//! it is NOT a control-barrier certificate, a stability proof, or a physical
//! safety guarantee. Limits must be calibrated and validated for the target airframe.

use symthaea_multirotor::QuadrotorCommand;

/// Research-configured residual authority and per-channel absolute delta limits.
///
/// Channel order is [thrust (N), roll moment (N·m), pitch moment (N·m),
/// yaw moment (N·m)]. These limits are experimental knobs, not universal safety
/// limits and not automatically inferred from the airframe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResidualEnvelope {
    /// Fraction of the clipped cognitive-minus-nominal delta applied, in [0, 1].
    pub authority: f32,
    /// Maximum absolute cognitive residual before authority scaling.
    pub max_abs_delta: [f32; 4],
}

impl ResidualEnvelope {
    /// Apply a bounded cognitive residual to a nominal controller command.
    ///
    /// Returns None for non-finite commands, invalid authority, or invalid limits.
    /// It deliberately fails closed instead of sanitizing a non-finite input.
    pub fn apply(
        &self,
        nominal: QuadrotorCommand,
        cognitive: QuadrotorCommand,
    ) -> Option<QuadrotorCommand> {
        let n = [
            nominal.thrust,
            nominal.roll_moment,
            nominal.pitch_moment,
            nominal.yaw_moment,
        ];
        let c = [
            cognitive.thrust,
            cognitive.roll_moment,
            cognitive.pitch_moment,
            cognitive.yaw_moment,
        ];

        if !n.iter().chain(c.iter()).all(|v| v.is_finite())
            || !self.authority.is_finite()
            || !(0.0..=1.0).contains(&self.authority)
            || !self
                .max_abs_delta
                .iter()
                .all(|limit| limit.is_finite() && *limit >= 0.0)
        {
            return None;
        }

        let nominal = nominal.clamped();
        let n = [
            nominal.thrust,
            nominal.roll_moment,
            nominal.pitch_moment,
            nominal.yaw_moment,
        ];
        let output = core::array::from_fn::<_, 4, _>(|i| {
            let residual = (c[i] - n[i]).clamp(-self.max_abs_delta[i], self.max_abs_delta[i]);
            n[i] + self.authority * residual
        });

        Some(
            QuadrotorCommand {
                thrust: output[0],
                roll_moment: output[1],
                pitch_moment: output[2],
                yaw_moment: output[3],
            }
            .clamped(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(authority: f32, max_abs_delta: [f32; 4]) -> ResidualEnvelope {
        ResidualEnvelope {
            authority,
            max_abs_delta,
        }
    }

    #[test]
    fn zero_authority_is_exactly_the_clamped_nominal_command() {
        let nominal = QuadrotorCommand::from_raw([0.3, 0.001, -0.002, 0.0003]);
        let cognitive = QuadrotorCommand::from_raw([0.6, -0.005, 0.005, -0.002]);
        let got = envelope(0.0, [0.6, 0.005, 0.005, 0.002])
            .apply(nominal, cognitive)
            .expect("finite valid commands should be accepted");
        let expected = nominal.clamped();
        assert_eq!(got.to_ctrl(), expected.to_ctrl());
    }

    #[test]
    fn cognitive_delta_is_capped_before_authority_scaling() {
        let nominal = QuadrotorCommand::hover();
        let cognitive = QuadrotorCommand::from_raw([0.6, 0.005, -0.005, 0.002]);
        let got = envelope(0.5, [0.02, 0.001, 0.001, 0.0002])
            .apply(nominal, cognitive)
            .expect("valid command should be accepted");
        assert!((got.thrust - 0.275).abs() < 1e-6);
        assert!((got.roll_moment - 0.0005).abs() < 1e-7);
        assert!((got.pitch_moment + 0.0005).abs() < 1e-7);
        assert!((got.yaw_moment - 0.0001).abs() < 1e-7);
    }

    #[test]
    fn invalid_authority_or_limits_fail_closed() {
        let nominal = QuadrotorCommand::hover();
        let cognitive = QuadrotorCommand::zero();
        assert!(envelope(f32::NAN, [0.0; 4]).apply(nominal, cognitive).is_none());
        assert!(envelope(1.01, [0.0; 4]).apply(nominal, cognitive).is_none());
        assert!(envelope(0.5, [0.0, -0.1, 0.0, 0.0]).apply(nominal, cognitive).is_none());
        assert!(envelope(0.5, [0.0, f32::INFINITY, 0.0, 0.0]).apply(nominal, cognitive).is_none());
    }

    #[test]
    fn non_finite_command_fails_closed_instead_of_being_clamped() {
        let nominal = QuadrotorCommand::hover();
        let cognitive = QuadrotorCommand::from_raw([f32::NAN, 0.0, 0.0, 0.0]);
        assert!(envelope(0.5, [0.1; 4]).apply(nominal, cognitive).is_none());
    }

    #[test]
    fn output_stays_inside_actuator_command_ranges() {
        let nominal = QuadrotorCommand::from_raw([0.6, 0.005, -0.005, 0.002]);
        let cognitive = QuadrotorCommand::from_raw([0.0, -0.005, 0.005, -0.002]);
        let got = envelope(1.0, [5.0; 4])
            .apply(nominal, cognitive)
            .expect("valid commands should be accepted");
        assert!((0.0..=QuadrotorCommand::MAX_THRUST).contains(&got.thrust));
        assert!(got.roll_moment.abs() <= QuadrotorCommand::MAX_MOMENT_RP);
        assert!(got.pitch_moment.abs() <= QuadrotorCommand::MAX_MOMENT_RP);
        assert!(got.yaw_moment.abs() <= QuadrotorCommand::MAX_MOMENT_YAW);
    }
}
