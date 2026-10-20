use csv::{ReaderBuilder, StringRecord, Trim};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs, process::ExitCode};

const DAY_SECONDS: u32 = 86_399;
const CLAIM_CEILING: &str = "Audit and workload characterization of public operational-summary CSVs. Source provenance/evidence class is declared by the caller. Idealized charge power assumes all reported off-shift seconds are chargeable at the depot. This is not a charge scheduler, observed charging outcome, grid-feasibility result, or evidence of South African fleet performance.";

#[derive(Debug)]
struct Metadata {
    fleet_id: String,
    dataset_id: String,
    source_uri: String,
    source_revision: String,
    evidence_class: String,
    kwh_per_mile: f64,
}

#[derive(Debug, Clone)]
struct OpDay {
    id: u64,
    vmt: f64,
    on_shift_seconds: u32,
    off_shift_seconds: u32,
}

#[derive(Debug, Clone)]
struct Segment {
    row_index: usize,
    operation_day_id: u64,
    start_second: u32,
    end_second: u32,
    total_seconds: u32,
    on_shift: bool,
    vmt: f64,
}

#[derive(Debug, Serialize)]
struct Distribution {
    count: usize,
    min: Option<f64>,
    p50: Option<f64>,
    p95: Option<f64>,
    max: Option<f64>,
    mean: Option<f64>,
}

#[derive(Debug, Serialize)]
struct InvalidRow {
    row_index_zero_based: usize,
    reason: String,
}

#[derive(Debug, Serialize)]
struct ReconciledOperationDayProfile {
    operation_day_id: u64,
    vmt_miles: f64,
    on_shift_seconds: u32,
    off_shift_seconds: u32,
    first_on_shift_start_second: Option<u32>,
    last_on_shift_end_second: Option<u32>,
}

#[derive(Debug, Serialize)]
struct DayIssue {
    operation_day_id: u64,
    reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    tool: String,
    fleet_id: String,
    dataset_id: String,
    source_uri: String,
    source_revision: String,
    evidence_class_declared_by_caller: String,
    kwh_per_mile_assumption: f64,
    expected_day_seconds: u32,
    operation_days_input_rows: usize,
    operation_days_parsed_valid_rows: usize,
    operation_days_invalid_rows: Vec<InvalidRow>,
    schedule_input_rows: usize,
    schedule_parsed_valid_rows_with_known_operation_day_ids: usize,
    schedule_invalid_rows: Vec<InvalidRow>,
    reconciled_operation_day_count: usize,
    operation_day_reconciliation_issues: Vec<DayIssue>,
    reconciled_operation_day_profiles: Vec<ReconciledOperationDayProfile>,
    schedule_segment_count_for_reconciled_days: usize,
    reconciled_vmt_miles_total_across_operation_day_records: f64,
    daily_vmt_miles: Distribution,
    daily_off_shift_hours: Distribution,
    idealized_min_average_recharge_power_kw_assuming_all_offshift_is_chargeable: Distribution,
    on_shift_seconds_total: u64,
    off_shift_seconds_total: u64,
    operation_days_without_offshift_charging_window: usize,
    operation_day_csv_sha256: String,
    schedule_csv_sha256: String,
    provenance_warning: String,
    claim_ceiling: String,
}

fn field<'a>(record: &'a StringRecord, index: usize, name: &str) -> Result<&'a str, String> {
    record
        .get(index)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name}_missing"))
}

fn column(headers: &StringRecord, name: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|header| header.trim() == name)
        .ok_or_else(|| format!("required_column_missing:{name}"))
}

fn finite_nonnegative(record: &StringRecord, index: usize, name: &str) -> Result<f64, String> {
    let text = field(record, index, name)?;
    let value: f64 = text.parse().map_err(|_| format!("{name}_not_numeric"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{name}_must_be_finite_and_nonnegative"));
    }
    Ok(value)
}

fn integer_u32(record: &StringRecord, index: usize, name: &str) -> Result<u32, String> {
    let text = field(record, index, name)?;
    text.parse::<u32>()
        .map_err(|_| format!("{name}_must_be_a_nonnegative_integer"))
}

fn operation_day_id(record: &StringRecord, index: usize) -> Result<u64, String> {
    let value = field(record, index, "veh_op_day_id")?
        .parse::<u64>()
        .map_err(|_| "veh_op_day_id_must_be_a_positive_integer".to_string())?;
    if value == 0 {
        return Err("veh_op_day_id_must_be_a_positive_integer".to_string());
    }
    Ok(value)
}

fn parse_clock(text: &str) -> Result<u32, String> {
    let pieces: Vec<&str> = text.split(':').collect();
    if pieces.len() != 3 {
        return Err("clock_must_use_HH:MM:SS".to_string());
    }
    let hour: u32 = pieces[0].parse().map_err(|_| "invalid_hour")?;
    let minute: u32 = pieces[1].parse().map_err(|_| "invalid_minute")?;
    let second: u32 = pieces[2].parse().map_err(|_| "invalid_second")?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err("clock_component_out_of_range".to_string());
    }
    Ok(hour * 3_600 + minute * 60 + second)
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn distribution(values: &[f64]) -> Distribution {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len();
    let p50 = if count == 0 {
        None
    } else if count % 2 == 1 {
        Some(sorted[count / 2])
    } else {
        Some((sorted[count / 2 - 1] + sorted[count / 2]) / 2.0)
    };
    let p95 = if count == 0 {
        None
    } else {
        Some(sorted[(95 * count).div_ceil(100).saturating_sub(1)])
    };
    Distribution {
        count,
        min: sorted.first().copied(),
        p50,
        p95,
        max: sorted.last().copied(),
        mean: if count == 0 {
            None
        } else {
            Some(sorted.iter().sum::<f64>() / count as f64)
        },
    }
}

fn audit(
    operation_bytes: &[u8],
    schedule_bytes: &[u8],
    metadata: Metadata,
) -> Result<Report, String> {
    if !metadata.kwh_per_mile.is_finite() || metadata.kwh_per_mile <= 0.0 {
        return Err("kwh_per_mile_must_be_finite_and_positive".to_string());
    }

    let mut op_reader = ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(operation_bytes);
    let op_headers = op_reader
        .headers()
        .map_err(|e| format!("operation_day_csv_headers:{e}"))?
        .clone();
    let op_id_col = column(&op_headers, "veh_op_day_id")?;
    let op_vmt_col = column(&op_headers, "vmt")?;
    let op_on_col = column(&op_headers, "time_on_shift_s")?;
    let op_off_col = column(&op_headers, "time_off_shift_s")?;
    let mut operation_days = BTreeMap::<u64, OpDay>::new();
    let mut invalid_op_rows = Vec::new();
    let mut op_rows = 0_usize;

    for (index, result) in op_reader.records().enumerate() {
        op_rows += 1;
        let record = match result {
            Ok(record) => record,
            Err(error) => {
                invalid_op_rows.push(InvalidRow {
                    row_index_zero_based: index,
                    reason: format!("csv_parse_error:{error}"),
                });
                continue;
            }
        };
        let parsed = (|| -> Result<OpDay, String> {
            let id = operation_day_id(&record, op_id_col)?;
            let vmt = finite_nonnegative(&record, op_vmt_col, "vmt")?;
            let on = integer_u32(&record, op_on_col, "time_on_shift_s")?;
            let off = integer_u32(&record, op_off_col, "time_off_shift_s")?;
            if on.saturating_add(off) != DAY_SECONDS {
                return Err(format!(
                    "on_plus_off_shift_seconds_must_equal_{DAY_SECONDS}"
                ));
            }
            Ok(OpDay {
                id,
                vmt,
                on_shift_seconds: on,
                off_shift_seconds: off,
            })
        })();
        match parsed {
            Ok(day) if operation_days.contains_key(&day.id) => invalid_op_rows.push(InvalidRow {
                row_index_zero_based: index,
                reason: format!("duplicate_operation_day_id:{}", day.id),
            }),
            Ok(day) => {
                operation_days.insert(day.id, day);
            }
            Err(reason) => invalid_op_rows.push(InvalidRow {
                row_index_zero_based: index,
                reason,
            }),
        }
    }
    if op_rows == 0 {
        return Err("operation_day_csv_has_no_data_rows".to_string());
    }

    let mut schedule_reader = ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(schedule_bytes);
    let schedule_headers = schedule_reader
        .headers()
        .map_err(|e| format!("schedule_csv_headers:{e}"))?
        .clone();
    let sc_id_col = column(&schedule_headers, "veh_op_day_id")?;
    let start_col = column(&schedule_headers, "start_time")?;
    let end_col = column(&schedule_headers, "end_time")?;
    let duration_col = column(&schedule_headers, "total_time_s")?;
    let on_col = column(&schedule_headers, "on_shift")?;
    let vmt_col = column(&schedule_headers, "vmt")?;
    let mut all_segments = Vec::<Segment>::new();
    let mut invalid_schedule_rows = Vec::new();
    let mut schedule_rows = 0_usize;

    for (index, result) in schedule_reader.records().enumerate() {
        schedule_rows += 1;
        let record = match result {
            Ok(record) => record,
            Err(error) => {
                invalid_schedule_rows.push(InvalidRow {
                    row_index_zero_based: index,
                    reason: format!("csv_parse_error:{error}"),
                });
                continue;
            }
        };
        let parsed = (|| -> Result<Segment, String> {
            let id = operation_day_id(&record, sc_id_col)?;
            let start = parse_clock(field(&record, start_col, "start_time")?)?;
            let end = parse_clock(field(&record, end_col, "end_time")?)?;
            let declared = integer_u32(&record, duration_col, "total_time_s")?;
            let on_text = field(&record, on_col, "on_shift")?;
            let on = match on_text {
                "0" => false,
                "1" => true,
                _ => return Err("on_shift_must_be_0_or_1".to_string()),
            };
            let vmt = finite_nonnegative(&record, vmt_col, "vmt")?;
            if end <= start || declared == 0 {
                return Err("segment_must_have_positive_duration_without_midnight_wrap".to_string());
            }
            if end - start != declared {
                return Err(format!(
                    "segment_duration_mismatch:clock={},declared={declared}",
                    end - start
                ));
            }
            if !on && vmt > 1.0e-9 {
                return Err("off_shift_segment_has_nonzero_vmt".to_string());
            }
            Ok(Segment {
                row_index: index,
                operation_day_id: id,
                start_second: start,
                end_second: end,
                total_seconds: declared,
                on_shift: on,
                vmt,
            })
        })();
        match parsed {
            Ok(segment) => {
                if operation_days.contains_key(&segment.operation_day_id) {
                    all_segments.push(segment);
                } else {
                    invalid_schedule_rows.push(InvalidRow {
                        row_index_zero_based: index,
                        reason: format!("unknown_operation_day_id:{}", segment.operation_day_id),
                    });
                }
            }
            Err(reason) => invalid_schedule_rows.push(InvalidRow {
                row_index_zero_based: index,
                reason,
            }),
        }
    }
    if schedule_rows == 0 {
        return Err("schedule_csv_has_no_data_rows".to_string());
    }

    let mut grouped = BTreeMap::<u64, Vec<Segment>>::new();
    for segment in &all_segments {
        grouped
            .entry(segment.operation_day_id)
            .or_default()
            .push(segment.clone());
    }
    let mut issues = Vec::<DayIssue>::new();
    let mut reconciled = Vec::<OpDay>::new();
    let mut reconciled_profiles = Vec::<ReconciledOperationDayProfile>::new();
    let mut segments_for_reconciled = 0_usize;

    for day in operation_days.values() {
        let mut reasons = Vec::new();
        let mut segments = grouped.remove(&day.id).unwrap_or_default();
        if segments.is_empty() {
            reasons.push("no_schedule_segments".to_string());
        }
        segments
            .sort_by_key(|segment| (segment.start_second, segment.end_second, segment.row_index));
        let mut expected_start = 0_u32;
        let mut schedule_on = 0_u64;
        let mut schedule_off = 0_u64;
        let mut schedule_vmt = 0.0_f64;
        for segment in &segments {
            if segment.start_second != expected_start {
                reasons.push(format!(
                    "segment_partition_gap_or_overlap_before_row_{}",
                    segment.row_index
                ));
            }
            expected_start = segment.end_second;
            if segment.on_shift {
                schedule_on += u64::from(segment.total_seconds);
                schedule_vmt += segment.vmt;
            } else {
                schedule_off += u64::from(segment.total_seconds);
            }
        }
        if expected_start != DAY_SECONDS {
            reasons.push(format!(
                "segments_end_at_{expected_start}_not_{DAY_SECONDS}"
            ));
        }
        if schedule_on.abs_diff(u64::from(day.on_shift_seconds)) > 1 {
            reasons.push(format!(
                "on_shift_seconds_mismatch:segments={schedule_on},operation_days={}",
                day.on_shift_seconds
            ));
        }
        if schedule_off.abs_diff(u64::from(day.off_shift_seconds)) > 1 {
            reasons.push(format!(
                "off_shift_seconds_mismatch:segments={schedule_off},operation_days={}",
                day.off_shift_seconds
            ));
        }
        let vmt_tolerance = 1.0e-6_f64.max(day.vmt.abs() * 1.0e-7);
        if (schedule_vmt - day.vmt).abs() > vmt_tolerance {
            reasons.push(format!("vmt_mismatch:segments={schedule_vmt:.9},operation_days={:.9},tolerance={vmt_tolerance:.9}", day.vmt));
        }
        if reasons.is_empty() {
            let first_on_shift_start_second = segments
                .iter()
                .find(|segment| segment.on_shift)
                .map(|segment| segment.start_second);
            let last_on_shift_end_second = segments
                .iter()
                .rev()
                .find(|segment| segment.on_shift)
                .map(|segment| segment.end_second);
            reconciled_profiles.push(ReconciledOperationDayProfile {
                operation_day_id: day.id,
                vmt_miles: day.vmt,
                on_shift_seconds: day.on_shift_seconds,
                off_shift_seconds: day.off_shift_seconds,
                first_on_shift_start_second,
                last_on_shift_end_second,
            });
            reconciled.push(day.clone());
            segments_for_reconciled += segments.len();
        } else {
            issues.push(DayIssue {
                operation_day_id: day.id,
                reasons,
            });
        }
    }
    let valid_schedule_rows = all_segments.len();
    let vmt_values: Vec<f64> = reconciled.iter().map(|day| day.vmt).collect();
    let offshift_hours: Vec<f64> = reconciled
        .iter()
        .map(|day| f64::from(day.off_shift_seconds) / 3600.0)
        .collect();
    let idealized_power: Vec<f64> = reconciled
        .iter()
        .filter(|day| day.off_shift_seconds > 0)
        .map(|day| {
            let hours = f64::from(day.off_shift_seconds) / 3600.0;
            day.vmt * metadata.kwh_per_mile / hours
        })
        .collect();
    let no_offshift_window_count = reconciled
        .iter()
        .filter(|day| day.off_shift_seconds == 0)
        .count();

    Ok(Report {
        schema_version: "0.1.0".to_string(),
        tool: "ev-depot-duty-audit".to_string(),
        fleet_id: metadata.fleet_id,
        dataset_id: metadata.dataset_id,
        source_uri: metadata.source_uri,
        source_revision: metadata.source_revision,
        evidence_class_declared_by_caller: metadata.evidence_class,
        kwh_per_mile_assumption: metadata.kwh_per_mile,
        expected_day_seconds: DAY_SECONDS,
        operation_days_input_rows: op_rows,
        operation_days_parsed_valid_rows: operation_days.len(),
        operation_days_invalid_rows: invalid_op_rows,
        schedule_input_rows: schedule_rows,
        schedule_parsed_valid_rows_with_known_operation_day_ids: valid_schedule_rows,
        schedule_invalid_rows: invalid_schedule_rows,
        reconciled_operation_day_count: reconciled.len(),
        operation_day_reconciliation_issues: issues,
        reconciled_operation_day_profiles: reconciled_profiles,
        schedule_segment_count_for_reconciled_days: segments_for_reconciled,
        reconciled_vmt_miles_total_across_operation_day_records: vmt_values.iter().sum(),
        daily_vmt_miles: distribution(&vmt_values),
        daily_off_shift_hours: distribution(&offshift_hours),
        idealized_min_average_recharge_power_kw_assuming_all_offshift_is_chargeable: distribution(&idealized_power),
        on_shift_seconds_total: reconciled.iter().map(|day| u64::from(day.on_shift_seconds)).sum(),
        off_shift_seconds_total: reconciled.iter().map(|day| u64::from(day.off_shift_seconds)).sum(),
        operation_days_without_offshift_charging_window: no_offshift_window_count,
        operation_day_csv_sha256: hash(operation_bytes),
        schedule_csv_sha256: hash(schedule_bytes),
        provenance_warning: "Dataset identity, source URI/revision, evidence class, kWh-per-mile assumption and licensing claims are caller-declared. File hashes identify input bytes but do not authenticate who published them. The idealized minimum recharge power assumes all off-shift time is physically available for depot charging.".to_string(),
        claim_ceiling: CLAIM_CEILING.to_string(),
    })
}

fn parse_args(args: &[String]) -> Result<(String, String, Metadata), String> {
    if args.len() < 3 {
        return Err("usage: ev-depot-duty-audit <veh_op_days.csv> <veh_schedules.csv> --fleet-id ID --dataset-id ID --source-uri HTTPS_URL --source-revision REF --evidence-class CLASS --kwh-per-mile VALUE".to_string());
    }
    let operation_path = args[1].clone();
    let schedule_path = args[2].clone();
    let mut fleet_id = None;
    let mut dataset_id = None;
    let mut source_uri = None;
    let mut source_revision = None;
    let mut evidence_class = None;
    let mut kwh_per_mile = None;
    let mut i = 3;
    while i < args.len() {
        if i + 1 >= args.len() {
            return Err(format!("{} requires a value", args[i]));
        }
        match args[i].as_str() {
            "--fleet-id" => fleet_id = Some(args[i + 1].clone()),
            "--dataset-id" => dataset_id = Some(args[i + 1].clone()),
            "--source-uri" => source_uri = Some(args[i + 1].clone()),
            "--source-revision" => source_revision = Some(args[i + 1].clone()),
            "--evidence-class" => evidence_class = Some(args[i + 1].clone()),
            "--kwh-per-mile" => {
                kwh_per_mile = Some(
                    args[i + 1]
                        .parse::<f64>()
                        .map_err(|_| "kwh-per-mile must be numeric")?,
                )
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 2;
    }
    let require = |value: Option<String>, name: &str| {
        value
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| format!("{name} is required"))
    };
    let metadata = Metadata {
        fleet_id: require(fleet_id, "--fleet-id")?,
        dataset_id: require(dataset_id, "--dataset-id")?,
        source_uri: require(source_uri, "--source-uri")?,
        source_revision: require(source_revision, "--source-revision")?,
        evidence_class: require(evidence_class, "--evidence-class")?,
        kwh_per_mile: kwh_per_mile.ok_or("--kwh-per-mile is required")?,
    };
    if !metadata.source_uri.starts_with("https://") {
        return Err("--source-uri must start with https://".to_string());
    }
    if ![
        "synthetic",
        "published_operational_summary",
        "measured",
        "unknown",
    ]
    .contains(&metadata.evidence_class.as_str())
    {
        return Err("unsupported --evidence-class".to_string());
    }
    if !metadata.kwh_per_mile.is_finite() || metadata.kwh_per_mile <= 0.0 {
        return Err("--kwh-per-mile must be finite and positive".to_string());
    }
    Ok((operation_path, schedule_path, metadata))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let (operation_path, schedule_path, metadata) = parse_args(&args)?;
    let operation_bytes =
        fs::read(&operation_path).map_err(|e| format!("cannot read {operation_path}: {e}"))?;
    let schedule_bytes =
        fs::read(&schedule_path).map_err(|e| format!("cannot read {schedule_path}: {e}"))?;
    let report = audit(&operation_bytes, &schedule_bytes, metadata)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|e| format!("cannot serialize report: {e}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ev-depot-duty-audit: {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OP_DAYS: &str =
        "veh_op_day_id,vmt,time_on_shift_s,time_off_shift_s\n1,90,32400,53999\n2,40,36000,50399\n";
    const SCHEDULES: &str = "veh_op_day_id,start_time,end_time,total_time_s,on_shift,vmt\n1,00:00:00,08:00:00,28800,0,0\n1,08:00:00,17:00:00,32400,1,90\n1,17:00:00,23:59:59,25199,0,0\n2,00:00:00,04:00:00,14400,0,0\n2,04:00:00,06:00:00,7200,1,10\n2,06:00:00,10:00:00,14400,0,0\n2,10:00:00,18:00:00,28800,1,30\n2,18:00:00,23:59:59,21599,0,0\n";

    fn metadata() -> Metadata {
        Metadata {
            fleet_id: "synthetic-fleet".to_string(),
            dataset_id: "synthetic-fixture-v0.1".to_string(),
            source_uri: "https://example.invalid/source".to_string(),
            source_revision: "fixture-v0.1".to_string(),
            evidence_class: "synthetic".to_string(),
            kwh_per_mile: 1.8,
        }
    }

    #[test]
    fn reconciles_operation_days_against_schedule_segments() {
        let report = audit(OP_DAYS.as_bytes(), SCHEDULES.as_bytes(), metadata()).unwrap();
        assert_eq!(report.operation_days_input_rows, 2);
        assert_eq!(report.operation_days_parsed_valid_rows, 2);
        assert_eq!(report.schedule_input_rows, 8);
        assert_eq!(
            report.schedule_parsed_valid_rows_with_known_operation_day_ids,
            8
        );
        assert_eq!(report.reconciled_operation_day_count, 2);
        assert_eq!(report.reconciled_operation_day_profiles.len(), 2);
        assert_eq!(
            report.reconciled_operation_day_profiles[0].first_on_shift_start_second,
            Some(8 * 3600)
        );
        assert_eq!(
            report.reconciled_operation_day_profiles[0].last_on_shift_end_second,
            Some(17 * 3600)
        );
        assert_eq!(
            report.reconciled_operation_day_profiles[1].first_on_shift_start_second,
            Some(4 * 3600)
        );
        assert_eq!(
            report.reconciled_operation_day_profiles[1].last_on_shift_end_second,
            Some(18 * 3600)
        );
        assert!(report.operation_day_reconciliation_issues.is_empty());
        assert_eq!(report.daily_vmt_miles.p50, Some(65.0));
        assert_eq!(report.daily_vmt_miles.p95, Some(90.0));
        assert_eq!(
            report.reconciled_vmt_miles_total_across_operation_day_records,
            130.0
        );
        assert_eq!(report.on_shift_seconds_total, 68_400);
        assert_eq!(report.off_shift_seconds_total, 104_398);
        assert_eq!(report.operation_day_csv_sha256.len(), 64);
    }

    #[test]
    fn detects_missing_schedule_day_and_invalid_row() {
        let days = "veh_op_day_id,vmt,time_on_shift_s,time_off_shift_s\n1,90,32400,53999\n2,40,36000,50399\n";
        let schedules = "veh_op_day_id,start_time,end_time,total_time_s,on_shift,vmt\n1,00:00:00,08:00:00,28800,0,0\n";
        let report = audit(days.as_bytes(), schedules.as_bytes(), metadata()).unwrap();
        assert_eq!(report.reconciled_operation_day_count, 0);
        assert_eq!(report.operation_day_reconciliation_issues.len(), 2);
        assert!(
            report
                .operation_day_reconciliation_issues
                .iter()
                .any(|issue| issue.operation_day_id == 2
                    && issue.reasons.contains(&"no_schedule_segments".to_string()))
        );
    }

    #[test]
    fn rejects_bad_clock_duration_and_nonfinite_values() {
        assert!(parse_clock("24:00:00").is_err());
        assert!(parse_clock("08:61:00").is_err());
        let days = "veh_op_day_id,vmt,time_on_shift_s,time_off_shift_s\n1,NaN,32400,53999\n";
        assert!(
            audit(days.as_bytes(), SCHEDULES.as_bytes(), metadata())
                .unwrap()
                .operation_days_invalid_rows
                .len()
                == 1
        );
    }

    #[test]
    fn reports_distributions_as_unavailable_for_empty_samples() {
        let values = distribution(&[]);
        assert_eq!(values.count, 0);
        assert_eq!(values.p50, None);
        assert_eq!(values.p95, None);
    }
    #[test]
    fn zero_offshift_window_is_reported_not_divided_by_zero() {
        let days = "veh_op_day_id,vmt,time_on_shift_s,time_off_shift_s\n1,1,86399,0\n";
        let schedules = "veh_op_day_id,start_time,end_time,total_time_s,on_shift,vmt\n1,00:00:00,23:59:59,86399,1,1\n";
        let report = audit(days.as_bytes(), schedules.as_bytes(), metadata()).unwrap();
        assert_eq!(report.operation_days_without_offshift_charging_window, 1);
        assert_eq!(
            report
                .idealized_min_average_recharge_power_kw_assuming_all_offshift_is_chargeable
                .count,
            0
        );
    }
}
