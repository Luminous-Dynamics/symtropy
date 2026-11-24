// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Cluster-aware calibration under shared maritime operating conditions.
//!
//! This is a synthetic falsification benchmark, not a real-port dependence
//! estimate. It creates independent operating clusters (for example, days or
//! disruption episodes) and perfectly correlated schedule rows within each
//! cluster. The schedule-level baseline deliberately pseudo-replicates those
//! rows as if they were independent. The cluster-unit comparator uses one
//! bounded cluster-mean loss vector per independent cluster.
//!
//! The expected-risk CRC and high-probability Hoeffding guarantees remain
//! distinct. The cluster-unit claims apply only to the synthetic IID-cluster
//! construction and the declared cluster-mean estimand.

use std::fmt::Write as FmtWrite;

use crate::high_probability::simultaneous_hoeffding_radius_milli;

const POLICY_COUNT: usize = 4;
const REPLICATES: usize = 2_000;
const TOTAL_SCHEDULES: usize = 50_000;
const CLUSTER_COUNTS: [usize; 8] = [10, 25, 50, 100, 250, 500, 1_000, 2_000];
const SEVERE_PROBABILITY_PER_MILLE: u64 = 50;
const TARGET_RISK_MILLI: u16 = 40;
const DELTA: f64 = 0.05;
const LOSS_BOUND_MILLI: u64 = 1_000;
const INTRA_CLUSTER_CORRELATION_MILLI: u16 = 1_000;
const SERVICE_COST_MILLI: [u16; POLICY_COUNT] = [100, 200, 300, 400];
const MASTER_SEED: u64 = 0x434C_5553_5445_5252;

// Pointwise-monotone bounded loss family for the marginal expected-risk CRC.
const CRC_NORMAL_LOSS: [u16; POLICY_COUNT] = [39, 38, 30, 0];
const CRC_SEVERE_LOSS: [u16; POLICY_COUNT] = [820, 39, 35, 0];

// Separate high-probability selector fixture. It intentionally makes the
// lowest-cost policy unsafe under the population severe-cluster frequency.
const UCB_NORMAL_LOSS: [u16; POLICY_COUNT] = [30, 38, 30, 0];
const UCB_SEVERE_LOSS: [u16; POLICY_COUNT] = [830, 38, 30, 0];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fixture {
    ExpectedRiskCrc,
    HighProbabilityUcb,
}

impl Fixture {
    fn label(self) -> &'static str {
        match self {
            Self::ExpectedRiskCrc => "EXPECTED_RISK_CRC_PROFILE",
            Self::HighProbabilityUcb => "HIGH_PROBABILITY_UCB_PROFILE",
        }
    }

    fn normal_loss(self) -> &'static [u16; POLICY_COUNT] {
        match self {
            Self::ExpectedRiskCrc => &CRC_NORMAL_LOSS,
            Self::HighProbabilityUcb => &UCB_NORMAL_LOSS,
        }
    }

    fn severe_loss(self) -> &'static [u16; POLICY_COUNT] {
        match self {
            Self::ExpectedRiskCrc => &CRC_SEVERE_LOSS,
            Self::HighProbabilityUcb => &UCB_SEVERE_LOSS,
        }
    }

    fn population_risk(self) -> [f64; POLICY_COUNT] {
        let mut risk = [0.0; POLICY_COUNT];
        for (index, value) in risk.iter_mut().enumerate() {
            *value = (1_000.0 - SEVERE_PROBABILITY_PER_MILLE as f64)
                * f64::from(self.normal_loss()[index])
                / 1_000.0
                + SEVERE_PROBABILITY_PER_MILLE as f64
                    * f64::from(self.severe_loss()[index])
                    / 1_000.0;
        }
        risk
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CalibrationUnit {
    SchedulePseudoReplicate,
    IndependentCluster,
}

impl CalibrationUnit {
    fn label(self) -> &'static str {
        match self {
            Self::SchedulePseudoReplicate => "SCHEDULE_ROWS_AS_IID",
            Self::IndependentCluster => "INDEPENDENT_CLUSTER_MEANS",
        }
    }

    fn assumption_status(self) -> &'static str {
        match self {
            Self::SchedulePseudoReplicate => "INVALID_IID_PSEUDOREPLICATION_NEGATIVE_CONTROL",
            Self::IndependentCluster => "IID_CLUSTERS_SYNTHETIC_ONLY",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Method {
    MarginalExpectedRiskCrc,
    FixedGridHighProbabilityHoeffding,
}

impl Method {
    fn label(self) -> &'static str {
        match self {
            Self::MarginalExpectedRiskCrc => "FINITE_SAMPLE_CRC",
            Self::FixedGridHighProbabilityHoeffding => "SIMULTANEOUS_HOEFFDING_UCB",
        }
    }

    fn guarantee_class(self) -> &'static str {
        match self {
            Self::MarginalExpectedRiskCrc => "MARGINAL_EXPECTED_RISK",
            Self::FixedGridHighProbabilityHoeffding => "FIXED_GRID_HIGH_PROBABILITY",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Experiment {
    independent_clusters: usize,
    schedules_per_cluster: usize,
    total_schedules: usize,
}

impl Experiment {
    fn new(independent_clusters: usize) -> Self {
        assert!(independent_clusters > 0);
        assert_eq!(TOTAL_SCHEDULES % independent_clusters, 0);
        Self {
            independent_clusters,
            schedules_per_cluster: TOTAL_SCHEDULES / independent_clusters,
            total_schedules: TOTAL_SCHEDULES,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Accumulator {
    selected_runs: usize,
    unsafe_selections: usize,
    true_risk_sum: f64,
    true_risk_square_sum: f64,
    service_cost_sum: f64,
    service_cost_square_sum: f64,
}

impl Accumulator {
    fn record(
        &mut self,
        selected: Option<usize>,
        population_risk: &[f64; POLICY_COUNT],
        replicates: usize,
    ) {
        let Some(policy_index) = selected else {
            return;
        };
        self.selected_runs += 1;
        let risk = population_risk[policy_index];
        self.true_risk_sum += risk;
        self.true_risk_square_sum += risk * risk;
        let service_cost = f64::from(SERVICE_COST_MILLI[policy_index]);
        self.service_cost_sum += service_cost;
        self.service_cost_square_sum += service_cost * service_cost;
        if risk > f64::from(TARGET_RISK_MILLI) {
            self.unsafe_selections += 1;
        }
        debug_assert!(self.selected_runs <= replicates);
    }

    fn mean_selected_risk(&self) -> Option<(f64, f64, f64)> {
        mean_interval(
            self.true_risk_sum,
            self.true_risk_square_sum,
            self.selected_runs,
        )
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

fn sample_severe_cluster_count(
    clusters: usize,
    replicate_seed: u64,
) -> usize {
    let mut rng = SplitMix64::new(replicate_seed);
    (0..clusters)
        .filter(|_| rng.below(1_000) < SEVERE_PROBABILITY_PER_MILLE)
        .count()
}

/// Sum of one bounded cluster-mean loss vector per independent cluster.
fn cluster_loss_sums(
    fixture: Fixture,
    independent_clusters: usize,
    severe_clusters: usize,
) -> [u64; POLICY_COUNT] {
    let normal_clusters = independent_clusters - severe_clusters;
    std::array::from_fn(|policy_index| {
        normal_clusters as u64 * u64::from(fixture.normal_loss()[policy_index])
            + severe_clusters as u64 * u64::from(fixture.severe_loss()[policy_index])
    })
}

/// The same observations expanded into repeated schedule rows. With equal
/// cluster sizes the sample mean is unchanged, but the caller incorrectly
/// substitutes G*S for G as the independent sample count.
fn schedule_pseudo_replicated_sums(
    cluster_sums: &[u64; POLICY_COUNT],
    schedules_per_cluster: usize,
) -> [u64; POLICY_COUNT] {
    std::array::from_fn(|index| cluster_sums[index] * schedules_per_cluster as u64)
}

fn select_crc(
    loss_sums_milli: &[u64; POLICY_COUNT],
    calibration_units: usize,
) -> usize {
    let denominator = calibration_units as u64 + 1;
    let target = u64::from(TARGET_RISK_MILLI);

    for (index, loss_sum) in loss_sums_milli.iter().copied().enumerate() {
        if loss_sum + LOSS_BOUND_MILLI <= target * denominator {
            return index;
        }
    }

    // Existing CRC contract: the terminal policy is admitted only because
    // its exact by-design loss bound is zero in this synthetic fixture.
    POLICY_COUNT - 1
}

fn select_hoeffding(
    loss_sums_milli: &[u64; POLICY_COUNT],
    calibration_units: usize,
) -> Option<usize> {
    let radius = simultaneous_hoeffding_radius_milli(
        POLICY_COUNT,
        calibration_units,
        DELTA,
    )
    .ok()?;

    for (index, loss_sum) in loss_sums_milli.iter().copied().enumerate() {
        let empirical_mean = loss_sum as f64 / calibration_units as f64;
        let upper = (empirical_mean + f64::from(radius)).ceil() as u32;
        if upper <= u32::from(TARGET_RISK_MILLI) {
            return Some(index);
        }
    }
    None
}

fn mean_interval(
    sum: f64,
    square_sum: f64,
    count: usize,
) -> Option<(f64, f64, f64)> {
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
    let margin = 1.96 * (sample_variance / n).sqrt();
    Some((mean, (mean - margin).max(0.0), mean + margin))
}

fn wilson_interval(successes: usize, trials: usize) -> (f64, f64) {
    if trials == 0 {
        return (0.0, 1.0);
    }
    let z = 1.96_f64;
    let n = trials as f64;
    let proportion = successes as f64 / n;
    let denominator = 1.0 + z * z / n;
    let center = (proportion + z * z / (2.0 * n)) / denominator;
    let half = z
        * (proportion * (1.0 - proportion) / n + z * z / (4.0 * n * n)).sqrt()
        / denominator;
    (
        (center - half).max(0.0),
        (center + half).min(1.0),
    )
}

fn push_optional_interval(
    output: &mut String,
    value: Option<(f64, f64, f64)>,
) {
    match value {
        Some((mean, low, high)) => {
            let _ = write!(output, "{mean:.3},{low:.3},{high:.3}");
        }
        None => output.push_str("NA,NA,NA"),
    }
}

#[derive(Clone, Copy, Debug)]
struct RowMetadata {
    fixture: Fixture,
    method: Method,
    unit: CalibrationUnit,
    experiment: Experiment,
}

fn push_row(
    output: &mut String,
    metadata: RowMetadata,
    accumulator: &Accumulator,
) {
    let selected = accumulator.selected_runs;
    let abstentions = REPLICATES.saturating_sub(selected);
    let _ = write!(
        output,
        "{},{},{},{},{},{},{},{},{},{},",
        metadata.fixture.label(),
        metadata.method.label(),
        metadata.unit.label(),
        metadata.experiment.independent_clusters,
        metadata.experiment.schedules_per_cluster,
        metadata.experiment.total_schedules,
        REPLICATES,
        TARGET_RISK_MILLI,
        if metadata.method == Method::FixedGridHighProbabilityHoeffding {
            format!("{DELTA:.3}")
        } else {
            "NA".to_owned()
        },
        INTRA_CLUSTER_CORRELATION_MILLI
    );
    let _ = write!(output, "{selected},{abstentions},");
    push_optional_interval(output, accumulator.mean_selected_risk());
    output.push(',');
    let unsafe_rate = accumulator.unsafe_selections as f64 / REPLICATES as f64;
    let (wilson_low, wilson_high) =
        wilson_interval(accumulator.unsafe_selections, REPLICATES);
    let _ = write!(
        output,
        "{unsafe_rate:.6},{wilson_low:.6},{wilson_high:.6},"
    );
    if selected == 0 {
        output.push_str("NA,NA");
    } else {
        let conditional_rate =
            accumulator.unsafe_selections as f64 / selected as f64;
        let mean_service_cost = accumulator.service_cost_sum / selected as f64;
        let _ = write!(output, "{conditional_rate:.6},{mean_service_cost:.3}");
    }
    let _ = writeln!(
        output,
        ",{},{}",
        metadata.method.guarantee_class(),
        metadata.unit.assumption_status()
    );
}

fn run_experiment(
    fixture: Fixture,
    method: Method,
    unit: CalibrationUnit,
    experiment: Experiment,
) -> Accumulator {
    let population_risk = fixture.population_risk();
    let mut accumulator = Accumulator::default();

    for replicate in 0..REPLICATES {
        let seed = MASTER_SEED
            ^ (experiment.independent_clusters as u64).rotate_left(17)
            ^ (replicate as u64).wrapping_mul(0xD134_2543_DE82_EF95);
        let severe_clusters = sample_severe_cluster_count(
            experiment.independent_clusters,
            seed,
        );
        let cluster_sums = cluster_loss_sums(
            fixture,
            experiment.independent_clusters,
            severe_clusters,
        );

        let (calibration_sums, calibration_units) = match unit {
            CalibrationUnit::SchedulePseudoReplicate => (
                schedule_pseudo_replicated_sums(
                    &cluster_sums,
                    experiment.schedules_per_cluster,
                ),
                experiment.total_schedules,
            ),
            CalibrationUnit::IndependentCluster => (
                cluster_sums,
                experiment.independent_clusters,
            ),
        };

        let selected = match method {
            Method::MarginalExpectedRiskCrc => {
                Some(select_crc(&calibration_sums, calibration_units))
            }
            Method::FixedGridHighProbabilityHoeffding => {
                select_hoeffding(&calibration_sums, calibration_units)
            }
        };

        accumulator.record(selected, &population_risk, REPLICATES);
    }

    accumulator
}

/// Repeated-sampling experiment with a fixed total schedule count and varying
/// independent-cluster count. The intra-cluster correlation is exactly one by
/// construction; it is a stress negative control, not a field estimate.
pub fn render_csv() -> String {
    let mut output = String::from(
        "fixture,method,calibration_unit,independent_clusters,schedules_per_cluster,total_schedules,replicates,alpha_milli,delta,icc_milli,selected_runs,abstentions,mean_selected_true_population_risk_milli,risk_mc95_low,risk_mc95_high,selected_unsafe_rate,wilson95_low,wilson95_high,conditional_selected_unsafe_rate,mean_selected_service_cost_milli,guarantee_class,assumption_status\n",
    );

    for independent_clusters in CLUSTER_COUNTS {
        let experiment = Experiment::new(independent_clusters);
        for fixture in [Fixture::ExpectedRiskCrc, Fixture::HighProbabilityUcb] {
            for method in [
                Method::MarginalExpectedRiskCrc,
                Method::FixedGridHighProbabilityHoeffding,
            ] {
                for unit in [
                    CalibrationUnit::SchedulePseudoReplicate,
                    CalibrationUnit::IndependentCluster,
                ] {
                    let accumulator =
                        run_experiment(fixture, method, unit, experiment);
                    push_row(
                        &mut output,
                        RowMetadata {
                            fixture,
                            method,
                            unit,
                            experiment,
                        },
                        &accumulator,
                    );
                }
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::non_monotone::WholeScheduleLossGrid;
    use crate::{
        FrozenRevisions, LossMatrix, MaxParameterContract, MonotonicityMode,
        calibrate,
    };

    #[test]
    fn cluster_level_population_risks_match_exact_known_answers() {
        assert_eq!(
            Fixture::ExpectedRiskCrc.population_risk(),
            [78.05, 38.05, 30.25, 0.0]
        );
        assert_eq!(
            Fixture::HighProbabilityUcb.population_risk(),
            [70.0, 38.0, 30.0, 0.0]
        );
    }

    #[test]
    fn one_thousand_schedule_rows_do_not_create_one_thousand_independent_units() {
        let experiment = Experiment::new(50);
        assert_eq!(experiment.schedules_per_cluster, 1_000);
        assert_eq!(experiment.total_schedules, 50_000);

        let cluster_sums = cluster_loss_sums(Fixture::ExpectedRiskCrc, 50, 0);
        let pseudo_sums =
            schedule_pseudo_replicated_sums(&cluster_sums, 1_000);

        assert_eq!(
            select_crc(&pseudo_sums, experiment.total_schedules),
            0
        );
        assert_eq!(select_crc(&cluster_sums, 50), 3);
    }

    #[test]
    fn pseudo_replicated_crc_exceeds_target_but_cluster_unit_crc_does_not() {
        let experiment = Experiment::new(50);
        let naive = run_experiment(
            Fixture::ExpectedRiskCrc,
            Method::MarginalExpectedRiskCrc,
            CalibrationUnit::SchedulePseudoReplicate,
            experiment,
        );
        let cluster = run_experiment(
            Fixture::ExpectedRiskCrc,
            Method::MarginalExpectedRiskCrc,
            CalibrationUnit::IndependentCluster,
            experiment,
        );

        let naive_mean = naive.true_risk_sum / naive.selected_runs as f64;
        let cluster_mean = cluster.true_risk_sum / cluster.selected_runs as f64;
        assert!(naive_mean > f64::from(TARGET_RISK_MILLI));
        assert!(cluster_mean <= f64::from(TARGET_RISK_MILLI));
    }

    #[test]
    fn pseudo_replicated_hoeffding_exceeds_delta_while_cluster_method_abstains() {
        let experiment = Experiment::new(50);
        let naive = run_experiment(
            Fixture::HighProbabilityUcb,
            Method::FixedGridHighProbabilityHoeffding,
            CalibrationUnit::SchedulePseudoReplicate,
            experiment,
        );
        let cluster = run_experiment(
            Fixture::HighProbabilityUcb,
            Method::FixedGridHighProbabilityHoeffding,
            CalibrationUnit::IndependentCluster,
            experiment,
        );

        let naive_unsafe_rate =
            naive.unsafe_selections as f64 / REPLICATES as f64;
        assert!(naive_unsafe_rate > DELTA);
        assert_eq!(cluster.selected_runs, 0);
        assert_eq!(cluster.unsafe_selections, 0);
    }

    #[test]
    fn enough_independent_clusters_allow_the_cluster_ucb_to_select_a_safe_policy() {
        let experiment = Experiment::new(2_000);
        let cluster = run_experiment(
            Fixture::HighProbabilityUcb,
            Method::FixedGridHighProbabilityHoeffding,
            CalibrationUnit::IndependentCluster,
            experiment,
        );
        assert!(cluster.selected_runs > 0);
        assert_eq!(cluster.unsafe_selections, 0);
    }

    #[test]
    fn repeated_receipt_is_deterministic_and_has_expected_schema() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 33);
        let width = first
            .lines()
            .next()
            .expect("CSV header")
            .split(',')
            .count();
        assert_eq!(width, 22);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
        assert!(first.contains("INVALID_IID_PSEUDOREPLICATION_NEGATIVE_CONTROL"));
        assert!(first.contains("IID_CLUSTERS_SYNTHETIC_ONLY"));
    }

    #[test]
    fn custom_crc_selector_matches_existing_finite_sample_implementation() {
        let normal_row = CRC_NORMAL_LOSS.to_vec();
        let cluster_matrix = LossMatrix {
            schedules: vec![normal_row.clone(); 50],
        };
        let revisions = FrozenRevisions {
            policy_revision: "cluster-policy-v1".into(),
            evaluator_revision: "cluster-evaluator-v1".into(),
            loss_contract_revision: "cluster-loss-v1".into(),
        };
        let contract = MaxParameterContract {
            scope: "synthetic fixed terminal policy".into(),
            evidence_ref: "exact-zero-terminal-risk-by-design".into(),
            declared_loss_bound_milli: 0,
        };
        let reference = calibrate(
            &cluster_matrix,
            TARGET_RISK_MILLI,
            MonotonicityMode::RequireMonotone,
            Some(&contract),
            &revisions,
        )
        .expect("existing cluster-unit CRC implementation");
        let sums = cluster_loss_sums(Fixture::ExpectedRiskCrc, 50, 0);
        assert_eq!(
            select_crc(&sums, 50),
            reference.selected_parameter_index
        );
    }

    #[test]
    fn custom_hoeffding_selector_matches_existing_fixed_grid_implementation() {
        let risk_grid = WholeScheduleLossGrid {
            schedule_losses_milli: vec![UCB_NORMAL_LOSS.to_vec(); 1_200],
        };
        let service_grid = WholeScheduleLossGrid {
            schedule_losses_milli: vec![SERVICE_COST_MILLI.to_vec(); 1_200],
        };
        let reference = crate::high_probability::select_simultaneous_hoeffding(
            &risk_grid,
            &service_grid,
            TARGET_RISK_MILLI,
            DELTA,
        )
        .expect("existing high-probability selector");
        let sums = cluster_loss_sums(Fixture::HighProbabilityUcb, 1_200, 0);
        let repeated = select_hoeffding(&sums, 1_200);
        assert_eq!(repeated, Some(reference.selected_policy_index));
    }

    #[test]
    fn seed_stream_is_reproducible_and_severe_cluster_count_is_bounded() {
        let first = sample_severe_cluster_count(2_000, MASTER_SEED);
        let second = sample_severe_cluster_count(2_000, MASTER_SEED);
        assert_eq!(first, second);
        assert!(first <= 2_000);
    }
}
