// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Seeded, noisy causal-drift benchmark with development/confirmatory separation.
//!
//! This layer estimates rates under a declared synthetic noise model. It
//! separates controller-visible evidence from evaluator-only causal truth,
//! reports Wilson intervals for rates, and freezes CUSUM tuning on disjoint
//! development seeds before any confirmatory seed is evaluated.

use crate::causal_drift::{ACTION_COST, CausalDiagnosis, CausalDriftController, CausalRegime};

pub const DEVELOPMENT_SEED_START: u64 = 1;
pub const DEVELOPMENT_SEED_COUNT: usize = 512;
pub const DEVELOPMENT_MAX_FALSE_ALARMS: usize = 1;
pub const CONFIRMATORY_SEED_START: u64 = 10_001;
pub const CONFIRMATORY_SEED_COUNT: usize = 1_000;
pub const PROBE_PAIRS: usize = 256;
pub const OBSERVATION_NOISE_AMPLITUDE: f64 = 2.0;
pub const COVARIATE_NOISE_AMPLITUDE: f64 = 1.0;
pub const PROBE_NOISE_AMPLITUDE: f64 = 2.0;
pub const ACTION_COST_PER_TICK: f64 = 0.5;
pub const PROBE_COST: f64 = 0.25;
pub const PROBE_CONFIDENCE_Z: f64 = 1.96;
pub const COVARIATE_SHIFT_THRESHOLD: f64 = 6.0;
pub const CUSUM_BASELINE_TICKS: usize = 2;

const CUSUM_CANDIDATES: [(f64, f64); 9] = [
    (2.25, 2.5),
    (2.25, 3.0),
    (2.25, 3.5),
    (2.0, 3.5),
    (2.0, 4.0),
    (2.0, 4.5),
    (2.0, 5.0),
    (1.75, 5.0),
    (1.5, 6.0),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CusumConfig {
    pub allowance: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CusumTuningReceipt {
    pub config: CusumConfig,
    pub development_false_alarms: usize,
    pub development_stable_episodes: usize,
    pub development_signal_detections: usize,
    pub development_changed_episodes: usize,
    pub false_alarm_constraint_met: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProbeEstimate {
    pub pairs: usize,
    pub mean_effect: f64,
    pub standard_error: f64,
    pub lower_95: f64,
    pub upper_95: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeClass {
    Beneficial,
    Harmful,
    NullEquivalent,
    Uncertain,
}

#[derive(Debug, Clone, PartialEq)]
struct ObservableEvidence {
    observational_association: [f64; 8],
    measured_covariate_imbalance: [f64; 8],
    matched_probes: [Option<ProbeEstimate>; 8],
}

#[derive(Debug, Clone, PartialEq)]
struct EvaluatorTruth {
    regime: CausalRegime,
    causal_effects: [f64; 8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeededEpisodeOutcome {
    pub regime: CausalRegime,
    pub controller: CausalDriftController,
    pub seed: u64,
    pub signal_detection_tick: Option<usize>,
    pub covariate_shift_tick: Option<usize>,
    pub diagnosis: Option<CausalDiagnosis>,
    pub diagnosis_tick: Option<usize>,
    pub correct_diagnosis: bool,
    pub false_diagnosis: bool,
    pub missed_diagnosis: bool,
    pub active_ticks: usize,
    pub nonpositive_effect_action_ticks: usize,
    pub missed_beneficial_action_ticks: usize,
    pub diagnostic_probe_count: usize,
    pub net_utility: f64,
    pub oracle_net_utility: f64,
    pub regret_to_oracle: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RateInterval {
    pub numerator: usize,
    pub denominator: usize,
    pub rate: f64,
    pub lower_95: f64,
    pub upper_95: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeededAggregateRow {
    pub regime: CausalRegime,
    pub controller: CausalDriftController,
    pub episodes: usize,
    pub expected_signal_change: bool,
    pub signal_alarm: RateInterval,
    pub signal_false_alarms: usize,
    pub correct_diagnoses: RateInterval,
    pub false_diagnoses: usize,
    pub missed_diagnoses: usize,
    pub mean_signal_detection_tick: Option<f64>,
    pub mean_diagnosis_tick: Option<f64>,
    pub mean_active_ticks: f64,
    pub mean_nonpositive_effect_action_ticks: f64,
    pub mean_missed_beneficial_action_ticks: f64,
    pub mean_probe_count: f64,
    pub mean_net_utility: f64,
    pub net_utility_ci_lower_95: f64,
    pub net_utility_ci_upper_95: f64,
    pub mean_oracle_regret: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeededBenchmarkReport {
    pub tuning: CusumTuningReceipt,
    pub confirmatory_seed_start: u64,
    pub confirmatory_seed_count: usize,
    pub rows: Vec<SeededAggregateRow>,
    /// Every confirmatory replicate, retained even when a controller fails
    /// to diagnose or produces an invalid policy response.
    pub replicates: Vec<SeededEpisodeOutcome>,
}

#[derive(Clone, Copy)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.state = value;
        value.wrapping_mul(2_685_821_657_736_338_717)
    }

    fn uniform(&mut self, amplitude: f64) -> f64 {
        let mantissa = self.next_u64() >> 11;
        let unit = mantissa as f64 / ((1_u64 << 53) as f64);
        (2.0 * unit - 1.0) * amplitude
    }
}

fn observational_imbalance(regime: CausalRegime, tick: usize) -> f64 {
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
        CausalRegime::ConfoundedNull => Some(CausalDiagnosis::NullEffectWithConfounding),
        CausalRegime::ConfounderShift => {
            Some(CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange)
        }
    }
}

fn expected_signal_change(regime: CausalRegime) -> bool {
    matches!(
        regime,
        CausalRegime::SignReversal
            | CausalRegime::GradualSignReversal
            | CausalRegime::ConfounderShift
    )
}

fn derive_seed(seed: u64, stream: u64) -> u64 {
    let mut value = seed ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn observation_stream(regime: CausalRegime, seed: u64) -> [f64; 8] {
    let mut random = XorShift64::new(seed);
    let mut observations = [0.0; 8];
    for (index, value) in observations.iter_mut().enumerate() {
        let tick = index + 1;
        *value = regime.causal_effect(tick)
            + observational_imbalance(regime, tick)
            + random.uniform(OBSERVATION_NOISE_AMPLITUDE);
    }
    observations
}

fn measured_covariate_stream(regime: CausalRegime, seed: u64) -> [f64; 8] {
    let mut random = XorShift64::new(derive_seed(seed, 0xC0A4_12A7));
    let mut values = [0.0; 8];
    for (index, value) in values.iter_mut().enumerate() {
        *value =
            observational_imbalance(regime, index + 1) + random.uniform(COVARIATE_NOISE_AMPLITUDE);
    }
    values
}

fn matched_probe(regime: CausalRegime, tick: usize, seed: u64) -> ProbeEstimate {
    let causal_effect = regime.causal_effect(tick);
    let mut random = XorShift64::new(derive_seed(seed, 0xB70B_E000_u64.wrapping_add(tick as u64)));
    let mut differences = Vec::with_capacity(PROBE_PAIRS);

    for pair in 0..PROBE_PAIRS {
        let common_cause = ((pair % 17) as f64 - 8.0) * 0.25;
        let treated = 10.0 + common_cause + causal_effect + random.uniform(PROBE_NOISE_AMPLITUDE);
        let control = 10.0 + common_cause + random.uniform(PROBE_NOISE_AMPLITUDE);
        differences.push(treated - control);
    }

    let mean = differences.iter().sum::<f64>() / PROBE_PAIRS as f64;
    let sample_variance = differences
        .iter()
        .map(|difference| (difference - mean).powi(2))
        .sum::<f64>()
        / (PROBE_PAIRS - 1) as f64;
    let standard_error = (sample_variance / PROBE_PAIRS as f64).sqrt();
    let margin = PROBE_CONFIDENCE_Z * standard_error;

    ProbeEstimate {
        pairs: PROBE_PAIRS,
        mean_effect: mean,
        standard_error,
        lower_95: mean - margin,
        upper_95: mean + margin,
    }
}

fn build_evidence(regime: CausalRegime, seed: u64) -> (ObservableEvidence, EvaluatorTruth) {
    let observations = observation_stream(regime, seed);
    let covariates = measured_covariate_stream(regime, seed);
    let mut probes = [None; 8];
    let mut causal_effects = [0.0; 8];

    for index in 0..8 {
        let tick = index + 1;
        causal_effects[index] = regime.causal_effect(tick);
        if tick % 2 == 0 {
            probes[index] = Some(matched_probe(regime, tick, seed));
        }
    }

    (
        ObservableEvidence {
            observational_association: observations,
            measured_covariate_imbalance: covariates,
            matched_probes: probes,
        },
        EvaluatorTruth {
            regime,
            causal_effects,
        },
    )
}

fn cusum_detection_tick(observations: &[f64; 8], config: CusumConfig) -> Option<usize> {
    let baseline = (observations[0] + observations[1]) / CUSUM_BASELINE_TICKS as f64;
    let mut positive = 0.0_f64;
    let mut negative = 0.0_f64;

    for (index, observation) in observations.iter().enumerate().skip(CUSUM_BASELINE_TICKS) {
        let deviation = observation - baseline;
        positive = (positive + deviation - config.allowance).max(0.0);
        negative = (negative - deviation - config.allowance).max(0.0);
        if positive >= config.threshold || negative >= config.threshold {
            return Some(index + 1);
        }
    }
    None
}

fn tune_cusum() -> CusumTuningReceipt {
    let development_seeds: Vec<u64> =
        (DEVELOPMENT_SEED_START..DEVELOPMENT_SEED_START + DEVELOPMENT_SEED_COUNT as u64).collect();
    let changed_regimes = [
        CausalRegime::SignReversal,
        CausalRegime::GradualSignReversal,
        CausalRegime::ConfounderShift,
    ];

    let mut best_eligible: Option<(CusumConfig, usize, usize, usize)> = None;
    let mut best_fallback: Option<(CusumConfig, usize, usize, usize)> = None;

    for (allowance, threshold) in CUSUM_CANDIDATES {
        let config = CusumConfig {
            allowance,
            threshold,
        };
        let stable_false_alarms = development_seeds
            .iter()
            .filter(|seed| {
                let observations = observation_stream(CausalRegime::StablePositive, **seed);
                cusum_detection_tick(&observations, config).is_some()
            })
            .count();

        let mut detections = 0usize;
        let mut changed_episodes = 0usize;
        for regime in changed_regimes {
            for seed in &development_seeds {
                let observations = observation_stream(regime, *seed);
                detections += if cusum_detection_tick(&observations, config).is_some() {
                    1
                } else {
                    0
                };
                changed_episodes += 1;
            }
        }

        let candidate = (config, stable_false_alarms, detections, changed_episodes);
        let better = |left: &(CusumConfig, usize, usize, usize),
                      right: &(CusumConfig, usize, usize, usize)| {
            left.2 * right.3 > right.2 * left.3
                || (left.2 * right.3 == right.2 * left.3 && left.1 < right.1)
        };

        if best_fallback.as_ref().is_none_or(|current| {
            candidate.1 < current.1 || (candidate.1 == current.1 && better(&candidate, current))
        }) {
            best_fallback = Some(candidate);
        }

        if stable_false_alarms <= DEVELOPMENT_MAX_FALSE_ALARMS
            && best_eligible
                .as_ref()
                .is_none_or(|current| better(&candidate, current))
        {
            best_eligible = Some(candidate);
        }
    }

    let selected = best_eligible
        .or(best_fallback)
        .expect("CUSUM candidate grid is non-empty");
    CusumTuningReceipt {
        config: selected.0,
        development_false_alarms: selected.1,
        development_stable_episodes: DEVELOPMENT_SEED_COUNT,
        development_signal_detections: selected.2,
        development_changed_episodes: selected.3,
        false_alarm_constraint_met: selected.1 <= DEVELOPMENT_MAX_FALSE_ALARMS,
    }
}

fn classify_probe(probe: ProbeEstimate) -> ProbeClass {
    if probe.lower_95 > ACTION_COST {
        ProbeClass::Beneficial
    } else if probe.upper_95 < -ACTION_COST {
        ProbeClass::Harmful
    } else if probe.lower_95 > -ACTION_COST && probe.upper_95 < ACTION_COST {
        ProbeClass::NullEquivalent
    } else {
        ProbeClass::Uncertain
    }
}

fn covariate_shift_tick(values: &[f64; 8]) -> Option<usize> {
    let baseline = (values[0] + values[1]) / 2.0;
    values
        .iter()
        .enumerate()
        .skip(CUSUM_BASELINE_TICKS)
        .find_map(|(index, value)| {
            ((value - baseline).abs() >= COVARIATE_SHIFT_THRESHOLD).then_some(index + 1)
        })
}

#[derive(Debug, Clone, PartialEq)]
struct ControllerDecision {
    actions: [bool; 8],
    signal_detection_tick: Option<usize>,
    covariate_shift_tick: Option<usize>,
    diagnosis: Option<CausalDiagnosis>,
    diagnosis_tick: Option<usize>,
    diagnostic_probe_count: usize,
}

/// The policy boundary receives only observations, measured covariates and
/// noisy matched-probe estimates. It cannot access the scenario regime or
/// evaluator-only causal-effect trajectory.
fn decide_actions(
    observable: &ObservableEvidence,
    controller: CausalDriftController,
    tuning: &CusumTuningReceipt,
) -> ControllerDecision {
    let signal_detection_tick =
        cusum_detection_tick(&observable.observational_association, tuning.config);
    let observed_covariate_shift_tick =
        covariate_shift_tick(&observable.measured_covariate_imbalance);

    let mut policy_enabled = true;
    let mut actions = [false; 8];
    let mut diagnosis = None;
    let mut diagnosis_tick = None;
    let mut previous_probe_class = None;
    let mut diagnostic_probe_count = 0usize;

    for (index, action) in actions.iter_mut().enumerate() {
        let tick = index + 1;
        // Decide the action before this tick's observation/probe is revealed.
        *action = policy_enabled;

        let observed_association = observable.observational_association[index];
        match controller {
            CausalDriftController::FrozenPolicy => {}
            CausalDriftController::OutcomeReactive => {
                if observed_association <= ACTION_COST {
                    policy_enabled = false;
                }
            }
            CausalDriftController::CusumSignalOnly => {
                if signal_detection_tick == Some(tick) {
                    policy_enabled = false;
                }
            }
            CausalDriftController::MechanismAware => {
                if tick % 2 == 0 && diagnosis.is_none() {
                    if let Some(probe) = observable.matched_probes[index] {
                        diagnostic_probe_count += 1;
                        let probe_class = classify_probe(probe);
                        match probe_class {
                            ProbeClass::Harmful => {
                                diagnosis = Some(CausalDiagnosis::CausalSignReversal);
                                diagnosis_tick = Some(tick);
                                policy_enabled = false;
                            }
                            ProbeClass::NullEquivalent => {
                                diagnosis = Some(CausalDiagnosis::NullEffectWithConfounding);
                                diagnosis_tick = Some(tick);
                                policy_enabled = false;
                            }
                            ProbeClass::Beneficial => {
                                if previous_probe_class == Some(ProbeClass::Beneficial)
                                    && signal_detection_tick
                                        .is_some_and(|detected| detected <= tick)
                                    && observed_covariate_shift_tick
                                        .is_some_and(|shifted| shifted <= tick)
                                {
                                    diagnosis = Some(
                                        CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange,
                                    );
                                    diagnosis_tick = Some(tick);
                                    // Preserve the intervention: measured
                                    // evidence says the treatment effect remains
                                    // beneficial despite the signal shift.
                                }
                            }
                            ProbeClass::Uncertain => {}
                        }
                        previous_probe_class = Some(probe_class);
                    }
                }
            }
        }
    }

    ControllerDecision {
        actions,
        signal_detection_tick,
        covariate_shift_tick: observed_covariate_shift_tick,
        diagnosis,
        diagnosis_tick,
        diagnostic_probe_count,
    }
}

/// Independent scoring happens only after the controller has returned its
/// complete action mask. Hidden causal effects never flow back into policy
/// selection.
fn score_decision(
    regime: CausalRegime,
    controller: CausalDriftController,
    seed: u64,
    truth: &EvaluatorTruth,
    decision: ControllerDecision,
) -> SeededEpisodeOutcome {
    let mut active_ticks = 0usize;
    let mut nonpositive_effect_action_ticks = 0usize;
    let mut missed_beneficial_action_ticks = 0usize;
    let mut net_utility = -(decision.diagnostic_probe_count as f64 * PROBE_COST);

    for (index, action) in decision.actions.iter().copied().enumerate() {
        let causal_effect = truth.causal_effects[index];
        if action {
            active_ticks += 1;
            net_utility += causal_effect - ACTION_COST_PER_TICK;
            if causal_effect <= 0.0 {
                nonpositive_effect_action_ticks += 1;
            }
        } else if causal_effect > ACTION_COST_PER_TICK {
            missed_beneficial_action_ticks += 1;
        }
    }

    let expected = expected_diagnosis(regime);
    let correct_diagnosis = decision.diagnosis == expected;
    let false_diagnosis = decision.diagnosis.is_some() && decision.diagnosis != expected;
    let missed_diagnosis = expected.is_some() && decision.diagnosis != expected;
    let oracle_net_utility = truth
        .causal_effects
        .iter()
        .map(|effect| {
            if *effect > ACTION_COST_PER_TICK {
                *effect - ACTION_COST_PER_TICK
            } else {
                0.0
            }
        })
        .sum::<f64>();
    let regret_to_oracle = (oracle_net_utility - net_utility).max(0.0);

    SeededEpisodeOutcome {
        regime: truth.regime,
        controller,
        seed,
        signal_detection_tick: decision.signal_detection_tick,
        covariate_shift_tick: decision.covariate_shift_tick,
        diagnosis: decision.diagnosis,
        diagnosis_tick: decision.diagnosis_tick,
        correct_diagnosis,
        false_diagnosis,
        missed_diagnosis,
        active_ticks,
        nonpositive_effect_action_ticks,
        missed_beneficial_action_ticks,
        diagnostic_probe_count: decision.diagnostic_probe_count,
        net_utility,
        oracle_net_utility,
        regret_to_oracle,
    }
}

fn run_controller(
    regime: CausalRegime,
    seed: u64,
    observable: &ObservableEvidence,
    truth: &EvaluatorTruth,
    controller: CausalDriftController,
    tuning: &CusumTuningReceipt,
) -> SeededEpisodeOutcome {
    let decision = decide_actions(observable, controller, tuning);
    score_decision(regime, controller, seed, truth, decision)
}

fn wilson_interval(successes: usize, total: usize) -> RateInterval {
    if total == 0 {
        return RateInterval {
            numerator: successes,
            denominator: total,
            rate: 0.0,
            lower_95: 0.0,
            upper_95: 1.0,
        };
    }
    let z = 1.959_963_984_540_054;
    let n = total as f64;
    let p = successes as f64 / n;
    let z_squared = z * z;
    let denominator = 1.0 + z_squared / n;
    let center = (p + z_squared / (2.0 * n)) / denominator;
    let margin = z * ((p * (1.0 - p) / n + z_squared / (4.0 * n * n)).sqrt()) / denominator;

    RateInterval {
        numerator: successes,
        denominator: total,
        rate: p,
        lower_95: (center - margin).max(0.0),
        upper_95: (center + margin).min(1.0),
    }
}

fn mean_and_ci(values: &[f64]) -> (f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    if values.len() == 1 {
        return (mean, mean, mean);
    }
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    let margin = 1.959_963_984_540_054 * (variance / values.len() as f64).sqrt();
    (mean, mean - margin, mean + margin)
}

fn mean_optional(values: impl Iterator<Item = Option<usize>>) -> Option<f64> {
    let collected: Vec<f64> = values
        .filter_map(|value| value.map(|tick| tick as f64))
        .collect();
    if collected.is_empty() {
        None
    } else {
        Some(collected.iter().sum::<f64>() / collected.len() as f64)
    }
}

fn aggregate(
    regime: CausalRegime,
    controller: CausalDriftController,
    outcomes: &[SeededEpisodeOutcome],
) -> SeededAggregateRow {
    let episodes = outcomes.len();
    let signal_count = outcomes
        .iter()
        .filter(|outcome| outcome.signal_detection_tick.is_some())
        .count();
    let signal_false_alarms = if expected_signal_change(regime) {
        0
    } else {
        signal_count
    };
    let correct_diagnoses = outcomes
        .iter()
        .filter(|outcome| outcome.correct_diagnosis)
        .count();
    let false_diagnoses = outcomes
        .iter()
        .filter(|outcome| outcome.false_diagnosis)
        .count();
    let missed_diagnoses = outcomes
        .iter()
        .filter(|outcome| outcome.missed_diagnosis)
        .count();

    let mean_values = |select: fn(&SeededEpisodeOutcome) -> f64| {
        let values: Vec<f64> = outcomes.iter().map(select).collect();
        mean_and_ci(&values)
    };
    let (mean_active_ticks, _, _) = mean_values(|outcome| outcome.active_ticks as f64);
    let (mean_nonpositive_effect_action_ticks, _, _) =
        mean_values(|outcome| outcome.nonpositive_effect_action_ticks as f64);
    let (mean_missed_beneficial_action_ticks, _, _) =
        mean_values(|outcome| outcome.missed_beneficial_action_ticks as f64);
    let (mean_probe_count, _, _) = mean_values(|outcome| outcome.diagnostic_probe_count as f64);
    let (mean_net_utility, net_utility_ci_lower_95, net_utility_ci_upper_95) =
        mean_values(|outcome| outcome.net_utility);
    let (mean_oracle_regret, _, _) = mean_values(|outcome| outcome.regret_to_oracle);

    SeededAggregateRow {
        regime,
        controller,
        episodes,
        expected_signal_change: expected_signal_change(regime),
        signal_alarm: wilson_interval(signal_count, episodes),
        signal_false_alarms,
        correct_diagnoses: wilson_interval(correct_diagnoses, episodes),
        false_diagnoses,
        missed_diagnoses,
        mean_signal_detection_tick: mean_optional(
            outcomes.iter().map(|outcome| outcome.signal_detection_tick),
        ),
        mean_diagnosis_tick: mean_optional(outcomes.iter().map(|outcome| outcome.diagnosis_tick)),
        mean_active_ticks,
        mean_nonpositive_effect_action_ticks,
        mean_missed_beneficial_action_ticks,
        mean_probe_count,
        mean_net_utility,
        net_utility_ci_lower_95,
        net_utility_ci_upper_95,
        mean_oracle_regret,
    }
}

pub fn run_seeded_confirmatory_benchmark() -> SeededBenchmarkReport {
    let tuning = tune_cusum();
    let mut rows = Vec::with_capacity(CausalRegime::ALL.len() * CausalDriftController::ALL.len());
    let mut replicates = Vec::with_capacity(
        CausalRegime::ALL.len() * CausalDriftController::ALL.len() * CONFIRMATORY_SEED_COUNT,
    );

    for regime in CausalRegime::ALL {
        let mut per_controller: Vec<Vec<SeededEpisodeOutcome>> = CausalDriftController::ALL
            .iter()
            .map(|_| Vec::with_capacity(CONFIRMATORY_SEED_COUNT))
            .collect();

        for seed in
            CONFIRMATORY_SEED_START..CONFIRMATORY_SEED_START + CONFIRMATORY_SEED_COUNT as u64
        {
            let (observable, truth) = build_evidence(regime, seed);
            for (index, controller) in CausalDriftController::ALL.iter().copied().enumerate() {
                per_controller[index].push(run_controller(
                    regime,
                    seed,
                    &observable,
                    &truth,
                    controller,
                    &tuning,
                ));
            }
        }

        for (index, controller) in CausalDriftController::ALL.iter().copied().enumerate() {
            rows.push(aggregate(regime, controller, &per_controller[index]));
            replicates.extend(per_controller[index].iter().cloned());
        }
    }

    SeededBenchmarkReport {
        tuning,
        confirmatory_seed_start: CONFIRMATORY_SEED_START,
        confirmatory_seed_count: CONFIRMATORY_SEED_COUNT,
        rows,
        replicates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    static REPORT: OnceLock<SeededBenchmarkReport> = OnceLock::new();

    fn report() -> &'static SeededBenchmarkReport {
        REPORT.get_or_init(run_seeded_confirmatory_benchmark)
    }

    #[test]
    fn random_stream_and_matched_probe_are_seed_reproducible() {
        let (first_evidence, first_truth) = build_evidence(CausalRegime::SignReversal, 10_042);
        let (second_evidence, second_truth) = build_evidence(CausalRegime::SignReversal, 10_042);
        assert_eq!(first_evidence, second_evidence);
        assert_eq!(first_truth, second_truth);
        assert_ne!(
            first_evidence,
            build_evidence(CausalRegime::SignReversal, 10_043).0
        );
    }

    #[test]
    fn matched_probe_uncertainty_separates_positive_null_and_negative_effects() {
        let positive = matched_probe(CausalRegime::StablePositive, 4, 10_042);
        let null = matched_probe(CausalRegime::ConfoundedNull, 4, 10_042);
        let negative = matched_probe(CausalRegime::SignReversal, 4, 10_042);
        assert_eq!(positive.pairs, PROBE_PAIRS);
        assert_eq!(classify_probe(positive), ProbeClass::Beneficial);
        assert_eq!(classify_probe(null), ProbeClass::NullEquivalent);
        assert_eq!(classify_probe(negative), ProbeClass::Harmful);
        assert!(positive.lower_95 > ACTION_COST);
        assert!(null.lower_95 > -ACTION_COST && null.upper_95 < ACTION_COST);
        assert!(negative.upper_95 < -ACTION_COST);
    }

    #[test]
    fn development_tuning_is_deterministic_and_respects_false_alarm_budget() {
        let first = tune_cusum();
        let second = tune_cusum();
        assert_eq!(first, second);
        assert!(first.false_alarm_constraint_met);
        assert!(first.development_false_alarms <= DEVELOPMENT_MAX_FALSE_ALARMS);
        assert_eq!(first.development_stable_episodes, DEVELOPMENT_SEED_COUNT);
        assert_eq!(
            first.development_changed_episodes,
            DEVELOPMENT_SEED_COUNT * 3
        );
    }

    #[test]
    fn noisy_confirmatory_portfolio_retains_all_replicates_and_rate_intervals() {
        let report = report();
        assert_eq!(report.rows.len(), 20);
        assert_eq!(report.replicates.len(), 20_000);
        assert_eq!(report.confirmatory_seed_count, CONFIRMATORY_SEED_COUNT);
        assert!(report.tuning.false_alarm_constraint_met);
        for row in &report.rows {
            assert_eq!(row.episodes, CONFIRMATORY_SEED_COUNT);
            assert!(row.signal_alarm.lower_95 <= row.signal_alarm.rate);
            assert!(row.signal_alarm.rate <= row.signal_alarm.upper_95);
            assert!(row.correct_diagnoses.lower_95 <= row.correct_diagnoses.rate);
            assert!(row.correct_diagnoses.rate <= row.correct_diagnoses.upper_95);
        }
    }

    #[test]
    fn mechanism_aware_controller_has_low_noise_false_diagnosis_and_high_detection() {
        let report = report();
        let stable = report
            .rows
            .iter()
            .find(|row| {
                row.regime == CausalRegime::StablePositive
                    && row.controller == CausalDriftController::MechanismAware
            })
            .expect("stable mechanism-aware aggregate");
        assert_eq!(stable.false_diagnoses, 0);
        assert_eq!(stable.missed_diagnoses, 0);

        for regime in [
            CausalRegime::SignReversal,
            CausalRegime::GradualSignReversal,
            CausalRegime::ConfoundedNull,
            CausalRegime::ConfounderShift,
        ] {
            let row = report
                .rows
                .iter()
                .find(|row| {
                    row.regime == regime && row.controller == CausalDriftController::MechanismAware
                })
                .expect("mechanism-aware aggregate");
            assert!(
                row.correct_diagnoses.rate >= 0.95,
                "{} diagnosis rate was {:.4}",
                regime.name(),
                row.correct_diagnoses.rate
            );
        }

        let stable_cusum = report
            .rows
            .iter()
            .find(|row| {
                row.regime == CausalRegime::StablePositive
                    && row.controller == CausalDriftController::CusumSignalOnly
            })
            .expect("stable CUSUM aggregate");
        assert!(stable_cusum.signal_alarm.upper_95 <= 0.01);
    }

    #[test]
    fn mechanism_aware_controller_preserves_beneficial_action_after_confounder_shift() {
        let report = report();
        let mechanism_aware = report
            .rows
            .iter()
            .find(|row| {
                row.regime == CausalRegime::ConfounderShift
                    && row.controller == CausalDriftController::MechanismAware
            })
            .expect("mechanism-aware confounder-shift aggregate");
        let signal_only = report
            .rows
            .iter()
            .find(|row| {
                row.regime == CausalRegime::ConfounderShift
                    && row.controller == CausalDriftController::CusumSignalOnly
            })
            .expect("signal-only confounder-shift aggregate");
        assert_eq!(mechanism_aware.missed_diagnoses, 0);
        assert!(
            mechanism_aware.mean_missed_beneficial_action_ticks
                < signal_only.mean_missed_beneficial_action_ticks
        );
    }
}
