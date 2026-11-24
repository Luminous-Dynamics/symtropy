// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Repeated-sampling evaluation for the maritime whole-schedule risk methods.
//!
//! The evaluator uses a finite synthetic population with exactly known risks.
//! The policy sees only sampled schedule outcomes; exact population values are
//! used only after selection to evaluate repeated-sampling behaviour.
//!
//! Expected-risk CRC methods are evaluated by their mean selected population
//! risk across calibration replicates. The fixed-grid Hoeffding method is
//! evaluated by its run-level selection-violation rate and Wilson interval.
//! These are deliberately different empirical questions.

use std::fmt::Write as FmtWrite;

use crate::high_probability::simultaneous_hoeffding_radius_milli;
use crate::non_monotone::excess_bound_milli;

const POLICY_COUNT: usize = 4;
const DEFAULT_REPLICATES: usize = 2_000;
const SMOKE_SCHEDULES: usize = 1_200;
const SAMPLE_SIZES: [usize; 7] = [50, 120, 500, 1_200, 5_000, 10_000, 50_000];
const ALPHA_MILLI: [u16; 4] = [10, 50, 100, 200];
const LOSS_BOUND_MILLI: u64 = 1_000;
const DELTA: f64 = 0.05;
const MASTER_SEED: u64 = 0x4D41_5249_5449_4D45;
const MASS_PER_THOUSAND: [u64; 4] = [780, 140, 60, 20];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fixture {
    Monotone,
    NonMonotone,
}

impl Fixture {
    fn label(self) -> &'static str {
        match self {
            Self::Monotone => "MONOTONE",
            Self::NonMonotone => "NON_MONOTONE",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Accumulator {
    selected_runs: usize,
    violations: usize,
    true_risk_sum: f64,
    true_risk_square_sum: f64,
    theorem_risk_sum: f64,
    theorem_risk_square_sum: f64,
    service_sum: f64,
    service_square_sum: f64,
    upper_bound_sum: f64,
    upper_bound_square_sum: f64,
}

#[derive(Clone, Copy, Debug)]
struct PopulationMetrics {
    true_risk: [f64; POLICY_COUNT],
    theorem_risk: Option<[f64; POLICY_COUNT]>,
    service_loss: [f64; POLICY_COUNT],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GuaranteeKind {
    MarginalExpectedRisk,
    FixedGridHighProbability,
    NoRiskClaim,
}

impl GuaranteeKind {
    fn label(self) -> &'static str {
        match self {
            Self::MarginalExpectedRisk => "MARGINAL_EXPECTED_RISK",
            Self::FixedGridHighProbability => "FIXED_GRID_HIGH_PROBABILITY",
            Self::NoRiskClaim => "NO_RISK_CLAIM",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RowMetadata {
    fixture: Fixture,
    method: &'static str,
    schedule_count: usize,
    replicates: usize,
    target: u16,
    guarantee: GuaranteeKind,
    status: &'static str,
}

impl Accumulator {
    fn record(
        &mut self,
        selected: Option<usize>,
        metrics: &PopulationMetrics,
        upper_bound: Option<u32>,
        target_risk_milli: u16,
        is_high_probability: bool,
    ) {
        let Some(index) = selected else {
            return;
        };

        self.selected_runs += 1;
        let risk = metrics.true_risk[index];
        self.true_risk_sum += risk;
        self.true_risk_square_sum += risk * risk;
        let service = metrics.service_loss[index];
        self.service_sum += service;
        self.service_square_sum += service * service;

        if let Some(theorem_axis) = metrics.theorem_risk {
            let value = theorem_axis[index];
            self.theorem_risk_sum += value;
            self.theorem_risk_square_sum += value * value;
        }

        if let Some(bound) = upper_bound {
            let value = f64::from(bound);
            self.upper_bound_sum += value;
            self.upper_bound_square_sum += value * value;
        }

        if is_high_probability && risk > f64::from(target_risk_milli) {
            self.violations += 1;
        }
    }
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
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// Unbiased integer draw in 0..upper using rejection sampling.
    fn below(&mut self, upper: u64) -> u64 {
        let threshold = upper.wrapping_neg() % upper;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % upper;
            }
        }
    }
}

fn hard_loss(fixture: Fixture, state: usize, policy_index: usize) -> u16 {
    let effort = policy_index as f64 / (POLICY_COUNT - 1) as f64;
    let failed = match fixture {
        Fixture::Monotone => match state {
            0 => false,
            1 => effort < 0.25,
            2 => effort < 0.75,
            3 => effort < 0.95,
            _ => unreachable!("four-state fixture"),
        },
        Fixture::NonMonotone => match state {
            0 => false,
            1 => effort < 0.25 || (0.65..=0.70).contains(&effort),
            2 => effort < 0.75 || (0.90..0.98).contains(&effort),
            3 => effort < 0.95,
            _ => unreachable!("four-state fixture"),
        },
    };
    u16::from(failed) * 1_000
}

fn service_loss(state: usize, policy_index: usize) -> u16 {
    let effort = policy_index as f64 / (POLICY_COUNT - 1) as f64;
    (20.0 + 800.0 * effort + 10.0 * state as f64).round() as u16
}

fn draw_state_counts(schedule_count: usize, seed: u64) -> [u64; 4] {
    let mut rng = SplitMix64::new(seed);
    let mut counts = [0_u64; 4];
    for _ in 0..schedule_count {
        let draw = rng.below(1_000);
        let state = if draw < 780 {
            0
        } else if draw < 920 {
            1
        } else if draw < 980 {
            2
        } else {
            3
        };
        counts[state] += 1;
    }
    counts
}

fn sampled_sums(
    fixture: Fixture,
    state_counts: &[u64; 4],
) -> (
    [u64; POLICY_COUNT],
    [u64; POLICY_COUNT],
    [f64; POLICY_COUNT],
) {
    let mut hard = [0_u64; POLICY_COUNT];
    let mut envelope = [0_u64; POLICY_COUNT];
    let mut service = [0.0_f64; POLICY_COUNT];

    for (policy_index, ((hard_total, envelope_total), service_total)) in hard
        .iter_mut()
        .zip(envelope.iter_mut())
        .zip(service.iter_mut())
        .enumerate()
    {
        for (state, count) in state_counts.iter().copied().enumerate() {
            let loss = hard_loss(fixture, state, policy_index);
            *hard_total += count * u64::from(loss);

            let future_max = (policy_index..POLICY_COUNT)
                .map(|future_index| hard_loss(fixture, state, future_index))
                .max()
                .unwrap_or(0);
            *envelope_total += count * u64::from(future_max);
            *service_total += count as f64 * f64::from(service_loss(state, policy_index));
        }
    }

    (hard, envelope, service)
}

fn exact_population_means(
    fixture: Fixture,
) -> (
    [f64; POLICY_COUNT],
    [f64; POLICY_COUNT],
    [f64; POLICY_COUNT],
) {
    let mut hard = [0.0_f64; POLICY_COUNT];
    let mut envelope = [0.0_f64; POLICY_COUNT];
    let mut service = [0.0_f64; POLICY_COUNT];

    for (policy_index, ((hard_mean, envelope_mean), service_mean)) in hard
        .iter_mut()
        .zip(envelope.iter_mut())
        .zip(service.iter_mut())
        .enumerate()
    {
        for (state, mass) in MASS_PER_THOUSAND.iter().copied().enumerate() {
            let probability = mass as f64 / 1_000.0;
            *hard_mean += probability * f64::from(hard_loss(fixture, state, policy_index));
            let future_max = (policy_index..POLICY_COUNT)
                .map(|future_index| hard_loss(fixture, state, future_index))
                .max()
                .unwrap_or(0);
            *envelope_mean += probability * f64::from(future_max);
            *service_mean += probability * f64::from(service_loss(state, policy_index));
        }
    }
    (hard, envelope, service)
}

fn select_crc(loss_sums: &[u64; POLICY_COUNT], schedule_count: usize, target: u16) -> usize {
    let rhs = u64::from(target) * (schedule_count as u64 + 1);
    for (index, sum) in loss_sums.iter().enumerate() {
        if *sum + LOSS_BOUND_MILLI <= rhs {
            return index;
        }
    }
    POLICY_COUNT - 1
}

fn select_crc_nm(
    loss_sums: &[u64; POLICY_COUNT],
    schedule_count: usize,
    target: u16,
) -> Result<usize, u32> {
    let correction = excess_bound_milli(POLICY_COUNT, schedule_count).unwrap_or(u32::MAX);
    if correction >= u32::from(target) {
        return Err(correction);
    }
    let adjusted_target = target - correction as u16;
    Ok(select_crc(loss_sums, schedule_count, adjusted_target))
}

fn select_hoeffding(
    risk_sums: &[u64; POLICY_COUNT],
    service_sums: &[f64; POLICY_COUNT],
    schedule_count: usize,
    radius_milli: u32,
    target: u16,
) -> (Option<usize>, Option<u32>) {
    let mut selected: Option<(usize, f64, u32)> = None;
    for (policy_index, (risk_sum, service_sum)) in
        risk_sums.iter().zip(service_sums.iter()).enumerate()
    {
        let empirical_mean = *risk_sum as f64 / schedule_count as f64;
        let upper = (empirical_mean + f64::from(radius_milli)).ceil() as u32;
        if upper > u32::from(target) {
            continue;
        }
        let service_mean = *service_sum / schedule_count as f64;
        let replace = selected.is_none_or(|(_, best_service, _)| service_mean < best_service);
        if replace {
            selected = Some((policy_index, service_mean, upper));
        }
    }
    selected.map_or((None, None), |(index, _, upper)| (Some(index), Some(upper)))
}

fn mean_interval(sum: f64, square_sum: f64, count: usize) -> Option<(f64, f64, f64)> {
    if count == 0 {
        return None;
    }
    let n = count as f64;
    let mean = sum / n;
    if count == 1 {
        return Some((mean, mean, mean));
    }
    let centered = (square_sum - sum * sum / n).max(0.0);
    let sample_variance = centered / (n - 1.0);
    let standard_error = (sample_variance / n).sqrt();
    let margin = 1.96 * standard_error;
    Some((mean, (mean - margin).max(0.0), mean + margin))
}

fn wilson_interval(successes: usize, trials: usize) -> (f64, f64) {
    if trials == 0 {
        return (0.0, 1.0);
    }
    let z = 1.96_f64;
    let n = trials as f64;
    let p = successes as f64 / n;
    let denominator = 1.0 + z * z / n;
    let center = (p + z * z / (2.0 * n)) / denominator;
    let half = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / denominator;
    ((center - half).max(0.0), (center + half).min(1.0))
}

fn push_optional_interval(output: &mut String, sum: f64, square_sum: f64, count: usize) {
    match mean_interval(sum, square_sum, count) {
        Some((mean, low, high)) => {
            let _ = write!(output, "{mean:.3},{low:.3},{high:.3}");
        }
        None => output.push_str("NA,NA,NA"),
    }
}

fn push_row(output: &mut String, metadata: RowMetadata, accumulator: &Accumulator) {
    let selected = accumulator.selected_runs;
    let abstentions = metadata.replicates.saturating_sub(selected);
    let _ = write!(
        output,
        "{},{},{POLICY_COUNT},{},{},{},",
        metadata.fixture.label(),
        metadata.method,
        metadata.schedule_count,
        metadata.replicates,
        metadata.target
    );

    let is_high_probability = metadata.guarantee == GuaranteeKind::FixedGridHighProbability;
    if is_high_probability {
        let _ = write!(output, "{DELTA:.3},");
    } else {
        output.push_str("NA,");
    }
    let _ = write!(output, "{selected},{abstentions},");

    push_optional_interval(
        output,
        accumulator.true_risk_sum,
        accumulator.true_risk_square_sum,
        selected,
    );
    output.push(',');
    if metadata.guarantee == GuaranteeKind::MarginalExpectedRisk {
        push_optional_interval(
            output,
            accumulator.theorem_risk_sum,
            accumulator.theorem_risk_square_sum,
            selected,
        );
    } else {
        output.push_str("NA,NA,NA");
    }
    output.push(',');
    push_optional_interval(
        output,
        accumulator.service_sum,
        accumulator.service_square_sum,
        selected,
    );
    output.push(',');
    if is_high_probability {
        push_optional_interval(
            output,
            accumulator.upper_bound_sum,
            accumulator.upper_bound_square_sum,
            selected,
        );
        let (wilson_low, wilson_high) =
            wilson_interval(accumulator.violations, metadata.replicates);
        let unconditional_rate = accumulator.violations as f64 / metadata.replicates as f64;
        let _ = write!(
            output,
            ",{unconditional_rate:.6},{wilson_low:.6},{wilson_high:.6}"
        );
        if selected == 0 {
            output.push_str(",NA");
        } else {
            let conditional_rate = accumulator.violations as f64 / selected as f64;
            let _ = write!(output, ",{conditional_rate:.6}");
        }
    } else {
        output.push_str("NA,NA,NA,NA,NA,NA,NA");
    }

    let _ = writeln!(
        output,
        ",{},{}",
        metadata.guarantee.label(),
        metadata.status
    );
}

fn render_for(sample_sizes: &[usize], replicates: usize) -> String {
    assert!(replicates > 0, "replicates must be positive");
    let mut output = String::from(
        "fixture,method,policy_grid_size,calibration_schedules,replicates,alpha_milli,delta,selected_runs,abstentions,mean_selected_true_hard_risk_milli,true_risk_mc95_low,true_risk_mc95_high,mean_selected_theorem_axis_risk_milli,theorem_risk_mc95_low,theorem_risk_mc95_high,mean_selected_service_loss_milli,service_mc95_low,service_mc95_high,mean_selected_hoeffding_ucb_milli,ucb_mc95_low,ucb_mc95_high,selection_violation_rate,wilson95_low,wilson95_high,conditional_selected_violation_rate,guarantee_kind,status\n",
    );

    let (mono_true_risk, _, mono_service) = exact_population_means(Fixture::Monotone);
    let (nonmono_true_risk, nonmono_envelope_risk, nonmono_service) =
        exact_population_means(Fixture::NonMonotone);
    let mono_expected_metrics = PopulationMetrics {
        true_risk: mono_true_risk,
        theorem_risk: Some(mono_true_risk),
        service_loss: mono_service,
    };
    let mono_hoeffding_metrics = PopulationMetrics {
        true_risk: mono_true_risk,
        theorem_risk: None,
        service_loss: mono_service,
    };
    let nonmono_envelope_metrics = PopulationMetrics {
        true_risk: nonmono_true_risk,
        theorem_risk: Some(nonmono_envelope_risk),
        service_loss: nonmono_service,
    };
    let nonmono_crc_nm_metrics = PopulationMetrics {
        true_risk: nonmono_true_risk,
        theorem_risk: Some(nonmono_true_risk),
        service_loss: nonmono_service,
    };
    let nonmono_high_probability_metrics = PopulationMetrics {
        true_risk: nonmono_true_risk,
        theorem_risk: None,
        service_loss: nonmono_service,
    };

    for &schedule_count in sample_sizes {
        let radius = simultaneous_hoeffding_radius_milli(POLICY_COUNT, schedule_count, DELTA)
            .unwrap_or(u32::MAX);
        let mut accumulators: [[Accumulator; ALPHA_MILLI.len()]; 5] =
            std::array::from_fn(|_| std::array::from_fn(|_| Accumulator::default()));

        for replicate in 0..replicates {
            let seed = MASTER_SEED
                ^ (schedule_count as u64).rotate_left(19)
                ^ (replicate as u64).wrapping_mul(0xD134_2543_DE82_EF95);
            let state_counts = draw_state_counts(schedule_count, seed);

            let (mono_hard_sums, _, mono_service_sums) =
                sampled_sums(Fixture::Monotone, &state_counts);
            let (nonmono_hard_sums, nonmono_envelope_sums, nonmono_service_sums) =
                sampled_sums(Fixture::NonMonotone, &state_counts);
            for (alpha_index, target) in ALPHA_MILLI.iter().copied().enumerate() {
                let mono_crc = select_crc(&mono_hard_sums, schedule_count, target);
                accumulators[0][alpha_index].record(
                    Some(mono_crc),
                    &mono_expected_metrics,
                    None,
                    target,
                    false,
                );

                let (mono_hoeffding, mono_upper) = select_hoeffding(
                    &mono_hard_sums,
                    &mono_service_sums,
                    schedule_count,
                    radius,
                    target,
                );
                accumulators[1][alpha_index].record(
                    mono_hoeffding,
                    &mono_hoeffding_metrics,
                    mono_upper,
                    target,
                    true,
                );

                let envelope_crc = select_crc(&nonmono_envelope_sums, schedule_count, target);
                accumulators[2][alpha_index].record(
                    Some(envelope_crc),
                    &nonmono_envelope_metrics,
                    None,
                    target,
                    false,
                );

                let nm_selected = select_crc_nm(&nonmono_hard_sums, schedule_count, target).ok();
                accumulators[3][alpha_index].record(
                    nm_selected,
                    &nonmono_crc_nm_metrics,
                    None,
                    target,
                    false,
                );

                let (nonmono_hoeffding, nonmono_upper) = select_hoeffding(
                    &nonmono_hard_sums,
                    &nonmono_service_sums,
                    schedule_count,
                    radius,
                    target,
                );
                accumulators[4][alpha_index].record(
                    nonmono_hoeffding,
                    &nonmono_high_probability_metrics,
                    nonmono_upper,
                    target,
                    true,
                );
            }
        }

        for (alpha_index, target) in ALPHA_MILLI.iter().copied().enumerate() {
            push_row(
                &mut output,
                RowMetadata {
                    fixture: Fixture::Monotone,
                    method: "MONOTONE_CRC",
                    schedule_count,
                    replicates,
                    target,
                    guarantee: GuaranteeKind::MarginalExpectedRisk,
                    status: "SYNTHETIC_ONLY",
                },
                &accumulators[0][alpha_index],
            );
            push_row(
                &mut output,
                RowMetadata {
                    fixture: Fixture::Monotone,
                    method: "SIMULTANEOUS_HOEFFDING",
                    schedule_count,
                    replicates,
                    target,
                    guarantee: GuaranteeKind::FixedGridHighProbability,
                    status: "SYNTHETIC_ONLY",
                },
                &accumulators[1][alpha_index],
            );
            push_row(
                &mut output,
                RowMetadata {
                    fixture: Fixture::NonMonotone,
                    method: "MONOTONE_ENVELOPE_CRC",
                    schedule_count,
                    replicates,
                    target,
                    guarantee: GuaranteeKind::MarginalExpectedRisk,
                    status: "SYNTHETIC_ONLY",
                },
                &accumulators[2][alpha_index],
            );
            let correction = excess_bound_milli(POLICY_COUNT, schedule_count).unwrap_or(u32::MAX);
            let nm_has_claim = correction < u32::from(target);
            let nm_status = if nm_has_claim {
                "SYNTHETIC_ONLY"
            } else {
                "NO_POSITIVE_ADJUSTED_TARGET"
            };
            push_row(
                &mut output,
                RowMetadata {
                    fixture: Fixture::NonMonotone,
                    method: "FINITE_GRID_CRC_NM",
                    schedule_count,
                    replicates,
                    target,
                    guarantee: if nm_has_claim {
                        GuaranteeKind::MarginalExpectedRisk
                    } else {
                        GuaranteeKind::NoRiskClaim
                    },
                    status: nm_status,
                },
                &accumulators[3][alpha_index],
            );
            push_row(
                &mut output,
                RowMetadata {
                    fixture: Fixture::NonMonotone,
                    method: "SIMULTANEOUS_HOEFFDING",
                    schedule_count,
                    replicates,
                    target,
                    guarantee: GuaranteeKind::FixedGridHighProbability,
                    status: "SYNTHETIC_ONLY",
                },
                &accumulators[4][alpha_index],
            );
        }
    }

    output
}

/// Fast exact-population smoke tournament: 2,000 replicates at n=1,200.
pub fn render_smoke_csv() -> String {
    render_for(&[SMOKE_SCHEDULES], DEFAULT_REPLICATES)
}

/// Full frozen synthetic sweep from #7303: 2,000 replicates at each n/alpha.
pub fn render_full_csv() -> String {
    render_for(&SAMPLE_SIZES, DEFAULT_REPLICATES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::non_monotone::{
        EndpointPopulationRiskContract, FrozenRevisions as NonMonotoneRevisions,
        WholeScheduleLossGrid,
    };
    use crate::{FrozenRevisions, LossMatrix, MaxParameterContract, MonotonicityMode, calibrate};

    #[test]
    fn splitmix_sampling_is_deterministic_and_range_checked() {
        let mut first = SplitMix64::new(123);
        let mut second = SplitMix64::new(123);
        for _ in 0..10_000 {
            let a = first.below(1_000);
            let b = second.below(1_000);
            assert_eq!(a, b);
            assert!(a < 1_000);
        }
    }

    #[test]
    fn finite_population_truth_matches_the_frozen_four_state_design() {
        let (mono, _, _) = exact_population_means(Fixture::Monotone);
        let (nonmono, _, _) = exact_population_means(Fixture::NonMonotone);
        assert_eq!(mono, [220.0, 80.0, 80.0, 0.0]);
        assert_eq!(nonmono, [220.0, 80.0, 220.0, 0.0]);
        assert!(nonmono[2] > nonmono[1]);
        assert_eq!(MASS_PER_THOUSAND.iter().sum::<u64>(), 1_000);
    }

    #[test]
    fn sampled_counts_conserve_the_complete_schedule_denominator() {
        let counts = draw_state_counts(50_000, MASTER_SEED);
        assert_eq!(counts.iter().sum::<u64>(), 50_000);
    }

    #[test]
    fn repeated_sampling_smoke_receipt_is_reproducible_and_well_formed() {
        let first = render_smoke_csv();
        assert_eq!(first, render_smoke_csv());
        assert_eq!(first.lines().count(), 21);
        let width = first.lines().next().expect("CSV header").split(',').count();
        assert_eq!(width, 27);
        assert!(first.contains("MARGINAL_EXPECTED_RISK"));
        assert!(first.contains("FIXED_GRID_HIGH_PROBABILITY"));
        assert!(first.contains("NO_RISK_CLAIM,NO_POSITIVE_ADJUSTED_TARGET"));
        assert!(first.lines().any(|line| {
            let fields = line.split(',').collect::<Vec<_>>();
            fields[1] == "SIMULTANEOUS_HOEFFDING"
                && fields[5] == "10"
                && fields[7] == "0"
                && fields[8] == "2000"
                && fields[24] == "NA"
        }));
    }

    #[test]
    fn repeated_sampling_selectors_match_the_existing_method_implementations() {
        let states = [0_usize, 1, 2, 3];
        let monotone_rows = (0..1_200)
            .map(|index| {
                let state = states[index % states.len()];
                (0..POLICY_COUNT)
                    .map(|policy| hard_loss(Fixture::Monotone, state, policy))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mono_matrix = LossMatrix {
            schedules: monotone_rows.clone(),
        };
        let revisions = FrozenRevisions {
            policy_revision: "policy-v1".into(),
            evaluator_revision: "evaluator-v1".into(),
            loss_contract_revision: "loss-v1".into(),
        };
        let max_contract = MaxParameterContract {
            scope: "finite synthetic fixture".into(),
            evidence_ref: "synthetic-terminal-policy-proof".into(),
            declared_loss_bound_milli: 0,
        };
        let mono_result = calibrate(
            &mono_matrix,
            200,
            MonotonicityMode::RequireMonotone,
            Some(&max_contract),
            &revisions,
        )
        .expect("existing monotone CRC implementation");
        let mono_sums = sum_rows(&monotone_rows);
        assert_eq!(
            select_crc(&mono_sums, monotone_rows.len(), 200),
            mono_result.selected_parameter_index
        );

        let nonmonotone_rows = (0..1_200)
            .map(|index| {
                let state = states[index % states.len()];
                (0..POLICY_COUNT)
                    .map(|policy| hard_loss(Fixture::NonMonotone, state, policy))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let nonmono_matrix = LossMatrix {
            schedules: nonmonotone_rows.clone(),
        };
        let envelope_result = calibrate(
            &nonmono_matrix,
            200,
            MonotonicityMode::UseUpperEnvelope,
            Some(&max_contract),
            &revisions,
        )
        .expect("existing monotone-envelope CRC implementation");
        let envelope = crate::upper_monotone_envelope(&nonmono_matrix);
        let envelope_sums = sum_rows(&envelope.schedules);
        assert_eq!(
            select_crc(&envelope_sums, nonmonotone_rows.len(), 200),
            envelope_result.selected_parameter_index
        );

        let nonmono_grid = WholeScheduleLossGrid {
            schedule_losses_milli: nonmonotone_rows.clone(),
        };
        let nonmono_revisions = NonMonotoneRevisions {
            policy_revision: "policy-v1".into(),
            evaluator_revision: "evaluator-v1".into(),
            loss_contract_revision: "loss-v1".into(),
        };
        let endpoint = EndpointPopulationRiskContract {
            scope: "finite synthetic fixture".into(),
            evidence_ref: "exact synthetic endpoint".into(),
            expected_loss_upper_milli: 0,
        };
        let nonmono_result =
            crate::non_monotone::calibrate(&nonmono_grid, 200, Some(&endpoint), &nonmono_revisions)
                .expect("existing finite-grid CRC-NM implementation");
        let nonmono_sums = sum_rows(&nonmonotone_rows);
        assert_eq!(
            select_crc(
                &nonmono_sums,
                nonmonotone_rows.len(),
                nonmono_result.adjusted_risk_milli
            ),
            nonmono_result.selected_parameter_index
        );

        let service_rows = (0..1_200)
            .map(|index| {
                let state = states[index % states.len()];
                (0..POLICY_COUNT)
                    .map(|policy| service_loss(state, policy))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let high_probability = crate::high_probability::select_simultaneous_hoeffding(
            &WholeScheduleLossGrid {
                schedule_losses_milli: nonmonotone_rows.clone(),
            },
            &WholeScheduleLossGrid {
                schedule_losses_milli: service_rows.clone(),
            },
            200,
            DELTA,
        );
        let service_sums = sum_f64_rows(&service_rows);
        let risk_sums = sum_rows(&nonmonotone_rows);
        let radius = simultaneous_hoeffding_radius_milli(POLICY_COUNT, 1_200, DELTA).unwrap();
        let repeated = select_hoeffding(&risk_sums, &service_sums, 1_200, radius, 200);
        match high_probability {
            Ok(result) => {
                assert_eq!(repeated.0, Some(result.selected_policy_index));
                assert_eq!(repeated.1, Some(result.selected_risk_upper_bound_milli));
            }
            Err(crate::high_probability::HighProbabilityError::NoPolicyMeetsTarget) => {
                assert_eq!(repeated, (None, None));
            }
            Err(other) => panic!("unexpected high-probability error: {other:?}"),
        }
    }

    fn sum_rows(rows: &[Vec<u16>]) -> [u64; POLICY_COUNT] {
        let mut sums = [0_u64; POLICY_COUNT];
        for row in rows {
            for (sum, loss) in sums.iter_mut().zip(row.iter()) {
                *sum += u64::from(*loss);
            }
        }
        sums
    }

    fn sum_f64_rows(rows: &[Vec<u16>]) -> [f64; POLICY_COUNT] {
        let mut sums = [0.0_f64; POLICY_COUNT];
        for row in rows {
            for (sum, loss) in sums.iter_mut().zip(row.iter()) {
                *sum += f64::from(*loss);
            }
        }
        sums
    }
}
