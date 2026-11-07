//! Exact continuous-time transition for a two-dimensional damped oscillator.
//!
//! State convention:
//!     dz = F z dt + L dW
//!     F = [[-kappa, -omega], [omega, -kappa]]
//!
//! Under stationary isotropic covariance P_inf = stationary_variance * I,
//! Q(dt) = P_inf - A(dt) P_inf A(dt)^T.
//!
//! This module is a numerical reference kernel, not a fitted biological model.

pub mod filter;
pub use filter::{
    ChannelMeasurement, ChannelSpec, FilterError, FilterUpdate, ObservationForecast,
    OscillatorObservationFilter, PhaseEstimate,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OscillatorParameters {
    /// Angular frequency in radians per declared time unit.
    pub angular_frequency: f64,
    /// Non-negative damping rate in inverse declared time units.
    pub damping_rate: f64,
    /// Marginal stationary variance of either latent coordinate.
    pub stationary_variance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OscillatorTransition {
    /// Exact continuous-time state transition matrix exp(F * dt).
    pub matrix: [[f64; 2]; 2],
    /// Exact discrete-time process-noise covariance for this elapsed time.
    pub process_covariance: [[f64; 2]; 2],
    /// Elapsed time used to construct this transition.
    pub elapsed_time: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OscillatorError {
    InvalidAngularFrequency,
    InvalidDampingRate,
    InvalidStationaryVariance,
    InvalidElapsedTime,
    NonFinitePhaseAdvance,
    NonFiniteTransition,
    NonFiniteState,
}

impl OscillatorParameters {
    pub fn validate(self) -> Result<(), OscillatorError> {
        if !self.angular_frequency.is_finite() {
            return Err(OscillatorError::InvalidAngularFrequency);
        }
        if !self.damping_rate.is_finite() || self.damping_rate < 0.0 {
            return Err(OscillatorError::InvalidDampingRate);
        }
        if !self.stationary_variance.is_finite() || self.stationary_variance <= 0.0 {
            return Err(OscillatorError::InvalidStationaryVariance);
        }
        Ok(())
    }

    /// Construct the exact transition and process covariance for dt > 0.
    ///
    /// exp_m1 is used for the process covariance so very small positive
    /// elapsed times do not lose all precision to subtraction from one.
    pub fn transition(self, elapsed_time: f64) -> Result<OscillatorTransition, OscillatorError> {
        self.validate()?;
        if !elapsed_time.is_finite() || elapsed_time <= 0.0 {
            return Err(OscillatorError::InvalidElapsedTime);
        }

        let phase_advance = self.angular_frequency * elapsed_time;
        if !phase_advance.is_finite() {
            return Err(OscillatorError::NonFinitePhaseAdvance);
        }
        let damping_exponent = -self.damping_rate * elapsed_time;
        let attenuation = damping_exponent.exp();
        let (sin_phase, cos_phase) = phase_advance.sin_cos();

        let matrix = [
            [attenuation * cos_phase, -attenuation * sin_phase],
            [attenuation * sin_phase, attenuation * cos_phase],
        ];

        // Q = sigma_inf^2 * (1 - exp(-2*kappa*dt)) * I.
        // For kappa == 0 this is exactly zero; the latent state rotates
        // deterministically while preserving its stationary covariance.
        let process_variance =
            self.stationary_variance * -(-2.0 * self.damping_rate * elapsed_time).exp_m1();
        let process_covariance = [[process_variance, 0.0], [0.0, process_variance]];

        if !matrix.iter().flatten().all(|value| value.is_finite())
            || !process_variance.is_finite()
            || process_variance < 0.0
        {
            return Err(OscillatorError::NonFiniteTransition);
        }

        Ok(OscillatorTransition {
            matrix,
            process_covariance,
            elapsed_time,
        })
    }
}

impl OscillatorTransition {
    pub fn apply(self, state: [f64; 2]) -> Result<[f64; 2], OscillatorError> {
        if !state.iter().all(|value| value.is_finite()) {
            return Err(OscillatorError::NonFiniteState);
        }
        let result = [
            self.matrix[0][0] * state[0] + self.matrix[0][1] * state[1],
            self.matrix[1][0] * state[0] + self.matrix[1][1] * state[1],
        ];
        if !result.iter().all(|value| value.is_finite()) {
            return Err(OscillatorError::NonFiniteState);
        }
        Ok(result)
    }

    pub fn apply_covariance(self, covariance: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
        let at = transpose(self.matrix);
        multiply(multiply(self.matrix, covariance), at)
    }

    /// Propagate a covariance through this transition and add process noise.
    pub fn propagate_covariance(self, covariance: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
        add(self.apply_covariance(covariance), self.process_covariance)
    }
}

pub fn transpose(matrix: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
    [[matrix[0][0], matrix[1][0]], [matrix[0][1], matrix[1][1]]]
}

pub fn multiply(left: [[f64; 2]; 2], right: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
    let mut result = [[0.0; 2]; 2];
    for (row, result_row) in result.iter_mut().enumerate() {
        for (column, value) in result_row.iter_mut().enumerate() {
            *value = left[row][0] * right[0][column] + left[row][1] * right[1][column];
        }
    }
    result
}

pub fn add(left: [[f64; 2]; 2], right: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
    [
        [left[0][0] + right[0][0], left[0][1] + right[0][1]],
        [left[1][0] + right[1][0], left[1][1] + right[1][1]],
    ]
}

pub fn max_abs_difference(left: [[f64; 2]; 2], right: [[f64; 2]; 2]) -> f64 {
    left.iter()
        .flatten()
        .zip(right.iter().flatten())
        .map(|(left, right)| (left - right).abs())
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters() -> OscillatorParameters {
        OscillatorParameters {
            angular_frequency: 2.0,
            damping_rate: 0.3,
            stationary_variance: 1.7,
        }
    }

    fn assert_matrix_close(left: [[f64; 2]; 2], right: [[f64; 2]; 2], tolerance: f64) {
        assert!(max_abs_difference(left, right) <= tolerance);
    }

    #[test]
    fn transition_matches_closed_form_for_known_parameters() {
        let transition = parameters().transition(0.5).unwrap();
        let attenuation = (-0.15_f64).exp();
        let (sin_phase, cos_phase) = 1.0_f64.sin_cos();
        assert!((transition.matrix[0][0] - attenuation * cos_phase).abs() < 1.0e-14);
        assert!((transition.matrix[0][1] + attenuation * sin_phase).abs() < 1.0e-14);
        assert!((transition.matrix[1][0] - attenuation * sin_phase).abs() < 1.0e-14);
        assert!((transition.matrix[1][1] - attenuation * cos_phase).abs() < 1.0e-14);
    }

    #[test]
    fn phase_advances_counterclockwise_under_declared_state_convention() {
        let transition = OscillatorParameters {
            angular_frequency: std::f64::consts::FRAC_PI_2,
            damping_rate: 0.0,
            stationary_variance: 1.0,
        }
        .transition(1.0)
        .unwrap();
        let next = transition.apply([1.0, 0.0]).unwrap();
        assert!(next[0].abs() < 1.0e-14);
        assert!((next[1] - 1.0).abs() < 1.0e-14);
    }

    #[test]
    fn process_covariance_is_exactly_stationary_under_isotropic_prior() {
        let params = parameters();
        let transition = params.transition(0.73).unwrap();
        let stationary = [
            [params.stationary_variance, 0.0],
            [0.0, params.stationary_variance],
        ];
        let propagated = transition.propagate_covariance(stationary);
        assert_matrix_close(propagated, stationary, 1.0e-12);
    }

    #[test]
    fn transition_mean_obeys_semigroup_composition() {
        let params = parameters();
        let dt_a = params.transition(0.17).unwrap();
        let dt_b = params.transition(0.41).unwrap();
        let dt_sum = params.transition(0.58).unwrap();

        let composed = multiply(dt_b.matrix, dt_a.matrix);
        assert_matrix_close(composed, dt_sum.matrix, 1.0e-12);
    }

    #[test]
    fn process_covariance_obeys_semigroup_composition() {
        let params = parameters();
        let dt_a = params.transition(0.17).unwrap();
        let dt_b = params.transition(0.41).unwrap();
        let dt_sum = params.transition(0.58).unwrap();

        let composed = add(
            dt_b.apply_covariance(dt_a.process_covariance),
            dt_b.process_covariance,
        );
        assert_matrix_close(composed, dt_sum.process_covariance, 1.0e-12);
    }

    #[test]
    fn very_small_elapsed_time_retains_nonzero_process_variance() {
        let transition = parameters().transition(1.0e-12).unwrap();
        assert!(transition.process_covariance[0][0] > 0.0);
        assert!(transition.process_covariance[1][1] > 0.0);
    }

    #[test]
    fn zero_damping_is_a_deterministic_rotation() {
        let transition = OscillatorParameters {
            angular_frequency: 3.0,
            damping_rate: 0.0,
            stationary_variance: 2.0,
        }
        .transition(0.7)
        .unwrap();
        assert_eq!(transition.process_covariance, [[0.0, 0.0], [0.0, 0.0]]);
        assert!(
            (transition.matrix[0][0].powi(2) + transition.matrix[1][0].powi(2) - 1.0).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn zero_frequency_is_a_damped_non_rotating_state() {
        let transition = OscillatorParameters {
            angular_frequency: 0.0,
            damping_rate: 0.5,
            stationary_variance: 1.0,
        }
        .transition(1.0)
        .unwrap();
        let attenuation = (-0.5_f64).exp();
        assert!((transition.matrix[0][0] - attenuation).abs() < 1.0e-14);
        assert_eq!(transition.matrix[0][1], 0.0);
        assert_eq!(transition.matrix[1][0], 0.0);
        assert!((transition.matrix[1][1] - attenuation).abs() < 1.0e-14);
    }

    #[test]
    fn rejects_invalid_parameters_and_elapsed_time() {
        let mut invalid = parameters();
        invalid.damping_rate = -0.1;
        assert_eq!(
            invalid.transition(0.1),
            Err(OscillatorError::InvalidDampingRate)
        );

        invalid = parameters();
        invalid.stationary_variance = 0.0;
        assert_eq!(
            invalid.transition(0.1),
            Err(OscillatorError::InvalidStationaryVariance)
        );

        let valid = parameters();
        assert_eq!(
            valid.transition(0.0),
            Err(OscillatorError::InvalidElapsedTime)
        );
        assert_eq!(
            valid.transition(f64::NAN),
            Err(OscillatorError::InvalidElapsedTime)
        );
    }

    #[test]
    fn rejects_nonfinite_state_and_phase_advance() {
        assert_eq!(
            parameters().transition(f64::MAX),
            Err(OscillatorError::NonFinitePhaseAdvance)
        );
        let transition = parameters().transition(0.1).unwrap();
        assert_eq!(
            transition.apply([f64::NAN, 0.0]),
            Err(OscillatorError::NonFiniteState)
        );
    }
}
