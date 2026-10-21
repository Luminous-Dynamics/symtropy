use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const THRESHOLDS: [u64; 4] = [300, 900, 1_800, 3_600];
const T_CRITICAL_DF_19: f64 = 2.093;
// For the fixed primary family (4 thresholds x 6 scenarios = 24 tests),
// t(1 - 0.05/(2*24), df=19) ~= 3.562. Round upward to 4.0 so the
// Bonferroni simultaneous intervals remain conservative without a runtime
// statistics dependency. This constant is valid only for this frozen design.
const T_CRITICAL_DF_19_BONFERRONI_24_CONSERVATIVE: f64 = 4.0;
const PRIMARY_COMPARISON_COUNT: usize = 24;

#[derive(Debug, Deserialize)]
struct Report {
    benchmark_id: String,
    dispatch_policy: String,
    maximum_queue_age_seconds: u64,
    replication_count: usize,
    replication_seeds: Vec<u64>,
    scenario_count: usize,
    release_ready: bool,
    output_never_exceeds_released_input: bool,
    all_buffers_respected_capacity: bool,
    station_time_accounting_within_horizon: bool,
    replication_scenario_records: Vec<Record>,
}
#[derive(Debug, Deserialize)]
struct Record {
    seed: u64,
    scenario_id: String,
    measured_good_units_per_hour: f64,
    good_units_total: u64,
    released_jobs_total: u64,
    scrapped_jobs_total: u64,
    measured_rework_attempts: u64,
    variant_wait_statistics: Vec<VariantWait>,
    stations: Vec<Station>,
}
#[derive(Debug, Deserialize)]
struct VariantWait {
    variant: usize,
    sample_count: usize,
    average_wait_seconds: Option<f64>,
    p95_wait_seconds: Option<f64>,
    maximum_wait_seconds: Option<u64>,
}
#[derive(Debug, Deserialize)]
struct Station {
    station_id: String,
    changeovers_started: u64,
    output_transfers: u64,
    out_of_order_output_transfers: u64,
}
#[derive(Debug, Clone, serde::Serialize)]
struct Pair {
    seed: u64,
    delta: f64,
}
#[derive(Debug, serde::Serialize)]
struct Summary {
    sample_count: usize,
    mean: f64,
    sample_standard_deviation: f64,
    confidence_95_lower_unadjusted: f64,
    confidence_95_upper_unadjusted: f64,
    paired_seed_deltas: Vec<Pair>,
}
#[derive(Debug, serde::Serialize)]
struct WaitDelta {
    variant: usize,
    average_wait_delta_seconds: Option<Summary>,
    p95_wait_delta_seconds: Option<Summary>,
    maximum_wait_delta_seconds: Option<Summary>,
}
#[derive(Debug, serde::Serialize)]
struct ScenarioDelta {
    scenario_id: String,
    good_units_per_hour_delta: Summary,
    good_units_total_delta: Summary,
    scrapped_jobs_delta: Summary,
    rework_attempts_delta: Summary,
    per_variant_wait_deltas: Vec<WaitDelta>,
}
#[derive(Debug, serde::Serialize)]
struct StationDelta {
    scenario_id: String,
    station_id: String,
    changeovers_delta: Summary,
    output_transfers_delta: Summary,
    out_of_order_output_transfers_delta: Summary,
}
#[derive(Debug, serde::Serialize)]
struct ThresholdResult {
    maximum_queue_age_seconds: u64,
    scenario_deltas: Vec<ScenarioDelta>,
    station_deltas: Vec<StationDelta>,
}
#[derive(Debug, serde::Serialize)]
struct ThroughputFamilywiseInterval {
    maximum_queue_age_seconds: u64,
    scenario_id: String,
    sample_count: usize,
    mean_delta_units_per_hour: f64,
    sample_standard_deviation: f64,
    confidence_95_lower_bonferroni: f64,
    confidence_95_upper_bonferroni: f64,
    family_size: usize,
    t_critical_conservative: f64,
}
#[derive(Debug, serde::Serialize)]
struct SensitivityReport {
    schema_version: String,
    benchmark_id: String,
    baseline_policy: String,
    candidate_policy: String,
    thresholds_seconds: Vec<u64>,
    replication_count: usize,
    replication_seeds: Vec<u64>,
    paired_seed_scenario_keys_match: bool,
    confidence_interval_method: String,
    primary_metric_family: String,
    primary_comparison_count: usize,
    primary_familywise_interval_method: String,
    primary_throughput_familywise_intervals: Vec<ThroughputFamilywiseInterval>,
    multiplicity_adjustment: String,
    threshold_results: Vec<ThresholdResult>,
    claim_ceiling: String,
}
type Index<'a> = BTreeMap<(u64, String), &'a Record>;

fn main() {
    if let Err(e) = run() {
        eprintln!("ev_factory_dispatch_threshold_report: {e}");
        process::exit(2);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 5 {
        return Err("usage: ev_factory_dispatch_threshold_report BASELINE_JSON 300=PATH 900=PATH 1800=PATH 3600=PATH".into());
    }
    let base: Report = read_json(&args[0])?;
    validate(&base, "fifo", None)?;
    let mut candidates = Vec::new();
    for arg in &args[1..] {
        let (v, p) = arg
            .split_once('=')
            .ok_or_else(|| format!("expected_THRESHOLD=PATH: {arg}"))?;
        let age = v
            .parse::<u64>()
            .map_err(|_| format!("invalid_threshold: {v}"))?;
        let r: Report = read_json(p)?;
        validate(&r, "minimize_changeover", Some(age))?;
        pair_compatible(&base, &r, age)?;
        candidates.push((age, r));
    }
    let ages: Vec<u64> = candidates.iter().map(|(a, _)| *a).collect();
    if ages != THRESHOLDS {
        return Err(format!(
            "threshold_grid_must_be_{THRESHOLDS:?}_got_{ages:?}"
        ));
    }
    let base_ix = index(&base)?;
    let mut results = Vec::new();
    for (age, r) in &candidates {
        results.push(compare(*age, &base_ix, r, &index(r)?)?);
    }
    let primary_throughput_familywise_intervals =
        build_primary_familywise_intervals(&results, PRIMARY_COMPARISON_COUNT)?;
    let out = SensitivityReport {
        schema_version:"ev-factory-age-threshold-sensitivity/0.2".into(),
        benchmark_id:base.benchmark_id.clone(), baseline_policy:"fifo".into(),
        candidate_policy:"minimize_changeover".into(), thresholds_seconds:ages,
        replication_count:base.replication_count, replication_seeds:base.replication_seeds.clone(),
        paired_seed_scenario_keys_match:true,
        confidence_interval_method:format!("secondary paired t intervals over same-seed deltas; 95%, df=19, t-critical={T_CRITICAL_DF_19}; unadjusted"),
        primary_metric_family:"good units/hour delta vs FIFO across all four thresholds and six fixed scenarios; two-sided family size 24".into(),
        primary_comparison_count:PRIMARY_COMPARISON_COUNT,
        primary_familywise_interval_method:format!("Bonferroni simultaneous 95% intervals for primary throughput family only; paired t, df=19; conservative t-critical={T_CRITICAL_DF_19_BONFERRONI_24_CONSERVATIVE}; all secondary metrics remain unadjusted and descriptive"),
        primary_throughput_familywise_intervals,
        multiplicity_adjustment:"Bonferroni-conservative family-wise intervals apply only to the predeclared 24 throughput comparisons; all secondary metrics are unadjusted sensitivity measures; no threshold winner is selected".into(),
        threshold_results:results,
        claim_ceiling:"Synthetic discrete-event sensitivity analysis over a frozen fixture and 20 seeds only; primary throughput intervals are Bonferroni-conservative for the fixed 24-comparison family; no plant calibration, hard maximum-wait guarantee, production recommendation, or state-of-the-art claim.".into(),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&out).map_err(|e| format!("serialize:{e}"))?
    );
    Ok(())
}
fn build_primary_familywise_intervals(
    results: &[ThresholdResult],
    family_size: usize,
) -> Result<Vec<ThroughputFamilywiseInterval>, String> {
    if results.len() != THRESHOLDS.len() || family_size != PRIMARY_COMPARISON_COUNT {
        return Err("primary_throughput_family_size_or_threshold_count_mismatch".into());
    }
    let expected = results.len() * 6;
    if expected != family_size || results.iter().any(|r| r.scenario_deltas.len() != 6) {
        return Err("primary_throughput_family_requires_four_thresholds_and_six_scenarios".into());
    }
    let mut intervals = Vec::with_capacity(family_size);
    for result in results {
        if !THRESHOLDS.contains(&result.maximum_queue_age_seconds) {
            return Err("unexpected_threshold_in_primary_family".into());
        }
        for scenario in &result.scenario_deltas {
            let summary = &scenario.good_units_per_hour_delta;
            if summary.sample_count != 20
                || !summary.mean.is_finite()
                || !summary.sample_standard_deviation.is_finite()
                || summary.sample_standard_deviation < 0.0
            {
                return Err("primary_throughput_interval_requires_twenty_finite_paired_deltas".into());
            }
            let half_width = T_CRITICAL_DF_19_BONFERRONI_24_CONSERVATIVE
                * summary.sample_standard_deviation
                / (summary.sample_count as f64).sqrt();
            if !half_width.is_finite() {
                return Err("non_finite_primary_familywise_interval".into());
            }
            intervals.push(ThroughputFamilywiseInterval {
                maximum_queue_age_seconds: result.maximum_queue_age_seconds,
                scenario_id: scenario.scenario_id.clone(),
                sample_count: summary.sample_count,
                mean_delta_units_per_hour: summary.mean,
                sample_standard_deviation: summary.sample_standard_deviation,
                confidence_95_lower_bonferroni: summary.mean - half_width,
                confidence_95_upper_bonferroni: summary.mean + half_width,
                family_size,
                t_critical_conservative: T_CRITICAL_DF_19_BONFERRONI_24_CONSERVATIVE,
            });
        }
    }
    if intervals.len() != family_size {
        return Err("primary_familywise_interval_count_mismatch".into());
    }
    Ok(intervals)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    let s = fs::read_to_string(path).map_err(|e| format!("read_{path}:{e}"))?;
    serde_json::from_str(&s).map_err(|e| format!("parse_{path}:{e}"))
}
fn validate(r: &Report, policy: &str, age: Option<u64>) -> Result<(), String> {
    if r.benchmark_id != "EV-FACTORY-SIM-001" || r.dispatch_policy != policy {
        return Err(format!(
            "unexpected_benchmark_or_policy:{}:{}",
            r.benchmark_id, r.dispatch_policy
        ));
    }
    if age.is_some_and(|v| r.maximum_queue_age_seconds != v) {
        return Err(format!(
            "threshold_mismatch_expected_{age:?}_got_{}",
            r.maximum_queue_age_seconds
        ));
    }
    let unique = r.replication_seeds.iter().copied().collect::<BTreeSet<_>>();
    if r.replication_count != 20
        || r.replication_seeds.len() != 20
        || unique.len() != 20
        || unique.contains(&0)
    {
        return Err("expected_twenty_unique_positive_seed_replications".into());
    }
    if r.scenario_count != 6 || r.replication_scenario_records.len() != 120 {
        return Err("expected_six_scenarios_and_120_records".into());
    }
    if r.release_ready
        || !r.output_never_exceeds_released_input
        || !r.all_buffers_respected_capacity
        || !r.station_time_accounting_within_horizon
    {
        return Err("simulation_guardrail_failed_or_release_ready_true".into());
    }
    let ix = index(r)?;
    if ix.keys().any(|(seed, _)| !unique.contains(seed)) {
        return Err("record_seed_not_in_declared_replications".into());
    }
    if ix.keys().map(|(_, s)| s).collect::<BTreeSet<_>>().len() != 6 {
        return Err("expected_six_scenario_ids".into());
    }
    Ok(())
}
fn index(r: &Report) -> Result<Index<'_>, String> {
    let mut out = BTreeMap::new();
    for x in &r.replication_scenario_records {
        if x.seed == 0 || x.scenario_id.trim().is_empty() {
            return Err("invalid_record_key".into());
        }
        if out.insert((x.seed, x.scenario_id.clone()), x).is_some() {
            return Err("duplicate_seed_scenario_key".into());
        }
        if x.good_units_total > x.released_jobs_total || !x.measured_good_units_per_hour.is_finite()
        {
            return Err("output_conservation_or_throughput_invalid".into());
        }
        if x.variant_wait_statistics.len() != 3
            || x.variant_wait_statistics
                .iter()
                .any(|v| v.sample_count == 0)
            || x.variant_wait_statistics
                .iter()
                .map(|v| v.variant)
                .collect::<BTreeSet<_>>()
                .len()
                != 3
        {
            return Err("variant_wait_metrics_invalid".into());
        }
        if x.stations.len() != 5
            || x.stations
                .iter()
                .map(|s| s.station_id.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                != 5
        {
            return Err("station_metrics_invalid".into());
        }
    }
    Ok(out)
}
fn pair_compatible(a: &Report, b: &Report, age: u64) -> Result<(), String> {
    if a.benchmark_id != b.benchmark_id
        || a.replication_count != b.replication_count
        || a.replication_seeds != b.replication_seeds
        || a.scenario_count != b.scenario_count
    {
        return Err(format!("metadata_mismatch_threshold_{age}"));
    }
    if index(a)?.keys().collect::<Vec<_>>() != index(b)?.keys().collect::<Vec<_>>() {
        return Err(format!("seed_scenario_key_mismatch_threshold_{age}"));
    }
    Ok(())
}
fn compare(
    age: u64,
    base: &Index<'_>,
    candidate: &Report,
    comp: &Index<'_>,
) -> Result<ThresholdResult, String> {
    let scenarios = base.keys().map(|(_, s)| s.clone()).collect::<BTreeSet<_>>();
    let mut scenario_deltas = Vec::new();
    let mut station_deltas = Vec::new();
    for scenario in scenarios {
        let (mut throughput, mut good, mut scrap, mut rework) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut waits: BTreeMap<usize, (Vec<Option<Pair>>, Vec<Option<Pair>>, Vec<Option<Pair>>)> =
            BTreeMap::new();
        let mut stations: BTreeMap<String, (Vec<Pair>, Vec<Pair>, Vec<Pair>)> = BTreeMap::new();
        for seed in candidate.replication_seeds.iter().copied() {
            let key = (seed, scenario.clone());
            let a = base
                .get(&key)
                .ok_or_else(|| format!("baseline_missing_{seed}_{scenario}"))?;
            let b = comp
                .get(&key)
                .ok_or_else(|| format!("candidate_missing_{seed}_{scenario}_age_{age}"))?;
            throughput.push(pair(
                seed,
                b.measured_good_units_per_hour,
                a.measured_good_units_per_hour,
            ));
            good.push(pair(
                seed,
                b.good_units_total as f64,
                a.good_units_total as f64,
            ));
            scrap.push(pair(
                seed,
                b.scrapped_jobs_total as f64,
                a.scrapped_jobs_total as f64,
            ));
            rework.push(pair(
                seed,
                b.measured_rework_attempts as f64,
                a.measured_rework_attempts as f64,
            ));
            let av = variant_map(a)?;
            let bv = variant_map(b)?;
            if av.keys().collect::<Vec<_>>() != bv.keys().collect::<Vec<_>>() {
                return Err("variant_set_mismatch".into());
            }
            for (v, x) in av {
                let y = bv.get(&v).ok_or("candidate_variant_missing")?;
                let e = waits.entry(v).or_default();
                e.0.push(optional_pair(
                    seed,
                    y.average_wait_seconds,
                    x.average_wait_seconds,
                )?);
                e.1.push(optional_pair(seed, y.p95_wait_seconds, x.p95_wait_seconds)?);
                e.2.push(optional_pair(
                    seed,
                    y.maximum_wait_seconds.map(|n| n as f64),
                    x.maximum_wait_seconds.map(|n| n as f64),
                )?);
            }
            let am = station_map(a)?;
            let bm = station_map(b)?;
            if am.keys().collect::<Vec<_>>() != bm.keys().collect::<Vec<_>>() {
                return Err("station_set_mismatch".into());
            }
            for (id, x) in am {
                let y = bm.get(&id).ok_or("candidate_station_missing")?;
                let e = stations.entry(id).or_default();
                e.0.push(pair(
                    seed,
                    y.changeovers_started as f64,
                    x.changeovers_started as f64,
                ));
                e.1.push(pair(
                    seed,
                    y.output_transfers as f64,
                    x.output_transfers as f64,
                ));
                e.2.push(pair(
                    seed,
                    y.out_of_order_output_transfers as f64,
                    x.out_of_order_output_transfers as f64,
                ));
            }
        }
        let per_variant_wait_deltas = waits
            .into_iter()
            .map(|(variant, (a, b, c))| {
                Ok(WaitDelta {
                    variant,
                    average_wait_delta_seconds: summary_optional(a)?,
                    p95_wait_delta_seconds: summary_optional(b)?,
                    maximum_wait_delta_seconds: summary_optional(c)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        scenario_deltas.push(ScenarioDelta {
            scenario_id: scenario.clone(),
            good_units_per_hour_delta: summarize(throughput)?,
            good_units_total_delta: summarize(good)?,
            scrapped_jobs_delta: summarize(scrap)?,
            rework_attempts_delta: summarize(rework)?,
            per_variant_wait_deltas,
        });
        for (station_id, (a, b, c)) in stations {
            station_deltas.push(StationDelta {
                scenario_id: scenario.clone(),
                station_id,
                changeovers_delta: summarize(a)?,
                output_transfers_delta: summarize(b)?,
                out_of_order_output_transfers_delta: summarize(c)?,
            });
        }
    }
    Ok(ThresholdResult {
        maximum_queue_age_seconds: age,
        scenario_deltas,
        station_deltas,
    })
}
fn variant_map(r: &Record) -> Result<BTreeMap<usize, &VariantWait>, String> {
    let mut m = BTreeMap::new();
    for v in &r.variant_wait_statistics {
        if m.insert(v.variant, v).is_some() {
            return Err("duplicate_variant".into());
        }
    }
    Ok(m)
}
fn station_map(r: &Record) -> Result<BTreeMap<String, &Station>, String> {
    let mut m = BTreeMap::new();
    for s in &r.stations {
        if m.insert(s.station_id.clone(), s).is_some() {
            return Err("duplicate_station".into());
        }
    }
    Ok(m)
}
fn pair(seed: u64, candidate: f64, baseline: f64) -> Pair {
    Pair {
        seed,
        delta: candidate - baseline,
    }
}
fn optional_pair(seed: u64, a: Option<f64>, b: Option<f64>) -> Result<Option<Pair>, String> {
    match (a, b) {
        (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Ok(Some(pair(seed, x, y))),
        (None, None) => Ok(None),
        _ => Err("optional_metric_missing_or_non_finite_mismatch".into()),
    }
}
fn summary_optional(xs: Vec<Option<Pair>>) -> Result<Option<Summary>, String> {
    if xs.iter().all(Option::is_none) {
        return Ok(None);
    }
    summarize(
        xs.into_iter()
            .map(|x| x.ok_or("incomplete_optional_metric"))
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map(Some)
}
fn summarize(xs: Vec<Pair>) -> Result<Summary, String> {
    if xs.len() != 20 || xs.iter().any(|x| !x.delta.is_finite()) {
        return Err("paired_metric_requires_twenty_finite_deltas".into());
    }
    let n = xs.len() as f64;
    let mean = xs.iter().map(|x| x.delta).sum::<f64>() / n;
    let var = xs.iter().map(|x| (x.delta - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let sd = var.sqrt();
    let h = T_CRITICAL_DF_19 * sd / n.sqrt();
    if !mean.is_finite() || !sd.is_finite() || !h.is_finite() {
        return Err("non_finite_summary".into());
    }
    Ok(Summary {
        sample_count: xs.len(),
        mean,
        sample_standard_deviation: sd,
        confidence_95_lower_unadjusted: mean - h,
        confidence_95_upper_unadjusted: mean + h,
        paired_seed_deltas: xs,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn twenty_constant_pairs_have_zero_width_interval_and_retain_seeds() {
        let xs = (1..=20).map(|seed| Pair { seed, delta: 2.0 }).collect();
        let s = summarize(xs).expect("valid pairs");
        assert_eq!(s.sample_count, 20);
        assert_eq!(s.mean, 2.0);
        assert_eq!(s.sample_standard_deviation, 0.0);
        assert_eq!(s.confidence_95_lower_unadjusted, 2.0);
        assert_eq!(s.confidence_95_upper_unadjusted, 2.0);
        assert_eq!(s.paired_seed_deltas.len(), 20);
    }
    fn synthetic_summary(offset: f64) -> Summary {
        summarize(
            (1..=20)
                .map(|seed| Pair {
                    seed,
                    delta: offset + (seed as f64) / 7.0,
                })
                .collect(),
        )
        .expect("twenty finite pairs")
    }

    #[test]
    fn bonferroni_primary_family_covers_all_24_throughput_comparisons() {
        let results = THRESHOLDS
            .iter()
            .map(|age| ThresholdResult {
                maximum_queue_age_seconds: *age,
                scenario_deltas: (0..6)
                    .map(|scenario| ScenarioDelta {
                        scenario_id: format!("scenario-{scenario}"),
                        good_units_per_hour_delta: synthetic_summary(scenario as f64),
                        good_units_total_delta: synthetic_summary(0.0),
                        scrapped_jobs_delta: synthetic_summary(0.0),
                        rework_attempts_delta: synthetic_summary(0.0),
                        per_variant_wait_deltas: Vec::new(),
                    })
                    .collect(),
                station_deltas: Vec::new(),
            })
            .collect::<Vec<_>>();
        let intervals = build_primary_familywise_intervals(&results, PRIMARY_COMPARISON_COUNT)
            .expect("exactly 4 x 6 primary comparisons");
        assert_eq!(intervals.len(), 24);
        assert!(intervals.iter().all(|x| {
            x.family_size == 24
                && x.sample_count == 20
                && x.t_critical_conservative == 4.0
        }));
        let unadjusted = &results[0].scenario_deltas[0].good_units_per_hour_delta;
        assert!(intervals[0].confidence_95_lower_bonferroni <= unadjusted.confidence_95_lower_unadjusted);
        assert!(intervals[0].confidence_95_upper_bonferroni >= unadjusted.confidence_95_upper_unadjusted);
        assert!(T_CRITICAL_DF_19_BONFERRONI_24_CONSERVATIVE > T_CRITICAL_DF_19);
        assert!(build_primary_familywise_intervals(&results[..3], PRIMARY_COMPARISON_COUNT).is_err());
        assert!(build_primary_familywise_intervals(&results, PRIMARY_COMPARISON_COUNT - 1).is_err());
    }

    #[test]
    fn summary_rejects_nineteen_pairs() {
        let xs = (1..=19)
            .map(|seed| Pair {
                seed,
                delta: seed as f64,
            })
            .collect();
        assert!(summarize(xs).is_err());
    }
}
