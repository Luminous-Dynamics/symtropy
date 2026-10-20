use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process::ExitCode};

const DAY_SECONDS: u64 = 86_399;
const SECONDS_PER_BIN: u64 = 900;
const BINS_PER_DAY: usize = 96;
const REPEATED_DAYS: u64 = 3;
const CLAIM_CEILING: &str = "NLR-style reference-profile reimplementation using published operational summary rows and explicit kWh/mile, fixed-power charging, sampling, and shift-access assumptions. This is not a bit-identical reproduction of Python/Pandas random draws, not a measured EV charging profile, not an AC grid/voltage model, and not evidence of South African depot performance.";

#[derive(Debug, Deserialize)]
struct AuditReport {
    schema_version: String,
    tool: String,
    fleet_id: String,
    dataset_id: String,
    source_uri: String,
    source_revision: String,
    evidence_class_declared_by_caller: String,
    operation_days_input_rows: usize,
    operation_days_parsed_valid_rows: usize,
    operation_days_invalid_rows: Vec<serde_json::Value>,
    schedule_input_rows: usize,
    schedule_parsed_valid_rows_with_known_operation_day_ids: usize,
    schedule_invalid_rows: Vec<serde_json::Value>,
    reconciled_operation_day_count: usize,
    operation_day_reconciliation_issues: Vec<serde_json::Value>,
    reconciled_operation_day_profiles: Vec<DutyDaySummary>,
    reconciled_operation_day_segments: Vec<DutySegment>,
    operation_day_csv_sha256: String,
    schedule_csv_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DutyDaySummary {
    operation_day_id: u64,
    vmt_miles: f64,
    on_shift_seconds: u32,
    off_shift_seconds: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct DutySegment {
    operation_day_id: u64,
    start_second: u32,
    end_second: u32,
    duration_seconds: u32,
    on_shift: bool,
    vmt_miles: f64,
}

#[derive(Debug, Clone)]
struct DutyDay {
    summary: DutyDaySummary,
    segments: Vec<DutySegment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Strategy {
    Immediate,
    Delayed,
    MinimumPower,
}

impl Strategy {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "immediate" => Ok(Self::Immediate),
            "delayed" => Ok(Self::Delayed),
            "min_power" => Ok(Self::MinimumPower),
            _ => Err("strategy_must_be_immediate_delayed_or_min_power".to_string()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::Delayed => "delayed",
            Self::MinimumPower => "min_power",
        }
    }
}

#[derive(Debug)]
struct Options {
    input_report: String,
    fleet_size: usize,
    strategy: Strategy,
    sample_count: usize,
    seed: u64,
    kwh_per_mile: f64,
    charger_power_kw: f64,
}

#[derive(Debug, Clone, Copy)]
struct ChargeInterval {
    start_second: u64,
    end_second: u64,
    power_kw: f64,
}

#[derive(Debug, Clone, Serialize)]
struct Distribution {
    count: usize,
    min: Option<f64>,
    p50: Option<f64>,
    p95: Option<f64>,
    max: Option<f64>,
    mean: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ProfilePoint {
    interval_index: usize,
    start_minute: u32,
    end_minute: u32,
    average_power_kw: f64,
}

#[derive(Debug, Serialize)]
struct SampleMetric {
    sample_index: usize,
    operation_day_ids: Vec<u64>,
    requested_battery_energy_kwh: f64,
    grid_energy_delivered_kwh: f64,
    profile_energy_15min_integral_kwh: f64,
    energy_reconciliation_residual_kwh: f64,
    energy_service_fraction: f64,
    instantaneous_peak_kw: f64,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    method: String,
    source_dataset_id: String,
    source_fleet_id: String,
    source_uri: String,
    source_revision: String,
    source_evidence_class_declared_by_caller: String,
    operation_day_csv_sha256: String,
    schedule_csv_sha256: String,
    profile_evidence_class: String,
    strategy: String,
    fleet_size: usize,
    sample_count: usize,
    seed: u64,
    sampling_method: String,
    kwh_per_mile_assumption: f64,
    fixed_charger_power_kw: Option<f64>,
    profile_resolution_minutes: u32,
    profile_intervals_per_day: usize,
    daily_requested_energy_kwh: Distribution,
    daily_grid_energy_delivered_kwh: Distribution,
    daily_energy_service_fraction: Distribution,
    instantaneous_peak_kw: Distribution,
    daily_minimum_peak_profile: Vec<ProfilePoint>,
    daily_average_profile: Vec<ProfilePoint>,
    daily_maximum_peak_profile: Vec<ProfilePoint>,
    samples: Vec<SampleMetric>,
    claim_ceiling: &'static str,
}

fn sha256_shape(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    if args.len() < 2 {
        return Err("usage: ev-nlr-profile-baseline --input-report audit.json --fleet-size N --strategy immediate|delayed|min_power --samples N --seed N --kwh-per-mile X --charger-power-kw X".to_string());
    }
    let allowed = [
        "--input-report",
        "--fleet-size",
        "--strategy",
        "--samples",
        "--seed",
        "--kwh-per-mile",
        "--charger-power-kw",
    ];
    let mut values = BTreeMap::<String, String>::new();
    let mut index = 1;
    while index < args.len() {
        let key = args[index].as_str();
        if !allowed.contains(&key) {
            return Err(format!("unknown_argument:{key}"));
        }
        if values.contains_key(key) {
            return Err(format!("duplicate_argument:{key}"));
        }
        let value = args.get(index + 1).ok_or_else(|| format!("{key}_requires_a_value"))?;
        values.insert(key.to_string(), value.clone());
        index += 2;
    }
    for key in allowed {
        if !values.contains_key(key) {
            return Err(format!("{key}_required"));
        }
    }
    let number = |key: &str| -> Result<f64, String> {
        values[key].parse::<f64>().map_err(|_| format!("{key}_must_be_numeric"))
    };
    let usize_value = |key: &str| -> Result<usize, String> {
        values[key].parse::<usize>().map_err(|_| format!("{key}_must_be_a_positive_integer"))
    };
    let seed = values["--seed"].parse::<u64>().map_err(|_| "--seed_must_be_a_nonnegative_integer".to_string())?;
    let options = Options {
        input_report: values["--input-report"].clone(),
        fleet_size: usize_value("--fleet-size")?,
        strategy: Strategy::parse(&values["--strategy"] )?,
        sample_count: usize_value("--samples")?,
        seed,
        kwh_per_mile: number("--kwh-per-mile")?,
        charger_power_kw: number("--charger-power-kw")?,
    };
    if options.fleet_size == 0 || options.sample_count == 0 {
        return Err("fleet_size_and_samples_must_be_positive".to_string());
    }
    if !options.kwh_per_mile.is_finite() || options.kwh_per_mile <= 0.0 {
        return Err("--kwh-per-mile_must_be_finite_and_positive".to_string());
    }
    if !options.charger_power_kw.is_finite() || options.charger_power_kw <= 0.0 {
        return Err("--charger-power-kw_must_be_finite_and_positive".to_string());
    }
    Ok(options)
}

fn validate_and_group_report(report: &AuditReport) -> Result<Vec<DutyDay>, String> {
    if report.schema_version != "0.1.0" || report.tool != "ev-depot-duty-audit" {
        return Err("unsupported_duty_audit_schema_or_tool".to_string());
    }
    if report.fleet_id.trim().is_empty()
        || report.dataset_id.trim().is_empty()
        || !report.source_uri.starts_with("https://")
        || report.source_revision.trim().is_empty()
        || report.evidence_class_declared_by_caller.trim().is_empty()
    {
        return Err("source_identity_and_evidence_class_are_required".to_string());
    }
    if !sha256_shape(&report.operation_day_csv_sha256) || !sha256_shape(&report.schedule_csv_sha256) {
        return Err("source_csv_sha256_must_be_64_hex_characters".to_string());
    }
    if report.operation_days_input_rows != report.operation_days_parsed_valid_rows
        || !report.operation_days_invalid_rows.is_empty()
        || report.schedule_input_rows != report.schedule_parsed_valid_rows_with_known_operation_day_ids
        || !report.schedule_invalid_rows.is_empty()
        || !report.operation_day_reconciliation_issues.is_empty()
        || report.reconciled_operation_day_count != report.reconciled_operation_day_profiles.len()
    {
        return Err("audit_report_contains_invalid_rows_or_unreconciled_days".to_string());
    }
    if report.reconciled_operation_day_segments.len() != report.schedule_input_rows {
        return Err("reconciled_segment_count_does_not_match_source_schedule_row_count".to_string());
    }

    let mut profile_ids = BTreeSet::new();
    let mut grouped = BTreeMap::<u64, Vec<DutySegment>>::new();
    for segment in &report.reconciled_operation_day_segments {
        if segment.operation_day_id == 0
            || segment.start_second >= DAY_SECONDS as u32
            || segment.end_second > DAY_SECONDS as u32
            || segment.end_second <= segment.start_second
            || segment.duration_seconds != segment.end_second - segment.start_second
            || !segment.vmt_miles.is_finite()
            || segment.vmt_miles < 0.0
            || (!segment.on_shift && segment.vmt_miles > 1.0e-9)
        {
            return Err(format!("invalid_reconciled_segment_for_day_{}", segment.operation_day_id));
        }
        grouped.entry(segment.operation_day_id).or_default().push(segment.clone());
    }

    let mut days = Vec::with_capacity(report.reconciled_operation_day_profiles.len());
    for summary in &report.reconciled_operation_day_profiles {
        if summary.operation_day_id == 0 || !profile_ids.insert(summary.operation_day_id) {
            return Err("empty_or_duplicate_operation_day_profile_id".to_string());
        }
        if !summary.vmt_miles.is_finite()
            || summary.vmt_miles < 0.0
            || u64::from(summary.on_shift_seconds) + u64::from(summary.off_shift_seconds) != DAY_SECONDS
        {
            return Err(format!("invalid_operation_day_summary_{}", summary.operation_day_id));
        }
        let mut segments = grouped.remove(&summary.operation_day_id).unwrap_or_default();
        if segments.is_empty() {
            return Err(format!("no_reconciled_segments_for_day_{}", summary.operation_day_id));
        }
        segments.sort_by_key(|s| (s.start_second, s.end_second));
        let mut cursor = 0_u32;
        let mut on_seconds = 0_u32;
        let mut off_seconds = 0_u32;
        let mut vmt = 0.0;
        for segment in &segments {
            if segment.start_second != cursor {
                return Err(format!("gap_or_overlap_in_day_{}", summary.operation_day_id));
            }
            cursor = segment.end_second;
            if segment.on_shift {
                on_seconds = on_seconds.saturating_add(segment.duration_seconds);
                vmt += segment.vmt_miles;
            } else {
                off_seconds = off_seconds.saturating_add(segment.duration_seconds);
            }
        }
        if cursor != DAY_SECONDS as u32
            || on_seconds.abs_diff(summary.on_shift_seconds) > 1
            || off_seconds.abs_diff(summary.off_shift_seconds) > 1
            || (vmt - summary.vmt_miles).abs() > 1.0e-6_f64.max(summary.vmt_miles.abs() * 1.0e-7)
        {
            return Err(format!("segment_reconciliation_failed_for_day_{}", summary.operation_day_id));
        }
        if off_seconds == 0 {
            return Err(format!("no_offshift_seconds_for_day_{}", summary.operation_day_id));
        }
        days.push(DutyDay { summary: summary.clone(), segments });
    }
    if !grouped.is_empty() {
        return Err("segments_reference_unknown_operation_day_id".to_string());
    }
    if days.is_empty() {
        return Err("audit_report_has_no_eligible_operation_days".to_string());
    }
    Ok(days)
}

fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    *state
}

fn vehicle_charge_intervals(
    day: &DutyDay,
    strategy: Strategy,
    kwh_per_mile: f64,
    fixed_power_kw: f64,
) -> Result<Vec<ChargeInterval>, String> {
    let mut intervals = Vec::new();
    if strategy == Strategy::MinimumPower {
        let total_energy_kwh = day.summary.vmt_miles * kwh_per_mile;
        let offshift_seconds = f64::from(day.summary.off_shift_seconds);
        let power_kw = total_energy_kwh * 3600.0 / offshift_seconds;
        if !power_kw.is_finite() || power_kw < 0.0 {
            return Err(format!("invalid_minimum_power_for_day_{}", day.summary.operation_day_id));
        }
        for repeat in 0..REPEATED_DAYS {
            let offset = repeat * DAY_SECONDS;
            for segment in &day.segments {
                if !segment.on_shift && segment.duration_seconds > 0 {
                    intervals.push(ChargeInterval {
                        start_second: offset + u64::from(segment.start_second),
                        end_second: offset + u64::from(segment.end_second),
                        power_kw,
                    });
                }
            }
        }
        return Ok(intervals);
    }

    let mut net_energy_consumed_kwh = 0.0_f64;
    for repeat in 0..REPEATED_DAYS {
        let offset = repeat * DAY_SECONDS;
        for segment in &day.segments {
            if segment.on_shift {
                net_energy_consumed_kwh += segment.vmt_miles * kwh_per_mile;
                continue;
            }
            let dwell_seconds = u64::from(segment.duration_seconds);
            let requested_charge_seconds = (net_energy_consumed_kwh / fixed_power_kw * 3600.0).ceil();
            if !requested_charge_seconds.is_finite() || requested_charge_seconds > (REPEATED_DAYS * DAY_SECONDS) as f64 {
                return Err(format!("required_charging_time_out_of_range_for_day_{}", day.summary.operation_day_id));
            }
            let requested_charge_seconds = requested_charge_seconds.max(0.0) as u64;
            if requested_charge_seconds > dwell_seconds && dwell_seconds > 0 {
                intervals.push(ChargeInterval {
                    start_second: offset + u64::from(segment.start_second),
                    end_second: offset + u64::from(segment.end_second),
                    power_kw: fixed_power_kw,
                });
                net_energy_consumed_kwh =
                    (net_energy_consumed_kwh - fixed_power_kw * dwell_seconds as f64 / 3600.0).max(0.0);
            } else {
                net_energy_consumed_kwh = 0.0;
                if requested_charge_seconds > 0 {
                    let start = u64::from(segment.start_second);
                    let end = u64::from(segment.end_second);
                    let (charge_start, charge_end) = match strategy {
                        Strategy::Immediate => (start, start + requested_charge_seconds),
                        Strategy::Delayed => (end - requested_charge_seconds, end),
                        Strategy::MinimumPower => unreachable!(),
                    };
                    intervals.push(ChargeInterval {
                        start_second: offset + charge_start,
                        end_second: offset + charge_end,
                        power_kw: fixed_power_kw,
                    });
                }
            }
        }
    }
    Ok(intervals)
}

fn clip_to_middle_day(intervals: &[ChargeInterval]) -> Vec<ChargeInterval> {
    let day_start = DAY_SECONDS;
    let day_end = DAY_SECONDS * 2;
    intervals.iter().filter_map(|interval| {
        let start = interval.start_second.max(day_start);
        let end = interval.end_second.min(day_end);
        if start >= end {
            None
        } else {
            Some(ChargeInterval {
                start_second: start - day_start,
                end_second: end - day_start,
                power_kw: interval.power_kw,
            })
        }
    }).collect()
}

fn profile_from_intervals(intervals: &[ChargeInterval]) -> (Vec<f64>, f64) {
    let mut bin_kw_seconds = vec![0.0_f64; BINS_PER_DAY];
    let mut events = Vec::<(u64, f64)>::with_capacity(intervals.len() * 2);
    let mut grid_energy_kwh = 0.0_f64;
    for interval in intervals {
        if interval.start_second >= interval.end_second || interval.power_kw <= 0.0 {
            continue;
        }
        grid_energy_kwh += interval.power_kw * (interval.end_second - interval.start_second) as f64 / 3600.0;
        events.push((interval.start_second, interval.power_kw));
        events.push((interval.end_second, -interval.power_kw));
        let first_bin = (interval.start_second / SECONDS_PER_BIN) as usize;
        let last_bin = ((interval.end_second - 1) / SECONDS_PER_BIN) as usize;
        for bin in first_bin..=last_bin {
            let bin_start = bin as u64 * SECONDS_PER_BIN;
            let bin_end = bin_start + SECONDS_PER_BIN;
            let overlap_start = interval.start_second.max(bin_start);
            let overlap_end = interval.end_second.min(bin_end);
            if overlap_start < overlap_end {
                bin_kw_seconds[bin] += interval.power_kw * (overlap_end - overlap_start) as f64;
            }
        }
    }
    // NLR's exported 24-hour curve appends the first second of its 86,399-second day
    // to the end so the CSV contains 86,400 samples. Preserve that wrap convention.
    let first_second_power: f64 = intervals.iter()
        .filter(|i| i.start_second == 0 && i.end_second > 0)
        .map(|i| i.power_kw)
        .sum();
    if first_second_power > 0.0 {
        bin_kw_seconds[BINS_PER_DAY - 1] += first_second_power;
    }
    events.sort_by(|a, b| a.0.cmp(&b.0));
    let mut current = 0.0_f64;
    let mut peak = 0.0_f64;
    let mut index = 0;
    while index < events.len() {
        let time = events[index].0;
        let mut delta = 0.0_f64;
        while index < events.len() && events[index].0 == time {
            delta += events[index].1;
            index += 1;
        }
        current += delta;
        peak = peak.max(current);
    }
    (
        bin_kw_seconds.into_iter().map(|kw_seconds| kw_seconds / SECONDS_PER_BIN as f64).collect(),
        if grid_energy_kwh.is_finite() { peak } else { 0.0 },
    )
}

fn distribution(values: &[f64]) -> Distribution {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len();
    let quantile = |p: f64| {
        if sorted.is_empty() {
            None
        } else {
            let index = ((p * sorted.len() as f64).ceil() as usize).max(1).min(sorted.len()) - 1;
            Some(sorted[index])
        }
    };
    Distribution {
        count,
        min: sorted.first().copied(),
        p50: quantile(0.50),
        p95: quantile(0.95),
        max: sorted.last().copied(),
        mean: if sorted.is_empty() { None } else { Some(sorted.iter().sum::<f64>() / sorted.len() as f64) },
    }
}

fn profile_points(values: &[f64]) -> Vec<ProfilePoint> {
    values.iter().enumerate().map(|(index, value)| ProfilePoint {
        interval_index: index,
        start_minute: (index as u32) * 15,
        end_minute: ((index as u32) + 1) * 15,
        average_power_kw: *value,
    }).collect()
}

fn simulate_sample(
    days: &[DutyDay],
    options: &Options,
    rng: &mut u64,
    sample_index: usize,
) -> Result<(Vec<f64>, SampleMetric), String> {
    let mut intervals = Vec::<ChargeInterval>::new();
    let mut operation_day_ids = Vec::with_capacity(options.fleet_size);
    let mut requested_energy = 0.0_f64;
    for _ in 0..options.fleet_size {
        let selected = (next_random(rng) % days.len() as u64) as usize;
        let day = &days[selected];
        operation_day_ids.push(day.summary.operation_day_id);
        requested_energy += day.summary.vmt_miles * options.kwh_per_mile;
        intervals.extend(vehicle_charge_intervals(
            day,
            options.strategy,
            options.kwh_per_mile,
            options.charger_power_kw,
        )?);
    }
    let clipped = clip_to_middle_day(&intervals);
    let (mut profile, peak) = profile_from_intervals(&clipped);
    let delivered: f64 = clipped.iter().map(|interval| {
        interval.power_kw * (interval.end_second - interval.start_second) as f64 / 3600.0
    }).sum();
    let profile_energy: f64 = profile.iter().sum::<f64>() * 0.25;
    // The 15-minute profile's final second wraps the first second; this small output
    // convention difference is kept separate from grid energy delivered in 86,399s.
    let residual = profile_energy - delivered;
    if profile.len() != BINS_PER_DAY {
        return Err("internal_profile_bin_count_mismatch".to_string());
    }
    let service = if requested_energy <= 1.0e-9 { 1.0 } else { (delivered / requested_energy).clamp(0.0, 2.0) };
    Ok((profile.drain(..).collect(), SampleMetric {
        sample_index,
        operation_day_ids,
        requested_battery_energy_kwh: requested_energy,
        grid_energy_delivered_kwh: delivered,
        profile_energy_15min_integral_kwh: profile_energy,
        energy_reconciliation_residual_kwh: residual,
        energy_service_fraction: service,
        instantaneous_peak_kw: peak,
    }))
}

fn generate_report(report: &AuditReport, days: &[DutyDay], options: &Options) -> Result<Report, String> {
    let mut rng = options.seed;
    let mut sample_metrics = Vec::with_capacity(options.sample_count);
    let mut summed_profile = vec![0.0_f64; BINS_PER_DAY];
    let mut min_peak = f64::INFINITY;
    let mut max_peak = f64::NEG_INFINITY;
    let mut min_profile = vec![0.0_f64; BINS_PER_DAY];
    let mut max_profile = vec![0.0_f64; BINS_PER_DAY];
    let mut requested = Vec::with_capacity(options.sample_count);
    let mut delivered = Vec::with_capacity(options.sample_count);
    let mut service = Vec::with_capacity(options.sample_count);
    let mut peaks = Vec::with_capacity(options.sample_count);

    for sample_index in 0..options.sample_count {
        let (profile, metric) = simulate_sample(days, options, &mut rng, sample_index)?;
        for (sum, value) in summed_profile.iter_mut().zip(&profile) {
            *sum += *value;
        }
        if metric.instantaneous_peak_kw < min_peak {
            min_peak = metric.instantaneous_peak_kw;
            min_profile.clone_from(&profile);
        }
        if metric.instantaneous_peak_kw > max_peak {
            max_peak = metric.instantaneous_peak_kw;
            max_profile.clone_from(&profile);
        }
        requested.push(metric.requested_battery_energy_kwh);
        delivered.push(metric.grid_energy_delivered_kwh);
        service.push(metric.energy_service_fraction);
        peaks.push(metric.instantaneous_peak_kw);
        sample_metrics.push(metric);
    }
    for value in &mut summed_profile {
        *value /= options.sample_count as f64;
    }
    Ok(Report {
        schema_version: "0.1.0".to_string(),
        benchmark_id: "EV-NLR-REFERENCE-PROFILES-001".to_string(),
        method: "NLR strategy semantics over three repeated daily duty schedules with the middle 86,399-second day selected, followed by NLR-style first-second wrap to 24 hours and 15-minute averaging. Source profile resampling uses the declared deterministic LCG rather than Python/Pandas draws, so outputs are not expected to be byte-identical to NLR's original random sampler.".to_string(),
        source_dataset_id: report.dataset_id.clone(),
        source_fleet_id: report.fleet_id.clone(),
        source_uri: report.source_uri.clone(),
        source_revision: report.source_revision.clone(),
        source_evidence_class_declared_by_caller: report.evidence_class_declared_by_caller.clone(),
        operation_day_csv_sha256: report.operation_day_csv_sha256.clone(),
        schedule_csv_sha256: report.schedule_csv_sha256.clone(),
        profile_evidence_class: "model_reimplementation".to_string(),
        strategy: options.strategy.as_str().to_string(),
        fleet_size: options.fleet_size,
        sample_count: options.sample_count,
        seed: options.seed,
        sampling_method: "linear-congruential deterministic sampling with replacement; not Python random or pandas DataFrame.sample".to_string(),
        kwh_per_mile_assumption: options.kwh_per_mile,
        fixed_charger_power_kw: if options.strategy == Strategy::MinimumPower { None } else { Some(options.charger_power_kw) },
        profile_resolution_minutes: 15,
        profile_intervals_per_day: BINS_PER_DAY,
        daily_requested_energy_kwh: distribution(&requested),
        daily_grid_energy_delivered_kwh: distribution(&delivered),
        daily_energy_service_fraction: distribution(&service),
        instantaneous_peak_kw: distribution(&peaks),
        daily_minimum_peak_profile: profile_points(&min_profile),
        daily_average_profile: profile_points(&summed_profile),
        daily_maximum_peak_profile: profile_points(&max_profile),
        samples: sample_metrics,
        claim_ceiling: CLAIM_CEILING,
    })
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let options = parse_args(&args)?;
    let report_bytes = fs::read(&options.input_report)
        .map_err(|error| format!("cannot_read_audit_report:{}:{error}", options.input_report))?;
    let report: AuditReport = serde_json::from_slice(&report_bytes)
        .map_err(|error| format!("invalid_audit_report_json:{error}"))?;
    let days = validate_and_group_report(&report)?;
    let result = generate_report(&report, &days, &options)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result)
            .map_err(|error| format!("cannot_serialize_reference_profile_report:{error}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ev-nlr-profile-baseline: {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_audit_report() -> AuditReport {
        AuditReport {
            schema_version: "0.1.0".into(),
            tool: "ev-depot-duty-audit".into(),
            fleet_id: "synthetic-fleet".into(),
            dataset_id: "synthetic-nlr-style-fixture".into(),
            source_uri: "https://example.invalid/dataset".into(),
            source_revision: "fixture-v0.1".into(),
            evidence_class_declared_by_caller: "synthetic".into(),
            operation_days_input_rows: 1,
            operation_days_parsed_valid_rows: 1,
            operation_days_invalid_rows: vec![],
            schedule_input_rows: 3,
            schedule_parsed_valid_rows_with_known_operation_day_ids: 3,
            schedule_invalid_rows: vec![],
            reconciled_operation_day_count: 1,
            operation_day_reconciliation_issues: vec![],
            reconciled_operation_day_profiles: vec![DutyDaySummary {
                operation_day_id: 1,
                vmt_miles: 10.0,
                on_shift_seconds: 3600,
                off_shift_seconds: 82_799,
            }],
            reconciled_operation_day_segments: vec![
                DutySegment {operation_day_id:1,start_second:0,end_second:28_800,duration_seconds:28_800,on_shift:false,vmt_miles:0.0},
                DutySegment {operation_day_id:1,start_second:28_800,end_second:32_400,duration_seconds:3600,on_shift:true,vmt_miles:10.0},
                DutySegment {operation_day_id:1,start_second:32_400,end_second:86_399,duration_seconds:53_999,on_shift:false,vmt_miles:0.0},
            ],
            operation_day_csv_sha256: "a".repeat(64),
            schedule_csv_sha256: "b".repeat(64),
        }
    }

    fn options(strategy: Strategy) -> Options {
        Options {
            input_report: "unused".to_string(),
            fleet_size: 1,
            strategy,
            sample_count: 2,
            seed: 17,
            kwh_per_mile: 1.8,
            charger_power_kw: 100.0,
        }
    }

    #[test]
    fn nlr_immediate_and_delayed_profiles_shift_charge_within_the_same_offshift_window() {
        let report = sample_audit_report();
        let days = validate_and_group_report(&report).unwrap();
        let (immediate, immediate_metric) = simulate_sample(&days, &options(Strategy::Immediate), &mut 3, 0).unwrap();
        let (delayed, delayed_metric) = simulate_sample(&days, &options(Strategy::Delayed), &mut 3, 0).unwrap();
        assert_eq!(immediate.len(), 96);
        assert_eq!(delayed.len(), 96);
        assert!((immediate_metric.grid_energy_delivered_kwh - delayed_metric.grid_energy_delivered_kwh).abs() < 1.0e-7);
        assert!(
            immediate[..48].iter().sum::<f64>() > delayed[..48].iter().sum::<f64>(),
            "immediate should put the overnight charge at the start of the dwell; delayed should put it at the end"
        );
    }

    #[test]
    fn minimum_power_uses_all_declared_offshift_intervals_and_reconciles_energy() {
        let report = sample_audit_report();
        let days = validate_and_group_report(&report).unwrap();
        let (_, metric) = simulate_sample(&days, &options(Strategy::MinimumPower), &mut 3, 0).unwrap();
        assert!((metric.requested_battery_energy_kwh - 18.0).abs() < 1.0e-7);
        assert!((metric.grid_energy_delivered_kwh - 18.0).abs() < 1.0e-7);
        assert!(metric.instantaneous_peak_kw > 0.0);
    }

    #[test]
    fn service_and_peak_are_preserved_in_the_summary() {
        let report = sample_audit_report();
        let days = validate_and_group_report(&report).unwrap();
        let result = generate_report(&report, &days, &options(Strategy::MinimumPower)).unwrap();
        assert_eq!(result.profile_intervals_per_day, 96);
        assert_eq!(result.samples.len(), 2);
        assert_eq!(result.daily_average_profile.len(), 96);
        assert_eq!(result.daily_minimum_peak_profile.len(), 96);
        assert_eq!(result.daily_maximum_peak_profile.len(), 96);
    }

    #[test]
    fn reports_with_unreconciled_rows_or_missing_segment_data_fail_closed() {
        let mut report = sample_audit_report();
        report.schedule_invalid_rows.push(serde_json::json!({"row": 3}));
        assert!(validate_and_group_report(&report).unwrap_err().contains("invalid_rows"));
        let mut report = sample_audit_report();
        report.reconciled_operation_day_segments.clear();
        assert!(validate_and_group_report(&report).unwrap_err().contains("segment_count"));
    }
}
