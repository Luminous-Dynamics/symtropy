//! Equation-level Rust reference port of the adaptive PID protection-level
//! controller described by Maharmeh, Resende & Nashashibi (Sensors 2026).
//!
//! This module implements the published control law from explicit precomputed
//! physical inputs. It does NOT implement the ES-EKF, LiDAR registration,
//! point-to-map residual extraction, Hessian construction, sensor calibration,
//! or paper route preprocessing. It is a reproducible controller baseline
//! harness, not yet a full paper reproduction or integrity certificate.

use std::f64::consts::PI;
use std::fmt;

/// Conditional anti-windup policy.
///
/// PaperConditional reproduces Equation (33) exactly: when the previous
/// integral state is inside the limit (or the error unwinds it), accept one
/// full discrete increment. This can overshoot the nominal threshold by one
/// time-step increment. HardClamped is a conservative implementation variant
/// that additionally clamps the new integral state to the threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegralAntiWindup {
    PaperConditional,
    HardClamped,
}

/// Published controller parameters plus explicit TIR inflation constant.
///
/// k_tir is deliberately caller-supplied. The paper states that it is derived
/// from the target integrity risk and its distributional convention, but the
/// exact convention must be fixed from the reproduction's chosen profile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptivePlConfig {
    pub base_imu_drift_mps: f64,
    pub drift_weight: f64,
    pub geometric_weight_mps: f64,
    pub target_integrity_risk: f64,
    pub k_tir: f64,
    pub max_protection_level_m: f64,
    pub saturation_exponent: f64,
    pub kp_base: f64,
    pub ki_base: f64,
    pub kd_base: f64,
    pub decay_modifier: f64,
    pub derivative_cutoff_hz: f64,
    pub integral_limit_fraction: f64,
    pub integral_anti_windup: IntegralAntiWindup,
    pub entropy_kp_scale: f64,
    pub entropy_ki_scale: f64,
    pub entropy_kd_scale: f64,
}

impl AdaptivePlConfig {
    /// The numerical parameters listed in the paper, with the distributional
    /// inflation multiplier supplied explicitly by the caller.
    pub fn paper_2026(k_tir: f64) -> Self {
        Self {
            base_imu_drift_mps: 0.05,
            drift_weight: 1.5,
            geometric_weight_mps: 0.5,
            target_integrity_risk: 1.0e-3,
            k_tir,
            max_protection_level_m: 10.0,
            saturation_exponent: 2.0,
            kp_base: 0.2,
            ki_base: 0.05,
            kd_base: 0.001,
            decay_modifier: 0.15,
            derivative_cutoff_hz: 5.0,
            integral_limit_fraction: 0.3,
            integral_anti_windup: IntegralAntiWindup::PaperConditional,
            entropy_kp_scale: 2.0,
            entropy_ki_scale: 0.8,
            entropy_kd_scale: 0.5,
        }
    }

    pub fn validate(self) -> Result<(), AdaptivePlError> {
        let nonnegative = [
            self.base_imu_drift_mps,
            self.drift_weight,
            self.geometric_weight_mps,
            self.kp_base,
            self.ki_base,
            self.kd_base,
            self.entropy_kp_scale,
        ];
        if nonnegative.iter().any(|value| !value.is_finite() || *value < 0.0) {
            return Err(AdaptivePlError::InvalidConfiguration);
        }
        if self.kp_base + self.ki_base + self.kd_base <= 0.0
            || !self.target_integrity_risk.is_finite()
            || !(0.0..0.5).contains(&self.target_integrity_risk)
            || !self.k_tir.is_finite()
            || self.k_tir <= 0.0
            || !self.max_protection_level_m.is_finite()
            || self.max_protection_level_m <= 0.0
            || !self.saturation_exponent.is_finite()
            || self.saturation_exponent <= 0.0
            || !self.decay_modifier.is_finite()
            || !(0.0..=1.0).contains(&self.decay_modifier)
            || !self.derivative_cutoff_hz.is_finite()
            || self.derivative_cutoff_hz <= 0.0
            || !self.integral_limit_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.integral_limit_fraction)
            || !self.entropy_ki_scale.is_finite()
            || !(0.0..1.0).contains(&self.entropy_ki_scale)
            || !self.entropy_kd_scale.is_finite()
            || !(0.0..1.0).contains(&self.entropy_kd_scale)
        {
            return Err(AdaptivePlError::InvalidConfiguration);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalDrift {
    pub longitudinal_m: f64,
    pub lateral_m: f64,
}

/// Project filter-vs-dead-reckoned displacement discrepancy into the local
/// heading frame, following the paper's Frenet along/cross-track projection.
pub fn project_directional_drift(
    filter_displacement_world_m: [f64; 2],
    imu_dead_reckoned_displacement_world_m: [f64; 2],
    heading_rad: f64,
) -> Result<DirectionalDrift, AdaptivePlError> {
    if !heading_rad.is_finite()
        || filter_displacement_world_m.iter().any(|v| !v.is_finite())
        || imu_dead_reckoned_displacement_world_m.iter().any(|v| !v.is_finite())
    {
        return Err(AdaptivePlError::InvalidObservation);
    }
    let dx = filter_displacement_world_m[0] - imu_dead_reckoned_displacement_world_m[0];
    let dy = filter_displacement_world_m[1] - imu_dead_reckoned_displacement_world_m[1];
    let (sin_heading, cos_heading) = heading_rad.sin_cos();
    let longitudinal = (dx * cos_heading + dy * sin_heading).abs();
    let lateral = (-dx * sin_heading + dy * cos_heading).abs();
    Ok(DirectionalDrift {
        longitudinal_m: longitudinal,
        lateral_m: lateral,
    })
}

/// The paper's geometric sensitivity factor xi = 1/sqrt(lambda_min + epsilon).
pub fn geometric_sensitivity_per_m(
    minimum_hessian_eigenvalue: f64,
    epsilon: f64,
) -> Result<f64, AdaptivePlError> {
    if !minimum_hessian_eigenvalue.is_finite()
        || minimum_hessian_eigenvalue < 0.0
        || !epsilon.is_finite()
        || epsilon <= 0.0
    {
        return Err(AdaptivePlError::InvalidObservation);
    }
    let sensitivity = 1.0 / (minimum_hessian_eigenvalue + epsilon).sqrt();
    sensitivity
        .is_finite()
        .then_some(sensitivity)
        .ok_or(AdaptivePlError::NonFiniteOutput)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnssBoundEvidence {
    /// Absolute projected innovation magnitude on this axis, in metres.
    pub projected_innovation_m: f64,
    /// GNSS standard deviation already projected to the same axis, in metres.
    pub projected_sigma_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptivePlInput {
    pub delta_time_s: f64,
    /// Absolute filter-vs-IMU dead-reckoning displacement discrepancy, metres.
    pub directional_drift_m: f64,
    /// LiDAR point-to-map registration residual supplied by the frontend, metres.
    pub geometric_residual_m: f64,
    /// Inverse-metre sensitivity derived from the registration Hessian.
    pub geometric_sensitivity_per_m: f64,
    /// Normalized Shannon entropy of the LiDAR residual histogram, in [0,1].
    pub normalized_environment_entropy: f64,
    /// Optional projected GNSS innovation / noise evidence for this axis.
    pub gnss_bound: Option<GnssBoundEvidence>,
}

impl AdaptivePlInput {
    fn validate(self) -> Result<(), AdaptivePlError> {
        if !self.delta_time_s.is_finite()
            || self.delta_time_s <= 0.0
            || !self.directional_drift_m.is_finite()
            || self.directional_drift_m < 0.0
            || !self.geometric_residual_m.is_finite()
            || self.geometric_residual_m < 0.0
            || !self.geometric_sensitivity_per_m.is_finite()
            || self.geometric_sensitivity_per_m < 0.0
            || !self.normalized_environment_entropy.is_finite()
            || !(0.0..=1.0).contains(&self.normalized_environment_entropy)
        {
            return Err(AdaptivePlError::InvalidObservation);
        }
        if let Some(gnss) = self.gnss_bound {
            if !gnss.projected_innovation_m.is_finite()
                || gnss.projected_innovation_m < 0.0
                || !gnss.projected_sigma_m.is_finite()
                || gnss.projected_sigma_m < 0.0
            {
                return Err(AdaptivePlError::InvalidObservation);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptivePlGains {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptivePlStep {
    pub instantaneous_error_rate_mps: f64,
    pub ideal_setpoint_m: f64,
    pub tracking_error_m: f64,
    pub normalized_environment_entropy: f64,
    pub gains: AdaptivePlGains,
    pub integral_state_m_s: f64,
    pub derivative_state_mps: f64,
    pub controller_command_mps: f64,
    pub applied_protection_level_delta_m: f64,
    pub gnss_candidate_bound_m: Option<f64>,
    pub protection_level_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptivePlState {
    ideal_setpoint_m: f64,
    protection_level_m: f64,
    previous_tracking_error_m: f64,
    integral_state_m_s: f64,
    derivative_state_mps: f64,
}

impl AdaptivePlState {
    pub fn new(
        config: AdaptivePlConfig,
        initial_setpoint_m: f64,
        initial_protection_level_m: f64,
    ) -> Result<Self, AdaptivePlError> {
        config.validate()?;
        if !initial_setpoint_m.is_finite()
            || !initial_protection_level_m.is_finite()
            || initial_setpoint_m < 0.0
            || initial_protection_level_m < 0.0
            || initial_setpoint_m > config.max_protection_level_m
            || initial_protection_level_m > config.max_protection_level_m
        {
            return Err(AdaptivePlError::InvalidInitialState);
        }
        Ok(Self {
            ideal_setpoint_m: initial_setpoint_m,
            protection_level_m: initial_protection_level_m,
            previous_tracking_error_m: 0.0,
            integral_state_m_s: 0.0,
            derivative_state_mps: 0.0,
        })
    }

    pub fn ideal_setpoint_m(&self) -> f64 {
        self.ideal_setpoint_m
    }

    pub fn protection_level_m(&self) -> f64 {
        self.protection_level_m
    }

    pub fn integral_state_m_s(&self) -> f64 {
        self.integral_state_m_s
    }

    pub fn derivative_state_mps(&self) -> f64 {
        self.derivative_state_mps
    }

    /// One equation-level controller update.
    ///
    /// The frontend is responsible for supplying correctly synchronized
    /// displacement, residual, Hessian sensitivity and GNSS innovation values.
    /// This function does not infer their provenance or independence.
    pub fn step(
        &mut self,
        config: AdaptivePlConfig,
        input: AdaptivePlInput,
    ) -> Result<AdaptivePlStep, AdaptivePlError> {
        config.validate()?;
        input.validate()?;

        let dt = input.delta_time_s;
        let entropy = input.normalized_environment_entropy;

        // Eq. (13): additive conservative instantaneous error-growth rate.
        let error_rate = config.base_imu_drift_mps
            + config.drift_weight * (input.directional_drift_m / dt)
            + config.geometric_weight_mps
                * input.geometric_residual_m
                * input.geometric_sensitivity_per_m;
        if !error_rate.is_finite() {
            return Err(AdaptivePlError::NonFiniteOutput);
        }

        // Eqs. (14)-(16): bounded ideal setpoint evolution.
        let normalized_setpoint =
            (self.ideal_setpoint_m / config.max_protection_level_m).clamp(0.0, 1.0);
        let saturation =
            (1.0 - normalized_setpoint.powf(config.saturation_exponent)).clamp(0.0, 1.0);
        let mut next_setpoint = (self.ideal_setpoint_m + saturation * error_rate * dt)
            .clamp(0.0, config.max_protection_level_m);

        // Eqs. (17)-(21): intersect with GNSS-derived candidate bound.
        let gnss_candidate_bound = input.gnss_bound.map(|gnss| {
            gnss.projected_innovation_m + config.k_tir * gnss.projected_sigma_m
        });
        if let Some(bound) = gnss_candidate_bound {
            next_setpoint = next_setpoint.min(bound);
        }
        next_setpoint = next_setpoint.clamp(0.0, config.max_protection_level_m);

        // Eq. (27): PID tracks the ideal setpoint with a separate deployed PL.
        let tracking_error = next_setpoint - self.protection_level_m;

        // Eqs. (29)-(31): entropy-based gain scheduling.
        let gains = AdaptivePlGains {
            kp: config.kp_base * (1.0 + config.entropy_kp_scale * entropy),
            ki: config.ki_base * (1.0 - config.entropy_ki_scale * entropy),
            kd: config.kd_base * (1.0 - config.entropy_kd_scale * entropy),
        };

        // Eq. (33): conditional integration anti-windup.
        let integral_limit =
            config.integral_limit_fraction * config.max_protection_level_m;
        let should_integrate = self.integral_state_m_s.abs() < integral_limit
            || tracking_error * self.integral_state_m_s < 0.0;
        let candidate_integral = if should_integrate {
            self.integral_state_m_s + tracking_error * dt
        } else {
            self.integral_state_m_s
        };
        let next_integral = match config.integral_anti_windup {
            IntegralAntiWindup::PaperConditional => candidate_integral,
            IntegralAntiWindup::HardClamped => {
                candidate_integral.clamp(-integral_limit, integral_limit)
            }
        };

        // Eqs. (34)-(35): low-pass filtered derivative.
        // Exact first-order low-pass coefficient from the paper:
        // alpha_D = exp(-2*pi*f_c*dt); old derivative gets alpha_D and the
        // finite-difference derivative gets (1-alpha_D).
        let derivative_alpha =
            (-2.0 * PI * config.derivative_cutoff_hz * dt).exp();
        let raw_derivative =
            (tracking_error - self.previous_tracking_error_m) / dt;
        let next_derivative = derivative_alpha * self.derivative_state_mps
            + (1.0 - derivative_alpha) * raw_derivative;

        // Eq. (32): PID command (m/s).
        let command = gains.kp * tracking_error
            + gains.ki * next_integral
            + gains.kd * next_derivative;
        if !next_integral.is_finite()
            || !next_derivative.is_finite()
            || !command.is_finite()
        {
            return Err(AdaptivePlError::NonFiniteOutput);
        }

        // Eq. (39): fast expansion and cautious contraction.
        let applied_rate = if command >= 0.0 {
            command
        } else {
            config.decay_modifier * command
        };
        let requested_delta = applied_rate * dt;

        // Eq. (37): lower noise floor and absolute operational PL ceiling.
        let minimum_pl = (config.k_tir * config.base_imu_drift_mps * dt)
            .min(config.max_protection_level_m);
        let next_pl = (self.protection_level_m + requested_delta)
            .clamp(minimum_pl, config.max_protection_level_m);
        let applied_delta = next_pl - self.protection_level_m;

        if !next_pl.is_finite() || !next_setpoint.is_finite() {
            return Err(AdaptivePlError::NonFiniteOutput);
        }

        self.ideal_setpoint_m = next_setpoint;
        self.protection_level_m = next_pl;
        self.previous_tracking_error_m = tracking_error;
        self.integral_state_m_s = next_integral;
        self.derivative_state_mps = next_derivative;

        Ok(AdaptivePlStep {
            instantaneous_error_rate_mps: error_rate,
            ideal_setpoint_m: next_setpoint,
            tracking_error_m: tracking_error,
            normalized_environment_entropy: entropy,
            gains,
            integral_state_m_s: next_integral,
            derivative_state_mps: next_derivative,
            controller_command_mps: command,
            applied_protection_level_delta_m: applied_delta,
            gnss_candidate_bound_m: gnss_candidate_bound,
            protection_level_m: next_pl,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AdaptivePlError {
    InvalidConfiguration,
    InvalidObservation,
    InvalidInitialState,
    NonFiniteOutput,
}

impl fmt::Display for AdaptivePlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration => write!(f, "adaptive PL controller configuration is invalid"),
            Self::InvalidObservation => write!(f, "adaptive PL controller input is invalid"),
            Self::InvalidInitialState => write!(f, "initial setpoint/PL state is invalid"),
            Self::NonFiniteOutput => write!(f, "adaptive PL controller produced a non-finite result"),
        }
    }
}
impl std::error::Error for AdaptivePlError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AdaptivePlConfig {
        AdaptivePlConfig::paper_2026(3.0)
    }

    fn input(entropy: f64) -> AdaptivePlInput {
        AdaptivePlInput {
            delta_time_s: 0.1,
            directional_drift_m: 0.0,
            geometric_residual_m: 0.0,
            geometric_sensitivity_per_m: 0.0,
            normalized_environment_entropy: entropy,
            gnss_bound: None,
        }
    }

    #[test]
    fn paper_2026_parameter_profile_matches_published_values() {
        let cfg = config();
        assert_eq!(cfg.base_imu_drift_mps, 0.05);
        assert_eq!(cfg.drift_weight, 1.5);
        assert_eq!(cfg.geometric_weight_mps, 0.5);
        assert_eq!(cfg.target_integrity_risk, 1.0e-3);
        assert_eq!(cfg.max_protection_level_m, 10.0);
        assert_eq!(cfg.saturation_exponent, 2.0);
        assert_eq!(cfg.kp_base, 0.2);
        assert_eq!(cfg.ki_base, 0.05);
        assert_eq!(cfg.kd_base, 0.001);
        assert_eq!(cfg.decay_modifier, 0.15);
        assert_eq!(cfg.derivative_cutoff_hz, 5.0);
        assert_eq!(cfg.integral_limit_fraction, 0.3);
        assert_eq!(cfg.entropy_kp_scale, 2.0);
        assert_eq!(cfg.entropy_ki_scale, 0.8);
        assert_eq!(cfg.entropy_kd_scale, 0.5);
    }

    #[test]
    fn entropy_scheduling_matches_published_linear_gain_laws() {
        let mut state = AdaptivePlState::new(config(), 2.0, 1.0).unwrap();
        let step = state.step(config(), input(1.0)).unwrap();
        assert!((step.gains.kp - 0.6).abs() < 1.0e-12);
        assert!((step.gains.ki - 0.01).abs() < 1.0e-12);
        assert!((step.gains.kd - 0.0005).abs() < 1.0e-12);
    }

    #[test]
    fn derivative_filter_uses_exact_exponential_cutoff_coefficient() {
        let mut cfg = config();
        cfg.base_imu_drift_mps = 0.0;
        cfg.drift_weight = 0.0;
        cfg.geometric_weight_mps = 0.0;
        cfg.kp_base = 0.0;
        cfg.ki_base = 1.0e-12;
        cfg.kd_base = 1.0;
        cfg.derivative_cutoff_hz = 5.0;
        let mut state = AdaptivePlState::new(cfg, 2.0, 1.0).unwrap();
        let mut sample = input(0.0);
        sample.delta_time_s = 0.01;
        let dt = sample.delta_time_s;
        let expected_alpha = (-2.0 * PI * cfg.derivative_cutoff_hz * dt).exp();
        let prior_derivative = state.derivative_state_mps();
        let prior_error = state.previous_tracking_error_m();
        let initial_error = state.ideal_setpoint_m() - state.protection_level_m();
        let raw = (initial_error - prior_error) / dt;
        let expected = expected_alpha * prior_derivative + (1.0 - expected_alpha) * raw;
        let output = state.step(cfg, sample).unwrap();
        assert!((output.derivative_state_mps - expected).abs() < 1.0e-10);
    }

    #[test]
    fn geometric_degeneracy_increases_sensitivity() {
        let well_conditioned = geometric_sensitivity_per_m(4.0, 1.0e-12).unwrap();
        let degenerate = geometric_sensitivity_per_m(0.01, 1.0e-12).unwrap();
        assert!(degenerate > well_conditioned * 10.0);
    }

    #[test]
    fn directional_drift_projects_into_heading_frame() {
        let drift = project_directional_drift([1.0, 0.5], [0.4, 0.1], 0.0).unwrap();
        assert!((drift.longitudinal_m - 0.6).abs() < 1.0e-12);
        assert!((drift.lateral_m - 0.4).abs() < 1.0e-12);

        let rotated = project_directional_drift([1.0, 0.5], [0.4, 0.1], PI / 2.0).unwrap();
        assert!((rotated.longitudinal_m - 0.4).abs() < 1.0e-12);
        assert!((rotated.lateral_m - 0.6).abs() < 1.0e-12);
    }

    #[test]
    fn instantaneous_error_rate_contains_three_published_terms() {
        let mut cfg = config();
        cfg.base_imu_drift_mps = 0.1;
        cfg.drift_weight = 2.0;
        cfg.geometric_weight_mps = 3.0;
        let mut state = AdaptivePlState::new(cfg, 1.0, 1.0).unwrap();
        let mut sample = input(0.0);
        sample.directional_drift_m = 0.2;
        sample.geometric_residual_m = 0.5;
        sample.geometric_sensitivity_per_m = 4.0;
        let step = state.step(cfg, sample).unwrap();
        // 0.1 + 2*(0.2/0.1) + 3*0.5*4 = 10.1 m/s.
        assert!((step.instantaneous_error_rate_mps - 10.1).abs() < 1.0e-12);
    }

    #[test]
    fn gnss_candidate_bound_only_tightens_the_ideal_setpoint() {
        let mut state = AdaptivePlState::new(config(), 2.0, 1.0).unwrap();
        let mut sample = input(0.0);
        sample.gnss_bound = Some(GnssBoundEvidence {
            projected_innovation_m: 0.2,
            projected_sigma_m: 0.1,
        });
        let step = state.step(config(), sample).unwrap();
        assert!((step.gnss_candidate_bound_m.unwrap() - 0.5).abs() < 1.0e-12);
        assert!(step.ideal_setpoint_m <= 0.5);

        let mut state = AdaptivePlState::new(config(), 2.0, 1.0).unwrap();
        let mut sample = input(0.0);
        sample.gnss_bound = Some(GnssBoundEvidence {
            projected_innovation_m: 100.0,
            projected_sigma_m: 0.1,
        });
        let step = state.step(config(), sample).unwrap();
        assert!(step.ideal_setpoint_m > 1.0);
    }

    #[test]
    fn protection_level_expands_faster_than_it_contracts() {
        let mut cfg = config();
        cfg.base_imu_drift_mps = 0.0;
        cfg.drift_weight = 0.0;
        cfg.geometric_weight_mps = 0.0;
        cfg.kp_base = 0.2;
        cfg.ki_base = 0.0;
        cfg.kd_base = 0.0;
        cfg.derivative_cutoff_hz = 1.0;

        let mut rising = AdaptivePlState::new(cfg, 2.0, 1.0).unwrap();
        let rise = rising.step(cfg, input(0.0)).unwrap();

        let mut falling = AdaptivePlState::new(cfg, 0.0, 1.0).unwrap();
        let fall = falling.step(cfg, input(0.0)).unwrap();
        assert!((rise.applied_protection_level_delta_m - 0.02).abs() < 1.0e-12);
        assert!((fall.applied_protection_level_delta_m + 0.003).abs() < 1.0e-12);
        assert!((fall.applied_protection_level_delta_m.abs()
            / rise.applied_protection_level_delta_m.abs()
            - cfg.decay_modifier).abs() < 1.0e-12);
    }

    #[test]
    fn saturation_and_absolute_pl_ceiling_bound_setpoint_and_pl() {
        let mut cfg = config();
        cfg.base_imu_drift_mps = 1000.0;
        let mut state = AdaptivePlState::new(cfg, 9.9, 9.9).unwrap();
        let step = state.step(cfg, input(0.0)).unwrap();
        assert!(step.ideal_setpoint_m <= cfg.max_protection_level_m);
        assert!(step.protection_level_m <= cfg.max_protection_level_m);
    }

    #[test]
    fn paper_conditional_anti_windup_stops_after_one_discrete_threshold_crossing() {
        let mut cfg = config();
        cfg.base_imu_drift_mps = 0.0;
        cfg.drift_weight = 0.0;
        cfg.geometric_weight_mps = 0.0;
        cfg.kp_base = 0.0;
        cfg.kd_base = 0.0;
        cfg.ki_base = 1.0e-12;
        cfg.integral_anti_windup = IntegralAntiWindup::PaperConditional;
        let mut state = AdaptivePlState::new(cfg, 10.0, 0.0).unwrap();
        let mut sample = input(0.0);
        sample.delta_time_s = 0.1;
        for _ in 0..1000 {
            state.step(cfg, sample).unwrap();
        }
        let limit = cfg.integral_limit_fraction * cfg.max_protection_level_m;
        // The paper's conditional rule accepts one full discrete update
        // when the previous state is below the threshold, so a one-step
        // overshoot is permitted. It must then stop integrating.
        assert!(state.integral_state_m_s() >= limit - 1.0e-9);
        assert!(state.integral_state_m_s() <= limit + 1.0 + 1.0e-9);
    }

    #[test]
    fn paper_conditional_rule_can_overshoot_by_one_increment_but_does_not_wind_up() {
        let mut cfg = config();
        cfg.base_imu_drift_mps = 0.0;
        cfg.drift_weight = 0.0;
        cfg.geometric_weight_mps = 0.0;
        cfg.kp_base = 0.0;
        cfg.kd_base = 0.0;
        cfg.ki_base = 1.0e-12;
        cfg.integral_anti_windup = IntegralAntiWindup::PaperConditional;
        let mut state = AdaptivePlState::new(cfg, 10.0, 0.0).unwrap();
        let mut sample = input(0.0);
        sample.delta_time_s = 0.4;
        let first = state.step(cfg, sample).unwrap();
        let limit = cfg.integral_limit_fraction * cfg.max_protection_level_m;
        assert!(first.integral_state_m_s > limit);
        for _ in 0..100 {
            state.step(cfg, sample).unwrap();
        }
        assert!((state.integral_state_m_s() - first.integral_state_m_s).abs() < 1.0e-9);
    }

    #[test]
    fn hard_clamped_anti_windup_never_exceeds_integral_limit() {
        let mut cfg = config();
        cfg.base_imu_drift_mps = 0.0;
        cfg.drift_weight = 0.0;
        cfg.geometric_weight_mps = 0.0;
        cfg.kp_base = 0.0;
        cfg.kd_base = 0.0;
        cfg.ki_base = 1.0e-12;
        cfg.integral_anti_windup = IntegralAntiWindup::HardClamped;
        let mut state = AdaptivePlState::new(cfg, 10.0, 0.0).unwrap();
        let mut sample = input(0.0);
        sample.delta_time_s = 0.4;
        for _ in 0..100 {
            state.step(cfg, sample).unwrap();
        }
        let limit = cfg.integral_limit_fraction * cfg.max_protection_level_m;
        assert!(state.integral_state_m_s().abs() <= limit + 1.0e-9);
    }

    #[test]
    fn invalid_observations_and_configs_fail_closed_without_mutation() {
        let cfg = config();
        let mut state = AdaptivePlState::new(cfg, 2.0, 1.0).unwrap();
        let before = state;
        let mut invalid = input(0.0);
        invalid.delta_time_s = f64::NAN;
        assert_eq!(state.step(cfg, invalid), Err(AdaptivePlError::InvalidObservation));
        assert_eq!(state, before);

        let mut invalid = cfg;
        invalid.entropy_ki_scale = 1.2;
        assert_eq!(state.step(invalid, input(0.0)), Err(AdaptivePlError::InvalidConfiguration));
        assert_eq!(state, before);
        assert!(AdaptivePlState::new(cfg, 11.0, 1.0).is_err());
    }
}
