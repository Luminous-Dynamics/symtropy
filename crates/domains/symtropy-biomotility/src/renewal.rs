// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit waiting-time laws and right-censor-aware survival estimates.
//!
//! The porous-agar reference uses exponential/double-exponential run
//! survival and a piecewise power-law dwell-time density. This module keeps
//! those assumptions typed and parameterized; it does not supply fitted
//! biological parameters. Those must come from an admitted source dataset.

use std::error::Error;
use std::fmt;

use super::{SampleSet, SeededRng};

/// A validated family of positive waiting-time distributions.
#[derive(Debug, Clone, PartialEq)]
pub enum WaitingTimeLaw {
    /// Non-parametric empirical resampling from positive observed durations.
    Empirical(SampleSet),
    /// Exponential duration distribution parameterized by its mean.
    Exponential { mean_s: f64 },
    /// Mixture survival S(t) = w exp(-t/tau_fast)
    /// + (1-w) exp(-t/tau_slow).
    DoubleExponential {
        fast_weight: f64,
        fast_mean_s: f64,
        slow_mean_s: f64,
    },
    /// Continuous piecewise power-law density:
    /// psi(t) ~ t^-alpha for min_s <= t <= crossover_s,
    /// psi(t) ~ t^-beta beyond crossover_s.
    ///
    /// The second branch is scaled to make the density continuous.
    /// alpha in (0,1) and beta > 2 provide a finite mean.
    PiecewisePowerLaw {
        alpha: f64,
        beta: f64,
        min_s: f64,
        crossover_s: f64,
    },
}

impl WaitingTimeLaw {
    pub fn empirical(samples_s: Vec<f64>) -> Self {
        Self::Empirical(SampleSet::new(samples_s))
    }

    pub fn validate(&self) -> Result<(), WaitingTimeLawError> {
        match self {
            Self::Empirical(samples) => {
                if samples.is_empty() {
                    return Err(WaitingTimeLawError::EmptyEmpiricalSamples);
                }
                if samples
                    .values()
                    .iter()
                    .any(|value| !value.is_finite() || *value <= 0.0)
                {
                    return Err(WaitingTimeLawError::InvalidEmpiricalSample);
                }
            }
            Self::Exponential { mean_s } => {
                if !mean_s.is_finite() || *mean_s <= 0.0 {
                    return Err(WaitingTimeLawError::InvalidTimeScale);
                }
            }
            Self::DoubleExponential {
                fast_weight,
                fast_mean_s,
                slow_mean_s,
            } => {
                if !fast_weight.is_finite() || !(0.0..=1.0).contains(fast_weight) {
                    return Err(WaitingTimeLawError::InvalidMixtureWeight);
                }
                if !fast_mean_s.is_finite()
                    || *fast_mean_s <= 0.0
                    || !slow_mean_s.is_finite()
                    || *slow_mean_s <= 0.0
                {
                    return Err(WaitingTimeLawError::InvalidTimeScale);
                }
            }
            Self::PiecewisePowerLaw {
                alpha,
                beta,
                min_s,
                crossover_s,
            } => {
                if !alpha.is_finite() || !(0.0..1.0).contains(alpha) {
                    return Err(WaitingTimeLawError::InvalidPowerLawExponent);
                }
                if !beta.is_finite() || *beta <= 2.0 {
                    return Err(WaitingTimeLawError::InvalidPowerLawTailExponent);
                }
                if !min_s.is_finite()
                    || *min_s <= 0.0
                    || !crossover_s.is_finite()
                    || *crossover_s <= *min_s
                {
                    return Err(WaitingTimeLawError::InvalidPowerLawBounds);
                }
            }
        }
        Ok(())
    }

    /// Probability density at t (seconds).
    ///
    /// Returns None for empirical laws, whose density is not defined by the
    /// discrete sample representation, and for non-finite query times.
    pub fn density(&self, t_s: f64) -> Option<f64> {
        if !t_s.is_finite() || self.validate().is_err() {
            return None;
        }
        match self {
            Self::Empirical(_) => None,
            Self::Exponential { mean_s } => {
                if t_s < 0.0 {
                    Some(0.0)
                } else {
                    Some((-t_s / mean_s).exp() / mean_s)
                }
            }
            Self::DoubleExponential {
                fast_weight,
                fast_mean_s,
                slow_mean_s,
            } => {
                if t_s < 0.0 {
                    return Some(0.0);
                }
                let fast = (-t_s / fast_mean_s).exp() / fast_mean_s;
                let slow = (-t_s / slow_mean_s).exp() / slow_mean_s;
                Some(fast_weight * fast + (1.0 - fast_weight) * slow)
            }
            Self::PiecewisePowerLaw {
                alpha,
                beta,
                min_s,
                crossover_s,
            } => {
                if t_s < *min_s {
                    return Some(0.0);
                }
                let normalizer = piecewise_normalizer(*alpha, *beta, *min_s, *crossover_s);
                let unnormalized = if t_s <= *crossover_s {
                    t_s.powf(-alpha)
                } else {
                    crossover_s.powf(beta - alpha) * t_s.powf(-beta)
                };
                Some(unnormalized / normalizer)
            }
        }
    }

    /// Cumulative distribution F(t) = P(T <= t).
    pub fn cdf(&self, t_s: f64) -> Option<f64> {
        if !t_s.is_finite() || self.validate().is_err() {
            return None;
        }
        match self {
            Self::Empirical(samples) => {
                if t_s < 0.0 {
                    return Some(0.0);
                }
                let count = samples
                    .values()
                    .iter()
                    .filter(|sample| **sample <= t_s)
                    .count();
                Some(count as f64 / samples.len() as f64)
            }
            Self::Exponential { mean_s } => {
                if t_s <= 0.0 {
                    Some(0.0)
                } else {
                    Some(1.0 - (-t_s / mean_s).exp())
                }
            }
            Self::DoubleExponential {
                fast_weight,
                fast_mean_s,
                slow_mean_s,
            } => {
                if t_s <= 0.0 {
                    return Some(0.0);
                }
                let survival = fast_weight * (-t_s / fast_mean_s).exp()
                    + (1.0 - fast_weight) * (-t_s / slow_mean_s).exp();
                Some((1.0 - survival).clamp(0.0, 1.0))
            }
            Self::PiecewisePowerLaw {
                alpha,
                beta,
                min_s,
                crossover_s,
            } => {
                if t_s < *min_s {
                    return Some(0.0);
                }
                let first_mass = piecewise_first_mass(*alpha, *min_s, *crossover_s);
                let normalizer = piecewise_normalizer(*alpha, *beta, *min_s, *crossover_s);
                let cumulative_mass = if t_s <= *crossover_s {
                    power_integral(*alpha, *min_s, t_s)
                } else {
                    let tail_scale = crossover_s.powf(beta - alpha);
                    first_mass
                        + tail_scale
                            * (crossover_s.powf(1.0 - beta) - t_s.powf(1.0 - beta))
                            / (beta - 1.0)
                };
                Some((cumulative_mass / normalizer).clamp(0.0, 1.0))
            }
        }
    }

    /// Survival function S(t) = P(T > t).
    pub fn survival(&self, t_s: f64) -> Option<f64> {
        if !t_s.is_finite() {
            return None;
        }
        if t_s < 0.0 {
            return Some(1.0);
        }
        self.cdf(t_s).map(|cdf| (1.0 - cdf).clamp(0.0, 1.0))
    }

    /// Analytic mean duration when finite and defined.
    pub fn mean_s(&self) -> Option<f64> {
        self.validate().ok()?;
        match self {
            Self::Empirical(samples) if !samples.is_empty() => {
                Some(samples.values().iter().sum::<f64>() / samples.len() as f64)
            }
            Self::Empirical(_) => None,
            Self::Exponential { mean_s } => Some(*mean_s),
            Self::DoubleExponential {
                fast_weight,
                fast_mean_s,
                slow_mean_s,
            } => Some(fast_weight * fast_mean_s + (1.0 - fast_weight) * slow_mean_s),
            Self::PiecewisePowerLaw {
                alpha,
                beta,
                min_s,
                crossover_s,
            } => {
                if self.validate().is_err() {
                    return None;
                }
                let first_moment =
                    (crossover_s.powf(2.0 - alpha) - min_s.powf(2.0 - alpha))
                        / (2.0 - alpha);
                let tail_moment =
                    crossover_s.powf(2.0 - alpha) / (beta - 2.0);
                let normalizer =
                    piecewise_normalizer(*alpha, *beta, *min_s, *crossover_s);
                Some((first_moment + tail_moment) / normalizer)
            }
        }
    }

    /// Deterministically draw one sample using a local seed.
    pub fn sample_with_seed(&self, seed: u64) -> Result<f64, WaitingTimeLawError> {
        self.validate()?;
        let mut rng = SeededRng::new(seed);
        Ok(self.sample_with_rng(&mut rng))
    }

    /// Deterministically draw n samples using one continuous seeded stream.
    pub fn sample_many(
        &self,
        count: usize,
        seed: u64,
    ) -> Result<Vec<f64>, WaitingTimeLawError> {
        self.validate()?;
        let mut rng = SeededRng::new(seed);
        Ok((0..count)
            .map(|_| self.sample_with_rng(&mut rng))
            .collect())
    }

    pub(crate) fn sample_with_rng(&self, rng: &mut SeededRng) -> f64 {
        match self {
            Self::Empirical(samples) => samples.sample(rng),
            Self::Exponential { mean_s } => {
                let unit = open_unit(rng.next_unit_f64());
                -mean_s * (1.0 - unit).ln()
            }
            Self::DoubleExponential {
                fast_weight,
                fast_mean_s,
                slow_mean_s,
            } => {
                let unit = rng.next_unit_f64();
                let mean_s = if unit < *fast_weight {
                    *fast_mean_s
                } else {
                    *slow_mean_s
                };
                let open = open_unit(rng.next_unit_f64());
                -mean_s * (1.0 - open).ln()
            }
            Self::PiecewisePowerLaw {
                alpha,
                beta,
                min_s,
                crossover_s,
            } => {
                let normalizer = piecewise_normalizer(*alpha, *beta, *min_s, *crossover_s);
                let first_mass = piecewise_first_mass(*alpha, *min_s, *crossover_s);
                let target = open_unit(rng.next_unit_f64()) * normalizer;
                if target <= first_mass {
                    (min_s.powf(1.0 - alpha)
                        + target * (1.0 - alpha))
                        .powf(1.0 / (1.0 - alpha))
                } else {
                    let tail_mass = target - first_mass;
                    let tail_scale = crossover_s.powf(beta - alpha);
                    let inverse_power = crossover_s.powf(1.0 - beta)
                        - tail_mass * (beta - 1.0) / tail_scale;
                    inverse_power
                        .max(f64::MIN_POSITIVE)
                        .powf(1.0 / (1.0 - beta))
                }
            }
        }
    }
}

/// Direction-conditioned waiting-time laws for run-time bias experiments.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectionalWaitingTimes {
    pub up_gradient: WaitingTimeLaw,
    pub cross_gradient: WaitingTimeLaw,
    pub down_gradient: WaitingTimeLaw,
}

impl DirectionalWaitingTimes {
    pub fn for_direction(&self, direction: super::DirectionClass) -> &WaitingTimeLaw {
        match direction {
            super::DirectionClass::UpGradient => &self.up_gradient,
            super::DirectionClass::CrossGradient => &self.cross_gradient,
            super::DirectionClass::DownGradient => &self.down_gradient,
        }
    }

    pub fn validate(&self) -> Result<(), WaitingTimeLawError> {
        self.up_gradient.validate()?;
        self.cross_gradient.validate()?;
        self.down_gradient.validate()?;
        Ok(())
    }

    pub(crate) fn sample_with_rng(
        &self,
        direction: super::DirectionClass,
        rng: &mut SeededRng,
    ) -> f64 {
        self.for_direction(direction).sample_with_rng(rng)
    }
}

/// Waiting-time observations, including right-censored episodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurvivalOutcome {
    EventObserved,
    RightCensored,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurvivalObservation {
    pub duration_s: f64,
    pub outcome: SurvivalOutcome,
}

/// One Kaplan–Meier product-limit step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurvivalPoint {
    pub time_s: f64,
    pub at_risk: usize,
    pub observed_events: usize,
    pub right_censored: usize,
    pub survival_probability: f64,
    /// Greenwood standard error; None when the estimate reaches zero.
    pub greenwood_standard_error: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurvivalCurve {
    pub observations: usize,
    pub observed_event_count: usize,
    pub right_censored_count: usize,
    pub points: Vec<SurvivalPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurvivalError {
    EmptyObservations,
    InvalidDuration,
}

impl fmt::Display for SurvivalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyObservations => {
                write!(formatter, "survival estimation requires observations")
            }
            Self::InvalidDuration => {
                write!(formatter, "survival durations must be finite and positive")
            }
        }
    }
}

impl Error for SurvivalError {}

/// Calculate a Kaplan–Meier survival curve for event times with right censoring.
///
/// At tied times, observed events are applied to the risk set before censors
/// are removed. Independent cluster identifiers are intentionally kept outside
/// this function so the caller can bootstrap at the recording/plate/day level.
pub fn kaplan_meier(
    observations: &[SurvivalObservation],
) -> Result<SurvivalCurve, SurvivalError> {
    if observations.is_empty() {
        return Err(SurvivalError::EmptyObservations);
    }
    if observations
        .iter()
        .any(|item| !item.duration_s.is_finite() || item.duration_s <= 0.0)
    {
        return Err(SurvivalError::InvalidDuration);
    }

    let mut sorted = observations.to_vec();
    sorted.sort_by(|left, right| left.duration_s.total_cmp(&right.duration_s));

    let observed_event_count = observations
        .iter()
        .filter(|item| item.outcome == SurvivalOutcome::EventObserved)
        .count();
    let right_censored_count = observations.len() - observed_event_count;

    let mut points = Vec::new();
    let mut at_risk = observations.len();
    let mut survival = 1.0;
    let mut greenwood_sum = 0.0;
    let mut index = 0;

    while index < sorted.len() {
        let time_s = sorted[index].duration_s;
        let mut events_at_time = 0;
        let mut censored_at_time = 0;
        let mut end = index;

        while end < sorted.len() && sorted[end].duration_s == time_s {
            match sorted[end].outcome {
                SurvivalOutcome::EventObserved => events_at_time += 1,
                SurvivalOutcome::RightCensored => censored_at_time += 1,
            }
            end += 1;
        }

        let risk_before_time = at_risk;
        if events_at_time > 0 {
            survival *=
                (risk_before_time - events_at_time) as f64 / risk_before_time as f64;
            if risk_before_time > events_at_time {
                greenwood_sum += events_at_time as f64
                    / (risk_before_time * (risk_before_time - events_at_time)) as f64;
            }
        }

        let greenwood_standard_error = if survival > 0.0 {
            Some(survival * greenwood_sum.sqrt())
        } else {
            None
        };

        points.push(SurvivalPoint {
            time_s,
            at_risk: risk_before_time,
            observed_events: events_at_time,
            right_censored: censored_at_time,
            survival_probability: survival,
            greenwood_standard_error,
        });

        at_risk -= events_at_time + censored_at_time;
        index = end;
    }

    Ok(SurvivalCurve {
        observations: observations.len(),
        observed_event_count,
        right_censored_count,
        points,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingTimeLawError {
    EmptyEmpiricalSamples,
    InvalidEmpiricalSample,
    InvalidTimeScale,
    InvalidMixtureWeight,
    InvalidPowerLawExponent,
    InvalidPowerLawTailExponent,
    InvalidPowerLawBounds,
}

impl fmt::Display for WaitingTimeLawError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEmpiricalSamples => {
                write!(formatter, "empirical waiting-time law needs samples")
            }
            Self::InvalidEmpiricalSample => {
                write!(formatter, "empirical durations must be finite and positive")
            }
            Self::InvalidTimeScale => {
                write!(formatter, "waiting-time means must be finite and positive")
            }
            Self::InvalidMixtureWeight => {
                write!(formatter, "double-exponential weight must be in [0, 1]")
            }
            Self::InvalidPowerLawExponent => {
                write!(formatter, "piecewise power-law alpha must be in (0, 1)")
            }
            Self::InvalidPowerLawTailExponent => {
                write!(formatter, "piecewise power-law beta must be greater than 2")
            }
            Self::InvalidPowerLawBounds => {
                write!(formatter, "power-law support must satisfy 0 < min < crossover")
            }
        }
    }
}

impl Error for WaitingTimeLawError {}

fn piecewise_first_mass(alpha: f64, min_s: f64, crossover_s: f64) -> f64 {
    power_integral(alpha, min_s, crossover_s)
}

fn power_integral(exponent: f64, lower_s: f64, upper_s: f64) -> f64 {
    (upper_s.powf(1.0 - exponent) - lower_s.powf(1.0 - exponent))
        / (1.0 - exponent)
}

fn piecewise_normalizer(alpha: f64, beta: f64, min_s: f64, crossover_s: f64) -> f64 {
    let first = piecewise_first_mass(alpha, min_s, crossover_s);
    let tail = crossover_s.powf(1.0 - alpha) / (beta - 1.0);
    first + tail
}

fn open_unit(value: f64) -> f64 {
    value.clamp(f64::EPSILON, 1.0 - f64::EPSILON)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piecewise_fixture() -> WaitingTimeLaw {
        WaitingTimeLaw::PiecewisePowerLaw {
            alpha: 0.79,
            beta: 2.63,
            min_s: 0.05,
            crossover_s: 2.0,
        }
    }

    #[test]
    fn invalid_empirical_law_returns_unknown_for_analytic_statistics() {
        let law = WaitingTimeLaw::empirical(Vec::new());
        assert_eq!(law.validate(), Err(WaitingTimeLawError::EmptyEmpiricalSamples));
        assert_eq!(law.cdf(1.0), None);
        assert_eq!(law.survival(1.0), None);
        assert_eq!(law.density(1.0), None);
        assert_eq!(law.mean_s(), None);
    }

    #[test]
    fn exponential_cdf_survival_and_mean_are_analytic() {
        let law = WaitingTimeLaw::Exponential { mean_s: 2.0 };
        law.validate().expect("valid exponential");
        let expected_survival = (-1.0_f64).exp();
        assert!((law.survival(2.0).unwrap() - expected_survival).abs() < 1.0e-12);
        assert!((law.cdf(2.0).unwrap() - (1.0 - expected_survival)).abs() < 1.0e-12);
        assert_eq!(law.mean_s(), Some(2.0));
    }

    #[test]
    fn double_exponential_uses_mixture_survival_and_weighted_mean() {
        let law = WaitingTimeLaw::DoubleExponential {
            fast_weight: 0.25,
            fast_mean_s: 1.0,
            slow_mean_s: 3.0,
        };
        law.validate().expect("valid double exponential");
        let expected = 0.25 * (-1.0_f64).exp() + 0.75 * (-1.0_f64 / 3.0).exp();
        assert!((law.survival(1.0).unwrap() - expected).abs() < 1.0e-12);
        assert_eq!(law.mean_s(), Some(2.5));
    }

    #[test]
    fn piecewise_power_law_density_is_continuous_and_normalized() {
        let law = piecewise_fixture();
        law.validate().expect("valid piecewise law");

        let left = law.density(2.0 - 1.0e-9).unwrap();
        let right = law.density(2.0 + 1.0e-9).unwrap();
        assert!((left - right).abs() < 1.0e-8);
        assert_eq!(law.cdf(0.05), Some(0.0));
        assert!((law.cdf(1.0e12).unwrap() - 1.0).abs() < 1.0e-9);
        assert!(law.mean_s().unwrap().is_finite());
    }

    #[test]
    fn piecewise_power_law_cdf_is_monotone_and_survival_is_its_complement() {
        let law = piecewise_fixture();
        let times = [0.01, 0.05, 0.1, 0.5, 2.0, 5.0, 100.0, 1.0e6];
        let cdfs: Vec<_> = times.iter().map(|time| law.cdf(*time).unwrap()).collect();
        assert!(cdfs.windows(2).all(|pair| pair[0] <= pair[1]));
        for time in times {
            let cdf = law.cdf(time).unwrap();
            let survival = law.survival(time).unwrap();
            assert!((cdf + survival - 1.0).abs() < 1.0e-12);
        }
    }

    #[test]
    fn seeded_sampling_respects_support_and_is_reproducible() {
        let laws = [
            WaitingTimeLaw::Exponential { mean_s: 0.3 },
            WaitingTimeLaw::DoubleExponential {
                fast_weight: 0.4,
                fast_mean_s: 0.2,
                slow_mean_s: 0.7,
            },
            piecewise_fixture(),
        ];

        for law in laws {
            let first = law.sample_many(10_000, 123).expect("valid sample law");
            let replay = law.sample_many(10_000, 123).expect("valid sample law");
            assert_eq!(first, replay);
            assert!(first.iter().all(|sample| sample.is_finite() && *sample > 0.0));
        }

        let power_law = piecewise_fixture();
        let draws = power_law.sample_many(10_000, 22).expect("valid power law");
        assert!(draws.iter().all(|sample| *sample >= 0.05));
    }

    #[test]
    fn empirical_law_preserves_nonparametric_samples() {
        let law = WaitingTimeLaw::empirical(vec![0.2, 0.4, 0.8]);
        law.validate().expect("valid empirical law");
        assert!((law.mean_s().unwrap() - 0.4666666666666667).abs() < 1.0e-12);
        assert_eq!(law.cdf(0.4), Some(2.0 / 3.0));
    }

    #[test]
    fn invalid_power_law_or_double_exponential_parameters_fail_closed() {
        let invalid_tail = WaitingTimeLaw::PiecewisePowerLaw {
            alpha: 0.79,
            beta: 2.0,
            min_s: 0.05,
            crossover_s: 2.0,
        };
        assert_eq!(
            invalid_tail.validate(),
            Err(WaitingTimeLawError::InvalidPowerLawTailExponent)
        );

        let invalid_bounds = WaitingTimeLaw::PiecewisePowerLaw {
            alpha: 0.79,
            beta: 2.63,
            min_s: 2.0,
            crossover_s: 1.0,
        };
        assert_eq!(
            invalid_bounds.validate(),
            Err(WaitingTimeLawError::InvalidPowerLawBounds)
        );

        let invalid_weight = WaitingTimeLaw::DoubleExponential {
            fast_weight: 1.1,
            fast_mean_s: 0.2,
            slow_mean_s: 0.8,
        };
        assert_eq!(
            invalid_weight.validate(),
            Err(WaitingTimeLawError::InvalidMixtureWeight)
        );
    }

    #[test]
    fn kaplan_meier_keeps_censors_in_the_risk_set_until_their_time() {
        let observations = [
            SurvivalObservation {
                duration_s: 1.0,
                outcome: SurvivalOutcome::EventObserved,
            },
            SurvivalObservation {
                duration_s: 2.0,
                outcome: SurvivalOutcome::EventObserved,
            },
            SurvivalObservation {
                duration_s: 2.0,
                outcome: SurvivalOutcome::RightCensored,
            },
            SurvivalObservation {
                duration_s: 3.0,
                outcome: SurvivalOutcome::EventObserved,
            },
        ];

        let curve = kaplan_meier(&observations).expect("valid survival observations");
        assert_eq!(curve.observations, 4);
        assert_eq!(curve.observed_event_count, 3);
        assert_eq!(curve.right_censored_count, 1);
        assert_eq!(curve.points.len(), 3);

        assert_eq!(curve.points[0].at_risk, 4);
        assert_eq!(curve.points[0].survival_probability, 0.75);
        assert_eq!(curve.points[1].at_risk, 3);
        assert_eq!(curve.points[1].observed_events, 1);
        assert_eq!(curve.points[1].right_censored, 1);
        assert_eq!(curve.points[1].survival_probability, 0.5);
        assert_eq!(curve.points[2].at_risk, 1);
        assert_eq!(curve.points[2].survival_probability, 0.0);
        assert_eq!(curve.points[2].greenwood_standard_error, None);
    }

    #[test]
    fn fully_censored_survival_curve_remains_at_one() {
        let observations = [
            SurvivalObservation {
                duration_s: 1.0,
                outcome: SurvivalOutcome::RightCensored,
            },
            SurvivalObservation {
                duration_s: 2.0,
                outcome: SurvivalOutcome::RightCensored,
            },
        ];
        let curve = kaplan_meier(&observations).expect("valid censored observations");
        assert!(curve
            .points
            .iter()
            .all(|point| point.survival_probability == 1.0));
        assert_eq!(curve.observed_event_count, 0);
    }

    #[test]
    fn kaplan_meier_rejects_empty_or_invalid_durations() {
        assert_eq!(kaplan_meier(&[]), Err(SurvivalError::EmptyObservations));
        assert_eq!(
            kaplan_meier(&[SurvivalObservation {
                duration_s: f64::NAN,
                outcome: SurvivalOutcome::EventObserved,
            }]),
            Err(SurvivalError::InvalidDuration)
        );
    }
}
