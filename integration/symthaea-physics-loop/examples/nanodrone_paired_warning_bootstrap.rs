//! Paired, whole-flight bootstrap for causal warning-method comparisons.
//!
//! One input CSV contains aligned scores from both methods and the shared target.
//! This prevents accidental pairing of different rows or target windows. It is an
//! offline research evaluator, not flight-time safety software.

use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const EPS: f64 = 1.0e-9;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PairedRow {
    flight_id: String,
    decision_time_s: f64,
    outcome_time_s: f64,
    error_value: f64,
    baseline_risk_score: f64,
    candidate_risk_score: f64,
    baseline_feature_max_time_s: f64,
    candidate_feature_max_time_s: f64,
}

impl PairedRow {
    fn label(&self, threshold: f64) -> bool { self.error_value >= threshold }
    fn horizon_s(&self) -> f64 { self.outcome_time_s - self.decision_time_s }
}

#[derive(Debug)]
struct Args {
    input: PathBuf,
    train_manifest: PathBuf,
    calibration_manifest: PathBuf,
    test_manifest: PathBuf,
    dataset_receipt_sha256: String,
    paired_window_receipt_sha256: String,
    baseline_feature_receipt_sha256: String,
    candidate_feature_receipt_sha256: String,
    baseline_method: String,
    candidate_method: String,
    baseline_source_commit: String,
    candidate_source_commit: String,
    channel: String,
    unit: String,
    error_threshold: f64,
    horizon_s: f64,
    event_gap_s: f64,
    alert_merge_gap_s: f64,
    alert_budgets: Vec<f64>,
    bootstrap_replicates: usize,
    seed: u64,
    evidence_class: String,
    output: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct Metrics {
    windows: usize,
    positive_windows: usize,
    alert_windows: usize,
    realized_alert_rate: f64,
    window_recall: Option<f64>,
    window_precision: Option<f64>,
    auroc: Option<f64>,
    auprc_average_precision: Option<f64>,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct OptionalDelta {
    baseline: Option<f64>,
    candidate: Option<f64>,
    candidate_minus_baseline: Option<f64>,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct BootstrapInterval {
    lower_95: Option<f64>,
    upper_95: Option<f64>,
    valid_replicates: usize,
}

#[derive(Clone, Debug, Serialize)]
struct ModelOperatingPoint {
    calibration_threshold: f64,
    calibration_metrics: Metrics,
    test_metrics: Metrics,
    test_event_count: usize,
    detected_event_count: usize,
    test_event_recall: Option<f64>,
    median_best_warning_lead_s: Option<f64>,
    false_alert_episodes: usize,
    observed_flight_minutes: Option<f64>,
    false_alert_episodes_per_flight_minute: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct PairedOperatingPoint {
    nominal_calibration_alert_budget: f64,
    baseline: ModelOperatingPoint,
    candidate: ModelOperatingPoint,
    test_alert_rate_delta_candidate_minus_baseline: f64,
    test_alert_rates_within_1_percentage_point: bool,
    delta_test_auroc: OptionalDelta,
    delta_test_auprc: OptionalDelta,
    delta_test_event_recall: OptionalDelta,
    delta_test_false_alert_episodes_per_flight_minute: OptionalDelta,
    paired_bootstrap_delta_auroc: BootstrapInterval,
    paired_bootstrap_delta_auprc: BootstrapInterval,
    paired_bootstrap_delta_event_recall: BootstrapInterval,
    paired_bootstrap_delta_false_alert_rate: BootstrapInterval,
    caution: String,
}

#[derive(Clone, Debug, Serialize)]
struct Report {
    schema_version: String,
    comparison_kind: String,
    evidence_class: String,
    baseline_method: String,
    baseline_source_commit: String,
    baseline_feature_receipt_sha256: String,
    candidate_method: String,
    candidate_source_commit: String,
    candidate_feature_receipt_sha256: String,
    channel: String,
    unit: String,
    error_threshold: f64,
    expected_horizon_s: f64,
    event_gap_s: f64,
    alert_merge_gap_s: f64,
    alert_budgets: Vec<f64>,
    bootstrap_replicates_requested: usize,
    bootstrap_seed: u64,
    input_csv_sha256: String,
    dataset_receipt_sha256: String,
    paired_window_receipt_sha256: String,
    train_manifest_sha256: String,
    calibration_manifest_sha256: String,
    test_manifest_sha256: String,
    train_flight_count: usize,
    calibration_flight_count: usize,
    test_flight_count: usize,
    calibration_window_count: usize,
    test_window_count: usize,
    warnings: Vec<String>,
    operating_points: Vec<PairedOperatingPoint>,
    claim_ceiling: String,
}

fn usage() -> &'static str {
    "Usage: nanodrone_paired_warning_bootstrap --input paired_scores.csv \\\n--train-flights train.txt --calibration-flights calibration.txt --test-flights test.txt \\\n--dataset-receipt-sha256 HEX64 --paired-window-receipt-sha256 HEX64 \\\n--baseline-feature-receipt-sha256 HEX64 --candidate-feature-receipt-sha256 HEX64 \\\n--baseline-method ID --candidate-method ID --baseline-source-commit HEX40 --candidate-source-commit HEX40 \\\n--channel position --unit m --error-threshold 0.1 --horizon-s 0.5 \\\n--alert-budgets 0.01,0.05,0.10 --event-gap-s 0.02 --alert-merge-gap-s 0.05 \\\n--evidence-class synthetic|simulation|real-flight [--bootstrap 2000] [--seed 17] [--output paired-report.json]"
}

fn parse_num<T: std::str::FromStr>(s: &str, field: &str) -> Result<T, String> {
    s.parse::<T>().map_err(|_| format!("{field} has invalid value {s:?}"))
}

fn parse_args() -> Result<Args, String> {
    let mut raw = HashMap::<String, String>::new();
    let mut iter = env::args().skip(1);
    while let Some(key) = iter.next() {
        if key == "--help" || key == "-h" {
            println!("{}", usage());
            std::process::exit(0);
        }
        if !key.starts_with("--") { return Err(format!("unexpected positional argument {key:?}")); }
        let value = iter.next().ok_or_else(|| format!("missing value for {key}"))?;
        if raw.insert(key.clone(), value).is_some() { return Err(format!("duplicate argument {key}")); }
    }
    let mut take = |key: &str| raw.remove(key).ok_or_else(|| format!("missing {key}\n{}", usage()));
    let input = PathBuf::from(take("--input")?);
    let train_manifest = PathBuf::from(take("--train-flights")?);
    let calibration_manifest = PathBuf::from(take("--calibration-flights")?);
    let test_manifest = PathBuf::from(take("--test-flights")?);
    let dataset_receipt_sha256 = take("--dataset-receipt-sha256")?;
    let paired_window_receipt_sha256 = take("--paired-window-receipt-sha256")?;
    let baseline_feature_receipt_sha256 = take("--baseline-feature-receipt-sha256")?;
    let candidate_feature_receipt_sha256 = take("--candidate-feature-receipt-sha256")?;
    let baseline_method = take("--baseline-method")?;
    let candidate_method = take("--candidate-method")?;
    let baseline_source_commit = take("--baseline-source-commit")?;
    let candidate_source_commit = take("--candidate-source-commit")?;
    let channel = take("--channel")?;
    let unit = take("--unit")?;
    let error_threshold: f64 = parse_num(&take("--error-threshold")?, "--error-threshold")?;
    let horizon_s: f64 = parse_num(&take("--horizon-s")?, "--horizon-s")?;
    let event_gap_s: f64 = parse_num(&take("--event-gap-s")?, "--event-gap-s")?;
    let alert_merge_gap_s: f64 = parse_num(&take("--alert-merge-gap-s")?, "--alert-merge-gap-s")?;
    let alert_budgets = take("--alert-budgets")?.split(',')
        .map(|s| parse_num::<f64>(s.trim(), "--alert-budgets")).collect::<Result<Vec<_>, _>>()?;
    let evidence_class = take("--evidence-class")?;
    let bootstrap_replicates = raw.remove("--bootstrap").map(|s| parse_num(&s, "--bootstrap")).transpose()?.unwrap_or(2000usize);
    let seed = raw.remove("--seed").map(|s| parse_num(&s, "--seed")).transpose()?.unwrap_or(17u64);
    let output = raw.remove("--output").map(PathBuf::from);
    if !raw.is_empty() { return Err(format!("unknown arguments: {:?}", raw.keys().collect::<Vec<_>>())); }
    let args = Args { input, train_manifest, calibration_manifest, test_manifest,
        dataset_receipt_sha256, paired_window_receipt_sha256, baseline_feature_receipt_sha256,
        candidate_feature_receipt_sha256, baseline_method, candidate_method, baseline_source_commit,
        candidate_source_commit, channel, unit, error_threshold, horizon_s, event_gap_s,
        alert_merge_gap_s, alert_budgets, bootstrap_replicates, seed, evidence_class, output };
    validate_args(&args)?;
    Ok(args)
}

fn valid_hex(s: &str, n: usize) -> bool {
    s.len() == n && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_args(a: &Args) -> Result<(), String> {
    for (name, value) in [
        ("baseline method", &a.baseline_method), ("candidate method", &a.candidate_method),
        ("channel", &a.channel), ("unit", &a.unit),
    ] {
        if value.trim().is_empty() { return Err(format!("{name} cannot be empty")); }
    }
    if a.baseline_method == a.candidate_method && a.baseline_source_commit == a.candidate_source_commit {
        return Err("baseline and candidate must identify different method/source pairs".into());
    }
    for (name, s, n) in [
        ("baseline source commit", &a.baseline_source_commit, 40),
        ("candidate source commit", &a.candidate_source_commit, 40),
        ("dataset receipt", &a.dataset_receipt_sha256, 64),
        ("paired target/window receipt", &a.paired_window_receipt_sha256, 64),
        ("baseline feature receipt", &a.baseline_feature_receipt_sha256, 64),
        ("candidate feature receipt", &a.candidate_feature_receipt_sha256, 64),
    ] {
        if !valid_hex(s, n) { return Err(format!("{name} must be a {n}-character hexadecimal digest")); }
    }
    if !a.error_threshold.is_finite() || a.error_threshold <= 0.0
        || !a.horizon_s.is_finite() || a.horizon_s <= 0.0
        || !a.event_gap_s.is_finite() || a.event_gap_s <= 0.0
        || !a.alert_merge_gap_s.is_finite() || a.alert_merge_gap_s <= 0.0 {
        return Err("error threshold, horizon, event gap and alert merge gap must be positive finite numbers".into());
    }
    if a.alert_budgets.is_empty() || a.alert_budgets.iter().any(|x| !x.is_finite() || *x <= 0.0 || *x >= 1.0) {
        return Err("alert budgets must be finite and strictly between zero and one".into());
    }
    if a.bootstrap_replicates == 0 { return Err("bootstrap replicates must be positive".into()); }
    if !matches!(a.evidence_class.as_str(), "synthetic" | "simulation" | "real-flight") {
        return Err("evidence class must be synthetic, simulation, or real-flight".into());
    }
    Ok(())
}

fn read_manifest(path: &Path) -> Result<(BTreeSet<String>, Vec<u8>), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read manifest {}: {e}", path.display()))?;
    let text = std::str::from_utf8(&bytes).map_err(|_| format!("manifest {} is not UTF-8", path.display()))?;
    let mut ids = BTreeSet::new();
    for (i, line) in text.lines().enumerate() {
        let id = line.trim();
        if id.is_empty() || id.starts_with('#') { continue; }
        if !ids.insert(id.to_string()) {
            return Err(format!("manifest {} has duplicate ID on line {}", path.display(), i + 1));
        }
    }
    if ids.is_empty() { return Err(format!("manifest {} contains no flight IDs", path.display())); }
    Ok((ids, bytes))
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn validate_row(r: &PairedRow, line: usize) -> Result<(), String> {
    if r.flight_id.trim().is_empty() { return Err(format!("line {line}: empty flight_id")); }
    for (name, v) in [
        ("decision_time_s", r.decision_time_s), ("outcome_time_s", r.outcome_time_s),
        ("error_value", r.error_value), ("baseline_risk_score", r.baseline_risk_score),
        ("candidate_risk_score", r.candidate_risk_score),
        ("baseline_feature_max_time_s", r.baseline_feature_max_time_s),
        ("candidate_feature_max_time_s", r.candidate_feature_max_time_s),
    ] {
        if !v.is_finite() { return Err(format!("line {line}: {name} must be finite")); }
    }
    if r.error_value < 0.0 { return Err(format!("line {line}: error_value cannot be negative")); }
    if !(0.0..=1.0).contains(&r.baseline_risk_score) || !(0.0..=1.0).contains(&r.candidate_risk_score) {
        return Err(format!("line {line}: both risk scores must be in [0,1]"));
    }
    if r.outcome_time_s <= r.decision_time_s { return Err(format!("line {line}: outcome must be after decision")); }
    if r.baseline_feature_max_time_s > r.decision_time_s + EPS || r.candidate_feature_max_time_s > r.decision_time_s + EPS {
        return Err(format!("line {line}: future-feature leakage detected"));
    }
    Ok(())
}

fn read_rows(path: &Path, expected_horizon: f64) -> Result<(Vec<PairedRow>, Vec<u8>), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read paired scores {}: {e}", path.display()))?;
    if bytes.is_empty() { return Err("paired score CSV is empty".into()); }
    let mut rdr = ReaderBuilder::new().trim(csv::Trim::All).from_reader(bytes.as_slice());
    let headers = rdr.headers().map_err(|e| format!("invalid CSV header: {e}"))?.clone();
    let required = [
        "flight_id", "decision_time_s", "outcome_time_s", "error_value",
        "baseline_risk_score", "candidate_risk_score",
        "baseline_feature_max_time_s", "candidate_feature_max_time_s",
    ];
    for col in required {
        if !headers.iter().any(|h| h == col) { return Err(format!("missing CSV column {col}")); }
    }
    let mut rows = Vec::new();
    let mut keys = HashSet::<(String, u64)>::new();
    let mut last = HashMap::<String, f64>::new();
    for (idx, record) in rdr.deserialize::<PairedRow>().enumerate() {
        let row = record.map_err(|e| format!("CSV record {}: {e}", idx + 2))?;
        let line = idx + 2;
        validate_row(&row, line)?;
        if (row.horizon_s() - expected_horizon).abs() > 1.0e-6 {
            return Err(format!("line {line}: horizon {} differs from expected {expected_horizon}", row.horizon_s()));
        }
        if !keys.insert((row.flight_id.clone(), row.decision_time_s.to_bits())) {
            return Err(format!("line {line}: duplicate flight_id/decision_time key"));
        }
        if let Some(previous) = last.insert(row.flight_id.clone(), row.decision_time_s) {
            if row.decision_time_s <= previous {
                return Err(format!("line {line}: decision times are not strictly increasing for {}", row.flight_id));
            }
        }
        rows.push(row);
    }
    if rows.is_empty() { return Err("paired score CSV has no rows".into()); }
    Ok((rows, bytes))
}

fn validate_splits(
    rows: &[PairedRow], train: &BTreeSet<String>, calibration: &BTreeSet<String>,
    test: &BTreeSet<String>,
) -> Result<(), String> {
    if !train.is_disjoint(calibration) || !train.is_disjoint(test) || !calibration.is_disjoint(test) {
        return Err("flight leakage: train, calibration and test manifests overlap".into());
    }
    let mut observed = BTreeSet::new();
    for row in rows {
        observed.insert(row.flight_id.clone());
        if train.contains(&row.flight_id) { return Err(format!("training flight {} appears in score input", row.flight_id)); }
        if !calibration.contains(&row.flight_id) && !test.contains(&row.flight_id) {
            return Err(format!("flight {} is absent from calibration/test manifests", row.flight_id));
        }
    }
    for id in calibration.iter().chain(test.iter()) {
        if !observed.contains(id) { return Err(format!("manifest flight {id} has no score rows")); }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Ranked { score: f64, positive: bool }

fn auc(values: &[Ranked]) -> Option<f64> {
    let p = values.iter().filter(|x| x.positive).count();
    let n = values.len().checked_sub(p)?;
    if p == 0 || n == 0 { return None; }
    let mut s = values.to_vec();
    s.sort_by(|a,b| a.score.total_cmp(&b.score));
    let mut rank_sum = 0.0;
    let mut i = 0;
    while i < s.len() {
        let mut j = i + 1;
        while j < s.len() && s[j].score == s[i].score { j += 1; }
        let average_rank = ((i + 1) as f64 + j as f64) / 2.0;
        rank_sum += average_rank * s[i..j].iter().filter(|x| x.positive).count() as f64;
        i = j;
    }
    let pf = p as f64;
    Some((rank_sum - pf * (pf + 1.0) / 2.0) / (pf * n as f64))
}

fn average_precision(values: &[Ranked]) -> Option<f64> {
    let positives = values.iter().filter(|x| x.positive).count();
    if positives == 0 { return None; }
    let mut s = values.to_vec();
    s.sort_by(|a,b| b.score.total_cmp(&a.score));
    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut ap = 0.0;
    let mut i = 0;
    while i < s.len() {
        let mut j = i + 1;
        while j < s.len() && s[j].score == s[i].score { j += 1; }
        let group = &s[i..j];
        let gtp = group.iter().filter(|x| x.positive).count();
        let gfp = group.len() - gtp;
        tp += gtp;
        fp += gfp;
        if gtp > 0 {
            ap += gtp as f64 / positives as f64 * (tp as f64 / (tp + fp) as f64);
        }
        i = j;
    }
    Some(ap)
}

fn threshold(scores: &[f64], budget: f64) -> Result<f64, String> {
    if scores.is_empty() { return Err("cannot fit threshold with empty calibration scores".into()); }
    let mut sorted = scores.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = (((1.0 - budget) * sorted.len() as f64).floor() as usize).min(sorted.len() - 1);
    Ok(sorted[index])
}

fn metric_summary(rows: &[PairedRow], baseline: bool, err_threshold: f64, risk_threshold: f64) -> Metrics {
    let ranked: Vec<Ranked> = rows.iter().map(|r| Ranked {
        score: if baseline { r.baseline_risk_score } else { r.candidate_risk_score },
        positive: r.label(err_threshold),
    }).collect();
    let labels = rows.iter().filter(|r| r.label(err_threshold)).count();
    let alerts: Vec<bool> = ranked.iter().map(|r| r.score >= risk_threshold).collect();
    let alert_count = alerts.iter().filter(|x| **x).count();
    let tp = rows.iter().zip(&alerts).filter(|(r,a)| **a && r.label(err_threshold)).count();
    Metrics {
        windows: rows.len(), positive_windows: labels, alert_windows: alert_count,
        realized_alert_rate: alert_count as f64 / rows.len().max(1) as f64,
        window_recall: if labels == 0 { None } else { Some(tp as f64 / labels as f64) },
        window_precision: if alert_count == 0 { None } else { Some(tp as f64 / alert_count as f64) },
        auroc: auc(&ranked), auprc_average_precision: average_precision(&ranked),
    }
}

#[derive(Clone)]
struct Event { flight_id: String, start: f64, end: f64 }

fn severe_events(rows: &[PairedRow], threshold: f64, gap: f64) -> Vec<Event> {
    let mut by_flight: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.error_value >= threshold) {
        by_flight.entry(r.flight_id.clone()).or_default().push(r.outcome_time_s);
    }
    let mut result = Vec::new();
    for (flight, mut times) in by_flight {
        times.sort_by(f64::total_cmp);
        times.dedup_by(|a,b| (*a - *b).abs() <= EPS);
        let mut start = None;
        let mut end = 0.0;
        for t in times {
            match start {
                None => { start = Some(t); end = t; }
                Some(_) if t - end <= gap + EPS => end = t,
                Some(s) => { result.push(Event { flight_id: flight.clone(), start: s, end }); start = Some(t); end = t; }
            }
        }
        if let Some(s) = start { result.push(Event { flight_id: flight, start: s, end }); }
    }
    result
}

#[derive(Clone, Copy)]
struct EventMetrics {
    count: usize,
    detected: usize,
    recall: Option<f64>,
    median_best_lead: Option<f64>,
    false_episodes: usize,
    flight_minutes: Option<f64>,
    false_rate: Option<f64>,
}

fn event_metrics(rows: &[PairedRow], baseline: bool, risk_threshold: f64, error_threshold: f64,
                 event_gap: f64, merge_gap: f64) -> EventMetrics {
    let events = severe_events(rows, error_threshold, event_gap);
    let mut detected = vec![false; events.len()];
    let mut best_leads = vec![None::<f64>; events.len()];
    let mut alert_is_true = vec![false; rows.len()];
    for (ri, row) in rows.iter().enumerate() {
        let score = if baseline { row.baseline_risk_score } else { row.candidate_risk_score };
        if score < risk_threshold { continue; }
        for (ei, ev) in events.iter().enumerate() {
            if row.flight_id == ev.flight_id && row.decision_time_s < ev.start
                && row.outcome_time_s + EPS >= ev.start && row.outcome_time_s <= ev.end + EPS {
                detected[ei] = true;
                let lead = ev.start - row.decision_time_s;
                best_leads[ei] = Some(best_leads[ei].map_or(lead, |old| old.max(lead)));
                alert_is_true[ri] = true;
            }
        }
    }
    let detected_count = detected.iter().filter(|x| **x).count();
    let mut leads: Vec<f64> = best_leads.into_iter().flatten().collect();
    leads.sort_by(f64::total_cmp);
    let median_lead = if leads.is_empty() { None } else {
        let n = leads.len();
        Some(if n % 2 == 1 { leads[n/2] } else { (leads[n/2-1] + leads[n/2]) / 2.0 })
    };

    let mut flights: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i,r) in rows.iter().enumerate() { flights.entry(r.flight_id.clone()).or_default().push(i); }
    let mut false_episodes = 0usize;
    let mut minutes = 0.0;
    for (_, indices) in flights {
        if let (Some(first), Some(last)) = (indices.first(), indices.last()) {
            minutes += ((rows[*last].decision_time_s - rows[*first].decision_time_s).max(0.0)) / 60.0;
        }
        let alerts: Vec<usize> = indices.into_iter().filter(|idx| {
            let score = if baseline { rows[*idx].baseline_risk_score } else { rows[*idx].candidate_risk_score };
            score >= risk_threshold
        }).collect();
        let mut episode: Vec<usize> = Vec::new();
        let mut previous_alert_time: Option<f64> = None;
        for idx in alerts {
            let time = rows[idx].decision_time_s;
            if let Some(previous) = previous_alert_time {
                if time - previous > merge_gap + EPS {
                    if !episode.iter().any(|i| alert_is_true[*i]) { false_episodes += 1; }
                    episode.clear();
                }
            }
            episode.push(idx);
            previous_alert_time = Some(time);
        }
        if !episode.is_empty() && !episode.iter().any(|i| alert_is_true[*i]) { false_episodes += 1; }
    }
    let rate = if minutes > 0.0 { Some(false_episodes as f64 / minutes) } else { None };
    EventMetrics {
        count: events.len(), detected: detected_count,
        recall: if events.is_empty() { None } else { Some(detected_count as f64 / events.len() as f64) },
        median_best_lead: median_lead, false_episodes, flight_minutes: if minutes > 0.0 { Some(minutes) } else { None }, false_rate: rate,
    }
}

fn delta(a: Option<f64>, b: Option<f64>) -> OptionalDelta {
    OptionalDelta { baseline: a, candidate: b, candidate_minus_baseline: match (a,b) {
        (Some(x),Some(y)) => Some(y-x), _ => None,
    }}
}

#[derive(Clone, Copy)]
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0
    }
    fn index(&mut self, n: usize) -> usize { if n <= 1 { 0 } else { (self.next() as usize) % n } }
}

fn percentile(mut values: Vec<f64>, q: f64) -> Option<f64> {
    if values.is_empty() { return None; }
    values.sort_by(f64::total_cmp);
    values.get(((values.len()-1) as f64 * q).round() as usize).copied()
}

fn interval(values: Vec<f64>) -> BootstrapInterval {
    BootstrapInterval { lower_95: percentile(values.clone(),0.025), upper_95: percentile(values.clone(),0.975), valid_replicates: values.len() }
}

fn paired_bootstrap(
    rows: &[PairedRow], baseline_threshold: f64, candidate_threshold: f64, error_threshold: f64,
    event_gap: f64, merge_gap: f64, reps: usize, seed: u64,
) -> (BootstrapInterval, BootstrapInterval, BootstrapInterval, BootstrapInterval) {
    let mut groups: BTreeMap<String, Vec<PairedRow>> = BTreeMap::new();
    for row in rows { groups.entry(row.flight_id.clone()).or_default().push(row.clone()); }
    let clusters: Vec<Vec<PairedRow>> = groups.into_values().collect();
    if clusters.len() < 2 {
        let empty = interval(Vec::new());
        return (empty, empty, empty, empty);
    }
    let mut rng = Rng(seed.max(1));
    let mut auc_deltas = Vec::new();
    let mut ap_deltas = Vec::new();
    let mut event_deltas = Vec::new();
    let mut false_rate_deltas = Vec::new();
    for replicate in 0..reps {
        let mut sample = Vec::new();
        for draw in 0..clusters.len() {
            let selected = &clusters[rng.index(clusters.len())];
            for original in selected {
                let mut row = original.clone();
                row.flight_id = format!("{}#bootstrap-{replicate}-{draw}", original.flight_id);
                sample.push(row);
            }
        }
        let bm = metric_summary(&sample,true,error_threshold,baseline_threshold);
        let cm = metric_summary(&sample,false,error_threshold,candidate_threshold);
        if let (Some(b),Some(c)) = (bm.auroc,cm.auroc) { auc_deltas.push(c-b); }
        if let (Some(b),Some(c)) = (bm.auprc_average_precision,cm.auprc_average_precision) { ap_deltas.push(c-b); }
        let be = event_metrics(&sample,true,baseline_threshold,error_threshold,event_gap,merge_gap);
        let ce = event_metrics(&sample,false,candidate_threshold,error_threshold,event_gap,merge_gap);
        if let (Some(b),Some(c)) = (be.recall,ce.recall) { event_deltas.push(c-b); }
        if let (Some(b),Some(c)) = (be.false_rate,ce.false_rate) { false_rate_deltas.push(c-b); }
    }
    (interval(auc_deltas),interval(ap_deltas),interval(event_deltas),interval(false_rate_deltas))
}

fn model_point(rows_cal: &[PairedRow], rows_test: &[PairedRow], baseline: bool, budget: f64,
               error_threshold: f64, event_gap: f64, merge_gap: f64) -> Result<(ModelOperatingPoint, f64), String> {
    let scores: Vec<f64> = rows_cal.iter().map(|r| if baseline {r.baseline_risk_score} else {r.candidate_risk_score}).collect();
    let th = threshold(&scores,budget)?;
    let cal_metrics = metric_summary(rows_cal,baseline,error_threshold,th);
    let test_metrics = metric_summary(rows_test,baseline,error_threshold,th);
    let em = event_metrics(rows_test,baseline,th,error_threshold,event_gap,merge_gap);
    Ok((ModelOperatingPoint {
        calibration_threshold: th, calibration_metrics: cal_metrics, test_metrics,
        test_event_count: em.count, detected_event_count: em.detected, test_event_recall: em.recall,
        median_best_warning_lead_s: em.median_best_lead, false_alert_episodes: em.false_episodes,
        observed_flight_minutes: em.flight_minutes, false_alert_episodes_per_flight_minute: em.false_rate,
    }, th))
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let (train, train_bytes) = read_manifest(&args.train_manifest)?;
    let (cal, cal_bytes) = read_manifest(&args.calibration_manifest)?;
    let (test, test_bytes) = read_manifest(&args.test_manifest)?;
    let (rows, csv_bytes) = read_rows(&args.input,args.horizon_s)?;
    validate_splits(&rows,&train,&cal,&test)?;
    let cal_rows: Vec<PairedRow> = rows.iter().filter(|r| cal.contains(&r.flight_id)).cloned().collect();
    let test_rows: Vec<PairedRow> = rows.iter().filter(|r| test.contains(&r.flight_id)).cloned().collect();
    let mut warnings = Vec::new();
    if cal.len() < 10 { warnings.push("fewer than 10 calibration flights; thresholds may be unstable".into()); }
    if test.len() < 10 { warnings.push("fewer than 10 test flights; flight-cluster bootstrap intervals may be unstable".into()); }
    let mut operating_points = Vec::new();
    for (i,budget) in args.alert_budgets.iter().enumerate() {
        let (base_point, base_threshold) = model_point(&cal_rows,&test_rows,true,*budget,args.error_threshold,args.event_gap_s,args.alert_merge_gap_s)?;
        let (candidate_point, candidate_threshold) = model_point(&cal_rows,&test_rows,false,*budget,args.error_threshold,args.event_gap_s,args.alert_merge_gap_s)?;
        let alert_delta = candidate_point.test_metrics.realized_alert_rate - base_point.test_metrics.realized_alert_rate;
        let (ci_auc,ci_ap,ci_event,ci_false) = paired_bootstrap(&test_rows,base_threshold,candidate_threshold,args.error_threshold,args.event_gap_s,args.alert_merge_gap_s,args.bootstrap_replicates,args.seed.wrapping_add(i as u64));
        operating_points.push(PairedOperatingPoint {
            nominal_calibration_alert_budget:*budget,
            delta_test_auroc:delta(base_point.test_metrics.auroc,candidate_point.test_metrics.auroc),
            delta_test_auprc:delta(base_point.test_metrics.auprc_average_precision,candidate_point.test_metrics.auprc_average_precision),
            delta_test_event_recall:delta(base_point.test_event_recall,candidate_point.test_event_recall),
            delta_test_false_alert_episodes_per_flight_minute:delta(base_point.false_alert_episodes_per_flight_minute,candidate_point.false_alert_episodes_per_flight_minute),
            paired_bootstrap_delta_auroc:ci_auc, paired_bootstrap_delta_auprc:ci_ap,
            paired_bootstrap_delta_event_recall:ci_event, paired_bootstrap_delta_false_alert_rate:ci_false,
            test_alert_rate_delta_candidate_minus_baseline:alert_delta,
            test_alert_rates_within_1_percentage_point:alert_delta.abs() <= 0.01 + EPS,
            baseline:base_point,candidate:candidate_point,
            caution:if alert_delta.abs() <= 0.01 + EPS {
                "Actual test alert rates are within 1 percentage point, but thresholds were calibration-frozen. Paired intervals are descriptive on the sampled flight population and do not prove controller benefit.".into()
            } else {
                "Different realized test alert rates: do not claim matched-budget superiority. Thresholds were not retuned on test labels.".into()
            },
        });
    }
    let mut seen = HashSet::new();
    let mut warnings = warnings;
    for b in &args.alert_budgets {
        if !seen.insert(b.to_bits()) { warnings.push(format!("duplicate alert budget {b}")); }
    }
    let report = Report {
        schema_version:"symtropy-paired-warning-bootstrap-v0.1".into(),
        comparison_kind:"paired-flight-cluster-bootstrap".into(),
        evidence_class:args.evidence_class, baseline_method:args.baseline_method,
        baseline_source_commit:args.baseline_source_commit,
        baseline_feature_receipt_sha256:args.baseline_feature_receipt_sha256,
        candidate_method:args.candidate_method, candidate_source_commit:args.candidate_source_commit,
        candidate_feature_receipt_sha256:args.candidate_feature_receipt_sha256,
        channel:args.channel, unit:args.unit, error_threshold:args.error_threshold,
        expected_horizon_s:args.horizon_s,event_gap_s:args.event_gap_s,alert_merge_gap_s:args.alert_merge_gap_s,
        alert_budgets:args.alert_budgets,bootstrap_replicates_requested:args.bootstrap_replicates,
        bootstrap_seed:args.seed,input_csv_sha256:sha256(&csv_bytes),dataset_receipt_sha256:args.dataset_receipt_sha256,
        paired_window_receipt_sha256:args.paired_window_receipt_sha256,train_manifest_sha256:sha256(&train_bytes),
        calibration_manifest_sha256:sha256(&cal_bytes),test_manifest_sha256:sha256(&test_bytes),
        train_flight_count:train.len(),calibration_flight_count:cal.len(),test_flight_count:test.len(),
        calibration_window_count:cal_rows.len(),test_window_count:test_rows.len(),warnings,operating_points,
        claim_ceiling:"Paired descriptive inference for fixed target windows on the held-out flight cohort. The bootstrap resamples whole flights with the same draw for both methods. This does not establish deployment-wide calibration, closed-loop control benefit or physical-flight safety.".into(),
    };
    let output = serde_json::to_string_pretty(&report).map_err(|e| format!("cannot serialize report: {e}"))?;
    if let Some(path)=args.output {
        fs::write(&path,format!("{output}\n")).map_err(|e| format!("cannot write {}: {e}",path.display()))?;
    } else { println!("{output}"); }
    Ok(())
}

fn main() {
    if let Err(error)=run() {
        eprintln!("nanodrone_paired_warning_bootstrap: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(flight:&str,t:f64,error:f64,base:f64,candidate:f64)->PairedRow {
        PairedRow { flight_id:flight.into(),decision_time_s:t,outcome_time_s:t+0.5,error_value:error,
            baseline_risk_score:base,candidate_risk_score:candidate,
            baseline_feature_max_time_s:t,candidate_feature_max_time_s:t }
    }
    #[test]
    fn perfect_and_reversed_auc_are_one_and_zero() {
        let vals=[Ranked{score:0.1,positive:false},Ranked{score:0.9,positive:true}];
        assert_eq!(auc(&vals),Some(1.0));
        let rev=[Ranked{score:0.9,positive:false},Ranked{score:0.1,positive:true}];
        assert_eq!(auc(&rev),Some(0.0));
    }
    #[test]
    fn tied_score_auc_is_half_and_ap_is_defined() {
        let vals=[Ranked{score:0.5,positive:true},Ranked{score:0.5,positive:false}];
        assert!((auc(&vals).unwrap()-0.5).abs()<1e-12);
        assert!((average_precision(&vals).unwrap()-0.5).abs()<1e-12);
    }
    #[test]
    fn no_positive_class_stays_undefined() {
        let vals=[Ranked{score:0.1,positive:false},Ranked{score:0.2,positive:false}];
        assert_eq!(auc(&vals),None);
        assert_eq!(average_precision(&vals),None);
    }
    #[test]
    fn feature_timestamp_after_decision_is_rejected() {
        let mut r=row("f",1.0,0.0,0.1,0.1);
        r.candidate_feature_max_time_s=1.1;
        assert!(validate_row(&r,2).is_err());
    }
    #[test]
    fn shared_rows_make_paired_delta_reproducible() {
        let rows=vec![row("f1",0.0,0.0,0.1,0.2),row("f1",0.1,1.0,0.8,0.9),
            row("f2",0.0,0.0,0.2,0.3),row("f2",0.1,1.0,0.7,0.9)];
        let a:Vec<Ranked>=rows.iter().map(|r|Ranked{score:r.baseline_risk_score,positive:r.label(0.5)}).collect();
        let b:Vec<Ranked>=rows.iter().map(|r|Ranked{score:r.candidate_risk_score,positive:r.label(0.5)}).collect();
        assert!(auc(&b).unwrap()>=auc(&a).unwrap());
        let (x1,_,_,_)=paired_bootstrap(&rows,0.5,0.5,0.5,0.11,0.1,30,19);
        let (x2,_,_,_)=paired_bootstrap(&rows,0.5,0.5,0.5,0.11,0.1,30,19);
        assert_eq!(x1.lower_95,x2.lower_95);
        assert_eq!(x1.upper_95,x2.upper_95);
    }
    #[test]
    fn false_alert_episodes_merge_by_elapsed_time() {
        let rows = vec![
            row("f", 0.0, 0.0, 0.0, 0.0),
            row("f", 0.1, 0.0, 0.9, 0.9),
            row("f", 0.2, 0.0, 0.9, 0.9),
            row("f", 0.3, 0.0, 0.0, 0.0),
        ];
        let merged = event_metrics(&rows, true, 0.5, 1.0, 0.11, 0.11);
        let separate = event_metrics(&rows, true, 0.5, 1.0, 0.11, 0.05);
        assert_eq!(merged.false_episodes, 1);
        assert_eq!(separate.false_episodes, 2);
    }

    #[test]
    fn disjoint_train_calibration_test_flights_required() {
        let rows=vec![row("train",0.0,0.0,0.1,0.2)];
        let train=BTreeSet::from(["train".to_string()]);
        let cal=BTreeSet::from(["train".to_string()]);
        let test=BTreeSet::from(["test".to_string()]);
        assert!(validate_splits(&rows,&train,&cal,&test).is_err());
    }
    #[test]
    fn threshold_reports_tie_overshoot_in_realized_rates() {
        let scores=[0.7,0.7,0.7,0.1];
        let threshold=threshold(&scores,0.25).unwrap();
        assert_eq!(scores.iter().filter(|s| **s>=threshold).count(),3);
    }
}
