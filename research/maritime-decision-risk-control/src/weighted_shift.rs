// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Known-ratio importance-weighted CRC under synthetic distribution shift.
//!
//! Implements the bounded-weight setup in Proposition 4 of Aldirawi, Li & Guo,
//! arXiv:2604.01502v3 (11 Nov 2026). The theorem assumes a known likelihood
//! ratio, P_test << P_train, a finite upper bound W, i.i.d. calibration episodes,
//! independent target test episodes, and the stated terminal-risk margin.
//!
//! This module is synthetic research only. It does not estimate a density ratio
//! or certify DCT/DGT regime transfer.

const LOSS_BOUND: f64 = 1.0;
const TARGET_RISK: f64 = 0.20;
const WEIGHT_BOUND: f64 = 5.0;
const TERMINAL_MARGIN: f64 = 0.15;
const TEST_EPISODES: usize = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Episode {
    pub severe_regime: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LossMatrix {
    /// One bounded hard-failure loss vector per complete schedule episode.
    pub hard_failure_loss_milli: Vec<[u16; 4]>,
    /// Service burden is deliberately separate from hard-failure risk.
    pub service_loss_milli: Vec<[u16; 4]>,
    pub severe_regime: Vec<bool>,
}

impl LossMatrix {
    pub fn len(&self) -> usize {
        self.severe_regime.len()
    }

    pub fn is_empty(&self) -> bool {
        self.severe_regime.is_empty()
    }

    fn validate(&self) -> Result<(), ShiftRiskError> {
        if self.is_empty() {
            return Err(ShiftRiskError::EmptyPopulation);
        }
        if self.hard_failure_loss_milli.len() != self.len()
            || self.service_loss_milli.len() != self.len()
        {
            return Err(ShiftRiskError::PopulationLengthMismatch);
        }
        if self
            .hard_failure_loss_milli
            .iter()
            .chain(self.service_loss_milli.iter())
            .flatten()
            .any(|value| *value > 1_000)
        {
            return Err(ShiftRiskError::LossOutsideUnitInterval);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShiftRiskError {
    EmptyPopulation,
    PopulationLengthMismatch,
    LossOutsideUnitInterval,
    EmptyCalibration,
    InvalidWeight,
    WeightExceedsDeclaredBound,
    InvalidMargin,
    NoTerminalMargin,
    NoWeightedPolicySelected,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeightedSelection {
    pub selected_policy_index: usize,
    pub calibration_episodes: usize,
    pub weighted_empirical_risk: f64,
    pub target_alpha: f64,
    pub weight_bound: f64,
    pub margin_a: f64,
    pub correction_d_w: f64,
    pub expected_risk_upper_bound: f64,
    pub terminal_margin_slack: f64,
    pub claim_is_informative: bool,
}

#[derive(Clone, Copy, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, upper: u32) -> u32 {
        (self.next_u64() % u64::from(upper)) as u32
    }
}

/// Source distribution: P_train(severe)=0.1.
/// Target distribution: P_test(severe)=0.5.
/// The exact likelihood ratio is 5 for severe and 5/9 for normal episodes.
pub fn exact_importance_weight(episode: Episode) -> f64 {
    if episode.severe_regime {
        WEIGHT_BOUND
    } else {
        WEIGHT_BOUND / 9.0
    }
}

fn hard_failure_losses(severe: bool) -> [u16; 4] {
    if severe {
        // Deterministic fails under severe conditions; buffered policy avoids it;
        // the third candidate is partially resilient; terminal fallback is safe.
        [1_000, 0, 500, 0]
    } else {
        // Buffering creates a small resource-interference failure in normal conditions.
        [0, 200, 0, 0]
    }
}

fn service_losses(severe: bool) -> [u16; 4] {
    if severe {
        [800, 300, 500, 1_000]
    } else {
        [0, 100, 0, 1_000]
    }
}

pub fn sample_train_episodes(count: usize, seed: u64) -> LossMatrix {
    let mut rng = SplitMix64::new(seed);
    let severe_regime = (0..count).map(|_| rng.below(10) == 0).collect::<Vec<_>>();
    let hard_failure_loss_milli = severe_regime
        .iter()
        .copied()
        .map(hard_failure_losses)
        .collect();
    let service_loss_milli = severe_regime.iter().copied().map(service_losses).collect();
    LossMatrix {
        hard_failure_loss_milli,
        service_loss_milli,
        severe_regime,
    }
}

pub fn sample_test_episodes(count: usize, seed: u64) -> LossMatrix {
    let mut rng = SplitMix64::new(seed);
    let severe_regime = (0..count).map(|_| rng.below(2) == 1).collect::<Vec<_>>();
    let hard_failure_loss_milli = severe_regime
        .iter()
        .copied()
        .map(hard_failure_losses)
        .collect();
    let service_loss_milli = severe_regime
        .iter()
        .copied()
        .map(service_losses)
        .collect();
    LossMatrix {
        hard_failure_loss_milli,
        service_loss_milli,
        severe_regime,
    }
}

fn empirical_risk(losses: &[[u16; 4]], weights: Option<&[f64]>, policy_index: usize) -> f64 {
    let total = losses
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let weight = weights.map_or(1.0, |values| values[index]);
            weight * f64::from(row[policy_index]) / 1_000.0
        })
        .sum::<f64>();
    total / losses.len() as f64
}

pub fn select_unweighted_policy(
    calibration: &LossMatrix,
    target_alpha: f64,
) -> Result<usize, ShiftRiskError> {
    calibration.validate()?;
    if target_alpha <= 0.0 || target_alpha >= LOSS_BOUND {
        return Err(ShiftRiskError::InvalidMargin);
    }
    let n = calibration.len() as f64;
    for policy_index in 0..4 {
        let risk = empirical_risk(&calibration.hard_failure_loss_milli, None, policy_index);
        if n / (n + 1.0) * risk + LOSS_BOUND / (n + 1.0) <= target_alpha {
            return Ok(policy_index);
        }
    }
    Ok(3)
}

/// Importance-weighted policy selection and Proposition 4 expected-risk bound.
///
/// Assumes the likelihood ratio is known exactly and bounded by WEIGHT_BOUND,
/// and the terminal policy has target risk 0 by construction. This is a synthetic
/// hard-failure-risk claim, not a guarantee on the separate service-loss vector.
pub fn select_weighted_policy(
    calibration: &LossMatrix,
    target_alpha: f64,
    weight_bound: f64,
    margin_a: f64,
) -> Result<WeightedSelection, ShiftRiskError> {
    calibration.validate()?;
    if calibration.is_empty() {
        return Err(ShiftRiskError::EmptyCalibration);
    }
    if weight_bound <= 0.0 || !weight_bound.is_finite() {
        return Err(ShiftRiskError::InvalidWeight);
    }
    if margin_a <= 0.0 || !margin_a.is_finite() {
        return Err(ShiftRiskError::InvalidMargin);
    }
    if target_alpha <= 0.0 || target_alpha >= LOSS_BOUND {
        return Err(ShiftRiskError::InvalidMargin);
    }

    let weights = calibration
        .severe_regime
        .iter()
        .copied()
        .map(|severe| {
            exact_importance_weight(Episode {
                severe_regime: severe,
            })
        })
        .collect::<Vec<_>>();
    if weights.iter().any(|weight| *weight > weight_bound) {
        return Err(ShiftRiskError::WeightExceedsDeclaredBound);
    }

    let n = calibration.len() as f64;
    let terminal_margin_slack =
        target_alpha - (weight_bound * LOSS_BOUND - target_alpha) / n - margin_a;
    if terminal_margin_slack <= 0.0 {
        return Err(ShiftRiskError::NoTerminalMargin);
    }

    let mut selected_policy_index = None;
    let mut selected_weighted_risk = 0.0;
    for policy_index in 0..4 {
        let weighted_risk = empirical_risk(
            &calibration.hard_failure_loss_milli,
            Some(&weights),
            policy_index,
        );
        if n / (n + 1.0) * weighted_risk + weight_bound * LOSS_BOUND / (n + 1.0) <= target_alpha {
            selected_policy_index = Some(policy_index);
            selected_weighted_risk = weighted_risk;
            break;
        }
    }

    let selected_policy_index =
        selected_policy_index.ok_or(ShiftRiskError::NoWeightedPolicySelected)?;
    let m = 4.0_f64;
    let d_w = weight_bound * LOSS_BOUND * ((2.0 * m).ln() / (2.0 * n)).sqrt()
        + weight_bound * LOSS_BOUND / (2.0 * (2.0 * n * (2.0 * m).ln()).sqrt());
    let tail_term = (LOSS_BOUND - target_alpha)
        * (-2.0 * n * margin_a * margin_a
            / (weight_bound * weight_bound * LOSS_BOUND * LOSS_BOUND))
            .exp();
    let risk_upper_bound = target_alpha + d_w + tail_term;

    Ok(WeightedSelection {
        selected_policy_index,
        calibration_episodes: calibration.len(),
        weighted_empirical_risk: selected_weighted_risk,
        target_alpha,
        weight_bound,
        margin_a,
        correction_d_w: d_w,
        expected_risk_upper_bound: risk_upper_bound,
        terminal_margin_slack,
        claim_is_informative: risk_upper_bound <= LOSS_BOUND,
    })
}

pub fn mean_test_loss(matrix: &LossMatrix, policy_index: usize, service: bool) -> f64 {
    let rows = if service {
        &matrix.service_loss_milli
    } else {
        &matrix.hard_failure_loss_milli
    };
    empirical_risk(rows, None, policy_index)
}

fn render_row(name: &str, calibration: &LossMatrix, test: &LossMatrix) -> String {
    let weighted = select_weighted_policy(calibration, TARGET_RISK, WEIGHT_BOUND, TERMINAL_MARGIN)
        .expect("known-weight synthetic calibration");
    let naive = select_unweighted_policy(calibration, TARGET_RISK)
        .expect("unweighted empirical calibration");
    let weighted_test_risk = mean_test_loss(test, weighted.selected_policy_index, false);
    let naive_test_risk = mean_test_loss(test, naive, false);
    let weighted_service_loss = mean_test_loss(test, weighted.selected_policy_index, true);
    let naive_service_loss = mean_test_loss(test, naive, true);
    let status = if weighted.claim_is_informative {
        "EXPECTED_RISK_BOUND"
    } else {
        "UPPER_BOUND_UNINFORMATIVE"
    };

    format!(
        "{name},KNOWN_EXACT_RATIO,{},10000,4,{:.3},{:.3},{:.3},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{status}\n",
        weighted.calibration_episodes,
        weighted.target_alpha,
        weighted.weight_bound,
        weighted.margin_a,
        weighted.selected_policy_index,
        naive,
        weighted_test_risk,
        naive_test_risk,
        weighted_service_loss,
        naive_service_loss,
        weighted.expected_risk_upper_bound,
    )
}

/// Deterministic 15-column receipt comparing naive and exact known-weight selection.
pub fn render_csv() -> String {
    let mut output = String::from(
        "experiment,shift_type,calibration_episodes,target_test_episodes,policy_grid_size,target_alpha,known_weight_bound,terminal_margin_a,weighted_selected_policy,naive_unweighted_selected_policy,weighted_policy_target_hard_risk,naive_policy_target_hard_risk,weighted_policy_service_loss,naive_policy_service_loss,weighted_expected_risk_upper_bound,status\n",
    );

    for (name, n, seed) in [
        ("N120", 120, 0x5745_4947_4854_0120_u64),
        ("N1200", 1_200, 0x5745_4947_4854_1200_u64),
        ("N50000", 50_000, 0x5745_4947_4854_5000_u64),
    ] {
        let calibration = sample_train_episodes(n, seed);
        let test = sample_test_episodes(TEST_EPISODES, seed.wrapping_add(1));
        output.push_str(&render_row(name, &calibration, &test));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_density_ratio_is_normalized_under_source_distribution() {
        let expected_weight = 0.1 * WEIGHT_BOUND + 0.9 * (WEIGHT_BOUND / 9.0);
        assert!((expected_weight - 1.0).abs() < 1e-12);
    }

    #[test]
    fn target_distribution_is_not_exchangeable_with_source_distribution() {
        let source = sample_train_episodes(100_000, 12);
        let target = sample_test_episodes(100_000, 13);
        let source_severe = source.severe_regime.iter().filter(|value| **value).count() as f64
            / source.len() as f64;
        let target_severe = target.severe_regime.iter().filter(|value| **value).count() as f64
            / target.len() as f64;
        assert!((source_severe - 0.1).abs() < 0.01);
        assert!((target_severe - 0.5).abs() < 0.01);
    }

    #[test]
    fn naive_unweighted_policy_is_not_target_calibrated() {
        let calibration = sample_train_episodes(1_200, 0x5745_4947_4854_1200);
        let test = sample_test_episodes(TEST_EPISODES, 0x5745_4947_4854_1201);
        let naive = select_unweighted_policy(&calibration, TARGET_RISK).expect("naive selector");
        let target_risk = mean_test_loss(&test, naive, false);
        assert_eq!(naive, 0);
        assert!(target_risk > 0.40);
    }

    #[test]
    fn known_weight_selector_changes_policy_and_reduces_target_hard_risk() {
        let calibration = sample_train_episodes(1_200, 0x5745_4947_4854_1200);
        let test = sample_test_episodes(TEST_EPISODES, 0x5745_4947_4854_1201);
        let weighted =
            select_weighted_policy(&calibration, TARGET_RISK, WEIGHT_BOUND, TERMINAL_MARGIN)
                .expect("weighted selector");
        let naive = select_unweighted_policy(&calibration, TARGET_RISK).expect("naive selector");
        let weighted_risk = mean_test_loss(&test, weighted.selected_policy_index, false);
        let naive_risk = mean_test_loss(&test, naive, false);

        assert_eq!(weighted.selected_policy_index, 1);
        assert_eq!(naive, 0);
        assert!(weighted_risk < naive_risk);
        assert!(weighted.expected_risk_upper_bound <= LOSS_BOUND);
    }

    #[test]
    fn weighted_selector_enforces_terminal_margin_and_weight_bound() {
        let calibration = sample_train_episodes(120, 0x5745_4947_4854_0120);
        assert_eq!(
            select_weighted_policy(&calibration, TARGET_RISK, WEIGHT_BOUND, 0.25),
            Err(ShiftRiskError::NoTerminalMargin)
        );
        assert_eq!(
            select_weighted_policy(&calibration, TARGET_RISK, 0.1, TERMINAL_MARGIN),
            Err(ShiftRiskError::WeightExceedsDeclaredBound)
        );
    }

    #[test]
    fn weighted_expected_risk_bound_tightens_with_calibration_size() {
        let small = sample_train_episodes(1_200, 0x5745_4947_4854_1200);
        let large = sample_train_episodes(50_000, 0x5745_4947_4854_5000);
        let small_result =
            select_weighted_policy(&small, TARGET_RISK, WEIGHT_BOUND, TERMINAL_MARGIN)
                .expect("small weighted fit");
        let large_result =
            select_weighted_policy(&large, TARGET_RISK, WEIGHT_BOUND, TERMINAL_MARGIN)
                .expect("large weighted fit");
        assert!(large_result.expected_risk_upper_bound < small_result.expected_risk_upper_bound);
    }

    #[test]
    fn service_loss_is_not_collapsed_into_hard_failure_risk() {
        let test = sample_test_episodes(TEST_EPISODES, 9876);
        let no_action_service_loss = mean_test_loss(&test, 3, true);
        let no_action_hard_failure = mean_test_loss(&test, 3, false);
        assert_eq!(no_action_hard_failure, 0.0);
        assert_eq!(no_action_service_loss, 1.0);
    }

    #[test]
    fn deterministic_receipt_replays_and_preserves_three_calibration_arms() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 4);
        assert_eq!(first.lines().next().expect("header").split(',').count(), 16);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), 16);
        }
    }
}
