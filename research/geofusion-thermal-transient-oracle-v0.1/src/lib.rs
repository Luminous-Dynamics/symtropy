#![forbid(unsafe_code)]

//! Closed-form reference solutions for a lumped thermal buffer.
//!
//! This is a synthetic benchmark oracle. It is not a physical geothermal or
//! heat-exchanger model, an FMI master, or an energy-accounting authority.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelError {
    NonFiniteParameter,
    NonPositiveHeatCapacity,
    NonPositiveConductance,
    ServiceFractionOutOfRange,
    NonPositiveInterval,
    NegativeStartTime,
    EventInPast,
    NonPositiveFrequency,
    NegativeSourcePowerEnvelope,
    NegativeCumulativeEnergy,
    CumulativeEnergyRegressed,
    NonPositiveSampleCount,
    NumericalOverflow,
}

/// Inputs to a multi-signal adaptive communication-step proposal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveStepInput {
    pub current_time_s: f64,
    pub attempted_step_s: f64,
    pub minimum_step_s: f64,
    pub maximum_step_s: f64,
    pub trajectory_error: f64,
    pub trajectory_tolerance: f64,
    pub interface_residual: f64,
    pub interface_tolerance: f64,
    pub energy_residual: f64,
    pub energy_tolerance: f64,
    /// Formal order used by the local error controller (p >= 1).
    pub method_order: u32,
    /// Earliest known event/clock time. None means no currently known fence.
    pub next_event_time_s: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveStepAction {
    Accept,
    RejectAndRetry,
    ProcessEventNow,
    FailClosedAtMinimumStep,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveStepDecision {
    pub action: AdaptiveStepAction,
    pub attempted_step_s: f64,
    /// Proposed next duration. Zero only when an event is due at current time.
    pub next_step_s: f64,
    /// max(|trajectory_error|/tol, |interface_residual|/tol, |energy_residual|/tol).
    pub normalized_error: f64,
    pub event_fence_applied: bool,
    pub event_time_s: Option<f64>,
    /// An exact known event may require a step smaller than the configured minimum.
    pub event_step_below_minimum: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalModel {
    pub heat_capacity_j_per_k: f64,
    pub loss_conductance_w_per_k: f64,
    pub service_fraction: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyBalance {
    pub start_time_s: f64,
    pub end_time_s: f64,
    pub theta_start_k: f64,
    pub theta_end_k: f64,
    pub energy_input_j: f64,
    pub energy_service_j: f64,
    pub energy_loss_j: f64,
    pub storage_delta_j: f64,
    pub residual_j: f64,
}

/// Result and diagnostics for a causal, bounded source extrapolation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CausalPredictorResult {
    /// Exact receiver response to the piecewise-constant exchanged predictor values.
    pub balance: EnergyBalance,
    /// Number of predictor values projected onto the declared source-power envelope.
    pub limiter_activations: usize,
    /// Largest absolute difference between the unbounded and exchanged predictor (W).
    pub max_unbounded_overshoot_w: f64,
}

/// Propose a communication interval from numerical, interface and physical error.
///
/// The controller uses the maximum normalized error across three independent
/// gates. It does not trade an energy violation against a smaller trajectory
/// error. A rejected attempt shrinks the next interval; an accepted attempt
/// may grow it by at most 2x. The final proposal is fenced at the earliest
/// known event/clock tick. If the event is due now, the caller must process
/// that event rather than take a zero-length doStep.
///
/// This function is a deterministic policy primitive, not an FMI master:
/// it does not execute an FMU, manufacture an error estimate, or infer an
/// unknown future event.
pub fn propose_adaptive_step(
    input: AdaptiveStepInput,
) -> Result<AdaptiveStepDecision, ModelError> {
    let finite_values = [
        input.current_time_s,
        input.attempted_step_s,
        input.minimum_step_s,
        input.maximum_step_s,
        input.trajectory_error,
        input.trajectory_tolerance,
        input.interface_residual,
        input.interface_tolerance,
        input.energy_residual,
        input.energy_tolerance,
    ];
    if !finite_values.iter().all(|value| value.is_finite()) {
        return Err(ModelError::NonFiniteParameter);
    }
    if input.current_time_s < 0.0
        || input.attempted_step_s <= 0.0
        || input.minimum_step_s <= 0.0
        || input.maximum_step_s < input.minimum_step_s
        || input.attempted_step_s < input.minimum_step_s
        || input.attempted_step_s > input.maximum_step_s
        || input.trajectory_tolerance <= 0.0
        || input.interface_tolerance <= 0.0
        || input.energy_tolerance <= 0.0
        || input.method_order == 0
    {
        return Err(ModelError::NonPositiveInterval);
    }

    let normalized_error = (input.trajectory_error.abs() / input.trajectory_tolerance)
        .max(input.interface_residual.abs() / input.interface_tolerance)
        .max(input.energy_residual.abs() / input.energy_tolerance);
    if !normalized_error.is_finite() {
        return Err(ModelError::NumericalOverflow);
    }

    if let Some(event_time) = input.next_event_time_s {
        if !event_time.is_finite() {
            return Err(ModelError::NonFiniteParameter);
        }
        if event_time < input.current_time_s {
            return Err(ModelError::EventInPast);
        }
        if event_time == input.current_time_s {
            return Ok(AdaptiveStepDecision {
                action: AdaptiveStepAction::ProcessEventNow,
                attempted_step_s: input.attempted_step_s,
                next_step_s: 0.0,
                normalized_error,
                event_fence_applied: true,
                event_time_s: Some(event_time),
                event_step_below_minimum: false,
            });
        }
    }

    let (action, factor) = if normalized_error > 1.0 {
        let factor = (0.9 * (1.0 / normalized_error).powf(1.0 / (input.method_order as f64 + 1.0)))
            .clamp(0.1, 0.9);
        (AdaptiveStepAction::RejectAndRetry, factor)
    } else {
        let factor = if normalized_error == 0.0 {
            2.0
        } else {
            (0.9 * (1.0 / normalized_error).powf(1.0 / (input.method_order as f64 + 1.0)))
                .clamp(1.0, 2.0)
        };
        (AdaptiveStepAction::Accept, factor)
    };

    let candidate = (input.attempted_step_s * factor).min(input.maximum_step_s);
    let mut next_step_s = candidate;
    let mut event_fence_applied = false;
    let mut event_time_s = None;
    if let Some(event_time) = input.next_event_time_s {
        let time_to_event = event_time - input.current_time_s;
        if time_to_event <= next_step_s {
            next_step_s = time_to_event;
            event_fence_applied = true;
            event_time_s = Some(event_time);
        }
    }

    let event_step_below_minimum = event_fence_applied && next_step_s < input.minimum_step_s;
    let final_action = if next_step_s < input.minimum_step_s
        && action == AdaptiveStepAction::RejectAndRetry
    {
        AdaptiveStepAction::FailClosedAtMinimumStep
    } else {
        action
    };

    Ok(AdaptiveStepDecision {
        action: final_action,
        attempted_step_s: input.attempted_step_s,
        next_step_s,
        normalized_error,
        event_fence_applied,
        event_time_s,
        event_step_below_minimum,
    })
}

/// Derive a non-negative interval-energy increment from two committed cumulative readings.
///
/// This function performs no interpolation or prediction. A decrease is
/// treated as an unobserved reset/restart and rejected; callers must establish
/// a new counter epoch explicitly before accepting further deltas.
pub fn cumulative_energy_delta_j(
    previous_cumulative_j: f64,
    current_cumulative_j: f64,
) -> Result<f64, ModelError> {
    if !previous_cumulative_j.is_finite() || !current_cumulative_j.is_finite() {
        return Err(ModelError::NonFiniteParameter);
    }
    if previous_cumulative_j < 0.0 || current_cumulative_j < 0.0 {
        return Err(ModelError::NegativeCumulativeEnergy);
    }
    if current_cumulative_j < previous_cumulative_j {
        return Err(ModelError::CumulativeEnergyRegressed);
    }
    let delta = current_cumulative_j - previous_cumulative_j;
    if !delta.is_finite() {
        return Err(ModelError::NumericalOverflow);
    }
    Ok(delta)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SinusoidalInput {
    pub mean_power_w: f64,
    pub amplitude_w: f64,
    pub angular_frequency_rad_per_s: f64,
    pub initial_theta_k: f64,
}

/// Compensated accumulator used for interval-integrated energy totals.
#[derive(Debug, Default, Clone, Copy)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let adjusted = value - self.correction;
        let next = self.sum + adjusted;
        self.correction = (next - self.sum) - adjusted;
        self.sum = next;
    }

    fn total(self) -> f64 {
        self.sum
    }
}

/// Stable sin(x)/x evaluation, including the zero-width limit.
fn sinc(x: f64) -> f64 {
    let x2 = x * x;
    if x.abs() < 1.0e-4 {
        1.0 - x2 / 6.0 + x2 * x2 / 120.0 - x2 * x2 * x2 / 5040.0
    } else {
        x.sin() / x
    }
}

impl ThermalModel {
    pub fn new(
        heat_capacity_j_per_k: f64,
        loss_conductance_w_per_k: f64,
        service_fraction: f64,
    ) -> Result<Self, ModelError> {
        let model = Self {
            heat_capacity_j_per_k,
            loss_conductance_w_per_k,
            service_fraction,
        };
        model.validate()?;
        Ok(model)
    }

    fn validate(self) -> Result<(), ModelError> {
        if !self.heat_capacity_j_per_k.is_finite()
            || !self.loss_conductance_w_per_k.is_finite()
            || !self.service_fraction.is_finite()
        {
            return Err(ModelError::NonFiniteParameter);
        }
        if self.heat_capacity_j_per_k <= 0.0 {
            return Err(ModelError::NonPositiveHeatCapacity);
        }
        if self.loss_conductance_w_per_k <= 0.0 {
            return Err(ModelError::NonPositiveConductance);
        }
        if !(0.0..=1.0).contains(&self.service_fraction) {
            return Err(ModelError::ServiceFractionOutOfRange);
        }
        Ok(())
    }

    /// Exact endpoint and interval energies for a constant input over [t0,t1].
    ///
    /// C d(theta)/dt = (1-alpha) Q - UA theta
    /// Q_service = alpha Q
    /// Q_loss = UA theta
    pub fn piecewise_constant(
        self,
        theta_start_k: f64,
        source_power_w: f64,
        duration_s: f64,
    ) -> Result<EnergyBalance, ModelError> {
        self.validate()?;
        if !theta_start_k.is_finite() || !source_power_w.is_finite() {
            return Err(ModelError::NonFiniteParameter);
        }
        if duration_s <= 0.0 || !duration_s.is_finite() {
            return Err(ModelError::NonPositiveInterval);
        }
        if source_power_w < 0.0 {
            return Err(ModelError::NegativeSourcePowerEnvelope);
        }

        let c = self.heat_capacity_j_per_k;
        let ua = self.loss_conductance_w_per_k;
        let alpha = self.service_fraction;
        let tau = c / ua;
        let theta_inf = (1.0 - alpha) * source_power_w / ua;
        let ratio = duration_s / tau;
        let decay = (-ratio).exp();
        let one_minus_decay = -(-ratio).exp_m1();

        let theta_end = theta_inf + (theta_start_k - theta_inf) * decay;
        let energy_input = source_power_w * duration_s;
        let energy_service = alpha * energy_input;
        let energy_loss = ua
            * (theta_inf * duration_s
                + (theta_start_k - theta_inf) * tau * one_minus_decay);
        let storage_delta = c * (theta_end - theta_start_k);
        let residual = energy_input - energy_service - energy_loss - storage_delta;

        if ![
            tau,
            theta_inf,
            theta_end,
            energy_input,
            energy_service,
            energy_loss,
            storage_delta,
            residual,
        ]
        .iter()
        .all(|x| x.is_finite())
        {
            return Err(ModelError::NumericalOverflow);
        }

        Ok(EnergyBalance {
            start_time_s: 0.0,
            end_time_s: duration_s,
            theta_start_k,
            theta_end_k: theta_end,
            energy_input_j: energy_input,
            energy_service_j: energy_service,
            energy_loss_j: energy_loss,
            storage_delta_j: storage_delta,
            residual_j: residual,
        })
    }

    fn validate_sinusoid(self, input: SinusoidalInput) -> Result<(), ModelError> {
        self.validate()?;
        if !input.mean_power_w.is_finite()
            || !input.amplitude_w.is_finite()
            || !input.angular_frequency_rad_per_s.is_finite()
            || !input.initial_theta_k.is_finite()
        {
            return Err(ModelError::NonFiniteParameter);
        }
        if input.angular_frequency_rad_per_s <= 0.0 {
            return Err(ModelError::NonPositiveFrequency);
        }
        if input.mean_power_w < input.amplitude_w.abs() {
            return Err(ModelError::NegativeSourcePowerEnvelope);
        }
        Ok(())
    }

    /// Exact solution for Q(t)=Q0+A sin(omega t), initialized at t=0.
    pub fn sinusoidal_theta(
        self,
        input: SinusoidalInput,
        time_s: f64,
    ) -> Result<f64, ModelError> {
        self.validate_sinusoid(input)?;
        if !time_s.is_finite() {
            return Err(ModelError::NonFiniteParameter);
        }
        if time_s < 0.0 {
            return Err(ModelError::NegativeStartTime);
        }

        let c = self.heat_capacity_j_per_k;
        let ua = self.loss_conductance_w_per_k;
        let alpha = self.service_fraction;
        let k = ua / c;
        let omega = input.angular_frequency_rad_per_s;
        let beta_amplitude = (1.0 - alpha) * input.amplitude_w / c;
        let denominator = k * k + omega * omega;

        let theta_constant = (1.0 - alpha) * input.mean_power_w / ua;
        let sine_coefficient = beta_amplitude * k / denominator;
        let cosine_coefficient = -beta_amplitude * omega / denominator;
        let homogeneous_coefficient =
            input.initial_theta_k - theta_constant - cosine_coefficient;

        let theta = theta_constant
            + sine_coefficient * (omega * time_s).sin()
            + cosine_coefficient * (omega * time_s).cos()
            + homogeneous_coefficient * (-k * time_s).exp();

        if !theta.is_finite() {
            return Err(ModelError::NumericalOverflow);
        }
        Ok(theta)
    }

    /// Exact interval-integrated balance for a continuous sinusoidal source.
    ///
    /// The solution is analytic; the source is not approximated by a sampled
    /// zero-order hold in this oracle. Co-simulation runs may be compared
    /// against this continuous reference to quantify communication error.
    pub fn sinusoidal_interval(
        self,
        input: SinusoidalInput,
        start_time_s: f64,
        end_time_s: f64,
    ) -> Result<EnergyBalance, ModelError> {
        self.validate_sinusoid(input)?;
        if !start_time_s.is_finite() || !end_time_s.is_finite() {
            return Err(ModelError::NonFiniteParameter);
        }
        if start_time_s < 0.0 {
            return Err(ModelError::NegativeStartTime);
        }
        let duration = end_time_s - start_time_s;
        if duration <= 0.0 || !duration.is_finite() {
            return Err(ModelError::NonPositiveInterval);
        }

        let c = self.heat_capacity_j_per_k;
        let ua = self.loss_conductance_w_per_k;
        let alpha = self.service_fraction;
        let k = ua / c;
        let omega = input.angular_frequency_rad_per_s;
        let beta_amplitude = (1.0 - alpha) * input.amplitude_w / c;
        let denominator = k * k + omega * omega;

        let theta_constant = (1.0 - alpha) * input.mean_power_w / ua;
        let a = beta_amplitude * k / denominator;
        let b = -beta_amplitude * omega / denominator;
        let homogeneous = input.initial_theta_k - theta_constant - b;

        let half_phase = omega * duration * 0.5;
        let mid_time = start_time_s + duration * 0.5;
        let mid_phase = omega * mid_time;
        // Midpoint/sinc forms avoid subtracting nearly equal trigonometric
        // endpoint values when the communication interval is very small.
        let source_sine_integral = duration * mid_phase.sin() * sinc(half_phase);
        let cosine_integral = duration * mid_phase.cos() * sinc(half_phase);

        let energy_input = input.mean_power_w * duration
            + input.amplitude_w * source_sine_integral;
        let energy_service = alpha * energy_input;

        let one_minus_decay = -(-k * duration).exp_m1();
        let decay_integral = (-k * start_time_s).exp() * one_minus_decay / k;

        let integral_theta = theta_constant * duration
            + a * source_sine_integral
            + b * cosine_integral
            + homogeneous * decay_integral;
        let energy_loss = ua * integral_theta;

        let theta_start = self.sinusoidal_theta(input, start_time_s)?;
        let theta_end = self.sinusoidal_theta(input, end_time_s)?;
        let storage_delta = c * (theta_end - theta_start);
        let residual = energy_input - energy_service - energy_loss - storage_delta;

        if ![
            energy_input,
            energy_service,
            energy_loss,
            theta_start,
            theta_end,
            storage_delta,
            residual,
        ]
        .iter()
        .all(|x| x.is_finite())
        {
            return Err(ModelError::NumericalOverflow);
        }

        Ok(EnergyBalance {
            start_time_s,
            end_time_s,
            theta_start_k: theta_start,
            theta_end_k: theta_end,
            energy_input_j: energy_input,
            energy_service_j: energy_service,
            energy_loss_j: energy_loss,
            storage_delta_j: storage_delta,
            residual_j: residual,
        })
    }

    /// Simulate a sinusoidal source using left-endpoint zero-order hold.
    ///
    /// This is not the analytic reference: each communication interval samples
    /// Q(t) at its left endpoint and holds that value constant. The exact
    /// piecewise-constant state/energy solution is applied on each interval.
    /// Comparing this result to `sinusoidal_interval` quantifies communication
    /// discretization error independently of an FMI master implementation.
    pub fn sinusoidal_zero_order_hold(
        self,
        input: SinusoidalInput,
        start_time_s: f64,
        end_time_s: f64,
        communication_steps: usize,
    ) -> Result<EnergyBalance, ModelError> {
        self.validate_sinusoid(input)?;
        if !start_time_s.is_finite() || !end_time_s.is_finite() {
            return Err(ModelError::NonFiniteParameter);
        }
        if start_time_s < 0.0 {
            return Err(ModelError::NegativeStartTime);
        }
        if end_time_s <= start_time_s || !(end_time_s - start_time_s).is_finite() {
            return Err(ModelError::NonPositiveInterval);
        }
        if communication_steps == 0 {
            return Err(ModelError::NonPositiveSampleCount);
        }

        let duration = end_time_s - start_time_s;
        let step_count = communication_steps as f64;
        // The input's initial temperature is defined at t=0, even when this
        // communication campaign begins at a later absolute time.
        let mut theta = self.sinusoidal_theta(input, start_time_s)?;
        let theta_start = theta;
        let mut energy_input = CompensatedSum::default();
        let mut energy_service = CompensatedSum::default();
        let mut energy_loss = CompensatedSum::default();

        for index in 0..communication_steps {
            let fraction_start = index as f64 / step_count;
            let fraction_end = (index + 1) as f64 / step_count;
            let interval_start = start_time_s + duration * fraction_start;
            let interval_end = if index + 1 == communication_steps {
                end_time_s
            } else {
                start_time_s + duration * fraction_end
            };
            let interval_duration = interval_end - interval_start;
            let held_power = input.mean_power_w
                + input.amplitude_w
                    * (input.angular_frequency_rad_per_s * interval_start).sin();
            let interval = self.piecewise_constant(theta, held_power, interval_duration)?;
            theta = interval.theta_end_k;
            energy_input.add(interval.energy_input_j);
            energy_service.add(interval.energy_service_j);
            energy_loss.add(interval.energy_loss_j);
        }

        let energy_input = energy_input.total();
        let energy_service = energy_service.total();
        let energy_loss = energy_loss.total();
        let storage_delta = self.heat_capacity_j_per_k * (theta - theta_start);
        let residual = energy_input - energy_service - energy_loss - storage_delta;
        if ![
            theta,
            energy_input,
            energy_service,
            energy_loss,
            storage_delta,
            residual,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(ModelError::NumericalOverflow);
        }

        Ok(EnergyBalance {
            start_time_s,
            end_time_s,
            theta_start_k: theta_start,
            theta_end_k: theta,
            energy_input_j: energy_input,
            energy_service_j: energy_service,
            energy_loss_j: energy_loss,
            storage_delta_j: storage_delta,
            residual_j: residual,
        })
    }

    /// Simulate a sinusoidal source using exact interval-average power.
    ///
    /// For each interval [t0,t1], the exchanged constant power is
    /// integral(Q_source(t), t0..t1) / (t1-t0). Therefore each interval's
    /// source-energy transfer is exact for this known sinusoidal waveform.
    /// The receiver still evolves under a piecewise-constant input, so the
    /// endpoint temperature is an approximation to the continuous response.
    ///
    /// This is a conservative *oracle/method reference*, not a universally
    /// causal master policy: a producer must know or forecast the waveform
    /// over the complete communication interval, or expose an integrated
    /// interval-flux output. Do not infer future values from a point sample.
    pub fn sinusoidal_interval_average_exchange(
        self,
        input: SinusoidalInput,
        start_time_s: f64,
        end_time_s: f64,
        communication_steps: usize,
    ) -> Result<EnergyBalance, ModelError> {
        self.validate_sinusoid(input)?;
        if !start_time_s.is_finite() || !end_time_s.is_finite() {
            return Err(ModelError::NonFiniteParameter);
        }
        if start_time_s < 0.0 {
            return Err(ModelError::NegativeStartTime);
        }
        if end_time_s <= start_time_s || !(end_time_s - start_time_s).is_finite() {
            return Err(ModelError::NonPositiveInterval);
        }
        if communication_steps == 0 {
            return Err(ModelError::NonPositiveSampleCount);
        }

        let duration = end_time_s - start_time_s;
        let step_count = communication_steps as f64;
        let mut theta = self.sinusoidal_theta(input, start_time_s)?;
        let theta_start = theta;
        let mut energy_input = CompensatedSum::default();
        let mut energy_service = CompensatedSum::default();
        let mut energy_loss = CompensatedSum::default();

        for index in 0..communication_steps {
            let fraction_start = index as f64 / step_count;
            let fraction_end = (index + 1) as f64 / step_count;
            let interval_start = start_time_s + duration * fraction_start;
            let interval_end = if index + 1 == communication_steps {
                end_time_s
            } else {
                start_time_s + duration * fraction_end
            };
            let interval_duration = interval_end - interval_start;
            let half_phase = input.angular_frequency_rad_per_s * interval_duration * 0.5;
            let midpoint = interval_start + interval_duration * 0.5;
            let midpoint_phase = input.angular_frequency_rad_per_s * midpoint;
            let average_source_power = input.mean_power_w
                + input.amplitude_w * midpoint_phase.sin() * sinc(half_phase);

            let interval =
                self.piecewise_constant(theta, average_source_power, interval_duration)?;
            theta = interval.theta_end_k;
            energy_input.add(interval.energy_input_j);
            energy_service.add(interval.energy_service_j);
            energy_loss.add(interval.energy_loss_j);
        }

        let energy_input = energy_input.total();
        let energy_service = energy_service.total();
        let energy_loss = energy_loss.total();
        let storage_delta = self.heat_capacity_j_per_k * (theta - theta_start);
        let residual = energy_input - energy_service - energy_loss - storage_delta;
        if ![
            theta,
            energy_input,
            energy_service,
            energy_loss,
            storage_delta,
            residual,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(ModelError::NumericalOverflow);
        }

        Ok(EnergyBalance {
            start_time_s,
            end_time_s,
            theta_start_k: theta_start,
            theta_end_k: theta,
            energy_input_j: energy_input,
            energy_service_j: energy_service,
            energy_loss_j: energy_loss,
            storage_delta_j: storage_delta,
            residual_j: residual,
        })
    }

    /// Simulate a sinusoidal source with a causal, bounded linear predictor.
    ///
    /// At communication point t_n the predictor uses only Q_n and Q_(n-1):
    ///   Qbar_n = Q_n + 0.5 * (Q_n - Q_(n-1))
    /// The first interval is bootstrapped with Q_0 (zero-order hold). The
    /// prediction is projected onto the explicitly declared source envelope
    /// [mean-|amplitude|, mean+|amplitude|]. Every projection is counted and
    /// its maximum magnitude is returned; it is never silently hidden.
    ///
    /// No future source sample is consulted. This is an oracle implementation
    /// of a deployable master-side predictor: a real importer supplies Q_n
    /// from the just-observed FMI output and retains Q_(n-1) from the prior
    /// committed communication point.
    ///
    /// The exchanged interval power is an approximation, not the exact future
    /// interval average. The receiver's balance closes against that declared
    /// exchanged value; the endpoint error is measured separately against the
    /// continuous sinusoidal reference.
    pub fn sinusoidal_causal_bounded_predictor(
        self,
        input: SinusoidalInput,
        start_time_s: f64,
        end_time_s: f64,
        communication_steps: usize,
    ) -> Result<CausalPredictorResult, ModelError> {
        self.validate_sinusoid(input)?;
        if !start_time_s.is_finite() || !end_time_s.is_finite() {
            return Err(ModelError::NonFiniteParameter);
        }
        if start_time_s < 0.0 {
            return Err(ModelError::NegativeStartTime);
        }
        if end_time_s <= start_time_s || !(end_time_s - start_time_s).is_finite() {
            return Err(ModelError::NonPositiveInterval);
        }
        if communication_steps == 0 {
            return Err(ModelError::NonPositiveSampleCount);
        }

        let duration = end_time_s - start_time_s;
        let step_count = communication_steps as f64;
        let minimum_power_w = input.mean_power_w - input.amplitude_w.abs();
        let maximum_power_w = input.mean_power_w + input.amplitude_w.abs();
        let mut theta = self.sinusoidal_theta(input, start_time_s)?;
        let theta_start = theta;
        let mut previous_source_sample_w: Option<f64> = None;
        let mut limiter_activations = 0_usize;
        let mut max_unbounded_overshoot_w: f64 = 0.0;
        let mut energy_input = CompensatedSum::default();
        let mut energy_service = CompensatedSum::default();
        let mut energy_loss = CompensatedSum::default();

        for index in 0..communication_steps {
            let fraction_start = index as f64 / step_count;
            let fraction_end = (index + 1) as f64 / step_count;
            let interval_start = start_time_s + duration * fraction_start;
            let interval_end = if index + 1 == communication_steps {
                end_time_s
            } else {
                start_time_s + duration * fraction_end
            };
            let interval_duration = interval_end - interval_start;

            // This is the only new source observation made for this interval.
            // The predictor never samples t_(n+1) before committing interval n.
            let current_source_sample_w = input.mean_power_w
                + input.amplitude_w
                    * (input.angular_frequency_rad_per_s * interval_start).sin();
            let unbounded_prediction_w = match previous_source_sample_w {
                Some(previous) => {
                    current_source_sample_w + 0.5 * (current_source_sample_w - previous)
                }
                None => current_source_sample_w,
            };

            let exchanged_power_w = unbounded_prediction_w
                .clamp(minimum_power_w, maximum_power_w);
            let overshoot_w = (unbounded_prediction_w - exchanged_power_w).abs();
            if overshoot_w > 0.0 {
                limiter_activations += 1;
                max_unbounded_overshoot_w = max_unbounded_overshoot_w.max(overshoot_w);
            }

            let interval = self.piecewise_constant(theta, exchanged_power_w, interval_duration)?;
            theta = interval.theta_end_k;
            energy_input.add(interval.energy_input_j);
            energy_service.add(interval.energy_service_j);
            energy_loss.add(interval.energy_loss_j);
            previous_source_sample_w = Some(current_source_sample_w);
        }

        let energy_input = energy_input.total();
        let energy_service = energy_service.total();
        let energy_loss = energy_loss.total();
        let storage_delta = self.heat_capacity_j_per_k * (theta - theta_start);
        let residual = energy_input - energy_service - energy_loss - storage_delta;
        if ![
            theta,
            energy_input,
            energy_service,
            energy_loss,
            storage_delta,
            residual,
            max_unbounded_overshoot_w,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(ModelError::NumericalOverflow);
        }

        Ok(CausalPredictorResult {
            balance: EnergyBalance {
                start_time_s,
                end_time_s,
                theta_start_k: theta_start,
                theta_end_k: theta,
                energy_input_j: energy_input,
                energy_service_j: energy_service,
                energy_loss_j: energy_loss,
                storage_delta_j: storage_delta,
                residual_j: residual,
            },
            limiter_activations,
            max_unbounded_overshoot_w,
        })
    }

    /// Consume a producer-reported interval energy in joules without future samples.
    ///
    /// A master can obtain the interval energy as the difference of two
    /// successful cumulative-energy output observations at the boundaries of
    /// one completed producer step. The receiver represents that exact
    /// interval integral as a constant average power over duration_s, then
    /// applies the exact piecewise-constant state transition.
    ///
    /// The input energy remains the exact declared J quantity in the returned
    /// balance. The receiver trajectory is still an average-input
    /// approximation when the physical within-window waveform is not constant.
    /// This routine cannot authenticate the producer's integral or interval ID;
    /// those belong to the FMI/API/physical-port evidence layers.
    pub fn integrated_flux_exchange(
        self,
        theta_start_k: f64,
        interval_energy_j: f64,
        start_time_s: f64,
        end_time_s: f64,
    ) -> Result<EnergyBalance, ModelError> {
        self.validate()?;
        if !theta_start_k.is_finite()
            || !interval_energy_j.is_finite()
            || !start_time_s.is_finite()
            || !end_time_s.is_finite()
        {
            return Err(ModelError::NonFiniteParameter);
        }
        if interval_energy_j < 0.0 {
            return Err(ModelError::NegativeSourcePowerEnvelope);
        }
        if start_time_s < 0.0 {
            return Err(ModelError::NegativeStartTime);
        }
        let duration_s = end_time_s - start_time_s;
        if duration_s <= 0.0 || !duration_s.is_finite() {
            return Err(ModelError::NonPositiveInterval);
        }

        let average_power_w = interval_energy_j / duration_s;
        if !average_power_w.is_finite() {
            return Err(ModelError::NumericalOverflow);
        }
        let mut balance = self.piecewise_constant(theta_start_k, average_power_w, duration_s)?;
        balance.start_time_s = start_time_s;
        balance.end_time_s = end_time_s;
        // Preserve the producer's committed integral exactly, rather than
        // reconstructing its energy from a rounded W × s multiplication.
        balance.energy_input_j = interval_energy_j;
        balance.energy_service_j = self.service_fraction * interval_energy_j;
        balance.residual_j = balance.energy_input_j
            - balance.energy_service_j
            - balance.energy_loss_j
            - balance.storage_delta_j;
        if ![
            balance.energy_service_j,
            balance.residual_j,
            balance.theta_end_k,
            balance.energy_loss_j,
            balance.storage_delta_j,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(ModelError::NumericalOverflow);
        }
        Ok(balance)
    }

    pub fn time_constant_s(self) -> Result<f64, ModelError> {
        self.validate()?;
        Ok(self.heat_capacity_j_per_k / self.loss_conductance_w_per_k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.12e}, expected={expected:.12e}, tolerance={tolerance:.3e}"
        );
    }

    fn model() -> ThermalModel {
        ThermalModel::new(10_000_000.0, 100_000.0, 0.4).unwrap()
    }

    fn adaptive_input() -> AdaptiveStepInput {
        AdaptiveStepInput {
            current_time_s: 1.0,
            attempted_step_s: 10.0,
            minimum_step_s: 0.5,
            maximum_step_s: 20.0,
            trajectory_error: 0.01,
            trajectory_tolerance: 0.1,
            interface_residual: 0.002,
            interface_tolerance: 0.01,
            energy_residual: 0.1,
            energy_tolerance: 1.0,
            method_order: 2,
            next_event_time_s: None,
        }
    }

    #[test]
    fn adaptive_controller_uses_the_worst_of_three_independent_error_gates() {
        let mut input = adaptive_input();
        input.trajectory_error = 0.001;
        input.interface_residual = 0.002;
        input.energy_residual = 2.0;
        let decision = propose_adaptive_step(input).unwrap();

        assert_eq!(decision.action, AdaptiveStepAction::RejectAndRetry);
        close(decision.normalized_error, 2.0, 1.0e-15);
        assert!(decision.next_step_s < input.attempted_step_s);
        assert!(!decision.event_fence_applied);
    }

    #[test]
    fn adaptive_controller_fences_an_accepted_step_at_the_next_event() {
        let mut input = adaptive_input();
        input.trajectory_error = 0.005;
        input.interface_residual = 0.001;
        input.energy_residual = 0.1;
        input.next_event_time_s = Some(17.0);
        let decision = propose_adaptive_step(input).unwrap();

        assert_eq!(decision.action, AdaptiveStepAction::Accept);
        assert!(decision.event_fence_applied);
        assert_eq!(decision.event_time_s, Some(17.0));
        close(decision.next_step_s, 16.0, 1.0e-15);
    }

    #[test]
    fn adaptive_controller_processes_an_event_due_at_current_time_without_zero_dostep() {
        let mut input = adaptive_input();
        input.next_event_time_s = Some(input.current_time_s);
        let decision = propose_adaptive_step(input).unwrap();

        assert_eq!(decision.action, AdaptiveStepAction::ProcessEventNow);
        assert_eq!(decision.next_step_s, 0.0);
        assert!(decision.event_fence_applied);
    }

    #[test]
    fn adaptive_controller_allows_exact_event_fence_below_minimum_step() {
        let mut input = adaptive_input();
        input.next_event_time_s = Some(input.current_time_s + 0.2);
        input.trajectory_error = 0.01;
        input.interface_residual = 0.001;
        input.energy_residual = 0.1;
        let decision = propose_adaptive_step(input).unwrap();

        assert_eq!(decision.action, AdaptiveStepAction::Accept);
        assert!(decision.event_fence_applied);
        assert!(decision.event_step_below_minimum);
        close(decision.next_step_s, 0.2, 1.0e-15);
    }

    #[test]
    fn adaptive_controller_fails_closed_when_error_requires_a_subminimum_retry() {
        let mut input = adaptive_input();
        input.minimum_step_s = 2.0;
        input.trajectory_error = 100.0;
        input.trajectory_tolerance = 0.1;
        input.next_event_time_s = None;
        let decision = propose_adaptive_step(input).unwrap();

        assert_eq!(decision.action, AdaptiveStepAction::FailClosedAtMinimumStep);
        assert!(decision.next_step_s < input.minimum_step_s);
    }

    #[test]
    fn adaptive_controller_rejects_past_events_and_invalid_tolerances() {
        let mut input = adaptive_input();
        input.next_event_time_s = Some(0.5);
        assert_eq!(
            propose_adaptive_step(input),
            Err(ModelError::EventInPast)
        );

        input = adaptive_input();
        input.energy_tolerance = 0.0;
        assert_eq!(
            propose_adaptive_step(input),
            Err(ModelError::NonPositiveInterval)
        );
    }

    #[test]
    fn cumulative_energy_delta_uses_only_committed_monotone_observations() {
        assert_eq!(cumulative_energy_delta_j(10_000.0, 20_000.0), Ok(10_000.0));
        assert_eq!(cumulative_energy_delta_j(10_000.0, 10_000.0), Ok(0.0));
        assert_eq!(
            cumulative_energy_delta_j(20_000.0, 10_000.0),
            Err(ModelError::CumulativeEnergyRegressed)
        );
        assert_eq!(
            cumulative_energy_delta_j(-1.0, 10_000.0),
            Err(ModelError::NegativeCumulativeEnergy)
        );
        assert_eq!(
            cumulative_energy_delta_j(0.0, f64::INFINITY),
            Err(ModelError::NonFiniteParameter)
        );
    }

    #[test]
    fn integrated_flux_handshake_preserves_exact_declared_source_energy() {
        let model = model();
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 100_000.0,
            angular_frequency_rad_per_s: 2.0 * PI / 400.0,
            initial_theta_k: 2.0,
        };
        let continuous = model.sinusoidal_interval(input, 0.0, 120.0).unwrap();
        let interval_energy = cumulative_energy_delta_j(
            0.0,
            continuous.energy_input_j,
        ).unwrap();
        let exchange = model
            .integrated_flux_exchange(2.0, interval_energy, 0.0, 120.0)
            .unwrap();
        let one_window_average = model
            .sinusoidal_interval_average_exchange(input, 0.0, 120.0, 1)
            .unwrap();

        assert_eq!(exchange.energy_input_j, continuous.energy_input_j);
        close(exchange.energy_service_j, 27_333_384.403_937_1, 1.0e-6);
        close(exchange.theta_end_k, 2.989_979_327_511_903, 1.0e-12);
        close(exchange.energy_loss_j, 31_100_283.330_786_623, 1.0e-6);
        close(exchange.storage_delta_j, 9_899_793.275_119_029, 1.0e-6);
        close(exchange.residual_j, 0.0, 1.0e-7);
        close(exchange.theta_end_k, one_window_average.theta_end_k, 1.0e-12);
        assert_eq!(exchange.start_time_s, 0.0);
        assert_eq!(exchange.end_time_s, 120.0);
    }

    #[test]
    fn integrated_flux_handshake_rejects_negative_energy_and_invalid_time() {
        assert_eq!(
            model().integrated_flux_exchange(2.0, -1.0, 0.0, 1.0),
            Err(ModelError::NegativeSourcePowerEnvelope)
        );
        assert_eq!(
            model().integrated_flux_exchange(2.0, 1.0, 2.0, 2.0),
            Err(ModelError::NonPositiveInterval)
        );
        assert_eq!(
            model().integrated_flux_exchange(2.0, 1.0, -1.0, 2.0),
            Err(ModelError::NegativeStartTime)
        );
    }

    #[test]
    fn piecewise_constant_known_answer_matches_frozen_oracle() {
        let result = model().piecewise_constant(1.5, 500_000.0, 60.0).unwrap();
        close(result.theta_end_k, 2.176_782_545_858_960_5, 1e-12);
        close(result.energy_input_j, 30_000_000.0, 1e-8);
        close(result.energy_service_j, 12_000_000.0, 1e-8);
        close(result.energy_loss_j, 11_232_174.541_410_396, 1e-6);
        close(result.storage_delta_j, 6_767_825.458_589_605, 1e-6);
        close(result.residual_j, 0.0, 1e-7);
    }

    #[test]
    fn piecewise_constant_result_is_partition_invariant() {
        let model = model();
        let whole = model.piecewise_constant(1.5, 500_000.0, 60.0).unwrap();
        let first = model.piecewise_constant(1.5, 500_000.0, 30.0).unwrap();
        let second = model
            .piecewise_constant(first.theta_end_k, 500_000.0, 30.0)
            .unwrap();

        close(whole.theta_end_k, second.theta_end_k, 1e-12);
        close(
            whole.energy_loss_j,
            first.energy_loss_j + second.energy_loss_j,
            1e-6,
        );
        close(
            whole.storage_delta_j,
            first.storage_delta_j + second.storage_delta_j,
            1e-7,
        );
        close(whole.residual_j, 0.0, 1e-7);
        close(first.residual_j + second.residual_j, 0.0, 1e-7);
    }

    #[test]
    fn sinusoidal_known_answer_matches_frozen_oracle() {
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 100_000.0,
            angular_frequency_rad_per_s: 2.0 * PI / 400.0,
            initial_theta_k: 2.0,
        };
        let result = model().sinusoidal_interval(input, 0.0, 120.0).unwrap();

        close(result.theta_start_k, 2.0, 1e-12);
        close(result.theta_end_k, 3.029_238_982_715_92, 1e-12);
        close(result.energy_input_j, 68_333_461.009_842_75, 1e-6);
        close(result.energy_service_j, 27_333_384.403_937_1, 1e-6);
        close(result.energy_loss_j, 30_707_686.778_746_45, 1e-6);
        close(result.storage_delta_j, 10_292_389.827_159_2, 1e-6);
        close(result.residual_j, 0.0, 1e-6);
    }

    #[test]
    fn zero_amplitude_sinusoid_reduces_to_constant_input_solution() {
        let model = model();
        let sinusoid = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 0.0,
            angular_frequency_rad_per_s: 0.01,
            initial_theta_k: 1.5,
        };
        let exact = model.sinusoidal_interval(sinusoid, 0.0, 60.0).unwrap();
        let constant = model.piecewise_constant(1.5, 500_000.0, 60.0).unwrap();

        close(exact.theta_end_k, constant.theta_end_k, 1e-12);
        close(exact.energy_loss_j, constant.energy_loss_j, 1e-6);
        close(exact.storage_delta_j, constant.storage_delta_j, 1e-7);
        close(exact.residual_j, 0.0, 1e-7);
    }

    #[test]
    fn sinusoidal_zero_order_hold_converges_to_continuous_reference() {
        let model = model();
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 100_000.0,
            angular_frequency_rad_per_s: 2.0 * PI / 400.0,
            initial_theta_k: 2.0,
        };
        let reference = model.sinusoidal_interval(input, 0.0, 120.0).unwrap();
        let frozen = [
            (15_usize, 0.010_188_510_122_55),
            (30, 0.004_946_932_769_56),
            (60, 0.002_437_463_242_23),
            (120, 0.001_209_833_516_80),
            (240, 0.000_602_705_019_81),
            (480, 0.000_300_801_170_68),
            (960, 0.000_150_262_949_80),
        ];

        let mut previous_error = f64::INFINITY;
        let mut errors = Vec::new();
        for (steps, expected_error) in frozen {
            let approximate = model
                .sinusoidal_zero_order_hold(input, 0.0, 120.0, steps)
                .unwrap();
            let error = (approximate.theta_end_k - reference.theta_end_k).abs();
            close(error, expected_error, 2.0e-12);
            assert!(error < previous_error, "error must decrease with finer communication grids");
            close(approximate.residual_j, 0.0, 2.0e-7);
            errors.push(error);
            previous_error = error;
        }

        // Left-endpoint zero-order hold should exhibit first-order global
        // convergence for this smooth, non-event input waveform.
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!((1.90..=2.10).contains(&ratio), "observed error ratio {ratio}");
        }
    }

    #[test]
    fn interval_average_exchange_preserves_source_energy_and_converges_quadratically() {
        let model = model();
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 100_000.0,
            angular_frequency_rad_per_s: 2.0 * PI / 400.0,
            initial_theta_k: 2.0,
        };
        let reference = model.sinusoidal_interval(input, 0.0, 120.0).unwrap();
        let frozen = [
            (15_usize, 0.000_128_312_646_191_731),
            (30, 0.000_032_039_611_065_57),
            (60, 0.000_008_007_493_556_22798),
            (120, 0.000_002_001_722_820_83244),
            (240, 0.000_000_500_421_294_624687),
            (480, 0.000_000_125_104_729_686213),
            (960, 0.000_000_031_276_170_5480069),
        ];

        let mut previous_error = f64::INFINITY;
        let mut errors = Vec::new();
        for (steps, expected_error) in frozen {
            let approximate = model
                .sinusoidal_interval_average_exchange(input, 0.0, 120.0, steps)
                .unwrap();
            let error = (approximate.theta_end_k - reference.theta_end_k).abs();
            close(error, expected_error, 2.0e-12);
            assert!(error < previous_error, "error must decrease with finer communication grids");
            close(approximate.energy_input_j, reference.energy_input_j, 1.0e-6);
            close(approximate.energy_service_j, reference.energy_service_j, 1.0e-6);
            close(approximate.residual_j, 0.0, 5.0e-7);
            errors.push(error);
            previous_error = error;
        }

        // Exact interval-average source power removes the left-endpoint bias;
        // for this smooth waveform the remaining global state error is O(h^2).
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!((3.90..=4.10).contains(&ratio), "observed error ratio {ratio}");
        }
    }

    #[test]
    fn causal_bounded_predictor_matches_frozen_reference_and_closes_its_declared_exchange() {
        let model = model();
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 100_000.0,
            angular_frequency_rad_per_s: 2.0 * PI / 400.0,
            initial_theta_k: 2.0,
        };
        let reference = model.sinusoidal_interval(input, 0.0, 120.0).unwrap();
        let frozen = [
            (15_usize, 0.000_787_129_425_244_615, 1_usize, 589.646_727_806_306),
            (30, 0.000_231_273_451_280_156, 2, 98.663_578_586_420),
            (60, 0.000_064_480_004_970_058, 2, 24.671_981_713_502),
            (120, 0.000_016_949_474_196_615, 2, 6.168_375_917_012),
            (240, 0.000_004_340_801_009_306, 2, 1.542_117_760_517),
            (480, 0.000_001_098_112_006_659, 2, 0.385_530_926_520),
            (960, 0.000_000_276_140_812_471, 2, 0.096_382_824_472),
        ];

        let mut previous_error = f64::INFINITY;
        let mut errors = Vec::new();
        for (steps, expected_error, expected_limits, expected_overshoot) in frozen {
            let result = model
                .sinusoidal_causal_bounded_predictor(input, 0.0, 120.0, steps)
                .unwrap();
            let balance = result.balance;
            let error = (balance.theta_end_k - reference.theta_end_k).abs();
            close(error, expected_error, 3.0e-12);
            assert!(error < previous_error, "error must decrease with finer communication grids");
            assert_eq!(result.limiter_activations, expected_limits);
            close(
                result.max_unbounded_overshoot_w,
                expected_overshoot,
                2.0e-9,
            );
            close(balance.residual_j, 0.0, 3.0e-7);
            assert!(balance.energy_input_j > 0.0);
            assert!(balance.energy_service_j > 0.0);
            assert!(balance.energy_loss_j > 0.0);
            errors.push(error);
            previous_error = error;
        }

        // The causal bounded predictor is second-order globally after the
        // first-interval bootstrap; the ratio approaches four under refinement.
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!((3.30..=4.10).contains(&ratio), "observed error ratio {ratio}");
        }
    }

    #[test]
    fn causal_bounded_predictor_rejects_an_empty_communication_grid() {
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 100_000.0,
            angular_frequency_rad_per_s: 2.0 * PI / 400.0,
            initial_theta_k: 2.0,
        };
        assert_eq!(
            model().sinusoidal_causal_bounded_predictor(input, 0.0, 120.0, 0),
            Err(ModelError::NonPositiveSampleCount)
        );
    }

    #[test]
    fn causal_predictor_starts_at_the_exact_declared_absolute_time() {
        let model = model();
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 100_000.0,
            angular_frequency_rad_per_s: 2.0 * PI / 400.0,
            initial_theta_k: 2.0,
        };
        let reference = model.sinusoidal_interval(input, 30.0, 120.0).unwrap();
        let result = model
            .sinusoidal_causal_bounded_predictor(input, 30.0, 120.0, 90)
            .unwrap();
        close(result.balance.theta_start_k, reference.theta_start_k, 1.0e-12);
        close(result.balance.residual_j, 0.0, 3.0e-7);
    }

    #[test]
    fn interval_average_exchange_uses_the_true_state_at_nonzero_start_time() {
        let model = model();
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 100_000.0,
            angular_frequency_rad_per_s: 2.0 * PI / 400.0,
            initial_theta_k: 2.0,
        };
        let reference = model.sinusoidal_interval(input, 30.0, 120.0).unwrap();
        let exchange = model
            .sinusoidal_interval_average_exchange(input, 30.0, 120.0, 90)
            .unwrap();

        close(exchange.theta_start_k, reference.theta_start_k, 1.0e-12);
        close(exchange.energy_input_j, reference.energy_input_j, 1.0e-6);
        close(exchange.energy_service_j, reference.energy_service_j, 1.0e-6);
        close(exchange.residual_j, 0.0, 2.0e-7);
    }

    #[test]
    fn interval_average_exchange_rejects_empty_communication_grid() {
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 0.0,
            angular_frequency_rad_per_s: 0.01,
            initial_theta_k: 1.5,
        };
        assert_eq!(
            model().sinusoidal_interval_average_exchange(input, 0.0, 60.0, 0),
            Err(ModelError::NonPositiveSampleCount)
        );
    }

    #[test]
    fn zero_order_hold_energy_balance_is_partition_invariant_for_constant_source() {
        let model = model();
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 0.0,
            angular_frequency_rad_per_s: 0.01,
            initial_theta_k: 1.5,
        };
        let one = model.sinusoidal_zero_order_hold(input, 0.0, 60.0, 1).unwrap();
        let many = model.sinusoidal_zero_order_hold(input, 0.0, 60.0, 120).unwrap();

        close(one.theta_end_k, many.theta_end_k, 1.0e-12);
        close(one.energy_input_j, many.energy_input_j, 1.0e-7);
        close(one.energy_loss_j, many.energy_loss_j, 1.0e-6);
        close(one.storage_delta_j, many.storage_delta_j, 1.0e-7);
    }

    #[test]
    fn zero_order_hold_rejects_empty_communication_grid() {
        let input = SinusoidalInput {
            mean_power_w: 500_000.0,
            amplitude_w: 0.0,
            angular_frequency_rad_per_s: 0.01,
            initial_theta_k: 1.5,
        };
        assert_eq!(
            model().sinusoidal_zero_order_hold(input, 0.0, 60.0, 0),
            Err(ModelError::NonPositiveSampleCount)
        );
    }

    #[test]
    fn invalid_parameters_and_intervals_fail_closed() {
        assert_eq!(
            ThermalModel::new(0.0, 100_000.0, 0.4),
            Err(ModelError::NonPositiveHeatCapacity)
        );
        assert_eq!(
            ThermalModel::new(10_000_000.0, 0.0, 0.4),
            Err(ModelError::NonPositiveConductance)
        );
        assert_eq!(
            ThermalModel::new(10_000_000.0, 100_000.0, 1.1),
            Err(ModelError::ServiceFractionOutOfRange)
        );
        assert_eq!(
            model().piecewise_constant(1.5, 500_000.0, 0.0),
            Err(ModelError::NonPositiveInterval)
        );
    }
}
