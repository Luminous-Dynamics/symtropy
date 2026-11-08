// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Seeded Monte Carlo evaluation for causal drift.
//!
//! This adds sampling noise to the deterministic structural controls. It is
//! still a synthetic benchmark: outcomes, mechanisms, confounders and noise
//! distributions are known by construction. Every seed/controller episode is
//! retained; summary intervals describe Monte Carlo repeatability, not
//! uncertainty about real-world model validity.

use crate::causal_drift::{
    ACTION_COST, CausalDiagnosis, CausalDriftController, CausalRegime,
};

pub const MONTE_CARLO_BASE_SEED: u64 = 20_261_108_100;
pub const MONTE_CARLO_REPLICATES_PER_REGIME: usize = 100;
pub const OBSERVATION_SAMPLE_SIZE_PER_ARM: usize = 50;
pub const MATCHED_PROBE_PAIRS: usize = 512;
pub const OUTCOME_NOISE_SD: f64 = 2.0;
pub const PROBE_CI_Z: f64 = 1.96;
pub const MONTE_CARLO_CUSUM_ALLOWANCE: f64 = 0.25;
pub const MONTE_CARLO_CUSUM_THRESHOLD: f64 = 7.0;
pub const MONTE_CARLO_PROBE_COST: f64 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonteCarloConfig {
    pub base_seed: u64,
    pub replicates_per_regime: usize,
    pub observation_sample_size_per_arm: usize,
    pub matched_probe_pairs: usize,
    pub outcome_noise_sd: f64,
    pub probe_ci_z: f64,
    pub cusum_allowance: f64,
    pub cusum_threshold: f64,
    pub diagnostic_probe_cost: f64,
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        Self {
            base_seed: MONTE_CARLO_BASE_SEED,
            replicates_per_regime: MONTE_CARLO_REPLICATES_PER_REGIME,
            observation_sample_size_per_arm: OBSERVATION_SAMPLE_SIZE_PER_ARM,
            matched_probe_pairs: MATCHED_PROBE_PAIRS,
            outcome_noise_sd: OUTCOME_NOISE_SD,
            probe_ci_z: PROBE_CI_Z,
            cusum_allowance: MONTE_CARLO_CUSUM_ALLOWANCE,
            cusum_threshold: MONTE_CARLO_CUSUM_THRESHOLD,
            diagnostic_probe_cost: MONTE_CARLO_PROBE_COST,
        }
    }
}

impl MonteCarloConfig {
    pub fn validate(self) -> Result<(), &'static str> {
        if self.replicates_per_regime == 0 {
            return Err("replicates_per_regime must be positive");
        }
        if self.observation_sample_size_per_arm < 2 {
            return Err("observation_sample_size_per_arm must be at least 2");
        }
        if self.matched_probe_pairs < 2 {
            return Err("matched_probe_pairs must be at least 2");
        }
        if !self.outcome_noise_sd.is_finite() || self.outcome_noise_sd <= 0.0 {
            return Err("outcome_noise_sd must be finite and positive");
        }
        if !self.probe_ci_z.is_finite() || self.probe_ci_z <= 0.0 {
            return Err("probe_ci_z must be finite and positive");
        }
        if !self.cusum_allowance.is_finite() || self.cusum_allowance < 0.0 {
            return Err("cusum_allowance must be finite and non-negative");
        }
        if !self.cusum_threshold.is_finite() || self.cusum_threshold <= 0.0 {
            return Err("cusum_threshold must be finite and positive");
        }
        if !self.diagnostic_probe_cost.is_finite() || self.diagnostic_probe_cost < 0.0 {
            return Err("diagnostic_probe_cost must be finite and non-negative");
        }
        if self
            .base_seed
            .checked_add(self.replicates_per_regime.saturating_sub(1) as u64)
            .is_none()
        {
            return Err("base_seed plus replicate count overflows u64");
        }
        Ok(())
    }
}

/// Small, dependency-free SplitMix64 stream. Each observation/probe channel
/// receives a derived seed so all controllers see common random numbers.
#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
    spare_normal: Option<f64>,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self {
            state: seed,
            spare_normal: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn uniform_open(&mut self) -> f64 {
        // Midpoint conversion avoids both zero and one for Box-Muller.
        ((self.next_u64() >> 11) as f64 + 0.5) * (1.0 / ((1_u64 << 53) as f64))
    }

    fn normal(&mut self) -> f64 {
        if let Some(spare) = self.spare_normal.take() {
            return spare;
        }
        let radius = (-2.0 * self.uniform_open().ln()).sqrt();
        let angle = std::f64::consts::TAU * self.uniform_open();
        self.spare_normal = Some(radius * angle.sin());
        radius * angle.cos()
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct CusumState {
    baseline: Option<f64>,
    positive: f64,
    negative: f64,
    alarm_tick: Option<usize>,
}

impl CusumState {
    fn observe(&mut self, value: f64, tick: usize, config: MonteCarloConfig) {
        let Some(baseline) = self.baseline else {
            self.baseline = Some(value);
            return;
        };
        if self.alarm_tick.is_some() {
            return;
        }
        let deviation = value - baseline;
        self.positive =
            (self.positive + deviation - config.cusum_allowance).max(0.0);
        self.negative =
            (self.negative - deviation - config.cusum_allowance).max(0.0);
        if self.positive >= config.cusum_threshold
            || self.negative >= config.cusum_threshold
        {
            self.alarm_tick = Some(tick);
        }
    }
}

fn mix_seed(seed: u64, channel: u64, tick: usize) -> u64 {
    seed ^ channel.wrapping_mul(0xD6E8_FEB8_6659_FD93)
        ^ (tick as u64).wrapping_mul(0xA076_1D64_78BD_642F)
}

fn confounder_imbalance(regime: CausalRegime, tick: usize) -> f64 {
    match regime {
        CausalRegime::ConfounderShift if tick >= 5 => -5.0,
        _ => 5.0,
    }
}

fn expected_diagnosis(regime: CausalRegime) -> Option<CausalDiagnosis> {
    match regime {
        CausalRegime::StablePositive => None,
        CausalRegime::SignReversal | CausalRegime::GradualSignReversal => {
            Some(CausalDiagnosis::CausalSignReversal)
        }
        CausalRegime::ConfoundedNull => {
            Some(CausalDiagnosis::NullEffectWithConfounding)
        }
        CausalRegime::ConfounderShift => {
            Some(CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange)
        }
    }
}

fn drift_onset(regime: CausalRegime) -> Option<usize> {
    match regime {
        CausalRegime::SignReversal => Some(4),
        CausalRegime::GradualSignReversal => Some(3),
        CausalRegime::ConfounderShift => Some(5),
        CausalRegime::StablePositive | CausalRegime::ConfoundedNull => None,
    }
}

fn noisy_observational_association(
    regime: CausalRegime,
    tick: usize,
    seed: u64,
    config: MonteCarloConfig,
) -> f64 {
    let mut rng = SplitMix64::new(mix_seed(seed, 1, tick));
    let standard_error = config.outcome_noise_sd
        * (2.0 / config.observation_sample_size_per_arm as f64).sqrt();
    regime.causal_effect(tick)
        + confounder_imbalance(regime, tick)
        + standard_error * rng.normal()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProbeEstimate {
    pub tick: usize,
    pub estimate: f64,
    pub standard_error: f64,
    pub lower_95: f64,
    pub upper_95: f64,
    pub interval_covers_truth: bool,
}

fn noisy_matched_probe(
    regime: CausalRegime,
    tick: usize,
    seed: u64,
    config: MonteCarloConfig,
) -> ProbeEstimate {
    let mut rng = SplitMix64::new(mix_seed(seed, 2, tick));
    let truth = regime.causal_effect(tick);
    let paired_noise_sd = std::f64::consts::SQRT_2 * config.outcome_noise_sd;
    let mut sum = 0.0;
    let mut sum_squares = 0.0;
    for _ in 0..config.matched_probe_pairs {
        let difference = truth + paired_noise_sd * rng.normal();
        sum += difference;
        sum_squares += difference * difference;
    }
    let count = config.matched_probe_pairs as f64;
    let estimate = sum / count;
    let sample_variance = ((sum_squares - count * estimate * estimate).max(0.0)
        / (count - 1.0))
        .max(0.0);
    let standard_error = (sample_variance / count).sqrt();
    let margin = config.probe_ci_z * standard_error;
    let lower_95 = estimate - margin;
    let upper_95 = estimate + margin;
    ProbeEstimate {
        tick,
        estimate,
        standard_error,
        lower_95,
        upper_95,
        interval_covers_truth: lower_95 <= truth && truth <= upper_95,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonteCarloEpisode {
    pub seed: u64,
    pub regime: CausalRegime,
    pub controller: CausalDriftController,
    pub signal_detected: bool,
    pub signal_detection_tick: Option<usize>,
    pub signal_detection_delay_ticks: Option<usize>,
    pub premature_signal_alarm: bool,
    pub diagnosis: Option<CausalDiagnosis>,
    pub diagnosis_tick: Option<usize>,
    pub diagnosis_correct: bool,
    pub false_mechanism_diagnosis: bool,
    pub diagnosis_after_signal_ticks: Option<usize>,
    pub active_ticks: usize,
    pub active_after_nonpositive_effect_ticks: usize,
    pub missed_beneficial_action_ticks: usize,
    pub proxy_driven_action_under_nonpositive_effect: bool,
    pub diagnostic_probe_count: usize,
    pub probe_ci_coverage_count: usize,
    pub probe_ci_count: usize,
    pub last_probe_estimate: Option<f64>,
    pub last_probe_lower_95: Option<f64>,
    pub last_probe_upper_95: Option<f64>,
    pub net_utility: f64,
    pub oracle_net_utility: f64,
    pub regret_to_oracle: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonteCarloSummary {
    pub regime: CausalRegime,
    pub controller: CausalDriftController,
    pub episodes: usize,
    pub signal_detection_rate: f64,
    pub signal_detection_ci_low: f64,
    pub signal_detection_ci_high: f64,
    pub premature_alarm_rate: f64,
    pub correct_diagnosis_rate: f64,
    pub correct_diagnosis_ci_low: f64,
    pub correct_diagnosis_ci_high: f64,
    pub premature_alarm_ci_low: f64,
    pub premature_alarm_ci_high: f64,
    pub false_diagnosis_rate: f64,
    pub false_diagnosis_ci_low: f64,
    pub false_diagnosis_ci_high: f64,
    pub mean_signal_detection_delay_ticks: Option<f64>,
    pub mean_diagnosis_after_signal_ticks: Option<f64>,
    pub mean_active_after_nonpositive_effect_ticks: f64,
    pub mean_missed_beneficial_action_ticks: f64,
    pub mean_probe_count: f64,
    pub mean_probe_ci_coverage: Option<f64>,
    pub mean_net_utility: f64,
    pub mean_regret_to_oracle: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonteCarloReport {
    pub config: MonteCarloConfig,
    pub episodes: Vec<MonteCarloEpisode>,
    pub summaries: Vec<MonteCarloSummary>,
}

fn wilson_interval(successes: usize, trials: usize) -> (f64, f64) {
    if trials == 0 {
        return (0.0, 0.0);
    }
    let z = 1.96_f64;
    let n = trials as f64;
    let p = successes as f64 / n;
    let denominator = 1.0 + z * z / n;
    let center = (p + z * z / (2.0 * n)) / denominator;
    let half = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt())
        / denominator;
    ((center - half).max(0.0), (center + half).min(1.0))
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let values: Vec<f64> = values.collect();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn run_episode(
    regime: CausalRegime,
    controller: CausalDriftController,
    seed: u64,
    config: MonteCarloConfig,
) -> MonteCarloEpisode {
    let mut policy_enabled = true;
    let mut detector = CusumState::default();
    let mut diagnosis = None;
    let mut diagnosis_tick = None;
    let mut previous_probe_lower = None;
    let mut active_ticks = 0;
    let mut active_after_nonpositive_effect_ticks = 0;
    let mut missed_beneficial_action_ticks = 0;
    let mut proxy_driven_action_under_nonpositive_effect = false;
    let mut cumulative_causal_contribution = 0.0;
    let mut action_cost = 0.0;
    let mut diagnostic_probe_cost = 0.0;
    let mut diagnostic_probe_count = 0;
    let mut probe_ci_coverage_count = 0;
    let mut probe_ci_count = 0;
    let mut last_probe = None;

    for tick in 1..=8 {
        let true_effect = regime.causal_effect(tick);
        let association =
            noisy_observational_association(regime, tick, seed, config);

        // Actions are executed before the current observation/probe arrives.
        if policy_enabled {
            active_ticks += 1;
            cumulative_causal_contribution += true_effect;
            action_cost += ACTION_COST;
            if true_effect <= 0.0 {
                active_after_nonpositive_effect_ticks += 1;
                if controller == CausalDriftController::OutcomeReactive
                    && association > ACTION_COST
                {
                    proxy_driven_action_under_nonpositive_effect = true;
                }
            }
        } else if true_effect > ACTION_COST {
            missed_beneficial_action_ticks += 1;
        }

        detector.observe(association, tick, config);

        match controller {
            CausalDriftController::FrozenPolicy => {}
            CausalDriftController::OutcomeReactive => {
                if association <= ACTION_COST {
                    policy_enabled = false;
                }
            }
            CausalDriftController::CusumSignalOnly => {
                if detector.alarm_tick.is_some() {
                    policy_enabled = false;
                }
            }
            CausalDriftController::MechanismAware => {
                if tick % 2 == 0 {
                    let probe = noisy_matched_probe(regime, tick, seed, config);
                    diagnostic_probe_count += 1;
                    diagnostic_probe_cost += config.diagnostic_probe_cost;
                    probe_ci_count += 1;
                    if probe.interval_covers_truth {
                        probe_ci_coverage_count += 1;
                    }
                    last_probe = Some(probe);

                    let identified = if probe.upper_95 < 0.0 {
                        Some(CausalDiagnosis::CausalSignReversal)
                    } else if probe.lower_95 <= 0.0 && probe.upper_95 <= ACTION_COST {
                        Some(CausalDiagnosis::NullEffectWithConfounding)
                    } else if detector.alarm_tick.is_some()
                        && previous_probe_lower.is_some_and(|lower| lower > ACTION_COST)
                        && probe.lower_95 > ACTION_COST
                    {
                        Some(CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange)
                    } else {
                        None
                    };

                    if let Some(identified) = identified {
                        diagnosis = Some(identified);
                        diagnosis_tick = Some(tick);
                        if matches!(
                            identified,
                            CausalDiagnosis::CausalSignReversal
                                | CausalDiagnosis::NullEffectWithConfounding
                        ) {
                            // A diagnosis affects action from the next tick.
                            policy_enabled = false;
                        }
                    }
                    previous_probe_lower = Some(probe.lower_95);
                }
            }
        }
    }

    let expected = expected_diagnosis(regime);
    let diagnosis_correct = diagnosis == expected;
    let false_mechanism_diagnosis = diagnosis.is_some() && diagnosis != expected;
    let signal_tick = detector.alarm_tick;
    let onset = drift_onset(regime);
    let signal_detection_delay_ticks = signal_tick
        .zip(onset)
        .filter(|(detected, onset_tick)| detected >= onset_tick)
        .map(|(detected, onset_tick)| detected - onset_tick);
    let premature_signal_alarm = signal_tick
        .zip(onset)
        .is_some_and(|(detected, onset_tick)| detected < onset_tick)
        || (onset.is_none() && signal_tick.is_some());
    let diagnosis_after_signal_ticks = diagnosis_tick
        .zip(signal_tick)
        .and_then(|(diagnosed, detected)| (diagnosed >= detected).then_some(diagnosed - detected));
    let oracle_net_utility = (1..=8)
        .map(|tick| (regime.causal_effect(tick) - ACTION_COST).max(0.0))
        .sum::<f64>();
    let net_utility =
        cumulative_causal_contribution - action_cost - diagnostic_probe_cost;

    MonteCarloEpisode {
        seed,
        regime,
        controller,
        signal_detected: signal_tick.is_some(),
        signal_detection_tick: signal_tick,
        signal_detection_delay_ticks,
        premature_signal_alarm,
        diagnosis,
        diagnosis_tick,
        diagnosis_correct,
        false_mechanism_diagnosis,
        diagnosis_after_signal_ticks,
        active_ticks,
        active_after_nonpositive_effect_ticks,
        missed_beneficial_action_ticks,
        proxy_driven_action_under_nonpositive_effect,
        diagnostic_probe_count,
        probe_ci_coverage_count,
        probe_ci_count,
        last_probe_estimate: last_probe.map(|probe| probe.estimate),
        last_probe_lower_95: last_probe.map(|probe| probe.lower_95),
        last_probe_upper_95: last_probe.map(|probe| probe.upper_95),
        net_utility,
        oracle_net_utility,
        regret_to_oracle: oracle_net_utility - net_utility,
    }
}

fn summarize(
    regime: CausalRegime,
    controller: CausalDriftController,
    episodes: &[MonteCarloEpisode],
) -> MonteCarloSummary {
    let count = episodes.len();
    let signal_successes = episodes
        .iter()
        .filter(|episode| {
            drift_onset(regime).is_some_and(|onset| {
                episode.signal_detection_tick.is_some_and(|tick| tick >= onset)
            })
        })
        .count();
    let correct = episodes
        .iter()
        .filter(|episode| episode.diagnosis_correct)
        .count();
    let false_diagnoses = episodes
        .iter()
        .filter(|episode| episode.false_mechanism_diagnosis)
        .count();
    let premature = episodes
        .iter()
        .filter(|episode| episode.premature_signal_alarm)
        .count();
    let (signal_detection_ci_low, signal_detection_ci_high) =
        wilson_interval(signal_successes, count);
    let (correct_diagnosis_ci_low, correct_diagnosis_ci_high) =
        wilson_interval(correct, count);
    let (premature_alarm_ci_low, premature_alarm_ci_high) =
        wilson_interval(premature, count);
    let (false_diagnosis_ci_low, false_diagnosis_ci_high) =
        wilson_interval(false_diagnoses, count);

    MonteCarloSummary {
        regime,
        controller,
        episodes: count,
        signal_detection_rate: signal_successes as f64 / count as f64,
        signal_detection_ci_low,
        signal_detection_ci_high,
        premature_alarm_rate: premature as f64 / count as f64,
        correct_diagnosis_rate: correct as f64 / count as f64,
        correct_diagnosis_ci_low,
        correct_diagnosis_ci_high,
        premature_alarm_ci_low,
        premature_alarm_ci_high,
        false_diagnosis_rate: false_diagnoses as f64 / count as f64,
        false_diagnosis_ci_low,
        false_diagnosis_ci_high,
        mean_signal_detection_delay_ticks: mean(
            episodes
                .iter()
                .filter_map(|episode| episode.signal_detection_delay_ticks)
                .map(|value| value as f64),
        ),
        mean_diagnosis_after_signal_ticks: mean(
            episodes
                .iter()
                .filter_map(|episode| episode.diagnosis_after_signal_ticks)
                .map(|value| value as f64),
        ),
        mean_active_after_nonpositive_effect_ticks: mean(
            episodes
                .iter()
                .map(|episode| episode.active_after_nonpositive_effect_ticks as f64),
        )
        .unwrap_or_default(),
        mean_missed_beneficial_action_ticks: mean(
            episodes
                .iter()
                .map(|episode| episode.missed_beneficial_action_ticks as f64),
        )
        .unwrap_or_default(),
        mean_probe_count: mean(
            episodes
                .iter()
                .map(|episode| episode.diagnostic_probe_count as f64),
        )
        .unwrap_or_default(),
        mean_probe_ci_coverage: {
            let total_probes: usize = episodes.iter().map(|episode| episode.probe_ci_count).sum();
            let covered: usize = episodes
                .iter()
                .map(|episode| episode.probe_ci_coverage_count)
                .sum();
            (total_probes > 0).then_some(covered as f64 / total_probes as f64)
        },
        mean_net_utility: mean(episodes.iter().map(|episode| episode.net_utility))
            .unwrap_or_default(),
        mean_regret_to_oracle: mean(episodes.iter().map(|episode| episode.regret_to_oracle))
            .unwrap_or_default(),
    }
}

pub fn run_monte_carlo(
    config: MonteCarloConfig,
) -> Result<MonteCarloReport, &'static str> {
    config.validate()?;
    let mut episodes = Vec::with_capacity(
        CausalRegime::ALL.len()
            * CausalDriftController::ALL.len()
            * config.replicates_per_regime,
    );
    let mut summaries = Vec::with_capacity(
        CausalRegime::ALL.len() * CausalDriftController::ALL.len(),
    );

    for regime in CausalRegime::ALL {
        let start = episodes.len();
        for replicate in 0..config.replicates_per_regime {
            let seed = config.base_seed + replicate as u64;
            for controller in CausalDriftController::ALL {
                episodes.push(run_episode(regime, controller, seed, config));
            }
        }
        for controller in CausalDriftController::ALL {
            let group: Vec<MonteCarloEpisode> = episodes[start..]
                .iter()
                .filter(|episode| episode.controller == controller)
                .cloned()
                .collect();
            summaries.push(summarize(regime, controller, &group));
        }
    }

    Ok(MonteCarloReport {
        config,
        episodes,
        summaries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monte_carlo_schedule_is_complete_and_seed_reproducible() {
        let config = MonteCarloConfig {
            replicates_per_regime: 12,
            ..MonteCarloConfig::default()
        };
        let first = run_monte_carlo(config).expect("valid config");
        let second = run_monte_carlo(config).expect("valid config");
        assert_eq!(first, second);
        assert_eq!(
            first.episodes.len(),
            CausalRegime::ALL.len() * CausalDriftController::ALL.len() * 12
        );
        assert_eq!(
            first.summaries.len(),
            CausalRegime::ALL.len() * CausalDriftController::ALL.len()
        );
    }

    #[test]
    fn mechanism_aware_probe_changes_diagnosis_not_signal_detection() {
        let config = MonteCarloConfig::default();
        let report = run_monte_carlo(config).expect("valid config");
        let get = |regime, controller| {
            report
                .summaries
                .iter()
                .find(|summary| {
                    summary.regime == regime && summary.controller == controller
                })
                .expect("summary exists")
        };
        let causal_shift = get(
            CausalRegime::SignReversal,
            CausalDriftController::MechanismAware,
        );
        let cusum = get(
            CausalRegime::SignReversal,
            CausalDriftController::CusumSignalOnly,
        );
        assert_eq!(
            causal_shift.signal_detection_rate,
            cusum.signal_detection_rate
        );
        assert!(causal_shift.correct_diagnosis_rate >= 0.95);
        assert!(causal_shift.mean_active_after_nonpositive_effect_ticks < 1.1);
    }

    #[test]
    fn confounder_shift_signal_only_policy_loses_beneficial_actions() {
        let report = run_monte_carlo(MonteCarloConfig::default()).expect("valid config");
        let signal_only = report
            .summaries
            .iter()
            .find(|summary| {
                summary.regime == CausalRegime::ConfounderShift
                    && summary.controller == CausalDriftController::CusumSignalOnly
            })
            .expect("summary exists");
        let mechanism_aware = report
            .summaries
            .iter()
            .find(|summary| {
                summary.regime == CausalRegime::ConfounderShift
                    && summary.controller == CausalDriftController::MechanismAware
            })
            .expect("summary exists");
        assert!(
            signal_only.mean_missed_beneficial_action_ticks
                > mechanism_aware.mean_missed_beneficial_action_ticks
        );
        assert!(mechanism_aware.correct_diagnosis_rate >= 0.90);
    }

    #[test]
    fn stable_positive_control_bounds_false_signal_and_causal_diagnosis() {
        let report = run_monte_carlo(MonteCarloConfig::default()).expect("valid config");
        for controller in [
            CausalDriftController::CusumSignalOnly,
            CausalDriftController::MechanismAware,
        ] {
            let summary = report
                .summaries
                .iter()
                .find(|summary| {
                    summary.regime == CausalRegime::StablePositive
                        && summary.controller == controller
                })
                .expect("summary exists");
            assert!(summary.premature_alarm_rate <= 0.05);
            assert!(summary.false_diagnosis_rate <= 0.05);
            assert!(summary.premature_alarm_ci_high <= 0.10);
            assert!(summary.false_diagnosis_ci_high <= 0.10);
            if controller == CausalDriftController::MechanismAware {
                assert!(
                    summary.mean_probe_ci_coverage.unwrap_or_default() >= 0.90
                );
            }
        }
    }

    #[test]
    fn gradual_shift_onset_is_the_first_mechanism_change_not_first_negative_effect() {
        assert_eq!(drift_onset(CausalRegime::GradualSignReversal), Some(3));
    }

    #[test]
    fn invalid_monte_carlo_config_fails_closed() {
        let config = MonteCarloConfig {
            replicates_per_regime: 0,
            ..MonteCarloConfig::default()
        };
        assert!(run_monte_carlo(config).is_err());
    }
}
