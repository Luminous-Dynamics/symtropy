//! Flight-clustered evaluation of causal warning scores.
//!
//! Offline research evaluator only. It is not a flight-time safety monitor.
//! One CSV represents one warning method, physical channel, and fixed horizon.
//! See docs/research/SYMTROPY_NANODRONE_DECISION_VALIDITY_BENCHMARK_V0_2.md
//! for the protocol and evidence limitations.

use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const EPS: f64 = 1.0e-9;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ScoreRow {
    flight_id: String,
    decision_time_s: f64,
    outcome_time_s: f64,
    risk_score: f64,
    feature_max_time_s: f64,
    error_value: f64,
}

impl ScoreRow {
    fn label(&self, error_threshold: f64) -> bool {
        self.error_value >= error_threshold
    }
    fn horizon_s(&self) -> f64 {
        self.outcome_time_s - self.decision_time_s
    }
}

#[derive(Debug)]
struct Args {
    input: PathBuf,
    train_flights: PathBuf,
    calibration_flights: PathBuf,
    test_flights: PathBuf,
    model_id: String,
    source_commit: String,
    dataset_receipt_sha256: String,
    paired_receipt_sha256: String,
    feature_receipt_sha256: String,
    channel: String,
    unit: String,
    error_threshold: f64,
    horizon_s: f64,
    event_gap_s: f64,
    alert_merge_gap_s: f64,
    alert_budgets: Vec<f64>,
    bootstrap: usize,
    permutations: usize,
    seed: u64,
    evidence_class: String,
    output: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
struct ScalarMetrics {
    window_count: usize,
    positive_windows: usize,
    alert_count: usize,
    realized_alert_rate: f64,
    true_positive_windows: usize,
    false_positive_windows: usize,
    window_recall: Option<f64>,
    window_precision: Option<f64>,
    auroc: Option<f64>,
    average_precision_auprc: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct Interval {
    lower_95: Option<f64>,
    upper_95: Option<f64>,
    valid_replicates: usize,
}

#[derive(Clone, Debug, Serialize)]
struct Distribution {
    mean: Option<f64>,
    lower_95: Option<f64>,
    upper_95: Option<f64>,
    replicates: usize,
}

#[derive(Clone, Debug, Serialize)]
struct OperatingPoint {
    nominal_calibration_alert_budget: f64,
    frozen_threshold: f64,
    calibration: ScalarMetrics,
    test: ScalarMetrics,
    test_auroc_flight_bootstrap: Interval,
    test_auprc_flight_bootstrap: Interval,
    test_event_count: usize,
    test_detected_event_count: usize,
    test_event_recall: Option<f64>,
    median_warning_lead_s: Option<f64>,
    false_alert_episodes: usize,
    observed_test_flight_minutes: Option<f64>,
    false_alert_episodes_per_flight_minute: Option<f64>,
    permutation_control_auroc: Distribution,
    permutation_control_auprc: Distribution,
}

#[derive(Clone, Debug, Serialize)]
struct Report {
    schema_version: String,
    evidence_class: String,
    warning_method: String,
    source_commit: String,
    channel: String,
    unit: String,
    error_threshold: f64,
    expected_horizon_s: f64,
    horizon_tolerance_s: f64,
    event_gap_s: f64,
    alert_merge_gap_s: f64,
    seed: u64,
    input_csv_sha256: String,
    dataset_receipt_sha256: String,
    paired_window_receipt_sha256: String,
    feature_provenance_receipt_sha256: String,
    train_manifest_sha256: String,
    calibration_manifest_sha256: String,
    test_manifest_sha256: String,
    calibration_flight_count: usize,
    test_flight_count: usize,
    calibration_window_count: usize,
    test_window_count: usize,
    warnings: Vec<String>,
    operating_points: Vec<OperatingPoint>,
}

#[derive(Clone, Copy)]
struct LabelScore {
    score: f64,
    positive: bool,
}

fn usage() -> &'static str {
    "Usage: nanodrone_decision_validity --input scores.csv --train-flights train.txt \
--calibration-flights calibration.txt --test-flights test.txt --model-id ID \
--source-commit HEX40 --dataset-receipt-sha256 HEX64 \
--paired-window-receipt-sha256 HEX64 --feature-receipt-sha256 HEX64 \
--channel position --unit m --error-threshold 0.1 --horizon-s 0.5 \
--alert-budgets 0.01,0.05,0.10 --event-gap-s 0.02 --alert-merge-gap-s 0.05 \
--evidence-class synthetic|simulation|real-flight --output report.json \
[--bootstrap 200] [--permutations 20] [--seed 1]"
}

fn parse_args() -> Result<Args, String> {
    let mut values: HashMap<String, String> = HashMap::new();
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
    let mut take = |name: &str| -> Result<String, String> {
        values.remove(name).ok_or_else(|| format!("missing {name}\n{}", usage()))
    };
    let input = PathBuf::from(take("--input")?);
    let train_flights = PathBuf::from(take("--train-flights")?);
    let calibration_flights = PathBuf::from(take("--calibration-flights")?);
    let test_flights = PathBuf::from(take("--test-flights")?);
    let model_id = take("--model-id")?;
    let source_commit = take("--source-commit")?;
    let dataset_receipt_sha256 = take("--dataset-receipt-sha256")?;
    let paired_receipt_sha256 = take("--paired-window-receipt-sha256")?;
    let feature_receipt_sha256 = take("--feature-receipt-sha256")?;
    let channel = take("--channel")?;
    let unit = take("--unit")?;
    let error_threshold: f64 = parse_number(&take("--error-threshold")?, "--error-threshold")?;
    let horizon_s: f64 = parse_number(&take("--horizon-s")?, "--horizon-s")?;
    let event_gap_s: f64 = parse_number(&take("--event-gap-s")?, "--event-gap-s")?;
    let alert_merge_gap_s: f64 = parse_number(&take("--alert-merge-gap-s")?, "--alert-merge-gap-s")?;
    let evidence_class = take("--evidence-class")?;
    let output = Some(PathBuf::from(take("--output")?));
    let alert_budgets = take("--alert-budgets")?
        .split(',')
        .map(|s| parse_number(s.trim(), "--alert-budgets"))
        .collect::<Result<Vec<f64>, String>>()?;
    let bootstrap = values.remove("--bootstrap").map(|v| v.parse::<usize>()
        .map_err(|_| "--bootstrap must be a non-negative integer".to_string())).transpose()?.unwrap_or(200);
    let permutations = values.remove("--permutations").map(|v| v.parse::<usize>()
        .map_err(|_| "--permutations must be a non-negative integer".to_string())).transpose()?.unwrap_or(20);
    let seed = values.remove("--seed").map(|v| v.parse::<u64>()
        .map_err(|_| "--seed must be an unsigned integer".to_string())).transpose()?.unwrap_or(1);
    if !values.is_empty() {
        return Err(format!("unknown arguments: {:?}", values.keys().collect::<Vec<_>>()));
    }
    let args = Args {
        input, train_flights, calibration_flights, test_flights, model_id, source_commit,
        dataset_receipt_sha256, paired_receipt_sha256, feature_receipt_sha256, channel, unit,
        error_threshold, horizon_s, event_gap_s, alert_merge_gap_s, alert_budgets, bootstrap,
        permutations, seed, evidence_class, output,
    };
    validate_args(&args)?;
    Ok(args)
}

fn parse_number(value: &str, field: &str) -> Result<f64, String> {
    let n = value.parse::<f64>().map_err(|_| format!("{field} must be numeric"))?;
    if !n.is_finite() {
        return Err(format!("{field} must be finite"));
    }
    Ok(n)
}

fn valid_hex(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_args(args: &Args) -> Result<(), String> {
    if args.model_id.trim().is_empty() || args.channel.trim().is_empty() || args.unit.trim().is_empty() {
        return Err("model id, channel, and unit must be non-empty".into());
    }
    if !valid_hex(&args.source_commit, 40) {
        return Err("--source-commit must be a 40-character hexadecimal commit SHA".into());
    }
    for (name, digest) in [
        ("--dataset-receipt-sha256", &args.dataset_receipt_sha256),
        ("--paired-window-receipt-sha256", &args.paired_receipt_sha256),
        ("--feature-receipt-sha256", &args.feature_receipt_sha256),
    ] {
        if !valid_hex(digest, 64) {
            return Err(format!("{name} must be a 64-character hexadecimal SHA-256"));
        }
    }
    if args.error_threshold <= 0.0 || args.horizon_s <= 0.0 || args.event_gap_s <= 0.0 || args.alert_merge_gap_s <= 0.0 {
        return Err("error threshold, horizon, event gap, and alert merge gap must be positive".into());
    }
    if !matches!(args.evidence_class.as_str(), "synthetic" | "simulation" | "real-flight") {
        return Err("--evidence-class must be synthetic, simulation, or real-flight".into());
    }
    if args.alert_budgets.is_empty() || args.alert_budgets.iter().any(|b| !(*b > 0.0 && *b < 1.0)) {
        return Err("each alert budget must lie strictly between 0 and 1".into());
    }
    if args.alert_budgets.iter().any(|b| !b.is_finite()) {
        return Err("alert budgets must be finite".into());
    }
    Ok(())
}

fn read_manifest(path: &Path) -> Result<(BTreeSet<String>, Vec<u8>), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read manifest {}: {e}", path.display()))?;
    let text = std::str::from_utf8(&bytes).map_err(|_| format!("manifest {} is not UTF-8", path.display()))?;
    let mut ids = BTreeSet::new();
    for (line_idx, line) in text.lines().enumerate() {
        let id = line.trim();
        if id.is_empty() || id.starts_with('#') {
            continue;
        }
        if !ids.insert(id.to_string()) {
            return Err(format!("manifest {} has duplicate flight id at line {}", path.display(), line_idx + 1));
        }
    }
    if ids.is_empty() {
        return Err(format!("manifest {} has no flight IDs", path.display()));
    }
    Ok((ids, bytes))
}

fn sha256(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("{:x}", hash.finalize())
}

fn validate_row(row: &ScoreRow, line: usize) -> Result<(), String> {
    if row.flight_id.trim().is_empty() {
        return Err(format!("line {line}: flight_id must be non-empty"));
    }
    for (name, value) in [
        ("decision_time_s", row.decision_time_s),
        ("outcome_time_s", row.outcome_time_s),
        ("risk_score", row.risk_score),
        ("feature_max_time_s", row.feature_max_time_s),
        ("error_value", row.error_value),
    ] {
        if !value.is_finite() { return Err(format!("line {line}: {name} must be finite")); }
    }
    if !(0.0..=1.0).contains(&row.risk_score) || row.error_value < 0.0 {
        return Err(format!("line {line}: risk_score must be in [0,1] and error_value non-negative"));
    }
    if row.outcome_time_s <= row.decision_time_s {
        return Err(format!("line {line}: outcome_time_s must be later than decision_time_s"));
    }
    if row.feature_max_time_s > row.decision_time_s + EPS {
        return Err(format!("line {line}: causal leakage; feature_max_time_s is after decision time"));
    }
    Ok(())
}

fn read_rows(path: &Path) -> Result<(Vec<ScoreRow>, Vec<u8>), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read scores {}: {e}", path.display()))?;
    if bytes.is_empty() {
        return Err("score CSV is empty".into());
    }
    let mut rdr = ReaderBuilder::new().trim(csv::Trim::All).from_reader(bytes.as_slice());
    let headers = rdr.headers().map_err(|e| format!("invalid CSV header: {e}"))?.clone();
    let required = ["flight_id", "decision_time_s", "outcome_time_s", "risk_score", "feature_max_time_s", "error_value"];
    for col in required {
        if !headers.iter().any(|h| h == col) {
            return Err(format!("missing required CSV column {col}"));
        }
    }
    let mut rows = Vec::new();
    for (offset, rec) in rdr.deserialize::<ScoreRow>().enumerate() {
        let row = rec.map_err(|e| format!("CSV record {}: {e}", offset + 2))?;
        let line = offset + 2;
        validate_row(&row, line)?;
        rows.push(row);
    }
    if rows.is_empty() {
        return Err("score CSV has a header but no data".into());
    }
    Ok((rows, bytes))
}

fn validate_splits(
    rows: &[ScoreRow], train: &BTreeSet<String>, calibration: &BTreeSet<String>,
    test: &BTreeSet<String>, expected_horizon_s: f64,
) -> Result<(), String> {
    if !train.is_disjoint(calibration) || !train.is_disjoint(test) || !calibration.is_disjoint(test) {
        return Err("flight leakage: train, calibration, and test manifests overlap".into());
    }
    let mut observed = BTreeSet::new();
    let mut last_time: HashMap<&str, f64> = HashMap::new();
    for row in rows {
        observed.insert(row.flight_id.clone());
        if train.contains(&row.flight_id) {
            return Err(format!("training flight {} appears in score CSV", row.flight_id));
        }
        if !calibration.contains(&row.flight_id) && !test.contains(&row.flight_id) {
            return Err(format!("flight {} is absent from calibration/test manifests", row.flight_id));
        }
        if (row.horizon_s() - expected_horizon_s).abs() > 1.0e-6 {
            return Err(format!("flight {} has horizon {:.9}s, expected {:.9}s", row.flight_id, row.horizon_s(), expected_horizon_s));
        }
        if let Some(prev) = last_time.insert(row.flight_id.as_str(), row.decision_time_s) {
            if row.decision_time_s <= prev {
                return Err(format!("flight {} has duplicate or non-increasing decision timestamps", row.flight_id));
            }
        }
    }
    for id in calibration.iter().chain(test.iter()) {
        if !observed.contains(id) {
            return Err(format!("manifest flight {id} has no rows in score CSV"));
        }
    }
    Ok(())
}

fn quantile_threshold(rows: &[ScoreRow], budget: f64) -> Result<f64, String> {
    if rows.is_empty() { return Err("cannot fit threshold from an empty calibration set".into()); }
    let mut values: Vec<f64> = rows.iter().map(|r| r.risk_score).collect();
    values.sort_by(f64::total_cmp);
    let idx = (((1.0 - budget) * values.len() as f64).floor() as usize).min(values.len() - 1);
    Ok(values[idx])
}

fn rank_auc(scores: &[LabelScore]) -> Option<f64> {
    let positives = scores.iter().filter(|s| s.positive).count();
    let negatives = scores.len().checked_sub(positives)?;
    if positives == 0 || negatives == 0 { return None; }
    let mut values = scores.to_vec();
    values.sort_by(|a, b| a.score.total_cmp(&b.score));
    let mut rank_sum = 0.0;
    let mut i = 0usize;
    while i < values.len() {
        let mut j = i + 1;
        while j < values.len() && values[j].score == values[i].score { j += 1; }
        let avg_rank = ((i + 1) as f64 + j as f64) / 2.0;
        rank_sum += avg_rank * values[i..j].iter().filter(|s| s.positive).count() as f64;
        i = j;
    }
    let p = positives as f64;
    let n = negatives as f64;
    Some((rank_sum - p * (p + 1.0) / 2.0) / (p * n))
}

fn average_precision(scores: &[LabelScore]) -> Option<f64> {
    let positives = scores.iter().filter(|s| s.positive).count();
    if positives == 0 { return None; }
    let mut values = scores.to_vec();
    values.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut ap = 0.0;
    let mut i = 0usize;
    while i < values.len() {
        let mut j = i + 1;
        while j < values.len() && values[j].score == values[i].score { j += 1; }
        let group_tp = values[i..j].iter().filter(|s| s.positive).count();
        let group_n = j - i;
        tp += group_tp;
        fp += group_n - group_tp;
        if group_tp > 0 {
            ap += group_tp as f64 / positives as f64 * (tp as f64 / (tp + fp) as f64);
        }
        i = j;
    }
    Some(ap)
}

fn scalar_metrics(rows: &[ScoreRow], error_threshold: f64, alert: &[bool]) -> ScalarMetrics {
    let positive_windows = rows.iter().filter(|r| r.label(error_threshold)).count();
    let alert_count = alert.iter().filter(|x| **x).count();
    let true_positive_windows = rows.iter().zip(alert).filter(|(r, a)| **a && r.label(error_threshold)).count();
    let false_positive_windows = alert_count - true_positive_windows;
    let au = rows.iter().map(|r| LabelScore { score: r.risk_score, positive: r.label(error_threshold) }).collect::<Vec<_>>();
    ScalarMetrics {
        window_count: rows.len(),
        positive_windows,
        alert_count,
        realized_alert_rate: alert_count as f64 / rows.len().max(1) as f64,
        true_positive_windows,
        false_positive_windows,
        window_recall: if positive_windows == 0 { None } else { Some(true_positive_windows as f64 / positive_windows as f64) },
        window_precision: if alert_count == 0 { None } else { Some(true_positive_windows as f64 / alert_count as f64) },
        auroc: rank_auc(&au),
        average_precision_auprc: average_precision(&au),
    }
}

#[derive(Clone)]
struct Event {
    flight_id: String,
    start_s: f64,
    end_s: f64,
}

fn events(rows: &[ScoreRow], error_threshold: f64, gap_s: f64) -> Vec<Event> {
    let mut per_flight: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.label(error_threshold)) {
        per_flight.entry(r.flight_id.clone()).or_default().push(r.outcome_time_s);
    }
    let mut out = Vec::new();
    for (flight_id, mut times) in per_flight {
        times.sort_by(f64::total_cmp);
        times.dedup_by(|a, b| (*a - *b).abs() <= EPS);
        let mut start = None;
        let mut end = 0.0;
        for t in times {
            match start {
                None => { start = Some(t); end = t; }
                Some(_) if t - end <= gap_s + EPS => { end = t; }
                Some(s) => { out.push(Event { flight_id: flight_id.clone(), start_s: s, end_s: end }); start = Some(t); end = t; }
            }
        }
        if let Some(s) = start { out.push(Event { flight_id, start_s: s, end_s: end }); }
    }
    out
}

fn event_and_alert_metrics(
    rows: &[ScoreRow], alerts: &[bool], error_threshold: f64, event_gap_s: f64, merge_gap_s: f64,
) -> (usize, usize, Option<f64>, Option<f64>, usize, Option<f64>, Option<f64>) {
    let evs = events(rows, error_threshold, event_gap_s);
    let mut detected = vec![false; evs.len()];
    let mut event_leads: Vec<Option<f64>> = vec![None; evs.len()];
    let mut alert_events = vec![false; rows.len()];
    for (ri, (row, is_alert)) in rows.iter().zip(alerts).enumerate() {
        if !*is_alert { continue; }
        for (ei, event) in evs.iter().enumerate() {
            if row.flight_id == event.flight_id
                && row.decision_time_s < event.start_s
                && row.outcome_time_s + EPS >= event.start_s
                && row.outcome_time_s <= event.end_s + EPS
            {
                detected[ei] = true;
                let lead = event.start_s - row.decision_time_s;
                event_leads[ei] = Some(event_leads[ei].map_or(lead, |old| old.max(lead)));
                alert_events[ri] = true;
            }
        }
    }
    let detected_count = detected.iter().filter(|x| **x).count();
    let mut leads: Vec<f64> = event_leads.into_iter().flatten().collect();
    leads.sort_by(f64::total_cmp);
    let median_lead = if leads.is_empty() { None } else {
        let n = leads.len();
        Some(if n % 2 == 1 { leads[n / 2] } else { (leads[n / 2 - 1] + leads[n / 2]) / 2.0 })
    };

    let mut per_flight: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, r) in rows.iter().enumerate() { per_flight.entry(r.flight_id.clone()).or_default().push(i); }
    let mut false_episodes = 0usize;
    let mut duration_minutes = 0.0;
    let mut duration_observable = false;
    for (_, indices) in per_flight {
        if let (Some(first), Some(last)) = (indices.first(), indices.last()) {
            let duration = rows[*last].decision_time_s - rows[*first].decision_time_s;
            if duration > 0.0 { duration_minutes += duration / 60.0; duration_observable = true; }
        }
        let mut episode_start: Option<usize> = None;
        let mut prev_alert: Option<usize> = None;
        let mut episode_has_event = false;
        for idx in indices {
            if !alerts[idx] {
                if episode_start.is_some() && !episode_has_event { false_episodes += 1; }
                episode_start = None; prev_alert = None; episode_has_event = false;
                continue;
            }
            if let Some(prev) = prev_alert {
                if rows[idx].decision_time_s - rows[prev].decision_time_s > merge_gap_s + EPS {
                    if episode_start.is_some() && !episode_has_event { false_episodes += 1; }
                    episode_start = None; episode_has_event = false;
                }
            }
            if episode_start.is_none() { episode_start = Some(idx); }
            episode_has_event |= alert_events[idx];
            prev_alert = Some(idx);
        }
        if episode_start.is_some() && !episode_has_event { false_episodes += 1; }
    }
    let rate = if duration_observable && duration_minutes > 0.0 { Some(false_episodes as f64 / duration_minutes) } else { None };
    let recall = if evs.is_empty() { None } else { Some(detected_count as f64 / evs.len() as f64) };
    (evs.len(), detected_count, recall, median_lead, false_episodes,
     if duration_observable { Some(duration_minutes) } else { None }, rate)
}

#[derive(Clone, Copy)]
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0
    }
    fn index(&mut self, upper: usize) -> usize {
        if upper <= 1 { 0 } else { (self.next() as usize) % upper }
    }
}

fn percentile(mut x: Vec<f64>, q: f64) -> Option<f64> {
    if x.is_empty() { return None; }
    x.sort_by(f64::total_cmp);
    let at = ((x.len() - 1) as f64 * q).round() as usize;
    x.get(at).copied()
}

fn flight_bootstrap(rows: &[ScoreRow], error_threshold: f64, reps: usize, seed: u64) -> (Interval, Interval) {
    if reps == 0 {
        return (Interval { lower_95: None, upper_95: None, valid_replicates: 0 },
                Interval { lower_95: None, upper_95: None, valid_replicates: 0 });
    }
    let mut groups: BTreeMap<String, Vec<ScoreRow>> = BTreeMap::new();
    for row in rows { groups.entry(row.flight_id.clone()).or_default().push(row.clone()); }
    let flights: Vec<Vec<ScoreRow>> = groups.into_values().collect();
    if flights.len() < 2 {
        return (Interval { lower_95: None, upper_95: None, valid_replicates: 0 },
                Interval { lower_95: None, upper_95: None, valid_replicates: 0 });
    }
    let mut rng = Rng(seed.max(1));
    let mut aucs = Vec::new(); let mut aps = Vec::new();
    for _ in 0..reps {
        let mut sample = Vec::new();
        for _ in 0..flights.len() {
            sample.extend(flights[rng.index(flights.len())].iter().cloned());
        }
        let scores: Vec<LabelScore> = sample.iter().map(|r| LabelScore { score: r.risk_score, positive: r.label(error_threshold) }).collect();
        if let Some(v) = rank_auc(&scores) { aucs.push(v); }
        if let Some(v) = average_precision(&scores) { aps.push(v); }
    }
    (Interval { lower_95: percentile(aucs.clone(), 0.025), upper_95: percentile(aucs.clone(), 0.975), valid_replicates: aucs.len() },
     Interval { lower_95: percentile(aps.clone(), 0.025), upper_95: percentile(aps.clone(), 0.975), valid_replicates: aps.len() })
}

fn permuted_scores(rows: &[ScoreRow], seed: u64) -> Vec<ScoreRow> {
    let mut out = rows.to_vec();
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, row) in out.iter().enumerate() { groups.entry(row.flight_id.clone()).or_default().push(i); }
    let mut rng = Rng(seed.max(1));
    for indices in groups.values() {
        let mut values: Vec<f64> = indices.iter().map(|i| out[*i].risk_score).collect();
        for i in (1..values.len()).rev() {
            let j = rng.index(i + 1); values.swap(i, j);
        }
        for (idx, value) in indices.iter().zip(values) { out[*idx].risk_score = value; }
    }
    out
}

fn permutation_distribution(rows: &[ScoreRow], error_threshold: f64, reps: usize, seed: u64, auc: bool) -> Distribution {
    if reps == 0 { return Distribution { mean: None, lower_95: None, upper_95: None, replicates: 0 }; }
    let mut values = Vec::new();
    for rep in 0..reps {
        let shuffled = permuted_scores(rows, seed.wrapping_add(rep as u64 + 1));
        let labels: Vec<LabelScore> = shuffled.iter().map(|r| LabelScore { score: r.risk_score, positive: r.label(error_threshold) }).collect();
        let metric = if auc { rank_auc(&labels) } else { average_precision(&labels) };
        if let Some(v) = metric { values.push(v); }
    }
    let mean = if values.is_empty() { None } else { Some(values.iter().sum::<f64>() / values.len() as f64) };
    Distribution { mean, lower_95: percentile(values.clone(), 0.025), upper_95: percentile(values.clone(), 0.975), replicates: values.len() }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let (train, train_bytes) = read_manifest(&args.train_flights)?;
    let (calibration, calibration_bytes) = read_manifest(&args.calibration_flights)?;
    let (test, test_bytes) = read_manifest(&args.test_flights)?;
    let (rows, csv_bytes) = read_rows(&args.input)?;
    validate_splits(&rows, &train, &calibration, &test, args.horizon_s)?;
    let cal_rows: Vec<ScoreRow> = rows.iter().filter(|r| calibration.contains(&r.flight_id)).cloned().collect();
    let test_rows: Vec<ScoreRow> = rows.iter().filter(|r| test.contains(&r.flight_id)).cloned().collect();
    let mut warnings = Vec::new();
    if calibration.len() < 10 { warnings.push("fewer than 10 independent calibration flights; frozen threshold may be unstable".into()); }
    if test.len() < 10 { warnings.push("fewer than 10 independent test flights; flight bootstrap intervals will be unstable".into()); }
    if args.bootstrap == 0 { warnings.push("flight-clustered confidence intervals disabled (--bootstrap 0)".into()); }
    if args.permutations == 0 { warnings.push("within-flight permutation control disabled (--permutations 0)".into()); }

    let mut operating_points = Vec::new();
    for (bi, budget) in args.alert_budgets.iter().enumerate() {
        let threshold = quantile_threshold(&cal_rows, *budget)?;
        let cal_alerts: Vec<bool> = cal_rows.iter().map(|r| r.risk_score >= threshold).collect();
        let test_alerts: Vec<bool> = test_rows.iter().map(|r| r.risk_score >= threshold).collect();
        let calibration_metrics = scalar_metrics(&cal_rows, args.error_threshold, &cal_alerts);
        let test_metrics = scalar_metrics(&test_rows, args.error_threshold, &test_alerts);
        let (event_count, detected_event_count, event_recall, median_warning_lead_s, false_alert_episodes,
             observed_test_flight_minutes, false_alert_episodes_per_flight_minute) =
            event_and_alert_metrics(&test_rows, &test_alerts, args.error_threshold, args.event_gap_s, args.alert_merge_gap_s);
        let (auc_ci, ap_ci) = flight_bootstrap(&test_rows, args.error_threshold, args.bootstrap, args.seed.wrapping_add(bi as u64));
        let perm_auc = permutation_distribution(&test_rows, args.error_threshold, args.permutations, args.seed.wrapping_add(1000 + bi as u64), true);
        let perm_ap = permutation_distribution(&test_rows, args.error_threshold, args.permutations, args.seed.wrapping_add(2000 + bi as u64), false);
        operating_points.push(OperatingPoint {
            nominal_calibration_alert_budget: *budget, frozen_threshold: threshold,
            calibration: calibration_metrics, test: test_metrics,
            test_auroc_flight_bootstrap: auc_ci, test_auprc_flight_bootstrap: ap_ci,
            test_event_count: event_count, test_detected_event_count: detected_event_count,
            test_event_recall: event_recall, median_warning_lead_s, false_alert_episodes,
            observed_test_flight_minutes, false_alert_episodes_per_flight_minute,
            permutation_control_auroc: perm_auc, permutation_control_auprc: perm_ap,
        });
    }
    let report = Report {
        schema_version: "symtropy-decision-validity-v0.2".into(),
        evidence_class: args.evidence_class,
        warning_method: args.model_id,
        source_commit: args.source_commit,
        channel: args.channel,
        unit: args.unit,
        error_threshold: args.error_threshold,
        expected_horizon_s: args.horizon_s,
        horizon_tolerance_s: 1.0e-6,
        event_gap_s: args.event_gap_s,
        alert_merge_gap_s: args.alert_merge_gap_s,
        seed: args.seed,
        input_csv_sha256: sha256(&csv_bytes),
        dataset_receipt_sha256: args.dataset_receipt_sha256,
        paired_window_receipt_sha256: args.paired_receipt_sha256,
        feature_provenance_receipt_sha256: args.feature_receipt_sha256,
        train_manifest_sha256: sha256(&train_bytes),
        calibration_manifest_sha256: sha256(&calibration_bytes),
        test_manifest_sha256: sha256(&test_bytes),
        calibration_flight_count: calibration.len(),
        test_flight_count: test.len(),
        calibration_window_count: cal_rows.len(),
        test_window_count: test_rows.len(),
        warnings,
        operating_points,
    };
    let json = serde_json::to_string_pretty(&report).map_err(|e| format!("cannot serialize report: {e}"))?;
    if let Some(path) = args.output {
        fs::write(&path, format!("{json}\n")).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    } else {
        println!("{json}");
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("nanodrone_decision_validity: {err}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(flight: &str, decision: f64, outcome: f64, score: f64, error: f64) -> ScoreRow {
        ScoreRow { flight_id: flight.into(), decision_time_s: decision, outcome_time_s: outcome,
            risk_score: score, feature_max_time_s: decision, error_value: error }
    }

    #[test]
    fn auc_perfect_and_reversed_rankings() {
        let perfect = vec![
            LabelScore { score: 0.1, positive: false }, LabelScore { score: 0.2, positive: false },
            LabelScore { score: 0.8, positive: true }, LabelScore { score: 0.9, positive: true },
        ];
        assert!((rank_auc(&perfect).unwrap() - 1.0).abs() < 1e-12);
        let reversed: Vec<LabelScore> = perfect.iter().map(|s| LabelScore { score: 1.0 - s.score, positive: s.positive }).collect();
        assert!((rank_auc(&reversed).unwrap() - 0.0).abs() < 1e-12);
    }

    #[test]
    fn tied_scores_have_half_auc_and_prevalence_average_precision() {
        let tied = vec![
            LabelScore { score: 0.5, positive: true }, LabelScore { score: 0.5, positive: false },
            LabelScore { score: 0.5, positive: false }, LabelScore { score: 0.5, positive: false },
        ];
        assert!((rank_auc(&tied).unwrap() - 0.5).abs() < 1e-12);
        assert!((average_precision(&tied).unwrap() - 0.25).abs() < 1e-12);
    }

    #[test]
    fn no_positive_label_has_undefined_auprc() {
        let rows = vec![LabelScore { score: 0.2, positive: false }];
        assert_eq!(average_precision(&rows), None);
        assert_eq!(rank_auc(&rows), None);
    }

    #[test]
    fn tied_calibration_scores_report_realized_alert_rate() {
        let cal = vec![row("c1", 0.0, 0.5, 0.7, 0.0), row("c1", 0.01, 0.51, 0.7, 0.0),
            row("c2", 0.0, 0.5, 0.7, 0.0), row("c2", 0.01, 0.51, 0.1, 0.0)];
        let threshold = quantile_threshold(&cal, 0.25).unwrap();
        let alerts: Vec<bool> = cal.iter().map(|r| r.risk_score >= threshold).collect();
        let metrics = scalar_metrics(&cal, 0.1, &alerts);
        assert!(metrics.realized_alert_rate >= 0.25);
        assert!(metrics.alert_count > 0);
    }

    #[test]
    fn future_feature_timestamp_is_rejected() {
        let r = row("f1", 1.0, 1.5, 0.8, 0.1);
        let mut invalid = r.clone();
        invalid.feature_max_time_s = 1.01;
        assert!(validate_row(&invalid, 2).is_err());
    }

    #[test]
    fn an_alert_after_event_onset_is_not_early_detection() {
        let rows = vec![row("f1", 0.0, 0.2, 0.0, 2.0), row("f1", 0.1, 0.3, 1.0, 2.0),
            row("f1", 0.2, 0.4, 1.0, 2.0), row("f1", 0.3, 0.5, 0.0, 2.0)];
        let alerts = vec![false, false, true, true];
        let (_, detected, recall, _, _, _, _) = event_and_alert_metrics(&rows, &alerts, 1.0, 0.11, 0.11);
        assert_eq!(detected, 0);
        assert_eq!(recall, Some(0.0));
    }

    #[test]
    fn early_alert_detects_event_and_counts_lead_time() {
        let rows = vec![row("f1", 0.0, 0.2, 1.0, 2.0), row("f1", 0.1, 0.3, 0.0, 2.0),
            row("f1", 0.2, 0.4, 0.0, 2.0), row("f1", 0.3, 0.5, 0.0, 0.0)];
        let alerts = vec![true, false, false, false];
        let (_, detected, recall, lead, _, _, _) = event_and_alert_metrics(&rows, &alerts, 1.0, 0.11, 0.11);
        assert_eq!(detected, 1);
        assert_eq!(recall, Some(1.0));
        assert!(lead.unwrap() > 0.0);
    }

    #[test]
    fn within_flight_permutation_preserves_score_multiset_and_is_deterministic() {
        let rows = vec![row("a", 0.0, 0.5, 0.1, 0.0), row("a", 0.1, 0.6, 0.2, 1.0),
            row("a", 0.2, 0.7, 0.3, 0.0), row("b", 0.0, 0.5, 0.4, 1.0), row("b", 0.1, 0.6, 0.5, 0.0)];
        let mut original = BTreeMap::new();
        for r in &rows { original.entry(r.flight_id.clone()).or_insert_with(Vec::new).push(r.risk_score); }
        let first = permuted_scores(&rows, 77);
        let second = permuted_scores(&rows, 77);
        assert_eq!(first.iter().map(|r| r.risk_score).collect::<Vec<_>>(), second.iter().map(|r| r.risk_score).collect::<Vec<_>>());
        let mut permuted = BTreeMap::new();
        for r in &first { permuted.entry(r.flight_id.clone()).or_insert_with(Vec::new).push(r.risk_score); }
        for values in original.values_mut() { values.sort_by(f64::total_cmp); }
        for values in permuted.values_mut() { values.sort_by(f64::total_cmp); }
        assert_eq!(original, permuted);
    }

    #[test]
    fn manifest_overlap_is_rejected() {
        let rows = vec![row("flight-a", 0.0, 0.5, 0.1, 0.0)];
        let train = BTreeSet::from(["flight-a".to_string()]);
        let cal = BTreeSet::from(["flight-a".to_string()]);
        let test = BTreeSet::from(["flight-b".to_string()]);
        assert!(validate_splits(&rows, &train, &cal, &test, 0.5).is_err());
    }

    #[test]
    fn horizon_mismatch_is_rejected() {
        let rows = vec![row("f1", 0.0, 0.51, 0.2, 0.0)];
        let train = BTreeSet::from(["train".to_string()]);
        let cal = BTreeSet::from(["f1".to_string()]);
        let test = BTreeSet::from(["f2".to_string()]);
        assert!(validate_splits(&rows, &train, &cal, &test, 0.5).is_err());
    }
}
