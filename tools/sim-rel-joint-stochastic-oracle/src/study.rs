//! Paired multi-seed benchmark runner and reproducible bootstrap summaries.

use super::{
    Rational,
    benchmark::{
        BenchmarkConfig, BenchmarkReport, MethodResult, fingerprint_model_family,
        run_benchmark_suite,
    },
    challenge::generate_stateful_confounding_family,
};
use std::collections::BTreeSet;

const MAX_STUDY_SEEDS: usize = 10_000;
const MAX_BOOTSTRAP_RESAMPLES: usize = 1_000_000;

/// Configuration for a paired generated-family benchmark study.
///
/// Use at least 30 seeds for comparative claims. The runner accepts 2+ for
/// deterministic smoke tests and early development.
#[derive(Clone, Debug, PartialEq)]
pub struct PairedStudyConfig {
    pub seeds: Vec<u64>,
    pub benchmark: BenchmarkConfig,
    pub bootstrap_resamples: usize,
    pub confidence_level: f64,
    pub bootstrap_seed: u64,
}

impl Default for PairedStudyConfig {
    fn default() -> Self {
        Self {
            seeds: (0..30).collect(),
            benchmark: BenchmarkConfig {
                horizon: 2,
                ..BenchmarkConfig::default()
            },
            bootstrap_resamples: 2_000,
            confidence_level: 0.95,
            bootstrap_seed: 0x243f_6a88_85a3_08d3,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairedStudyRun {
    pub seed: u64,
    pub observation_noise_rate: Rational,
    pub shifted_hazard_rate: Rational,
    pub calibration_damage_rate: Rational,
    pub model_family_fingerprint: String,
    pub exact_robust_optimum_feasible: bool,
    pub benchmark: BenchmarkReport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactMetricSummary {
    pub mean: Rational,
    pub median: Rational,
    pub maximum: Rational,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PairedBootstrapInterval {
    pub paired_sample_count: usize,
    pub mean_regret: Rational,
    pub median_regret: Rational,
    pub lower_percentile: f64,
    pub upper_percentile: f64,
    pub confidence_level: f64,
    pub bootstrap_resamples: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairedMethodSummary {
    pub method: String,
    pub run_count: usize,
    pub feasible_run_count: usize,
    pub infeasible_run_count: usize,
    pub paired_feasible_run_count: usize,
    pub feasible_rate: Rational,
    pub worst_case_cost: ExactMetricSummary,
    pub worst_case_hazard: ExactMetricSummary,
    pub worst_case_false_identification: ExactMetricSummary,
    pub worst_case_abstention: ExactMetricSummary,
    pub worst_case_correct_identification: ExactMetricSummary,
    /// Conditional on runs where both this method and the exact robust optimum
    /// satisfy the same budgets. Infeasible-run frequency remains separate.
    pub paired_cost_regret: Option<PairedBootstrapInterval>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairedStudyReport {
    pub seeds: Vec<u64>,
    pub unique_model_family_count: usize,
    pub horizon: usize,
    pub max_hazard: Rational,
    pub max_false_identification: Rational,
    pub max_abstention: Rational,
    pub source_commit: Option<String>,
    pub bootstrap_seed: u64,
    pub runs: Vec<PairedStudyRun>,
    pub method_summaries: Vec<PairedMethodSummary>,
}

/// Run the same methods under paired generated model families for every seed.
pub fn run_paired_study(config: &PairedStudyConfig) -> Result<PairedStudyReport, String> {
    if config.seeds.len() < 2 {
        return Err("paired study requires at least two seeds".into());
    }
    if config.seeds.len() > MAX_STUDY_SEEDS {
        return Err(format!("paired study seed count exceeds {MAX_STUDY_SEEDS}"));
    }
    let mut unique_seeds = BTreeSet::new();
    for seed in &config.seeds {
        if !unique_seeds.insert(*seed) {
            return Err("paired study seeds must be unique".into());
        }
    }
    if config.bootstrap_resamples == 0
        || config.bootstrap_resamples > MAX_BOOTSTRAP_RESAMPLES
    {
        return Err(format!(
            "bootstrap resample count must lie in 1..={MAX_BOOTSTRAP_RESAMPLES}"
        ));
    }
    if !config.confidence_level.is_finite()
        || config.confidence_level <= 0.0
        || config.confidence_level >= 1.0
    {
        return Err("bootstrap confidence level must lie strictly between zero and one".into());
    }

    let mut runs = Vec::with_capacity(config.seeds.len());
    for seed in &config.seeds {
        let family = generate_stateful_confounding_family(*seed)?;
        let fingerprint = fingerprint_model_family(&family.worlds)?;
        let mut benchmark_config = config.benchmark.clone();
        benchmark_config.seed = *seed;
        let benchmark = run_benchmark_suite(
            &family.worlds,
            &family.nominal_world_id,
            &benchmark_config,
        )?;
        runs.push(PairedStudyRun {
            seed: *seed,
            observation_noise_rate: family.observation_noise_rate,
            shifted_hazard_rate: family.shifted_hazard_rate,
            calibration_damage_rate: family.calibration_damage_rate,
            model_family_fingerprint: fingerprint,
            exact_robust_optimum_feasible: benchmark.exact_robust_optimal.is_some(),
            benchmark,
        });
    }

    let method_names: Vec<String> = runs[0]
        .benchmark
        .methods
        .iter()
        .map(|method| method.method.clone())
        .collect();
    let mut method_summaries = Vec::with_capacity(method_names.len());
    for method_name in method_names {
        let mut results: Vec<(&MethodResult, bool)> = Vec::with_capacity(runs.len());
        for run in &runs {
            let result = run.benchmark.methods
                .iter()
                .find(|method| method.method == method_name)
                .ok_or_else(|| format!("method {method_name} missing from paired seed {}", run.seed))?;
            results.push((result, run.exact_robust_optimum_feasible));
        }
        method_summaries.push(summarize_method(
            &method_name,
            &results,
            runs.len(),
            config.bootstrap_resamples,
            config.confidence_level,
            config.bootstrap_seed,
        )?);
    }

    let unique_model_family_count = runs
        .iter()
        .map(|run| run.model_family_fingerprint.as_str())
        .collect::<BTreeSet<_>>()
        .len();

    Ok(PairedStudyReport {
        seeds: config.seeds.clone(),
        unique_model_family_count,
        horizon: config.benchmark.horizon,
        max_hazard: config.benchmark.max_hazard,
        max_false_identification: config.benchmark.max_false_identification,
        max_abstention: config.benchmark.max_abstention,
        source_commit: config.benchmark.source_commit.clone(),
        bootstrap_seed: config.bootstrap_seed,
        runs,
        method_summaries,
    })
}

fn summarize_method(
    method_name: &str,
    results: &[(&MethodResult, bool)],
    run_count: usize,
    bootstrap_resamples: usize,
    confidence_level: f64,
    bootstrap_seed: u64,
) -> Result<PairedMethodSummary, String> {
    let mut costs = Vec::with_capacity(run_count);
    let mut hazards = Vec::with_capacity(run_count);
    let mut false_claims = Vec::with_capacity(run_count);
    let mut abstentions = Vec::with_capacity(run_count);
    let mut correct = Vec::with_capacity(run_count);
    let mut regrets = Vec::new();
    let mut feasible_run_count = 0_usize;
    let mut paired_feasible_run_count = 0_usize;

    for (result, exact_optimum_feasible) in results {
        costs.push(result.evaluation.worst_case_expected_cost);
        hazards.push(result.evaluation.worst_case_hazard_probability);
        false_claims.push(result.evaluation.worst_case_false_identification_probability);
        abstentions.push(result.evaluation.worst_case_abstention_probability);
        correct.push(result.evaluation.worst_case_correct_identification_probability);

        if result.satisfies_robust_budgets {
            feasible_run_count += 1;
            if *exact_optimum_feasible {
                if let Some(regret) = result.worst_case_cost_regret {
                    regrets.push(regret);
                    paired_feasible_run_count += 1;
                }
            }
        }
    }

    let paired_cost_regret = if regrets.len() >= 2 {
        Some(bootstrap_mean_interval(
            &regrets,
            bootstrap_resamples,
            confidence_level,
            bootstrap_seed ^ stable_hash(method_name),
        )?)
    } else {
        None
    };

    Ok(PairedMethodSummary {
        method: method_name.to_string(),
        run_count,
        feasible_run_count,
        infeasible_run_count: run_count - feasible_run_count,
        paired_feasible_run_count,
        feasible_rate: Rational::new(feasible_run_count as u128, run_count as u128)?,
        worst_case_cost: summarize_exact(&costs)?,
        worst_case_hazard: summarize_exact(&hazards)?,
        worst_case_false_identification: summarize_exact(&false_claims)?,
        worst_case_abstention: summarize_exact(&abstentions)?,
        worst_case_correct_identification: summarize_exact(&correct)?,
        paired_cost_regret,
    })
}

fn summarize_exact(values: &[Rational]) -> Result<ExactMetricSummary, String> {
    if values.is_empty() {
        return Err("cannot summarize an empty paired metric".into());
    }
    let sum = values.iter().try_fold(
        Rational::zero(),
        |total, value| total.checked_add(*value),
    )?;
    let mean = sum.checked_div(Rational::new(values.len() as u128, 1)?)?;
    let mut sorted = values.to_vec();
    sorted.sort();
    let median = if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2]
    } else {
        sorted[sorted.len() / 2 - 1]
            .checked_add(sorted[sorted.len() / 2])?
            .checked_mul(Rational::new(1, 2)?)?
    };
    let maximum = *sorted.last().ok_or_else(|| "metric sort unexpectedly empty".to_string())?;
    Ok(ExactMetricSummary { mean, median, maximum })
}

fn bootstrap_mean_interval(
    paired_regrets: &[Rational],
    resamples: usize,
    confidence_level: f64,
    seed: u64,
) -> Result<PairedBootstrapInterval, String> {
    if paired_regrets.len() < 2 || resamples == 0 {
        return Err("paired bootstrap requires at least two samples and positive repetitions".into());
    }
    // Sorting before sampling makes the finite bootstrap sequence invariant to input seed order.
    let mut values = paired_regrets.to_vec();
    values.sort();
    let exact = summarize_exact(&values)?;
    let n = values.len();
    let mut rng = StudyRng::new(seed);
    let mut bootstrap_means = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let mut sample_sum = 0.0_f64;
        for _ in 0..n {
            let index = rng.index(n);
            sample_sum += to_f64(values[index]);
        }
        bootstrap_means.push(sample_sum / n as f64);
    }
    bootstrap_means.sort_by(f64::total_cmp);
    let tail = (1.0 - confidence_level) / 2.0;
    let lower_index = ((tail * resamples as f64).floor() as usize).min(resamples - 1);
    let upper_index = (((1.0 - tail) * resamples as f64).ceil() as usize)
        .saturating_sub(1)
        .min(resamples - 1);
    Ok(PairedBootstrapInterval {
        paired_sample_count: n,
        mean_regret: exact.mean,
        median_regret: exact.median,
        lower_percentile: bootstrap_means[lower_index],
        upper_percentile: bootstrap_means[upper_index],
        confidence_level,
        bootstrap_resamples: resamples,
    })
}

fn to_f64(value: Rational) -> f64 {
    value.numerator() as f64 / value.denominator() as f64
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[derive(Clone, Copy, Debug)]
struct StudyRng {
    state: u64,
}

impl StudyRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xa076_1d64_78bd_642f } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn index(&mut self, length: usize) -> usize {
        let bound = length as u64;
        let rejection_threshold = (0_u64.wrapping_sub(bound)) % bound;
        loop {
            let value = self.next_u64();
            if value >= rejection_threshold {
                return (value % bound) as usize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smoke_config(seeds: Vec<u64>) -> PairedStudyConfig {
        PairedStudyConfig {
            seeds,
            benchmark: BenchmarkConfig {
                horizon: 2,
                max_hazard: Rational::zero(),
                max_false_identification: Rational::zero(),
                max_abstention: Rational::zero(),
                ..BenchmarkConfig::default()
            },
            bootstrap_resamples: 200,
            confidence_level: 0.95,
            bootstrap_seed: 12345,
        }
    }

    #[test]
    fn paired_study_reproduces_per_seed_reports_and_bootstrap_intervals() {
        let config = smoke_config(vec![0, 1, 2]);
        let first = run_paired_study(&config).unwrap();
        let second = run_paired_study(&config).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.runs.len(), 3);
        assert_eq!(first.unique_model_family_count, 3);
        assert!(first.method_summaries.iter().all(|summary| summary.run_count == 3));
        assert_ne!(
            first.runs[0].shifted_hazard_rate,
            first.runs[0].calibration_damage_rate
        );
    }

    #[test]
    fn aggregate_summaries_are_invariant_to_seed_order() {
        let forward = run_paired_study(&smoke_config(vec![0, 1, 2])).unwrap();
        let reversed = run_paired_study(&smoke_config(vec![2, 1, 0])).unwrap();
        assert_eq!(forward.method_summaries, reversed.method_summaries);
    }

    #[test]
    fn exact_robust_optimizer_chooses_independent_reference_under_zero_budgets() {
        let report = run_paired_study(&smoke_config(vec![0, 1, 2])).unwrap();
        for run in &report.runs {
            let optimum = run.benchmark.exact_robust_optimal.as_ref().unwrap();
            assert_eq!(optimum.worst_case_expected_cost(), Rational::from_u64(5));
            assert_eq!(optimum.worst_case_false_identification_probability(), Rational::zero());
            assert_eq!(optimum.worst_case_hazard_probability(), Rational::zero());
            assert_eq!(optimum.worst_case_abstention_probability(), Rational::zero());
            assert!(run.exact_robust_optimum_feasible);
        }
        let positive_control = report.method_summaries.iter()
            .find(|summary| summary.method == "independent-reference-positive-control")
            .unwrap();
        assert_eq!(positive_control.feasible_run_count, 3);
        assert_eq!(positive_control.paired_feasible_run_count, 3);
        let interval = positive_control.paired_cost_regret.unwrap();
        assert_eq!(interval.paired_sample_count, 3);
        assert_eq!(interval.mean_regret, Rational::zero());
        assert_eq!(interval.lower_percentile, 0.0);
        assert_eq!(interval.upper_percentile, 0.0);
    }

    #[test]
    fn infeasible_baselines_are_not_mixed_into_feasible_regret_intervals() {
        let report = run_paired_study(&smoke_config(vec![0, 1, 2])).unwrap();
        let greedy = report.method_summaries.iter()
            .find(|summary| summary.method == "receding-horizon-greedy-one-step-EIG")
            .unwrap();
        assert!(greedy.infeasible_run_count > 0);
        if let Some(interval) = greedy.paired_cost_regret {
            assert_eq!(interval.paired_sample_count, greedy.paired_feasible_run_count);
        }
    }

    #[test]
    fn default_seed_grid_produces_thirty_distinct_finite_model_families() {
        let fingerprints: std::collections::BTreeSet<String> = (0..30)
            .map(|seed| {
                let family = generate_stateful_confounding_family(seed).unwrap();
                fingerprint_model_family(&family.worlds).unwrap()
            })
            .collect();
        assert_eq!(fingerprints.len(), 30);
    }

    #[test]
    fn duplicate_seeds_invalid_bootstrap_and_too_few_seeds_fail_closed() {
        assert!(run_paired_study(&smoke_config(vec![1])).is_err());
        assert!(run_paired_study(&smoke_config(vec![1, 1])).is_err());
        assert!(run_paired_study(&PairedStudyConfig {
            bootstrap_resamples: 0,
            ..smoke_config(vec![0, 1])
        }).is_err());
        assert!(run_paired_study(&PairedStudyConfig {
            bootstrap_resamples: MAX_BOOTSTRAP_RESAMPLES + 1,
            ..smoke_config(vec![0, 1])
        }).is_err());
        assert!(run_paired_study(&PairedStudyConfig {
            confidence_level: 1.0,
            ..smoke_config(vec![0, 1])
        }).is_err());
    }
}
