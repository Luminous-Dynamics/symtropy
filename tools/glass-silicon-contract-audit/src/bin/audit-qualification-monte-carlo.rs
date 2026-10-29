use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "glass-qualification-paired-operating-characteristics-v0-1";
const CLAIM_CEILING: &str = "Synthetic IID Bernoulli simulation only; finite Monte Carlo estimates do not prove type-I error control or validate real-material, production-yield, metrology, dependence, drift, or industrial-performance claims.";
const P0: f64 = 0.95;
const ALPHA: f64 = 0.05;
const CANDIDATE_FAMILY_SIZE: usize = 5;
const DEFAULT_REPLICATIONS: usize = 2_500;
const DEFAULT_SEED: u64 = 20_261_029;
const DEFAULT_MAX_TRIALS: usize = 400;
const PROBABILITIES: [f64; 4] = [0.90, 0.95, 0.98, 0.99];
const Z_975: f64 = 1.959_963_984_540_054;

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn close(left: f64, right: f64, tolerance: f64) -> bool {
    left.is_finite() && right.is_finite() && (left - right).abs() <= tolerance
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    Qualified,
    NotQualified,
    Unresolved,
}

impl Decision {
    fn as_str(self) -> &'static str {
        match self {
            Self::Qualified => "QUALIFIED",
            Self::NotQualified => "NOT_QUALIFIED",
            Self::Unresolved => "UNRESOLVED",
        }
    }

    fn is_terminal(self) -> bool {
        self != Self::Unresolved
    }
}

#[derive(Clone, Debug)]
struct Thresholds {
    // index -> n -> successes -> decision
    alpha_spending: Vec<Vec<Vec<Decision>>>,
    mixture_e_process: Vec<Vec<Vec<Decision>>>,
}

#[derive(Clone, Copy, Debug)]
struct MethodCounts {
    qualified: u64,
    not_qualified: u64,
    unresolved: u64,
    trials_total: u64,
    any_qualified_families: u64,
    any_rejected_families: u64,
    false_decision_families: u64,
}

impl MethodCounts {
    fn new() -> Self {
        Self {
            qualified: 0,
            not_qualified: 0,
            unresolved: 0,
            trials_total: 0,
            any_qualified_families: 0,
            any_rejected_families: 0,
            false_decision_families: 0,
        }
    }

    fn add_candidate(&mut self, decision: Decision, trials: usize) {
        self.trials_total += trials as u64;
        match decision {
            Decision::Qualified => self.qualified += 1,
            Decision::NotQualified => self.not_qualified += 1,
            Decision::Unresolved => self.unresolved += 1,
        }
    }

    fn add_family(
        &mut self,
        any_qualified: bool,
        any_rejected: bool,
        true_probability: f64,
    ) {
        if any_qualified {
            self.any_qualified_families += 1;
        }
        if any_rejected {
            self.any_rejected_families += 1;
        }
        // At p < p0, qualification is the false terminal decision. At p > p0,
        // rejection is false. At p == p0, either terminal direction is false.
        let false_decision = if true_probability < P0 {
            any_qualified
        } else if true_probability > P0 {
            any_rejected
        } else {
            any_qualified || any_rejected
        };
        if false_decision {
            self.false_decision_families += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
struct WilsonInterval {
    lower: f64,
    upper: f64,
}

#[derive(Clone, Debug, Serialize)]
struct MethodSummary {
    method: String,
    candidate_attempts: u64,
    mean_trials_per_candidate: f64,
    qualified_candidates: u64,
    not_qualified_candidates: u64,
    unresolved_candidates: u64,
    candidate_qualification_rate: f64,
    candidate_rejection_rate: f64,
    candidate_unresolved_rate: f64,
    families_with_any_qualification: u64,
    families_with_any_rejection: u64,
    familywise_false_decision_count: u64,
    familywise_false_decision_rate: f64,
    familywise_false_decision_wilson_95: WilsonInterval,
}

#[derive(Clone, Debug, Serialize)]
struct ScenarioSummary {
    true_success_probability: f64,
    alpha_spending: MethodSummary,
    mixture_e_process: MethodSummary,
    paired_mean_trials_saved_per_candidate: f64,
    paired_trial_reduction_fraction: f64,
}

#[derive(Clone, Debug, Serialize)]
struct BenchmarkReport {
    schema_version: u32,
    status: String,
    benchmark_id: String,
    evidence_class: String,
    claim_ceiling: String,
    configuration_sha256: String,
    prng: String,
    seed: u64,
    replications_per_probability: usize,
    candidate_family_size: usize,
    max_trials_per_candidate: usize,
    total_candidate_attempts_per_probability: u64,
    target_success_probability: f64,
    familywise_alpha: f64,
    probabilities: Vec<f64>,
    notes: Vec<String>,
    scenarios: Vec<ScenarioSummary>,
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
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn next_unit_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / 9_007_199_254_740_992.0)
    }

    fn bernoulli(&mut self, p: f64) -> bool {
        self.next_unit_f64() < p
    }
}

fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn stream_seed(root_seed: u64, scenario: usize, replication: usize, candidate: usize) -> u64 {
    mix64(
        root_seed
            ^ mix64((scenario as u64 + 1).wrapping_mul(0xd6e8_feb8_6659_fd93))
            ^ mix64((replication as u64 + 1).wrapping_mul(0xa076_1d64_78bd_642f))
            ^ mix64((candidate as u64 + 1).wrapping_mul(0xe703_7ed1_a0b4_28db)),
    )
}

fn log_add_exp(left: f64, right: f64) -> f64 {
    if left == f64::NEG_INFINITY {
        return right;
    }
    if right == f64::NEG_INFINITY {
        return left;
    }
    let maximum = left.max(right);
    maximum + ((left - maximum).exp() + (right - maximum).exp()).ln()
}

fn log_binomial_pmf(n: usize, k: usize, p: f64, log_factorial: &[f64]) -> f64 {
    log_factorial[n] - log_factorial[k] - log_factorial[n - k]
        + (k as f64) * p.ln()
        + ((n - k) as f64) * (-p).ln_1p()
}

// Compute stable log tails in O(max_n^2) total time. Each row contains
// P[X <= x] and P[X >= x] for X ~ Binomial(n, P0).
fn binomial_log_tails(
    max_n: usize,
    log_factorial: &[f64],
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let mut lower_tails = Vec::with_capacity(max_n + 1);
    let mut upper_tails = Vec::with_capacity(max_n + 1);

    for n in 0..=max_n {
        let pmf: Vec<f64> = (0..=n)
            .map(|x| log_binomial_pmf(n, x, P0, log_factorial))
            .collect();

        let mut lower = vec![f64::NEG_INFINITY; n + 1];
        lower[0] = pmf[0];
        for x in 1..=n {
            lower[x] = log_add_exp(lower[x - 1], pmf[x]).min(0.0);
        }

        let mut upper = vec![f64::NEG_INFINITY; n + 1];
        upper[n] = pmf[n];
        for x in (0..n).rev() {
            upper[x] = log_add_exp(pmf[x], upper[x + 1]).min(0.0);
        }

        lower_tails.push(lower);
        upper_tails.push(upper);
    }
    (lower_tails, upper_tails)
}

fn log_e_upper(
    n: usize,
    successes: usize,
    log_factorial: &[f64],
    lower_tails: &[Vec<f64>],
) -> f64 {
    let failures = n - successes;
    let log_beta = log_factorial[successes] + log_factorial[failures]
        - log_factorial[n + 1];
    // Beta(x+1,n-x+1) survival at P0 is P[Binomial(n+1,P0) <= x].
    log_beta + lower_tails[n + 1][successes]
        - (1.0 - P0).ln()
        - (successes as f64) * P0.ln()
        - (failures as f64) * (1.0 - P0).ln()
}

fn log_e_lower(
    n: usize,
    successes: usize,
    log_factorial: &[f64],
    upper_tails: &[Vec<f64>],
) -> f64 {
    let failures = n - successes;
    let log_beta = log_factorial[successes] + log_factorial[failures]
        - log_factorial[n + 1];
    // Beta(x+1,n-x+1) CDF at P0 is P[Binomial(n+1,P0) >= x+1].
    log_beta + upper_tails[n + 1][successes + 1]
        - P0.ln()
        - (successes as f64) * P0.ln()
        - (failures as f64) * (1.0 - P0).ln()
}

fn make_thresholds(max_trials: usize) -> Thresholds {
    let max_n = max_trials + 1;
    let mut log_factorial = vec![0.0; max_n + 1];
    for n in 1..=max_n {
        log_factorial[n] = log_factorial[n - 1] + (n as f64).ln();
    }
    let (lower_tails, upper_tails) = binomial_log_tails(max_n, &log_factorial);

    let mut alpha_all = Vec::with_capacity(CANDIDATE_FAMILY_SIZE + 1);
    let mut e_all = Vec::with_capacity(CANDIDATE_FAMILY_SIZE + 1);
    alpha_all.push(Vec::new());
    e_all.push(Vec::new());

    for index in 1..=CANDIDATE_FAMILY_SIZE {
        let mut alpha_by_n = Vec::with_capacity(max_trials + 1);
        let mut e_by_n = Vec::with_capacity(max_trials + 1);
        alpha_by_n.push(vec![Decision::Unresolved]);
        e_by_n.push(vec![Decision::Unresolved]);

        for n in 1..=max_trials {
            let alpha_n = ALPHA
                / (2.0 * index as f64 * (index + 1) as f64 * n as f64 * (n + 1) as f64);
            let log_alpha_n = alpha_n.ln();
            let log_e_threshold = -(ALPHA
                / (2.0 * index as f64 * (index + 1) as f64))
                .ln();

            let mut alpha_row = Vec::with_capacity(n + 1);
            let mut e_row = Vec::with_capacity(n + 1);
            for x in 0..=n {
                let alpha_qualifies = upper_tails[n][x] <= log_alpha_n;
                let alpha_rejects = lower_tails[n][x] <= log_alpha_n;
                let alpha_decision = match (alpha_qualifies, alpha_rejects) {
                    (true, false) => Decision::Qualified,
                    (false, true) => Decision::NotQualified,
                    _ => Decision::Unresolved,
                };
                alpha_row.push(alpha_decision);

                let e_qualifies = log_e_upper(n, x, &log_factorial, &lower_tails)
                    >= log_e_threshold;
                let e_rejects = log_e_lower(n, x, &log_factorial, &upper_tails)
                    >= log_e_threshold;
                let e_decision = match (e_qualifies, e_rejects) {
                    (true, false) => Decision::Qualified,
                    (false, true) => Decision::NotQualified,
                    _ => Decision::Unresolved,
                };
                e_row.push(e_decision);
            }
            alpha_by_n.push(alpha_row);
            e_by_n.push(e_row);
        }
        alpha_all.push(alpha_by_n);
        e_all.push(e_by_n);
    }

    Thresholds {
        alpha_spending: alpha_all,
        mixture_e_process: e_all,
    }
}

fn run_one(
    outcomes: &[bool],
    registration_index: usize,
    tables: &[Vec<Vec<Decision>>],
) -> (Decision, usize) {
    let mut successes = 0usize;
    for (offset, passed) in outcomes.iter().enumerate() {
        if *passed {
            successes += 1;
        }
        let n = offset + 1;
        let decision = tables[registration_index][n][successes];
        if decision.is_terminal() {
            return (decision, n);
        }
    }
    (Decision::Unresolved, outcomes.len())
}

fn wilson_interval(successes: u64, trials: u64) -> WilsonInterval {
    if trials == 0 {
        return WilsonInterval { lower: 0.0, upper: 1.0 };
    }
    let n = trials as f64;
    let p_hat = successes as f64 / n;
    let z2 = Z_975 * Z_975;
    let denominator = 1.0 + z2 / n;
    let center = (p_hat + z2 / (2.0 * n)) / denominator;
    let half_width = Z_975
        * ((p_hat * (1.0 - p_hat) / n + z2 / (4.0 * n * n)).sqrt())
        / denominator;
    WilsonInterval {
        lower: (center - half_width).max(0.0),
        upper: (center + half_width).min(1.0),
    }
}

fn summarize(
    method: &str,
    counts: MethodCounts,
    replications: usize,
) -> MethodSummary {
    let attempts = (replications * CANDIDATE_FAMILY_SIZE) as u64;
    let family_trials = replications as u64;
    MethodSummary {
        method: method.to_owned(),
        candidate_attempts: attempts,
        mean_trials_per_candidate: counts.trials_total as f64 / attempts as f64,
        qualified_candidates: counts.qualified,
        not_qualified_candidates: counts.not_qualified,
        unresolved_candidates: counts.unresolved,
        candidate_qualification_rate: counts.qualified as f64 / attempts as f64,
        candidate_rejection_rate: counts.not_qualified as f64 / attempts as f64,
        candidate_unresolved_rate: counts.unresolved as f64 / attempts as f64,
        families_with_any_qualification: counts.any_qualified_families,
        families_with_any_rejection: counts.any_rejected_families,
        familywise_false_decision_count: counts.false_decision_families,
        familywise_false_decision_rate: counts.false_decision_families as f64
            / family_trials as f64,
        familywise_false_decision_wilson_95: wilson_interval(
            counts.false_decision_families,
            family_trials,
        ),
    }
}

fn configuration_sha256(reps: usize, seed: u64, max_trials: usize) -> String {
    let configuration = format!(
        "{{\"benchmark_id\":\"{BENCHMARK_ID}\",\"p0\":{P0},\"alpha\":{ALPHA},\"probabilities\":[0.9,0.95,0.98,0.99],\"candidate_family_size\":{CANDIDATE_FAMILY_SIZE},\"replications\":{reps},\"seed\":{seed},\"max_trials\":{max_trials},\"prng\":\"SplitMix64-v1\",\"alpha_spending\":\"alpha/[2*i*(i+1)*n*(n+1)] per tail\",\"e_process\":\"uniform Bernoulli likelihood-ratio mixtures on [0,p0] and [p0,1], alpha/[i*(i+1)] split across tails\"}}"
    );
    sha256(configuration.as_bytes())
}

fn run_benchmark(replications: usize, seed: u64, max_trials: usize) -> BenchmarkReport {
    let thresholds = make_thresholds(max_trials);
    let mut scenarios = Vec::with_capacity(PROBABILITIES.len());

    for (scenario_index, probability) in PROBABILITIES.iter().copied().enumerate() {
        let mut alpha_counts = MethodCounts::new();
        let mut e_counts = MethodCounts::new();

        for replication in 0..replications {
            let mut alpha_any_qualified = false;
            let mut alpha_any_rejected = false;
            let mut e_any_qualified = false;
            let mut e_any_rejected = false;

            for candidate in 1..=CANDIDATE_FAMILY_SIZE {
                let mut rng = SplitMix64::new(stream_seed(seed, scenario_index, replication, candidate));
                let outcomes: Vec<bool> = (0..max_trials)
                    .map(|_| rng.bernoulli(probability))
                    .collect();

                let (alpha_decision, alpha_trials) =
                    run_one(&outcomes, candidate, &thresholds.alpha_spending);
                let (e_decision, e_trials) =
                    run_one(&outcomes, candidate, &thresholds.mixture_e_process);

                alpha_counts.add_candidate(alpha_decision, alpha_trials);
                e_counts.add_candidate(e_decision, e_trials);

                alpha_any_qualified |= alpha_decision == Decision::Qualified;
                alpha_any_rejected |= alpha_decision == Decision::NotQualified;
                e_any_qualified |= e_decision == Decision::Qualified;
                e_any_rejected |= e_decision == Decision::NotQualified;
            }

            alpha_counts.add_family(alpha_any_qualified, alpha_any_rejected, probability);
            e_counts.add_family(e_any_qualified, e_any_rejected, probability);
        }

        let alpha_summary = summarize("alpha_spending", alpha_counts, replications);
        let e_summary = summarize("mixture_e_process", e_counts, replications);
        let saved = alpha_summary.mean_trials_per_candidate
            - e_summary.mean_trials_per_candidate;
        let reduction = if alpha_summary.mean_trials_per_candidate > 0.0 {
            saved / alpha_summary.mean_trials_per_candidate
        } else {
            0.0
        };
        scenarios.push(ScenarioSummary {
            true_success_probability: probability,
            alpha_spending: alpha_summary,
            mixture_e_process: e_summary,
            paired_mean_trials_saved_per_candidate: saved,
            paired_trial_reduction_fraction: reduction,
        });
    }

    let config_digest = configuration_sha256(replications, seed, max_trials);
    BenchmarkReport {
        schema_version: 1,
        status: "PASS".to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        evidence_class: "synthetic_monte_carlo".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        configuration_sha256: config_digest,
        prng: "SplitMix64-v1; fixed seed derivation by scenario, replication, and candidate".to_owned(),
        seed,
        replications_per_probability: replications,
        candidate_family_size: CANDIDATE_FAMILY_SIZE,
        max_trials_per_candidate: max_trials,
        total_candidate_attempts_per_probability: (replications * CANDIDATE_FAMILY_SIZE) as u64,
        target_success_probability: P0,
        familywise_alpha: ALPHA,
        probabilities: PROBABILITIES.to_vec(),
        notes: vec![
            "Both methods receive identical pre-generated Bernoulli outcomes for every scenario, family replication, and candidate.".to_owned(),
            "The fixed family size is five; registration indices 1..=5 receive their respective candidate alpha allocations.".to_owned(),
            "Mean trials count all unresolved attempts at the cap; terminal attempts count only trials observed through the first decision.".to_owned(),
            "False-decision family definition: any false qualification when true p < p0; any false rejection when true p > p0; any terminal decision at p == p0.".to_owned(),
            "Wilson intervals quantify only finite Monte Carlo uncertainty in the familywise false-decision rate; they do not prove the method's theoretical guarantees.".to_owned(),
            "IID Bernoulli assumptions are deliberately idealized; dependence, drift, censoring, changing process conditions, and metrology error are not modeled in this version.".to_owned(),
        ],
        scenarios,
    }
}

fn parse_args() -> Result<(usize, u64, usize), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut replications = DEFAULT_REPLICATIONS;
    let mut seed = DEFAULT_SEED;
    let mut max_trials = DEFAULT_MAX_TRIALS;
    let mut i = 0;
    while i < args.len() {
        if i + 1 >= args.len() {
            return Err(format!("missing value for {}", args[i]));
        }
        match args[i].as_str() {
            "--replications" => {
                replications = args[i + 1].parse().map_err(|_| "invalid --replications".to_owned())?;
                if replications == 0 || replications > 100_000 {
                    return Err("--replications must be in 1..=100000".to_owned());
                }
            }
            "--seed" => {
                seed = args[i + 1].parse().map_err(|_| "invalid --seed".to_owned())?;
            }
            "--max-trials" => {
                max_trials = args[i + 1].parse().map_err(|_| "invalid --max-trials".to_owned())?;
                if max_trials < 1 || max_trials > 1_000 {
                    return Err("--max-trials must be in 1..=1000".to_owned());
                }
            }
            other => return Err(format!("unknown argument {other}")),
        }
        i += 2;
    }
    Ok((replications, seed, max_trials))
}

fn main() -> ExitCode {
    let (replications, seed, max_trials) = match parse_args() {
        Ok(config) => config,
        Err(error) => {
            eprintln!(
                "Usage: audit-qualification-monte-carlo [--replications N] [--seed U64] [--max-trials N]\nError: {error}"
            );
            return ExitCode::from(64);
        }
    };
    let report = run_benchmark(replications, seed, max_trials);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("benchmark report serializes")
    );
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_known_answer_is_stable() {
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(rng.next_u64(), 0x6e78_9e6a_a1b9_65f4);
    }

    #[test]
    fn wilson_interval_contains_observed_proportion_and_is_bounded() {
        let interval = wilson_interval(5, 10);
        assert!(interval.lower < 0.5 && interval.upper > 0.5);
        let zero = wilson_interval(0, 100);
        assert_eq!(zero.lower, 0.0);
        assert!(zero.upper > 0.0 && zero.upper < 0.05);
        let all = wilson_interval(100, 100);
        assert!(all.lower > 0.95 && all.upper == 1.0);
    }

    fn all_pass_crossing(index: usize, table: &[Vec<Vec<Decision>>]) -> Option<usize> {
        (1..table[index].len())
            .find(|n| table[index][*n][*n] == Decision::Qualified)
    }

    #[test]
    fn paired_threshold_tables_reproduce_prior_all_pass_crossings() {
        let tables = make_thresholds(400);
        assert_eq!(all_pass_crossing(1, &tables.alpha_spending), Some(310));
        assert_eq!(all_pass_crossing(1, &tables.mixture_e_process), Some(121));
        assert_eq!(all_pass_crossing(2, &tables.alpha_spending), Some(334));
        assert_eq!(all_pass_crossing(2, &tables.mixture_e_process), Some(146));
    }

    #[test]
    fn methods_share_exact_stream_and_simulation_replays_deterministically() {
        let tables = make_thresholds(60);
        let stream = vec![true, false, true, true, false, true, false, false, true, true];
        let a = run_one(&stream, 1, &tables.alpha_spending);
        let e = run_one(&stream, 1, &tables.mixture_e_process);
        assert!(a.1 <= stream.len() && e.1 <= stream.len());

        let first = run_benchmark(8, 20_261_029, 40);
        let second = run_benchmark(8, 20_261_029, 40);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first.scenarios.len(), 4);
        assert!(first.scenarios.iter().all(|s| {
            close(
                s.alpha_spending.candidate_qualification_rate
                    + s.alpha_spending.candidate_rejection_rate
                    + s.alpha_spending.candidate_unresolved_rate,
                1.0,
                1e-12,
            )
        }));
    }

    #[test]
    fn familywise_false_decision_uses_the_correct_direction_on_each_side() {
        let mut counts = MethodCounts::new();
        counts.add_family(true, false, 0.90);
        counts.add_family(false, true, 0.98);
        counts.add_family(true, false, 0.95);
        counts.add_family(false, false, 0.95);
        assert_eq!(counts.false_decision_families, 3);
        assert_eq!(counts.any_qualified_families, 2);
        assert_eq!(counts.any_rejected_families, 1);
    }

    #[test]
    fn rejects_invalid_cli_bounds() {
        assert!(parse_args_with(&["--replications", "0"]).is_err());
    }

    fn parse_args_with(args: &[&str]) -> Result<(usize, u64, usize), String> {
        let mut replications = DEFAULT_REPLICATIONS;
        let mut seed = DEFAULT_SEED;
        let mut max_trials = DEFAULT_MAX_TRIALS;
        let mut i = 0;
        while i < args.len() {
            if i + 1 >= args.len() {
                return Err("missing value".to_owned());
            }
            match args[i] {
                "--replications" => {
                    replications = args[i + 1].parse().map_err(|_| "invalid reps".to_owned())?;
                    if replications == 0 || replications > 100_000 {
                        return Err("bad reps".to_owned());
                    }
                }
                "--seed" => {
                    seed = args[i + 1].parse().map_err(|_| "invalid seed".to_owned())?;
                }
                "--max-trials" => {
                    max_trials = args[i + 1].parse().map_err(|_| "invalid max".to_owned())?;
                    if max_trials < 1 || max_trials > 1_000 {
                        return Err("bad max".to_owned());
                    }
                }
                _ => return Err("unknown".to_owned()),
            }
            i += 2;
        }
        Ok((replications, seed, max_trials))
    }
}
