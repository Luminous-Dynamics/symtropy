#![forbid(unsafe_code)]

//! Closed-form reference solutions for a lumped thermal buffer.
//!
//! This is a synthetic benchmark oracle. It is not a physical geothermal or
//! heat-exchanger model, an FMI master, or an energy-accounting authority.

use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelError {
    NonFiniteParameter,
    NonPositiveHeatCapacity,
    NonPositiveConductance,
    ServiceFractionOutOfRange,
    NonPositiveInterval,
    NegativeStartTime,
    NonPositiveFrequency,
    NegativeSourcePowerEnvelope,
    NumericalOverflow,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SinusoidalInput {
    pub mean_power_w: f64,
    pub amplitude_w: f64,
    pub angular_frequency_rad_per_s: f64,
    pub initial_theta_k: f64,
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
        let mid_phase = omega * (start_time_s + end_time_s) * 0.5;
        let sin_half = half_phase.sin();
        let cos_difference = 2.0 * mid_phase.sin() * sin_half;
        let sin_difference = 2.0 * mid_phase.cos() * sin_half;
        let source_sine_integral = cos_difference / omega;

        let energy_input = input.mean_power_w * duration
            + input.amplitude_w * source_sine_integral;
        let energy_service = alpha * energy_input;

        let one_minus_decay = -( -k * duration).exp_m1();
        let decay_integral =
            (-k * start_time_s).exp() * one_minus_decay / k;

        let integral_theta = theta_constant * duration
            + a * source_sine_integral
            + b * (sin_difference / omega)
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

    pub fn time_constant_s(self) -> Result<f64, ModelError> {
        self.validate()?;
        Ok(self.heat_capacity_j_per_k / self.loss_conductance_w_per_k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.12e}, expected={expected:.12e}, tolerance={tolerance:.3e}"
        );
    }

    fn model() -> ThermalModel {
        ThermalModel::new(10_000_000.0, 100_000.0, 0.4).unwrap()
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
