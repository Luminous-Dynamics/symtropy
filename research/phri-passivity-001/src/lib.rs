#![forbid(unsafe_code)]

//! Deterministic interaction-port passivity observer reference.
//!
//! The measured wrench is the environment-on-robot wrench and the measured
//! twist is the robot's co-located spatial velocity. Positive wrench·twist is
//! power flowing into the robot/controller system.
//!
//! This crate measures port work and a conservative passivity residual only.
//! It is not a variable-admittance controller, passivity controller, force
//! safety limiter, actuator authority, or proof that a complete robot is safe.

use core::fmt;

/// Identifies one monotonic clock instance and boot epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClockDomain {
    pub clock_id: u64,
    pub boot_epoch: u64,
}

/// One synchronized sample at a declared physical interaction port.
///
/// Wrench coordinates are [Fx, Fy, Fz, Tx, Ty, Tz] in SI units (N and N·m).
/// Twist coordinates are [vx, vy, vz, wx, wy, wz] in SI units (m/s and rad/s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortSample {
    pub sample_id: u64,
    pub at_ns: u64,
    pub clock: ClockDomain,
    pub frame_id: u64,
    pub wrench_environment_on_robot: [f64; 6],
    pub twist_robot: [f64; 6],
}

/// Invalid input or event ordering. Unknown clock/frame relationships never
/// become zero work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassivityError {
    InvalidInitialStorage,
    InvalidEnergyUncertainty,
    InvalidTolerance,
    ClockDomainMismatch,
    FrameMismatch,
    NonMonotonicTimestamp,
    NonMonotonicSampleId,
    NonFinitePortSample,
    ArithmeticOverflow,
}

impl fmt::Display for PassivityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PassivityError {}

/// Snapshot of the cumulative interaction-port energy balance.
///
/// conservative_storage_lower_bound_j is:
/// initial_storage_j + cumulative_port_work_j - energy_uncertainty_j.
/// A negative lower bound is evidence of a passivity deficit under the stated
/// port/sign/model assumptions; it is not a complete stability or safety verdict.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassivitySnapshot {
    pub sample_count: u64,
    pub elapsed_ns: Option<u64>,
    pub cumulative_port_work_j: f64,
    pub conservative_storage_lower_bound_j: f64,
    pub passivity_deficit_j: f64,
    pub maximum_passivity_deficit_j: f64,
    /// Number of observed sample endpoints at which the deficit exceeded the
    /// declared tolerance. This is not a count of independent physical events.
    pub violation_sample_count: u64,
    pub last_port_power_into_robot_w: Option<f64>,
}

/// Integrates measured interaction-port power and tracks the passivity
/// residual against a declared initial energy store and fixed uncertainty
/// allowance.
///
/// Samples must use the same monotonic clock/boot epoch and spatial frame.
/// The observer uses trapezoidal integration between consecutive samples.
/// This reference does not estimate wrench/twist, synchronize clocks, correct
/// frame transforms, or model time-varying virtual-parameter storage.
#[derive(Clone, Debug)]
pub struct PassivityObserver {
    clock: ClockDomain,
    frame_id: u64,
    initial_storage_j: f64,
    energy_uncertainty_j: f64,
    violation_tolerance_j: f64,
    first_timestamp_ns: Option<u64>,
    previous_sample: Option<PortSample>,
    previous_power_w: Option<f64>,
    cumulative_port_work_j: f64,
    maximum_passivity_deficit_j: f64,
    violation_sample_count: u64,
    sample_count: u64,
}

impl PassivityObserver {
    /// Create an observer for one exact clock/boot epoch and interaction frame.
    ///
    /// Energy uncertainty is a conservative fixed allowance in joules. It is
    /// not automatically propagated from instrument certificates; the caller
    /// must derive and justify it for the actual measurement chain.
    pub fn new(
        clock: ClockDomain,
        frame_id: u64,
        initial_storage_j: f64,
        energy_uncertainty_j: f64,
        violation_tolerance_j: f64,
    ) -> Result<Self, PassivityError> {
        if !initial_storage_j.is_finite() || initial_storage_j < 0.0 {
            return Err(PassivityError::InvalidInitialStorage);
        }
        if !energy_uncertainty_j.is_finite() || energy_uncertainty_j < 0.0 {
            return Err(PassivityError::InvalidEnergyUncertainty);
        }
        if !violation_tolerance_j.is_finite() || violation_tolerance_j < 0.0 {
            return Err(PassivityError::InvalidTolerance);
        }

        Ok(Self {
            clock,
            frame_id,
            initial_storage_j,
            energy_uncertainty_j,
            violation_tolerance_j,
            first_timestamp_ns: None,
            previous_sample: None,
            previous_power_w: None,
            cumulative_port_work_j: 0.0,
            maximum_passivity_deficit_j: 0.0,
            violation_sample_count: 0,
            sample_count: 0,
        })
    }

    /// Add one measured sample and return the updated cumulative snapshot.
    ///
    /// State mutation is transactional: a rejected sample leaves the observer
    /// unchanged. The first sample establishes the integration origin and
    /// contributes no interval work.
    pub fn observe(
        &mut self,
        sample: PortSample,
    ) -> Result<PassivitySnapshot, PassivityError> {
        if sample.clock != self.clock {
            return Err(PassivityError::ClockDomainMismatch);
        }
        if sample.frame_id != self.frame_id {
            return Err(PassivityError::FrameMismatch);
        }
        let power_w = port_power_into_robot_w(
            &sample.wrench_environment_on_robot,
            &sample.twist_robot,
        )?;

        let next_work = match (self.previous_sample, self.previous_power_w) {
            (Some(previous), Some(previous_power)) => {
                let elapsed_ns = sample
                    .at_ns
                    .checked_sub(previous.at_ns)
                    .filter(|elapsed| *elapsed > 0)
                    .ok_or(PassivityError::NonMonotonicTimestamp)?;
                if sample.sample_id <= previous.sample_id {
                    return Err(PassivityError::NonMonotonicSampleId);
                }
                let dt_s = elapsed_ns as f64 * 1.0e-9;
                let average_power_w = 0.5 * (previous_power + power_w);
                let interval_work_j = average_power_w * dt_s;
                let total = self.cumulative_port_work_j + interval_work_j;
                if !dt_s.is_finite()
                    || !average_power_w.is_finite()
                    || !interval_work_j.is_finite()
                    || !total.is_finite()
                {
                    return Err(PassivityError::ArithmeticOverflow);
                }
                total
            }
            (None, None) => self.cumulative_port_work_j,
            _ => return Err(PassivityError::ArithmeticOverflow),
        };

        let conservative_storage =
            self.initial_storage_j + next_work - self.energy_uncertainty_j;
        if !conservative_storage.is_finite() {
            return Err(PassivityError::ArithmeticOverflow);
        }
        let deficit = (-conservative_storage).max(0.0);
        let next_maximum_deficit = self.maximum_passivity_deficit_j.max(deficit);
        let next_violation_count = if deficit > self.violation_tolerance_j {
            self.violation_sample_count
                .checked_add(1)
                .ok_or(PassivityError::ArithmeticOverflow)?
        } else {
            self.violation_sample_count
        };
        let next_sample_count = self
            .sample_count
            .checked_add(1)
            .ok_or(PassivityError::ArithmeticOverflow)?;

        self.cumulative_port_work_j = next_work;
        self.maximum_passivity_deficit_j = next_maximum_deficit;
        self.violation_sample_count = next_violation_count;
        self.sample_count = next_sample_count;
        if self.first_timestamp_ns.is_none() {
            self.first_timestamp_ns = Some(sample.at_ns);
        }
        self.previous_sample = Some(sample);
        self.previous_power_w = Some(power_w);

        Ok(self.snapshot())
    }

    /// Return the current snapshot without changing observer state.
    pub fn snapshot(&self) -> PassivitySnapshot {
        let conservative_storage = self.initial_storage_j + self.cumulative_port_work_j
            - self.energy_uncertainty_j;
        let deficit = (-conservative_storage).max(0.0);
        let elapsed_ns = match (self.first_timestamp_ns, self.previous_sample) {
            (Some(first), Some(last)) => Some(last.at_ns - first),
            _ => None,
        };

        PassivitySnapshot {
            sample_count: self.sample_count,
            elapsed_ns,
            cumulative_port_work_j: self.cumulative_port_work_j,
            conservative_storage_lower_bound_j: conservative_storage,
            passivity_deficit_j: deficit,
            maximum_passivity_deficit_j: self.maximum_passivity_deficit_j,
            violation_sample_count: self.violation_sample_count,
            last_port_power_into_robot_w: self.previous_power_w,
        }
    }

    /// Declared interaction-port clock and boot identity.
    pub const fn clock_domain(&self) -> ClockDomain {
        self.clock
    }

    /// Declared spatial frame identity.
    pub const fn frame_id(&self) -> u64 {
        self.frame_id
    }
}

/// Compute one instantaneous wrench/twist power sample.
///
/// Positive output means energy flows from the environment into the robot.
pub fn port_power_into_robot_w(
    wrench_environment_on_robot: &[f64; 6],
    twist_robot: &[f64; 6],
) -> Result<f64, PassivityError> {
    let mut power = 0.0;
    for index in 0..6 {
        let wrench = wrench_environment_on_robot[index];
        let velocity = twist_robot[index];
        if !wrench.is_finite() || !velocity.is_finite() {
            return Err(PassivityError::NonFinitePortSample);
        }
        power += wrench * velocity;
        if !power.is_finite() {
            return Err(PassivityError::ArithmeticOverflow);
        }
    }
    Ok(power)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock() -> ClockDomain {
        ClockDomain {
            clock_id: 17,
            boot_epoch: 4,
        }
    }

    fn sample(
        sample_id: u64,
        at_ns: u64,
        force_x_n: f64,
        velocity_x_mps: f64,
    ) -> PortSample {
        PortSample {
            sample_id,
            at_ns,
            clock: clock(),
            frame_id: 99,
            wrench_environment_on_robot: [force_x_n, 0.0, 0.0, 0.0, 0.0, 0.0],
            twist_robot: [velocity_x_mps, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    fn observer(
        initial_storage_j: f64,
        uncertainty_j: f64,
        tolerance_j: f64,
    ) -> PassivityObserver {
        PassivityObserver::new(clock(), 99, initial_storage_j, uncertainty_j, tolerance_j)
            .unwrap()
    }

    #[test]
    fn positive_environment_work_replenishes_storage() {
        let mut subject = observer(0.5, 0.0, 1.0e-12);
        subject.observe(sample(1, 0, 10.0, 0.1)).unwrap();
        let result = subject.observe(sample(2, 1_000_000_000, 10.0, 0.1)).unwrap();

        assert!((result.cumulative_port_work_j - 1.0).abs() < 1.0e-12);
        assert!((result.conservative_storage_lower_bound_j - 1.5).abs() < 1.0e-12);
        assert_eq!(result.passivity_deficit_j, 0.0);
        assert_eq!(result.violation_sample_count, 0);
    }

    #[test]
    fn negative_environment_work_drains_storage_and_exposes_deficit() {
        let mut subject = observer(0.5, 0.0, 1.0e-9);
        subject.observe(sample(1, 0, -10.0, 0.1)).unwrap();
        let result = subject.observe(sample(2, 1_000_000_000, -10.0, 0.1)).unwrap();

        assert!((result.cumulative_port_work_j + 1.0).abs() < 1.0e-12);
        assert!((result.conservative_storage_lower_bound_j + 0.5).abs() < 1.0e-12);
        assert!((result.passivity_deficit_j - 0.5).abs() < 1.0e-12);
        assert!((result.maximum_passivity_deficit_j - 0.5).abs() < 1.0e-12);
        assert_eq!(result.violation_sample_count, 1);
    }

    #[test]
    fn force_and_velocity_sign_convention_is_explicit() {
        let positive = port_power_into_robot_w(
            &[5.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            &[0.2, 0.0, 0.0, 0.0, 0.0, 0.0],
        )
        .unwrap();
        let negative = port_power_into_robot_w(
            &[-5.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            &[0.2, 0.0, 0.0, 0.0, 0.0, 0.0],
        )
        .unwrap();

        assert!((positive - 1.0).abs() < 1.0e-12);
        assert!((negative + 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn trapezoidal_integral_handles_ramping_interaction_power() {
        let mut subject = observer(0.0, 0.0, 1.0e-12);
        subject.observe(sample(1, 0, 10.0, 0.1)).unwrap(); // 1 W
        let result = subject.observe(sample(2, 2_000_000_000, 30.0, 0.1)).unwrap(); // 3 W

        // Average power is 2 W over 2 seconds.
        assert!((result.cumulative_port_work_j - 4.0).abs() < 1.0e-12);
    }

    #[test]
    fn irregular_sample_intervals_are_integrated_using_actual_elapsed_time() {
        let mut subject = observer(0.0, 0.0, 1.0e-12);
        subject.observe(sample(1, 0, 10.0, 0.1)).unwrap(); // 1 W
        subject.observe(sample(2, 1_000_000_000, 10.0, 0.1)).unwrap(); // +1 J
        let result = subject.observe(sample(3, 3_000_000_000, 30.0, 0.1)).unwrap(); // +4 J

        assert!((result.cumulative_port_work_j - 5.0).abs() < 1.0e-12);
        assert_eq!(result.elapsed_ns, Some(3_000_000_000));
    }

    #[test]
    fn energy_uncertainty_is_subtracted_conservatively() {
        let mut subject = observer(0.5, 0.4, 1.0e-12);
        subject.observe(sample(1, 0, -10.0, 0.1)).unwrap();
        let result = subject.observe(sample(2, 1_000_000_000, -10.0, 0.1)).unwrap();

        assert!((result.conservative_storage_lower_bound_j + 0.9).abs() < 1.0e-12);
        assert!((result.passivity_deficit_j - 0.9).abs() < 1.0e-12);
    }

    #[test]
    fn wrong_clock_domain_is_rejected_without_mutating_state() {
        let mut subject = observer(1.0, 0.0, 0.0);
        subject.observe(sample(1, 10, 0.0, 0.0)).unwrap();
        let before = subject.snapshot();
        let mut bad = sample(2, 20, 0.0, 0.0);
        bad.clock.boot_epoch += 1;

        assert_eq!(subject.observe(bad), Err(PassivityError::ClockDomainMismatch));
        assert_eq!(subject.snapshot(), before);
    }

    #[test]
    fn wrong_spatial_frame_is_rejected() {
        let mut subject = observer(1.0, 0.0, 0.0);
        subject.observe(sample(1, 10, 0.0, 0.0)).unwrap();
        let mut bad = sample(2, 20, 0.0, 0.0);
        bad.frame_id += 1;

        assert_eq!(subject.observe(bad), Err(PassivityError::FrameMismatch));
    }

    #[test]
    fn duplicate_or_reordered_sample_ids_are_rejected() {
        let mut subject = observer(1.0, 0.0, 0.0);
        subject.observe(sample(2, 10, 0.0, 0.0)).unwrap();
        assert_eq!(
            subject.observe(sample(2, 20, 0.0, 0.0)),
            Err(PassivityError::NonMonotonicSampleId)
        );
    }

    #[test]
    fn duplicate_or_nonmonotonic_timestamps_are_rejected() {
        let mut subject = observer(1.0, 0.0, 0.0);
        subject.observe(sample(1, 20, 0.0, 0.0)).unwrap();
        assert_eq!(
            subject.observe(sample(2, 20, 0.0, 0.0)),
            Err(PassivityError::NonMonotonicTimestamp)
        );
    }

    #[test]
    fn nonfinite_port_inputs_are_rejected_without_mutation() {
        let mut subject = observer(1.0, 0.0, 0.0);
        subject.observe(sample(1, 10, 0.0, 0.0)).unwrap();
        let before = subject.snapshot();
        let mut bad = sample(2, 20, 0.0, 0.0);
        bad.wrench_environment_on_robot[2] = f64::NAN;

        assert_eq!(subject.observe(bad), Err(PassivityError::NonFinitePortSample));
        assert_eq!(subject.snapshot(), before);
    }

    #[test]
    fn storage_uncertainty_and_tolerance_are_validated() {
        assert!(matches!(
            PassivityObserver::new(clock(), 99, -1.0, 0.0, 0.0),
            Err(PassivityError::InvalidInitialStorage)
        ));
        assert!(matches!(
            PassivityObserver::new(clock(), 99, 0.0, -0.1, 0.0),
            Err(PassivityError::InvalidEnergyUncertainty)
        ));
        assert!(matches!(
            PassivityObserver::new(clock(), 99, 0.0, 0.0, f64::NAN),
            Err(PassivityError::InvalidTolerance)
        ));
    }

    #[test]
    fn deficit_at_or_below_tolerance_is_not_counted_as_violation() {
        let mut subject = observer(0.5, 0.0, 0.5);
        subject.observe(sample(1, 0, -10.0, 0.1)).unwrap();
        let result = subject.observe(sample(2, 1_000_000_000, -10.0, 0.1)).unwrap();

        assert!((result.passivity_deficit_j - 0.5).abs() < 1.0e-12);
        assert_eq!(result.violation_sample_count, 0);
    }
}
