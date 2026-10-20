use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const CLAIM_CEILING: &str = "Exact one-sided binomial confidence limits for simulated scenario pass rates, conditional on the declared resampling model and independent, representative trials. The calculation does not establish field reliability, source-population representativeness, model validity, or physical grid feasibility.";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    vocation: String,
    fleet_size: usize,
    policy: String,
    capacity_basis_points: u32,
}

#[derive(Debug, Clone)]
struct Observation {
    key: Key,
    seed: u64,
    passed: bool,
    site_budget_kw: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ConditionResult {
    vocation: String,
    fleet_size: usize,
    policy: String,
    capacity_fraction_of_base: f64,
    site_budget_kw: f64,
    trials: usize,
    passes: usize,
    failures: usize,
    empirical_pass_rate: f64,
    confidence_level: f64,
    exact_one_sided_lower_confidence_bound: f64,
    target_pass_probability: f64,
    target_statistically_demonstrated: bool,
    additional_independent_all_pass_trials_needed_to_demonstrate_target: Option<usize>,
}

#[derive(Debug, Serialize)]
struct FrontierResult {
    vocation: String,
    fleet_size: usize,
    policy: String,
    target_pass_probability: f64,
    confidence_level: f64,
    minimum_sampled_capacity_meeting_empirical_target_kw: Option<f64>,
    empirical_pass_rate_at_capacity: Option<f64>,
    minimum_sampled_capacity_statistically_demonstrating_target_kw: Option<f64>,
    exact_lower_confidence_bound_at_demonstrated_capacity: Option<f64>,
    trials_at_demonstrated_capacity: Option<usize>,
    claim: String,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    input_case_count: usize,
    observation_count: usize,
    confidence_level: f64,
    target_pass_probability: f64,
    statistical_method: String,
    claim_ceiling: String,
    conditions: Vec<ConditionResult>,
    frontiers: Vec<FrontierResult>,
}

fn get<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing_required_field:{key}"))
}

fn string_field(value: &Value, key: &str) -> Result<String, String> {
    get(value, key)?
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| format!("invalid_string_field:{key}"))
}

fn usize_field(value: &Value, key: &str) -> Result<usize, String> {
    get(value, key)?
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| format!("invalid_nonnegative_integer_field:{key}"))
}

fn u64_field(value: &Value, key: &str) -> Result<u64, String> {
    get(value, key)?
        .as_u64()
        .ok_or_else(|| format!("invalid_nonnegative_integer_field:{key}"))
}

fn finite_number(value: &Value, key: &str) -> Result<f64, String> {
    let number = get(value, key)?
        .as_f64()
        .ok_or_else(|| format!("invalid_numeric_field:{key}"))?;
    if number.is_finite() {
        Ok(number)
    } else {
        Err(format!("nonfinite_numeric_field:{key}"))
    }
}

fn bool_field(value: &Value, key: &str) -> Result<bool, String> {
    get(value, key)?
        .as_bool()
        .ok_or_else(|| format!("invalid_boolean_field:{key}"))
}

fn parse_observations(root: &Value) -> Result<(usize, Vec<Observation>), String> {
    let cases = get(root, "cases")?
        .as_array()
        .ok_or("cases_must_be_array")?;
    if cases.is_empty() {
        return Err("cases_must_not_be_empty".to_string());
    }

    let mut observations = Vec::new();
    let mut unique = BTreeSet::new();
    for case in cases {
        let vocation = string_field(case, "vocation")?;
        let fleet_size = usize_field(case, "fleet_size")?;
        let seed = u64_field(case, "seed")?;
        if fleet_size == 0 {
            return Err("fleet_size_must_be_positive".to_string());
        }
        let report = get(case, "report")?;
        let frontiers = get(report, "capacity_frontiers")?
            .as_array()
            .ok_or("capacity_frontiers_must_be_array")?;
        if frontiers.is_empty() {
            return Err("case_has_no_capacity_frontiers".to_string());
        }
        for frontier in frontiers {
            let policy = string_field(frontier, "policy")?;
            let points = get(frontier, "points")?
                .as_array()
                .ok_or("frontier_points_must_be_array")?;
            if points.is_empty() {
                return Err("capacity_frontier_has_no_points".to_string());
            }
            for point in points {
                let fraction = finite_number(point, "fraction_of_declared_base_capacity")?;
                if !(0.0..=1.0).contains(&fraction) {
                    return Err("capacity_fraction_out_of_range".to_string());
                }
                let capacity_basis_points_float = fraction * 10_000.0;
                let rounded = capacity_basis_points_float.round();
                if (capacity_basis_points_float - rounded).abs() > 1.0e-6 {
                    return Err(format!(
                        "capacity_fraction_not_representable_to_basis_point:{fraction}"
                    ));
                }
                let capacity_basis_points = rounded as u32;
                let site_budget_kw = finite_number(point, "installed_site_power_budget_kw")?;
                if site_budget_kw < 0.0 {
                    return Err("negative_site_budget".to_string());
                }
                let passed = bool_field(point, "meets_service_target")?;
                let key = Key {
                    vocation: vocation.clone(),
                    fleet_size,
                    policy: policy.clone(),
                    capacity_basis_points,
                };
                if !unique.insert((key.clone(), seed)) {
                    return Err(format!(
                        "duplicate_seed_for_condition:vocation={vocation},fleet_size={fleet_size},policy={policy},capacity_basis_points={capacity_basis_points},seed={seed}"
                    ));
                }
                observations.push(Observation {
                    key,
                    seed,
                    passed,
                    site_budget_kw,
                });
            }
        }
    }
    Ok((cases.len(), observations))
}

// log(C(n,k)) is evaluated by summing log-ratios, avoiding factorial overflow.
fn log_binomial_coefficient(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    (1..=k).map(|i| ((n - k + i) as f64 / i as f64).ln()).sum()
}

// P[X >= k] for X~Binomial(n,p). For this lower-bound inversion we only
// evaluate p in (0, k/n] when k<n, so the recurrence stays numerically useful.
fn binomial_upper_tail(n: usize, k: usize, p: f64) -> f64 {
    if k == 0 {
        return 1.0;
    }
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }

    let log_pmf_k =
        log_binomial_coefficient(n, k) + k as f64 * p.ln() + (n - k) as f64 * (-p).ln_1p();
    let mut term = log_pmf_k.exp();
    let mut total = term;
    for i in k..n {
        term *= ((n - i) as f64 / (i + 1) as f64) * (p / (1.0 - p));
        total += term;
        if total >= 1.0 {
            return 1.0;
        }
    }
    total.clamp(0.0, 1.0)
}

// Exact one-sided Clopper-Pearson lower confidence limit:
// Beta^{-1}(1-confidence; successes, trials-successes+1).
// It solves P_p[X >= successes] = 1-confidence.
fn exact_lower_bound(successes: usize, trials: usize, confidence_level: f64) -> f64 {
    if trials == 0 || successes == 0 {
        return 0.0;
    }
    if successes > trials {
        return f64::NAN;
    }
    let alpha = 1.0 - confidence_level;
    if successes == trials {
        return alpha.powf(1.0 / trials as f64);
    }

    let mut low = 0.0;
    let mut high = successes as f64 / trials as f64;
    for _ in 0..100 {
        let mid = (low + high) / 2.0;
        if binomial_upper_tail(trials, successes, mid) < alpha {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) / 2.0
}

fn additional_all_pass_trials(
    passes: usize,
    trials: usize,
    confidence_level: f64,
    target: f64,
) -> Option<usize> {
    if exact_lower_bound(passes, trials, confidence_level) + 1.0e-12 >= target {
        return Some(0);
    }
    if target >= 1.0 {
        return None;
    }

    // Bracket exponentially, then binary-search the minimum additional all-success trials.
    let mut low = 1_usize;
    let mut high = 1_usize;
    loop {
        let k = passes.checked_add(high)?;
        let n = trials.checked_add(high)?;
        if exact_lower_bound(k, n, confidence_level) + 1.0e-12 >= target {
            break;
        }
        if high >= 100_000 {
            return None;
        }
        low = high.saturating_add(1);
        high = high.saturating_mul(2).min(100_000);
    }

    while low < high {
        let mid = low + (high - low) / 2;
        let k = passes.checked_add(mid)?;
        let n = trials.checked_add(mid)?;
        if exact_lower_bound(k, n, confidence_level) + 1.0e-12 >= target {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    Some(low)
}

fn analyze(root: &Value, confidence_level: f64, target: f64) -> Result<Report, String> {
    if !confidence_level.is_finite() || confidence_level <= 0.0 || confidence_level >= 1.0 {
        return Err("confidence_level_must_be_between_zero_and_one".to_string());
    }
    if !target.is_finite() || target <= 0.0 || target > 1.0 {
        return Err("target_pass_probability_must_be_in_(0,1]".to_string());
    }

    let (input_case_count, observations) = parse_observations(root)?;
    let observation_count = observations.len();
    let mut grouped: BTreeMap<Key, Vec<Observation>> = BTreeMap::new();
    for observation in observations {
        grouped
            .entry(observation.key.clone())
            .or_default()
            .push(observation);
    }

    let mut conditions = Vec::with_capacity(grouped.len());
    let mut policy_frontiers: BTreeMap<(String, usize, String), Vec<ConditionResult>> =
        BTreeMap::new();
    for (key, trials_data) in grouped {
        let trials = trials_data.len();
        let passes = trials_data
            .iter()
            .filter(|observation| observation.passed)
            .count();
        let failures = trials - passes;
        let empirical_pass_rate = passes as f64 / trials as f64;
        let lower = exact_lower_bound(passes, trials, confidence_level);
        let site_budget_kw = trials_data
            .iter()
            .map(|observation| observation.site_budget_kw)
            .sum::<f64>()
            / trials as f64;
        let condition = ConditionResult {
            vocation: key.vocation.clone(),
            fleet_size: key.fleet_size,
            policy: key.policy.clone(),
            capacity_fraction_of_base: f64::from(key.capacity_basis_points) / 10_000.0,
            site_budget_kw,
            trials,
            passes,
            failures,
            empirical_pass_rate,
            confidence_level,
            exact_one_sided_lower_confidence_bound: lower,
            target_pass_probability: target,
            target_statistically_demonstrated: lower + 1.0e-12 >= target,
            additional_independent_all_pass_trials_needed_to_demonstrate_target:
                if empirical_pass_rate + 1.0e-12 >= target {
                    additional_all_pass_trials(passes, trials, confidence_level, target)
                } else {
                    None
                },
        };
        policy_frontiers
            .entry((key.vocation, key.fleet_size, key.policy))
            .or_default()
            .push(condition.clone());
        conditions.push(condition);
    }
    conditions.sort_by(|a, b| {
        a.vocation
            .cmp(&b.vocation)
            .then_with(|| a.fleet_size.cmp(&b.fleet_size))
            .then_with(|| a.policy.cmp(&b.policy))
            .then_with(|| a.capacity_fraction_of_base.total_cmp(&b.capacity_fraction_of_base))
    });

    let mut frontiers = Vec::new();
    for ((vocation, fleet_size, policy), mut rows) in policy_frontiers {
        rows.sort_by(|a, b| {
            a.capacity_fraction_of_base
                .total_cmp(&b.capacity_fraction_of_base)
        });
        let empirical_hit = rows
            .iter()
            .find(|row| row.empirical_pass_rate + 1.0e-12 >= target);
        let demonstrated_hit = rows
            .iter()
            .find(|row| row.target_statistically_demonstrated);
        frontiers.push(FrontierResult {
            vocation,
            fleet_size,
            policy,
            target_pass_probability: target,
            confidence_level,
            minimum_sampled_capacity_meeting_empirical_target_kw: empirical_hit.map(|row| row.site_budget_kw),
            empirical_pass_rate_at_capacity: empirical_hit.map(|row| row.empirical_pass_rate),
            minimum_sampled_capacity_statistically_demonstrating_target_kw: demonstrated_hit.map(|row| row.site_budget_kw),
            exact_lower_confidence_bound_at_demonstrated_capacity: demonstrated_hit.map(|row| row.exact_one_sided_lower_confidence_bound),
            trials_at_demonstrated_capacity: demonstrated_hit.map(|row| row.trials),
            claim: if demonstrated_hit.is_some() {
                "Target lower confidence bound met for this simulated resampling distribution; not field reliability or physical feasibility.".to_string()
            } else {
                "No tested capacity statistically demonstrates the target at the declared confidence level; empirical success alone is insufficient.".to_string()
            },
        });
    }
    frontiers.sort_by(|a, b| {
        (&a.vocation, a.fleet_size, &a.policy).cmp(&(&b.vocation, b.fleet_size, &b.policy))
    });

    Ok(Report {
        schema_version: "0.1.0".to_string(),
        benchmark_id: "EV-DEPOT-CONFIDENCE-001".to_string(),
        input_case_count,
        observation_count,
        confidence_level,
        target_pass_probability: target,
        statistical_method: "one-sided exact Clopper-Pearson binomial lower confidence bound using binomial-tail inversion".to_string(),
        claim_ceiling: CLAIM_CEILING.to_string(),
        conditions,
        frontiers,
    })
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 6 || args[1] != "--confidence" || args[3] != "--target" {
        return Err("usage: ev-depot-confidence-audit <input-robust-cases.json> --confidence 0.95 --target 0.95 <output-report.json>".to_string());
    }
    let input_path = &args[0];
    let confidence = args[2]
        .parse::<f64>()
        .map_err(|_| "confidence must be numeric")?;
    let target = args[4]
        .parse::<f64>()
        .map_err(|_| "target must be numeric")?;
    let bytes = fs::read(input_path).map_err(|e| format!("cannot read {input_path}: {e}"))?;
    let root: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid robust case JSON: {e}"))?;
    let report = analyze(&root, confidence, target)?;
    let out_path = args[5].clone();
    fs::write(
        &out_path,
        serde_json::to_vec_pretty(&report).map_err(|e| format!("cannot serialize report: {e}"))?,
    )
    .map_err(|e| format!("cannot write {out_path}: {e}"))?;
    println!(
        "wrote exact-binomial confidence report: {}; conditions={}; frontiers={}",
        out_path,
        report.conditions.len(),
        report.frontiers.len()
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ev-depot-confidence-audit: {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thirty_of_thirty_does_not_demonstrate_95_percent_at_95_percent_confidence() {
        let lower = exact_lower_bound(30, 30, 0.95);
        assert!((lower - 0.904966).abs() < 0.00002, "lower={lower}");
        assert!(lower < 0.95);
        assert_eq!(additional_all_pass_trials(30, 30, 0.95, 0.95), Some(29));
    }

    #[test]
    fn fifty_nine_of_fifty_nine_clears_the_95_95_gate() {
        let lower = exact_lower_bound(59, 59, 0.95);
        assert!(lower > 0.95, "lower={lower}");
        assert!(additional_all_pass_trials(59, 59, 0.95, 0.95) == Some(0));
    }

    #[test]
    fn failures_reduce_the_lower_confidence_bound() {
        assert!(exact_lower_bound(27, 30, 0.95) < exact_lower_bound(30, 30, 0.95));
        assert!(exact_lower_bound(0, 30, 0.95).abs() < 1.0e-12);
    }

    #[test]
    fn exact_bound_is_monotone_in_successes_for_fixed_n() {
        for successes in 1..30 {
            assert!(
                exact_lower_bound(successes + 1, 30, 0.95)
                    >= exact_lower_bound(successes, 30, 0.95)
            );
        }
    }

    #[test]
    fn reads_capacity_point_trials_and_marks_empirical_vs_statistical_frontier() {
        let cases = (1..=30).map(|seed| serde_json::json!({
            "vocation":"fleetA","fleet_size":10,"seed":seed,
            "report":{"capacity_frontiers":[{
                "policy":"fcfs","points":[
                    {"fraction_of_declared_base_capacity":0.5,"installed_site_power_budget_kw":50.0,"meets_service_target":seed <= 30},
                    {"fraction_of_declared_base_capacity":1.0,"installed_site_power_budget_kw":100.0,"meets_service_target":true}
                ]
            }]}
        })).collect::<Vec<_>>();
        let root = serde_json::json!({"cases":cases});
        let report = analyze(&root, 0.95, 0.95).unwrap();
        assert_eq!(report.input_case_count, 30);
        assert_eq!(report.conditions.len(), 2);
        assert_eq!(report.conditions[0].trials, 30);
        assert_eq!(report.conditions[0].passes, 30);
        assert!(
            (report.conditions[0].exact_one_sided_lower_confidence_bound - 0.904966).abs()
                < 0.00002
        );
        assert_eq!(
            report.frontiers[0].minimum_sampled_capacity_meeting_empirical_target_kw,
            Some(50.0)
        );
        assert_eq!(
            report.frontiers[0].minimum_sampled_capacity_statistically_demonstrating_target_kw,
            None
        );
    }

    #[test]
    fn rejects_duplicate_seed_capacity_observations() {
        let case = serde_json::json!({
            "vocation":"fleetA","fleet_size":10,"seed":1,
            "report":{"capacity_frontiers":[{"policy":"fcfs","points":[
              {"fraction_of_declared_base_capacity":0.5,"installed_site_power_budget_kw":50.0,"meets_service_target":true},
              {"fraction_of_declared_base_capacity":0.5,"installed_site_power_budget_kw":50.0,"meets_service_target":true}
            ]}]}
        });
        let root = serde_json::json!({"cases":[case]});
        assert!(
            parse_observations(&root)
                .unwrap_err()
                .contains("duplicate_seed")
        );
    }
}
