// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Common-stream comparison of sequential causal-inference procedures.
//! This initial reference assumes independent Gaussian paired differences
//! with known variance from the declared synthetic generator. It is not
//! distribution-free and does not establish external causal validity.

use core::fmt;

use crate::{
    causal_drift::{CAUSAL_DRIFT_TICKS, CausalRegime},
    noisy_causal_drift::{MATCHED_PAIRS_PER_LOOK, NOISY_BASE_SEED, NOISY_SEEDS, OUTCOME_NOISE_STANDARD_DEVIATION},
};

pub const MAX_PAIRS: usize = MATCHED_PAIRS_PER_LOOK;
pub const LOOKS: [usize; 4] = [512, 1024, 1536, 2048];
pub const ACTION_BENEFIT_THRESHOLD: f64 = 0.5;
pub const PAIRED_DIFFERENCE_VARIANCE: f64 = 8.0;
pub const DIRECTIONAL_ALPHA: f64 = 0.025;
pub const LOG_E_THRESHOLD: f64 = 3.6888794541139363; // ln(1 / 0.025)
pub const MIXTURE_LAMBDAS: [f64; 4] = [1.0 / 16.0, 1.0 / 8.0, 1.0 / 4.0, 1.0 / 2.0];
pub const OBF_CRITICAL_VALUES: [f64; 4] = [4.084, 2.888, 2.358, 2.042];
pub const POCOCK_CRITICAL_VALUE: f64 = 2.361;
pub const FIXED_HORIZON_CRITICAL_VALUE: f64 = 1.959963984540054;
pub const CONSERVATIVE_FOUR_LOOK_CRITICAL_VALUE: f64 = 2.8;
const CONFONDER_VALUES: [f64; 8] = [-3.0, -2.0, -1.0, 0.0, 0.0, 1.0, 2.0, 3.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequentialMethod {
    FixedHorizon,
    ConservativeFourLook,
    OBrienFleming,
    Pocock,
    GaussianMixtureEProcess,
}

impl SequentialMethod {
    pub const ALL: [Self; 5] = [
        Self::FixedHorizon,
        Self::ConservativeFourLook,
        Self::OBrienFleming,
        Self::Pocock,
        Self::GaussianMixtureEProcess,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::FixedHorizon => "fixed_horizon",
            Self::ConservativeFourLook => "conservative_four_look_z28",
            Self::OBrienFleming => "obrien_fleming",
            Self::Pocock => "pocock",
            Self::GaussianMixtureEProcess => "gaussian_mixture_e_process",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequentialDecision {
    HarmfulEffectEstablished,
    BenefitEstablished,
    Inconclusive,
}

impl SequentialDecision {
    pub const fn name(self) -> &'static str {
        match self {
            Self::HarmfulEffectEstablished => "harmful_effect_established",
            Self::BenefitEstablished => "benefit_established",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequentialError {
    InvalidStreamLength { expected: usize, actual: usize },
    NonFiniteObservation { index: usize },
    InvalidInterval,
}

impl fmt::Display for SequentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStreamLength { expected, actual } =>
                write!(f, "expected {expected} paired observations, received {actual}"),
            Self::NonFiniteObservation { index } =>
                write!(f, "paired observation at index {index} is not finite"),
            Self::InvalidInterval => write!(f, "Wilson interval requires 0 <= events <= trials and trials > 0"),
        }
    }
}
impl std::error::Error for SequentialError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SequentialReceipt {
    pub method: SequentialMethod,
    pub decision: SequentialDecision,
    pub pairs_used: usize,
    pub mean_difference: f64,
    pub z_harm: f64,
    pub z_benefit: f64,
    pub log_e_harm: Option<f64>,
    pub log_e_benefit: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialEpisodeResult {
    pub seed: u64,
    pub seed_index: usize,
    pub regime: CausalRegime,
    pub true_effect: f64,
    pub observational_association: f64,
    pub stream_hash: u64,
    pub receipt: SequentialReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WilsonInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
    spare_normal: Option<f64>,
}
impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed, spare_normal: None }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }
    fn uniform_open(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        (mantissa as f64 + 0.5) / 9007199254740992.0
    }
    fn standard_normal(&mut self) -> f64 {
        if let Some(spare) = self.spare_normal.take() { return spare; }
        let radius = (-2.0 * self.uniform_open().ln()).sqrt();
        let angle = core::f64::consts::TAU * self.uniform_open();
        let first = radius * angle.cos();
        self.spare_normal = Some(radius * angle.sin());
        first
    }
    fn normal(&mut self, sd: f64) -> f64 { self.standard_normal() * sd }
}

fn episode_seed(regime: CausalRegime, seed_index: usize) -> u64 {
    let regime_index = CausalRegime::ALL.iter()
        .position(|candidate| *candidate == regime)
        .expect("regime belongs to frozen portfolio") as u64;
    NOISY_BASE_SEED
        ^ (seed_index as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ regime_index.wrapping_mul(0xbf58476d1ce4e5b9)
}

fn stream_hash(values: &[f64]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for value in values {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x00000100000001b3);
        }
    }
    hash
}

fn generate_paired_stream(regime: CausalRegime, seed_index: usize) -> (u64, Vec<f64>, u64) {
    let seed = episode_seed(regime, seed_index);
    let mut rng = SplitMix64::new(seed);
    let effect = regime.causal_effect(CAUSAL_DRIFT_TICKS);
    let mut differences = Vec::with_capacity(MAX_PAIRS);
    for pair_index in 0..MAX_PAIRS {
        let confounder = CONFONDER_VALUES[pair_index % CONFONDER_VALUES.len()];
        let treatment = 10.0 + confounder + effect + rng.normal(OUTCOME_NOISE_STANDARD_DEVIATION);
        let control = 10.0 + confounder + rng.normal(OUTCOME_NOISE_STANDARD_DEVIATION);
        differences.push(treatment - control);
    }
    let digest = stream_hash(&differences);
    (seed, differences, digest)
}

fn log_sum_exp(values: [f64; 4]) -> f64 {
    let maximum = values.into_iter().fold(f64::NEG_INFINITY, f64::max);
    maximum + values.into_iter().map(|v| (v - maximum).exp()).sum::<f64>().ln()
}

/// Equal-weight mixture of Gaussian likelihood-ratio supermartingales.
/// The known-variance Gaussian assumption is part of this method's contract.
fn log_gaussian_mixture_e(sum_centered: f64, n: usize) -> f64 {
    let n = n as f64;
    let log_weight = -4.0_f64.ln();
    let terms = MIXTURE_LAMBDAS.map(|lambda| {
        log_weight + lambda * sum_centered
            - 0.5 * lambda * lambda * PAIRED_DIFFERENCE_VARIANCE * n
    });
    log_sum_exp(terms)
}

fn make_receipt(
    method: SequentialMethod,
    decision: SequentialDecision,
    pairs_used: usize,
    sum: f64,
    log_e_harm: Option<f64>,
    log_e_benefit: Option<f64>,
) -> SequentialReceipt {
    let n = pairs_used as f64;
    let mean = sum / n;
    let standard_error = (PAIRED_DIFFERENCE_VARIANCE / n).sqrt();
    SequentialReceipt {
        method, decision, pairs_used, mean_difference: mean,
        z_harm: mean / standard_error,
        z_benefit: (mean - ACTION_BENEFIT_THRESHOLD) / standard_error,
        log_e_harm, log_e_benefit,
    }
}

/// Evaluate one method on one frozen common stream of paired differences.
pub fn evaluate_stream(
    method: SequentialMethod,
    differences: &[f64],
) -> Result<SequentialReceipt, SequentialError> {
    if differences.len() != MAX_PAIRS {
        return Err(SequentialError::InvalidStreamLength { expected: MAX_PAIRS, actual: differences.len() });
    }
    for (index, value) in differences.iter().enumerate() {
        if !value.is_finite() { return Err(SequentialError::NonFiniteObservation { index }); }
    }

    if method == SequentialMethod::GaussianMixtureEProcess {
        let mut sum = 0.0;
        let mut last_harm = 0.0;
        let mut last_benefit = 0.0;
        for (index, value) in differences.iter().enumerate() {
            sum += value;
            let n = index + 1;
            last_harm = log_gaussian_mixture_e(-sum, n);
            last_benefit = log_gaussian_mixture_e(sum - ACTION_BENEFIT_THRESHOLD * n as f64, n);
            // Each directional null receives alpha=0.025; Ville's inequality
            // plus a union bound controls either declaration at 0.05 per episode.
            if last_harm >= LOG_E_THRESHOLD {
                return Ok(make_receipt(method, SequentialDecision::HarmfulEffectEstablished, n, sum, Some(last_harm), Some(last_benefit)));
            }
            if last_benefit >= LOG_E_THRESHOLD {
                return Ok(make_receipt(method, SequentialDecision::BenefitEstablished, n, sum, Some(last_harm), Some(last_benefit)));
            }
        }
        return Ok(make_receipt(method, SequentialDecision::Inconclusive, MAX_PAIRS, sum, Some(last_harm), Some(last_benefit)));
    }

    let mut sum = 0.0;
    let mut next_look = 0usize;
    let mut latest_sum = 0.0;
    for (index, value) in differences.iter().enumerate() {
        sum += value;
        let n = index + 1;
        if next_look >= LOOKS.len() || n != LOOKS[next_look] { continue; }
        latest_sum = sum;
        let se = (PAIRED_DIFFERENCE_VARIANCE / n as f64).sqrt();
        let mean = sum / n as f64;
        let critical = match method {
            SequentialMethod::FixedHorizon => FIXED_HORIZON_CRITICAL_VALUE,
            SequentialMethod::ConservativeFourLook => CONSERVATIVE_FOUR_LOOK_CRITICAL_VALUE,
            SequentialMethod::OBrienFleming => OBF_CRITICAL_VALUES[next_look],
            SequentialMethod::Pocock => POCOCK_CRITICAL_VALUE,
            SequentialMethod::GaussianMixtureEProcess => unreachable!(),
        };
        let decision = if mean / se <= -critical {
            Some(SequentialDecision::HarmfulEffectEstablished)
        } else if (mean - ACTION_BENEFIT_THRESHOLD) / se >= critical {
            Some(SequentialDecision::BenefitEstablished)
        } else {
            None
        };
        let is_final_look = next_look == LOOKS.len() - 1;
        if let Some(decision) = decision {
            if method != SequentialMethod::FixedHorizon || is_final_look {
                return Ok(make_receipt(method, decision, n, sum, None, None));
            }
        }
        next_look += 1;
    }
    Ok(make_receipt(method, SequentialDecision::Inconclusive, MAX_PAIRS, latest_sum, None, None))
}

pub fn run_sequential_method_portfolio() -> Vec<SequentialEpisodeResult> {
    let mut results = Vec::with_capacity(CausalRegime::ALL.len() * NOISY_SEEDS * SequentialMethod::ALL.len());
    for regime in CausalRegime::ALL {
        let true_effect = regime.causal_effect(CAUSAL_DRIFT_TICKS);
        let observational_association = true_effect + regime.observational_confounder_imbalance(CAUSAL_DRIFT_TICKS);
        for seed_index in 0..NOISY_SEEDS {
            let (seed, stream, digest) = generate_paired_stream(regime, seed_index);
            for method in SequentialMethod::ALL {
                let tested = evaluate_stream(method, &stream).expect("frozen stream satisfies method contract");
                results.push(SequentialEpisodeResult {
                    seed, seed_index, regime, true_effect, observational_association,
                    stream_hash: digest, receipt: tested,
                });
            }
        }
    }
    results
}

/// Wilson score interval for a binomial event rate.
pub fn wilson_95(events: usize, trials: usize) -> Result<WilsonInterval, SequentialError> {
    if trials == 0 || events > trials { return Err(SequentialError::InvalidInterval); }
    let z = FIXED_HORIZON_CRITICAL_VALUE;
    let n = trials as f64;
    let p = events as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let radius = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / denominator;
    Ok(WilsonInterval { lower: (center - radius).max(0.0), upper: (center + radius).min(1.0) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_is_replayable_and_all_methods_receive_same_digest() {
        let (seed_a, stream_a, hash_a) = generate_paired_stream(CausalRegime::SignReversal, 17);
        let (seed_b, stream_b, hash_b) = generate_paired_stream(CausalRegime::SignReversal, 17);
        assert_eq!((seed_a, stream_a.clone(), hash_a), (seed_b, stream_b.clone(), hash_b));
        assert_eq!(stream_a.len(), MAX_PAIRS);
        assert_ne!(hash_a, 0);
        for method in SequentialMethod::ALL {
            let receipt = evaluate_stream(method, &stream_a).expect("valid stream");
            assert!(receipt.pairs_used <= MAX_PAIRS);
        }
    }

    #[test]
    fn e_process_starts_at_one_and_separates_harm_benefit_and_inconclusive() {
        assert!(log_gaussian_mixture_e(0.0, 0).abs() < 1e-12);
        assert!((LOG_E_THRESHOLD - 40.0_f64.ln()).abs() < 1e-12);

        let (_, harmful, _) = generate_paired_stream(CausalRegime::SignReversal, 9);
        let harm = evaluate_stream(SequentialMethod::GaussianMixtureEProcess, &harmful).unwrap();
        assert_eq!(harm.decision, SequentialDecision::HarmfulEffectEstablished);
        assert!(harm.pairs_used < 100);

        let (_, beneficial, _) = generate_paired_stream(CausalRegime::StablePositive, 9);
        let benefit = evaluate_stream(SequentialMethod::GaussianMixtureEProcess, &beneficial).unwrap();
        assert_eq!(benefit.decision, SequentialDecision::BenefitEstablished);
        assert!(benefit.pairs_used < 200);

        let (_, null, _) = generate_paired_stream(CausalRegime::ConfoundedNull, 9);
        let neither = evaluate_stream(SequentialMethod::GaussianMixtureEProcess, &null).unwrap();
        assert_eq!(neither.decision, SequentialDecision::Inconclusive);
    }

    #[test]
    fn fixed_and_group_sequential_use_nested_prefix_looks() {
        assert_eq!(LOOKS, [512, 1024, 1536, MAX_PAIRS]);
        let (_, stream, _) = generate_paired_stream(CausalRegime::StablePositive, 21);
        let fixed = evaluate_stream(SequentialMethod::FixedHorizon, &stream).unwrap();
        let pocock = evaluate_stream(SequentialMethod::Pocock, &stream).unwrap();
        assert_eq!(fixed.pairs_used, MAX_PAIRS);
        assert_eq!(pocock.pairs_used, 512);
        assert_eq!(pocock.decision, SequentialDecision::BenefitEstablished);
    }

    #[test]
    fn null_operating_characteristics_stay_within_frozen_seed_gate() {
        for method in SequentialMethod::ALL {
            let false_actions = (0..NOISY_SEEDS).filter(|seed_index| {
                let (_, stream, _) = generate_paired_stream(CausalRegime::ConfoundedNull, *seed_index);
                evaluate_stream(method, &stream).unwrap().decision != SequentialDecision::Inconclusive
            }).count();
            assert!(false_actions <= 13, "{} false actions: {false_actions}/{}", method.name(), NOISY_SEEDS);
        }
    }

    #[test]
    fn all_four_non_null_endpoint_regimes_are_classified_in_the_frozen_seed_suite() {
        for regime in [CausalRegime::StablePositive, CausalRegime::SignReversal, CausalRegime::GradualSignReversal, CausalRegime::ConfounderShift] {
            for method in SequentialMethod::ALL {
                let correct_decision = if regime.causal_effect(CAUSAL_DRIFT_TICKS) < 0.0 {
                    SequentialDecision::HarmfulEffectEstablished
                } else {
                    SequentialDecision::BenefitEstablished
                };
                for seed_index in 0..NOISY_SEEDS {
                    let (_, stream, _) = generate_paired_stream(regime, seed_index);
                    let result = evaluate_stream(method, &stream).unwrap();
                    assert_eq!(result.decision, correct_decision, "{} {:?} seed {}", method.name(), regime, seed_index);
                }
            }
        }
    }

    #[test]
    fn wilson_interval_rejects_empty_denominators() {
        assert!(wilson_95(7, 256).is_ok());
        assert!(wilson_95(0, 0).is_err());
    }
}
