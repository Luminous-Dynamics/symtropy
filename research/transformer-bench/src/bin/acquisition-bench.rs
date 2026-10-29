//! Budgeted transformer-candidate verification policy benchmark.
//!
//! This replays candidate-selection policies over a frozen reference pool. The reference value is
//! used only to score selections and update the observed incumbent after a query; selection scores
//! use predictions/intervals/features only. It does not call FEA or retrain a surrogate.
//! Synthetic fixture output is plumbing validation, never a physical or SOTA result.

use std::collections::{BTreeSet, HashSet};
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 12] = [
    "candidate_id",
    "geometry_family",
    "material_id",
    "frequency_hz",
    "excitation_class",
    "feature_1",
    "feature_2",
    "feature_3",
    "reference_core_loss_w",
    "predicted_core_loss_w",
    "prediction_lower_w",
    "prediction_upper_w",
];
const POLICY_NAMES: [&str; 6] = [
    "RANDOM",
    "MIN_PREDICTED",
    "MAX_UNCERTAINTY",
    "LOWER_CONFIDENCE_BOUND",
    "EXPECTED_IMPROVEMENT",
    "DIVERSE",
];
const LCB_BETA: f64 = 1.96;

#[derive(Clone, Debug)]
struct Candidate {
    id: String,
    geometry_family: String,
    material_id: String,
    frequency_hz: f64,
    excitation_class: String,
    features: [f64; 3],
    reference: f64,
    predicted: f64,
    lower: f64,
    upper: f64,
}

impl Candidate {
    fn sigma(&self) -> f64 {
        (self.upper - self.lower) / 3.92
    }

    fn interval_width(&self) -> f64 {
        self.upper - self.lower
    }
}

#[derive(Clone, Debug)]
struct Trial {
    policy: String,
    seed: u64,
    budget: usize,
    oracle_best: f64,
    observed_best: f64,
    regret_pct: f64,
    top_k: usize,
    top_k_recall_pct: f64,
    queried_ids: String,
    mean_query_relative_error_pct: f64,
}

#[derive(Clone, Debug)]
struct SplitPool<'a> {
    candidates: &'a [Candidate],
    min_features: [f64; 3],
    max_features: [f64; 3],
}

fn main() {
    if let Err(error) = run() {
        eprintln!("acquisition-bench: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 || args.len() > 6 {
        return Err(format!(
            "usage: {} <candidate-pool.csv> <report.csv> <budget> <replicates> [top_k=8]",
            args.first().map(String::as_str).unwrap_or("acquisition-bench")
        ));
    }
    let candidates = read_candidates(&args[1])?;
    let budget: usize = args[3]
        .parse()
        .map_err(|_| "budget must be a positive integer".to_string())?;
    if budget == 0 || budget > candidates.len() {
        return Err(format!("budget must be in 1..={}", candidates.len()));
    }
    let replicates: usize = args[4]
        .parse()
        .map_err(|_| "replicates must be a positive integer".to_string())?;
    if replicates == 0 || replicates > 10_000 {
        return Err("replicates must be in 1..=10000".to_string());
    }
    let top_k: usize = if args.len() == 6 {
        args[5].parse().map_err(|_| "top_k must be a positive integer".to_string())?
    } else {
        8
    };
    if top_k == 0 {
        return Err("top_k must be positive".to_string());
    }

    let pool = SplitPool::new(&candidates);
    let mut trials = Vec::new();
    for replicate in 0..replicates {
        // Every policy in a replicate shares a seed for paired reproducibility.
        let seed = 0x5458_0000_u64.wrapping_add(replicate as u64);
        for policy in POLICY_NAMES {
            trials.push(evaluate_policy(&pool, policy, budget, top_k, seed)?);
        }
    }
    write_report(&args[2], &trials)?;
    eprintln!(
        "budgeted verification replay complete: candidates={}, policies={}, replicates={}, per-policy budget={}, total simulated queries={}, reference domain={} Hz/{}; offline replay only, no FEA calls were made",
        candidates.len(),
        POLICY_NAMES.len(),
        replicates,
        budget,
        trials.len() * budget,
        candidates[0].frequency_hz,
        candidates[0].excitation_class
    );
    Ok(())
}

impl<'a> SplitPool<'a> {
    fn new(candidates: &'a [Candidate]) -> Self {
        let mut min_features = candidates[0].features;
        let mut max_features = candidates[0].features;
        for candidate in candidates.iter().skip(1) {
            for i in 0..3 {
                min_features[i] = min_features[i].min(candidate.features[i]);
                max_features[i] = max_features[i].max(candidate.features[i]);
            }
        }
        Self { candidates, min_features, max_features }
    }

    fn normalized_features(&self, candidate: &Candidate) -> [f64; 3] {
        let mut result = [0.0; 3];
        for i in 0..3 {
            let range = self.max_features[i] - self.min_features[i];
            result[i] = if range > 0.0 {
                (candidate.features[i] - self.min_features[i]) / range
            } else {
                0.0
            };
        }
        result
    }

    fn distance_squared(&self, left: &Candidate, right: &Candidate) -> f64 {
        let a = self.normalized_features(left);
        let b = self.normalized_features(right);
        let continuous: f64 = (0..3).map(|i| (a[i] - b[i]).powi(2)).sum();
        let geometry_distance = if left.geometry_family == right.geometry_family { 0.0 } else { 1.0 };
        let material_distance = if left.material_id == right.material_id { 0.0 } else { 1.0 };
        continuous + geometry_distance + material_distance
    }
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{context}: empty value is not allowed"))
    } else {
        Ok(value.to_string())
    }
}

fn finite_number(value: &str, context: &str) -> Result<f64, String> {
    let number: f64 = value.trim().parse()
        .map_err(|_| format!("{context}: expected a finite number, got {value:?}"))?;
    if !number.is_finite() {
        return Err(format!("{context}: non-finite values are not allowed"));
    }
    Ok(number)
}

fn read_candidates(path: &str) -> Result<Vec<Candidate>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing CSV header"))?.trim_end_matches('\r');
    let fields: Vec<&str> = header.split(',').collect();
    if fields.as_slice() != HEADER.as_slice() {
        return Err(format!("{path}: expected header [{}]", HEADER.join(",")));
    }
    let mut candidates = Vec::new();
    let mut ids = BTreeSet::new();
    let mut common_domain: Option<(f64, String)> = None;
    for (index, raw_line) in lines.enumerate() {
        let line_no = index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() { return Err(format!("{path}:{line_no}: blank rows not allowed")); }
        if line.contains('"') { return Err(format!("{path}:{line_no}: quoted CSV fields unsupported")); }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() != HEADER.len() { return Err(format!("{path}:{line_no}: expected {} fields, got {}", HEADER.len(), f.len())); }
        let id = nonempty(f[0], "candidate_id")?;
        if !ids.insert(id.clone()) { return Err(format!("{path}:{line_no}: duplicate candidate_id {id:?}")); }
        let geometry_family = nonempty(f[1], "geometry_family")?;
        let material_id = nonempty(f[2], "material_id")?;
        let frequency_hz = finite_number(f[3], "frequency_hz")?;
        let excitation_class = nonempty(f[4], "excitation_class")?;
        let features = [
            finite_number(f[5], "feature_1")?,
            finite_number(f[6], "feature_2")?,
            finite_number(f[7], "feature_3")?,
        ];
        let reference = finite_number(f[8], "reference_core_loss_w")?;
        let predicted = finite_number(f[9], "predicted_core_loss_w")?;
        let lower = finite_number(f[10], "prediction_lower_w")?;
        let upper = finite_number(f[11], "prediction_upper_w")?;
        if frequency_hz <= 0.0 { return Err(format!("{path}:{line_no}: frequency_hz must be positive")); }
        if !matches!(excitation_class.as_str(), "SINUSOIDAL" | "DISTORTED" | "OTHER_DECLARED") {
            return Err(format!("{path}:{line_no}: unsupported excitation_class"));
        }
        if reference <= 0.0 || predicted <= 0.0 || lower <= 0.0 || upper <= 0.0 {
            return Err(format!("{path}:{line_no}: losses and interval bounds must be positive"));
        }
        if lower > predicted || predicted > upper { return Err(format!("{path}:{line_no}: prediction must lie within interval")); }
        match &common_domain {
            None => common_domain = Some((frequency_hz, excitation_class.clone())),
            Some((frequency, waveform)) if *frequency != frequency_hz || waveform != &excitation_class => {
                return Err(format!("{path}:{line_no}: candidate pool mixes operating domains; evaluate each frequency/excitation domain separately"));
            }
            _ => {}
        }
        candidates.push(Candidate {
            id, geometry_family, material_id, frequency_hz, excitation_class, features,
            reference, predicted, lower, upper,
        });
    }
    if candidates.len() < 2 { return Err(format!("{path}: at least two candidates are required")); }
    Ok(candidates)
}

#[derive(Clone, Debug)]
struct SplitMix64 { state: u64 }
impl SplitMix64 {
    fn new(seed: u64) -> Self { Self { state: seed } }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn index(&mut self, len: usize) -> usize { (self.next_u64() % len as u64) as usize }
    fn unit(&mut self) -> f64 { (self.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64) }
}

fn normal_pdf(z: f64) -> f64 {
    const INV_SQRT_2PI: f64 = 0.3989422804014327;
    INV_SQRT_2PI * (-0.5 * z * z).exp()
}

// Abramowitz-Stegun 7.1.26 approximation; adequate for a policy-ranking benchmark, not inference.
fn normal_cdf(z: f64) -> f64 {
    let x = z.abs();
    let t = 1.0 / (1.0 + 0.2316419 * x);
    let polynomial = t * (0.319381530 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    let tail = normal_pdf(x) * polynomial;
    if z >= 0.0 { 1.0 - tail } else { tail }
}

fn expected_improvement(best_observed: f64, predicted: f64, sigma: f64) -> f64 {
    if sigma <= f64::EPSILON { return (best_observed - predicted).max(0.0); }
    let z = (best_observed - predicted) / sigma;
    (best_observed - predicted) * normal_cdf(z) + sigma * normal_pdf(z)
}

fn choose_next(
    pool: &SplitPool<'_>,
    policy: &str,
    selected: &[usize],
    best_observed: Option<f64>,
    rng: &mut SplitMix64,
) -> Result<usize, String> {
    let candidates = pool.candidates;
    let selected_set: HashSet<usize> = selected.iter().copied().collect();
    let available: Vec<usize> = (0..candidates.len()).filter(|i| !selected_set.contains(i)).collect();
    if available.is_empty() { return Err("no unqueried candidates remain".to_string()); }

    if policy == "RANDOM" { return Ok(available[rng.index(available.len())]); }
    if policy == "DIVERSE" {
        if selected.is_empty() { return Ok(available[rng.index(available.len())]); }
        let mut best = available[0];
        let mut best_distance = f64::NEG_INFINITY;
        for index in available {
            let min_distance = selected.iter().map(|&seen| pool.distance_squared(&candidates[index], &candidates[seen])).fold(f64::INFINITY, f64::min);
            if min_distance > best_distance || (min_distance == best_distance && candidates[index].id < candidates[best].id) {
                best = index;
                best_distance = min_distance;
            }
        }
        return Ok(best);
    }

    let mut best = available[0];
    let mut best_score = f64::NEG_INFINITY;
    for index in available {
        let candidate = &candidates[index];
        let sigma = candidate.sigma();
        let score = match policy {
            "MIN_PREDICTED" => -candidate.predicted,
            "MAX_UNCERTAINTY" => candidate.interval_width(),
            "LOWER_CONFIDENCE_BOUND" => -(candidate.predicted - LCB_BETA * sigma),
            "EXPECTED_IMPROVEMENT" => {
                match best_observed {
                    Some(best_value) => expected_improvement(best_value, candidate.predicted, sigma),
                    None => -candidate.predicted,
                }
            }
            _ => return Err(format!("unknown policy {policy:?}")),
        };
        if score > best_score || (score == best_score && candidate.id < candidates[best].id) {
            best = index;
            best_score = score;
        }
    }
    Ok(best)
}

fn evaluate_policy(
    pool: &SplitPool<'_>,
    policy: &str,
    budget: usize,
    requested_top_k: usize,
    seed: u64,
) -> Result<Trial, String> {
    if !POLICY_NAMES.contains(&policy) { return Err(format!("unknown policy {policy:?}")); }
    let candidates = pool.candidates;
    if budget == 0 || budget > candidates.len() { return Err("invalid budget".to_string()); }
    let mut rng = SplitMix64::new(seed);
    let mut selected = Vec::with_capacity(budget);
    let mut selected_set = HashSet::new();
    let mut best_observed = f64::INFINITY;
    let mut relative_errors = Vec::with_capacity(budget);
    for _ in 0..budget {
        let next = choose_next(pool, policy, &selected, if best_observed.is_finite() { Some(best_observed) } else { None }, &mut rng)?;
        if !selected_set.insert(next) { return Err("policy selected duplicate candidate".to_string()); }
        selected.push(next);
        // This is the only place the oracle/reference is consulted: after the choice is made.
        let candidate = &candidates[next];
        best_observed = best_observed.min(candidate.reference);
        relative_errors.push((candidate.predicted - candidate.reference).abs() / candidate.reference * 100.0);
    }

    let oracle_best = candidates.iter().map(|candidate| candidate.reference).fold(f64::INFINITY, f64::min);
    let regret_pct = (best_observed - oracle_best).max(0.0) / oracle_best * 100.0;
    let top_k = requested_top_k.min(candidates.len());
    let mut oracle_rank: Vec<usize> = (0..candidates.len()).collect();
    oracle_rank.sort_by(|&a, &b| candidates[a].reference.total_cmp(&candidates[b].reference).then_with(|| candidates[a].id.cmp(&candidates[b].id)));
    let oracle_top: HashSet<usize> = oracle_rank.into_iter().take(top_k).collect();
    let found = selected.iter().filter(|index| oracle_top.contains(index)).count();
    Ok(Trial {
        policy: policy.to_string(),
        seed,
        budget,
        oracle_best,
        observed_best: best_observed,
        regret_pct,
        top_k,
        top_k_recall_pct: found as f64 / top_k as f64 * 100.0,
        queried_ids: selected.iter().map(|&i| candidates[i].id.as_str()).collect::<Vec<_>>().join(";"),
        mean_query_relative_error_pct: relative_errors.iter().sum::<f64>() / relative_errors.len() as f64,
    })
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else { value.to_string() }
}

fn write_report(path: &str, trials: &[Trial]) -> Result<(), String> {
    let mut output = String::from("policy,seed,budget,oracle_best_reference_loss_w,best_observed_reference_loss_w,regret_pct,top_k,top_k_recall_pct,queried_candidate_ids,mean_query_relative_error_pct\n");
    for row in trials {
        output.push_str(&format!(
            "{},{},{},{:.9},{:.9},{:.9},{},{:.6},{},{:.6}\n",
            row.policy, row.seed, row.budget, row.oracle_best, row.observed_best,
            row.regret_pct, row.top_k, row.top_k_recall_pct, csv_escape(&row.queried_ids),
            row.mean_query_relative_error_pct
        ));
    }
    fs::write(path, output).map_err(|e| format!("cannot write report {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, features: [f64;3], reference: f64, predicted: f64, width: f64) -> Candidate {
        Candidate {
            id:id.to_string(), geometry_family:"G1".to_string(), material_id:"M5".to_string(),
            frequency_hz:50.0, excitation_class:"SINUSOIDAL".to_string(), features,
            reference, predicted, lower:predicted-width, upper:predicted+width,
        }
    }

    fn test_pool() -> Vec<Candidate> {
        vec![
            candidate("a",[0.0,0.0,0.0],10.0,11.0,1.0),
            candidate("b",[1.0,0.0,0.0],20.0,19.0,1.0),
            candidate("c",[0.0,1.0,0.0],12.0,12.5,4.0),
            candidate("d",[0.0,0.0,1.0],18.0,16.0,2.0),
            candidate("e",[1.0,1.0,1.0],30.0,29.0,3.0),
        ]
    }

    #[test]
    fn splitmix_is_deterministic() {
        let mut a=SplitMix64::new(7); let mut b=SplitMix64::new(7);
        for _ in 0..50 { assert_eq!(a.next_u64(),b.next_u64()); }
    }

    #[test]
    fn expected_improvement_is_nonnegative_and_prefers_plausible_improvements() {
        assert!(expected_improvement(10.0, 5.0, 1.0) > expected_improvement(10.0, 20.0, 1.0));
        assert!(expected_improvement(10.0, 15.0, 0.0).abs() < 1.0e-12);
        assert!((normal_cdf(0.0)-0.5).abs() < 1.0e-6);
    }

    #[test]
    fn every_policy_respects_budget_and_never_queries_twice() {
        let rows=test_pool();
        let pool=SplitPool::new(&rows);
        for policy in POLICY_NAMES {
            let trial=evaluate_policy(&pool,policy,3,2,123).unwrap();
            let ids:Vec<&str>=trial.queried_ids.split(';').collect();
            assert_eq!(ids.len(),3);
            assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(),3);
            assert!(trial.regret_pct >= 0.0);
            assert!(trial.top_k_recall_pct >= 0.0 && trial.top_k_recall_pct <= 100.0);
        }
    }

    #[test]
    fn minimum_predicted_selects_the_predicted_minimum_first() {
        let rows=test_pool();
        let pool=SplitPool::new(&rows);
        let trial=evaluate_policy(&pool,"MIN_PREDICTED",1,2,1).unwrap();
        assert_eq!(trial.queried_ids,"a");
    }

    #[test]
    fn uncertainty_selects_widest_interval_first() {
        let rows=test_pool();
        let pool=SplitPool::new(&rows);
        let trial=evaluate_policy(&pool,"MAX_UNCERTAINTY",1,2,1).unwrap();
        assert_eq!(trial.queried_ids,"c");
    }

    #[test]
    fn random_policy_is_seed_reproducible() {
        let rows=test_pool(); let pool=SplitPool::new(&rows);
        let first=evaluate_policy(&pool,"RANDOM",4,2,456).unwrap();
        let second=evaluate_policy(&pool,"RANDOM",4,2,456).unwrap();
        assert_eq!(first.queried_ids,second.queried_ids);
    }

    #[test]
    fn parser_rejects_mixed_operating_domains() {
        let path=env::temp_dir().join("transformer-acquisition-bench-test.csv");
        let path=path.to_string_lossy().into_owned();
        let header=HEADER.join(",");
        fs::write(&path,format!("{header}\na,G1,M1,50,SINUSOIDAL,0,0,0,10,10,9,11\nb,G1,M1,60,SINUSOIDAL,0,0,0,11,11,10,12\n")).unwrap();
        assert!(read_candidates(&path).unwrap_err().contains("mixes operating domains"));
        let _=fs::remove_file(path);
    }

    #[test]
    fn checked_in_synthetic_pool_runs_all_policies_under_equal_budget() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/acquisition-pool-synthetic.csv");
        let rows = read_candidates(path.to_str().unwrap()).unwrap();
        assert_eq!(rows.len(), 24);
        let pool = SplitPool::new(&rows);
        let trials: Vec<Trial> = POLICY_NAMES
            .iter()
            .map(|policy| evaluate_policy(&pool, policy, 6, 8, 42).unwrap())
            .collect();
        assert_eq!(trials.len(), POLICY_NAMES.len());
        assert!(trials.iter().all(|trial| trial.budget == 6));
        assert!(trials.iter().all(|trial| trial.oracle_best > 0.0));
        assert!(trials.iter().all(|trial| trial.observed_best >= trial.oracle_best));
    }

    #[test]
    fn rejects_duplicate_ids_and_predictions_outside_intervals() {
        let path=env::temp_dir().join("transformer-acquisition-bench-invalid.csv");
        let path=path.to_string_lossy().into_owned();
        let header=HEADER.join(",");
        fs::write(&path,format!("{header}\na,G1,M1,50,SINUSOIDAL,0,0,0,10,10,9,11\na,G1,M1,50,SINUSOIDAL,1,1,1,12,12,10,11\n")).unwrap();
        assert!(read_candidates(&path).unwrap_err().contains("duplicate candidate_id"));
        fs::write(&path,format!("{header}\na,G1,M1,50,SINUSOIDAL,0,0,0,10,12,9,11\nb,G2,M1,50,SINUSOIDAL,1,1,1,12,12,10,14\n")).unwrap();
        assert!(read_candidates(&path).unwrap_err().contains("prediction must lie within interval"));
        let _=fs::remove_file(path);
    }
}
