//! Failure-aware open-loop rollout comparison for learned quadrotor dynamics.
//!
//! Evaluates every preregistered flight/start/seed/horizon in one paired table.
//! Diverged and missing predictions remain in the primary metric rather than being
//! silently dropped. Training seeds and physical recordings are resampled as crossed
//! factors. This is offline research tooling, not flight software.

use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const EPS: f64 = 1.0e-9;
const HORIZON_TOLERANCE_S: f64 = 1.0e-6;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RolloutRow {
    flight_id: String,
    rollout_start_id: String,
    regime: String,
    training_seed: u64,
    horizon_s: f64,
    baseline_status: String,
    baseline_position_error_m: Option<f64>,
    candidate_status: String,
    candidate_position_error_m: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StartRow {
    flight_id: String,
    rollout_start_id: String,
    regime: String,
}

#[derive(Debug)]
struct Args {
    input: PathBuf,
    starts_manifest: PathBuf,
    train_manifest: PathBuf,
    dataset_receipt_sha256: String,
    paired_target_receipt_sha256: String,
    baseline_artifact_sha256: String,
    candidate_artifact_sha256: String,
    baseline_method: String,
    candidate_method: String,
    baseline_source_commit: String,
    candidate_source_commit: String,
    training_seeds: Vec<u64>,
    horizons_s: Vec<f64>,
    channel: String,
    divergence_limit_m: f64,
    bootstrap_replicates: usize,
    bootstrap_seed: u64,
    evidence_class: String,
    output: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
struct ModelSummary {
    registered_rollout_count: usize,
    penalized_rollout_count: usize,
    penalized_rollout_rate: f64,
    failure_aware_position_rmse_m: f64,
    conditional_finite_position_rmse_m: Option<f64>,
    conditional_finite_rollout_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct Interval {
    lower_95: Option<f64>,
    upper_95: Option<f64>,
    valid_replicates: usize,
}

#[derive(Clone, Debug, Serialize)]
struct HorizonResult {
    horizon_s: f64,
    baseline: ModelSummary,
    candidate: ModelSummary,
    delta_failure_aware_rmse_candidate_minus_baseline_m: f64,
    paired_crossed_bootstrap_delta_rmse_m: Interval,
    delta_penalized_rollout_rate_candidate_minus_baseline: f64,
    paired_crossed_bootstrap_delta_penalized_rollout_rate: Interval,
    rmse_interval_excludes_zero: bool,
    claim_note: String,
}

#[derive(Clone, Debug, Serialize)]
struct RegimeHorizonResult {
    regime: String,
    horizon_s: f64,
    baseline: ModelSummary,
    candidate: ModelSummary,
    delta_failure_aware_rmse_candidate_minus_baseline_m: f64,
    note: String,
}

#[derive(Clone, Debug, Serialize)]
struct Report {
    schema_version: String,
    comparison_kind: String,
    evidence_class: String,
    baseline_method: String,
    baseline_source_commit: String,
    baseline_artifact_sha256: String,
    candidate_method: String,
    candidate_source_commit: String,
    candidate_artifact_sha256: String,
    channel: String,
    unit: String,
    divergence_limit_m: f64,
    dataset_receipt_sha256: String,
    paired_target_receipt_sha256: String,
    input_csv_sha256: String,
    starts_manifest_sha256: String,
    train_manifest_sha256: String,
    training_seeds: Vec<u64>,
    horizons_s: Vec<f64>,
    bootstrap_replicates_requested: usize,
    bootstrap_seed: u64,
    registered_flight_count: usize,
    registered_rollout_start_count: usize,
    starts_per_flight: usize,
    calibration_used_for_thresholds: bool,
    warnings: Vec<String>,
    horizons: Vec<HorizonResult>,
    regime_horizons: Vec<RegimeHorizonResult>,
    claim_ceiling: String,
}

#[derive(Clone, Copy, Debug)]
struct EffectiveError {
    capped_squared_error: f64,
    penalized: bool,
    conditional_error_squared: Option<f64>,
}

fn usage() -> &'static str {
    "Usage: nanodrone_failure_aware_rollout --input rollout_scores.csv --starts-manifest evaluation_starts.csv --train-flights train.txt --dataset-receipt-sha256 HEX64 --paired-target-receipt-sha256 HEX64 --baseline-artifact-sha256 HEX64 --candidate-artifact-sha256 HEX64 --baseline-method ID --candidate-method ID --baseline-source-commit HEX40 --candidate-source-commit HEX40 --training-seeds 100,101,102 --horizons-s 0.5,1.0,2.0 --channel position --divergence-limit-m 2.0 --evidence-class synthetic|simulation|real-flight [--bootstrap 2000] [--seed 17] [--output rollout-report.json]"
}

fn parse_value<T: std::str::FromStr>(value: &str, name: &str) -> Result<T, String> {
    value.parse::<T>().map_err(|_| format!("{name} has invalid value {value:?}"))
}

fn parse_args() -> Result<Args, String> {
    let mut values = HashMap::<String, String>::new();
    let mut iter = env::args().skip(1);
    while let Some(key) = iter.next() {
        if key == "--help" || key == "-h" {
            println!("{}", usage());
            std::process::exit(0);
        }
        if !key.starts_with("--") {
            return Err(format!("unexpected positional argument {key:?}\n{}", usage()));
        }
        let value = iter.next().ok_or_else(|| format!("missing value for {key}"))?;
        if values.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate argument {key}"));
        }
    }
    let mut take = |key: &str| values.remove(key).ok_or_else(|| format!("missing {key}\n{}", usage()));
    let input = PathBuf::from(take("--input")?);
    let starts_manifest = PathBuf::from(take("--starts-manifest")?);
    let train_manifest = PathBuf::from(take("--train-flights")?);
    let dataset_receipt_sha256 = take("--dataset-receipt-sha256")?;
    let paired_target_receipt_sha256 = take("--paired-target-receipt-sha256")?;
    let baseline_artifact_sha256 = take("--baseline-artifact-sha256")?;
    let candidate_artifact_sha256 = take("--candidate-artifact-sha256")?;
    let baseline_method = take("--baseline-method")?;
    let candidate_method = take("--candidate-method")?;
    let baseline_source_commit = take("--baseline-source-commit")?;
    let candidate_source_commit = take("--candidate-source-commit")?;
    let training_seeds = take("--training-seeds")?.split(',')
        .map(|x| parse_value::<u64>(x.trim(), "--training-seeds"))
        .collect::<Result<Vec<_>, _>>()?;
    let horizons_s = take("--horizons-s")?.split(',')
        .map(|x| parse_value::<f64>(x.trim(), "--horizons-s"))
        .collect::<Result<Vec<_>, _>>()?;
    let channel = take("--channel")?;
    let divergence_limit_m: f64 = parse_value(&take("--divergence-limit-m")?, "--divergence-limit-m")?;
    let evidence_class = take("--evidence-class")?;
    let bootstrap_replicates = values.remove("--bootstrap")
        .map(|x| parse_value(&x, "--bootstrap")).transpose()?.unwrap_or(2000usize);
    let bootstrap_seed = values.remove("--seed")
        .map(|x| parse_value(&x, "--seed")).transpose()?.unwrap_or(17u64);
    let output = values.remove("--output").map(PathBuf::from);
    if !values.is_empty() {
        return Err(format!("unknown arguments: {:?}", values.keys().collect::<Vec<_>>()));
    }
    let args = Args {
        input, starts_manifest, train_manifest, dataset_receipt_sha256, paired_target_receipt_sha256,
        baseline_artifact_sha256, candidate_artifact_sha256, baseline_method, candidate_method,
        baseline_source_commit, candidate_source_commit, training_seeds, horizons_s, channel,
        divergence_limit_m, bootstrap_replicates, bootstrap_seed, evidence_class, output,
    };
    validate_args(&args)?;
    Ok(args)
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_args(args: &Args) -> Result<(), String> {
    if args.baseline_method.trim().is_empty() || args.candidate_method.trim().is_empty() {
        return Err("method names must be non-empty".into());
    }
    if args.channel != "position" {
        return Err("this evaluator currently supports only channel=position".into());
    }
    if args.baseline_method == args.candidate_method && args.baseline_source_commit == args.candidate_source_commit {
        return Err("baseline and candidate must be distinct method/source pairs".into());
    }
    for (name, value, length) in [
        ("baseline source commit", &args.baseline_source_commit, 40usize),
        ("candidate source commit", &args.candidate_source_commit, 40usize),
        ("dataset receipt", &args.dataset_receipt_sha256, 64usize),
        ("paired target receipt", &args.paired_target_receipt_sha256, 64usize),
        ("baseline artifact digest", &args.baseline_artifact_sha256, 64usize),
        ("candidate artifact digest", &args.candidate_artifact_sha256, 64usize),
    ] {
        if !valid_hex(value, length) {
            return Err(format!("{name} must be a {length}-character hexadecimal digest"));
        }
    }
    if !args.divergence_limit_m.is_finite() || args.divergence_limit_m <= 0.0 {
        return Err("divergence limit must be positive and finite".into());
    }
    if args.bootstrap_replicates == 0 {
        return Err("bootstrap replicate count must be positive".into());
    }
    if args.training_seeds.len() < 2 {
        return Err("at least two training seeds are required for crossed inference".into());
    }
    let mut seeds = BTreeSet::new();
    if args.training_seeds.iter().any(|seed| !seeds.insert(*seed)) {
        return Err("training seed list contains duplicates".into());
    }
    if args.horizons_s.is_empty() || args.horizons_s.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return Err("horizons must be positive finite values".into());
    }
    let mut horizon_bits = BTreeSet::new();
    if args.horizons_s.iter().any(|x| !horizon_bits.insert(x.to_bits())) {
        return Err("horizon list contains duplicates".into());
    }
    for pair in args.horizons_s.windows(2) {
        if pair[1] <= pair[0] || (pair[1] - pair[0]).abs() <= HORIZON_TOLERANCE_S {
            return Err("horizons must be strictly increasing and separated by more than the matching tolerance".into());
        }
    }
    if !matches!(args.evidence_class.as_str(), "synthetic" | "simulation" | "real-flight") {
        return Err("evidence class must be synthetic, simulation, or real-flight".into());
    }
    Ok(())
}

fn read_manifest(path: &Path) -> Result<(BTreeSet<String>, Vec<u8>), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read manifest {}: {e}", path.display()))?;
    let text = std::str::from_utf8(&bytes).map_err(|_| format!("manifest {} is not UTF-8", path.display()))?;
    let mut ids = BTreeSet::new();
    for (idx, line) in text.lines().enumerate() {
        let id = line.trim();
        if id.is_empty() || id.starts_with('#') { continue; }
        if !ids.insert(id.to_string()) {
            return Err(format!("manifest {} has duplicate flight ID at line {}", path.display(), idx + 1));
        }
    }
    if ids.is_empty() { return Err(format!("manifest {} contains no IDs", path.display())); }
    Ok((ids, bytes))
}

fn read_starts(path: &Path) -> Result<(Vec<StartRow>, Vec<u8>), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read starts manifest {}: {e}", path.display()))?;
    if bytes.is_empty() { return Err("evaluation-start manifest is empty".into()); }
    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_reader(bytes.as_slice());
    let headers = reader.headers().map_err(|e| format!("invalid starts manifest header: {e}"))?.clone();
    for col in ["flight_id", "rollout_start_id", "regime"] {
        if !headers.iter().any(|x| x == col) { return Err(format!("starts manifest missing column {col}")); }
    }
    let mut starts = Vec::new();
    let mut keys = HashSet::new();
    for (idx, record) in reader.deserialize::<StartRow>().enumerate() {
        let row = record.map_err(|e| format!("starts manifest row {}: {e}", idx + 2))?;
        if row.flight_id.trim().is_empty() || row.rollout_start_id.trim().is_empty() || row.regime.trim().is_empty() {
            return Err(format!("starts manifest row {} has empty identity/regime", idx + 2));
        }
        if !keys.insert((row.flight_id.clone(), row.rollout_start_id.clone())) {
            return Err(format!("duplicate flight/start key at starts manifest row {}", idx + 2));
        }
        starts.push(row);
    }
    if starts.is_empty() { return Err("evaluation-start manifest has header but no starts".into()); }
    let mut count_by_flight = BTreeMap::<String, usize>::new();
    for row in &starts { *count_by_flight.entry(row.flight_id.clone()).or_default() += 1; }
    if count_by_flight.len() < 2 {
        return Err("at least two evaluation flights are required for crossed inference".into());
    }
    if count_by_flight.values().copied().collect::<BTreeSet<_>>().len() != 1 {
        return Err("registered rollout starts must be balanced across flights".into());
    }
    Ok((starts, bytes))
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn effective_error(status: &str, error: Option<f64>, limit: f64) -> Result<EffectiveError, String> {
    if !matches!(status, "ok" | "non_finite" | "missing_prediction" | "constraint_violation" | "timeout") {
        return Err(format!("unsupported rollout status {status:?}"));
    }
    if let Some(value) = error {
        if !value.is_finite() || value < 0.0 {
            return Err("position error must be finite and non-negative when present".into());
        }
    }
    if status != "ok" {
        return Ok(EffectiveError { capped_squared_error: limit * limit, penalized: true, conditional_error_squared: None });
    }
    let value = error.ok_or_else(|| "status ok requires a finite position error".to_string())?;
    if value > limit {
        return Ok(EffectiveError { capped_squared_error: limit * limit, penalized: true, conditional_error_squared: None });
    }
    Ok(EffectiveError {
        capped_squared_error: value * value,
        penalized: false,
        conditional_error_squared: Some(value * value),
    })
}

type RolloutKey = (String, String, u64, usize);

struct ValidatedData {
    rows: HashMap<RolloutKey, RolloutRow>,
    starts: Vec<StartRow>,
    flights: Vec<String>,
    seeds: Vec<u64>,
    input_csv_sha256: String,
}

fn validate_complete_grid(
    rows: &HashMap<RolloutKey, RolloutRow>,
    starts: &[StartRow],
    seeds: &[u64],
    horizon_count: usize,
) -> Result<(), String> {
    for start in starts {
        for seed in seeds {
            for horizon_idx in 0..horizon_count {
                let key = (start.flight_id.clone(), start.rollout_start_id.clone(), *seed, horizon_idx);
                if !rows.contains_key(&key) {
                    return Err(format!("missing registered rollout for flight={} start={} seed={} horizon-index={}",
                        start.flight_id, start.rollout_start_id, seed, horizon_idx));
                }
            }
        }
    }
    let expected = starts.len().checked_mul(seeds.len()).and_then(|n| n.checked_mul(horizon_count))
        .ok_or_else(|| "expected rollout count overflow".to_string())?;
    if rows.len() != expected {
        return Err(format!("rollout grid is not complete: got {}, expected {expected}", rows.len()));
    }
    Ok(())
}

fn status_failed(status: &str, error: Option<f64>, limit: f64) -> bool {
    status != "ok" || error.map(|value| value > limit).unwrap_or(true)
}

fn validate_failure_persistence(
    rows: &HashMap<RolloutKey, RolloutRow>,
    starts: &[StartRow],
    seeds: &[u64],
    horizons: &[f64],
    limit: f64,
) -> Result<(), String> {
    for start in starts {
        for seed in seeds {
            let mut baseline_failed = false;
            let mut candidate_failed = false;
            for hidx in 0..horizons.len() {
                let key = (start.flight_id.clone(), start.rollout_start_id.clone(), *seed, hidx);
                let row = rows.get(&key).ok_or_else(|| "validated rollout unexpectedly missing".to_string())?;
                let b_failed_now = status_failed(&row.baseline_status, row.baseline_position_error_m, limit);
                let c_failed_now = status_failed(&row.candidate_status, row.candidate_position_error_m, limit);
                if baseline_failed && !b_failed_now {
                    return Err(format!("baseline appears to recover after prior divergence: flight={} start={} seed={} horizon={}s",
                        start.flight_id,start.rollout_start_id,seed,horizons[hidx]));
                }
                if candidate_failed && !c_failed_now {
                    return Err(format!("candidate appears to recover after prior divergence: flight={} start={} seed={} horizon={}s",
                        start.flight_id,start.rollout_start_id,seed,horizons[hidx]));
                }
                baseline_failed |= b_failed_now;
                candidate_failed |= c_failed_now;
            }
        }
    }
    Ok(())
}

fn read_and_validate(args: &Args, train: &BTreeSet<String>, starts: &[StartRow]) -> Result<ValidatedData, String> {
    let bytes = fs::read(&args.input).map_err(|e| format!("cannot read rollout CSV {}: {e}", args.input.display()))?;
    if bytes.is_empty() { return Err("rollout CSV is empty".into()); }
    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_reader(bytes.as_slice());
    let headers = reader.headers().map_err(|e| format!("invalid rollout CSV header: {e}"))?.clone();
    for col in [
        "flight_id", "rollout_start_id", "regime", "training_seed", "horizon_s",
        "baseline_status", "baseline_position_error_m", "candidate_status", "candidate_position_error_m",
    ] {
        if !headers.iter().any(|x| x == col) { return Err(format!("rollout CSV missing column {col}")); }
    }
    let start_map: HashMap<(String, String), String> = starts.iter()
        .map(|s| ((s.flight_id.clone(), s.rollout_start_id.clone()), s.regime.clone())).collect();
    let seed_set: BTreeSet<u64> = args.training_seeds.iter().copied().collect();
    let mut rows = HashMap::<RolloutKey, RolloutRow>::new();
    for (idx, record) in reader.deserialize::<RolloutRow>().enumerate() {
        let row = record.map_err(|e| format!("rollout CSV row {}: {e}", idx + 2))?;
        let line = idx + 2;
        if row.flight_id.trim().is_empty() || row.rollout_start_id.trim().is_empty() || row.regime.trim().is_empty() {
            return Err(format!("row {line}: flight, rollout start and regime must be non-empty"));
        }
        if !row.horizon_s.is_finite() || row.horizon_s <= 0.0 {
            return Err(format!("row {line}: horizon must be positive finite"));
        }
        let start_key = (row.flight_id.clone(), row.rollout_start_id.clone());
        let registered_regime = start_map.get(&start_key)
            .ok_or_else(|| format!("row {line}: rollout start absent from preregistered manifest"))?;
        if registered_regime != &row.regime {
            return Err(format!("row {line}: regime disagrees with registered start manifest"));
        }
        if train.contains(&row.flight_id) {
            return Err(format!("row {line}: evaluation flight {} appears in training manifest", row.flight_id));
        }
        if !seed_set.contains(&row.training_seed) {
            return Err(format!("row {line}: unexpected training seed {}", row.training_seed));
        }
        let hidx = args.horizons_s.iter().position(|h| (row.horizon_s - *h).abs() <= HORIZON_TOLERANCE_S)
            .ok_or_else(|| format!("row {line}: horizon {} is not in the preregistered grid", row.horizon_s))?;
        effective_error(&row.baseline_status, row.baseline_position_error_m, args.divergence_limit_m)
            .map_err(|e| format!("row {line}: baseline {e}"))?;
        effective_error(&row.candidate_status, row.candidate_position_error_m, args.divergence_limit_m)
            .map_err(|e| format!("row {line}: candidate {e}"))?;
        let key = (row.flight_id.clone(), row.rollout_start_id.clone(), row.training_seed, hidx);
        if rows.insert(key, row).is_some() {
            return Err(format!("row {line}: duplicate flight/start/seed/horizon key"));
        }
    }
    let flights: Vec<String> = starts.iter().map(|s| s.flight_id.clone()).collect::<BTreeSet<_>>().into_iter().collect();
    if flights.iter().any(|flight| train.contains(flight)) {
        return Err("training and evaluation flight manifests overlap".into());
    }
    validate_complete_grid(&rows, starts, &args.training_seeds, args.horizons_s.len())?;
    validate_failure_persistence(&rows, starts, &args.training_seeds, &args.horizons_s, args.divergence_limit_m)?;
    Ok(ValidatedData {
        rows, starts: starts.to_vec(), flights, seeds: args.training_seeds.clone(),
        input_csv_sha256: sha256(&bytes),
    })
}

fn model_summary(rows: &[&RolloutRow], baseline: bool, limit: f64) -> Result<ModelSummary, String> {
    let mut sumsq = 0.0;
    let mut penalized = 0usize;
    let mut conditional_sumsq = 0.0;
    let mut conditional_count = 0usize;
    for row in rows {
        let (status, error) = if baseline {
            (row.baseline_status.as_str(), row.baseline_position_error_m)
        } else {
            (row.candidate_status.as_str(), row.candidate_position_error_m)
        };
        let effective = effective_error(status, error, limit)?;
        sumsq += effective.capped_squared_error;
        penalized += usize::from(effective.penalized);
        if let Some(value) = effective.conditional_error_squared {
            conditional_sumsq += value;
            conditional_count += 1;
        }
    }
    Ok(ModelSummary {
        registered_rollout_count: rows.len(),
        penalized_rollout_count: penalized,
        penalized_rollout_rate: penalized as f64 / rows.len().max(1) as f64,
        failure_aware_position_rmse_m: (sumsq / rows.len().max(1) as f64).sqrt(),
        conditional_finite_position_rmse_m: if conditional_count == 0 { None } else { Some((conditional_sumsq / conditional_count as f64).sqrt()) },
        conditional_finite_rollout_count: conditional_count,
    })
}

#[derive(Clone, Copy, Debug)]
struct Cell {
    baseline_sumsq: f64,
    candidate_sumsq: f64,
    baseline_penalized_count: usize,
    candidate_penalized_count: usize,
    rollout_count: usize,
}

fn cell_stats(data: &ValidatedData, horizon_idx: usize, limit: f64) -> Result<BTreeMap<(String, u64), Cell>, String> {
    let mut starts_by_flight = BTreeMap::<String, Vec<&StartRow>>::new();
    for start in &data.starts {
        starts_by_flight.entry(start.flight_id.clone()).or_default().push(start);
    }
    let mut cells = BTreeMap::new();
    for flight in &data.flights {
        let flight_starts = starts_by_flight.get(flight).ok_or_else(|| format!("no starts for flight {flight}"))?;
        for seed in &data.seeds {
            let mut cell = Cell { baseline_sumsq: 0.0, candidate_sumsq: 0.0,
                baseline_penalized_count: 0, candidate_penalized_count: 0, rollout_count: 0 };
            for start in flight_starts {
                let key = (flight.clone(), start.rollout_start_id.clone(), *seed, horizon_idx);
                let row = data.rows.get(&key).ok_or_else(|| "validated rollout unexpectedly missing".to_string())?;
                let b = effective_error(&row.baseline_status, row.baseline_position_error_m, limit)?;
                let c = effective_error(&row.candidate_status, row.candidate_position_error_m, limit)?;
                cell.baseline_sumsq += b.capped_squared_error;
                cell.candidate_sumsq += c.capped_squared_error;
                cell.baseline_penalized_count += usize::from(b.penalized);
                cell.candidate_penalized_count += usize::from(c.penalized);
                cell.rollout_count += 1;
            }
            cells.insert((flight.clone(), *seed), cell);
        }
    }
    Ok(cells)
}

#[derive(Clone, Copy)]
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn index(&mut self, upper: usize) -> usize {
        if upper <= 1 { 0 } else { (self.next() as usize) % upper }
    }
}

fn percentile(mut values: Vec<f64>, q: f64) -> Option<f64> {
    if values.is_empty() { return None; }
    values.sort_by(f64::total_cmp);
    values.get(((values.len() - 1) as f64 * q).round() as usize).copied()
}
fn interval(values: Vec<f64>) -> Interval {
    Interval { lower_95: percentile(values.clone(), 0.025), upper_95: percentile(values.clone(), 0.975), valid_replicates: values.len() }
}

fn crossed_bootstrap(
    cells: &BTreeMap<(String, u64), Cell>, flights: &[String], seeds: &[u64], reps: usize, seed: u64,
) -> Result<(Interval, Interval), String> {
    if flights.len() < 2 || seeds.len() < 2 {
        let empty = interval(Vec::new());
        return Ok((empty.clone(), empty));
    }
    let mut rng = Rng(seed.max(1));
    let mut rmse_deltas = Vec::with_capacity(reps);
    let mut penalized_rate_deltas = Vec::with_capacity(reps);
    for _ in 0..reps {
        let sampled_flights: Vec<&String> = (0..flights.len()).map(|_| &flights[rng.index(flights.len())]).collect();
        let sampled_seeds: Vec<&u64> = (0..seeds.len()).map(|_| &seeds[rng.index(seeds.len())]).collect();
        let mut b_sumsq = 0.0;
        let mut c_sumsq = 0.0;
        let mut b_penalized = 0usize;
        let mut c_penalized = 0usize;
        let mut count = 0usize;
        for flight in &sampled_flights {
            for s in &sampled_seeds {
                let cell = cells.get(&(flight.to_string(), **s))
                    .ok_or_else(|| "missing crossed flight-by-seed cell during bootstrap".to_string())?;
                b_sumsq += cell.baseline_sumsq;
                c_sumsq += cell.candidate_sumsq;
                b_penalized += cell.baseline_penalized_count;
                c_penalized += cell.candidate_penalized_count;
                count += cell.rollout_count;
            }
        }
        if count > 0 {
            rmse_deltas.push((c_sumsq / count as f64).sqrt() - (b_sumsq / count as f64).sqrt());
            penalized_rate_deltas.push(c_penalized as f64 / count as f64 - b_penalized as f64 / count as f64);
        }
    }
    Ok((interval(rmse_deltas), interval(penalized_rate_deltas)))
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let (train, train_bytes) = read_manifest(&args.train_manifest)?;
    let (starts, starts_bytes) = read_starts(&args.starts_manifest)?;
    let data = read_and_validate(&args, &train, &starts)?;
    let mut warnings = Vec::new();
    if data.flights.len() < 10 { warnings.push("fewer than 10 evaluation recordings; crossed intervals may be unstable".into()); }
    if data.seeds.len() < 10 { warnings.push("fewer than 10 training seeds; seed uncertainty is weakly estimated".into()); }
    let mut horizon_results = Vec::new();
    let mut regime_results = Vec::new();
    for (hidx, horizon) in args.horizons_s.iter().enumerate() {
        let mut rows: Vec<&RolloutRow> = data.rows.iter().filter(|(key, _)| key.3 == hidx).map(|(_, row)| row).collect();
        rows.sort_by(|a, b| a.flight_id.cmp(&b.flight_id)
            .then_with(|| a.training_seed.cmp(&b.training_seed))
            .then_with(|| a.rollout_start_id.cmp(&b.rollout_start_id)));
        let baseline = model_summary(&rows, true, args.divergence_limit_m)?;
        let candidate = model_summary(&rows, false, args.divergence_limit_m)?;
        let delta_rmse = candidate.failure_aware_position_rmse_m - baseline.failure_aware_position_rmse_m;
        let delta_penalty_rate = candidate.penalized_rollout_rate - baseline.penalized_rollout_rate;
        let cells = cell_stats(&data, hidx, args.divergence_limit_m)?;
        let (rmse_ci, penalty_rate_ci) = crossed_bootstrap(&cells, &data.flights, &data.seeds,
            args.bootstrap_replicates, args.bootstrap_seed.wrapping_add(hidx as u64))?;
        let excludes_zero = match (rmse_ci.lower_95, rmse_ci.upper_95) {
            (Some(lo), Some(hi)) => lo > 0.0 || hi < 0.0,
            _ => false,
        };
        horizon_results.push(HorizonResult {
            horizon_s: *horizon, baseline: baseline.clone(), candidate: candidate.clone(),
            delta_failure_aware_rmse_candidate_minus_baseline_m: delta_rmse,
            paired_crossed_bootstrap_delta_rmse_m: rmse_ci,
            delta_penalized_rollout_rate_candidate_minus_baseline: delta_penalty_rate,
            paired_crossed_bootstrap_delta_penalized_rollout_rate: penalty_rate_ci,
            rmse_interval_excludes_zero: excludes_zero,
            claim_note: if excludes_zero {
                "The crossed-bootstrap interval excludes zero on this fixed flight-by-seed cohort. This does not prove controller benefit, deployment-wide generalization, or physical-flight safety.".into()
            } else {
                "The crossed-bootstrap interval does not exclude zero or is undefined. Treat the point estimate as descriptive; do not claim a confirmatory improvement.".into()
            },
        });
        let mut by_regime = BTreeMap::<String, Vec<&RolloutRow>>::new();
        for row in &rows { by_regime.entry(row.regime.clone()).or_default().push(*row); }
        for (regime, regime_rows) in by_regime {
            let b = model_summary(&regime_rows, true, args.divergence_limit_m)?;
            let c = model_summary(&regime_rows, false, args.divergence_limit_m)?;
            regime_results.push(RegimeHorizonResult {
                regime, horizon_s: *horizon, baseline: b.clone(), candidate: c.clone(),
                delta_failure_aware_rmse_candidate_minus_baseline_m: c.failure_aware_position_rmse_m - b.failure_aware_position_rmse_m,
                note: "Descriptive regime-stratified result; no per-regime interval or superiority claim.".into(),
            });
        }
    }
    let report = Report {
        schema_version: "symtropy-failure-aware-rollout-v0.1".into(),
        comparison_kind: "paired-crossed-flight-by-training-seed".into(),
        evidence_class: args.evidence_class,
        baseline_method: args.baseline_method, baseline_source_commit: args.baseline_source_commit,
        baseline_artifact_sha256: args.baseline_artifact_sha256,
        candidate_method: args.candidate_method, candidate_source_commit: args.candidate_source_commit,
        candidate_artifact_sha256: args.candidate_artifact_sha256,
        channel: args.channel, unit: "m".into(), divergence_limit_m: args.divergence_limit_m,
        dataset_receipt_sha256: args.dataset_receipt_sha256,
        paired_target_receipt_sha256: args.paired_target_receipt_sha256,
        input_csv_sha256: data.input_csv_sha256.clone(), starts_manifest_sha256: sha256(&starts_bytes),
        train_manifest_sha256: sha256(&train_bytes), training_seeds: data.seeds.clone(),
        horizons_s: args.horizons_s.clone(), bootstrap_replicates_requested: args.bootstrap_replicates,
        bootstrap_seed: args.bootstrap_seed, registered_flight_count: data.flights.len(),
        registered_rollout_start_count: data.starts.len(),
        starts_per_flight: data.starts.len() / data.flights.len().max(1),
        calibration_used_for_thresholds: false, warnings, horizons: horizon_results,
        regime_horizons: regime_results,
        claim_ceiling: "Failure-aware open-loop prediction comparison only. Every registered rollout remains in the primary score. Whole physical recordings and training seeds are resampled as crossed factors with identical draws for both models. No controller, HIL, or physical-flight safety claim follows from this report.".into(),
    };
    let json = serde_json::to_string_pretty(&report).map_err(|e| format!("cannot serialize report: {e}"))?;
    if let Some(path) = args.output {
        fs::write(&path, format!("{json}\n")).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    } else { println!("{json}"); }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("nanodrone_failure_aware_rollout: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args() -> Args {
        Args { input: PathBuf::new(), starts_manifest: PathBuf::new(), train_manifest: PathBuf::new(),
            dataset_receipt_sha256: "d".repeat(64), paired_target_receipt_sha256: "p".repeat(64),
            baseline_artifact_sha256: "b".repeat(64), candidate_artifact_sha256: "c".repeat(64),
            baseline_method: "baseline".into(), candidate_method: "candidate".into(),
            baseline_source_commit: "a".repeat(40), candidate_source_commit: "e".repeat(40),
            training_seeds: vec![1,2], horizons_s: vec![0.5,1.0], channel: "position".into(),
            divergence_limit_m: 2.0, bootstrap_replicates: 50, bootstrap_seed: 13,
            evidence_class: "synthetic".into(), output: None }
    }
    fn row(f:&str,start:&str,regime:&str,seed:u64,h:f64,bs:&str,be:Option<f64>,cs:&str,ce:Option<f64>)->RolloutRow{
        RolloutRow { flight_id:f.into(), rollout_start_id:start.into(), regime:regime.into(),
            training_seed:seed, horizon_s:h, baseline_status:bs.into(), baseline_position_error_m:be,
            candidate_status:cs.into(), candidate_position_error_m:ce }
    }
    #[test]
    fn failed_and_missing_predictions_remain_in_failure_aware_rmse(){
        let rows=[row("f","s","r",1,0.5,"ok",Some(0.5),"missing_prediction",None)];
        let baseline=model_summary(&[&rows[0]],true,2.0).unwrap();
        let candidate=model_summary(&[&rows[0]],false,2.0).unwrap();
        assert!((baseline.failure_aware_position_rmse_m-0.5).abs()<1e-12);
        assert!((candidate.failure_aware_position_rmse_m-2.0).abs()<1e-12);
        assert_eq!(candidate.penalized_rollout_count,1);
        assert_eq!(candidate.conditional_finite_position_rmse_m,None);
    }
    #[test]
    fn errors_above_limit_are_capped_and_marked_divergent(){
        let e=effective_error("ok",Some(2.7),2.0).unwrap();
        assert_eq!(e.capped_squared_error,4.0);
        assert!(e.penalized);
        assert_eq!(e.conditional_error_squared,None);
    }
    #[test]
    fn finite_error_at_limit_is_not_over_threshold_divergence(){
        let e=effective_error("ok",Some(2.0),2.0).unwrap();
        assert!(!e.penalized);
        assert_eq!(e.capped_squared_error,4.0);
    }
    #[test]
    fn non_ok_status_uses_fixed_penalty(){
        let e=effective_error("non_finite",None,2.0).unwrap();
        assert!(e.penalized);
        assert_eq!(e.capped_squared_error,4.0);
    }
    #[test]
    fn unknown_failure_status_fails_closed(){
        assert!(effective_error("maybe_ok",Some(0.1),2.0).is_err());
    }
    #[test]
    fn incomplete_rollout_grid_is_rejected(){
        let starts=vec![StartRow{flight_id:"f1".into(),rollout_start_id:"s1".into(),regime:"slow".into()}];
        let mut rows=HashMap::new();
        rows.insert(("f1".into(),"s1".into(),1,0),row("f1","s1","slow",1,0.5,"ok",Some(0.1),"ok",Some(0.1)));
        assert!(validate_complete_grid(&rows,&starts,&[1,2],2).is_err());
    }
    #[test]
    fn prior_failure_cannot_silently_recover_at_a_later_horizon(){
        let starts=vec![StartRow{flight_id:"f1".into(),rollout_start_id:"s1".into(),regime:"slow".into()}];
        let mut rows=HashMap::new();
        rows.insert(("f1".into(),"s1".into(),1,0),row("f1","s1","slow",1,0.5,"non_finite",None,"ok",Some(0.1)));
        rows.insert(("f1".into(),"s1".into(),1,1),row("f1","s1","slow",1,1.0,"ok",Some(0.1),"ok",Some(0.1)));
        assert!(validate_failure_persistence(&rows,&starts,&[1],&[0.5,1.0],2.0).is_err());
    }
    #[test]
    fn crossed_bootstrap_is_deterministic_and_detects_synthetic_gain(){
        let mut cells=BTreeMap::new();
        for f in ["f1","f2","f3"] { for seed in [1,2] {
            let b=if f=="f1"{1.0}else{1.5};
            let c=if seed==1{0.5}else{0.75};
            cells.insert((f.to_string(),seed),Cell{baseline_sumsq:b*b*4.0,candidate_sumsq:c*c*4.0,
                baseline_penalized_count:0,candidate_penalized_count:0,rollout_count:4});
        }}
        let flights=vec!["f1".to_string(),"f2".to_string(),"f3".to_string()];
        let seeds=vec![1,2];
        let (a,ad)=crossed_bootstrap(&cells,&flights,&seeds,100,71).unwrap();
        let (b,bd)=crossed_bootstrap(&cells,&flights,&seeds,100,71).unwrap();
        assert_eq!(a.lower_95,b.lower_95);
        assert_eq!(a.upper_95,b.upper_95);
        assert_eq!(ad.lower_95,bd.lower_95);
        assert!(a.upper_95.unwrap()<0.0);
    }
    fn fixture_dir() -> PathBuf {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("symtropy-failure-aware-{stamp}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_rows(path: &Path, rows: &[RolloutRow]) {
        let mut writer = csv::Writer::from_path(path).unwrap();
        for row in rows { writer.serialize(row).unwrap(); }
        writer.flush().unwrap();
    }

    fn complete_fixture_rows() -> Vec<RolloutRow> {
        let mut rows = Vec::new();
        for flight in ["f1", "f2"] {
            for seed in [1_u64, 2_u64] {
                for horizon in [0.5_f64, 1.0_f64] {
                    rows.push(row(flight, "s1", "slow", seed, horizon, "ok", Some(0.2), "ok", Some(0.1)));
                }
            }
        }
        rows
    }

    fn complete_fixture_starts() -> Vec<StartRow> {
        vec![
            StartRow { flight_id: "f1".into(), rollout_start_id: "s1".into(), regime: "slow".into() },
            StartRow { flight_id: "f2".into(), rollout_start_id: "s1".into(), regime: "slow".into() },
        ]
    }

    #[test]
    fn complete_flight_seed_horizon_grid_loads_and_hashes_exact_input() {
        let dir = fixture_dir();
        let input = dir.join("scores.csv");
        write_rows(&input, &complete_fixture_rows());
        let mut a = args();
        a.input = input.clone();
        let data = read_and_validate(&a, &BTreeSet::new(), &complete_fixture_starts()).unwrap();
        assert_eq!(data.rows.len(), 8);
        assert_eq!(data.flights.len(), 2);
        assert_eq!(data.seeds.len(), 2);
        assert_eq!(data.input_csv_sha256.len(), 64);
        fs::remove_file(&input).unwrap();
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn missing_registered_rollout_fails_closed() {
        let dir = fixture_dir();
        let input = dir.join("scores.csv");
        let mut rows = complete_fixture_rows();
        rows.pop();
        write_rows(&input, &rows);
        let mut a = args();
        a.input = input.clone();
        assert!(read_and_validate(&a, &BTreeSet::new(), &complete_fixture_starts()).is_err());
        fs::remove_file(&input).unwrap();
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn regime_mismatch_against_registered_start_is_rejected() {
        let dir = fixture_dir();
        let input = dir.join("scores.csv");
        let mut rows = complete_fixture_rows();
        rows[0].regime = "fast".into();
        write_rows(&input, &rows);
        let mut a = args();
        a.input = input.clone();
        assert!(read_and_validate(&a, &BTreeSet::new(), &complete_fixture_starts()).is_err());
        fs::remove_file(&input).unwrap();
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn divergence_must_persist_at_later_horizons_in_real_input() {
        let dir = fixture_dir();
        let input = dir.join("scores.csv");
        let mut rows = complete_fixture_rows();
        let early = rows.iter_mut().find(|r| r.flight_id == "f1" && r.training_seed == 1 && r.horizon_s == 0.5).unwrap();
        early.baseline_status = "non_finite".into();
        early.baseline_position_error_m = None;
        write_rows(&input, &rows);
        let mut a = args();
        a.input = input.clone();
        assert!(read_and_validate(&a, &BTreeSet::new(), &complete_fixture_starts()).is_err());
        fs::remove_file(&input).unwrap();
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn validation_rejects_duplicate_seed_or_horizon(){
        let mut a=args();
        a.training_seeds=vec![1,1];
        assert!(validate_args(&a).is_err());
        let mut b=args();
        b.horizons_s=vec![0.5,0.5];
        assert!(validate_args(&b).is_err());
    }
}
