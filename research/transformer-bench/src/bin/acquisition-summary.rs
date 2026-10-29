//! Bootstrap summaries and paired seed-level comparisons for acquisition-bench output.
//! These intervals quantify variation across the supplied randomized replicate seeds on one pool;
//! they are not confidence intervals over independent transformer designs or datasets.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const INPUT_HEADER: [&str; 10] = [
    "policy", "seed", "budget", "oracle_best_reference_loss_w",
    "best_observed_reference_loss_w", "regret_pct", "top_k",
    "top_k_recall_pct", "queried_candidate_ids", "mean_query_relative_error_pct",
];
const POLICIES: [&str; 6] = [
    "RANDOM", "MIN_PREDICTED", "MAX_UNCERTAINTY",
    "LOWER_CONFIDENCE_BOUND", "EXPECTED_IMPROVEMENT", "DIVERSE",
];
const SIMPLE_BASELINES: [&str; 3] = ["RANDOM", "MIN_PREDICTED", "DIVERSE"];

#[derive(Clone, Debug)]
struct Trial {
    policy: String,
    seed: u64,
    budget: usize,
    oracle_best: f64,
    best_observed: f64,
    regret_pct: f64,
    top_k: usize,
    top_k_recall_pct: f64,
    mean_query_relative_error_pct: f64,
}

#[derive(Clone, Debug)]
struct OutputRow {
    row_type: String,
    policy: String,
    baseline: String,
    replicates: usize,
    budget: usize,
    top_k: usize,
    mean_regret: Option<f64>,
    median_regret: Option<f64>,
    mean_top_k_recall: Option<f64>,
    mean_query_error: Option<f64>,
    regret_ci_low: Option<f64>,
    regret_ci_high: Option<f64>,
    mean_delta: Option<f64>,
    delta_ci_low: Option<f64>,
    delta_ci_high: Option<f64>,
    fraction_better: Option<f64>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("acquisition-summary: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 || args.len() > 4 {
        return Err(format!(
            "usage: {} <acquisition-trials.csv> <summary.csv> [bootstrap_replicates=5000]",
            args.first().map(String::as_str).unwrap_or("acquisition-summary")
        ));
    }
    let bootstrap_replicates: usize = if args.len() == 4 {
        args[3].parse().map_err(|_| "bootstrap_replicates must be a positive integer".to_string())?
    } else { 5000 };
    if !(100..=1_000_000).contains(&bootstrap_replicates) {
        return Err("bootstrap_replicates must be in 100..=1000000".to_string());
    }

    let trials = read_trials(&args[1])?;
    let mut rows = Vec::new();
    for (index, policy) in POLICIES.iter().enumerate() {
        let policy_trials: Vec<&Trial> = trials.values().filter(|trial| trial.policy == *policy).collect();
        let regrets: Vec<f64> = policy_trials.iter().map(|trial| trial.regret_pct).collect();
        let recalls: Vec<f64> = policy_trials.iter().map(|trial| trial.top_k_recall_pct).collect();
        let errors: Vec<f64> = policy_trials.iter().map(|trial| trial.mean_query_relative_error_pct).collect();
        let ci = bootstrap_mean_ci(&regrets, bootstrap_replicates, 0xac01_0000_u64.wrapping_add(index as u64))?;
        rows.push(OutputRow {
            row_type: "POLICY_SUMMARY".to_string(),
            policy: policy.to_string(),
            baseline: String::new(),
            replicates: policy_trials.len(),
            budget: policy_trials[0].budget,
            top_k: policy_trials[0].top_k,
            mean_regret: Some(mean(&regrets)),
            median_regret: Some(median(&regrets)),
            mean_top_k_recall: Some(mean(&recalls)),
            mean_query_error: Some(mean(&errors)),
            regret_ci_low: Some(ci.0),
            regret_ci_high: Some(ci.1),
            mean_delta: None,
            delta_ci_low: None,
            delta_ci_high: None,
            fraction_better: None,
        });
    }

    for (policy_index, policy) in POLICIES.iter().enumerate() {
        for (baseline_index, baseline) in SIMPLE_BASELINES.iter().enumerate() {
            if policy == baseline { continue; }
            let mut deltas = Vec::new();
            for trial in trials.values().filter(|trial| trial.policy == *policy) {
                let baseline_trial = trials
                    .get(&(baseline.to_string(), trial.seed))
                    .ok_or_else(|| format!("missing paired seed {} for baseline {baseline}", trial.seed))?;
                // Negative means treatment policy had less regret than the baseline.
                deltas.push(trial.regret_pct - baseline_trial.regret_pct);
            }
            let ci = bootstrap_mean_ci(
                &deltas,
                bootstrap_replicates,
                0xbc01_0000_u64
                    .wrapping_add((policy_index as u64) * 101)
                    .wrapping_add(baseline_index as u64),
            )?;
            let policy_trials: Vec<&Trial> = trials.values().filter(|trial| trial.policy == *policy).collect();
            let fraction_better = deltas.iter().filter(|delta| **delta < 0.0).count() as f64 / deltas.len() as f64 * 100.0;
            rows.push(OutputRow {
                row_type: "PAIRED_SEED_DELTA".to_string(),
                policy: policy.to_string(),
                baseline: baseline.to_string(),
                replicates: deltas.len(),
                budget: policy_trials[0].budget,
                top_k: policy_trials[0].top_k,
                mean_regret: None,
                median_regret: None,
                mean_top_k_recall: None,
                mean_query_error: None,
                regret_ci_low: None,
                regret_ci_high: None,
                mean_delta: Some(mean(&deltas)),
                delta_ci_low: Some(ci.0),
                delta_ci_high: Some(ci.1),
                fraction_better: Some(fraction_better),
            });
        }
    }
    write_output(&args[2], &rows)?;
    eprintln!(
        "acquisition summary written: policy_summaries={}, paired_comparisons={}, seed_replicates_per_policy={}; bootstrap intervals quantify seed variation on this supplied pool only, not cross-dataset uncertainty",
        POLICIES.len(),
        rows.len() - POLICIES.len(),
        trials.values().filter(|trial| trial.policy == "RANDOM").count()
    );
    Ok(())
}

fn parse_number<T: std::str::FromStr>(value: &str, context: &str) -> Result<T, String> {
    value.trim().parse().map_err(|_| format!("{context}: invalid value {value:?}"))
}

fn finite_number(value: &str, context: &str) -> Result<f64, String> {
    let parsed: f64 = parse_number(value, context)?;
    if !parsed.is_finite() { return Err(format!("{context}: non-finite value")); }
    Ok(parsed)
}

fn read_trials(path: &str) -> Result<BTreeMap<(String, u64), Trial>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing header"))?.trim_end_matches('\r');
    let fields: Vec<&str> = header.split(',').collect();
    if fields.as_slice() != INPUT_HEADER.as_slice() {
        return Err(format!("{path}: expected acquisition-bench report header [{}]", INPUT_HEADER.join(",")));
    }
    let mut trials = BTreeMap::new();
    for (index, raw) in lines.enumerate() {
        let line_no=index+2;
        let line=raw.trim_end_matches('\r');
        if line.trim().is_empty() { return Err(format!("{path}:{line_no}: blank row not allowed")); }
        if line.contains('"') { return Err(format!("{path}:{line_no}: quoted fields are unsupported")); }
        let f:Vec<&str>=line.split(',').collect();
        if f.len()!=INPUT_HEADER.len() { return Err(format!("{path}:{line_no}: expected {} fields, got {}",INPUT_HEADER.len(),f.len())); }
        let policy=f[0].trim().to_string();
        if !POLICIES.contains(&policy.as_str()) { return Err(format!("{path}:{line_no}: unknown policy {policy:?}")); }
        let seed:u64=parse_number(f[1],"seed")?;
        let budget:usize=parse_number(f[2],"budget")?;
        let oracle_best=finite_number(f[3],"oracle_best_reference_loss_w")?;
        let best_observed=finite_number(f[4],"best_observed_reference_loss_w")?;
        let regret_pct=finite_number(f[5],"regret_pct")?;
        let top_k:usize=parse_number(f[6],"top_k")?;
        let top_k_recall_pct=finite_number(f[7],"top_k_recall_pct")?;
        let query_ids=f[8].trim();
        if query_ids.is_empty() { return Err(format!("{path}:{line_no}: empty queried_candidate_ids")); }
        let queried:Vec<&str>=query_ids.split(';').collect();
        if queried.len()!=budget || queried.iter().collect::<BTreeSet<_>>().len()!=budget {
            return Err(format!("{path}:{line_no}: queried IDs must be unique and match budget"));
        }
        let mean_query_relative_error_pct=finite_number(f[9],"mean_query_relative_error_pct")?;
        if budget==0 || top_k==0 || oracle_best<=0.0 || best_observed<oracle_best || regret_pct<0.0
            || !(0.0..=100.0).contains(&top_k_recall_pct) || mean_query_relative_error_pct<0.0 {
            return Err(format!("{path}:{line_no}: metric values violate their declared ranges"));
        }
        let trial=Trial {
            policy:policy.clone(), seed, budget, oracle_best,best_observed,regret_pct,top_k,
            top_k_recall_pct,mean_query_relative_error_pct
        };
        if trials.insert((policy.clone(),seed),trial).is_some() {
            return Err(format!("{path}:{line_no}: duplicate policy/seed pair ({policy},{seed})"));
        }
    }
    if trials.is_empty() { return Err(format!("{path}: no trial rows")); }
    validate_paired_design(&trials)?;
    Ok(trials)
}

fn validate_paired_design(trials: &BTreeMap<(String,u64),Trial>) -> Result<(),String> {
    let seeds: BTreeSet<u64> = trials.values().filter(|trial| trial.policy=="RANDOM").map(|trial|trial.seed).collect();
    if seeds.len()<2 { return Err("at least two paired seeds are required for bootstrap summaries".to_string()); }
    for seed in &seeds {
        for policy in POLICIES {
            if !trials.contains_key(&(policy.to_string(),*seed)) {
                return Err(format!("seed {seed} is missing policy {policy}"));
            }
        }
    }
    if trials.len()!=seeds.len()*POLICIES.len() {
        return Err("trial report must contain exactly one row for every policy at every paired seed".to_string());
    }
    for seed in seeds {
        let random=&trials[&("RANDOM".to_string(),seed)];
        for policy in POLICIES {
            let row=&trials[&(policy.to_string(),seed)];
            if row.budget!=random.budget || row.top_k!=random.top_k || (row.oracle_best-random.oracle_best).abs()>1.0e-9 {
                return Err(format!("policy {policy}, seed {seed}: budget, top_k, and oracle best must match across policies"));
            }
        }
    }
    Ok(())
}

fn mean(values: &[f64]) -> f64 { values.iter().sum::<f64>() / values.len() as f64 }

fn median(values: &[f64]) -> f64 {
    let mut sorted=values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.len()%2==1 { sorted[sorted.len()/2] }
    else { (sorted[sorted.len()/2-1]+sorted[sorted.len()/2])/2.0 }
}

fn bootstrap_mean_ci(values: &[f64], replicates: usize, seed: u64) -> Result<(f64,f64),String> {
    if values.len()<2 { return Err("bootstrap requires at least two paired values".to_string()); }
    if values.iter().any(|value| !value.is_finite()) { return Err("bootstrap values must be finite".to_string()); }
    let mut rng=SplitMix64::new(seed);
    let mut means=Vec::with_capacity(replicates);
    for _ in 0..replicates {
        let mut total=0.0;
        for _ in 0..values.len() {
            total += values[rng.index(values.len())];
        }
        means.push(total/values.len() as f64);
    }
    means.sort_by(f64::total_cmp);
    let low=((replicates as f64*0.025).floor() as usize).min(replicates-1);
    let high=((replicates as f64*0.975).ceil() as usize).min(replicates-1);
    Ok((means[low],means[high]))
}

struct SplitMix64 { state:u64 }
impl SplitMix64 {
    fn new(seed:u64)->Self { Self{state:seed} }
    fn next_u64(&mut self)->u64 {
        self.state=self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z=self.state;
        z=(z^(z>>30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z=(z^(z>>27)).wrapping_mul(0x94d049bb133111eb);
        z^(z>>31)
    }
    fn index(&mut self,len:usize)->usize { (self.next_u64()%len as u64) as usize }
}

fn optional(value: Option<f64>) -> String {
    value.map(|x|format!("{x:.8}")).unwrap_or_default()
}

fn write_output(path:&str,rows:&[OutputRow])->Result<(),String> {
    let mut out=String::from("row_type,policy,baseline,replicates,budget,top_k,mean_regret_pct,median_regret_pct,mean_top_k_recall_pct,mean_query_relative_error_pct,regret_seed_bootstrap_ci95_low_pct,regret_seed_bootstrap_ci95_high_pct,mean_paired_regret_delta_pct,paired_seed_bootstrap_ci95_low_pct,paired_seed_bootstrap_ci95_high_pct,fraction_paired_trials_better_pct\n");
    for r in rows {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.row_type,r.policy,r.baseline,r.replicates,r.budget,r.top_k,
            optional(r.mean_regret),optional(r.median_regret),optional(r.mean_top_k_recall),
            optional(r.mean_query_error),optional(r.regret_ci_low),optional(r.regret_ci_high),
            optional(r.mean_delta),optional(r.delta_ci_low),optional(r.delta_ci_high),
            optional(r.fraction_better)
        ));
    }
    fs::write(path,out).map_err(|e|format!("cannot write {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trial(policy:&str,seed:u64,regret:f64)->Trial {
        Trial {
            policy:policy.to_string(),seed,budget:3,oracle_best:10.0,best_observed:10.0*(1.0+regret/100.0),
            regret_pct:regret,top_k:2,top_k_recall_pct:50.0,mean_query_relative_error_pct:1.0
        }
    }

    #[test]
    fn bootstrap_is_deterministic_and_brackets_identical_values() {
        let first=bootstrap_mean_ci(&[2.0,2.0,2.0],1000,99).unwrap();
        let second=bootstrap_mean_ci(&[2.0,2.0,2.0],1000,99).unwrap();
        assert_eq!(first,second);
        assert!((first.0-2.0).abs()<1.0e-12);
        assert!((first.1-2.0).abs()<1.0e-12);
    }

    #[test]
    fn reads_complete_paired_policy_trials_and_detects_a_better_policy() {
        let mut rows=BTreeMap::new();
        for seed in 1..=5 {
            for policy in POLICIES {
                let regret=match policy {
                    "RANDOM"=>10.0+seed as f64,
                    "MIN_PREDICTED"=>8.0,
                    "DIVERSE"=>12.0,
                    "LOWER_CONFIDENCE_BOUND"=>5.0,
                    "MAX_UNCERTAINTY"=>15.0,
                    "EXPECTED_IMPROVEMENT"=>6.0,
                    _=>unreachable!(),
                };
                rows.insert((policy.to_string(),seed),trial(policy,seed,regret));
            }
        }
        validate_paired_design(&rows).unwrap();
        let deltas:Vec<f64>=(1..=5).map(|seed|rows[&("LOWER_CONFIDENCE_BOUND".to_string(),seed)].regret_pct-rows[&("RANDOM".to_string(),seed)].regret_pct).collect();
        assert!(mean(&deltas)<0.0);
        let (lo,hi)=bootstrap_mean_ci(&deltas,2000,123).unwrap();
        assert!(lo<0.0 && hi<0.0);
    }

    #[test]
    fn parses_complete_acquisition_bench_csv() {
        let path = env::temp_dir().join("transformer-acquisition-summary-test.csv");
        let path = path.to_string_lossy().into_owned();
        let mut csv = format!("{}\\n", INPUT_HEADER.join(","));
        for seed in 1..=2 {
            for policy in POLICIES {
                csv.push_str(&format!(
                    "{policy},{seed},3,10,10.5,5,2,50,c-{policy}-{seed}-a;c-{policy}-{seed}-b;c-{policy}-{seed}-c,1.5\\n"
                ));
            }
        }
        fs::write(&path, csv).unwrap();
        let trials = read_trials(&path).unwrap();
        assert_eq!(trials.len(), POLICIES.len() * 2);
        assert!(trials.contains_key(&("EXPECTED_IMPROVEMENT".to_string(), 2)));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_missing_policy_for_one_seed() {
        let mut rows=BTreeMap::new();
        for seed in 1..=3 {
            for policy in POLICIES {
                if seed==2 && policy=="DIVERSE" { continue; }
                rows.insert((policy.to_string(),seed),trial(policy,seed,1.0));
            }
        }
        assert!(validate_paired_design(&rows).unwrap_err().contains("missing policy DIVERSE"));
    }

    #[test]
    fn rejects_duplicate_or_mismatched_policy_design() {
        let mut rows=BTreeMap::new();
        for seed in 1..=3 {
            for policy in POLICIES { rows.insert((policy.to_string(),seed),trial(policy,seed,1.0)); }
        }
        rows.get_mut(&("DIVERSE".to_string(),2)).unwrap().budget=4;
        assert!(validate_paired_design(&rows).unwrap_err().contains("budget, top_k"));
    }
}
