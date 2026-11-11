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
    pub port_id: u64,
    pub frame_id: u64,
    /// Digest of the exact port sensor/calibration/frame/time-alignment manifest.
    pub measurement_profile_digest: [u8; 32],
    pub wrench_environment_on_robot: [f64; 6],
    pub twist_robot: [f64; 6],
}

/// Invalid input or event ordering. Unknown clock/frame relationships never
/// become zero work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassivityError {
    InvalidInitialEnergyBudget,
    InvalidEnergyUncertainty,
    InvalidTolerance,
    InvalidMaximumSampleGap,
    InvalidMeasurementProfileDigest,
    ClockDomainMismatch,
    PortMismatch,
    MeasurementProfileMismatch,
    SampleGapExceeded,
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
/// initial_energy_budget_j + cumulative_port_work_j - energy_uncertainty_j.
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
    /// Number of sample intervals whose within-interval minimum deficit exceeded
    /// the declared tolerance. This is not a count of independent physical events.
    pub violation_interval_count: u64,
    pub last_port_power_into_robot_w: Option<f64>,
}

/// Frozen observer configuration for one calibrated physical interaction port.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassivityObserverConfig {
    pub port_id: u64,
    pub frame_id: u64,
    pub measurement_profile_digest: [u8; 32],
    pub max_sample_gap_ns: u64,
    pub initial_energy_budget_j: f64,
    pub energy_uncertainty_j: f64,
    pub violation_tolerance_j: f64,
}

/// Integrates measured interaction-port power and tracks the passivity
/// residual against a declared initial available-energy budget and fixed uncertainty
/// allowance.
///
/// Samples must use the same monotonic clock/boot epoch, port, spatial frame,
/// and measurement-profile digest. The profile digest binds instrument
/// calibration, geometry/frame registration, filtering and time alignment.
/// The observer uses trapezoidal integration and checks the minimum energy
/// budget inside each piecewise-linear power interval, not just endpoints.
/// This reference does not estimate wrench/twist, synchronize clocks, correct
/// frame transforms, or model time-varying virtual-parameter storage.
#[derive(Clone, Debug)]
pub struct PassivityObserver {
    clock: ClockDomain,
    frame_id: u64,
    initial_energy_budget_j: f64,
    port_id: u64,
    measurement_profile_digest: [u8; 32],
    max_sample_gap_ns: u64,
    energy_uncertainty_j: f64,
    violation_tolerance_j: f64,
    first_timestamp_ns: Option<u64>,
    previous_sample: Option<PortSample>,
    previous_power_w: Option<f64>,
    cumulative_port_work_j: f64,
    maximum_passivity_deficit_j: f64,
    violation_interval_count: u64,
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
        config: PassivityObserverConfig,
    ) -> Result<Self, PassivityError> {
        if !config.initial_energy_budget_j.is_finite() || config.initial_energy_budget_j < 0.0 {
            return Err(PassivityError::InvalidInitialEnergyBudget);
        }
        if !config.energy_uncertainty_j.is_finite() || config.energy_uncertainty_j < 0.0 {
            return Err(PassivityError::InvalidEnergyUncertainty);
        }
        if !config.violation_tolerance_j.is_finite() || config.violation_tolerance_j < 0.0 {
            return Err(PassivityError::InvalidTolerance);
        }
        if config.max_sample_gap_ns == 0 {
            return Err(PassivityError::InvalidMaximumSampleGap);
        }
        if config.measurement_profile_digest == [0; 32] {
            return Err(PassivityError::InvalidMeasurementProfileDigest);
        }

        Ok(Self {
            clock,
            port_id: config.port_id,
            frame_id: config.frame_id,
            measurement_profile_digest: config.measurement_profile_digest,
            max_sample_gap_ns: config.max_sample_gap_ns,
            initial_energy_budget_j: config.initial_energy_budget_j,
            energy_uncertainty_j: config.energy_uncertainty_j,
            violation_tolerance_j: config.violation_tolerance_j,
            first_timestamp_ns: None,
            previous_sample: None,
            previous_power_w: None,
            cumulative_port_work_j: 0.0,
            maximum_passivity_deficit_j: 0.0,
            violation_interval_count: 0,
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
        if sample.port_id != self.port_id {
            return Err(PassivityError::PortMismatch);
        }
        if sample.frame_id != self.frame_id {
            return Err(PassivityError::FrameMismatch);
        }
        if sample.measurement_profile_digest != self.measurement_profile_digest {
            return Err(PassivityError::MeasurementProfileMismatch);
        }
        let power_w = port_power_into_robot_w(
            &sample.wrench_environment_on_robot,
            &sample.twist_robot,
        )?;

        let has_previous_interval = self.previous_sample.is_some();
        let mut interval_minimum_work_j = 0.0_f64;
        let next_work = match (self.previous_sample, self.previous_power_w) {
            (Some(previous), Some(previous_power)) => {
                let elapsed_ns = sample
                    .at_ns
                    .checked_sub(previous.at_ns)
                    .filter(|elapsed| *elapsed > 0)
                    .ok_or(PassivityError::NonMonotonicTimestamp)?;
                if elapsed_ns > self.max_sample_gap_ns {
                    return Err(PassivityError::SampleGapExceeded);
                }
                if sample.sample_id <= previous.sample_id {
                    return Err(PassivityError::NonMonotonicSampleId);
                }
                let dt_s = elapsed_ns as f64 * 1.0e-9;
                let average_power_w = 0.5 * (previous_power + power_w);
                let interval_work_j = average_power_w * dt_s;
                interval_minimum_work_j = minimum_linear_power_prefix_work_j(
                    previous_power,
                    power_w,
                    dt_s,
                );
                let total = self.cumulative_port_work_j + interval_work_j;
                if !dt_s.is_finite()
                    || !average_power_w.is_finite()
                    || !interval_work_j.is_finite()
                    || !interval_minimum_work_j.is_finite()
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
            self.initial_energy_budget_j + next_work - self.energy_uncertainty_j;
        let interval_minimum_storage = self.initial_energy_budget_j
            + self.cumulative_port_work_j
            + interval_minimum_work_j
            - self.energy_uncertainty_j;
        if !conservative_storage.is_finite() || !interval_minimum_storage.is_finite() {
            return Err(PassivityError::ArithmeticOverflow);
        }
        let endpoint_deficit = (-conservative_storage).max(0.0);
        let interval_deficit = (-interval_minimum_storage).max(0.0);
        let maximum_interval_deficit = endpoint_deficit.max(interval_deficit);
        let next_maximum_deficit = self.maximum_passivity_deficit_j.max(maximum_interval_deficit);
        let next_violation_count = if has_previous_interval
            && maximum_interval_deficit > self.violation_tolerance_j
        {
            self.violation_interval_count
                .checked_add(1)
                .ok_or(PassivityError::ArithmeticOverflow)?
        } else {
            self.violation_interval_count
        };
        let next_sample_count = self
            .sample_count
            .checked_add(1)
            .ok_or(PassivityError::ArithmeticOverflow)?;

        self.cumulative_port_work_j = next_work;
        self.maximum_passivity_deficit_j = next_maximum_deficit;
        self.violation_interval_count = next_violation_count;
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
        let conservative_storage = self.initial_energy_budget_j + self.cumulative_port_work_j
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
            violation_interval_count: self.violation_interval_count,
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

    /// Declared physical interaction port identity.
    pub const fn port_id(&self) -> u64 {
        self.port_id
    }

    /// Exact metrology/calibration/registration profile digest.
    pub const fn measurement_profile_digest(&self) -> &[u8; 32] {
        &self.measurement_profile_digest
    }
}

fn minimum_linear_power_prefix_work_j(
    power_start_w: f64,
    power_end_w: f64,
    duration_s: f64,
) -> f64 {
    let end_work_j = 0.5 * (power_start_w + power_end_w) * duration_s;
    let mut minimum_work_j = 0.0_f64.min(end_work_j);

    if power_start_w < 0.0 && power_end_w > 0.0 {
        let scale = power_start_w.abs().max(power_end_w.abs());
        let scaled_start = power_start_w / scale;
        let scaled_end = power_end_w / scale;
        let fraction_to_zero = -scaled_start / (scaled_end - scaled_start);
        let interior_work_j = 0.5 * power_start_w * fraction_to_zero * duration_s;
        minimum_work_j = minimum_work_j.min(interior_work_j);
    }

    minimum_work_j
}

/// Compute one instantaneous wrench/twist power sample.
///
/// Positive output means energy flows from the environment into the robot.
pub fn port_power_into_robot_w(
    wrench_environment_on_robot: &[f64; 6],
    twist_robot: &[f64; 6],
) -> Result<f64, PassivityError> {
    let mut power = 0.0;
    for (wrench, velocity) in wrench_environment_on_robot.iter().zip(twist_robot.iter()) {
        let wrench = *wrench;
        let velocity = *velocity;
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

/// Diagonal virtual impedance state for one six-dimensional interaction port.
///
/// Entries 0..3 are translational axes; entries 3..6 are rotational axes.
/// Virtual inertia units are kg for translation and kg·m² for rotation.
/// Stiffness units are N/m for translation and N·m/rad for rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiagonalVirtualImpedance {
    pub virtual_inertia: [f64; 6],
    pub stiffness: [f64; 6],
}

/// Energy change caused solely by changing virtual inertia/stiffness while
/// holding the current error and relative velocity fixed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualParameterStorageDelta {
    /// Signed change in the quadratic virtual storage function.
    pub delta_storage_j: f64,
    /// Conservative energy-budget debit; only positive storage creation is
    /// charged. A reduction in virtual storage is not automatically credited.
    pub required_energy_debit_j: f64,
    /// Potentially released storage that is deliberately not credited.
    pub uncredited_storage_release_j: f64,
}

/// Request for the diagonal variable-impedance passivity-rate QP.
///
/// The model follows the sufficient continuous-time passivity inequality
/// derived for diagonal desired inertia/stiffness in Dinc et al. (arXiv:2611.10778):
///
/// 0.5 * sum(v_i^2 * Lambda_d_i * alpha_dot_i + e_i^2 * K_dot_i)
///     - sum(v_i^2 * D_i) <= 0
///
/// with D_i = 2 * sqrt(K_i * alpha_i * Lambda_d_i).
///
/// This is a task-space diagonal model. It does not qualify null-space
/// dynamics, full-matrix compliance, discrete integration or physical safety.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiagonalPassivityQpRequest {
    pub base_inertia: [f64; 6],
    pub inertia_scale: [f64; 6],
    pub minimum_inertia_scale: [f64; 6],
    pub stiffness: [f64; 6],
    pub error: [f64; 6],
    pub error_velocity: [f64; 6],
    pub inertia_decay_rate: [f64; 6],
    pub stiffness_decay_rate: [f64; 6],
    pub inertia_reference_rate: [f64; 6],
    pub stiffness_reference_rate: [f64; 6],
    pub inertia_max_rate: [f64; 6],
    pub stiffness_max_rate: [f64; 6],
    /// Positive diagonal objective weights ordered [alpha_dot[0..6], K_dot[0..6]].
    pub objective_weights: [f64; 12],
}

/// Solution to the diagonal variable-impedance rate QP.
///
/// Rates satisfy the current-state continuous-time sufficient passivity
/// inequality and the parameter lower/upper rate bounds. The caller must
/// still qualify sampled integration, state evolution between updates,
/// null-space behavior, actuator limits and its independent safety authority.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiagonalPassivityQpSelection {
    pub inertia_rate: [f64; 6],
    pub stiffness_rate: [f64; 6],
    pub passivity_lhs_w: f64,
    pub damping_power_rhs_w: f64,
    pub passivity_slack_w: f64,
    pub dual_multiplier: f64,
    pub projection_active: bool,
}

/// Solve the diagonal passivity-constrained rate QP with a scalar KKT
/// multiplier and 80 fixed bisection iterations.
///
/// The objective is the weighted squared distance to the reference rates.
/// There is one affine passivity inequality and twelve box constraints, so
/// the solution is the weighted projection onto a half-space intersected
/// with the per-parameter rate box. No solver dependency or allocation is used.
pub fn solve_diagonal_passivity_rate_qp(
    request: DiagonalPassivityQpRequest,
) -> Result<DiagonalPassivityQpSelection, VirtualImpedanceError> {
    let mut coefficients = [0.0_f64; 12];
    let mut lower = [0.0_f64; 12];
    let mut upper = [0.0_f64; 12];
    let mut reference = [0.0_f64; 12];
    let weights = request.objective_weights;
    let mut damping_power_rhs_w = 0.0_f64;

    for (index, values) in request.base_inertia.iter()
        .zip(request.inertia_scale.iter())
        .zip(request.minimum_inertia_scale.iter())
        .zip(request.stiffness.iter())
        .zip(request.error.iter())
        .zip(request.error_velocity.iter())
        .zip(request.inertia_decay_rate.iter())
        .zip(request.stiffness_decay_rate.iter())
        .zip(request.inertia_reference_rate.iter())
        .zip(request.stiffness_reference_rate.iter())
        .zip(request.inertia_max_rate.iter())
        .zip(request.stiffness_max_rate.iter())
        .enumerate()
    {
        let (((((((((((base_inertia, inertia_scale), min_scale), stiffness), error), velocity), alpha_decay), stiffness_decay), alpha_rate_ref), stiffness_rate_ref), alpha_rate_max), stiffness_rate_max) = values;
        let base_inertia = *base_inertia;
        let inertia_scale = *inertia_scale;
        let min_scale = *min_scale;
        let stiffness = *stiffness;
        let error = *error;
        let velocity = *velocity;
        let alpha_decay = *alpha_decay;
        let stiffness_decay = *stiffness_decay;
        let alpha_rate_ref = *alpha_rate_ref;
        let stiffness_rate_ref = *stiffness_rate_ref;
        let alpha_rate_max = *alpha_rate_max;
        let stiffness_rate_max = *stiffness_rate_max;

        if !base_inertia.is_finite() || base_inertia <= 0.0
            || !inertia_scale.is_finite() || inertia_scale <= 0.0
            || !min_scale.is_finite() || min_scale <= 0.0
            || inertia_scale < min_scale
        {
            return Err(VirtualImpedanceError::NonPositiveVirtualInertia);
        }
        if !stiffness.is_finite() || stiffness <= 0.0 {
            return Err(VirtualImpedanceError::NegativeStiffness);
        }
        if !error.is_finite() || !velocity.is_finite()
            || !alpha_decay.is_finite() || alpha_decay <= 0.0
            || !stiffness_decay.is_finite() || stiffness_decay <= 0.0
            || !alpha_rate_ref.is_finite() || !stiffness_rate_ref.is_finite()
            || !alpha_rate_max.is_finite() || !stiffness_rate_max.is_finite()
        {
            return Err(VirtualImpedanceError::NonFiniteParameter);
        }

        let alpha_lower = alpha_decay * (min_scale - inertia_scale);
        let stiffness_lower = -stiffness_decay * stiffness;
        if !alpha_lower.is_finite() || !stiffness_lower.is_finite()
            || alpha_rate_max < alpha_lower
            || stiffness_rate_max < stiffness_lower
        {
            return Err(VirtualImpedanceError::InvalidUpdateBounds);
        }

        let damping = 2.0 * (stiffness * inertia_scale * base_inertia).sqrt();
        let alpha_coefficient = 0.5 * velocity * velocity * base_inertia;
        let stiffness_coefficient = 0.5 * error * error;
        let damping_power = velocity * velocity * damping;
        if !damping.is_finite()
            || !alpha_coefficient.is_finite()
            || !stiffness_coefficient.is_finite()
            || !damping_power.is_finite()
        {
            return Err(VirtualImpedanceError::ArithmeticOverflow);
        }

        coefficients[index] = alpha_coefficient;
        coefficients[index + 6] = stiffness_coefficient;
        lower[index] = alpha_lower;
        lower[index + 6] = stiffness_lower;
        upper[index] = alpha_rate_max;
        upper[index + 6] = stiffness_rate_max;
        reference[index] = alpha_rate_ref;
        reference[index + 6] = stiffness_rate_ref;
        damping_power_rhs_w += damping_power;
        if !damping_power_rhs_w.is_finite() {
            return Err(VirtualImpedanceError::ArithmeticOverflow);
        }
    }

    for (weight, (lower_rate, upper_rate)) in weights.iter().zip(lower.iter().zip(upper.iter())) {
        if !weight.is_finite() || *weight <= 0.0 {
            return Err(VirtualImpedanceError::InvalidUpdateWeight);
        }
        if lower_rate > upper_rate {
            return Err(VirtualImpedanceError::InvalidUpdateBounds);
        }
    }

    let minimum_lhs = coefficients.iter().zip(lower.iter())
        .map(|(coefficient, rate)| coefficient * rate)
        .sum::<f64>();
    if !minimum_lhs.is_finite() {
        return Err(VirtualImpedanceError::ArithmeticOverflow);
    }
    if minimum_lhs > damping_power_rhs_w {
        return Err(VirtualImpedanceError::EnergyBudgetInfeasible);
    }

    let mut accepted = core::array::from_fn(|index| {
        reference[index].clamp(lower[index], upper[index])
    });
    let unconstrained_lhs = coefficients.iter().zip(accepted.iter())
        .map(|(coefficient, rate)| coefficient * rate)
        .sum::<f64>();
    if !unconstrained_lhs.is_finite() {
        return Err(VirtualImpedanceError::ArithmeticOverflow);
    }

    let mut dual_multiplier = 0.0_f64;
    let mut projection_active = false;
    if unconstrained_lhs > damping_power_rhs_w {
        projection_active = true;
        let mut upper_multiplier = 0.0_f64;
        for ((coefficient, reference_rate), (lower_rate, weight)) in coefficients
            .iter()
            .zip(reference.iter())
            .zip(lower.iter().zip(weights.iter()))
        {
            if *coefficient > 0.0 {
                let threshold = (*reference_rate - *lower_rate).max(0.0)
                    * *weight / *coefficient;
                if !threshold.is_finite() {
                    return Err(VirtualImpedanceError::ArithmeticOverflow);
                }
                upper_multiplier = upper_multiplier.max(threshold);
            }
        }
        let mut lower_multiplier = 0.0_f64;
        for _ in 0..80 {
            let midpoint = lower_multiplier.midpoint(upper_multiplier);
            let candidate = passivity_rate_candidate(
                &reference, &lower, &upper, &coefficients, &weights, midpoint,
            );
            let lhs = coefficients.iter().zip(candidate.iter())
                .map(|(coefficient, rate)| coefficient * rate)
                .sum::<f64>();
            if !lhs.is_finite() {
                return Err(VirtualImpedanceError::ArithmeticOverflow);
            }
            if lhs > damping_power_rhs_w {
                lower_multiplier = midpoint;
            } else {
                upper_multiplier = midpoint;
            }
        }
        dual_multiplier = upper_multiplier;
        accepted = passivity_rate_candidate(
            &reference, &lower, &upper, &coefficients, &weights, dual_multiplier,
        );
    }

    let passivity_lhs_w = coefficients.iter().zip(accepted.iter())
        .map(|(coefficient, rate)| coefficient * rate)
        .sum::<f64>();
    if !passivity_lhs_w.is_finite() || passivity_lhs_w > damping_power_rhs_w + 1.0e-9 {
        return Err(VirtualImpedanceError::ArithmeticOverflow);
    }

    let inertia_rate = core::array::from_fn(|index| accepted[index]);
    let stiffness_rate = core::array::from_fn(|index| accepted[index + 6]);

    Ok(DiagonalPassivityQpSelection {
        inertia_rate,
        stiffness_rate,
        passivity_lhs_w,
        damping_power_rhs_w,
        passivity_slack_w: damping_power_rhs_w - passivity_lhs_w,
        dual_multiplier,
        projection_active,
    })
}

fn passivity_rate_candidate(
    reference: &[f64; 12],
    lower: &[f64; 12],
    upper: &[f64; 12],
    coefficients: &[f64; 12],
    weights: &[f64; 12],
    multiplier: f64,
) -> [f64; 12] {
    core::array::from_fn(|index| {
        let unconstrained = reference[index]
            - multiplier * coefficients[index] / weights[index];
        unconstrained.clamp(lower[index], upper[index])
    })
}

/// A bounded proposed update for the diagonal virtual inertia/stiffness model.
///
/// Parameter order is [M0..M5, K0..K5]. Bounds are deltas and must already
/// encode rate limits plus absolute minimum/maximum parameter limits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualParameterUpdateRequest {
    pub old: DiagonalVirtualImpedance,
    pub desired_delta: [f64; 12],
    pub lower_delta: [f64; 12],
    pub upper_delta: [f64; 12],
    /// Positive diagonal weights in the quadratic deviation objective.
    pub objective_weights: [f64; 12],
    pub error: [f64; 6],
    pub relative_velocity: [f64; 6],
    /// Available energy budget for positive parameter-induced storage change.
    pub available_energy_budget_j: f64,
}

/// Fixed-size solution to the one-budget diagonal parameter projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualParameterUpdateSelection {
    pub accepted_delta: [f64; 12],
    pub parameter_energy_debit_j: f64,
    /// Reported but not credited against the budget.
    pub uncredited_storage_release_j: f64,
    /// Dual multiplier for the active positive-storage budget constraint.
    pub energy_budget_multiplier: f64,
    pub budget_constraint_active: bool,
}

/// Project a desired diagonal parameter update onto a conservative energy budget.
///
/// Solves the separable convex problem:
///
/// minimize 0.5 * sum_i w_i * (delta_i - desired_i)^2
/// subject to sum_i a_i * max(delta_i, 0) <= available_budget
///            lower_i <= delta_i <= upper_i
///
/// where a_i = 0.5*v_i^2 for virtual inertia and 0.5*e_i^2 for stiffness.
///
/// Negative parameter changes are allowed but are not used to fund positive
/// changes. The implementation uses the scalar KKT multiplier and 80 fixed
/// bisection iterations. It is allocation-free and handles 12 diagonal
/// parameters; it is not a general matrix-valued QP solver.
pub fn project_diagonal_virtual_parameter_update(
    request: VirtualParameterUpdateRequest,
) -> Result<VirtualParameterUpdateSelection, VirtualImpedanceError> {
    let mut coefficients = [0.0_f64; 12];
    for (index, (((mass, stiffness), error), velocity)) in request.old.virtual_inertia.iter()
        .zip(request.old.stiffness.iter())
        .zip(request.error.iter())
        .zip(request.relative_velocity.iter())
        .enumerate()
    {
        let mass = *mass;
        let stiffness = *stiffness;
        let error = *error;
        let velocity = *velocity;

        if !mass.is_finite() || mass <= 0.0 {
            return Err(VirtualImpedanceError::NonPositiveVirtualInertia);
        }
        if !stiffness.is_finite() || stiffness < 0.0 {
            return Err(VirtualImpedanceError::NegativeStiffness);
        }
        if !error.is_finite() || !velocity.is_finite() {
            return Err(VirtualImpedanceError::NonFiniteErrorOrVelocity);
        }
        coefficients[index] = 0.5 * velocity * velocity;
        coefficients[index + 6] = 0.5 * error * error;
        if !coefficients[index].is_finite() || !coefficients[index + 6].is_finite() {
            return Err(VirtualImpedanceError::ArithmeticOverflow);
        }
    }

    if !request.available_energy_budget_j.is_finite()
        || request.available_energy_budget_j < 0.0
    {
        return Err(VirtualImpedanceError::InvalidEnergyBudget);
    }

    let mut minimum_debit = 0.0_f64;
    let mut maximum_multiplier = 0.0_f64;
    let mut baseline = [0.0_f64; 12];

    for (index, baseline_slot) in baseline.iter_mut().enumerate() {
        let desired = request.desired_delta[index];
        let lower = request.lower_delta[index];
        let upper = request.upper_delta[index];
        let weight = request.objective_weights[index];
        let coefficient = coefficients[index];

        if !desired.is_finite() || !lower.is_finite() || !upper.is_finite() {
            return Err(VirtualImpedanceError::NonFiniteParameter);
        }
        if !weight.is_finite() || weight <= 0.0 {
            return Err(VirtualImpedanceError::InvalidUpdateWeight);
        }
        if lower > upper {
            return Err(VirtualImpedanceError::InvalidUpdateBounds);
        }

        if index < 6 {
            let old = request.old.virtual_inertia[index];
            if !((old + lower).is_finite() && (old + upper).is_finite()) || old + lower <= 0.0 {
                return Err(VirtualImpedanceError::InvalidUpdateBounds);
            }
        } else {
            let old = request.old.stiffness[index - 6];
            if !((old + lower).is_finite() && (old + upper).is_finite()) || old + lower < 0.0 {
                return Err(VirtualImpedanceError::InvalidUpdateBounds);
            }
        }

        let nonnegative_floor = lower.max(0.0);
        minimum_debit += coefficient * nonnegative_floor;
        if !minimum_debit.is_finite() {
            return Err(VirtualImpedanceError::ArithmeticOverflow);
        }

        // The unconstrained minimizer is clipped to the box. For a parameter
        // whose proposed change is non-positive and whose lower bound permits
        // it, keep the negative/zero update independent of positive-energy
        // budgeting; released storage is not converted into extra budget.
        *baseline_slot = if upper < 0.0 || (desired <= 0.0 && lower <= 0.0) {
            desired.clamp(lower, upper.min(0.0))
        } else {
            desired.clamp(nonnegative_floor, upper)
        };

        if coefficient > 0.0 && *baseline_slot > nonnegative_floor {
            let threshold = (desired - nonnegative_floor).max(0.0) * weight / coefficient;
            if !threshold.is_finite() {
                return Err(VirtualImpedanceError::ArithmeticOverflow);
            }
            maximum_multiplier = maximum_multiplier.max(threshold);
        }
    }

    if minimum_debit > request.available_energy_budget_j {
        return Err(VirtualImpedanceError::EnergyBudgetInfeasible);
    }

    let baseline_debit = parameter_update_debit(&baseline, &coefficients);
    if !baseline_debit.is_finite() {
        return Err(VirtualImpedanceError::ArithmeticOverflow);
    }
    if baseline_debit <= request.available_energy_budget_j {
        let released = parameter_update_uncredited_release(&baseline, &coefficients);
        if !released.is_finite() {
            return Err(VirtualImpedanceError::ArithmeticOverflow);
        }
        return Ok(VirtualParameterUpdateSelection {
            accepted_delta: baseline,
            parameter_energy_debit_j: baseline_debit,
            uncredited_storage_release_j: released,
            energy_budget_multiplier: 0.0,
            budget_constraint_active: false,
        });
    }

    let mut lower_multiplier = 0.0_f64;
    let mut upper_multiplier = maximum_multiplier;
    for _ in 0..80 {
        let midpoint = lower_multiplier.midpoint(upper_multiplier);
        let candidate = parameter_update_at_multiplier(&request, &coefficients, midpoint);
        let debit = parameter_update_debit(&candidate, &coefficients);
        if debit > request.available_energy_budget_j {
            lower_multiplier = midpoint;
        } else {
            upper_multiplier = midpoint;
        }
    }

    let accepted_delta =
        parameter_update_at_multiplier(&request, &coefficients, upper_multiplier);
    let parameter_energy_debit_j = parameter_update_debit(&accepted_delta, &coefficients);
    let uncredited_storage_release_j =
        parameter_update_uncredited_release(&accepted_delta, &coefficients);
    if !parameter_energy_debit_j.is_finite() || !uncredited_storage_release_j.is_finite() {
        return Err(VirtualImpedanceError::ArithmeticOverflow);
    }
    if parameter_energy_debit_j > request.available_energy_budget_j + 1.0e-10 {
        return Err(VirtualImpedanceError::ArithmeticOverflow);
    }

    Ok(VirtualParameterUpdateSelection {
        accepted_delta,
        parameter_energy_debit_j,
        uncredited_storage_release_j,
        energy_budget_multiplier: upper_multiplier,
        budget_constraint_active: true,
    })
}

fn parameter_update_at_multiplier(
    request: &VirtualParameterUpdateRequest,
    coefficients: &[f64; 12],
    multiplier: f64,
) -> [f64; 12] {
    core::array::from_fn(|index| {
        let desired = request.desired_delta[index];
        let lower = request.lower_delta[index];
        let upper = request.upper_delta[index];
        let coefficient = coefficients[index];

        if upper < 0.0 || (desired <= 0.0 && lower <= 0.0) {
            desired.clamp(lower, upper.min(0.0))
        } else {
            let weight = request.objective_weights[index];
            let floor = lower.max(0.0);
            let projected = desired - multiplier * coefficient / weight;
            projected.clamp(floor, upper)
        }
    })
}

fn parameter_update_debit(delta: &[f64; 12], coefficients: &[f64; 12]) -> f64 {
    delta.iter().zip(coefficients.iter())
        .map(|(delta, coefficient)| coefficient * delta.max(0.0))
        .sum()
}

fn parameter_update_uncredited_release(
    delta: &[f64; 12],
    coefficients: &[f64; 12],
) -> f64 {
    delta.iter().zip(coefficients.iter())
        .map(|(delta, coefficient)| coefficient * (-delta).max(0.0))
        .sum()
}

/// Invalid diagonal virtual impedance data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualImpedanceError {
    NonFiniteErrorOrVelocity,
    NonPositiveVirtualInertia,
    NegativeStiffness,
    NonFiniteParameter,
    InvalidEnergyBudget,
    InvalidUpdateWeight,
    InvalidUpdateBounds,
    EnergyBudgetInfeasible,
    ArithmeticOverflow,
}

impl fmt::Display for VirtualImpedanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for VirtualImpedanceError {}

/// Compute the exact change in the diagonal quadratic storage approximation:
///
/// H = 0.5 * sum(M_i * velocity_i^2 + K_i * error_i^2).
///
/// This isolates the energy change due to parameter updates at fixed state.
/// It does not prove passivity of the complete robot/controller, handle
/// off-diagonal coupling, derive the allowed parameter-rate set, or solve a
/// parameter-optimization QP.
///
/// A positive delta is conservatively treated as an energy-budget debit.
/// A negative delta is reported but is not automatically credited because
/// released virtual storage may be converted into physical motion.
pub fn diagonal_virtual_parameter_storage_delta(
    old: DiagonalVirtualImpedance,
    new: DiagonalVirtualImpedance,
    error: [f64; 6],
    relative_velocity: [f64; 6],
) -> Result<VirtualParameterStorageDelta, VirtualImpedanceError> {
    let mut delta = 0.0_f64;
    let mut required_energy_debit_j = 0.0_f64;
    let mut uncredited_storage_release_j = 0.0_f64;
    for (((((old_mass, new_mass), old_stiffness), new_stiffness), e), velocity) in old.virtual_inertia.iter()
        .zip(new.virtual_inertia.iter())
        .zip(old.stiffness.iter())
        .zip(new.stiffness.iter())
        .zip(error.iter())
        .zip(relative_velocity.iter())
    {
        let e = *e;
        let velocity = *velocity;
        let old_mass = *old_mass;
        let new_mass = *new_mass;
        let old_stiffness = *old_stiffness;
        let new_stiffness = *new_stiffness;

        if !e.is_finite() || !velocity.is_finite() {
            return Err(VirtualImpedanceError::NonFiniteErrorOrVelocity);
        }
        if !old_mass.is_finite()
            || !new_mass.is_finite()
            || !old_stiffness.is_finite()
            || !new_stiffness.is_finite()
        {
            return Err(VirtualImpedanceError::NonFiniteParameter);
        }
        if old_mass <= 0.0 || new_mass <= 0.0 {
            return Err(VirtualImpedanceError::NonPositiveVirtualInertia);
        }
        if old_stiffness < 0.0 || new_stiffness < 0.0 {
            return Err(VirtualImpedanceError::NegativeStiffness);
        }

        let inertia_term = 0.5 * (new_mass - old_mass) * velocity * velocity;
        let stiffness_term = 0.5 * (new_stiffness - old_stiffness) * e * e;
        delta += inertia_term + stiffness_term;
        required_energy_debit_j += inertia_term.max(0.0) + stiffness_term.max(0.0);
        uncredited_storage_release_j += (-inertia_term).max(0.0) + (-stiffness_term).max(0.0);
        if !delta.is_finite()
            || !required_energy_debit_j.is_finite()
            || !uncredited_storage_release_j.is_finite()
        {
            return Err(VirtualImpedanceError::ArithmeticOverflow);
        }
    }

    Ok(VirtualParameterStorageDelta {
        delta_storage_j: delta,
        required_energy_debit_j,
        uncredited_storage_release_j,
    })
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

    const PORT_ID: u64 = 5;
    const FRAME_ID: u64 = 99;
    const PROFILE_DIGEST: [u8; 32] = [0x44; 32];
    const MAX_GAP_NS: u64 = 2_000_000_000;

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
            port_id: PORT_ID,
            frame_id: FRAME_ID,
            measurement_profile_digest: PROFILE_DIGEST,
            wrench_environment_on_robot: [force_x_n, 0.0, 0.0, 0.0, 0.0, 0.0],
            twist_robot: [velocity_x_mps, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    fn observer(
        initial_energy_budget_j: f64,
        uncertainty_j: f64,
        tolerance_j: f64,
    ) -> PassivityObserver {
        PassivityObserver::new(
            clock(),
            PassivityObserverConfig {
                port_id: PORT_ID,
                frame_id: FRAME_ID,
                measurement_profile_digest: PROFILE_DIGEST,
                max_sample_gap_ns: MAX_GAP_NS,
                initial_energy_budget_j,
                energy_uncertainty_j: uncertainty_j,
                violation_tolerance_j: tolerance_j,
            },
        )
        .unwrap()
    }

    #[test]
    fn increasing_virtual_stiffness_at_fixed_error_is_charged_to_energy_budget() {
        let old = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [100.0; 6],
        };
        let mut new = old;
        new.stiffness[0] = 200.0;
        let result = diagonal_virtual_parameter_storage_delta(
            old, new, [0.5, 0.0, 0.0, 0.0, 0.0, 0.0], [0.0; 6],
        )
        .unwrap();

        assert!((result.delta_storage_j - 12.5).abs() < 1.0e-12);
        assert!((result.required_energy_debit_j - 12.5).abs() < 1.0e-12);
        assert!(result.uncredited_storage_release_j.abs() < 1.0e-12);
    }

    #[test]
    fn reducing_virtual_stiffness_reports_release_without_automatic_credit() {
        let old = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [200.0; 6],
        };
        let mut new = old;
        new.stiffness[0] = 100.0;
        let result = diagonal_virtual_parameter_storage_delta(
            old, new, [0.5, 0.0, 0.0, 0.0, 0.0, 0.0], [0.0; 6],
        )
        .unwrap();

        assert!((result.delta_storage_j + 12.5).abs() < 1.0e-12);
        assert!(result.required_energy_debit_j.abs() < 1.0e-12);
        assert!((result.uncredited_storage_release_j - 12.5).abs() < 1.0e-12);
    }

    #[test]
    fn increasing_virtual_inertia_at_fixed_velocity_is_charged() {
        let old = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [100.0; 6],
        };
        let mut new = old;
        new.virtual_inertia[3] = 3.0;
        let result = diagonal_virtual_parameter_storage_delta(
            old, new, [0.0; 6], [0.0, 0.0, 0.0, 2.0, 0.0, 0.0],
        )
        .unwrap();

        assert!((result.required_energy_debit_j - 4.0).abs() < 1.0e-12);
    }

    #[test]
    fn simultaneous_diagonal_mass_and_stiffness_update_has_exact_storage_delta() {
        let old = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [100.0; 6],
        };
        let mut new = old;
        new.virtual_inertia[0] = 2.0;
        new.stiffness[1] = 300.0;
        let result = diagonal_virtual_parameter_storage_delta(
            old, new, [0.2, 0.5, 0.0, 0.0, 0.0, 0.0], [3.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        )
        .unwrap();

        let expected = 0.5 * (1.0 * 9.0 + 200.0 * 0.25);
        assert!((result.delta_storage_j - expected).abs() < 1.0e-12);
        assert!((result.required_energy_debit_j - expected).abs() < 1.0e-12);
    }

    #[test]
    fn positive_parameter_storage_increase_is_not_funded_by_another_axis_release() {
        let old = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [200.0, 100.0, 100.0, 100.0, 100.0, 100.0],
        };
        let mut new = old;
        new.stiffness[0] = 100.0;
        new.stiffness[1] = 200.0;
        let result = diagonal_virtual_parameter_storage_delta(
            old,
            new,
            [0.5, 0.5, 0.0, 0.0, 0.0, 0.0],
            [0.0; 6],
        )
        .unwrap();

        assert!(result.delta_storage_j.abs() < 1.0e-12);
        assert!((result.required_energy_debit_j - 12.5).abs() < 1.0e-12);
        assert!((result.uncredited_storage_release_j - 12.5).abs() < 1.0e-12);
    }

    #[test]
    fn parameter_projection_rejects_overflowing_energy_coefficients() {
        let old = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [100.0; 6],
        };
        assert_eq!(
            project_diagonal_virtual_parameter_update(update_request(
                old,
                [0.0; 12],
                [0.0; 12],
                [1.0; 12],
                [f64::MAX, 0.0, 0.0, 0.0, 0.0, 0.0],
                [0.0; 6],
                1.0,
            )),
            Err(VirtualImpedanceError::ArithmeticOverflow)
        );
    }

    #[test]
    fn virtual_impedance_rejects_invalid_inertia_stiffness_and_state() {
        let nominal = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [100.0; 6],
        };
        let mut invalid = nominal;
        invalid.virtual_inertia[2] = 0.0;
        assert_eq!(
            diagonal_virtual_parameter_storage_delta(nominal, invalid, [0.0; 6], [0.0; 6]),
            Err(VirtualImpedanceError::NonPositiveVirtualInertia)
        );

        invalid = nominal;
        invalid.stiffness[2] = -1.0;
        assert_eq!(
            diagonal_virtual_parameter_storage_delta(nominal, invalid, [0.0; 6], [0.0; 6]),
            Err(VirtualImpedanceError::NegativeStiffness)
        );

        assert_eq!(
            diagonal_virtual_parameter_storage_delta(nominal, nominal, [f64::NAN; 6], [0.0; 6]),
            Err(VirtualImpedanceError::NonFiniteErrorOrVelocity)
        );
    }

    fn update_request(
        old: DiagonalVirtualImpedance,
        desired_delta: [f64; 12],
        lower_delta: [f64; 12],
        upper_delta: [f64; 12],
        error: [f64; 6],
        velocity: [f64; 6],
        budget_j: f64,
    ) -> VirtualParameterUpdateRequest {
        VirtualParameterUpdateRequest {
            old,
            desired_delta,
            lower_delta,
            upper_delta,
            objective_weights: [1.0; 12],
            error,
            relative_velocity: velocity,
            available_energy_budget_j: budget_j,
        }
    }

    fn passivity_qp_request(
        reference_alpha_rate: [f64; 6],
        reference_stiffness_rate: [f64; 6],
        alpha_rate_max: [f64; 6],
        stiffness_rate_max: [f64; 6],
        error: [f64; 6],
        velocity: [f64; 6],
    ) -> DiagonalPassivityQpRequest {
        DiagonalPassivityQpRequest {
            base_inertia: [2.0; 6],
            inertia_scale: [1.0; 6],
            minimum_inertia_scale: [0.5; 6],
            stiffness: [4.0; 6],
            error,
            error_velocity: velocity,
            inertia_decay_rate: [1.0; 6],
            stiffness_decay_rate: [1.0; 6],
            inertia_reference_rate,
            stiffness_reference_rate,
            inertia_max_rate: alpha_rate_max,
            stiffness_max_rate: stiffness_rate_max,
            objective_weights: [1.0; 12],
        }
    }

    #[test]
    fn published_diagonal_vic_qp_projects_rates_onto_passivity_halfspace() {
        // Only axis 0 is active:
        // a_alpha = 0.5 * 1^2 * 2 = 1
        // a_K = 0.5 * 2^2 = 2
        // D = 2 * sqrt(4 * 1 * 2) = 5.656854...
        // Projection of (4,4) onto alpha_dot + 2*K_dot <= D.
        let mut request = passivity_qp_request(
            [4.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [4.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [10.0; 6],
            [10.0; 6],
            [2.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        );
        request.base_inertia[0] = 2.0;
        let result = solve_diagonal_passivity_rate_qp(request).unwrap();
        let rhs = 2.0 * (4.0_f64 * 1.0 * 2.0).sqrt();
        let excess = 12.0 - rhs;
        let lambda = excess / 5.0;

        assert!(result.projection_active);
        assert!((result.inertia_rate[0] - (4.0 - lambda)).abs() < 1.0e-8);
        assert!((result.stiffness_rate[0] - (4.0 - 2.0 * lambda)).abs() < 1.0e-8);
        assert!(result.passivity_lhs_w <= result.damping_power_rhs_w + 1.0e-9);
        assert!(result.passivity_slack_w.abs() < 1.0e-8);
    }

    #[test]
    fn published_diagonal_vic_qp_leaves_already_passive_reference_unchanged() {
        let result = solve_diagonal_passivity_rate_qp(passivity_qp_request(
            [0.1; 6],
            [0.1; 6],
            [1.0; 6],
            [1.0; 6],
            [0.0; 6],
            [0.0; 6],
        ))
        .unwrap();

        assert!(!result.projection_active);
        assert_eq!(result.inertia_rate, [0.1; 6]);
        assert_eq!(result.stiffness_rate, [0.1; 6]);
        assert!(result.passivity_slack_w.abs() < 1.0e-12);
    }

    #[test]
    fn published_diagonal_vic_qp_enforces_parameter_decay_barriers() {
        let mut request = passivity_qp_request(
            [0.0; 6],
            [0.0; 6],
            [1.0; 6],
            [1.0; 6],
            [0.0; 6],
            [0.0; 6],
        );
        request.inertia_scale[0] = 0.75;
        request.minimum_inertia_scale[0] = 0.5;
        request.inertia_decay_rate[0] = 2.0;
        request.stiffness[1] = 3.0;
        request.stiffness_decay_rate[1] = 0.5;

        let result = solve_diagonal_passivity_rate_qp(request).unwrap();
        assert!(result.inertia_rate[0] >= -0.5);
        assert!(result.stiffness_rate[1] >= -1.5);
    }

    #[test]
    fn published_diagonal_vic_qp_rejects_invalid_state_and_infeasible_bounds() {
        let mut request = passivity_qp_request(
            [0.0; 6], [0.0; 6], [1.0; 6], [1.0; 6], [0.0; 6], [0.0; 6],
        );
        request.inertia_scale[0] = 0.0;
        assert_eq!(
            solve_diagonal_passivity_rate_qp(request),
            Err(VirtualImpedanceError::NonPositiveVirtualInertia)
        );

        request = passivity_qp_request(
            [0.0; 6], [0.0; 6], [-1.0; 6], [1.0; 6], [0.0; 6], [0.0; 6],
        );
        assert_eq!(
            solve_diagonal_passivity_rate_qp(request),
            Err(VirtualImpedanceError::InvalidUpdateBounds)
        );
    }

    #[test]
    fn parameter_projection_meets_exact_energy_budget_without_exceeding() {
        let old = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [100.0; 6],
        };
        let mut desired = [0.0; 12];
        desired[6] = 100.0;
        let mut upper = [200.0; 12];
        upper[6] = 50.0;
        let request = update_request(
            old, desired, [0.0; 12], upper,
            [0.5, 0.0, 0.0, 0.0, 0.0, 0.0], [0.0; 6], 5.0,
        );
        let result = project_diagonal_virtual_parameter_update(request).unwrap();

        assert!((result.accepted_delta[6] - 40.0).abs() < 1.0e-8);
        assert!((result.parameter_energy_debit_j - 5.0).abs() < 1.0e-8);
        assert!(result.budget_constraint_active);
        assert!(result.energy_budget_multiplier > 0.0);
    }

    #[test]
    fn negative_parameter_changes_do_not_fund_positive_parameter_changes() {
        let old = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [200.0; 6],
        };
        let mut desired = [0.0; 12];
        desired[6] = -100.0;
        desired[7] = 100.0;
        let mut lower = [-100.0; 12];
        lower[6] = -200.0;
        let mut upper = [0.0; 12];
        upper[7] = 200.0;
        let result = project_diagonal_virtual_parameter_update(update_request(
            old, desired, lower, upper,
            [0.5, 0.5, 0.0, 0.0, 0.0, 0.0], [0.0; 6], 5.0,
        ))
        .unwrap();

        assert!((result.accepted_delta[6] + 100.0).abs() < 1.0e-8);
        assert!((result.accepted_delta[7] - 40.0).abs() < 1.0e-8);
        assert!((result.parameter_energy_debit_j - 5.0).abs() < 1.0e-8);
        assert!((result.uncredited_storage_release_j - 12.5).abs() < 1.0e-8);
    }

    #[test]
    fn parameter_projection_rejects_infeasible_positive_lower_bounds() {
        let old = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [100.0; 6],
        };
        let mut lower = [0.0; 12];
        lower[6] = 50.0;
        assert_eq!(
            project_diagonal_virtual_parameter_update(update_request(
                old, [100.0; 12], lower, [100.0; 12],
                [0.5, 0.0, 0.0, 0.0, 0.0, 0.0], [0.0; 6], 5.0,
            )),
            Err(VirtualImpedanceError::EnergyBudgetInfeasible)
        );
    }

    #[test]
    fn parameter_projection_preserves_box_and_physical_parameter_bounds() {
        let old = DiagonalVirtualImpedance {
            virtual_inertia: [1.0; 6],
            stiffness: [100.0; 6],
        };
        let mut desired = [0.0; 12];
        desired[0] = -2.0;
        desired[6] = -200.0;
        let mut lower = [-0.5; 12];
        lower[0] = -0.9;
        lower[6] = -100.0;
        let upper = [0.5; 12];
        let result = project_diagonal_virtual_parameter_update(update_request(
            old, desired, lower, upper,
            [0.0; 6], [0.0; 6], 0.0,
        ))
        .unwrap();

        assert!(result.accepted_delta[0] > -1.0);
        assert!(result.accepted_delta[6] >= -100.0);
        assert!(result.parameter_energy_debit_j.abs() < 1.0e-12);
    }

    #[test]
    fn positive_environment_work_replenishes_storage() {
        let mut subject = observer(0.5, 0.0, 1.0e-12);
        subject.observe(sample(1, 0, 10.0, 0.1)).unwrap();
        let result = subject.observe(sample(2, 1_000_000_000, 10.0, 0.1)).unwrap();

        assert!((result.cumulative_port_work_j - 1.0).abs() < 1.0e-12);
        assert!((result.conservative_storage_lower_bound_j - 1.5).abs() < 1.0e-12);
        assert!(result.passivity_deficit_j.abs() < 1.0e-12);
        assert_eq!(result.violation_interval_count, 0);
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
        assert_eq!(result.violation_interval_count, 1);
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
    fn interior_negative_power_minimum_is_not_hidden_by_zero_net_interval_work() {
        let mut subject = observer(0.1, 0.0, 1.0e-12);
        subject.observe(sample(1, 0, -10.0, 0.1)).unwrap(); // -1 W
        let result = subject.observe(sample(2, 2_000_000_000, 10.0, 0.1)).unwrap(); // +1 W

        // Net trapezoidal work is zero, but the power crosses zero at 1 s and
        // drains 0.5 J before replenishing it.
        assert!(result.cumulative_port_work_j.abs() < 1.0e-12);
        assert!(result.passivity_deficit_j.abs() < 1.0e-12);
        assert!((result.maximum_passivity_deficit_j - 0.4).abs() < 1.0e-12);
        assert_eq!(result.violation_interval_count, 1);
    }

    #[test]
    fn scaled_zero_crossing_handles_extreme_opposite_signed_port_powers() {
        let mut subject = observer(1.0e307, 0.0, 1.0e306);
        subject.observe(sample(1, 0, -1.0e308, 1.0)).unwrap();
        let result = subject.observe(sample(2, 2_000_000_000, 1.0e308, 1.0)).unwrap();

        assert!(result.cumulative_port_work_j.abs() < 1.0e-12);
        assert!((result.maximum_passivity_deficit_j / 1.0e307 - 4.0).abs() < 1.0e-12);
        assert_eq!(result.violation_interval_count, 1);
    }

    #[test]
    fn excessive_sample_gap_is_unknown_and_does_not_mutate_state() {
        let mut subject = observer(1.0, 0.0, 0.0);
        subject.observe(sample(1, 0, 0.0, 0.0)).unwrap();
        let before = subject.snapshot();
        assert_eq!(
            subject.observe(sample(2, MAX_GAP_NS + 1, 0.0, 0.0)),
            Err(PassivityError::SampleGapExceeded)
        );
        assert_eq!(subject.snapshot(), before);
    }

    #[test]
    fn wrong_port_and_measurement_profile_are_rejected() {
        let mut subject = observer(1.0, 0.0, 0.0);
        subject.observe(sample(1, 10, 0.0, 0.0)).unwrap();
        let mut wrong_port = sample(2, 20, 0.0, 0.0);
        wrong_port.port_id += 1;
        assert_eq!(subject.observe(wrong_port), Err(PassivityError::PortMismatch));

        let mut wrong_profile = sample(2, 20, 0.0, 0.0);
        wrong_profile.measurement_profile_digest[0] ^= 0xff;
        assert_eq!(
            subject.observe(wrong_profile),
            Err(PassivityError::MeasurementProfileMismatch)
        );
    }

    #[test]
    fn zero_profile_digest_and_zero_maximum_gap_are_rejected() {
        assert!(matches!(
            PassivityObserver::new(clock(), PassivityObserverConfig { port_id: PORT_ID, frame_id: FRAME_ID, measurement_profile_digest: [0; 32], max_sample_gap_ns: MAX_GAP_NS, initial_energy_budget_j: 1.0, energy_uncertainty_j: 0.0, violation_tolerance_j: 0.0 }),
            Err(PassivityError::InvalidMeasurementProfileDigest)
        ));
        assert!(matches!(
            PassivityObserver::new(clock(), PassivityObserverConfig { port_id: PORT_ID, frame_id: FRAME_ID, measurement_profile_digest: PROFILE_DIGEST, max_sample_gap_ns: 0, initial_energy_budget_j: 1.0, energy_uncertainty_j: 0.0, violation_tolerance_j: 0.0 }),
            Err(PassivityError::InvalidMaximumSampleGap)
        ));
    }

    #[test]
    fn storage_uncertainty_and_tolerance_are_validated() {
        assert!(matches!(
            PassivityObserver::new(clock(), PassivityObserverConfig { port_id: PORT_ID, frame_id: FRAME_ID, measurement_profile_digest: PROFILE_DIGEST, max_sample_gap_ns: MAX_GAP_NS, initial_energy_budget_j: -1.0, energy_uncertainty_j: 0.0, violation_tolerance_j: 0.0 }),
            Err(PassivityError::InvalidInitialEnergyBudget)
        ));
        assert!(matches!(
            PassivityObserver::new(clock(), PassivityObserverConfig { port_id: PORT_ID, frame_id: FRAME_ID, measurement_profile_digest: PROFILE_DIGEST, max_sample_gap_ns: MAX_GAP_NS, initial_energy_budget_j: 0.0, energy_uncertainty_j: -0.1, violation_tolerance_j: 0.0 }),
            Err(PassivityError::InvalidEnergyUncertainty)
        ));
        assert!(matches!(
            PassivityObserver::new(clock(), PassivityObserverConfig { port_id: PORT_ID, frame_id: FRAME_ID, measurement_profile_digest: PROFILE_DIGEST, max_sample_gap_ns: MAX_GAP_NS, initial_energy_budget_j: 0.0, energy_uncertainty_j: 0.0, violation_tolerance_j: f64::NAN }),
            Err(PassivityError::InvalidTolerance)
        ));
    }

    #[test]
    fn deficit_at_or_below_tolerance_is_not_counted_as_violation() {
        let mut subject = observer(0.5, 0.0, 0.5);
        subject.observe(sample(1, 0, -10.0, 0.1)).unwrap();
        let result = subject.observe(sample(2, 1_000_000_000, -10.0, 0.1)).unwrap();

        assert!((result.passivity_deficit_j - 0.5).abs() < 1.0e-12);
        assert_eq!(result.violation_interval_count, 0);
    }
}
